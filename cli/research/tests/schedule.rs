use std::time::Duration;
use std::time::Instant;

use research::schedule::Deadline;
use research::schedule::RetryNumber;
use research::schedule::RetrySchedule;

const fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

fn nth_retry(n: usize) -> RetryNumber {
    (1..n).fold(RetryNumber::first(), |retry, _| retry.next())
}

#[test]
fn retries_back_off_three_six_then_twelve_seconds() {
    assert_eq!(
        RetrySchedule::wait_before(nth_retry(1), None),
        Some(secs(3))
    );
    assert_eq!(
        RetrySchedule::wait_before(nth_retry(2), None),
        Some(secs(6))
    );
    assert_eq!(
        RetrySchedule::wait_before(nth_retry(3), None),
        Some(secs(12))
    );
}

#[test]
fn there_is_no_fourth_retry() {
    assert_eq!(RetrySchedule::wait_before(nth_retry(4), None), None);
    assert_eq!(
        RetrySchedule::wait_before(nth_retry(4), Some(secs(1))),
        None
    );
}

#[test]
fn a_retry_after_hint_replaces_the_backoff() {
    assert_eq!(
        RetrySchedule::wait_before(nth_retry(1), Some(secs(5))),
        Some(secs(5))
    );
    assert_eq!(
        RetrySchedule::wait_before(nth_retry(3), Some(Duration::ZERO)),
        Some(Duration::ZERO)
    );
}

#[test]
fn a_retry_after_hint_is_clamped_to_thirty_seconds() {
    assert_eq!(
        RetrySchedule::wait_before(nth_retry(1), Some(secs(120))),
        Some(secs(30))
    );
}

#[test]
fn a_deferral_follows_the_schedule() {
    assert_eq!(RetrySchedule::deferral_after(nth_retry(1), None), secs(3));
    assert_eq!(RetrySchedule::deferral_after(nth_retry(3), None), secs(12));
    assert_eq!(
        RetrySchedule::deferral_after(nth_retry(2), Some(secs(120))),
        secs(30)
    );
}

#[test]
fn a_deferral_after_the_final_attempt_holds_the_last_backoff() {
    assert_eq!(RetrySchedule::deferral_after(nth_retry(4), None), secs(12));
    assert_eq!(
        RetrySchedule::deferral_after(nth_retry(4), Some(secs(120))),
        secs(30)
    );
}

#[test]
fn a_deadline_reports_what_remains_of_its_budget() {
    let start = Instant::now();
    let deadline = Deadline::starting(start, secs(100), secs(30));
    assert_eq!(deadline.remaining(start), secs(100));
    assert_eq!(deadline.remaining(start + secs(65)), secs(35));
    assert_eq!(deadline.remaining(start + secs(100)), Duration::ZERO);
    assert_eq!(deadline.remaining(start + secs(130)), Duration::ZERO);
}

#[test]
fn a_deadline_admits_an_attempt_whose_request_ends_exactly_on_it() {
    let start = Instant::now();
    let deadline = Deadline::starting(start, secs(100), secs(30));
    assert!(deadline.admits_attempt_after(start + secs(70), Duration::ZERO));
    assert!(deadline.admits_attempt_after(start + secs(64), secs(6)));
}

#[test]
fn a_deadline_refuses_an_attempt_that_could_overrun_it() {
    let start = Instant::now();
    let deadline = Deadline::starting(start, secs(100), secs(30));
    assert!(!deadline.admits_attempt_after(
        start + secs(70) + Duration::from_millis(1),
        Duration::ZERO
    ));
    assert!(!deadline.admits_attempt_after(start + secs(68), secs(6)));
}

#[test]
fn a_deadline_carries_its_request_budget() {
    let deadline = Deadline::starting(Instant::now(), secs(100), secs(30));
    assert_eq!(deadline.per_request(), secs(30));
}
