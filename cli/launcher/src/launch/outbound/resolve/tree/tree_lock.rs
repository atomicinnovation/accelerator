use crate::launch::core::tree::TreeError;

use super::layout::{Artifact, TreePaths};
use super::lease::{take_single_flight, try_single_flight, SingleFlight};

/// A tree lock this process holds, the artifact it guards and the root it was
/// taken under; the only way to say whose reap is running.
pub struct HeldTreeLock {
    paths: TreePaths,
    artifact: Artifact,
    _guard: SingleFlight,
}

impl HeldTreeLock {
    /// Take the artifact's tree lock, blocking until it is available.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be created or locked.
    pub fn take(
        paths: &TreePaths,
        artifact: Artifact,
    ) -> Result<Self, TreeError> {
        let guard =
            take_single_flight(&paths.single_flight_lock_of(&artifact))?;
        Ok(Self::holding(paths, artifact, guard))
    }

    /// Take the artifact's tree lock without blocking; `None` when another
    /// process holds it.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file cannot be opened, or locking
    /// fails for a reason other than contention.
    pub fn try_take(
        paths: &TreePaths,
        artifact: Artifact,
    ) -> Result<Option<Self>, TreeError> {
        let guard = try_single_flight(&paths.single_flight_lock_of(&artifact))?;
        Ok(guard.map(|guard| Self::holding(paths, artifact, guard)))
    }

    fn holding(
        paths: &TreePaths,
        artifact: Artifact,
        guard: SingleFlight,
    ) -> Self {
        Self {
            paths: paths.clone(),
            artifact,
            _guard: guard,
        }
    }

    #[must_use]
    pub const fn artifact(&self) -> &Artifact {
        &self.artifact
    }

    #[must_use]
    pub const fn paths(&self) -> &TreePaths {
        &self.paths
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::Path;

    use super::super::layout::{Artifact, TreePaths};
    use super::super::lease::{
        probe_single_flight, take_single_flight, SingleFlightProbe,
    };
    use super::super::testing::{within, BLOCKING_BOUND};
    use super::HeldTreeLock;

    fn browser() -> Artifact {
        Artifact::new("browser", "linux-x64")
    }

    fn trees_under(cache_root: &Path) -> TreePaths {
        let paths = TreePaths::under(cache_root);
        std::fs::create_dir_all(paths.root()).expect("trees");
        paths
    }

    fn is_held(paths: &TreePaths, artifact: &Artifact) -> bool {
        let lock = paths.single_flight_lock_of(artifact);
        within(BLOCKING_BOUND, "the probe", move || {
            matches!(probe_single_flight(&lock), Ok(SingleFlightProbe::Held))
        })
    }

    #[test]
    fn holding_a_tree_lock_holds_its_artifacts_single_flight_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees_under(dir.path());

        let held = HeldTreeLock::take(&paths, browser()).expect("taken");

        assert_eq!(held.artifact(), &browser());
        assert_eq!(held.paths().root(), paths.root());
        assert!(is_held(&paths, &browser()), "the lock is not held");
    }

    #[test]
    fn trying_a_free_tree_lock_takes_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees_under(dir.path());

        let held = HeldTreeLock::try_take(&paths, browser())
            .expect("tried")
            .expect("free");

        assert_eq!(held.artifact(), &browser());
        assert!(is_held(&paths, &browser()), "the lock is not held");
    }

    #[test]
    fn trying_a_tree_lock_held_elsewhere_gives_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = trees_under(dir.path());
        let _elsewhere =
            take_single_flight(&paths.single_flight_lock_of(&browser()))
                .expect("held elsewhere");

        let tried = within(BLOCKING_BOUND, "the try", move || {
            HeldTreeLock::try_take(&paths, browser()).map(|held| held.is_some())
        });

        assert!(matches!(tried, Ok(false)), "the held lock was taken");
    }
}
