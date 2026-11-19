//! The bounded transport: bearer auth, the two resend policies counted on the
//! mock, the may-have-applied flag, and the guards that keep the token on
//! Shortcut's origin.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::time::Duration;

use config::credentials::Secret;
use config::credentials::TokenSource;
use http_test_support::refused_base_url;
use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use serde_json::json;
use shortcut_client::paths::Paths;
use shortcut_client::paths::WorkspaceSlug;
use shortcut_client::transport::Deadline;
use shortcut_client::transport::Resend;
use shortcut_client::transport::Transport;
use shortcut_client::ClientError;
use shortcut_client::Credentials;
use shortcut_client::Method;
use shortcut_client::Url;
use support::NoJitter;
use support::RecordingSleeper;
use tracker_support::TransportConfig;

const TOKEN: &str = "sct_rw_acme_secret";
const PATH: &str = "/api/v4/acme/stories/12";

fn credentials() -> Credentials {
    Credentials {
        token: Secret::new(TOKEN.to_owned()),
        source: TokenSource::Env,
        refusals: Vec::new(),
        notice: None,
    }
}

fn transport_with(config: TransportConfig) -> (Transport, RecordingSleeper) {
    let sleeper = RecordingSleeper::new();
    let transport = Transport::new(
        &credentials(),
        config,
        Box::new(sleeper.clone()),
        Box::new(NoJitter),
    )
    .expect("the transport builds");
    (transport, sleeper)
}

fn transport() -> (Transport, RecordingSleeper) {
    transport_with(TransportConfig::default())
}

fn url(server: &MockHTTPServer) -> Url {
    Url::parse(&format!("{}{PATH}", server.base_url())).expect("parses")
}

fn json(status: u16) -> Route {
    Route::Json {
        status,
        body: "{}".to_owned(),
    }
}

fn never() -> Deadline {
    Deadline::starting_now(Duration::MAX)
}

fn patch_key() -> RequestKey {
    RequestKey::new("PATCH", PATH)
}

#[test]
fn every_request_carries_the_bearer_token_and_a_json_body_type() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::post(PATH), json(201));
    let (transport, _) = transport();

    let delivery = transport.send(
        &Method::POST,
        &url(&server),
        &json!({"name": "x"}),
        Resend::OnlyWhenThrottled,
        &never(),
    );

    assert_eq!(delivery.result.expect("arrives").status, 201);
    let key = RequestKey::post(PATH);
    assert_eq!(
        server.last_header(&key, "authorization").as_deref(),
        Some(format!("Bearer {TOKEN}").as_str())
    );
    assert_eq!(
        server.last_header(&key, "content-type").as_deref(),
        Some("application/json")
    );
}

#[test]
fn a_get_is_resent_after_a_rate_limit() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(PATH),
        Route::Sequence(vec![json(429), json(200)]),
    );
    let (transport, _) = transport();

    let delivery = transport.get(&url(&server), &never());

    assert_eq!(delivery.result.expect("arrives").status, 200);
    assert_eq!(server.hits(&RequestKey::get(PATH)), 2);
}

#[test]
fn a_get_stops_resending_server_errors_after_four_attempts() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get(PATH), json(500));
    let (transport, _) = transport();

    let delivery = transport.get(&url(&server), &never());

    assert_eq!(delivery.result.expect("arrives").status, 500);
    assert_eq!(server.hits(&RequestKey::get(PATH)), 4);
}

#[test]
fn a_get_that_times_out_is_sent_once() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get(PATH), Route::Stall(Duration::from_secs(5)));
    let (transport, _) = transport_with(TransportConfig {
        timeout: Duration::from_millis(200),
        ..TransportConfig::default()
    });

    let delivery = transport.get(&url(&server), &never());

    assert!(
        matches!(delivery.result, Err(ClientError::Transport { .. })),
        "{:?}",
        delivery.result
    );
    assert_eq!(server.hits(&RequestKey::get(PATH)), 1);
}

#[test]
fn a_throttled_only_post_is_not_resent_after_a_server_error() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::post(PATH), json(500));
    let (transport, _) = transport();

    let delivery = transport.send(
        &Method::POST,
        &url(&server),
        &json!({}),
        Resend::OnlyWhenThrottled,
        &never(),
    );

    assert_eq!(delivery.result.expect("arrives").status, 500);
    assert!(delivery.may_have_applied);
    assert_eq!(server.hits(&RequestKey::post(PATH)), 1);
}

#[test]
fn a_throttled_only_post_is_resent_after_a_rate_limit() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::post(PATH),
        Route::Sequence(vec![json(429), json(201)]),
    );
    let (transport, _) = transport();

    let delivery = transport.send(
        &Method::POST,
        &url(&server),
        &json!({}),
        Resend::OnlyWhenThrottled,
        &never(),
    );

    assert_eq!(delivery.result.expect("arrives").status, 201);
    assert!(!delivery.may_have_applied);
    assert_eq!(server.hits(&RequestKey::post(PATH)), 2);
}

#[test]
fn a_post_that_times_out_is_sent_once_and_may_have_applied() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::post(PATH), Route::Stall(Duration::from_secs(5)));
    let (transport, _) = transport_with(TransportConfig {
        timeout: Duration::from_millis(200),
        ..TransportConfig::default()
    });

    let delivery = transport.send(
        &Method::POST,
        &url(&server),
        &json!({}),
        Resend::OnlyWhenThrottled,
        &never(),
    );

    assert!(matches!(
        delivery.result,
        Err(ClientError::Transport { .. })
    ));
    assert!(delivery.may_have_applied);
    assert_eq!(server.hits(&RequestKey::post(PATH)), 1);
}

#[test]
fn a_post_that_cannot_connect_is_not_sent() {
    let base = refused_base_url();
    let (transport, _) = transport();

    let delivery = transport.send(
        &Method::POST,
        &Url::parse(&format!("{base}{PATH}")).expect("parses"),
        &json!({}),
        Resend::OnlyWhenThrottled,
        &never(),
    );

    assert!(
        matches!(delivery.result, Err(ClientError::NotSent { .. })),
        "{:?}",
        delivery.result
    );
    assert!(!delivery.may_have_applied);
}

#[test]
fn a_retryable_patch_is_resent_after_a_server_error() {
    let server = MockHTTPServer::start();
    server.route(patch_key(), Route::Sequence(vec![json(500), json(200)]));
    let (transport, _) = transport();

    let delivery = transport.send(
        &Method::PATCH,
        &url(&server),
        &json!({}),
        Resend::WhileRetryable,
        &never(),
    );

    assert_eq!(delivery.result.expect("arrives").status, 200);
    assert!(delivery.may_have_applied);
    assert_eq!(server.hits(&patch_key()), 2);
}

#[test]
fn a_server_error_before_a_rate_limit_marks_the_write_as_possibly_applied() {
    let server = MockHTTPServer::start();
    server.route(patch_key(), Route::Sequence(vec![json(500), json(429)]));
    let (transport, _) = transport();

    let delivery = transport.send(
        &Method::PATCH,
        &url(&server),
        &json!({}),
        Resend::WhileRetryable,
        &never(),
    );

    assert_eq!(delivery.result.expect("arrives").status, 429);
    assert!(delivery.may_have_applied);
    assert_eq!(server.hits(&patch_key()), 4);
}

#[test]
fn a_rejection_with_an_html_body_arrives_as_its_status() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(PATH),
        Route::Bytes {
            status: 403,
            body: b"<html>forbidden</html>".to_vec(),
        },
    );
    let (transport, _) = transport();

    let delivery = transport.get(&url(&server), &never());

    let received = delivery.result.expect("arrives");
    assert_eq!(received.status, 403);
    assert!(received.json().is_none());
}

#[test]
fn a_retry_after_header_sets_the_backoff() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(PATH),
        Route::Sequence(vec![
            Route::Headers {
                status: 429,
                headers: vec![("Retry-After".to_owned(), "2".to_owned())],
                body: "{}".to_owned(),
            },
            json(200),
        ]),
    );
    let (transport, sleeper) = transport();

    let delivery = transport.get(&url(&server), &never());

    assert_eq!(delivery.result.expect("arrives").status, 200);
    assert_eq!(sleeper.slept(), vec![Duration::from_secs(2)]);
}

#[test]
fn an_expired_deadline_stops_the_retries() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get(PATH), json(500));
    let (transport, sleeper) = transport();

    let delivery =
        transport.get(&url(&server), &Deadline::starting_now(Duration::ZERO));

    assert_eq!(delivery.result.expect("arrives").status, 500);
    assert_eq!(server.hits(&RequestKey::get(PATH)), 1);
    assert!(sleeper.slept().is_empty());
}

#[test]
fn a_redirect_is_not_followed() {
    let server = MockHTTPServer::start();
    let elsewhere = format!("{}/elsewhere", server.base_url());
    server.route(
        RequestKey::get(PATH),
        Route::Redirect {
            status: 302,
            location: elsewhere,
        },
    );
    server.route(RequestKey::get("/elsewhere"), json(200));
    let (transport, _) = transport();

    let delivery = transport.get(&url(&server), &never());

    assert_eq!(delivery.result.expect("arrives").status, 302);
    assert_eq!(server.hits(&RequestKey::get("/elsewhere")), 0);
}

#[test]
fn an_oversized_body_is_refused() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(PATH),
        Route::Json {
            status: 200,
            body: format!("\"{}\"", "x".repeat(64)),
        },
    );
    let (transport, _) = transport_with(TransportConfig {
        max_response_bytes: 16,
        ..TransportConfig::default()
    });

    let delivery = transport.get(&url(&server), &never());

    assert!(matches!(
        delivery.result,
        Err(ClientError::OversizedResponse { limit: 16 })
    ));
}

#[test]
fn a_foreign_next_page_url_is_refused_without_a_request() {
    let server = MockHTTPServer::start();
    let foreign = MockHTTPServer::start();
    foreign.route(RequestKey::get("/api/v4/acme/stories"), json(200));
    let paths = Paths::new(
        Url::parse(&server.base_url()).expect("parses"),
        WorkspaceSlug::parse("acme").expect("valid"),
    );
    let (transport, _) = transport();

    let delivery = transport.follow(
        &format!("{}/api/v4/acme/stories?cursor=x", foreign.base_url()),
        &paths,
        &never(),
    );

    assert!(
        matches!(delivery.result, Err(ClientError::ForeignPage { .. })),
        "{:?}",
        delivery.result
    );
    assert_eq!(foreign.hits(&RequestKey::get("/api/v4/acme/stories")), 0);
}

#[test]
fn an_admitted_next_page_url_is_followed() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get("/api/v4/acme/stories"), json(200));
    let paths = Paths::new(
        Url::parse(&server.base_url()).expect("parses"),
        WorkspaceSlug::parse("acme").expect("valid"),
    );
    let (transport, _) = transport();

    let delivery = transport.follow(
        &format!("{}/api/v4/acme/stories?cursor=x", server.base_url()),
        &paths,
        &never(),
    );

    assert_eq!(delivery.result.expect("arrives").status, 200);
    assert_eq!(
        server
            .last_query(&RequestKey::get("/api/v4/acme/stories"))
            .as_deref(),
        Some("cursor=x")
    );
}
