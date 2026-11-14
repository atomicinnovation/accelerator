//! `attach` end to end: https links only, and no file upload yet.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::RequestKey;
use serde_json::json;

const STORY: &str = "/api/v4/scratch-workspace/stories/14";

fn attach(
    scenarios: &[&str],
    args: &[&str],
) -> (std::process::Output, http_test_support::MockHTTPServer) {
    let server = support::serving(scenarios);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    let output = support::run(dir.path(), &server, args);
    (output, server)
}

#[test]
fn an_https_link_is_attached() {
    let (output, server) = attach(
        &["story-link-200"],
        &["attach", "sc-14", "--url", "https://example.com/spec"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout_json(&output)["outcome"], "attached");
    assert_eq!(
        support::sent_json(&server, &RequestKey::new("PATCH", STORY)),
        json!({"external_links_add": ["https://example.com/spec"]})
    );
}

#[test]
fn an_http_link_is_refused_unsent() {
    let (output, server) = attach(
        &["story-link-200"],
        &["attach", "sc-14", "--url", "http://example.com/spec"],
    );

    assert_eq!(output.status.code(), Some(134));
    assert_eq!(server.hits(&RequestKey::new("PATCH", STORY)), 0);
}

#[test]
fn a_file_is_a_usage_error_saying_upload_is_not_supported() {
    let (output, _) = attach(&[], &["attach", "sc-14", "--file", "notes.txt"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(support::stderr(&output).contains("not supported yet"));
}

#[test]
fn an_epic_key_is_a_usage_error() {
    let (output, _) =
        attach(&[], &["attach", "epic-17", "--url", "https://example.com"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(support::stderr(&output).contains("E_NOT_A_STORY"));
}
