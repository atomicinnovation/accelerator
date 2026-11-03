//! The pure sync planner: classify then decide over pre-gathered facts, with
//! any `--resolve` orders applied afterwards.
//!
//! Planning makes no remote call. Local reads still happen, lazily, through
//! the injected [`ItemDigests`] port whenever `classify` asks for one.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use tracker::ExternalId;
use tracker::RemoteTimestamp;

use crate::sync::classify;
use crate::sync::classify::BaselineEntry;
use crate::sync::classify::ItemComparison;
use crate::sync::classify::ItemDigests;
use crate::sync::classify::RemotePresence;
use crate::sync::classify::Subject;
use crate::sync::decide;
use crate::sync::decide::Action;
use crate::sync::decide::Dirtiness;
use crate::sync::decide::Resolution;
use crate::sync::decide::SyncDirection;
use crate::sync::push_precondition::PendingPushOrder;
use crate::sync::state::SyncState;

pub struct RemoteFacts<'a> {
    pub presence: RemotePresence,
    pub remote_updated: &'a RemoteTimestamp,
}

/// What a person told the run to do with one item, beyond what planning
/// decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Order(PendingPushOrder),
    Resolution(AppliedResolution),
}

/// A `--resolve` that rewrote a prompt into an action; a skip leaves the
/// prompt in place, so it is never one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppliedResolution {
    AcceptRemote,
    PushLocal,
}

impl AppliedResolution {
    const fn of(resolution: Resolution) -> Option<Self> {
        match resolution {
            Resolution::AcceptRemote => Some(Self::AcceptRemote),
            Resolution::PushLocal => Some(Self::PushLocal),
            Resolution::Skip => None,
        }
    }

    const fn action(self) -> Action {
        match self {
            Self::AcceptRemote => Action::Pull,
            Self::PushLocal => Action::Overwrite,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedAction {
    id: String,
    state: SyncState,
    action: Action,
    instruction: Option<Instruction>,
}

impl PlannedAction {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        state: SyncState,
        action: Action,
    ) -> Self {
        Self {
            id: id.into(),
            state,
            action,
            instruction: None,
        }
    }

    /// An unsynced item a pending-push order settles, so its create knows
    /// whether to adopt an issue or make a new one.
    pub(crate) fn ordered(
        id: impl Into<String>,
        state: SyncState,
        order: PendingPushOrder,
    ) -> Self {
        Self {
            instruction: Some(Instruction::Order(order)),
            ..Self::new(id, state, Action::Noop)
        }
    }

    /// An item whose link is unconfirmed, re-linked by a pending-push order
    /// whatever its classified state.
    pub(crate) fn relink(
        id: impl Into<String>,
        state: SyncState,
        order: PendingPushOrder,
    ) -> Self {
        Self {
            instruction: Some(Instruction::Order(order)),
            ..Self::new(id, state, Action::Relink)
        }
    }

    pub(crate) fn resolved(
        id: impl Into<String>,
        state: SyncState,
        resolution: AppliedResolution,
    ) -> Self {
        Self {
            instruction: Some(Instruction::Resolution(resolution)),
            ..Self::new(id, state, resolution.action())
        }
    }

    /// This action carrying `planned`'s pending-push order, when `planned`
    /// is for the same item; only the planner can give an action an order.
    #[must_use]
    pub fn with_order_of(self, planned: &Self) -> Self {
        match &planned.instruction {
            Some(Instruction::Order(order)) if planned.id == self.id => Self {
                instruction: Some(Instruction::Order(order.clone())),
                ..self
            },
            _ => self,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub const fn state(&self) -> SyncState {
        self.state
    }

    #[must_use]
    pub const fn action(&self) -> Action {
        self.action
    }

    #[must_use]
    pub const fn order(&self) -> Option<&PendingPushOrder> {
        match &self.instruction {
            Some(Instruction::Order(order)) => Some(order),
            Some(Instruction::Resolution(_)) | None => None,
        }
    }

    #[must_use]
    pub const fn resolution(&self) -> Option<AppliedResolution> {
        match &self.instruction {
            Some(Instruction::Resolution(resolution)) => Some(*resolution),
            Some(Instruction::Order(_)) | None => None,
        }
    }
}

/// An item whose classification failed, so it was left out of the plan
/// while every other item was planned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unclassifiable {
    pub id: String,
    pub reason: String,
}

pub struct SyncPlan {
    pub actions: Vec<PlannedAction>,
    pub unclassifiable: Vec<Unclassifiable>,
}

impl SyncPlan {
    #[must_use]
    pub fn pull_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|planned| planned.action() == Action::Pull)
            .count()
    }

    #[must_use]
    pub fn push_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|planned| planned.action().sends_to_tracker())
            .count()
    }
}

/// What a person ordered beyond planning, by item id, and the items whose
/// link is unconfirmed and so may still be replaced.
pub struct PlanOrders<'a> {
    pub resolutions: &'a BTreeMap<String, Resolution>,
    pub pending_push: &'a BTreeMap<String, PendingPushOrder>,
    pub unconfirmed_links: &'a BTreeSet<String>,
}

static NO_PENDING_PUSH_ORDERS: BTreeMap<String, PendingPushOrder> =
    BTreeMap::new();
static NO_UNCONFIRMED_LINKS: BTreeSet<String> = BTreeSet::new();

impl<'a> PlanOrders<'a> {
    #[must_use]
    pub const fn resolving(
        resolutions: &'a BTreeMap<String, Resolution>,
    ) -> Self {
        Self {
            resolutions,
            pending_push: &NO_PENDING_PUSH_ORDERS,
            unconfirmed_links: &NO_UNCONFIRMED_LINKS,
        }
    }
}

/// One item's classification inputs, carrying its own [`ItemDigests`] and
/// [`ItemComparison`] so `plan` needs no separate lookup port.
pub struct PlanInput<'a> {
    pub id: String,
    pub external_id: Option<&'a ExternalId>,
    pub facts: RemoteFacts<'a>,
    pub dirty: Dirtiness,
    pub baseline: BaselineEntry<'a>,
    pub baseline_timestamp: u64,
    pub digests: &'a dyn ItemDigests,
    pub comparison: &'a dyn ItemComparison,
}

/// The two-tier read rule as a pure function: the ids whose stamp does not
/// prove them unchanged against their baseline entry, and which therefore
/// need a `show` before the plan can be completed.
#[must_use]
pub fn needs_body_read(items: &[PlanInput<'_>]) -> Vec<ExternalId> {
    items
        .iter()
        .filter_map(|item| {
            let external_id = item.external_id?;
            if !matches!(item.facts.presence, RemotePresence::Present) {
                return None;
            }
            let unchanged = item
                .facts
                .remote_updated
                .proves_unchanged_since(item.baseline.remote_updated_at);
            if unchanged {
                None
            } else {
                Some(external_id.clone())
            }
        })
        .collect()
}

/// Computes a complete plan: one [`PlannedAction`] per classifiable item,
/// and one [`Unclassifiable`] per item whose classification failed.
///
/// A resolution naming an id that did not decide `Prompt` is inert; the map
/// is read, never validated, so a stale or misdirected order finds nothing
/// to apply to. A pending-push order is recorded on the unsynced item it
/// names, or plans a relink of an unconfirmed link; one naming any other
/// item is left for the run to report.
#[must_use]
pub fn plan(
    items: &[PlanInput<'_>],
    direction: SyncDirection,
    orders: &PlanOrders<'_>,
) -> SyncPlan {
    let mut actions = Vec::with_capacity(items.len());
    let mut unclassifiable = Vec::new();
    for item in items {
        let subject = Subject {
            external_id: item.external_id,
            presence: item.facts.presence,
            remote_updated: item.facts.remote_updated,
            baseline_timestamp: item.baseline_timestamp,
        };
        let classification = match classify::classify(
            &subject,
            &item.baseline,
            item.digests,
            item.comparison,
        ) {
            Ok(classification) => classification,
            Err(error) => {
                unclassifiable.push(Unclassifiable {
                    id: item.id.clone(),
                    reason: error.to_string(),
                });
                continue;
            }
        };
        let action = decide::decide(direction, &classification, item.dirty);
        let resolution = orders
            .resolutions
            .get(&item.id)
            .filter(|_| action == Action::Prompt)
            .and_then(|resolution| AppliedResolution::of(*resolution));
        let state = classification.state();
        actions.push(match (orders.pending_push.get(&item.id), resolution) {
            (Some(order), _) if orders.unconfirmed_links.contains(&item.id) => {
                PlannedAction::relink(item.id.clone(), state, order.clone())
            }
            (Some(order), _) if state == SyncState::Unsynced => {
                PlannedAction::ordered(item.id.clone(), state, order.clone())
            }
            (_, Some(resolution)) => {
                PlannedAction::resolved(item.id.clone(), state, resolution)
            }
            (_, None) => PlannedAction::new(item.id.clone(), state, action),
        });
    }
    SyncPlan {
        actions,
        unclassifiable,
    }
}

#[cfg(test)]
mod tests {
    use super::PlannedAction;
    use crate::sync::decide::Action;
    use crate::sync::push_precondition::PendingPushOrder;
    use crate::sync::state::SyncState;

    #[test]
    fn with_order_of_copies_only_an_order_for_the_same_id() {
        let ordered = PlannedAction::ordered(
            "0001",
            SyncState::Unsynced,
            PendingPushOrder::Create,
        );
        let row =
            || PlannedAction::new("0001", SyncState::Unsynced, Action::Noop);

        assert_eq!(
            row().with_order_of(&ordered).order(),
            Some(&PendingPushOrder::Create)
        );
        assert_eq!(
            PlannedAction::new("0002", SyncState::Unsynced, Action::Noop)
                .with_order_of(&ordered)
                .order(),
            None
        );
        assert_eq!(row().with_order_of(&row()).order(), None);
    }
}
