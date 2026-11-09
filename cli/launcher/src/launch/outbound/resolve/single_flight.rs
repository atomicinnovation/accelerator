//! The path-level single-flight lock every prune and materialise takes, behind
//! a port so a test can fail it or see when it is released.

use std::path::Path;

use crate::launch::core::tree::TreeError;

/// Whatever guard an implementation returns, released when this drops.
pub struct LockGuard {
    _inner: Box<dyn Send>,
}

impl LockGuard {
    pub fn holding(inner: impl Send + 'static) -> Self {
        Self {
            _inner: Box::new(inner),
        }
    }
}

pub enum LockProbe {
    Taken(LockGuard),
    Held,
    Absent,
}

/// The verb waits for its locks; a cold path only tries them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockPatience {
    Wait,
    Try,
}

pub trait SingleFlightLocks {
    /// Take the lock, creating its file, and block until it is free.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be created or locked.
    fn take(&self, path: &Path) -> Result<LockGuard, TreeError>;

    /// Take the lock, creating its file, without blocking; `None` when another
    /// process holds it.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be opened, or locking
    /// fails for a reason other than contention.
    fn try_take(&self, path: &Path) -> Result<Option<LockGuard>, TreeError>;

    /// Take an existing lock, blocking until it is free; `None` when there is
    /// no lock file, and none is created.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be opened or locked, or
    /// keeps being replaced.
    fn wait_existing(
        &self,
        path: &Path,
    ) -> Result<Option<LockGuard>, TreeError>;

    /// Try an existing lock without blocking and without creating it.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file exists but cannot be locked.
    fn probe(&self, path: &Path) -> Result<LockProbe, TreeError>;
}
