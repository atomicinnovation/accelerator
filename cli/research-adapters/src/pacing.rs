//! Pacing gates: none for sources that meter by budget, and a file-locked
//! one for arXiv, which asks every client for one request at a time, three
//! seconds apart.

use std::fs::File;
use std::rc::Rc;
use std::time::Duration;
use std::time::SystemTime;

use research::sources::fetch::Attempted;
use research::sources::fetch::Clock;
use research::sources::fetch::PacingGate;
use research::sources::fetch::ServingTurn;
use research::sources::fetch::WaitTooLong;
use research::sources::schedule::Deadline;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use rustix::io::Errno;
use serde::Deserialize;
use serde::Serialize;

use crate::clock::millis_since_epoch;
use crate::diagnostics::Diagnostics;
use crate::scratch::ScratchDir;

/// Serves every call at once. OpenAlex meters callers by budget rather than
/// by request spacing, so there is nothing to hold or defer.
pub struct NoPacing;

impl PacingGate for NoPacing {
    fn spacing(&self) -> Duration {
        Duration::ZERO
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

const LOCK: &str = "arxiv.lock";
const STATE: &str = "arxiv-pacing";
const REQUEST_LOG: &str = "arxiv-requests.log";

const SPACING: Duration = Duration::from_secs(3);
/// A send is stamped before its request leaves, so its arrival at arXiv is
/// unobserved and can trail the stamp by more than the next request's does.
const SEND_MARGIN: Duration = Duration::from_millis(100);
const SPACED_SEND: Duration = SPACING.saturating_add(SEND_MARGIN);
const DEFERRAL_CEILING: Duration = Duration::from_secs(30);

/// Serialises arXiv calls across the project through an exclusive `flock`,
/// held by each call's turn from its first request to its last.
///
/// Spacing runs from the last request's finish, when arXiv has certainly
/// seen it. A send with no finish after it, from a call killed mid-request,
/// is spaced from the send instead, with a margin for its unseen arrival.
///
/// Its waits run on the caller's clock, so time spent here is spent from
/// the same deadline the caller measures.
pub struct FilePacingGate {
    scratch: ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
}

impl FilePacingGate {
    pub fn new(
        scratch: ScratchDir,
        clock: Rc<dyn Clock>,
        diagnostics: Rc<dyn Diagnostics>,
    ) -> Self {
        Self {
            scratch,
            clock,
            diagnostics,
        }
    }

    fn stored(&self) -> PacingState {
        self.scratch
            .read(STATE)
            .and_then(|bytes| {
                serde_json::from_slice::<StoredState>(&bytes).ok()
            })
            .map_or_else(PacingState::default, |stored| {
                PacingState::from(stored).trusted_at(self.clock.wall_now())
            })
    }

    fn store(&self, state: PacingState) {
        let written = serde_json::to_vec(&StoredState::from(state))
            .map_err(|error| error.to_string())
            .and_then(|bytes| self.scratch.replace_without_sync(STATE, &bytes));
        if let Err(error) = written {
            self.report(&error);
        }
    }

    fn log_request(&self) {
        let line = millis_since_epoch(self.clock.wall_now()).to_string();
        if let Err(error) = self.scratch.append_line(REQUEST_LOG, &line) {
            self.report(&error);
        }
    }

    fn report(&self, detail: &str) {
        self.diagnostics
            .report(&format!("research fetch: arxiv {detail}"));
    }

    fn unlocked_turn(&self, detail: &str) -> Box<dyn ServingTurn + '_> {
        self.report(&format!("{detail}; pacing without the lock"));
        Box::new(ArxivTurn {
            gate: self,
            _lock: None,
        })
    }
}

impl PacingGate for FilePacingGate {
    fn spacing(&self) -> Duration {
        SPACING
    }

    /// A filesystem that cannot lock still gets a turn, so pacing still
    /// spaces this call's own requests.
    fn try_serve(&self) -> Option<Box<dyn ServingTurn + '_>> {
        let lock = match self.scratch.open(LOCK) {
            Ok(lock) => lock,
            Err(error) => return Some(self.unlocked_turn(&error)),
        };
        match flock(&lock, FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Some(Box::new(ArxivTurn {
                gate: self,
                _lock: Some(lock),
            })),
            Err(Errno::WOULDBLOCK) => None,
            Err(errno) => Some(self.unlocked_turn(&format!(
                "could not lock {}: {errno}",
                self.scratch.path(LOCK).display()
            ))),
        }
    }
}

struct ArxivTurn<'g> {
    gate: &'g FilePacingGate,
    _lock: Option<File>,
}

impl ServingTurn for ArxivTurn<'_> {
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        deadline: &Deadline,
    ) -> Result<(), WaitTooLong> {
        let gate = self.gate;
        let wait = gate.stored().wait_at(gate.clock.wall_now());
        if !deadline.admits_attempt_after(gate.clock.now(), wait) {
            return Err(WaitTooLong);
        }
        if !wait.is_zero() {
            gate.clock.sleep(wait);
        }
        gate.store(gate.stored().sent_at(gate.clock.wall_now()));
        gate.log_request();
        let attempted = attempt();
        let now = gate.clock.wall_now();
        let finished = gate.stored().finished_at(now);
        gate.store(attempted.defer_until.map_or(finished, |defer_until| {
            finished.deferred_until(defer_until, now)
        }));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct PacingState {
    last_sent: Option<SystemTime>,
    last_finish: Option<SystemTime>,
    not_before: Option<SystemTime>,
}

impl PacingState {
    /// A deferral further ahead than any this gate writes can only come
    /// from a wall-clock step, so it is dropped rather than obeyed.
    fn trusted_at(self, now: SystemTime) -> Self {
        Self {
            not_before: self
                .not_before
                .filter(|not_before| *not_before <= now + DEFERRAL_CEILING),
            ..self
        }
    }

    fn wait_at(self, now: SystemTime) -> Duration {
        let until = |at: Option<SystemTime>, ceiling: Duration| {
            at.and_then(|at| at.duration_since(now).ok())
                .map_or(Duration::ZERO, |wait| wait.min(ceiling))
        };
        let spacing = match (self.last_sent, self.last_finish) {
            (Some(sent), finish) if finish < Some(sent) => {
                until(Some(sent + SPACED_SEND), SPACED_SEND)
            }
            (_, finish) => {
                until(finish.map(|finish| finish + SPACING), SPACING)
            }
        };
        let deferral = until(self.not_before, DEFERRAL_CEILING);
        spacing.max(deferral)
    }

    const fn sent_at(self, sent: SystemTime) -> Self {
        Self {
            last_sent: Some(sent),
            ..self
        }
    }

    const fn finished_at(self, finish: SystemTime) -> Self {
        Self {
            last_finish: Some(finish),
            ..self
        }
    }

    /// Deferrals only move forward, so a shorter one never cuts short a
    /// longer one another caller is still honouring.
    fn deferred_until(self, defer_until: SystemTime, now: SystemTime) -> Self {
        let latest = self
            .not_before
            .map_or(defer_until, |existing| existing.max(defer_until));
        Self {
            not_before: Some(latest.min(now + DEFERRAL_CEILING)),
            ..self
        }
    }
}

#[derive(Serialize, Deserialize)]
struct StoredState {
    #[serde(rename = "last_sent_ms", default)]
    last_sent: Option<u64>,
    #[serde(rename = "last_finish_ms")]
    last_finish: Option<u64>,
    #[serde(rename = "not_before_ms")]
    not_before: Option<u64>,
}

impl From<StoredState> for PacingState {
    fn from(stored: StoredState) -> Self {
        let at = |millis: u64| {
            SystemTime::UNIX_EPOCH + Duration::from_millis(millis)
        };
        Self {
            last_sent: stored.last_sent.map(at),
            last_finish: stored.last_finish.map(at),
            not_before: stored.not_before.map(at),
        }
    }
}

impl From<PacingState> for StoredState {
    fn from(state: PacingState) -> Self {
        Self {
            last_sent: state.last_sent.map(millis_since_epoch),
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
        assert_eq!(NoPacing.spacing(), Duration::ZERO);
    }
}
