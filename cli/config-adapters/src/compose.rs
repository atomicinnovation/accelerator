//! The wiring protocol as a single tested helper: discover the root once, run
//! the legacy guard against it, probe the personal file, then build the store
//! and service rooted at the same directory.

use std::path::Path;

use config::consent::Refusal;
use config::{ConfigError, ConfigService, PersonalFile};

use crate::legacy;
use crate::screen::ScreenedStore;
use crate::store::{FileConfigStore, LegacyPolicy};

/// The composed configuration ports handed to the composition root.
///
/// The service reads through `screened` and writes through the checked store.
/// `screened` also serves the view-assembling block subcommands as a raw level
/// reader, so both read paths tolerate an ignored personal file alike. `store`
/// serves only the ports that never read a config level.
pub struct Composed {
    pub service: ConfigService<ScreenedStore, FileConfigStore>,
    pub store: FileConfigStore,
    pub screened: ScreenedStore,
}

impl Composed {
    /// Composes the ports over an already-rooted store: probes its personal
    /// file once and screens every read through that fact.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Io`] when the personal file's metadata cannot be read.
    pub fn over(store: FileConfigStore) -> Result<Self, ConfigError> {
        let personal_file = store.probe_personal_file()?;
        let screened = ScreenedStore::new(store.clone(), personal_file.clone());
        let service = ConfigService::with_personal_file(
            screened.clone(),
            store.clone(),
            personal_file,
        );
        Ok(Self {
            service,
            store,
            screened,
        })
    }

    #[must_use]
    pub const fn personal_file(&self) -> &PersonalFile {
        self.screened.personal_file()
    }

    /// Reports an ignored personal file on stderr, once per process however
    /// many times the process composes.
    pub fn report_ignored_personal_file(&self) {
        if let Some(refusal) = Refusal::for_personal_file(self.personal_file())
        {
            kernel::render::personal_file_warning(&refusal);
        }
    }

    /// For a command that writes: its writes would persist after the user
    /// fixed the file's mode, so it refuses rather than run on team values.
    ///
    /// # Errors
    ///
    /// [`ConfigError::InsecurePersonalFile`] when the personal file is ignored.
    pub fn require_readable_personal_file(&self) -> Result<(), ConfigError> {
        self.personal_file().require_readable()
    }
}

/// Wires the configuration ports at `cwd`'s project root.
///
/// Under [`LegacyPolicy::Reject`] it fails closed on the legacy layout; under
/// [`LegacyPolicy::Allow`] it suppresses that refusal and reads the legacy pair
/// when the current one is absent.
///
/// # Errors
///
/// [`ConfigError::LegacyLayout`] when the discovered root carries the legacy
/// `.claude/accelerator.md` layout and the policy is [`LegacyPolicy::Reject`];
/// [`ConfigError::Io`] when the personal file's metadata cannot be read.
pub fn compose(
    cwd: &Path,
    policy: LegacyPolicy,
) -> Result<Composed, ConfigError> {
    let root = FileConfigStore::discover_root(cwd);
    if policy == LegacyPolicy::Reject {
        legacy::assert_no_legacy_layout(&root)?;
    }
    Composed::over(FileConfigStore::at(root).with_legacy_policy(policy))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use config::{ConfigAccess, ConfigError, Key, Resolved, Scalar, Value};
    use tempfile::TempDir;

    use super::compose;
    use crate::store::LegacyPolicy;

    type TestError = Box<dyn std::error::Error>;

    fn tempdir() -> Result<TempDir, TestError> {
        let dir = tempfile::Builder::new().prefix("cfg-compose-").tempdir()?;
        fs::create_dir_all(dir.path().join(".git"))?;
        Ok(dir)
    }

    #[test]
    fn composes_a_reader_rooted_at_the_discovered_root() -> Result<(), TestError>
    {
        let root = tempdir()?;
        fs::create_dir_all(root.path().join(".accelerator"))?;
        fs::write(
            root.path().join(".accelerator/config.md"),
            "---\npaths:\n  work: wired\n---\n",
        )?;
        let service = compose(root.path(), LegacyPolicy::Reject)?.service;
        assert_eq!(
            service.get(&Key::parse("paths.work")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::String("wired".to_owned())))
        );
        Ok(())
    }

    #[test]
    fn fails_closed_on_the_legacy_layout() -> Result<(), TestError> {
        let root = tempdir()?;
        fs::create_dir_all(root.path().join(".claude"))?;
        fs::write(root.path().join(".claude/accelerator.md"), "legacy")?;
        assert!(matches!(
            compose(root.path(), LegacyPolicy::Reject),
            Err(ConfigError::LegacyLayout)
        ));
        Ok(())
    }

    #[test]
    fn allow_suppresses_the_refusal_and_reads_the_legacy_pair(
    ) -> Result<(), TestError> {
        use std::os::unix::fs::PermissionsExt as _;

        let root = tempdir()?;
        fs::create_dir_all(root.path().join(".claude"))?;
        fs::write(
            root.path().join(".claude/accelerator.md"),
            "---\npaths:\n  work: legacy-team\n---\n",
        )?;
        let local = root.path().join(".claude/accelerator.local.md");
        fs::write(&local, "---\npaths:\n  work: legacy-local\n---\n")?;
        fs::set_permissions(&local, fs::Permissions::from_mode(0o600))?;
        let service = compose(root.path(), LegacyPolicy::Allow)?.service;
        assert_eq!(
            service.get(&Key::parse("paths.work")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::String(
                "legacy-local".to_owned()
            )))
        );
        Ok(())
    }

    #[test]
    fn the_legacy_fallback_is_inert_when_the_current_pair_is_present(
    ) -> Result<(), TestError> {
        let root = tempdir()?;
        fs::create_dir_all(root.path().join(".accelerator"))?;
        fs::write(
            root.path().join(".accelerator/config.md"),
            "---\npaths:\n  work: current\n---\n",
        )?;
        fs::create_dir_all(root.path().join(".claude"))?;
        fs::write(
            root.path().join(".claude/accelerator.md"),
            "---\npaths:\n  work: legacy\n---\n",
        )?;
        let service = compose(root.path(), LegacyPolicy::Allow)?.service;
        assert_eq!(
            service.get(&Key::parse("paths.work")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::String(
                "current".to_owned()
            )))
        );
        Ok(())
    }
}
