#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::cell::Cell;
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use std::time::Duration;
use std::time::SystemTime;

use research::classify::Reason;
use research::fetch::Attempted;
use research::fetch::Cause;
use research::fetch::Clock as _;
use research::fetch::PacingGate as _;
use research::fetch::Unavailable;
use research::schedule::Deadline;
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

    fn pass(&self, defer_by: Option<Duration>) -> Result<(), Unavailable> {
        let clock = self.clock.clone();
        self.gate().paced(
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
        let millis = |at: Option<SystemTime>| {
            at.map_or_else(|| "null".to_owned(), |at| millis(at).to_string())
        };
        self.scratch.write(
            "arxiv-pacing",
            &format!(
                "{{\"last_finish_ms\":{},\"not_before_ms\":{}}}",
                millis(last_finish),
                millis(not_before)
            ),
        );
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
        .gate()
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
        .gate()
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
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(reported[0].contains("arxiv-pacing"), "{reported:?}");
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
fn a_wait_the_deadline_cannot_admit_refuses_and_releases_the_lock() {
    let harness = Harness::new();
    harness.store_state(None, Some(harness.clock.wall_now() + secs(20)));
    let ran = Cell::new(false);

    let refused = harness.gate().paced(
        &mut || {
            ran.set(true);
            Attempted { defer_until: None }
        },
        &harness.deadline(secs(40)),
    );

    assert_eq!(refused, Err(Unavailable::new(Reason::RateLimited)));
    assert!(!ran.get());
    assert!(harness.clock.slept().is_empty());
    assert!(harness.lock_is_free());
}

#[test]
fn a_lock_held_past_the_deadline_ends_in_lock_contention() {
    let harness = Harness::new();
    let _held = harness.hold_lock();
    let ran = Cell::new(false);

    let refused = harness.gate().paced(
        &mut || {
            ran.set(true);
            Attempted { defer_until: None }
        },
        &harness.deadline(secs(100)),
    );

    let unavailable = refused.expect_err("refused");
    assert_eq!(unavailable.reason(), Reason::RateLimited);
    assert_eq!(unavailable.cause(), Some(Cause::LockContention));
    assert!(!ran.get());
    assert_eq!(harness.scratch.lines("arxiv-contention.log").len(), 1);
    assert!(harness.scratch.lines("arxiv-requests.log").is_empty());
    assert!(harness
        .diagnostics
        .lines()
        .iter()
        .any(|line| line.contains("lock contention")));
    assert!(harness
        .clock
        .slept()
        .iter()
        .all(|poll| *poll == millis_duration(100)));
}

#[test]
fn the_lock_is_held_for_as_long_as_the_attempt_runs() {
    let harness = Harness::new();
    let observed_free = RefCell::new(None);

    harness
        .gate()
        .paced(
            &mut || {
                *observed_free.borrow_mut() = Some(harness.lock_is_free());
                Attempted { defer_until: None }
            },
            &harness.deadline(secs(100)),
        )
        .expect("admitted");

    assert_eq!(*observed_free.borrow(), Some(false));
    assert!(harness.lock_is_free());
}

#[test]
fn an_in_lock_wait_is_spent_from_the_callers_deadline() {
    let harness = Harness::new();
    harness.store_state(None, Some(harness.clock.wall_now() + secs(12)));
    let deadline = harness.deadline(secs(100));

    harness
        .gate()
        .paced(&mut || Attempted { defer_until: None }, &deadline)
        .expect("admitted");

    assert_eq!(deadline.remaining(harness.clock.now()), secs(88));
}

const fn millis_duration(millis: u64) -> Duration {
    Duration::from_millis(millis)
}
