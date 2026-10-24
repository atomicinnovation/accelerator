use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::super::cache_root::{CacheRoot, PluginBinRoot};
use super::super::ownership::safe_root::{
    SafePruneRoot, TrustedOwners, UnsafeRoot,
};
use super::super::retention_windows::CLAIM_WINDOW;
use super::super::single_flight::LockPatience;
use super::super::tree::after_materialise::{
    AfterMaterialise, ReapPermit, RootTrust,
};
use super::super::tree::lease::HeldLeases;
use super::super::tree::tree_lock::HeldTreeLock;
use super::super::tree::{TreePrune, TreePruneRegime};
use super::super::AfterStore;
use super::launcher_id;
use super::ports::CachePorts;
use super::root_prune::{PruneOutcome, PruneReport, RootPrune};
use super::version_claims::{self, VersionClaim};

pub trait RootRetention: AfterStore + AfterMaterialise {
    fn claim_running_version(&self);

    /// Waits for the root's locks; `older_than` replaces the claim window for
    /// version and tree claims alike.
    fn prune(&self, older_than: Option<Duration>) -> PruneOutcome;
}

pub struct PluginBinRetention {
    root: PluginBinRoot,
    ports: Arc<CachePorts>,
}

impl PluginBinRetention {
    #[must_use]
    pub const fn new(root: PluginBinRoot, ports: Arc<CachePorts>) -> Self {
        Self { root, ports }
    }
}

impl AfterStore for PluginBinRetention {
    fn after_store(&self, _held_leases: &HeldLeases) {}
}

/// The materialise has already reaped under its lock, wherever the user owns
/// `trees/`.
impl AfterMaterialise for PluginBinRetention {
    fn reap_permit(&self) -> Option<ReapPermit> {
        ReapPermit::for_root(
            RootTrust::PluginBin(&self.root),
            self.ports.filesystem.as_ref(),
            self.ports.ownership.as_ref(),
        )
    }

    fn after_materialise(&self, held: HeldTreeLock) {
        drop(held);
    }
}

impl RootRetention for PluginBinRetention {
    fn claim_running_version(&self) {}

    fn prune(&self, older_than: Option<Duration>) -> PruneOutcome {
        let mut report = PruneReport::default();
        report.absorb_trees(
            TreePrune {
                regime: TreePruneRegime::PinnedOnHost(&self.root),
                claim_window: older_than.unwrap_or(CLAIM_WINDOW),
                ports: self.ports.tree_prune_ports(&HeldLeases::none()),
            }
            .run(LockPatience::Wait),
        );
        PruneOutcome::Ran(report)
    }
}

/// A root shared across plugin versions, which each version claims and any
/// of them prunes once the root is checked to be its owner's alone.
pub struct RelocatedRetention {
    root: PathBuf,
    running_version: String,
    plugin_root: Option<PathBuf>,
    trusted_owners: TrustedOwners,
    ports: Arc<CachePorts>,
    refusal_logged: AtomicBool,
}

impl RelocatedRetention {
    fn safe_root(&self) -> Result<SafePruneRoot, UnsafeRoot> {
        SafePruneRoot::check(
            &self.root,
            self.ports.filesystem.as_ref(),
            self.ports.ownership.as_ref(),
            &self.trusted_owners,
        )
    }

    fn root_prune<'a>(
        &'a self,
        safe: &'a SafePruneRoot,
        claim_window: Duration,
    ) -> RootPrune<'a> {
        RootPrune {
            safe,
            running_version: &self.running_version,
            plugin_root: self.plugin_root.as_deref(),
            claim_window,
            ports: &self.ports,
        }
    }

    /// A cold path reports nothing to the user, so an unsafe root is logged
    /// instead, once per process.
    fn safe_for_cold_path(&self) -> Option<SafePruneRoot> {
        self.safe_root()
            .inspect_err(|unsafe_root| {
                if !self.refusal_logged.swap(true, Ordering::Relaxed) {
                    tracing::warn!(%unsafe_root, "not pruning the cache root");
                }
            })
            .ok()
    }
}

/// A cold path skips what it cannot lock, so what it could not do is logged.
fn warn_of(report: &PruneReport) {
    for problem in report.problems() {
        tracing::warn!("{problem}");
    }
}

impl AfterStore for RelocatedRetention {
    fn after_store(&self, held_leases: &HeldLeases) {
        if let Some(safe) = self.safe_for_cold_path() {
            warn_of(
                &self
                    .root_prune(&safe, CLAIM_WINDOW)
                    .run(LockPatience::Try, held_leases),
            );
        }
    }
}

/// Only a root checked safe to prune is reaped, so no prune path in an
/// unsafe root removes anything.
impl AfterMaterialise for RelocatedRetention {
    fn reap_permit(&self) -> Option<ReapPermit> {
        let safe = self.safe_root().ok()?;
        ReapPermit::for_root(
            RootTrust::Relocated(&safe),
            self.ports.filesystem.as_ref(),
            self.ports.ownership.as_ref(),
        )
    }

    fn after_materialise(&self, held: HeldTreeLock) {
        match self.safe_for_cold_path() {
            Some(safe) => warn_of(
                &self.root_prune(&safe, CLAIM_WINDOW).after_materialise(held),
            ),
            None => drop(held),
        }
    }
}

impl RootRetention for RelocatedRetention {
    /// Written even in an unsafe root, so a root made safe later keeps its
    /// protection.
    fn claim_running_version(&self) {
        let launcher_id = launcher_id();
        version_claims::refresh(
            &self.root,
            &VersionClaim {
                version: &self.running_version,
                launcher_id: &launcher_id,
            },
            self.ports.filesystem.as_ref(),
            self.ports.ownership.as_ref(),
            self.ports.clock.as_ref(),
        );
    }

    fn prune(&self, older_than: Option<Duration>) -> PruneOutcome {
        let safe = match self.safe_root() {
            Ok(safe) => safe,
            Err(unsafe_root) => return PruneOutcome::Refused(unsafe_root),
        };
        PruneOutcome::Ran(
            self.root_prune(&safe, older_than.unwrap_or(CLAIM_WINDOW))
                .run(LockPatience::Wait, &HeldLeases::none()),
        )
    }
}

pub struct UnidentifiedRetention {
    detail: String,
}

impl AfterStore for UnidentifiedRetention {
    fn after_store(&self, _held_leases: &HeldLeases) {}
}

impl AfterMaterialise for UnidentifiedRetention {
    fn reap_permit(&self) -> Option<ReapPermit> {
        None
    }

    fn after_materialise(&self, held: HeldTreeLock) {
        drop(held);
    }
}

impl RootRetention for UnidentifiedRetention {
    fn claim_running_version(&self) {}

    fn prune(&self, _older_than: Option<Duration>) -> PruneOutcome {
        PruneOutcome::Unidentified {
            detail: self.detail.clone(),
        }
    }
}

#[must_use]
pub fn for_root(
    root: Result<&CacheRoot, &str>,
    plugin_root: Option<&Path>,
    trusted_owners: TrustedOwners,
    ports: Arc<CachePorts>,
) -> Arc<dyn RootRetention + Send + Sync> {
    let root = match root {
        Ok(root) => root,
        Err(detail) => {
            return Arc::new(UnidentifiedRetention {
                detail: detail.to_owned(),
            })
        }
    };
    match root.plugin_bin() {
        Some(plugin_bin) => {
            Arc::new(PluginBinRetention::new(plugin_bin, ports))
        }
        None => Arc::new(RelocatedRetention {
            root: root.path().to_path_buf(),
            running_version: env!("CARGO_PKG_VERSION").to_owned(),
            plugin_root: plugin_root.map(Path::to_path_buf),
            trusted_owners,
            ports,
            refusal_logged: AtomicBool::new(false),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::super::super::cache_root::{
        identify, CacheRoot, CacheRootConfig,
    };
    use super::super::super::filesystem::SystemFilesystem;
    use super::super::super::ownership::safe_root::TrustedOwners;
    use super::super::launcher_id;
    use super::super::ports::CachePorts;
    use super::super::root_prune::PruneOutcome;
    use super::super::testing::{now, Ports};
    use super::super::version_claims::VERSION_CLAIMS_DIR;
    use super::{for_root, PluginBinRetention, RootRetention as _};

    fn ports() -> Arc<CachePorts> {
        Arc::new(Ports::at(now()).build())
    }

    fn plugin_bin() -> (tempfile::TempDir, CacheRoot) {
        let plugin = tempfile::tempdir().expect("plugin root");
        std::fs::create_dir(plugin.path().join("bin")).expect("plugin bin");
        let root = identify(
            &CacheRootConfig {
                cache_dir_override: None,
                plugin_root: Some(plugin.path().to_path_buf()),
            },
            &SystemFilesystem,
        )
        .expect("identified");
        (plugin, root)
    }

    fn relocated() -> (tempfile::TempDir, CacheRoot) {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = identify(
            &CacheRootConfig {
                cache_dir_override: Some(temp.path().to_path_buf()),
                plugin_root: None,
            },
            &SystemFilesystem,
        )
        .expect("identified");
        (temp, root)
    }

    fn claims(root: &CacheRoot) -> PathBuf {
        root.path().join(VERSION_CLAIMS_DIR)
    }

    fn retention_for(
        root: Result<&CacheRoot, &str>,
    ) -> Arc<dyn super::RootRetention + Send + Sync> {
        for_root(root, None, TrustedOwners::none(), ports())
    }

    #[test]
    fn the_plugin_bin_retention_claims_nothing() {
        let (_temp, root) = plugin_bin();

        PluginBinRetention::new(
            root.plugin_bin().expect("plugin bin"),
            ports(),
        )
        .claim_running_version();

        assert!(!claims(&root).exists());
    }

    #[test]
    fn an_unidentifiable_root_is_never_claimed() {
        let (_temp, relocated) = relocated();

        retention_for(Err("unidentified")).claim_running_version();
        retention_for(Ok(&relocated)).claim_running_version();

        assert_eq!(
            std::fs::read_dir(claims(&relocated))
                .expect("claims")
                .count(),
            1,
            "only the identified root is claimed"
        );
    }

    #[test]
    fn an_unidentifiable_root_is_never_pruned() {
        assert_eq!(
            retention_for(Err("unidentified")).prune(None),
            PruneOutcome::Unidentified {
                detail: "unidentified".to_owned()
            }
        );
    }

    #[test]
    fn for_root_chooses_the_retention_from_the_root_kind() {
        let (_plugin_temp, plugin_bin) = plugin_bin();
        let (_relocated_temp, relocated) = relocated();

        retention_for(Ok(&plugin_bin)).claim_running_version();
        retention_for(Ok(&relocated)).claim_running_version();

        assert!(!claims(&plugin_bin).exists());
        assert!(claims(&relocated)
            .join(format!("{}.{}", env!("CARGO_PKG_VERSION"), launcher_id()))
            .is_file());
    }
}
