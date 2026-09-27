//! Resolving the launcher's plugin root and state directory.
//!
//! A dispatched sub-binary runs from the launcher's cache directory, so it
//! cannot derive the plugin root from its own path; it reads it from the
//! environment, the way every other composition root in this workspace does.
//! The vendored runtime is resolved by the launcher's tree cache, not from a
//! lockhash namespace, so nothing here computes one.

use std::fmt::Write as _;
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;
use std::path::PathBuf;

use design::executor::ports::PathResolution;
use design::executor::DaemonBrowser;
use sha2::Digest as _;
use sha2::Sha256;

/// Where the state directory sits under the repository.
const STATE_DIR_LEAF: &str = "inventory-design-playwright";
const BOOTSTRAP_LOG: &str = "server.bootstrap.log";
const BUNDLED_SLOT: &str = "bundled";
const CUSTOM_SLOT_PREFIX: &str = "custom-";
const CUSTOM_SLOT_DIGEST_BYTES: usize = 8;

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

    /// The state directory of `browser`'s daemon, for a repository root and
    /// its configured temporary path.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] naming the leaf or slot directory when either is a
    /// symlink, which could alias one browser's slot to another's.
    pub fn state_dir_for(
        repository_root: &Path,
        tmp_relative: &Path,
        browser: &DaemonBrowser,
    ) -> Result<PathBuf, kernel::Error> {
        let leaf = repository_root.join(tmp_relative).join(STATE_DIR_LEAF);
        let slot = leaf.join(slot_name(browser));
        refuse_symlink(&leaf)?;
        refuse_symlink(&slot)?;
        Ok(slot)
    }
}

/// A digest rather than the path itself, which may be too long for a
/// directory name. It must be stable across releases, so `DefaultHasher`
/// cannot serve.
fn slot_name(browser: &DaemonBrowser) -> String {
    match browser {
        DaemonBrowser::Bundled => BUNDLED_SLOT.to_owned(),
        DaemonBrowser::Custom(path) => {
            let digest = Sha256::digest(path.as_os_str().as_bytes());
            digest.iter().take(CUSTOM_SLOT_DIGEST_BYTES).fold(
                String::from(CUSTOM_SLOT_PREFIX),
                |mut name, byte| {
                    let _ = write!(name, "{byte:02x}");
                    name
                },
            )
        }
    }
}

fn refuse_symlink(directory: &Path) -> Result<(), kernel::Error> {
    let is_symlink = std::fs::symlink_metadata(directory)
        .is_ok_and(|metadata| metadata.file_type().is_symlink());
    if is_symlink {
        return Err(kernel::Error::Failed(format!(
            "{} is a symlink, but the executor owns this directory and it must \
             be a real one. It is safe to remove when no crawl is running.",
            directory.display()
        )));
    }
    Ok(())
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
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use std::path::PathBuf;

    use design::executor::ports::PathResolution as _;
    use design::executor::DaemonBrowser;

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

    fn custom(path: &str) -> DaemonBrowser {
        DaemonBrowser::Custom(PathBuf::from(path))
    }

    #[test]
    fn the_bundled_browser_has_its_own_slot_beneath_the_leaf(
    ) -> Result<(), TestError> {
        let repo = tempfile::tempdir()?;
        assert_eq!(
            HostPaths::state_dir_for(
                repo.path(),
                Path::new(".accelerator/tmp"),
                &DaemonBrowser::Bundled
            )?,
            repo.path()
                .join(".accelerator/tmp/inventory-design-playwright/bundled")
        );
        Ok(())
    }

    /// Pinned so a custom browser's daemon is found again after an upgrade.
    #[test]
    fn a_custom_browsers_slot_is_a_stable_digest_of_its_path(
    ) -> Result<(), TestError> {
        let repo = tempfile::tempdir()?;
        assert_eq!(
            HostPaths::state_dir_for(
                repo.path(),
                Path::new(".accelerator/tmp"),
                &custom("/opt/google/chrome/chrome")
            )?,
            repo.path().join(
                ".accelerator/tmp/inventory-design-playwright/\
                 custom-611d62187568662c"
            )
        );
        Ok(())
    }

    #[test]
    fn each_browser_has_a_slot_of_its_own() -> Result<(), TestError> {
        let repo = tempfile::tempdir()?;
        let slot = |browser: &DaemonBrowser| {
            HostPaths::state_dir_for(repo.path(), Path::new("tmp"), browser)
        };
        let bundled = slot(&DaemonBrowser::Bundled)?;
        let chrome = slot(&custom("/opt/chrome"))?;
        let chromium = slot(&custom("/opt/chromium"))?;
        assert_ne!(bundled, chrome);
        assert_ne!(chrome, chromium);
        assert_eq!(chrome, slot(&custom("/opt/chrome"))?);
        Ok(())
    }

    fn assert_refused_naming(
        refused: Result<PathBuf, kernel::Error>,
        symlink: &Path,
    ) -> Result<(), TestError> {
        let Err(error) = refused else {
            return Err("expected a symlink refusal".into());
        };
        let message = error.to_string();
        assert!(
            message.contains(&symlink.display().to_string()),
            "{message}"
        );
        assert!(message.contains("no crawl is running"), "{message}");
        Ok(())
    }

    #[test]
    fn a_symlinked_leaf_is_refused_naming_it() -> Result<(), TestError> {
        let repo = tempfile::tempdir()?;
        let elsewhere = tempfile::tempdir()?;
        fs::create_dir_all(repo.path().join("tmp"))?;
        let leaf = repo.path().join("tmp/inventory-design-playwright");
        symlink(elsewhere.path(), &leaf)?;

        assert_refused_naming(
            HostPaths::state_dir_for(
                repo.path(),
                Path::new("tmp"),
                &DaemonBrowser::Bundled,
            ),
            &leaf,
        )
    }

    #[test]
    fn a_symlinked_slot_is_refused_naming_it() -> Result<(), TestError> {
        let repo = tempfile::tempdir()?;
        let leaf = repo.path().join("tmp/inventory-design-playwright");
        fs::create_dir_all(leaf.join("custom-611d62187568662c"))?;
        let slot = leaf.join("bundled");
        symlink(leaf.join("custom-611d62187568662c"), &slot)?;

        assert_refused_naming(
            HostPaths::state_dir_for(
                repo.path(),
                Path::new("tmp"),
                &DaemonBrowser::Bundled,
            ),
            &slot,
        )
    }

    #[test]
    fn a_symlinked_tmp_base_is_the_users_choice() -> Result<(), TestError> {
        let repo = tempfile::tempdir()?;
        let scratch = tempfile::tempdir()?;
        fs::create_dir_all(repo.path().join(".accelerator"))?;
        symlink(scratch.path(), repo.path().join(".accelerator/tmp"))?;
        fs::create_dir_all(
            scratch.path().join("inventory-design-playwright/bundled"),
        )?;

        assert!(HostPaths::state_dir_for(
            repo.path(),
            Path::new(".accelerator/tmp"),
            &DaemonBrowser::Bundled,
        )
        .is_ok());
        Ok(())
    }

    #[test]
    fn an_absolute_tmp_path_is_used_as_given_and_still_vetted(
    ) -> Result<(), TestError> {
        let repo = tempfile::tempdir()?;
        let tmp = tempfile::tempdir()?;
        assert_eq!(
            HostPaths::state_dir_for(
                repo.path(),
                tmp.path(),
                &DaemonBrowser::Bundled
            )?,
            tmp.path().join("inventory-design-playwright/bundled")
        );

        let leaf = tmp.path().join("inventory-design-playwright");
        fs::create_dir_all(leaf.join("elsewhere"))?;
        let slot = leaf.join("bundled");
        symlink(leaf.join("elsewhere"), &slot)?;
        assert_refused_naming(
            HostPaths::state_dir_for(
                repo.path(),
                tmp.path(),
                &DaemonBrowser::Bundled,
            ),
            &slot,
        )
    }
}
