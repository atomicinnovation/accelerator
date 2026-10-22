//! A cache root in a temporary directory, seeded and back-dated, and the
//! ports a prune over it runs with.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tempfile::TempDir;

use crate::launch::core::tree::Clock;

use super::super::filesystem::{CacheFilesystem, SystemFilesystem};
use super::super::ownership::{Ownership, ProcessOwnership};
use super::super::prune_report::{NoPruneSteps, PruneSteps};
use super::super::retention_windows::AGE_BACKSTOP;
use super::super::single_flight::SingleFlightLocks;
use super::super::tree::lease::{FlockProbe, LivenessProbe};
use super::super::tree::locks::FlockLocks;
use super::super::tree::reap::PinnedDigests;
use super::super::tree::ExpectedDigests;
use super::ports::CachePorts;
use super::residue::BootstrapHolderLiveness;

pub struct AtClock(pub u64);

impl Clock for AtClock {
    fn now_seconds(&self) -> u64 {
        self.0
    }

    fn sleep_poll_interval(&self) {}
}

#[derive(Default)]
pub struct LiveHolders(pub BTreeSet<i32>);

impl BootstrapHolderLiveness for LiveHolders {
    fn is_alive(&self, pid: i32) -> bool {
        self.0.contains(&pid)
    }
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs()
}

pub const fn aged(now: u64) -> u64 {
    now - AGE_BACKSTOP.as_secs()
}

pub const fn young(now: u64) -> u64 {
    now - AGE_BACKSTOP.as_secs() + 60
}

/// A canonical, `0700` cache root, so the host's temp-path ancestry and
/// umask decide nothing.
pub struct Root {
    _temp: TempDir,
    path: PathBuf,
}

impl Root {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().canonicalize().expect("canonical root");
        set_mode(&path, 0o700);
        Self { _temp: temp, path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    pub fn exists(&self, name: &str) -> bool {
        self.join(name).symlink_metadata().is_ok()
    }

    pub fn file(&self, name: &str, bytes: &[u8], modified: u64) -> PathBuf {
        let path = self.join(name);
        std::fs::write(&path, bytes).expect("seeded file");
        set_mtime(&path, modified);
        path
    }

    pub fn dir(&self, name: &str, mode: u32) -> PathBuf {
        let path = self.join(name);
        std::fs::create_dir(&path).expect("seeded directory");
        set_mode(&path, mode);
        path
    }
}

pub fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .expect("mode");
}

/// Works on a directory too, unlike the port's `set_modified`.
pub fn set_mtime(path: &Path, seconds: u64) {
    std::fs::File::open(path)
        .expect("opened to set its mtime")
        .set_modified(
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds),
        )
        .expect("mtime");
}

pub struct Ports {
    pub filesystem: Arc<dyn CacheFilesystem + Send + Sync>,
    pub ownership: Arc<dyn Ownership + Send + Sync>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub locks: Arc<dyn SingleFlightLocks + Send + Sync>,
    pub steps: Arc<dyn PruneSteps + Send + Sync>,
    pub bootstrap_holders: Arc<dyn BootstrapHolderLiveness + Send + Sync>,
    pub lease_liveness: Arc<dyn LivenessProbe + Send + Sync>,
    pub pins: Arc<dyn PinnedDigests + Send + Sync>,
}

impl Ports {
    pub fn at(now: u64) -> Self {
        Self {
            filesystem: Arc::new(SystemFilesystem),
            ownership: Arc::new(ProcessOwnership::default()),
            clock: Arc::new(AtClock(now)),
            locks: Arc::new(FlockLocks),
            steps: Arc::new(NoPruneSteps),
            bootstrap_holders: Arc::new(LiveHolders::default()),
            lease_liveness: Arc::new(FlockProbe),
            pins: Arc::new(ExpectedDigests::Compiled),
        }
    }

    pub fn build(self) -> CachePorts {
        CachePorts {
            filesystem: self.filesystem,
            ownership: self.ownership,
            clock: self.clock,
            locks: self.locks,
            steps: self.steps,
            bootstrap_holders: self.bootstrap_holders,
            lease_liveness: self.lease_liveness,
            pins: self.pins,
        }
    }
}
