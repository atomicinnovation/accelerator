//! Table-driven tests for pending computation, unknown-ID preservation, and
//! applied-wins/cross-state warnings — no filesystem, no adapters.

use migrate::ledger;
use migrate::ports::MigrationContext;
use migrate::ports::MigrationError;
use migrate::registry::ApplyOutcome;
use migrate::registry::Migration;
use migrate::registry::MigrationEntry;
use migrate::registry::MigrationMeta;

struct StubMigration {
    id: &'static str,
}

impl MigrationMeta for StubMigration {
    fn id(&self) -> &'static str {
        self.id
    }

    fn description(&self) -> &'static str {
        "a stub migration"
    }
}

impl Migration for StubMigration {
    fn apply(
        &self,
        _ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        Ok(ApplyOutcome::Applied)
    }
}

fn entries(ids: &[&'static str]) -> Vec<MigrationEntry> {
    ids.iter()
        .map(|id| MigrationEntry::Mechanical(Box::new(StubMigration { id })))
        .collect()
}

fn owned(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

#[test]
fn pending_excludes_applied_and_skipped_preserving_registry_order() {
    let entries = entries(&["0001", "0002", "0003"]);
    let applied = owned(&["0001"]);
    let skipped = owned(&["0003"]);

    let pending = ledger::pending(&entries, &applied, &skipped);

    assert_eq!(
        pending.iter().map(|e| e.id()).collect::<Vec<_>>(),
        vec!["0002"]
    );
}

#[test]
fn an_id_in_both_applied_and_skipped_is_excluded_from_pending() {
    let entries = entries(&["0001", "0002"]);
    let applied = owned(&["0001"]);
    let skipped = owned(&["0001"]);

    let pending = ledger::pending(&entries, &applied, &skipped);

    assert_eq!(
        pending.iter().map(|e| e.id()).collect::<Vec<_>>(),
        vec!["0002"]
    );
}

#[test]
fn an_unknown_applied_id_is_reported_but_not_treated_as_pending() {
    let entries = entries(&["0001"]);
    let applied = owned(&["9999"]);
    let skipped = owned(&[]);

    let warnings = ledger::warnings(&entries, &applied, &skipped);

    assert_eq!(warnings.unknown_applied, vec!["9999".to_owned()]);
    assert!(warnings.unknown_skipped.is_empty());
    assert!(warnings.both.is_empty());
}

#[test]
fn an_unknown_skipped_id_is_reported_in_skipped_order() {
    let entries = entries(&["0001"]);
    let applied = owned(&[]);
    let skipped = owned(&["9999", "8888"]);

    let warnings = ledger::warnings(&entries, &applied, &skipped);

    assert_eq!(
        warnings.unknown_skipped,
        vec!["9999".to_owned(), "8888".to_owned()]
    );
}

#[test]
fn an_id_present_in_both_applied_and_skipped_warns_applied_takes_precedence() {
    let entries = entries(&["0001"]);
    let applied = owned(&["0001"]);
    let skipped = owned(&["0001"]);

    let warnings = ledger::warnings(&entries, &applied, &skipped);

    assert_eq!(warnings.both, vec!["0001".to_owned()]);
}

#[test]
fn append_unique_is_idempotent_and_order_preserving() {
    let ids = vec!["0001".to_owned(), "0002".to_owned()];
    let once = ledger::append_unique(ids, "0003");
    let twice = ledger::append_unique(once.clone(), "0003");

    assert_eq!(once, vec!["0001", "0002", "0003"]);
    assert_eq!(twice, once);
}

#[test]
fn remove_line_is_a_no_op_when_the_id_is_absent() {
    let ids = vec!["0001".to_owned()];
    assert_eq!(ledger::remove_line(ids.clone(), "9999"), ids);
}
