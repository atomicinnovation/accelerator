//! How a file is pinned against reclamation by a shared lease, and how a
//! reclaimer probes whether it is pinned.
//!
//! The lease is a `flock(LOCK_SH)` on a sidecar file. The kernel is the
//! liveness oracle: a crashed holder releases with no cleanup code and leaves
//! no stale state. A held lease is inherited across `exec`, so it pins for as
//! long as any detached descendant runs.
//!
//! Where `flock` is unavailable — NFS returning `ENOLCK`, some FUSE and SMB
//! backends — liveness is probed as unknown rather than as free, so a
//! spuriously successful probe cannot reclaim what a live process uses.

use std::path::{Path, PathBuf};

/// Children may repurpose low fixed descriptors, which would silently close a
/// lease sitting there.
pub const LOWEST_LEASE_DESCRIPTOR: i32 = 10;

#[cfg(unix)]
pub use unix::{hold_shared_lease, probe_shared_lease, SharedLease};

#[cfg(not(unix))]
pub use stubs::{hold_shared_lease, probe_shared_lease, SharedLease};

/// Whether a shared lease is held by any live process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// A live holder — `LOCK_EX | LOCK_NB` would block.
    Held,
    /// No holder — the exclusive probe succeeded.
    Free,
    /// `flock` does not work on this filesystem, so liveness cannot be
    /// determined; the caller must fall through to the age backstop rather
    /// than reclaiming.
    Unknown,
}

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
    use std::os::fd::{AsFd, OwnedFd};
    use std::os::unix::fs::OpenOptionsExt as _;
    use std::path::Path;

    use rustix::fs::{flock, FlockOperation};
    use rustix::io::{
        fcntl_dupfd_cloexec, fcntl_getfd, fcntl_setfd, Errno, FdFlags,
    };

    use super::{LeaseError, Liveness, LOWEST_LEASE_DESCRIPTOR};

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
        let raised = opened_above_floor(path)?;
        flock(raised.as_fd(), FlockOperation::LockShared).map_err(|error| {
            LeaseError::new("cannot take the lease", path, error)
        })?;
        let file = File::from(raised);
        clear_cloexec(&file, path)?;
        Ok(SharedLease { file })
    }

    /// Where `flock` is emulated with POSIX locks, closing any descriptor for
    /// the file drops the process's lock, so the low descriptor is closed
    /// before the lock is taken on the raised one.
    fn opened_above_floor(path: &Path) -> Result<OwnedFd, LeaseError> {
        let opened = open_lease_file(path)?;
        fcntl_dupfd_cloexec(opened.as_fd(), LOWEST_LEASE_DESCRIPTOR).map_err(
            |error| {
                LeaseError::new(
                    "cannot raise the lease descriptor of",
                    path,
                    error,
                )
            },
        )
    }

    /// Probe the lease with a non-blocking exclusive `flock`, released at
    /// once if it succeeds, so the probe never leaves the lease held. An absent
    /// lease file is `Free`: there is no holder.
    #[must_use]
    pub fn probe_shared_lease(path: &Path) -> Liveness {
        let file = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Liveness::Free
            }
            Err(_) => return Liveness::Unknown,
        };
        match flock(file.as_fd(), FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => {
                let _ = flock(file.as_fd(), FlockOperation::Unlock);
                Liveness::Free
            }
            Err(Errno::WOULDBLOCK) => Liveness::Held,
            Err(_) => Liveness::Unknown,
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
        use std::os::fd::{AsFd as _, AsRawFd as _};

        use rustix::io::{fcntl_getfd, FdFlags};

        use super::{
            hold_shared_lease, probe_shared_lease, Liveness,
            LOWEST_LEASE_DESCRIPTOR,
        };

        fn cloexec_set(file: &std::fs::File) -> bool {
            fcntl_getfd(file.as_fd())
                .expect("read fd flags")
                .contains(FdFlags::CLOEXEC)
        }

        #[test]
        fn a_held_lease_is_probed_held_and_a_released_one_free() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            let lease = hold_shared_lease(&path).expect("hold");
            assert_eq!(probe_shared_lease(&path), Liveness::Held);
            let moved = vec![lease];
            assert_eq!(probe_shared_lease(&path), Liveness::Held);
            drop(moved);
            assert_eq!(probe_shared_lease(&path), Liveness::Free);
        }

        #[test]
        fn a_shared_lease_admits_a_concurrent_holder() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            let first = hold_shared_lease(&path).expect("first");
            let second = hold_shared_lease(&path).expect("a second crawl");
            assert_eq!(probe_shared_lease(&path), Liveness::Held);
            drop(first);
            assert_eq!(probe_shared_lease(&path), Liveness::Held);
            drop(second);
            assert_eq!(probe_shared_lease(&path), Liveness::Free);
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
        fn a_lease_never_sits_on_a_low_descriptor() {
            let dir = tempfile::tempdir().expect("tempdir");
            let lease =
                hold_shared_lease(&dir.path().join("gen.lease")).expect("hold");
            assert!(lease.file().as_raw_fd() >= LOWEST_LEASE_DESCRIPTOR);
        }

        #[test]
        fn an_unopenable_lease_is_a_lease_error() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("absent").join("gen.lease");
            assert!(hold_shared_lease(&path).is_err());
        }

        #[test]
        fn an_absent_lease_file_is_probed_free() {
            let dir = tempfile::tempdir().expect("tempdir");
            assert_eq!(
                probe_shared_lease(&dir.path().join("absent.lease")),
                Liveness::Free
            );
        }

        #[test]
        fn probing_a_free_lease_leaves_it_free() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            drop(hold_shared_lease(&path).expect("hold"));
            assert_eq!(probe_shared_lease(&path), Liveness::Free);
            assert_eq!(probe_shared_lease(&path), Liveness::Free);
        }

        #[test]
        fn a_lease_that_cannot_be_opened_is_probed_unknown() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            std::fs::create_dir(&path).expect("a directory in its place");
            assert_eq!(probe_shared_lease(&path), Liveness::Unknown);
        }
    }
}

#[cfg(not(unix))]
mod stubs {
    use std::path::Path;

    use super::{LeaseError, Liveness};

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

    #[must_use]
    pub fn probe_shared_lease(_path: &Path) -> Liveness {
        Liveness::Unknown
    }
}
