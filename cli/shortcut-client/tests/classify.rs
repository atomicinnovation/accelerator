//! Per-operation classification, table-driven: every outcome on a read and on
//! each write, with and without an earlier attempt that may have applied.

#![allow(clippy::expect_used, clippy::panic)]

use shortcut_client::classify::classify;
use shortcut_client::classify::explain;
use shortcut_client::classify::Failure;
use shortcut_client::classify::Operation;
use shortcut_client::classify::Outcome;
use shortcut_client::transport::Received;
use tracker::TrackerError;

const WRITES: [Operation; 5] = [
    Operation::Create,
    Operation::Update,
    Operation::Comment,
    Operation::Transition,
    Operation::Attach,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Retryable,
    Terminal,
    Rejected,
}

fn class_of(error: &TrackerError) -> Class {
    match error {
        TrackerError::Retryable { .. } => Class::Retryable,
        TrackerError::Terminal { .. } => Class::Terminal,
        TrackerError::Rejected { .. } => Class::Rejected,
        TrackerError::Unconfigured { .. } => {
            panic!("classify never says unconfigured")
        }
    }
}

fn classed(
    outcome: Outcome,
    operation: Operation,
    may_have_applied: bool,
) -> Class {
    class_of(&classify(
        Failure {
            outcome,
            may_have_applied,
        },
        operation,
        "detail",
    ))
}

#[test]
fn every_failed_read_is_retryable() {
    for outcome in [
        Outcome::NotSent,
        Outcome::RequestInvalid,
        Outcome::Status(403),
        Outcome::Status(500),
        Outcome::Status(418),
        Outcome::Transport,
        Outcome::NonJsonBody,
        Outcome::UnusableBody,
    ] {
        for applied in [false, true] {
            assert_eq!(
                classed(outcome, Operation::Read, applied),
                Class::Retryable,
                "{outcome:?} applied={applied}"
            );
        }
    }
}

#[test]
fn a_write_the_tracker_provably_rejected_is_retryable() {
    for status in [400, 401, 403, 404, 409, 410, 422, 429] {
        for operation in WRITES {
            assert_eq!(
                classed(Outcome::Status(status), operation, false),
                Class::Retryable,
                "{status} on {operation:?}"
            );
        }
    }
}

#[test]
fn an_unsent_write_is_retryable() {
    for operation in WRITES {
        assert_eq!(
            classed(Outcome::NotSent, operation, false),
            Class::Retryable
        );
    }
}

#[test]
fn a_write_refused_before_sending_is_rejected() {
    for operation in WRITES {
        assert_eq!(
            classed(Outcome::RequestInvalid, operation, false),
            Class::Rejected
        );
    }
}

#[test]
fn a_write_that_may_have_applied_is_terminal() {
    for outcome in [
        Outcome::Status(500),
        Outcome::Status(503),
        Outcome::Status(418),
        Outcome::Transport,
        Outcome::NonJsonBody,
        Outcome::UnusableBody,
    ] {
        for operation in WRITES {
            assert_eq!(
                classed(outcome, operation, false),
                Class::Terminal,
                "{outcome:?} on {operation:?}"
            );
        }
    }
}

#[test]
fn an_earlier_attempt_that_may_have_applied_makes_any_write_terminal() {
    for outcome in
        [Outcome::Status(429), Outcome::Status(403), Outcome::NotSent]
    {
        assert_eq!(
            classed(outcome, Operation::Update, true),
            Class::Terminal,
            "{outcome:?}"
        );
    }
}

#[test]
fn the_detail_names_the_operation() {
    let error = classify(
        Failure {
            outcome: Outcome::Status(500),
            may_have_applied: false,
        },
        Operation::Create,
        "boom",
    );

    let TrackerError::Terminal { detail } = error else {
        panic!("a 5xx create is terminal");
    };
    assert!(detail.starts_with("shortcut create: "), "{detail}");
    assert!(detail.ends_with("boom"), "{detail}");
}

fn received(status: u16, body: &str) -> Received {
    Received {
        status,
        body: body.to_owned(),
    }
}

#[test]
fn a_401_explains_where_the_token_comes_from() {
    let detail = explain(&received(401, r#"{"message":"Unauthorized"}"#), &[]);

    assert!(detail.contains("401"), "{detail}");
    assert!(detail.contains("ACCELERATOR_SHORTCUT_TOKEN"), "{detail}");
    assert!(
        detail.contains("https://app.shortcut.com/settings/account/api-tokens"),
        "{detail}"
    );
}

#[test]
fn a_403_on_a_read_only_token_says_it_cannot_write() {
    let read_only = ["read".to_owned()];

    let detail = explain(&received(403, "{}"), &read_only);

    assert!(detail.contains("403"), "{detail}");
    assert!(detail.contains("read-only token cannot write"), "{detail}");
}

#[test]
fn a_403_on_a_writing_token_names_the_missing_scope() {
    let scopes = ["read".to_owned(), "write".to_owned()];

    let detail =
        explain(&received(403, r#"{"message":"needs admin"}"#), &scopes);

    assert!(detail.contains("lacks the scope"), "{detail}");
    assert!(detail.contains("needs admin"), "{detail}");
    assert!(!detail.contains("read-only"), "{detail}");
}

#[test]
fn an_error_envelope_supplies_the_message_and_tag() {
    let detail = explain(
        &received(422, r#"{"message":"name too long","tag":"invalid"}"#),
        &[],
    );

    assert!(detail.contains("422"), "{detail}");
    assert!(detail.contains("name too long"), "{detail}");
    assert!(detail.contains("invalid"), "{detail}");
}

#[test]
fn a_body_without_an_envelope_still_explains_the_status() {
    for body in ["{}", "<html>proxy error</html>", ""] {
        let detail = explain(&received(502, body), &[]);
        assert!(detail.contains("502"), "{body:?}: {detail}");
    }
}
