//! What follows a materialise, and whether its root may be reaped at all.

use super::super::cache_root::PluginBinRoot;
use super::super::filesystem::CacheFilesystem;
use super::super::ownership::safe_root::SafePruneRoot;
use super::super::ownership::{is_trusted_directory, Ownership};
use super::layout::TreePaths;
use super::tree_lock::HeldTreeLock;

pub trait AfterMaterialise {
    /// Asked before a materialise or repair reaps under its tree lock.
    fn reap_permit(&self) -> Option<ReapPermit>;

    /// Takes the materialise's lock by value, so releasing it is part of
    /// the contract.
    fn after_materialise(&self, held: HeldTreeLock);
}

/// The root a reap would run in, as it was checked.
#[derive(Clone, Copy)]
pub enum RootTrust<'a> {
    PluginBin(&'a PluginBinRoot),
    Relocated(&'a SafePruneRoot),
}

/// Proof that a root's `trees/` may be reaped: no other user can write to
/// it, and a relocated root was itself checked safe to prune.
pub struct ReapPermit {
    paths: TreePaths,
}

impl ReapPermit {
    #[must_use]
    pub fn for_root(
        trust: RootTrust<'_>,
        filesystem: &dyn CacheFilesystem,
        ownership: &dyn Ownership,
    ) -> Option<Self> {
        let paths = match trust {
            RootTrust::PluginBin(root) => TreePaths::under(root.path()),
            RootTrust::Relocated(safe) => TreePaths::under(safe.path()),
        };
        is_trusted_directory(paths.root(), filesystem, ownership)
            .then_some(Self { paths })
    }

    #[cfg(test)]
    pub(crate) const fn trusting(paths: TreePaths) -> Self {
        Self { paths }
    }

    #[must_use]
    pub const fn paths(&self) -> &TreePaths {
        &self.paths
    }
}
