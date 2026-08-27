//! The `show` flow through the test-loopback seam: the raw issue fetched with
//! the field/expand queries, ADF fields rendered to Markdown by default, the
//! comment slice, and the `outcome` discriminant.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};
use serde_json::Value;

const ISSUE: &str = "/rest/api/3/issue/ENG-42";

fn adf_issue() -> String {
    // A minimal issue whose description is an ADF document.
    r#"{"key":"ENG-42","fields":{"summary":"a bug","description":
       {"type":"doc","version":1,"content":[{"type":"paragraph","content":
       [{"type":"text","text":"hello"}]}]}}}"#
        .to_owned()
}

#[test]
fn show_renders_adf_and_stamps_the_outcome() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(ISSUE),
        Route::Json {
            status: 200,
            body: adf_issue(),
        },
    );
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["show", "ENG-42"]);
    assert!(
        output.status.success(),
        "show exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let rendered: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        rendered.pointer("/outcome").and_then(Value::as_str),
        Some("found")
    );
    assert_eq!(
        rendered
            .pointer("/fields/description")
            .and_then(Value::as_str),
        Some("hello"),
        "the ADF description renders to Markdown: {rendered}"
    );

    let query = server.last_query(&RequestKey::get(ISSUE)).expect("a query");
    assert!(query.contains("fields=*all"), "query: {query}");
    assert!(query.contains("expand="), "query: {query}");
}

#[test]
fn no_render_adf_leaves_the_document_intact() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(ISSUE),
        Route::Json {
            status: 200,
            body: adf_issue(),
        },
    );
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["show", "ENG-42", "--no-render-adf"],
    );
    let rendered: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert!(
        rendered.pointer("/fields/description/type").is_some(),
        "the raw ADF document is preserved: {rendered}"
    );
}
