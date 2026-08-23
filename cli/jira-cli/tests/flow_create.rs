//! The `create` flow: the JSON envelope with the `created` outcome, the bare
//! validated key `--emit key` projects (byte-for-byte, no keyword), and the
//! fail-closed exit-16 case when the remote returns an unusable key.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};
use serde_json::Value;

const ISSUE: &str = "/rest/api/3/issue";

fn created(body: &str) -> MockServer {
    let server = MockServer::start();
    server.route(
        RequestKey::post(ISSUE),
        Route::Json {
            status: 201,
            body: body.to_owned(),
        },
    );
    server
}

#[test]
fn create_emits_the_json_envelope_and_outcome() {
    let server = created(r#"{"key":"ENG-123","id":"1","self":"u"}"#);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["create", "--summary", "A brand new issue", "--type", "bug"],
    );
    assert!(
        output.status.success(),
        "create exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        envelope.pointer("/outcome").and_then(Value::as_str),
        Some("created")
    );
    assert_eq!(
        envelope.pointer("/key").and_then(Value::as_str),
        Some("ENG-123")
    );

    let sent = String::from_utf8(
        server.last_body(&RequestKey::post(ISSUE)).expect("body"),
    )
    .unwrap();
    assert!(sent.contains("A brand new issue"), "sent: {sent}");
    assert!(
        sent.contains("\"name\":\"bug\""),
        "the issue type rides: {sent}"
    );
}

#[test]
fn emit_key_prints_only_the_bare_key() {
    let server = created(r#"{"key":"ENG-123","id":"1","self":"u"}"#);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["create", "--summary", "x", "--emit", "key"],
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"ENG-123\n", "the bare key, no keyword");
}

#[test]
fn a_control_byte_key_fails_closed_at_exit_16() {
    let server = created("{\"key\":\"ENG-1\\u0001\"}");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["create", "--summary", "x", "--emit", "key"],
    );
    assert_eq!(output.status.code(), Some(16));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("do NOT blindly retry"),
        "stderr steers to reconcile: {stderr}"
    );
}

#[test]
fn a_missing_summary_exits_before_the_wire() {
    let server = created("{}");
    let dir = support::scratch(support::CONFIG);
    let output = support::run(dir.path(), &server, &["create"]);
    assert_eq!(output.status.code(), Some(102));
    assert_eq!(server.hits(&RequestKey::post(ISSUE)), 0, "nothing sent");
}
