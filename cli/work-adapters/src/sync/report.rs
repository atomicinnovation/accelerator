//! What a finished sync run reports: one line per item and the run-level
//! facts beside them.

use std::path::PathBuf;

use tracker::ExternalId;
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
    /// What a person should know about a local create that applied.
    pub notice: Option<CreateNotice>,
}

/// Something a local create settled by an order did that a person should
/// know, though it applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateNotice {
    /// The adopted issue's title or body differ from the item's, so the
    /// link stays unrecorded and the next sync reports a conflict.
    DifferingAdoption(ExternalId),
    /// A create was ordered with no pending push to settle.
    NothingToSettle,
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

/// Why untracked discovery imported nothing it would otherwise have
/// imported.
#[derive(Debug)]
pub enum DiscoveryDeferral {
    CorpusIncomplete(ApplyError),
    PendingPush(PendingPushDeferral),
}

/// Why an outstanding pending push stopped untracked import: an earlier
/// create may have made an issue no key yet names, so importing it would
/// author a second file for the same issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingPushDeferral {
    /// Items whose create attempt has an unknown outcome and no order.
    Unsettled(Vec<String>),
    /// A marker records a created issue whose whereabouts the tracker did
    /// not answer.
    Unchecked {
        id: String,
        key: ExternalId,
        cause: String,
    },
    /// The pending pushes could not be listed.
    Unlisted(String),
}

impl std::fmt::Display for PendingPushDeferral {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsettled(ids) => write!(
                formatter,
                "a pending push is unresolved; settle {} with --resolve \
                 (without --pull-only), then sync",
                ids.join(", ")
            ),
            Self::Unchecked { id, key, cause } => write!(
                formatter,
                "could not check pending push {id}'s issue {key}: {cause}; \
                 sync again"
            ),
            Self::Unlisted(cause) => write!(
                formatter,
                "could not list the pending pushes: {cause}; sync again"
            ),
        }
    }
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
    /// Pending-push orders that reached no create, each refused.
    pub refused_orders: Vec<ApplyError>,
    pub read_failure: Option<TrackerError>,
    pub baseline_degradation: Degradation,
    pub finalisation: Finalisation,
    pub dossiers: Vec<ConflictDossier>,
    pub discovery: DiscoveryStatus,
    pub discovery_deferral: Option<DiscoveryDeferral>,
    /// The bulk keyed read was cut short by a transient condition (a deadline,
    /// rate limit, or wire failure), so the affected items degraded to
    /// indeterminate and the run proceeded. A soft signal, not the fatal
    /// [`RunError::KeyedReadCapped`]: the operator sees the read was
    /// budget-limited rather than the incompleteness passing silently.
    pub keyed_read_budget_limited: bool,
}

impl RunReport {
    /// Every failure this run reports, in report order: failed rows
    /// (a preview's included), refused orders, unclassifiable items, work files the run
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
        let refused_orders = self.refused_orders.iter().map(|error| {
            RunFailure::error(
                error.item_id(),
                error.operation(),
                error.cause(),
                error.class(),
            )
        });
        let unclassifiable = self.unclassifiable.iter().map(|item| {
            RunFailure::error(
                &item.id,
                SyncOperation::ClassifyLocal,
                item.reason.clone(),
                FailureClass::Retryable,
            )
        });
        let deferral =
            self.discovery_deferral
                .iter()
                .map(|deferral| match deferral {
                    DiscoveryDeferral::CorpusIncomplete(error) => {
                        RunFailure::error(
                            error.item_id(),
                            error.operation(),
                            error.cause(),
                            error.class(),
                        )
                    }
                    DiscoveryDeferral::PendingPush(deferral) => {
                        RunFailure::error(
                            "-",
                            SyncOperation::Discover,
                            deferral.to_string(),
                            FailureClass::Retryable,
                        )
                    }
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
        rows.chain(refused_orders)
            .chain(unclassifiable)
            .chain(files)
            .chain(deferral)
            .chain(finalise)
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
                item.planned.action(),
                Action::Prompt | Action::SkipConflict | Action::SkipDirty
            ) || matches!(
                item.planned.state(),
                SyncState::RemoteAbsent | SyncState::Indeterminate
            )
        })
    }
}
