use super::super::single_flight::LockGuard;
use super::layout::{Artifact, TreePaths};

/// A tree lock this process holds, the artifact it guards and the root it was
/// taken under; the only way to say whose reap is running.
pub struct HeldTreeLock {
    paths: TreePaths,
    artifact: Artifact,
    _guard: LockGuard,
}

impl HeldTreeLock {
    pub(super) fn holding(
        paths: &TreePaths,
        artifact: &Artifact,
        guard: LockGuard,
    ) -> Self {
        Self {
            paths: paths.clone(),
            artifact: artifact.clone(),
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
