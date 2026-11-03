//! What a prune reports, shared by single-file and tree prune so neither
//! imports the other.

use std::io;
use std::path::{Path, PathBuf};

use super::filesystem::CacheFilesystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PruneStep {
    AboutToList,
    Listed,
    RootLockHeld,
    TreeLockHeld,
}

/// Sees a prune reach each step; a test pauses it there.
pub trait PruneSteps {
    fn reached(&self, step: PruneStep);
}

pub struct NoPruneSteps;

impl PruneSteps for NoPruneSteps {
    fn reached(&self, _step: PruneStep) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockFailure {
    pub lock: PathBuf,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableClaims {
    pub directory: PathBuf,
    pub kind: io::ErrorKind,
}

/// A directory another user could rename into, so a prune went no further
/// through it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UntrustedDirectory {
    pub path: PathBuf,
}

/// A removal that failed for any reason but the entry already being gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalFailure {
    pub path: PathBuf,
    pub kind: io::ErrorKind,
}

/// Whether a removal adds to the reclaimed count: an entry's signature, a
/// lock directory's pid file and a generation's sidecars go uncounted with
/// what they belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Count {
    Once,
    WithItsEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    Removed,
    AlreadyGone,
    Failed,
}

impl Removal {
    #[must_use]
    pub const fn left_nothing(self) -> bool {
        matches!(self, Self::Removed | Self::AlreadyGone)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Removals {
    pub reclaimed: u64,
    pub failures: Vec<RemovalFailure>,
}

impl Removals {
    pub fn remove_file(
        &mut self,
        filesystem: &dyn CacheFilesystem,
        path: &Path,
        rule: &str,
        count: Count,
    ) -> Removal {
        tracing::debug!(path = %path.display(), rule, "removing");
        self.tally(path, filesystem.remove_file(path), count)
    }

    pub fn remove_dir(
        &mut self,
        filesystem: &dyn CacheFilesystem,
        path: &Path,
        rule: &str,
        count: Count,
    ) -> Removal {
        tracing::debug!(path = %path.display(), rule, "removing");
        self.tally(path, filesystem.remove_dir(path), count)
    }

    pub fn remove_tree(
        &mut self,
        filesystem: &dyn CacheFilesystem,
        path: &Path,
        rule: &str,
        count: Count,
    ) -> Removal {
        tracing::debug!(path = %path.display(), rule, "removing");
        self.tally(path, filesystem.remove_tree(path), count)
    }

    pub fn absorb(&mut self, other: Self) {
        self.reclaimed += other.reclaimed;
        self.failures.extend(other.failures);
    }

    fn tally(
        &mut self,
        path: &Path,
        outcome: io::Result<()>,
        count: Count,
    ) -> Removal {
        match outcome {
            Ok(()) => {
                if count == Count::Once {
                    self.reclaimed += 1;
                }
                Removal::Removed
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Removal::AlreadyGone
            }
            Err(error) => {
                self.failures.push(RemovalFailure {
                    path: path.to_path_buf(),
                    kind: error.kind(),
                });
                Removal::Failed
            }
        }
    }
}
