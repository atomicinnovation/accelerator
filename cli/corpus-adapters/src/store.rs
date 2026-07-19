//! The filesystem atomic-store: same-directory-temp + atomic rename behind the
//! `AtomicWrite` port. The `stage`/`persist` split is the fault-injection seam
//! the interruption invariant is tested through.

use std::fs;
use std::io::Error as IoError;
use std::io::Write as _;
use std::path::Path;

use corpus::{AtomicWrite, StoreError};
use tempfile::NamedTempFile;

pub struct FileCorpusStore;

impl FileCorpusStore {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for FileCorpusStore {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn atomic_write(
    path: &Path,
    bytes: &[u8],
) -> Result<(), StoreError> {
    let staged = stage(path, bytes)?;
    persist(staged, path)
}

fn stage(path: &Path, bytes: &[u8]) -> Result<NamedTempFile, StoreError> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        fs::create_dir_all(parent).map_err(|error| io(parent, &error))?;
    }
    let dir = parent.unwrap_or_else(|| Path::new("."));
    let mut temp = NamedTempFile::new_in(dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            StoreError::NotWritable { path: show(dir) }
        } else {
            io(dir, &error)
        }
    })?;
    temp.write_all(bytes).map_err(|error| io(dir, &error))?;
    Ok(temp)
}

fn persist(temp: NamedTempFile, path: &Path) -> Result<(), StoreError> {
    temp.persist(path)
        .map(|_| ())
        .map_err(|error| classify_persist_error(path, &error.error))
}

fn classify_persist_error(path: &Path, error: &IoError) -> StoreError {
    if error.raw_os_error() == Some(libc::EXDEV) {
        StoreError::CrossFilesystem { path: show(path) }
    } else {
        io(path, error)
    }
}

fn show(path: &Path) -> String {
    path.display().to_string()
}

fn io(path: &Path, error: &IoError) -> StoreError {
    StoreError::Io {
        path: show(path),
        detail: error.to_string(),
    }
}

impl AtomicWrite for FileCorpusStore {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        atomic_write(path, bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use corpus::{AtomicWrite, StoreError};
    use tempfile::TempDir;

    use super::{atomic_write, classify_persist_error, stage, FileCorpusStore};

    type TestError = Box<dyn std::error::Error>;

    fn temp_names(dir: &Path) -> Result<Vec<String>, TestError> {
        let mut names = Vec::new();
        for entry in fs::read_dir(dir)? {
            names.push(entry?.file_name().to_string_lossy().into_owned());
        }
        Ok(names)
    }

    #[test]
    fn the_temp_is_staged_in_the_targets_directory() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("log.jsonl");
        let temp = stage(&target, b"hello")?;
        assert_eq!(temp.path().parent(), Some(dir.path()));
        Ok(())
    }

    #[test]
    fn a_successful_write_leaves_no_stray_temp() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("log.jsonl");
        atomic_write(&target, b"hello")?;
        assert_eq!(fs::read(&target)?, b"hello");
        assert_eq!(temp_names(dir.path())?, vec!["log.jsonl".to_owned()]);
        Ok(())
    }

    #[test]
    fn a_write_through_the_port_replaces_existing_content(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("log.jsonl");
        fs::write(&target, b"old contents")?;
        FileCorpusStore::new().write(&target, b"new")?;
        assert_eq!(fs::read(&target)?, b"new");
        assert_eq!(temp_names(dir.path())?, vec!["log.jsonl".to_owned()]);
        Ok(())
    }

    #[test]
    fn a_write_creates_the_parent_directory() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("nested/deeper/log.jsonl");
        atomic_write(&target, b"hello")?;
        assert_eq!(fs::read(&target)?, b"hello");
        Ok(())
    }

    #[test]
    fn interruption_before_rename_leaves_existing_content_intact(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("log.jsonl");
        fs::write(&target, b"seeded")?;
        {
            let _staged = stage(&target, b"never persisted")?;
        }
        assert_eq!(fs::read(&target)?, b"seeded");
        assert_eq!(temp_names(dir.path())?, vec!["log.jsonl".to_owned()]);
        Ok(())
    }

    #[test]
    fn interruption_before_rename_leaves_a_fresh_path_absent(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("log.jsonl");
        {
            let _staged = stage(&target, b"never persisted")?;
        }
        assert!(!target.exists());
        assert!(temp_names(dir.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn cross_filesystem_errno_classifies_as_cross_filesystem() {
        let error = std::io::Error::from_raw_os_error(libc::EXDEV);
        assert_eq!(
            classify_persist_error(Path::new("/x/log"), &error),
            StoreError::CrossFilesystem {
                path: "/x/log".to_owned()
            }
        );
    }

    #[test]
    fn any_other_errno_classifies_as_io() {
        let error = std::io::Error::from_raw_os_error(libc::ENOENT);
        assert!(matches!(
            classify_persist_error(Path::new("/x/log"), &error),
            StoreError::Io { .. }
        ));
    }
}
