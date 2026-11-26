//! Pacing gates: none for sources that meter by budget, and a file-locked
//! one for each queued service, paced as that service asks.

use std::cell::Cell;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use std::time::SystemTime;

use research::sources::pacing::Confirmation;
use research::sources::pacing::Pacing;
use research::sources::pacing::PacingState;
use research::sources::pacing::Reservation;
use research::sources::ports::Attempted;
use research::sources::ports::Clock;
use research::sources::ports::PacingGate;
use research::sources::ports::ServingTurn;
use research::sources::ports::WaitTooLong;
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
///
/// A companion lock is taken too, while serving, for as long as another
/// release paces the same service through it; the gate never reads or
/// writes anything else of that release's.
pub struct FilePacingGate {
    service: Service,
    scratch: ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
    companion_locks: Vec<PathBuf>,
}

impl FilePacingGate {
    pub fn new(
        service: Service,
        research: &ScratchDir,
        clock: Rc<dyn Clock>,
        diagnostics: Rc<dyn Diagnostics>,
        companion_locks: Vec<PathBuf>,
    ) -> Self {
        Self {
            service,
            scratch: research.service_state(service),
            clock,
            diagnostics,
            companion_locks,
        }
    }

    fn report(&self, detail: &str) {
        self.diagnostics.report(&format!(
            "research fetch: {} {detail}",
            self.service.code()
        ));
    }

    /// A turn on `slot`, or `None` while a companion lock is busy, which
    /// gives the slot back at once.
    fn turn(&self, slot: Option<File>) -> Option<Box<dyn ServingTurn + '_>> {
        let mut companions = Vec::new();
        for path in &self.companion_locks {
            match Self::companion(path) {
                Ok(lock) => companions.push(lock),
                Err(Companion::Busy) => return None,
                Err(Companion::Unusable(error)) => self.report(&format!(
                    "could not lock {}: {error}; serving without it",
                    path.display()
                )),
            }
        }
        Some(Box::new(Turn {
            gate: self,
            _slot: slot,
            _companions: companions,
        }))
    }

    /// Opened or created as the release that owns it opens it, so the two
    /// can never race on its first creation.
    fn companion(path: &Path) -> Result<File, Companion> {
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .map_err(|error| Companion::Unusable(error.to_string()))?;
        match flock(&lock, FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(lock),
            Err(Errno::WOULDBLOCK) => Err(Companion::Busy),
            Err(errno) => Err(Companion::Unusable(errno.to_string())),
        }
    }

    fn unlocked_turn(&self, detail: &str) -> Option<Box<dyn ServingTurn + '_>> {
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
                Err(error) => return self.unlocked_turn(&error),
            };
            match flock(&lock, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => return self.turn(Some(lock)),
                Err(Errno::WOULDBLOCK) => {}
                Err(errno) => {
                    return self.unlocked_turn(&format!(
                        "could not lock {}: {errno}",
                        self.scratch.path(&name).display()
                    ))
                }
            }
        }
        None
    }
}

enum Companion {
    Busy,
    Unusable(String),
}

struct Turn<'g> {
    gate: &'g FilePacingGate,
    _slot: Option<File>,
    _companions: Vec<File>,
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
            unlocked: Reported::default(),
            unreadable_state: Reported::default(),
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
    unlocked: Reported,
    unreadable_state: Reported,
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
                self.unlocked.once(
                    self.gate,
                    &format!("{error}; pacing without the lock"),
                );
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
                    self.unreadable_state.once(
                        self.gate,
                        &format!(
                            "{} is not a pacing state this release reads; \
                             counting it as no previous request",
                            self.gate.scratch.path(STATE).display()
                        ),
                    );
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
}

/// One kind of diagnostic, written at most once per paced call.
#[derive(Default)]
struct Reported(Cell<bool>);

impl Reported {
    fn once(&self, gate: &FilePacingGate, detail: &str) {
        if !self.0.replace(true) {
            gate.report(detail);
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

    use research::sources::pacing::Pacing;
    use research::sources::ports::Attempted;
    use research::sources::ports::PacingGate as _;
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
