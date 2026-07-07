//! The Model-1 wiring protocol as a single tested helper: discover the root
//! once, run the legacy guard against it, then build the store and service
//! rooted at the same directory.

use std::path::Path;

use config::{ConfigError, ConfigService};

use crate::legacy;
use crate::store::FileConfigStore;

/// Wires a full-stack reader at `cwd`'s project root, failing closed on the
/// legacy layout.
///
/// # Errors
///
/// [`ConfigError::LegacyLayout`] when the discovered root carries the legacy
/// `.claude/accelerator.md` layout.
pub fn compose(
    cwd: &Path,
) -> Result<ConfigService<FileConfigStore, FileConfigStore>, ConfigError> {
    let root = FileConfigStore::discover_root(cwd);
    legacy::assert_no_legacy_layout(&root)?;
    let store = FileConfigStore::at(root);
    Ok(ConfigService::new(store.clone(), store))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use config::{ConfigAccess, ConfigError, Key, Resolved, Scalar, Value};

    use super::compose;

    type TestError = Box<dyn std::error::Error>;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tempdir() -> Result<PathBuf, TestError> {
        let dir = std::env::temp_dir().join(format!(
            "cfg-compose-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(dir.join(".git"))?;
        Ok(dir)
    }

    #[test]
    fn composes_a_reader_rooted_at_the_discovered_root() -> Result<(), TestError>
    {
        let root = tempdir()?;
        fs::create_dir_all(root.join(".accelerator"))?;
        fs::write(
            root.join(".accelerator/config.md"),
            "---\npaths:\n  work: wired\n---\n",
        )?;
        let service = compose(&root)?;
        assert_eq!(
            service.get(&Key::parse("paths.work")?, None)?,
            Resolved::Found(Value::Scalar(Scalar::String("wired".to_owned())))
        );
        Ok(())
    }

    #[test]
    fn fails_closed_on_the_legacy_layout() -> Result<(), TestError> {
        let root = tempdir()?;
        fs::create_dir_all(root.join(".claude"))?;
        fs::write(root.join(".claude/accelerator.md"), "legacy")?;
        assert!(matches!(compose(&root), Err(ConfigError::LegacyLayout)));
        Ok(())
    }
}
