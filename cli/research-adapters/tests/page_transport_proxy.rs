//! Alone in its own test binary: the proxy variables it sets are process-wide,
//! so they must be in place before any client exists and seen by no other
//! test.

#![allow(clippy::expect_used)]

use std::time::Duration;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use research::sources::web::fetch::PageRequest;
use research::sources::web::fetch::PageResponse;
use research::sources::web::fetch::PageTransport as _;
use research::sources::web::fetch::PAGE_ACCEPT;
use research::sources::web::page_url::PageUrl;
use research_adapters::page_transport::HttpPageTransport;

#[test]
fn a_configured_proxy_is_never_used() {
    let unreachable = "http://127.0.0.1:9";
    // SAFETY: this binary runs only this test, so no other thread reads the
    // environment while it is written.
    unsafe {
        std::env::set_var("HTTP_PROXY", unreachable);
        std::env::set_var("ALL_PROXY", unreachable);
    }
    let server = MockHTTPServer::start();
    server.route(RequestKey::get("/page"), Route::Status(200));
    let url = PageUrl::parse(&format!(
        "http://fixture.accelerator.test:{}/page",
        server.address().port()
    ))
    .expect("a page URL");

    let response = HttpPageTransport.get(&PageRequest {
        url: &url,
        addresses: &[server.address()],
        accept: PAGE_ACCEPT,
        max_body: 1024,
        timeout: Duration::from_secs(5),
    });

    assert!(
        matches!(response, PageResponse::Received { status: 200, .. }),
        "{response:?}"
    );
    assert_eq!(server.hits(&RequestKey::get("/page")), 1);
}
