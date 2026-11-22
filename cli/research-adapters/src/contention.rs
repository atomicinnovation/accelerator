//! The log of a service's calls that ended in lock contention, one line per
//! call: the wall-clock milliseconds, the kind of contention, and the ticket
//! when there is one.

use std::rc::Rc;

use research::sources::fetch::Clock;
use research::sources::fetch::Contention;
use research::sources::fetch::ContentionLog;
use research::sources::queue;
use research::sources::service::Service;

use crate::clock::millis_since_epoch;
use crate::diagnostics::Diagnostics;
use crate::scratch::ScratchDir;

const CONTENTION_LOG: &str = "contention.log";

pub struct FileContentionLog {
    service: Service,
    scratch: ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
}

impl FileContentionLog {
    pub fn new(
        service: Service,
        research: &ScratchDir,
        clock: Rc<dyn Clock>,
        diagnostics: Rc<dyn Diagnostics>,
    ) -> Self {
        Self {
            service,
            scratch: research.service_state(service),
            clock,
            diagnostics,
        }
    }

    fn report(&self, detail: &str) {
        self.diagnostics.report(&format!(
            "research fetch: {} {detail}",
            self.service.code()
        ));
    }
}

impl ContentionLog for FileContentionLog {
    fn record(&self, contention: Contention) {
        let at = millis_since_epoch(self.clock.wall_now());
        let (line, explanation) = match contention {
            Contention::TicketPastCap(ticket) => (
                format!("{at} ticket_past_cap {ticket}"),
                format!(
                    "ticket {ticket} waited past its {} s cap",
                    queue::CAP.as_secs()
                ),
            ),
            Contention::QueueUnusable => (
                format!("{at} queue_unusable"),
                "this call ran out of budget while the queue was unusable"
                    .to_owned(),
            ),
        };
        self.report(&format!("lock contention: {explanation}"));
        if let Err(error) = self.scratch.append_line(CONTENTION_LOG, &line) {
            self.report(&error);
        }
    }
}
