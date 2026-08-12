//! Selects and probes the runtime cache directory.
//!
//! [`candidate`] selects: the `ACCELERATOR_CACHE_DIR` override when set, else
//! `${ACCELERATOR_PLUGIN_ROOT}/bin`. Selection only — no filesystem access —
//! so a warm cache hit pays nothing here. There is no XDG fallback: an
//! XDG-resident binary would break the plugin-root `allowed-tools` glob match.
//!
//! Cache-root writability is owned by the resolver — new callers route
//! through it rather than probing directly.
//!
//! Builds only for the platforms `HOST_PLATFORM` names — linux and macOS on
//! `x86_64` and aarch64. The `#[cfg(not(unix))]` `make_executable` arm is
//! unreachable dead code, retained only as a marker.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crate::launch::core::ResolutionError;

/// Injected environment inputs, so tests supply temp dirs.
pub struct CacheRootConfig {
    pub cache_dir_override: Option<PathBuf>,
    pub plugin_root: Option<PathBuf>,
}

impl CacheRootConfig {
    /// Reads `ACCELERATOR_CACHE_DIR` itself; `plugin_root` is injected rather
    /// than read here, so `main` stays the launcher's one module that names
    /// `config_adapters` (`config_adapters::plugin_root_from_env`).
    #[must_use]
    pub fn from_env(plugin_root: Option<PathBuf>) -> Self {
        Self {
            cache_dir_override: std::env::var_os("ACCELERATOR_CACHE_DIR")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            plugin_root: plugin_root
                .filter(|value| !value.as_os_str().is_empty()),
        }
    }
}

/// The cache root candidate: the `ACCELERATOR_CACHE_DIR` override, else
/// `${ACCELERATOR_PLUGIN_ROOT}/bin`.
///
/// Selection only — no filesystem write, no process spawn — so a warm cache
/// hit never pays the write+exec probe `verify_writable` performs.
///
/// # Errors
///
/// [`ResolutionError::CacheRootUnavailable`] when neither
/// `ACCELERATOR_CACHE_DIR` nor `ACCELERATOR_PLUGIN_ROOT` is set (no XDG
/// fallback).
pub fn candidate(config: &CacheRootConfig) -> Result<PathBuf, ResolutionError> {
    if let Some(override_dir) = &config.cache_dir_override {
        tracing::info!(
            path = %override_dir.display(),
            "using ACCELERATOR_CACHE_DIR override for the cache root"
        );
        return Ok(override_dir.clone());
    }
    let plugin_root = config.plugin_root.as_ref().ok_or_else(|| {
        ResolutionError::CacheRootUnavailable {
            detail: "ACCELERATOR_PLUGIN_ROOT is not set and no \
                     ACCELERATOR_CACHE_DIR override was given"
                .to_owned(),
        }
    })?;
    Ok(plugin_root.join("bin"))
}

thread_local! {
    static PROBE_ATTEMPTS: Cell<u64> = const { Cell::new(0) };
}

/// Calls to [`verify_writable`] on this thread.
///
/// Includes calls that fail before writing anything — unlike `SEQUENCE`, whose
/// increment sits after the `create_dir_all` guard and so counts only probes
/// that reached the write stage.
///
/// A test-only observation point, `pub` because the launcher's integration
/// tests are a separate crate. Read as a delta either side of the call under
/// test.
#[must_use]
pub fn probe_attempts() -> u64 {
    PROBE_ATTEMPTS.with(Cell::get)
}

/// Probe `dir` for writability and exec-capability, creating it if needed.
///
/// Only matters on the write path — a resolver that already has a cached,
/// re-verified binary should never call this.
///
/// # Errors
///
/// [`ResolutionError::CacheRootUnavailable`] when `dir` is not
/// writable+exec-capable.
pub(super) fn verify_writable(dir: &Path) -> Result<(), ResolutionError> {
    PROBE_ATTEMPTS.with(|attempts| attempts.set(attempts.get() + 1));
    if probe_writable_and_executable(dir) {
        Ok(())
    } else {
        Err(ResolutionError::CacheRootUnavailable {
            detail: format!(
                "{} is not writable+exec-capable (no XDG fallback)",
                dir.display()
            ),
        })
    }
}

/// Probe writability and exec-capability by writing then running a script —
/// catching `noexec` mounts, which a write-only probe would miss.
///
/// The filename carries a per-process sequence number alongside the PID: two
/// threads in one process (the launcher's own concurrent-first-use tests
/// resolve from more than one thread) would otherwise collide on the same
/// PID-only path and race each other's write/exec/remove cycle.
fn probe_writable_and_executable(dir: &Path) -> bool {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let probe = dir.join(format!(
        ".accelerator-probe-{}-{sequence}",
        std::process::id()
    ));
    let written = std::fs::write(&probe, b"#!/bin/sh\nexit 0\n").is_ok()
        && make_executable(&probe);
    let executable = written
        && Command::new(&probe)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    let _ = std::fs::remove_file(&probe);
    executable
}

#[cfg(unix)]
fn make_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .is_ok()
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::{candidate, probe_attempts, verify_writable, CacheRootConfig};

    fn config() -> CacheRootConfig {
        CacheRootConfig {
            cache_dir_override: None,
            plugin_root: None,
        }
    }

    fn tempdir() -> Result<TempDir, Box<dyn Error>> {
        Ok(tempfile::Builder::new()
            .prefix("acc-cacheroot-")
            .tempdir()?)
    }

    #[test]
    fn unset_plugin_root_with_no_override_is_a_named_error() {
        let result = candidate(&config());
        assert!(result.is_err(), "expected an ACCELERATOR_PLUGIN_ROOT error");
        if let Err(error) = result {
            let message = error.to_string();
            assert!(message.contains("ACCELERATOR_PLUGIN_ROOT"));
            // Distinguishes this step from the config layer's plugin-root
            // refusal, which also names the variable.
            assert!(
                message.contains("no ACCELERATOR_CACHE_DIR override was given")
            );
        }
    }

    #[test]
    fn verify_writable_creates_a_missing_directory(
    ) -> Result<(), Box<dyn Error>> {
        let temp = tempdir()?;
        let target = temp.path().join("bin");
        verify_writable(&target)?;
        assert!(
            target.is_dir(),
            "the probe must create a missing cache root"
        );
        Ok(())
    }

    #[test]
    fn candidate_performs_no_filesystem_write_or_process_spawn(
    ) -> Result<(), Box<dyn Error>> {
        // A non-existent parent with no permission to create it: had
        // `candidate` performed any I/O, `create_dir_all` inside the probe
        // would fail loudly; instead it must return the path unexamined.
        let unwritable_parent = PathBuf::from("/nonexistent-acc-parent-dir");
        let plugin_root = unwritable_parent.join("plugin-root");
        let resolved = candidate(&CacheRootConfig {
            plugin_root: Some(plugin_root.clone()),
            ..config()
        })?;
        assert_eq!(resolved, plugin_root.join("bin"));
        assert!(
            !unwritable_parent.exists(),
            "candidate must not create any directory"
        );
        Ok(())
    }

    #[test]
    fn verify_writable_accepts_a_writable_directory(
    ) -> Result<(), Box<dyn Error>> {
        let temp = tempdir()?;
        verify_writable(temp.path())?;
        Ok(())
    }

    #[test]
    fn verify_writable_rejects_a_read_only_directory(
    ) -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::PermissionsExt as _;
        let temp = tempdir()?;
        std::fs::set_permissions(
            temp.path(),
            std::fs::Permissions::from_mode(0o555),
        )?;
        let result = verify_writable(temp.path());
        std::fs::set_permissions(
            temp.path(),
            std::fs::Permissions::from_mode(0o755),
        )?;
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn each_verify_writable_call_counts_one_attempt(
    ) -> Result<(), Box<dyn Error>> {
        let temp = tempdir()?;
        let before = probe_attempts();
        verify_writable(temp.path())?;
        verify_writable(temp.path())?;
        assert_eq!(probe_attempts() - before, 2);
        Ok(())
    }

    #[test]
    fn a_probe_against_an_uncreatable_directory_still_counts(
    ) -> Result<(), Box<dyn Error>> {
        let temp = tempdir()?;
        let blocker = temp.path().join("blocker");
        std::fs::write(&blocker, b"not a directory")?;
        let target = blocker.join("cache");
        let before = probe_attempts();
        assert!(
            verify_writable(&target).is_err(),
            "a directory beneath a regular file cannot be created"
        );
        assert!(
            !target.exists(),
            "create_dir_all must have failed, or this test no longer \
             discriminates"
        );
        assert_eq!(probe_attempts() - before, 1);
        Ok(())
    }

    #[test]
    fn an_override_is_used_verbatim_without_touching_the_filesystem(
    ) -> Result<(), Box<dyn Error>> {
        let temp = tempdir()?;
        let override_dir = temp.path().join("some-override-dir");
        let resolved = candidate(&CacheRootConfig {
            cache_dir_override: Some(override_dir.clone()),
            ..config()
        })?;
        assert_eq!(resolved, override_dir);
        assert!(
            !override_dir.exists(),
            "candidate must not create any directory"
        );
        Ok(())
    }
}
