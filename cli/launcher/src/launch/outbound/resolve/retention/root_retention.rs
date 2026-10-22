//! The retention regime governing a cache root, chosen once from its kind.

use std::path::PathBuf;
use std::sync::Arc;

use super::super::cache_root::{CacheRoot, CacheRootKind};
use super::launcher_id;
use super::ports::CachePorts;
use super::version_claims::{self, VersionClaim};

pub trait RootRetention {
    /// Declare that the running version still uses this root.
    fn claim_running_version(&self);
}

/// The plugin's own `bin`, which ADR-0063 governs: nothing is claimed.
pub struct PluginBinRetention;

impl RootRetention for PluginBinRetention {
    fn claim_running_version(&self) {}
}

/// A root shared across plugin versions, which each version claims.
pub struct RelocatedRetention {
    root: PathBuf,
    running_version: String,
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
}

/// The retention for `root`; a root that could not be identified is never
/// claimed.
#[must_use]
pub fn for_root(
    root: Option<&CacheRoot>,
    ports: Arc<CachePorts>,
) -> Arc<dyn RootRetention + Send + Sync> {
    match root {
        Some(CacheRoot {
            path,
            kind: CacheRootKind::Relocated,
        }) => Arc::new(RelocatedRetention {
            root: path.clone(),
            running_version: env!("CARGO_PKG_VERSION").to_owned(),
            ports,
        }),
        Some(CacheRoot {
            kind: CacheRootKind::PluginBin,
            ..
        })
        | None => Arc::new(PluginBinRetention),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::launch::core::tree::Clock;

    use super::super::super::cache_root::{CacheRoot, CacheRootKind};
    use super::super::super::filesystem::SystemFilesystem;
    use super::super::super::ownership::ProcessOwnership;
    use super::super::super::prune_report::NoPruneSteps;
    use super::super::super::tree::locks::FlockLocks;
    use super::super::launcher_id;
    use super::super::ports::CachePorts;
    use super::super::residue::KillZeroProbe;
    use super::super::version_claims::VERSION_CLAIMS_DIR;
    use super::{for_root, PluginBinRetention, RootRetention as _};

    struct AtClock(u64);

    impl Clock for AtClock {
        fn now_seconds(&self) -> u64 {
            self.0
        }

        fn sleep_poll_interval(&self) {}
    }

    fn ports() -> Arc<CachePorts> {
        Arc::new(CachePorts {
            filesystem: Arc::new(SystemFilesystem),
            ownership: Arc::new(ProcessOwnership::default()),
            clock: Arc::new(AtClock(1_000_000)),
            locks: Arc::new(FlockLocks),
            steps: Arc::new(NoPruneSteps),
            bootstrap_holders: Arc::new(KillZeroProbe),
        })
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

    #[test]
    fn the_plugin_bin_retention_claims_nothing() {
        let (_temp, root) = root_of(CacheRootKind::PluginBin);

        PluginBinRetention.claim_running_version();

        assert!(!claims(&root).exists());
    }

    #[test]
    fn an_unidentifiable_root_selects_the_plugin_bin_retention() {
        let (_temp, relocated) = root_of(CacheRootKind::Relocated);

        for_root(None, ports()).claim_running_version();
        for_root(Some(&relocated), ports()).claim_running_version();

        assert_eq!(
            std::fs::read_dir(claims(&relocated))
                .expect("claims")
                .count(),
            1,
            "only the identified root is claimed"
        );
    }

    #[test]
    fn for_root_chooses_the_retention_from_the_root_kind() {
        let (_plugin_temp, plugin_bin) = root_of(CacheRootKind::PluginBin);
        let (_relocated_temp, relocated) = root_of(CacheRootKind::Relocated);

        for_root(Some(&plugin_bin), ports()).claim_running_version();
        for_root(Some(&relocated), ports()).claim_running_version();

        assert!(!claims(&plugin_bin).exists());
        assert!(claims(&relocated)
            .join(format!("{}.{}", env!("CARGO_PKG_VERSION"), launcher_id()))
            .is_file());
    }
}
