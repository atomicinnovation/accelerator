use std::path::Path;
use std::time::Duration;

use super::super::ownership::safe_root::SafePruneRoot;
use super::super::prune_report::{
    LockFailure, PruneBudget, PruneReport, UnlistableDirectory,
};
use super::super::single_flight::LockPatience;
use super::super::tree::lease::HeldLeases;
use super::super::tree::tree_lock::HeldTreeLock;
use super::super::tree::{TreePrune, TreePruneRegime};
use super::ports::CachePorts;
use super::residue::ResidueSweep;
use super::root_entries::RootListing;
use super::root_lock::{HeldRootLock, ROOT_LOCK};
use super::single_file::SingleFilePrune;

pub struct RootPrune<'a> {
    pub safe: &'a SafePruneRoot,
    pub running_version: &'a str,
    pub plugin_root: Option<&'a Path>,
    pub claim_window: Duration,
    pub budget: PruneBudget,
    pub ports: &'a CachePorts,
}

impl RootPrune<'_> {
    /// Tree prune keeps the generations in `held_leases` without probing them.
    pub fn run(
        &self,
        patience: LockPatience,
        held_leases: &HeldLeases,
    ) -> PruneReport {
        let mut report = PruneReport::default();
        self.prune_into(patience, held_leases, &mut report);
        report
    }

    fn prune_into(
        &self,
        patience: LockPatience,
        held_leases: &HeldLeases,
        report: &mut PruneReport,
    ) {
        self.prune_single_files(patience, report);
        report.absorb(self.prune_trees(patience, held_leases));
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
                report.unlistable_directories.push(UnlistableDirectory {
                    path: root.to_path_buf(),
                    kind: error.kind(),
                });
                return;
            }
        };
        report.absorb(
            SingleFilePrune {
                root,
                running_version: self.running_version,
                plugin_root: self.plugin_root,
                claim_window: self.claim_window,
                budget: self.budget,
                ports: self.ports,
            }
            .run(&listing, &held),
        );
        report.removals.absorb(
            ResidueSweep::new(root, self.ports)
                .within(self.budget)
                .run(&listing),
        );
    }

    fn prune_trees(
        &self,
        patience: LockPatience,
        held_leases: &HeldLeases,
    ) -> PruneReport {
        self.whole_tree(held_leases).run(patience)
    }

    fn whole_tree<'a>(&'a self, held_leases: &'a HeldLeases) -> TreePrune<'a> {
        TreePrune {
            regime: TreePruneRegime::WholeTree(self.safe),
            claim_window: self.claim_window,
            budget: self.budget,
            ports: self.ports.tree_prune_ports(held_leases),
        }
    }

    /// The materialised artifact is pruned under the lock its materialise
    /// still holds; the lock is released before the rest of the root is
    /// pruned, so no other artifact's trees go while it is held.
    pub fn after_materialise(&self, held: HeldTreeLock) -> PruneReport {
        let none = HeldLeases::none();
        let mut report = PruneReport::default();
        report.absorb(self.whole_tree(&none).prune_held(&held));
        drop(held);
        self.prune_into(LockPatience::Try, &none, &mut report);
        report
    }
}
