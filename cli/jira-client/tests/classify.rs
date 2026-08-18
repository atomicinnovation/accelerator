//! Both classification tables, per operation, with a coverage guard on each so
//! a row added without an assertion fails the build.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;

use jira_client::classify::{bash_code, classify, classify_bash_code, Outcome};
use jira_client::Operation;
use tracker::TrackerError;

const fn is_retryable(error: &TrackerError) -> bool {
    matches!(*error, TrackerError::Retryable { .. })
}

/// One row of the status table, transcribed from `jira-request.sh:363-442`:
/// the observed outcome, the bash code it produces, and whether each
/// operation can prove no mutation happened.
struct StatusRow {
    outcome: Outcome,
    code: u16,
    create_retryable: bool,
    update_retryable: bool,
}

const STATUS_TABLE: &[StatusRow] = &[
    StatusRow {
        outcome: Outcome::Status(400),
        code: 34,
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(401),
        code: 11,
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(403),
        code: 12,
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(404),
        code: 13,
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(410),
        code: 14,
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(429),
        code: 19,
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(503),
        code: 20,
        create_retryable: false,
        update_retryable: false,
    },
    StatusRow {
        outcome: Outcome::NonJsonBody,
        code: 16,
        create_retryable: false,
        update_retryable: false,
    },
    StatusRow {
        outcome: Outcome::Transport,
        code: 21,
        create_retryable: false,
        update_retryable: false,
    },
    StatusRow {
        outcome: Outcome::Status(302),
        code: 20,
        create_retryable: false,
        update_retryable: false,
    },
    StatusRow {
        outcome: Outcome::Status(418),
        code: 20,
        create_retryable: false,
        update_retryable: false,
    },
];

#[test]
fn every_status_row_classifies_per_operation() {
    let mut asserted = 0;
    for row in STATUS_TABLE {
        assert_eq!(
            bash_code(row.outcome),
            row.code,
            "{:?} maps to bash code {}",
            row.outcome,
            row.code
        );
        assert_eq!(
            is_retryable(&classify(row.outcome, Operation::Create, "detail")),
            row.create_retryable,
            "create: {:?}",
            row.outcome
        );
        assert_eq!(
            is_retryable(&classify(row.outcome, Operation::Update, "detail")),
            row.update_retryable,
            "update: {:?}",
            row.outcome
        );
        assert!(
            is_retryable(&classify(row.outcome, Operation::Read, "detail")),
            "a read never produces Terminal: {:?}",
            row.outcome
        );
        asserted += 1;
    }
    assert_eq!(
        asserted,
        STATUS_TABLE.len(),
        "every row of the status table must be asserted"
    );
}

#[test]
fn the_status_table_covers_every_condition_the_bash_distinguishes() {
    let covered: Vec<u16> = STATUS_TABLE.iter().map(|row| row.code).collect();
    for code in [34, 11, 12, 13, 14, 19, 20, 16, 21] {
        assert!(
            covered.contains(&code),
            "bash code {code} has no row in the status table"
        );
    }
}

#[test]
fn a_classification_names_the_provider_operation_and_code() {
    let error = classify(Outcome::Status(404), Operation::Read, "ABC-1");
    let TrackerError::Retryable { detail } = error else {
        panic!("a read is retryable");
    };
    assert!(detail.contains("jira read"), "{detail}");
    assert!(detail.contains("(13)"), "{detail}");
    assert!(detail.contains("ABC-1"), "{detail}");
}

/// The committed transcription of the five bash mappers, filtered to Jira.
struct FixtureRow {
    code: u16,
    operation: Operation,
    retryable: bool,
}

fn fixture_rows() -> Vec<FixtureRow> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tracker-support/tests/fixtures/bridge-exit-code-tables.txt");
    let raw = std::fs::read_to_string(path)
        .expect("the committed exit-code table is readable");
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields[1] != "jira" {
                return None;
            }
            Some(FixtureRow {
                code: fields[0].parse().expect("the code is numeric"),
                operation: match fields[2] {
                    "create" => Operation::Create,
                    "update" => Operation::Update,
                    other => panic!("unrecognised operation {other}"),
                },
                retryable: match fields[3] {
                    "retryable" => true,
                    "terminal" => false,
                    other => panic!("unrecognised class {other}"),
                },
            })
        })
        .collect()
}

#[test]
fn every_jira_row_of_the_committed_fixture_is_asserted() {
    let rows = fixture_rows();
    assert_eq!(
        rows.len(),
        43,
        "the fixture's Jira rows are the coverage guard: a row added without \
         an assertion must fail the build"
    );

    let mut consumed = 0;
    for row in &rows {
        assert_eq!(
            is_retryable(&classify_bash_code(row.code, row.operation, "d")),
            row.retryable,
            "bash code {} on {:?}",
            row.code,
            row.operation
        );
        consumed += 1;
    }
    assert_eq!(consumed, rows.len(), "every row present was consumed");
}

#[test]
fn a_read_is_retryable_for_every_code_in_the_fixture() {
    for row in fixture_rows() {
        assert!(
            is_retryable(&classify_bash_code(row.code, Operation::Read, "d")),
            "code {} must degrade a read rather than terminate it",
            row.code
        );
    }
}
