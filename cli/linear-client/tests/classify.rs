//! The status-and-body classification table, per operation, with a coverage
//! guard over the committed fixture's Linear rows.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;

use linear_client::classify::{
    bash_code, carries_errors, classify_bash_code, classify_errors,
};
use linear_client::{classify, GraphQlError, Operation, Outcome};
use serde_json::json;
use serde_json::Value;
use tracker::TrackerError;

const fn is_retryable(error: &TrackerError) -> bool {
    matches!(*error, TrackerError::Retryable { .. })
}

/// One row of the transcribed table: the observed condition, the bash code, and
/// whether each mutating operation can prove nothing was applied.
struct Row {
    outcome: Outcome,
    code: u16,
    create_retryable: bool,
    update_retryable: bool,
    note: &'static str,
}

const TABLE: &[Row] = &[
    Row {
        outcome: Outcome::SuccessWithErrors(GraphQlError::Auth),
        code: 11,
        create_retryable: true,
        update_retryable: true,
        note: "a 200 carrying an auth error: provably unapplied on both",
    },
    Row {
        outcome: Outcome::SuccessWithErrors(GraphQlError::Complexity),
        code: 36,
        create_retryable: true,
        update_retryable: true,
        note: "the query was rejected before executing",
    },
    Row {
        outcome: Outcome::SuccessWithErrors(GraphQlError::BadRequest),
        code: 34,
        create_retryable: true,
        update_retryable: false,
        note: "the divergence: a 200-body error may mean the update applied",
    },
    Row {
        outcome: Outcome::NonJsonBody,
        code: 16,
        create_retryable: false,
        update_retryable: false,
        note: "the response was lost, so the mutation may have applied",
    },
    Row {
        outcome: Outcome::Unauthorised,
        code: 11,
        create_retryable: true,
        update_retryable: true,
        note: "HTTP 401",
    },
    Row {
        outcome: Outcome::BadRequest(GraphQlError::Auth),
        code: 11,
        create_retryable: true,
        update_retryable: true,
        note: "a 400 whose body classifies as auth",
    },
    Row {
        outcome: Outcome::BadRequest(GraphQlError::Complexity),
        code: 36,
        create_retryable: true,
        update_retryable: true,
        note: "the complexity cap",
    },
    Row {
        outcome: Outcome::BadRequest(GraphQlError::RateLimited),
        code: 35,
        create_retryable: true,
        update_retryable: true,
        note: "rate limiting arrives as HTTP 400, not 429",
    },
    Row {
        outcome: Outcome::BadRequest(GraphQlError::BadRequest),
        code: 34,
        create_retryable: true,
        update_retryable: false,
        note: "the same divergence as the 200-body case",
    },
    Row {
        outcome: Outcome::ServerError,
        code: 20,
        create_retryable: false,
        update_retryable: false,
        note: "a 5xx with retries exhausted",
    },
    Row {
        outcome: Outcome::Transport,
        code: 21,
        create_retryable: false,
        update_retryable: false,
        note: "connect, DNS and timeout collapse into one code",
    },
    Row {
        outcome: Outcome::Unexpected,
        code: 20,
        create_retryable: false,
        update_retryable: false,
        note: "any other status",
    },
];

#[test]
fn every_row_classifies_per_operation() {
    let mut asserted = 0;
    for row in TABLE {
        assert_eq!(bash_code(row.outcome), row.code, "{}", row.note);
        assert_eq!(
            is_retryable(&classify(row.outcome, Operation::Create, "d")),
            row.create_retryable,
            "create: {}",
            row.note
        );
        assert_eq!(
            is_retryable(&classify(row.outcome, Operation::Update, "d")),
            row.update_retryable,
            "update: {}",
            row.note
        );
        assert!(
            is_retryable(&classify(row.outcome, Operation::Read, "d")),
            "a read never produces Terminal: {}",
            row.note
        );
        asserted += 1;
    }
    assert_eq!(asserted, TABLE.len(), "every row must be asserted");
}

#[test]
fn the_two_hundred_body_auth_row_is_retryable_on_update() {
    // Code 11 is retryable on update; only 34 is the terminal 200-body error.
    // Making this terminal would tell the caller a provably-unapplied auth
    // rejection may have mutated the remote, and change a push failure's exit
    // code from 70 to 71.
    let error = classify(
        Outcome::SuccessWithErrors(GraphQlError::Auth),
        Operation::Update,
        "detail",
    );

    assert!(is_retryable(&error), "{error}");
}

#[test]
fn linear_emits_no_403_404_410_or_429_so_those_codes_are_reserved() {
    // Every reserved code must still classify — a caller can pass one — but
    // no Outcome maps to it, which is the property that matters.
    for code in [12, 13, 14, 15, 17, 19] {
        let mapped = TABLE.iter().any(|row| row.code == code);
        assert!(
            !mapped,
            "code {code} is Jira-only and reserved in Linear's EXIT_CODES.md"
        );
    }
}

#[test]
fn the_error_classifier_orders_auth_before_complexity_before_ratelimit() {
    let auth_and_ratelimit = json!({"errors": [
        {"extensions": {"type": "authentication error"}},
        {"extensions": {"code": "RATELIMITED"}}
    ]});
    assert_eq!(classify_errors(&auth_and_ratelimit), GraphQlError::Auth);

    let complexity_and_ratelimit = json!({"errors": [
        {"message": "Query exceeded COMPLEXITY limit"},
        {"extensions": {"code": "RATELIMITED"}}
    ]});
    assert_eq!(
        classify_errors(&complexity_and_ratelimit),
        GraphQlError::Complexity
    );

    let ratelimited =
        json!({"errors": [{"extensions": {"code": "ratelimited"}}]});
    assert_eq!(classify_errors(&ratelimited), GraphQlError::RateLimited);

    let other = json!({"errors": [{"message": "Field does not exist"}]});
    assert_eq!(classify_errors(&other), GraphQlError::BadRequest);
}

#[test]
fn the_complexity_discriminator_requires_the_full_word() {
    let stem_only = json!({"errors": [{"message": "this is a complex query"}]});

    assert_eq!(
        classify_errors(&stem_only),
        GraphQlError::BadRequest,
        "the bare `complex` stem must not match — only the full word does"
    );
}

#[test]
fn an_auth_code_is_recognised_from_either_type_or_code() {
    for body in [
        json!({"errors": [{"extensions": {"type": "AUTHENTICATION ERROR"}}]}),
        json!({"errors": [{"extensions": {"code": "AUTHENTICATION_ERROR"}}]}),
    ] {
        assert_eq!(classify_errors(&body), GraphQlError::Auth);
    }
}

#[test]
fn an_empty_errors_array_is_not_an_error() {
    assert!(!carries_errors(&json!({"data": {}, "errors": []})));
    assert!(!carries_errors(&json!({"data": {}})));
    assert!(carries_errors(&json!({"errors": [{"message": "x"}]})));
}

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
            if fields[1] != "linear" {
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
fn every_linear_row_of_the_committed_fixture_is_asserted() {
    let rows = fixture_rows();
    assert_eq!(
        rows.len(),
        31,
        "the fixture's Linear rows are the coverage guard: a row added \
         without an assertion must fail the build"
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
    assert_eq!(consumed, rows.len());
}

#[test]
fn both_directions_of_the_divergence_are_reproduced() {
    // 34 runs one way...
    assert!(is_retryable(&classify_bash_code(
        34,
        Operation::Create,
        "d"
    )));
    assert!(!is_retryable(&classify_bash_code(
        34,
        Operation::Update,
        "d"
    )));
    // ...and 18, 23, 25, 27, 29 the other, with no rationale anywhere.
    for code in [18, 23, 25, 27, 29] {
        assert!(
            !is_retryable(&classify_bash_code(code, Operation::Create, "d")),
            "code {code} on create"
        );
        assert!(
            is_retryable(&classify_bash_code(code, Operation::Update, "d")),
            "code {code} on update"
        );
    }
    // 11, 22, 35 and 36 are retryable on both.
    for code in [11, 22, 35, 36] {
        assert!(is_retryable(&classify_bash_code(
            code,
            Operation::Create,
            "d"
        )));
        assert!(is_retryable(&classify_bash_code(
            code,
            Operation::Update,
            "d"
        )));
    }
}

#[test]
fn a_classification_names_the_provider_operation_and_code() {
    let error = classify(Outcome::Transport, Operation::Read, "ENG-1");
    let TrackerError::Retryable { detail } = error else {
        panic!("a read is retryable");
    };
    assert!(detail.contains("linear read"), "{detail}");
    assert!(detail.contains("(21)"), "{detail}");
    assert!(detail.contains("ENG-1"), "{detail}");
}

#[test]
fn a_body_with_no_errors_carries_none_whatever_its_shape() {
    let value: Value = json!({"data": {"issue": null}});
    assert!(!carries_errors(&value));
}
