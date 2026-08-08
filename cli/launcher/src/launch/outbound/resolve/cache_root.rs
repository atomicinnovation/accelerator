//! Resolves the runtime cache directory.
//!
//! `${ACCELERATOR_PLUGIN_ROOT}/bin` when writable and exec-capable, else the
//! `ACCELERATOR_CACHE_DIR` override, else a named error. No XDG fallback — an
//! XDG-resident binary would break the plugin-root `allowed-tools` glob match.
//! Read-only/noexec roots are probed.

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
/// hit never pays the write+exec probe [`verify_writable`] performs.
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

/// Resolve a writable, exec-capable cache directory (creating it if needed).
///
/// # Errors
///
/// [`ResolutionError::CacheRootUnavailable`] when no candidate is usable — an
/// unset `ACCELERATOR_PLUGIN_ROOT` with no override, or a candidate failing the
/// write+exec probe (with no XDG fallback).
pub fn resolve(config: &CacheRootConfig) -> Result<PathBuf, ResolutionError> {
    let dir = candidate(config)?;
    verify_writable(&dir)?;
    Ok(dir)
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
pub fn verify_writable(dir: &Path) -> Result<(), ResolutionError> {
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

    use super::{candidate, resolve, verify_writable, CacheRootConfig};

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
        let result = resolve(&config());
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
    fn a_writable_plugin_root_is_used() -> Result<(), Box<dyn Error>> {
        let temp = tempdir()?;
        let temp = temp.path().to_path_buf();
        let resolved = resolve(&CacheRootConfig {
            plugin_root: Some(temp.clone()),
            ..config()
        })?;
        assert_eq!(resolved, temp.join("bin"));
        Ok(())
    }

    #[test]
    fn a_read_only_plugin_root_with_no_override_is_a_named_error(
    ) -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::PermissionsExt as _;
        let plugin_root = tempdir()?;
        let plugin_root = plugin_root.path().to_path_buf();
        std::fs::create_dir_all(plugin_root.join("bin"))?;
        std::fs::set_permissions(
            plugin_root.join("bin"),
            std::fs::Permissions::from_mode(0o555),
        )?;
        let result = resolve(&CacheRootConfig {
            plugin_root: Some(plugin_root),
            ..config()
        });
        assert!(result.is_err(), "no XDG fallback: expected a named error");
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
    fn an_override_is_honoured() -> Result<(), Box<dyn Error>> {
        let temp = tempdir()?;
        let temp = temp.path().to_path_buf();
        let resolved = resolve(&CacheRootConfig {
            cache_dir_override: Some(temp.clone()),
            ..config()
        })?;
        assert_eq!(resolved, temp);
        Ok(())
    }
}
