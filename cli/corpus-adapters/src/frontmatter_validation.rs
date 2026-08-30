//! Whole-corpus walk, the referential-integrity index, and per-file
//! orchestration for frontmatter validation.
//!
//! The pure per-file/per-reference checks live in
//! `corpus::frontmatter_validation`; this module owns the filesystem walk,
//! the whole-corpus index build, and wiring the two check categories
//! (structural, referential) together per file.

use std::path::Path;
use std::path::PathBuf;

use corpus::frontmatter_validation::dangling_refs;
use corpus::frontmatter_validation::declared_type;
use corpus::frontmatter_validation::duplicate_check;
use corpus::frontmatter_validation::parse_entries;
use corpus::frontmatter_validation::raw_value;
use corpus::frontmatter_validation::strip_surrounding_quote;
use corpus::frontmatter_validation::validate_file;
use corpus::frontmatter_validation::Index;
use corpus::frontmatter_validation::Violation;
use corpus::scan::CorpusWalker;
use corpus::scan::FileReader;
use corpus::DocTypeKey;

use crate::document::parse as classify;
use crate::document::FrontmatterState;

/// Every `*.md` file under any directory in `table`, deduplicated (a
/// configured directory nested under another would otherwise be walked
/// twice).
///
/// # Errors
///
/// A [`kernel::Error`] when a configured directory exists but cannot be
/// walked.
pub fn corpus_files<W: CorpusWalker>(
    table: &[(DocTypeKey, PathBuf)],
    walker: &W,
) -> Result<Vec<PathBuf>, kernel::Error> {
    let roots: Vec<PathBuf> =
        table.iter().map(|(_, dir)| dir.clone()).collect();
    let mut files = walker.walk_markdown(&roots)?;
    files.sort();
    files.dedup();
    Ok(files)
}

/// The frontmatter text between a file's fences, or `None` when the content
/// isn't `Parsed` (no fence, unclosed fence, or unparseable YAML).
fn parsed_frontmatter_text(content: &str) -> Option<String> {
    let classified = classify(content.as_bytes());
    if !matches!(classified.state, FrontmatterState::Parsed(_)) {
        return None;
    }
    document::split(content).ok().map(|split| split.frontmatter)
}

/// A file's own resolved `(type, id)` key, `type:id`-joined — the same
/// resolution `build_index` performs for every corpus file, factored out so
/// a file's own key (needed for the [`DuplicateId`](Violation::DuplicateId)
/// check) can never drift from what the index itself recorded for it.
///
/// Type: the frontmatter's own `type:` field, else path-inferred through
/// `table`. Id: `id:`, else `work_item_id:`, else `adr_id:`, else the
/// filename stem. `None` when neither a type nor an id can be resolved.
fn resolve_own_type_id(
    raw_frontmatter: &str,
    path: &Path,
    table: &[(DocTypeKey, PathBuf)],
) -> Option<String> {
    let entries = parse_entries(raw_frontmatter);
    let type_name = declared_type(&entries)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            corpus::linkage::type_from_path(&path.to_string_lossy(), table)
                .map(str::to_owned)
        })?;

    let id = ["id", "work_item_id", "adr_id"]
        .into_iter()
        .find_map(|key| {
            let inner = strip_surrounding_quote(raw_value(&entries, key)?);
            (!inner.is_empty()).then(|| inner.to_owned())
        })
        .or_else(|| {
            let stem = path.file_stem()?.to_str()?;
            (!stem.is_empty()).then(|| stem.to_owned())
        })?;

    Some(format!("{type_name}:{id}"))
}

/// Builds the whole-corpus referential-integrity index.
///
/// Every file under a configured doc-type directory whose frontmatter is
/// `Parsed`, keyed by its own resolved `(type, id)`. A `Malformed` or
/// `Absent` file is excluded — its type/id cannot be trusted, so it must
/// not be usable as a reference target (a deliberate improvement on bash's
/// own less careful index).
///
/// # Errors
///
/// A [`kernel::Error`] when the walk or a file read fails.
pub fn build_index<W: CorpusWalker + FileReader>(
    table: &[(DocTypeKey, PathBuf)],
    walker: &W,
) -> Result<Index, kernel::Error> {
    let mut index = Index::new();
    for path in corpus_files(table, walker)? {
        let Some(content) = walker.read(&path)? else {
            continue;
        };
        let Some(frontmatter) = parsed_frontmatter_text(&content) else {
            continue;
        };
        let Some(type_id) = resolve_own_type_id(&frontmatter, &path, table)
        else {
            continue;
        };
        index.insert(type_id, path);
    }
    Ok(index)
}

/// Validates one file's structural conformance.
///
/// `Ok(None)` for the missing/unreadable-file case degrading to
/// [`Violation::NoFence`] — matching the retired bash implementation's own
/// fence check, which treats a nonexistent file identically to one with no
/// fence, never surfacing a distinct "file not found" message.
///
/// # Errors
///
/// A [`kernel::Error`] when the file read fails for a reason other than
/// absence.
pub fn validate_path<F: FileReader>(
    path: &Path,
    file_reader: &F,
) -> Result<Vec<Violation>, kernel::Error> {
    let Some(content) = file_reader.read(path)? else {
        return Ok(vec![Violation::NoFence]);
    };
    Ok(parsed_frontmatter_text(&content)
        .map_or_else(|| vec![Violation::NoFence], |fm| validate_file(&fm)))
}

/// Which check categories a `frontmatter validate` invocation has enabled.
///
/// `canonical` gates the canonical-quoting check (`UNQUOTED-STRING`)
/// independently of the rest of the structural checks: it is enforced by the
/// standard-checking `validate` command but suppressed for a migration's own
/// in-process self-check, since a migration that predates the canonical
/// standard emits schema-valid but unquoted frontmatter.
#[derive(Debug, Clone, Copy)]
pub struct Checks {
    pub structure: bool,
    pub references: bool,
    pub canonical: bool,
}

/// The outcome `validate_targets` reports for one target file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetOutcome {
    Violations(Vec<Violation>),
    /// Structural checks were disabled and this file is ineligible for the
    /// referential-integrity check that ran (it would fail
    /// `NO-FENCE`/`INVALID-TYPE`) — reported explicitly so a
    /// `--checks references`-only run over a broken file can never be
    /// mistaken for "0 violations, fully clean".
    Skipped,
}

fn structural_gate_failed(violations: &[Violation]) -> bool {
    violations.iter().any(|violation| {
        matches!(
            violation,
            Violation::NoFence | Violation::InvalidType { .. }
        )
    })
}

/// Validates every file in `files` under the enabled `checks`, against the
/// always-whole-corpus `index`.
///
/// # Errors
///
/// A [`kernel::Error`] when a file read fails for a reason other than
/// absence.
pub fn validate_targets<F: FileReader>(
    files: &[PathBuf],
    table: &[(DocTypeKey, PathBuf)],
    index: &Index,
    checks: Checks,
    file_reader: &F,
) -> Result<Vec<(PathBuf, TargetOutcome)>, kernel::Error> {
    let mut results = Vec::with_capacity(files.len());
    for path in files {
        let mut structural = validate_path(path, file_reader)?;
        if !checks.canonical {
            structural.retain(|violation| {
                !matches!(violation, Violation::UnquotedString { .. })
            });
        }
        let gate_failed = structural_gate_failed(&structural);

        let mut emitted = if checks.structure {
            structural
        } else {
            Vec::new()
        };

        if checks.references && !gate_failed {
            if let Some(content) = file_reader.read(path)? {
                if let Some(frontmatter) = parsed_frontmatter_text(&content) {
                    emitted.extend(dangling_refs(&frontmatter, index));
                    if let Some(type_id) =
                        resolve_own_type_id(&frontmatter, path, table)
                    {
                        emitted.extend(duplicate_check(&type_id, index));
                    }
                }
            }
        }

        if !checks.structure && checks.references && gate_failed {
            results.push((path.clone(), TargetOutcome::Skipped));
        } else {
            results.push((path.clone(), TargetOutcome::Violations(emitted)));
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;

    use corpus::frontmatter_validation::Violation;
    use corpus::scan::CorpusWalker;
    use corpus::scan::FileReader;
    use corpus::DocTypeKey;

    use super::{
        build_index, corpus_files, validate_path, validate_targets, Checks,
        TargetOutcome,
    };

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
            (DocTypeKey::Decisions, PathBuf::from("meta/decisions")),
        ]
    }

    fn work_item(id: &str, parent: Option<&str>) -> String {
        let parent_line = parent
            .map(|value| format!("parent: \"{value}\"\n"))
            .unwrap_or_default();
        format!(
            "---\ntype: \"work-item\"\nid: \"{id}\"\ntitle: \"t\"\n\
             date: \"2026-01-01T00:00:00Z\"\nauthor: \"a\"\ntags: []\n\
             last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: \"a\"\n\
             schema_version: 1\nstatus: \"draft\"\nkind: \"task\"\n\
             priority: \"normal\"\n{parent_line}---\nbody\n"
        )
    }

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn corpus_files_deduplicates_a_file_reachable_from_two_roots(
    ) -> Result<(), TestError> {
        let mut walker = StubFs::default();
        walker.roots.insert(
            PathBuf::from("meta/work"),
            vec![PathBuf::from("meta/work/0001.md")],
        );
        walker.roots.insert(
            PathBuf::from("meta"),
            vec![PathBuf::from("meta/work/0001.md")],
        );
        let table = vec![
            (DocTypeKey::WorkItems, PathBuf::from("meta/work")),
            (DocTypeKey::Plans, PathBuf::from("meta")),
        ];
        let files = corpus_files(&table, &walker)?;
        assert_eq!(files, vec![PathBuf::from("meta/work/0001.md")]);
        Ok(())
    }

    #[test]
    fn validate_path_reports_no_fence_for_a_missing_file(
    ) -> Result<(), TestError> {
        let walker = StubFs::default();
        let violations =
            validate_path(Path::new("meta/work/0001.md"), &walker)?;
        assert_eq!(violations, vec![Violation::NoFence]);
        Ok(())
    }

    #[test]
    fn validate_path_reports_no_fence_for_unparseable_yaml(
    ) -> Result<(), TestError> {
        let walker = StubFs::default().with_file(
            "meta/work/0001.md",
            "---\nkey: !custom value\n---\nbody\n",
        );
        let violations =
            validate_path(Path::new("meta/work/0001.md"), &walker)?;
        assert_eq!(violations, vec![Violation::NoFence]);
        Ok(())
    }

    #[test]
    fn validate_path_validates_a_well_formed_file() -> Result<(), TestError> {
        let content = work_item("0001", None);
        let walker = StubFs::default().with_file("meta/work/0001.md", &content);
        let violations =
            validate_path(Path::new("meta/work/0001.md"), &walker)?;
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn build_index_excludes_a_malformed_file_from_reference_resolution(
    ) -> Result<(), TestError> {
        let mut walker = StubFs::default()
            .with_file(
                "meta/work/0001.md",
                "---\nkey: !custom value\n---\nbody\n",
            )
            .with_file("meta/work/0002.md", &work_item("0002", None));
        walker.roots.insert(
            PathBuf::from("meta/work"),
            vec![
                PathBuf::from("meta/work/0001.md"),
                PathBuf::from("meta/work/0002.md"),
            ],
        );
        walker.roots.insert(PathBuf::from("meta/plans"), Vec::new());
        walker
            .roots
            .insert(PathBuf::from("meta/decisions"), Vec::new());

        let index = build_index(&table(), &walker)?;
        assert!(!index.contains("work-item:0001"));
        assert!(index.contains("work-item:0002"));
        Ok(())
    }

    #[test]
    fn validate_targets_flags_a_dangling_reference_selected_via_files(
    ) -> Result<(), TestError> {
        let content = work_item("0001", Some("work-item:9999"));
        let mut walker =
            StubFs::default().with_file("meta/work/0001.md", &content);
        walker.roots.insert(PathBuf::from("meta/work"), Vec::new());
        walker.roots.insert(PathBuf::from("meta/plans"), Vec::new());
        walker
            .roots
            .insert(PathBuf::from("meta/decisions"), Vec::new());

        let index = build_index(&table(), &walker)?;
        let results = validate_targets(
            &[PathBuf::from("meta/work/0001.md")],
            &table(),
            &index,
            Checks {
                structure: true,
                references: true,
                canonical: true,
            },
            &walker,
        )?;

        let TargetOutcome::Violations(violations) = &results[0].1 else {
            return Err("expected Violations".into());
        };
        assert!(violations
            .iter()
            .any(|v| matches!(v, Violation::DanglingRef { .. })));
        Ok(())
    }

    #[test]
    fn validate_targets_never_reaches_dangling_refs_for_a_file_that_fails_the_structural_gate(
    ) -> Result<(), TestError> {
        let mut walker = StubFs::default().with_file(
            "meta/work/0001.md",
            "---\ntype: not-a-real-type\n---\nbody\n",
        );
        walker.roots.insert(PathBuf::from("meta/work"), Vec::new());
        walker.roots.insert(PathBuf::from("meta/plans"), Vec::new());
        walker
            .roots
            .insert(PathBuf::from("meta/decisions"), Vec::new());
        let index = build_index(&table(), &walker)?;

        let results = validate_targets(
            &[PathBuf::from("meta/work/0001.md")],
            &table(),
            &index,
            Checks {
                structure: true,
                references: true,
                canonical: true,
            },
            &walker,
        )?;

        let TargetOutcome::Violations(violations) = &results[0].1 else {
            return Err("expected Violations".into());
        };
        assert_eq!(violations.len(), 1);
        assert!(matches!(violations[0], Violation::InvalidType { .. }));
        Ok(())
    }

    #[test]
    fn validate_targets_reports_skipped_when_structure_is_disabled_and_the_file_fails_the_gate(
    ) -> Result<(), TestError> {
        let mut walker = StubFs::default();
        walker.roots.insert(PathBuf::from("meta/work"), Vec::new());
        walker.roots.insert(PathBuf::from("meta/plans"), Vec::new());
        walker
            .roots
            .insert(PathBuf::from("meta/decisions"), Vec::new());
        let index = build_index(&table(), &walker)?;

        let results = validate_targets(
            &[PathBuf::from("meta/work/missing.md")],
            &table(),
            &index,
            Checks {
                structure: false,
                references: true,
                canonical: false,
            },
            &walker,
        )?;

        assert_eq!(results[0].1, TargetOutcome::Skipped);
        Ok(())
    }

    #[test]
    fn validate_targets_duplicate_id_fires_for_two_files_sharing_a_key(
    ) -> Result<(), TestError> {
        let mut walker = StubFs::default()
            .with_file("meta/work/0001-a.md", &work_item("0001", None));
        walker.files.insert(
            PathBuf::from("meta/work/0001-b.md"),
            work_item("0001", None),
        );
        walker.roots.insert(
            PathBuf::from("meta/work"),
            vec![
                PathBuf::from("meta/work/0001-a.md"),
                PathBuf::from("meta/work/0001-b.md"),
            ],
        );
        walker.roots.insert(PathBuf::from("meta/plans"), Vec::new());
        walker
            .roots
            .insert(PathBuf::from("meta/decisions"), Vec::new());

        let index = build_index(&table(), &walker)?;
        let results = validate_targets(
            &[PathBuf::from("meta/work/0001-a.md")],
            &table(),
            &index,
            Checks {
                structure: true,
                references: true,
                canonical: true,
            },
            &walker,
        )?;

        let TargetOutcome::Violations(violations) = &results[0].1 else {
            return Err("expected Violations".into());
        };
        assert!(violations
            .iter()
            .any(|v| matches!(v, Violation::DuplicateId { .. })));
        Ok(())
    }
}
