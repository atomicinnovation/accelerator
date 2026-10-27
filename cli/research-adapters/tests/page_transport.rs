#![allow(clippy::expect_used, clippy::panic)]

use std::net::SocketAddr;
use std::str::FromStr as _;
use std::time::Duration;
use std::time::Instant;

use http_test_support::refused_base_url;
use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use reqwest::dns::Name;
use reqwest::dns::Resolve as _;
use research::sources::web::fetch::PageRequest;
use research::sources::web::fetch::PageResponse;
use research::sources::web::fetch::PageTransport as _;
use research::sources::web::fetch::PAGE_ACCEPT;
use research::sources::web::page_url::PageUrl;
use research_adapters::page_transport::HttpPageTransport;
use research_adapters::page_transport::PinnedResolver;

const PAGE: &str = "/page";

fn page() -> RequestKey {
    RequestKey::get(PAGE)
}

fn url(raw: &str) -> PageUrl {
    PageUrl::parse(raw).expect("a page URL")
}

fn get_pinned(
    raw: &str,
    addresses: &[SocketAddr],
    max_body: usize,
    timeout: Duration,
) -> PageResponse {
    HttpPageTransport.get(&PageRequest {
        url: &url(raw),
        addresses,
        accept: PAGE_ACCEPT,
        max_body,
        timeout,
    })
}

fn get(server: &MockHTTPServer, raw: &str) -> PageResponse {
    get_pinned(raw, &[server.address()], 1024, Duration::from_secs(5))
}

fn under_fixture_host(server: &MockHTTPServer) -> String {
    format!(
        "http://fixture.accelerator.test:{}{PAGE}",
        server.address().port()
    )
}

fn received(
    response: PageResponse,
) -> (u16, Option<String>, Option<String>, Vec<u8>) {
    match response {
        PageResponse::Received {
            status,
            location,
            content_type,
            body,
        } => (status, location, content_type, body),
        other => panic!("expected a response, got {other:?}"),
    }
}

fn headers(status: u16, headers: &[(&str, &str)], body: &str) -> Route {
    Route::Headers {
        status,
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        body: body.to_owned(),
    }
}

#[test]
fn a_redirect_is_returned_with_its_location_rather_than_followed() {
    let server = MockHTTPServer::start();
    server.route(
        page(),
        Route::Redirect {
            status: 302,
            location: "/elsewhere".to_owned(),
        },
    );
    server.route(RequestKey::get("/elsewhere"), Route::Status(200));

    let (status, location, _, _) =
        received(get(&server, &under_fixture_host(&server)));

    assert_eq!(status, 302);
    assert_eq!(location.as_deref(), Some("/elsewhere"));
    assert_eq!(server.hits(&RequestKey::get("/elsewhere")), 0);
}

#[test]
fn a_location_carrying_raw_utf8_arrives_intact() {
    let server = MockHTTPServer::start();
    server.route(page(), headers(302, &[("Location", "/bücher")], ""));

    let (_, location, _, _) =
        received(get(&server, &under_fixture_host(&server)));

    assert_eq!(location.as_deref(), Some("/bücher"));
}

fn with_location_bytes(status: u16, location: &[u8]) -> Route {
    let mut response = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: text/html\r\n\
         Content-Length: 0\r\nConnection: close\r\nLocation: "
    )
    .into_bytes();
    response.extend_from_slice(location);
    response.extend_from_slice(b"\r\n\r\n");
    Route::Raw(response)
}

#[test]
fn a_redirect_whose_location_is_not_utf8_is_refused() {
    let server = MockHTTPServer::start();
    server.route(page(), with_location_bytes(302, b"/caf\xe9"));

    assert_eq!(
        get(&server, &under_fixture_host(&server)),
        PageResponse::Refused
    );
}

#[test]
fn a_page_whose_location_is_not_utf8_is_received_without_it() {
    let server = MockHTTPServer::start();
    server.route(page(), with_location_bytes(200, b"/caf\xe9"));

    let (status, location, _, _) =
        received(get(&server, &under_fixture_host(&server)));

    assert_eq!(status, 200);
    assert_eq!(location, None);
}

#[test]
fn a_body_cut_off_mid_transfer_is_a_connection_failure() {
    let server = MockHTTPServer::start();
    server.route(
        page(),
        Route::Truncated {
            body: b"<html><head></head></html>".to_vec(),
            sent: 6,
        },
    );

    assert_eq!(
        get(&server, &under_fixture_host(&server)),
        PageResponse::ConnectionFailed
    );
}

#[test]
fn the_content_type_and_body_are_surfaced() {
    let server = MockHTTPServer::start();
    server.route(
        page(),
        headers(
            200,
            &[("Content-Type", "text/html; charset=utf-8")],
            "<html></html>",
        ),
    );

    let (status, location, content_type, body) =
        received(get(&server, &under_fixture_host(&server)));

    assert_eq!(status, 200);
    assert_eq!(location, None);
    assert_eq!(content_type.as_deref(), Some("text/html; charset=utf-8"));
    assert_eq!(body, b"<html></html>");
}

#[test]
fn a_body_is_read_up_to_its_cap_and_truncated_past_it() {
    for (length, delivered) in [(64, 64), (65, 64)] {
        let server = MockHTTPServer::start();
        server.route(
            page(),
            Route::Bytes {
                status: 200,
                body: vec![b'x'; length],
            },
        );

        let (_, _, _, body) = received(get_pinned(
            &under_fixture_host(&server),
            &[server.address()],
            64,
            Duration::from_secs(5),
        ));

        assert_eq!(body.len(), delivered, "{length}");
    }
}

#[test]
fn the_request_names_the_client_and_what_it_accepts() {
    let server = MockHTTPServer::start();
    server.route(page(), Route::Status(200));

    get(&server, &under_fixture_host(&server));

    assert!(server
        .last_header(&page(), "User-Agent")
        .is_some_and(|agent| agent.starts_with("accelerator-research/")));
    assert_eq!(
        server.last_header(&page(), "Accept").as_deref(),
        Some(PAGE_ACCEPT)
    );
}

#[test]
fn a_body_that_stalls_past_the_request_budget_is_a_timeout() {
    let server = MockHTTPServer::start();
    server.route(page(), Route::Stall(Duration::from_secs(2)));

    let response = get_pinned(
        &under_fixture_host(&server),
        &[server.address()],
        1024,
        Duration::from_millis(200),
    );

    assert_eq!(response, PageResponse::TimedOut);
}

#[test]
fn a_body_that_trickles_past_the_request_budget_is_a_timeout() {
    let server = MockHTTPServer::start();
    server.route(
        page(),
        Route::Trickle {
            body: vec![b'x'; 40],
            gap: Duration::from_millis(50),
        },
    );
    let started = Instant::now();

    let response = get_pinned(
        &under_fixture_host(&server),
        &[server.address()],
        1024,
        Duration::from_millis(500),
    );

    assert_eq!(response, PageResponse::TimedOut);
    assert!(started.elapsed() < Duration::from_millis(1500));
}

#[test]
fn a_closed_port_is_a_connection_failure() {
    let closed = refused_base_url();
    let port = closed
        .rsplit(':')
        .next()
        .and_then(|port| port.parse().ok())
        .expect("a port");

    let response = get_pinned(
        &format!("{closed}/"),
        &[SocketAddr::from(([127, 0, 0, 1], port))],
        1024,
        Duration::from_secs(5),
    );

    assert_eq!(response, PageResponse::ConnectionFailed);
}

#[test]
fn a_request_pinned_to_the_mock_reaches_it_under_any_name() {
    for name in ["fixture.accelerator.test", "bücher.accelerator.test"] {
        let server = MockHTTPServer::start();
        server.route(page(), Route::Status(200));

        let (status, _, _, _) = received(get(
            &server,
            &format!("http://{name}:{}{PAGE}", server.address().port()),
        ));

        assert_eq!(status, 200, "{name}");
        assert_eq!(server.hits(&page()), 1, "{name}");
    }
}

#[test]
fn a_request_pinned_to_an_ipv6_literal_reaches_it() {
    let server = MockHTTPServer::start_on("[::1]:0");
    server.route(page(), Route::Status(200));

    let (status, _, _, _) = received(get(
        &server,
        &format!("http://[::1]:{}{PAGE}", server.address().port()),
    ));

    assert_eq!(status, 200);
}

#[test]
fn a_host_the_client_would_read_differently_is_refused_unsent() {
    for host in [
        "a%2Fb",
        "exa%20mple.org",
        "a%40127.0.0.1",
        "127.0.0.1%2F.example",
        "127.0.0.1%23.example",
        "127.0.0.1%3F.example",
    ] {
        let server = MockHTTPServer::start();
        server.route(page(), Route::Status(200));

        let response = get(
            &server,
            &format!("http://{host}:{}{PAGE}", server.address().port()),
        );

        assert_eq!(response, PageResponse::Refused, "{host}");
        assert_eq!(server.hits(&page()), 0, "{host}");
    }
}

fn resolved(resolver: &PinnedResolver, name: &str) -> Option<Vec<SocketAddr>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    let name = Name::from_str(name).expect("a name");
    runtime
        .block_on(resolver.resolve(name))
        .ok()
        .map(Iterator::collect)
}

#[test]
fn the_pinned_resolver_answers_only_its_own_name() {
    let pinned = vec![SocketAddr::from(([93, 184, 216, 34], 443))];
    let resolver = PinnedResolver::new("example.org", pinned.clone());

    assert_eq!(resolved(&resolver, "example.org"), Some(pinned));
    for other in ["other.example", "localhost", "example.org.evil"] {
        assert_eq!(resolved(&resolver, other), None, "{other}");
    }
}
