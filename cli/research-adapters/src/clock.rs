//! The real clock every wait and deadline is measured on.

use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use research::sources::fetch::Clock;

pub struct SystemClock;

pub(crate) fn millis_since_epoch(at: SystemTime) -> u64 {
    at.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|since| u64::try_from(since.as_millis()).ok())
        .unwrap_or_default()
}

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wall_now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use research::sources::fetch::Clock as _;

    use super::SystemClock;

    #[test]
    fn sleeping_advances_the_clock_by_at_least_the_wait() {
        let before = SystemClock.now();
        SystemClock.sleep(Duration::from_millis(20));
        assert!(SystemClock.now() - before >= Duration::from_millis(20));
    }
}
