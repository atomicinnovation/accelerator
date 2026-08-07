//! The `frontmatter` command layer: target-set resolution and rendering
//! only, generic over the injected walk/read port — no
//! `config_adapters::compose` call and no direct `std::fs` access lives
//! here.

use std::path::Path;
use std::path::PathBuf;

use corpus::scan::CorpusWalker;
use corpus::scan::FileReader;
use corpus::DocTypeKey;
use corpus_adapters::frontmatter_validation::build_index;
use corpus_adapters::frontmatter_validation::corpus_files;
use corpus_adapters::frontmatter_validation::validate_targets;
use corpus_adapters::frontmatter_validation::Checks;
use corpus_adapters::frontmatter_validation::TargetOutcome;

use crate::outcome::Outcome;

/// Files under `dirs` are filtered to those resolving to a configured
/// doc-type, matching bash's own `out_of_scope` gate on its whole-corpus
/// walk — a `--dir meta` call must skip `meta/docs/` the same way bash's
/// validator did, not report every `*.md` under the given directory. Files
/// named directly via `--file` are never filtered, matching bash's file-list
/// mode (which never scope-checked its arguments either).
fn target_files<W: CorpusWalker>(
    dirs: &[PathBuf],
    files: &[PathBuf],
    table: &[(DocTypeKey, PathBuf)],
    walker: &W,
) -> Result<Vec<PathBuf>, kernel::Error> {
    if dirs.is_empty() && files.is_empty() {
        return corpus_files(table, walker);
    }
    let mut target: Vec<PathBuf> = walker
        .walk_markdown(dirs)?
        .into_iter()
        .filter(|path| {
            corpus::linkage::type_from_path(&path.to_string_lossy(), table)
                .is_some()
        })
        .collect();
    target.extend(files.iter().cloned());
    target.sort();
    target.dedup();
    Ok(target)
}

fn render(path: &Path, outcome: &TargetOutcome, stderr: &mut String) {
    match outcome {
        TargetOutcome::Violations(violations) => {
            for violation in violations {
                stderr.push_str(&path.to_string_lossy());
                stderr.push_str(": ");
                stderr.push_str(&violation.to_string());
                stderr.push('\n');
            }
        }
        TargetOutcome::Skipped => {
            stderr.push_str(&path.to_string_lossy());
            stderr.push_str(
                ": SKIPPED — structural checks disabled and this file is \
                 ineligible for the referential-integrity check that ran \
                 (would fail NO-FENCE/INVALID-TYPE)\n",
            );
        }
    }
}

/// Runs `frontmatter validate`.
///
/// # Errors
///
/// A [`kernel::Error`] when the walk or a file read fails, or when any file
/// carries a violation.
pub fn run_validate<W: CorpusWalker + FileReader>(
    dirs: &[PathBuf],
    files: &[PathBuf],
    checks: Checks,
    table: &[(DocTypeKey, PathBuf)],
    walker_and_reader: &W,
) -> Result<Outcome, kernel::Error> {
    let index = build_index(table, walker_and_reader)?;
    let target = target_files(dirs, files, table, walker_and_reader)?;
    let results =
        validate_targets(&target, table, &index, checks, walker_and_reader)?;

    let mut stderr = String::new();
    let mut any_violations = false;
    for (path, outcome) in &results {
        if matches!(outcome, TargetOutcome::Violations(violations) if !violations.is_empty())
        {
            any_violations = true;
        }
        render(path, outcome, &mut stderr);
    }

    if any_violations {
        return Err(kernel::Error::Failed(stderr));
    }
    Ok(Outcome {
        stdout: String::new(),
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;

    use corpus::scan::CorpusWalker;
    use corpus::scan::FileReader;
    use corpus::DocTypeKey;
    use corpus_adapters::frontmatter_validation::Checks;

    use super::run_validate;

    #[derive(Default)]
    struct StubFs {
        files: HashMap<PathBuf, String>,
        roots: HashMap<PathBuf, Vec<PathBuf>>,
    }

    impl StubFs {
        fn with_file(mut self, path: &str, content: &str) -> Self {
            self.files.insert(PathBuf::from(path), content.to_owned());
            self
        }

        fn with_root(mut self, root: &str, files: &[&str]) -> Self {
            self.roots.insert(
                PathBuf::from(root),
                files.iter().map(PathBuf::from).collect(),
            );
            self
        }
    }

    impl FileReader for StubFs {
        fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
            Ok(self.files.get(path).cloned())
        }
    }

    impl CorpusWalker for StubFs {
        fn walk_markdown(
            &self,
            roots: &[PathBuf],
        ) -> Result<Vec<PathBuf>, kernel::Error> {
            let mut found = Vec::new();
            for root in roots {
                if let Some(files) = self.roots.get(root) {
                    found.extend(files.iter().cloned());
                }
            }
            Ok(found)
        }
    }

    fn table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![
            (DocTypeKey::WorkItems, PathBuf::from("meta/work")),
            (DocTypeKey::Plans, PathBuf::from("meta/plans")),
        ]
    }

    fn valid_work_item() -> &'static str {
        "---\ntype: work-item\nid: \"0001\"\ntitle: t\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
         schema_version: 1\nstatus: draft\nkind: task\npriority: normal\n\
         ---\nbody\n"
    }

    fn both_checks() -> Checks {
        Checks {
            structure: true,
            references: true,
        }
    }

    #[test]
    fn with_no_dir_or_file_the_whole_corpus_is_the_target(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let walker = StubFs::default()
            .with_file("meta/work/0001.md", valid_work_item())
            .with_root("meta/work", &["meta/work/0001.md"])
            .with_root("meta/plans", &[]);
        let outcome = run_validate(&[], &[], both_checks(), &table(), &walker)?;
        assert_eq!(outcome.stderr, String::new());
        Ok(())
    }

    #[test]
    fn dir_only_walks_the_given_directory(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let walker = StubFs::default()
            .with_file("meta/work/0001.md", valid_work_item())
            .with_root("meta/work", &["meta/work/0001.md"]);
        let outcome = run_validate(
            &[PathBuf::from("meta/work")],
            &[],
            both_checks(),
            &table(),
            &walker,
        )?;
        assert_eq!(outcome.stderr, String::new());
        Ok(())
    }

    #[test]
    fn file_only_validates_the_named_file(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let walker =
            StubFs::default().with_file("meta/work/0001.md", valid_work_item());
        let outcome = run_validate(
            &[],
            &[PathBuf::from("meta/work/0001.md")],
            both_checks(),
            &table(),
            &walker,
        )?;
        assert_eq!(outcome.stderr, String::new());
        Ok(())
    }

    #[test]
    fn dir_and_file_together_deduplicate_the_union(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let walker = StubFs::default()
            .with_file("meta/work/0001.md", valid_work_item())
            .with_root("meta/work", &["meta/work/0001.md"]);
        let outcome = run_validate(
            &[PathBuf::from("meta/work")],
            &[PathBuf::from("meta/work/0001.md")],
            both_checks(),
            &table(),
            &walker,
        )?;
        assert_eq!(outcome.stderr, String::new());
        Ok(())
    }

    #[test]
    fn a_violation_fails_the_run() {
        let walker = StubFs::default()
            .with_file("meta/work/0001.md", "---\ntype: bogus\n---\nbody\n");
        let result = run_validate(
            &[],
            &[PathBuf::from("meta/work/0001.md")],
            both_checks(),
            &table(),
            &walker,
        );
        assert!(result.is_err());
    }

    #[test]
    fn checks_structure_only_omits_referential_violations(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = "---\ntype: work-item\nid: \"0001\"\ntitle: t\n\
             date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
             last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
             schema_version: 1\nstatus: draft\nkind: task\npriority: normal\n\
             parent: \"work-item:9999\"\n---\nbody\n";
        let walker = StubFs::default().with_file("meta/work/0001.md", content);
        let outcome = run_validate(
            &[],
            &[PathBuf::from("meta/work/0001.md")],
            Checks {
                structure: true,
                references: false,
            },
            &table(),
            &walker,
        )?;
        assert_eq!(outcome.stderr, String::new());
        Ok(())
    }

    #[test]
    fn checks_references_only_against_a_fenceless_file_is_skipped_not_clean(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let walker = StubFs::default();
        let outcome = run_validate(
            &[],
            &[PathBuf::from("meta/work/missing.md")],
            Checks {
                structure: false,
                references: true,
            },
            &table(),
            &walker,
        )?;
        assert!(outcome.stderr.contains("SKIPPED"));
        Ok(())
    }
}
