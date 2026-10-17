//! One `work rekey` invocation: the run-plan state machine, every
//! transition taken under the identity-run, retirement and create locks.

use std::path::PathBuf;

use corpus::lock::LockName;
use corpus::WorkItemIdScheme;
use vcs::RepositoryProbe;
use work::identity::ItemIdentity;
use work::rekey::creation_date::CreationDate;
use work::rekey::entry::RekeyEntry;
use work::rekey::fingerprint::IdentityFingerprint;
use work::rekey::mapping::numeric_mapping;
use work::rekey::mapping::RekeyCandidate;
use work::rekey::outcome::Holder;
use work::rekey::outcome::NotRekeyed;
use work::rekey::partly_applied::partly_applied;
use work::rekey::run_plan::InProgressPlan;
use work::rekey::run_plan::RunPlan;
use work::rekey::run_plan::UnreadablePlan;
use work::rekey::write_set::Base;
use work::rekey::write_set::Unreverted;
use work::rekey::write_set::WriteSet;
use work::work_item_files::identity_of;
use work::work_item_files::WorkItemFiles;

use crate::identity_run::begin_rekey_run;
use crate::identity_run::IdentityRunPort;
use crate::rekey::execute::current_states;
use crate::rekey::execute::execute;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RekeyAction {
    Apply,
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

    /// The evidence of a half-applied entry an unreadable plan's discard
    /// shows before it refuses or sets the plan aside.
    fn partly_applied(&self, items: &[PathBuf], recoveries: &[PathBuf]);
}

#[derive(Clone, Copy)]
pub struct RekeyPorts<'a> {
    pub identity_run: &'a dyn IdentityRunPort,
    pub store: &'a dyn RunPlanStore,
    pub corpus: RekeyCorpus<'a>,
    pub items: &'a dyn WorkItemFiles,
    pub sync: SyncState<'a>,
    pub repository: &'a dyn RepositoryProbe,
    /// Repo-relative prefixes whose dirty paths refuse an apply.
    pub scopes: &'a [String],
    pub progress: &'a dyn RekeyProgress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RekeyReport {
    AlreadyConverged,
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
        RekeyAction::Discard => discard(loaded, request.forcing, ports),
    }
}

fn apply(
    loaded: Result<Option<RunPlan>, UnreadablePlan>,
    request: &RekeyRequest,
    ports: &RekeyPorts<'_>,
    lock: &RetirementLockGuard,
) -> Result<RekeyReport, NotRekeyed> {
    match loaded.map_err(NotRekeyed::PlanUnreadable)? {
        Some(RunPlan::InProgress(plan)) => {
            let base = working_copy(request.forcing, ports, plan.write_set())?;
            finish(plan, base.as_ref(), ports, lock)?;
            Ok(RekeyReport::Rekeyed { resumed: true })
        }
        Some(RunPlan::Proposed(_)) | None => fresh(request, ports, lock),
    }
}

fn fresh(
    request: &RekeyRequest,
    ports: &RekeyPorts<'_>,
    lock: &RetirementLockGuard,
) -> Result<RekeyReport, NotRekeyed> {
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
    let entries = numeric_mapping(&request.scheme, &candidates)?;
    if entries.is_empty() {
        return Ok(RekeyReport::AlreadyConverged);
    }
    let pushes = pending_pushes(&ports.sync, &entries, &items);
    if !pushes.is_empty() {
        return Err(NotRekeyed::PendingPush(pushes));
    }
    let base = working_copy(request.forcing, ports, &WriteSet::default())?;
    let recorded = write_set(&entries, &ports.corpus, base.as_ref())?;
    let plan = InProgressPlan::new(
        IdentityFingerprint::of(&request.scheme, &items),
        entries,
        recorded,
    );
    ports.store.save(&RunPlan::InProgress(plan.clone()))?;
    finish(plan, base.as_ref(), ports, lock)?;
    Ok(RekeyReport::Rekeyed { resumed: false })
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
