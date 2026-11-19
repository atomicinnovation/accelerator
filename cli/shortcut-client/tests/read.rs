//! A read separates a request that never answered from an answer the client
//! cannot use.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use shortcut_client::client::ReadFailure;
use shortcut_client::Url;
use support::client::client;
use support::client::path;

#[test]
fn a_success_that_is_not_json_is_an_unusable_body() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path("/stories/14")),
        Route::Bytes {
            status: 200,
            body: b"<html>not json</html>".to_vec(),
        },
    );
    let url =
        Url::parse(&format!("{}{}", server.base_url(), path("/stories/14")))
            .expect("parses");

    let failure = client(&server)
        .read(&url)
        .expect_err("the body is unusable");

    assert!(
        matches!(failure, ReadFailure::UnusableBody(_)),
        "{failure:?}"
    );
}
