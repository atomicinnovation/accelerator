//! The transport's bounds, asserted here rather than inherited by inspection:
//! this phase is independently mergeable, and Linear's transport is a copy of
//! Jira's, which makes an omission likelier rather than less likely.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod support;

use std::time::Duration;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::transport::Transport;
use linear_client::{ClientError, Credentials};
use reqwest::Url;
use serde_json::json;
use serde_json::Value;
use support::{NoJitter, RecordingSleeper};
use tracker_support::{Secret, TokenSource, TransportConfig};

const GRAPHQL: &str = "/graphql";
const QUERY: &str = "query { viewer { id } }";

fn credentials() -> Credentials {
    Credentials {
        token: Secret::new("lin_api_secret".to_owned()),
        team_id: "team-1".to_owned(),
        source: TokenSource::Env,
    }
}

fn brief() -> TransportConfig {
    TransportConfig {
        timeout: Duration::from_millis(400),
        ..TransportConfig::default()
    }
}

fn transport_at(
    base: &str,
    config: TransportConfig,
    sleeper: &RecordingSleeper,
) -> Transport {
    Transport::new(
        Url::parse(&format!("{base}{GRAPHQL}")).expect("an endpoint"),
        credentials(),
        config,
        Box::new(sleeper.clone()),
        Box::new(NoJitter),
    )
    .expect("the transport builds")
}

fn errors_body(code: &str) -> String {
    serde_json::to_string(
        &json!({"errors": [{"message": "no", "extensions": {"code": code}}]}),
    )
    .expect("the body serialises")
}

#[test]
fn the_production_endpoint_is_linears_single_graphql_url() {
    let sleeper = RecordingSleeper::new();
    let transport = Transport::to_linear(
        credentials(),
        TransportConfig::default(),
        Box::new(sleeper),
        Box::new(NoJitter),
    )
    .expect("the transport builds");

    assert_eq!(transport.config().timeout, Duration::from_secs(30));
    assert_eq!(transport.config().max_pages, 20);
    assert_eq!(transport.config().max_response_bytes, 8 * 1024 * 1024);
}

#[test]
fn constructing_a_transport_installs_the_crypto_provider() {
    let sleeper = RecordingSleeper::new();
    let _transport = Transport::to_linear(
        credentials(),
        TransportConfig::default(),
        Box::new(sleeper),
        Box::new(NoJitter),
    )
    .expect("the transport builds");

    assert!(
        rustls::crypto::CryptoProvider::get_default().is_some(),
        "the pinned reqwest feature installs no provider, and this call is a \
         copy of Jira's — the likelier of the two to be omitted"
    );
}

#[test]
fn a_request_carries_its_token_document_and_variables() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: "{\"data\":{}}".to_owned(),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_at(&server.base_url(), brief(), &sleeper);

    let received = transport
        .send(QUERY, &json!({"first": 50}))
        .expect("the request completes");

    assert_eq!(received.status, 200);
    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert_eq!(sent["query"], QUERY);
    assert_eq!(sent["variables"]["first"], 50);
    assert_eq!(
        server.last_header(&key, "authorization"),
        Some("lin_api_secret".to_owned()),
        "Linear takes the personal API key as a bare Authorization value"
    );
}

#[test]
fn a_persistent_5xx_is_attempted_exactly_four_times() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(503));
    let sleeper = RecordingSleeper::new();
    let transport = transport_at(&server.base_url(), brief(), &sleeper);

    let received = transport
        .send(QUERY, &json!({}))
        .expect("an exhausted retry still yields the response");

    assert_eq!(received.status, 503);
    assert_eq!(server.hits(&key), 4);
    assert_eq!(
        sleeper.slept(),
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
        ]
    );
}

#[test]
fn a_rate_limited_400_retries_and_carries_retry_after_as_a_duration() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        Route::Headers {
            status: 400,
            headers: vec![("Retry-After".to_owned(), "7".to_owned())],
            body: errors_body("RATELIMITED"),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_at(&server.base_url(), brief(), &sleeper);

    let received = transport
        .send(QUERY, &json!({}))
        .expect("an exhausted rate limit still yields the response");

    assert_eq!(received.status, 400, "rate limiting arrives as HTTP 400");
    assert_eq!(server.hits(&key), 4);
    assert_eq!(
        sleeper.slept(),
        vec![
            Duration::from_secs(7),
            Duration::from_secs(7),
            Duration::from_secs(7),
        ],
        "the hint wins over the exponential term"
    );
}

#[test]
fn a_two_hundred_carrying_errors_is_never_retried() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: errors_body("RATELIMITED"),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_at(&server.base_url(), brief(), &sleeper);

    let received = transport.send(QUERY, &json!({})).expect("it returns");

    assert_eq!(received.status, 200);
    assert_eq!(
        server.hits(&key),
        1,
        "re-issuing a non-idempotent mutation because its 200 reported an \
         error is how a duplicate gets created"
    );
    assert!(sleeper.slept().is_empty());
}

#[test]
fn a_four_hundred_that_is_not_rate_limited_is_not_retried() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        Route::Json {
            status: 400,
            body: errors_body("BAD_USER_INPUT"),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_at(&server.base_url(), brief(), &sleeper);

    transport.send(QUERY, &json!({})).expect("it returns");

    assert_eq!(server.hits(&key), 1);
}

#[test]
fn a_transport_failure_makes_exactly_one_attempt() {
    let sleeper = RecordingSleeper::new();
    let transport = transport_at("http://127.0.0.1:1", brief(), &sleeper);

    let error = transport
        .send(QUERY, &json!({}))
        .expect_err("a refused connection is an error");

    assert!(matches!(error, ClientError::Transport { .. }), "{error}");
    assert!(sleeper.slept().is_empty());
}

#[test]
fn an_injected_timeout_takes_effect() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Stall(Duration::from_secs(30)),
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_at(&server.base_url(), brief(), &sleeper);

    let started = std::time::Instant::now();
    let error = transport
        .send(QUERY, &json!({}))
        .expect_err("a stalled body read times out");
    let elapsed = started.elapsed();

    assert!(matches!(error, ClientError::Transport { .. }), "{error}");
    assert!(elapsed >= Duration::from_millis(400), "{elapsed:?}");
    assert!(elapsed < Duration::from_millis(1200), "{elapsed:?}");
}

#[test]
fn a_redirect_is_refused_rather_than_followed() {
    let server = MockServer::start();
    let target = RequestKey::post("/elsewhere");
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Redirect {
            status: 302,
            location: format!("{}/elsewhere", server.base_url()),
        },
    );
    server.route(
        target.clone(),
        Route::Json {
            status: 200,
            body: "{\"data\":{}}".to_owned(),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_at(&server.base_url(), brief(), &sleeper);

    let received = transport.send(QUERY, &json!({})).expect("it returns");

    assert_eq!(received.status, 302);
    assert_eq!(server.hits(&target), 0);
}

#[test]
fn a_response_beyond_the_bound_is_rejected_rather_than_buffered() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Json {
            status: 200,
            body: format!("{{\"data\":\"{}\"}}", "x".repeat(4096)),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_at(
        &server.base_url(),
        TransportConfig {
            max_response_bytes: 128,
            ..brief()
        },
        &sleeper,
    );

    let error = transport
        .send(QUERY, &json!({}))
        .expect_err("an oversized body is refused");

    assert!(
        matches!(error, ClientError::OversizedResponse { limit: 128 }),
        "{error}"
    );
}

#[test]
fn a_non_json_body_is_reported_as_such_rather_than_as_a_transport_failure() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Bytes {
            status: 200,
            body: b"<html>maintenance</html>".to_vec(),
        },
    );
    let sleeper = RecordingSleeper::new();
    let transport = transport_at(&server.base_url(), brief(), &sleeper);

    let received = transport.send(QUERY, &json!({})).expect("it returns");

    assert_eq!(received.status, 200);
    assert!(
        received.json().is_none(),
        "a 2xx non-JSON body is bash code 16, not a transport failure"
    );
}
