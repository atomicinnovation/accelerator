//! The parked-create hook tests use to hold one create mid-flight.
#![allow(clippy::expect_used)]

use std::sync::mpsc;
use std::time::Duration;

use tracker::RemoteTracker;
use tracker::TrackerError;
use tracker_test_support::RecordingTracker;

#[test]
fn a_parked_create_blocks_until_released() {
    let mut tracker = RecordingTracker::holding(Vec::new());
    let park = tracker.parking_create();
    let (done, finished) = mpsc::channel();

    let creator = std::thread::spawn(move || {
        let created = tracker.create("T", "B", "task");
        done.send(()).expect("send");
        created
    });
    park.wait_parked();
    assert!(
        finished.recv_timeout(Duration::from_millis(100)).is_err(),
        "the create is still parked"
    );
    park.release();

    assert!(creator.join().expect("joined").is_ok());
}

#[test]
fn a_create_failing_once_then_succeeds() {
    let tracker = RecordingTracker::holding(Vec::new()).failing_create_once(
        TrackerError::Retryable {
            detail: "refused".to_owned(),
        },
    );

    assert!(tracker.create("T", "B", "task").is_err());
    assert!(tracker.create("T", "B", "task").is_ok());
}
