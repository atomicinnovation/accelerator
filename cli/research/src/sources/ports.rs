//! What the fetch engine asks of the world: the wire, the clock, the gate
//! that paces a service, the log of contention, and where degraded but
//! survivable conditions are reported.

use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use crate::sources::classify::Reason;
use crate::sources::classify::Response;
use crate::sources::pacing::Pacing;
use crate::sources::queue::Ticket;
use crate::sources::request::UpstreamRequest;
use crate::sources::schedule::Deadline;

pub trait Transport {
    fn send(&self, request: &UpstreamRequest) -> Response;
}

/// `now` serves in-process arithmetic; `wall_now` stamps values persisted
/// for other processes to read.
pub trait Clock {
    fn now(&self) -> Instant;
    fn wall_now(&self) -> SystemTime;
    fn sleep(&self, duration: Duration);
}

/// One line, never a secret, and never a reason to fail the call.
pub trait Diagnostics {
    fn report(&self, line: &str);
}

/// What the gate needs to know once an attempt has run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempted {
    /// When a throttled source asks every caller to wait until.
    pub defer_until: Option<SystemTime>,
}

/// Admits calls to a service as its pacing allows.
pub trait PacingGate {
    fn pacing(&self) -> Pacing;

    /// A turn holding one of the service's slots, or `None` while other
    /// calls hold every one.
    fn try_serve(&self) -> Option<Box<dyn ServingTurn + '_>>;
}

/// Holds a slot for as long as it lives, so every attempt and backoff of a
/// call runs within one turn.
pub trait ServingTurn {
    /// Runs `attempt` exactly once, or refuses without running it when the
    /// deadline cannot admit the wait the service still asks for.
    ///
    /// # Errors
    ///
    /// [`WaitTooLong`] when the attempt could not be admitted in time.
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        deadline: &Deadline,
    ) -> Result<(), WaitTooLong>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitTooLong;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfBudget {
    pub last_retryable: Option<Reason>,
}

/// Why a queued call ended in lock contention with no upstream failure
/// behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Contention {
    TicketPastCap(Ticket),
    /// The call ran out of budget with no place in the queue to keep.
    QueueUnusable,
}

/// Where a call that lost its turn to contention is recorded.
pub trait ContentionLog {
    fn record(&self, contention: Contention);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    LockContention,
}

impl Cause {
    pub const fn code(self) -> &'static str {
        match self {
            Self::LockContention => "lock_contention",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unavailable {
    reason: Reason,
    cause: Option<Cause>,
}

impl Unavailable {
    pub const fn new(reason: Reason) -> Self {
        Self {
            reason,
            cause: None,
        }
    }

    /// The call's ticket passed its cap, or the queue could not be used, and
    /// no upstream failure was left to report instead.
    pub const fn lock_contention() -> Self {
        Self {
            reason: Reason::RateLimited,
            cause: Some(Cause::LockContention),
        }
    }

    pub const fn reason(self) -> Reason {
        self.reason
    }

    pub const fn cause(self) -> Option<Cause> {
        self.cause
    }
}
