//! Runs the contract harness against `RecordingTracker`.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use tracker::ExternalId;
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
fn an_unaccountable_id_is_indeterminate_not_absent() {
    let unseen = ExternalId::new("REC-unseen".to_owned());
    let subject = RecordingTracker::truncating(Vec::new(), vec![unseen]);

    contract::unaccounted_id_is_indeterminate_not_absent(&subject);
}

#[test]
fn a_failing_show_is_retryable() {
    let id = ExternalId::new("REC-1".to_owned());
    let subject = RecordingTracker::holding(Vec::new()).failing_show(
        id,
        TrackerError::Retryable {
            detail: "connection refused".to_owned(),
        },
    );

    contract::a_failing_read_is_retryable(&subject);
}
