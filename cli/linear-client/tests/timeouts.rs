//! Timeout, page-cap and deadline behaviour, in an asymmetric shape: a tight
//! lower bound and a generous 3×T upper bound.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::time::Duration;
use std::time::Instant;

use http_test_support::{MockServer, RequestKey, Route};
use support::client::client_for;
use tracker::{ExternalId, RemoteTracker as _, TrackerError};
use tracker_support::TransportConfig;

const GRAPHQL: &str = "/graphql";

fn id(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

fn at(timeout: Duration) -> TransportConfig {
    TransportConfig {
        timeout,
        ..TransportConfig::default()
    }
}

fn stalling() -> MockServer {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Stall(Duration::from_secs(30)),
    );
    server
}

#[test]
fn show_fails_within_the_window_at_both_injected_timeouts() {
    for timeout in [Duration::from_millis(400), Duration::from_secs(1)] {
        let server = stalling();
        let client = client_for(&server, at(timeout));

        let started = Instant::now();
        let error = client
            .show(&id("ENG-1"))
            .expect_err("a stalled read times out");
        let elapsed = started.elapsed();

        assert!(elapsed >= timeout, "{elapsed:?} < {timeout:?}");
        assert!(elapsed < timeout * 3, "{elapsed:?}");
        assert!(
            matches!(error, TrackerError::Retryable { .. }),
            "a read is never terminal: {error}"
        );
        assert!(error.to_string().contains("E_GQL_CONNECT"), "{error}");
    }
}

#[test]
fn fetch_all_returns_ok_with_every_id_indeterminate_on_a_timeout() {
    for timeout in [Duration::from_millis(400), Duration::from_secs(1)] {
        let server = stalling();
        let client = client_for(&server, at(timeout));

        let started = Instant::now();
        let outcome = client
            .fetch_all(&[id("ENG-1"), id("ENG-2")])
            .expect("a post-attempt transport failure is an Ok");
        let elapsed = started.elapsed();

        assert!(elapsed >= timeout, "{elapsed:?}");
        assert!(elapsed < timeout * 3, "{elapsed:?}");
        assert_eq!(outcome.indeterminate.len(), 2);
        assert!(outcome.absent.is_empty());
    }
}

#[test]
fn the_page_cap_stops_the_cursor_walk_and_reports_indeterminate() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    // Every page offers another cursor, so only the cap ends the walk.
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: "{\"data\":{\"issues\":{\"nodes\":[],\
                   \"pageInfo\":{\"hasNextPage\":true,\"endCursor\":\"c\"}}}}"
                .to_owned(),
        },
    );
    let client = client_for(&server, at(Duration::from_millis(400)));

    let outcome = client
        .fetch_all(&[id("ENG-1")])
        .expect("a cap-hit is an Ok");

    assert_eq!(server.hits(&key), 20, "MAX_PAGES is 20");
    assert_eq!(outcome.indeterminate, vec![id("ENG-1")]);
    assert!(outcome.absent.is_empty());
}

#[test]
fn the_operation_deadline_fires_while_each_request_stays_inside_its_timeout() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Json {
            status: 200,
            body: "{\"data\":{\"issues\":{\"nodes\":[],\
                   \"pageInfo\":{\"hasNextPage\":true,\"endCursor\":\"c\"}}}}"
                .to_owned(),
        },
    );
    let client = client_for(
        &server,
        TransportConfig {
            timeout: Duration::from_millis(400),
            deadline: Duration::from_millis(0),
            ..TransportConfig::default()
        },
    );

    let outcome = client
        .fetch_all(&[id("ENG-1")])
        .expect("a deadline expiry degrades as a cap-hit does");

    assert_eq!(outcome.indeterminate, vec![id("ENG-1")]);
    assert!(outcome.absent.is_empty());
}

#[test]
fn the_default_timeout_is_thirty_seconds_and_the_cap_is_twenty_pages() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    assert_eq!(
        client.transport().config().timeout,
        Duration::from_secs(30),
        "transcribed from linear-graphql.sh:519"
    );
    assert_eq!(client.transport().config().max_pages, 20);
}
