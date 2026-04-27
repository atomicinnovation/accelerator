//! 60s interval loop that fires shutdown on owner-PID death or
//! prolonged idleness.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::activity::Activity;
use crate::shutdown::ShutdownReason;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub tick: Duration,
    pub idle_limit_ms: i64,
}

impl Settings {
    /// Production defaults: 60s tick, 30-minute idle window.
    /// Tests pass shortened values via `Settings { tick: 50ms,
    /// idle_limit_ms: 200 }` without any test-only conditional
    /// in the module itself.
    pub const DEFAULT: Settings = Settings {
        tick: Duration::from_secs(60),
        idle_limit_ms: 30 * 60 * 1000,
    };
}

pub fn spawn(
    activity: Arc<Activity>,
    owner_pid: i32,
    owner_start_time: Option<u64>,
    settings: Settings,
    tx: mpsc::Sender<ShutdownReason>,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(settings.tick);
        ticker.tick().await; // drop the immediate tick.
        loop {
            ticker.tick().await;
            if owner_pid > 0 && !owner_alive(owner_pid, owner_start_time) {
                let _ = tx.send(ShutdownReason::OwnerPidExited).await;
                return;
            }
            let idle = now_millis() - activity.last_millis();
            if idle >= settings.idle_limit_ms {
                let _ = tx.send(ShutdownReason::IdleTimeout).await;
                return;
            }
        }
    });
}

/// True if the process identified by `pid` is still alive **and**,
/// if `expected_start_time` is provided, still has the same
/// start-time stamp. The start-time cross-check defends against PID
/// reuse — a recycled PID will not have the same start-time as the
/// process we originally recorded. When `expected_start_time` is
/// `None`, falls back to a bare PID probe.
pub(crate) fn owner_alive(pid: i32, expected_start_time: Option<u64>) -> bool {
    use nix::errno::Errno;
    use nix::unistd::Pid;
    let probe = match nix::sys::signal::kill(Pid::from_raw(pid), None) {
        Ok(()) => true,
        Err(Errno::EPERM) => true, // exists, we just can't signal it
        Err(_) => false,           // ESRCH or similar — gone
    };
    if !probe {
        return false;
    }
    match expected_start_time {
        Some(expected) => match crate::server::process_start_time(pid) {
            Some(current) => current == expected,
            // If we recorded a start-time at launch but can't obtain
            // one now, treat it as identity-mismatch (conservative).
            None => false,
        },
        None => true,
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_pid_is_alive() {
        let me = std::process::id() as i32;
        assert!(owner_alive(me, None));
    }

    #[tokio::test]
    async fn reaped_child_pid_is_dead() {
        let child = tokio::process::Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn()
            .unwrap();
        let pid = child.id().unwrap() as i32;
        let _ = child.wait_with_output().await;
        assert!(!owner_alive(pid, None));
    }

    #[test]
    fn start_time_mismatch_treats_pid_as_dead() {
        let me = std::process::id() as i32;
        let real_start = crate::server::process_start_time(me);
        if real_start.is_none() {
            // Platform without start-time support; skip.
            return;
        }
        let wrong = real_start.unwrap().wrapping_add(1);
        assert!(!owner_alive(me, Some(wrong)));
        assert!(owner_alive(me, real_start));
    }
}
