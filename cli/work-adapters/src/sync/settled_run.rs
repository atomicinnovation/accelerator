//! A whole sync run: pre-flight, the identity pass, rediscovery, then the
//! engine over the settled corpus.

use std::collections::BTreeMap;

use tracker::Ceiling;

use crate::sync::baseline_store::BaselineStore;
use crate::sync::fetch::GatheredFacts;
use crate::sync::fetch::LocalItem;
use crate::sync::fetch::WorkingCopyStatus;
use crate::sync::identity_settlement::plannable;
use crate::sync::identity_settlement::resume_interrupted;
use crate::sync::identity_settlement::settle_identities;
use crate::sync::identity_settlement::IdentityOutcome;
use crate::sync::identity_settlement::SettlementPorts;
use crate::sync::identity_settlement::SettlementReport;
use crate::sync::run::preflight;
use crate::sync::run::run_with;
use crate::sync::run::ItemSelection;
use crate::sync::run::RunError;
use crate::sync::run::RunReport;
use crate::sync::run::SyncPorts;
use crate::sync::run::SyncRequest;

/// The corpus as it stands now, with a working-copy status probed now.
pub struct DiscoveredCorpus {
    pub items: Vec<LocalItem>,
    pub status: Box<dyn WorkingCopyStatus>,
}

/// Reads the corpus afresh, so the engine plans over what the identity pass
/// left behind.
pub trait CorpusDiscovery {
    /// # Errors
    /// [`RunError`] when the corpus cannot be read.
    fn discover(&self) -> Result<DiscoveredCorpus, RunError>;
}

/// A run that failed, and how many identity changes had already landed:
/// each is complete in itself, so they stand.
#[derive(Debug)]
pub struct SettledRunFailure {
    pub error: RunError,
    pub identity_applied: usize,
}

const fn remaining(ceiling: Ceiling, used: usize) -> Ceiling {
    match ceiling {
        Ceiling::Bounded(cap) => Ceiling::Bounded(cap.saturating_sub(used)),
        Ceiling::Unlimited => Ceiling::Unlimited,
    }
}

/// The re-discovered items the run still targets: those it targeted before,
/// under whatever id `renamed` gave them.
fn retargeted<'a>(
    targets: &[LocalItem],
    rediscovered: &'a [LocalItem],
    renamed: &BTreeMap<String, String>,
) -> Vec<&'a LocalItem> {
    let ids: Vec<&str> = targets
        .iter()
        .map(|target| {
            renamed
                .get(&target.id)
                .map_or(target.id.as_str(), String::as_str)
        })
        .collect();
    rediscovered
        .iter()
        .filter(|item| ids.contains(&item.id.as_str()))
        .collect()
}

/// `request` over `corpus`, reconciling `selection`.
fn rescoped<'a>(
    request: &SyncRequest<'a>,
    corpus: &'a [LocalItem],
    selection: ItemSelection<'a>,
) -> SyncRequest<'a> {
    SyncRequest {
        corpus,
        selection,
        direction: request.direction,
        strategy: request.strategy,
        resolutions: request.resolutions,
        max_pulls: request.max_pulls,
        max_pushes: request.max_pushes,
        mode: request.mode,
        integrations_root: request.integrations_root,
        integration: request.integration,
        scope: request.scope.clone(),
        promote: request.promote,
    }
}

/// The pass's facts for the items the engine will plan, with dirtiness
/// probed again after the pass rewrote files.
fn refreshed(
    facts: GatheredFacts,
    planned: &[LocalItem],
    status: &dyn WorkingCopyStatus,
) -> GatheredFacts {
    let mut per_id = facts.per_id;
    for item in planned {
        if let Some((_, dirty)) = per_id.get_mut(&item.id) {
            *dirty = status.is_dirty(&item.path);
        }
    }
    GatheredFacts { per_id, ..facts }
}

/// Runs a whole sync: refuses a misconfigured run before anything changes,
/// settles every identity change, re-reads the corpus, then runs the engine
/// over the settled corpus with the budget the pass left.
///
/// The remote is read once, by the identity pass. An item that appeared
/// after that read has no fact to plan from and waits for the next run.
///
/// # Errors
///
/// [`SettledRunFailure`] carrying the [`RunError`] that stopped the run and
/// how many identity changes had landed before it.
pub fn run_settled<'a>(
    request: &SyncRequest<'_>,
    ports: &SyncPorts<'a>,
    settlement: &SettlementPorts<'_>,
    baseline: &mut BaselineStore<'a>,
    discovery: &dyn CorpusDiscovery,
) -> Result<RunReport, SettledRunFailure> {
    let failed = |applied| {
        move |error| SettledRunFailure {
            error,
            identity_applied: applied,
        }
    };
    preflight(ports, request).map_err(failed(0))?;
    let resumed =
        resume_interrupted(request.mode, settlement).map_err(failed(0))?;
    let resumed_renames: BTreeMap<String, String> = resumed
        .iter()
        .filter(|row| row.outcome == IdentityOutcome::Applied)
        .map(|row| (row.id.clone(), row.settled_id.clone()))
        .collect();
    let resumed_corpus = if resumed_renames.is_empty() {
        None
    } else {
        Some(
            discovery
                .discover()
                .map_err(failed(resumed_renames.len()))?,
        )
    };
    let resumed_targets: Vec<LocalItem> =
        match (&resumed_corpus, &request.selection) {
            (Some(current), ItemSelection::Targeted { items, .. }) => {
                retargeted(items, &current.items, &resumed_renames)
                    .into_iter()
                    .cloned()
                    .collect()
            }
            _ => Vec::new(),
        };
    let resumed_request = resumed_corpus.as_ref().map(|current| {
        let selection = match &request.selection {
            ItemSelection::Targeted { pull_ids, .. } => {
                ItemSelection::Targeted {
                    items: &resumed_targets,
                    pull_ids,
                }
            }
            ItemSelection::All | ItemSelection::Settled { .. } => {
                ItemSelection::All
            }
        };
        rescoped(request, &current.items, selection)
    });
    let request = resumed_request.as_ref().unwrap_or(request);
    let settled = settle_identities(request, ports, settlement, resumed)
        .map_err(failed(resumed_renames.len()))?;
    let applied = settled.applied;
    let rediscovered = discovery.discover().map_err(failed(applied))?;

    let (planned, deferred) = match &request.selection {
        ItemSelection::Targeted { items, .. } => plannable(
            retargeted(items, &rediscovered.items, &settled.renamed),
            &settled,
        ),
        ItemSelection::All | ItemSelection::Settled { .. } => {
            plannable(&rediscovered.items, &settled)
        }
    };
    let selection = match &request.selection {
        ItemSelection::Targeted { pull_ids, .. } => ItemSelection::Targeted {
            items: &planned,
            pull_ids,
        },
        ItemSelection::All | ItemSelection::Settled { .. } => {
            ItemSelection::Settled { items: &planned }
        }
    };
    let engine_request = SyncRequest {
        max_pulls: remaining(request.max_pulls, settled.pulls_used),
        max_pushes: remaining(request.max_pushes, settled.pushes_used),
        ..rescoped(request, &rediscovered.items, selection)
    };
    let SettlementReport {
        rows,
        facts,
        view,
        promotions,
        ..
    } = settled;
    let facts = refreshed(facts, &planned, rediscovered.status.as_ref());
    let mut report = run_with(ports, baseline, &engine_request, facts, &view)
        .map_err(failed(applied))?;
    report.identity = rows;
    report.promotions = promotions;
    report.deferred = deferred;
    Ok(report)
}
