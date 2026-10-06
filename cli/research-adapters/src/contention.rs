//! The log of arXiv calls that ended in lock contention, one line per call:
//! the wall-clock milliseconds and the kind of contention.

use std::rc::Rc;

use research::sources::fetch::Clock;
use research::sources::fetch::Contention;
use research::sources::fetch::ContentionLog;

use crate::clock::millis_since_epoch;
use crate::diagnostics::Diagnostics;
use crate::scratch::ScratchDir;

const CONTENTION_LOG: &str = "arxiv-contention.log";

pub struct FileContentionLog {
    scratch: ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
}

impl FileContentionLog {
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

    fn report(&self, detail: &str) {
        self.diagnostics
            .report(&format!("research fetch: arxiv {detail}"));
    }
}

impl ContentionLog for FileContentionLog {
    fn record(&self, contention: Contention) {
        let (kind, explanation) = match contention {
            Contention::LockHeldPastDeadline => (
                "lock_held_past_deadline",
                "another call held arXiv past this call's deadline",
            ),
        };
        self.report(&format!("lock contention: {explanation}"));
        let line =
            format!("{} {kind}", millis_since_epoch(self.clock.wall_now()));
        if let Err(error) = self.scratch.append_line(CONTENTION_LOG, &line) {
            self.report(&error);
        }
    }
}
