//! The whole-corpus sync run: plan-then-apply over the gathered facts.

use std::collections::BTreeMap;

use corpus::store::AtomicWrite;
use tracker::RemoteTracker;
use tracker::TrackerError;
use work::sync::plan as compute_plan;
use work::sync::Action;
use work::sync::PlannedAction;
use work::sync::Resolution;
use work::sync::RunClock;
use work::sync::SyncDirection;
use work::sync::SyncState;

use crate::sync::apply::ApplyError;
use crate::sync::apply::ItemApplier;
use crate::sync::apply::PullRequest;
use crate::sync::apply::PushRequest;
use crate::sync::baseline::Degradation;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::digest::LazyItemDigests;
use crate::sync::fetch;
use crate::sync::fetch::LocalItem;
use crate::sync::fetch::RetrievalStrategy;
use crate::sync::fetch::WorkingCopyStatus;

pub enum RunError {
    Refused {
        pulls: usize,
        pushes: usize,
        max_pulls: usize,
        max_pushes: usize,
    },
    Read(TrackerError),
    Internal(kernel::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Preview,
    Apply,
}

pub struct SyncPorts<'a> {
    pub tracker: &'a dyn RemoteTracker,
    pub status: &'a dyn WorkingCopyStatus,
    pub writer: &'a dyn AtomicWrite,
    pub clock: &'a dyn RunClock,
}

pub struct SyncRequest<'a> {
    pub items: &'a [LocalItem],
    pub direction: SyncDirection,
    pub strategy: RetrievalStrategy,
    pub resolutions: &'a BTreeMap<String, Resolution>,
    pub max_pulls: usize,
    pub max_pushes: usize,
    pub mode: RunMode,
}

pub enum ItemOutcome {
    Applied,
    NotApplied,
    Failed(ApplyError),
}

pub struct ReportedItem {
    pub planned: PlannedAction,
    pub outcome: ItemOutcome,
}

pub struct RunReport {
    pub reported: Vec<ReportedItem>,
    pub read_failure: Option<TrackerError>,
    pub baseline_degradation: Degradation,
    pub finalised: bool,
}

impl RunReport {
    /// Items this run left for a human: `Prompt`, `SkipConflict`,
    /// `SkipDirty`, `RemoteAbsent` and `Indeterminate`. Derived, never
    /// stored — a filter applied to a stored field and not to the report
    /// could disagree, yielding a run that reports conflicts and exits 0.
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

fn local_title_and_body(content: &str) -> (String, String) {
    let (frontmatter, body) =
        crate::sync::digest::split_frontmatter_and_body(content)
            .unwrap_or_default();
    let title =
        work::show::read_field_raw(&frontmatter, "title").unwrap_or_default();
    (title, body)
}

fn reconstruct_pulled_content(
    local_content: &str,
    remote_body: &str,
) -> String {
    let (frontmatter, _) =
        crate::sync::digest::split_frontmatter_and_body(local_content)
            .unwrap_or_default();
    format!("---\n{frontmatter}\n---\n\n{remote_body}")
}

/// Runs a complete sync: gather, plan, and — under [`RunMode::Apply`] —
/// execute.
///
/// Refuses before any write, in both modes, when the plan's pull or push
/// count exceeds its bound; a preview that reported an over-threshold plan
/// and exited 0 would break preview's own fidelity guarantee for exactly
/// the plan with the largest blast radius.
///
/// # Errors
///
/// [`RunError::Internal`] for a clock, baseline-store or planning failure;
/// [`RunError::Refused`] when the plan would exceed the write bounds.
#[allow(clippy::too_many_lines)]
pub fn run<'a>(
    ports: &SyncPorts<'a>,
    baseline: &mut BaselineStore<'a>,
    request: &SyncRequest<'_>,
) -> Result<RunReport, RunError> {
    let run_start_epoch =
        ports.clock.run_start_epoch().map_err(RunError::Internal)?;

    let (loaded_baseline, degradation) = baseline
        .load()
        .map_err(|error| RunError::Internal(error.into()))?;

    let facts = fetch::gather(
        request.items,
        &loaded_baseline,
        ports.tracker,
        ports.status,
        request.strategy,
    );

    let digests: Vec<LazyItemDigests<'_>> = request
        .items
        .iter()
        .map(|item| {
            let remote_content = facts
                .per_id
                .get(&item.id)
                .and_then(|(remote, _)| remote.body.clone());
            LazyItemDigests::new(&item.path, remote_content)
        })
        .collect();

    let plan_inputs = facts.plan_inputs(
        request.items,
        &digests,
        &loaded_baseline,
        loaded_baseline.timestamp(),
    );

    let plan =
        compute_plan(&plan_inputs, request.direction, request.resolutions)
            .map_err(RunError::Internal)?;

    let pulls = plan.pull_count();
    let pushes = plan.push_count();
    if pulls > request.max_pulls || pushes > request.max_pushes {
        return Err(RunError::Refused {
            pulls,
            pushes,
            max_pulls: request.max_pulls,
            max_pushes: request.max_pushes,
        });
    }

    if matches!(request.mode, RunMode::Preview) {
        return Ok(RunReport {
            reported: plan
                .actions
                .into_iter()
                .map(|planned| ReportedItem {
                    planned,
                    outcome: ItemOutcome::NotApplied,
                })
                .collect(),
            read_failure: facts.read_failure,
            baseline_degradation: degradation,
            finalised: false,
        });
    }

    let mut reported = Vec::with_capacity(plan.actions.len());
    let mut blank_local_hash: Vec<String> = Vec::new();

    {
        let mut applier =
            ItemApplier::new(ports.tracker, ports.writer, baseline);
        for planned in plan.actions {
            let item = request
                .items
                .iter()
                .find(|candidate| candidate.id == planned.id);
            let outcome = match planned.action {
                Action::Push => {
                    let Some(item) = item else {
                        reported.push(ReportedItem {
                            planned,
                            outcome: ItemOutcome::NotApplied,
                        });
                        continue;
                    };
                    let Some(external_id) = &item.external_id else {
                        reported.push(ReportedItem {
                            planned,
                            outcome: ItemOutcome::NotApplied,
                        });
                        continue;
                    };
                    match std::fs::read_to_string(&item.path) {
                        Ok(content) => {
                            let (title, body) = local_title_and_body(&content);
                            match applier.push(&PushRequest {
                                id: &item.id,
                                external_id,
                                title: &title,
                                body: &body,
                                file_path: &item.path,
                            }) {
                                Ok(()) => ItemOutcome::Applied,
                                Err(error) => ItemOutcome::Failed(error),
                            }
                        }
                        Err(error) => ItemOutcome::Failed(ApplyError::Io {
                            item_id: item.id.clone(),
                            operation: "read-local",
                            detail: error.to_string(),
                        }),
                    }
                }
                Action::Pull => {
                    let Some(item) = item else {
                        reported.push(ReportedItem {
                            planned,
                            outcome: ItemOutcome::NotApplied,
                        });
                        continue;
                    };
                    let remote = facts.per_id.get(&item.id).map(|(r, _)| r);
                    let projected_body =
                        remote.and_then(|r| r.body.as_deref()).unwrap_or("");
                    let local_content =
                        std::fs::read_to_string(&item.path).unwrap_or_default();
                    let content = reconstruct_pulled_content(
                        &local_content,
                        projected_body,
                    );
                    let remote_updated = remote
                        .map_or(tracker::RemoteTimestamp::NotRead, |r| {
                            r.remote_updated.clone()
                        });
                    match applier.pull(&PullRequest {
                        id: &item.id,
                        file_path: &item.path,
                        content: &content,
                        projected_body,
                        remote_updated,
                    }) {
                        Ok(()) => ItemOutcome::Applied,
                        Err(error) => ItemOutcome::Failed(error),
                    }
                }
                Action::Prompt | Action::SkipConflict | Action::SkipDirty => {
                    if matches!(planned.state, SyncState::Conflict) {
                        blank_local_hash.push(planned.id.clone());
                    }
                    ItemOutcome::NotApplied
                }
                Action::Noop => ItemOutcome::NotApplied,
            };
            reported.push(ReportedItem { planned, outcome });
        }
    }

    let blank_refs: Vec<&str> =
        blank_local_hash.iter().map(String::as_str).collect();
    let finalised = baseline.finalise_run(&blank_refs, run_start_epoch).is_ok();

    Ok(RunReport {
        reported,
        read_failure: facts.read_failure,
        baseline_degradation: degradation,
        finalised,
    })
}
