//! Serialises concurrent `start` invocations with an exclusive `flock(2)`.
//!
//! `flock(2)` is available at the syscall level on both linux and macOS, so it
//! supersedes the old shell's `flock`-command-with-`mkdir`-fallback pair. Its
//! exclusion guarantee holds for a local filesystem; the visualiser state dir is
//! assumed local (the local-dev deployment target), which a network-mounted
//! `tmp` would silently weaken.

use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use nix::errno::Errno;
use nix::fcntl::{Flock, FlockArg};

/// Holds an exclusive lock for its lifetime; releases it on drop.
pub struct LaunchLock {
    _flock: Flock<File>,
}

impl LaunchLock {
    /// Try to acquire the lock without blocking. `Ok(None)` means another
    /// launcher already holds it.
    pub fn try_acquire(path: &Path) -> std::io::Result<Option<Self>> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o600)
            .open(path)?;
        match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(flock) => Ok(Some(Self { _flock: flock })),
            Err((_, Errno::EWOULDBLOCK | Errno::EACCES)) => Ok(None),
            Err((_, errno)) => Err(errno.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_is_refused_while_first_is_held() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("launcher.lock");
        let first = LaunchLock::try_acquire(&path).unwrap();
        assert!(first.is_some(), "first acquire must succeed");
        // Same process, different fd: flock is per-open-file-description, so a
        // fresh open contends with the held lock.
        let second = LaunchLock::try_acquire(&path).unwrap();
        assert!(second.is_none(), "second acquire must be refused");
        drop(first);
        let third = LaunchLock::try_acquire(&path).unwrap();
        assert!(third.is_some(), "acquire after release must succeed");
    }
}
