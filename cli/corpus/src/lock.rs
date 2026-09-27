//! Exclusive locks over the corpus, named in domain terms.
//!
//! Every holder acquires in one global order — retirement, then create, then
//! per-file — and no holder of a later lock ever takes an earlier one, so two
//! holders can never wait on each other.

use std::path::PathBuf;

use crate::store::StoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockName {
    Retirement,
    ForFile(PathBuf),
}

/// Releases its lock when dropped.
pub struct HeldLock {
    _guard: Box<dyn Send>,
}

impl HeldLock {
    #[must_use]
    pub fn new(guard: impl Send + 'static) -> Self {
        Self {
            _guard: Box::new(guard),
        }
    }
}

pub trait ExclusiveLock {
    /// Blocks until `name` is free.
    ///
    /// # Errors
    /// [`StoreError::LockTimeout`] when it stays held past the adapter's
    /// ceiling, or another [`StoreError`] when the lock cannot be taken.
    fn acquire(&self, name: &LockName) -> Result<HeldLock, StoreError>;
}
