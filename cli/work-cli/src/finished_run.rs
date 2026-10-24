//! Everything known once a sync run has finished, carried once to both the
//! report and the finaliser.

use std::path::Path;
use std::path::PathBuf;

use tracker::ExternalId;
use work_adapters::sync::kept_edits::KeptEditsReview;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::RunReport;

pub struct FinishedRun<'a> {
    pub report: &'a RunReport,
    /// The `external_id` of every tracked work item in the corpus.
    pub corpus_external_ids: Vec<ExternalId>,
    pub integrations_root: &'a Path,
    pub repo_root: &'a Path,
    /// The kept-edits review on a preview, or what the sweep left on an
    /// apply run.
    pub kept_edits: KeptEditsReview,
    pub stale_dossiers: Vec<StaleDossier>,
}

/// A conflict dossier left in place that no longer describes its item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleDossier {
    pub id: String,
    pub path: PathBuf,
    pub cause: StaleDossierCause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleDossierCause {
    PartialResolution,
    NotRefreshed,
}

impl FinishedRun<'_> {
    /// The remote issues this run imported as new work items. A
    /// create-from-remote item is planned under its external id, having no
    /// local id until it is authored.
    #[must_use]
    pub fn applied_imports(&self) -> Vec<ExternalId> {
        self.report
            .reported
            .iter()
            .filter(|item| {
                matches!(
                    item.planned.action(),
                    work::sync::Action::CreateFromRemote
                ) && matches!(item.outcome, ItemOutcome::Applied)
            })
            .map(|item| ExternalId::new(item.planned.id().to_owned()))
            .collect()
    }
}

#[cfg(test)]
impl<'a> FinishedRun<'a> {
    /// `report` alone, as an apply run with nothing else to say.
    pub fn for_report(report: &'a RunReport) -> Self {
        Self {
            report,
            corpus_external_ids: Vec::new(),
            integrations_root: Path::new(""),
            repo_root: Path::new(""),
            kept_edits: KeptEditsReview {
                entries: Vec::new(),
            },
            stale_dossiers: Vec::new(),
        }
    }
}
