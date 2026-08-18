//! Transition resolution and application: case-insensitive name matching, the
//! zero- and ambiguous-match typed errors, and the POST body shape.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::transition::Target;
use jira_client::SurfaceError;
use serde_json::Value;
use support::client::client_for;
use tracker_support::TransportConfig;

const KEY: &str = "ENG-1";
const TRANSITIONS: &str = "/rest/api/3/issue/ENG-1/transitions";

fn lookup(body: &str) -> Route {
    Route::Json {
        status: 200,
        body: body.to_owned(),
    }
}

fn two_transitions() -> Route {
    lookup(
        r#"{"transitions":[
            {"id":"21","to":{"name":"In Progress"}},
            {"id":"31","to":{"name":"Done"}}
        ]}"#,
    )
}

fn post_body(server: &MockServer) -> Value {
    let bytes = server
        .last_body(&RequestKey::post(TRANSITIONS))
        .expect("a recorded body");
    serde_json::from_slice(&bytes).expect("the body is JSON")
}

#[test]
fn a_state_name_resolves_case_insensitively() {
    let server = MockServer::start();
    server.route(RequestKey::get(TRANSITIONS), two_transitions());
    server.route(RequestKey::post(TRANSITIONS), Route::Status(204));
    let client = client_for(&server, TransportConfig::default());

    client
        .transition(
            KEY,
            &Target::State("in progress".to_owned()),
            None,
            None,
            true,
        )
        .expect("the transition applies");

    assert_eq!(post_body(&server)["transition"]["id"], "21");
}

#[test]
fn a_known_transition_id_skips_the_lookup() {
    let server = MockServer::start();
    server.route(RequestKey::post(TRANSITIONS), Route::Status(204));
    let client = client_for(&server, TransportConfig::default());

    client
        .transition(KEY, &Target::Id("21".to_owned()), None, None, true)
        .expect("the transition applies");

    assert_eq!(server.hits(&RequestKey::get(TRANSITIONS)), 0);
    assert_eq!(post_body(&server)["transition"]["id"], "21");
}

#[test]
fn no_matching_state_is_not_found() {
    let server = MockServer::start();
    server.route(RequestKey::get(TRANSITIONS), two_transitions());
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .transition(KEY, &Target::State("Nope".to_owned()), None, None, true)
        .expect_err("no match");

    assert!(matches!(error, SurfaceError::TransitionNotFound { .. }));
    assert_eq!(server.hits(&RequestKey::post(TRANSITIONS)), 0);
}

#[test]
fn multiple_matches_are_ambiguous_and_carry_the_candidates() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(TRANSITIONS),
        lookup(
            r#"{"transitions":[
                {"id":"21","to":{"name":"Done"}},
                {"id":"41","to":{"name":"done"}}
            ]}"#,
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .transition(KEY, &Target::State("Done".to_owned()), None, None, true)
        .expect_err("ambiguous");

    match error {
        SurfaceError::TransitionAmbiguous { matches, .. } => {
            assert_eq!(matches.len(), 2);
        }
        other => panic!("expected ambiguous, got {other:?}"),
    }
    assert_eq!(server.hits(&RequestKey::post(TRANSITIONS)), 0);
}

#[test]
fn a_comment_folds_into_update_comment_add_as_adf() {
    let server = MockServer::start();
    server.route(RequestKey::post(TRANSITIONS), Route::Status(204));
    let client = client_for(&server, TransportConfig::default());

    client
        .transition(
            KEY,
            &Target::Id("21".to_owned()),
            None,
            Some("moving on"),
            true,
        )
        .expect("the transition applies");

    let body = post_body(&server);
    assert_eq!(body["update"]["comment"][0]["add"]["body"]["type"], "doc");
}

#[test]
fn a_resolution_sets_the_fields_resolution() {
    let server = MockServer::start();
    server.route(RequestKey::post(TRANSITIONS), Route::Status(204));
    let client = client_for(&server, TransportConfig::default());

    client
        .transition(
            KEY,
            &Target::Id("31".to_owned()),
            Some("Fixed"),
            None,
            false,
        )
        .expect("the transition applies");

    let key = RequestKey::post(TRANSITIONS);
    assert_eq!(post_body(&server)["fields"]["resolution"]["name"], "Fixed");
    assert_eq!(
        server.last_query(&key).as_deref(),
        Some("notifyUsers=false")
    );
}
