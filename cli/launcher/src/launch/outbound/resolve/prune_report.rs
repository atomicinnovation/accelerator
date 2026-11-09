//! What a prune reports, shared by single-file and tree prune so neither
//! imports the other.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::launch::core::tree::Clock;

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

/// How long a prune may go on starting removals.
///
/// A cold path runs before its dispatch, so it stops starting them once its
/// allowance is spent and leaves the rest to a later prune; a removal already
/// started finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PruneBudget {
    Unbounded,
    Until(u64),
}

impl PruneBudget {
    #[must_use]
    pub fn starting_now(clock: &dyn Clock, allowance: Duration) -> Self {
        Self::Until(clock.now_seconds().saturating_add(allowance.as_secs()))
    }

    #[must_use]
    pub fn allows_more(self, clock: &dyn Clock) -> bool {
        match self {
            Self::Unbounded => true,
            Self::Until(deadline) => clock.now_seconds() < deadline,
        }
    }
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

/// A directory whose listing failed, so nothing in it was pruned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlistableDirectory {
    pub path: PathBuf,
    pub kind: io::ErrorKind,
}

/// A reap that could not start, so it removed none of the orphans under
/// `root`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReapFailure {
    pub root: PathBuf,
    pub detail: String,
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
