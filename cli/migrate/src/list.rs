//! `--list`: the read-only enumeration of pending interactive decisions.
//!
//! Skips the whole pre-flight (no manifest/run-id setup, no lock — matching
//! `--list`'s bash treatment exactly), never writes to a session log, and
//! never calls `apply_decision`. Mechanical migrations are not visited at
//! all — only `MigrationEntry::Interactive` entries have anything to list.
//! Rendering (position numbering, `# migration <id>` segmentation, the
//! multi-migration/in-flight-session stderr notes) is the caller's job —
//! this module only computes the data those notes are a pure function of.

use crate::engine;
use crate::interactive::Transformation;
use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::ports::Reporter;
use crate::ports::SessionLogFactory;
use crate::registry::MigrationEntry;

/// One pending interactive migration's undecided transformations, in
/// emission order.
pub struct ListGroup {
    pub id: &'static str,
    pub transformations: Vec<Transformation>,
    /// Whether this migration's session log already had at least one
    /// record — `--list` excludes decided keys via the same resume filter
    /// a live run applies, so a caller renders bash's "in-flight session"
    /// note from this rather than re-deriving it.
    pub had_in_flight_session: bool,
}

/// # Errors
/// The first pending interactive migration's `Fail` predicate message (see
/// [`engine::pending_transformations`]).
pub fn list_pending(
    entries: &[MigrationEntry],
    applied: &[String],
    skipped: &[String],
    ctx: &dyn MigrationContext,
    session_logs: &dyn SessionLogFactory,
    reporter: &dyn Reporter,
) -> Result<Vec<ListGroup>, MigrationError> {
    let pending = crate::ledger::pending(entries, applied, skipped);
    let mut groups = Vec::new();
    for entry in pending {
        let MigrationEntry::Interactive(migration) = entry else {
            continue;
        };
        let id = migration.id();
        let session_log = session_logs.for_migration(id);
        let had_in_flight_session = !session_log.records()?.is_empty();

        let transformations = engine::pending_transformations(
            migration.as_ref(),
            ctx,
            session_log.as_ref(),
            reporter,
        )?;
        if transformations.is_empty() {
            continue;
        }
        groups.push(ListGroup {
            id,
            transformations,
            had_in_flight_session,
        });
    }
    Ok(groups)
}
