//! The real clock behind the poll deadline.

use std::time::Duration;
use std::time::Instant;

use design::executor::ports::Clock;

/// The interval between readiness checks, matching the retired shell's own
/// 300 × 0.1s poll.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Elapsed seconds since construction.
///
/// A monotonic origin rather than the wall clock: the deadline is a duration,
/// and a clock stepped backwards by NTP mid-poll would otherwise extend it.
pub struct MonotonicClock {
    origin: Instant,
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Clock for MonotonicClock {
    fn now_seconds(&self) -> u64 {
        self.origin.elapsed().as_secs()
    }

    fn sleep_poll_interval(&self) {
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use design::executor::ports::Clock as _;

    use super::MonotonicClock;
    use super::POLL_INTERVAL;

    #[test]
    fn the_clock_starts_at_zero_and_does_not_go_backwards() {
        let clock = MonotonicClock::default();
        let first = clock.now_seconds();
        assert_eq!(first, 0);
        clock.sleep_poll_interval();
        assert!(clock.now_seconds() >= first);
    }

    /// Short enough that a 30-second budget is ~300 checks, long enough not to
    /// spin — the shell's own figure.
    #[test]
    fn the_poll_interval_matches_the_retired_shell_s() {
        assert_eq!(POLL_INTERVAL.as_millis(), 100);
    }
}
