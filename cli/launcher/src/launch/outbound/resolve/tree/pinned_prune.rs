//! Tree prune as the plugin's own `bin` has always run it: each pinned
//! artifact on the host platform, under its tree lock.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::launch::core::tree::{Clock, TreeError};

use super::super::filesystem::SystemFilesystem;
use super::super::HOST_PLATFORM;
use super::claims;
use super::layout::{
    read_pointer, validated_generation, Artifact, TreePaths, LAYOUT_VERSION,
};
use super::lease::LivenessProbe;
use super::pins;
use super::reap::{self, PinnedDigests, Reaper};
use super::tree_lock::HeldTreeLock;

/// What a pinned prune consults: no resolver, so a retention can run it
/// without building one.
pub struct PinnedPruneCollaborators<'a> {
    pub clock: &'a dyn Clock,
    pub liveness: &'a dyn LivenessProbe,
    pub pins: &'a dyn PinnedDigests,
}

/// [`prune_pinned_artifact`] for every pinned artifact on the host platform.
///
/// # Errors
///
/// The first [`TreeError`] any artifact's prune meets.
pub fn prune_pinned_on_host(
    cache_root: &Path,
    collaborators: &PinnedPruneCollaborators<'_>,
    older_than: Option<Duration>,
) -> Result<reap::Reclaimed, TreeError> {
    let paths = TreePaths::under(cache_root);
    let mut reclaimed = reap::Reclaimed::default();
    for artifact in pins::artifact_names() {
        reclaimed.entries += prune_pinned_artifact(
            &paths,
            &Artifact::new(artifact, HOST_PLATFORM),
            collaborators,
            older_than,
        )?
        .entries;
    }
    Ok(reclaimed)
}

/// Reclaim residue for one artifact under its single-flight lock.
///
/// Pointed-at generations whose digest no installed launcher has claimed
/// inside the window go first; the artifact's own pinned digest is spared,
/// since the running launcher just refreshed its claim. Another artifact's
/// temp residue is removed only under that artifact's tree lock, taken
/// without blocking, or, all but its partial archives, when that artifact has
/// no lock file; the reap module records why. Every artifact's pinned partial
/// archive is spared, so a resumable download survives.
///
/// # Errors
///
/// [`TreeError`] if the lock cannot be taken or the directory cannot be
/// read.
pub fn prune_pinned_artifact(
    paths: &TreePaths,
    artifact: &Artifact,
    collaborators: &PinnedPruneCollaborators<'_>,
    older_than: Option<Duration>,
) -> Result<reap::Reclaimed, TreeError> {
    let held = HeldTreeLock::take(paths, artifact.clone())?;
    let mut reclaimed =
        reclaim_unclaimed(paths, artifact, collaborators, older_than);
    let orphans = Reaper::new(
        &held,
        collaborators.clock,
        collaborators.liveness,
        collaborators.pins,
    )
    .reap_orphans()?;
    reclaimed.entries += orphans.entries;
    Ok(reclaimed)
}

/// Remove pointers, and the generations they name, whose digest has no
/// fresh claim — the artifact's own pinned digest excepted.
fn reclaim_unclaimed(
    paths: &TreePaths,
    artifact: &Artifact,
    collaborators: &PinnedPruneCollaborators<'_>,
    older_than: Option<Duration>,
) -> reap::Reclaimed {
    let window = older_than.unwrap_or(claims::CLAIM_WINDOW);
    let own = collaborators.pins.pinned_digest(artifact);
    let mut reclaimed = reap::Reclaimed::default();
    for (pointer, digest, target) in artifact_pointers(paths, artifact) {
        let PointerTarget::Current(generation) = target else {
            continue;
        };
        if own.as_deref() == Some(digest.as_str()) {
            continue;
        }
        if claims::claimed_within(paths, &digest, window, collaborators.clock) {
            continue;
        }
        if !reap::unclaimed_is_reclaimable(
            collaborators.liveness.probe(&paths.lease(&generation)),
            reap::generation_is_aged(paths, &generation, collaborators.clock),
        ) {
            continue;
        }
        let _ = std::fs::remove_file(&pointer);
        if std::fs::remove_dir_all(paths.generation(&generation)).is_ok() {
            reclaimed.entries += 1;
        }
        for sidecar in reap::sidecars_of(paths, &generation) {
            let _ = std::fs::remove_file(sidecar);
        }
        claims::drop_all(paths, &digest);
    }
    reclaimed
}

/// The `(pointer path, digest, target)` of every pointer this artifact
/// currently publishes.
fn artifact_pointers(
    paths: &TreePaths,
    artifact: &Artifact,
) -> Vec<(PathBuf, String, PointerTarget)> {
    let prefix = format!("{}-{}-", artifact.name(), artifact.platform());
    let Ok(entries) = std::fs::read_dir(paths.root()) else {
        return Vec::new();
    };
    let mut pointers = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(digest) = name
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix(".ref"))
        else {
            continue;
        };
        let target = PointerTarget::of(
            read_pointer(&entry.path(), &SystemFilesystem),
            artifact,
            digest,
        );
        pointers.push((entry.path(), digest.to_owned(), target));
    }
    pointers
}

/// What a pointer's contents name, judged by the grammar `locate` holds a
/// pointer to before any of it becomes a path.
enum PointerTarget {
    Current(String),
    OtherLayout,
    Malformed,
}

impl PointerTarget {
    fn of(contents: Option<String>, artifact: &Artifact, digest: &str) -> Self {
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
