//! The in-use lease and the single-flight lock, both `flock`.
//!
//! `flock` is the only cross-process liveness mechanism here — no pid gates,
//! which repeat a documented failure where a daemon bound to `$$` shut down
//! seconds after every bootstrap. The kernel is the liveness oracle: a crashed
//! holder releases with no cleanup code and leaves no stale state.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub use lease::Liveness;

#[cfg(unix)]
pub use unix::{
    probe_single_flight, take_single_flight, try_exclusive_lease,
    try_single_flight, wait_existing, ExclusiveLease, SingleFlight,
};

pub enum SingleFlightProbe {
    Taken(SingleFlight),
    Held,
    Absent,
}

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

/// The generation directories this process holds shared leases on.
///
/// Each is held without a probe: where `flock` is emulated with
/// process-owned POSIX locks, probing a lease the process holds would
/// release it.
#[derive(Debug, Default)]
pub struct HeldLeases {
    generations: BTreeSet<PathBuf>,
}

impl HeldLeases {
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    pub fn holding(generations: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            generations: generations.into_iter().collect(),
        }
    }

    /// `probe`, asked only about leases this process does not hold.
    #[must_use]
    pub const fn before<'a>(
        &'a self,
        probe: &'a dyn LivenessProbe,
    ) -> HeldLeasesFirst<'a> {
        HeldLeasesFirst { held: self, probe }
    }

    fn holds_lease(&self, lease: &Path) -> bool {
        self.generations
            .iter()
            .any(|generation| lease::lease_beside(generation) == lease)
    }
}

pub struct HeldLeasesFirst<'a> {
    held: &'a HeldLeases,
    probe: &'a dyn LivenessProbe,
}

impl LivenessProbe for HeldLeasesFirst<'_> {
    fn probe(&self, lease: &Path) -> Liveness {
        if self.held.holds_lease(lease) {
            Liveness::Held
        } else {
            self.probe.probe(lease)
        }
    }
}

#[cfg(unix)]
mod unix {
    use std::fs::{File, OpenOptions};
    use std::os::fd::{AsFd, BorrowedFd};
    use std::os::unix::fs::OpenOptionsExt as _;
    use std::path::Path;

    use lease::LeaseError;
    use rustix::fs::{flock, FlockOperation};

    use crate::launch::core::tree::{HeldLease, TreeError};

    use super::SingleFlightProbe;

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
        /// Closed only with `_file`: where `flock` is emulated with POSIX
        /// locks, closing any descriptor for the file drops the lock.
        _confirming: File,
    }

    impl SingleFlight {
        const fn confirmed(held: File, confirming: File) -> Self {
            Self {
                _file: held,
                _confirming: confirming,
            }
        }
    }

    /// Take `LOCK_EX` on the per-`(name, platform)` lock file, blocking until it
    /// is available.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be created or locked.
    pub fn take_single_flight(path: &Path) -> Result<SingleFlight, TreeError> {
        lock_confirmed(path, open_creating, |fd| {
            flock(fd, FlockOperation::LockExclusive)
        })
        .and_then(take_outcome)
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
        lock_confirmed(path, open_creating, |fd| {
            flock(fd, FlockOperation::NonBlockingLockExclusive)
        })
        .and_then(try_outcome)
    }

    /// Try `LOCK_EX` without blocking and without creating the lock file, so
    /// a probe of a retired artifact's lock leaves nothing behind.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file exists but cannot be opened for
    /// writing, its identity cannot be read, or locking fails for a reason
    /// other than contention.
    pub fn probe_single_flight(
        path: &Path,
    ) -> Result<SingleFlightProbe, TreeError> {
        lock_confirmed(path, open_existing, |fd| {
            flock(fd, FlockOperation::NonBlockingLockExclusive)
        })
        .map(probe_outcome)
    }

    /// Take `LOCK_EX` on an existing lock file, blocking until it is
    /// available, and never create one: `Ok(None)` when there is none.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file exists but cannot be opened or
    /// locked, or keeps being replaced.
    pub fn wait_existing(
        path: &Path,
    ) -> Result<Option<SingleFlight>, TreeError> {
        lock_confirmed(path, open_existing, |fd| {
            flock(fd, FlockOperation::LockExclusive)
        })
        .and_then(wait_existing_outcome)
    }

    /// An exclusive lock on a generation's lease, held across its removal so
    /// no dispatch can lease the generation meanwhile.
    pub struct ExclusiveLease {
        _file: File,
    }

    /// Lock the lease exclusively without blocking, creating its file if
    /// missing; `Ok(None)` when a live process holds it shared.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lease cannot be opened or `flock` fails
    /// for a reason other than contention, as where it is unsupported.
    pub fn try_exclusive_lease(
        path: &Path,
    ) -> Result<Option<ExclusiveLease>, TreeError> {
        let file = open_creating(path)
            .map_err(|error| {
                lease(&format!(
                    "cannot open the lease {}: {error}",
                    path.display()
                ))
            })?
            .ok_or_else(no_lock_file)?;
        match flock(file.as_fd(), FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(Some(ExclusiveLease { _file: file })),
            Err(rustix::io::Errno::WOULDBLOCK) => Ok(None),
            Err(error) => Err(lease(&format!(
                "cannot lock the lease {}: {error}",
                path.display()
            ))),
        }
    }

    const MAX_LOCK_ATTEMPTS: usize = 8;

    enum Placement {
        InPlace(File),
        Displaced,
    }

    /// Reopens the path rather than comparing with a path `stat`, which can
    /// disagree with `fstat` on some overlayfs and FUSE mounts.
    fn placement(held: &File, path: &Path) -> Result<Placement, TreeError> {
        let identity_error = |error: std::io::Error| {
            lease(&format!(
                "cannot read the identity of {}: {error}",
                path.display()
            ))
        };
        let confirming = match File::open(path) {
            Ok(confirming) => confirming,
            Err(error) if is_displacement(&error) => {
                return Ok(Placement::Displaced)
            }
            Err(error) => return Err(identity_error(error)),
        };
        let held_identity = match identity(held) {
            Ok(held_identity) => held_identity,
            Err(error) if is_displacement(&error) => {
                return Ok(Placement::Displaced)
            }
            Err(error) => return Err(identity_error(error)),
        };
        Ok(
            if held_identity == identity(&confirming).map_err(identity_error)? {
                Placement::InPlace(confirming)
            } else {
                Placement::Displaced
            },
        )
    }

    fn is_displacement(error: &std::io::Error) -> bool {
        error.kind() == std::io::ErrorKind::NotFound || is_stale_handle(error)
    }

    /// On a root shared by several NFS clients, another client's unlink shows
    /// up as `ESTALE`, on a descriptor that is still open or on a path whose
    /// lookup was cached.
    fn is_stale_handle(error: &std::io::Error) -> bool {
        rustix::io::Errno::from_io_error(error)
            == Some(rustix::io::Errno::STALE)
    }

    fn identity(file: &File) -> std::io::Result<(u64, u64)> {
        use std::os::unix::fs::MetadataExt as _;
        file.metadata()
            .map(|metadata| (metadata.dev(), metadata.ino()))
    }

    enum Locked {
        Taken(SingleFlight),
        Held,
        Absent,
        KeptBeingReplaced,
    }

    fn lock_confirmed(
        path: &Path,
        open: impl Fn(&Path) -> std::io::Result<Option<File>>,
        lock: impl Fn(BorrowedFd<'_>) -> rustix::io::Result<()>,
    ) -> Result<Locked, TreeError> {
        for _ in 0..MAX_LOCK_ATTEMPTS {
            let file = match open(path) {
                Ok(Some(file)) => file,
                Ok(None) => return Ok(Locked::Absent),
                Err(error) if is_stale_handle(&error) => continue,
                Err(error) => {
                    return Err(lease(&format!(
                        "cannot open the single-flight lock: {error}"
                    )))
                }
            };
            match lock(file.as_fd()) {
                Ok(()) => match placement(&file, path)? {
                    Placement::InPlace(confirming) => {
                        return Ok(Locked::Taken(SingleFlight::confirmed(
                            file, confirming,
                        )))
                    }
                    Placement::Displaced => {}
                },
                Err(rustix::io::Errno::WOULDBLOCK) => return Ok(Locked::Held),
                Err(rustix::io::Errno::STALE) => {}
                Err(error) => {
                    return Err(lease(&format!(
                        "cannot take the single-flight lock: {error}"
                    )))
                }
            }
        }
        tracing::warn!(
            lock = %path.display(),
            "the single-flight lock file keeps being replaced"
        );
        Ok(Locked::KeptBeingReplaced)
    }

    fn take_outcome(locked: Locked) -> Result<SingleFlight, TreeError> {
        match locked {
            Locked::Taken(guard) => Ok(guard),
            Locked::Held => {
                Err(lease("a blocking single-flight lock reported contention"))
            }
            Locked::Absent => Err(no_lock_file()),
            Locked::KeptBeingReplaced => Err(kept_being_replaced()),
        }
    }

    /// A lock file that keeps being replaced is contention, not a fault, so
    /// the bounded waiter stays on its non-sticky path.
    fn try_outcome(locked: Locked) -> Result<Option<SingleFlight>, TreeError> {
        match locked {
            Locked::Taken(guard) => Ok(Some(guard)),
            Locked::Held | Locked::KeptBeingReplaced => Ok(None),
            Locked::Absent => Err(no_lock_file()),
        }
    }

    /// A churning lock file is a fault here, not contention: a caller that
    /// was willing to wait has no lock-free arm to fall back to.
    fn wait_existing_outcome(
        locked: Locked,
    ) -> Result<Option<SingleFlight>, TreeError> {
        match locked {
            Locked::Taken(guard) => Ok(Some(guard)),
            Locked::Absent => Ok(None),
            Locked::Held => {
                Err(lease("a blocking single-flight lock reported contention"))
            }
            Locked::KeptBeingReplaced => Err(kept_being_replaced()),
        }
    }

    fn no_lock_file() -> TreeError {
        lease("the single-flight lock file could not be created")
    }

    fn kept_being_replaced() -> TreeError {
        lease("the single-flight lock file keeps being replaced")
    }

    /// Mapping a lock file that keeps being replaced to `Held` keeps the
    /// reap away from residue whose owner it could not pin down.
    fn probe_outcome(locked: Locked) -> SingleFlightProbe {
        match locked {
            Locked::Taken(guard) => SingleFlightProbe::Taken(guard),
            Locked::Held | Locked::KeptBeingReplaced => SingleFlightProbe::Held,
            Locked::Absent => SingleFlightProbe::Absent,
        }
    }

    fn open_existing(path: &Path) -> std::io::Result<Option<File>> {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(not_following_links())
            .open(path)
        {
            Ok(file) => Ok(Some(file)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn open_creating(path: &Path) -> std::io::Result<Option<File>> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(not_following_links())
            .open(path)
            .map(Some)
    }

    const fn not_following_links() -> i32 {
        rustix::fs::OFlags::NOFOLLOW.bits().cast_signed()
    }

    fn lease(detail: &str) -> TreeError {
        TreeError::Lease {
            detail: detail.to_owned(),
        }
    }

    #[cfg(test)]
    #[allow(clippy::expect_used, clippy::panic)]
    mod tests {
        use std::fs::File;
        use std::os::fd::AsFd;
        use std::path::{Path, PathBuf};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        use lease::{probe_shared_lease, Liveness};
        use rustix::fs::{flock, FlockOperation};
        use rustix::io::Errno;

        use crate::launch::core::tree::TreeError;

        use super::super::super::testing::{logged, within, BLOCKING_BOUND};
        use super::super::SingleFlightProbe;
        use super::{
            identity, is_displacement, lock_confirmed, open_creating,
            placement, probe_outcome, probe_single_flight, take_outcome,
            take_single_flight, try_outcome, wait_existing,
            wait_existing_outcome, Locked, Placement, MAX_LOCK_ATTEMPTS,
        };

        #[test]
        fn the_single_flight_lock_is_exclusive() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("lock");
            let held = take_single_flight(&path).expect("first holder");
            assert_eq!(probe_shared_lease(&path), Liveness::Held);
            drop(held);
            let _second = take_single_flight(&path).expect("after release");
        }

        fn created(path: &Path) -> File {
            open_creating(path).expect("open").expect("created")
        }

        fn replace(path: &Path) {
            let fresh = path.with_extension("fresh");
            drop(created(&fresh));
            std::fs::rename(&fresh, path).expect("replace");
        }

        fn is_locked_elsewhere(path: &Path) -> bool {
            let file = File::options()
                .read(true)
                .write(true)
                .open(path)
                .expect("open from a new description");
            match flock(file.as_fd(), FlockOperation::NonBlockingLockExclusive)
            {
                Ok(()) => false,
                Err(Errno::WOULDBLOCK) => true,
                Err(error) => panic!("probe failed: {error}"),
            }
        }

        fn counted(
            opens: &Arc<AtomicUsize>,
            open: impl Fn(&Path, usize) -> std::io::Result<Option<File>>,
        ) -> impl Fn(&Path) -> std::io::Result<Option<File>> {
            let opens = Arc::clone(opens);
            move |path| open(path, opens.fetch_add(1, Ordering::SeqCst) + 1)
        }

        fn lock_path() -> (tempfile::TempDir, PathBuf) {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("lock");
            (dir, path)
        }

        #[test]
        fn a_locked_file_still_at_its_path_is_in_place() {
            let (_dir, path) = lock_path();
            let held = created(&path);
            assert!(matches!(
                placement(&held, &path),
                Ok(Placement::InPlace(_))
            ));
        }

        #[test]
        fn a_locked_file_unlinked_from_its_path_is_displaced() {
            let (_dir, path) = lock_path();
            let held = created(&path);
            std::fs::remove_file(&path).expect("unlink");
            assert!(matches!(
                placement(&held, &path),
                Ok(Placement::Displaced)
            ));
        }

        #[test]
        fn a_locked_file_replaced_at_its_path_is_displaced() {
            let (_dir, path) = lock_path();
            let held = created(&path);
            replace(&path);
            assert!(matches!(
                placement(&held, &path),
                Ok(Placement::Displaced)
            ));
        }

        #[test]
        fn a_lock_whose_file_is_replaced_once_locks_the_replacement() {
            let (_dir, path) = lock_path();
            let opens = Arc::new(AtomicUsize::new(0));
            let opener = counted(&opens, |path, nth| {
                let file = created(path);
                if nth == 1 {
                    replace(path);
                }
                Ok(Some(file))
            });

            let locked = lock_confirmed(&path, opener, |fd| {
                flock(fd, FlockOperation::NonBlockingLockExclusive)
            })
            .expect("locked");

            assert!(matches!(locked, Locked::Taken(_)));
            assert_eq!(opens.load(Ordering::SeqCst), 2);
            assert!(is_locked_elsewhere(&path), "the replacement is unlocked");
        }

        #[test]
        fn a_stale_handle_on_open_is_retried_as_a_displacement() {
            let (_dir, path) = lock_path();
            let opens = Arc::new(AtomicUsize::new(0));
            let opener = counted(&opens, |path, nth| {
                if nth == 1 {
                    return Err(os_error(Errno::STALE));
                }
                Ok(Some(created(path)))
            });

            let locked = lock_confirmed(&path, opener, |fd| {
                flock(fd, FlockOperation::NonBlockingLockExclusive)
            })
            .expect("locked");

            assert!(matches!(locked, Locked::Taken(_)));
            assert_eq!(opens.load(Ordering::SeqCst), 2);
        }

        #[test]
        fn a_stale_handle_on_lock_is_retried_as_a_displacement() {
            let (_dir, path) = lock_path();
            let opens = Arc::new(AtomicUsize::new(0));
            let opener = counted(&opens, |path, _| Ok(Some(created(path))));
            let locks = AtomicUsize::new(0);

            let locked = lock_confirmed(&path, opener, |fd| {
                if locks.fetch_add(1, Ordering::SeqCst) == 0 {
                    return Err(Errno::STALE);
                }
                flock(fd, FlockOperation::NonBlockingLockExclusive)
            })
            .expect("locked");

            assert!(matches!(locked, Locked::Taken(_)));
            assert_eq!(opens.load(Ordering::SeqCst), 2);
        }

        #[test]
        fn a_lock_whose_path_is_unlinked_reports_it_absent() {
            let (_dir, path) = lock_path();
            let opens = Arc::new(AtomicUsize::new(0));
            let opener = counted(&opens, |path, nth| {
                if nth > 1 {
                    return Ok(None);
                }
                let file = created(path);
                std::fs::remove_file(path).expect("unlink");
                Ok(Some(file))
            });

            let locked = lock_confirmed(&path, opener, |fd| {
                flock(fd, FlockOperation::NonBlockingLockExclusive)
            })
            .expect("probed");

            assert!(matches!(locked, Locked::Absent));
            assert_eq!(opens.load(Ordering::SeqCst), 2);
        }

        #[test]
        fn a_lock_that_keeps_being_replaced_says_so() {
            let (_dir, path) = lock_path();
            let opens = Arc::new(AtomicUsize::new(0));
            let opener = counted(&opens, |path, _| {
                let file = created(path);
                replace(path);
                Ok(Some(file))
            });

            let mut gave_up = false;
            let log = logged(|| {
                let path = path.clone();
                gave_up = within(BLOCKING_BOUND, "the lock", move || {
                    matches!(
                        lock_confirmed(&path, opener, |fd| flock(
                            fd,
                            FlockOperation::NonBlockingLockExclusive
                        ),),
                        Ok(Locked::KeptBeingReplaced)
                    )
                });
            });

            assert!(gave_up, "the loop did not give up");
            assert_eq!(opens.load(Ordering::SeqCst), MAX_LOCK_ATTEMPTS);
            assert!(log.contains("WARN"), "no warning: {log}");
            assert!(
                log.contains(&path.display().to_string()),
                "the warning names no lock: {log}"
            );
        }

        #[test]
        #[allow(clippy::used_underscore_binding)]
        fn a_confirmed_lock_keeps_its_confirming_descriptor() {
            let (_dir, path) = lock_path();
            let Ok(Locked::Taken(guard)) =
                lock_confirmed(&path, open_creating, |fd| {
                    flock(fd, FlockOperation::NonBlockingLockExclusive)
                })
            else {
                panic!("the lock was not taken");
            };

            assert_eq!(
                identity(&guard._file).expect("held identity"),
                identity(&guard._confirming).expect("confirming identity")
            );
        }

        #[test]
        fn an_identity_error_is_a_lease_error() {
            let dir = tempfile::tempdir().expect("tempdir");
            let not_a_directory = dir.path().join("file");
            drop(created(&not_a_directory));
            let path = not_a_directory.join("lock");
            let opens = Arc::new(AtomicUsize::new(0));
            let opener = counted(&opens, |_, _| tempfile::tempfile().map(Some));

            let outcome = lock_confirmed(&path, opener, |fd| {
                flock(fd, FlockOperation::NonBlockingLockExclusive)
            });

            assert_eq!(opens.load(Ordering::SeqCst), 1);
            assert!(
                matches!(
                    &outcome,
                    Err(TreeError::Lease { detail })
                        if detail.contains(&path.display().to_string())
                ),
                "not a lease error naming the lock"
            );
        }

        #[test]
        fn taking_a_lock_whose_file_is_replaced_while_waiting_locks_the_replacement(
        ) {
            let (_dir, path) = lock_path();
            let a = take_single_flight(&path).expect("A");
            let (waiting, waits) = std::sync::mpsc::channel();
            let taker = {
                let path = path.clone();
                std::thread::spawn(move || {
                    within(BLOCKING_BOUND, "the taker", move || {
                        lock_confirmed(&path, open_creating, |fd| {
                            let _ = waiting.send(());
                            flock(fd, FlockOperation::LockExclusive)
                        })
                    })
                })
            };
            waits.recv().expect("the taker reached the lock");

            replace(&path);
            drop(a);
            let locked = taker.join().expect("taker").expect("taken");

            assert!(matches!(locked, Locked::Taken(_)));
            assert!(is_locked_elsewhere(&path), "B is unlocked");
        }

        fn is_lease(outcome: &Result<impl Sized, TreeError>) -> bool {
            matches!(outcome, Err(TreeError::Lease { .. }))
        }

        #[test]
        fn take_treats_contention_on_a_blocking_lock_as_a_fault() {
            assert!(is_lease(&take_outcome(Locked::Held)));
        }

        #[test]
        fn take_treats_an_absent_lock_file_as_a_fault() {
            assert!(is_lease(&take_outcome(Locked::Absent)));
        }

        #[test]
        fn take_treats_a_lock_that_keeps_being_replaced_as_a_fault() {
            assert!(is_lease(&take_outcome(Locked::KeptBeingReplaced)));
        }

        #[test]
        fn try_treats_an_absent_lock_file_as_a_fault() {
            assert!(is_lease(&try_outcome(Locked::Absent)));
        }

        #[test]
        fn try_treats_a_lock_that_keeps_being_replaced_as_contention() {
            assert!(matches!(try_outcome(Locked::KeptBeingReplaced), Ok(None)));
        }

        #[test]
        fn probe_treats_a_lock_that_keeps_being_replaced_as_held() {
            assert!(matches!(
                probe_outcome(Locked::KeptBeingReplaced),
                SingleFlightProbe::Held
            ));
        }

        #[test]
        fn wait_existing_maps_a_churning_lock_to_an_error() {
            assert!(is_lease(&wait_existing_outcome(
                Locked::KeptBeingReplaced
            )));
        }

        #[test]
        fn wait_existing_treats_contention_on_a_blocking_lock_as_a_fault() {
            assert!(is_lease(&wait_existing_outcome(Locked::Held)));
        }

        #[test]
        fn waiting_on_an_absent_lock_finds_none_and_creates_nothing() {
            let (_dir, path) = lock_path();
            let waited = {
                let path = path.clone();
                within(BLOCKING_BOUND, "the wait", move || {
                    wait_existing(&path).map(|guard| guard.is_some())
                })
            };
            assert!(matches!(waited, Ok(false)));
            assert!(!path.exists(), "the wait created the lock file");
        }

        #[test]
        fn waiting_on_a_held_lock_takes_it_once_released() {
            let (_dir, path) = lock_path();
            let held = take_single_flight(&path).expect("held");
            let (waiting, waits) = std::sync::mpsc::channel();
            let waiter = std::thread::spawn(move || {
                let _ = waiting.send(());
                within(BLOCKING_BOUND, "the wait", move || {
                    wait_existing(&path).map(|guard| guard.is_some())
                })
            });
            waits.recv().expect("the waiter started");

            drop(held);

            assert!(matches!(waiter.join().expect("waiter"), Ok(true)));
        }

        #[test]
        fn a_symlinked_lock_file_is_refused_and_its_target_untouched() {
            let (dir, path) = lock_path();
            let target = dir.path().join("target");
            std::fs::write(&target, b"sentinel").expect("target");
            std::os::unix::fs::symlink(&target, &path).expect("symlink");

            assert!(is_lease(&take_single_flight(&path)));
            assert!(is_lease(&probed(&path)));
            assert!(is_lease(&wait_existing(&path)));
            assert!(!is_locked_elsewhere(&target), "the target was locked");
            assert_eq!(std::fs::read(&target).expect("target"), b"sentinel");
        }

        fn probed(path: &Path) -> Result<SingleFlightProbe, TreeError> {
            let path = path.to_owned();
            within(BLOCKING_BOUND, "the probe", move || {
                probe_single_flight(&path)
            })
        }

        #[test]
        fn probing_an_absent_lock_finds_it_absent_and_creates_nothing() {
            let (_dir, path) = lock_path();
            assert!(matches!(probed(&path), Ok(SingleFlightProbe::Absent)));
            assert!(!path.exists(), "the probe created the lock file");
        }

        #[test]
        fn probing_a_held_lock_finds_it_held() {
            let (_dir, path) = lock_path();
            let _held = take_single_flight(&path).expect("held");
            assert!(matches!(probed(&path), Ok(SingleFlightProbe::Held)));
        }

        #[test]
        fn probing_a_free_lock_takes_it_until_the_guard_drops() {
            let (_dir, path) = lock_path();
            drop(take_single_flight(&path).expect("created"));

            let Ok(SingleFlightProbe::Taken(guard)) = probed(&path) else {
                panic!("the free lock was not taken");
            };
            assert!(is_locked_elsewhere(&path), "the probe holds nothing");
            drop(guard);
            assert!(!is_locked_elsewhere(&path), "the guard did not release");
        }

        #[test]
        fn probing_a_lock_that_cannot_be_opened_is_an_error() {
            let (_dir, path) = lock_path();
            std::fs::create_dir(&path).expect("a directory at the lock");
            assert!(matches!(probed(&path), Err(TreeError::Lease { .. })));
        }

        fn os_error(errno: Errno) -> std::io::Error {
            std::io::Error::from_raw_os_error(errno.raw_os_error())
        }

        #[test]
        fn a_missing_file_is_a_displacement() {
            assert!(is_displacement(&os_error(Errno::NOENT)));
        }

        #[test]
        fn a_stale_handle_is_a_displacement() {
            assert!(is_displacement(&os_error(Errno::STALE)));
        }

        #[test]
        fn a_denied_access_is_not_a_displacement() {
            assert!(!is_displacement(&os_error(Errno::ACCESS)));
        }

        #[test]
        fn a_path_through_a_file_is_not_a_displacement() {
            assert!(!is_displacement(&os_error(Errno::NOTDIR)));
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

    pub fn probe_single_flight(
        _path: &Path,
    ) -> Result<super::SingleFlightProbe, TreeError> {
        unimplemented!("the single-flight lock is a Unix-only path")
    }

    pub fn wait_existing(
        _path: &Path,
    ) -> Result<Option<SingleFlight>, TreeError> {
        unimplemented!("the single-flight lock is a Unix-only path")
    }
}

#[cfg(not(unix))]
pub use stubs::{
    probe_single_flight, take_single_flight, try_single_flight, wait_existing,
    SingleFlight,
};
