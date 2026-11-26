#![allow(clippy::expect_used)]

use std::num::NonZeroU8;
use std::time::Duration;

use research::sources::pacing::Pacing;
use research::sources::pacing::SpacedFrom;
use research::sources::request::Family;
use research::sources::service::CrossrefPool;
use research::sources::service::Service;

const fn millis(value: u64) -> Duration {
    Duration::from_millis(value)
}

const fn slots(count: u8) -> NonZeroU8 {
    NonZeroU8::new(count).expect("a non-zero slot count")
}

#[test]
fn arxiv_is_spaced_three_seconds_from_the_previous_finish() {
    let pacing = Service::Arxiv.pacing();

    assert_eq!(pacing.interval(), Duration::from_secs(3));
    assert_eq!(pacing.spaced_from(), SpacedFrom::PreviousFinish);
    assert_eq!(pacing.slots(), slots(1));
}

#[test]
fn crossref_s_public_pool_is_spaced_from_the_previous_start_one_at_a_time() {
    let pacing = Service::Crossref(CrossrefPool::Public).pacing();

    assert_eq!(pacing.interval(), millis(210));
    assert_eq!(
        pacing.spaced_from(),
        SpacedFrom::PreviousStart { slots: slots(1) }
    );
    assert_eq!(pacing.slots(), slots(1));
}

#[test]
fn crossref_s_polite_pool_is_spaced_from_the_previous_start_three_at_a_time() {
    let pacing = Service::Crossref(CrossrefPool::Polite).pacing();

    assert_eq!(pacing.interval(), millis(102));
    assert_eq!(
        pacing.spaced_from(),
        SpacedFrom::PreviousStart { slots: slots(3) }
    );
    assert_eq!(pacing.slots(), slots(3));
}

#[test]
fn each_service_names_its_state_directory() {
    assert_eq!(Service::Arxiv.code(), "arxiv");
    assert_eq!(
        Service::Crossref(CrossrefPool::Public).code(),
        "crossref-public"
    );
    assert_eq!(
        Service::Crossref(CrossrefPool::Polite).code(),
        "crossref-polite"
    );
}

#[test]
fn both_crossref_pools_report_one_source_and_issue_doi_tickets() {
    assert_eq!(Service::Arxiv.source(), "arxiv");
    assert_eq!(Service::Arxiv.ticket_family(), Family::Arxiv);
    for pool in [CrossrefPool::Public, CrossrefPool::Polite] {
        assert_eq!(Service::Crossref(pool).source(), "crossref");
        assert_eq!(Service::Crossref(pool).ticket_family(), Family::Doi);
    }
}

#[test]
fn every_service_is_listed_once() {
    assert_eq!(
        Service::ALL,
        [
            Service::Arxiv,
            Service::Crossref(CrossrefPool::Public),
            Service::Crossref(CrossrefPool::Polite),
        ]
    );
}

#[test]
fn an_unpaced_source_polls_admission_every_ten_milliseconds() {
    assert_eq!(Pacing::UNPACED.interval(), Duration::ZERO);
    assert_eq!(Pacing::UNPACED.admission_poll(), millis(10));
}

#[test]
fn admission_is_polled_every_quarter_interval_within_ten_to_a_hundred_ms() {
    let poll = |service: Service| service.pacing().admission_poll();

    assert_eq!(poll(Service::Arxiv), millis(100));
    assert_eq!(poll(Service::Crossref(CrossrefPool::Public)), millis(52));
    assert_eq!(poll(Service::Crossref(CrossrefPool::Polite)), millis(25));
}
