//! The outbound VCS probes: an ancestor marker-walk for the repository root,
//! and a subprocess probe for the working-copy revision.
//!
//! The probe subprocess runs with a scrubbed environment and colour disabled,
//! so ambient user config cannot redirect the root or inject ANSI into a
//! revision. Every way the probe can fail to answer — an absent binary, a
//! non-zero exit, empty output, or a run that outlives its time cap — resolves
//! to `None` and is warn-logged, so a real failure leaves a trace instead of
//! reading like a legitimately revision-less repository.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tracing::warn;
use vcs::{RepoFacts, RepoRoot, VcsKind, VcsProbe};

/// Headroom for a legitimately slow or lock-contended repository, while still
/// bounding metadata derivation.
const DEFAULT_CAP: Duration = Duration::from_secs(10);

const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Finds the repository root by walking ancestors for the first `.jj` or
/// `.git` marker, testing for *existence* so a `.git` file — a worktree or
/// submodule — counts alongside a `.git` directory.
///
/// The filesystem root itself is never tested, matching the bash walk.
#[derive(Debug, Clone, Copy, Default)]
pub struct MarkerWalkRoot;

impl RepoRoot for MarkerWalkRoot {
    fn discover(&self, start: &Path) -> Option<PathBuf> {
        let mut dir = start;
        while dir.parent().is_some() {
            if dir.join(".jj").exists() || dir.join(".git").exists() {
                return Some(dir.to_path_buf());
            }
            dir = dir.parent()?;
        }
        None
    }
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
        if root.join(".jj").exists() {
            VcsKind::Jj
        } else if root.join(".git").exists() {
            VcsKind::Git
        } else {
            VcsKind::None
        }
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
        capped_stdout(command, self.cap, kind.as_str())
    }
}

/// Denies the subprocess the ambient config that could redirect it at another
/// repository or dress its output up in ANSI.
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
/// when it cannot be spawned, exits non-zero, outlives `cap`, or says nothing.
fn capped_stdout(
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
            warn!(vcs, %error, "could not run the revision probe");
            return None;
        }
    };

    let deadline = Instant::now() + cap;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                warn!(vcs, %error, "could not await the revision probe");
                return None;
            }
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            warn!(
                vcs,
                cap = ?cap,
                "the revision probe outlived its time cap and was killed"
            );
            return None;
        }

        std::thread::sleep(POLL_INTERVAL);
    };

    if !status.success() {
        warn!(vcs, %status, "the revision probe exited non-zero");
        return None;
    }

    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        if let Err(error) = pipe.read_to_string(&mut stdout) {
            warn!(vcs, %error, "could not read the revision probe's output");
            return None;
        }
    }

    let revision = stdout.trim();
    if revision.is_empty() {
        warn!(vcs, "the revision probe reported no revision");
        return None;
    }
    Some(revision.to_owned())
}

/// The facts for the repository containing `start`, probed against the real
/// filesystem and the real VCS binaries.
#[must_use]
pub fn facts(start: &Path) -> Option<RepoFacts> {
    vcs::facts(start, &MarkerWalkRoot, &CommandProbe::new())
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::Duration;

    use super::{capped_stdout, facts, scrub_environment};

    #[test]
    fn a_tree_with_no_marker_has_no_facts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // The marker walk needs no VCS binary, so this stays outside the
        // `bash-parity` detection fixtures and runs on a bare machine.
        let loose = std::env::temp_dir()
            .join(format!("vcs-loose-{}", std::process::id()));
        std::fs::create_dir_all(&loose)?;

        assert_eq!(
            facts(&loose),
            None,
            "a tree with no .jj or .git must be representable as absent"
        );

        std::fs::remove_dir_all(&loose)?;
        Ok(())
    }

    #[test]
    fn a_probe_that_outlives_its_cap_reports_no_revision() {
        let mut command = Command::new("sleep");
        command.arg("30");

        let started = std::time::Instant::now();
        let revision =
            capped_stdout(command, Duration::from_millis(100), "test");

        assert_eq!(revision, None);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the probe should have been killed at its cap, not waited out"
        );
    }

    #[test]
    fn a_probe_that_cannot_be_spawned_reports_no_revision() {
        let command = Command::new("accelerator-no-such-binary");
        assert_eq!(
            capped_stdout(command, Duration::from_secs(1), "test"),
            None
        );
    }

    #[test]
    fn a_probe_that_exits_non_zero_reports_no_revision() {
        let command = Command::new("false");
        assert_eq!(
            capped_stdout(command, Duration::from_secs(1), "test"),
            None
        );
    }

    #[test]
    fn a_probe_that_says_nothing_reports_no_revision() {
        let command = Command::new("true");
        assert_eq!(
            capped_stdout(command, Duration::from_secs(1), "test"),
            None
        );
    }

    #[test]
    fn stdout_is_trimmed() {
        let mut command = Command::new("printf");
        command.arg("abc123\n");
        assert_eq!(
            capped_stdout(command, Duration::from_secs(1), "test").as_deref(),
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
            capped_stdout(command, Duration::from_secs(1), "test").as_deref(),
            Some("unset")
        );
    }
}
