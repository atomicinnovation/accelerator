//! The shared in-use lease that pins a vendored-runtime tree against
//! reclamation, and the one observation every reclaimer and test agrees on.
//!
//! The lease is a `flock(LOCK_SH)` on a sidecar file. The kernel is the
//! liveness oracle: a crashed holder releases with no cleanup code and leaves
//! no stale state.

use std::path::{Path, PathBuf};

#[cfg(unix)]
pub use unix::{hold_shared_lease, observed_held, SharedLease};

#[cfg(not(unix))]
pub use stubs::{hold_shared_lease, observed_held, SharedLease};

/// A lease that could not be opened, taken or observed.
#[derive(Debug)]
pub struct LeaseError {
    action: &'static str,
    path: PathBuf,
    cause: std::io::Error,
}

impl LeaseError {
    fn new(
        action: &'static str,
        path: &Path,
        cause: impl Into<std::io::Error>,
    ) -> Self {
        Self {
            action,
            path: path.to_path_buf(),
            cause: cause.into(),
        }
    }
}

impl std::fmt::Display for LeaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.action,
            self.path.display(),
            self.cause
        )
    }
}

impl std::error::Error for LeaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

#[cfg(unix)]
mod unix {
    use std::fs::{File, OpenOptions};
    use std::os::fd::AsFd;
    use std::os::unix::fs::OpenOptionsExt as _;
    use std::path::Path;

    use rustix::fs::{flock, FlockOperation};
    use rustix::io::{fcntl_getfd, fcntl_setfd, Errno, FdFlags};

    use super::LeaseError;

    /// A held shared lease. The tree it pins stays reclamation-proof for as
    /// long as this value — or any process that inherited the open file
    /// description across `exec` — is alive.
    pub struct SharedLease {
        // Holding the open file description is the lock; the field exists to be
        // dropped, releasing it, when the lease value dies.
        #[allow(dead_code)]
        file: File,
    }

    impl SharedLease {
        #[cfg(test)]
        pub(crate) const fn file(&self) -> &File {
            &self.file
        }
    }

    /// Open the lease sidecar and take `LOCK_SH`, clearing `FD_CLOEXEC` so the
    /// open file description survives `exec` into a detached daemon.
    ///
    /// # Errors
    ///
    /// [`LeaseError`] if the sidecar cannot be created or locked.
    pub fn hold_shared_lease(path: &Path) -> Result<SharedLease, LeaseError> {
        let file = open_lease_file(path)?;
        flock(file.as_fd(), FlockOperation::LockShared).map_err(|error| {
            LeaseError::new("cannot take the lease", path, error)
        })?;
        clear_cloexec(&file, path)?;
        Ok(SharedLease { file })
    }

    /// Whether any live process holds the lease, probed with a non-blocking
    /// exclusive `flock` that is released at once if it succeeds. An absent
    /// lease file is not held.
    ///
    /// # Errors
    ///
    /// [`LeaseError`] if the file cannot be opened, or the probe fails for a
    /// reason other than contention.
    pub fn observed_held(path: &Path) -> Result<bool, LeaseError> {
        let file = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(false)
            }
            Err(error) => {
                return Err(LeaseError::new(
                    "cannot open the lease",
                    path,
                    error,
                ))
            }
        };
        match flock(file.as_fd(), FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => {
                let _ = flock(file.as_fd(), FlockOperation::Unlock);
                Ok(false)
            }
            Err(Errno::WOULDBLOCK) => Ok(true),
            Err(error) => {
                Err(LeaseError::new("cannot probe the lease", path, error))
            }
        }
    }

    fn open_lease_file(path: &Path) -> Result<File, LeaseError> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|error| {
                LeaseError::new("cannot open the lease", path, error)
            })
    }

    fn clear_cloexec(file: &File, path: &Path) -> Result<(), LeaseError> {
        let flags = fcntl_getfd(file.as_fd()).map_err(|error| {
            LeaseError::new(
                "cannot read the lease descriptor flags of",
                path,
                error,
            )
        })?;
        fcntl_setfd(file.as_fd(), flags - FdFlags::CLOEXEC).map_err(|error| {
            LeaseError::new("cannot clear FD_CLOEXEC on the lease", path, error)
        })
    }

    #[cfg(test)]
    #[allow(clippy::expect_used)]
    mod tests {
        use std::os::fd::AsFd as _;

        use rustix::io::{fcntl_getfd, FdFlags};

        use super::{hold_shared_lease, observed_held};

        fn cloexec_set(file: &std::fs::File) -> bool {
            fcntl_getfd(file.as_fd())
                .expect("read fd flags")
                .contains(FdFlags::CLOEXEC)
        }

        fn held(path: &std::path::Path) -> bool {
            observed_held(path).expect("observe the lease")
        }

        #[test]
        fn a_held_lease_is_observed_held_and_a_released_one_free() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            let lease = hold_shared_lease(&path).expect("hold");
            assert!(held(&path));
            drop(lease);
            assert!(!held(&path));
        }

        #[test]
        fn a_shared_lease_admits_a_concurrent_holder() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            let first = hold_shared_lease(&path).expect("first");
            let second = hold_shared_lease(&path).expect("a second crawl");
            assert!(held(&path));
            drop(first);
            assert!(held(&path));
            drop(second);
            assert!(!held(&path));
        }

        #[test]
        fn the_lease_descriptor_survives_exec() {
            let dir = tempfile::tempdir().expect("tempdir");
            let lease =
                hold_shared_lease(&dir.path().join("gen.lease")).expect("hold");
            assert!(
                !cloexec_set(lease.file()),
                "FD_CLOEXEC must be cleared so the lease survives exec"
            );
        }

        #[test]
        fn an_unopenable_lease_is_a_lease_error() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("absent").join("gen.lease");
            assert!(hold_shared_lease(&path).is_err());
        }

        #[test]
        fn an_absent_lease_file_is_not_observed_held() {
            let dir = tempfile::tempdir().expect("tempdir");
            assert!(!held(&dir.path().join("absent.lease")));
        }

        #[test]
        fn observing_a_free_lease_leaves_it_free() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            drop(hold_shared_lease(&path).expect("hold"));
            assert!(!held(&path));
            assert!(!held(&path));
        }
    }
}

#[cfg(not(unix))]
mod stubs {
    use std::path::Path;

    use super::LeaseError;

    pub struct SharedLease;

    fn unsupported(path: &Path) -> LeaseError {
        LeaseError::new(
            "leases are unsupported on this platform for",
            path,
            std::io::Error::from(std::io::ErrorKind::Unsupported),
        )
    }

    /// # Errors
    ///
    /// Always a [`LeaseError`]: leases are a Unix-only path.
    pub fn hold_shared_lease(path: &Path) -> Result<SharedLease, LeaseError> {
        Err(unsupported(path))
    }

    /// # Errors
    ///
    /// Always a [`LeaseError`]: leases are a Unix-only path.
    pub fn observed_held(path: &Path) -> Result<bool, LeaseError> {
        Err(unsupported(path))
    }
}
