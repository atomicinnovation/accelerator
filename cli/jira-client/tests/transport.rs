//! The transport's bounds: the retry schedule, the redirect refusal, the
//! response bound, and the seams that keep the suite off the wall clock.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod support;

use std::time::Duration;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::transport::Transport;
use jira_client::{ClientError, Credentials};
use reqwest::{Method, Url};
use support::{NoJitter, RecordingSleeper};
use tracker_support::{Secret, TokenSource, TransportConfig};

const MYSELF: &str = "/rest/api/3/myself";

fn credentials(base: &str) -> Credentials {
    Credentials {
        base: Url::parse(base).expect("a base URL"),
        email: "toby@example.com".to_owned(),
        token: Secret::new("secret-token".to_owned()),
        source: TokenSource::Env,
    }
}

fn transport_with(
    base: &str,
    config: TransportConfig,
    sleeper: &RecordingSleeper,
) -> Transport {
    Transport::new(
        credentials(base),
        config,
        Box::new(sleeper.clone()),
        Box::new(NoJitter),
    )
    .expect("the transport builds")
}

fn brief() -> TransportConfig {
    TransportConfig {
        timeout: Duration::from_millis(400),
        ..TransportConfig::default()
    }
}

#[test]
fn the_constructed_bounds_are_the_transcribed_ones() {
    let sleeper = RecordingSleeper::new();
    let transport = transport_with(
        "https://tenant.atlassian.net",
        TransportConfig::default(),
        &sleeper,
    );

    assert_eq!(transport.config().timeout, Duration::from_secs(30));
    assert_eq!(transport.config().max_pages, 20);
    assert_eq!(transport.config().max_response_bytes, 8 * 1024 * 1024);
}

#[test]
fn constructing_a_transport_installs_the_crypto_provider() {
    let sleeper = RecordingSleeper::new();
    let _transport = transport_with(
        "https://tenant.atlassian.net",
        TransportConfig::default(),
        &sleeper,
    );

    assert!(
        rustls::crypto::CryptoProvider::get_default().is_some(),
        "the pinned reqwest feature installs no provider, so every https \
         request would fail at handshake without this call"
    );
}

#[test]
fn a_request_carries_its_credentials_body_and_query() {
    let server = MockServer::start();
    let key = RequestKey::post("/rest/api/3/search/jql");
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: "{}".to_owned(),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_with(&server.base_url(), brief(), &sleeper);

    let received = transport
        .send(
            &Method::POST,
            "/rest/api/3/search/jql",
            &[("expand", "names")],
            Some("{\"jql\":\"key in (ABC-1)\"}"),
        )
        .expect("the request completes");

    assert_eq!(received.status, 200);
    assert_eq!(
        server.last_body(&key),
        Some(b"{\"jql\":\"key in (ABC-1)\"}".to_vec())
    );
    assert_eq!(server.last_query(&key), Some("expand=names".to_owned()));
    assert_eq!(
        server.last_header(&key, "content-type"),
        Some("application/json".to_owned())
    );
    assert!(
        server
            .last_header(&key, "authorization")
            .expect("the token reaches the request")
            .starts_with("Basic "),
        "Jira authenticates with basic auth over email and token"
    );
}

#[test]
fn a_persistent_5xx_is_attempted_exactly_four_times() {
    let server = MockServer::start();
    let key = RequestKey::get(MYSELF);
    server.route(key.clone(), Route::Status(503));
    let sleeper = RecordingSleeper::new();
    let transport = transport_with(&server.base_url(), brief(), &sleeper);

    let received = transport
        .send(&Method::GET, MYSELF, &[], None)
        .expect("an exhausted retry still yields the response");

    assert_eq!(received.status, 503);
    assert_eq!(server.hits(&key), 4, "four attempts, as the bash makes");
    assert_eq!(
        sleeper.slept(),
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
        ],
        "the recorded sequence proves the seam is wired and the real clock \
         was never consulted"
    );
}

#[test]
fn retry_after_is_honoured_as_a_duration_not_merely_as_a_trigger() {
    let server = MockServer::start();
    let key = RequestKey::get(MYSELF);
    server.route(
        key.clone(),
        Route::Headers {
            status: 429,
            headers: vec![("Retry-After".to_owned(), "7".to_owned())],
            body: String::new(),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_with(&server.base_url(), brief(), &sleeper);

    let received = transport
        .send(&Method::GET, MYSELF, &[], None)
        .expect("an exhausted 429 still yields the response");

    assert_eq!(received.status, 429);
    assert_eq!(server.hits(&key), 4);
    assert_eq!(
        sleeper.slept(),
        vec![
            Duration::from_secs(7),
            Duration::from_secs(7),
            Duration::from_secs(7),
        ],
        "the hint wins over the 1s, 2s, 4s the default backoff would take"
    );
}

#[test]
fn a_transport_failure_makes_exactly_one_attempt() {
    let sleeper = RecordingSleeper::new();
    // Port 1 on loopback refuses immediately: a connect failure, not a status.
    let transport = transport_with("http://127.0.0.1:1", brief(), &sleeper);

    let error = transport
        .send(&Method::GET, MYSELF, &[], None)
        .expect_err("a refused connection is an error, not a response");

    assert!(matches!(error, ClientError::Transport { .. }), "{error}");
    assert!(
        sleeper.slept().is_empty(),
        "a transport failure resolves on the first attempt, with no retry"
    );
}

#[test]
fn an_injected_timeout_takes_effect_and_is_not_retried() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(MYSELF),
        Route::Stall(Duration::from_secs(30)),
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_with(&server.base_url(), brief(), &sleeper);

    let started = std::time::Instant::now();
    let error = transport
        .send(&Method::GET, MYSELF, &[], None)
        .expect_err("a stalled body read times out");
    let elapsed = started.elapsed();

    assert!(matches!(error, ClientError::Transport { .. }), "{error}");
    assert!(
        elapsed >= Duration::from_millis(400),
        "the call must not return before the injected timeout: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(1200),
        "and must not have waited on the 30s default: {elapsed:?}"
    );
    assert!(sleeper.slept().is_empty());
}

#[test]
fn a_redirect_is_refused_rather_than_followed() {
    let server = MockServer::start();
    let target = RequestKey::get("/rest/api/3/elsewhere");
    server.route(
        RequestKey::get(MYSELF),
        Route::Redirect {
            status: 302,
            location: format!("{}/rest/api/3/elsewhere", server.base_url()),
        },
    );
    server.route(
        target.clone(),
        Route::Json {
            status: 200,
            body: "{}".to_owned(),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_with(&server.base_url(), brief(), &sleeper);

    let received = transport
        .send(&Method::GET, MYSELF, &[], None)
        .expect("the redirect arrives as a response");

    assert_eq!(received.status, 302);
    assert_eq!(
        server.hits(&target),
        0,
        "following a redirect would defeat the site validator"
    );
}

#[test]
fn a_response_beyond_the_bound_is_rejected_rather_than_buffered() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(MYSELF),
        Route::Json {
            status: 200,
            body: "x".repeat(4096),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_with(
        &server.base_url(),
        TransportConfig {
            max_response_bytes: 128,
            ..brief()
        },
        &sleeper,
    );

    let error = transport
        .send(&Method::GET, MYSELF, &[], None)
        .expect_err("an oversized body is refused");

    assert!(
        matches!(error, ClientError::OversizedResponse { limit: 128 }),
        "{error}"
    );
}

#[test]
fn a_bad_path_is_refused_before_anything_is_sent() {
    let server = MockServer::start();
    let key = RequestKey::get("/rest/api/2/myself");
    server.route(key.clone(), Route::Status(200));
    let sleeper = RecordingSleeper::new();
    let transport = transport_with(&server.base_url(), brief(), &sleeper);

    let error = transport
        .send(&Method::GET, "/rest/api/2/myself", &[], None)
        .expect_err("only /rest/api/3/ is sendable");

    assert!(matches!(error, ClientError::BadPath { .. }), "{error}");
    assert_eq!(server.hits(&key), 0);
}
