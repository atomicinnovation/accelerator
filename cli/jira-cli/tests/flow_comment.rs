//! The `comment` subcommands: add, the reshaped list envelope, edit, and the
//! silent delete.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};
use serde_json::Value;

fn comment_path(key: &str) -> String {
    format!("/rest/api/3/issue/{key}/comment")
}

#[test]
fn add_posts_the_comment_and_stamps_added() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(&comment_path("ENG-1")),
        Route::Json {
            status: 201,
            body: r#"{"id":"c-1","body":{"type":"doc"}}"#.to_owned(),
        },
    );
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["comment", "add", "ENG-1", "--body", "looks good"],
    );
    assert!(output.status.success(), "exited {:?}", output.status.code());
    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        envelope.pointer("/outcome").and_then(Value::as_str),
        Some("added")
    );
}

#[test]
fn list_reshapes_the_envelope() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(&comment_path("ENG-1")),
        Route::Json {
            status: 200,
            body: r#"{"startAt":0,"maxResults":50,"total":1,
                     "comments":[{"id":"c-1","body":{"type":"doc"}}]}"#
                .to_owned(),
        },
    );
    let dir = support::scratch(support::CONFIG);

    let output =
        support::run(dir.path(), &server, &["comment", "list", "ENG-1"]);
    assert!(output.status.success(), "exited {:?}", output.status.code());
    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(envelope.pointer("/total").and_then(Value::as_u64), Some(1));
    assert_eq!(
        envelope.pointer("/outcome").and_then(Value::as_str),
        Some("listed")
    );
    assert_eq!(
        envelope.pointer("/truncated").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn delete_is_silent_on_success() {
    let server = MockServer::start();
    server.route(
        RequestKey::new("DELETE", "/rest/api/3/issue/ENG-1/comment/c-1"),
        Route::Status(204),
    );
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["comment", "delete", "ENG-1", "c-1"],
    );
    assert!(output.status.success(), "exited {:?}", output.status.code());
    assert!(output.stdout.is_empty(), "delete prints nothing on success");
}

#[test]
fn a_missing_body_exits_before_the_wire() {
    let server = MockServer::start();
    let dir = support::scratch(support::CONFIG);
    let output =
        support::run(dir.path(), &server, &["comment", "add", "ENG-1"]);
    assert_eq!(output.status.code(), Some(94), "no body → COMMENT_NO_BODY");
}

#[test]
fn a_bad_visibility_exits_before_the_wire() {
    let server = MockServer::start();
    let dir = support::scratch(support::CONFIG);
    let output = support::run(
        dir.path(),
        &server,
        &[
            "comment",
            "add",
            "ENG-1",
            "--body",
            "x",
            "--visibility",
            "nope",
        ],
    );
    assert_eq!(output.status.code(), Some(98), "bad visibility → 98");
}
