//! `--list`/decisions-file support: `engine::pending_transformations`,
//! `list::list_pending`, and `decisions_file::validate` exercised against a
//! `FixtureMigration` test double and in-memory ports — no real migration
//! 0007 needed for the generic filtering/parsing contract these cover.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use corpus::Outcome;
use corpus::Record;
use migrate::decisions_file;
use migrate::interactive::Decision;
use migrate::interactive::InteractiveMigration;
use migrate::interactive::PredicateOutcome;
use migrate::interactive::Transformation;
use migrate::list::list_pending;
use migrate::ports::CorpusIndex;
use migrate::ports::DocTypeDir;
use migrate::ports::MigrationContext;
use migrate::ports::MigrationError;
use migrate::ports::PreviewEntry;
use migrate::ports::Reporter;
use migrate::ports::SessionLog;
use migrate::ports::SessionLogFactory;
use migrate::registry::MigrationEntry;
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

fn transformation(
    key: &str,
    short_key: &str,
    proposed: &str,
) -> Transformation {
    Transformation {
        key: key.to_owned(),
        path: "meta/work/0050-example-a.md".to_owned(),
        anchor: "body/relates_to".to_owned(),
        proposed: proposed.to_owned(),
        predicate_value: "ambiguous".to_owned(),
        display: format!("display for {key}"),
        extras: vec![("linkage_key".to_owned(), short_key.to_owned())],
    }
}

enum PredicateMode {
    Prompt,
    Fail(&'static str),
}

struct FixtureMigration {
    id: &'static str,
    transformations: Vec<Transformation>,
    predicate: PredicateMode,
}

impl FixtureMigration {
    const fn prompting(
        id: &'static str,
        transformations: Vec<Transformation>,
    ) -> Self {
        Self {
            id,
            transformations,
            predicate: PredicateMode::Prompt,
        }
    }
}

impl MigrationMeta for FixtureMigration {
    fn id(&self) -> &'static str {
        self.id
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
        self.transformations.clone()
    }

    fn evaluate_predicate(
        &self,
        _transformation: &Transformation,
    ) -> PredicateOutcome {
        match &self.predicate {
            PredicateMode::Prompt => PredicateOutcome::Prompt,
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
        if value.is_empty() {
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
        Ok(())
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

struct SharedLog(Rc<InMemorySessionLog>);

impl SessionLog for SharedLog {
    fn records(&self) -> Result<Vec<Record>, MigrationError> {
        self.0.records()
    }

    fn append(
        &self,
        key: &str,
        outcome: Outcome,
        proposed_value: &str,
        user_value: Option<&str>,
    ) -> Result<(), MigrationError> {
        self.0.append(key, outcome, proposed_value, user_value)
    }

    fn remove_by_key(&self, key: &str) -> Result<(), MigrationError> {
        self.0.remove_by_key(key)
    }
}

struct SingleLog(Rc<InMemorySessionLog>);

impl SessionLogFactory for SingleLog {
    fn for_migration(&self, _id: &str) -> Box<dyn SessionLog> {
        Box::new(SharedLog(Rc::clone(&self.0)))
    }
}

#[derive(Default)]
struct NoOpReporter;

impl Reporter for NoOpReporter {
    fn preview(&self, _pending: &[PreviewEntry<'_>]) {}
    fn no_pending_migrations(&self, _skipped: &[String]) {}
    fn unknown_applied_id(&self, _id: &str) {}
    fn unknown_skipped_id(&self, _id: &str) {}
    fn applied_and_skipped(&self, _id: &str) {}
    fn migration_running(&self, _id: &str) {}
    fn migration_applied(&self, _id: &str) {}
    fn migration_no_op(&self, _id: &str) {}
    fn migration_failed(&self, _id: &str, _error: &MigrationError) {}
    fn interactive_fail(&self, _id: &str, _message: &str) {}
    fn interactive_validation_rejected(&self, _message: &str) {}
    fn interactive_stalled(&self, _id: &str, _pending_keys: &[String]) {}
    fn interactive_timeout(&self, _id: &str) {}
    fn summary(
        &self,
        _applied: usize,
        _skipped: &[String],
        _pending_remaining: usize,
    ) {
    }
}

#[test]
fn list_pending_excludes_a_decided_key_and_reports_the_rest(
) -> Result<(), TestError> {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![
            transformation("k1", "relates_to", "work-item:0042"),
            transformation("k2", "parent", "work-item:0031"),
        ],
    );
    let entries = vec![MigrationEntry::Interactive(Box::new(migration))];
    let session_log = Rc::new(InMemorySessionLog::default());
    session_log.append("k1", Outcome::Accepted, "work-item:0042", None)?;
    let session_logs = SingleLog(session_log);

    let groups = list_pending(
        &entries,
        &[],
        &[],
        &StubContext,
        &session_logs,
        &NoOpReporter,
    )?;

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].id, "0099-fixture");
    assert_eq!(groups[0].transformations.len(), 1);
    assert_eq!(groups[0].transformations[0].key, "k2");
    assert!(groups[0].had_in_flight_session);
    Ok(())
}

#[test]
fn list_pending_refuses_a_proposed_value_carrying_an_embedded_tab() {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![transformation("k1", "relates_to", "work-item:0042\tsneaky")],
    );
    let entries = vec![MigrationEntry::Interactive(Box::new(migration))];
    let session_log = Rc::new(InMemorySessionLog::default());
    let session_logs = SingleLog(session_log);

    let result = list_pending(
        &entries,
        &[],
        &[],
        &StubContext,
        &session_logs,
        &NoOpReporter,
    );

    assert_eq!(
        result.err().map(|error| error.to_string()),
        Some(
            "[0099-fixture] --list field for key 'relates_to' contains a \
             tab or newline; --list output is undefined for such values."
                .to_owned()
        )
    );
}

#[test]
fn list_pending_refuses_a_key_carrying_an_embedded_newline() {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![transformation("k1", "relates_to\nsneaky", "work-item:0042")],
    );
    let entries = vec![MigrationEntry::Interactive(Box::new(migration))];
    let session_log = Rc::new(InMemorySessionLog::default());
    let session_logs = SingleLog(session_log);

    let result = list_pending(
        &entries,
        &[],
        &[],
        &StubContext,
        &session_logs,
        &NoOpReporter,
    );

    assert!(result.is_err());
}

#[test]
fn list_pending_reports_no_in_flight_session_when_the_log_is_empty(
) -> Result<(), TestError> {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![transformation("k1", "relates_to", "work-item:0042")],
    );
    let entries = vec![MigrationEntry::Interactive(Box::new(migration))];
    let session_log = Rc::new(InMemorySessionLog::default());
    let session_logs = SingleLog(session_log);

    let groups = list_pending(
        &entries,
        &[],
        &[],
        &StubContext,
        &session_logs,
        &NoOpReporter,
    )?;

    assert_eq!(groups.len(), 1);
    assert!(!groups[0].had_in_flight_session);
    Ok(())
}

struct TwoLogs {
    a: Rc<InMemorySessionLog>,
    b: Rc<InMemorySessionLog>,
}

impl SessionLogFactory for TwoLogs {
    fn for_migration(&self, id: &str) -> Box<dyn SessionLog> {
        if id == "0098-fixture" {
            Box::new(SharedLog(Rc::clone(&self.a)))
        } else {
            Box::new(SharedLog(Rc::clone(&self.b)))
        }
    }
}

#[test]
fn two_pending_interactive_migrations_each_get_their_own_list_group(
) -> Result<(), TestError> {
    let first = FixtureMigration::prompting(
        "0098-fixture",
        vec![transformation("k1", "relates_to", "work-item:0001")],
    );
    let second = FixtureMigration::prompting(
        "0099-fixture",
        vec![
            transformation("k2", "parent", "work-item:0002"),
            transformation("k3", "relates_to", "work-item:0003"),
        ],
    );
    let entries = vec![
        MigrationEntry::Interactive(Box::new(first)),
        MigrationEntry::Interactive(Box::new(second)),
    ];
    let session_logs = TwoLogs {
        a: Rc::new(InMemorySessionLog::default()),
        b: Rc::new(InMemorySessionLog::default()),
    };

    let groups = list_pending(
        &entries,
        &[],
        &[],
        &StubContext,
        &session_logs,
        &NoOpReporter,
    )?;

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].id, "0098-fixture");
    assert_eq!(groups[0].transformations.len(), 1);
    assert_eq!(groups[0].transformations[0].key, "k1");
    assert_eq!(groups[1].id, "0099-fixture");
    assert_eq!(groups[1].transformations.len(), 2);
    assert_eq!(groups[1].transformations[0].key, "k2");
    assert_eq!(groups[1].transformations[1].key, "k3");
    Ok(())
}

#[test]
fn list_pending_propagates_a_fail_predicate() {
    let migration = FixtureMigration {
        id: "0099-fixture",
        transformations: vec![transformation("k1", "relates_to", "v1")],
        predicate: PredicateMode::Fail("boom"),
    };
    let entries = vec![MigrationEntry::Interactive(Box::new(migration))];
    let session_log = Rc::new(InMemorySessionLog::default());
    let session_logs = SingleLog(session_log);

    let result = list_pending(
        &entries,
        &[],
        &[],
        &StubContext,
        &session_logs,
        &NoOpReporter,
    );

    assert!(result.is_err());
}

#[test]
fn a_drifted_record_still_needs_a_decision_and_is_not_a_false_surplus(
) -> Result<(), TestError> {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![
            transformation("k1", "relates_to", "v1"),
            transformation("parent", "parent", "work-item:0031"),
            transformation("k3", "relates_to", "v3"),
        ],
    );
    let session_log = Rc::new(InMemorySessionLog::default());
    // A stale, drifted proposed_value — the live emission now proposes
    // work-item:0031, not this record's work-item:9999-STALE.
    session_log.append(
        "parent",
        Outcome::Skipped,
        "work-item:9999-STALE",
        None,
    )?;
    let list_entries = vec![MigrationEntry::Interactive(Box::new(
        FixtureMigration::prompting(
            "0099-fixture",
            vec![
                transformation("k1", "relates_to", "v1"),
                transformation("parent", "parent", "work-item:0031"),
                transformation("k3", "relates_to", "v3"),
            ],
        ),
    ))];
    let session_logs = SingleLog(session_log);

    let groups = list_pending(
        &list_entries,
        &[],
        &[],
        &StubContext,
        &session_logs,
        &NoOpReporter,
    )?;

    assert_eq!(groups.len(), 1);
    let keys: Vec<&str> = groups[0]
        .transformations
        .iter()
        .map(|t| t.key.as_str())
        .collect();
    assert_eq!(
        keys.len(),
        3,
        "the drifted record must not be filtered out as already-decided: \
         {keys:?}"
    );

    decisions_file::validate(
        &migration,
        &groups[0].transformations,
        "accept\nskip\nedit work-item:0100\n",
    )?;
    Ok(())
}

fn prompts_for(migration: &FixtureMigration) -> Vec<Transformation> {
    migration.emit_transformations(&StubContext)
}

#[test]
fn a_complete_accept_skip_edit_sequence_validates() -> Result<(), TestError> {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![
            transformation("k1", "relates_to", "v1"),
            transformation("k2", "parent", "v2"),
            transformation("k3", "relates_to", "v3"),
        ],
    );
    let prompts = prompts_for(&migration);

    decisions_file::validate(
        &migration,
        &prompts,
        "accept\nskip\nedit work-item:0099\n",
    )?;
    Ok(())
}

#[test]
fn blank_lines_and_crlf_are_tolerated() -> Result<(), TestError> {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![transformation("k1", "relates_to", "v1")],
    );
    let prompts = prompts_for(&migration);

    decisions_file::validate(&migration, &prompts, "\r\n\naccept\r\n")?;
    Ok(())
}

fn assert_validation_error(
    migration: &FixtureMigration,
    prompts: &[Transformation],
    content: &str,
    expected: &str,
) {
    let result = decisions_file::validate(migration, prompts, content);
    assert_eq!(
        result.err().map(|error| error.to_string()),
        Some(expected.to_owned())
    );
}

#[test]
fn a_hash_prefixed_line_is_an_unknown_verb_not_a_comment() {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![transformation("k1", "relates_to", "v1")],
    );
    let prompts = prompts_for(&migration);

    assert_validation_error(
        &migration,
        &prompts,
        "# a comment line\n",
        "Error: decisions file position 1: unknown verb '# a comment line' \
         (expected accept | skip | edit <value>).",
    );
}

#[test]
fn too_few_decisions_names_the_missing_position_and_short_key() {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![
            transformation("k1", "relates_to", "v1"),
            transformation("k2", "parent", "v2"),
            transformation("k3", "relates_to", "v3"),
        ],
    );
    let prompts = prompts_for(&migration);

    assert_validation_error(
        &migration,
        &prompts,
        "accept\nskip\n",
        "Error: decisions file is missing a decision for position 3 \
         (relates_to).",
    );
}

#[test]
fn too_many_decisions_names_the_surplus_position() {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![transformation("k1", "relates_to", "v1")],
    );
    let prompts = prompts_for(&migration);

    assert_validation_error(
        &migration,
        &prompts,
        "accept\naccept\n",
        "Error: decisions file has a surplus decision at position 2 (only \
         1 transformation(s) require one).",
    );
}

#[test]
fn an_unknown_verb_names_its_position() {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![
            transformation("k1", "relates_to", "v1"),
            transformation("k2", "parent", "v2"),
        ],
    );
    let prompts = prompts_for(&migration);

    assert_validation_error(
        &migration,
        &prompts,
        "accept\nfrobnicate\n",
        "Error: decisions file position 2: unknown verb 'frobnicate' \
         (expected accept | skip | edit <value>).",
    );
}

#[test]
fn a_rejected_edit_with_no_recovery_line_fails_naming_the_reason() {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![transformation("k1", "relates_to", "v1")],
    );
    let prompts = prompts_for(&migration);

    assert_validation_error(
        &migration,
        &prompts,
        "edit\n",
        "Error: decisions file position 1: edit rejected (empty value not \
         allowed) and no further decision to recover.",
    );
}

#[test]
fn a_rejected_edit_recovers_from_the_next_line() -> Result<(), TestError> {
    let migration = FixtureMigration::prompting(
        "0099-fixture",
        vec![transformation("k1", "relates_to", "v1")],
    );
    let prompts = prompts_for(&migration);

    decisions_file::validate(
        &migration,
        &prompts,
        "edit\nedit work-item:0042\n",
    )?;
    Ok(())
}
