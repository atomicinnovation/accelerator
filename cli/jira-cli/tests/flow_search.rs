//! The `search` flow: the verbatim Jira envelope (`issues` + `nextPageToken`)
//! with an `outcome` stamp, the composed-JQL audit line on stderr, and its
//! `--quiet` suppression.

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
fn search_echoes_the_envelope_and_audits_the_jql() {
    let server = server_returning(
        r#"{"issues":[{"key":"ENG-1"}],"nextPageToken":"tok-2"}"#,
    );
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
    assert_eq!(
        envelope.pointer("/nextPageToken").and_then(Value::as_str),
        Some("tok-2"),
        "the Jira envelope passes through verbatim"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("INFO: composed JQL:") && stderr.contains("text ~"),
        "the audit line names the composed JQL: {stderr}"
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
