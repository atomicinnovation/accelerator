//! The decisions-file `DecisionSource`.
//!
//! Consumes one line per call, in order, from a decisions file already
//! validated (dry-apply) against the exact prompts a live run will hit — so,
//! by construction, an ordinary run never observes an unparseable line
//! here.
//!
//! Only ever exercised with at most one `Interactive` registry entry
//! (migration 0007 today — the compiled-in registry never contains a
//! second one). Sharing a single cursor across more than one interactive
//! migration in the same run is therefore unexercised and undefined
//! behaviour.

use std::cell::Cell;
use std::time::Duration;

use migrate::decisions_file::DecisionLines;
use migrate::interactive::Decision;
use migrate::interactive::Transformation;
use migrate::ports::DecisionError;
use migrate::ports::DecisionSource;

pub struct DecisionsFileDecisionSource {
    lines: DecisionLines,
    cursor: Cell<usize>,
}

impl DecisionsFileDecisionSource {
    #[must_use]
    pub fn new(content: &str) -> Self {
        Self {
            lines: DecisionLines::parse(content),
            cursor: Cell::new(0),
        }
    }
}

impl DecisionSource for DecisionsFileDecisionSource {
    fn next_decision(
        &self,
        _transformation: &Transformation,
        _timeout: Duration,
    ) -> Result<Decision, DecisionError> {
        let index = self.cursor.get();
        let Some(decision) = self.lines.decision_at(index) else {
            return Err(DecisionError::Eof);
        };
        self.cursor.set(index + 1);
        decision.ok_or(DecisionError::Eof)
    }
}
