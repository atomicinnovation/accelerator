#![allow(clippy::panic)]

use std::time::Duration;
use std::time::SystemTime;

use research::sources::pacing::Confirmation;
use research::sources::pacing::Pacing;
use research::sources::pacing::PacingState;
use research::sources::pacing::Reservation;
use research::sources::service::CrossrefPool;
use research::sources::service::Service;

const ARXIV: Pacing = Service::Arxiv.pacing();
const PUBLIC: Pacing = Service::Crossref(CrossrefPool::Public).pacing();
const POLITE: Pacing = Service::Crossref(CrossrefPool::Polite).pacing();

const fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

fn epoch() -> SystemTime {
    SystemTime::UNIX_EPOCH + secs(1_800_000_000)
}

/// How long a caller arriving at `now` waits before its reserved send.
fn wait(pacing: &Pacing, state: PacingState, now: SystemTime) -> Duration {
    let (reservation, _) = state.trust(pacing, now).reserve(pacing, now);
    reservation
        .send_at()
        .duration_since(now)
        .unwrap_or(Duration::ZERO)
}

fn reserved(
    pacing: &Pacing,
    state: PacingState,
    now: SystemTime,
) -> (Reservation, PacingState) {
    state.trust(pacing, now).reserve(pacing, now)
}

fn sent(confirmation: Confirmation) -> PacingState {
    match confirmation {
        Confirmation::Send(state) => state,
        other => panic!("expected a send, got {other:?}"),
    }
}

fn finished_at(at: SystemTime) -> PacingState {
    PacingState {
        last_finish: Some(at),
        ..PacingState::default()
    }
}

// Pure equivalents of the file gate's wait-rule cases, for arXiv.

#[test]
fn a_first_request_waits_for_nothing() {
    assert_eq!(
        wait(&ARXIV, PacingState::default(), epoch()),
        Duration::ZERO
    );
}

#[test]
fn a_request_is_spaced_three_seconds_after_the_last_finish() {
    let now = epoch();

    assert_eq!(wait(&ARXIV, finished_at(now - secs(1)), now), secs(2));
}

#[test]
fn a_last_finish_in_the_future_waits_at_most_three_seconds() {
    let now = epoch();

    assert_eq!(wait(&ARXIV, finished_at(now + secs(100)), now), secs(3));
}

#[test]
fn a_deferral_is_honoured_by_the_next_request() {
    let now = epoch();
    let deferred = PacingState::default().defer(now + secs(12), now);

    assert_eq!(wait(&ARXIV, deferred, now), secs(12));
}

#[test]
fn a_shorter_deferral_never_shortens_a_longer_one_already_stored() {
    let now = epoch();
    let deferred = PacingState::default()
        .defer(now + secs(12), now)
        .defer(now + secs(3), now);

    assert_eq!(wait(&ARXIV, deferred, now), secs(12));
}

#[test]
fn a_deferral_is_kept_no_more_than_thirty_seconds_ahead() {
    let now = epoch();
    let deferred = PacingState::default().defer(now + secs(45), now);

    assert_eq!(wait(&ARXIV, deferred, now), secs(30));
}

#[test]
fn a_stored_deferral_more_than_thirty_seconds_ahead_is_discarded() {
    let now = epoch();
    let stored = PacingState {
        not_before: Some(now + secs(31)),
        ..PacingState::default()
    };

    assert_eq!(wait(&ARXIV, stored, now), Duration::ZERO);
}

#[test]
fn spacing_runs_from_an_unfinished_send_with_a_margin_for_its_arrival() {
    let now = epoch();
    let stored = PacingState {
        last_sent: Some(now - secs(1)),
        last_finish: Some(now - secs(10)),
        ..PacingState::default()
    };

    assert_eq!(wait(&ARXIV, stored, now), ms(2100));
}

#[test]
fn a_send_stamped_ahead_of_now_waits_no_more_than_one_spaced_send() {
    let now = epoch();
    let stored = PacingState {
        last_sent: Some(now + secs(60)),
        ..PacingState::default()
    };

    assert_eq!(wait(&ARXIV, stored, now), ms(3100));
}

#[test]
fn spacing_runs_from_the_last_finish_when_it_follows_the_last_send() {
    let now = epoch();
    let stored = PacingState {
        last_sent: Some(now - secs(10)),
        last_finish: Some(now - secs(1)),
        ..PacingState::default()
    };

    assert_eq!(wait(&ARXIV, stored, now), secs(2));
}

#[test]
fn a_finish_spaced_reservation_leaves_the_state_unchanged() {
    let now = epoch();
    let stored = finished_at(now - secs(1));

    let (_, state) = reserved(&ARXIV, stored, now);

    assert_eq!(state, stored);
}

#[test]
fn a_finish_spaced_caller_that_wakes_on_time_sends() {
    let now = epoch();
    let (reservation, state) = reserved(&ARXIV, finished_at(now), now);
    let woken = reservation.send_at();

    let state = sent(state.confirm(&ARXIV, &reservation, woken));

    assert_eq!(state.last_sent, Some(woken));
}

// Start-spaced pacing.

#[test]
fn start_spacing_runs_from_the_previous_send_whenever_it_finished() {
    let now = epoch();
    let stored = PacingState {
        last_sent: Some(now - ms(50)),
        last_finish: Some(now - ms(10)),
        ..PacingState::default()
    };

    assert_eq!(wait(&PUBLIC, stored, now), ms(160));
}

#[test]
fn a_deferral_outranks_start_spacing() {
    let now = epoch();
    let stored = PacingState {
        last_sent: Some(now - ms(50)),
        not_before: Some(now + secs(1)),
        ..PacingState::default()
    };

    assert_eq!(wait(&PUBLIC, stored, now), secs(1));
}

#[test]
fn a_lone_caller_reserves_wakes_at_its_send_time_and_sends() {
    let now = epoch();
    let stored = PacingState {
        last_sent: Some(now - ms(100)),
        ..PacingState::default()
    };

    let (reservation, state) = reserved(&PUBLIC, stored, now);
    assert_eq!(reservation.send_at(), now + ms(110));
    assert_eq!(state.next_free, Some(now + ms(320)));

    let state = sent(state.confirm(&PUBLIC, &reservation, now + ms(110)));
    assert_eq!(state.last_sent, Some(now + ms(110)));
}

#[test]
fn three_back_to_back_reservations_all_send_within_three_intervals() {
    let now = epoch();
    let (first, state) = reserved(&POLITE, PacingState::default(), now);
    let (second, state) = reserved(&POLITE, state, now);
    let (third, state) = reserved(&POLITE, state, now);
    assert_eq!(
        [first.send_at(), second.send_at(), third.send_at()],
        [now, now + ms(102), now + ms(204)]
    );

    let state = sent(state.confirm(&POLITE, &first, first.send_at()));
    let state = sent(state.confirm(&POLITE, &second, second.send_at()));
    let state = sent(state.confirm(&POLITE, &third, third.send_at()));

    assert!(state.last_sent <= Some(now + ms(306)));
}

#[test]
fn a_late_earlier_sender_holds_the_next_send_back_by_as_much() {
    let now = epoch();
    let (first, state) = reserved(&POLITE, PacingState::default(), now);
    let (second, state) = reserved(&POLITE, state, now);
    let state = sent(state.confirm(&POLITE, &first, now + ms(20)));

    assert_eq!(
        state.confirm(&POLITE, &second, second.send_at()),
        Confirmation::WaitUntil(now + ms(122))
    );
}

#[test]
fn a_deferral_written_after_the_reservation_releases_it() {
    let now = epoch();
    let (reservation, state) = reserved(&PUBLIC, PacingState::default(), now);
    let deferred = state.defer(now + secs(1), now);

    let Confirmation::Deferred(released) =
        deferred.confirm(&PUBLIC, &reservation, now)
    else {
        panic!("expected the reservation to be deferred");
    };
    assert_eq!(released.next_free, Some(now));
    assert_eq!(released.not_before, Some(now + secs(1)));
}

#[test]
fn a_deferral_that_began_after_the_reservation_but_has_ended_sends() {
    let now = epoch();
    let (reservation, state) = reserved(&PUBLIC, PacingState::default(), now);
    let deferred = state.defer(now + ms(50), now);

    let state = sent(deferred.confirm(&PUBLIC, &reservation, now + ms(60)));

    assert_eq!(state.last_sent, Some(now + ms(60)));
}

#[test]
fn a_deferral_the_reservation_already_honoured_holds_an_early_waker() {
    let now = epoch();
    let stored = PacingState {
        not_before: Some(now + ms(300)),
        ..PacingState::default()
    };
    let (reservation, state) = reserved(&PUBLIC, stored, now);

    assert_eq!(
        state.confirm(&PUBLIC, &reservation, now + ms(100)),
        Confirmation::WaitUntil(now + ms(300))
    );
}

#[test]
fn spacing_and_a_deferral_both_forbidding_now_wait_for_the_later() {
    let now = epoch();
    for (deferral, later) in [(ms(150), ms(200)), (ms(250), ms(250))] {
        let stored = PacingState {
            last_sent: Some(now - ms(10)),
            not_before: Some(now + deferral),
            ..PacingState::default()
        };
        let (reservation, state) = reserved(&PUBLIC, stored, now);

        assert_eq!(
            state.confirm(&PUBLIC, &reservation, now),
            Confirmation::WaitUntil(now + later),
            "deferred by {deferral:?}"
        );
    }
}

#[test]
fn releasing_rolls_the_cursor_back_only_for_the_latest_reservation() {
    let now = epoch();
    let (first, state) = reserved(&PUBLIC, PacingState::default(), now);
    let (second, state) = reserved(&PUBLIC, state, now);

    let after_first = state.release(&PUBLIC, &first);
    let after_second = state.release(&PUBLIC, &second);

    assert_eq!(after_first, state);
    assert_eq!(after_second.next_free, Some(now + ms(210)));
}

#[test]
fn releasing_a_reservation_made_at_a_sub_millisecond_instant_rolls_back() {
    let now = epoch() + Duration::from_micros(400);
    let (reservation, state) = reserved(&PUBLIC, PacingState::default(), now);
    assert_eq!(reservation.send_at(), epoch());

    let released = state.release(&PUBLIC, &reservation);

    assert_eq!(released.next_free, Some(epoch()));
}

#[test]
fn releasing_a_finish_spaced_reservation_changes_nothing() {
    let now = epoch();
    let (reservation, state) = reserved(&ARXIV, finished_at(now), now);

    assert_eq!(state.release(&ARXIV, &reservation), state);
}

// Repairs only a clock step or a corrupt file can call for.

#[test]
fn trust_leaves_a_state_no_clock_step_touched_unchanged() {
    let now = epoch();
    let stored = PacingState {
        last_sent: Some(now - secs(4)),
        next_free: Some(now + ms(100)),
        last_finish: Some(now - secs(3)),
        not_before: Some(now + secs(2)),
    };

    assert_eq!(stored.trust(&PUBLIC, now), stored);
    assert_eq!(stored.trust(&ARXIV, now), stored);
}

#[test]
fn a_future_finish_and_an_earlier_future_send_wait_one_spaced_send() {
    let now = epoch();
    let stored = PacingState {
        last_sent: Some(now + secs(60)),
        last_finish: Some(now + secs(30)),
        ..PacingState::default()
    };

    assert_eq!(wait(&ARXIV, stored, now), ms(3100));
}

#[test]
fn a_cursor_up_to_every_slot_beyond_the_next_send_is_kept() {
    let now = epoch();
    let base = now + ms(50);
    let at_bound = PacingState {
        last_sent: Some(base - ms(102)),
        next_free: Some(base + ms(306)),
        ..PacingState::default()
    };
    let beyond = PacingState {
        next_free: Some(base + ms(307)),
        ..at_bound
    };

    assert_eq!(at_bound.trust(&POLITE, now), at_bound);
    assert_eq!(beyond.trust(&POLITE, now).next_free, Some(base + ms(306)));
}

#[test]
fn reservations_chained_after_a_two_second_deferral_are_kept() {
    let now = epoch();
    let deferred = PacingState::default().defer(now + secs(2), now);
    let (_, state) = reserved(&POLITE, deferred, now);
    let (_, state) = reserved(&POLITE, state, now);
    let (_, state) = reserved(&POLITE, state, now);

    assert_eq!(state.next_free, Some(now + secs(2) + ms(306)));
    assert_eq!(state.trust(&POLITE, now), state);
}
