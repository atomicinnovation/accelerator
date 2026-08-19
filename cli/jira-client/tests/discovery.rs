//! Init discovery: each call's endpoint, and the cache shape it returns.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::SurfaceError;
use serde_json::json;
use support::client::client_for;
use tracker_support::TransportConfig;

fn json_route(body: &str) -> Route {
    Route::Json {
        status: 200,
        body: body.to_owned(),
    }
}

#[test]
fn discover_site_returns_the_site_and_account_id() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/rest/api/3/myself"),
        json_route(r#"{"accountId":"5b10ac","emailAddress":"t@x.io"}"#),
    );
    let client = client_for(&server, TransportConfig::default());

    let shape = client.discover_site().expect("discovery succeeds");

    assert_eq!(server.hits(&RequestKey::get("/rest/api/3/myself")), 1);
    assert_eq!(shape["accountId"], "5b10ac");
    // The loopback base has no .atlassian.net suffix, so the label is the host.
    assert_eq!(shape["site"], "127.0.0.1");
}

#[test]
fn discover_site_without_an_account_id_is_a_bad_response() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/rest/api/3/myself"),
        json_route(r#"{"emailAddress":"t@x.io"}"#),
    );
    let client = client_for(&server, TransportConfig::default());

    let error = client.discover_site().expect_err("no accountId");

    assert!(matches!(error, SurfaceError::BadResponse { .. }));
}

#[test]
fn discover_projects_projects_key_id_name() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/rest/api/3/project"),
        json_route(
            r#"[{"key":"ENG","id":"10000","name":"Engineering","extra":1},
                {"key":"OPS","id":"10001","name":"Operations"}]"#,
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    let shape = client.discover_projects().expect("discovery succeeds");

    let projects = shape["projects"].as_array().expect("an array");
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0]["key"], "ENG");
    assert_eq!(projects[0]["name"], "Engineering");
    // Only key/id/name are projected — the extra field is dropped.
    assert!(projects[0].get("extra").is_none());
}

#[test]
fn discover_fields_slugifies_and_carries_schema() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/rest/api/3/field"),
        json_route(
            r#"[
              {"id":"summary","key":"summary","name":"Summary"},
              {"id":"customfield_1","key":"cf1","name":"Story Points!",
               "schema":{"custom":"com.pyxis:float","type":"number"}}
            ]"#,
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    let shape = client.discover_fields().expect("discovery succeeds");

    let fields = shape["fields"].as_array().expect("an array");
    assert_eq!(fields[0]["slug"], "summary");
    assert!(fields[0].get("schema").is_none());

    assert_eq!(fields[1]["slug"], "story-points");
    assert_eq!(
        fields[1]["schema"],
        json!({ "custom": "com.pyxis:float", "type": "number" })
    );
}

#[test]
fn a_non_success_status_surfaces_as_a_status_error() {
    let server = MockServer::start();
    server.route(RequestKey::get("/rest/api/3/myself"), Route::Status(401));
    let client = client_for(&server, TransportConfig::default());

    let error = client.discover_site().expect_err("a 401 is an error");

    assert!(matches!(error, SurfaceError::Status { status: 401, .. }));
}
