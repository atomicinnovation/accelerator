//! The seams the launcher's volatile inputs arrive through.
//!
//! Every one exists so an outcome is deterministic by construction rather than
//! by normalising it afterwards: cold start, warm reuse, stale-PID recovery,
//! PID-recycle rejection, lock contention and daemon-start timeout are all
//! reachable without a real process, a real browser or a real sleep.

use std::convert::Infallible;
use std::path::PathBuf;

use crate::executor::daemon_identity::ObservedDaemon;
use crate::executor::daemon_identity::RecordedState;
use crate::executor::handoff::Identity;

/// Wall-clock reads and the poll deadline.
pub trait Clock {
    /// Monotonic-enough seconds for measuring a deadline.
    fn now_seconds(&self) -> u64;
    /// Sleeps for the poll interval. Injected so a timeout test costs no real
    /// time.
    fn sleep_poll_interval(&self);
}

/// Liveness, and the start time of a live process.
pub trait ProcessProbe {
    fn observe(&self, pid: i32) -> ObservedDaemon;
}

/// The state directory's record, as a domain value rather than raw JSON.
///
/// The design crate cannot parse JSON, so the parse happens in the adapter —
/// but the reuse verdict turns on parse *outcomes*, so the port's return type
/// is what carries them.
pub trait StateStore {
    fn read(&self) -> RecordedState;
    /// Removes the record, under the lock. Never called on the pre-lock path:
    /// the two files are written as separate atomic renames, so a reader
    /// between them would otherwise delete a live daemon's state.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when the record could not be removed.
    fn clear(&self) -> Result<(), kernel::Error>;
    /// Removes a previous daemon's stop reason, so a failed start is not
    /// mistaken for a completed shutdown by whoever diagnoses it.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when the stop reason could not be removed.
    fn clear_stop_reason(&self) -> Result<(), kernel::Error>;
}

/// Whether this launcher won the right to spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockOutcome {
    Acquired,
    HeldByAnother,
}

/// The single lock backend.
///
/// The shell's flock-or-mkdir dichotomy existed because the `flock(1)` binary
/// is absent on macOS — a constraint that vanishes here, where `flock(2)` is
/// available on every supported target.
pub trait Lock {
    /// # Errors
    ///
    /// A [`kernel::Error`] when the lock file could not be opened or flocked.
    fn acquire(&self) -> Result<LockOutcome, kernel::Error>;
    /// Released explicitly before handing the process image to the client,
    /// where no destructor can run. Every other path releases through a guard.
    fn release(&self);
}

/// Starts the daemon and hands it its identity.
pub trait Spawner {
    /// Spawns, observes the child's start time with the same probe the reuse
    /// check will later use, and writes the identity down the inherited pipe.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when the child could not be spawned, `setsid` failed,
    /// or the identity could not be written down the pipe.
    fn spawn(&self) -> Result<Identity, kernel::Error>;
}

/// Signalling, kept apart from creation: bundling them made one port do two
/// unrelated things, and only the start-poll timeout ever signals.
pub trait ProcessControl {
    fn terminate(&self, pid: i32);
}

/// Where the launcher's inputs and outputs live.
///
/// `run.sh` derived all of these from `BASH_SOURCE`, so it always found its own
/// sibling files. A dispatched sub-binary executes from the launcher's cache
/// directory and cannot, so the paths are resolved through a port that refuses
/// with a named error when it has nothing to resolve from — the failure mode an
/// `ACCELERATOR_DESIGN_BIN` override would otherwise hit with no diagnosable
/// message.
///
/// Both path-bearing envelopes read their path from here, so each is
/// byte-identical whatever directory the caller invoked from.
pub trait PathResolution {
    /// The plugin installation root.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when the root cannot be established.
    fn plugin_root(&self) -> Result<PathBuf, kernel::Error>;

    /// The lockhash namespace the runtime is installed under.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when the lockfile cannot be read or hashed.
    fn namespace_root(&self) -> Result<PathBuf, kernel::Error>;

    /// The daemon's stdio redirect target.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when the state directory cannot be established.
    fn bootstrap_log(&self) -> Result<PathBuf, kernel::Error>;
}

/// Runs the client and does not come back.
///
/// The terminal implementation replaces the process image, so it cannot
/// return. Typing it as diverging makes that visible at the call site: no
/// domain logic — lock release, envelope rendering, exit-code mapping — may be
/// sequenced after this call, because it would run in a test's fake and never
/// in production.
pub trait RunClient {
    /// # Errors
    ///
    /// A [`kernel::Error`] when the process image could not be replaced. There
    /// is no success value: the call cannot return having succeeded.
    fn run(
        self: Box<Self>,
        arguments: &[String],
    ) -> Result<Infallible, kernel::Error>;
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::RunClient;

    /// A fake that returns is the whole reason the port is typed as diverging:
    /// it can only report an error, never a success the caller could continue
    /// past.
    struct NeverStarts;

    impl RunClient for NeverStarts {
        fn run(
            self: Box<Self>,
            _arguments: &[String],
        ) -> Result<Infallible, kernel::Error> {
            Err(kernel::Error::Failed("exec failed".to_owned()))
        }
    }

    #[test]
    fn the_client_port_can_only_report_failure() {
        let client: Box<dyn RunClient> = Box::new(NeverStarts);
        let Err(error) = client.run(&["ping".to_owned()]);
        assert!(error.to_string().contains("exec failed"));
    }
}
