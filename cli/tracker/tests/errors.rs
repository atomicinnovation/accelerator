//! The error taxonomy: two classes, closed, held 1:1 against the bash
//! dispatch codes that remain authoritative until the bridges are retired.

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

/// The name and numeric value one shell line declares, if it declares one.
fn declaration(line: &str) -> Option<(String, String)> {
    let at = line.find("E_DISPATCH_")?;
    let (name, rest) = line[at..].split_once('=')?;
    let value = rest.trim().trim_matches(|c| c == '"' || c == '\'');
    let number: String =
        value.chars().take_while(char::is_ascii_digit).collect();
    if number.is_empty() {
        return None;
    }
    Some((name.trim().to_owned(), number))
}

fn codes_declared_by_the_bash_taxonomy(
) -> Result<BTreeMap<String, String>, TestError> {
    let script = read("../../skills/work/scripts/work-item-bridge-codes.sh")?;
    let mut declared = BTreeMap::new();
    for line in script.lines().map(str::trim) {
        if line.starts_with('#') || !line.contains("E_DISPATCH_") {
            continue;
        }
        let (name, number) = declaration(line).ok_or_else(|| {
            format!("could not read a dispatch declaration from: {line}")
        })?;
        declared.insert(name, number);
    }
    Ok(declared)
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
fn the_fixture_enumerates_exactly_the_codes_the_bash_taxonomy_declares(
) -> Result<(), TestError> {
    let declared = codes_declared_by_the_bash_taxonomy()?;
    let recorded: BTreeMap<String, String> = codes_recorded_by_the_fixture()?
        .into_iter()
        .map(|(name, code)| (name, code.number))
        .collect();
    assert_eq!(
        recorded, declared,
        "the bash dispatch taxonomy and tests/fixtures/dispatch-codes.txt \
         disagree — update the fixture deliberately, and check whether \
         TrackerError's two classes still cover it. A reformatted declaration \
         in work-item-bridge-codes.sh reads the same way here as a changed one."
    );
    Ok(())
}

#[test]
fn each_dispatch_code_maps_onto_the_class_it_names() -> Result<(), TestError> {
    let recorded = codes_recorded_by_the_fixture()?;
    for (name, expected) in [
        ("E_DISPATCH_RETRYABLE", retryable()),
        ("E_DISPATCH_TERMINAL", terminal()),
    ] {
        let code = recorded
            .get(name)
            .ok_or_else(|| format!("the fixture does not record {name}"))?;
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
