//! Which of the work directory's files a sync can use: the configured id
//! scheme names work items from their filenames, and the version control
//! system names the files it holds conflicted.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use corpus::IdConvention as _;
use tracker::ExternalId;
use vcs::status::ChangeType;
use vcs::RepositoryProbe;
use vcs::VcsKind;
use vcs::VcsReporter;
use work::draft_id::DraftId;
use work::id_scheme::IdScheme;
use work::inventory::identify;
use work::inventory::ConflictedPaths;
use work::inventory::CorpusInventory;
use work::inventory::NamesWorkItem;
use work::work_item_files::WorkItemFiles;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::sync::apply::FailureClass;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::operation::SyncOperation;
use work_adapters::sync::report::FailureSeverity;
use work_adapters::sync::report::RunFailure;

/// Names work items from filenames as the configured id scheme lays them
/// out, drafts included.
pub struct ConfiguredNames {
    scheme: IdScheme,
}

impl ConfiguredNames {
    #[must_use]
    pub fn for_scheme(scheme: &IdScheme) -> Self {
        Self {
            scheme: scheme.clone(),
        }
    }
}

fn leading_parts(file_name: &str, count: usize) -> String {
    file_name
        .trim_end_matches(".md")
        .splitn(count + 1, '-')
        .take(count)
        .collect::<Vec<_>>()
        .join("-")
}

impl NamesWorkItem for ConfiguredNames {
    fn work_item_named(&self, file_name: &str) -> Option<String> {
        let two_parts = leading_parts(file_name, 2);
        if DraftId::parse(&two_parts).is_some() {
            return Some(two_parts);
        }
        self.scheme.id_in_filename(file_name)
    }
}

/// The paths the version control system reported conflicted when the run
/// began; empty outside a repository.
pub struct VcsConflicts {
    root: PathBuf,
    conflicted: BTreeSet<String>,
}

impl VcsConflicts {
    /// Reads the conflict status of the repository holding `start` once.
    /// The warning, when there is one, says the status could not be read and
    /// conflicts are found from markers in file text alone.
    pub fn read(
        start: &Path,
        repository: &dyn RepositoryProbe,
        reporter: &dyn VcsReporter,
    ) -> (Self, Option<RunFailure>) {
        let unknown = |root: PathBuf| Self {
            root,
            conflicted: BTreeSet::new(),
        };
        let Some(facts) = repository
            .facts_at(start)
            .filter(|facts| facts.kind != VcsKind::None)
        else {
            return (unknown(start.to_path_buf()), None);
        };
        match reporter.status_report(&facts.root, facts.kind) {
            Ok(report) => (
                Self {
                    root: facts.root,
                    conflicted: report
                        .changes
                        .into_iter()
                        .filter(|change| {
                            change.change_type == ChangeType::Conflicted
                        })
                        .map(|change| change.path)
                        .collect(),
                },
                None,
            ),
            Err(_) => (
                unknown(facts.root),
                Some(RunFailure {
                    subject: "-".to_owned(),
                    operation: SyncOperation::IdentifyLocal,
                    cause: format!(
                        "could not read conflict status from {}; relying on \
                         conflict markers in file text",
                        vcs_name(facts.kind)
                    ),
                    class: FailureClass::Retryable,
                    severity: FailureSeverity::Warning,
                }),
            ),
        }
    }
}

const fn vcs_name(kind: VcsKind) -> &'static str {
    match kind {
        VcsKind::Jj => "jj",
        VcsKind::Git | VcsKind::None => "git",
    }
}

impl ConflictedPaths for VcsConflicts {
    fn is_conflicted(&self, path: &Path) -> bool {
        let canonical = path.canonicalize();
        let path = canonical.as_deref().unwrap_or(path);
        path.strip_prefix(&self.root).is_ok_and(|relative| {
            self.conflicted.contains(
                &relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/"),
            )
        })
    }
}

/// The work directory's items, sorted by id, and the inventory of every
/// file that is not one.
///
/// # Errors
///
/// When the work directory cannot be listed.
pub fn discover_corpus(
    work_dir: &Path,
    names: &dyn NamesWorkItem,
    conflicted: &dyn ConflictedPaths,
) -> Result<(Vec<LocalItem>, CorpusInventory), kernel::Error> {
    let listing = FilesystemWorkItemFiles::new(work_dir).listing()?;
    let identified = identify(&listing, names, conflicted);
    let mut items: Vec<LocalItem> = identified
        .items
        .into_iter()
        .map(|(_, identity)| LocalItem {
            id: identity.id,
            path: identity.path,
            external_id: identity.external_id.map(ExternalId::new),
        })
        .collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((items, identified.inventory))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::literal_string_with_formatting_args
)]
mod tests {
    use std::path::Path;

    use vcs::log::LogReport;
    use vcs::status::StatusReport;
    use vcs::VcsKind;
    use vcs::VcsReporter;
    use vcs_adapters::library::InProcessProbe;
    use work::id_pattern::IdKey;
    use work::id_pattern::IdPattern;
    use work::id_scheme::IdScheme;
    use work::inventory::ConflictedPaths;
    use work::inventory::NamesWorkItem;

    use super::ConfiguredNames;
    use super::VcsConflicts;

    fn names(id_pattern: &str, key: Option<&str>) -> ConfiguredNames {
        ConfiguredNames::for_scheme(&IdScheme::new(
            IdPattern::parse(id_pattern).expect("the pattern parses"),
            key.map(|key| IdKey::parse(key).expect("the key parses")),
        ))
    }

    #[test]
    fn the_configured_names_recognise_each_work_item_filename() {
        let numbered = names("{number:04d}", None);
        let keyed = names("{key}-{number:04d}", Some("PP"));
        let tracker = names("{tracker}", None);

        assert_eq!(
            numbered.work_item_named("0007-title.md").as_deref(),
            Some("0007")
        );
        assert_eq!(
            keyed.work_item_named("PP-0007-title.md").as_deref(),
            Some("PP-0007")
        );
        assert_eq!(
            tracker.work_item_named("ENG-42-add-search.md").as_deref(),
            Some("ENG-42")
        );
        for scheme in [&numbered, &keyed, &tracker] {
            assert_eq!(
                scheme.work_item_named("draft-k7mq3x-title.md").as_deref(),
                Some("draft-k7mq3x")
            );
            assert_eq!(scheme.work_item_named("README.md"), None);
        }
    }

    struct FailingReporter;

    impl VcsReporter for FailingReporter {
        fn status_report(
            &self,
            _root: &Path,
            _kind: VcsKind,
        ) -> Result<StatusReport, kernel::Error> {
            Err(kernel::Error::Failed("unreadable".to_owned()))
        }

        fn log_report(
            &self,
            _root: &Path,
            _kind: VcsKind,
        ) -> Result<LogReport, kernel::Error> {
            Err(kernel::Error::Failed("unreadable".to_owned()))
        }
    }

    #[test]
    fn conflict_detection_without_a_repository_uses_the_text_scan() {
        let dir = tempfile::tempdir().expect("a temporary directory");

        let (conflicts, warning) =
            VcsConflicts::read(dir.path(), &InProcessProbe, &FailingReporter);

        assert!(warning.is_none());
        assert!(!conflicts.is_conflicted(&dir.path().join("0001-x.md")));
    }

    #[test]
    fn an_unreadable_vcs_status_warns_and_uses_the_text_scan() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let status = std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(dir.path())
            .status()
            .expect("git runs");
        assert!(status.success());

        let (conflicts, warning) =
            VcsConflicts::read(dir.path(), &InProcessProbe, &FailingReporter);

        assert_eq!(
            warning
                .map(|warning| crate::report_records::failure_line(
                    &warning, "failed"
                ))
                .as_deref(),
            Some(
                "warning: -: identify-local failed: could not read conflict \
                 status from git; relying on conflict markers in file text"
            )
        );
        assert!(!conflicts.is_conflicted(&dir.path().join("0001-x.md")));
    }
}
