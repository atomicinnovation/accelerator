//! The filesystem corpus store: whole-file atomic writes and canonical-order
//! JSONL append/remove behind the corpus ports.
//!
//! Built over the shared `store` crate's `atomic_write` and the mkdir-lock.
//! Every write is bounded by the store's root, so a target resolving outside it
//! through a symlink is refused.

use std::fs;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use corpus::{AtomicWrite, Record, RecordStore, StoreError};
use store::{NewFileMode, WriteBounds, WriteError};

use crate::jsonl::{compose_record, remove_prefix};
use crate::lock::{self, LockOptions};

/// A corpus store rooted at a directory that bounds every write.
///
/// The fresh-file mode is resolved from the umask once at construction rather
/// than per write, so a concurrent `append_record` never races the
/// process-global `umask`.
pub struct FileCorpusStore {
    root: PathBuf,
    lock: LockOptions,
    fresh_mode: u32,
}

impl FileCorpusStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::with_lock_options(root, LockOptions::default())
    }

    #[must_use]
    pub fn with_lock_options(
        root: impl Into<PathBuf>,
        lock: LockOptions,
    ) -> Self {
        Self {
            root: root.into(),
            lock,
            fresh_mode: 0o666 & !store::current_umask(),
        }
    }

    fn bounds(&self) -> WriteBounds<'_> {
        WriteBounds {
            permitted_root: &self.root,
            project_root: &self.root,
        }
    }

    fn write_atomic(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        store::atomic_write(
            path,
            bytes,
            &self.bounds(),
            NewFileMode::PreserveOr(self.fresh_mode),
        )
        .map_err(to_store_error)
    }
}

fn lockdir(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".lockdir");
    PathBuf::from(name)
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

fn to_store_error(error: WriteError) -> StoreError {
    match error {
        WriteError::NotWritable { path } => StoreError::NotWritable { path },
        WriteError::CrossFilesystem { path } => {
            StoreError::CrossFilesystem { path }
        }
        WriteError::UnsafePath { path } => StoreError::UnsafePath { path },
        WriteError::Io { path, detail } => StoreError::Io { path, detail },
        other => StoreError::Io {
            path: String::new(),
            detail: other.to_string(),
        },
    }
}

impl AtomicWrite for FileCorpusStore {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.write_atomic(path, bytes)
    }
}

impl RecordStore for FileCorpusStore {
    fn append_record(
        &self,
        path: &Path,
        record: &Record,
    ) -> Result<(), StoreError> {
        let line = compose_record(record)?;
        // The mkdir-lock needs the parent to exist before acquiring, so the
        // parent is created before locking — but only after the containment
        // check, so a symlinked component cannot redirect the tree that is built.
        store::ensure_contained(path, &self.bounds())
            .map_err(to_store_error)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| io(parent, &error))?;
        }
        let _guard = lock::acquire(&lockdir(path), self.lock)?;
        let mut content = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(io(path, &error)),
        };
        if content.last().is_some_and(|byte| *byte != b'\n') {
            content.push(b'\n');
        }
        content.extend_from_slice(line.as_bytes());
        content.push(b'\n');
        self.write_atomic(path, &content)
    }

    fn remove_by_key(&self, path: &Path, key: &str) -> Result<(), StoreError> {
        if !path.exists() {
            return Ok(());
        }
        store::ensure_contained(path, &self.bounds())
            .map_err(to_store_error)?;
        let prefix = remove_prefix(key)?;
        let _guard = lock::acquire(&lockdir(path), self.lock)?;
        let existing = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(());
            }
            Err(error) => return Err(io(path, &error)),
        };
        let mut out = String::with_capacity(existing.len());
        for line in existing.lines() {
            if !line.starts_with(&prefix) {
                out.push_str(line);
                out.push('\n');
            }
        }
        self.write_atomic(path, out.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use corpus::{AtomicWrite, StoreError};
    use tempfile::TempDir;

    use super::FileCorpusStore;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn a_write_through_the_port_replaces_existing_content(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("log.jsonl");
        fs::write(&target, b"old contents")?;
        FileCorpusStore::new(dir.path()).write(&target, b"new")?;
        assert_eq!(fs::read(&target)?, b"new");
        Ok(())
    }

    #[test]
    fn a_write_escaping_the_root_through_a_symlink_is_refused(
    ) -> Result<(), TestError> {
        let root = TempDir::new()?;
        let elsewhere = TempDir::new()?;
        let outside = elsewhere.path().join("stolen.jsonl");
        fs::write(&outside, b"secret")?;
        let target = root.path().join("log.jsonl");
        std::os::unix::fs::symlink(&outside, &target)?;
        assert!(matches!(
            FileCorpusStore::new(root.path()).write(&target, b"new"),
            Err(StoreError::UnsafePath { .. })
        ));
        assert_eq!(
            fs::read(&outside)?,
            b"secret",
            "the symlink target must not be clobbered"
        );
        Ok(())
    }
}
