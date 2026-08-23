//! Byte-exact stdout goldens for the strict contracts. The mock responses are
//! deterministic and `serde_json` orders keys stably, so the whole rendered
//! document — including the `outcome` discriminant and the Markdown an ADF field
//! renders to — is pinned as raw bytes, catching a shape change a field-wise
//! assertion would miss. Run with `UPDATE_GOLDEN=1` to accept an intended
//! change.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};

#[test]
fn search_stdout_matches_the_golden() {
    let server = MockServer::start();
    // The widened read-side projection (Decision 20): each issue's full `fields`
    // map — Summary/Status/Assignee — plus the `nextPageToken` cursor, emitted
    // verbatim with the outcome stamp.
    server.route(
        RequestKey::post("/rest/api/3/search/jql"),
        Route::Json {
            status: 200,
            body: r#"{"issues":[{"key":"ENG-1","fields":{"summary":"a bug",
                "status":{"name":"In Progress"},"assignee":{"displayName":"Ada"}}}],
                "nextPageToken":"tok-2"}"#
                .to_owned(),
        },
    );
    let dir = support::scratch(support::CONFIG);
    let output =
        support::run(dir.path(), &server, &["search", "--text", "bug", "-q"]);
    assert!(output.status.success(), "exited {:?}", output.status.code());
    support::assert_golden("search.golden", &output.stdout);
}

#[test]
fn show_stdout_matches_the_golden() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/rest/api/3/issue/ENG-42"),
        Route::Json {
            status: 200,
            body: r#"{"key":"ENG-42","fields":{"summary":"a bug","description":
                {"type":"doc","version":1,"content":[{"type":"paragraph",
                "content":[{"type":"text","text":"hello"}]}]}}}"#
                .to_owned(),
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
    support::assert_golden("show.golden", &output.stdout);
}
