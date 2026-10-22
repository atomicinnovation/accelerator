//! Tree prune: the launcher's one reclamation of trees.
//!
//! A process holds at most one tree lock at a time, and each artifact's lock
//! is taken inside that artifact's own iteration and released before the
//! next, so no prune ever probes or retakes a tree lock it holds.

use std::collections::BTreeSet;
use std::io;
use std::path::Path;
use std::time::Duration;

use crate::launch::core::tree::{Clock, TreeError};

use super::super::filesystem::{CacheFilesystem, EntryMetadata};
use super::super::ownership::safe_root::SafePruneRoot;
use super::super::ownership::Ownership;
use super::super::prune_report::{
    Count, LockFailure, PruneStep, PruneSteps, Removal, RemovalFailure,
    Removals,
};
use super::super::retention_windows::is_aged;
use super::super::single_flight::LockPatience;
use super::claims::{self, is_stale_claim_entry, TreeClaimIndex};
use super::layout::{
    classify_trees_entry, read_pointer, validated_generation, Artifact,
    TreePaths, TreesEntry, LAYOUT_VERSION,
};
use super::lease::LivenessProbe;
use super::locks::TreeLocks;
use super::reap::{self, PinnedDigests, Reaper};
use super::tree_lock::HeldTreeLock;

/// Which artifacts a tree prune visits.
pub enum TreePruneRegime<'a> {
    /// Every artifact and platform under a relocated root's `trees/`.
    WholeTree(&'a SafePruneRoot),
}

impl TreePruneRegime<'_> {
    fn paths(&self) -> TreePaths {
        match self {
            Self::WholeTree(safe) => TreePaths::under(safe.path()),
        }
    }
}

pub struct TreePrunePorts<'a> {
    pub filesystem: &'a dyn CacheFilesystem,
    pub ownership: &'a dyn Ownership,
    pub clock: &'a dyn Clock,
    pub lease_liveness: &'a dyn LivenessProbe,
    pub pins: &'a dyn PinnedDigests,
    pub locks: TreeLocks<'a>,
    pub steps: &'a dyn PruneSteps,
}

pub struct TreePrune<'a> {
    pub regime: TreePruneRegime<'a>,
    pub claim_window: Duration,
    pub ports: TreePrunePorts<'a>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TreePruneReport {
    pub removals: Removals,
    pub lock_failures: Vec<LockFailure>,
}

enum ArtifactLock {
    Taken(HeldTreeLock),
    Skipped,
    Failed(LockFailure),
}

impl TreePrune<'_> {
    pub fn run(&self, patience: LockPatience) -> TreePruneReport {
        let mut report = TreePruneReport::default();
        let paths = self.regime.paths();
        if self
            .ports
            .filesystem
            .entry(paths.root())
            .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
        {
            return report;
        }
        let claims = match claims::list(
            &paths,
            self.ports.filesystem,
            self.ports.ownership,
        ) {
            claims::TreeClaimsListing::Read(claims) => claims,
            claims::TreeClaimsListing::Unreadable(_) => return report,
        };
        let index = match TreesIndex::read(&paths, self.ports.filesystem) {
            Ok(index) => index,
            Err(error) => {
                report.removals.failures.push(RemovalFailure {
                    path: paths.root().to_path_buf(),
                    kind: error.kind(),
                });
                return report;
            }
        };
        let mut reaped = false;
        for artifact in index.artifacts() {
            match self.lock(&paths, &artifact, &index, patience) {
                ArtifactLock::Taken(held) => {
                    self.ports.steps.reached(PruneStep::TreeLockHeld);
                    if !reaped {
                        self.reap_orphans_under(&held, &mut report);
                        reaped = true;
                    }
                    self.reclaim_unclaimed_trees(
                        &held,
                        &index,
                        &claims,
                        &mut report,
                    );
                }
                ArtifactLock::Skipped => {}
                ArtifactLock::Failed(failure) => {
                    report.lock_failures.push(failure);
                }
            }
        }
        self.sweep_stale_tree_claims(&claims, &mut report);
        report
    }

    fn lock(
        &self,
        paths: &TreePaths,
        artifact: &Artifact,
        index: &TreesIndex,
        patience: LockPatience,
    ) -> ArtifactLock {
        if !index.has_work_for(artifact) {
            return ArtifactLock::Skipped;
        }
        let taken = match patience {
            LockPatience::Wait => {
                self.ports.locks.take_tree(paths, artifact).map(Some)
            }
            LockPatience::Try => {
                self.ports.locks.try_take_tree(paths, artifact)
            }
        };
        match taken {
            Ok(Some(held)) => ArtifactLock::Taken(held),
            Ok(None) => ArtifactLock::Skipped,
            Err(error) => {
                ArtifactLock::Failed(lock_failure(paths, artifact, &error))
            }
        }
    }

    fn reap_orphans_under(
        &self,
        held: &HeldTreeLock,
        report: &mut TreePruneReport,
    ) {
        match Reaper::new(
            held,
            self.ports.locks,
            self.ports.clock,
            self.ports.lease_liveness,
            self.ports.pins,
        )
        .reap_orphans()
        {
            Ok(reaped) => report.removals.reclaimed += reaped.entries,
            Err(_) => report.removals.failures.push(RemovalFailure {
                path: held.paths().root().to_path_buf(),
                kind: io::ErrorKind::Other,
            }),
        }
    }

    /// Step 1: a pointer whose digest nothing protects goes with the
    /// generation it names. The index decides every keep; a re-read of the
    /// digest's claims guards every removal.
    fn reclaim_unclaimed_trees(
        &self,
        held: &HeldTreeLock,
        index: &TreesIndex,
        claims: &TreeClaimIndex,
        report: &mut TreePruneReport,
    ) {
        let paths = held.paths();
        let artifact = held.artifact();
        let pinned = self.ports.pins.pinned_digest(artifact);
        for (name, digest) in index.pointers_of(artifact) {
            let pointer = paths.root().join(name);
            let PointerTarget::Current(generation) = PointerTarget::of(
                read_pointer(&pointer, self.ports.filesystem),
                artifact,
                digest,
            ) else {
                continue;
            };
            if pinned.as_deref() == Some(digest)
                || self.is_protected(claims, digest)
                || !reap::unclaimed_is_reclaimable(
                    self.ports.lease_liveness.probe(&paths.lease(&generation)),
                    self.is_aged(&paths.generation(&generation)),
                )
            {
                continue;
            }
            let claimed_since = match claims::list(
                paths,
                self.ports.filesystem,
                self.ports.ownership,
            ) {
                claims::TreeClaimsListing::Read(reread)
                    if !self.is_protected(&reread, digest) =>
                {
                    reread
                }
                _ => continue,
            };
            self.remove_tree(
                paths,
                &pointer,
                &generation,
                &claimed_since,
                digest,
                report,
            );
        }
    }

    /// The pointer goes first, so a concurrent `locate` misses and
    /// re-materialises rather than leasing a generation about to go.
    fn remove_tree(
        &self,
        paths: &TreePaths,
        pointer: &Path,
        generation: &str,
        claims: &TreeClaimIndex,
        digest: &str,
        report: &mut TreePruneReport,
    ) {
        let removals = &mut report.removals;
        let filesystem = self.ports.filesystem;
        if removals.remove_file(
            filesystem,
            pointer,
            "unclaimed tree",
            Count::WithItsEntry,
        ) == Removal::Failed
        {
            return;
        }
        removals.remove_tree(
            filesystem,
            &paths.generation(generation),
            "unclaimed tree",
            Count::Once,
        );
        for sidecar in reap::sidecars_of(paths, generation) {
            removals.remove_file(
                filesystem,
                &sidecar,
                "unclaimed tree",
                Count::WithItsEntry,
            );
        }
        for claim in claims.on(digest) {
            removals.remove_file(
                filesystem,
                &claim.path,
                "claim on an unclaimed tree",
                Count::WithItsEntry,
            );
        }
    }

    /// Each stale claim is re-read immediately before its unlink, so one
    /// refreshed since the index was read is kept.
    fn sweep_stale_tree_claims(
        &self,
        claims: &TreeClaimIndex,
        report: &mut TreePruneReport,
    ) {
        let now = self.ports.clock.now_seconds();
        let uid = self.ports.ownership.effective_uid();
        let is_stale = |entry: &EntryMetadata| {
            is_stale_claim_entry(entry, self.claim_window, now, uid)
        };
        for claim in claims
            .claims()
            .iter()
            .filter(|claim| claim.sweepable && is_stale(&claim.entry))
        {
            if self
                .ports
                .filesystem
                .entry(&claim.path)
                .is_ok_and(|entry| is_stale(&entry))
            {
                report.removals.remove_file(
                    self.ports.filesystem,
                    &claim.path,
                    "stale tree claim",
                    Count::Once,
                );
            }
        }
    }

    fn is_protected(&self, claims: &TreeClaimIndex, digest: &str) -> bool {
        claims.protects(
            digest,
            self.claim_window,
            self.ports.clock.now_seconds(),
            self.ports.ownership.effective_uid(),
        )
    }

    fn is_aged(&self, path: &Path) -> bool {
        is_aged(
            self.ports
                .filesystem
                .entry(path)
                .ok()
                .and_then(|entry| entry.modified_seconds),
            self.ports.clock.now_seconds(),
        )
    }
}

fn lock_failure(
    paths: &TreePaths,
    artifact: &Artifact,
    error: &TreeError,
) -> LockFailure {
    LockFailure {
        lock: paths.single_flight_lock(artifact),
        detail: error.to_string(),
    }
}

/// `trees/` listed once, so every step judges the same names.
struct TreesIndex {
    names: Vec<String>,
}

impl TreesIndex {
    /// A name that is not UTF-8 is no name the launcher writes, so it is left
    /// out and kept.
    fn read(
        paths: &TreePaths,
        filesystem: &dyn CacheFilesystem,
    ) -> io::Result<Self> {
        let mut names: Vec<String> = filesystem
            .list(paths.root())?
            .into_iter()
            .filter_map(|name| name.into_string().ok())
            .collect();
        names.sort();
        Ok(Self { names })
    }

    fn entries(&self) -> impl Iterator<Item = (&str, TreesEntry<'_>)> {
        self.names
            .iter()
            .map(|name| (name.as_str(), classify_trees_entry(name)))
    }

    fn artifacts(&self) -> BTreeSet<Artifact> {
        self.entries()
            .filter_map(|(_, entry)| match entry {
                TreesEntry::Pointer { artifact, .. }
                | TreesEntry::Sidecar { artifact, .. }
                | TreesEntry::TempResidue { artifact }
                | TreesEntry::Lock { artifact } => Some(artifact),
                TreesEntry::Generation(generation) => Some(generation.artifact),
                TreesEntry::Unrecognised => None,
            })
            .collect()
    }

    /// A pointer, a generation or temp residue: something a materialise of
    /// the artifact left, so taking its lock creates none it never had.
    fn has_work_for(&self, artifact: &Artifact) -> bool {
        self.entries().any(|(_, entry)| match entry {
            TreesEntry::Pointer {
                artifact: owner, ..
            }
            | TreesEntry::TempResidue { artifact: owner } => &owner == artifact,
            TreesEntry::Generation(generation) => {
                &generation.artifact == artifact
            }
            _ => false,
        })
    }

    fn pointers_of<'a>(
        &'a self,
        artifact: &'a Artifact,
    ) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.entries().filter_map(move |(name, entry)| match entry {
            TreesEntry::Pointer {
                artifact: owner,
                digest,
            } if &owner == artifact => Some((name, digest)),
            _ => None,
        })
    }
}

/// What a pointer's contents name, judged by the grammar `locate` holds a
/// pointer to before any of it becomes a path.
pub(super) enum PointerTarget {
    Current(String),
    OtherLayout,
    Malformed,
}

impl PointerTarget {
    pub(super) fn of(
        contents: Option<String>,
        artifact: &Artifact,
        digest: &str,
    ) -> Self {
        let Some(contents) = contents else {
            return Self::Malformed;
        };
        match validated_generation(
            &contents,
            artifact.name(),
            artifact.platform(),
            digest,
            LAYOUT_VERSION,
        ) {
            Ok(generation) => Self::Current(generation),
            Err(TreeError::LayoutUnsupported { .. }) => Self::OtherLayout,
            Err(_) => Self::Malformed,
        }
    }
}
