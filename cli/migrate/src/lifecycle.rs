//! The lifecycle contract: preview, the apply loop (mechanical entries
//! dispatched directly, interactive entries routed through
//! `engine::run_interactive`), ledger append, and summary.

use std::time::Duration;

use crate::decisions_file;
use crate::engine;
use crate::ledger;
use crate::ports::DecisionSource;
use crate::ports::LedgerStore;
use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::ports::PreviewEntry;
use crate::ports::Reporter;
use crate::ports::SessionLogFactory;
use crate::registry::ApplyOutcome;
use crate::registry::MigrationEntry;

/// Computes `pending = registry() - applied - skipped` and applies each
/// pending entry in order.
///
/// Reports the preview banner (or the no-pending sentinel), applies each
/// pending entry in order, and reports the summary. Returns `Err` on the
/// first failure, leaving the ledger at the last success.
///
/// When `decisions_file_content` is `Some`, every pending interactive entry
/// is validated against it — dry, no `apply_decision` call, no session-log
/// write — immediately after the preview banner and before the apply loop
/// begins.
///
/// The doc type directories must resolve before anything is computed, and
/// again after each applied entry before it is recorded, so a configuration
/// whose doc roots overlap stops the run with nothing applied or recorded.
///
/// # Errors
/// The first [`MigrationError`] a pending entry's `apply()` returns, one
/// from the injected `LedgerStore`, the first decisions-file validation
/// failure (naming the offending position), or the failure to resolve the
/// doc type directories.
#[allow(clippy::too_many_arguments)]
pub fn run_pending(
    entries: &[MigrationEntry],
    ctx: &dyn MigrationContext,
    ledger_store: &dyn LedgerStore,
    reporter: &dyn Reporter,
    decisions: &dyn DecisionSource,
    session_logs: &dyn SessionLogFactory,
    timeout: Duration,
    decisions_file_content: Option<&str>,
) -> Result<(), MigrationError> {
    ensure_doc_roots_resolve(ctx)?;
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

    if let Some(content) = decisions_file_content {
        for entry in &pending {
            let MigrationEntry::Interactive(migration) = entry else {
                continue;
            };
            let session_log = session_logs.for_migration(migration.id());
            let prompts = engine::pending_transformations(
                migration.as_ref(),
                ctx,
                session_log.as_ref(),
                reporter,
            )?;
            decisions_file::validate(migration.as_ref(), &prompts, content)?;
        }
    }

    let mut applied_count = 0usize;
    for entry in &pending {
        reporter.migration_running(entry.id());
        match dispatch_entry(
            entry,
            ctx,
            decisions,
            session_logs,
            timeout,
            reporter,
        )
        .and_then(|outcome| {
            if matches!(outcome, ApplyOutcome::Applied) {
                ensure_doc_roots_resolve(ctx)?;
            }
            Ok(outcome)
        }) {
            Ok(ApplyOutcome::Applied) => {
                // Re-read rather than reuse the loop-start snapshot: a
                // migration's own `apply()` may have written directly to
                // this same ledger file (m0003 merges legacy
                // `meta/.migrations-*` into it), so the append must land on
                // whatever is currently on disk, never a stale in-memory
                // list.
                let current_applied =
                    ledger::append_unique(ledger_store.applied()?, entry.id());
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

/// # Errors
/// [`MigrationError`] when the configuration cannot be read, or resolves to
/// an unsafe or overlapping set of doc roots.
pub fn ensure_doc_roots_resolve(
    ctx: &dyn MigrationContext,
) -> Result<(), MigrationError> {
    ctx.doc_type_dirs().map(|_| ())
}

fn dispatch_entry(
    entry: &MigrationEntry,
    ctx: &dyn MigrationContext,
    decisions: &dyn DecisionSource,
    session_logs: &dyn SessionLogFactory,
    timeout: Duration,
    reporter: &dyn Reporter,
) -> Result<ApplyOutcome, MigrationError> {
    match entry {
        MigrationEntry::Mechanical(migration) => migration.apply(ctx),
        MigrationEntry::Interactive(migration) => {
            let session_log = session_logs.for_migration(migration.id());
            engine::run_interactive(
                migration.as_ref(),
                ctx,
                decisions,
                session_log.as_ref(),
                timeout,
                reporter,
            )
        }
    }
}
