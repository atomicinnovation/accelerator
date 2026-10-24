//! Where each reported item stands with its conflict once the run is over,
//! which decides whose dossier the run refreshes, flags stale or retires.

use std::collections::BTreeSet;

use work::sync::Action;
use work::sync::PlannedAction;
use work::sync::SyncState;

use crate::sync::apply::ItemProgress;
use crate::sync::report::CorpusScope;
use crate::sync::report::ItemOutcome;
use crate::sync::report::ReportedItem;
use crate::sync::report::RunReport;
use crate::sync::run::RunMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStatus {
    NotInConflict,
    AwaitingResolution,
    ResolutionPending,
    ResolutionFailed,
    Settled,
    /// The run could not tell whether the item is still in conflict, so
    /// its dossier is never retired.
    Unobserved,
}

enum ConflictShape {
    NotInConflict,
    Unobserved,
    AwaitingResolution,
    Resolution,
    Recorded,
}

impl ConflictShape {
    /// Every combination the planner cannot produce is `Unobserved`, so a
    /// surprise never retires a dossier.
    const fn of(planned: &PlannedAction) -> Self {
        if planned.resolution().is_some() {
            Self::of_resolved(planned.state(), planned.action())
        } else {
            Self::of_unresolved(planned.state(), planned.action())
        }
    }

    const fn of_resolved(state: SyncState, action: Action) -> Self {
        use Action as A;
        use SyncState as S;
        match (state, action) {
            (S::RemotelyModified | S::Conflict, A::Pull | A::Overwrite) => {
                Self::Resolution
            }
            (
                S::Synced
                | S::Unsynced
                | S::LocallyModified
                | S::RemoteAbsent
                | S::Indeterminate,
                A::Pull | A::Overwrite,
            )
            | (
                _,
                A::Push(_)
                | A::Retitle
                | A::SkipConflict
                | A::SkipDirty
                | A::Prompt
                | A::Noop
                | A::RecordBaseline
                | A::CreateFromRemote
                | A::CreateFromLocal
                | A::Relink,
            ) => Self::Unobserved,
        }
    }

    const fn of_unresolved(state: SyncState, action: Action) -> Self {
        use Action as A;
        use SyncState as S;
        match (state, action) {
            (S::RemotelyModified | S::Conflict, A::Prompt) => {
                Self::AwaitingResolution
            }
            (_, A::Relink) => Self::Resolution,
            (S::Synced | S::LocallyModified, A::RecordBaseline) => {
                Self::Recorded
            }
            (S::Synced | S::Unsynced, A::Noop)
            | (S::RemotelyModified, A::Pull)
            | (S::LocallyModified, A::Push(_) | A::Retitle)
            | (S::Unsynced, A::CreateFromRemote | A::CreateFromLocal) => {
                Self::NotInConflict
            }
            (S::RemoteAbsent | S::Indeterminate, _)
            | (
                S::LocallyModified | S::RemotelyModified | S::Conflict,
                A::Noop,
            )
            | (
                S::Synced | S::Unsynced | S::LocallyModified | S::Conflict,
                A::Pull | A::SkipDirty,
            )
            | (S::Synced | S::Unsynced | S::LocallyModified, A::Prompt)
            | (
                S::Synced
                | S::Unsynced
                | S::LocallyModified
                | S::RemotelyModified
                | S::Conflict,
                A::SkipConflict | A::Overwrite,
            )
            | (S::RemotelyModified, A::SkipDirty)
            | (
                S::Unsynced | S::RemotelyModified | S::Conflict,
                A::RecordBaseline,
            )
            | (
                S::Synced | S::Unsynced | S::RemotelyModified | S::Conflict,
                A::Push(_) | A::Retitle,
            )
            | (
                S::Synced
                | S::LocallyModified
                | S::RemotelyModified
                | S::Conflict,
                A::CreateFromRemote | A::CreateFromLocal,
            ) => Self::Unobserved,
        }
    }
}

impl ConflictStatus {
    #[must_use]
    pub const fn of(item: &ReportedItem) -> Self {
        if matches!(item.outcome, ItemOutcome::UnsupportedRecipe) {
            return Self::Unobserved;
        }
        match ConflictShape::of(&item.planned) {
            ConflictShape::NotInConflict => Self::NotInConflict,
            ConflictShape::Unobserved => Self::Unobserved,
            ConflictShape::AwaitingResolution => Self::AwaitingResolution,
            ConflictShape::Resolution => resolution_status(&item.outcome),
            ConflictShape::Recorded => recorded_status(&item.outcome),
        }
    }

    const fn is_settled(self) -> bool {
        matches!(self, Self::Settled | Self::NotInConflict)
    }
}

const fn resolution_status(outcome: &ItemOutcome) -> ConflictStatus {
    match outcome {
        ItemOutcome::Applied => ConflictStatus::Settled,
        ItemOutcome::NotApplied => ConflictStatus::ResolutionPending,
        ItemOutcome::Failed(_) => ConflictStatus::ResolutionFailed,
        ItemOutcome::UnsupportedRecipe => ConflictStatus::Unobserved,
    }
}

const fn recorded_status(outcome: &ItemOutcome) -> ConflictStatus {
    match outcome {
        ItemOutcome::Applied => ConflictStatus::NotInConflict,
        ItemOutcome::NotApplied
        | ItemOutcome::Failed(_)
        | ItemOutcome::UnsupportedRecipe => ConflictStatus::Unobserved,
    }
}

const fn failure_progress(item: &ReportedItem) -> Option<ItemProgress> {
    match &item.outcome {
        ItemOutcome::Failed(error) => Some(error.progress()),
        ItemOutcome::Applied
        | ItemOutcome::NotApplied
        | ItemOutcome::UnsupportedRecipe => None,
    }
}

/// The items whose dossier this run writes afresh: those still awaiting a
/// resolution, and those whose resolution has not landed yet.
#[must_use]
pub fn dossiers_to_refresh(items: &[ReportedItem]) -> BTreeSet<&str> {
    items
        .iter()
        .filter(|item| match ConflictStatus::of(item) {
            ConflictStatus::AwaitingResolution
            | ConflictStatus::ResolutionPending => true,
            ConflictStatus::ResolutionFailed => {
                failure_progress(item) == Some(ItemProgress::NothingLanded)
            }
            ConflictStatus::NotInConflict
            | ConflictStatus::Settled
            | ConflictStatus::Unobserved => false,
        })
        .map(|item| item.planned.id())
        .collect()
}

/// The items whose resolution failed part-way, so their prior dossier
/// describes a state the item has left and is kept only as a stale record.
#[must_use]
pub fn dossiers_gone_stale(items: &[ReportedItem]) -> BTreeSet<&str> {
    items
        .iter()
        .filter(|item| {
            ConflictStatus::of(item) == ConflictStatus::ResolutionFailed
                && failure_progress(item) == Some(ItemProgress::MayHaveLanded)
        })
        .map(|item| item.planned.id())
        .collect()
}

/// Which dossiers a finished run may retire, judged from what the run
/// looked at and whether it applied anything.
pub enum DossierAuthority<'a> {
    Preview,
    Targeted { report: &'a RunReport },
    WholeCorpus { report: &'a RunReport },
}

impl<'a> DossierAuthority<'a> {
    #[must_use]
    pub const fn for_run(report: &'a RunReport) -> Self {
        match (report.mode, report.scope) {
            (RunMode::Preview, _) => Self::Preview,
            (RunMode::Apply, CorpusScope::Targeted) => {
                Self::Targeted { report }
            }
            (RunMode::Apply, CorpusScope::WholeCorpus) => {
                Self::WholeCorpus { report }
            }
        }
    }

    /// A whole-corpus run also retires the dossier of an id no work item
    /// carries, but only when every work file was identified: an unusable
    /// file may be that item.
    #[must_use]
    pub fn may_retire(&self, id: &str) -> bool {
        match self {
            Self::Preview => false,
            Self::Targeted { report } => settled_by(report, id),
            Self::WholeCorpus { report } => {
                settled_by(report, id)
                    || (report.inventory.proves_absence()
                        && !report.inventory.ids.contains(id))
            }
        }
    }

    /// Whether the run may remove interrupted dossier writes.
    #[must_use]
    pub const fn retires_artefacts(&self) -> bool {
        !matches!(self, Self::Preview)
    }
}

fn settled_by(report: &RunReport, id: &str) -> bool {
    let mut rows = report
        .reported
        .iter()
        .filter(|item| item.planned.id() == id)
        .peekable();
    rows.peek().is_some()
        && rows.all(|item| ConflictStatus::of(item).is_settled())
}
