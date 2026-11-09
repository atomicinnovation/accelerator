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

/// Everything a prune did and could not do, gathered from each part of it.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PruneReport {
    pub removals: Removals,
    pub lock_failures: Vec<LockFailure>,
    pub unreadable_claims: Vec<UnreadableClaims>,
    pub untrusted_directories: Vec<UntrustedDirectory>,
    pub reap_failures: Vec<ReapFailure>,
    pub unlistable_directories: Vec<UnlistableDirectory>,
}

impl PruneReport {
    pub fn absorb(&mut self, other: Self) {
        self.removals.absorb(other.removals);
        self.lock_failures.extend(other.lock_failures);
        self.unreadable_claims.extend(other.unreadable_claims);
        self.untrusted_directories
            .extend(other.untrusted_directories);
        self.reap_failures.extend(other.reap_failures);
        self.unlistable_directories
            .extend(other.unlistable_directories);
    }

    /// An unreadable or untrusted directory only kept entries, so it leaves
    /// the prune finished.
    #[must_use]
    pub const fn finished(&self) -> bool {
        self.lock_failures.is_empty()
            && self.removals.failures.is_empty()
            && self.reap_failures.is_empty()
            && self.unlistable_directories.is_empty()
    }

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
        let removals = self.removals.failures.iter().map(|failure| {
            format!(
                "could not remove {}: {}",
                failure.path.display(),
                failure.kind
            )
        });
        let unlistable = self.unlistable_directories.iter().map(|directory| {
            format!(
                "could not list {}: {}",
                directory.path.display(),
                directory.kind
            )
        });
        let reaps = self.reap_failures.iter().map(|failure| {
            format!(
                "could not reap orphans under {}: {}",
                failure.root.display(),
                failure.detail
            )
        });
        locks
            .chain(claims)
            .chain(untrusted)
            .chain(unlistable)
            .chain(removals)
            .chain(reaps)
            .collect()
    }
}
