//! Pacing gates: none for sources that meter by budget, and a file-locked
//! one for arXiv, which asks every client for one request at a time, three
//! seconds apart.

use std::fs::File;
use std::rc::Rc;
use std::time::Duration;
use std::time::SystemTime;

use research::classify::Reason;
use research::fetch::Attempted;
use research::fetch::Clock;
use research::fetch::PacingGate;
use research::fetch::Unavailable;
use research::schedule::Deadline;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use rustix::io::Errno;
use serde::Deserialize;
use serde::Serialize;

use crate::diagnostics::Diagnostics;
use crate::scratch::ScratchDir;

/// Admits every attempt at once. OpenAlex meters callers by budget rather
/// than by request spacing, so there is nothing to hold or defer.
pub struct NoPacing;

impl PacingGate for NoPacing {
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        _deadline: &Deadline,
    ) -> Result<(), Unavailable> {
        attempt();
        Ok(())
    }
}

const LOCK: &str = "arxiv.lock";
const STATE: &str = "arxiv-pacing";
const REQUEST_LOG: &str = "arxiv-requests.log";
const CONTENTION_LOG: &str = "arxiv-contention.log";

const SPACING: Duration = Duration::from_secs(3);
const DEFERRAL_CEILING: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(100);

/// Serialises arXiv requests across every call in the project through an
/// exclusive `flock`, holding it until the response body has been read.
///
/// Spacing runs from when the last request finished, not when it was sent:
/// only then is it certain arXiv has seen it, so the next request can never
/// arrive sooner than three seconds after it.
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

    /// The lock, once no other call holds it; `None` when this filesystem
    /// cannot lock, in which case pacing still spaces this call's requests.
    fn hold(&self, deadline: &Deadline) -> Result<Option<File>, Unavailable> {
        let lock = match self.scratch.open(LOCK) {
            Ok(lock) => lock,
            Err(error) => {
                self.report(&format!("{error}; pacing without the lock"));
                return Ok(None);
            }
        };
        loop {
            match flock(&lock, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => return Ok(Some(lock)),
                Err(Errno::WOULDBLOCK) => {
                    if !deadline.admits_attempt_after(self.clock.now(), POLL) {
                        self.report(
                            "lock contention: another call held arXiv past \
                             this call's deadline",
                        );
                        self.log(CONTENTION_LOG);
                        return Err(Unavailable::lock_contention());
                    }
                    self.clock.sleep(POLL);
                }
                Err(errno) => {
                    self.report(&format!(
                        "could not lock {}: {errno}; pacing without the lock",
                        self.scratch.path(LOCK).display()
                    ));
                    return Ok(None);
                }
            }
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
            .and_then(|bytes| self.scratch.replace(STATE, &bytes));
        if let Err(error) = written {
            self.report(&error);
        }
    }

    fn log(&self, name: &str) {
        let line = millis_since_epoch(self.clock.wall_now()).to_string();
        if let Err(error) = self.scratch.append_line(name, &line) {
            self.report(&error);
        }
    }

    fn report(&self, detail: &str) {
        self.diagnostics
            .report(&format!("research fetch: arxiv {detail}"));
    }
}

impl PacingGate for FilePacingGate {
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        deadline: &Deadline,
    ) -> Result<(), Unavailable> {
        let _held = self.hold(deadline)?;
        let stored = self.stored();
        let wait = stored.wait_at(self.clock.wall_now());
        if !deadline.admits_attempt_after(self.clock.now(), wait) {
            return Err(Unavailable::new(Reason::RateLimited));
        }
        if !wait.is_zero() {
            self.clock.sleep(wait);
        }
        self.log(REQUEST_LOG);
        let attempted = attempt();
        let now = self.clock.wall_now();
        let finished = self.stored().finished_at(now);
        self.store(attempted.defer_until.map_or(finished, |defer_until| {
            finished.deferred_until(defer_until, now)
        }));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct PacingState {
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
        let spacing =
            until(self.last_finish.map(|finish| finish + SPACING), SPACING);
        let deferral = until(self.not_before, DEFERRAL_CEILING);
        spacing.max(deferral)
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
    last_finish_ms: Option<u64>,
    not_before_ms: Option<u64>,
}

impl From<StoredState> for PacingState {
    fn from(stored: StoredState) -> Self {
        let at = |millis: u64| {
            SystemTime::UNIX_EPOCH + Duration::from_millis(millis)
        };
        Self {
            last_finish: stored.last_finish_ms.map(at),
            not_before: stored.not_before_ms.map(at),
        }
    }
}

impl From<PacingState> for StoredState {
    fn from(state: PacingState) -> Self {
        Self {
            last_finish_ms: state.last_finish.map(millis_since_epoch),
            not_before_ms: state.not_before.map(millis_since_epoch),
        }
    }
}

fn millis_since_epoch(at: SystemTime) -> u64 {
    at.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|since| u64::try_from(since.as_millis()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::time::Instant;

    use research::fetch::Attempted;
    use research::fetch::PacingGate as _;
    use research::schedule::Deadline;

    use super::NoPacing;

    #[test]
    fn every_attempt_runs_exactly_once() {
        let deadline = Deadline::starting(
            Instant::now(),
            Duration::from_secs(100),
            Duration::from_secs(30),
        );
        let mut runs = 0;

        let passed = NoPacing.paced(
            &mut || {
                runs += 1;
                Attempted { defer_until: None }
            },
            &deadline,
        );

        assert_eq!(passed, Ok(()));
        assert_eq!(runs, 1);
    }
}
