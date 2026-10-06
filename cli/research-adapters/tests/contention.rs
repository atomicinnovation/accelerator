#![allow(clippy::expect_used)]

mod support;

use std::rc::Rc;

use research::sources::fetch::Clock as _;
use research::sources::fetch::Contention;
use research::sources::fetch::ContentionLog as _;
use research::sources::queue::Ticket;
use research_adapters::contention::FileContentionLog;
use support::millis;
use support::RecordedDiagnostics;
use support::RecordingClock;
use support::Scratch;

#[test]
fn recording_contention_appends_one_line_per_event_and_reports_it() {
    let scratch = Scratch::new();
    let clock = RecordingClock::new();
    let diagnostics = Rc::new(RecordedDiagnostics::default());
    let log = FileContentionLog::new(
        scratch.dir(),
        clock.clone(),
        diagnostics.clone(),
    );
    let ticket = Ticket::parse("42-9f1c2a").expect("a ticket");

    log.record(Contention::TicketPastCap(ticket));
    log.record(Contention::QueueUnusable);

    let at = millis(clock.wall_now());
    assert_eq!(
        scratch.lines("arxiv-contention.log"),
        [
            format!("{at} ticket_past_cap 42-9f1c2a"),
            format!("{at} queue_unusable"),
        ]
    );
    let reported = diagnostics.lines();
    assert_eq!(reported.len(), 2, "{reported:?}");
    assert!(
        reported.iter().all(|line| line.contains("lock contention")),
        "{reported:?}"
    );
    assert!(reported[0].contains("42-9f1c2a"), "{reported:?}");
}
