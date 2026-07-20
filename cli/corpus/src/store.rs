//! The atomic-store error taxonomy and the two driven ports the adapter
//! implements: `AtomicWrite` for whole-file atomic replacement and
//! `RecordStore` for canonical-order JSONL append/remove.

use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;

use crate::record::Record;

/// An atomic-store operation failure.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotWritable { path: String },
    LockTimeout { path: String },
    CrossFilesystem { path: String },
    UnsafePath { path: String },
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
