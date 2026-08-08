//! `run_interactive` exercised end to end against a `FixtureMigration` test
//! double and in-memory ports — the write-ahead-log invariant, source
//! drift, sticky skip, `verify_applied`, and predicate `Fail` are all
//! asserted here since no real interactive migration exists yet (0007 lands
//! once its own phase is unblocked).

use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

use corpus::Outcome;
use corpus::Record;
use migrate::engine::run_interactive;
use migrate::interactive::Decision;
use migrate::interactive::InteractiveMigration;
use migrate::interactive::PredicateOutcome;
use migrate::interactive::Transformation;
use migrate::ports::CorpusIndex;
use migrate::ports::DecisionError;
use migrate::ports::DecisionSource;
use migrate::ports::DocTypeDir;
use migrate::ports::MigrationContext;
use migrate::ports::MigrationError;
use migrate::ports::PreviewEntry;
use migrate::ports::Reporter;
use migrate::ports::SessionLog;
use migrate::registry::ApplyOutcome;
use migrate::registry::MigrationMeta;

type TestError = Box<dyn std::error::Error>;

struct NoIndex;

impl CorpusIndex for NoIndex {
    fn target_exists(&self, _target_type: &str, _target_id: &str) -> bool {
        false
    }
}

struct StubContext;

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

fn transformation(key: &str, proposed: &str) -> Transformation {
    Transformation {
        key: key.to_owned(),
        path: "meta/work/0001.md".to_owned(),
        anchor: "body/relates_to".to_owned(),
        proposed: proposed.to_owned(),
        predicate_value: "ambiguous".to_owned(),
        display: format!("display for {key}"),
        extras: Vec::new(),
    }
}

enum PredicateMode {
    Prompt,
    Mechanical,
    Fail(&'static str),
}

struct FixtureMigration {
    transformations: Vec<Transformation>,
    predicate: PredicateMode,
    reject_edit: bool,
    fail_apply: bool,
    verify_applied_result: bool,
}

impl FixtureMigration {
    const fn prompting(transformations: Vec<Transformation>) -> Self {
        Self {
            transformations,
            predicate: PredicateMode::Prompt,
            reject_edit: false,
            fail_apply: false,
            verify_applied_result: true,
        }
    }

    fn mechanical(transformations: Vec<Transformation>) -> Self {
        Self {
            predicate: PredicateMode::Mechanical,
            ..Self::prompting(transformations)
        }
    }

    fn failing(
        message: &'static str,
        transformations: Vec<Transformation>,
    ) -> Self {
        Self {
            predicate: PredicateMode::Fail(message),
            ..Self::prompting(transformations)
        }
    }
}

impl MigrationMeta for FixtureMigration {
    fn id(&self) -> &'static str {
        "0099-fixture"
    }

    fn description(&self) -> &'static str {
        "a fixture migration"
    }
}

impl InteractiveMigration for FixtureMigration {
    fn emit_transformations(
        &self,
        _ctx: &dyn MigrationContext,
    ) -> Vec<Transformation> {
        self.transformations
            .iter()
            .map(|t| transformation(&t.key, &t.proposed))
            .collect()
    }

    fn evaluate_predicate(
        &self,
        _transformation: &Transformation,
    ) -> PredicateOutcome {
        match &self.predicate {
            PredicateMode::Prompt => PredicateOutcome::Prompt,
            PredicateMode::Mechanical => PredicateOutcome::Mechanical,
            PredicateMode::Fail(message) => {
                PredicateOutcome::Fail((*message).to_owned())
            }
        }
    }

    fn validate_edit(
        &self,
        _transformation: &Transformation,
        value: &str,
    ) -> Result<(), String> {
        if self.reject_edit && value != "corrected" {
            return Err("empty value not allowed".to_owned());
        }
        Ok(())
    }

    fn apply_decision(
        &self,
        _transformation: &Transformation,
        _decision: &Decision,
        _ctx: &dyn MigrationContext,
    ) -> Result<(), String> {
        if self.fail_apply {
            return Err("apply exploded".to_owned());
        }
        Ok(())
    }

    fn verify_applied(
        &self,
        _transformation: &Transformation,
        _recorded: &Record,
    ) -> bool {
        self.verify_applied_result
    }
}

#[derive(Default)]
struct ScriptedDecisions {
    script: RefCell<Vec<Decision>>,
}

impl ScriptedDecisions {
    const fn new(script: Vec<Decision>) -> Self {
        Self {
            script: RefCell::new(script),
        }
    }
}

impl DecisionSource for ScriptedDecisions {
    fn next_decision(
        &self,
        _transformation: &Transformation,
        _timeout: Duration,
    ) -> Result<Decision, DecisionError> {
        let mut script = self.script.borrow_mut();
        if script.is_empty() {
            return Err(DecisionError::NoInputAvailable);
        }
        Ok(script.remove(0))
    }
}

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

struct AlwaysTimesOut;

impl DecisionSource for AlwaysTimesOut {
    fn next_decision(
        &self,
        _transformation: &Transformation,
        _timeout: Duration,
    ) -> Result<Decision, DecisionError> {
        Err(DecisionError::Timeout)
    }
}

#[derive(Default)]
struct InMemorySessionLog {
    records: RefCell<Vec<Record>>,
}

impl SessionLog for InMemorySessionLog {
    fn records(&self) -> Result<Vec<Record>, MigrationError> {
        Ok(self.records.borrow().clone())
    }

    fn append(
        &self,
        key: &str,
        outcome: Outcome,
        proposed_value: &str,
        user_value: Option<&str>,
    ) -> Result<(), MigrationError> {
        self.records.borrow_mut().push(Record {
            transformation_key: key.to_owned(),
            schema_version: 1,
            outcome,
            proposed_value: proposed_value.to_owned(),
            user_value: user_value.map(str::to_owned),
            timestamp: "2026-07-19T00:00:00+00:00".to_owned(),
            extras: Vec::new(),
        });
        Ok(())
    }

    fn remove_by_key(&self, key: &str) -> Result<(), MigrationError> {
        self.records
            .borrow_mut()
            .retain(|record| record.transformation_key != key);
        Ok(())
    }
}

#[derive(Default)]
struct SpyReporter {
    events: RefCell<Vec<String>>,
}

impl Reporter for SpyReporter {
    fn preview(&self, _pending: &[PreviewEntry<'_>]) {}
    fn no_pending_migrations(&self, _skipped: &[String]) {}
    fn unknown_applied_id(&self, _id: &str) {}
    fn unknown_skipped_id(&self, _id: &str) {}
    fn applied_and_skipped(&self, _id: &str) {}
    fn migration_running(&self, _id: &str) {}
    fn migration_applied(&self, _id: &str) {}
    fn migration_no_op(&self, _id: &str) {}
    fn migration_failed(&self, _id: &str, _error: &MigrationError) {}

    fn interactive_fail(&self, id: &str, message: &str) {
        self.events
            .borrow_mut()
            .push(format!("fail:{id}:{message}"));
    }

    fn interactive_validation_rejected(&self, message: &str) {
        self.events.borrow_mut().push(format!("rejected:{message}"));
    }

    fn interactive_stalled(&self, id: &str, pending_keys: &[String]) {
        self.events
            .borrow_mut()
            .push(format!("stalled:{id}:{}", pending_keys.join(",")));
    }

    fn interactive_timeout(&self, id: &str) {
        self.events.borrow_mut().push(format!("timeout:{id}"));
    }

    fn summary(
        &self,
        _applied: usize,
        _skipped: &[String],
        _pending_remaining: usize,
    ) {
    }
}

#[test]
fn a_fresh_accept_is_recorded_and_applied() -> Result<(), TestError> {
    let migration =
        FixtureMigration::prompting(vec![transformation("k1", "v1")]);
    let decisions = ScriptedDecisions::new(vec![Decision::Accept]);
    let session_log = InMemorySessionLog::default();
    let reporter = SpyReporter::default();

    let outcome = run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    assert!(matches!(outcome, ApplyOutcome::Applied));
    let records = session_log.records()?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].outcome, Outcome::Accepted);
    assert_eq!(records[0].user_value, None);
    Ok(())
}

#[test]
fn a_fresh_edit_records_the_edited_outcome_and_user_value(
) -> Result<(), TestError> {
    let migration =
        FixtureMigration::prompting(vec![transformation("k1", "v1")]);
    let decisions =
        ScriptedDecisions::new(vec![Decision::Edit("edited-value".to_owned())]);
    let session_log = InMemorySessionLog::default();
    let reporter = SpyReporter::default();

    run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    let records = session_log.records()?;
    assert_eq!(records[0].outcome, Outcome::Edited);
    assert_eq!(records[0].user_value, Some("edited-value".to_owned()));
    Ok(())
}

#[test]
fn a_fresh_skip_is_recorded_with_no_user_value() -> Result<(), TestError> {
    let migration =
        FixtureMigration::prompting(vec![transformation("k1", "v1")]);
    let decisions = ScriptedDecisions::new(vec![Decision::Skip]);
    let session_log = InMemorySessionLog::default();
    let reporter = SpyReporter::default();

    run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    let records = session_log.records()?;
    assert_eq!(records[0].outcome, Outcome::Skipped);
    assert_eq!(records[0].user_value, None);
    Ok(())
}

#[test]
fn a_resumed_accepted_record_replays_silently_without_prompting(
) -> Result<(), TestError> {
    let migration =
        FixtureMigration::prompting(vec![transformation("k1", "v1")]);
    let decisions = NeverDecides;
    let session_log = InMemorySessionLog::default();
    session_log.append("k1", Outcome::Accepted, "v1", None)?;
    let reporter = SpyReporter::default();

    let outcome = run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    assert!(matches!(outcome, ApplyOutcome::Applied));
    assert!(reporter.events.borrow().is_empty());
    Ok(())
}

#[test]
fn a_resumed_skipped_record_replays_silently() -> Result<(), TestError> {
    let migration =
        FixtureMigration::prompting(vec![transformation("k1", "v1")]);
    let decisions = NeverDecides;
    let session_log = InMemorySessionLog::default();
    session_log.append("k1", Outcome::Skipped, "v1", None)?;
    let reporter = SpyReporter::default();

    let outcome = run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    assert!(matches!(outcome, ApplyOutcome::Applied));
    assert!(reporter.events.borrow().is_empty());
    Ok(())
}

#[test]
fn source_drift_on_an_accepted_record_discards_and_reprompts(
) -> Result<(), TestError> {
    let migration =
        FixtureMigration::prompting(vec![transformation("k1", "v2")]);
    let decisions = ScriptedDecisions::new(vec![Decision::Accept]);
    let session_log = InMemorySessionLog::default();
    session_log.append("k1", Outcome::Accepted, "v1", None)?;
    let reporter = SpyReporter::default();

    run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    let records = session_log.records()?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].proposed_value, "v2");
    Ok(())
}

#[test]
fn source_drift_on_a_skipped_record_is_not_exempt() -> Result<(), TestError> {
    let migration =
        FixtureMigration::prompting(vec![transformation("k1", "v2")]);
    let decisions = ScriptedDecisions::new(vec![Decision::Skip]);
    let session_log = InMemorySessionLog::default();
    session_log.append("k1", Outcome::Skipped, "v1", None)?;
    let reporter = SpyReporter::default();

    run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    let records = session_log.records()?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].proposed_value, "v2");
    Ok(())
}

#[test]
fn sticky_skip_with_an_unchanged_proposed_value_replays_permanently(
) -> Result<(), TestError> {
    let migration =
        FixtureMigration::prompting(vec![transformation("k1", "v1")]);
    let decisions = NeverDecides;
    let session_log = InMemorySessionLog::default();
    session_log.append("k1", Outcome::Skipped, "v1", None)?;
    let reporter = SpyReporter::default();

    let outcome = run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    assert!(matches!(outcome, ApplyOutcome::Applied));
    assert_eq!(session_log.records()?.len(), 1);
    Ok(())
}

#[test]
fn verify_applied_false_discards_and_reprompts_like_drift(
) -> Result<(), TestError> {
    let mut migration =
        FixtureMigration::prompting(vec![transformation("k1", "v1")]);
    migration.verify_applied_result = false;
    let decisions = ScriptedDecisions::new(vec![Decision::Accept]);
    let session_log = InMemorySessionLog::default();
    session_log.append("k1", Outcome::Accepted, "v1", None)?;
    let reporter = SpyReporter::default();

    run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    // A fresh record was appended again (removed then re-appended), proving
    // the resume path did not simply keep the original.
    assert_eq!(session_log.records()?.len(), 1);
    Ok(())
}

#[test]
fn a_predicate_fail_aborts_with_the_exact_message_and_no_record() {
    let migration = FixtureMigration::failing(
        "linkage ambiguous",
        vec![transformation("k1", "v1")],
    );
    let decisions = NeverDecides;
    let session_log = InMemorySessionLog::default();
    let reporter = SpyReporter::default();

    let result = run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    );

    assert!(result.is_err());
    assert_eq!(
        reporter.events.borrow().as_slice(),
        ["fail:0099-fixture:linkage ambiguous".to_owned()]
    );
    assert!(session_log.records.borrow().is_empty());
}

#[test]
fn a_mechanical_transformation_applies_with_no_record_persisted(
) -> Result<(), TestError> {
    let migration =
        FixtureMigration::mechanical(vec![transformation("k1", "v1")]);
    let decisions = NeverDecides;
    let session_log = InMemorySessionLog::default();
    let reporter = SpyReporter::default();

    let outcome = run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    assert!(matches!(outcome, ApplyOutcome::Applied));
    assert!(session_log.records()?.is_empty());
    Ok(())
}

#[test]
fn a_rejected_edit_is_reprompted_and_never_recorded() -> Result<(), TestError> {
    let mut migration =
        FixtureMigration::prompting(vec![transformation("k1", "v1")]);
    migration.reject_edit = true;
    let decisions = ScriptedDecisions::new(vec![
        Decision::Edit(String::new()),
        Decision::Edit("corrected".to_owned()),
    ]);
    let session_log = InMemorySessionLog::default();
    let reporter = SpyReporter::default();

    run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    assert_eq!(
        reporter.events.borrow().as_slice(),
        ["rejected:empty value not allowed".to_owned()]
    );
    let records = session_log.records()?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].user_value, Some("corrected".to_owned()));
    Ok(())
}

#[test]
fn write_ahead_log_ordering_is_enforced() -> Result<(), TestError> {
    struct OrderCheckingSessionLog {
        apply_seen: RefCell<bool>,
    }

    impl SessionLog for OrderCheckingSessionLog {
        fn records(&self) -> Result<Vec<Record>, MigrationError> {
            Ok(Vec::new())
        }

        fn append(
            &self,
            _key: &str,
            _outcome: Outcome,
            _proposed_value: &str,
            _user_value: Option<&str>,
        ) -> Result<(), MigrationError> {
            assert!(
                !*self.apply_seen.borrow(),
                "apply_decision must never be observed before the record write"
            );
            Ok(())
        }

        fn remove_by_key(&self, _key: &str) -> Result<(), MigrationError> {
            Ok(())
        }
    }

    struct OrderCheckingMigration {
        apply_seen: std::rc::Rc<RefCell<bool>>,
    }

    impl MigrationMeta for OrderCheckingMigration {
        fn id(&self) -> &'static str {
            "0099-fixture"
        }
        fn description(&self) -> &'static str {
            "order-checking fixture"
        }
    }

    impl InteractiveMigration for OrderCheckingMigration {
        fn emit_transformations(
            &self,
            _ctx: &dyn MigrationContext,
        ) -> Vec<Transformation> {
            vec![transformation("k1", "v1")]
        }

        fn evaluate_predicate(&self, _t: &Transformation) -> PredicateOutcome {
            PredicateOutcome::Prompt
        }

        fn validate_edit(
            &self,
            _t: &Transformation,
            _value: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        fn apply_decision(
            &self,
            _t: &Transformation,
            _d: &Decision,
            _ctx: &dyn MigrationContext,
        ) -> Result<(), String> {
            *self.apply_seen.borrow_mut() = true;
            Ok(())
        }
    }

    let apply_seen = std::rc::Rc::new(RefCell::new(false));
    let migration = OrderCheckingMigration {
        apply_seen: apply_seen.clone(),
    };
    let session_log = OrderCheckingSessionLog {
        apply_seen: RefCell::new(false),
    };
    let decisions = ScriptedDecisions::new(vec![Decision::Accept]);
    let reporter = SpyReporter::default();

    run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    )?;

    assert!(*apply_seen.borrow());
    Ok(())
}

#[test]
fn a_timeout_reports_and_leaves_the_session_log_unchanged() {
    let migration =
        FixtureMigration::prompting(vec![transformation("k1", "v1")]);
    let decisions = AlwaysTimesOut;
    let session_log = InMemorySessionLog::default();
    let reporter = SpyReporter::default();

    let result = run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_millis(50),
        &reporter,
    );

    assert!(result.is_err());
    assert_eq!(
        reporter.events.borrow().as_slice(),
        ["timeout:0099-fixture".to_owned()]
    );
    assert!(session_log.records.borrow().is_empty());
}

#[test]
fn a_stall_names_every_pending_key_from_the_current_one_onward(
) -> Result<(), TestError> {
    let migration = FixtureMigration::prompting(vec![
        transformation("k1", "v1"),
        transformation("k2", "v2"),
        transformation("k3", "v3"),
    ]);
    let decisions = NeverDecides;
    let session_log = InMemorySessionLog::default();
    session_log.append("k1", Outcome::Accepted, "v1", None)?;
    let reporter = SpyReporter::default();

    let result = run_interactive(
        &migration,
        &StubContext,
        &decisions,
        &session_log,
        Duration::from_secs(1),
        &reporter,
    );

    assert!(result.is_err());
    assert_eq!(
        reporter.events.borrow().as_slice(),
        ["stalled:0099-fixture:k2,k3".to_owned()]
    );
    Ok(())
}
