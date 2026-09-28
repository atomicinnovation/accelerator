//! Sending one create and classifying what became of it.
#![allow(clippy::expect_used)]

use tracker::TrackerError;
use tracker_test_support::Call;
use tracker_test_support::RecordingTracker;
use work_adapters::remote_create::send_create;
use work_adapters::remote_create::CreateRequest;
use work_adapters::remote_create::RemoteCreate;

const REQUEST: CreateRequest<'static> = CreateRequest {
    title: "T",
    body: "# draft-k7mq3x: T\n",
    kind: "task",
};

fn creates(tracker: &RecordingTracker) -> usize {
    tracker
        .calls()
        .iter()
        .filter(|call| matches!(call, Call::Create { .. }))
        .count()
}

fn retryable() -> TrackerError {
    TrackerError::Retryable {
        detail: "connection refused".to_owned(),
    }
}

#[test]
fn a_retryable_create_is_retried_once_then_succeeds() {
    let tracker =
        RecordingTracker::holding(Vec::new()).failing_create_once(retryable());

    let created = send_create(&REQUEST, &tracker);

    assert!(matches!(created, RemoteCreate::Created(_)), "{created:?}");
    assert_eq!(creates(&tracker), 2);
}

#[test]
fn two_retryable_creates_are_an_unreachable_tracker() {
    let tracker =
        RecordingTracker::holding(Vec::new()).failing_create(retryable());

    assert_eq!(
        send_create(&REQUEST, &tracker),
        RemoteCreate::TrackerUnreachable
    );
    assert_eq!(creates(&tracker), 2, "the retry is bounded to one");
}

#[test]
fn an_unconfigured_tracker_is_unreachable_without_a_retry() {
    let tracker = RecordingTracker::holding(Vec::new()).failing_create(
        TrackerError::Unconfigured {
            detail: "no team".to_owned(),
        },
    );

    assert_eq!(
        send_create(&REQUEST, &tracker),
        RemoteCreate::TrackerUnreachable
    );
    assert_eq!(creates(&tracker), 1);
}

#[test]
fn a_terminal_create_is_never_retried_and_its_outcome_is_unknown() {
    let tracker = RecordingTracker::holding(Vec::new()).failing_create(
        TrackerError::Terminal {
            detail: "response lost".to_owned(),
        },
    );

    assert_eq!(
        send_create(&REQUEST, &tracker),
        RemoteCreate::OutcomeUnknown {
            detail: "response lost".to_owned()
        }
    );
    assert_eq!(creates(&tracker), 1);
}

#[test]
fn a_rejected_create_is_never_retried() {
    let tracker = RecordingTracker::holding(Vec::new()).failing_create(
        TrackerError::Rejected {
            detail: "a table".to_owned(),
        },
    );

    assert_eq!(
        send_create(&REQUEST, &tracker),
        RemoteCreate::Rejected {
            detail: "a table".to_owned()
        }
    );
    assert_eq!(creates(&tracker), 1);
}
