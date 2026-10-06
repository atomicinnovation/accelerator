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
use research_adapters::pacing::FilePacingGate;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use support::millis;
use support::secs;
use support::RecordedDiagnostics;
use support::RecordingClock;
use support::Scratch;

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
        FilePacingGate::new(
            self.scratch.dir(),
            self.clock.clone(),
            self.diagnostics.clone(),
        )
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
        let millis = |at: Option<SystemTime>| {
            at.map_or_else(|| "null".to_owned(), |at| millis(at).to_string())
        };
        self.scratch.write(
            "arxiv-pacing",
            &format!(
                "{{\"last_sent_ms\":{},\"last_finish_ms\":{},\
                 \"not_before_ms\":{}}}",
                millis(last_sent),
                millis(last_finish),
                millis(not_before)
            ),
        );
    }

    fn stored_last_sent_ms(&self) -> Option<u64> {
        let state: serde_json::Value = serde_json::from_str(
            &self.scratch.read("arxiv-pacing").expect("a pacing state"),
        )
        .expect("JSON pacing state");
        state["last_sent_ms"].as_u64()
    }

    fn hold_lock(&self) -> File {
        let lock = self.scratch.open("arxiv.lock");
        flock(&lock, FlockOperation::NonBlockingLockExclusive)
            .expect("the test holds the lock");
        lock
    }

    fn lock_is_free(&self) -> bool {
        let lock = self.scratch.open("arxiv.lock");
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
    harness.scratch.write("arxiv-pacing", "{not json");

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
    harness.scratch.mkdir("arxiv-pacing");
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
        reported.iter().all(|line| line.contains("arxiv-pacing")),
        "{reported:?}"
    );
}

#[test]
fn every_request_sent_appends_one_line_to_the_request_log() {
    let harness = Harness::new();

    harness.pass(None).expect("admitted");
    harness.pass(None).expect("admitted");

    assert_eq!(harness.scratch.lines("arxiv-requests.log").len(), 2);
    assert!(harness.scratch.lines("arxiv-contention.log").is_empty());
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
fn spacing_runs_from_a_send_whose_finish_was_never_recorded() {
    let harness = Harness::new();
    let now = harness.clock.wall_now();
    harness.store_sent_state(Some(now - secs(1)), Some(now - secs(10)), None);

    harness.pass(None).expect("admitted");

    assert_eq!(harness.clock.slept(), [secs(2)]);
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
fn an_in_lock_wait_is_spent_from_the_callers_deadline() {
    let harness = Harness::new();
    harness.store_state(None, Some(harness.clock.wall_now() + secs(12)));
    let deadline = harness.deadline(secs(100));

    harness
        .paced(&mut || Attempted { defer_until: None }, &deadline)
        .expect("admitted");

    assert_eq!(deadline.remaining(harness.clock.now()), secs(88));
}
