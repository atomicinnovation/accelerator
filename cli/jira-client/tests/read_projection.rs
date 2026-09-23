//! The read-side projections the `search` and `show` subcommands render:
//! the merged `issues` envelope a `search` builds from its internally-paginated
//! pages, the composed JQL its audit line prints, and the raw issue a `show`
//! renders ADF over. The port `search`/`show` reshape to the sync contract;
//! these keep Jira's own wire shape the established flows emitted.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::jql::Search;
use serde_json::Value;
use support::client::{brief, client_for, PROJECT};
use tracker::Completeness;

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
fn search_detailed_merges_a_complete_walk_and_posts_the_body() {
    let server = MockServer::start();
    // A page with no cursor completes the walk in one request.
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: r#"{"issues":[{"key":"ENG-1"}]}"#.to_owned(),
        },
    );

    let client = client_for(&server, brief());
    let response = client
        .search_detailed(&search_over(PROJECT), &[], 50, None)
        .expect("search runs");

    assert_eq!(response.completeness, Completeness::Complete);
    assert_eq!(
        response.envelope.pointer("/issues"),
        Some(&serde_json::json!([{"key": "ENG-1"}])),
        "the merged envelope carries the page's issues"
    );
    assert!(
        response.envelope.get("truncated").is_none(),
        "a complete walk is not marked truncated"
    );

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
fn search_detailed_paginates_to_the_cap_and_reports_a_cap_hit() {
    let server = MockServer::start();
    // Every page offers another cursor, so only the discovery cap ends the
    // walk: the merged envelope is marked truncated and carries the resume
    // cursor.
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: r#"{"issues":[{"key":"ENG-1"}],"nextPageToken":"more"}"#
                .to_owned(),
        },
    );

    let client = client_for(&server, brief());
    let response = client
        .search_detailed(&search_over(PROJECT), &[], 50, None)
        .expect("a cap-hit is a successful, truncated result");

    assert_eq!(response.completeness, Completeness::CapHit);
    assert_eq!(
        server.hits(&RequestKey::post(SEARCH)),
        50,
        "the walk paginates internally to the default 50-page cap"
    );
    assert_eq!(
        response.envelope.pointer("/truncated"),
        Some(&Value::Bool(true)),
        "a cap-hit envelope is marked truncated"
    );
    assert_eq!(
        response
            .envelope
            .pointer("/nextPageToken")
            .and_then(Value::as_str),
        Some("more"),
        "the resume cursor rides the envelope for --page-token"
    );
    let merged = response
        .envelope
        .pointer("/issues")
        .and_then(Value::as_array)
        .expect("issues array");
    assert_eq!(merged.len(), 50, "every page's issue is merged in");
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
