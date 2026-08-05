//! The zero-spawn harness: marker-writing `git`/`jj` stubs on a synthetic
//! `PATH`, and a platform-aware report of the absolute paths `PATH` cannot
//! reach.
//!
//! **Never writes outside its own temp directories.** It resolves and *reports*
//! absolute paths; it does not move, replace or chmod them, and never invokes
//! `sudo`. `/opt/homebrew/bin` is user-writable, so an "attempt and record the
//! failure" design would succeed there and could leave a developer without `git`
//! or `jj`. All privileged mutation lives in the task that shadows them.

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::Error;

/// Names the workflow step and the harness agree on, so neither assumes the
/// other did the work.
pub const MODE_VARIABLE: &str = "ACCELERATOR_ZERO_SPAWN_MODE";
pub const SHADOWED_VARIABLE: &str = "ACCELERATOR_ZERO_SPAWN_SHADOWED";

/// How thoroughly the real binaries were made unreachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `PATH` stubs only. Absolute paths remain reachable and are reported.
    PathOnly,
    /// `PATH` stubs *and* the absolute paths shadowed by the workflow step.
    Strong,
}

impl Mode {
    /// Reads the declared mode, failing closed on malformed input: an
    /// unrecognised value, or a shadow list without `strong`, is an error rather
    /// than a silent downgrade to `PathOnly`.
    ///
    /// # Errors
    ///
    /// When the mode is unrecognised, or the two variables disagree.
    pub fn from_environment() -> Result<Self, Error> {
        let declared = env::var(MODE_VARIABLE).unwrap_or_default();
        let shadowed = env::var(SHADOWED_VARIABLE).unwrap_or_default();
        let mode = match declared.as_str() {
            "" | "path-only" => Self::PathOnly,
            "strong" => Self::Strong,
            other => {
                return Err(Error::message(format!(
                    "{MODE_VARIABLE}={other} is not a recognised zero-spawn \
                     mode; expected `strong` or `path-only`"
                )));
            }
        };
        if mode == Self::PathOnly && !shadowed.trim().is_empty() {
            return Err(Error::message(format!(
                "{SHADOWED_VARIABLE} names shadowed paths but \
                 {MODE_VARIABLE} is not `strong` — the workflow step and the \
                 harness disagree about what was done"
            )));
        }
        Ok(mode)
    }
}

/// A synthetic `PATH` whose `git` and `jj` write a marker instead of running.
#[derive(Debug)]
pub struct Stubs {
    directory: PathBuf,
    marker: PathBuf,
    path: String,
}

impl Stubs {
    /// Writes the stubs beneath `base` and computes the synthetic `PATH`.
    ///
    /// # Errors
    ///
    /// When the stubs cannot be written or made executable.
    pub fn rooted_at(base: &Path) -> Result<Self, Error> {
        let directory = base.join("zero-spawn-stubs");
        fs::create_dir_all(&directory)?;
        let marker = base.join("zero-spawn-marker");

        for binary in ["git", "jj"] {
            let stub = directory.join(binary);
            fs::write(
                &stub,
                format!(
                    "#!/bin/sh\nprintf '%s %s\\n' {binary} \"$*\" >> {}\nexit 0\n",
                    marker.display()
                ),
            )?;
            make_executable(&stub)?;
        }

        // Every directory that resolves git or jj, not just the first: macOS
        // commonly provides it in two.
        let existing = env::var("PATH").unwrap_or_default();
        let kept: Vec<&str> = existing
            .split(':')
            .filter(|entry| {
                !entry.is_empty()
                    && !["git", "jj"].iter().any(|binary| {
                        is_executable(&Path::new(entry).join(binary))
                    })
            })
            .collect();
        let path = format!("{}:{}", directory.display(), kept.join(":"));

        Ok(Self {
            directory,
            marker,
            path,
        })
    }

    /// The synthetic `PATH`, with the stub directory first and every directory
    /// that could resolve a real `git`/`jj` removed.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Applies the synthetic `PATH` to `command`.
    pub fn apply(&self, command: &mut Command) {
        command.env("PATH", &self.path);
    }

    /// Whatever a stub recorded, or `None` when none ran.
    ///
    /// # Errors
    ///
    /// When the marker exists but cannot be read.
    pub fn spawns(&self) -> Result<Option<String>, Error> {
        if !self.marker.exists() {
            return Ok(None);
        }
        Ok(Some(fs::read_to_string(&self.marker)?))
    }
}

/// Absolute paths that would still resolve a real `git` or `jj` even with the
/// synthetic `PATH` in force.
///
/// Resolved at run time by asking the environment where each binary actually
/// lives, because the meaningful targets differ per platform: on Linux CI the jj
/// target is the mise install path and there is no system `jj` at all, while on
/// a developer's macOS machine `/opt/homebrew/bin/jj` **is** the real binary.
///
/// # Errors
///
/// When `PATH` cannot be read.
pub fn unshadowed_paths() -> Result<Vec<PathBuf>, Error> {
    let mut found = Vec::new();
    let path = env::var("PATH").unwrap_or_default();
    for entry in path.split(':').filter(|entry| !entry.is_empty()) {
        for binary in ["git", "jj"] {
            let candidate = Path::new(entry).join(binary);
            if is_executable(&candidate) && !found.contains(&candidate) {
                found.push(candidate);
            }
        }
    }
    // The conventional absolute locations, whether or not they are on PATH —
    // a caller reaching them by absolute path does not consult PATH at all.
    for candidate in [
        "/usr/bin/git",
        "/usr/local/bin/git",
        "/opt/homebrew/bin/git",
        "/usr/bin/jj",
        "/usr/local/bin/jj",
        "/opt/homebrew/bin/jj",
    ] {
        let candidate = PathBuf::from(candidate);
        if is_executable(&candidate) && !found.contains(&candidate) {
            found.push(candidate);
        }
    }
    found.sort();
    Ok(found)
}

/// Fails when `strong` was declared but a listed path is still executable.
///
/// Without this, a runner image that relocates `git` turns the workflow's
/// `sudo mv` into a no-op and the job goes green with the property unproven.
///
/// # Errors
///
/// When the mode is malformed, or any path the step claimed to shadow still
/// resolves.
pub fn assert_shadowing_holds(mode: Mode) -> Result<(), Error> {
    if mode != Mode::Strong {
        return Ok(());
    }
    let claimed = env::var(SHADOWED_VARIABLE).unwrap_or_default();
    let still_live: Vec<&str> = claimed
        .split(':')
        .filter(|entry| !entry.trim().is_empty())
        .filter(|entry| is_executable(Path::new(entry)))
        .collect();
    if !still_live.is_empty() {
        return Err(Error::message(format!(
            "{MODE_VARIABLE}=strong was declared but these paths are still \
             executable: {}",
            still_live.join(", ")
        )));
    }

    let remaining = unshadowed_paths()?;
    if !remaining.is_empty() {
        return Err(Error::message(format!(
            "{MODE_VARIABLE}=strong was declared but a real git/jj is still \
             reachable at: {}",
            remaining
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt as _;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), Error> {
    Ok(())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    fs::metadata(path).is_ok_and(|metadata| {
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    })
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// The compiled `vcs-adapters-fixture` binary, resolved beside the calling test
/// binary.
///
/// `CARGO_BIN_EXE_*` is injected only for the declaring crate's own integration
/// tests, so a consumer in another crate derives the path from its test exe
/// instead. An absent binary fails loudly rather than silently skipping the one
/// assertion that proves the property.
///
/// # Errors
///
/// When the target directory cannot be derived, or the binary is not built.
pub fn reference_artefact() -> Result<PathBuf, Error> {
    let exe = env::current_exe()?;
    // .../target/<profile>/deps/<testbin> -> .../target/<profile>
    let profile = exe.parent().and_then(Path::parent).ok_or_else(|| {
        Error::message(
            "could not locate the target profile dir from the test exe",
        )
    })?;
    let binary = profile.join("vcs-adapters-fixture");
    if !binary.is_file() {
        return Err(Error::message(format!(
            "the reference artefact is not built at {} — build the workspace \
             first (`cargo build -p vcs-adapters`)",
            binary.display()
        )));
    }
    Ok(binary)
}
