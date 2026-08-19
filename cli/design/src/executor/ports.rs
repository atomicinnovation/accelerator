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

/// The single lock backend, `flock(2)` on every supported target.
pub trait Lock {
    /// # Errors
    ///
    /// A [`kernel::Error`] when the lock file could not be opened or flocked.
    fn acquire(&self) -> Result<LockOutcome, kernel::Error>;
    /// Released explicitly before handing the process image to the client,
    /// where no destructor can run. Every other path releases through a guard.
    fn release(&self);
}

/// Why a daemon spawn did not produce a running child.
///
/// `NotFound` is kept apart from every other failure because it is the
/// post-fetch signal for `loader-unresolvable`: the program was resolved from
/// the driver tree and exists, so an `execve` answering `ENOENT` means the
/// dynamic loader it names is absent, not the program. It is an errno, never a
/// shell message — the executor spawns with no shell, so `cannot execute:
/// required file not found` can never appear.
#[derive(Debug)]
pub enum SpawnError {
    /// `execve` answered `ENOENT`: the program's interpreter is missing.
    NotFound,
    /// Any other failure — `setsid`, the pipe, the identity handoff, or a
    /// spawn error that is not `ENOENT`.
    Failed(kernel::Error),
}

impl From<kernel::Error> for SpawnError {
    fn from(error: kernel::Error) -> Self {
        Self::Failed(error)
    }
}

impl std::fmt::Display for SpawnError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str(
                "the daemon's program or its interpreter is absent (ENOENT)",
            ),
            Self::Failed(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for SpawnError {}

/// Starts the daemon and hands it its identity.
pub trait Spawner {
    /// Spawns, observes the child's start time with the same probe the reuse
    /// check will later use, and writes the identity down the inherited pipe.
    ///
    /// # Errors
    ///
    /// A [`SpawnError`] — `NotFound` when `execve` answered `ENOENT`, else
    /// `Failed` carrying the underlying cause.
    fn spawn(&self) -> Result<Identity, SpawnError>;
}

/// Reads a failed daemon's bootstrap log so the launch state machine can
/// classify why it never became ready.
///
/// The log is already `0600` and truncated per attempt, so what it returns is
/// this attempt's output. Reads only on the spawn path, when readiness fails;
/// the `exec` path replaces the process image and has no Rust left to diagnose.
pub trait BootstrapDiagnostics {
    /// The lines of the current attempt's bootstrap log, or empty when it
    /// cannot be read — an unreadable log is not itself a classification.
    fn read_log(&self) -> Vec<String>;
}

/// Signalling, kept apart from creation: only the start-poll timeout signals.
pub trait ProcessControl {
    fn terminate(&self, pid: i32);
}

/// Where the launcher's inputs and outputs live.
///
/// A dispatched sub-binary executes from the launcher's cache directory, so it
/// cannot find its own sibling files by location. The paths resolve through a
/// port that refuses with a named error when it has nothing to resolve from —
/// the failure mode an `ACCELERATOR_DESIGN_BIN` override would otherwise hit
/// with no diagnosable message.
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
