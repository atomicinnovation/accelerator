use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::super::cache_root::{
    CacheRoot, NoCacheRoot, PluginBinRoot, Unconfirmed,
};
use super::super::ownership::safe_root::{
    SafePruneRoot, TrustedOwners, UnsafeRoot,
};
use super::super::prune_report::PruneBudget;
use super::super::retention_windows::{CLAIM_WINDOW, COLD_PATH_BUDGET};
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
                budget: PruneBudget::Unbounded,
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
        budget: PruneBudget,
    ) -> RootPrune<'a> {
        RootPrune {
            safe,
            running_version: &self.running_version,
            plugin_root: self.plugin_root.as_deref(),
            claim_window,
            budget,
            ports: &self.ports,
        }
    }

    fn cold_path_budget(&self) -> PruneBudget {
        PruneBudget::starting_now(self.ports.clock.as_ref(), COLD_PATH_BUDGET)
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
                    .root_prune(&safe, CLAIM_WINDOW, self.cold_path_budget())
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
                &self
                    .root_prune(&safe, CLAIM_WINDOW, self.cold_path_budget())
                    .after_materialise(held),
            ),
            None => drop(held),
        }
    }
}

impl RootRetention for RelocatedRetention {
    /// Written only where no other user could redirect the write, and
    /// walked only once a claim is due. A root made safe later begins its
    /// grace with its first claim.
    fn claim_running_version(&self) {
        let launcher_id = launcher_id();
        let claim = VersionClaim {
            version: &self.running_version,
            launcher_id: &launcher_id,
        };
        let filesystem = self.ports.filesystem.as_ref();
        let ownership = self.ports.ownership.as_ref();
        let clock = self.ports.clock.as_ref();
        if version_claims::is_due(
            &self.root, &claim, filesystem, ownership, clock,
        ) && self.safe_root().is_ok()
        {
            version_claims::write(
                &self.root, &claim, filesystem, ownership, clock,
            );
        }
    }

    fn prune(&self, older_than: Option<Duration>) -> PruneOutcome {
        let safe = match self.safe_root() {
            Ok(safe) => safe,
            Err(unsafe_root) => return PruneOutcome::Refused(unsafe_root),
        };
        PruneOutcome::Ran(
            self.root_prune(
                &safe,
                older_than.unwrap_or(CLAIM_WINDOW),
                PruneBudget::Unbounded,
            )
            .run(LockPatience::Wait, &HeldLeases::none()),
        )
    }
}

pub struct UnidentifiedRetention {
    error: NoCacheRoot,
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
        PruneOutcome::Unidentified(self.error.clone())
    }
}

pub struct UnconfirmedRetention {
    root: PathBuf,
    reason: Unconfirmed,
}

impl AfterStore for UnconfirmedRetention {
    fn after_store(&self, _held_leases: &HeldLeases) {}
}

impl AfterMaterialise for UnconfirmedRetention {
    fn reap_permit(&self) -> Option<ReapPermit> {
        None
    }

    fn after_materialise(&self, held: HeldTreeLock) {
        drop(held);
    }
}

impl RootRetention for UnconfirmedRetention {
    fn claim_running_version(&self) {}

    fn prune(&self, _older_than: Option<Duration>) -> PruneOutcome {
        PruneOutcome::Unconfirmed {
            root: self.root.clone(),
            reason: self.reason,
        }
    }
}

#[must_use]
pub fn for_root(
    root: Result<&CacheRoot, &NoCacheRoot>,
    plugin_root: Option<&Path>,
    trusted_owners: TrustedOwners,
    ports: Arc<CachePorts>,
) -> Arc<dyn RootRetention + Send + Sync> {
    let root = match root {
        Ok(root) => root,
        Err(error) => {
            return Arc::new(UnidentifiedRetention {
                error: error.clone(),
            })
        }
    };
    if let Some(reason) = root.unconfirmed() {
        return Arc::new(UnconfirmedRetention {
            root: root.path().to_path_buf(),
            reason,
        });
    }
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
        identify, CacheRoot, CacheRootConfig, NoCacheRoot, Unconfirmed,
    };
    use super::super::super::filesystem::testing::ReportingFilesystem;
    use super::super::super::filesystem::SystemFilesystem;
    use super::super::super::ownership::safe_root::TrustedOwners;
    use super::super::super::retention_windows::{AGE_BACKSTOP, CLAIM_WINDOW};
    use super::super::super::tree::lease::HeldLeases;
    use super::super::super::tree::testing::logged;
    use super::super::launcher_id;
    use super::super::ports::CachePorts;
    use super::super::root_prune::PruneOutcome;
    use super::super::testing::{now, set_mtime, Ports};
    use super::super::version_claims::{CLAIM_AWARE_SINCE, VERSION_CLAIMS_DIR};
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

    fn overriding(
        cache_dir: PathBuf,
        plugin_root: Option<PathBuf>,
    ) -> CacheRoot {
        identify(
            &CacheRootConfig {
                cache_dir_override: Some(cache_dir),
                plugin_root,
            },
            &SystemFilesystem,
        )
        .expect("identified")
    }

    fn relocated() -> (tempfile::TempDir, CacheRoot) {
        let temp = tempfile::tempdir().expect("tempdir");
        let plugin = temp.path().join("plugin");
        std::fs::create_dir_all(plugin.join("bin")).expect("plugin bin");
        let cache = temp.path().join("cache");
        std::fs::create_dir(&cache).expect("cache");
        let root = overriding(cache, Some(plugin));
        (temp, root)
    }

    fn unconfirmed() -> (tempfile::TempDir, CacheRoot) {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = overriding(temp.path().to_path_buf(), None);
        (temp, root)
    }

    fn claims(root: &CacheRoot) -> PathBuf {
        root.path().join(VERSION_CLAIMS_DIR)
    }

    fn retention_for(
        root: Result<&CacheRoot, &NoCacheRoot>,
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

        retention_for(Err(&NoCacheRoot)).claim_running_version();
        retention_for(Ok(&relocated)).claim_running_version();

        assert_eq!(
            std::fs::read_dir(claims(&relocated))
                .expect("claims")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name() != CLAIM_AWARE_SINCE)
                .count(),
            1,
            "only the identified root is claimed"
        );
    }

    #[test]
    fn an_unidentifiable_root_is_never_pruned() {
        assert_eq!(
            retention_for(Err(&NoCacheRoot)).prune(None),
            PruneOutcome::Unidentified(NoCacheRoot)
        );
    }

    #[test]
    fn an_unconfirmed_root_is_never_claimed_pruned_or_reaped() {
        let (_temp, root) = unconfirmed();
        let retention = retention_for(Ok(&root));

        retention.claim_running_version();

        assert!(!claims(&root).exists());
        assert!(retention.reap_permit().is_none());
        assert_eq!(
            retention.prune(None),
            PruneOutcome::Unconfirmed {
                root: root.path().to_path_buf(),
                reason: Unconfirmed::NoPluginBinToCompare,
            }
        );
    }

    fn world_writable(root: &CacheRoot) {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(
            root.path(),
            std::fs::Permissions::from_mode(0o777),
        )
        .expect("world-writable");
    }

    #[cfg(unix)]
    #[test]
    fn a_cold_path_logs_an_unsafe_root_once_per_process() {
        let (_temp, root) = relocated();
        world_writable(&root);
        let retention = retention_for(Ok(&root));

        let log = logged(|| {
            retention.after_store(&HeldLeases::none());
            retention.after_store(&HeldLeases::none());
        });

        assert_eq!(
            log.matches("not pruning the cache root").count(),
            1,
            "{log}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_cold_path_logs_what_its_prune_could_not_do() {
        let (_temp, root) = relocated();
        let at = now();
        std::fs::create_dir(claims(&root)).expect("claims");
        let marker = claims(&root).join(CLAIM_AWARE_SINCE);
        std::fs::write(&marker, b"").expect("marker");
        set_mtime(&marker, at - 2 * CLAIM_WINDOW.as_secs());
        let stem = format!("vcs-1.0.0-{}", "0".repeat(64));
        for name in [stem.clone(), format!("{stem}.minisig")] {
            let path = root.path().join(name);
            std::fs::write(&path, b"x").expect("entry");
            set_mtime(&path, at - 2 * AGE_BACKSTOP.as_secs());
        }
        let signature = root.path().join(format!("{stem}.minisig"));
        let filesystem = ReportingFilesystem::over_system()
            .failing_removal(&signature, std::io::ErrorKind::PermissionDenied);
        let retention = for_root(
            Ok(&root),
            None,
            TrustedOwners::none(),
            Arc::new(
                Ports {
                    filesystem: Arc::new(filesystem),
                    ..Ports::at(now())
                }
                .build(),
            ),
        );

        let log = logged(|| retention.after_store(&HeldLeases::none()));

        assert!(
            log.contains(&format!("could not remove {}", signature.display())),
            "{log}"
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
