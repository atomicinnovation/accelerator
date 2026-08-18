//! The bounded-retry schedule both providers run.
//!
//! [`RetryPolicy::delay_for`] returns the delay as data rather than sleeping,
//! and takes the hint per attempt because `Retry-After` arrives on each
//! individual response — attempt 2 and attempt 3 can carry different values or
//! none. A whole-sequence API would force the caller to recompute and discard,
//! or to pin the first response's hint across every later backoff, and the
//! retry loop would then grow its own arithmetic beside the shared policy.

use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// The ± spread applied to an exponential delay, as a percentage of it.
const JITTER_PERCENT: u64 = 30;
const FLOOR: Duration = Duration::from_secs(1);
const CEILING: Duration = Duration::from_secs(60);

/// A source of the ± offset applied to an exponential delay, injected so a
/// test asserts a delay sequence as data rather than by wall clock.
pub trait Jitter {
    /// An offset in `-spread ..= spread` whole seconds.
    fn offset(&mut self, spread: u64) -> i64;
}

/// The production jitter: seeded from the clock.
pub struct ClockJitter;

impl Jitter for ClockJitter {
    fn offset(&mut self, spread: u64) -> i64 {
        if spread == 0 {
            return 0;
        }
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| u64::from(elapsed.subsec_nanos()));
        let magnitude = i64::try_from(seed % (spread + 1)).unwrap_or(0);
        if seed.is_multiple_of(2) {
            magnitude
        } else {
            -magnitude
        }
    }
}

/// Where a retry loop waits, injected so the retry suites never wait on real
/// time.
pub trait Sleeper {
    fn sleep(&mut self, duration: Duration);
}

/// The production sleeper.
pub struct SystemSleeper;

impl Sleeper for SystemSleeper {
    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// How many attempts a request gets and how long it waits between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: usize,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 4 }
    }
}

impl RetryPolicy {
    /// The delay to take after `attempt` (1-based) failed, or `None` once the
    /// attempts are exhausted.
    ///
    /// A `Retry-After` hint wins outright, clamped to 1s..=60s; otherwise the
    /// delay is `2^(attempt - 1)` seconds with a ±30% offset, clamped the same
    /// way.
    pub fn delay_for(
        &self,
        attempt: usize,
        retry_after: Option<Duration>,
        jitter: &mut dyn Jitter,
    ) -> Option<Duration> {
        if attempt >= self.max_attempts {
            return None;
        }
        if let Some(hint) = retry_after {
            return Some(hint.clamp(FLOOR, CEILING));
        }
        let exponent =
            u32::try_from(attempt.saturating_sub(1)).unwrap_or(u32::MAX);
        let base = 1_u64.checked_shl(exponent).unwrap_or(CEILING.as_secs());
        let base = base.min(CEILING.as_secs());
        let spread = base * JITTER_PERCENT / 100;
        let offset = jitter.offset(spread).clamp(
            -i64::try_from(spread).unwrap_or(i64::MAX),
            i64::try_from(spread).unwrap_or(i64::MAX),
        );
        let seconds = i64::try_from(base).unwrap_or(i64::MAX) + offset;
        let seconds = u64::try_from(seconds).unwrap_or(0);
        Some(Duration::from_secs(seconds).clamp(FLOOR, CEILING))
    }
}
