#![allow(clippy::expect_used)]

mod support;

use std::rc::Rc;

use research::sources::fetch::Clock as _;
use research::sources::fetch::Contention;
use research::sources::fetch::ContentionLog as _;
use research::sources::queue::Ticket;
use support::arxiv_contention_log;
use support::millis;
use support::RecordedDiagnostics;
use support::RecordingClock;
use support::Scratch;
use support::StateFile;

#[test]
fn recording_contention_appends_one_line_per_event_and_reports_it() {
    let scratch = Scratch::new();
    let clock = RecordingClock::new();
    let diagnostics = Rc::new(RecordedDiagnostics::default());
    let log =
        arxiv_contention_log(&scratch, clock.clone(), diagnostics.clone());
    let ticket = Ticket::parse("42-9f1c2a").expect("a ticket");

    log.record(Contention::TicketPastCap(ticket));
    log.record(Contention::QueueUnusable);

    let at = millis(clock.wall_now());
    assert_eq!(
        scratch.lines(scratch.arxiv_state(StateFile::ContentionLog)),
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
    assert!(reported[0].contains("past its 900 s cap"), "{reported:?}");
}
