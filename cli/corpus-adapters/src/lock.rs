use std::path::PathBuf;

use corpus::lock::{ExclusiveLock, HeldLock, LockName};
use corpus::StoreError;
use store::lock::{acquire, LockOptions};

use crate::store::from_lock_error;

/// The lockdir-backed [`ExclusiveLock`]: each [`LockName`] resolves to one
/// lockdir, acquired with the same backoff and dead-holder reclaim as
/// [`acquire`].
pub struct LockdirLock {
    work_dir: PathBuf,
    options: LockOptions,
}

impl LockdirLock {
    pub const RETIREMENT_LOCKDIR: &'static str =
        ".accelerator-work-retire.lockdir";

    #[must_use]
    pub fn new(work_dir: impl Into<PathBuf>) -> Self {
        Self::with_options(work_dir, LockOptions::default())
    }

    #[must_use]
    pub fn with_options(
        work_dir: impl Into<PathBuf>,
        options: LockOptions,
    ) -> Self {
        Self {
            work_dir: work_dir.into(),
            options,
        }
    }

    fn lockdir(&self, name: &LockName) -> PathBuf {
        match name {
            LockName::Retirement => {
                self.work_dir.join(Self::RETIREMENT_LOCKDIR)
            }
            LockName::ForFile(path) => crate::store::lockdir(path),
        }
    }
}

impl ExclusiveLock for LockdirLock {
    fn acquire(&self, name: &LockName) -> Result<HeldLock, StoreError> {
        acquire(&self.lockdir(name), self.options)
            .map(HeldLock::new)
            .map_err(from_lock_error)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use corpus::lock::{ExclusiveLock as _, LockName};
    use corpus::StoreError;
    use store::lock::LockOptions;
    use tempfile::TempDir;

    use super::LockdirLock;

    type TestError = Box<dyn std::error::Error>;

    const FAST: LockOptions = LockOptions {
        ceiling_ms: 1,
        base_ms: 1,
        cap_ms: 1,
    };

    #[test]
    fn a_held_exclusive_lock_times_out_as_lock_timeout() -> Result<(), TestError>
    {
        let dir = TempDir::new()?;
        let lock = LockdirLock::with_options(dir.path(), FAST);
        let held = lock.acquire(&LockName::Retirement)?;
        let second = lock.acquire(&LockName::Retirement);
        assert!(matches!(second, Err(StoreError::LockTimeout { .. })));
        drop(held);
        assert!(lock.acquire(&LockName::Retirement).is_ok());
        Ok(())
    }

    #[test]
    fn the_retirement_lock_lives_in_the_work_directory() -> Result<(), TestError>
    {
        let dir = TempDir::new()?;
        let lock = LockdirLock::new(dir.path());
        let _held = lock.acquire(&LockName::Retirement)?;
        assert!(dir.path().join(LockdirLock::RETIREMENT_LOCKDIR).is_dir());
        Ok(())
    }

    #[test]
    fn a_for_file_lock_name_resolves_to_the_store_s_own_lockdir(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let target = dir.path().join("0001-item.md");
        fs::write(&target, b"x")?;
        let lock = LockdirLock::with_options(dir.path(), FAST);
        let _held = lock.acquire(&LockName::ForFile(target.clone()))?;
        let store =
            crate::store::FileCorpusStore::with_lock_options(dir.path(), FAST);
        assert!(matches!(
            store.replace_locked(&target, b"blocked"),
            Err(StoreError::LockTimeout { .. })
        ));
        Ok(())
    }
}
