//! Planting trees, pointers, sidecars, locks and claims under a cache root's
//! `trees/`, and the fixed clock and lease liveness a prune of them runs with.

use std::path::{Path, PathBuf};
use std::time::Duration;

use accelerator::launch::core::tree::{Clock, TreeError};
use accelerator::launch::outbound::resolve::single_flight::{
    LockGuard, LockProbe, SingleFlightLocks,
};
use accelerator::launch::outbound::resolve::tree::layout::{
    generation_name, Artifact, TreePaths, LAYOUT_VERSION,
};
use accelerator::launch::outbound::resolve::tree::lease::{
    take_single_flight, Liveness, LivenessProbe,
};
use accelerator::launch::outbound::resolve::tree::locks::FlockLocks;
use accelerator::launch::outbound::resolve::HOST_PLATFORM;
use accelerator::launch::outbound::resolve::KNOWN_PLATFORMS;

pub const SUFFIX: &str = "0011223344556677";

pub struct FixedClock(pub u64);

impl Clock for FixedClock {
    fn now_seconds(&self) -> u64 {
        self.0
    }

    fn sleep_poll_interval(&self) {}
}

/// Answers every probe the same way, so `Held` and `Unknown` can be exercised
/// without a live holder or a filesystem that lacks `flock`.
pub struct FixedLiveness(pub Liveness);

impl LivenessProbe for FixedLiveness {
    fn probe(&self, _lease: &Path) -> Liveness {
        self.0
    }
}

/// Fails every lock call on one path and takes every other for real.
pub struct FailingLockAt(pub PathBuf);

impl FailingLockAt {
    fn fails(&self, path: &Path) -> Result<(), TreeError> {
        if path == self.0 {
            return Err(TreeError::Lease {
                detail: "No locks available (os error 77)".to_owned(),
            });
        }
        Ok(())
    }
}

impl SingleFlightLocks for FailingLockAt {
    fn take(&self, path: &Path) -> Result<LockGuard, TreeError> {
        self.fails(path)?;
        FlockLocks.take(path)
    }

    fn try_take(&self, path: &Path) -> Result<Option<LockGuard>, TreeError> {
        self.fails(path)?;
        FlockLocks.try_take(path)
    }

    fn wait_existing(
        &self,
        path: &Path,
    ) -> Result<Option<LockGuard>, TreeError> {
        self.fails(path)?;
        FlockLocks.wait_existing(path)
    }

    fn probe(&self, path: &Path) -> Result<LockProbe, TreeError> {
        self.fails(path)?;
        FlockLocks.probe(path)
    }
}

pub fn on_host(name: &str) -> Artifact {
    Artifact::new(name, HOST_PLATFORM)
}

/// A known platform this host is not.
pub fn another_platform() -> &'static str {
    KNOWN_PLATFORMS
        .iter()
        .find(|platform| **platform != HOST_PLATFORM)
        .expect("a second known platform")
}

pub fn digest(seed: char) -> String {
    seed.to_string().repeat(64)
}

pub fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .expect("mode");
}

/// Works on a directory too.
pub fn set_mtime(path: &Path, seconds: u64) {
    std::fs::File::open(path)
        .expect("opened to set its mtime")
        .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(seconds))
        .expect("mtime");
}

pub fn exists(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

/// A published tree: its pointer, generation directory and sidecars.
pub struct PlantedTree {
    pub pointer: PathBuf,
    pub generation: PathBuf,
    pub sidecars: [PathBuf; 3],
}

impl PlantedTree {
    fn parts(&self) -> impl Iterator<Item = &PathBuf> {
        [&self.pointer, &self.generation]
            .into_iter()
            .chain(&self.sidecars)
    }

    #[track_caller]
    pub fn assert_survives(&self) {
        for part in self.parts() {
            assert!(exists(part), "{} was removed", part.display());
        }
    }

    #[track_caller]
    pub fn assert_reclaimed(&self) {
        for part in self.parts() {
            assert!(!exists(part), "{} survived", part.display());
        }
    }
}

/// The `trees/` of one cache root.
pub struct Trees {
    paths: TreePaths,
}

impl Trees {
    /// Creates `trees/` at `0700`, as a materialise does.
    pub fn under(cache_root: &Path) -> Self {
        let paths = TreePaths::under(cache_root);
        if !exists(paths.root()) {
            std::fs::create_dir(paths.root()).expect("trees");
            set_mode(paths.root(), 0o700);
        }
        Self { paths }
    }

    pub const fn paths(&self) -> &TreePaths {
        &self.paths
    }

    pub fn root(&self) -> &Path {
        self.paths.root()
    }

    pub fn generation_of(artifact: &Artifact, digest: &str) -> String {
        Self::generation_at_layout(artifact, digest, LAYOUT_VERSION)
    }

    pub fn generation_at_layout(
        artifact: &Artifact,
        digest: &str,
        layout: u32,
    ) -> String {
        generation_name(
            artifact.name(),
            artifact.platform(),
            digest,
            layout,
            SUFFIX,
        )
    }

    /// A pointer, the generation it names and its sidecars, all modified at
    /// `modified`.
    pub fn plant_tree(
        &self,
        artifact: &Artifact,
        digest: &str,
        modified: u64,
    ) -> PlantedTree {
        let generation = Self::generation_of(artifact, digest);
        let pointer = self.plant_pointer(artifact, digest, &generation);
        set_mtime(&pointer, modified);
        PlantedTree {
            pointer,
            sidecars: self.plant_sidecars(&generation, modified),
            generation: self.plant_generation(&generation, modified),
        }
    }

    pub fn plant_generation(&self, generation: &str, modified: u64) -> PathBuf {
        let path = self.paths.generation(generation);
        std::fs::create_dir(&path).expect("generation");
        set_mtime(&path, modified);
        path
    }

    pub fn plant_pointer(
        &self,
        artifact: &Artifact,
        digest: &str,
        contents: impl AsRef<[u8]>,
    ) -> PathBuf {
        let pointer =
            self.paths
                .pointer(artifact.name(), artifact.platform(), digest);
        std::fs::write(&pointer, contents).expect("pointer");
        pointer
    }

    pub fn sidecars_of(&self, generation: &str) -> [PathBuf; 3] {
        [
            self.paths.attestation(generation),
            self.paths.attestation_signature(generation),
            self.paths.lease(generation),
        ]
    }

    pub fn plant_sidecars(
        &self,
        generation: &str,
        modified: u64,
    ) -> [PathBuf; 3] {
        let sidecars = self.sidecars_of(generation);
        for sidecar in &sidecars {
            std::fs::write(sidecar, b"").expect("sidecar");
            set_mtime(sidecar, modified);
        }
        sidecars
    }

    /// An artifact's tree lock file, left free.
    pub fn plant_lock(&self, artifact: &Artifact, modified: u64) -> PathBuf {
        let lock = self.paths.single_flight_lock(artifact);
        drop(take_single_flight(&lock).expect("lock file"));
        set_mtime(&lock, modified);
        lock
    }

    pub fn plant_temp_generation(
        &self,
        artifact: &Artifact,
        digest: &str,
        modified: u64,
    ) -> PathBuf {
        let path = self
            .paths
            .temp_generation(&Self::generation_of(artifact, digest));
        std::fs::create_dir(&path).expect("temp generation");
        set_mtime(&path, modified);
        path
    }

    pub fn plant_partial_archive(
        &self,
        artifact: &Artifact,
        digest: &str,
        modified: u64,
    ) -> PathBuf {
        let path = self.paths.temp_archive(artifact, digest);
        std::fs::write(&path, b"partial").expect("partial archive");
        set_mtime(&path, modified);
        path
    }

    pub fn claims(&self) -> PathBuf {
        self.paths.claims()
    }

    pub fn plant_claim(
        &self,
        digest: &str,
        launcher_id: &str,
        modified: u64,
    ) -> PathBuf {
        if !exists(&self.claims()) {
            std::fs::create_dir(self.claims()).expect("tree claims");
            set_mode(&self.claims(), 0o700);
        }
        let claim = self.claims().join(format!("{digest}.{launcher_id}"));
        std::fs::write(&claim, b"").expect("tree claim");
        set_mtime(&claim, modified);
        claim
    }
}
