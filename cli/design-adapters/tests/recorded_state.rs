//! The reader against a record the daemon itself wrote.
//!
//! The unit tests in `state.rs` build their JSON by hand, so they cannot catch
//! the writer and the reader disagreeing about a field. This reads the shared
//! fixture both languages assert against: `lib/state.js` publishes this shape,
//! and `identity-handoff.test.js` pins the same file from the writing side.

use design::executor::daemon_identity::RecordedStartTime;
use design::executor::daemon_identity::RecordedState;
use design_adapters::state::interpret;

const WRITER_UNAVAILABLE: &str = include_str!(
    "../../../skills/design/inventory-design/scripts/playwright/lib/\
     __fixtures__/server-info-writer-unavailable.json"
);

/// The container case: `/proc` unreadable on the writing side, so there is no
/// probed start time to record. Reading it as absent recovers and respawns on
/// every invocation, losing the crawl's page state each time.
#[test]
fn the_daemon_s_writer_unavailable_record_reuses_on_liveness_alone(
) -> Result<(), String> {
    let RecordedState::Daemon(daemon) = interpret(WRITER_UNAVAILABLE) else {
        return Err("expected a daemon record".to_owned());
    };
    assert_eq!(daemon.pid, 4242);
    assert_eq!(daemon.start_time, RecordedStartTime::WriterUnavailable);
    Ok(())
}
