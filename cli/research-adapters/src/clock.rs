//! The real clock every wait and deadline is measured on.

use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use research::fetch::Clock;

pub struct SystemClock;

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

    use research::fetch::Clock as _;

    use super::SystemClock;

    #[test]
    fn sleeping_advances_the_clock_by_at_least_the_wait() {
        let before = SystemClock.now();
        SystemClock.sleep(Duration::from_millis(20));
        assert!(SystemClock.now() - before >= Duration::from_millis(20));
    }
}
