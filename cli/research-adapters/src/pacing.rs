//! Pacing gates: none for sources that meter by budget, and a file-locked
//! one for each queued service, paced as that service asks.

use std::cell::Cell;
use std::fs::File;
use std::rc::Rc;
use std::time::Duration;
use std::time::SystemTime;

use research::sources::fetch::Attempted;
use research::sources::fetch::Clock;
use research::sources::fetch::PacingGate;
use research::sources::fetch::ServingTurn;
use research::sources::fetch::WaitTooLong;
use research::sources::pacing::Confirmation;
use research::sources::pacing::Pacing;
use research::sources::pacing::PacingState;
use research::sources::pacing::Reservation;
use research::sources::schedule::Deadline;
use research::sources::service::Service;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use rustix::io::Errno;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;

use crate::clock::millis_since_epoch;
use crate::diagnostics::Diagnostics;
use crate::scratch::ScratchDir;

/// Serves every call at once. OpenAlex meters callers by budget rather than
/// by request spacing, so there is nothing to hold or defer.
pub struct NoPacing;

impl PacingGate for NoPacing {
    fn pacing(&self) -> Pacing {
        Pacing::UNPACED
    }

    fn try_serve(&self) -> Option<Box<dyn ServingTurn + '_>> {
        Some(Box::new(Unpaced))
    }
}

struct Unpaced;

impl ServingTurn for Unpaced {
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        _deadline: &Deadline,
    ) -> Result<(), WaitTooLong> {
        attempt();
        Ok(())
    }
}

const PACING_LOCK: &str = "pacing.lock";
const STATE: &str = "pacing";
const REQUEST_LOG: &str = "requests.log";
const SCHEMA_VERSION: u32 = 1;

fn serving_lock(slot: u8) -> String {
    format!("serving-{slot}.lock")
}

/// Paces one service's calls across the project.
///
/// Each call's turn holds one slot lock from its first request to its last.
/// The pacing state is read and written only under the short `pacing.lock`.
///
/// Every send is reserved, slept towards outside the lock, then confirmed
/// against the last actual send, so spacing holds between real sends. Its
/// waits run on the caller's clock, so time spent here is spent from the
/// same deadline the caller measures.
pub struct FilePacingGate {
    service: Service,
    scratch: ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
}

impl FilePacingGate {
    pub fn new(
        service: Service,
        research: &ScratchDir,
        clock: Rc<dyn Clock>,
        diagnostics: Rc<dyn Diagnostics>,
    ) -> Self {
        Self {
            service,
            scratch: research.service_state(service),
            clock,
            diagnostics,
        }
    }

    fn report(&self, detail: &str) {
        self.diagnostics.report(&format!(
            "research fetch: {} {detail}",
            self.service.code()
        ));
    }

    fn turn(&self, slot: Option<File>) -> Box<dyn ServingTurn + '_> {
        Box::new(Turn {
            gate: self,
            _slot: slot,
        })
    }

    fn unlocked_turn(&self, detail: &str) -> Box<dyn ServingTurn + '_> {
        self.report(&format!("{detail}; pacing without the lock"));
        self.turn(None)
    }

    fn log_request(&self, at: SystemTime) {
        let line = millis_since_epoch(at).to_string();
        if let Err(error) = self.scratch.append_line(REQUEST_LOG, &line) {
            self.report(&error);
        }
    }
}

impl PacingGate for FilePacingGate {
    fn pacing(&self) -> Pacing {
        self.service.pacing()
    }

    /// Takes the first free slot. A filesystem that cannot lock still gets
    /// a turn, so pacing still spaces this call's own requests.
    fn try_serve(&self) -> Option<Box<dyn ServingTurn + '_>> {
        for slot in 0..self.service.pacing().slots().get() {
            let name = serving_lock(slot);
            let lock = match self.scratch.open(&name) {
                Ok(lock) => lock,
                Err(error) => return Some(self.unlocked_turn(&error)),
            };
            match flock(&lock, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => return Some(self.turn(Some(lock))),
                Err(Errno::WOULDBLOCK) => {}
                Err(errno) => {
                    return Some(self.unlocked_turn(&format!(
                        "could not lock {}: {errno}",
                        self.scratch.path(&name).display()
                    )))
                }
            }
        }
        None
    }
}

struct Turn<'g> {
    gate: &'g FilePacingGate,
    _slot: Option<File>,
}

impl ServingTurn for Turn<'_> {
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        deadline: &Deadline,
    ) -> Result<(), WaitTooLong> {
        let call = PacedCall {
            gate: self.gate,
            pacing: self.gate.service.pacing(),
            deadline,
            reported: Cell::new(false),
        };
        let sent_at = call.sent()?;
        self.gate.log_request(sent_at);
        let attempted = attempt();
        call.finished(attempted.defer_until);
        Ok(())
    }
}

/// One paced attempt's dealings with the shared pacing state. Each step
/// reads the wall clock only once `pacing.lock` is held, so a send another
/// process records while this one waits for the lock is never clamped back
/// to a stale `now`.
struct PacedCall<'a> {
    gate: &'a FilePacingGate,
    pacing: Pacing,
    deadline: &'a Deadline,
    reported: Cell<bool>,
}

impl PacedCall<'_> {
    /// Reserves, sleeps and confirms until the send is allowed, recording
    /// it before returning when it was sent.
    fn sent(&self) -> Result<SystemTime, WaitTooLong> {
        loop {
            let reservation = self.reserve_send()?;
            if let Some(sent_at) = self.confirm_send(&reservation)? {
                return Ok(sent_at);
            }
        }
    }

    fn reserve_send(&self) -> Result<Reservation, WaitTooLong> {
        let held = self.locked();
        let now = self.gate.clock.wall_now();
        let stored = self.stored();
        let (reservation, reserved) =
            stored.trust(&self.pacing, now).reserve(&self.pacing, now);
        let wait = until(reservation.send_at(), now);
        if !self.admits(wait) {
            return Err(WaitTooLong);
        }
        if reserved != stored {
            self.store(reserved);
        }
        drop(held);
        self.sleep(wait);
        Ok(reservation)
    }

    /// The send time once it is confirmed, or `None` when a deferral began
    /// after the reservation and the send must be reserved again.
    fn confirm_send(
        &self,
        reservation: &Reservation,
    ) -> Result<Option<SystemTime>, WaitTooLong> {
        loop {
            let held = self.locked();
            let now = self.gate.clock.wall_now();
            let state = self.stored().trust(&self.pacing, now);
            match state.confirm(&self.pacing, reservation, now) {
                Confirmation::Send(sent) => {
                    self.store(sent);
                    return Ok(Some(now));
                }
                Confirmation::Deferred(released) => {
                    self.store(released);
                    return Ok(None);
                }
                Confirmation::WaitUntil(allowed) => {
                    let wait = until(allowed, now);
                    if !self.admits(wait) {
                        self.store(state.release(&self.pacing, reservation));
                        return Err(WaitTooLong);
                    }
                    drop(held);
                    self.sleep(wait);
                }
            }
        }
    }

    fn finished(&self, defer_until: Option<SystemTime>) {
        let _held = self.locked();
        let now = self.gate.clock.wall_now();
        let finished = self.stored().trust(&self.pacing, now).finish(now);
        self.store(
            defer_until.map_or(finished, |until| finished.defer(until, now)),
        );
    }

    fn admits(&self, wait: Duration) -> bool {
        self.deadline
            .admits_attempt_after(self.gate.clock.now(), wait)
    }

    fn sleep(&self, wait: Duration) {
        if !wait.is_zero() {
            self.gate.clock.sleep(wait);
        }
    }

    /// `pacing.lock`, held exclusively until dropped. A filesystem that
    /// cannot lock still paces this call, reported once per call.
    fn locked(&self) -> Option<File> {
        let locked = self.gate.scratch.open(PACING_LOCK).and_then(|lock| {
            flock(&lock, FlockOperation::LockExclusive)
                .map(|()| lock)
                .map_err(|errno| {
                    format!(
                        "could not lock {}: {errno}",
                        self.gate.scratch.path(PACING_LOCK).display()
                    )
                })
        });
        locked
            .map_err(|error| {
                self.report_once(&format!("{error}; pacing without the lock"));
            })
            .ok()
    }

    /// A file that cannot be read at all counts as no previous request, as
    /// one of another shape does, but only the latter is reported.
    fn stored(&self) -> PacingState {
        let Some(bytes) = self.gate.scratch.read(STATE) else {
            return PacingState::default();
        };
        serde_json::from_slice::<StoredState>(&bytes)
            .ok()
            .filter(|stored| stored.schema_version == SCHEMA_VERSION)
            .map_or_else(
                || {
                    self.report_once(&format!(
                        "{} is not a pacing state this release reads; \
                         counting it as no previous request",
                        self.gate.scratch.path(STATE).display()
                    ));
                    PacingState::default()
                },
                PacingState::from,
            )
    }

    fn store(&self, state: PacingState) {
        let written = serde_json::to_vec(&StoredState::from(state))
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                self.gate.scratch.replace_without_sync(STATE, &bytes)
            });
        if let Err(error) = written {
            self.gate.report(&error);
        }
    }

    fn report_once(&self, detail: &str) {
        if !self.reported.replace(true) {
            self.gate.report(detail);
        }
    }
}

fn until(at: SystemTime, now: SystemTime) -> Duration {
    at.duration_since(now).unwrap_or(Duration::ZERO)
}

/// Every field is required, so a file from before reservations were
/// recorded is not mistaken for one with no reservation.
#[derive(Serialize, Deserialize)]
struct StoredState {
    schema_version: u32,
    #[serde(rename = "last_sent_ms", deserialize_with = "required")]
    last_sent: Option<u64>,
    #[serde(rename = "next_free_ms", deserialize_with = "required")]
    next_free: Option<u64>,
    #[serde(rename = "last_finish_ms", deserialize_with = "required")]
    last_finish: Option<u64>,
    #[serde(rename = "not_before_ms", deserialize_with = "required")]
    not_before: Option<u64>,
}

fn required<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u64>, D::Error> {
    Option::deserialize(deserializer)
}

impl From<StoredState> for PacingState {
    fn from(stored: StoredState) -> Self {
        let at = |millis: u64| {
            SystemTime::UNIX_EPOCH + Duration::from_millis(millis)
        };
        Self {
            last_sent: stored.last_sent.map(at),
            next_free: stored.next_free.map(at),
            last_finish: stored.last_finish.map(at),
            not_before: stored.not_before.map(at),
        }
    }
}

impl From<PacingState> for StoredState {
    fn from(state: PacingState) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            last_sent: state.last_sent.map(millis_since_epoch),
            next_free: state.next_free.map(millis_since_epoch),
            last_finish: state.last_finish.map(millis_since_epoch),
            not_before: state.not_before.map(millis_since_epoch),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::time::Duration;
    use std::time::Instant;

    use research::sources::fetch::Attempted;
    use research::sources::fetch::PacingGate as _;
    use research::sources::pacing::Pacing;
    use research::sources::schedule::Deadline;

    use super::NoPacing;

    #[test]
    fn every_attempt_runs_exactly_once_without_spacing() {
        let deadline = Deadline::starting(
            Instant::now(),
            Duration::from_secs(100),
            Duration::from_secs(30),
        );
        let mut runs = 0;

        let turn = NoPacing.try_serve().expect("always served");
        let passed = turn.paced(
            &mut || {
                runs += 1;
                Attempted { defer_until: None }
            },
            &deadline,
        );

        assert_eq!(passed, Ok(()));
        assert_eq!(runs, 1);
        assert_eq!(NoPacing.pacing(), Pacing::UNPACED);
    }
}
