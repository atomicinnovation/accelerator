//! Timeout, page-cap and deadline behaviour.
//!
//! The assertions are asymmetric on purpose: a tight lower bound (the call must
//! not return before T — the property carrying the signal) and a generous 3×T
//! upper bound. A 1.35×T bound leaves 140ms of slack at T = 400ms, inside
//! scheduler jitter on a loaded runner, and this repository has a documented
//! flake history.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::time::Duration;
use std::time::Instant;

use http_test_support::{MockServer, RequestKey, Route};
use support::client::{client_for, credentials};
use tracker::{ExternalId, RemoteTracker as _, TrackerError};
use tracker_support::TransportConfig;

const SEARCH: &str = "/rest/api/3/search/jql";

fn id(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

fn at(timeout: Duration) -> TransportConfig {
    TransportConfig {
        timeout,
        ..TransportConfig::default()
    }
}

fn stalling_server(paths: &[&str]) -> MockServer {
    let server = MockServer::start();
    for path in paths {
        server.route(
            RequestKey::get(path),
            Route::Stall(Duration::from_secs(30)),
        );
        server.route(
            RequestKey::post(path),
            Route::Stall(Duration::from_secs(30)),
        );
    }
    server
}

#[test]
fn show_fails_within_the_window_at_both_injected_timeouts() {
    for timeout in [Duration::from_millis(400), Duration::from_secs(1)] {
        let server = stalling_server(&["/rest/api/3/issue/ENG-1"]);
        let client = client_for(&server, at(timeout));

        let started = Instant::now();
        let error = client
            .show(&id("ENG-1"))
            .expect_err("a stalled read times out");
        let elapsed = started.elapsed();

        assert!(
            elapsed >= timeout,
            "the call must not return before {timeout:?}: {elapsed:?}"
        );
        assert!(
            elapsed < timeout * 3,
            "and must return within 3× it: {elapsed:?}"
        );
        assert!(
            matches!(error, TrackerError::Retryable { .. }),
            "a read is never terminal: {error}"
        );
        assert!(
            error.to_string().contains("E_REQ_CONNECT"),
            "the failure is the transport class — bash code 21 covers \
             connect, DNS and timeout as one: {error}"
        );
    }
}

#[test]
fn fetch_all_returns_ok_with_every_id_indeterminate_on_a_timeout() {
    for timeout in [Duration::from_millis(400), Duration::from_secs(1)] {
        let server = stalling_server(&[SEARCH]);
        let client = client_for(&server, at(timeout));

        let started = Instant::now();
        let outcome = client
            .fetch_all(&[id("ENG-1"), id("ENG-2")])
            .expect("a post-attempt transport failure is an Ok, not an Err");
        let elapsed = started.elapsed();

        assert!(elapsed >= timeout, "{elapsed:?}");
        assert!(elapsed < timeout * 3, "{elapsed:?}");
        assert_eq!(outcome.indeterminate.len(), 2);
        assert!(outcome.absent.is_empty());
        assert!(outcome.found.is_empty());
    }
}

#[test]
fn a_paginated_fixture_stops_at_the_page_cap_and_reports_indeterminate() {
    let server = MockServer::start();
    // Every page offers another cursor, so the cap is the only thing that
    // stops the loop.
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: "{\"issues\":[{\"key\":\"ENG-1\",\"fields\":{}}],\
                   \"nextPageToken\":\"more\"}"
                .to_owned(),
        },
    );
    let client = client_for(&server, at(Duration::from_millis(400)));

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-2")])
        .expect("a cap-hit is an Ok");

    assert_eq!(
        server.hits(&RequestKey::post(SEARCH)),
        20,
        "the cap is 20 pages per chunk"
    );
    assert_eq!(
        outcome.indeterminate.len(),
        2,
        "the unseen ids are indeterminate, never absent"
    );
    assert!(outcome.absent.is_empty());
}

#[test]
fn the_operation_deadline_fires_while_each_request_stays_inside_its_timeout() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: "{\"issues\":[],\"nextPageToken\":\"more\"}".to_owned(),
        },
    );
    let client = client_for(
        &server,
        TransportConfig {
            timeout: Duration::from_millis(400),
            // Already expired by the time the first page is requested.
            deadline: Duration::from_millis(0),
            ..TransportConfig::default()
        },
    );

    let outcome = client
        .fetch_all(&[id("ENG-1")])
        .expect("a deadline expiry degrades exactly as a cap-hit does");

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
        "transcribed from jira-request.sh:298"
    );
    assert_eq!(client.transport().config().max_pages, 20);
}

#[test]
fn a_loopback_base_is_reachable_through_the_constructor() {
    // The counterpart assertion — that configuration refuses it — lives in
    // tests/auth.rs. The seam is the constructor, not process state.
    let credentials = credentials("http://127.0.0.1:9");
    assert_eq!(credentials.base.host_str(), Some("127.0.0.1"));
}
