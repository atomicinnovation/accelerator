//! Process-identity and termination primitives for the lifecycle commands.
//!
//! The detached daemon is not a child of the `stop` invocation, so termination
//! polls `kill(pid, 0)` rather than `waitpid` (which returns `ECHILD` for a
//! non-child). Identity is keyed on the pid's start-time so a recycled pid is
//! never signalled.

use std::thread::sleep;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

pub(crate) use crate::server::process_start_time;

/// Whether the process `pid` is currently alive.
#[must_use]
pub fn is_alive(pid: i32) -> bool {
    match kill(Pid::from_raw(pid), None) {
        Ok(()) | Err(Errno::EPERM) => true,
        Err(_) => false,
    }
}

/// Whether `pid` is alive **and** its start-time matches `expected`. A recorded
/// start-time that can no longer be read is treated as a mismatch (conservative,
/// so a recycled pid is never mistaken for the original).
#[must_use]
pub fn identity_matches(pid: i32, expected: Option<u64>) -> bool {
    if !is_alive(pid) {
        return false;
    }
    match expected {
        None => true,
        Some(expected) => process_start_time(pid) == Some(expected),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    AlreadyDead,
    Terminated,
    Forced,
    Failed,
}

/// SIGTERM, poll for exit up to `grace`, then escalate to SIGKILL.
pub fn terminate(pid: i32, grace: Duration, tick: Duration) -> Termination {
    if !is_alive(pid) {
        return Termination::AlreadyDead;
    }
    let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
    if wait_for_exit(pid, grace, tick) {
        return Termination::Terminated;
    }
    let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
    if wait_for_exit(pid, tick.saturating_mul(5), tick) {
        Termination::Forced
    } else {
        Termination::Failed
    }
}

fn wait_for_exit(pid: i32, budget: Duration, tick: Duration) -> bool {
    let deadline = Instant::now() + budget;
    loop {
        if !is_alive(pid) {
            return true;
        }
        if Instant::now() >= deadline {
            return !is_alive(pid);
        }
        sleep(tick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_process_is_alive() {
        assert!(is_alive(std::process::id() as i32));
    }

    #[test]
    fn identity_matches_own_process() {
        let me = std::process::id() as i32;
        let start = process_start_time(me);
        assert!(identity_matches(me, start));
        if let Some(start) = start {
            assert!(!identity_matches(me, Some(start.wrapping_add(1))));
        }
    }

    #[test]
    fn terminate_reports_already_dead_for_unused_pid() {
        // PID 2^31-1 is not a live process on any supported target.
        assert_eq!(
            terminate(
                i32::MAX,
                Duration::from_millis(10),
                Duration::from_millis(5)
            ),
            Termination::AlreadyDead
        );
    }

    #[test]
    fn terminate_kills_a_sleeping_child() {
        let child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id() as i32;
        // Reap concurrently: a direct child that is signalled but not waited on
        // lingers as a zombie, which `kill(pid, 0)` still reports as alive. A
        // detached daemon (the real `stop` target) is reparented to init, which
        // reaps it, so this concern is a test-only artefact.
        let reaper = std::thread::spawn(move || {
            let mut child = child;
            let _ = child.wait();
        });
        let outcome = terminate(
            pid,
            Duration::from_millis(500),
            Duration::from_millis(20),
        );
        reaper.join().unwrap();
        assert!(
            matches!(outcome, Termination::Terminated | Termination::Forced),
            "got {outcome:?}"
        );
        assert!(!is_alive(pid));
    }
}
