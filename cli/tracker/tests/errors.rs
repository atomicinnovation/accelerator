//! The error taxonomy: two classes, closed, held against the dispatch codes
//! the remote-tracker protocol defines.
//!
//! The taxonomy is an independent frozen oracle inlined here: 70 and 71 are
//! the two classes the port expresses, and 72/73/74 resolve above it at the
//! composition root selecting the client from `work.integration`. Which class
//! a given wire condition maps to is operation-scoped — see `TrackerError`'s
//! doc comment.

use std::error::Error;

use tracker::TrackerError;

type TestError = Box<dyn Error>;

/// The class a dispatch code maps onto, as the frozen taxonomy records it.
#[derive(Debug, PartialEq, Eq)]
enum Resolution {
    Class(&'static str),
    AboveThePort,
}

struct DispatchCode {
    name: &'static str,
    number: &'static str,
    resolution: Resolution,
}

const fn recorded_codes() -> [DispatchCode; 5] {
    [
        DispatchCode {
            name: "E_DISPATCH_RETRYABLE",
            number: "70",
            resolution: Resolution::Class("Retryable"),
        },
        DispatchCode {
            name: "E_DISPATCH_TERMINAL",
            number: "71",
            resolution: Resolution::Class("Terminal"),
        },
        DispatchCode {
            name: "E_DISPATCH_NOT_AVAILABLE",
            number: "72",
            resolution: Resolution::AboveThePort,
        },
        DispatchCode {
            name: "E_DISPATCH_UNRECOGNISED",
            number: "73",
            resolution: Resolution::AboveThePort,
        },
        DispatchCode {
            name: "E_DISPATCH_UNCONFIGURED",
            number: "74",
            resolution: Resolution::AboveThePort,
        },
    ]
}

/// The variant name `Debug` prints, which is the identifier itself — so a
/// rename propagates here instead of being absorbed by a match arm.
fn class_of(error: &TrackerError) -> String {
    let rendered = format!("{error:?}");
    rendered
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or_default()
        .to_owned()
}

const fn retryable() -> TrackerError {
    TrackerError::Retryable {
        detail: String::new(),
    }
}

const fn terminal() -> TrackerError {
    TrackerError::Terminal {
        detail: String::new(),
    }
}

#[test]
fn each_dispatch_code_maps_onto_the_class_it_names() -> Result<(), TestError> {
    let recorded = recorded_codes();
    for (name, number, expected) in [
        ("E_DISPATCH_RETRYABLE", "70", retryable()),
        ("E_DISPATCH_TERMINAL", "71", terminal()),
    ] {
        let code = recorded
            .iter()
            .find(|code| code.name == name)
            .ok_or_else(|| format!("the taxonomy does not record {name}"))?;
        assert_eq!(
            code.number, number,
            "{name} is the exit code a client reports the class as"
        );
        let Resolution::Class(class) = &code.resolution else {
            return Err(
                format!("{name} does not resolve to a port class").into()
            );
        };
        assert_eq!(
            class_of(&expected).as_str(),
            *class,
            "{name} maps onto the wrong TrackerError class"
        );
    }
    Ok(())
}

#[test]
fn exactly_two_dispatch_codes_reach_the_port() {
    let mapped = recorded_codes()
        .iter()
        .filter(|code| matches!(code.resolution, Resolution::Class(_)))
        .count();
    assert_eq!(
        mapped, 2,
        "the port expresses two classes; every other code must be recorded as \
         resolving above it"
    );
}

#[test]
fn each_class_routes_to_a_distinct_outcome() {
    // A closed-set guard: fails to compile if a variant is added or removed
    // without this match arm list moving with it.
    let outcome = |error: TrackerError| match error {
        TrackerError::Retryable { .. } => "retry",
        TrackerError::Terminal { .. } => "surface",
    };
    assert_eq!(
        outcome(TrackerError::Retryable {
            detail: String::new()
        }),
        "retry"
    );
    assert_eq!(
        outcome(TrackerError::Terminal {
            detail: String::new()
        }),
        "surface"
    );
}

#[test]
fn a_tracker_error_is_usable_as_a_std_error() {
    let boxed: Box<dyn Error> = Box::new(retryable());
    assert!(boxed.source().is_none());
}

#[test]
fn a_retryable_failure_says_nothing_changed_remotely() {
    assert_eq!(
        TrackerError::Retryable {
            detail: "linear: create ENG-2 failed, connection refused"
                .to_owned()
        }
        .to_string(),
        "tracker call failed with no remote change: linear: create ENG-2 \
         failed, connection refused"
    );
}

#[test]
fn a_terminal_failure_says_the_remote_state_is_unknown() {
    assert_eq!(
        TrackerError::Terminal {
            detail: "jira: create PROJ-? failed, response lost".to_owned()
        }
        .to_string(),
        "tracker call failed and a remote change may have applied, so the \
         remote state is unknown: jira: create PROJ-? failed, response lost"
    );
}
