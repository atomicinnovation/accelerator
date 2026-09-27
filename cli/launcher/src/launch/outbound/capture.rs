//! The Unix capture adapter: runs a sub-binary as a child in its own process
//! group and reads its stdout against a deadline.
//!
//! stdout is read with `poll`, and the leader's exit is observed without
//! reaping it, so a descendant that holds stdout open cannot stretch the call
//! past the deadline. The whole group is killed before the leader is reaped,
//! on every exit path, so the signal never reaches a reused process group.
//!
//! The deadline binds only while the launcher lives. A launcher killed
//! mid-capture, as a hook can be, leaves the child running to its own end.

use std::ffi::OsString;
use std::io;
use std::io::Read as _;
use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::process::Child;
use std::process::ChildStdout;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use rustix::event::PollFd;
use rustix::event::PollFlags;
use rustix::process::Pid;
use rustix::process::Signal;
use rustix::process::WaitId;
use rustix::process::WaitIdOptions;

use crate::launch::core::{CaptureBinary, CaptureFailure, Captured};

/// The most stdout a capture holds.
pub const CAPTURE_LIMIT: usize = 4096;

const TICK: Duration = Duration::from_millis(20);

pub struct UnixCapture;

impl CaptureBinary for UnixCapture {
    fn capture(
        &self,
        program: &Path,
        args: &[OsString],
        deadline: Duration,
    ) -> Result<Captured, CaptureFailure> {
        let leader = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|error| CaptureFailure::CouldNotSpawn {
                program: program.to_path_buf(),
                detail: error.to_string(),
            })?;
        Supervision::of(leader, deadline).conclude()
    }
}

/// Why the supervision loop stopped watching the child.
enum Ending {
    LeaderExited,
    DeadlinePassed,
    OutputExceeded,
}

struct Supervision {
    leader: Child,
    group: Pid,
    stdout: Option<ChildStdout>,
    captured: Vec<u8>,
    deadline: Instant,
}

impl Supervision {
    fn of(mut leader: Child, deadline: Duration) -> Self {
        let group = Pid::from_child(&leader);
        let stdout = leader.stdout.take();
        Self {
            leader,
            group,
            stdout,
            captured: Vec::new(),
            deadline: Instant::now() + deadline,
        }
    }

    fn conclude(mut self) -> Result<Captured, CaptureFailure> {
        let ending = self.watch();
        let _ = rustix::process::kill_process_group(self.group, Signal::KILL);
        self.stdout = None;
        let status = self.leader.wait();
        match ending {
            Ending::DeadlinePassed => Err(CaptureFailure::TimedOut),
            Ending::OutputExceeded => Err(CaptureFailure::OutputExceeded {
                limit: CAPTURE_LIMIT,
            }),
            Ending::LeaderExited => Ok(Captured {
                succeeded: status.is_ok_and(|status| status.success()),
                stdout: self.captured,
            }),
        }
    }

    fn watch(&mut self) -> Ending {
        loop {
            self.read_within(self.tick());
            if self.exceeded() {
                return Ending::OutputExceeded;
            }
            if self.leader_exited() {
                self.drain_what_is_buffered();
                if self.exceeded() {
                    return Ending::OutputExceeded;
                }
                return Ending::LeaderExited;
            }
            if Instant::now() >= self.deadline {
                return Ending::DeadlinePassed;
            }
        }
    }

    fn tick(&self) -> Duration {
        TICK.min(self.deadline.saturating_duration_since(Instant::now()))
    }

    const fn exceeded(&self) -> bool {
        self.captured.len() > CAPTURE_LIMIT
    }

    fn drain_what_is_buffered(&mut self) {
        while self.stdout.is_some() && !self.exceeded() {
            if !self.read_within(Duration::ZERO) {
                return;
            }
        }
    }

    /// Waits up to `wait` for stdout, then reads from it once. Returns
    /// whether it was ready; a pipe at end of file, or in error, is dropped.
    fn read_within(&mut self, wait: Duration) -> bool {
        let Some(stdout) = self.stdout.as_mut() else {
            std::thread::sleep(wait);
            return false;
        };
        let mut fds = [PollFd::new(&*stdout, PollFlags::IN)];
        let timeout = rustix::event::Timespec::try_from(wait).ok();
        if rustix::event::poll(&mut fds, timeout.as_ref()).is_err()
            || fds[0].revents().is_empty()
        {
            return false;
        }
        let mut buffer = [0_u8; 4096];
        match stdout.read(&mut buffer) {
            Ok(0) => self.stdout = None,
            Ok(count) => self.captured.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => self.stdout = None,
        }
        true
    }

    /// Observes the leader's exit without reaping it, so its pid, and so the
    /// group id, cannot be reused before the group is killed.
    fn leader_exited(&self) -> bool {
        rustix::process::waitid(
            WaitId::Pid(self.group),
            WaitIdOptions::EXITED
                | WaitIdOptions::NOWAIT
                | WaitIdOptions::NOHANG,
        )
        .is_ok_and(|status| status.is_some())
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::time::Instant;

    use rustix::io::Errno;
    use rustix::process::Pid;

    use super::{UnixCapture, CAPTURE_LIMIT};
    use crate::launch::core::{CaptureBinary, CaptureFailure, Captured};

    type TestError = Box<dyn std::error::Error>;

    const SH: &str = "/bin/sh";
    const DEADLINE: Duration = Duration::from_secs(2);

    fn script(body: &str) -> Vec<OsString> {
        vec![OsString::from("-c"), OsString::from(body)]
    }

    fn capture(
        body: &str,
        deadline: Duration,
    ) -> Result<Captured, CaptureFailure> {
        UnixCapture.capture(Path::new(SH), &script(body), deadline)
    }

    fn scratch() -> Result<tempfile::TempDir, TestError> {
        Ok(tempfile::Builder::new().prefix("capture-").tempdir()?)
    }

    fn recorded_pid(file: &Path) -> Result<Pid, TestError> {
        let started = Instant::now();
        loop {
            if let Ok(text) = std::fs::read_to_string(file) {
                if let Ok(raw) = text.trim().parse::<i32>() {
                    return Pid::from_raw(raw).ok_or_else(|| "pid 0".into());
                }
            }
            if started.elapsed() > DEADLINE {
                return Err("the grandchild never recorded its pid".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn gone(pid: Pid) -> bool {
        let started = Instant::now();
        while started.elapsed() < DEADLINE {
            if rustix::process::test_kill_process(pid) == Err(Errno::SRCH) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    fn pid_file(dir: &tempfile::TempDir) -> (PathBuf, String) {
        let file = dir.path().join("grandchild.pid");
        let quoted = format!("'{}'", file.display());
        (file, quoted)
    }

    #[test]
    fn stdout_is_captured_with_the_leaders_success() {
        assert_eq!(
            capture("printf untracked", DEADLINE),
            Ok(Captured {
                succeeded: true,
                stdout: b"untracked".to_vec(),
            })
        );
    }

    #[test]
    fn a_failing_leader_is_captured_as_unsuccessful() {
        assert!(matches!(
            capture("printf partial; exit 3", DEADLINE),
            Ok(Captured {
                succeeded: false,
                ..
            })
        ));
    }

    #[test]
    fn the_childs_stdin_is_at_end_of_file() {
        assert_eq!(
            capture(
                "if read line; then printf read; else printf eof; fi",
                DEADLINE
            )
            .map(|captured| captured.stdout),
            Ok(b"eof".to_vec())
        );
    }

    #[test]
    fn output_past_the_limit_is_refused() {
        let body = format!("head -c {} /dev/zero", CAPTURE_LIMIT + 1);

        assert_eq!(
            capture(&body, DEADLINE),
            Err(CaptureFailure::OutputExceeded {
                limit: CAPTURE_LIMIT
            })
        );
    }

    #[test]
    fn output_at_the_limit_is_captured() {
        let body = format!("head -c {CAPTURE_LIMIT} /dev/zero");

        assert_eq!(
            capture(&body, DEADLINE).map(|captured| captured.stdout.len()),
            Ok(CAPTURE_LIMIT)
        );
    }

    #[test]
    fn a_missing_program_could_not_spawn() {
        let missing = Path::new("/nonexistent/accelerator-vcs");

        assert!(matches!(
            UnixCapture.capture(missing, &[], DEADLINE),
            Err(CaptureFailure::CouldNotSpawn { program, .. })
                if program == missing
        ));
    }

    #[test]
    fn a_leader_outliving_the_deadline_is_killed_with_its_grandchild(
    ) -> Result<(), TestError> {
        let dir = scratch()?;
        let (file, quoted) = pid_file(&dir);
        let body = format!("sleep 30 & echo $! > {quoted}; wait");
        let started = Instant::now();

        let outcome = capture(&body, Duration::from_millis(500));

        assert_eq!(outcome, Err(CaptureFailure::TimedOut));
        assert!(started.elapsed() < DEADLINE);
        assert!(gone(recorded_pid(&file)?), "the grandchild survived");
        Ok(())
    }

    #[test]
    fn a_background_child_holding_stdout_cannot_stretch_the_capture(
    ) -> Result<(), TestError> {
        let dir = scratch()?;
        let (file, quoted) = pid_file(&dir);
        let body =
            format!("printf untracked; sleep 30 & echo $! > {quoted}; exit 0");
        let started = Instant::now();

        let outcome = capture(&body, DEADLINE);

        assert!(started.elapsed() < DEADLINE);
        assert_eq!(
            outcome,
            Ok(Captured {
                succeeded: true,
                stdout: b"untracked".to_vec(),
            })
        );
        assert!(gone(recorded_pid(&file)?), "the background child survived");
        Ok(())
    }
}
