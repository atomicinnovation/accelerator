//! Resolving the launcher's plugin root and state directory.
//!
//! A dispatched sub-binary runs from the launcher's cache directory, so it
//! cannot derive the plugin root from its own path; it reads it from the
//! environment, the way every other composition root in this workspace does.
//! The vendored runtime is resolved by the launcher's tree cache, not from a
//! lockhash namespace, so nothing here computes one.

use std::path::Path;
use std::path::PathBuf;

use design::executor::ports::PathResolution;

/// Where the state directory sits under the repository.
const STATE_DIR_LEAF: &str = "inventory-design-playwright";
const BOOTSTRAP_LOG: &str = "server.bootstrap.log";

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

    fn bootstrap_log(&self) -> Result<PathBuf, kernel::Error> {
        Ok(self.state_dir.join(BOOTSTRAP_LOG))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use design::executor::ports::PathResolution as _;

    use super::HostPaths;

    type TestError = Box<dyn std::error::Error>;

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

    /// Pinned so an existing install's state directory is the one this port
    /// finds.
    #[test]
    fn the_state_directory_keeps_its_layout() {
        assert_eq!(
            HostPaths::state_dir_for(
                Path::new("/repo"),
                Path::new(".accelerator/tmp")
            ),
            Path::new("/repo/.accelerator/tmp/inventory-design-playwright")
        );
    }
}
