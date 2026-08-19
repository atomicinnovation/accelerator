//! Both providers are held to the one shared identifier rule, driven from the
//! one committed fixture.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;

use linear_client::auth::check_identifier;
use linear_client::ClientError;

fn unescape(raw: &str) -> String {
    let mut out = String::new();
    let mut characters = raw.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            match characters.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('0') => out.push('\0'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(character);
        }
    }
    out
}

/// The fixture lives with the rule, in `tracker-support`.
fn cases() -> Vec<(String, bool)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tracker-support/tests/fixtures/identifiers.txt");
    let raw = std::fs::read_to_string(path)
        .expect("the shared identifier fixture is readable");
    raw.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (verdict, candidate) =
                line.split_once('\t').unwrap_or((line, ""));
            (unescape(candidate), verdict == "accept")
        })
        .collect()
}

#[test]
fn the_shared_fixture_decides_what_this_client_accepts() {
    let cases = cases();
    assert!(!cases.is_empty(), "the fixture must drive the assertions");

    for (candidate, accepted) in cases {
        assert_eq!(
            check_identifier(&candidate).is_ok(),
            accepted,
            "identifier {candidate:?}"
        );
    }
}

#[test]
fn a_refusal_names_the_identifier_and_the_property_that_failed() {
    let error = check_identifier("---ENG-1")
        .expect_err("a YAML document separator is refused");

    assert!(matches!(error, ClientError::BadIdentifier { .. }));
    assert!(error.to_string().contains("---ENG-1"), "{error}");
    assert!(error.to_string().contains("separator"), "{error}");
}
