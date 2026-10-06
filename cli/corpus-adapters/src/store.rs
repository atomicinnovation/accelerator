//! The filesystem corpus store: whole-file atomic writes and canonical-order
//! JSONL append/remove behind the corpus ports.
//!
//! Built over the shared `store` crate's `atomic_write` and mkdir-lock.
//! Every write is bounded by the store's root, so a target resolving outside it
//! through a symlink is refused.

use std::fs;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use corpus::{AtomicWrite, FileRemove, Record, RecordStore, StoreError};
use store::lock::{self, LockOptions};
use store::{NewFileMode, WriteBounds, WriteError};

use crate::jsonl::{compose_record, remove_prefix};

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

    /// Replaces `path`'s whole content under the same lock
    /// `append_record`/`remove_by_key` take, so a whole-file rewrite (a
    /// format cutover, say) participates in the same critical section as
    /// every other writer of that path.
    ///
    /// # Errors
    /// [`StoreError`] on containment failure, lock-acquisition timeout, or
    /// I/O.
    pub fn replace_locked(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        store::ensure_contained(path, &self.bounds())
            .map_err(to_store_error)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| io(parent, &error))?;
        }
        let _guard = lock::acquire(&lockdir(path), self.lock)
            .map_err(from_lock_error)?;
        self.write_atomic(path, bytes)
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

fn from_lock_error(error: lock::LockError) -> StoreError {
    match error {
        lock::LockError::Timeout { path } => StoreError::LockTimeout { path },
        lock::LockError::NotWritable { path } => {
            StoreError::NotWritable { path }
        }
        lock::LockError::Io { path, detail } => StoreError::Io { path, detail },
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

impl FileRemove for FileCorpusStore {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        store::ensure_contained(path, &self.bounds())
            .map_err(to_store_error)?;
        match fs::remove_file(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(io(path, &error))
            }
            _ => Ok(()),
        }
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
        let _guard = lock::acquire(&lockdir(path), self.lock)
            .map_err(from_lock_error)?;
        let mut content = store::read_within(path, &self.bounds())
            .map_err(to_store_error)?
            .unwrap_or_default();
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
        let _guard = lock::acquire(&lockdir(path), self.lock)
            .map_err(from_lock_error)?;
        let Some(bytes) =
            store::read_within(path, &self.bounds()).map_err(to_store_error)?
        else {
            return Ok(());
        };
        let existing =
            String::from_utf8(bytes).map_err(|error| StoreError::Io {
                path: show(path),
                detail: error.to_string(),
            })?;
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

    use corpus::{
        AtomicWrite, FileRemove, Outcome, Record, RecordStore, StoreError,
    };
    use store::lock::{acquire, LockError, LockOptions};
    use tempfile::TempDir;

    use super::{from_lock_error, FileCorpusStore};

    type TestError = Box<dyn std::error::Error>;

    fn record() -> Record {
        Record {
            transformation_key: "greeting".to_owned(),
            schema_version: 1,
            outcome: Outcome::Accepted,
            proposed_value: "hello".to_owned(),
            user_value: None,
            timestamp: "2026-07-19T00:00:00+00:00".to_owned(),
            extras: Vec::new(),
        }
    }

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

    #[test]
    fn remove_by_key_on_invalid_utf8_errors_and_leaves_the_file(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("log.jsonl");
        let bad = b"\xff\xfe not valid utf8\n";
        fs::write(&target, bad)?;
        let result =
            FileCorpusStore::new(dir.path()).remove_by_key(&target, "greeting");
        assert!(
            result.is_err(),
            "invalid UTF-8 must not be silently rewritten"
        );
        assert_eq!(
            fs::read(&target)?,
            bad,
            "the file must be left byte-identical"
        );
        Ok(())
    }

    #[test]
    fn append_record_refuses_a_symlinked_intermediate_component(
    ) -> Result<(), TestError> {
        let root = TempDir::new()?;
        let elsewhere = TempDir::new()?;
        std::os::unix::fs::symlink(elsewhere.path(), root.path().join("sub"))?;
        let target = root.path().join("sub").join("log.jsonl");
        let result =
            FileCorpusStore::new(root.path()).append_record(&target, &record());
        assert!(matches!(result, Err(StoreError::UnsafePath { .. })));
        assert!(
            !elsewhere.path().join("log.jsonl").exists(),
            "no file may be created outside the root"
        );
        Ok(())
    }

    #[test]
    fn replace_locked_overwrites_the_whole_file() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("log.jsonl");
        fs::write(&target, b"old contents\nmore\n")?;
        FileCorpusStore::new(dir.path()).replace_locked(&target, b"new")?;
        assert_eq!(fs::read(&target)?, b"new");
        Ok(())
    }

    #[test]
    fn replace_locked_creates_missing_parent_directories(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("state").join("log.jsonl");
        FileCorpusStore::new(dir.path()).replace_locked(&target, b"new")?;
        assert_eq!(fs::read(&target)?, b"new");
        Ok(())
    }

    #[test]
    fn replace_locked_refuses_a_symlinked_intermediate_component(
    ) -> Result<(), TestError> {
        let root = TempDir::new()?;
        let elsewhere = TempDir::new()?;
        std::os::unix::fs::symlink(elsewhere.path(), root.path().join("sub"))?;
        let target = root.path().join("sub").join("log.jsonl");
        let result =
            FileCorpusStore::new(root.path()).replace_locked(&target, b"new");
        assert!(matches!(result, Err(StoreError::UnsafePath { .. })));
        assert!(!elsewhere.path().join("log.jsonl").exists());
        Ok(())
    }

    #[test]
    fn replace_locked_contends_on_the_same_lockdir_as_append_record(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("log.jsonl");
        fs::write(&target, b"existing\n")?;
        let fast = LockOptions {
            ceiling_ms: 1,
            base_ms: 1,
            cap_ms: 1,
        };
        let held = acquire(&super::lockdir(&target), fast)?;
        let result = FileCorpusStore::with_lock_options(dir.path(), fast)
            .replace_locked(&target, b"blocked");
        assert!(matches!(result, Err(StoreError::LockTimeout { .. })));
        drop(held);
        Ok(())
    }

    #[test]
    fn a_removal_through_the_port_deletes_the_file() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("ledger.json");
        fs::write(&target, b"{}")?;
        FileCorpusStore::new(dir.path()).remove(&target)?;
        assert!(!target.exists());
        Ok(())
    }

    #[test]
    fn removing_an_absent_file_succeeds() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        FileCorpusStore::new(dir.path())
            .remove(&dir.path().join("ledger.json"))?;
        Ok(())
    }

    #[test]
    fn a_removal_escaping_the_root_through_a_symlink_is_refused(
    ) -> Result<(), TestError> {
        let root = TempDir::new()?;
        let elsewhere = TempDir::new()?;
        let outside = elsewhere.path().join("ledger.json");
        fs::write(&outside, b"{}")?;
        std::os::unix::fs::symlink(elsewhere.path(), root.path().join("sub"))?;
        let result = FileCorpusStore::new(root.path())
            .remove(&root.path().join("sub").join("ledger.json"));
        assert!(matches!(result, Err(StoreError::UnsafePath { .. })));
        assert!(outside.exists());
        Ok(())
    }

    #[test]
    fn a_lock_timeout_is_a_store_lock_timeout_with_the_same_text() {
        let lock = LockError::Timeout {
            path: "/c/log.jsonl.lockdir".to_owned(),
        };
        let store = from_lock_error(lock.clone());
        assert_eq!(
            store,
            StoreError::LockTimeout {
                path: "/c/log.jsonl.lockdir".to_owned()
            }
        );
        assert_eq!(store.to_string(), lock.to_string());
    }

    #[test]
    fn an_unwritable_lockdir_is_a_store_not_writable_with_the_same_text() {
        let lock = LockError::NotWritable {
            path: "/c/log.jsonl.lockdir".to_owned(),
        };
        let store = from_lock_error(lock.clone());
        assert_eq!(
            store,
            StoreError::NotWritable {
                path: "/c/log.jsonl.lockdir".to_owned()
            }
        );
        assert_eq!(store.to_string(), lock.to_string());
    }

    #[test]
    fn a_lock_io_failure_is_a_store_io_failure_with_the_same_text() {
        let lock = LockError::Io {
            path: "/c/log.jsonl.lockdir".to_owned(),
            detail: "Not a directory".to_owned(),
        };
        let store = from_lock_error(lock.clone());
        assert_eq!(
            store,
            StoreError::Io {
                path: "/c/log.jsonl.lockdir".to_owned(),
                detail: "Not a directory".to_owned()
            }
        );
        assert_eq!(store.to_string(), lock.to_string());
    }
}
