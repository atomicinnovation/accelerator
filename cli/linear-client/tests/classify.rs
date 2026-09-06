//! The per-operation retry table keyed on the wire outcome, with a coverage
//! guard so an outcome added without an assertion fails the build.

#![allow(clippy::expect_used, clippy::panic)]

use linear_client::classify::{carries_errors, classify_errors};
use linear_client::{classify, GraphQlError, Operation, Outcome};
use serde_json::json;
use serde_json::Value;
use tracker::TrackerError;

const fn is_retryable(error: &TrackerError) -> bool {
    matches!(*error, TrackerError::Retryable { .. })
}

/// One row of the table: the observed condition and whether each mutating
/// operation can prove nothing was applied.
struct Row {
    outcome: Outcome,
    create_retryable: bool,
    update_retryable: bool,
    note: &'static str,
}

const TABLE: &[Row] = &[
    Row {
        outcome: Outcome::SuccessWithErrors(GraphQlError::Auth),
        create_retryable: true,
        update_retryable: true,
        note: "a 200 carrying an auth error: provably unapplied on both",
    },
    Row {
        outcome: Outcome::SuccessWithErrors(GraphQlError::Complexity),
        create_retryable: true,
        update_retryable: true,
        note: "the query was rejected before executing",
    },
    Row {
        outcome: Outcome::SuccessWithErrors(GraphQlError::RateLimited),
        create_retryable: true,
        update_retryable: false,
        note: "the divergence: a 200-body error may mean the update applied",
    },
    Row {
        outcome: Outcome::SuccessWithErrors(GraphQlError::BadRequest),
        create_retryable: true,
        update_retryable: false,
        note: "the divergence: a 200-body error may mean the update applied",
    },
    Row {
        outcome: Outcome::NonJsonBody,
        create_retryable: false,
        update_retryable: false,
        note: "the response was lost, so the mutation may have applied",
    },
    Row {
        outcome: Outcome::Unauthorised,
        create_retryable: true,
        update_retryable: true,
        note: "HTTP 401",
    },
    Row {
        outcome: Outcome::BadRequest(GraphQlError::Auth),
        create_retryable: true,
        update_retryable: true,
        note: "a 400 whose body classifies as auth",
    },
    Row {
        outcome: Outcome::BadRequest(GraphQlError::Complexity),
        create_retryable: true,
        update_retryable: true,
        note: "the complexity cap",
    },
    Row {
        outcome: Outcome::BadRequest(GraphQlError::RateLimited),
        create_retryable: true,
        update_retryable: true,
        note: "rate limiting arrives as HTTP 400, not 429",
    },
    Row {
        outcome: Outcome::BadRequest(GraphQlError::BadRequest),
        create_retryable: true,
        update_retryable: false,
        note: "the same divergence as the 200-body case",
    },
    Row {
        outcome: Outcome::ServerError,
        create_retryable: false,
        update_retryable: false,
        note: "a 5xx with retries exhausted",
    },
    Row {
        outcome: Outcome::Transport,
        create_retryable: false,
        update_retryable: false,
        note: "connect, DNS and timeout collapse into one code",
    },
    Row {
        outcome: Outcome::Unexpected,
        create_retryable: false,
        update_retryable: false,
        note: "any other status",
    },
];

#[test]
fn every_row_classifies_per_operation() {
    let mut asserted = 0;
    for row in TABLE {
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
fn the_table_covers_every_outcome_variant() {
    let present = |wanted: &Outcome| {
        TABLE.iter().any(|row| {
            std::mem::discriminant(&row.outcome)
                == std::mem::discriminant(wanted)
        })
    };
    for witness in [
        Outcome::SuccessWithErrors(GraphQlError::Auth),
        Outcome::NonJsonBody,
        Outcome::Unauthorised,
        Outcome::BadRequest(GraphQlError::Auth),
        Outcome::ServerError,
        Outcome::Transport,
        Outcome::Unexpected,
    ] {
        // An empty exhaustive match: a new `Outcome` variant fails to compile
        // here, forcing a witness and a table row for it.
        match witness {
            Outcome::SuccessWithErrors(_)
            | Outcome::NonJsonBody
            | Outcome::Unauthorised
            | Outcome::BadRequest(_)
            | Outcome::ServerError
            | Outcome::Transport
            | Outcome::Unexpected => {}
        }
        assert!(present(&witness), "no row covers {witness:?}");
    }
}

#[test]
fn the_two_hundred_body_auth_row_is_retryable_on_update() {
    // Auth is retryable on update; only the plain body error is the terminal
    // 200-body case. Making auth terminal would tell the caller a
    // provably-unapplied auth rejection may have mutated the remote, and change
    // a push failure's exit code from 70 to 71.
    let error = classify(
        Outcome::SuccessWithErrors(GraphQlError::Auth),
        Operation::Update,
        "detail",
    );

    assert!(is_retryable(&error), "{error}");
}

#[test]
fn both_directions_of_the_divergence_are_reproduced() {
    // The body-error outcomes run one way: retryable on create, terminal on
    // update.
    for outcome in [
        Outcome::BadRequest(GraphQlError::BadRequest),
        Outcome::SuccessWithErrors(GraphQlError::BadRequest),
        Outcome::SuccessWithErrors(GraphQlError::RateLimited),
    ] {
        assert!(
            is_retryable(&classify(outcome, Operation::Create, "d")),
            "{outcome:?} on create"
        );
        assert!(
            !is_retryable(&classify(outcome, Operation::Update, "d")),
            "{outcome:?} on update"
        );
    }
    // The auth, complexity and 400-rate-limit outcomes are retryable on both.
    for outcome in [
        Outcome::SuccessWithErrors(GraphQlError::Auth),
        Outcome::Unauthorised,
        Outcome::BadRequest(GraphQlError::Auth),
        Outcome::SuccessWithErrors(GraphQlError::Complexity),
        Outcome::BadRequest(GraphQlError::Complexity),
        Outcome::BadRequest(GraphQlError::RateLimited),
    ] {
        assert!(
            is_retryable(&classify(outcome, Operation::Create, "d")),
            "{outcome:?} on create"
        );
        assert!(
            is_retryable(&classify(outcome, Operation::Update, "d")),
            "{outcome:?} on update"
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

#[test]
fn a_classification_names_the_provider_and_operation() {
    let error = classify(Outcome::Transport, Operation::Read, "ENG-1");
    let TrackerError::Retryable { detail } = error else {
        panic!("a read is retryable");
    };
    assert!(detail.contains("linear read"), "{detail}");
    assert!(detail.contains("ENG-1"), "{detail}");
}

#[test]
fn a_body_with_no_errors_carries_none_whatever_its_shape() {
    let value: Value = json!({"data": {"issue": null}});
    assert!(!carries_errors(&value));
}
