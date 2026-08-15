//! Reading and clearing the daemon's state directory.
//!
//! The record is read from `server-info.json` **alone**. That file is the one
//! value published by one atomic rename; `server.pid` is a second,
//! independently-renamed file a reader can observe between the two writes, so
//! nothing here reads it.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use design::executor::daemon_identity::RecordedDaemon;
use design::executor::daemon_identity::RecordedStartTime;
use design::executor::daemon_identity::RecordedState;
use design::executor::ports::StateStore;

const SERVER_INFO: &str = "server-info.json";
const SERVER_PID: &str = "server.pid";
const SERVER_STOPPED: &str = "server-stopped.json";

/// The tag the daemon records alongside its start time.
const PROBE_SOURCE: &str = "probe";
const WALLCLOCK_SOURCE: &str = "wallclock";
const WRITER_UNAVAILABLE_SOURCE: &str = "writer-unavailable";

/// The state directory on disk.
pub struct StateDirectory {
    root: PathBuf,
}

impl StateDirectory {
    #[must_use]
    pub const fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn info_path(&self) -> PathBuf {
        self.root.join(SERVER_INFO)
    }
}

/// Interprets a `server-info.json` body.
///
/// Separated from the read so every provenance rule is testable against a
/// string rather than a filesystem.
#[must_use]
pub fn interpret(body: &str) -> RecordedState {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return RecordedState::PidUnparseable;
    };
    let Some(pid) = value.get("pid").and_then(serde_json::Value::as_i64) else {
        return RecordedState::PidUnparseable;
    };
    let Ok(pid) = i32::try_from(pid) else {
        return RecordedState::PidUnparseable;
    };

    RecordedState::Daemon(RecordedDaemon {
        pid,
        start_time: interpret_start_time(&value),
    })
}

/// The source is read before the value, because one source declares that there
/// is no value: the daemon publishes a null start time along
/// `writer-unavailable` when the launcher's own probe could not read one. Read
/// as a bare absence that record is stale, so every invocation recovers and
/// respawns — exactly what the liveness-only verdict rows exist to prevent.
///
/// A record with **no** source key reads as `Wallclock`, not `Probe`. Written
/// before the source key existed, its value is of genuinely unknown provenance,
/// and reading it as a kernel probe would hold it to the one-second tolerance
/// on the strength of a guess.
///
/// A source that is present but recognised as neither reads as absent: the
/// reader cannot validate what it cannot interpret.
fn interpret_start_time(value: &serde_json::Value) -> RecordedStartTime {
    let source = value
        .get("start_time_source")
        .and_then(serde_json::Value::as_str);

    if source == Some(WRITER_UNAVAILABLE_SOURCE) {
        return RecordedStartTime::WriterUnavailable;
    }

    let Some(seconds) =
        value.get("start_time").and_then(serde_json::Value::as_u64)
    else {
        return RecordedStartTime::AbsentOrUnparseable;
    };
    match source {
        Some(PROBE_SOURCE) => RecordedStartTime::Probe(seconds),
        Some(WALLCLOCK_SOURCE) | None => RecordedStartTime::Wallclock(seconds),
        Some(_) => RecordedStartTime::AbsentOrUnparseable,
    }
}

fn remove(path: &Path) -> Result<(), kernel::Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(kernel::Error::Failed(format!(
            "could not remove {}: {error}",
            path.display()
        ))),
    }
}

impl StateStore for StateDirectory {
    fn read(&self) -> RecordedState {
        fs::read_to_string(self.info_path())
            .map_or(RecordedState::None, |body| interpret(&body))
    }

    fn clear(&self) -> Result<(), kernel::Error> {
        remove(&self.info_path())?;
        // Removed too, despite not being read: leaving it would let anything
        // that does read it see a daemon this launcher has just declared stale.
        remove(&self.root.join(SERVER_PID))
    }

    fn clear_stop_reason(&self) -> Result<(), kernel::Error> {
        remove(&self.root.join(SERVER_STOPPED))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use design::executor::daemon_identity::RecordedStartTime;
    use design::executor::daemon_identity::RecordedState;
    use design::executor::ports::StateStore as _;

    use super::interpret;
    use super::StateDirectory;

    type TestError = Box<dyn std::error::Error>;

    fn start_time_of(body: &str) -> Option<RecordedStartTime> {
        match interpret(body) {
            RecordedState::Daemon(daemon) => Some(daemon.start_time),
            _ => None,
        }
    }

    #[test]
    fn a_probe_record_is_read_as_a_probe_value() {
        assert_eq!(
            start_time_of(
                r#"{"pid":42,"start_time":1000,"start_time_source":"probe"}"#
            ),
            Some(RecordedStartTime::Probe(1000))
        );
    }

    #[test]
    fn a_wallclock_record_is_read_as_a_wallclock_value() {
        assert_eq!(
            start_time_of(
                r#"{"pid":42,"start_time":1000,"start_time_source":"wallclock"}"#
            ),
            Some(RecordedStartTime::Wallclock(1000))
        );
    }

    /// Every record written before this port existed carries no source key.
    /// Reading it as a probe value would hold it to the tolerance on a guess.
    #[test]
    fn a_record_with_no_source_key_is_read_as_wallclock_not_probe() {
        assert_eq!(
            start_time_of(r#"{"pid":42,"start_time":1000}"#),
            Some(RecordedStartTime::Wallclock(1000))
        );
    }

    /// The daemon writes a null start time alongside this source when the
    /// launcher's own probe could not read one. Reading that as absent would
    /// recover, and so respawn, on every invocation — the failure the
    /// liveness-only verdict rows exist to prevent.
    #[test]
    fn a_writer_unavailable_source_survives_its_null_start_time() {
        assert_eq!(
            start_time_of(
                r#"{"pid":42,"start_time":null,
                    "start_time_source":"writer-unavailable"}"#
            ),
            Some(RecordedStartTime::WriterUnavailable)
        );
    }

    #[test]
    fn an_unrecognised_source_is_read_as_absent_rather_than_guessed() {
        assert_eq!(
            start_time_of(
                r#"{"pid":42,"start_time":1000,"start_time_source":"vibes"}"#
            ),
            Some(RecordedStartTime::AbsentOrUnparseable)
        );
    }

    #[test]
    fn an_absent_or_unusable_start_time_is_read_as_absent() {
        for body in [
            r#"{"pid":42}"#,
            r#"{"pid":42,"start_time":null}"#,
            r#"{"pid":42,"start_time":"1000"}"#,
            r#"{"pid":42,"start_time":-5}"#,
        ] {
            assert_eq!(
                start_time_of(body),
                Some(RecordedStartTime::AbsentOrUnparseable),
                "{body}"
            );
        }
    }

    /// Present but unusable, which is not the same as never written — the two
    /// are distinct states and a single label would conflate a corrupted record
    /// with a cold start.
    #[test]
    fn a_record_with_no_usable_pid_is_unparseable_not_absent() {
        for body in [
            "not json at all",
            r#"{"start_time":1000}"#,
            r#"{"pid":"42"}"#,
            r#"{"pid":99999999999999}"#,
        ] {
            assert_eq!(
                interpret(body),
                RecordedState::PidUnparseable,
                "{body}"
            );
        }
    }

    #[test]
    fn an_unreadable_directory_is_a_cold_start() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let store = StateDirectory::new(work.path().to_path_buf());
        assert_eq!(store.read(), RecordedState::None);
        Ok(())
    }

    #[test]
    fn a_written_record_reads_back() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        fs::write(
            work.path().join("server-info.json"),
            r#"{"pid":4242,"start_time":1000,"start_time_source":"probe"}"#,
        )?;
        let store = StateDirectory::new(work.path().to_path_buf());
        let RecordedState::Daemon(daemon) = store.read() else {
            return Err("expected a daemon record".into());
        };
        assert_eq!(daemon.pid, 4242);
        assert_eq!(daemon.start_time, RecordedStartTime::Probe(1000));
        Ok(())
    }

    #[test]
    fn clearing_removes_both_state_files_and_tolerates_their_absence(
    ) -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let store = StateDirectory::new(work.path().to_path_buf());
        fs::write(work.path().join("server-info.json"), "{}")?;
        fs::write(work.path().join("server.pid"), "42")?;

        store.clear()?;
        assert!(!work.path().join("server-info.json").exists());
        assert!(!work.path().join("server.pid").exists());
        store.clear()?;
        Ok(())
    }

    /// Without this, a failed start looks like a completed shutdown to whoever
    /// diagnoses a daemon that never came up.
    #[test]
    fn clearing_the_stop_reason_removes_a_previous_daemon_s_record(
    ) -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let store = StateDirectory::new(work.path().to_path_buf());
        let stopped = work.path().join("server-stopped.json");
        fs::write(&stopped, r#"{"reason":"idle-timeout"}"#)?;

        store.clear_stop_reason()?;
        assert!(!stopped.exists());
        store.clear_stop_reason()?;
        Ok(())
    }
}
