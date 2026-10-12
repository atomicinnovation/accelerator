//! The production command runner: `bash -c` with nothing of the repository's
//! choosing in reach, and nothing it starts left running.
//!
//! The command runs in a fresh temporary directory outside the repository,
//! with only its policy's environment and a `PATH` stripped of every entry the
//! repository could control. It gets its own process group, so the whole group
//! can be torn down: on its leader's exit, on the deadline, past the output
//! cap, or on an interrupt to the CLI. stdout and stderr are read on the
//! calling thread against one byte counter, so no pipe is closed under a
//! blocked reader and the runner returns by its deadline even when a
//! descendant holds a pipe open.
//!
//! A descendant that calls `setsid` leaves the group and escapes the teardown;
//! it costs at most the grace period, never the timeout.

mod interrupts;

use std::io;
use std::io::Read;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::process::CommandExt as _;
use std::os::unix::process::ExitStatusExt as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::ChildStderr;
use std::process::ChildStdout;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use config::consent::CommandFailure;
use config::consent::CommandPolicy;
use config::consent::CommandRunner;
use config::consent::Environment;
use config::consent::FailureCause;
use config::consent::RepositoryRoots;
use config::consent::StartFailure;
use rustix::event::PollFd;
use rustix::event::PollFlags;
use rustix::io::Errno;
use rustix::process::Pid;
use rustix::process::Signal;
use rustix::process::WaitId;
use rustix::process::WaitIdOptions;
use tempfile::TempDir;

use crate::command_runner::interrupts::ActiveRun;

const TICK: Duration = Duration::from_millis(20);
const GRACE: Duration = Duration::from_secs(1);
const LOCATOR_VARIABLES: [&str; 3] =
    ["XDG_CONFIG_HOME", "XDG_RUNTIME_DIR", "GH_CONFIG_DIR"];

pub struct BashCommandRunner {
    roots: RepositoryRoots,
    parent: Box<dyn Environment>,
    temp_base: PathBuf,
}

impl BashCommandRunner {
    /// `roots` must be canonical. `temp_base` is where working directories
    /// are made, unless it lies inside `roots`.
    #[must_use]
    pub fn new(
        roots: RepositoryRoots,
        parent: Box<dyn Environment>,
        temp_base: PathBuf,
    ) -> Self {
        Self {
            roots,
            parent,
            temp_base,
        }
    }

    /// Whether the repository could have chosen `path`: anything but an
    /// absolute path that exists outside every root, judged both as written
    /// and resolved.
    fn inside_the_repository(&self, path: &Path) -> bool {
        path.is_relative()
            || self.roots.contains(path)
            || std::fs::canonicalize(path)
                .map_or(true, |resolved| self.roots.contains(&resolved))
    }

    fn search_path(&self) -> Vec<String> {
        self.parent.read("PATH").map_or_else(Vec::new, |path| {
            path.split(':')
                .filter(|entry| !self.inside_the_repository(Path::new(entry)))
                .map(str::to_owned)
                .collect()
        })
    }

    fn environment(
        &self,
        policy: &CommandPolicy,
        search_path: &[String],
    ) -> Vec<(&'static str, String)> {
        policy
            .admitted_environment()
            .iter()
            .filter_map(|name| {
                let value = if *name == "PATH" {
                    Some(search_path.join(":"))
                } else {
                    self.parent.read(name)
                }?;
                let locator = LOCATOR_VARIABLES.contains(name);
                (!locator || !self.inside_the_repository(Path::new(&value)))
                    .then_some((*name, value))
            })
            .collect()
    }

    fn working_directory(&self) -> Result<TempDir, StartFailure> {
        [self.temp_base.as_path(), Path::new("/tmp")]
            .into_iter()
            .filter_map(|base| fresh_directory_in(base).ok())
            .find(|directory| !self.inside_the_repository(directory.path()))
            .ok_or(StartFailure::NoWorkingDirectoryOutsideTheRepository)
    }
}

fn fresh_directory_in(base: &Path) -> io::Result<TempDir> {
    tempfile::Builder::new()
        .prefix("accelerator-cmd-")
        .tempdir_in(base)
}

fn bash_on(search_path: &[String]) -> Option<PathBuf> {
    search_path
        .iter()
        .map(|entry| Path::new(entry).join("bash"))
        .find(|candidate| {
            candidate.metadata().is_ok_and(|facts| {
                facts.is_file() && facts.permissions().mode() & 0o111 != 0
            })
        })
}

const fn could_not_start(failure: StartFailure) -> CommandFailure {
    CommandFailure::Failed(FailureCause::CouldNotStart(failure))
}

impl CommandRunner for BashCommandRunner {
    fn run(
        &self,
        command: &str,
        policy: &CommandPolicy,
    ) -> Result<String, CommandFailure> {
        let run = ActiveRun::begin();
        let working_directory =
            self.working_directory().map_err(could_not_start)?;
        let search_path = self.search_path();
        let bash = bash_on(&search_path)
            .ok_or(could_not_start(StartFailure::NoBashOnPath))?;
        let child = Command::new(bash)
            .arg("-c")
            .arg(command)
            .current_dir(working_directory.path())
            .env_clear()
            .envs(self.environment(policy, &search_path))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .map_err(|_| could_not_start(StartFailure::SpawnFailed))?;
        Supervision::of(child, policy.timeout()).conclude(&run)
    }
}

/// The two pipes of one command, read against one byte counter.
struct Output {
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    captured: Vec<u8>,
    total: usize,
}

enum Stream {
    Stdout,
    Stderr,
}

impl Output {
    const fn exceeded(&self) -> bool {
        self.total > CommandPolicy::OUTPUT_LIMIT
    }

    /// Waits up to `wait` for either pipe, then reads once from each that is
    /// ready. A pipe at end of file, or in error, is dropped.
    fn drain_for(&mut self, wait: Duration) {
        let ready = self.ready_within(wait);
        for (stream, flags) in ready {
            if flags.is_empty() {
                continue;
            }
            let mut buffer = [0_u8; 8192];
            let read = match stream {
                Stream::Stdout => {
                    self.stdout.as_mut().map(|s| s.read(&mut buffer))
                }
                Stream::Stderr => {
                    self.stderr.as_mut().map(|s| s.read(&mut buffer))
                }
            };
            match read {
                None => {}
                Some(Ok(0)) => self.close(&stream),
                Some(Ok(count)) => self.account(&stream, &buffer[..count]),
                Some(Err(error))
                    if error.kind() == io::ErrorKind::Interrupted => {}
                Some(Err(_)) => self.close(&stream),
            }
        }
    }

    fn ready_within(&self, wait: Duration) -> Vec<(Stream, PollFlags)> {
        let interest = PollFlags::IN;
        let mut fds = Vec::with_capacity(2);
        let mut streams = Vec::with_capacity(2);
        if let Some(stdout) = &self.stdout {
            fds.push(PollFd::new(stdout, interest));
            streams.push(Stream::Stdout);
        }
        if let Some(stderr) = &self.stderr {
            fds.push(PollFd::new(stderr, interest));
            streams.push(Stream::Stderr);
        }
        if fds.is_empty() {
            std::thread::sleep(wait);
            return Vec::new();
        }
        let timeout = rustix::event::Timespec::try_from(wait).ok();
        if rustix::event::poll(&mut fds, timeout.as_ref()).is_err() {
            return Vec::new();
        }
        let flags: Vec<PollFlags> = fds.iter().map(PollFd::revents).collect();
        streams.into_iter().zip(flags).collect()
    }

    fn close(&mut self, stream: &Stream) {
        match stream {
            Stream::Stdout => self.stdout = None,
            Stream::Stderr => self.stderr = None,
        }
    }

    fn account(&mut self, stream: &Stream, bytes: &[u8]) {
        self.total = self.total.saturating_add(bytes.len());
        if matches!(stream, Stream::Stdout) && !self.exceeded() {
            self.captured.extend_from_slice(bytes);
        }
    }

    const fn drained(&self) -> bool {
        self.stdout.is_none() && self.stderr.is_none()
    }

    fn close_all(&mut self) {
        self.stdout = None;
        self.stderr = None;
    }

    fn value(&self) -> String {
        String::from_utf8_lossy(&self.captured).trim().to_owned()
    }
}

/// Why the supervision loop stopped watching a running command.
enum Ending {
    LeaderExited,
    DeadlinePassed,
    OutputExceeded,
    Interrupted,
}

struct Supervision {
    leader: Child,
    group: Pid,
    output: Output,
    deadline: Instant,
    reaped: Option<ExitStatus>,
}

impl Supervision {
    fn of(mut leader: Child, timeout: Duration) -> Self {
        let group = Pid::from_child(&leader);
        let output = Output {
            stdout: leader.stdout.take(),
            stderr: leader.stderr.take(),
            captured: Vec::new(),
            total: 0,
        };
        Self {
            leader,
            group,
            output,
            deadline: Instant::now() + timeout,
            reaped: None,
        }
    }

    fn conclude(mut self, run: &ActiveRun) -> Result<String, CommandFailure> {
        match self.watch(run) {
            Ending::DeadlinePassed => {
                self.kill_now();
                Err(CommandFailure::TimedOut)
            }
            Ending::OutputExceeded => {
                self.kill_now();
                Err(CommandFailure::OutputExceeded)
            }
            Ending::LeaderExited | Ending::Interrupted => {
                self.signal_group(Signal::TERM);
                self.reap_if_exited();
                self.grace_period();
                self.output.close_all();
                let status = self.reap();
                self.verdict(status)
            }
        }
    }

    fn watch(&mut self, run: &ActiveRun) -> Ending {
        loop {
            self.output.drain_for(self.tick());
            if self.output.exceeded() {
                return Ending::OutputExceeded;
            }
            if run.interrupted() {
                return Ending::Interrupted;
            }
            if self.leader_exited() {
                return Ending::LeaderExited;
            }
            if Instant::now() >= self.deadline {
                return Ending::DeadlinePassed;
            }
        }
    }

    fn grace_period(&mut self) {
        let end = (Instant::now() + GRACE).min(self.deadline);
        loop {
            self.reap_if_exited();
            if self.reaped.is_some()
                && self.group_is_empty()
                && self.output.drained()
            {
                return;
            }
            if Instant::now() >= end {
                self.signal_group(Signal::KILL);
                return;
            }
            let remaining = end.saturating_duration_since(Instant::now());
            self.output.drain_for(TICK.min(remaining));
        }
    }

    fn kill_now(&mut self) {
        self.signal_group(Signal::KILL);
        self.output.close_all();
        self.reap();
    }

    fn tick(&self) -> Duration {
        TICK.min(self.deadline.saturating_duration_since(Instant::now()))
    }

    /// Observes the leader's exit without reaping it, so its pid, and so the
    /// group id, cannot be reused while the group is still signalled.
    fn leader_exited(&self) -> bool {
        self.reaped.is_some()
            || rustix::process::waitid(
                WaitId::Pid(self.group),
                WaitIdOptions::EXITED
                    | WaitIdOptions::NOWAIT
                    | WaitIdOptions::NOHANG,
            )
            .is_ok_and(|status| status.is_some())
    }

    fn reap_if_exited(&mut self) {
        if self.reaped.is_none() && self.leader_exited() {
            self.reap();
        }
    }

    fn reap(&mut self) -> Option<ExitStatus> {
        if self.reaped.is_none() {
            self.reaped = self.leader.wait().ok();
        }
        self.reaped
    }

    fn group_is_empty(&self) -> bool {
        rustix::process::test_kill_process_group(self.group) == Err(Errno::SRCH)
    }

    fn signal_group(&self, signal: Signal) {
        let _ = rustix::process::kill_process_group(self.group, signal);
    }

    fn verdict(
        &self,
        status: Option<ExitStatus>,
    ) -> Result<String, CommandFailure> {
        if self.output.exceeded() {
            return Err(CommandFailure::OutputExceeded);
        }
        match status.map(exit_code) {
            Some(0) => Ok(self.output.value()),
            Some(code) => {
                Err(CommandFailure::Failed(FailureCause::Exited(code)))
            }
            None => Err(could_not_start(StartFailure::SpawnFailed)),
        }
    }
}

fn exit_code(status: ExitStatus) -> i32 {
    status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::*;

    fn burst(bytes: usize, stream: &str) -> String {
        let redirect = if stream == "stderr" { " >&2" } else { "" };
        format!(
            "printf %s \"$(head -c {bytes} /dev/zero | tr '\\0' a)\"{redirect}; "
        )
    }

    fn exited_with_output_unread(command: &str) -> Child {
        let child = Command::new("bash")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .expect("spawn bash");
        let started = Instant::now();
        while rustix::process::waitid(
            WaitId::Pid(Pid::from_child(&child)),
            WaitIdOptions::EXITED
                | WaitIdOptions::NOWAIT
                | WaitIdOptions::NOHANG,
        )
        .expect("waitid")
        .is_none()
        {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "the command blocked on a full pipe"
            );
            std::thread::sleep(TICK);
        }
        child
    }

    fn concluded(command: &str) -> Result<String, CommandFailure> {
        let run = ActiveRun::begin();
        Supervision::of(
            exited_with_output_unread(command),
            Duration::from_secs(5),
        )
        .conclude(&run)
    }

    #[test]
    fn output_left_in_the_pipe_by_an_exited_command_is_returned() {
        assert_eq!(
            concluded(&burst(16_000, "stdout")).map(|value| value.len()),
            Ok(16_000)
        );
    }

    #[test]
    fn output_left_in_the_pipes_by_an_exited_command_counts_towards_the_cap() {
        assert_eq!(
            concluded(&format!(
                "{}{}",
                burst(40_000, "stdout"),
                burst(30_000, "stderr")
            )),
            Err(CommandFailure::OutputExceeded)
        );
    }
}
