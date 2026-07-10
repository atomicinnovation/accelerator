//! The filesystem side of the legacy-layout guard: it stats the two paths at a
//! root and applies the domain predicate.

use std::path::Path;

use config::ConfigError;

/// Fails when the legacy `.claude/accelerator.md` layout blocks reading.
///
/// # Errors
///
/// [`ConfigError::LegacyLayout`] when the team file is absent and the legacy
/// file is present.
pub fn assert_no_legacy_layout(root: &Path) -> Result<(), ConfigError> {
    let team = root.join(".accelerator/config.md").exists();
    let legacy = root.join(".claude/accelerator.md").exists();
    if config::legacy::is_blocked(team, legacy) {
        return Err(ConfigError::LegacyLayout);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use config::ConfigError;

    use super::assert_no_legacy_layout;

    type TestError = Box<dyn std::error::Error>;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tempdir() -> Result<PathBuf, TestError> {
        let dir = std::env::temp_dir().join(format!(
            "cfg-legacy-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn blocks_a_legacy_only_layout() -> Result<(), TestError> {
        let root = tempdir()?;
        fs::create_dir_all(root.join(".claude"))?;
        fs::write(root.join(".claude/accelerator.md"), "legacy")?;
        assert!(matches!(
            assert_no_legacy_layout(&root),
            Err(ConfigError::LegacyLayout)
        ));
        Ok(())
    }

    #[test]
    fn allows_a_migrated_layout() -> Result<(), TestError> {
        let root = tempdir()?;
        fs::create_dir_all(root.join(".accelerator"))?;
        fs::write(root.join(".accelerator/config.md"), "---\n---\n")?;
        fs::create_dir_all(root.join(".claude"))?;
        fs::write(root.join(".claude/accelerator.md"), "legacy")?;
        assert!(assert_no_legacy_layout(&root).is_ok());
        Ok(())
    }

    #[test]
    fn allows_a_repo_with_neither_file() -> Result<(), TestError> {
        let root = tempdir()?;
        assert!(assert_no_legacy_layout(&root).is_ok());
        Ok(())
    }
}
