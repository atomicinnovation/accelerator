//! Resolving the launcher's plugin root, lockhash namespace and state
//! directory.
//!
//! The digest here is the one number that must agree with a script this change
//! does not delete. `ensure-playwright.sh` is the only thing that populates the
//! namespace, and it computes `sha256sum FILE | cut -c1-8` over the shipped
//! `package-lock.json`. Off by anything at all and the executor looks in a
//! directory the surviving bootstrap never fills, so every invocation returns
//! `playwright-not-installed` at exit 3 on a correctly-bootstrapped machine —
//! a failure that looks like a broken install rather than a broken port.

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use design::executor::ports::PathResolution;
use sha2::Digest as _;

/// The number of hex characters the shell's `cut -c1-8` keeps.
const LOCKHASH_LENGTH: usize = 8;

/// Where the state directory sits under the repository, matching the shell.
const STATE_DIR_LEAF: &str = "inventory-design-playwright";
const BOOTSTRAP_LOG: &str = "server.bootstrap.log";

/// The variable overriding the cache root, and the default when it is unset.
const CACHE_OVERRIDE: &str = "ACCELERATOR_PLAYWRIGHT_CACHE";
const DEFAULT_CACHE_SUFFIX: &str = ".cache/accelerator/playwright";

/// The launcher's resolved locations.
pub struct HostPaths {
    plugin_root: Option<PathBuf>,
    /// The repository-relative temporary directory, already joined onto the
    /// repository root by the caller.
    state_dir: PathBuf,
}

impl HostPaths {
    /// Reads the plugin root from the environment, the way every other
    /// composition root in this workspace does.
    #[must_use]
    pub fn new(state_dir: PathBuf) -> Self {
        Self {
            plugin_root: std::env::var_os("ACCELERATOR_PLUGIN_ROOT")
                .map(PathBuf::from),
            state_dir,
        }
    }

    /// The state directory for a repository root and its configured temporary
    /// path.
    #[must_use]
    pub fn state_dir_for(
        repository_root: &Path,
        tmp_relative: &Path,
    ) -> PathBuf {
        repository_root.join(tmp_relative).join(STATE_DIR_LEAF)
    }
}

/// The first eight lowercase hex characters of the file's SHA-256.
///
/// # Errors
///
/// A [`kernel::Error`] when the lockfile cannot be read.
pub fn lockhash(lockfile: &Path) -> Result<String, kernel::Error> {
    let bytes = std::fs::read(lockfile).map_err(|error| {
        kernel::Error::Failed(format!(
            "could not read {} to resolve the Playwright namespace: {error}",
            lockfile.display()
        ))
    })?;
    Ok(lockhash_of(&bytes))
}

/// The digest over bytes already in hand, so the arithmetic is testable without
/// a filesystem.
#[must_use]
pub fn lockhash_of(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    let mut hex = String::with_capacity(LOCKHASH_LENGTH);
    for byte in digest.iter().take(LOCKHASH_LENGTH.div_ceil(2)) {
        let _ = write!(hex, "{byte:02x}");
    }
    hex.truncate(LOCKHASH_LENGTH);
    hex
}

/// The cache root the namespace sits under.
///
/// # Errors
///
/// A [`kernel::Error`] when neither the override nor a home directory is
/// available, since there is then nowhere the runtime could be installed.
pub fn cache_root() -> Result<PathBuf, kernel::Error> {
    if let Some(overridden) = std::env::var_os(CACHE_OVERRIDE) {
        if !overridden.is_empty() {
            return Ok(PathBuf::from(overridden));
        }
    }
    let home = std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .ok_or_else(|| {
            kernel::Error::Failed(format!(
                "neither {CACHE_OVERRIDE} nor HOME is set, so the Playwright \
                 cache root cannot be resolved"
            ))
        })?;
    Ok(PathBuf::from(home).join(DEFAULT_CACHE_SUFFIX))
}

impl PathResolution for HostPaths {
    fn plugin_root(&self) -> Result<PathBuf, kernel::Error> {
        self.plugin_root.clone().ok_or_else(|| {
            kernel::Error::Failed(
                "ACCELERATOR_PLUGIN_ROOT is not set, so the executor cannot \
                 locate the Playwright runner. A dispatched sub-binary runs \
                 from the launcher's cache directory, so it cannot derive the \
                 plugin root from its own path."
                    .to_owned(),
            )
        })
    }

    fn namespace_root(&self) -> Result<PathBuf, kernel::Error> {
        let lockfile = self
            .plugin_root()?
            .join("skills/design/inventory-design/scripts/playwright")
            .join("package-lock.json");
        Ok(cache_root()?.join(lockhash(&lockfile)?))
    }

    fn bootstrap_log(&self) -> Result<PathBuf, kernel::Error> {
        Ok(self.state_dir.join(BOOTSTRAP_LOG))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use design::executor::ports::PathResolution as _;

    use super::lockhash_of;
    use super::HostPaths;

    type TestError = Box<dyn std::error::Error>;

    /// `sha256sum` of the empty input, first eight characters — a value that can
    /// be checked against any host's own tooling.
    #[test]
    fn the_digest_is_the_first_eight_hex_characters() {
        assert_eq!(lockhash_of(b""), "e3b0c442");
        assert_eq!(lockhash_of(b"abc"), "ba7816bf");
    }

    #[test]
    fn the_digest_is_lowercase_hex_of_exactly_eight_characters() {
        for input in [b"".as_slice(), b"x", b"{\n  \"name\": \"x\"\n}\n"] {
            let hex = lockhash_of(input);
            assert_eq!(hex.len(), 8, "{hex}");
            assert!(
                hex.bytes().all(|byte| byte.is_ascii_digit()
                    || (b'a'..=b'f').contains(&byte)),
                "{hex}"
            );
        }
    }

    /// The digest is over the file's bytes exactly, with no trailing-newline or
    /// whitespace normalisation — the shell hashed the file, not its content
    /// interpreted.
    #[test]
    fn a_trailing_newline_changes_the_digest() {
        assert_ne!(lockhash_of(b"{}"), lockhash_of(b"{}\n"));
    }

    /// The one path-bearing failure a caller cannot diagnose from a stack
    /// trace, so it names the variable and says why the binary cannot infer it.
    #[test]
    fn an_unset_plugin_root_refuses_with_a_named_error() -> Result<(), TestError>
    {
        let paths = HostPaths {
            plugin_root: None,
            state_dir: PathBuf::from("/state"),
        };
        let Err(error) = paths.plugin_root() else {
            return Err("expected a refusal".into());
        };
        let message = error.to_string();
        assert!(message.contains("ACCELERATOR_PLUGIN_ROOT"));
        assert!(message.contains("cache directory"));
        Ok(())
    }

    #[test]
    fn the_bootstrap_log_sits_in_the_state_directory() -> Result<(), TestError>
    {
        let paths = HostPaths {
            plugin_root: None,
            state_dir: PathBuf::from("/repo/.tmp/inventory-design-playwright"),
        };
        assert_eq!(
            paths.bootstrap_log()?,
            Path::new(
                "/repo/.tmp/inventory-design-playwright/server.bootstrap.log"
            )
        );
        Ok(())
    }

    /// The leaf the shell appended, so an existing install's state directory is
    /// the one this port finds.
    #[test]
    fn the_state_directory_keeps_the_shell_s_layout() {
        assert_eq!(
            HostPaths::state_dir_for(
                Path::new("/repo"),
                Path::new(".accelerator/tmp")
            ),
            Path::new("/repo/.accelerator/tmp/inventory-design-playwright")
        );
    }
}
