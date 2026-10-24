//! A process holds at most one tree lock at a time, and each artifact's lock
//! is taken inside that artifact's own iteration and released before the
//! next, so no prune ever probes or retakes a tree lock it holds.
//!
//! Unlinking a lock file this prune holds is safe only while every launcher
//! confirms its lock's inode after taking it: one that does not can go on
//! holding the unlinked inode while another takes a fresh file, and neither
//! then excludes the other. A held tree lock means a materialise or prune of
//! that artifact is in progress.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::launch::core::tree::{Clock, TreeError};

use super::super::cache_root::PluginBinRoot;
use super::super::filesystem::{CacheFilesystem, EntryMetadata};
use super::super::ownership::safe_root::SafePruneRoot;
use super::super::ownership::{is_trusted_directory, Ownership};
use super::super::prune_report::{
    Count, LockFailure, PruneStep, PruneSteps, Removal, RemovalFailure,
    Removals, UnreadableClaims, UntrustedDirectory,
};
use super::super::retention_windows::is_aged;
use super::super::single_flight::LockPatience;
use super::super::HOST_PLATFORM;
use super::after_materialise::{ReapPermit, RootTrust};
use super::claims::{self, is_stale_claim_entry, TreeClaimIndex};
use super::layout::{
    classify_trees_entry, read_pointer, validated_generation, Artifact,
    TreePaths, TreesEntry, LAYOUT_VERSION,
};
use super::lease::{
    try_exclusive_lease, HeldLeases, HeldLeasesFirst, LivenessProbe,
};
use super::locks::{TreeLockProbe, TreeLocks};
use super::pins;
use super::reap::{self, PinnedDigests, Reaper, RemovalObserver};
use super::tree_lock::HeldTreeLock;

/// Which artifacts a tree prune visits, and whether it sweeps the
/// bookkeeping a materialise leaves: orphan sidecars, orphan tree locks and
/// stale tree claims.
pub enum TreePruneRegime<'a> {
    /// The pinned artifacts on this host, and no bookkeeping.
    PinnedOnHost(&'a PluginBinRoot),
    /// Every artifact and platform under a relocated root's `trees/`.
    WholeTree(&'a SafePruneRoot),
}

impl TreePruneRegime<'_> {
    fn paths(&self) -> TreePaths {
        match self {
            Self::PinnedOnHost(root) => TreePaths::under(root.path()),
            Self::WholeTree(safe) => TreePaths::under(safe.path()),
        }
    }

    fn artifacts(&self, index: &TreesIndex) -> BTreeSet<Artifact> {
        match self {
            Self::PinnedOnHost(_) => pinned_on_host().collect(),
            Self::WholeTree(_) => index.artifacts(),
        }
    }

    const fn trust(&self) -> RootTrust<'_> {
        match self {
            Self::PinnedOnHost(root) => RootTrust::PluginBin(root),
            Self::WholeTree(safe) => RootTrust::Relocated(safe),
        }
    }

    const fn sweeps_bookkeeping(&self) -> bool {
        matches!(self, Self::WholeTree(_))
    }
}

fn pinned_on_host() -> impl Iterator<Item = Artifact> {
    pins::artifact_names()
        .into_iter()
        .map(|name| Artifact::new(name, HOST_PLATFORM))
}

pub struct TreePrunePorts<'a> {
    pub held_leases: &'a HeldLeases,
    pub filesystem: &'a dyn CacheFilesystem,
    pub ownership: &'a dyn Ownership,
    pub clock: &'a dyn Clock,
    pub lease_liveness: &'a dyn LivenessProbe,
    pub pins: &'a dyn PinnedDigests,
    pub locks: TreeLocks<'a>,
    pub steps: &'a dyn PruneSteps,
    pub removals: &'a dyn RemovalObserver,
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
    pub unreadable_claims: Option<UnreadableClaims>,
    pub untrusted_directories: Vec<UntrustedDirectory>,
}

enum ArtifactLock {
    Taken(HeldTreeLock),
    LockFree,
    Skipped,
    Failed(LockFailure),
}

impl TreePrune<'_> {
    pub fn run(&self, patience: LockPatience) -> TreePruneReport {
        let mut report = TreePruneReport::default();
        let paths = self.regime.paths();
        let Some((claims, index)) = self.survey(&paths, &mut report) else {
            return report;
        };
        let mut reaped = false;
        for artifact in self.regime.artifacts(&index) {
            match self.lock(&paths, &artifact, &index, patience) {
                ArtifactLock::Taken(held) => {
                    self.ports.steps.reached(PruneStep::TreeLockHeld);
                    if !reaped {
                        self.reap_orphans_under(&held, &index, &mut report);
                        reaped = true;
                    }
                    if let Some(claims) = &claims {
                        self.reclaim_unclaimed_trees(
                            &held,
                            &index,
                            claims,
                            &mut report,
                        );
                    }
                    if self.regime.sweeps_bookkeeping() {
                        self.reclaim_orphan_sidecars(
                            &paths,
                            &artifact,
                            &index,
                            &mut report,
                        );
                        self.reclaim_orphan_lock(held, &index, &mut report);
                    }
                }
                ArtifactLock::LockFree => self.reclaim_orphan_sidecars(
                    &paths,
                    &artifact,
                    &index,
                    &mut report,
                ),
                ArtifactLock::Skipped => {}
                ArtifactLock::Failed(failure) => {
                    report.lock_failures.push(failure);
                }
            }
        }
        if !reaped && index.holds_orphan_residue(self.ports.filesystem) {
            self.reap_under_the_first_pinned_artifact(
                &paths,
                &index,
                patience,
                &mut report,
            );
        }
        if let Some(claims) =
            claims.filter(|_| self.regime.sweeps_bookkeeping())
        {
            self.sweep_stale_tree_claims(&claims, &mut report);
        }
        report
    }

    /// No artifact's own lock reached the residue, as when it belongs only to
    /// artifacts this regime does not visit.
    fn reap_under_the_first_pinned_artifact(
        &self,
        paths: &TreePaths,
        index: &TreesIndex,
        patience: LockPatience,
        report: &mut TreePruneReport,
    ) {
        let Some(first) = pinned_on_host().next() else {
            return;
        };
        match self.take(paths, &first, patience) {
            ArtifactLock::Taken(held) => {
                self.ports.steps.reached(PruneStep::TreeLockHeld);
                self.reap_orphans_under(&held, index, report);
            }
            ArtifactLock::Failed(failure) => report.lock_failures.push(failure),
            ArtifactLock::LockFree | ArtifactLock::Skipped => {}
        }
    }

    /// Called after the caller reaped under `held`; no other tree lock is
    /// taken, since a process must never probe or retake one it holds.
    pub fn prune_held(&self, held: &HeldTreeLock) -> TreePruneReport {
        let mut report = TreePruneReport::default();
        let paths = self.regime.paths();
        if held.paths().root() != paths.root() {
            return report;
        }
        let Some((claims, index)) = self.survey(&paths, &mut report) else {
            return report;
        };
        if let Some(claims) = &claims {
            self.reclaim_unclaimed_trees(held, &index, claims, &mut report);
        }
        if self.regime.sweeps_bookkeeping() {
            self.reclaim_orphan_sidecars(
                &paths,
                held.artifact(),
                &index,
                &mut report,
            );
        }
        report
    }

    fn survey(
        &self,
        paths: &TreePaths,
        report: &mut TreePruneReport,
    ) -> Option<(Option<TreeClaimIndex>, TreesIndex)> {
        if self
            .ports
            .filesystem
            .entry(paths.root())
            .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
        {
            return None;
        }
        if !is_trusted_directory(
            paths.root(),
            self.ports.filesystem,
            self.ports.ownership,
        ) {
            report.untrusted_directories.push(UntrustedDirectory {
                path: paths.root().to_path_buf(),
            });
            return None;
        }
        let claims = self.read_claims(paths, report);
        match TreesIndex::read(paths, self.ports.filesystem) {
            Ok(index) => Some((claims, index)),
            Err(error) => {
                report.removals.failures.push(RemovalFailure {
                    path: paths.root().to_path_buf(),
                    kind: error.kind(),
                });
                None
            }
        }
    }

    /// Claims that cannot be read or trusted protect nothing and are swept
    /// by nothing, so every step that depends on them is skipped.
    fn read_claims(
        &self,
        paths: &TreePaths,
        report: &mut TreePruneReport,
    ) -> Option<TreeClaimIndex> {
        match claims::list(paths, self.ports.filesystem, self.ports.ownership) {
            claims::TreeClaimsListing::Read(claims) => Some(claims),
            claims::TreeClaimsListing::Unreadable(kind) => {
                report.unreadable_claims = Some(UnreadableClaims {
                    directory: paths.claims(),
                    kind,
                });
                None
            }
            claims::TreeClaimsListing::Untrusted => {
                report.untrusted_directories.push(UntrustedDirectory {
                    path: paths.claims(),
                });
                None
            }
        }
    }

    fn lock(
        &self,
        paths: &TreePaths,
        artifact: &Artifact,
        index: &TreesIndex,
        patience: LockPatience,
    ) -> ArtifactLock {
        if index.has_work_for(artifact) {
            return self.take(paths, artifact, patience);
        }
        if !self.regime.sweeps_bookkeeping() {
            return ArtifactLock::Skipped;
        }
        if index.has_lock_of(artifact) {
            return self.take_existing(paths, artifact, patience);
        }
        if index.sidecars_of(artifact).is_empty() {
            ArtifactLock::Skipped
        } else {
            ArtifactLock::LockFree
        }
    }

    /// Taking the lock creates it if missing, which only an artifact a
    /// materialise left something of may do.
    fn take(
        &self,
        paths: &TreePaths,
        artifact: &Artifact,
        patience: LockPatience,
    ) -> ArtifactLock {
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

    /// An artifact with only bookkeeping left never has its lock created.
    fn take_existing(
        &self,
        paths: &TreePaths,
        artifact: &Artifact,
        patience: LockPatience,
    ) -> ArtifactLock {
        let locks = self.ports.locks;
        let taken = match patience {
            LockPatience::Wait => {
                locks.wait_existing_tree(paths, artifact).map(|held| {
                    held.map_or(ArtifactLock::LockFree, ArtifactLock::Taken)
                })
            }
            LockPatience::Try => {
                locks
                    .probe_tree(paths, artifact)
                    .map(|probed| match probed {
                        TreeLockProbe::Taken(held) => ArtifactLock::Taken(held),
                        TreeLockProbe::Held => ArtifactLock::Skipped,
                        TreeLockProbe::Absent => ArtifactLock::LockFree,
                    })
            }
        };
        taken.unwrap_or_else(|error| {
            ArtifactLock::Failed(lock_failure(paths, artifact, &error))
        })
    }

    fn reap_orphans_under(
        &self,
        held: &HeldTreeLock,
        index: &TreesIndex,
        report: &mut TreePruneReport,
    ) {
        let Some(permit) = ReapPermit::for_root(
            self.regime.trust(),
            self.ports.filesystem,
            self.ports.ownership,
        ) else {
            return;
        };
        let liveness = self.liveness();
        match Reaper::new(
            held,
            self.ports.locks,
            self.ports.clock,
            &liveness,
            self.ports.pins,
        )
        .observed_by(&Announcing {
            index,
            observer: self.ports.removals,
        })
        .reap_orphans(&permit)
        {
            Ok(reaped) => report.removals.reclaimed += reaped.entries,
            Err(_) => report.removals.failures.push(RemovalFailure {
                path: held.paths().root().to_path_buf(),
                kind: io::ErrorKind::Other,
            }),
        }
    }

    /// The index decides every keep; a re-read of the digest's claims guards
    /// every removal.
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
                    self.liveness().probe(&paths.lease(&generation)),
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
                &UnclaimedTree {
                    paths,
                    pointer,
                    generation,
                    digest,
                },
                &claimed_since,
                index,
                report,
            );
        }
    }

    fn reclaim_orphan_sidecars(
        &self,
        paths: &TreePaths,
        artifact: &Artifact,
        index: &TreesIndex,
        report: &mut TreePruneReport,
    ) {
        for (name, generation) in index.sidecars_of(artifact) {
            if index.holds(&generation) {
                continue;
            }
            let sidecar = paths.root().join(&name);
            if reap::orphan_is_reclaimable(
                self.liveness().probe(&paths.lease(&generation)),
                self.is_aged(&sidecar),
            ) && self
                .unlink_file(
                    &mut report.removals,
                    &sidecar,
                    "orphan sidecar",
                    Count::Once,
                )
                .left_nothing()
            {
                index.forget(&sidecar);
            }
        }
    }

    fn reclaim_orphan_lock(
        &self,
        held: HeldTreeLock,
        index: &TreesIndex,
        report: &mut TreePruneReport,
    ) {
        let paths = held.paths();
        let artifact = held.artifact();
        let lock = paths.single_flight_lock(artifact);
        if index.has_work_for(artifact) || !self.is_aged(&lock) {
            return;
        }
        let confirmed = TreesIndex::read(paths, self.ports.filesystem)
            .is_ok_and(|relisted| !relisted.has_work_for(artifact));
        if confirmed
            && self
                .unlink_file(
                    &mut report.removals,
                    &lock,
                    "orphan tree lock",
                    Count::Once,
                )
                .left_nothing()
        {
            index.forget(&lock);
        }
        drop(held);
    }

    /// The pointer goes first, so a concurrent `locate` misses and
    /// re-materialises rather than leasing a generation about to go. The
    /// generation then goes under an exclusive lock on its lease; when a
    /// dispatch leased it after the probe, it is left for a later reap.
    /// Where `flock` fails outright the probe already answered `Unknown`, and
    /// only the age backstop let the removal this far.
    fn remove_tree(
        &self,
        tree: &UnclaimedTree<'_>,
        claims: &TreeClaimIndex,
        index: &TreesIndex,
        report: &mut TreePruneReport,
    ) {
        let removals = &mut report.removals;
        let paths = tree.paths;
        if self.unlink_file(
            removals,
            &tree.pointer,
            "unclaimed tree",
            Count::WithItsEntry,
        ) == Removal::Failed
        {
            return;
        }
        index.forget(&tree.pointer);
        let exclusive =
            match try_exclusive_lease(&paths.lease(&tree.generation)) {
                Ok(Some(exclusive)) => Some(exclusive),
                Ok(None) => return,
                Err(_) => None,
            };
        self.unlink_tree(
            removals,
            &paths.generation(&tree.generation),
            "unclaimed tree",
            Count::Once,
        );
        for sidecar in reap::sidecars_of(paths, &tree.generation) {
            self.unlink_file(
                removals,
                &sidecar,
                "unclaimed tree",
                Count::WithItsEntry,
            );
        }
        for claim in claims.on(tree.digest) {
            self.unlink_file(
                removals,
                &claim.path,
                "claim on an unclaimed tree",
                Count::WithItsEntry,
            );
        }
        index.forget(&paths.generation(&tree.generation));
        drop(exclusive);
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
                self.unlink_file(
                    &mut report.removals,
                    &claim.path,
                    "stale tree claim",
                    Count::Once,
                );
            }
        }
    }

    fn unlink_file(
        &self,
        removals: &mut Removals,
        path: &Path,
        rule: &str,
        count: Count,
    ) -> Removal {
        self.ports.removals.removing(path);
        removals.remove_file(self.ports.filesystem, path, rule, count)
    }

    fn unlink_tree(
        &self,
        removals: &mut Removals,
        path: &Path,
        rule: &str,
        count: Count,
    ) -> Removal {
        self.ports.removals.removing(path);
        removals.remove_tree(self.ports.filesystem, path, rule, count)
    }

    fn is_protected(&self, claims: &TreeClaimIndex, digest: &str) -> bool {
        claims.protects(
            digest,
            self.claim_window,
            self.ports.clock.now_seconds(),
            self.ports.ownership.effective_uid(),
        )
    }

    fn liveness(&self) -> HeldLeasesFirst<'_> {
        self.ports.held_leases.before(self.ports.lease_liveness)
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

struct UnclaimedTree<'a> {
    paths: &'a TreePaths,
    pointer: PathBuf,
    generation: String,
    digest: &'a str,
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

/// `trees/` listed once, so every step judges the same names, less what
/// this prune has since removed.
struct TreesIndex {
    paths: TreePaths,
    names: Vec<String>,
    removed: RefCell<BTreeSet<String>>,
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
        Ok(Self {
            paths: paths.clone(),
            names,
            removed: RefCell::default(),
        })
    }

    fn entries(&self) -> impl Iterator<Item = (&str, TreesEntry<'_>)> {
        self.names
            .iter()
            .filter(|name| !self.removed.borrow().contains(name.as_str()))
            .map(|name| (name.as_str(), classify_trees_entry(name)))
    }

    fn holds(&self, name: &str) -> bool {
        self.entries().any(|(listed, _)| listed == name)
    }

    /// A generation's sidecars go with it, so they are forgotten with it.
    fn forget(&self, path: &Path) {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return;
        };
        let mut removed = self.removed.borrow_mut();
        if matches!(classify_trees_entry(name), TreesEntry::Generation(_)) {
            for sidecar in reap::sidecars_of(&self.paths, name) {
                if let Some(sidecar) =
                    sidecar.file_name().and_then(|name| name.to_str())
                {
                    removed.insert(sidecar.to_owned());
                }
            }
        }
        removed.insert(name.to_owned());
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

    /// Temp residue of any artifact, or a generation no pointer names: what
    /// only a reap reclaims.
    fn holds_orphan_residue(&self, filesystem: &dyn CacheFilesystem) -> bool {
        let referenced: BTreeSet<String> = self
            .entries()
            .filter(|(_, entry)| matches!(entry, TreesEntry::Pointer { .. }))
            .filter_map(|(name, _)| {
                read_pointer(&self.paths.root().join(name), filesystem)
            })
            .map(|contents| contents.trim().to_owned())
            .collect();
        self.entries().any(|(name, entry)| match entry {
            TreesEntry::TempResidue { .. } => true,
            TreesEntry::Generation(_) => !referenced.contains(name),
            _ => false,
        })
    }

    fn has_lock_of(&self, artifact: &Artifact) -> bool {
        self.entries().any(|(_, entry)| {
            matches!(entry, TreesEntry::Lock { artifact: owner } if &owner == artifact)
        })
    }

    fn pointers_of(&self, artifact: &Artifact) -> Vec<(&str, &str)> {
        self.entries()
            .filter_map(|(name, entry)| match entry {
                TreesEntry::Pointer {
                    artifact: owner,
                    digest,
                } if &owner == artifact => Some((name, digest)),
                _ => None,
            })
            .collect()
    }

    /// `(sidecar name, generation name)` for each of the artifact's sidecars.
    fn sidecars_of(&self, artifact: &Artifact) -> Vec<(String, String)> {
        self.entries()
            .filter_map(|(name, entry)| match entry {
                TreesEntry::Sidecar {
                    artifact: owner,
                    generation,
                } if &owner == artifact => {
                    Some((name.to_owned(), generation.to_owned()))
                }
                _ => None,
            })
            .collect()
    }
}

/// The reap removes through `std::fs`, so the index forgets what it is told
/// goes, and the prune's own observer is told too.
struct Announcing<'a> {
    index: &'a TreesIndex,
    observer: &'a dyn RemovalObserver,
}

impl RemovalObserver for Announcing<'_> {
    fn removing(&self, entry: &Path) {
        self.observer.removing(entry);
        self.index.forget(entry);
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
