//! The identity pass: everything that changes an item's identity settles
//! before the engine plans, so the engine only ever sees the corpus after
//! every retirement has landed.
//!
//! It finishes retirements an earlier run left interrupted, then follows the
//! key changes the run's single remote read revealed.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

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

use work::draft_id::DraftId;
use work::promotion::NotPromoted;
use work::promotion::Promotion;
use work::promotion::PromotionMode;
use work::promotion::PromotionStage;
use work::sync::SyncDirection;

use crate::promotion::finish_promotion;
use crate::promotion::promote;
use crate::promotion::PromotionOutcome;
use crate::promotion::PromotionPorts;
use crate::promotion::PromotionRow;
use crate::promotion_records::PromotionRecords;
use crate::promotion_records::StoredRecord;
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
use crate::sync::fetch::WorkingCopyStatus;
use crate::sync::run::RunError;
use crate::sync::run::RunMode;
use crate::sync::run::SettledView;
use crate::sync::run::SyncPorts;
use crate::sync::run::SyncRequest;

pub struct SettlementPorts<'a> {
    pub retirement: &'a RetirementPorts<'a>,
    pub records: &'a dyn RetirementRecords,
    pub promotions: &'a dyn PromotionRecords,
    pub ownership: IdOwnership,
    /// Where recovery directories live, so a failure can name one on disk.
    pub state_dir: &'a Path,
    /// Probes the working copy afresh, so each promotion plans its
    /// retirement over the dirtiness the ones before it left.
    pub probe_status: &'a dyn Fn() -> Box<dyn WorkingCopyStatus>,
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
    pub promotions: Vec<PlannedPromotion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPromotion {
    pub draft: DraftId,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ceilings {
    pub max_pulls: Ceiling,
    pub max_pushes: Ceiling,
}

/// Refuses an identity plan whose decided changes exceed a bound: key
/// changes count as pulls, promotions as pushes. Every decided change
/// counts, whether or not it then applies.
///
/// # Errors
///
/// [`RunError::Refused`] when the plan is over a bound.
pub const fn check_ceilings(
    plan: &IdentityPlan,
    limits: &Ceilings,
) -> Result<(), RunError> {
    let pulls = plan.key_changes.len();
    let pushes = plan.promotions.len();
    if limits.max_pulls.exceeds(pulls) || limits.max_pushes.exceeds(pushes) {
        return Err(RunError::Refused {
            pulls,
            pushes,
            max_pulls: limits.max_pulls,
            max_pushes: limits.max_pushes,
            new_local_files: 0,
            new_remote_issues: pushes,
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
    /// Decided promotions, which the engine's push budget no longer has.
    pub pushes_used: usize,
    pub promotions: Vec<PromotionRow>,
    /// Drafts this run set out to promote, which the engine leaves alone.
    pub promoting: BTreeSet<String>,
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

/// Finishes every retirement an earlier run left interrupted, as a sync
/// does before anything else changes an identity.
///
/// # Errors
///
/// [`RunError::RetirementIncomplete`] when a retirement could not restore
/// the corpus, and [`RunError::Internal`] when the records cannot be read.
pub fn finish_interrupted_retirements(
    settlement: &SettlementPorts<'_>,
) -> Result<Vec<IdentityRow>, RunError> {
    reconcile(RunMode::Apply, settlement)
}

/// Finishes every promotion a record says was retiring its draft when its
/// run stopped. Earlier stages wait for the draft's next promotion, which
/// resumes them.
fn reconcile_promotions(
    mode: RunMode,
    settlement: &SettlementPorts<'_>,
) -> Result<Vec<IdentityRow>, RunError> {
    let mut rows = Vec::new();
    for record in settlement
        .promotions
        .outstanding()
        .map_err(internal)?
        .into_iter()
        .flatten()
    {
        let PromotionStage::Retiring { key, .. } = &record.stage else {
            continue;
        };
        let row = |outcome| IdentityRow {
            id: record.draft_id.as_str().to_owned(),
            settled_id: key.to_string(),
            action: IdentityAction::Resumed,
            detail: arrow(record.draft_id.as_str(), key.as_str()),
            outcome,
        };
        if mode == RunMode::Preview {
            rows.push(row(IdentityOutcome::NotApplied));
            continue;
        }
        let lock = acquire_retirement_lock(settlement.retirement.lock)
            .map_err(internal)?;
        let StoredRecord::Present(current) =
            settlement.promotions.read(&record.draft_id)
        else {
            continue;
        };
        if *current != record {
            continue;
        }
        let retirement = Retirement {
            old_id: record.draft_id.as_str(),
            new_id: key.as_str(),
            new_external_id: Some(key.as_str()),
        };
        let outcome = match finish_promotion(
            &record,
            settlement.retirement,
            settlement.promotions,
            &lock,
        ) {
            Ok(_) => IdentityOutcome::Applied,
            Err(NotPromoted::RetirementFailed(
                failure @ RetirementFailure::RestoreIncomplete { .. },
            )) => return Err(incomplete(&failure, &retirement, settlement)),
            Err(NotPromoted::Refused(refusal)) => {
                IdentityOutcome::Refused(refusal.message(&retirement))
            }
            Err(NotPromoted::RetirementFailed(failure)) => {
                IdentityOutcome::Failed(failure.message(
                    &retirement,
                    &settlement.state_dir.join(retirement.recovery_dir()),
                ))
            }
            Err(other) => IdentityOutcome::Failed(other.keyword().to_owned()),
        };
        rows.push(row(outcome));
    }
    Ok(rows)
}

/// The key of every promotion record that has one and whose draft still
/// exists: its draft's promotion will resume onto it, so it is tracked.
fn keys_held_by_promotions(
    request: &SyncRequest<'_>,
    settlement: &SettlementPorts<'_>,
) -> Result<Vec<ExternalId>, RunError> {
    let draft_exists = |draft: &DraftId| {
        request
            .corpus
            .iter()
            .any(|item| item.id.eq_ignore_ascii_case(draft.as_str()))
    };
    Ok(settlement
        .promotions
        .outstanding()
        .map_err(internal)?
        .into_iter()
        .flatten()
        .filter(|record| draft_exists(&record.draft_id))
        .filter_map(|record| record.stage.key().cloned())
        .collect())
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
    plan.promotions = drafts_to_promote(request, settlement);
    Ok((plan, not_found, facts))
}

/// The drafts a run promotes, in id order: under a tracker-owned pattern,
/// on a run that may push, unless promotion is turned off.
fn drafts_to_promote(
    request: &SyncRequest<'_>,
    settlement: &SettlementPorts<'_>,
) -> Vec<PlannedPromotion> {
    let promotes = settlement.ownership == IdOwnership::Tracker
        && request.direction != SyncDirection::PullOnly
        && request.promote;
    if !promotes {
        return Vec::new();
    }
    let mut drafts: Vec<PlannedPromotion> = request
        .reconciled()
        .iter()
        .filter_map(|item| {
            DraftId::parse(&item.id).map(|draft| PlannedPromotion {
                draft,
                path: item.path.clone(),
            })
        })
        .collect();
    drafts.sort_by(|left, right| left.draft.as_str().cmp(right.draft.as_str()));
    drafts
}

/// Promotes one draft over a working copy probed afresh. A retirement that
/// could not restore the corpus stops the pass, as it does for a key
/// change.
fn promote_draft(
    planned: &PlannedPromotion,
    ports: &SyncPorts<'_>,
    settlement: &SettlementPorts<'_>,
) -> Result<PromotionRow, RunError> {
    let status = (settlement.probe_status)();
    let retirement = RetirementPorts {
        status: status.as_ref(),
        ..*settlement.retirement
    };
    let promotion_ports = PromotionPorts {
        tracker: ports.tracker,
        retirement: &retirement,
        records: settlement.promotions,
    };
    let result =
        promote(&planned.draft, &PromotionMode::Standard, &promotion_ports);
    if let Err(NotPromoted::RetirementFailed(
        failure @ RetirementFailure::RestoreIncomplete { .. },
    )) = &result
    {
        if let Some(key) = settlement.promotions.read(&planned.draft).key() {
            return Err(incomplete(
                failure,
                &Retirement {
                    old_id: planned.draft.as_str(),
                    new_id: key.as_str(),
                    new_external_id: Some(key.as_str()),
                },
                settlement,
            ));
        }
    }
    Ok(PromotionRow::of(
        &planned.draft,
        planned.path.clone(),
        result,
        &promotion_ports,
        settlement.state_dir,
    ))
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

/// Finishes every retirement and every retiring promotion an earlier run
/// left interrupted, before a sync reads the remote. Under preview nothing
/// is finished.
///
/// # Errors
///
/// [`RunError::RetirementIncomplete`] when a retirement could not restore
/// the corpus, and [`RunError::Internal`] when the records cannot be read.
pub fn resume_interrupted(
    mode: RunMode,
    settlement: &SettlementPorts<'_>,
) -> Result<Vec<IdentityRow>, RunError> {
    let mut rows = reconcile(mode, settlement)?;
    rows.extend(reconcile_promotions(mode, settlement)?);
    Ok(rows)
}

/// Settles every identity change before the engine plans, following the
/// key changes the run's one remote read revealed.
///
/// `resumed` are the rows [`resume_interrupted`] reported; `request` must
/// describe the corpus as that resumption left it.
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
    resumed: Vec<IdentityRow>,
) -> Result<SettlementReport, RunError> {
    let mut rows = resumed;
    let promotion_keys = keys_held_by_promotions(request, settlement)?;
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

    let mut promotions = Vec::new();
    for planned in &plan.promotions {
        promotions.push(if request.mode == RunMode::Preview {
            PromotionRow::previewed(&planned.draft, planned.path.clone())
        } else {
            promote_draft(planned, ports, settlement)?
        });
    }

    let promoted_keys: Vec<ExternalId> = promotions
        .iter()
        .filter_map(|row| match &row.outcome {
            PromotionOutcome::Promoted(
                Promotion::Completed(key, _) | Promotion::AlreadyDone(key),
            ) => Some(key.clone()),
            PromotionOutcome::Previewed
            | PromotionOutcome::NotPromoted { .. } => None,
        })
        .collect();
    let applied = rows
        .iter()
        .filter(|row| row.outcome == IdentityOutcome::Applied)
        .count()
        + promoted_keys.len();
    let view = SettledView::following(
        plan.key_changes
            .iter()
            .map(|change| change.new_key().clone())
            .chain(promotion_keys),
    )
    .promoting(promoted_keys);
    Ok(SettlementReport {
        rows,
        facts: rekeyed(facts, &renamed),
        pulls_used: plan.key_changes.len(),
        pushes_used: plan.promotions.len(),
        promoting: plan
            .promotions
            .iter()
            .map(|planned| planned.draft.as_str().to_owned())
            .collect(),
        promotions,
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
        if report.unsettled.contains(&item.id)
            || report.promoting.contains(&item.id)
            || report.view.promoted(&item.id)
        {
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
            promotions: Vec::new(),
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
