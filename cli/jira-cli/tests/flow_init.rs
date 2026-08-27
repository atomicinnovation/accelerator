//! The `init verify` flow: it verifies credentials against `/myself`, caches the
//! site identity, stamps the `verified` outcome, and — the load-bearing
//! guarantee — never prints the token on any exit path. The
//! `Secret` redaction in `tracker_support` is why the guarantee holds.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};
use serde_json::Value;
use support::{Token, TOKEN_SENTINEL};

const MYSELF: &str = "/rest/api/3/myself";

fn no_leak(output: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains(TOKEN_SENTINEL), "token leaked to stdout");
    assert!(!stderr.contains(TOKEN_SENTINEL), "token leaked to stderr");
}

#[test]
fn verify_caches_the_site_and_stamps_the_outcome() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(MYSELF),
        Route::Json {
            status: 200,
            body: r#"{"accountId":"acc-1","displayName":"Ada"}"#.to_owned(),
        },
    );
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "verify"]);
    assert!(
        output.status.success(),
        "verify exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    no_leak(&output);

    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        envelope.pointer("/outcome").and_then(Value::as_str),
        Some("verified")
    );
    assert_eq!(
        envelope.pointer("/accountId").and_then(Value::as_str),
        Some("acc-1")
    );

    let cache = dir
        .path()
        .join(".accelerator/state/integrations/jira/site.json");
    assert!(cache.exists(), "the site cache is written");
}

#[test]
fn a_verify_failure_never_leaks_the_token() {
    let server = MockServer::start();
    server.route(RequestKey::get(MYSELF), Route::Status(401));
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "verify"]);
    assert_eq!(output.status.code(), Some(61), "verify-failed maps to 61");
    no_leak(&output);
}

#[test]
fn a_missing_token_never_leaks_and_maps_to_no_token() {
    let server = MockServer::start();
    server.route(RequestKey::get(MYSELF), Route::Status(200));
    let dir = support::scratch(support::CONFIG);

    let output = support::run_with(
        dir.path(),
        &["init", "verify"],
        Some(&server.base_url()),
        &Token::Absent,
    );
    assert_eq!(output.status.code(), Some(24), "no token → NO_TOKEN");
    no_leak(&output);
}
