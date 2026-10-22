//! Tree locks: the artifact-level lock over the injected single-flight
//! primitive, and the production primitive over `flock`.

use std::path::Path;

use crate::launch::core::tree::TreeError;

use super::super::single_flight::{LockGuard, LockProbe, SingleFlightLocks};
use super::layout::{Artifact, TreePaths};
use super::lease::{
    probe_single_flight, take_single_flight, try_single_flight, wait_existing,
    SingleFlightProbe,
};
use super::tree_lock::HeldTreeLock;

/// The only way to take a tree lock: the lock path is derived from the
/// artifact, so a guard and the artifact it names cannot disagree.
#[derive(Clone, Copy)]
pub struct TreeLocks<'a> {
    primitive: &'a dyn SingleFlightLocks,
}

pub enum TreeLockProbe {
    Taken(HeldTreeLock),
    Held,
    Absent,
}

impl<'a> TreeLocks<'a> {
    #[must_use]
    pub const fn over(primitive: &'a dyn SingleFlightLocks) -> Self {
        Self { primitive }
    }

    /// Take the artifact's tree lock, creating its file, blocking until it is
    /// free.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be created or locked.
    pub fn take_tree(
        &self,
        paths: &TreePaths,
        artifact: &Artifact,
    ) -> Result<HeldTreeLock, TreeError> {
        let guard = self.primitive.take(&paths.single_flight_lock(artifact))?;
        Ok(HeldTreeLock::holding(paths, artifact, guard))
    }

    /// Take the artifact's tree lock, creating its file, without blocking;
    /// `None` when another process holds it.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be opened, or locking
    /// fails for a reason other than contention.
    pub fn try_take_tree(
        &self,
        paths: &TreePaths,
        artifact: &Artifact,
    ) -> Result<Option<HeldTreeLock>, TreeError> {
        let guard = self
            .primitive
            .try_take(&paths.single_flight_lock(artifact))?;
        Ok(guard.map(|guard| HeldTreeLock::holding(paths, artifact, guard)))
    }

    /// Take the artifact's existing tree lock, blocking until it is free;
    /// `None` when it has no lock file, and none is created.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be opened or locked, or
    /// keeps being replaced.
    pub fn wait_existing_tree(
        &self,
        paths: &TreePaths,
        artifact: &Artifact,
    ) -> Result<Option<HeldTreeLock>, TreeError> {
        let guard = self
            .primitive
            .wait_existing(&paths.single_flight_lock(artifact))?;
        Ok(guard.map(|guard| HeldTreeLock::holding(paths, artifact, guard)))
    }

    /// Try the artifact's existing tree lock without blocking and without
    /// creating it.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file exists but cannot be locked.
    pub fn probe_tree(
        &self,
        paths: &TreePaths,
        artifact: &Artifact,
    ) -> Result<TreeLockProbe, TreeError> {
        Ok(
            match self.primitive.probe(&paths.single_flight_lock(artifact))? {
                LockProbe::Taken(guard) => TreeLockProbe::Taken(
                    HeldTreeLock::holding(paths, artifact, guard),
                ),
                LockProbe::Held => TreeLockProbe::Held,
                LockProbe::Absent => TreeLockProbe::Absent,
            },
        )
    }
}

pub struct FlockLocks;

impl SingleFlightLocks for FlockLocks {
    fn take(&self, path: &Path) -> Result<LockGuard, TreeError> {
        take_single_flight(path).map(LockGuard::holding)
    }

    fn try_take(&self, path: &Path) -> Result<Option<LockGuard>, TreeError> {
        Ok(try_single_flight(path)?.map(LockGuard::holding))
    }

    fn wait_existing(
        &self,
        path: &Path,
    ) -> Result<Option<LockGuard>, TreeError> {
        Ok(wait_existing(path)?.map(LockGuard::holding))
    }

    fn probe(&self, path: &Path) -> Result<LockProbe, TreeError> {
        Ok(match probe_single_flight(path)? {
            SingleFlightProbe::Taken(guard) => {
                LockProbe::Taken(LockGuard::holding(guard))
            }
            SingleFlightProbe::Held => LockProbe::Held,
            SingleFlightProbe::Absent => LockProbe::Absent,
        })
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::Path;

    use super::super::super::single_flight::{LockProbe, SingleFlightLocks};
    use super::super::layout::{Artifact, TreePaths};
    use super::super::testing::{within, BLOCKING_BOUND};
    use super::{FlockLocks, TreeLockProbe, TreeLocks};

    const LOCKS: TreeLocks<'static> = TreeLocks::over(&FlockLocks);

    fn browser() -> Artifact {
        Artifact::new("browser", "linux-x64")
    }

    fn trees_under(cache_root: &Path) -> TreePaths {
        let paths = TreePaths::under(cache_root);
        std::fs::create_dir_all(paths.root()).expect("trees");
        paths
    }

    fn is_held(paths: &TreePaths, artifact: &Artifact) -> bool {
        let lock = paths.single_flight_lock(artifact);
        within(BLOCKING_BOUND, "the probe", move || {
            matches!(FlockLocks.probe(&lock), Ok(LockProbe::Held))
        })
    }

    #[test]
    fn taking_a_tree_lock_holds_its_artifacts_single_flight_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees_under(dir.path());

        let held = LOCKS.take_tree(&paths, &browser()).expect("taken");

        assert_eq!(held.artifact(), &browser());
        assert_eq!(held.paths().root(), paths.root());
        assert!(is_held(&paths, &browser()), "the lock is not held");
    }

    #[test]
    fn trying_a_free_tree_lock_takes_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees_under(dir.path());

        let held = LOCKS
            .try_take_tree(&paths, &browser())
            .expect("tried")
            .expect("free");

        assert_eq!(held.artifact(), &browser());
        assert!(is_held(&paths, &browser()), "the lock is not held");
    }

    #[test]
    fn trying_a_tree_lock_held_elsewhere_gives_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees_under(dir.path());
        let _elsewhere = FlockLocks
            .take(&paths.single_flight_lock(&browser()))
            .expect("held elsewhere");

        let tried = within(BLOCKING_BOUND, "the try", move || {
            LOCKS
                .try_take_tree(&paths, &browser())
                .map(|held| held.is_some())
        });

        assert!(matches!(tried, Ok(false)), "the held lock was taken");
    }

    #[test]
    fn waiting_for_or_probing_an_absent_tree_lock_creates_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees_under(dir.path());

        let waited = LOCKS
            .wait_existing_tree(&paths, &browser())
            .expect("waited");
        let probed = LOCKS.probe_tree(&paths, &browser()).expect("probed");

        assert!(waited.is_none());
        assert!(matches!(probed, TreeLockProbe::Absent));
        assert!(!paths.single_flight_lock(&browser()).exists());
    }

    #[test]
    fn an_existing_tree_lock_is_waited_for_or_probed_as_its_artifacts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees_under(dir.path());
        drop(LOCKS.take_tree(&paths, &browser()).expect("created"));

        let waited = LOCKS
            .wait_existing_tree(&paths, &browser())
            .expect("waited")
            .expect("existing");
        let probed = LOCKS.probe_tree(&paths, &browser()).expect("probed");

        assert_eq!(waited.artifact(), &browser());
        assert!(matches!(probed, TreeLockProbe::Held));
        drop(waited);
        let TreeLockProbe::Taken(probed) =
            LOCKS.probe_tree(&paths, &browser()).expect("probed")
        else {
            panic!("the free lock was not taken");
        };
        assert_eq!(probed.artifact(), &browser());
    }

    #[test]
    fn a_taken_lock_is_held_until_its_guard_drops() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("lock");

        let guard = FlockLocks.take(&path).expect("taken");
        let probed = {
            let path = path.clone();
            within(BLOCKING_BOUND, "the probe", move || {
                matches!(FlockLocks.probe(&path), Ok(LockProbe::Held))
            })
        };
        assert!(probed, "the taken lock is not held");
        drop(guard);

        within(BLOCKING_BOUND, "the retake", move || FlockLocks.take(&path))
            .expect("retaken once released");
    }

    #[test]
    fn neither_waiting_nor_probing_creates_a_lock_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("lock");

        assert!(FlockLocks.wait_existing(&path).expect("waited").is_none());
        assert!(matches!(FlockLocks.probe(&path), Ok(LockProbe::Absent)));
        assert!(!path.exists());
    }
}
