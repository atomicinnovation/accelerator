//! The applied/skipped ledger's file-backed persistence:
//! `.accelerator/state/migrations-applied` and `-skipped`, one ID per line.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use migrate::ports::LedgerStore;
use migrate::ports::MigrationError;
use store::NewFileMode;
use store::WriteBounds;

pub struct FileLedgerStore {
    root: PathBuf,
    fresh_mode: u32,
}

impl FileLedgerStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            fresh_mode: 0o666 & !store::current_umask(),
        }
    }

    fn bounds(&self) -> WriteBounds<'_> {
        WriteBounds {
            permitted_root: &self.root,
            project_root: &self.root,
        }
    }

    fn applied_path(&self) -> PathBuf {
        self.root.join(".accelerator/state/migrations-applied")
    }

    fn skipped_path(&self) -> PathBuf {
        self.root.join(".accelerator/state/migrations-skipped")
    }

    fn read(&self, path: &Path) -> Result<Vec<String>, MigrationError> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let bytes = store::read_within(path, &self.bounds())
            .map_err(|error| MigrationError::new(error.to_string()))?
            .unwrap_or_default();
        let text = String::from_utf8(bytes)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        Ok(text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect())
    }

    fn write(&self, path: &Path, ids: &[String]) -> Result<(), MigrationError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| MigrationError::new(error.to_string()))?;
        }
        let mut content = String::new();
        for id in ids {
            content.push_str(id);
            content.push('\n');
        }
        store::atomic_write(
            path,
            content.as_bytes(),
            &self.bounds(),
            NewFileMode::PreserveOr(self.fresh_mode),
        )
        .map_err(|error| MigrationError::new(error.to_string()))
    }
}

impl LedgerStore for FileLedgerStore {
    fn applied(&self) -> Result<Vec<String>, MigrationError> {
        self.read(&self.applied_path())
    }

    fn write_applied(&self, ids: &[String]) -> Result<(), MigrationError> {
        self.write(&self.applied_path(), ids)
    }

    fn skipped(&self) -> Result<Vec<String>, MigrationError> {
        self.read(&self.skipped_path())
    }

    fn write_skipped(&self, ids: &[String]) -> Result<(), MigrationError> {
        self.write(&self.skipped_path(), ids)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::FileLedgerStore;
    use migrate::ports::LedgerStore;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn an_absent_ledger_reads_as_empty() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let store = FileLedgerStore::new(dir.path());
        assert_eq!(store.applied()?, Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn a_written_ledger_reads_back_in_order() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let store = FileLedgerStore::new(dir.path());
        store.write_applied(&["0001".to_owned(), "0002".to_owned()])?;
        assert_eq!(
            store.applied()?,
            vec!["0001".to_owned(), "0002".to_owned()]
        );
        Ok(())
    }

    #[test]
    fn blank_lines_are_ignored_on_read() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let path = dir.path().join(".accelerator/state/migrations-applied");
        std::fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        std::fs::write(&path, "0001\n\n0002\n")?;
        let store = FileLedgerStore::new(dir.path());
        assert_eq!(
            store.applied()?,
            vec!["0001".to_owned(), "0002".to_owned()]
        );
        Ok(())
    }
}
