//! How long to wait between attempts, and how long a whole call may take.

use std::time::Duration;
use std::time::Instant;

/// Which retry is next: the first retry follows the first attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryNumber(usize);

impl RetryNumber {
    pub const fn first() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    const fn index(self) -> usize {
        self.0
    }
}

pub struct RetrySchedule;

impl RetrySchedule {
    const BACKOFF: [Duration; 3] = [
        Duration::from_secs(3),
        Duration::from_secs(6),
        Duration::from_secs(12),
    ];
    const RETRY_AFTER_CEILING: Duration = Duration::from_secs(30);

    /// The wait before `retry`, or `None` when retries are exhausted. An
    /// upstream `Retry-After` replaces the backoff, clamped to 30 s.
    pub fn wait_before(
        retry: RetryNumber,
        retry_after: Option<Duration>,
    ) -> Option<Duration> {
        let backoff = Self::BACKOFF.get(retry.index())?;
        Some(Self::honouring(retry_after, *backoff))
    }

    /// How long a throttled attempt asks every other caller to hold off.
    /// Unlike [`Self::wait_before`] it has a value after the final attempt
    /// too, the last backoff.
    pub fn deferral_after(
        retry: RetryNumber,
        retry_after: Option<Duration>,
    ) -> Duration {
        let last = Self::BACKOFF.len() - 1;
        let backoff = Self::BACKOFF[retry.index().min(last)];
        Self::honouring(retry_after, backoff)
    }

    fn honouring(retry_after: Option<Duration>, backoff: Duration) -> Duration {
        retry_after.map_or(backoff, |hint| hint.min(Self::RETRY_AFTER_CEILING))
    }
}

/// A call's overall budget and each request's budget within it.
///
/// A pure value: every query takes the current instant from the caller's
/// clock, so the deadline and every wait are measured on one timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deadline {
    ends_at: Instant,
    per_request: Duration,
}

impl Deadline {
    pub fn starting(
        now: Instant,
        total: Duration,
        per_request: Duration,
    ) -> Self {
        Self {
            ends_at: now + total,
            per_request,
        }
    }

    pub fn remaining(&self, now: Instant) -> Duration {
        self.ends_at.saturating_duration_since(now)
    }

    /// Whether an attempt begun after `wait` would finish in time even if
    /// its request ran for its whole budget.
    pub fn admits_attempt_after(&self, now: Instant, wait: Duration) -> bool {
        now + wait + self.per_request <= self.ends_at
    }

    pub const fn per_request(&self) -> Duration {
        self.per_request
    }
}
