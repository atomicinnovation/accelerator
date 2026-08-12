//! The launcher's sequence: reuse if we can, otherwise take the lock, re-check,
//! recover, spawn, wait for readiness, and hand the process image to the client.
//!
//! Two corrections to the shell are load-bearing here.
//!
//! **The pre-lock check is read-only.** The shell's first check ended in an
//! unconditional removal of the state files when the daemon looked dead — but
//! the two files are written as separate atomic renames, so a launcher reading
//! between them, or racing a daemon that has just written both, judged a
//! *live* daemon stale and deleted its state outside any lock. Two launchers
//! starting concurrently could therefore orphan a healthy daemon and spawn a
//! second, losing page state mid-crawl: exactly the symptom the start-time
//! contract exists to prevent. So the pre-lock check short-circuits on a hit or
//! falls through, and removal happens only under the lock.
//!
//! **The lock releases at launcher exit, on every path.** The two shell
//! backends genuinely differed — one leaked its descriptor into the daemon, the
//! other removed its directory before exec — and one guard cannot reproduce
//! both. Releasing at exit is the more defensible: holding it for the daemon's
//! lifetime makes a stale-start-time recovery while the daemon still lives
//! report `another-launcher-running` falsely.

use std::convert::Infallible;

use crate::executor::envelope::LauncherError;
use crate::executor::ports::Clock;
use crate::executor::ports::Lock;
use crate::executor::ports::LockOutcome;
use crate::executor::ports::ProcessControl;
use crate::executor::ports::ProcessProbe;
use crate::executor::ports::RunClient;
use crate::executor::ports::Spawner;
use crate::executor::ports::StateStore;
use crate::executor::reuse;
use crate::executor::reuse::Reuse;

/// How long the launcher waits for the daemon to publish its record.
pub const START_TIMEOUT_SECONDS: u64 = 30;

/// Everything the launch sequence needs from the outside world.
pub struct Launcher<'ports> {
    pub clock: &'ports dyn Clock,
    pub probe: &'ports dyn ProcessProbe,
    pub state: &'ports dyn StateStore,
    pub lock: &'ports dyn Lock,
    pub spawner: &'ports dyn Spawner,
    pub control: &'ports dyn ProcessControl,
    /// The path named by the timeout envelope, resolved through the path port
    /// rather than recomputed, so the envelope is byte-identical from any
    /// working directory.
    pub bootstrap_log: String,
}

/// Why the launcher did not reach the client.
#[derive(Debug)]
pub enum LaunchFailure {
    /// A launcher-level envelope: stderr, non-zero exit, three keys.
    Envelope(LauncherError),
    /// An internal failure with no envelope of its own.
    Failed(kernel::Error),
}

impl From<kernel::Error> for LaunchFailure {
    fn from(error: kernel::Error) -> Self {
        Self::Failed(error)
    }
}

impl<'ports> Launcher<'ports> {
    /// Runs `arguments` against a reused or freshly-spawned daemon.
    ///
    /// Returns only on failure: the success path hands the process image to
    /// `client`, which cannot come back.
    ///
    /// # Errors
    ///
    /// A [`LaunchFailure`] carrying either a launcher envelope or an internal
    /// error.
    pub fn launch(
        &self,
        arguments: &[String],
        client: Box<dyn RunClient + 'ports>,
    ) -> Result<Infallible, LaunchFailure> {
        if let Reuse::Reuse(_) = self.verdict() {
            return Self::hand_over(arguments, client);
        }

        if self.lock.acquire()? == LockOutcome::HeldByAnother {
            return Err(LaunchFailure::Envelope(
                LauncherError::AnotherLauncherRunning,
            ));
        }

        // Another launcher may have spawned the daemon between our read and
        // our acquisition, so the verdict is taken again — this time with the
        // authority to remove what it judges stale.
        if let Reuse::Reuse(_) = self.verdict() {
            self.lock.release();
            return Self::hand_over(arguments, client);
        }

        self.state.clear()?;
        self.state.clear_stop_reason()?;

        let identity = self.spawner.spawn()?;
        if let Err(timeout) = self.await_readiness() {
            self.control.terminate(identity.pid);
            self.lock.release();
            return Err(LaunchFailure::Envelope(timeout));
        }

        self.lock.release();
        Self::hand_over(arguments, client)
    }

    fn verdict(&self) -> Reuse {
        let recorded = self.state.read();
        let observed = match recorded {
            crate::executor::RecordedState::Daemon(daemon) => {
                self.probe.observe(daemon.pid)
            }
            // Nothing to observe, and nothing a probe could tell us.
            _ => crate::executor::ObservedDaemon::Absent,
        };
        reuse::evaluate(recorded, observed)
    }

    /// Polls until the daemon's record appears or the deadline passes.
    ///
    /// The deadline is arithmetic over an injected clock, so a timeout test
    /// costs no real time.
    fn await_readiness(&self) -> Result<(), LauncherError> {
        let deadline = self.clock.now_seconds() + START_TIMEOUT_SECONDS;
        loop {
            if matches!(
                self.state.read(),
                crate::executor::RecordedState::Daemon(_)
            ) {
                return Ok(());
            }
            if self.clock.now_seconds() >= deadline {
                return Err(LauncherError::DaemonStartTimeout {
                    bootstrap_log: self.bootstrap_log.clone(),
                });
            }
            self.clock.sleep_poll_interval();
        }
    }

    /// The lock is released immediately before this, mirroring the shell's own
    /// explicit release before `exec`: no destructor runs once the process
    /// image is replaced.
    fn hand_over(
        arguments: &[String],
        client: Box<dyn RunClient + 'ports>,
    ) -> Result<Infallible, LaunchFailure> {
        client.run(arguments).map_err(LaunchFailure::Failed)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::convert::Infallible;
    use std::rc::Rc;

    use super::LaunchFailure;
    use super::Launcher;
    use super::START_TIMEOUT_SECONDS;
    use crate::executor::daemon_identity::ObservedDaemon;
    use crate::executor::daemon_identity::ObservedStartTime;
    use crate::executor::daemon_identity::RecordedDaemon;
    use crate::executor::daemon_identity::RecordedStartTime;
    use crate::executor::daemon_identity::RecordedState;
    use crate::executor::envelope::LauncherError;
    use crate::executor::handoff::Identity;
    use crate::executor::ports::Clock;
    use crate::executor::ports::Lock;
    use crate::executor::ports::LockOutcome;
    use crate::executor::ports::ProcessControl;
    use crate::executor::ports::ProcessProbe;
    use crate::executor::ports::RunClient;
    use crate::executor::ports::Spawner;
    use crate::executor::ports::StateStore;

    const LIVE_PID: i32 = 4242;

    /// Advances a second per poll, so a timeout is reached in bounded steps
    /// with no real sleeping.
    struct TickingClock {
        now: Cell<u64>,
    }

    impl Clock for TickingClock {
        fn now_seconds(&self) -> u64 {
            self.now.get()
        }
        fn sleep_poll_interval(&self) {
            self.now.set(self.now.get() + 1);
        }
    }

    struct FixedProbe(ObservedDaemon);

    impl ProcessProbe for FixedProbe {
        fn observe(&self, _pid: i32) -> ObservedDaemon {
            self.0
        }
    }

    /// Returns each queued state in turn, repeating the last — enough to model
    /// "absent before the lock, present after" and "never becomes ready".
    struct ScriptedStore {
        reads: RefCell<Vec<RecordedState>>,
        cleared: Cell<usize>,
        stop_reason_cleared: Cell<usize>,
    }

    impl ScriptedStore {
        fn new(reads: Vec<RecordedState>) -> Self {
            Self {
                reads: RefCell::new(reads),
                cleared: Cell::new(0),
                stop_reason_cleared: Cell::new(0),
            }
        }
    }

    impl StateStore for ScriptedStore {
        fn read(&self) -> RecordedState {
            let mut reads = self.reads.borrow_mut();
            if reads.len() > 1 {
                reads.remove(0)
            } else {
                reads.first().copied().unwrap_or(RecordedState::None)
            }
        }
        fn clear(&self) -> Result<(), kernel::Error> {
            self.cleared.set(self.cleared.get() + 1);
            Ok(())
        }
        fn clear_stop_reason(&self) -> Result<(), kernel::Error> {
            self.stop_reason_cleared
                .set(self.stop_reason_cleared.get() + 1);
            Ok(())
        }
    }

    struct RecordingLock {
        outcome: LockOutcome,
        acquired: Cell<usize>,
        released: Cell<usize>,
    }

    impl RecordingLock {
        fn free() -> Self {
            Self {
                outcome: LockOutcome::Acquired,
                acquired: Cell::new(0),
                released: Cell::new(0),
            }
        }
        fn held() -> Self {
            Self {
                outcome: LockOutcome::HeldByAnother,
                acquired: Cell::new(0),
                released: Cell::new(0),
            }
        }
    }

    impl Lock for RecordingLock {
        fn acquire(&self) -> Result<LockOutcome, kernel::Error> {
            self.acquired.set(self.acquired.get() + 1);
            Ok(self.outcome)
        }
        fn release(&self) {
            self.released.set(self.released.get() + 1);
        }
    }

    struct RecordingSpawner {
        spawns: Cell<usize>,
    }

    impl Spawner for RecordingSpawner {
        fn spawn(&self) -> Result<Identity, kernel::Error> {
            self.spawns.set(self.spawns.get() + 1);
            Ok(Identity {
                pid: 9999,
                start_time: RecordedStartTime::Probe(1000),
                token: "t".to_owned(),
            })
        }
    }

    struct RecordingControl {
        signalled: RefCell<Vec<i32>>,
    }

    impl ProcessControl for RecordingControl {
        fn terminate(&self, pid: i32) {
            self.signalled.borrow_mut().push(pid);
        }
    }

    struct RecordingClient {
        ran: Rc<Cell<bool>>,
    }

    impl RunClient for RecordingClient {
        fn run(
            self: Box<Self>,
            _arguments: &[String],
        ) -> Result<Infallible, kernel::Error> {
            self.ran.set(true);
            Err(kernel::Error::Failed("client reached".to_owned()))
        }
    }

    struct Harness {
        clock: TickingClock,
        probe: FixedProbe,
        state: ScriptedStore,
        lock: RecordingLock,
        spawner: RecordingSpawner,
        control: RecordingControl,
    }

    impl Harness {
        fn new(
            reads: Vec<RecordedState>,
            observed: ObservedDaemon,
            lock: RecordingLock,
        ) -> Self {
            Self {
                clock: TickingClock { now: Cell::new(0) },
                probe: FixedProbe(observed),
                state: ScriptedStore::new(reads),
                lock,
                spawner: RecordingSpawner {
                    spawns: Cell::new(0),
                },
                control: RecordingControl {
                    signalled: RefCell::new(Vec::new()),
                },
            }
        }

        fn launcher(&self) -> Launcher<'_> {
            Launcher {
                clock: &self.clock,
                probe: &self.probe,
                state: &self.state,
                lock: &self.lock,
                spawner: &self.spawner,
                control: &self.control,
                bootstrap_log: "/state/server.bootstrap.log".to_owned(),
            }
        }
    }

    fn live_record() -> RecordedState {
        RecordedState::Daemon(RecordedDaemon {
            pid: LIVE_PID,
            start_time: RecordedStartTime::Probe(1000),
        })
    }

    fn matching() -> ObservedDaemon {
        ObservedDaemon::Live(ObservedStartTime::Known(1000))
    }

    /// Runs the sequence and reports both the outcome and whether the client
    /// was reached, which is the observable the reuse and refusal paths differ
    /// on.
    fn run(harness: &Harness) -> (Result<Infallible, LaunchFailure>, bool) {
        let ran = Rc::new(Cell::new(false));
        let client: Box<dyn RunClient> = Box::new(RecordingClient {
            ran: Rc::clone(&ran),
        });
        let outcome = harness.launcher().launch(&["ping".to_owned()], client);
        (outcome, ran.get())
    }

    /// Warm reuse: no lock taken, no spawn, straight to the client.
    #[test]
    fn a_reusable_daemon_is_reached_without_taking_the_lock() {
        let harness = Harness::new(
            vec![live_record()],
            matching(),
            RecordingLock::free(),
        );

        let (outcome, ran) = run(&harness);
        assert!(matches!(outcome, Err(LaunchFailure::Failed(_))));
        assert!(ran, "the client must be reached");
        assert_eq!(harness.lock.acquired.get(), 0);
        assert_eq!(harness.spawner.spawns.get(), 0);
        assert_eq!(
            harness.state.cleared.get(),
            0,
            "the pre-lock path must never remove state"
        );
    }

    /// Cold start: lock, clear, spawn, wait, release, client.
    #[test]
    fn a_cold_start_spawns_once_and_releases_the_lock() {
        // Absent for the pre-lock read and the under-lock re-check, then
        // present once the daemon publishes.
        let harness = Harness::new(
            vec![RecordedState::None, RecordedState::None, live_record()],
            matching(),
            RecordingLock::free(),
        );

        let (outcome, ran) = run(&harness);
        assert!(matches!(outcome, Err(LaunchFailure::Failed(_))));
        assert!(ran);
        assert_eq!(harness.lock.acquired.get(), 1);
        assert_eq!(harness.spawner.spawns.get(), 1);
        assert_eq!(harness.lock.released.get(), 1);
        assert!(harness.control.signalled.borrow().is_empty());
    }

    /// A stale record is removed only under the lock, and the previous
    /// daemon's stop reason goes with it so a failed start is not read as a
    /// completed shutdown.
    #[test]
    fn recovery_clears_the_record_and_the_stale_stop_reason_under_the_lock() {
        let harness = Harness::new(
            vec![live_record(), live_record(), live_record()],
            ObservedDaemon::Absent,
            RecordingLock::free(),
        );

        let _ = run(&harness);
        assert_eq!(harness.state.cleared.get(), 1);
        assert_eq!(harness.state.stop_reason_cleared.get(), 1);
    }

    /// The PID-recycle case reaches recovery, and signals nothing on the way.
    #[test]
    fn a_recycled_pid_recovers_without_signalling_the_live_process() {
        let harness = Harness::new(
            vec![live_record(), live_record(), live_record()],
            ObservedDaemon::Live(ObservedStartTime::Known(9999)),
            RecordingLock::free(),
        );

        let _ = run(&harness);
        assert_eq!(harness.spawner.spawns.get(), 1);
        assert!(
            !harness.control.signalled.borrow().contains(&LIVE_PID),
            "the process now holding the recorded pid must not be signalled"
        );
    }

    #[test]
    fn the_loser_of_a_contended_lock_reports_another_launcher_running(
    ) -> Result<(), String> {
        let harness = Harness::new(
            vec![RecordedState::None],
            ObservedDaemon::Absent,
            RecordingLock::held(),
        );

        let (outcome, ran) = run(&harness);
        let Err(LaunchFailure::Envelope(error)) = outcome else {
            return Err("expected an envelope".to_owned());
        };
        assert_eq!(error, LauncherError::AnotherLauncherRunning);
        assert!(!ran, "the client must not be reached");
        assert_eq!(harness.spawner.spawns.get(), 0);
        Ok(())
    }

    /// A daemon spawned between our read and our acquisition is reused rather
    /// than duplicated — and the lock is released before the handover.
    #[test]
    fn a_daemon_that_appears_under_the_lock_is_reused_not_duplicated() {
        let harness = Harness::new(
            vec![RecordedState::None, live_record()],
            matching(),
            RecordingLock::free(),
        );

        let _ = run(&harness);
        assert_eq!(harness.spawner.spawns.get(), 0, "no second daemon");
        assert_eq!(harness.lock.released.get(), 1);
        assert_eq!(harness.state.cleared.get(), 0);
    }

    /// A daemon that never publishes is killed, the lock released, and the
    /// timeout envelope names the bootstrap log.
    #[test]
    fn a_daemon_that_never_becomes_ready_is_killed_and_reported(
    ) -> Result<(), String> {
        let harness = Harness::new(
            vec![RecordedState::None],
            ObservedDaemon::Absent,
            RecordingLock::free(),
        );

        let (outcome, ran) = run(&harness);
        let Err(LaunchFailure::Envelope(error)) = outcome else {
            return Err("expected an envelope".to_owned());
        };
        assert_eq!(
            error,
            LauncherError::DaemonStartTimeout {
                bootstrap_log: "/state/server.bootstrap.log".to_owned()
            }
        );
        assert_eq!(
            *harness.control.signalled.borrow(),
            vec![9999],
            "the launcher's own just-spawned child is the only thing signalled"
        );
        assert_eq!(harness.lock.released.get(), 1);
        assert!(!ran);
        assert!(
            harness.clock.now_seconds() >= START_TIMEOUT_SECONDS,
            "the deadline must be reached by clock arithmetic, not sleeping"
        );
        Ok(())
    }
}
