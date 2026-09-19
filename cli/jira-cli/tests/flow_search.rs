//! The `search` flow: the merged `issues` envelope with an `outcome` stamp, the
//! fail-loud cap-hit exit and its truncated envelope, the composed-JQL audit
//! line on stderr, and its `--quiet` suppression.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};
use serde_json::Value;

const SEARCH: &str = "/rest/api/3/search/jql";

fn server_returning(body: &str) -> MockServer {
    let server = MockServer::start();
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: body.to_owned(),
        },
    );
    server
}

#[test]
fn search_within_the_cap_echoes_results_and_audits_the_jql() {
    // A page with no cursor completes the walk, so the search exits zero — the
    // negative control against an always-abort.
    let server = server_returning(r#"{"issues":[{"key":"ENG-1"}]}"#);
    let dir = support::scratch(support::CONFIG);

    let output =
        support::run(dir.path(), &server, &["search", "--text", "bug"]);
    assert!(output.status.success(), "exited {:?}", output.status.code());

    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        envelope.pointer("/outcome").and_then(Value::as_str),
        Some("results")
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("INFO: composed JQL:") && stderr.contains("text ~"),
        "the audit line names the composed JQL: {stderr}"
    );
}

#[test]
fn search_cap_hit_exits_nonzero_with_the_truncated_envelope() {
    // Every page offers another cursor, so the walk cap-hits: the search fails
    // loud with the dedicated code, still emitting the truncated envelope and
    // the resume cursor.
    let server = server_returning(
        r#"{"issues":[{"key":"ENG-1"}],"nextPageToken":"tok-2"}"#,
    );
    let dir = support::scratch(support::CONFIG);

    let output =
        support::run(dir.path(), &server, &["search", "--text", "bug"]);
    assert_eq!(
        output.status.code(),
        Some(79),
        "a cap-hit exits with the dedicated SEARCH_CAP_HIT code"
    );

    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        envelope.pointer("/outcome").and_then(Value::as_str),
        Some("truncated")
    );
    assert_eq!(
        envelope.pointer("/truncated").and_then(Value::as_bool),
        Some(true),
        "the truncated field rides the envelope alongside the non-zero exit"
    );
    assert_eq!(
        envelope.pointer("/nextPageToken").and_then(Value::as_str),
        Some("tok-2"),
        "the resume cursor rides the envelope"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E_SEARCH_CAP_HIT") && stderr.contains("max_pages"),
        "the cap-hit names the max_pages remedy: {stderr}"
    );
}

#[test]
fn a_page_token_and_field_selection_ride_the_request() {
    let server = server_returning(r#"{"issues":[{"key":"ENG-9"}]}"#);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &[
            "search",
            "--text",
            "bug",
            "--page-token",
            "tok-2",
            "--field",
            "summary",
            "--field",
            "status",
        ],
    );
    assert!(output.status.success(), "exited {:?}", output.status.code());

    let sent = String::from_utf8(
        server.last_body(&RequestKey::post(SEARCH)).expect("a body"),
    )
    .unwrap();
    let body: Value = serde_json::from_str(&sent).expect("json body");
    assert_eq!(
        body.pointer("/nextPageToken").and_then(Value::as_str),
        Some("tok-2"),
        "--page-token fetches the next page: {sent}"
    );
    assert_eq!(
        body.pointer("/fields"),
        Some(&serde_json::json!(["summary", "status"])),
        "--field selections pass through: {sent}"
    );
}

#[test]
fn render_adf_renders_a_description_in_the_results() {
    let server = server_returning(
        r#"{"issues":[{"key":"ENG-1","fields":{"description":
           {"type":"doc","version":1,"content":[{"type":"paragraph","content":
           [{"type":"text","text":"hello"}]}]}}}]}"#,
    );
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["search", "--text", "bug", "--render-adf", "-q"],
    );
    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        envelope
            .pointer("/issues/0/fields/description")
            .and_then(Value::as_str),
        Some("hello"),
        "the ADF description in the result renders to Markdown: {envelope}"
    );
}

#[test]
fn quiet_suppresses_the_audit_line() {
    let server = server_returning(r#"{"issues":[]}"#);
    let dir = support::scratch(support::CONFIG);

    let output =
        support::run(dir.path(), &server, &["search", "--text", "bug", "-q"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("composed JQL"),
        "quiet is silent: {stderr}"
    );

    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        envelope.pointer("/outcome").and_then(Value::as_str),
        Some("empty"),
        "an empty result set stamps the empty outcome"
    );
}
