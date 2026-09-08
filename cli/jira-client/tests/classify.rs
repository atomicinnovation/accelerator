//! The per-operation retry table, with a coverage guard so an outcome added
//! without an assertion fails the build.

#![allow(clippy::expect_used, clippy::panic)]

use jira_client::classify::{classify, Outcome};
use jira_client::Operation;
use tracker::TrackerError;

const fn is_retryable(error: &TrackerError) -> bool {
    matches!(*error, TrackerError::Retryable { .. })
}

/// One row of the status table: the observed outcome and whether each operation
/// can prove no mutation happened.
struct StatusRow {
    outcome: Outcome,
    create_retryable: bool,
    update_retryable: bool,
}

const STATUS_TABLE: &[StatusRow] = &[
    StatusRow {
        outcome: Outcome::Status(400),
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(401),
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(403),
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(404),
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(410),
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(429),
        create_retryable: true,
        update_retryable: true,
    },
    StatusRow {
        outcome: Outcome::Status(503),
        create_retryable: false,
        update_retryable: false,
    },
    StatusRow {
        outcome: Outcome::NonJsonBody,
        create_retryable: false,
        update_retryable: false,
    },
    StatusRow {
        outcome: Outcome::Transport,
        create_retryable: false,
        update_retryable: false,
    },
    StatusRow {
        outcome: Outcome::Status(302),
        create_retryable: false,
        update_retryable: false,
    },
    StatusRow {
        outcome: Outcome::Status(418),
        create_retryable: false,
        update_retryable: false,
    },
];

#[test]
fn every_status_row_classifies_per_operation() {
    let mut asserted = 0;
    for row in STATUS_TABLE {
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
fn the_status_table_covers_every_outcome_variant() {
    let present = |wanted: &Outcome| {
        STATUS_TABLE.iter().any(|row| {
            std::mem::discriminant(&row.outcome)
                == std::mem::discriminant(wanted)
        })
    };
    for witness in
        [Outcome::Status(0), Outcome::NonJsonBody, Outcome::Transport]
    {
        // An empty exhaustive match: a new `Outcome` variant fails to compile
        // here, forcing a witness and a table row for it.
        match witness {
            Outcome::Status(_) | Outcome::NonJsonBody | Outcome::Transport => {}
        }
        assert!(present(&witness), "no row covers {witness:?}");
    }
}

#[test]
fn a_classification_names_the_provider_and_operation() {
    let error = classify(Outcome::Status(404), Operation::Read, "ABC-1");
    let TrackerError::Retryable { detail } = error else {
        panic!("a read is retryable");
    };
    assert!(detail.contains("jira read"), "{detail}");
    assert!(detail.contains("ABC-1"), "{detail}");
}
