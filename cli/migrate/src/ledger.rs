//! Pending computation, unknown-ID preservation, and the applied/skipped
//! ledger's idempotent, order-preserving set mutation. No filesystem — reads
//! and writes go through the injected `LedgerStore` port.

use crate::ports::LedgerStore;
use crate::ports::MigrationError;
use crate::registry::MigrationEntry;

pub struct Warnings {
    pub unknown_applied: Vec<String>,
    pub unknown_skipped: Vec<String>,
    pub both: Vec<String>,
}

/// Unknown-ID and cross-state warnings, in the same order bash emits them.
///
/// Every unknown applied ID (applied order), then every unknown skipped ID
/// (skipped order), then every ID present in both (applied order).
#[must_use]
pub fn warnings(
    entries: &[MigrationEntry],
    applied: &[String],
    skipped: &[String],
) -> Warnings {
    let known: Vec<&str> = entries.iter().map(MigrationEntry::id).collect();
    let unknown_applied = applied
        .iter()
        .filter(|id| !known.contains(&id.as_str()))
        .cloned()
        .collect();
    let unknown_skipped = skipped
        .iter()
        .filter(|id| !known.contains(&id.as_str()))
        .cloned()
        .collect();
    let both = applied
        .iter()
        .filter(|id| skipped.contains(id))
        .cloned()
        .collect();
    Warnings {
        unknown_applied,
        unknown_skipped,
        both,
    }
}

/// The pending subset of `entries`, in registry order: neither applied nor
/// skipped.
///
/// Applied takes precedence over skipped for an ID in both states — such an
/// ID is already excluded by the applied check, so no separate precedence
/// branch is needed.
#[must_use]
pub fn pending<'a>(
    entries: &'a [MigrationEntry],
    applied: &[String],
    skipped: &[String],
) -> Vec<&'a MigrationEntry> {
    entries
        .iter()
        .filter(|entry| {
            let id = entry.id();
            !applied.iter().any(|existing| existing == id)
                && !skipped.iter().any(|existing| existing == id)
        })
        .collect()
}

/// Idempotent, order-preserving append — the in-memory equivalent of
/// `atomic_append_unique`.
#[must_use]
pub fn append_unique(mut ids: Vec<String>, id: &str) -> Vec<String> {
    if !ids.iter().any(|existing| existing == id) {
        ids.push(id.to_owned());
    }
    ids
}

/// The in-memory equivalent of `atomic_remove_line` — removes every line
/// matching `id`, order-preserving, a no-op if absent.
#[must_use]
pub fn remove_line(mut ids: Vec<String>, id: &str) -> Vec<String> {
    ids.retain(|existing| existing != id);
    ids
}

/// No ID validation against the registry, matching bash exactly: an
/// unrecognised ID is silently accepted (it surfaces later as an unknown-ID
/// warning on the next default run).
///
/// # Errors
/// [`MigrationError`] when the skip list cannot be read or written.
pub fn skip(store: &dyn LedgerStore, id: &str) -> Result<(), MigrationError> {
    let updated = append_unique(store.skipped()?, id);
    store.write_skipped(&updated)
}

/// # Errors
/// [`MigrationError`] when the skip list cannot be read or written.
pub fn unskip(store: &dyn LedgerStore, id: &str) -> Result<(), MigrationError> {
    let updated = remove_line(store.skipped()?, id);
    store.write_skipped(&updated)
}

/// # Errors
/// [`MigrationError`] when the applied ledger cannot be read or written.
pub fn unapply(
    store: &dyn LedgerStore,
    id: &str,
) -> Result<(), MigrationError> {
    let updated = remove_line(store.applied()?, id);
    store.write_applied(&updated)
}
