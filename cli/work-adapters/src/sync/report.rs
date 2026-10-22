//! What a finished sync run reports: one line per item and the run-level
//! facts beside them.

use std::path::PathBuf;

use tracker::TrackerError;
use work::inventory::CorpusInventory;
use work::inventory::Fault;
use work::inventory::Unidentified;
use work::inventory::UnidentifiedReason;
use work::sync::Action;
use work::sync::PlannedAction;
use work::sync::SyncState;
use work::sync::Unclassifiable;

use crate::promotion::PromotionRow;
use crate::sync::apply::ApplyError;
use crate::sync::apply::FailureClass;
use crate::sync::apply::LeftoverMarker;
use crate::sync::baseline::Degradation;
use crate::sync::identity_settlement::IdentityRow;
use crate::sync::operation::SyncOperation;
use crate::sync::run::ConflictDossier;
use crate::sync::run::DiscoveryStatus;
use crate::sync::run::RunMode;

pub enum ItemOutcome {
    Applied,
    NotApplied,
    /// Nothing was read or written: the item's baseline entry was recorded
    /// under a digest recipe newer than this binary knows.
    UnsupportedRecipe,
    Failed(ApplyError),
}

pub struct ReportedItem {
    pub planned: PlannedAction,
    pub outcome: ItemOutcome,
    /// The local payload-composition check attached to a `Push` entry during
    /// a preview. `None` outside preview, and for every non-`Push` action.
    ///
    /// A `Rejected` outcome surfaces a locally-detectable missing required
    /// field before any mutation. It does not reproduce a live-tracker field
    /// check — a `Valid` preview does not guarantee the tracker accepts the
    /// push.
    pub validation: Option<tracker::ValidationOutcome>,
    /// Where this run kept the item's local edits before pulling over them.
    pub kept_edits: Option<PathBuf>,
    /// A local create's marker this run linked yet could not remove.
    pub leftover_marker: Option<LeftoverMarker>,
}

/// Whether a failure counts against the run's exit code or is only noted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureSeverity {
    Error,
    Warning,
}

/// One failure a run reports: what it concerned (an item id, a path, or
/// `-` for the run itself), the operation that failed, why, and its class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFailure {
    pub subject: String,
    pub operation: SyncOperation,
    pub cause: String,
    pub class: FailureClass,
    pub severity: FailureSeverity,
}

impl RunFailure {
    fn error(
        subject: &str,
        operation: SyncOperation,
        cause: String,
        class: FailureClass,
    ) -> Self {
        Self {
            subject: subject.to_owned(),
            operation,
            cause,
            class,
            severity: FailureSeverity::Error,
        }
    }
}

/// How much of the corpus a run looked at, which decides what it may judge
/// about the corpus as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusScope {
    WholeCorpus,
    Targeted,
}

/// Whether a run recorded its baseline bookkeeping: a preview does not
/// try, and a failed write keeps the store's reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finalisation {
    NotAttempted,
    Finalised,
    Failed(String),
}

pub struct RunReport {
    pub scope: CorpusScope,
    pub mode: RunMode,
    /// The work files discovery could not use, judged against the corpus
    /// the engine planned over.
    pub inventory: CorpusInventory,
    /// The identity pass's rows, each keyed by the item's id when the run
    /// started. The engine's row for the same item is not rendered.
    pub identity: Vec<IdentityRow>,
    /// Every draft the run set out to promote, in id order.
    pub promotions: Vec<PromotionRow>,
    /// Items that appeared after the identity pass read the remote, left
    /// for the next run.
    pub deferred: usize,
    pub reported: Vec<ReportedItem>,
    /// Items whose classification failed, so they were neither planned nor
    /// applied.
    pub unclassifiable: Vec<Unclassifiable>,
    pub read_failure: Option<TrackerError>,
    pub baseline_degradation: Degradation,
    pub finalisation: Finalisation,
    pub dossiers: Vec<ConflictDossier>,
    pub discovery: DiscoveryStatus,
    /// The bulk keyed read was cut short by a transient condition (a deadline,
    /// rate limit, or wire failure), so the affected items degraded to
    /// indeterminate and the run proceeded. A soft signal, not the fatal
    /// [`RunError::KeyedReadCapped`]: the operator sees the read was
    /// budget-limited rather than the incompleteness passing silently.
    pub keyed_read_budget_limited: bool,
}

impl RunReport {
    /// Every failure this run reports, in report order: failed rows
    /// (a preview's included), unclassifiable items, work files the run
    /// could not use, then a failed baseline finalise. Only a whole-corpus
    /// run may judge the corpus, so on a targeted run an unusable file is
    /// a warning.
    pub fn failures(&self) -> impl Iterator<Item = RunFailure> + '_ {
        let rows =
            self.reported.iter().filter_map(|item| match &item.outcome {
                ItemOutcome::Failed(error) => Some(RunFailure::error(
                    error.item_id(),
                    error.operation(),
                    error.cause(),
                    error.class(),
                )),
                _ => None,
            });
        let unclassifiable = self.unclassifiable.iter().map(|item| {
            RunFailure::error(
                &item.id,
                SyncOperation::ClassifyLocal,
                item.reason.clone(),
                FailureClass::Retryable,
            )
        });
        let files = self
            .inventory
            .unidentified
            .iter()
            .filter_map(|entry| self.unusable_file(entry));
        let finalise = match &self.finalisation {
            Finalisation::Failed(reason) => Some(RunFailure::error(
                "-",
                SyncOperation::BaselineSet,
                reason.clone(),
                FailureClass::Retryable,
            )),
            Finalisation::NotAttempted | Finalisation::Finalised => None,
        };
        rows.chain(unclassifiable).chain(files).chain(finalise)
    }

    fn unusable_file(&self, entry: &Unidentified) -> Option<RunFailure> {
        let fault = entry.reason.fault()?;
        let operation = match entry.reason {
            UnidentifiedReason::Malformed(_) => SyncOperation::IdentifyLocal,
            _ => SyncOperation::ReadLocal,
        };
        Some(RunFailure {
            subject: entry.path.display().to_string(),
            operation,
            cause: entry.reason.to_string(),
            class: match fault {
                Fault::Environment => FailureClass::Retryable,
                Fault::Content => FailureClass::Unparseable,
            },
            severity: match self.scope {
                CorpusScope::WholeCorpus => FailureSeverity::Error,
                CorpusScope::Targeted => FailureSeverity::Warning,
            },
        })
    }

    /// Items this run left for a human. Derived, never stored: a stored
    /// field could disagree with the report, yielding a run that prints
    /// conflicts and exits 0.
    pub fn awaiting_human(&self) -> impl Iterator<Item = &ReportedItem> {
        self.reported.iter().filter(|item| {
            matches!(
                item.planned.action,
                Action::Prompt | Action::SkipConflict | Action::SkipDirty
            ) || matches!(
                item.planned.state,
                SyncState::RemoteAbsent | SyncState::Indeterminate
            )
        })
    }
}
