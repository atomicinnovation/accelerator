//! Resolves the runtime cache directory.
//!
//! `${CLAUDE_PLUGIN_ROOT}/bin` when writable and exec-capable (so the bare-path
//! invocation contract keeps matching `allowed-tools` globs and Claude Code
//! reclaims the cache on upgrade), else the `ACCELERATOR_CACHE_DIR` override,
//! else a named error. There is deliberately **no XDG fallback**: an
//! XDG-resident binary would break the plugin-root `allowed-tools` glob match
//! that motivates the location (0136 resolved constraint). Read-only installs
//! and `noexec` mounts are probed, not inferred.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::launch::core::ResolutionError;

/// The environment inputs the resolution needs — injected so tests supply temp
/// dirs instead of reading the real process environment.
pub struct CacheRootConfig {
    pub cache_dir_override: Option<PathBuf>,
    pub plugin_root: Option<PathBuf>,
}

impl CacheRootConfig {
    /// Read the config from the process environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            cache_dir_override: std::env::var_os("ACCELERATOR_CACHE_DIR")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            plugin_root: std::env::var_os("CLAUDE_PLUGIN_ROOT")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
        }
    }
}

/// Resolve a writable, exec-capable cache directory (creating it if needed).
///
/// # Errors
///
/// [`ResolutionError::CacheRootUnavailable`] when no candidate is usable — an
/// unset `CLAUDE_PLUGIN_ROOT` with no override, or a candidate failing the
/// write+exec probe (with no XDG fallback).
pub fn resolve(config: &CacheRootConfig) -> Result<PathBuf, ResolutionError> {
    if let Some(override_dir) = &config.cache_dir_override {
        // An active override changes only the location, never disabling
        // re-verification of what is fetched into it.
        tracing::info!(
            path = %override_dir.display(),
            "using ACCELERATOR_CACHE_DIR override for the cache root"
        );
        return if probe_writable_and_executable(override_dir) {
            Ok(override_dir.clone())
        } else {
            Err(ResolutionError::CacheRootUnavailable {
                detail: format!(
                    "ACCELERATOR_CACHE_DIR {} is not writable+exec-capable",
                    override_dir.display()
                ),
            })
        };
    }

    let plugin_root = config.plugin_root.as_ref().ok_or_else(|| {
        ResolutionError::CacheRootUnavailable {
            detail: "CLAUDE_PLUGIN_ROOT is not set and no \
                     ACCELERATOR_CACHE_DIR override was given"
                .to_owned(),
        }
    })?;
    let primary = plugin_root.join("bin");
    if probe_writable_and_executable(&primary) {
        return Ok(primary);
    }
    Err(ResolutionError::CacheRootUnavailable {
        detail: format!(
            "{} is not writable+exec-capable and no ACCELERATOR_CACHE_DIR \
             override was given (no XDG fallback)",
            primary.display()
        ),
    })
}

/// Probe a directory for both writability and exec-capability by writing a
/// trivial script and executing it — catching `noexec` mounts, which a
/// write-only probe would miss.
fn probe_writable_and_executable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(format!(".accelerator-probe-{}", std::process::id()));
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
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{resolve, CacheRootConfig};

    fn config() -> CacheRootConfig {
        CacheRootConfig {
            cache_dir_override: None,
            plugin_root: None,
        }
    }

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tempdir() -> Result<PathBuf, Box<dyn Error>> {
        let dir = std::env::temp_dir().join(format!(
            "acc-cacheroot-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn unset_plugin_root_with_no_override_is_a_named_error() {
        let result = resolve(&config());
        assert!(result.is_err(), "expected a CLAUDE_PLUGIN_ROOT error");
        if let Err(error) = result {
            assert!(error.to_string().contains("CLAUDE_PLUGIN_ROOT"));
        }
    }

    #[test]
    fn a_writable_plugin_root_is_used() -> Result<(), Box<dyn Error>> {
        let temp = tempdir()?;
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
    fn an_override_is_honoured() -> Result<(), Box<dyn Error>> {
        let temp = tempdir()?;
        let resolved = resolve(&CacheRootConfig {
            cache_dir_override: Some(temp.clone()),
            ..config()
        })?;
        assert_eq!(resolved, temp);
        Ok(())
    }
}
