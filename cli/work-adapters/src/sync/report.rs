//! What a finished sync run reports: one line per item and the run-level
//! facts beside them.

use std::path::PathBuf;

use tracker::TrackerError;
use work::sync::Action;
use work::sync::PlannedAction;
use work::sync::SyncState;
use work::sync::Unclassifiable;

use crate::promotion::PromotionRow;
use crate::sync::apply::ApplyError;
use crate::sync::apply::LeftoverMarker;
use crate::sync::baseline::Degradation;
use crate::sync::identity_settlement::IdentityRow;
use crate::sync::run::ConflictDossier;
use crate::sync::run::DiscoveryStatus;

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

pub struct RunReport {
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
    pub finalised: bool,
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
