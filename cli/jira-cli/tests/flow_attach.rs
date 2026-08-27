//! The `attach` flow: a file inside the project root is posted as multipart and
//! stamps the `attached` outcome; a missing file is refused.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};
use serde_json::Value;

const ATTACHMENTS: &str = "/rest/api/3/issue/ENG-1/attachments";

#[test]
fn a_file_is_attached_and_stamps_the_outcome() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(ATTACHMENTS),
        Route::Json {
            status: 200,
            body: r#"[{"id":"att-1","filename":"note.txt"}]"#.to_owned(),
        },
    );
    let dir = support::scratch(support::CONFIG);
    std::fs::write(dir.path().join("note.txt"), "hello\n").unwrap();

    let output =
        support::run(dir.path(), &server, &["attach", "ENG-1", "note.txt"]);
    assert!(
        output.status.success(),
        "attach exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        response.pointer("/0/id").and_then(Value::as_str),
        Some("att-1"),
        "the attachments array is emitted verbatim"
    );
    assert_eq!(server.hits(&RequestKey::post(ATTACHMENTS)), 1);
}

#[test]
fn a_missing_file_is_refused() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(ATTACHMENTS),
        Route::Json {
            status: 200,
            body: "[]".to_owned(),
        },
    );
    let dir = support::scratch(support::CONFIG);

    let output =
        support::run(dir.path(), &server, &["attach", "ENG-1", "no-such.txt"]);
    assert_eq!(output.status.code(), Some(132), "missing file → 132");
    assert_eq!(
        server.hits(&RequestKey::post(ATTACHMENTS)),
        0,
        "nothing sent"
    );
}
