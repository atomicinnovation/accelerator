//! The in-use lease and the single-flight lock, both `flock`.
//!
//! `flock` is the only cross-process liveness mechanism here — no pid gates,
//! which repeat a documented failure where a daemon bound to `$$` shut down
//! seconds after every bootstrap. The kernel is the liveness oracle: a crashed
//! holder releases with no cleanup code and leaves no stale state.

pub use lease::Liveness;

#[cfg(unix)]
pub use unix::{take_single_flight, try_single_flight, SingleFlight};

/// How a reclaimer learns whether a generation's lease is held — a test seam,
/// so `Unknown` can be exercised without a filesystem that lacks `flock`.
pub trait LivenessProbe {
    fn probe(&self, lease: &std::path::Path) -> Liveness;
}

/// The production probe: the kernel's own view of who holds the lease.
pub struct FlockProbe;

impl LivenessProbe for FlockProbe {
    fn probe(&self, lease: &std::path::Path) -> Liveness {
        lease::probe_shared_lease(lease)
    }
}

#[cfg(unix)]
mod unix {
    use std::fs::{File, OpenOptions};
    use std::os::fd::AsFd;
    use std::os::unix::fs::OpenOptionsExt as _;
    use std::path::Path;

    use lease::LeaseError;
    use rustix::fs::{flock, FlockOperation};

    use crate::launch::core::tree::{HeldLease, TreeError};

    impl HeldLease for lease::SharedLease {}

    impl From<LeaseError> for TreeError {
        fn from(error: LeaseError) -> Self {
            Self::Lease {
                detail: error.to_string(),
            }
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
        let file = open_lock_file(path)?;
        flock(file.as_fd(), FlockOperation::LockExclusive).map_err(
            |error| {
                lease(&format!("cannot take the single-flight lock: {error}"))
            },
        )?;
        Ok(SingleFlight { _file: file })
    }

    /// Try to take `LOCK_EX` without blocking.
    ///
    /// `Ok(None)` means another process holds it — the loser's cue to wait and
    /// re-check rather than to hang for the winner's whole download. This is the
    /// basis of the bounded waiter, which is what keeps a slow-but-healthy first
    /// fetch from stranding the rest of a crawl behind a lock.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be opened, or locking fails
    /// for a reason other than contention.
    pub fn try_single_flight(
        path: &Path,
    ) -> Result<Option<SingleFlight>, TreeError> {
        let file = open_lock_file(path)?;
        match flock(file.as_fd(), FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(Some(SingleFlight { _file: file })),
            Err(rustix::io::Errno::WOULDBLOCK) => Ok(None),
            Err(error) => Err(lease(&format!(
                "cannot take the single-flight lock: {error}"
            ))),
        }
    }

    fn open_lock_file(path: &Path) -> Result<File, TreeError> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|error| {
                lease(&format!("cannot open the single-flight lock: {error}"))
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
        use lease::{probe_shared_lease, Liveness};

        use super::take_single_flight;

        #[test]
        fn the_single_flight_lock_is_exclusive() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("lock");
            let held = take_single_flight(&path).expect("first holder");
            // A non-blocking exclusive probe against the same file blocks.
            assert_eq!(probe_shared_lease(&path), Liveness::Held);
            drop(held);
            let _second = take_single_flight(&path).expect("after release");
        }
    }
}

#[cfg(not(unix))]
mod stubs {
    use std::path::Path;

    use lease::LeaseError;

    use crate::launch::core::tree::{HeldLease, TreeError};

    impl HeldLease for lease::SharedLease {}

    impl From<LeaseError> for TreeError {
        fn from(error: LeaseError) -> Self {
            Self::Lease {
                detail: error.to_string(),
            }
        }
    }

    pub struct SingleFlight;

    pub fn take_single_flight(_path: &Path) -> Result<SingleFlight, TreeError> {
        unimplemented!("the single-flight lock is a Unix-only path")
    }

    pub fn try_single_flight(
        _path: &Path,
    ) -> Result<Option<SingleFlight>, TreeError> {
        unimplemented!("the single-flight lock is a Unix-only path")
    }
}

#[cfg(not(unix))]
pub use stubs::{take_single_flight, try_single_flight, SingleFlight};
