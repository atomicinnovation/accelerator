//! The retention regime governing a cache root, chosen once from its kind.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use super::super::cache_root::{CacheRoot, CacheRootKind};
use super::super::ownership::safe_root::{SafePruneRoot, TrustedOwners};
use super::super::prune_report::LockFailure;
use super::super::retention_windows::CLAIM_WINDOW;
use super::super::single_flight::LockPatience;
use super::super::tree::{prune_pinned_on_host, PinnedPruneCollaborators};
use super::launcher_id;
use super::ports::CachePorts;
use super::root_prune::{PruneOutcome, PruneReport, RootPrune};
use super::version_claims::{self, VersionClaim};

pub trait RootRetention {
    /// Declare that the running version still uses this root.
    fn claim_running_version(&self);

    /// Prune the root, waiting for its locks; `older_than` replaces the claim
    /// window for version and tree claims alike.
    fn prune(&self, older_than: Option<Duration>) -> PruneOutcome;
}

/// The plugin's own `bin`, which ADR-0063 governs: nothing is claimed, and
/// prune visits only the pinned artifacts' trees on this host.
pub struct PluginBinRetention {
    root: PathBuf,
    ports: Arc<CachePorts>,
}

impl PluginBinRetention {
    #[must_use]
    pub const fn new(root: PathBuf, ports: Arc<CachePorts>) -> Self {
        Self { root, ports }
    }
}

impl RootRetention for PluginBinRetention {
    fn claim_running_version(&self) {}

    fn prune(&self, older_than: Option<Duration>) -> PruneOutcome {
        let collaborators = PinnedPruneCollaborators {
            clock: self.ports.clock.as_ref(),
            liveness: self.ports.lease_liveness.as_ref(),
            pins: self.ports.pins.as_ref(),
        };
        let mut report = PruneReport::default();
        match prune_pinned_on_host(&self.root, &collaborators, older_than) {
            Ok(reclaimed) => report.reclaimed = reclaimed.entries,
            Err(error) => report.lock_failures.push(LockFailure {
                lock: self.root.join("trees"),
                detail: error.to_string(),
            }),
        }
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
}

impl RootRetention for RelocatedRetention {
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

    /// An unsafe root removes nothing; its claims are still written, so a root
    /// made safe later keeps its protection.
    fn prune(&self, older_than: Option<Duration>) -> PruneOutcome {
        let safe = match SafePruneRoot::check(
            &self.root,
            self.ports.filesystem.as_ref(),
            self.ports.ownership.as_ref(),
            &self.trusted_owners,
        ) {
            Ok(safe) => safe,
            Err(unsafe_root) => return PruneOutcome::Refused(unsafe_root),
        };
        PruneOutcome::Ran(
            RootPrune {
                safe: &safe,
                running_version: &self.running_version,
                plugin_root: self.plugin_root.as_deref(),
                claim_window: older_than.unwrap_or(CLAIM_WINDOW),
                ports: &self.ports,
            }
            .run(LockPatience::Wait),
        )
    }
}

/// A root that could not be identified is never claimed or pruned.
pub struct UnidentifiedRetention {
    detail: String,
}

impl RootRetention for UnidentifiedRetention {
    fn claim_running_version(&self) {}

    fn prune(&self, _older_than: Option<Duration>) -> PruneOutcome {
        PruneOutcome::Unavailable {
            detail: self.detail.clone(),
        }
    }
}

/// The retention for the identified `root`.
#[must_use]
pub fn for_root(
    root: Result<&CacheRoot, &str>,
    plugin_root: Option<&Path>,
    trusted_owners: TrustedOwners,
    ports: Arc<CachePorts>,
) -> Arc<dyn RootRetention + Send + Sync> {
    match root {
        Ok(CacheRoot {
            path,
            kind: CacheRootKind::Relocated,
        }) => Arc::new(RelocatedRetention {
            root: path.clone(),
            running_version: env!("CARGO_PKG_VERSION").to_owned(),
            plugin_root: plugin_root.map(Path::to_path_buf),
            trusted_owners,
            ports,
        }),
        Ok(CacheRoot {
            path,
            kind: CacheRootKind::PluginBin,
        }) => Arc::new(PluginBinRetention::new(path.clone(), ports)),
        Err(detail) => Arc::new(UnidentifiedRetention {
            detail: detail.to_owned(),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::super::super::cache_root::{CacheRoot, CacheRootKind};
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

    fn root_of(kind: CacheRootKind) -> (tempfile::TempDir, CacheRoot) {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = CacheRoot {
            path: temp.path().to_path_buf(),
            kind,
        };
        (temp, root)
    }

    fn claims(root: &CacheRoot) -> PathBuf {
        root.path.join(VERSION_CLAIMS_DIR)
    }

    fn retention_for(
        root: Result<&CacheRoot, &str>,
    ) -> Arc<dyn super::RootRetention + Send + Sync> {
        for_root(root, None, TrustedOwners::none(), ports())
    }

    #[test]
    fn the_plugin_bin_retention_claims_nothing() {
        let (_temp, root) = root_of(CacheRootKind::PluginBin);

        PluginBinRetention::new(root.path.clone(), ports())
            .claim_running_version();

        assert!(!claims(&root).exists());
    }

    #[test]
    fn an_unidentifiable_root_is_never_claimed() {
        let (_temp, relocated) = root_of(CacheRootKind::Relocated);

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
            PruneOutcome::Unavailable {
                detail: "unidentified".to_owned()
            }
        );
    }

    #[test]
    fn for_root_chooses_the_retention_from_the_root_kind() {
        let (_plugin_temp, plugin_bin) = root_of(CacheRootKind::PluginBin);
        let (_relocated_temp, relocated) = root_of(CacheRootKind::Relocated);

        retention_for(Ok(&plugin_bin)).claim_running_version();
        retention_for(Ok(&relocated)).claim_running_version();

        assert!(!claims(&plugin_bin).exists());
        assert!(claims(&relocated)
            .join(format!("{}.{}", env!("CARGO_PKG_VERSION"), launcher_id()))
            .is_file());
    }
}
