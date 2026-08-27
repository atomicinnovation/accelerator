//! The `comment add` flow: one `commentCreate` mutation carrying the verbatim
//! Markdown body, and the `added` outcome keyword in the JSON envelope.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::{MockServer, RequestKey};
use serde_json::Value;

fn install(server: &MockServer) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios/comment-201.json");
    Scenario::load(&path).expect("scenario").install(server);
}

#[test]
fn comment_add_posts_the_body_and_reports_the_added_keyword() {
    let server = MockServer::start();
    install(&server);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["comment", "add", "BLA-42", "--body", "a review remark"],
    );

    assert!(
        output.status.success(),
        "comment exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(stdout.get("outcome").and_then(Value::as_str), Some("added"));

    let key = RequestKey::post("/graphql");
    assert_eq!(server.hits(&key), 1);
    let body = server.last_body(&key).expect("a recorded body");
    let sent = String::from_utf8_lossy(&body);
    assert!(sent.contains("commentCreate"), "sent: {sent}");
    assert!(sent.contains("a review remark"), "the body passes through");
}
