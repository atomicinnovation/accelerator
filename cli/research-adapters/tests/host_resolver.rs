#![allow(clippy::expect_used)]

use std::net::SocketAddr;
use std::time::Duration;
use std::time::Instant;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use network::Host;
use research::sources::web::fetch::HostResolver as _;
use research::sources::web::fetch::PageRequest;
use research::sources::web::fetch::PageResponse;
use research::sources::web::fetch::PageTransport as _;
use research::sources::web::fetch::ResolveFailure;
use research::sources::web::fetch::PAGE_ACCEPT;
use research::sources::web::page_url::PageUrl;
use research_adapters::host_resolver::bounded_lookup;
use research_adapters::host_resolver::DnsHostResolver;
use research_adapters::page_transport::HttpPageTransport;

fn localhost() -> Host {
    Host::canonicalise("localhost").expect("a host")
}

#[test]
fn a_lookup_that_never_answers_is_unresolved_once_its_bound_passes() {
    let started = Instant::now();

    let outcome = bounded_lookup(
        Duration::from_millis(50),
        std::future::pending::<Result<(), ResolveFailure>>(),
    );

    assert_eq!(outcome, Err(ResolveFailure::Unresolved));
    assert!(started.elapsed() < Duration::from_millis(500));
}

#[test]
fn localhost_resolves_to_a_loopback_address_carrying_the_port() {
    let addresses = DnsHostResolver
        .resolve(&localhost(), 8080, Duration::from_secs(5))
        .expect("localhost resolves");

    assert!(!addresses.is_empty());
    for address in addresses {
        assert!(address.ip().is_loopback(), "{address}");
        assert_eq!(address.port(), 8080);
    }
}

#[test]
fn a_host_with_no_ascii_form_is_refused_unasked() {
    let folding = Host::canonicalise("１２７.０.０.１").expect("a host");

    assert_eq!(
        DnsHostResolver.resolve(&folding, 80, Duration::from_secs(5)),
        Err(ResolveFailure::Refused)
    );
}

#[test]
fn a_resolved_address_can_then_be_fetched_from_in_the_same_process() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get("/page"), Route::Status(200));
    let port = server.address().port();
    let resolved = DnsHostResolver
        .resolve(&localhost(), port, Duration::from_secs(5))
        .expect("localhost resolves");
    let reachable = resolved
        .into_iter()
        .filter(SocketAddr::is_ipv4)
        .collect::<Vec<_>>();
    let url = PageUrl::parse(&format!("http://localhost:{port}/page"))
        .expect("a page URL");

    let response = HttpPageTransport.get(&PageRequest {
        url: &url,
        addresses: &reachable,
        accept: PAGE_ACCEPT,
        max_body: 1024,
        timeout: Duration::from_secs(5),
    });

    assert!(
        matches!(response, PageResponse::Received { status: 200, .. }),
        "{response:?}"
    );
}
