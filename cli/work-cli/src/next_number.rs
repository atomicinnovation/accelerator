//! Adapter/binary wiring for `work next-number`: a thin display wrapper
//! around `work::next_number::allocate`, or `work::draft_id::mint_draft_ids`
//! under `{tracker}`. It writes nothing in the corpus and commits no number;
//! it takes the identity-run lock under `.accelerator/state/`, so it never
//! allocates while a re-key is moving IDs.

use std::path::Path;

use ::config::ConfigAccess;
use work::draft_id::mint_draft_ids;
use work::next_number::allocate;
use work::next_number::allocation_key;
use work::next_number::AllocationError;
use work::resolve::DirectoryLister;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles;
use work_adapters::draft_id::RandomSuffixDraws;
use work_adapters::filesystem::FilesystemLister;
use work_adapters::filesystem::FilesystemWorkItemFiles;

use crate::config::announced_scheme_with_key;
use crate::config::resolve_work_dir;

pub enum RunOutcome {
    Allocated(Vec<String>),
    Overflow {
        partial: Vec<String>,
        message: String,
    },
    Failed(String),
    /// The identity run could not begin; nothing was read.
    Refused {
        message: String,
        code: u8,
    },
}

fn mint_drafts(work_dir: &Path, count: u32) -> RunOutcome {
    let files = match FilesystemWorkItemFiles::new(work_dir).files() {
        Ok(files) => files,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    match mint_draft_ids(&mut RandomSuffixDraws, &identities(&files), count) {
        Ok(drafts) => RunOutcome::Allocated(
            drafts
                .into_iter()
                .map(|draft| draft.as_str().to_owned())
                .collect(),
        ),
        Err(error) => RunOutcome::Failed(error.to_string()),
    }
}

/// # Errors
///
/// Never returns `Err`; every failure is reported through [`RunOutcome`].
#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    project: Option<&str>,
    count: u32,
) -> RunOutcome {
    let scheme = match announced_scheme_with_key(config) {
        Ok(scheme) => scheme,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let pattern = scheme.pattern();
    let root = config_adapters::FileConfigStore::discover_root(start);
    let work_dir = match resolve_work_dir(config, &root) {
        Ok(dir) => dir,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let _identity_run = match crate::identity_run::begin(&root, "next-number") {
        Ok(guard) => guard,
        Err(refusal) => {
            let (message, code) = crate::identity_run::refused(refusal, &root);
            return RunOutcome::Refused { message, code };
        }
    };
    let shown_dir = work_dir.strip_prefix(&root).unwrap_or(&work_dir);
    let key = match allocation_key(pattern, project, scheme.key()) {
        Ok(key) => key,
        Err(error) => {
            return RunOutcome::Failed(error.message(pattern, shown_dir))
        }
    };
    if pattern.is_tracker_owned() {
        return mint_drafts(&work_dir, count);
    }

    let filenames = FilesystemLister::new(&work_dir).filenames();
    match allocate(&scheme, key.as_ref(), count, &filenames) {
        Ok(ids) => RunOutcome::Allocated(ids),
        Err(error) => {
            let message = error.message(pattern, shown_dir);
            match error {
                AllocationError::Overflow { partial, .. } => {
                    RunOutcome::Overflow { partial, message }
                }
                _ => RunOutcome::Failed(message),
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::BTreeSet;
    use std::collections::HashMap;

    use ::config::ConfigAccess;
    use ::config::ConfigError;
    use ::config::Key;
    use ::config::Level;
    use ::config::Resolved;
    use ::config::Scalar;
    use ::config::Value;
    use work::draft_id::DraftId;

    use super::run;
    use super::RunOutcome;

    struct TrackerConfig(HashMap<String, String>);

    impl TrackerConfig {
        fn new() -> Self {
            Self(
                [
                    ("work.id_pattern", "{tracker}"),
                    ("work.integration", "jira"),
                ]
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
            )
        }
    }

    impl ConfigAccess for TrackerConfig {
        fn get(
            &self,
            key: &Key,
            _level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            Ok(self
                .0
                .get(&key.to_string())
                .map_or(Resolved::Absent, |value| {
                    Resolved::Found(Value::Scalar(Scalar::String(
                        value.clone(),
                    )))
                }))
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            unreachable!("next-number never writes config")
        }
    }

    fn repo() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join(".jj")).expect("anchor root");
        root
    }

    #[test]
    fn next_number_refuses_while_a_rekey_is_in_progress() {
        let numeric = crate::test_support::FakeConfig(HashMap::new());
        let configs: [&dyn ConfigAccess; 2] = [&numeric, &TrackerConfig::new()];
        for config in configs {
            let root = repo();
            crate::test_support::rekey_in_progress(root.path());

            let RunOutcome::Refused { message, code } =
                run(root.path(), config, None, 1)
            else {
                panic!("next-number allocated during a re-key");
            };

            assert_eq!(code, crate::exit_codes::UNRESOLVED);
            assert!(
                message.starts_with("Error: E_REKEY_IN_PROGRESS"),
                "{message}"
            );
        }
    }

    #[test]
    fn next_number_under_tracker_returns_distinct_draft_ids() {
        let root = repo();

        let RunOutcome::Allocated(ids) =
            run(root.path(), &TrackerConfig::new(), None, 5)
        else {
            panic!("next-number under {{tracker}} mints drafts");
        };

        assert_eq!(ids.len(), 5);
        assert!(ids.iter().all(|id| DraftId::parse(id).is_some()), "{ids:?}");
        assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), 5, "{ids:?}");
    }

    #[test]
    fn next_number_with_project_under_tracker_is_e_pattern_key_unused() {
        let root = repo();

        let RunOutcome::Failed(message) =
            run(root.path(), &TrackerConfig::new(), Some("ENG"), 1)
        else {
            panic!("--project names no token under {{tracker}}");
        };

        assert!(message.starts_with("E_PATTERN_KEY_UNUSED: "), "{message}");
    }
}
