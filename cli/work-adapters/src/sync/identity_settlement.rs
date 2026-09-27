//! The identity pass: everything that changes an item's identity settles
//! before the engine plans, so the engine only ever sees the corpus after
//! every retirement has landed.
//!
//! It finishes retirements an earlier run left interrupted, then follows the
//! key changes the run's single remote read revealed.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use corpus::IdOwnership;
use tracker::Ceiling;
use tracker::Completeness;
use tracker::ExternalId;
use work::identity::linker_of;
use work::retirement::Retirement;
use work::retirement::RetirementFailure;
use work::retirement::RetirementRecord;
use work::sync::decide_key_change;
use work::sync::IdentityAction;
use work::sync::KeyChange;

use crate::retirement::acquire_retirement_lock;
use crate::retirement::corpus_identities;
use crate::retirement::finish_retirement;
use crate::retirement::retirement_outstanding;
use crate::retirement::FinishFailure;
use crate::retirement::RetirementPorts;
use crate::retirement_records::RetirementRecords;
use crate::sync::fetch;
use crate::sync::fetch::GatheredFacts;
use crate::sync::fetch::IdentityObservation;
use crate::sync::fetch::LocalItem;
use crate::sync::run::RunError;
use crate::sync::run::RunMode;
use crate::sync::run::SettledView;
use crate::sync::run::SyncPorts;
use crate::sync::run::SyncRequest;

pub struct SettlementPorts<'a> {
    pub retirement: &'a RetirementPorts<'a>,
    pub records: &'a dyn RetirementRecords,
    pub ownership: IdOwnership,
    /// Where recovery directories live, so a failure can name one on disk.
    pub state_dir: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityOutcome {
    Applied,
    NotApplied,
    /// The change would have made two items claim one identity.
    Refused(String),
    /// The change was attempted and rolled back.
    Failed(String),
}

/// One item's identity change, keyed by the id the item had when the run
/// started; `settled_id` is the id it has after, which the engine knows it
/// by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRow {
    pub id: String,
    pub settled_id: String,
    pub action: IdentityAction,
    pub detail: String,
    pub outcome: IdentityOutcome,
}

impl IdentityRow {
    /// A refused or failed change, and an issue the tracker cannot find,
    /// each need a person.
    #[must_use]
    pub const fn awaits_human(&self) -> bool {
        matches!(
            self.outcome,
            IdentityOutcome::Refused(_) | IdentityOutcome::Failed(_)
        ) || matches!(self.action, IdentityAction::NotFound)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityPlan {
    pub key_changes: Vec<KeyChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ceilings {
    pub max_pulls: Ceiling,
    pub max_pushes: Ceiling,
}

/// Refuses an identity plan whose decided changes exceed the pull bound.
/// Every decided change counts, whether or not it then applies.
///
/// # Errors
///
/// [`RunError::Refused`] when the plan is over a bound.
pub const fn check_ceilings(
    plan: &IdentityPlan,
    limits: &Ceilings,
) -> Result<(), RunError> {
    let pulls = plan.key_changes.len();
    if limits.max_pulls.exceeds(pulls) {
        return Err(RunError::Refused {
            pulls,
            pushes: 0,
            max_pulls: limits.max_pulls,
            max_pushes: limits.max_pushes,
            new_local_files: 0,
            new_remote_issues: 0,
        });
    }
    Ok(())
}

pub struct SettlementReport {
    pub rows: Vec<IdentityRow>,
    /// The run's remote facts, with each applied retirement's fact moved to
    /// the item's new id.
    pub facts: GatheredFacts,
    /// Decided key changes, which the engine's pull budget no longer has.
    pub pulls_used: usize,
    pub view: SettledView,
    /// Items whose detected key change was not applied; the engine leaves
    /// them for the next run.
    pub unsettled: BTreeSet<String>,
    /// Old id to new id, for every retirement this run applied.
    pub renamed: BTreeMap<String, String>,
    /// Identity changes that landed, including finished retirements.
    pub applied: usize,
}

fn arrow(old: &str, new: &str) -> String {
    format!("{old}->{new}")
}

fn internal(error: impl std::fmt::Display) -> RunError {
    RunError::Internal(kernel::Error::Failed(error.to_string()))
}

fn incomplete(
    failure: &RetirementFailure,
    retirement: &Retirement<'_>,
    settlement: &SettlementPorts<'_>,
) -> RunError {
    RunError::RetirementIncomplete {
        message: failure.message(
            retirement,
            &settlement.state_dir.join(retirement.recovery_dir()),
        ),
    }
}

/// Finishes a retirement under the held lock and clears its record: landed,
/// or a row explaining why not. A restore that could not complete stops the
/// pass instead, and leaves the record for the next run.
fn finish_locked(
    retirement: &Retirement<'_>,
    settlement: &SettlementPorts<'_>,
    lock: &crate::retirement::RetirementLockGuard,
) -> Result<IdentityOutcome, RunError> {
    let record = RetirementRecord::of(retirement);
    let outcome =
        match finish_retirement(retirement, settlement.retirement, lock) {
            Ok(()) => IdentityOutcome::Applied,
            Err(FinishFailure::Refused(refusal)) => {
                IdentityOutcome::Refused(refusal.message(retirement))
            }
            Err(FinishFailure::Failed(
                failure @ RetirementFailure::RestoreIncomplete { .. },
            )) => return Err(incomplete(&failure, retirement, settlement)),
            Err(FinishFailure::Failed(failure)) => {
                IdentityOutcome::Failed(failure.message(
                    retirement,
                    &settlement.state_dir.join(retirement.recovery_dir()),
                ))
            }
        };
    settlement.records.remove(&record).map_err(internal)?;
    Ok(outcome)
}

/// Finishes every retirement a record says is still in progress, wherever
/// in the corpus it is: reconciliation completes work already started, so
/// it ignores the run's targets and budget.
fn reconcile(
    mode: RunMode,
    settlement: &SettlementPorts<'_>,
) -> Result<Vec<IdentityRow>, RunError> {
    let mut rows = Vec::new();
    for record in settlement.records.outstanding().map_err(internal)? {
        let retirement = record.retirement();
        let row = |outcome| IdentityRow {
            id: record.old.clone(),
            settled_id: record.new.clone(),
            action: IdentityAction::Resumed,
            detail: arrow(&record.old, &record.new),
            outcome,
        };
        if mode == RunMode::Preview {
            if retirement_outstanding(&retirement, settlement.retirement)
                .unwrap_or(true)
            {
                rows.push(row(IdentityOutcome::NotApplied));
            }
            continue;
        }
        let lock = acquire_retirement_lock(settlement.retirement.lock)
            .map_err(internal)?;
        let still_recorded = settlement
            .records
            .outstanding()
            .map_err(internal)?
            .contains(&record);
        if !still_recorded {
            continue;
        }
        if !retirement_outstanding(&retirement, settlement.retirement)
            .unwrap_or(true)
        {
            settlement.records.remove(&record).map_err(internal)?;
            continue;
        }
        rows.push(row(finish_locked(&retirement, settlement, &lock)?));
    }
    Ok(rows)
}

fn not_found_row(item: &str, key: &ExternalId) -> IdentityRow {
    IdentityRow {
        id: item.to_owned(),
        settled_id: item.to_owned(),
        action: IdentityAction::NotFound,
        detail: key.to_string(),
        outcome: IdentityOutcome::NotApplied,
    }
}

/// Reads the remote once over the run's items and decides, without
/// applying, every key change it revealed.
fn detect_identity_changes(
    request: &SyncRequest<'_>,
    ports: &SyncPorts<'_>,
    settlement: &SettlementPorts<'_>,
) -> Result<(IdentityPlan, Vec<IdentityRow>, GatheredFacts), RunError> {
    let (baseline, _) = settlement
        .retirement
        .baseline
        .load()
        .map_err(|error| RunError::Internal(error.into()))?;
    let facts = fetch::gather(
        request.reconciled(),
        &baseline,
        ports.tracker,
        ports.status,
        request.strategy,
    );
    if facts.keyed_read == Completeness::CapHit {
        return Err(RunError::KeyedReadCapped);
    }
    let mut plan = IdentityPlan::default();
    let mut not_found = Vec::new();
    for (item, observation) in &facts.identity {
        match observation {
            IdentityObservation::Moved { old, new } => plan
                .key_changes
                .push(decide_key_change(settlement.ownership, item, old, new)),
            IdentityObservation::NotFound { key } => {
                not_found.push(not_found_row(item, key));
            }
        }
    }
    Ok((plan, not_found, facts))
}

fn key_change_row(change: &KeyChange, outcome: IdentityOutcome) -> IdentityRow {
    let settled_id = match (&outcome, change) {
        (IdentityOutcome::Applied, KeyChange::RetireKey { new, .. }) => {
            new.to_string()
        }
        _ => change.item().to_owned(),
    };
    IdentityRow {
        id: change.item().to_owned(),
        settled_id,
        action: IdentityAction::KeyChanged,
        detail: arrow(change.old().as_str(), change.new_key().as_str()),
        outcome,
    }
}

fn follow_external_id(
    item: &str,
    old: &ExternalId,
    new: &ExternalId,
    request: &SyncRequest<'_>,
    ports: &SyncPorts<'_>,
    settlement: &SettlementPorts<'_>,
) -> Result<IdentityOutcome, RunError> {
    let Some(local) = request.corpus.iter().find(|local| local.id == item)
    else {
        return Ok(IdentityOutcome::Failed(format!(
            "cannot follow {old} to {new}: {item} is no longer in the corpus"
        )));
    };
    let identities = corpus_identities(settlement.retirement)
        .map_err(|failure| internal(format!("{failure:?}")))?;
    let others: Vec<_> = identities
        .into_iter()
        .filter(|identity| identity.id != item)
        .collect();
    if let Some(linker) = linker_of(new.as_str(), &others) {
        return Ok(IdentityOutcome::Refused(format!(
            "cannot follow {old} to {new} for {item}: {} ({}) is already \
             linked to {new}",
            linker.id,
            linker.path.display()
        )));
    }
    Ok(match ports.author.link_external_id(&local.path, new) {
        Ok(()) => IdentityOutcome::Applied,
        Err(error) => IdentityOutcome::Failed(format!(
            "cannot follow {old} to {new} for {item}: {error}"
        )),
    })
}

fn retire_key(
    item: &str,
    new: &ExternalId,
    settlement: &SettlementPorts<'_>,
) -> Result<IdentityOutcome, RunError> {
    let retirement = Retirement {
        old_id: item,
        new_id: new.as_str(),
        new_external_id: Some(new.as_str()),
    };
    let lock = acquire_retirement_lock(settlement.retirement.lock)
        .map_err(internal)?;
    settlement
        .records
        .save(&RetirementRecord::of(&retirement))
        .map_err(internal)?;
    finish_locked(&retirement, settlement, &lock)
}

fn apply_key_change(
    change: &KeyChange,
    request: &SyncRequest<'_>,
    ports: &SyncPorts<'_>,
    settlement: &SettlementPorts<'_>,
) -> Result<IdentityOutcome, RunError> {
    match change {
        KeyChange::FollowExternalId { item, old, new } => {
            follow_external_id(item, old, new, request, ports, settlement)
        }
        KeyChange::RetireKey { item, new, .. } => {
            retire_key(item, new, settlement)
        }
    }
}

/// Moves each applied retirement's fact to the item's new id and drops the
/// identity observations the pass has now dealt with.
fn rekeyed(
    mut facts: GatheredFacts,
    renamed: &BTreeMap<String, String>,
) -> GatheredFacts {
    for (old, new) in renamed {
        if let Some(fact) = facts.per_id.remove(old) {
            facts.per_id.insert(new.clone(), fact);
        }
    }
    facts.identity.clear();
    facts
}

/// Settles every identity change before the engine plans: finishes
/// interrupted retirements, then follows the key changes the run's one
/// remote read revealed.
///
/// Every change is decided, and every ceiling checked, before any is
/// applied. Under preview nothing is applied.
///
/// # Errors
///
/// [`RunError::Refused`] when the decided changes exceed the pull bound,
/// [`RunError::RetirementIncomplete`] when a retirement could not restore
/// the corpus, [`RunError::KeyedReadCapped`] when the remote read hit its
/// page cap, and [`RunError::Internal`] when the records, the baseline or
/// the corpus cannot be read.
pub fn settle_identities(
    request: &SyncRequest<'_>,
    ports: &SyncPorts<'_>,
    settlement: &SettlementPorts<'_>,
) -> Result<SettlementReport, RunError> {
    let mut rows = reconcile(request.mode, settlement)?;
    let (plan, not_found, facts) =
        detect_identity_changes(request, ports, settlement)?;
    check_ceilings(
        &plan,
        &Ceilings {
            max_pulls: request.max_pulls,
            max_pushes: request.max_pushes,
        },
    )?;

    let mut renamed = BTreeMap::new();
    let mut unsettled = BTreeSet::new();
    for change in &plan.key_changes {
        let outcome = if request.mode == RunMode::Preview {
            IdentityOutcome::NotApplied
        } else {
            apply_key_change(change, request, ports, settlement)?
        };
        match (&outcome, change) {
            (
                IdentityOutcome::Applied,
                KeyChange::RetireKey { item, new, .. },
            ) => {
                renamed.insert(item.clone(), new.to_string());
            }
            (IdentityOutcome::Applied, KeyChange::FollowExternalId { .. }) => {}
            _ => {
                unsettled.insert(change.item().to_owned());
            }
        }
        rows.push(key_change_row(change, outcome));
    }
    rows.extend(not_found);

    let applied = rows
        .iter()
        .filter(|row| row.outcome == IdentityOutcome::Applied)
        .count();
    let view = SettledView::following(
        plan.key_changes
            .iter()
            .map(|change| change.new_key().clone()),
    );
    Ok(SettlementReport {
        rows,
        facts: rekeyed(facts, &renamed),
        pulls_used: plan.key_changes.len(),
        view,
        unsettled,
        renamed,
        applied,
    })
}

/// The ids the engine may plan: re-discovered items the pass has a fact for,
/// and whose identity it settled. The rest are counted as deferred.
#[must_use]
pub fn plannable<'a>(
    items: impl IntoIterator<Item = &'a LocalItem>,
    report: &SettlementReport,
) -> (Vec<LocalItem>, usize) {
    let mut plannable = Vec::new();
    let mut deferred = 0;
    for item in items {
        if report.unsettled.contains(&item.id) {
            continue;
        }
        if report.facts.per_id.contains_key(&item.id) {
            plannable.push(item.clone());
        } else {
            deferred += 1;
        }
    }
    (plannable, deferred)
}

#[cfg(test)]
mod tests {
    use tracker::Ceiling;
    use tracker::ExternalId;
    use work::sync::KeyChange;

    use super::check_ceilings;
    use super::Ceilings;
    use super::IdentityPlan;
    use crate::sync::run::RunError;

    fn changes(count: usize) -> IdentityPlan {
        IdentityPlan {
            key_changes: (0..count)
                .map(|index| KeyChange::FollowExternalId {
                    item: format!("{index:04}"),
                    old: ExternalId::new(format!("PP-{index}")),
                    new: ExternalId::new(format!("ENG-{index}")),
                })
                .collect(),
        }
    }

    #[test]
    fn check_ceilings_counts_decided_actions() {
        let cases = [
            (0, Ceiling::Bounded(0), true),
            (1, Ceiling::Bounded(0), false),
            (2, Ceiling::Bounded(2), true),
            (3, Ceiling::Bounded(2), false),
            (100, Ceiling::Unlimited, true),
        ];
        for (count, max_pulls, allowed) in cases {
            let checked = check_ceilings(
                &changes(count),
                &Ceilings {
                    max_pulls,
                    max_pushes: Ceiling::Bounded(0),
                },
            );

            assert_eq!(checked.is_ok(), allowed, "{count} over {max_pulls}");
            if let Err(RunError::Refused { pulls, pushes, .. }) = checked {
                assert_eq!((pulls, pushes), (count, 0));
            }
        }
    }
}
