//! Attach: link mode, the three-step binary upload, the SSRF trust boundary
//! around the server-supplied URLs, the echoed-header allowlist, and the
//! non-atomic failure semantics.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use std::path::Path;
use std::time::Duration;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::upload::UPLOAD_TIMEOUT;
use linear_client::SurfaceError;
use serde_json::Value;
use support::client::client_for;
use tempfile::TempDir;
use tracker_support::TransportConfig;

const GRAPHQL: &str = "/graphql";
const KEY: &str = "ENG-1";

fn graphql() -> RequestKey {
    RequestKey::post(GRAPHQL)
}

fn upload_put() -> RequestKey {
    RequestKey::put("/upload")
}

const fn json_route(body: String) -> Route {
    Route::Json { status: 200, body }
}

fn write_file(root: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = root.join(name);
    std::fs::write(&path, bytes).expect("the fixture file is written");
    path
}

/// A `fileUpload` response nominating `upload_url`/`asset_url` and echoing
/// `headers` (a JSON array literal).
fn file_upload_body(
    upload_url: &str,
    asset_url: &str,
    headers: &str,
) -> String {
    format!(
        "{{\"data\":{{\"fileUpload\":{{\"success\":true,\
         \"uploadFile\":{{\"uploadUrl\":\"{upload_url}\",\
         \"assetUrl\":\"{asset_url}\",\"headers\":{headers}}}}}}}}}"
    )
}

fn attachment_created() -> String {
    "{\"data\":{\"attachmentCreate\":{\"success\":true,\
     \"attachment\":{\"id\":\"a1\"}}}}"
        .to_owned()
}

#[test]
fn link_mode_sends_one_attachment_create() {
    let server = MockServer::start();
    server.route(graphql(), json_route(attachment_created()));
    let client = client_for(&server, TransportConfig::default());

    client
        .attach_link(KEY, "https://example.com/spec", Some("Spec"))
        .expect("the link is attached");

    assert_eq!(server.hits(&graphql()), 1);
    let sent: Value =
        serde_json::from_slice(&server.last_body(&graphql()).expect("a body"))
            .expect("JSON");
    assert!(sent["query"]
        .as_str()
        .expect("a document")
        .contains("attachmentCreate"));
    assert_eq!(sent["variables"]["input"]["issueId"], KEY);
    assert_eq!(sent["variables"]["input"]["title"], "Spec");
    assert_eq!(
        sent["variables"]["input"]["url"],
        "https://example.com/spec"
    );
}

#[test]
fn a_non_http_link_url_is_refused() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_link(KEY, "ftp://example.com/x", None)
        .expect_err("a non-http(s) link is refused");

    assert!(matches!(error, SurfaceError::BadLinkUrl { .. }), "{error}");
    assert_eq!(server.hits(&graphql()), 0);
}

#[test]
fn binary_mode_makes_exactly_three_requests_and_the_put_carries_no_auth() {
    let root = TempDir::new().expect("a temp root");
    let file = write_file(root.path(), "a.txt", b"attachment bytes");

    let server = MockServer::start();
    let base = server.base_url();
    server.route(
        graphql(),
        Route::Sequence(vec![
            json_route(file_upload_body(
                &format!("{base}/upload"),
                &format!("{base}/asset"),
                "[]",
            )),
            json_route(attachment_created()),
        ]),
    );
    server.route(upload_put(), Route::Status(200));
    let client = client_for(&server, TransportConfig::default());

    client
        .attach_file(KEY, &file, None)
        .expect("the upload succeeds");

    assert_eq!(
        server.hits(&graphql()),
        2,
        "fileUpload then attachmentCreate"
    );
    assert_eq!(server.hits(&upload_put()), 1, "one PUT between them");
    assert_eq!(
        server.last_header(&upload_put(), "authorization"),
        None,
        "the PUT to the server-nominated host must carry no bearer token"
    );
    assert!(
        server.last_header(&graphql(), "authorization").is_some(),
        "the GraphQL requests still authenticate"
    );

    // Step 3 registers the returned assetUrl, not the uploadUrl.
    let sent: Value =
        serde_json::from_slice(&server.last_body(&graphql()).expect("a body"))
            .expect("JSON");
    assert_eq!(sent["variables"]["input"]["url"], format!("{base}/asset"));
}

#[test]
fn file_upload_carries_the_sniffed_content_type_filename_and_size() {
    let root = TempDir::new().expect("a temp root");
    let png = write_file(root.path(), "logo.png", b"\x89PNG\r\n\x1a\n\x00\x00");

    let server = MockServer::start();
    // The PUT fails, so attachmentCreate is never sent and the last /graphql
    // body is the fileUpload request this test asserts.
    server.route(
        graphql(),
        json_route(file_upload_body(
            &format!("{}/upload", server.base_url()),
            &format!("{}/asset", server.base_url()),
            "[]",
        )),
    );
    server.route(upload_put(), Route::Status(500));
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, &png, None)
        .expect_err("the PUT fails");

    assert!(
        matches!(error, SurfaceError::UploadFailed { .. }),
        "{error}"
    );
    let sent: Value =
        serde_json::from_slice(&server.last_body(&graphql()).expect("a body"))
            .expect("JSON");
    assert_eq!(sent["variables"]["contentType"], "image/png");
    assert_eq!(sent["variables"]["filename"], "logo.png");
    assert_eq!(sent["variables"]["size"], 10);
    assert_eq!(server.hits(&upload_put()), 3, "the PUT retries three times");
}

#[test]
fn an_upload_url_off_linear_app_is_refused_before_any_bytes_move() {
    let root = TempDir::new().expect("a temp root");
    let file = write_file(root.path(), "a.txt", b"bytes");

    let server = MockServer::start();
    server.route(
        graphql(),
        json_route(file_upload_body(
            "https://uploads.linear.app.evil.com/x",
            &format!("{}/asset", server.base_url()),
            "[]",
        )),
    );
    server.route(upload_put(), Route::Status(200));
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, &file, None)
        .expect_err("a look-alike upload host is refused");

    assert!(
        matches!(
            error,
            SurfaceError::BadUploadUrl {
                role: "uploadUrl",
                ..
            }
        ),
        "{error}"
    );
    assert_eq!(server.hits(&upload_put()), 0);
}

#[test]
fn a_non_https_non_loopback_upload_url_is_refused() {
    let root = TempDir::new().expect("a temp root");
    let file = write_file(root.path(), "a.txt", b"bytes");

    let server = MockServer::start();
    server.route(
        graphql(),
        json_route(file_upload_body(
            "http://uploads.linear.app/x",
            &format!("{}/asset", server.base_url()),
            "[]",
        )),
    );
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, &file, None)
        .expect_err("a non-https upload host is refused");

    assert!(
        matches!(
            error,
            SurfaceError::BadUploadUrl {
                role: "uploadUrl",
                ..
            }
        ),
        "{error}"
    );
}

#[test]
fn an_asset_url_on_a_foreign_host_is_refused_before_step_two() {
    let root = TempDir::new().expect("a temp root");
    let file = write_file(root.path(), "a.txt", b"bytes");

    let server = MockServer::start();
    server.route(
        graphql(),
        json_route(file_upload_body(
            &format!("{}/upload", server.base_url()),
            "https://evil.com/asset",
            "[]",
        )),
    );
    server.route(upload_put(), Route::Status(200));
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, &file, None)
        .expect_err("an untrusted assetUrl is refused");

    assert!(
        matches!(
            error,
            SurfaceError::BadUploadUrl {
                role: "assetUrl",
                ..
            }
        ),
        "{error}"
    );
    assert_eq!(server.hits(&upload_put()), 0, "no bytes move");
}

#[test]
fn a_failed_upload_diagnostic_carries_no_signed_query_string() {
    let root = TempDir::new().expect("a temp root");
    let file = write_file(root.path(), "a.txt", b"bytes");

    let server = MockServer::start();
    server.route(
        graphql(),
        json_route(file_upload_body(
            &format!("{}/upload?sig=SUPERSECRET", server.base_url()),
            &format!("{}/asset", server.base_url()),
            "[]",
        )),
    );
    server.route(upload_put(), Route::Status(503));
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, &file, None)
        .expect_err("the PUT fails");

    let text = error.to_string();
    assert!(
        !text.contains("SUPERSECRET"),
        "the signed query leaked: {text}"
    );
    assert!(
        text.contains("/upload"),
        "the base URL is still named: {text}"
    );
}

#[test]
fn an_x_amz_header_is_forwarded_and_a_foreign_header_is_dropped() {
    let root = TempDir::new().expect("a temp root");
    let file = write_file(root.path(), "a.txt", b"bytes");

    let server = MockServer::start();
    let base = server.base_url();
    server.route(
        graphql(),
        Route::Sequence(vec![
            json_route(file_upload_body(
                &format!("{base}/upload"),
                &format!("{base}/asset"),
                "[{\"key\":\"x-amz-meta-token\",\"value\":\"signed\"},\
                 {\"key\":\"x-forwarded-for\",\"value\":\"1.2.3.4\"}]",
            )),
            json_route(attachment_created()),
        ]),
    );
    server.route(upload_put(), Route::Status(200));
    let client = client_for(&server, TransportConfig::default());

    client
        .attach_file(KEY, &file, None)
        .expect("the upload succeeds");

    assert_eq!(
        server
            .last_header(&upload_put(), "x-amz-meta-token")
            .as_deref(),
        Some("signed"),
        "the signed x-amz header is forwarded"
    );
    assert_eq!(
        server.last_header(&upload_put(), "x-forwarded-for"),
        None,
        "a header outside x-amz-* is dropped"
    );
}

#[test]
fn an_echoed_header_carrying_crlf_is_refused() {
    let root = TempDir::new().expect("a temp root");
    let file = write_file(root.path(), "a.txt", b"bytes");

    let server = MockServer::start();
    let base = server.base_url();
    server.route(
        graphql(),
        json_route(file_upload_body(
            &format!("{base}/upload"),
            &format!("{base}/asset"),
            "[{\"key\":\"x-amz-meta-x\",\"value\":\"a\\r\\nInjected: 1\"}]",
        )),
    );
    server.route(upload_put(), Route::Status(200));
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, &file, None)
        .expect_err("a CRLF-bearing echoed header is refused");

    assert!(
        matches!(error, SurfaceError::BadEchoedHeader { .. }),
        "{error}"
    );
    assert_eq!(server.hits(&upload_put()), 0);
}

#[test]
fn a_redirect_response_to_the_put_is_refused_rather_than_followed() {
    let root = TempDir::new().expect("a temp root");
    let file = write_file(root.path(), "a.txt", b"bytes");

    let server = MockServer::start();
    let base = server.base_url();
    server.route(
        graphql(),
        Route::Sequence(vec![
            json_route(file_upload_body(
                &format!("{base}/upload"),
                &format!("{base}/asset"),
                "[]",
            )),
            json_route(attachment_created()),
        ]),
    );
    server.route(
        upload_put(),
        Route::Redirect {
            status: 302,
            location: "https://evil.com/take-my-bytes".to_owned(),
        },
    );
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, &file, None)
        .expect_err("a 30x is not followed");

    assert!(
        matches!(error, SurfaceError::UploadFailed { .. }),
        "{error}"
    );
    assert_eq!(
        server.hits(&upload_put()),
        3,
        "the 302 is retried, not chased"
    );
    assert_eq!(
        server.hits(&graphql()),
        1,
        "attachmentCreate is never reached"
    );
}

#[test]
fn the_upload_timeout_is_sixty_seconds_while_the_port_stays_thirty() {
    assert_eq!(UPLOAD_TIMEOUT, Duration::from_secs(60));
    assert_eq!(
        TransportConfig::default().timeout,
        Duration::from_secs(30),
        "the upload's longer timeout does not move the port default"
    );
}

#[test]
fn a_step_three_failure_after_a_successful_put_reports_an_orphaned_asset() {
    let root = TempDir::new().expect("a temp root");
    let file = write_file(root.path(), "a.txt", b"bytes");

    let server = MockServer::start();
    let base = server.base_url();
    server.route(
        graphql(),
        Route::Sequence(vec![
            json_route(file_upload_body(
                &format!("{base}/upload"),
                &format!("{base}/asset"),
                "[]",
            )),
            json_route(
                "{\"errors\":[{\"message\":\"registration failed\"}]}"
                    .to_owned(),
            ),
        ]),
    );
    server.route(upload_put(), Route::Status(200));
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, &file, None)
        .expect_err("registration fails after the upload");

    assert!(
        matches!(error, SurfaceError::RegisterFailed { .. }),
        "{error}"
    );
    assert_eq!(server.hits(&upload_put()), 1, "the bytes did move");
    assert_eq!(
        server.hits(&graphql()),
        2,
        "fileUpload and attachmentCreate"
    );
}

#[test]
fn a_missing_file_is_refused() {
    let root = TempDir::new().expect("a temp root");
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, &root.path().join("absent.txt"), None)
        .expect_err("a missing file is refused");

    assert!(matches!(error, SurfaceError::FileRefused { .. }), "{error}");
    assert_eq!(server.hits(&graphql()), 0);
}

#[cfg(unix)]
#[test]
fn a_device_file_is_refused_by_the_handle_check() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach_file(KEY, Path::new("/dev/null"), None)
        .expect_err("a device is not a regular file");

    assert!(matches!(error, SurfaceError::FileRefused { .. }), "{error}");
    assert_eq!(server.hits(&graphql()), 0);
}
