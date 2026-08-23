//! The read-side projections the `search` and `show` subcommands render
//! (Decision 20): the verbatim Jira envelope a `search` echoes, the composed
//! JQL its audit line prints, and the raw issue a `show` renders ADF over. The
//! port `search`/`show` reshape to the sync contract; these keep Jira's own
//! wire shape the bash flows emitted.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::jql::Search;
use serde_json::Value;
use support::client::{brief, client_for, PROJECT};

const SEARCH: &str = "/rest/api/3/search/jql";
const ISSUE: &str = "/rest/api/3/issue";

fn search_over(project: &str) -> Search {
    Search {
        project: Some(project.to_owned()),
        text: vec!["bug".to_owned()],
        ..Search::default()
    }
}

#[test]
fn search_detailed_returns_the_verbatim_envelope_and_posts_the_body() {
    let server = MockServer::start();
    let envelope = r#"{"issues":[{"key":"ENG-1"}],"nextPageToken":"tok-2"}"#;
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: envelope.to_owned(),
        },
    );

    let client = client_for(&server, brief());
    let response = client
        .search_detailed(&search_over(PROJECT), &[], 50, None)
        .expect("search runs");

    // The binary emits the envelope verbatim, so the parsed value round-trips.
    let expected: Value = serde_json::from_str(envelope).expect("json");
    assert_eq!(response, expected, "the Jira envelope passes through");

    let sent = String::from_utf8(
        server.last_body(&RequestKey::post(SEARCH)).expect("a body"),
    )
    .expect("utf8");
    let body: Value = serde_json::from_str(&sent).expect("json body");
    assert_eq!(body.pointer("/fieldsByKeys"), Some(&Value::Bool(false)));
    assert_eq!(body.pointer("/maxResults"), Some(&Value::from(50)));
    assert_eq!(body.pointer("/fields"), Some(&Value::Array(vec![])));
    assert!(
        body.pointer("/jql")
            .and_then(Value::as_str)
            .is_some_and(|jql| jql.contains("project = 'ENG'")),
        "the composed JQL rides in the body: {sent}"
    );
    assert!(
        body.get("nextPageToken").is_none(),
        "no page token by default"
    );
}

#[test]
fn a_page_token_adds_the_cursor_to_the_body() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: "{}".to_owned(),
        },
    );

    let client = client_for(&server, brief());
    client
        .search_detailed(&search_over(PROJECT), &[], 25, Some("tok-2"))
        .expect("search runs");

    let sent = String::from_utf8(
        server.last_body(&RequestKey::post(SEARCH)).expect("a body"),
    )
    .expect("utf8");
    let body: Value = serde_json::from_str(&sent).expect("json body");
    assert_eq!(
        body.pointer("/nextPageToken").and_then(Value::as_str),
        Some("tok-2")
    );
    assert_eq!(body.pointer("/maxResults"), Some(&Value::from(25)));
}

#[test]
fn compose_search_jql_returns_the_audit_string() {
    let server = MockServer::start();
    let client = client_for(&server, brief());
    let jql = client
        .compose_search_jql(&search_over(PROJECT))
        .expect("composes");
    assert!(jql.contains("project = 'ENG'"), "jql: {jql}");
    assert!(jql.contains("text ~"), "the text clause is present: {jql}");
}

#[test]
fn an_uncomposable_search_is_refused_before_the_wire() {
    let server = MockServer::start();
    let client = client_for(&server, brief());
    let neither = Search::default();

    let error = client
        .search_detailed(&neither, &[], 50, None)
        .expect_err("no project");
    assert!(error.to_string().contains("E_JQL_NO_PROJECT"), "{error}");
    assert_eq!(server.hits(&RequestKey::post(SEARCH)), 0, "nothing sent");
}

#[test]
fn show_detailed_returns_the_raw_issue_and_carries_the_queries() {
    let server = MockServer::start();
    let issue = r#"{"key":"ENG-42","fields":{"summary":"a bug"}}"#;
    server.route(
        RequestKey::get(&format!("{ISSUE}/ENG-42")),
        Route::Json {
            status: 200,
            body: issue.to_owned(),
        },
    );

    let client = client_for(&server, brief());
    let response = client
        .show_detailed("ENG-42", "*all", "names,schema,transitions")
        .expect("show runs");

    let expected: Value = serde_json::from_str(issue).expect("json");
    assert_eq!(response, expected, "the raw issue passes through");

    let query = server
        .last_query(&RequestKey::get(&format!("{ISSUE}/ENG-42")))
        .expect("a query");
    assert!(
        query.contains("fields=*all"),
        "fields ride the query: {query}"
    );
    assert!(query.contains("expand="), "expand rides the query: {query}");
}

#[test]
fn show_detailed_surfaces_a_status_error() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(&format!("{ISSUE}/ENG-9")),
        Route::Status(404),
    );

    let client = client_for(&server, brief());
    let error = client
        .show_detailed("ENG-9", "*all", "names")
        .expect_err("404");
    assert!(
        matches!(error, jira_client::SurfaceError::Status { status: 404, .. }),
        "expected a 404 status surface error, got {error:?}"
    );
}
