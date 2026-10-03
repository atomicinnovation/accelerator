//! The atomic-store error taxonomy and the driven ports the adapter
//! implements.
//!
//! `AtomicWrite` replaces a whole file atomically, `ExclusiveCreate` and
//! `RemoveFile` move one, `RecordStore` appends and removes canonical-order
//! JSONL, and `RecoveryCopies` keeps the pre-change bytes of a multi-file
//! change until it lands.

use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;
use std::path::PathBuf;

use crate::record::Record;

/// An atomic-store operation failure.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotWritable { path: String },
    LockTimeout { path: String },
    CrossFilesystem { path: String },
    UnsafePath { path: String },
    AlreadyExists { path: String },
    Validation { detail: String },
    Io { path: String, detail: String },
}

impl Display for StoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotWritable { path } => {
                write!(formatter, "cannot write under '{path}': not writable")
            }
            Self::LockTimeout { path } => {
                write!(formatter, "lock acquisition timed out on '{path}'")
            }
            Self::CrossFilesystem { path } => write!(
                formatter,
                "atomic rename to '{path}' crossed a filesystem boundary"
            ),
            Self::UnsafePath { path } => write!(
                formatter,
                "refusing to write through an unsafe path '{path}'"
            ),
            Self::AlreadyExists { path } => {
                write!(formatter, "'{path}' already exists")
            }
            Self::Validation { detail } => {
                write!(formatter, "invalid record: {detail}")
            }
            Self::Io { path, detail } => {
                write!(formatter, "I/O error on '{path}': {detail}")
            }
        }
    }
}

impl std::error::Error for StoreError {}

impl From<StoreError> for kernel::Error {
    fn from(error: StoreError) -> Self {
        Self::Failed(error.to_string())
    }
}

/// Whole-file atomic replacement: a reader never observes a partial file.
pub trait AtomicWrite {
    /// # Errors
    /// [`StoreError`] when the destination directory is not writable, the rename
    /// crosses a filesystem boundary, or the write fails.
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError>;
}

/// Creation that never replaces: a reader sees either no file or the whole
/// new one.
pub trait ExclusiveCreate {
    /// # Errors
    /// [`StoreError`] when `path` already exists, the destination is not
    /// writable, or the write fails.
    fn create_new(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError>;
}

pub trait RemoveFile {
    /// # Errors
    /// [`StoreError`] when `path` does not exist or cannot be removed.
    fn remove(&self, path: &Path) -> Result<(), StoreError>;
}

/// Why a recovery directory outlived the change it was made for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeptState {
    /// These originals could not be restored and still await a person.
    RestorePending(Vec<PathBuf>),
    /// The change has since completed; the copies remain for reference.
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptRecovery {
    pub dir: PathBuf,
    pub state: KeptState,
}

/// Write-once copies of files a change is about to overwrite, kept in a
/// named directory outside the corpus until the change has landed.
///
/// Each copy mirrors the original's position in the corpus, so a person can
/// find it without a manifest.
pub trait RecoveryCopies {
    fn exists(&self, dir: &Path) -> bool;

    /// Creates `dir` with an ignore rule for everything in it, verified
    /// before any copy is written, so copies of uncommitted content can never
    /// be committed.
    ///
    /// # Errors
    /// [`StoreError`] when the directory or its ignore rule cannot be
    /// written or verified.
    fn prepare(&self, dir: &Path) -> Result<(), StoreError>;

    /// # Errors
    /// [`StoreError`] when a copy of `original` already exists in `dir`, so
    /// the first copy of a file is never replaced, or the write fails.
    fn write_once(
        &self,
        dir: &Path,
        original: &Path,
        bytes: &[u8],
    ) -> Result<(), StoreError>;

    /// Records in `dir` that `unrestored` still differ from their copies.
    ///
    /// # Errors
    /// [`StoreError`] when the record cannot be written.
    fn mark_restore_pending(
        &self,
        dir: &Path,
        unrestored: &[PathBuf],
    ) -> Result<(), StoreError>;

    fn is_restore_pending(&self, dir: &Path) -> bool;

    /// Replaces a restore-pending record with a notice that the change has
    /// since completed and the copies remain for reference.
    ///
    /// # Errors
    /// [`StoreError`] when the notice cannot be written.
    fn mark_completed(&self, dir: &Path) -> Result<(), StoreError>;

    /// # Errors
    /// [`StoreError`] when `dir` exists but cannot be removed.
    fn remove_dir(&self, dir: &Path) -> Result<(), StoreError>;

    /// Every directory under `parent` kept past its change, in name order.
    ///
    /// # Errors
    /// [`StoreError`] when `parent` exists but cannot be listed.
    fn kept(&self, parent: &Path) -> Result<Vec<KeptRecovery>, StoreError>;

    /// Whether a person has dealt with `original`'s copy in `dir`: the
    /// original matches it again, or the copy has been deleted.
    fn copy_settled(&self, dir: &Path, original: &Path) -> bool;
}

/// Canonical-order JSONL append and anchored-prefix remove-by-key.
pub trait RecordStore {
    /// # Errors
    /// [`StoreError`] on validation failure, lock-acquisition timeout, or I/O.
    fn append_record(
        &self,
        path: &Path,
        record: &Record,
    ) -> Result<(), StoreError>;

    /// # Errors
    /// [`StoreError`] on lock-acquisition timeout or I/O.
    fn remove_by_key(&self, path: &Path, key: &str) -> Result<(), StoreError>;
}

#[cfg(test)]
mod tests {
    use super::StoreError;

    #[test]
    fn not_writable_names_the_path() {
        let error = StoreError::NotWritable {
            path: "/x/log".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "cannot write under '/x/log': not writable"
        );
    }

    #[test]
    fn lock_timeout_names_the_path() {
        let error = StoreError::LockTimeout {
            path: "/x/log".to_owned(),
        };
        assert_eq!(error.to_string(), "lock acquisition timed out on '/x/log'");
    }

    #[test]
    fn cross_filesystem_names_the_path() {
        let error = StoreError::CrossFilesystem {
            path: "/x/log".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "atomic rename to '/x/log' crossed a filesystem boundary"
        );
    }

    #[test]
    fn already_exists_names_the_path() {
        let error = StoreError::AlreadyExists {
            path: "/x/item.md".to_owned(),
        };
        assert_eq!(error.to_string(), "'/x/item.md' already exists");
    }

    #[test]
    fn validation_names_the_detail() {
        let error = StoreError::Validation {
            detail: "proposed_value is required".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "invalid record: proposed_value is required"
        );
    }

    #[test]
    fn io_names_the_path_and_detail() {
        let error = StoreError::Io {
            path: "/x/log".to_owned(),
            detail: "permission denied".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "I/O error on '/x/log': permission denied"
        );
    }

    #[test]
    fn maps_into_the_kernel_boundary_error() {
        let error = StoreError::LockTimeout {
            path: "/x/log".to_owned(),
        };
        let boundary: kernel::Error = error.into();
        assert_eq!(
            boundary.to_string(),
            "lock acquisition timed out on '/x/log'"
        );
    }
}
