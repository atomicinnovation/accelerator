//! Waiting for a serving turn while the call's budget allows.

use crate::sources::ports::Clock;
use crate::sources::ports::OutOfBudget;
use crate::sources::ports::PacingGate;
use crate::sources::ports::ServingTurn;
use crate::sources::schedule::Deadline;

/// Waits for a turn while `ready` holds back and the deadline still covers a
/// serving window: the service's interval and one whole request.
pub fn admitted<'g>(
    gate: &'g dyn PacingGate,
    clock: &dyn Clock,
    deadline: &Deadline,
    ready: &dyn Fn() -> bool,
) -> Result<Box<dyn ServingTurn + 'g>, OutOfBudget> {
    let pacing = gate.pacing();
    loop {
        if !deadline.admits_attempt_after(clock.now(), pacing.interval()) {
            return Err(OutOfBudget {
                last_retryable: None,
            });
        }
        if ready() {
            if let Some(turn) = gate.try_serve() {
                return Ok(turn);
            }
        }
        clock.sleep(pacing.admission_poll());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn admission_depends_on_no_fetch_workflow() {
        assert!(!include_str!("admission.rs")
            .contains(concat!("sources::", "fetch")));
    }
}
