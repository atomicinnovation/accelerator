//! The identity run: the one lock every command that retires or allocates
//! work-item IDs holds for its whole run, and the run-plan gate read under
//! it.

use std::time::Duration;

use crate::rekey::outcome::Holder;
use crate::rekey::outcome::NotRekeyed;
use crate::rekey::run_plan::UnreadablePlan;
use corpus::StoreError;

/// Short enough that a skill's shell call sees a refusal code rather than
/// its own tool timeout.
pub const IDENTITY_RUN_CEILING: Duration = Duration::from_secs(60);

const REKEY: &str = "rekey";

/// The identity-run lock as taken; released when dropped.
pub struct HeldIdentityRun {
    _held: Box<dyn Send + Sync>,
}

impl HeldIdentityRun {
    #[must_use]
    pub fn new(guard: impl Send + Sync + 'static) -> Self {
        Self {
            _held: Box::new(guard),
        }
    }
}

/// Proof that the identity-run lock is held and the run plan was found
/// clear under it.
///
/// Only [`begin_identity_run`] and [`begin_rekey_run`] mint one:
///
/// ```compile_fail
/// use work::identity_run::HeldIdentityRun;
/// use work::identity_run::IdentityRunGuard;
///
/// let _ = IdentityRunGuard::new(HeldIdentityRun::new(()));
/// ```
pub struct IdentityRunGuard {
    _held: HeldIdentityRun,
}

impl IdentityRunGuard {
    const fn new(held: HeldIdentityRun) -> Self {
        Self { _held: held }
    }
}

pub enum IdentityRunAttempt {
    Acquired(HeldIdentityRun),
    HeldBy(Option<Holder>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityRunLockError {
    Timeout(Option<Holder>),
    Lock(StoreError),
}

pub trait IdentityRun {
    /// Takes the lock if it is free, without waiting for it.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the lock cannot be taken for any reason but
    /// another holder.
    fn try_acquire(
        &self,
        command: &str,
    ) -> Result<IdentityRunAttempt, StoreError>;

    /// Waits up to `ceiling` for the lock.
    ///
    /// # Errors
    ///
    /// [`IdentityRunLockError::Timeout`] naming the holder when it is still
    /// held at the ceiling, and [`IdentityRunLockError::Lock`] otherwise.
    fn acquire_within(
        &self,
        command: &str,
        ceiling: Duration,
    ) -> Result<HeldIdentityRun, IdentityRunLockError>;
}

/// What a re-key run plan means for every other identity command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunPlanGate {
    Clear,
    InProgress,
    Unreadable(UnreadablePlan),
}

pub trait RunPlanStatus {
    fn gate(&self) -> RunPlanGate;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityRunRefusal {
    Busy(Option<Holder>),
    Lock(StoreError),
    RekeyInProgress,
    RekeyRunning(Holder),
    RekeyPlanUnreadable(UnreadablePlan),
}

impl From<IdentityRunRefusal> for NotRekeyed {
    fn from(refusal: IdentityRunRefusal) -> Self {
        match refusal {
            IdentityRunRefusal::Busy(holder) => Self::IdentityRunBusy(holder),
            IdentityRunRefusal::Lock(error) => {
                Self::StateUnwritable(error.to_string())
            }
            IdentityRunRefusal::RekeyInProgress => Self::InProgress(None),
            IdentityRunRefusal::RekeyRunning(holder) => {
                Self::InProgress(Some(holder))
            }
            IdentityRunRefusal::RekeyPlanUnreadable(plan) => {
                Self::PlanUnreadable(plan)
            }
        }
    }
}

/// The stderr line a command prints before it waits for `holder`.
#[must_use]
pub fn waiting_line(holder: Option<&Holder>) -> String {
    match holder {
        Some(Holder {
            pid,
            command: Some(command),
        }) => {
            format!(
                "waiting for accelerator work {command} (pid {pid}) to finish"
            )
        }
        _ => "waiting for another accelerator command to finish".to_owned(),
    }
}

fn acquire(
    lock: &dyn IdentityRun,
    command: &str,
    waiting: &dyn Fn(Option<&Holder>),
) -> Result<HeldIdentityRun, IdentityRunLockError> {
    match lock
        .try_acquire(command)
        .map_err(IdentityRunLockError::Lock)?
    {
        IdentityRunAttempt::Acquired(held) => Ok(held),
        IdentityRunAttempt::HeldBy(holder) => {
            waiting(holder.as_ref());
            lock.acquire_within(command, IDENTITY_RUN_CEILING)
        }
    }
}

fn refused_by(gate: RunPlanGate) -> Option<IdentityRunRefusal> {
    match gate {
        RunPlanGate::Clear => None,
        RunPlanGate::InProgress => Some(IdentityRunRefusal::RekeyInProgress),
        RunPlanGate::Unreadable(plan) => {
            Some(IdentityRunRefusal::RekeyPlanUnreadable(plan))
        }
    }
}

fn timed_out(gate: RunPlanGate, holder: Option<Holder>) -> IdentityRunRefusal {
    match (gate, holder) {
        (RunPlanGate::InProgress, Some(holder))
            if holder.command.as_deref() == Some(REKEY) =>
        {
            IdentityRunRefusal::RekeyRunning(holder)
        }
        (gate, holder) => {
            refused_by(gate).unwrap_or(IdentityRunRefusal::Busy(holder))
        }
    }
}

/// Begins `command`'s identity run: takes the lock, waiting for it when
/// held, then refuses while a re-key is in progress or unreadable.
///
/// # Errors
///
/// [`IdentityRunRefusal`] when the gate refuses, the wait times out, or the
/// lock cannot be taken; nothing has been read or written.
pub fn begin_identity_run(
    lock: &dyn IdentityRun,
    plan: &dyn RunPlanStatus,
    command: &str,
    waiting: &dyn Fn(Option<&Holder>),
) -> Result<IdentityRunGuard, IdentityRunRefusal> {
    let held =
        acquire(lock, command, waiting).map_err(|error| match error {
            IdentityRunLockError::Timeout(holder) => {
                timed_out(plan.gate(), holder)
            }
            IdentityRunLockError::Lock(error) => {
                IdentityRunRefusal::Lock(error)
            }
        })?;
    refused_by(plan.gate()).map_or(Ok(IdentityRunGuard::new(held)), Err)
}

/// Begins a re-key's identity run; the re-key reads its own plan.
///
/// # Errors
///
/// [`NotRekeyed::IdentityRunBusy`] when the wait times out, and
/// [`NotRekeyed::StateUnwritable`] when the lock cannot be taken.
pub fn begin_rekey_run(
    lock: &dyn IdentityRun,
    waiting: &dyn Fn(Option<&Holder>),
) -> Result<IdentityRunGuard, NotRekeyed> {
    acquire(lock, REKEY, waiting)
        .map(IdentityRunGuard::new)
        .map_err(|error| match error {
            IdentityRunLockError::Timeout(holder) => {
                NotRekeyed::IdentityRunBusy(holder)
            }
            IdentityRunLockError::Lock(error) => {
                NotRekeyed::StateUnwritable(error.to_string())
            }
        })
}
