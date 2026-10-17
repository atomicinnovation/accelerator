//! Beginning an identity run: the lock first, then the run plan's gate, and
//! a refusal before anything is read or written.

use std::cell::Cell;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use corpus::StoreError;
use work::rekey::outcome::Holder;
use work::rekey::outcome::NotRekeyed;
use work::rekey::run_plan::Unreadability;
use work::rekey::run_plan::UnreadablePlan;
use work_adapters::identity_run::begin_identity_run;
use work_adapters::identity_run::begin_rekey_run;
use work_adapters::identity_run::waiting_line;
use work_adapters::identity_run::HeldIdentityRun;
use work_adapters::identity_run::IdentityRunAttempt;
use work_adapters::identity_run::IdentityRunLockError;
use work_adapters::identity_run::IdentityRunPort;
use work_adapters::identity_run::IdentityRunRefusal;
use work_adapters::identity_run::RunPlanGate;
use work_adapters::identity_run::RunPlanStatus;
use work_adapters::identity_run::IDENTITY_RUN_CEILING;

type Events = Rc<RefCell<Vec<String>>>;

struct Released(Arc<AtomicBool>);

impl Drop for Released {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

enum Contention {
    Free,
    HeldThenFree(Option<Holder>),
    HeldThroughout(Option<Holder>),
    Broken,
}

struct FakeLock {
    contention: Contention,
    events: Events,
    released: Arc<AtomicBool>,
    ceiling: Cell<Option<Duration>>,
}

impl FakeLock {
    fn new(contention: Contention, events: &Events) -> Self {
        Self {
            contention,
            events: Rc::clone(events),
            released: Arc::new(AtomicBool::new(false)),
            ceiling: Cell::new(None),
        }
    }

    fn held(&self) -> HeldIdentityRun {
        HeldIdentityRun::new(Released(Arc::clone(&self.released)))
    }
}

fn broken() -> StoreError {
    StoreError::Io {
        path: ".accelerator/state/locks/identity-run.lockdir".to_owned(),
        detail: "permission denied".to_owned(),
    }
}

impl IdentityRunPort for FakeLock {
    fn try_acquire(
        &self,
        command: &str,
    ) -> Result<IdentityRunAttempt, StoreError> {
        self.events.borrow_mut().push(format!("try {command}"));
        match &self.contention {
            Contention::Free => Ok(IdentityRunAttempt::Acquired(self.held())),
            Contention::HeldThenFree(holder)
            | Contention::HeldThroughout(holder) => {
                Ok(IdentityRunAttempt::HeldBy(holder.clone()))
            }
            Contention::Broken => Err(broken()),
        }
    }

    fn acquire_within(
        &self,
        command: &str,
        ceiling: Duration,
    ) -> Result<HeldIdentityRun, IdentityRunLockError> {
        self.events.borrow_mut().push(format!("wait {command}"));
        self.ceiling.set(Some(ceiling));
        match &self.contention {
            Contention::HeldThroughout(holder) => {
                Err(IdentityRunLockError::Timeout(holder.clone()))
            }
            Contention::Broken => Err(IdentityRunLockError::Lock(broken())),
            Contention::Free | Contention::HeldThenFree(_) => Ok(self.held()),
        }
    }
}

struct FakePlan {
    gate: RunPlanGate,
    events: Events,
}

impl RunPlanStatus for FakePlan {
    fn gate(&self) -> RunPlanGate {
        self.events.borrow_mut().push("gate".to_owned());
        self.gate.clone()
    }
}

fn plan(gate: RunPlanGate, events: &Events) -> FakePlan {
    FakePlan {
        gate,
        events: Rc::clone(events),
    }
}

fn unreadable() -> UnreadablePlan {
    UnreadablePlan {
        path: PathBuf::from(".accelerator/state/rekey/run-plan.json"),
        cause: Unreadability::Malformed("expected value".to_owned()),
    }
}

fn holder(command: Option<&str>) -> Holder {
    Holder {
        pid: 4242,
        command: command.map(str::to_owned),
    }
}

const fn no_wait(_: Option<&Holder>) {}

#[test]
fn identity_run_is_acquired_before_the_plan_status_is_read() {
    let events = Events::default();
    let lock = FakeLock::new(Contention::Free, &events);

    let begun = begin_identity_run(
        &lock,
        &plan(RunPlanGate::Clear, &events),
        "sync",
        &no_wait,
    );

    assert!(begun.is_ok());
    assert_eq!(*events.borrow(), vec!["try sync", "gate"]);
}

#[test]
fn a_non_rekey_command_is_refused_while_a_plan_is_in_progress() {
    let events = Events::default();
    let lock = FakeLock::new(Contention::Free, &events);

    let begun = begin_identity_run(
        &lock,
        &plan(RunPlanGate::InProgress, &events),
        "create",
        &no_wait,
    );

    assert!(matches!(begun, Err(IdentityRunRefusal::RekeyInProgress)));
    assert!(lock.released.load(Ordering::SeqCst));
}

#[test]
fn a_non_rekey_command_is_refused_while_a_plan_is_unreadable() {
    let events = Events::default();
    let lock = FakeLock::new(Contention::Free, &events);

    let begun = begin_identity_run(
        &lock,
        &plan(RunPlanGate::Unreadable(unreadable()), &events),
        "promote",
        &no_wait,
    );

    assert!(matches!(
        begun,
        Err(IdentityRunRefusal::RekeyPlanUnreadable(found)) if found == unreadable()
    ));
}

#[test]
fn a_non_rekey_command_begins_with_a_proposed_plan() {
    let events = Events::default();
    let lock = FakeLock::new(Contention::Free, &events);

    let begun = begin_identity_run(
        &lock,
        &plan(RunPlanGate::Clear, &events),
        "next-number",
        &no_wait,
    );

    assert!(begun.is_ok());
}

#[test]
fn a_held_lock_prints_a_waiting_line_naming_its_holder() {
    for (contended_by, line) in [
        (
            Some(holder(Some("sync"))),
            "waiting for accelerator work sync (pid 4242) to finish",
        ),
        (None, "waiting for another accelerator command to finish"),
    ] {
        let events = Events::default();
        let lock = FakeLock::new(
            Contention::HeldThenFree(contended_by.clone()),
            &events,
        );
        let printed = RefCell::new(Vec::new());

        let begun = begin_identity_run(
            &lock,
            &plan(RunPlanGate::Clear, &events),
            "create",
            &|holder| printed.borrow_mut().push(waiting_line(holder)),
        );

        assert!(begun.is_ok());
        assert_eq!(*printed.borrow(), vec![line.to_owned()]);
        assert_eq!(lock.ceiling.get(), Some(IDENTITY_RUN_CEILING));
    }
}

#[test]
fn a_timeout_with_a_plan_in_progress_is_rekey_in_progress() {
    let events = Events::default();
    let lock = FakeLock::new(
        Contention::HeldThroughout(Some(holder(Some("sync")))),
        &events,
    );

    let begun = begin_identity_run(
        &lock,
        &plan(RunPlanGate::InProgress, &events),
        "create",
        &no_wait,
    );

    assert!(matches!(begun, Err(IdentityRunRefusal::RekeyInProgress)));
}

#[test]
fn a_timeout_behind_a_running_rekey_says_the_rekey_is_running() {
    let events = Events::default();
    let rekey = holder(Some("rekey"));
    let lock =
        FakeLock::new(Contention::HeldThroughout(Some(rekey.clone())), &events);

    let begun = begin_identity_run(
        &lock,
        &plan(RunPlanGate::InProgress, &events),
        "sync",
        &no_wait,
    );

    let Err(refusal) = begun else {
        unreachable!("a running re-key refuses");
    };
    assert_eq!(
        NotRekeyed::from(refusal),
        NotRekeyed::InProgress(Some(rekey))
    );
}

#[test]
fn a_timeout_with_an_unreadable_plan_is_plan_unreadable() {
    let events = Events::default();
    let lock = FakeLock::new(Contention::HeldThroughout(None), &events);

    let begun = begin_identity_run(
        &lock,
        &plan(RunPlanGate::Unreadable(unreadable()), &events),
        "sync",
        &no_wait,
    );

    assert!(matches!(
        begun,
        Err(IdentityRunRefusal::RekeyPlanUnreadable(_))
    ));
}

#[test]
fn a_timeout_without_a_plan_is_identity_run_busy() {
    let events = Events::default();
    let sync = holder(Some("sync"));
    let lock =
        FakeLock::new(Contention::HeldThroughout(Some(sync.clone())), &events);

    let begun = begin_identity_run(
        &lock,
        &plan(RunPlanGate::Clear, &events),
        "create",
        &no_wait,
    );

    let Err(refusal) = begun else {
        unreachable!("a held lock refuses");
    };
    assert_eq!(
        NotRekeyed::from(refusal),
        NotRekeyed::IdentityRunBusy(Some(sync))
    );
}

#[test]
fn a_lock_io_failure_is_not_reported_as_busy() {
    let events = Events::default();
    let lock = FakeLock::new(Contention::Broken, &events);

    let begun = begin_identity_run(
        &lock,
        &plan(RunPlanGate::Clear, &events),
        "sync",
        &no_wait,
    );
    assert!(matches!(begun, Err(IdentityRunRefusal::Lock(_))));

    let rekey = begin_rekey_run(&lock, &no_wait);
    assert!(matches!(rekey, Err(NotRekeyed::StateUnwritable(_))));
}

#[test]
fn a_rekey_run_times_out_as_identity_run_busy() {
    let events = Events::default();
    let sync = holder(Some("sync"));
    let lock =
        FakeLock::new(Contention::HeldThroughout(Some(sync.clone())), &events);

    let begun = begin_rekey_run(&lock, &no_wait);

    assert!(matches!(
        begun,
        Err(NotRekeyed::IdentityRunBusy(Some(found))) if found == sync
    ));
}

#[test]
fn a_rekey_run_reads_no_plan_gate() {
    let events = Events::default();
    let lock = FakeLock::new(Contention::Free, &events);

    let begun = begin_rekey_run(&lock, &no_wait);

    assert!(begun.is_ok());
    assert_eq!(*events.borrow(), vec!["try rekey"]);
}

#[test]
fn the_lock_is_released_when_the_guard_drops() {
    let events = Events::default();
    let lock = FakeLock::new(Contention::Free, &events);

    let guard = begin_identity_run(
        &lock,
        &plan(RunPlanGate::Clear, &events),
        "sync",
        &no_wait,
    );
    assert!(!lock.released.load(Ordering::SeqCst));

    drop(guard);
    assert!(lock.released.load(Ordering::SeqCst));
}
