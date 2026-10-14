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
//! spuriously successful probe cannot reclaim what a live process uses. NFS
//! is probed as unknown even where `flock` succeeds, because its lock belongs
//! to the process that took it and is not inherited across `fork`.

use std::path::{Path, PathBuf};

/// Children may repurpose low fixed descriptors, which would silently close a
/// lease sitting there.
pub const LOWEST_LEASE_DESCRIPTOR: i32 = 10;

pub const LEASE_SUFFIX: &str = ".lease";

/// The lease that pins `pinned`: its sibling, named with [`LEASE_SUFFIX`].
#[must_use]
pub fn lease_beside(pinned: &Path) -> PathBuf {
    let mut lease = pinned.as_os_str().to_owned();
    lease.push(LEASE_SUFFIX);
    PathBuf::from(lease)
}

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

    /// Whose a `flock` is on a lease's filesystem. Only a lock owned by the
    /// open file passes to a forked daemon with its descriptor; one owned by
    /// the process is released when the process that took it exits.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FlockOwner {
        OpenFile,
        Process,
    }

    /// Probe the lease with a non-blocking exclusive `flock`, released at
    /// once if it succeeds, so the probe never leaves the lease held. An absent
    /// lease file is `Free`: there is no holder.
    #[must_use]
    pub fn probe_shared_lease(path: &Path) -> Liveness {
        match flock_owner_on(path) {
            Ok(owner) => probe_where_locks_belong_to(owner, path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Liveness::Free
            }
            Err(_) => Liveness::Unknown,
        }
    }

    /// A lease locked per process may have been taken by a client that has
    /// since exited, leaving the daemon that inherited its descriptor running
    /// unlocked, so its probe cannot tell a live holder from none.
    fn probe_where_locks_belong_to(owner: FlockOwner, path: &Path) -> Liveness {
        match owner {
            FlockOwner::OpenFile => probe_flock(path),
            FlockOwner::Process => Liveness::Unknown,
        }
    }

    fn flock_owner_on(path: &Path) -> std::io::Result<FlockOwner> {
        let filesystem = rustix::fs::statfs(path)?;
        #[cfg(target_os = "linux")]
        let owner = flock_owner_of_filesystem_type(filesystem.f_type);
        #[cfg(target_os = "macos")]
        let owner = flock_owner_of_filesystem_name(
            &filesystem.f_fstypename.map(|byte| byte.to_ne_bytes()[0]),
        );
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let owner = {
            let _ = filesystem;
            FlockOwner::OpenFile
        };
        Ok(owner)
    }

    /// Linux's NFS client emulates `flock` with `fcntl` record locks, which
    /// belong to the process, unless mounted with `local_lock=flock`.
    #[cfg(target_os = "linux")]
    const fn flock_owner_of_filesystem_type(
        filesystem_type: rustix::fs::FsWord,
    ) -> FlockOwner {
        if filesystem_type == rustix::fs::NFS_SUPER_MAGIC {
            FlockOwner::Process
        } else {
            FlockOwner::OpenFile
        }
    }

    #[cfg(target_os = "macos")]
    fn flock_owner_of_filesystem_name(name: &[u8]) -> FlockOwner {
        let name = name.split(|byte| *byte == 0).next().unwrap_or_default();
        if name == b"nfs" {
            FlockOwner::Process
        } else {
            FlockOwner::OpenFile
        }
    }

    fn probe_flock(path: &Path) -> Liveness {
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
            flock_owner_on, hold_shared_lease, probe_shared_lease,
            probe_where_locks_belong_to, FlockOwner, Liveness,
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

        #[test]
        fn a_local_lease_is_locked_per_open_file() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            drop(hold_shared_lease(&path).expect("hold"));
            assert_eq!(
                flock_owner_on(&path).expect("statfs"),
                FlockOwner::OpenFile
            );
        }

        #[test]
        fn a_held_lease_locked_per_process_is_probed_unknown() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            let _lease = hold_shared_lease(&path).expect("hold");
            assert_eq!(
                probe_where_locks_belong_to(FlockOwner::Process, &path),
                Liveness::Unknown
            );
        }

        #[test]
        fn a_free_lease_locked_per_process_is_probed_unknown() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("gen.lease");
            drop(hold_shared_lease(&path).expect("hold"));
            assert_eq!(
                probe_where_locks_belong_to(FlockOwner::Process, &path),
                Liveness::Unknown
            );
        }

        #[cfg(target_os = "linux")]
        #[test]
        fn nfs_locks_belong_to_the_process() {
            assert_eq!(
                super::flock_owner_of_filesystem_type(
                    rustix::fs::NFS_SUPER_MAGIC
                ),
                FlockOwner::Process
            );
        }

        #[cfg(target_os = "linux")]
        #[test]
        fn ext4_locks_belong_to_the_open_file() {
            const EXT4_SUPER_MAGIC: rustix::fs::FsWord = 0xEF53;
            assert_eq!(
                super::flock_owner_of_filesystem_type(EXT4_SUPER_MAGIC),
                FlockOwner::OpenFile
            );
        }

        #[cfg(target_os = "macos")]
        #[test]
        fn nfs_locks_belong_to_the_process() {
            assert_eq!(
                super::flock_owner_of_filesystem_name(b"nfs"),
                FlockOwner::Process
            );
        }

        #[cfg(target_os = "macos")]
        #[test]
        fn apfs_locks_belong_to_the_open_file() {
            assert_eq!(
                super::flock_owner_of_filesystem_name(b"apfs"),
                FlockOwner::OpenFile
            );
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{lease_beside, LEASE_SUFFIX};

    #[test]
    fn a_directory_is_pinned_by_its_suffixed_sibling() {
        assert_eq!(
            lease_beside(Path::new("/cache/trees/driver-1")),
            Path::new("/cache/trees/driver-1.lease")
        );
    }

    #[test]
    fn a_file_keeps_its_extension_beneath_the_suffix() {
        assert_eq!(
            lease_beside(Path::new("/cache/archive.tar")),
            Path::new("/cache/archive.tar.lease")
        );
    }

    #[test]
    fn the_lease_shares_the_parent_of_what_it_pins() {
        let pinned = Path::new("/cache/trees/driver-1");
        assert_eq!(lease_beside(pinned).parent(), pinned.parent());
    }

    #[test]
    fn the_lease_name_ends_with_the_suffix() {
        let lease = lease_beside(Path::new("driver-1"));
        assert!(lease.to_string_lossy().ends_with(LEASE_SUFFIX));
    }
}
