//! The in-use lease and the single-flight lock, both `flock`.
//!
//! `flock` is the only cross-process liveness mechanism here — no pid gates,
//! which repeat a documented failure where a daemon bound to `$$` shut down
//! seconds after every bootstrap. The kernel is the liveness oracle: a crashed
//! holder releases with no cleanup code and leaves no stale state.
//!
//! Where `flock` is unavailable — NFS returning `ENOLCK`, some FUSE and SMB
//! backends — liveness is reported as *unknown* rather than as free, so a
//! spuriously successful probe cannot reclaim a live daemon's tree.

#[cfg(unix)]
pub use unix::{
    hold_shared_lease, probe_liveness, take_single_flight, Liveness,
    SharedLease, SingleFlight,
};

#[cfg(unix)]
mod unix {
    use std::fs::{File, OpenOptions};
    use std::os::fd::AsFd;
    use std::os::unix::fs::OpenOptionsExt as _;
    use std::path::Path;

    use rustix::fs::{flock, FlockOperation};
    use rustix::io::{fcntl_getfd, fcntl_setfd, FdFlags};

    use crate::launch::core::tree::{HeldLease, TreeError};

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
        const fn file(&self) -> &File {
            &self.file
        }
    }

    impl HeldLease for SharedLease {}

    /// Open the lease sidecar and take `LOCK_SH`, clearing `FD_CLOEXEC` so the
    /// open file description survives the `exec` into the design binary and on
    /// into the detached daemon.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the sidecar cannot be created or locked.
    pub fn hold_shared_lease(path: &Path) -> Result<SharedLease, TreeError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|error| {
                lease(&format!("cannot open the lease: {error}"))
            })?;
        flock(file.as_fd(), FlockOperation::LockShared).map_err(|error| {
            lease(&format!("cannot take the lease: {error}"))
        })?;
        clear_cloexec(&file)?;
        Ok(SharedLease { file })
    }

    /// Whether a generation's lease is held by any live process.
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

    /// Probe a lease non-destructively with `LOCK_EX | LOCK_NB`.
    ///
    /// The lock is released immediately if acquired, so the probe never leaves
    /// a lease held. An absent sidecar is `Free`: there is no holder.
    #[must_use]
    pub fn probe_liveness(path: &Path) -> Liveness {
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
            Err(rustix::io::Errno::WOULDBLOCK) => Liveness::Held,
            // ENOLCK/EOPNOTSUPP and friends: the filesystem cannot answer, so a
            // reclaimer must treat this as "do not reclaim on this signal".
            Err(_) => Liveness::Unknown,
        }
    }

    /// A held single-flight lock. Materialisation and a whole-root prune run
    /// under it, so two reclaimers and a materialiser cannot disagree about the
    /// same directory.
    pub struct SingleFlight {
        _file: File,
    }

    /// Take `LOCK_EX` on the per-`(name, platform)` lock file, blocking until it
    /// is available.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be created or locked.
    pub fn take_single_flight(path: &Path) -> Result<SingleFlight, TreeError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|error| {
                lease(&format!("cannot open the single-flight lock: {error}"))
            })?;
        flock(file.as_fd(), FlockOperation::LockExclusive).map_err(
            |error| {
                lease(&format!("cannot take the single-flight lock: {error}"))
            },
        )?;
        Ok(SingleFlight { _file: file })
    }

    fn clear_cloexec(file: &File) -> Result<(), TreeError> {
        let flags = fcntl_getfd(file.as_fd()).map_err(|error| {
            lease(&format!("cannot read the lease descriptor flags: {error}"))
        })?;
        fcntl_setfd(file.as_fd(), flags - FdFlags::CLOEXEC).map_err(|error| {
            lease(&format!("cannot clear FD_CLOEXEC on the lease: {error}"))
        })
    }

    fn lease(detail: &str) -> TreeError {
        TreeError::Lease {
            detail: detail.to_owned(),
        }
    }

    #[cfg(test)]
    #[allow(clippy::expect_used)]
    mod tests {
        use std::os::fd::AsFd as _;

        use rustix::io::{fcntl_getfd, FdFlags};

        use super::{
            hold_shared_lease, probe_liveness, take_single_flight, Liveness,
        };

        fn cloexec_set(file: &std::fs::File) -> bool {
            fcntl_getfd(file.as_fd())
                .expect("read fd flags")
                .contains(FdFlags::CLOEXEC)
        }

        #[test]
        fn an_absent_lease_reads_as_free() {
            let dir = tempfile::tempdir().expect("tempdir");
            assert_eq!(
                probe_liveness(&dir.path().join("absent.lease")),
                Liveness::Free
            );
        }

        #[test]
        fn a_held_lease_reads_as_held_and_a_released_one_as_free() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            let lease = hold_shared_lease(&path).expect("hold");
            assert_eq!(probe_liveness(&path), Liveness::Held);
            drop(lease);
            assert_eq!(probe_liveness(&path), Liveness::Free);
        }

        #[test]
        fn a_shared_lease_admits_a_concurrent_holder() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            let first = hold_shared_lease(&path).expect("first");
            let second = hold_shared_lease(&path).expect("a second crawl");
            // Both held: an exclusive probe still blocks.
            assert_eq!(probe_liveness(&path), Liveness::Held);
            drop(first);
            assert_eq!(probe_liveness(&path), Liveness::Held);
            drop(second);
            assert_eq!(probe_liveness(&path), Liveness::Free);
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
        fn the_single_flight_lock_is_exclusive() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("lock");
            let held = take_single_flight(&path).expect("first holder");
            // A non-blocking exclusive probe against the same file blocks.
            assert_eq!(probe_liveness(&path), Liveness::Held);
            drop(held);
            let _second = take_single_flight(&path).expect("after release");
        }
    }
}

#[cfg(not(unix))]
mod stubs {
    use std::path::Path;

    use crate::launch::core::tree::{HeldLease, TreeError};

    pub struct SharedLease;
    impl HeldLease for SharedLease {}
    pub struct SingleFlight;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Liveness {
        Held,
        Free,
        Unknown,
    }

    pub fn hold_shared_lease(_path: &Path) -> Result<SharedLease, TreeError> {
        unimplemented!("leases are a Unix-only path")
    }

    #[must_use]
    pub fn probe_liveness(_path: &Path) -> Liveness {
        Liveness::Unknown
    }

    pub fn take_single_flight(_path: &Path) -> Result<SingleFlight, TreeError> {
        unimplemented!("the single-flight lock is a Unix-only path")
    }
}

#[cfg(not(unix))]
pub use stubs::{
    hold_shared_lease, probe_liveness, take_single_flight, Liveness,
    SharedLease, SingleFlight,
};
