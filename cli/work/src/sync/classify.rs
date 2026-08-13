//! The change-detection engine.

use tracker::ExternalId;
use tracker::RemoteTimestamp;

use crate::sync::state::SyncState;

/// A lazy source of the values `classify` may need to hash or stat.
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
    fn local(&self) -> Result<String, kernel::Error>;

    /// `Ok(None)` means no body was fetched, not that a read failed. A read
    /// that failed is `Err`; the two must not collapse.
    ///
    /// # Errors
    ///
    /// When a body was fetched but could not be normalised.
    fn remote_body(&self) -> Result<Option<String>, kernel::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemotePresence {
    Present,
    Absent,
    Indeterminate,
}

pub struct BaselineEntry<'a> {
    pub remote_updated_at: &'a RemoteTimestamp,
    pub remote_hash: Option<&'a str>,
    pub local_hash: Option<&'a str>,
}

pub struct Subject<'a> {
    pub external_id: Option<&'a ExternalId>,
    pub presence: RemotePresence,
    pub remote_updated: &'a RemoteTimestamp,
    pub baseline_timestamp: u64,
}

fn local_changed(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    digests: &dyn ItemDigests,
) -> Result<bool, kernel::Error> {
    let Some(base_hash) = baseline.local_hash else {
        return Ok(true);
    };
    if let Some(mtime) = digests.mtime()? {
        if mtime <= subject.baseline_timestamp {
            return Ok(false);
        }
    }
    Ok(digests.local()? != base_hash)
}

fn remote_changed(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    digests: &dyn ItemDigests,
) -> Result<bool, kernel::Error> {
    let Some(base_hash) = baseline.remote_hash else {
        return Ok(true);
    };
    if subject
        .remote_updated
        .proves_unchanged_since(baseline.remote_updated_at)
    {
        return Ok(false);
    }
    digests
        .remote_body()?
        .map_or(Ok(true), |body| Ok(body != base_hash))
}

/// Classifies one item against its baseline.
///
/// Each side defaults to changed when it cannot be proven otherwise, so a
/// missing baseline or an unreadable stamp yields a conflict a human
/// resolves rather than a silent overwrite.
///
/// # Errors
///
/// Propagates any [`ItemDigests`] read failure.
pub fn classify(
    subject: &Subject<'_>,
    baseline: &BaselineEntry<'_>,
    digests: &dyn ItemDigests,
) -> Result<SyncState, kernel::Error> {
    if subject.external_id.is_none() {
        return Ok(SyncState::Unsynced);
    }

    match subject.presence {
        RemotePresence::Indeterminate => return Ok(SyncState::Indeterminate),
        RemotePresence::Absent => return Ok(SyncState::RemoteAbsent),
        RemotePresence::Present => {}
    }

    let local = local_changed(subject, baseline, digests)?;
    let remote = remote_changed(subject, baseline, digests)?;

    Ok(match (local, remote) {
        (false, false) => SyncState::Synced,
        (true, false) => SyncState::LocallyModified,
        (false, true) => SyncState::RemotelyModified,
        (true, true) => SyncState::Conflict,
    })
}
