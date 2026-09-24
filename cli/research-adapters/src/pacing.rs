//! Pacing gates for sources that tolerate concurrent requests.

use research::fetch::Attempted;
use research::fetch::PacingGate;
use research::fetch::Unavailable;
use research::schedule::Deadline;

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
