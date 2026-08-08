//! The non-interactive lifecycle contract: preview, mechanical apply loop,
//! ledger append, and summary.

use crate::ledger;
use crate::ports::LedgerStore;
use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::ports::PreviewEntry;
use crate::ports::Reporter;
use crate::registry::ApplyOutcome;
use crate::registry::MigrationEntry;

/// Computes `pending = registry() - applied - skipped` and applies each
/// pending entry in order.
///
/// Reports the preview banner (or the no-pending sentinel), applies each
/// pending entry in order, and reports the summary. Returns `Err` on the
/// first failure, leaving the ledger at the last success — matching
/// `run-migrations.sh`'s fail-fast apply loop.
///
/// # Errors
/// The first [`MigrationError`] a pending entry's `apply()` returns, or one
/// from the injected `LedgerStore`.
pub fn run_pending(
    entries: &[MigrationEntry],
    ctx: &dyn MigrationContext,
    ledger_store: &dyn LedgerStore,
    reporter: &dyn Reporter,
) -> Result<(), MigrationError> {
    let applied_ids = ledger_store.applied()?;
    let skipped_ids = ledger_store.skipped()?;

    let warnings = ledger::warnings(entries, &applied_ids, &skipped_ids);
    for id in &warnings.unknown_applied {
        reporter.unknown_applied_id(id);
    }
    for id in &warnings.unknown_skipped {
        reporter.unknown_skipped_id(id);
    }
    for id in &warnings.both {
        reporter.applied_and_skipped(id);
    }

    let pending = ledger::pending(entries, &applied_ids, &skipped_ids);

    if pending.is_empty() {
        reporter.no_pending_migrations(&skipped_ids);
        return Ok(());
    }

    let preview: Vec<PreviewEntry<'_>> = pending
        .iter()
        .map(|entry| PreviewEntry {
            id: entry.id(),
            description: entry.description(),
        })
        .collect();
    reporter.preview(&preview);

    let mut current_applied = applied_ids;
    let mut applied_count = 0usize;
    for entry in &pending {
        reporter.migration_running(entry.id());
        match dispatch_entry(entry, ctx) {
            Ok(ApplyOutcome::Applied) => {
                current_applied =
                    ledger::append_unique(current_applied, entry.id());
                ledger_store.write_applied(&current_applied)?;
                reporter.migration_applied(entry.id());
                applied_count += 1;
            }
            Ok(ApplyOutcome::NoOpPending) => {
                reporter.migration_no_op(entry.id());
            }
            Err(error) => {
                reporter.migration_failed(entry.id(), &error);
                return Err(error);
            }
        }
    }

    let pending_remaining = pending.len() - applied_count;
    reporter.summary(applied_count, &skipped_ids, pending_remaining);
    Ok(())
}

fn dispatch_entry(
    entry: &MigrationEntry,
    ctx: &dyn MigrationContext,
) -> Result<ApplyOutcome, MigrationError> {
    match entry {
        MigrationEntry::Mechanical(migration) => migration.apply(ctx),
    }
}
