//! The language-neutral classification corpus, asserted against the Rust
//! classifier. The same fixture drives the JavaScript classifier in the design
//! skill's Playwright daemon, so a case failing on either side means the two
//! implementations have drifted.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use network::host_reach;
use network::Host;
use network::HostError;
use network::HostReach;

use serde_json::Value;

type TestError = Box<dyn std::error::Error>;

const EMBEDDED_FORMS: [&str; 6] = [
    "6to4",
    "teredo",
    "nat64",
    "ipv4-mapped",
    "ipv4-compatible",
    "ipv4-translated",
];

const fn reach_token(reach: HostReach) -> &'static str {
    match reach {
        HostReach::Loopback => "loopback",
        HostReach::Private => "private",
        HostReach::LinkLocal => "link-local",
        HostReach::Reserved => "reserved",
        HostReach::Unspecified => "unspecified",
        HostReach::Public => "public",
    }
}

const fn error_token(error: &HostError) -> &'static str {
    match error {
        HostError::Userinfo => "userinfo",
        HostError::Empty => "empty",
        HostError::ControlCharacter => "control-character",
        HostError::NumericEncoding => "numeric-encoding",
    }
}

fn corpus() -> Result<Value, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/host-classification-vectors.json");
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn reach_cases(corpus: &Value) -> Result<&Vec<Value>, TestError> {
    corpus["reach"]
        .as_array()
        .ok_or_else(|| "corpus has no `reach` array".into())
}

#[test]
fn every_reach_case_classifies_as_the_corpus_says() -> Result<(), TestError> {
    let corpus = corpus()?;
    let cases = reach_cases(&corpus)?;
    assert!(!cases.is_empty(), "the reach corpus is empty");

    for case in cases {
        let authority = case["authority"]
            .as_str()
            .ok_or("a reach case has no `authority`")?;
        let outcome = Host::canonicalise(authority);

        if let Some(expected) = case["reach"].as_str() {
            let host = outcome.map_err(|error| {
                format!("'{authority}' failed to canonicalise: {error}")
            })?;
            assert_eq!(
                reach_token(host_reach::classify(&host)),
                expected,
                "{authority}"
            );
        } else if let Some(expected) = case["error"].as_str() {
            let error = outcome.err().ok_or_else(|| {
                format!("'{authority}' canonicalised but was expected to fail")
            })?;
            assert_eq!(error_token(&error), expected, "{authority}");
        } else {
            return Err(format!(
                "case for '{authority}' names neither reach nor error"
            )
            .into());
        }
    }
    Ok(())
}

#[test]
fn the_corpus_exercises_every_reach_and_error_branch() -> Result<(), TestError>
{
    let corpus = corpus()?;
    let cases = reach_cases(&corpus)?;

    let reaches: BTreeSet<&str> = cases
        .iter()
        .filter_map(|case| case["reach"].as_str())
        .collect();
    for variant in [
        HostReach::Loopback,
        HostReach::Private,
        HostReach::LinkLocal,
        HostReach::Reserved,
        HostReach::Unspecified,
        HostReach::Public,
    ] {
        assert!(
            reaches.contains(reach_token(variant)),
            "no corpus case reaches {}",
            reach_token(variant)
        );
    }

    let errors: BTreeSet<&str> = cases
        .iter()
        .filter_map(|case| case["error"].as_str())
        .collect();
    for kind in [
        HostError::Userinfo,
        HostError::Empty,
        HostError::ControlCharacter,
        HostError::NumericEncoding,
    ] {
        assert!(
            errors.contains(error_token(&kind)),
            "no corpus case errors with {}",
            error_token(&kind)
        );
    }

    let embedded: BTreeSet<&str> = cases
        .iter()
        .filter_map(|case| case["embedded"].as_str())
        .collect();
    for form in EMBEDDED_FORMS {
        assert!(embedded.contains(form), "no corpus case exercises {form}");
    }
    Ok(())
}
