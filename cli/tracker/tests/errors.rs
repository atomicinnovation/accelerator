//! The error taxonomy: two classes, closed, held against the dispatch codes
//! the remote-tracker protocol defines.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

use tracker::TrackerError;

type TestError = Box<dyn Error>;

const ABOVE_THE_PORT: &str = "above-the-port";

fn read(relative: &str) -> Result<String, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()).into())
}

/// The class a dispatch code maps onto, as the fixture records it.
#[derive(Debug, PartialEq, Eq)]
enum Resolution {
    Class(String),
    AboveThePort,
}

#[derive(Debug)]
struct DispatchCode {
    number: String,
    resolution: Resolution,
}

fn codes_recorded_by_the_fixture(
) -> Result<BTreeMap<String, DispatchCode>, TestError> {
    let fixture = read("tests/fixtures/dispatch-codes.txt")?;
    fixture
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (declaration, resolution) = line
                .split_once(' ')
                .ok_or_else(|| format!("malformed fixture row: {line}"))?;
            let (name, number) = declaration
                .split_once('=')
                .ok_or_else(|| format!("malformed fixture row: {line}"))?;
            let resolution = if resolution == ABOVE_THE_PORT {
                Resolution::AboveThePort
            } else {
                Resolution::Class(resolution.to_owned())
            };
            Ok((
                name.to_owned(),
                DispatchCode {
                    number: number.to_owned(),
                    resolution,
                },
            ))
        })
        .collect()
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
    let recorded = codes_recorded_by_the_fixture()?;
    for (name, number, expected) in [
        ("E_DISPATCH_RETRYABLE", "70", retryable()),
        ("E_DISPATCH_TERMINAL", "71", terminal()),
    ] {
        let code = recorded
            .get(name)
            .ok_or_else(|| format!("the fixture does not record {name}"))?;
        assert_eq!(
            code.number, number,
            "{name} is the exit code a client reports the class as"
        );
        assert_eq!(
            code.resolution,
            Resolution::Class(class_of(&expected)),
            "{name} maps onto the wrong TrackerError class"
        );
    }
    Ok(())
}

#[test]
fn exactly_two_dispatch_codes_reach_the_port() -> Result<(), TestError> {
    let recorded = codes_recorded_by_the_fixture()?;
    let mapped = recorded
        .values()
        .filter(|code| matches!(code.resolution, Resolution::Class(_)))
        .count();
    assert_eq!(
        mapped, 2,
        "the port expresses two classes; every other code must be recorded as \
         resolving above it"
    );
    Ok(())
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
