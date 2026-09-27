//! Adapter/binary wiring for `work resolve`: config resolution, the `Path`
//! class (handled directly), identity lookup over every item's `id`,
//! `aliases` and `external_id`, and, when no item's identity names the
//! token, the filename cascade of `work::resolve::resolve`.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use config_adapters::FileConfigStore;
use corpus::WorkItemIdScheme;
use work::identity::resolve_identity;
use work::identity::IdentityField;
use work::identity::IdentityMatch;
use work::identity::IdentityResolution;
use work::resolve::classify_input;
use work::resolve::resolve as domain_resolve;
use work::resolve::InputClass;
use work::resolve::ResolveOutcome;
use work::resolve::SearchClass;
use work::resolve::TaggedCandidate;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles;
use work_adapters::filesystem::FilesystemLister;
use work_adapters::filesystem::FilesystemWorkItemFiles;

use crate::config::resolve_scheme;
use crate::config::resolve_work_dir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCandidate {
    pub path: PathBuf,
    pub field: IdentityField,
}

impl From<IdentityMatch<'_>> for IdentityCandidate {
    fn from(found: IdentityMatch<'_>) -> Self {
        Self {
            path: found.item.path.clone(),
            field: found.field,
        }
    }
}

/// The outcome `main` maps to the binary's exit codes: `Resolved` → 0,
/// `Ambiguous` and `Conflicting` → 2, `NotFound` → 3, `Invalid` → 1,
/// `OutsideWorkDir` → 6.
#[derive(Debug)]
pub enum RunOutcome {
    Resolved(PathBuf),
    Ambiguous(Vec<TaggedCandidate>),
    Conflicting(Vec<IdentityCandidate>),
    NotFound(String),
    Invalid(String),
    OutsideWorkDir(String),
}

fn resolve_path_class(root: &Path, start: &Path, input: &str) -> RunOutcome {
    let candidate = start.join(input);
    let Ok(resolved) = candidate.canonicalize() else {
        return RunOutcome::NotFound(format!("no work item at path '{input}'"));
    };
    if !resolved.starts_with(root) {
        return RunOutcome::OutsideWorkDir(format!(
            "path '{input}' is outside the work directory {}",
            root.display()
        ));
    }
    if resolved.is_file() {
        RunOutcome::Resolved(resolved)
    } else {
        RunOutcome::NotFound(format!("no work item at path '{input}'"))
    }
}

/// # Errors
///
/// A [`kernel::Error`] when the configuration cannot be read.
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    input: &str,
) -> Result<RunOutcome, kernel::Error> {
    let scheme = resolve_scheme(config)?;
    let work_dir = canonical_work_dir(start, config)?;
    Ok(resolve_with(&scheme, &work_dir, start, input))
}

/// Resolves the canonical work directory once, so a caller resolving many
/// tokens reads config a single time and then resolves each infallibly through
/// [`resolve_with`].
///
/// # Errors
///
/// A [`kernel::Error`] when the configuration cannot be read or the work
/// directory cannot be canonicalised — an environment fault, not a missing
/// item.
pub fn canonical_work_dir(
    start: &Path,
    config: &dyn ConfigAccess,
) -> Result<PathBuf, kernel::Error> {
    let root = FileConfigStore::discover_root(start);
    let work_dir = resolve_work_dir(config, &root)?;
    work_dir.canonicalize().map_err(|error| {
        kernel::Error::Failed(format!(
            "could not resolve the work directory {}: {error}",
            work_dir.display()
        ))
    })
}

/// The infallible core: resolve `input` against the already resolved
/// `scheme` and canonical `work_dir`. `Resolved` carries a canonicalised path
/// so a caller can match it against an equally-canonicalised managed-item
/// set.
#[must_use]
pub fn resolve_with(
    scheme: &WorkItemIdScheme,
    work_dir: &Path,
    start: &Path,
    input: &str,
) -> RunOutcome {
    let class = classify_input(input, scheme);
    if class == InputClass::Path {
        return resolve_path_class(work_dir, start, input);
    }
    let files = FilesystemWorkItemFiles::new(work_dir)
        .files()
        .unwrap_or_default();
    let identities = identities(&files);
    match resolve_identity(input, &identities) {
        IdentityResolution::Unique(item) => {
            RunOutcome::Resolved(item.path.clone())
        }
        IdentityResolution::Conflicting(matches) => RunOutcome::Conflicting(
            matches.into_iter().map(IdentityCandidate::from).collect(),
        ),
        IdentityResolution::Unmatched => {
            resolve_by_filename(class, scheme, work_dir, input)
        }
    }
}

fn resolve_by_filename(
    class: InputClass,
    scheme: &WorkItemIdScheme,
    work_dir: &Path,
    input: &str,
) -> RunOutcome {
    let search_class = match class {
        InputClass::FullId => SearchClass::FullId,
        InputClass::BareNumber => SearchClass::BareNumber,
        InputClass::Path | InputClass::Invalid => {
            return RunOutcome::Invalid(format!(
                "input '{input}' is not a recognised path, full ID, or bare \
                 number"
            ));
        }
    };
    let lister = FilesystemLister::new(work_dir);
    match domain_resolve(input, search_class, scheme, &lister) {
        ResolveOutcome::Single(filename) => {
            RunOutcome::Resolved(work_dir.join(filename))
        }
        ResolveOutcome::Ambiguous(candidates) => {
            RunOutcome::Ambiguous(candidates)
        }
        ResolveOutcome::NotFound => {
            let noun = if search_class == SearchClass::FullId {
                "ID"
            } else {
                "bare number"
            };
            RunOutcome::NotFound(format!(
                "no work item matching {noun} '{input}' in {}",
                work_dir.display()
            ))
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use corpus::WorkItemIdScheme;
    use work::identity::IdentityField;

    use super::resolve_with;
    use super::IdentityCandidate;
    use super::RunOutcome;

    fn corpus() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let work_dir = dir.path().canonicalize().expect("canonical work dir");
        std::fs::create_dir_all(work_dir.join("drafts")).expect("drafts dir");
        (dir, work_dir)
    }

    fn item(path: &Path, frontmatter: &str) -> PathBuf {
        std::fs::write(path, format!("---\n{frontmatter}---\n\n# Title\n"))
            .expect("write item");
        path.to_path_buf()
    }

    fn tracker() -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: corpus::TRACKER_TOKEN.to_owned(),
            key: None,
        }
    }

    fn resolved(outcome: RunOutcome) -> Option<PathBuf> {
        match outcome {
            RunOutcome::Resolved(path) => Some(path),
            _ => None,
        }
    }

    #[test]
    fn work_resolve_returns_a_draft_path() {
        let (_dir, work_dir) = corpus();
        let draft = item(
            &work_dir.join("drafts/draft-k7mq3x-title.md"),
            "id: \"draft-k7mq3x\"\n",
        );

        let outcome = resolve_with(
            &WorkItemIdScheme::numeric(),
            &work_dir,
            &work_dir,
            "draft-k7mq3x",
        );

        assert_eq!(resolved(outcome), Some(draft));
    }

    #[test]
    fn work_resolve_matches_an_alias_case_insensitively() {
        let (_dir, work_dir) = corpus();
        let promoted = item(
            &work_dir.join("ENG-42-title.md"),
            "id: \"ENG-42\"\naliases: [\"draft-k7mq3x\"]\n\
             external_id: \"ENG-42\"\n",
        );

        let outcome =
            resolve_with(&tracker(), &work_dir, &work_dir, "DRAFT-K7MQ3X");

        assert_eq!(resolved(outcome), Some(promoted));
    }

    #[test]
    fn work_resolve_pp_900_and_lowercase_resolve_an_item_whose_id_and_external_id_agree(
    ) {
        let (_dir, work_dir) = corpus();
        let keyed = item(
            &work_dir.join("PP-900-title.md"),
            "id: \"PP-900\"\nexternal_id: \"PP-900\"\n",
        );

        for token in ["PP-900", "pp-900"] {
            let outcome = resolve_with(&tracker(), &work_dir, &work_dir, token);
            assert_eq!(resolved(outcome), Some(keyed.clone()), "{token}");
        }
    }

    #[test]
    fn an_unknown_tracker_key_under_tracker_is_not_found() {
        let (_dir, work_dir) = corpus();
        item(
            &work_dir.join("PP-900-title.md"),
            "id: \"PP-900\"\nexternal_id: \"PP-900\"\n",
        );

        let outcome = resolve_with(&tracker(), &work_dir, &work_dir, "ENG-999");

        assert!(matches!(outcome, RunOutcome::NotFound(_)));
    }

    #[test]
    fn a_bare_legacy_number_under_tracker_resolves_the_padded_item() {
        let (_dir, work_dir) = corpus();
        let legacy = item(
            &work_dir.join("0042-title.md"),
            "id: \"0042\"\nexternal_id: \"PP-760\"\n",
        );

        let outcome = resolve_with(&tracker(), &work_dir, &work_dir, "42");

        assert_eq!(resolved(outcome), Some(legacy));
    }

    #[test]
    fn work_resolve_exits_ambiguous_naming_both_items_and_fields() {
        let (_dir, work_dir) = corpus();
        let linked = item(
            &work_dir.join("0001-title.md"),
            "id: \"0001\"\nexternal_id: \"ENG-42\"\n",
        );
        let retired = item(
            &work_dir.join("ENG-7-title.md"),
            "id: \"ENG-7\"\naliases: [\"ENG-42\"]\n",
        );

        let outcome = resolve_with(&tracker(), &work_dir, &work_dir, "eng-42");

        let RunOutcome::Conflicting(candidates) = outcome else {
            unreachable!("two items claim ENG-42")
        };
        assert_eq!(
            candidates,
            vec![
                IdentityCandidate {
                    path: linked,
                    field: IdentityField::ExternalId,
                },
                IdentityCandidate {
                    path: retired,
                    field: IdentityField::Alias,
                },
            ]
        );
    }
}
