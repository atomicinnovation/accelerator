//! Shells the real `diff -u` per differing section, for byte-identical
//! output.
//!
//! The same "shell the real tool" adapter pattern already established for
//! `vcs status`/`vcs log`. Port of `work-item-section-diff.sh:104-109`'s
//! exact header format.
//!
//! This is the crate's one subprocess-spawning module, isolated (via
//! `cli/pup.ron`'s `work_adapters_filesystem_reads_in_process` rule) from
//! `filesystem.rs`'s pure reads.

use std::io::Read as _;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use work::section_diff::SectionDiff;

const DEFAULT_CAP: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// `diff` could not be spawned, or outlived its time cap.
#[derive(Debug)]
pub struct DiffUnavailable;

/// Renders one section's `=== name (- LOCAL / + REMOTE) ===` header plus
/// its `diff -u` body, matching the shell original's exact framing.
///
/// # Errors
///
/// [`DiffUnavailable`] when the `diff` binary cannot be spawned (e.g. not on
/// `PATH`) or does not complete within the time cap.
pub fn render(diff: &SectionDiff) -> Result<String, DiffUnavailable> {
    render_with(diff, DEFAULT_CAP, |_| {})
}

/// `cap` and `configure` are test-only knobs (production always calls
/// through [`render`], which passes [`DEFAULT_CAP`] and a no-op) —
/// `configure` lets a test scope a stand-in `PATH` (or another override) to
/// just this one spawned `Command`, never the whole process's environment.
fn render_with(
    diff: &SectionDiff,
    cap: Duration,
    configure: impl FnOnce(&mut Command),
) -> Result<String, DiffUnavailable> {
    let tmp = tempfile::tempdir().map_err(|_| DiffUnavailable)?;
    std::fs::write(tmp.path().join("LOCAL"), format!("{}\n", diff.local))
        .map_err(|_| DiffUnavailable)?;
    std::fs::write(tmp.path().join("REMOTE"), format!("{}\n", diff.remote))
        .map_err(|_| DiffUnavailable)?;

    let mut command = Command::new("diff");
    command.args(["-u", "LOCAL", "REMOTE"]);
    command.current_dir(tmp.path());
    configure(&mut command);
    let body = run_capped(command, cap)?;

    let mut out = format!("=== {} (- LOCAL / + REMOTE) ===\n", diff.name);
    out.push_str(&body);
    if !body.is_empty() && !body.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    Ok(out)
}

/// Runs `command` and returns its stdout, tolerating `diff`'s exit 1 (files
/// differ — expected, not an error). Only a failed spawn or a cap timeout is
/// [`DiffUnavailable`].
fn run_capped(
    mut command: Command,
    cap: Duration,
) -> Result<String, DiffUnavailable> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|_| DiffUnavailable)?;

    let deadline = Instant::now() + cap;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {}
            Err(_) => return Err(DiffUnavailable),
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(DiffUnavailable);
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout)
            .map_err(|_| DiffUnavailable)?;
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::time::Duration;

    use work::section_diff::SectionDiff;

    use super::render_with;

    fn diff(name: &str, local: &str, remote: &str) -> SectionDiff {
        SectionDiff {
            name: name.to_owned(),
            local: local.to_owned(),
            remote: remote.to_owned(),
        }
    }

    #[test]
    fn renders_the_header_and_diff_body(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let section = diff("Summary", "local summary", "remote summary");
        let rendered = render_with(&section, Duration::from_secs(5), |_| {})
            .map_err(|_| "diff unavailable")?;
        assert!(rendered.starts_with("=== Summary (- LOCAL / + REMOTE) ===\n"));
        assert!(rendered.contains("-local summary"));
        assert!(rendered.contains("+remote summary"));
        Ok(())
    }

    #[test]
    fn a_missing_diff_binary_is_reported(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let empty_path_dir = tempfile::tempdir()?;
        let path = empty_path_dir.path().display().to_string();
        let section = diff("Summary", "a", "b");
        let result = render_with(&section, Duration::from_secs(1), |command| {
            command.env("PATH", &path);
        });
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn a_hanging_diff_is_killed_at_the_cap(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::Builder::new()
            .prefix("work-diff-standin-")
            .tempdir()?;
        let script = dir.path().join("diff");
        fs::write(&script, "#!/bin/sh\nsleep 30\n")?;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
        let path = format!(
            "{}:{}",
            dir.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );

        let section = diff("Summary", "a", "b");
        let started = std::time::Instant::now();
        let result =
            render_with(&section, Duration::from_millis(100), |command| {
                command.env("PATH", &path);
            });
        assert!(result.is_err());
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "should have been killed at its cap, not waited out"
        );
        Ok(())
    }
}
