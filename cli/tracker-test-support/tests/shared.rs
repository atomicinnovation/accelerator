//! The shared handle that lets a test keep a `RecordingTracker`'s call log
//! after boxing the tracker away.
#![allow(clippy::expect_used)]

use std::rc::Rc;

use tracker::RemoteTracker;
use tracker_test_support::Call;
use tracker_test_support::RecordingTracker;
use tracker_test_support::SharedTracker;

#[test]
fn calls_through_a_boxed_share_reach_the_retained_tracker() {
    let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
    let boxed: Box<dyn RemoteTracker> =
        Box::new(SharedTracker(Rc::clone(&tracker)));

    boxed.create("T", "B", "task").expect("created");

    assert_eq!(
        tracker.calls(),
        vec![Call::Create {
            title: "T".to_owned(),
            body: "B".to_owned(),
            kind: "task".to_owned(),
        }]
    );
}
