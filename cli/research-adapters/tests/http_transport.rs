#![allow(clippy::expect_used, clippy::panic)]

use std::net::TcpListener;
use std::time::Duration;

use http_test_support::MockServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use research::classify::Received;
use research::classify::Response;
use research::fetch::Transport as _;
use research::request::ApiKey;
use research::request::Endpoint;
use research::request::KeySource;
use research::request::OpenAlexId;
use research::request::OpenAlexRequest;
use research::request::UpstreamRequest;
use research_adapters::transport::HttpTransport;

const WORK: &str = "/works/W1";

fn lookup(base: &str, key: Option<&ApiKey>) -> UpstreamRequest {
    OpenAlexRequest::Lookup(
        OpenAlexId::parse("W1").unwrap_or_else(|error| panic!("{error}")),
    )
    .upstream(&Endpoint::new(base), key)
}

fn transport(per_request: Duration) -> HttpTransport {
    HttpTransport::new(per_request).expect("transport")
}

fn received(response: Response) -> Received {
    match response {
        Response::Received(received) => received,
        other => panic!("expected a received response, got {other:?}"),
    }
}

#[test]
fn a_body_that_stalls_past_the_request_budget_is_a_timeout() {
    let server = MockServer::start();
    server.route(RequestKey::get(WORK), Route::Stall(Duration::from_secs(2)));

    let response = transport(Duration::from_millis(200))
        .send(&lookup(&server.base_url(), None));

    assert_eq!(response, Response::TimedOut);
}

#[test]
fn a_closed_port_is_a_connection_failure() {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();

    let response = transport(Duration::from_secs(5))
        .send(&lookup(&format!("http://127.0.0.1:{port}"), None));

    assert_eq!(response, Response::ConnectionFailed);
}

#[test]
fn the_status_body_and_rate_limit_headers_are_reported() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(WORK),
        Route::Headers {
            status: 429,
            headers: vec![
                ("Retry-After".to_owned(), "7".to_owned()),
                ("X-RateLimit-Remaining".to_owned(), "5".to_owned()),
                ("X-RateLimit-Credits-Required".to_owned(), "10".to_owned()),
            ],
            body: "slow down".to_owned(),
        },
    );

    let received = received(
        transport(Duration::from_secs(5))
            .send(&lookup(&server.base_url(), None)),
    );

    assert_eq!(
        received,
        Received {
            status: 429,
            retry_after: Some(Duration::from_secs(7)),
            rate_limit_remaining: Some(5),
            rate_limit_credits_required: Some(10),
            body: b"slow down".to_vec(),
        }
    );
}

#[test]
fn the_request_carries_its_headers_and_only_its_own_bearer() {
    let server = MockServer::start();
    server.route(RequestKey::get(WORK), Route::Status(200));
    let transport = transport(Duration::from_secs(5));
    let key = ApiKey::new("sentinel-key".to_owned(), KeySource::new("env"));

    transport.send(&lookup(&server.base_url(), None));
    let keyless_authorization =
        server.last_header(&RequestKey::get(WORK), "Authorization");
    transport.send(&lookup(&server.base_url(), Some(&key)));

    assert_eq!(keyless_authorization, None);
    assert_eq!(
        server.last_header(&RequestKey::get(WORK), "Authorization"),
        Some("Bearer sentinel-key".to_owned())
    );
    assert!(server
        .last_header(&RequestKey::get(WORK), "User-Agent")
        .is_some_and(|agent| agent.starts_with("accelerator-research/")));
    assert!(server
        .last_query(&RequestKey::get(WORK))
        .is_some_and(|query| query.starts_with("select=")));
}

#[test]
fn a_same_origin_redirect_is_followed() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(WORK),
        Route::Redirect {
            status: 301,
            location: format!("{}/works/W2", server.base_url()),
        },
    );
    server.route(RequestKey::get("/works/W2"), Route::Status(200));

    let received = received(
        transport(Duration::from_secs(5))
            .send(&lookup(&server.base_url(), None)),
    );

    assert_eq!(received.status, 200);
    assert_eq!(server.hits(&RequestKey::get("/works/W2")), 1);
}

#[test]
fn a_redirect_to_another_origin_is_not_followed() {
    let server = MockServer::start();
    let elsewhere = MockServer::start();
    server.route(
        RequestKey::get(WORK),
        Route::Redirect {
            status: 301,
            location: format!("{}/works/W2", elsewhere.base_url()),
        },
    );
    let key = ApiKey::new("sentinel-key".to_owned(), KeySource::new("env"));

    let received = received(
        transport(Duration::from_secs(5))
            .send(&lookup(&server.base_url(), Some(&key))),
    );

    assert_eq!(received.status, 301);
    assert_eq!(elsewhere.hits(&RequestKey::get("/works/W2")), 0);
}

#[test]
fn a_redirect_that_changes_scheme_is_not_followed() {
    let server = MockServer::start();
    let https = server.base_url().replacen("http://", "https://", 1);
    server.route(
        RequestKey::get(WORK),
        Route::Redirect {
            status: 301,
            location: format!("{https}/works/W2"),
        },
    );

    let received = received(
        transport(Duration::from_secs(5))
            .send(&lookup(&server.base_url(), None)),
    );

    assert_eq!(received.status, 301);
}

#[test]
fn a_fourth_redirect_is_not_followed() {
    let server = MockServer::start();
    for hop in 1..=4 {
        server.route(
            RequestKey::get(&format!("/works/W{hop}")),
            Route::Redirect {
                status: 301,
                location: format!("{}/works/W{}", server.base_url(), hop + 1),
            },
        );
    }
    server.route(RequestKey::get("/works/W5"), Route::Status(200));

    let received = received(
        transport(Duration::from_secs(5))
            .send(&lookup(&server.base_url(), None)),
    );

    assert_eq!(received.status, 301);
    assert_eq!(server.hits(&RequestKey::get("/works/W4")), 1);
    assert_eq!(server.hits(&RequestKey::get("/works/W5")), 0);
}

#[test]
fn a_body_over_eight_mebibytes_is_withheld() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(WORK),
        Route::Bytes {
            status: 200,
            body: vec![b'x'; 8 * 1024 * 1024 + 1],
        },
    );

    let received = received(
        transport(Duration::from_secs(10))
            .send(&lookup(&server.base_url(), None)),
    );

    assert_eq!(received.status, 200);
    assert!(received.body.is_empty());
}

#[test]
fn a_body_of_exactly_eight_mebibytes_is_delivered() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(WORK),
        Route::Bytes {
            status: 200,
            body: vec![b'x'; 8 * 1024 * 1024],
        },
    );

    let received = received(
        transport(Duration::from_secs(10))
            .send(&lookup(&server.base_url(), None)),
    );

    assert_eq!(received.body.len(), 8 * 1024 * 1024);
}
