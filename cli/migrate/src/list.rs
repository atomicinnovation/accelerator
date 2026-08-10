//! `--list`: the read-only enumeration of pending interactive decisions.
//!
//! Skips the whole pre-flight (no manifest/run-id setup, no lock), never
//! writes to a session log, and never calls `apply_decision`. Mechanical
//! migrations are not visited at all — only `MigrationEntry::Interactive`
//! entries have anything to list.
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
    /// a live run applies, so a caller renders the "in-flight session"
    /// note from this rather than re-deriving it.
    pub had_in_flight_session: bool,
}

/// # Errors
/// The first pending interactive migration's `Fail` predicate message (see
/// [`engine::pending_transformations`]), or the first transformation whose
/// key/proposed/path/anchor carries an embedded tab or newline: `--list`'s
/// output is otherwise undefined for such a value, since the tab-delimited
/// `render_list` line has an unresolvable corruption risk.
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
        if let Some(unsafe_field) =
            transformations.iter().find_map(unsafe_list_field)
        {
            return Err(MigrationError::new(format!(
                "[{id}] --list field for key '{unsafe_field}' contains a \
                 tab or newline; --list output is undefined for such \
                 values."
            )));
        }
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

/// The transformation's own `short_key()`, when any of its `--list`-rendered
/// fields (`short_key`/`proposed`/`path`/`anchor`) carries an embedded tab
/// or newline — `None` when the transformation is safe to render.
fn unsafe_list_field(transformation: &Transformation) -> Option<&str> {
    let carries_tab_or_newline =
        |value: &str| value.contains('\t') || value.contains('\n');
    let unsafe_ = carries_tab_or_newline(transformation.short_key())
        || carries_tab_or_newline(&transformation.proposed)
        || carries_tab_or_newline(&transformation.path)
        || carries_tab_or_newline(&transformation.anchor);
    unsafe_.then(|| transformation.short_key())
}
