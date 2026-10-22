//! The root prune of a relocated cache root: single-file prune and the
//! residue sweep under the root lock, then tree prune once it is released.

use std::path::Path;
use std::time::Duration;

use super::super::ownership::safe_root::{SafePruneRoot, UnsafeRoot};
use super::super::prune_report::{
    LockFailure, RemovalFailure, Removals, UnreadableClaims, UntrustedDirectory,
};
use super::super::single_flight::LockPatience;
use super::super::tree::lease::HeldLeases;
use super::super::tree::locks::TreeLocks;
use super::super::tree::{
    TreePrune, TreePrunePorts, TreePruneRegime, TreePruneReport,
};
use super::ports::CachePorts;
use super::residue::ResidueSweep;
use super::root_entries::RootListing;
use super::root_lock::{HeldRootLock, ROOT_LOCK};
use super::single_file::SingleFilePrune;

/// Prunes the root that was checked safe, and nothing else.
pub struct RootPrune<'a> {
    pub safe: &'a SafePruneRoot,
    pub running_version: &'a str,
    pub plugin_root: Option<&'a Path>,
    pub claim_window: Duration,
    pub ports: &'a CachePorts,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PruneReport {
    pub reclaimed: u64,
    pub lock_failures: Vec<LockFailure>,
    pub unreadable_claims: Vec<UnreadableClaims>,
    pub removal_failures: Vec<RemovalFailure>,
    pub untrusted_directories: Vec<UntrustedDirectory>,
}

impl PruneReport {
    fn absorb(&mut self, removals: Removals) {
        self.reclaimed += removals.reclaimed;
        self.removal_failures.extend(removals.failures);
    }

    /// Whether every lock was taken and every removal made; an unreadable or
    /// untrusted directory only kept entries.
    #[must_use]
    pub const fn finished(&self) -> bool {
        self.lock_failures.is_empty() && self.removal_failures.is_empty()
    }

    /// One line for each thing the prune could not do.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let locks = self.lock_failures.iter().map(|failure| {
            format!(
                "could not lock {}: {}",
                failure.lock.display(),
                failure.detail
            )
        });
        let claims = self.unreadable_claims.iter().map(|unreadable| {
            format!(
                "claims unreadable ({}) in {}; kept everything they could \
                 protect",
                unreadable.kind,
                unreadable.directory.display()
            )
        });
        let untrusted = self.untrusted_directories.iter().map(|untrusted| {
            format!(
                "not pruning through {}: another user could write to it",
                untrusted.path.display()
            )
        });
        let removals = self.removal_failures.iter().map(|failure| {
            format!(
                "could not remove {}: {}",
                failure.path.display(),
                failure.kind
            )
        });
        locks
            .chain(claims)
            .chain(untrusted)
            .chain(removals)
            .collect()
    }

    pub fn absorb_trees(&mut self, trees: TreePruneReport) {
        self.absorb(trees.removals);
        self.lock_failures.extend(trees.lock_failures);
        self.unreadable_claims.extend(trees.unreadable_claims);
        self.untrusted_directories
            .extend(trees.untrusted_directories);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PruneOutcome {
    Refused(UnsafeRoot),
    Ran(PruneReport),
    /// The cache root could not be identified, so there is nothing to prune.
    Unavailable {
        detail: String,
    },
}

impl RootPrune<'_> {
    /// `held_leases` are the generations this process holds leases on, which
    /// tree prune keeps without probing.
    pub fn run(
        &self,
        patience: LockPatience,
        held_leases: &HeldLeases,
    ) -> PruneReport {
        let mut report = PruneReport::default();
        self.prune_single_files(patience, &mut report);
        report.absorb_trees(self.prune_trees(patience, held_leases));
        report
    }

    /// The root lock is released when this returns, before any tree lock is
    /// taken.
    fn prune_single_files(
        &self,
        patience: LockPatience,
        report: &mut PruneReport,
    ) {
        let root = self.safe.path();
        let held = match HeldRootLock::acquire(root, patience, self.ports) {
            Ok(Some(held)) => held,
            Ok(None) => return,
            Err(error) => {
                report.lock_failures.push(LockFailure {
                    lock: root.join(ROOT_LOCK),
                    detail: error.to_string(),
                });
                return;
            }
        };
        let listing = match RootListing::read(
            root,
            self.ports.filesystem.as_ref(),
            self.ports.steps.as_ref(),
        ) {
            Ok(listing) => listing,
            Err(error) => {
                report.removal_failures.push(RemovalFailure {
                    path: root.to_path_buf(),
                    kind: error.kind(),
                });
                return;
            }
        };
        let single_files = SingleFilePrune {
            root,
            running_version: self.running_version,
            plugin_root: self.plugin_root,
            claim_window: self.claim_window,
            ports: self.ports,
        }
        .run(&listing, &held);
        report.absorb(single_files.removals);
        report
            .unreadable_claims
            .extend(single_files.unreadable_claims);
        report.absorb(ResidueSweep::new(root, self.ports).run(&listing));
    }

    fn prune_trees(
        &self,
        patience: LockPatience,
        held_leases: &HeldLeases,
    ) -> TreePruneReport {
        TreePrune {
            regime: TreePruneRegime::WholeTree(self.safe),
            claim_window: self.claim_window,
            ports: TreePrunePorts {
                held_leases,
                filesystem: self.ports.filesystem.as_ref(),
                ownership: self.ports.ownership.as_ref(),
                clock: self.ports.clock.as_ref(),
                lease_liveness: self.ports.lease_liveness.as_ref(),
                pins: self.ports.pins.as_ref(),
                locks: TreeLocks::over(self.ports.locks.as_ref()),
                steps: self.ports.steps.as_ref(),
                removals: self.ports.removals.as_ref(),
            },
        }
        .run(patience)
    }
}
