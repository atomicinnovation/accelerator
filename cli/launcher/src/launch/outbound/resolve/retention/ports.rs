//! The one bundle of ports a cache root's resolvers and retention share, so a
//! single recorder sees every call any of them makes.
//!
//! Only the parts that compose a prune hold the bundle; each collaborator they
//! assemble is handed the narrow view of it that it uses.

use std::sync::Arc;

use crate::launch::core::tree::Clock;

use super::super::filesystem::CacheFilesystem;
use super::super::ownership::Ownership;
use super::super::prune_report::PruneSteps;
use super::super::single_flight::SingleFlightLocks;
use super::super::tree::lease::{HeldLeases, LivenessProbe};
use super::super::tree::locks::TreeLocks;
use super::super::tree::reap::{PinnedDigests, RemovalObserver};
use super::super::tree::TreePrunePorts;
use super::residue::BootstrapHolderLiveness;

pub struct CachePorts {
    pub filesystem: Arc<dyn CacheFilesystem + Send + Sync>,
    pub ownership: Arc<dyn Ownership + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub locks: Arc<dyn SingleFlightLocks + Send + Sync>,
    pub steps: Arc<dyn PruneSteps + Send + Sync>,
    pub bootstrap_holders: Arc<dyn BootstrapHolderLiveness + Send + Sync>,
    pub lease_liveness: Arc<dyn LivenessProbe + Send + Sync>,
    pub pins: Arc<dyn PinnedDigests + Send + Sync>,
    pub removals: Arc<dyn RemovalObserver + Send + Sync>,
}

/// What a prune of the root's own files uses.
#[derive(Clone, Copy)]
pub struct RootFilePorts<'a> {
    pub filesystem: &'a dyn CacheFilesystem,
    pub ownership: &'a dyn Ownership,
    pub clock: &'a dyn Clock,
    pub bootstrap_holders: &'a dyn BootstrapHolderLiveness,
}

/// What taking the root lock uses.
#[derive(Clone, Copy)]
pub struct RootLockPorts<'a> {
    pub filesystem: &'a dyn CacheFilesystem,
    pub ownership: &'a dyn Ownership,
    pub locks: &'a dyn SingleFlightLocks,
    pub steps: &'a dyn PruneSteps,
}

impl CachePorts {
    #[must_use]
    pub fn root_file_ports(&self) -> RootFilePorts<'_> {
        RootFilePorts {
            filesystem: self.filesystem.as_ref(),
            ownership: self.ownership.as_ref(),
            clock: self.clock.as_ref(),
            bootstrap_holders: self.bootstrap_holders.as_ref(),
        }
    }

    #[must_use]
    pub fn root_lock_ports(&self) -> RootLockPorts<'_> {
        RootLockPorts {
            filesystem: self.filesystem.as_ref(),
            ownership: self.ownership.as_ref(),
            locks: self.locks.as_ref(),
            steps: self.steps.as_ref(),
        }
    }

    #[must_use]
    pub fn tree_prune_ports<'a>(
        &'a self,
        held_leases: &'a HeldLeases,
    ) -> TreePrunePorts<'a> {
        TreePrunePorts {
            held_leases,
            filesystem: self.filesystem.as_ref(),
            ownership: self.ownership.as_ref(),
            clock: self.clock.as_ref(),
            lease_liveness: self.lease_liveness.as_ref(),
            pins: self.pins.as_ref(),
            locks: TreeLocks::over(self.locks.as_ref()),
            steps: self.steps.as_ref(),
            removals: self.removals.as_ref(),
        }
    }
}
