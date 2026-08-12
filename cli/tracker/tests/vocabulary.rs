//! The value types hold their bytes and nothing else: construction and
//! read-back are lossless, and comparison is exact. A stamp additionally names
//! the two ways it can be unknown, and only a reported pair proves a match.

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;

type TestError = Box<dyn Error>;

const A_STAMP: &str = "2026-07-09T08:00:00.000+0000";

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
            updated: RemoteTimestamp::Reported(stamp.clone()),
            body: String::new(),
        };
        assert_eq!(
            issue.updated.reported(),
            Some(stamp.as_str()),
            "stamp {stamp} did not survive the round trip"
        );
    }
    Ok(())
}

#[test]
fn neither_unknown_reports_a_stamp_to_read() {
    assert_eq!(RemoteTimestamp::NotReported.reported(), None);
    assert_eq!(RemoteTimestamp::NotRead.reported(), None);
}

#[test]
fn the_two_unknowns_are_distinguishable_from_each_other() {
    // A sync report says "the tracker has no stamp for this issue" separately
    // from "we pushed and could not read the stamp back".
    assert_ne!(RemoteTimestamp::NotReported, RemoteTimestamp::NotRead);
}

#[test]
fn stamps_differing_only_in_whitespace_do_not_prove_a_match() {
    let stamp =
        RemoteTimestamp::Reported("2026-06-21T00:06:10.647Z".to_owned());
    let padded =
        RemoteTimestamp::Reported("2026-06-21T00:06:10.647Z ".to_owned());
    assert_ne!(stamp, padded);
    assert!(!stamp.proves_unchanged_since(&padded));
}

#[test]
fn identical_reported_stamps_prove_the_issue_is_unchanged() {
    let stamp = || RemoteTimestamp::Reported(A_STAMP.to_owned());
    assert!(stamp().proves_unchanged_since(&stamp()));
}

#[test]
fn no_unknown_on_either_side_proves_a_match() {
    // An item whose baseline was never written must not classify as already
    // synced, however the unknown arose.
    let unknowns = [RemoteTimestamp::NotReported, RemoteTimestamp::NotRead];
    let reported = RemoteTimestamp::Reported(A_STAMP.to_owned());

    for unknown in &unknowns {
        assert!(!unknown.proves_unchanged_since(&reported));
        assert!(!reported.proves_unchanged_since(unknown));
        for baseline in &unknowns {
            assert!(
                !unknown.proves_unchanged_since(baseline),
                "{unknown:?} must not prove a match against {baseline:?}"
            );
        }
    }
}

#[test]
fn structural_equality_of_two_unknowns_is_not_a_sync_verdict() {
    // Why `proves_unchanged_since` exists rather than callers reaching for
    // `==`: the derive reports two identical unknowns as equal, which says only
    // that both sides know nothing.
    assert_eq!(RemoteTimestamp::NotRead, RemoteTimestamp::NotRead);
    assert!(!RemoteTimestamp::NotRead
        .proves_unchanged_since(&RemoteTimestamp::NotRead));
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
