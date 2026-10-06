#![allow(clippy::expect_used)]

mod support;

use std::rc::Rc;

use research::sources::fetch::Clock as _;
use research::sources::fetch::Contention;
use research::sources::fetch::ContentionLog as _;
use research_adapters::contention::FileContentionLog;
use support::millis;
use support::RecordedDiagnostics;
use support::RecordingClock;
use support::Scratch;

#[test]
fn recording_contention_appends_one_line_and_reports_it() {
    let scratch = Scratch::new();
    let clock = RecordingClock::new();
    let diagnostics = Rc::new(RecordedDiagnostics::default());
    let log = FileContentionLog::new(
        scratch.dir(),
        clock.clone(),
        diagnostics.clone(),
    );

    log.record(Contention::LockHeldPastDeadline);

    assert_eq!(
        scratch.lines("arxiv-contention.log"),
        [format!(
            "{} lock_held_past_deadline",
            millis(clock.wall_now())
        )]
    );
    let reported = diagnostics.lines();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(reported[0].contains("lock contention"), "{reported:?}");
}
