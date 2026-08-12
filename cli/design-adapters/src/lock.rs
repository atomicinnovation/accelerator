//! The launcher lock: one backend, released at exit on every path.
//!
//! The shell carried two — `flock(1)` when the binary was present, a `mkdir`
//! sentinel otherwise — because that binary is absent on macOS. In Rust
//! `flock(2)` is available on every supported target, so the dichotomy and the
//! `ACCELERATOR_LOCK_FORCE_MKDIR` escape hatch both go, along with the
//! `owner.<nonce>` sentinel protocol neither backend needed here.
//!
//! Rust opens files `O_CLOEXEC` by default, so the descriptor does **not**
//! leak into the daemon the way the shell's did. That is deliberate: holding
//! the lock for the daemon's lifetime makes a stale-start-time recovery while
//! the daemon still lives report `another-launcher-running` falsely.

use std::cell::Cell;
use std::fs::File;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;

use design::executor::ports::Lock;
use design::executor::ports::LockOutcome;

/// An advisory whole-file lock, held for as long as the handle lives.
pub struct FileLock {
    handle: File,
    /// Released explicitly before the process image is replaced, where no
    /// destructor runs. Tracked so the drop guard does not double-release.
    released: Cell<bool>,
}

impl FileLock {
    /// Opens the lock file without taking the lock.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when the file cannot be created or opened.
    pub fn open(path: &Path) -> Result<Self, kernel::Error> {
        let handle = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|error| {
                kernel::Error::Failed(format!(
                    "could not open the launcher lock {}: {error}",
                    path.display()
                ))
            })?;
        Ok(Self {
            handle,
            released: Cell::new(false),
        })
    }
}

impl Lock for FileLock {
    fn acquire(&self) -> Result<LockOutcome, kernel::Error> {
        let taken = unsafe {
            libc::flock(self.handle.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB)
        };
        if taken == 0 {
            return Ok(LockOutcome::Acquired);
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(code) if code == libc::EWOULDBLOCK || code == libc::EAGAIN => {
                Ok(LockOutcome::HeldByAnother)
            }
            _ => Err(kernel::Error::Failed(format!(
                "could not lock the launcher lock: {error}"
            ))),
        }
    }

    fn release(&self) {
        if self.released.replace(true) {
            return;
        }
        unsafe { libc::flock(self.handle.as_raw_fd(), libc::LOCK_UN) };
    }
}

/// The backstop for every path that returns from `main` normally — usage
/// errors, refusals, envelopes. The client path releases explicitly instead,
/// because `exec` runs no destructors.
impl Drop for FileLock {
    fn drop(&mut self) {
        if !self.released.get() {
            unsafe { libc::flock(self.handle.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

#[cfg(test)]
mod tests {
    use design::executor::ports::Lock as _;
    use design::executor::ports::LockOutcome;

    use super::FileLock;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn an_uncontended_lock_is_acquired() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let lock = FileLock::open(&work.path().join("launcher.lock"))?;
        assert_eq!(lock.acquire()?, LockOutcome::Acquired);
        Ok(())
    }

    /// Two launchers, one daemon: the loser must be told rather than blocking.
    #[test]
    fn a_second_holder_is_told_the_lock_is_held() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let path = work.path().join("launcher.lock");
        let first = FileLock::open(&path)?;
        assert_eq!(first.acquire()?, LockOutcome::Acquired);

        // A separate handle is a separate open file description, which is what
        // flock arbitrates between.
        let second = FileLock::open(&path)?;
        assert_eq!(second.acquire()?, LockOutcome::HeldByAnother);
        Ok(())
    }

    #[test]
    fn releasing_lets_the_next_holder_in() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let path = work.path().join("launcher.lock");
        let first = FileLock::open(&path)?;
        assert_eq!(first.acquire()?, LockOutcome::Acquired);
        first.release();

        let second = FileLock::open(&path)?;
        assert_eq!(second.acquire()?, LockOutcome::Acquired);
        Ok(())
    }

    #[test]
    fn dropping_without_releasing_still_frees_the_lock() -> Result<(), TestError>
    {
        let work = tempfile::tempdir()?;
        let path = work.path().join("launcher.lock");
        {
            let held = FileLock::open(&path)?;
            assert_eq!(held.acquire()?, LockOutcome::Acquired);
        }
        let next = FileLock::open(&path)?;
        assert_eq!(next.acquire()?, LockOutcome::Acquired);
        Ok(())
    }

    #[test]
    fn releasing_twice_is_harmless() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let lock = FileLock::open(&work.path().join("launcher.lock"))?;
        assert_eq!(lock.acquire()?, LockOutcome::Acquired);
        lock.release();
        lock.release();
        Ok(())
    }
}
