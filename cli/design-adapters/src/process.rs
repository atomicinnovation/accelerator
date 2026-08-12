//! Spawning, signalling, probing and finally replacing the process image.
//!
//! This module spawns by design, which is why `cli/pup.ron` scopes its
//! no-subprocess rule to the sibling `filesystem` and `environment` modules
//! rather than to the crate.
//!
//! Three properties come from shell primitives the port removes, and each is
//! reproduced explicitly rather than inherited. `nohup … & disown` made the
//! daemon SIGHUP-immune and reparented, so the spawn calls `setsid`; without it
//! a Ctrl-C in an interactive session kills the daemon mid-crawl.
//! `>>"$BOOTSTRAP_LOG" 2>&1` kept the daemon's chatter out of the caller's
//! streams, so the spawn redirects rather than inheriting. And `umask 077` was
//! inherited by the daemon too, not only the launcher — every file the daemon
//! creates without an explicit mode, notably screenshot output, lands `0600`
//! today — so the child's `pre_exec` sets it as well.
//!
//! It is deliberately **not** a double fork. The shell spawned the daemon as a
//! direct child, so `$!` was the daemon's own pid, which both the
//! kill-on-timeout and the identity handoff need. `setsid` before `exec`
//! preserves that pid; a double fork does not, because the launcher never
//! learns the grandchild's.

use std::fmt::Write as _;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::os::fd::AsRawFd as _;
use std::os::fd::FromRawFd as _;
use std::os::fd::OwnedFd;
use std::os::unix::fs::OpenOptionsExt as _;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use design::executor::daemon_identity::ObservedDaemon;
use design::executor::daemon_identity::ObservedStartTime;
use design::executor::daemon_identity::RecordedStartTime;
use design::executor::handoff::Identity;
use design::executor::ports::ProcessControl;
use design::executor::ports::ProcessProbe;
use design::executor::ports::Spawner;

/// The mode every file the launcher and the daemon create inherits.
const OWNER_ONLY_UMASK: libc::mode_t = 0o077;

/// The descriptor the identity pipe's read end is placed on in the child.
///
/// A fixed number the child is told about by name, rather than an inherited
/// stdio slot, so nothing the daemon does to its own streams disturbs it.
pub const IDENTITY_FD: libc::c_int = 3;

/// The variable naming [`IDENTITY_FD`] to the child.
pub const IDENTITY_FD_VAR: &str = "ACCELERATOR_PLAYWRIGHT_IDENTITY_FD";

/// Liveness and start time, over the shared epoch read.
#[derive(Debug, Clone, Copy, Default)]
pub struct HostProbe;

impl ProcessProbe for HostProbe {
    fn observe(&self, pid: i32) -> ObservedDaemon {
        if !is_alive(pid) {
            return ObservedDaemon::Absent;
        }
        ObservedDaemon::Live(
            process_probe::start_time(pid).map_or(
                ObservedStartTime::Unavailable,
                ObservedStartTime::Known,
            ),
        )
    }
}

/// `kill(pid, 0)`: permission-denied still proves the process exists.
fn is_alive(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    let result = unsafe { libc::kill(pid, 0) };
    if result == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// SIGTERM, and only ever to the launcher's own just-spawned child.
///
/// Recovery never signals — see the reuse verdict — so nothing else reaches
/// this.
#[derive(Debug, Clone, Copy, Default)]
pub struct HostControl;

impl ProcessControl for HostControl {
    fn terminate(&self, pid: i32) {
        if pid > 0 {
            unsafe { libc::kill(pid, libc::SIGTERM) };
        }
    }
}

/// Everything the daemon spawn needs.
pub struct DaemonSpawner {
    /// The interpreter and script, plus the daemon subcommand and its flags.
    pub program: PathBuf,
    pub arguments: Vec<String>,
    /// Redirect target for the child's stdout and stderr.
    pub bootstrap_log: PathBuf,
    pub environment: Vec<(String, String)>,
}

impl Spawner for DaemonSpawner {
    fn spawn(&self) -> Result<Identity, kernel::Error> {
        let log = self.prepare_bootstrap_log()?;
        let (read_end, mut write_end) = identity_pipe()?;

        let mut command = Command::new(&self.program);
        command.args(&self.arguments);
        command.stdin(std::process::Stdio::null());
        command
            .stdout(log.try_clone().map_err(failed("clone the log handle"))?);
        command.stderr(log);
        for (name, value) in &self.environment {
            command.env(name, value);
        }
        command.env(IDENTITY_FD_VAR, IDENTITY_FD.to_string());

        let read_fd = read_end.as_raw_fd();
        // Runs in the forked child, before exec, so only async-signal-safe
        // calls are permitted — which is why these are raw libc rather than a
        // safe wrapper.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() == -1 {
                    // Reported rather than fallen back to a double fork: a
                    // daemon in the caller's session dies with a Ctrl-C.
                    return Err(std::io::Error::last_os_error());
                }
                libc::umask(OWNER_ONLY_UMASK);
                // Placing the read end on a fixed descriptor clears
                // O_CLOEXEC on the duplicate, so it survives into `node`.
                if libc::dup2(read_fd, IDENTITY_FD) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let child = command.spawn().map_err(failed("spawn the daemon"))?;
        let pid = i32::try_from(child.id()).map_err(|_| {
            kernel::Error::Failed("daemon pid overflowed".into())
        })?;

        // The child's inherited copy of the write end closed at its own exec,
        // so once this scope drops ours no writable copy remains anywhere and
        // the daemon's read reaches EOF deterministically rather than blocking.
        drop(read_end);

        let identity = Identity {
            pid,
            // Observed with the same probe the reuse check will later use, so
            // the two can never disagree about what they are measuring.
            start_time: process_probe::start_time(pid).map_or(
                RecordedStartTime::WriterUnavailable,
                RecordedStartTime::Probe,
            ),
            token: generate_token(),
        };
        write_end
            .write_all(identity.render().as_bytes())
            .map_err(failed("write the identity handoff"))?;
        write_end.flush().map_err(failed("flush the handoff"))?;
        drop(write_end);

        Ok(identity)
    }
}

impl DaemonSpawner {
    /// Truncated and `0600` before the daemon writes, so the timeout envelope's
    /// "check this log" points at *this* attempt's output rather than a
    /// previous one's, under a umask the Rust binary does not inherit from a
    /// shell.
    fn prepare_bootstrap_log(&self) -> Result<File, kernel::Error> {
        let log = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&self.bootstrap_log)
            .map_err(failed("open the bootstrap log"))?;
        // `mode` applies only when the file is created, so an existing log
        // keeps whatever permissions it had — which the shell's unconditional
        // `chmod 0600` did not allow. The daemon's stderr lands here, so a
        // world-readable log is a disclosure route for anything it prints.
        log.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(failed("restrict the bootstrap log"))?;
        Ok(log)
    }
}

/// The pipe carrying the identity record.
///
/// The write end is `O_CLOEXEC` from the start, so the child's inherited copy
/// closes as a side effect of the kernel performing `exec`, leaving the daemon
/// holding only the read side.
fn identity_pipe() -> Result<(OwnedFd, File), kernel::Error> {
    let mut ends: [libc::c_int; 2] = [-1, -1];
    // `pipe` rather than `pipe2`: Darwin has no `pipe2`, so the flag is set
    // afterwards. The window that opens in a multithreaded process does not
    // exist here — the launcher is single-threaded until it spawns.
    if unsafe { libc::pipe(ends.as_mut_ptr()) } != 0 {
        return Err(kernel::Error::Failed(format!(
            "could not create the identity pipe: {}",
            std::io::Error::last_os_error()
        )));
    }
    let read_end = unsafe { OwnedFd::from_raw_fd(ends[0]) };
    let write_end = unsafe { File::from_raw_fd(ends[1]) };
    for end in [read_end.as_raw_fd(), write_end.as_raw_fd()] {
        if unsafe { libc::fcntl(end, libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
            return Err(kernel::Error::Failed(format!(
                "could not close-on-exec the identity pipe: {}",
                std::io::Error::last_os_error()
            )));
        }
    }
    Ok((read_end, write_end))
}

/// A 128-bit CSPRNG value, hex-encoded.
fn generate_token() -> String {
    use rand::TryRngCore as _;

    let mut bytes = [0u8; 16];
    // An OS entropy failure is not something to paper over with a weaker
    // source, so it panics rather than degrading a security control silently.
    // This is the one place a fallback would be worse than a crash.
    rand::rngs::OsRng
        .try_fill_bytes(&mut bytes)
        .unwrap_or_else(|error| {
            unreachable!("the OS refused to supply entropy: {error}")
        });

    bytes
        .iter()
        .fold(String::with_capacity(32), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn failed(what: &'static str) -> impl FnOnce(std::io::Error) -> kernel::Error {
    move |error| kernel::Error::Failed(format!("could not {what}: {error}"))
}

/// Replaces the process image with the client, so its exit status, streams and
/// signal death *are* the launcher's with no forwarding logic.
///
/// The skill's error discrimination depends on that, and forwarding it by hand
/// would be a second place for the mapping to drift.
pub struct ExecClient {
    pub program: PathBuf,
    pub leading_arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
}

impl design::executor::ports::RunClient for ExecClient {
    fn run(
        self: Box<Self>,
        arguments: &[String],
    ) -> Result<std::convert::Infallible, kernel::Error> {
        let mut command = Command::new(&self.program);
        command.args(&self.leading_arguments);
        command.args(arguments);
        for (name, value) in &self.environment {
            command.env(name, value);
        }
        // `exec` only returns on failure.
        Err(kernel::Error::Failed(format!(
            "could not exec the executor client: {}",
            command.exec()
        )))
    }
}

/// Whether `path` names a readable directory, for the state-dir preflight.
#[must_use]
pub fn is_directory(path: &Path) -> bool {
    path.is_dir()
}

#[cfg(test)]
mod tests {
    use design::executor::ports::ProcessProbe as _;

    use super::generate_token;
    use super::is_alive;
    use super::HostProbe;
    use design::executor::daemon_identity::ObservedDaemon;
    use design::executor::daemon_identity::ObservedStartTime;

    #[test]
    fn this_process_is_observed_live() {
        let me = i32::try_from(std::process::id()).unwrap_or(-1);
        let observed = HostProbe.observe(me);
        assert!(
            matches!(observed, ObservedDaemon::Live(_)),
            "got {observed:?}"
        );
    }

    /// The host can answer for a live process on both supported platforms, so
    /// the unavailable branch is a container concern rather than a normal one.
    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn a_live_process_reports_a_known_start_time() {
        let me = i32::try_from(std::process::id()).unwrap_or(-1);
        assert!(matches!(
            HostProbe.observe(me),
            ObservedDaemon::Live(ObservedStartTime::Known(_))
        ));
    }

    #[test]
    fn an_implausible_pid_is_absent() {
        for pid in [-1, 0] {
            assert_eq!(HostProbe.observe(pid), ObservedDaemon::Absent, "{pid}");
            assert!(!is_alive(pid), "{pid}");
        }
    }

    #[test]
    fn a_token_is_128_bits_of_lowercase_hex() {
        let token = generate_token();
        assert_eq!(token.len(), 32);
        assert!(
            token
                .bytes()
                .all(|byte| byte.is_ascii_digit()
                    || (b'a'..=b'f').contains(&byte))
        );
    }

    #[test]
    fn two_tokens_differ() {
        assert_ne!(generate_token(), generate_token());
    }
}
