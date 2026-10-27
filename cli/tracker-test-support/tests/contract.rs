//! Runs the contract harness against `RecordingTracker`.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::TrackerError;
use tracker_test_support::contract;
use tracker_test_support::RecordingTracker;

#[test]
fn the_conformance_properties_hold_against_the_recording_tracker(
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = RecordingTracker::holding(Vec::new());
    let ids = vec![ExternalId::new("REC-does-not-matter".to_owned())];

    let executed = contract::run_all(&subject, &ids)?;

    assert!(executed > 0, "the gate must report having run something");
    Ok(())
}

#[test]
fn an_unaccountable_id_is_indeterminate_not_absent(
) -> Result<(), Box<dyn std::error::Error>> {
    let unseen = ExternalId::new("REC-unseen".to_owned());
    let subject = RecordingTracker::truncating(Vec::new(), vec![unseen]);

    contract::unaccounted_id_is_indeterminate_not_absent(&subject)?;
    Ok(())
}

#[test]
fn a_failing_show_is_retryable() -> Result<(), Box<dyn std::error::Error>> {
    let id = ExternalId::new("REC-1".to_owned());
    let subject = RecordingTracker::holding(Vec::new()).failing_show(
        id,
        TrackerError::Retryable {
            detail: "connection refused".to_owned(),
        },
    );

    contract::a_failing_read_is_retryable(&subject)?;
    Ok(())
}

#[test]
fn an_update_without_a_title_keeps_the_title_against_the_recording_tracker() {
    contract::update_without_a_title_keeps_the_title_property(
        &RecordingTracker::holding(Vec::new()),
    );
}

#[test]
fn a_retitle_keeps_the_description_against_the_recording_tracker() {
    contract::retitle_keeps_the_description_property(
        &RecordingTracker::holding(Vec::new()),
    );
}

#[test]
fn a_recording_tracker_finds_a_moved_issue_only_under_its_current_key(
) -> Result<(), Box<dyn std::error::Error>> {
    let old = ExternalId::new("PP-76".to_owned());
    let subject = RecordingTracker::holding(vec![(
        old.clone(),
        RemoteIssue {
            key: old.clone(),
            updated: RemoteTimestamp::NotReported,
            body: "Moved\n".to_owned(),
        },
    )])
    .moving(&old, &ExternalId::new("OPS-5".to_owned()));

    contract::a_moved_issue_is_found_only_under_its_current_key(&subject)?;
    Ok(())
}
