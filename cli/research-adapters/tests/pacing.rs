#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::cell::Cell;
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use std::time::Duration;
use std::time::SystemTime;

use research::sources::fetch::Attempted;
use research::sources::fetch::Clock as _;
use research::sources::fetch::PacingGate as _;
use research::sources::fetch::WaitTooLong;
use research::sources::schedule::Deadline;
use research::sources::service::CrossrefPool;
use research::sources::service::Service;
use research_adapters::diagnostics::Diagnostics;
use research_adapters::pacing::FilePacingGate;
use research_adapters::scratch::ScratchDir;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use support::arxiv_gate;
use support::gate_with_companions;
use support::millis;
use support::secs;
use support::service_gate;
use support::write_pacing_at;
use support::HookedClock;
use support::LegacyFile;
use support::PacingRecord;
use support::RecordedDiagnostics;
use support::RecordingClock;
use support::Scratch;
use support::Silent;
use support::StateFile;

struct Harness {
    scratch: Scratch,
    clock: Rc<RecordingClock>,
    diagnostics: Rc<RecordedDiagnostics>,
}

impl Harness {
    fn new() -> Self {
        Self {
            scratch: Scratch::new(),
            clock: RecordingClock::new(),
            diagnostics: Rc::default(),
        }
    }

    fn gate(&self) -> FilePacingGate {
        arxiv_gate(&self.scratch, self.clock.clone(), self.diagnostics.clone())
    }

    fn deadline(&self, total: Duration) -> Deadline {
        Deadline::starting(self.clock.now(), total, secs(30))
    }

    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        deadline: &Deadline,
    ) -> Result<(), WaitTooLong> {
        let gate = self.gate();
        let turn = gate.try_serve().expect("the lock is free");
        turn.paced(attempt, deadline)
    }

    fn pass(&self, defer_by: Option<Duration>) -> Result<(), WaitTooLong> {
        let clock = self.clock.clone();
        self.paced(
            &mut || Attempted {
                defer_until: defer_by.map(|by| clock.wall_now() + by),
            },
            &self.deadline(secs(100)),
        )
    }

    fn store_state(
        &self,
        last_finish: Option<SystemTime>,
        not_before: Option<SystemTime>,
    ) {
        self.store_sent_state(None, last_finish, not_before);
    }

    fn store_sent_state(
        &self,
        last_sent: Option<SystemTime>,
        last_finish: Option<SystemTime>,
        not_before: Option<SystemTime>,
    ) {
        self.scratch.write_pacing(&PacingRecord {
            last_sent,
            last_finish,
            not_before,
            ..PacingRecord::default()
        });
    }

    fn stored_last_sent_ms(&self) -> Option<u64> {
        self.scratch.stored_pacing()["last_sent_ms"].as_u64()
    }

    fn state(&self, file: StateFile) -> std::path::PathBuf {
        self.scratch.arxiv_state(file)
    }

    fn hold_lock(&self) -> File {
        let lock = self.scratch.open(self.state(StateFile::ServingLock));
        flock(&lock, FlockOperation::NonBlockingLockExclusive)
            .expect("the test holds the lock");
        lock
    }

    fn lock_is_free(&self) -> bool {
        let lock = self.scratch.open(self.state(StateFile::ServingLock));
        flock(&lock, FlockOperation::NonBlockingLockExclusive).is_ok()
    }
}

#[test]
fn a_first_request_waits_for_nothing() {
    let harness = Harness::new();

    harness.pass(None).expect("admitted");

    assert!(harness.clock.slept().is_empty());
}

#[test]
fn a_request_is_spaced_three_seconds_after_the_last_finish() {
    let harness = Harness::new();
    harness.pass(None).expect("admitted");
    harness.clock.advance(secs(1));

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [secs(2)]);
}

#[test]
fn a_last_finish_in_the_future_waits_at_most_three_seconds() {
    let harness = Harness::new();
    harness.store_state(Some(harness.clock.wall_now() + secs(100)), None);

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [secs(3)]);
}

#[test]
fn a_corrupt_state_file_counts_as_no_previous_request() {
    let harness = Harness::new();
    harness
        .scratch
        .write(harness.state(StateFile::Pacing), "{not json");

    harness.pass(None).expect("admitted");

    assert!(harness.clock.slept().is_empty());
}

#[test]
fn a_deferral_is_persisted_for_the_next_gate_on_the_same_directory() {
    let harness = Harness::new();
    harness.pass(Some(secs(12))).expect("admitted");

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [secs(12)]);
}

#[test]
fn a_shorter_deferral_never_shortens_a_longer_one_already_stored() {
    let harness = Harness::new();
    let clock = harness.clock.clone();
    harness
        .paced(
            &mut || {
                harness.store_state(None, Some(clock.wall_now() + secs(12)));
                Attempted {
                    defer_until: Some(clock.wall_now() + secs(3)),
                }
            },
            &harness.deadline(secs(100)),
        )
        .expect("admitted");

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [secs(12)]);
}

#[test]
fn a_deferral_is_persisted_no_more_than_thirty_seconds_ahead() {
    let harness = Harness::new();
    harness.pass(Some(secs(45))).expect("admitted");

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [secs(30)]);
}

#[test]
fn a_stored_deferral_more_than_thirty_seconds_ahead_is_discarded() {
    let harness = Harness::new();
    harness.store_state(None, Some(harness.clock.wall_now() + secs(31)));

    harness.pass(None).expect("admitted");

    assert!(harness.clock.slept().is_empty());
}

#[test]
fn a_failed_state_write_is_reported_and_the_attempt_still_runs() {
    let harness = Harness::new();
    let pacing = harness.state(StateFile::Pacing);
    harness.scratch.mkdir(&pacing);
    let ran = Cell::new(false);

    harness
        .paced(
            &mut || {
                ran.set(true);
                Attempted { defer_until: None }
            },
            &harness.deadline(secs(100)),
        )
        .expect("admitted");

    assert!(ran.get());
    let reported = harness.diagnostics.lines();
    assert_eq!(reported.len(), 2, "{reported:?}");
    assert!(
        reported
            .iter()
            .all(|line| line.contains(&pacing.display().to_string())),
        "{reported:?}"
    );
}

#[test]
fn every_request_sent_appends_one_line_to_the_request_log() {
    let harness = Harness::new();

    harness.pass(None).expect("admitted");
    harness.pass(None).expect("admitted");

    let lines = |file| harness.scratch.lines(harness.state(file));
    assert_eq!(lines(StateFile::RequestLog).len(), 2);
    assert!(lines(StateFile::ContentionLog).is_empty());
}

#[test]
fn a_wait_the_deadline_cannot_admit_refuses_without_running() {
    let harness = Harness::new();
    harness.store_state(None, Some(harness.clock.wall_now() + secs(20)));
    let ran = Cell::new(false);
    let gate = harness.gate();
    let turn = gate.try_serve().expect("the lock is free");

    let refused = turn.paced(
        &mut || {
            ran.set(true);
            Attempted { defer_until: None }
        },
        &harness.deadline(secs(40)),
    );

    assert_eq!(refused, Err(WaitTooLong));
    assert!(!ran.get());
    assert!(harness.clock.slept().is_empty());
    assert!(!harness.lock_is_free());
    drop(turn);
    assert!(harness.lock_is_free());
}

#[test]
fn a_lock_that_cannot_open_still_serves_a_spaced_turn_and_reports_it() {
    let harness = Harness::new();
    let lock = harness.state(StateFile::ServingLock);
    harness.scratch.mkdir(&lock);
    harness.pass(None).expect("admitted");
    harness.clock.advance(secs(1));

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [secs(2)]);
    let reported = harness.diagnostics.lines();
    assert_eq!(reported.len(), 2, "{reported:?}");
    assert!(
        reported
            .iter()
            .all(|line| line.contains(&lock.display().to_string())
                && line.contains("pacing without the lock")),
        "{reported:?}"
    );
}

#[test]
fn try_serve_is_none_while_another_holder_has_the_lock() {
    let harness = Harness::new();
    let _held = harness.hold_lock();

    assert!(harness.gate().try_serve().is_none());
    assert!(harness.clock.slept().is_empty());
}

#[test]
fn a_turn_holds_the_lock_between_attempts_until_dropped() {
    let harness = Harness::new();
    let gate = harness.gate();
    let turn = gate.try_serve().expect("the lock is free");
    let observed_free = RefCell::new(Vec::new());
    let mut attempt = || {
        observed_free.borrow_mut().push(harness.lock_is_free());
        Attempted { defer_until: None }
    };

    turn.paced(&mut attempt, &harness.deadline(secs(100)))
        .expect("admitted");
    let free_between = harness.lock_is_free();
    turn.paced(&mut attempt, &harness.deadline(secs(100)))
        .expect("admitted");
    drop(turn);

    assert_eq!(*observed_free.borrow(), [false, false]);
    assert!(!free_between);
    assert!(harness.lock_is_free());
}

#[test]
fn dropping_a_turn_frees_the_lock_at_once() {
    let harness = Harness::new();
    let first = harness.gate();
    let second = harness.gate();
    let turn = first.try_serve().expect("the lock is free");
    assert!(second.try_serve().is_none());

    drop(turn);

    assert!(second.try_serve().is_some());
    assert!(harness.clock.slept().is_empty());
}

#[test]
fn spacing_runs_from_a_send_whose_finish_was_never_recorded_with_a_margin_for_its_arrival(
) {
    let harness = Harness::new();
    let now = harness.clock.wall_now();
    harness.store_sent_state(Some(now - secs(1)), Some(now - secs(10)), None);

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [Duration::from_millis(2100)]);
}

#[test]
fn a_send_stamped_ahead_of_now_waits_no_more_than_one_spaced_send() {
    let harness = Harness::new();
    let now = harness.clock.wall_now();
    harness.store_sent_state(Some(now + secs(60)), None, None);

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [Duration::from_millis(3100)]);
}

#[test]
fn spacing_runs_from_the_last_finish_when_it_follows_the_last_send() {
    let harness = Harness::new();
    let now = harness.clock.wall_now();
    harness.store_sent_state(Some(now - secs(10)), Some(now - secs(1)), None);

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [secs(2)]);
}

#[test]
fn last_sent_is_stored_before_the_attempt_runs() {
    let harness = Harness::new();
    harness.clock.advance(secs(7));
    let observed = Cell::new(None);

    harness
        .paced(
            &mut || {
                observed.set(harness.stored_last_sent_ms());
                Attempted { defer_until: None }
            },
            &harness.deadline(secs(100)),
        )
        .expect("admitted");

    let expected = u64::try_from(millis(harness.clock.wall_now()))
        .expect("milliseconds fit");
    assert_eq!(observed.get(), Some(expected));
}

#[test]
fn an_out_of_lock_wait_is_spent_from_the_callers_deadline() {
    let harness = Harness::new();
    harness.store_state(None, Some(harness.clock.wall_now() + secs(12)));
    let deadline = harness.deadline(secs(100));

    harness
        .paced(&mut || Attempted { defer_until: None }, &deadline)
        .expect("admitted");

    assert_eq!(deadline.remaining(harness.clock.now()), secs(88));
}

#[test]
fn a_corrupt_state_file_is_reported_once_per_paced_call() {
    let harness = Harness::new();
    harness
        .scratch
        .write(harness.state(StateFile::Pacing), "{not json");

    harness.pass(None).expect("admitted");

    let reported = harness.diagnostics.lines();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(reported[0].contains("pacing"), "{reported:?}");
}

const PUBLIC: Service = Service::Crossref(CrossrefPool::Public);
const POLITE: Service = Service::Crossref(CrossrefPool::Polite);

const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

fn ms_since_epoch(at: SystemTime) -> u64 {
    u64::try_from(millis(at)).expect("milliseconds fit")
}

/// A gate for any service on a clock other threads and hooks can share.
struct Pool {
    service: Service,
    scratch: Scratch,
    clock: HookedClock,
    diagnostics: Rc<RecordedDiagnostics>,
}

impl Pool {
    fn of(service: Service) -> Self {
        Self {
            service,
            scratch: Scratch::new(),
            clock: HookedClock::new(),
            diagnostics: Rc::default(),
        }
    }

    fn gate(&self) -> FilePacingGate {
        service_gate(
            self.service,
            &self.scratch.dir(),
            self.clock.shared(),
            self.diagnostics.clone(),
        )
    }

    fn deadline(&self, total: Duration, per_request: Duration) -> Deadline {
        Deadline::starting(self.clock.now(), total, per_request)
    }

    fn state(&self, file: StateFile) -> std::path::PathBuf {
        self.scratch.service_state(self.service, file)
    }

    fn store(&self, record: &PacingRecord) {
        self.scratch.write_service_pacing(self.service, record);
    }

    fn stored(&self, key: &str) -> Option<u64> {
        self.scratch.stored_service_pacing(self.service)[key].as_u64()
    }

    fn pacing_bytes(&self) -> Option<String> {
        self.scratch.read(self.state(StateFile::Pacing))
    }

    fn sends(&self) -> Vec<u64> {
        self.scratch
            .lines(self.state(StateFile::RequestLog))
            .iter()
            .map(|line| line.parse().expect("a send in milliseconds"))
            .collect()
    }

    fn pass(&self, deadline: &Deadline) -> Result<(), WaitTooLong> {
        let gate = self.gate();
        let turn = gate.try_serve().expect("a free slot");
        turn.paced(&mut || Attempted { defer_until: None }, deadline)
    }

    fn hold(&self, file: StateFile) -> File {
        let lock = self.scratch.open(self.state(file));
        flock(&lock, FlockOperation::NonBlockingLockExclusive)
            .expect("the test holds the lock");
        lock
    }

    fn is_free(&self, file: StateFile) -> bool {
        let lock = self.scratch.open(self.state(file));
        flock(&lock, FlockOperation::NonBlockingLockExclusive).is_ok()
    }

    /// What another process's gate needs to pace the same service.
    fn elsewhere(&self) -> Elsewhere {
        Elsewhere {
            service: self.service,
            research: self.scratch.dir(),
            clock: self.clock.clone(),
            pacing: self.state(StateFile::Pacing),
        }
    }
}

/// Everything a gate built inside another caller's sleep, or on another
/// thread, needs; all of it can cross threads.
#[derive(Clone)]
struct Elsewhere {
    service: Service,
    research: ScratchDir,
    clock: HookedClock,
    pacing: std::path::PathBuf,
}

impl Elsewhere {
    fn gate(&self, diagnostics: Rc<dyn Diagnostics>) -> FilePacingGate {
        service_gate(
            self.service,
            &self.research,
            self.clock.shared(),
            diagnostics,
        )
    }

    fn send_once(&self) {
        let gate = self.gate(Rc::new(Silent));
        let turn = gate.try_serve().expect("a free slot");
        let deadline =
            Deadline::starting(self.clock.now(), secs(100), secs(30));
        turn.paced(&mut || Attempted { defer_until: None }, &deadline)
            .expect("admitted");
    }

    fn store(&self, record: &PacingRecord) {
        write_pacing_at(&self.pacing, record);
    }
}

#[test]
fn with_three_slots_three_turns_are_served_and_a_fourth_is_not() {
    let pool = Pool::of(POLITE);
    let gate = pool.gate();

    let first = gate.try_serve().expect("slot one");
    let second = gate.try_serve().expect("slot two");
    let third = gate.try_serve().expect("slot three");
    assert!(gate.try_serve().is_none());

    drop(second);
    let again = gate.try_serve().expect("the freed slot");
    assert!(gate.try_serve().is_none());
    drop((first, third, again));
}

#[test]
fn try_serve_is_none_while_other_holders_have_every_slot() {
    let pool = Pool::of(POLITE);
    let _held = [0, 1, 2].map(|slot| pool.hold(StateFile::ServingSlot(slot)));

    assert!(pool.gate().try_serve().is_none());
    assert!(pool.clock.slept().is_empty());
}

#[test]
fn a_turn_holds_its_slot_between_attempts_until_dropped() {
    let pool = Pool::of(POLITE);
    let gate = pool.gate();
    let turn = gate.try_serve().expect("a free slot");
    let deadline = pool.deadline(secs(100), secs(30));

    turn.paced(&mut || Attempted { defer_until: None }, &deadline)
        .expect("admitted");
    let held_between = !pool.is_free(StateFile::ServingSlot(0));
    drop(turn);

    assert!(held_between);
    assert!(pool.is_free(StateFile::ServingSlot(0)));
}

#[test]
fn an_unlockable_filesystem_still_serves_a_turn_with_three_slots() {
    let pool = Pool::of(POLITE);
    let lock = pool.state(StateFile::ServingSlot(0));
    pool.scratch.mkdir(&lock);

    pool.pass(&pool.deadline(secs(100), secs(30)))
        .expect("admitted");

    let reported = pool.diagnostics.lines();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(
        reported[0].contains("pacing without the lock"),
        "{reported:?}"
    );
}

#[test]
fn a_start_spaced_send_waits_one_interval_from_the_previous_send() {
    let pool = Pool::of(PUBLIC);
    let now = pool.clock.current_wall();
    pool.store(&PacingRecord {
        last_sent: Some(now - ms(100)),
        last_finish: Some(now - ms(10)),
        ..PacingRecord::default()
    });

    pool.pass(&pool.deadline(secs(100), secs(30)))
        .expect("admitted");

    assert_eq!(pool.clock.slept(), [ms(110)]);
    assert_eq!(pool.sends(), [ms_since_epoch(now + ms(110))]);
}

#[test]
fn a_start_spaced_wait_is_spent_from_the_callers_deadline() {
    let pool = Pool::of(PUBLIC);
    let now = pool.clock.current_wall();
    pool.store(&PacingRecord {
        last_sent: Some(now - ms(100)),
        ..PacingRecord::default()
    });
    let deadline = pool.deadline(secs(100), secs(30));

    pool.pass(&deadline).expect("admitted");

    assert_eq!(deadline.remaining(pool.clock.now()), secs(100) - ms(110));
}

#[test]
fn a_caller_reserving_inside_another_s_sleep_gets_the_next_interval() {
    let pool = Pool::of(POLITE);
    let now = pool.clock.current_wall();
    pool.store(&PacingRecord {
        last_sent: Some(now - ms(50)),
        ..PacingRecord::default()
    });
    let other = pool.elsewhere();
    pool.clock.on_next_sleep(move || other.send_once());

    pool.pass(&pool.deadline(secs(100), secs(30)))
        .expect("admitted");

    let mut sends = pool.sends();
    sends.sort_unstable();
    assert_eq!(sends.len(), 2, "{sends:?}");
    assert_eq!(sends[0], ms_since_epoch(now + ms(52 + 102)));
    assert!(sends[1] - sends[0] >= 102, "{sends:?}");
}

#[test]
fn a_deferral_written_inside_a_callers_sleep_holds_it_until_it_ends() {
    let pool = Pool::of(PUBLIC);
    let now = pool.clock.current_wall();
    pool.store(&PacingRecord {
        last_sent: Some(now - ms(100)),
        ..PacingRecord::default()
    });
    let other = pool.elsewhere();
    let deferred_until = now + secs(2);
    pool.clock.on_next_sleep(move || {
        other.store(&PacingRecord {
            last_sent: Some(now - ms(100)),
            next_free: Some(now + ms(320)),
            not_before: Some(deferred_until),
            ..PacingRecord::default()
        });
    });

    pool.pass(&pool.deadline(secs(100), secs(30)))
        .expect("admitted");

    assert_eq!(pool.sends(), [ms_since_epoch(deferred_until)]);
}

#[test]
fn a_call_refused_before_reserving_writes_nothing() {
    let pool = Pool::of(PUBLIC);
    let now = pool.clock.current_wall();
    pool.store(&PacingRecord {
        not_before: Some(now + secs(20)),
        ..PacingRecord::default()
    });
    let before = pool.pacing_bytes();

    let refused = pool.pass(&pool.deadline(secs(40), secs(30)));

    assert_eq!(refused, Err(WaitTooLong));
    assert_eq!(pool.pacing_bytes(), before);
    assert!(pool.clock.slept().is_empty());
}

#[test]
fn a_call_refused_after_waking_hands_its_reservation_back() {
    let pool = Pool::of(PUBLIC);
    let now = pool.clock.current_wall();
    let send_at = now + ms(100);
    pool.store(&PacingRecord {
        last_sent: Some(now - ms(110)),
        ..PacingRecord::default()
    });
    let other = pool.elsewhere();
    pool.clock.on_next_sleep(move || {
        other.store(&PacingRecord {
            last_sent: Some(send_at),
            next_free: Some(send_at + ms(210)),
            ..PacingRecord::default()
        });
    });

    let refused = pool.pass(&pool.deadline(secs(1), ms(700)));

    assert_eq!(refused, Err(WaitTooLong));
    assert_eq!(pool.stored("next_free_ms"), Some(ms_since_epoch(send_at)));
    assert!(pool.sends().is_empty());
}

#[test]
fn now_is_read_only_once_the_pacing_lock_is_held() {
    let pool = Pool::of(PUBLIC);
    let elsewhere = pool.elsewhere();
    let pacing_lock = pool.hold(StateFile::PacingLock);
    let (read_wall, wall_was_read) = std::sync::mpsc::channel();
    pool.clock.on_next_wall_read(move || {
        let _ = read_wall.send(());
    });
    let caller = {
        let elsewhere = elsewhere.clone();
        std::thread::spawn(move || {
            elsewhere.send_once();
            elsewhere.clock.slept()
        })
    };

    let read_while_blocked =
        wall_was_read.recv_timeout(std::time::Duration::from_millis(200));
    pool.clock.advance(secs(1));
    let other_send = pool.clock.current_wall();
    elsewhere.store(&PacingRecord {
        last_sent: Some(other_send),
        ..PacingRecord::default()
    });
    drop(pacing_lock);
    let slept = caller.join().expect("the caller finishes");

    assert!(read_while_blocked.is_err(), "now was read before the lock");
    assert_eq!(slept, [ms(210)]);
    assert_eq!(pool.sends(), [ms_since_epoch(other_send + ms(210))]);
}

#[test]
fn a_pacing_file_of_another_shape_counts_as_no_previous_request() {
    for (shape, contents) in [
        (
            "another schema version",
            "{\"schema_version\":2,\"last_sent_ms\":1800000000000,\
             \"next_free_ms\":null,\"last_finish_ms\":null,\
             \"not_before_ms\":null}",
        ),
        (
            "no next_free_ms",
            "{\"schema_version\":1,\"last_sent_ms\":1800000000000,\
             \"last_finish_ms\":null,\"not_before_ms\":null}",
        ),
    ] {
        let pool = Pool::of(PUBLIC);
        pool.scratch.write(pool.state(StateFile::Pacing), contents);

        pool.pass(&pool.deadline(secs(100), secs(30)))
            .expect("admitted");

        assert!(pool.clock.slept().is_empty(), "{shape}");
        assert_eq!(pool.diagnostics.lines().len(), 1, "{shape}");
    }
}

/// An arXiv gate that also takes the previous release's lock.
struct Upgrading {
    pool: Pool,
}

impl Upgrading {
    fn new() -> Self {
        Self {
            pool: Pool::of(Service::Arxiv),
        }
    }

    fn legacy_lock(&self) -> std::path::PathBuf {
        self.pool.scratch.legacy_arxiv_state(LegacyFile::Lock)
    }

    fn gate(&self) -> FilePacingGate {
        gate_with_companions(
            Service::Arxiv,
            &self.pool.scratch.dir(),
            self.pool.clock.shared(),
            self.pool.diagnostics.clone(),
            vec![self.legacy_lock()],
        )
    }

    /// Holds the lock as an older release's arXiv call would.
    fn hold_as_older_release(&self) -> File {
        let lock = self.pool.scratch.open(self.legacy_lock());
        flock(&lock, FlockOperation::NonBlockingLockExclusive)
            .expect("the older release holds arxiv.lock");
        lock
    }

    fn older_release_can_lock(&self) -> bool {
        let lock = self.pool.scratch.open(self.legacy_lock());
        flock(&lock, FlockOperation::NonBlockingLockExclusive).is_ok()
    }
}

#[test]
fn an_older_release_holding_its_lock_keeps_a_new_release_call_waiting() {
    let upgrading = Upgrading::new();
    let _older = upgrading.hold_as_older_release();

    assert!(upgrading.gate().try_serve().is_none());
    assert!(upgrading.pool.is_free(StateFile::ServingLock));
    assert!(upgrading.pool.diagnostics.lines().is_empty());
}

#[test]
fn a_new_release_turn_keeps_an_older_release_call_waiting() {
    let upgrading = Upgrading::new();
    let gate = upgrading.gate();

    let turn = gate.try_serve().expect("a free slot");
    let older_blocked = !upgrading.older_release_can_lock();
    drop(turn);

    assert!(older_blocked);
    assert!(upgrading.older_release_can_lock());
}

#[test]
fn an_absent_companion_lock_is_created_without_a_diagnostic() {
    let upgrading = Upgrading::new();

    let turn = upgrading.gate().try_serve().map(drop);

    assert!(turn.is_some());
    assert!(upgrading.legacy_lock().is_file());
    assert!(upgrading.pool.diagnostics.lines().is_empty());
}

#[test]
fn a_companion_lock_that_cannot_open_still_serves_with_one_diagnostic() {
    let upgrading = Upgrading::new();
    upgrading.pool.scratch.mkdir(upgrading.legacy_lock());

    let gate = upgrading.gate();
    let turn = gate.try_serve();

    assert!(turn.is_some());
    let reported = upgrading.pool.diagnostics.lines();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(
        reported[0].contains(&upgrading.legacy_lock().display().to_string()),
        "{reported:?}"
    );
}

#[test]
fn a_degraded_slot_lock_still_waits_for_a_busy_companion() {
    let upgrading = Upgrading::new();
    upgrading
        .pool
        .scratch
        .mkdir(upgrading.pool.state(StateFile::ServingLock));
    let _older = upgrading.hold_as_older_release();

    assert!(upgrading.gate().try_serve().is_none());
}
