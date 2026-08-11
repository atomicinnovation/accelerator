//! The value types hold their bytes and nothing else: construction and
//! read-back are lossless, and comparison is exact.

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;

type TestError = Box<dyn Error>;

fn stamps() -> Result<Vec<String>, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/remote-updated-at.txt");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    Ok(text.lines().map(str::to_owned).collect())
}

#[test]
fn the_fixture_covers_both_incompatible_provider_formats(
) -> Result<(), TestError> {
    let stamps = stamps()?;
    assert!(
        stamps.iter().any(|stamp| stamp.ends_with('Z')),
        "no Linear-shaped stamp in the fixture: {stamps:?}"
    );
    assert!(
        stamps.iter().any(|stamp| stamp.contains("+0000")),
        "no Jira-shaped stamp in the fixture: {stamps:?}"
    );
    assert!(
        stamps.iter().all(|stamp| !stamp.trim().is_empty()),
        "a blank fixture row round-trips trivially: {stamps:?}"
    );
    Ok(())
}

#[test]
fn every_committed_stamp_survives_a_round_trip_byte_identically(
) -> Result<(), TestError> {
    for stamp in stamps()? {
        let issue = RemoteIssue {
            updated: RemoteTimestamp::new(stamp.clone()),
            body: String::new(),
        };
        assert_eq!(
            issue.updated.as_str(),
            stamp,
            "stamp {stamp} did not survive the round trip"
        );
    }
    Ok(())
}

#[test]
fn stamps_differing_only_in_whitespace_compare_unequal() {
    let stamp = RemoteTimestamp::new("2026-06-21T00:06:10.647Z".to_owned());
    let padded = RemoteTimestamp::new("2026-06-21T00:06:10.647Z ".to_owned());
    assert_ne!(stamp, padded);
}

#[test]
fn the_empty_stamp_is_a_legal_value() {
    assert_eq!(RemoteTimestamp::new(String::new()).as_str(), "");
}

#[test]
fn two_unknown_stamps_compare_equal_and_must_not_be_read_as_unchanged() {
    // The trap this pins is the derived `PartialEq`, not a bug: both empty
    // stamps mean "unknown", so a caller comparing them learns nothing.
    let unknown = RemoteTimestamp::new(String::new());
    assert_eq!(unknown, RemoteTimestamp::new(String::new()));
    assert!(unknown.as_str().is_empty(), "check this before comparing");
}

#[test]
fn an_external_id_returns_the_bytes_it_was_given() {
    let id = ExternalId::new("ENG-1".to_owned());
    assert_eq!(id.as_str(), "ENG-1");
}

#[test]
fn an_external_id_displays_without_reaching_for_its_bytes() {
    let id = ExternalId::new("ENG-1".to_owned());
    assert_eq!(format!("{id}"), "ENG-1");
}

#[test]
fn external_ids_key_a_map() {
    let mut index = HashMap::new();
    index.insert(ExternalId::new("ENG-1".to_owned()), "local-0204");
    assert_eq!(
        index.get(&ExternalId::new("ENG-1".to_owned())),
        Some(&"local-0204")
    );
}
