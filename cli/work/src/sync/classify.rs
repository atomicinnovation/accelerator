//! The change-detection engine.

use tracker::ExternalId;
use tracker::RemoteTimestamp;

use crate::sync::baseline::BaselineFingerprints;
use crate::sync::baseline::LegacyHashes;
use crate::sync::baseline::LocalFingerprints;
use crate::sync::baseline::RemoteFingerprints;
use crate::sync::baseline::SplitFingerprints;
use crate::sync::pull_residue::PullResidue;
use crate::sync::state::SyncState;

/// A lazy source of the values `classify` may need to hash or stat, under
/// either digest recipe.
///
/// A port rather than pre-computed values so a caller pays for a syscall
/// only when `classify` actually asks: the mtime pre-filter short-circuits
/// without hashing, and an absent baseline hash skips the `stat` and the
/// hash entirely.
///
/// Every method is fallible. An infallible port has nowhere to put a read
/// failure but a panic or a fabricated hash, and a fabricated empty hash
/// would silently classify the item as locally changed with no log line to
/// diagnose it from.
pub trait ItemDigests {
    /// `Ok(None)` when no mtime is available, which forces the hash path.
    ///
    /// # Errors
    ///
    /// When the mtime read fails in a way other than "not present".
    fn mtime(&self) -> Result<Option<u64>, kernel::Error>;

    /// # Errors
    ///
    /// When the local content cannot be read or normalised.
    fn legacy_local(&self) -> Result<String, kernel::Error>;

    /// `Ok(None)` means no body was fetched, not that a read failed. A read
    /// that failed is `Err`; the two must not collapse.
    ///
    /// # Errors
    ///
    /// When a body was fetched but could not be normalised.
    fn legacy_remote(&self) -> Result<Option<String>, kernel::Error>;

    /// # Errors
    ///
    /// When the local content cannot be read or fingerprinted.
    fn local(&self) -> Result<LocalFingerprints, kernel::Error>;

    /// `Ok(None)` means no body was fetched.
    ///
    /// # Errors
    ///
    /// When a body was fetched but could not be fingerprinted.
    fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error>;
}

/// What an item's local body says about how it came to be.
pub trait ItemComparison {
    /// The raw tracker content an earlier pull left in the local file, when
    /// the remote was fetched and the file still holds some.
    fn pull_residue(&self) -> Option<PullResidue>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemotePresence {
    Present,
    Absent,
    Indeterminate,
}

/// An item's baseline entry; an item with none compares against empty
/// legacy hashes.
pub struct BaselineEntry<'a> {
    pub remote_updated_at: &'a RemoteTimestamp,
    pub fingerprints: BaselineFingerprints,
}

pub struct Subject<'a> {
    pub external_id: Option<&'a ExternalId>,
    pub presence: RemotePresence,
    pub remote_updated: &'a RemoteTimestamp,
    /// The item's change-detection watermark: the epoch its local file was
    /// last synced. The mtime pre-filter short-circuits when the file's mtime
    /// does not exceed this. Sourced per item from the baseline (the entry's
    /// own watermark, or the document-level timestamp for an item with no
    /// entry yet).
    pub baseline_timestamp: u64,
}

/// How a synced item's baseline stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncedReconciliation {
    /// The baseline already records the item under the current recipe.
    ByContent,
    /// The item is synced, but its baseline must be rewritten under the
    /// current recipe.
    RecordOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    Unsynced,
    Synced(SyncedReconciliation),
    LocallyModified,
    RemotelyModified,
    Conflict,
    RemoteAbsent,
    Indeterminate,
}

impl Classification {
    /// The state a report names this classification by.
    #[must_use]
    pub const fn state(self) -> SyncState {
        match self {
            Self::Unsynced => SyncState::Unsynced,
            Self::Synced(_) => SyncState::Synced,
            Self::LocallyModified => SyncState::LocallyModified,
            Self::RemotelyModified => SyncState::RemotelyModified,
            Self::Conflict => SyncState::Conflict,
            Self::RemoteAbsent => SyncState::RemoteAbsent,
            Self::Indeterminate => SyncState::Indeterminate,
        }
    }
}

/// Whether the local file may have moved since `watermark`; `false` lets
/// the caller skip hashing it.
fn local_may_have_moved(
    subject: &Subject<'_>,
    digests: &dyn ItemDigests,
) -> Result<bool, kernel::Error> {
    Ok(digests
        .mtime()?
        .is_none_or(|mtime| mtime > subject.baseline_timestamp))
}

fn remote_proven_unchanged(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
) -> bool {
    subject
        .remote_updated
        .proves_unchanged_since(baseline.remote_updated_at)
}

fn legacy_local_changed(
    subject: &Subject<'_>,
    legacy: &LegacyHashes,
    digests: &dyn ItemDigests,
) -> Result<bool, kernel::Error> {
    let Some(recorded) = &legacy.local else {
        return Ok(true);
    };
    if !local_may_have_moved(subject, digests)? {
        return Ok(false);
    }
    Ok(&digests.legacy_local()? != recorded)
}

fn legacy_remote_changed(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    legacy: &LegacyHashes,
    digests: &dyn ItemDigests,
) -> Result<bool, kernel::Error> {
    let Some(recorded) = &legacy.remote else {
        return Ok(true);
    };
    if remote_proven_unchanged(subject, baseline) {
        return Ok(false);
    }
    Ok(digests
        .legacy_remote()?
        .is_none_or(|current| &current != recorded))
}

fn split_local_changed(
    subject: &Subject<'_>,
    split: &SplitFingerprints,
    digests: &dyn ItemDigests,
) -> Result<bool, kernel::Error> {
    let Some(recorded) = &split.local else {
        return Ok(true);
    };
    if !local_may_have_moved(subject, digests)? {
        return Ok(false);
    }
    Ok(&digests.local()? != recorded)
}

fn split_remote_changed(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    split: &SplitFingerprints,
    digests: &dyn ItemDigests,
) -> Result<bool, kernel::Error> {
    let Some(recorded) = &split.remote else {
        return Ok(true);
    };
    if remote_proven_unchanged(subject, baseline) {
        return Ok(false);
    }
    Ok(digests.remote()?.is_none_or(|current| &current != recorded))
}

const fn by_change(local: bool, remote: bool) -> Classification {
    match (local, remote) {
        (false, false) => {
            Classification::Synced(SyncedReconciliation::ByContent)
        }
        (true, false) => Classification::LocallyModified,
        (false, true) => Classification::RemotelyModified,
        (true, true) => Classification::Conflict,
    }
}

/// A legacy entry is classified by its residue first: a file holding the
/// current remote's own document, unedited since it was recorded, is synced
/// and only needs its baseline rewritten; any other residue is a conflict,
/// so polluted content reaches a person rather than a baseline or a send.
fn classify_legacy(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    legacy: &LegacyHashes,
    digests: &dyn ItemDigests,
    comparison: &dyn ItemComparison,
) -> Result<Classification, kernel::Error> {
    match comparison.pull_residue() {
        Some(PullResidue::VerbatimRemote)
            if !legacy_local_changed(subject, legacy, digests)? =>
        {
            return Ok(Classification::Synced(
                SyncedReconciliation::RecordOnly,
            ));
        }
        Some(_) => return Ok(Classification::Conflict),
        None => {}
    }
    let local = legacy_local_changed(subject, legacy, digests)?;
    let remote = legacy_remote_changed(subject, baseline, legacy, digests)?;
    Ok(match by_change(local, remote) {
        Classification::Synced(_) if digests.legacy_remote()?.is_some() => {
            Classification::Synced(SyncedReconciliation::RecordOnly)
        }
        classification => classification,
    })
}

/// Classifies one item against its baseline, under the entry's own recipe.
///
/// Each side defaults to changed when it cannot be proven otherwise, so a
/// missing baseline or an unreadable stamp yields a conflict a human
/// resolves rather than a silent overwrite. A synced item whose entry
/// predates the current recipe is rewritten only once its remote was
/// fetched, so the rewritten entry records the remote side too.
///
/// # Errors
///
/// Propagates any [`ItemDigests`] read failure.
pub fn classify(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    digests: &dyn ItemDigests,
    comparison: &dyn ItemComparison,
) -> Result<Classification, kernel::Error> {
    if subject.external_id.is_none() {
        return Ok(Classification::Unsynced);
    }

    match subject.presence {
        RemotePresence::Indeterminate => {
            return Ok(Classification::Indeterminate)
        }
        RemotePresence::Absent => return Ok(Classification::RemoteAbsent),
        RemotePresence::Present => {}
    }

    match &baseline.fingerprints {
        BaselineFingerprints::Legacy(legacy) => {
            classify_legacy(subject, baseline, legacy, digests, comparison)
        }
        BaselineFingerprints::Split(split) => {
            let local = split_local_changed(subject, split, digests)?;
            let remote =
                split_remote_changed(subject, baseline, split, digests)?;
            Ok(by_change(local, remote))
        }
    }
}
