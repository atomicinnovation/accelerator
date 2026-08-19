//! Init discovery: each query's document, and the cache shape it produces
//! against a committed golden.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::SurfaceError;
use serde_json::Value;
use support::client::client_for;
use tracker_support::TransportConfig;

const GRAPHQL: &str = "/graphql";

fn json_route(body: &str) -> Route {
    Route::Json {
        status: 200,
        body: body.to_owned(),
    }
}

fn sent_document(server: &MockServer) -> String {
    let body = server
        .last_body(&RequestKey::post(GRAPHQL))
        .expect("a request body");
    let value: Value = serde_json::from_slice(&body).expect("JSON");
    value["query"].as_str().expect("a document").to_owned()
}

fn golden(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).expect("the golden is readable");
    serde_json::from_str(&text).expect("the golden is JSON")
}

#[test]
fn discover_viewer_queries_the_viewer_and_matches_the_golden() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(
            "{\"data\":{\"viewer\":{\"id\":\"user-1\",\
             \"name\":\"Ada Lovelace\"}}}",
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    let shape = client.discover_viewer().expect("discovery succeeds");

    assert!(sent_document(&server).contains("viewer"));
    assert_eq!(shape, golden("viewer.golden.json"));
}

#[test]
fn discover_viewer_without_an_id_is_a_bad_response() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route("{\"data\":{\"viewer\":{\"name\":\"Ada\"}}}"),
    );
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .discover_viewer()
        .expect_err("no viewer id is a failure");

    assert!(matches!(error, SurfaceError::BadResponse { .. }), "{error}");
}

#[test]
fn list_teams_queries_teams_and_returns_the_nodes() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(
            "{\"data\":{\"teams\":{\"nodes\":[\
             {\"id\":\"team-1\",\"name\":\"Engineering\",\"key\":\"ENG\"}]}}}",
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    let teams = client.list_teams().expect("listing succeeds");

    assert!(sent_document(&server).contains("teams"));
    let teams = teams.as_array().expect("an array");
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0]["key"], "ENG");
}

#[test]
fn discover_team_queries_the_states_and_matches_the_catalogue_golden() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        json_route(
            "{\"data\":{\"team\":{\"id\":\"team-1\",\"key\":\"ENG\",\
             \"name\":\"Engineering\",\"states\":{\"nodes\":[\
             {\"id\":\"s1\",\"name\":\"Todo\",\"type\":\"unstarted\",\"position\":0},\
             {\"id\":\"s2\",\"name\":\"In Progress\",\"type\":\"started\",\"position\":1},\
             {\"id\":\"s3\",\"name\":\"Done\",\"type\":\"completed\",\"position\":2}]}}}}",
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    let catalogue = client.discover_team("team-1").expect("discovery succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert!(sent["query"].as_str().expect("a document").contains("team"));
    assert_eq!(sent["variables"]["id"], "team-1");
    assert_eq!(catalogue, golden("catalogue.golden.json"));
}

#[test]
fn a_team_with_no_states_is_a_bad_response() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(
            "{\"data\":{\"team\":{\"id\":\"team-1\",\"key\":\"ENG\",\
             \"name\":\"E\",\"states\":{\"nodes\":[]}}}}",
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .discover_team("team-1")
        .expect_err("a team with no states is refused");

    assert!(matches!(error, SurfaceError::BadResponse { .. }), "{error}");
}
