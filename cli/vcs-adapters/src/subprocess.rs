//! The subprocess-backed probes: a marker walk for the root, and the VCS
//! binaries themselves for the working-copy revision.
//!
//! The subprocess runs with a scrubbed environment and colour disabled, so
//! ambient config cannot redirect the root or inject ANSI into a revision. Every
//! way it can fail to answer resolves to `None` and is warn-logged, so a real
//! failure does not read like a revision-less repository.
//!
//! Spawning lives here and only here; [`crate::library`] carries an import rule
//! denying `std::process`.

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tracing::warn;
use vcs::{RepoRoot, VcsKind, VcsProbe};

use crate::markers::{carries_any_marker, marker_kind, walk_up};

/// Headroom for a lock-contended repository, while still bounding derivation.
const DEFAULT_CAP: Duration = Duration::from_secs(10);

const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Finds the repository root by walking ancestors for the first `.jj` or `.git`
/// marker, testing existence so a `.git` *file* counts alongside a directory.
#[derive(Debug, Clone, Copy, Default)]
pub struct MarkerWalkRoot;

impl RepoRoot for MarkerWalkRoot {
    fn discover(&self, start: &Path) -> Option<PathBuf> {
        walk_up(start, carries_any_marker)
    }

    fn repository_root(&self, working_copy_root: &Path) -> PathBuf {
        jj_repository_root(working_copy_root)
            .unwrap_or_else(|| working_copy_root.to_path_buf())
    }
}

/// Resolves a jj secondary workspace to the repository whose store it shares.
///
/// A secondary workspace's `.jj/repo` is a *file* pointing at the shared
/// `<repo>/.jj/repo`, so the repository is the store's grandparent; a primary
/// workspace's `.jj/repo` is that store directory.
fn jj_repository_root(working_copy_root: &Path) -> Option<PathBuf> {
    let marker = working_copy_root.join(".jj").join("repo");
    let metadata = fs::symlink_metadata(&marker).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let pointer = fs::read_to_string(&marker).ok()?;
    let store =
        fs::canonicalize(working_copy_root.join(".jj").join(pointer.trim()))
            .ok()?;
    Some(store.parent()?.parent()?.to_path_buf())
}

/// Reads the repository's idiom from its markers and its revision by running
/// the matching VCS binary.
#[derive(Debug, Clone, Copy)]
pub struct CommandProbe {
    cap: Duration,
}

impl CommandProbe {
    #[must_use]
    pub const fn new() -> Self {
        Self { cap: DEFAULT_CAP }
    }

    /// A probe bounded by `cap` rather than the default.
    #[must_use]
    pub const fn with_cap(cap: Duration) -> Self {
        Self { cap }
    }
}

impl Default for CommandProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl VcsProbe for CommandProbe {
    fn kind(&self, root: &Path) -> VcsKind {
        marker_kind(root)
    }

    fn revision(&self, root: &Path, kind: VcsKind) -> Option<String> {
        let mut command = match kind {
            VcsKind::Jj => {
                let mut command = Command::new("jj");
                command.args([
                    "--color=never",
                    "--no-pager",
                    "log",
                    "-r",
                    "@",
                    "--no-graph",
                    "-T",
                    "commit_id",
                ]);
                command
            }
            VcsKind::Git => {
                let mut command = Command::new("git");
                command.args(["-c", "color.ui=false", "rev-parse", "HEAD"]);
                command
            }
            VcsKind::None => return None,
        };

        command.current_dir(root);
        scrub_environment(&mut command);

        let vcs = kind.as_str();
        let stdout = run_capped(command, self.cap, vcs)?;
        if stdout.is_empty() {
            warn!(vcs, "the revision probe reported no revision");
            return None;
        }
        Some(stdout)
    }
}

/// `jj status`, or `git diff --cached --stat` for `Git`/`None` (matching the
/// shell's implicit git fallback when no repository is found at all).
///
/// Never fails: falls back to the shell's literal `(... unavailable)` text on
/// any [`run_capped`] failure, which is already `warn!`-logged internally and
/// so diagnosable via `ACCELERATOR_LOG` rather than silently indistinguishable
/// from a clean, empty repository.
#[must_use]
pub fn status(root: &Path, kind: VcsKind) -> String {
    run_vcs_text(root, kind, StatusOrLog::Status, DEFAULT_CAP, |_| {})
}

/// `jj log --limit 5`, or `git log --oneline -5` for `Git`/`None`. See
/// [`status`] for the fallback and diagnostic contract.
#[must_use]
pub fn log(root: &Path, kind: VcsKind) -> String {
    run_vcs_text(root, kind, StatusOrLog::Log, DEFAULT_CAP, |_| {})
}

#[derive(Debug, Clone, Copy)]
enum StatusOrLog {
    Status,
    Log,
}

/// `cap` and `configure` are test-only knobs (production always calls
/// through [`status`]/[`log`], which pass [`DEFAULT_CAP`] and a no-op) —
/// `configure` runs *before* [`scrub_environment`], so a test can set an
/// ambient variable on the command itself (never the whole process's
/// environment) and assert it is scrubbed, or override `PATH` to swap
/// `jj`/`git` for a stand-in, scoped to this one spawned command.
fn run_vcs_text(
    root: &Path,
    kind: VcsKind,
    which: StatusOrLog,
    cap: Duration,
    configure: impl FnOnce(&mut Command),
) -> String {
    let mut command = match (kind, which) {
        (VcsKind::Jj, StatusOrLog::Status) => {
            let mut command = Command::new("jj");
            command.args(["--color=never", "--no-pager", "status"]);
            command
        }
        (VcsKind::Jj, StatusOrLog::Log) => {
            let mut command = Command::new("jj");
            command.args([
                "--color=never",
                "--no-pager",
                "log",
                "--limit",
                "5",
            ]);
            command
        }
        (VcsKind::Git | VcsKind::None, StatusOrLog::Status) => {
            let mut command = Command::new("git");
            command.args([
                "-c",
                "color.ui=false",
                "diff",
                "--cached",
                "--stat",
            ]);
            command
        }
        (VcsKind::Git | VcsKind::None, StatusOrLog::Log) => {
            let mut command = Command::new("git");
            command.args(["-c", "color.ui=false", "log", "--oneline", "-5"]);
            command
        }
    };
    command.current_dir(root);
    configure(&mut command);
    scrub_environment(&mut command);

    let jj_wins = matches!(kind, VcsKind::Jj);
    let vcs = if jj_wins { "jj" } else { "git" };
    let unavailable = match (jj_wins, which) {
        (true, StatusOrLog::Status) => "(jj status unavailable)",
        (true, StatusOrLog::Log) => "(jj log unavailable)",
        (false, StatusOrLog::Status) => "(git status unavailable)",
        (false, StatusOrLog::Log) => "(git log unavailable)",
    };

    run_capped(command, cap, vcs).unwrap_or_else(|| unavailable.to_owned())
}

/// Denies the subprocess the ambient config that could redirect it at another
/// repository.
fn scrub_environment(command: &mut Command) {
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_CONFIG",
        "GIT_CONFIG_COUNT",
        "JJ_CONFIG",
    ] {
        command.env_remove(key);
    }
    command.env("GIT_CONFIG_NOSYSTEM", "1");
    command.env("GIT_CONFIG_GLOBAL", "/dev/null");
    command.env("GIT_CONFIG_SYSTEM", "/dev/null");
}

/// Runs `command` and returns its trimmed stdout, or `None` — warn-logged —
/// when it cannot be spawned, exits non-zero, or outlives `cap`.
///
/// Unlike [`CommandProbe::revision`], empty output is not itself treated as a
/// failure here: a clean `git diff --cached --stat` or an empty `jj status`
/// are legitimate, common results for [`status`]/[`log`], not failures.
fn run_capped(
    mut command: Command,
    cap: Duration,
    vcs: &str,
) -> Option<String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            warn!(vcs, %error, "could not run the probe");
            return None;
        }
    };

    let deadline = Instant::now() + cap;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                warn!(vcs, %error, "could not await the probe");
                return None;
            }
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            warn!(
                vcs,
                cap = ?cap,
                "the probe outlived its time cap and was killed"
            );
            return None;
        }

        std::thread::sleep(POLL_INTERVAL);
    };

    if !status.success() {
        warn!(vcs, %status, "the probe exited non-zero");
        return None;
    }

    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        if let Err(error) = pipe.read_to_string(&mut stdout) {
            warn!(vcs, %error, "could not read the probe's output");
            return None;
        }
    }

    // Trailing-only: a leading space is significant in `git diff --stat`'s
    // output (` file.txt | 1 +`), so a full `.trim()` would corrupt it.
    Some(stdout.trim_end().to_owned())
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::process::Command;
    use std::time::Duration;

    use vcs::VcsKind;

    use super::{run_capped, run_vcs_text, scrub_environment, StatusOrLog};

    type TestError = Box<dyn Error>;

    /// A directory on a synthetic `PATH` carrying `jj` and `git` stand-ins
    /// that sleep, so `status`/`log` can be proven bounded without a real
    /// repository or the real binaries.
    fn slow_stand_ins() -> Result<tempfile::TempDir, TestError> {
        let dir = tempfile::Builder::new()
            .prefix("vcs-subprocess-standin-")
            .tempdir()?;
        for name in ["jj", "git"] {
            let script = dir.path().join(name);
            fs::write(&script, "#!/bin/sh\nsleep 30\n")?;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
        }
        Ok(dir)
    }

    #[test]
    fn status_and_log_are_bounded_by_the_same_cap_as_run_capped(
    ) -> Result<(), TestError> {
        let stand_ins = slow_stand_ins()?;
        let path = format!(
            "{}:{}",
            stand_ins.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let root = std::env::temp_dir();

        for kind in [VcsKind::Jj, VcsKind::Git] {
            for which in [StatusOrLog::Status, StatusOrLog::Log] {
                let started = std::time::Instant::now();
                let output = run_vcs_text(
                    &root,
                    kind,
                    which,
                    Duration::from_millis(100),
                    |command| {
                        command.env("PATH", &path);
                    },
                );
                assert!(
                    started.elapsed() < Duration::from_secs(5),
                    "should have been killed at its cap, not waited out"
                );
                assert!(
                    output.contains("unavailable"),
                    "expected the unavailable fallback text, got {output:?}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn status_and_log_scrub_ambient_git_and_jj_config() -> Result<(), TestError>
    {
        let dir = tempfile::Builder::new()
            .prefix("vcs-subprocess-scrub-")
            .tempdir()?;
        let attacker_config = dir.path().join("attacker.toml");
        fs::write(&attacker_config, "attacker-controlled\n")?;

        let output = run_vcs_text(
            dir.path(),
            VcsKind::Git,
            StatusOrLog::Status,
            Duration::from_secs(5),
            |command| {
                command.env("GIT_CONFIG", &attacker_config);
                command.env("JJ_CONFIG", &attacker_config);
            },
        );

        assert!(
            !output.contains("attacker-controlled"),
            "the attacker config must never be read: {output:?}"
        );
        Ok(())
    }

    #[test]
    fn a_probe_that_outlives_its_cap_reports_no_revision() {
        let mut command = Command::new("sleep");
        command.arg("30");

        let started = std::time::Instant::now();
        let revision = run_capped(command, Duration::from_millis(100), "test");

        assert_eq!(revision, None);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the probe should have been killed at its cap, not waited out"
        );
    }

    #[test]
    fn a_probe_that_cannot_be_spawned_reports_no_revision() {
        let command = Command::new("accelerator-no-such-binary");
        assert_eq!(run_capped(command, Duration::from_secs(1), "test"), None);
    }

    #[test]
    fn a_probe_that_exits_non_zero_reports_no_revision() {
        let command = Command::new("false");
        assert_eq!(run_capped(command, Duration::from_secs(1), "test"), None);
    }

    #[test]
    fn a_probe_that_says_nothing_reports_empty_output_not_failure() {
        // Unlike `revision`, `run_capped` itself does not treat empty output
        // as a failure — a clean `status`/`log` legitimately says nothing.
        let command = Command::new("true");
        assert_eq!(
            run_capped(command, Duration::from_secs(1), "test").as_deref(),
            Some("")
        );
    }

    #[test]
    fn stdout_is_trimmed() {
        let mut command = Command::new("printf");
        command.arg("abc123\n");
        assert_eq!(
            run_capped(command, Duration::from_secs(1), "test").as_deref(),
            Some("abc123")
        );
    }

    #[test]
    fn the_scrubbed_environment_drops_the_redirecting_variables() {
        let mut command = Command::new("sh");
        command
            .args([
                "-c",
                "if [ -n \"$GIT_DIR\" ]; then printf carried; \
                 else printf unset; fi",
            ])
            .env("GIT_DIR", "/somewhere/else/.git");
        scrub_environment(&mut command);

        assert_eq!(
            run_capped(command, Duration::from_secs(1), "test").as_deref(),
            Some("unset")
        );
    }
}
