//! The pure sync planner: classify then decide over pre-gathered facts, with
//! any `--resolve` orders applied afterwards.
//!
//! "Fetching stays outside planning" refers to `fetch_all`/`show` only —
//! local reads still happen, lazily, through the injected [`ItemDigests`]
//! port whenever `classify` asks for one.

use std::collections::BTreeMap;

use tracker::ExternalId;
use tracker::RemoteTimestamp;

use crate::sync::classify;
use crate::sync::classify::BaselineEntry;
use crate::sync::classify::ItemDigests;
use crate::sync::classify::RemotePresence;
use crate::sync::classify::Subject;
use crate::sync::decide;
use crate::sync::decide::Action;
use crate::sync::decide::Dirtiness;
use crate::sync::decide::Resolution;
use crate::sync::decide::SyncDirection;
use crate::sync::state::SyncState;

pub struct RemoteFacts<'a> {
    pub presence: RemotePresence,
    pub remote_updated: &'a RemoteTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedAction {
    pub id: String,
    pub state: SyncState,
    pub action: Action,
}

pub struct SyncPlan {
    pub actions: Vec<PlannedAction>,
}

impl SyncPlan {
    #[must_use]
    pub fn pull_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|planned| planned.action == Action::Pull)
            .count()
    }

    #[must_use]
    pub fn push_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|planned| planned.action == Action::Push)
            .count()
    }
}

/// One item's classification inputs.
///
/// Reuses [`Subject`] and [`BaselineEntry`] rather than a parallel shape:
/// everything `classify` needs, including the item's own
/// `&dyn ItemDigests`, so `plan` needs no separate lookup port.
pub struct PlanInput<'a> {
    pub id: String,
    pub external_id: Option<&'a ExternalId>,
    pub facts: RemoteFacts<'a>,
    pub dirty: Dirtiness,
    pub baseline: BaselineEntry<'a>,
    pub baseline_timestamp: u64,
    pub digests: &'a dyn ItemDigests,
}

/// The two-tier read rule as a pure function: which ids need a `show`
/// before the plan can be completed, expressed as the ids whose stamp does
/// not prove unchanged against their baseline entry.
///
/// Uses `proves_unchanged_since`, not bash's raw string equality — a
/// deliberate divergence recorded where it is decided; the adapter's fetch
/// shell asks this rather than owning the rule itself.
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

/// Computes a complete plan: one [`PlannedAction`] per item.
///
/// A `Prompt` with a matching resolution becomes `Pull` (`AcceptRemote`) or
/// `Push` (`PushLocal`); `Skip` and no order leave it `Prompt`. Resolutions
/// naming an id that did not decide `Prompt` are inert — `plan` only reads
/// the map, it does not validate it, so a stale or misdirected order simply
/// finds nothing to apply to.
///
/// # Errors
///
/// Propagates any [`ItemDigests`] read failure encountered while
/// classifying.
pub fn plan(
    items: &[PlanInput<'_>],
    direction: SyncDirection,
    resolutions: &BTreeMap<String, Resolution>,
) -> Result<SyncPlan, kernel::Error> {
    let mut actions = Vec::with_capacity(items.len());
    for item in items {
        let subject = Subject {
            external_id: item.external_id,
            presence: item.facts.presence,
            remote_updated: item.facts.remote_updated,
            baseline_timestamp: item.baseline_timestamp,
        };
        let state = classify::classify(&subject, &item.baseline, item.digests)?;
        let mut action = decide::decide(direction, state, item.dirty);
        if action == Action::Prompt {
            if let Some(resolution) = resolutions.get(&item.id) {
                action = match resolution {
                    Resolution::AcceptRemote => Action::Pull,
                    Resolution::PushLocal => Action::Push,
                    Resolution::Skip => Action::Prompt,
                };
            }
        }
        actions.push(PlannedAction {
            id: item.id.clone(),
            state,
            action,
        });
    }
    Ok(SyncPlan { actions })
}
