//! One `work rekey` invocation: the run-plan state machine, every
//! transition taken under the identity-run, retirement and create locks.

use std::cell::RefCell;
use std::path::PathBuf;

use corpus::lock::LockName;
use corpus::IdOwnership;
use corpus::WorkItemIdScheme;
use tracker::ExternalId;
use vcs::RepositoryProbe;
use work::draft_id::SuffixDraws;
use work::identity::ItemIdentity;
use work::rekey::collision::refuse_collisions;
use work::rekey::creation_date::CreationDate;
use work::rekey::entry::RekeyEntry;
use work::rekey::fingerprint::IdentityFingerprint;
use work::rekey::mapping::keys_to_refresh;
use work::rekey::mapping::numeric_mapping;
use work::rekey::mapping::tracker_mapping;
use work::rekey::mapping::RefreshedKeys;
use work::rekey::mapping::RekeyCandidate;
use work::rekey::outcome::Holder;
use work::rekey::outcome::NotRekeyed;
use work::rekey::partly_applied::partly_applied;
use work::rekey::run_plan::preview_lines;
use work::rekey::run_plan::InProgressPlan;
use work::rekey::run_plan::ProposedPlan;
use work::rekey::run_plan::RunPlan;
use work::rekey::run_plan::UnreadablePlan;
use work::rekey::write_set::Base;
use work::rekey::write_set::Unreverted;
use work::rekey::write_set::WriteSet;
use work::work_item_files::identity_of;
use work::work_item_files::WorkItemFiles;

use crate::rekey::execute::current_states;
use crate::rekey::execute::execute;
use crate::rekey::execute::outstanding_entries;
use crate::rekey::execute::write_set;
use crate::rekey::execute::RekeyCorpus;
use crate::rekey::guards::pending_pushes;
use crate::rekey::guards::unfinished_sync_work;
use crate::rekey::guards::unsettled_recoveries;
use crate::rekey::guards::SyncState;
use crate::rekey::run_plan_store::RunPlanStore;
use crate::rekey::working_copy::guarded_working_copy;
use crate::retirement::acquire_retirement_lock;
use crate::retirement::RetirementLockGuard;
use work::identity_run::begin_rekey_run;
use work::identity_run::IdentityRun;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RekeyAction {
    Apply,
    Preview,
    Discard,
}

/// Whether `ACCELERATOR_FORCE` asked to skip what would otherwise refuse:
/// the working-copy check on an apply, the revert check on a discard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forcing {
    Unforced,
    Forced,
}

#[derive(Debug, Clone)]
pub struct RekeyRequest {
    pub action: RekeyAction,
    pub forcing: Forcing,
    pub scheme: WorkItemIdScheme,
}

/// What a person sees while a run proceeds, before its report.
pub trait RekeyProgress {
    fn waiting(&self, holder: Option<&Holder>);

    fn applied(&self, entry: &RekeyEntry, path: &str);

    /// The entries an in-progress run has still to retire, shown before a
    /// preview refuses.
    fn remaining(&self, lines: &[String]);

    /// The evidence of a half-applied entry an unreadable plan's discard
    /// shows before it refuses or sets the plan aside.
    fn partly_applied(&self, items: &[PathBuf], recoveries: &[PathBuf]);
}

/// The keys synced items' issues answer to now, read only when a run
/// computes a `{tracker}` mapping, so finishing, applying a preview or
/// discarding never needs a tracker.
pub trait KeyRefresh {
    /// # Errors
    ///
    /// [`NotRekeyed::TrackerUnreachable`] when a read fails.
    fn refresh(&self, keys: &[ExternalId])
        -> Result<RefreshedKeys, NotRekeyed>;
}

#[derive(Clone, Copy)]
pub struct RekeyPorts<'a> {
    pub identity_run: &'a dyn IdentityRun,
    pub store: &'a dyn RunPlanStore,
    pub corpus: RekeyCorpus<'a>,
    pub items: &'a dyn WorkItemFiles,
    pub sync: SyncState<'a>,
    pub repository: &'a dyn RepositoryProbe,
    /// Repo-relative prefixes whose dirty paths refuse an apply.
    pub scopes: &'a [String],
    pub progress: &'a dyn RekeyProgress,
    pub refresh: &'a dyn KeyRefresh,
    pub draws: &'a RefCell<dyn SuffixDraws>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RekeyReport {
    AlreadyConverged,
    /// `dirty` names the uncommitted paths an apply would refuse over.
    Previewed {
        lines: Vec<String>,
        dirty: Vec<String>,
    },
    Rekeyed {
        resumed: bool,
    },
    NothingToDiscard,
    Discarded,
    SetAside {
        aside: PathBuf,
        mapping: Vec<String>,
        unreverted: Unreverted,
    },
    UnreadableSetAside {
        aside: PathBuf,
    },
}

fn unwritable(error: &impl std::fmt::Display) -> NotRekeyed {
    NotRekeyed::StateUnwritable(error.to_string())
}

/// Runs `request` to its end, or to the refusal that stops it.
///
/// # Errors
///
/// [`NotRekeyed`] naming why the run stopped; a refusal before the first
/// write leaves everything, the run plan included, as it found it.
pub fn run(
    request: &RekeyRequest,
    ports: &RekeyPorts<'_>,
) -> Result<RekeyReport, NotRekeyed> {
    let _identity_run = begin_rekey_run(ports.identity_run, &|holder| {
        ports.progress.waiting(holder);
    })?;
    std::fs::create_dir_all(ports.corpus.ports.layout.work_dir)
        .map_err(|error| unwritable(&error))?;
    let locks = ports.corpus.ports.lock;
    let retirement =
        acquire_retirement_lock(locks).map_err(|error| unwritable(&error))?;
    let _create = locks
        .acquire(&LockName::Create)
        .map_err(|error| unwritable(&error))?;
    let loaded = ports.store.load();
    match request.action {
        RekeyAction::Apply => apply(loaded, request, ports, &retirement),
        RekeyAction::Preview => preview(loaded, request, ports),
        RekeyAction::Discard => discard(loaded, request.forcing, ports),
    }
}

fn apply(
    loaded: Result<Option<RunPlan>, UnreadablePlan>,
    request: &RekeyRequest,
    ports: &RekeyPorts<'_>,
    lock: &RetirementLockGuard,
) -> Result<RekeyReport, NotRekeyed> {
    let mapping = match loaded.map_err(NotRekeyed::PlanUnreadable)? {
        Some(RunPlan::InProgress(plan)) => {
            let base = working_copy(request.forcing, ports, plan.write_set())?;
            finish(plan, base.as_ref(), ports, lock)?;
            return Ok(RekeyReport::Rekeyed { resumed: true });
        }
        Some(RunPlan::Proposed(plan)) => Mapping::Previewed(plan),
        None => Mapping::Computed,
    };
    let Some(checked) = preflight(mapping, DirtyPaths::Refuse, request, ports)?
    else {
        return Ok(RekeyReport::AlreadyConverged);
    };
    let base = checked.base.as_ref();
    let recorded = write_set(checked.plan.entries(), &ports.corpus, base)?;
    let plan = checked.plan.started(recorded);
    ports.store.save(&RunPlan::InProgress(plan.clone()))?;
    finish(plan, base, ports, lock)?;
    Ok(RekeyReport::Rekeyed { resumed: false })
}

fn preview(
    loaded: Result<Option<RunPlan>, UnreadablePlan>,
    request: &RekeyRequest,
    ports: &RekeyPorts<'_>,
) -> Result<RekeyReport, NotRekeyed> {
    if let Some(RunPlan::InProgress(plan)) =
        loaded.map_err(NotRekeyed::PlanUnreadable)?
    {
        let remaining = outstanding_entries(&plan, &ports.corpus)?;
        ports.progress.remaining(&preview_lines(&remaining));
        return Err(NotRekeyed::InProgress(None));
    }
    let Some(checked) =
        preflight(Mapping::Computed, DirtyPaths::Warn, request, ports)?
    else {
        ports.store.remove()?;
        return Ok(RekeyReport::AlreadyConverged);
    };
    let lines = checked.plan.preview_lines();
    ports.store.save(&RunPlan::Proposed(checked.plan))?;
    Ok(RekeyReport::Previewed {
        lines,
        dirty: checked.dirty,
    })
}

/// Where a run's entries come from: computed from the corpus now, or
/// stored by the preview that showed them.
enum Mapping {
    Computed,
    Previewed(ProposedPlan),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DirtyPaths {
    Refuse,
    Warn,
}

/// A mapping every guard has passed, with the working copy it was checked
/// against.
struct Checked {
    plan: ProposedPlan,
    base: Option<Base>,
    dirty: Vec<String>,
}

/// Every check a mapping must pass before a run records it; `None` when
/// every item is already on the pattern.
fn preflight(
    mapping: Mapping,
    dirty_paths: DirtyPaths,
    request: &RekeyRequest,
    ports: &RekeyPorts<'_>,
) -> Result<Option<Checked>, NotRekeyed> {
    let blockers = unfinished_sync_work(&ports.sync);
    if !blockers.is_empty() {
        return Err(NotRekeyed::SyncUnfinished(blockers));
    }
    let candidates = candidates(ports.items)?;
    let items: Vec<ItemIdentity> = candidates
        .iter()
        .map(|candidate| candidate.identity.clone())
        .collect();
    refuse_partly_applied(&items, ports)?;
    let plan = match mapping {
        Mapping::Computed => ProposedPlan::new(
            IdentityFingerprint::of(&request.scheme, &items),
            computed(&request.scheme, &candidates, ports)?,
        ),
        Mapping::Previewed(plan) => {
            still_current(plan, &request.scheme, &items)?
        }
    };
    if plan.entries().is_empty() {
        return Ok(None);
    }
    let pushes = pending_pushes(&ports.sync, plan.entries(), &items);
    if !pushes.is_empty() {
        return Err(NotRekeyed::PendingPush(pushes));
    }
    let (base, dirty) =
        match working_copy(request.forcing, ports, &WriteSet::default()) {
            Err(NotRekeyed::DirtyWorkingCopy(dirty))
                if dirty_paths == DirtyPaths::Warn =>
            {
                (None, dirty)
            }
            observed => (observed?, Vec::new()),
        };
    Ok(Some(Checked { plan, base, dirty }))
}

fn computed(
    scheme: &WorkItemIdScheme,
    candidates: &[RekeyCandidate],
    ports: &RekeyPorts<'_>,
) -> Result<Vec<RekeyEntry>, NotRekeyed> {
    match scheme.ownership() {
        IdOwnership::Local => numeric_mapping(scheme, candidates),
        IdOwnership::Tracker => {
            let stored: Vec<ExternalId> = keys_to_refresh(candidates)?
                .iter()
                .map(|key| ExternalId::new(key.as_str().to_owned()))
                .collect();
            tracker_mapping(
                candidates,
                &ports.refresh.refresh(&stored)?,
                &mut *ports.draws.borrow_mut(),
            )
        }
    }
}

/// `plan` when the identities and scheme it was previewed from still hold
/// and none of its targets has since been claimed.
fn still_current(
    plan: ProposedPlan,
    scheme: &WorkItemIdScheme,
    items: &[ItemIdentity],
) -> Result<ProposedPlan, NotRekeyed> {
    if let Some(staleness) = IdentityFingerprint::of(scheme, items)
        .staleness_since(plan.fingerprint())
    {
        return Err(NotRekeyed::PreviewStale(staleness));
    }
    refuse_collisions(plan.entries(), items)?;
    Ok(plan)
}

fn finish(
    plan: InProgressPlan,
    base: Option<&Base>,
    ports: &RekeyPorts<'_>,
    lock: &RetirementLockGuard,
) -> Result<(), NotRekeyed> {
    execute(
        plan,
        base,
        ports.store,
        &ports.corpus,
        lock,
        &|entry, path| {
            ports.progress.applied(entry, path);
        },
    )?;
    ports.store.remove()
}

fn working_copy(
    forcing: Forcing,
    ports: &RekeyPorts<'_>,
    exempt: &WriteSet,
) -> Result<Option<Base>, NotRekeyed> {
    match forcing {
        Forcing::Forced => Ok(None),
        Forcing::Unforced => guarded_working_copy(
            ports.corpus.repo_root,
            ports.repository,
            ports.scopes,
            exempt,
        )
        .map(Some),
    }
}

fn candidates(
    items: &dyn WorkItemFiles,
) -> Result<Vec<RekeyCandidate>, NotRekeyed> {
    Ok(items
        .files()
        .map_err(|error| unwritable(&error))?
        .iter()
        .filter_map(|file| {
            identity_of(file).map(|identity| RekeyCandidate {
                identity,
                created: CreationDate::of(file),
            })
        })
        .collect())
}

/// Every item and recovery directory showing an entry a lost run left
/// half-applied.
fn half_applied(
    items: &[ItemIdentity],
    ports: &RekeyPorts<'_>,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    (partly_applied(items), unsettled_recoveries(&ports.sync))
}

fn refuse_partly_applied(
    items: &[ItemIdentity],
    ports: &RekeyPorts<'_>,
) -> Result<(), NotRekeyed> {
    let (items, recoveries) = half_applied(items, ports);
    if items.is_empty() && recoveries.is_empty() {
        Ok(())
    } else {
        Err(NotRekeyed::PartlyApplied { items, recoveries })
    }
}

fn discard(
    loaded: Result<Option<RunPlan>, UnreadablePlan>,
    forcing: Forcing,
    ports: &RekeyPorts<'_>,
) -> Result<RekeyReport, NotRekeyed> {
    match loaded {
        Ok(None) => Ok(RekeyReport::NothingToDiscard),
        Ok(Some(RunPlan::Proposed(_))) => {
            ports.store.remove()?;
            Ok(RekeyReport::Discarded)
        }
        Ok(Some(RunPlan::InProgress(plan))) => {
            discard_in_progress(&plan, forcing, ports)
        }
        Err(unreadable) => {
            let items: Vec<ItemIdentity> = candidates(ports.items)?
                .into_iter()
                .map(|candidate| candidate.identity)
                .collect();
            let (items, recoveries) = half_applied(&items, ports);
            ports.progress.partly_applied(&items, &recoveries);
            match forcing {
                Forcing::Unforced => {
                    Err(NotRekeyed::PlanUnreadable(unreadable))
                }
                Forcing::Forced => Ok(RekeyReport::UnreadableSetAside {
                    aside: ports.store.set_aside()?,
                }),
            }
        }
    }
}

fn discard_in_progress(
    plan: &InProgressPlan,
    forcing: Forcing,
    ports: &RekeyPorts<'_>,
) -> Result<RekeyReport, NotRekeyed> {
    let now = current_states(plan.write_set(), &ports.corpus)?;
    let unreverted = plan.write_set().unreverted(&now);
    if unreverted.is_empty() {
        ports.store.remove()?;
        return Ok(RekeyReport::Discarded);
    }
    match forcing {
        Forcing::Unforced => Err(NotRekeyed::NotReverted(unreverted)),
        Forcing::Forced => Ok(RekeyReport::SetAside {
            aside: ports.store.set_aside()?,
            mapping: plan.preview_lines(),
            unreverted,
        }),
    }
}
