//! `run_pending` exercised end to end against in-memory test doubles: no
//! filesystem, no adapters — just the domain's own ports.

use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

use migrate::interactive::Decision;
use migrate::interactive::Transformation;
use migrate::lifecycle::run_pending;
use migrate::ports::CorpusIndex;
use migrate::ports::DecisionError;
use migrate::ports::DecisionSource;
use migrate::ports::DocTypeDir;
use migrate::ports::LedgerStore;
use migrate::ports::MigrationContext;
use migrate::ports::MigrationError;
use migrate::ports::PreviewEntry;
use migrate::ports::Reporter;
use migrate::ports::SessionLog;
use migrate::ports::SessionLogFactory;
use migrate::registry::ApplyOutcome;
use migrate::registry::Migration;
use migrate::registry::MigrationEntry;
use migrate::registry::MigrationMeta;

struct NeverDecides;

impl DecisionSource for NeverDecides {
    fn next_decision(
        &self,
        _transformation: &Transformation,
        _timeout: Duration,
    ) -> Result<Decision, DecisionError> {
        Err(DecisionError::NoInputAvailable)
    }
}

struct NoSessionLogs;

impl SessionLogFactory for NoSessionLogs {
    fn for_migration(&self, _id: &str) -> Box<dyn SessionLog> {
        unreachable!("no interactive entry in this test needs a session log")
    }
}

struct StubContext;

struct NoIndex;

impl CorpusIndex for NoIndex {
    fn target_exists(&self, _target_type: &str, _target_id: &str) -> bool {
        false
    }
}

impl MigrationContext for StubContext {
    fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
        Vec::new()
    }

    fn revision(&self) -> Option<String> {
        None
    }

    fn corpus_index(&self) -> &dyn CorpusIndex {
        &NoIndex
    }

    fn write(
        &self,
        _path: &Path,
        _content: &str,
    ) -> Result<(), MigrationError> {
        Ok(())
    }
}

#[derive(Default)]
struct InMemoryLedgerStore {
    applied: RefCell<Vec<String>>,
    skipped: RefCell<Vec<String>>,
}

impl LedgerStore for InMemoryLedgerStore {
    fn applied(&self) -> Result<Vec<String>, MigrationError> {
        Ok(self.applied.borrow().clone())
    }

    fn write_applied(&self, ids: &[String]) -> Result<(), MigrationError> {
        *self.applied.borrow_mut() = ids.to_vec();
        Ok(())
    }

    fn skipped(&self) -> Result<Vec<String>, MigrationError> {
        Ok(self.skipped.borrow().clone())
    }

    fn write_skipped(&self, ids: &[String]) -> Result<(), MigrationError> {
        *self.skipped.borrow_mut() = ids.to_vec();
        Ok(())
    }
}

#[derive(Default)]
struct SpyReporter {
    events: RefCell<Vec<String>>,
}

impl Reporter for SpyReporter {
    fn preview(&self, pending: &[PreviewEntry<'_>]) {
        self.events.borrow_mut().push(format!(
            "preview:{}",
            pending.iter().map(|e| e.id).collect::<Vec<_>>().join(",")
        ));
    }

    fn no_pending_migrations(&self, _skipped: &[String]) {
        self.events.borrow_mut().push("no_pending".to_owned());
    }

    fn unknown_applied_id(&self, id: &str) {
        self.events
            .borrow_mut()
            .push(format!("unknown_applied:{id}"));
    }

    fn unknown_skipped_id(&self, id: &str) {
        self.events
            .borrow_mut()
            .push(format!("unknown_skipped:{id}"));
    }

    fn applied_and_skipped(&self, id: &str) {
        self.events.borrow_mut().push(format!("both:{id}"));
    }

    fn migration_running(&self, id: &str) {
        self.events.borrow_mut().push(format!("running:{id}"));
    }

    fn migration_applied(&self, id: &str) {
        self.events.borrow_mut().push(format!("applied:{id}"));
    }

    fn migration_no_op(&self, id: &str) {
        self.events.borrow_mut().push(format!("no_op:{id}"));
    }

    fn migration_failed(&self, id: &str, error: &MigrationError) {
        self.events
            .borrow_mut()
            .push(format!("failed:{id}:{error}"));
    }

    fn interactive_fail(&self, id: &str, message: &str) {
        self.events
            .borrow_mut()
            .push(format!("interactive_fail:{id}:{message}"));
    }

    fn interactive_validation_rejected(&self, message: &str) {
        self.events
            .borrow_mut()
            .push(format!("interactive_validation_rejected:{message}"));
    }

    fn interactive_stalled(&self, id: &str, pending_keys: &[String]) {
        self.events.borrow_mut().push(format!(
            "interactive_stalled:{id}:{}",
            pending_keys.join(",")
        ));
    }

    fn interactive_timeout(&self, id: &str) {
        self.events
            .borrow_mut()
            .push(format!("interactive_timeout:{id}"));
    }

    fn summary(
        &self,
        applied: usize,
        _skipped: &[String],
        pending_remaining: usize,
    ) {
        self.events
            .borrow_mut()
            .push(format!("summary:{applied}:{pending_remaining}"));
    }
}

struct AlwaysApplies(&'static str);

impl MigrationMeta for AlwaysApplies {
    fn id(&self) -> &'static str {
        self.0
    }

    fn description(&self) -> &'static str {
        "always applies"
    }
}

impl Migration for AlwaysApplies {
    fn apply(
        &self,
        _ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        Ok(ApplyOutcome::Applied)
    }
}

struct AlwaysNoOp(&'static str);

impl MigrationMeta for AlwaysNoOp {
    fn id(&self) -> &'static str {
        self.0
    }

    fn description(&self) -> &'static str {
        "always no-ops"
    }
}

impl Migration for AlwaysNoOp {
    fn apply(
        &self,
        _ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        Ok(ApplyOutcome::NoOpPending)
    }
}

struct AlwaysFails(&'static str);

impl MigrationMeta for AlwaysFails {
    fn id(&self) -> &'static str {
        self.0
    }

    fn description(&self) -> &'static str {
        "always fails"
    }
}

impl Migration for AlwaysFails {
    fn apply(
        &self,
        _ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        Err(MigrationError::new("boom"))
    }
}

type TestError = Box<dyn std::error::Error>;

#[test]
fn an_empty_pending_set_reports_no_pending_migrations_and_touches_no_ledger(
) -> Result<(), TestError> {
    let entries: Vec<MigrationEntry> = Vec::new();
    let ledger = InMemoryLedgerStore::default();
    let reporter = SpyReporter::default();

    run_pending(
        &entries,
        &StubContext,
        &ledger,
        &reporter,
        &NeverDecides,
        &NoSessionLogs,
        Duration::from_secs(30),
    )?;

    assert_eq!(
        reporter.events.borrow().as_slice(),
        ["no_pending".to_owned()]
    );
    Ok(())
}

#[test]
fn applied_and_no_op_entries_are_dispatched_and_reported_in_order(
) -> Result<(), TestError> {
    let entries = vec![
        MigrationEntry::Mechanical(Box::new(AlwaysApplies("0001"))),
        MigrationEntry::Mechanical(Box::new(AlwaysNoOp("0002"))),
    ];
    let ledger = InMemoryLedgerStore::default();
    let reporter = SpyReporter::default();

    run_pending(
        &entries,
        &StubContext,
        &ledger,
        &reporter,
        &NeverDecides,
        &NoSessionLogs,
        Duration::from_secs(30),
    )?;

    assert_eq!(ledger.applied()?, vec!["0001".to_owned()]);
    assert_eq!(
        reporter.events.borrow().as_slice(),
        [
            "preview:0001,0002".to_owned(),
            "running:0001".to_owned(),
            "applied:0001".to_owned(),
            "running:0002".to_owned(),
            "no_op:0002".to_owned(),
            "summary:1:1".to_owned(),
        ]
    );
    Ok(())
}

#[test]
fn a_failure_stops_the_run_leaving_the_ledger_at_the_last_success(
) -> Result<(), TestError> {
    let entries = vec![
        MigrationEntry::Mechanical(Box::new(AlwaysApplies("0001"))),
        MigrationEntry::Mechanical(Box::new(AlwaysFails("0002"))),
        MigrationEntry::Mechanical(Box::new(AlwaysApplies("0003"))),
    ];
    let ledger = InMemoryLedgerStore::default();
    let reporter = SpyReporter::default();

    let result = run_pending(
        &entries,
        &StubContext,
        &ledger,
        &reporter,
        &NeverDecides,
        &NoSessionLogs,
        Duration::from_secs(30),
    );

    assert!(result.is_err());
    assert_eq!(ledger.applied()?, vec!["0001".to_owned()]);
    assert_eq!(
        reporter.events.borrow().as_slice(),
        [
            "preview:0001,0002,0003".to_owned(),
            "running:0001".to_owned(),
            "applied:0001".to_owned(),
            "running:0002".to_owned(),
            "failed:0002:boom".to_owned(),
        ]
    );
    Ok(())
}

#[test]
fn an_id_already_applied_is_not_re_run() -> Result<(), TestError> {
    let entries =
        vec![MigrationEntry::Mechanical(Box::new(AlwaysFails("0001")))];
    let ledger = InMemoryLedgerStore::default();
    ledger.write_applied(&["0001".to_owned()])?;
    let reporter = SpyReporter::default();

    run_pending(
        &entries,
        &StubContext,
        &ledger,
        &reporter,
        &NeverDecides,
        &NoSessionLogs,
        Duration::from_secs(30),
    )?;

    assert_eq!(
        reporter.events.borrow().as_slice(),
        ["no_pending".to_owned()]
    );
    Ok(())
}
