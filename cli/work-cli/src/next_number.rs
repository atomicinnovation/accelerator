//! Adapter/binary wiring for `work next-number`: a thin display wrapper
//! around `work::next_number::allocate`, or `work::draft_id::mint_draft_ids`
//! under `{tracker}` — never writes a file, never commits a number.

use std::path::Path;

use ::config::ConfigAccess;
use corpus::IdOwnership;
use corpus::WorkItemIdScheme;
use corpus_adapters::compile_scan_regex;
use corpus_adapters::RegexScanner;
use work::draft_id::mint_draft_ids;
use work::next_number::allocate;
use work::next_number::AllocationError;
use work::resolve::DirectoryLister;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles;
use work_adapters::draft_id::RandomSuffixDraws;
use work_adapters::filesystem::FilesystemLister;
use work_adapters::filesystem::FilesystemWorkItemFiles;

use crate::config::resolve_scheme;
use crate::config::resolve_work_dir;

pub enum RunOutcome {
    Allocated(Vec<String>),
    Overflow {
        partial: Vec<String>,
        message: String,
    },
    Failed(String),
}

fn overflow_message(
    highest: u64,
    highest_file: Option<&str>,
    cap: u64,
    pattern: &str,
) -> String {
    if highest > cap {
        format!(
            "E_PATTERN_OVERFLOW: out-of-width file '{}' has number \
             {highest} exceeding the pattern '{pattern}' cap of {cap}. \
             Rename the stray file or widen the pattern.",
            highest_file.unwrap_or("<unknown>")
        )
    } else {
        format!(
            "E_PATTERN_OVERFLOW: pattern '{pattern}' number space \
             exhausted (highest={highest}, cap={cap}). Archive completed \
             work items or widen the pattern."
        )
    }
}

fn allocation_message(error: &AllocationError, pattern: &str) -> String {
    match error {
        AllocationError::MissingKey => format!(
            "E_PATTERN_MISSING_KEY: pattern '{pattern}' references the \
             {{key}} prefix but no value supplied — pass --project or set \
             work.key"
        ),
        AllocationError::ProjectUnused => format!(
            "E_PATTERN_KEY_UNUSED: --project is meaningless for pattern \
             '{pattern}' (no {{key}} token)"
        ),
        AllocationError::Overflow { .. } => {
            unreachable!("Overflow is handled by the caller separately")
        }
    }
}

fn mint_drafts(
    scheme: &WorkItemIdScheme,
    work_dir: &Path,
    project: Option<&str>,
    count: u32,
) -> RunOutcome {
    if project.is_some() {
        return RunOutcome::Failed(allocation_message(
            &AllocationError::ProjectUnused,
            &scheme.id_pattern,
        ));
    }
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
    let scheme = match resolve_scheme(config) {
        Ok(scheme) => scheme,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let root = config_adapters::FileConfigStore::discover_root(start);
    let work_dir = match resolve_work_dir(config, &root) {
        Ok(dir) => dir,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let project = project.map(str::to_owned).or_else(|| scheme.key.clone());
    if scheme.ownership() == IdOwnership::Tracker {
        return mint_drafts(&scheme, &work_dir, project.as_deref(), count);
    }

    let filenames = FilesystemLister::new(&work_dir).filenames();
    let scan_regex = match compile_scan_regex(
        &scheme.id_pattern,
        project.as_deref().unwrap_or(""),
    ) {
        Ok(regex) => regex,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let scanner = match RegexScanner::compile(&scan_regex) {
        Ok(scanner) => scanner,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };

    match allocate(&scheme, project.as_deref(), count, &filenames, &scanner) {
        Ok(ids) => RunOutcome::Allocated(ids),
        Err(AllocationError::Overflow {
            partial,
            highest,
            highest_file,
            cap,
        }) => RunOutcome::Overflow {
            partial,
            message: overflow_message(
                highest,
                highest_file.as_deref(),
                cap,
                &scheme.id_pattern,
            ),
        },
        Err(other) => {
            RunOutcome::Failed(allocation_message(&other, &scheme.id_pattern))
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
