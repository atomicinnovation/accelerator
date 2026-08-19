//! Transitions: state-name resolution through the catalogue, and the
//! `issueUpdate` mutation that carries the resolved `stateId`.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::collections::BTreeMap;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::filter::{FixedStates, StateResolver};
use linear_client::{CatalogueStates, SurfaceError};
use serde_json::{json, Value};
use support::client::client_with_states;

const GRAPHQL: &str = "/graphql";

fn json_route(body: &str) -> Route {
    Route::Json {
        status: 200,
        body: body.to_owned(),
    }
}

fn states(pairs: &[(&str, &str)]) -> Box<dyn StateResolver> {
    let mut map = BTreeMap::new();
    for (name, id) in pairs {
        map.insert((*name).to_owned(), (*id).to_owned());
    }
    Box::new(FixedStates(map))
}

/// A resolver whose display name matches two catalogue states, which only the
/// catalogue can produce — `FixedStates` has unique keys.
struct AmbiguousStates;

impl StateResolver for AmbiguousStates {
    fn resolve(&self, _name: &str) -> Option<String> {
        None
    }

    fn resolve_all(&self, _name: &str) -> Vec<String> {
        vec!["uuid-a".to_owned(), "uuid-b".to_owned()]
    }
}

#[test]
fn a_known_state_resolves_to_its_uuid_and_the_mutation_carries_it() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        json_route(
            "{\"data\":{\"issueUpdate\":{\"success\":true,\
             \"issue\":{\"id\":\"u\",\"identifier\":\"ENG-1\"}}}}",
        ),
    );
    let catalogue = json!({
        "workflowStates": [{ "id": "st-42", "name": "In Progress" }]
    });
    let client = client_with_states(
        &server,
        Box::new(CatalogueStates::from_catalogue(&catalogue)),
    );

    client
        .transition("ENG-1", "in progress")
        .expect("the transition succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert!(sent["query"]
        .as_str()
        .expect("a document")
        .contains("issueUpdate"));
    assert_eq!(sent["variables"]["id"], "ENG-1");
    assert_eq!(
        sent["variables"]["input"]["stateId"], "st-42",
        "matching is case-insensitive and resolves to the catalogue UUID"
    );
}

#[test]
fn an_unknown_state_is_a_typed_error_before_any_request() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(200));
    let client = client_with_states(&server, states(&[("Done", "st-9")]));

    let error = client
        .transition("ENG-1", "Nonexistent")
        .expect_err("an unknown state is refused");

    assert!(
        matches!(error, SurfaceError::UnknownState { .. }),
        "{error}"
    );
    assert_eq!(server.hits(&key), 0, "no request is made");
}

#[test]
fn an_ambiguous_state_is_a_distinct_typed_error() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(200));
    let client = client_with_states(&server, Box::new(AmbiguousStates));

    let error = client
        .transition("ENG-1", "Review")
        .expect_err("a shared display name cannot be disambiguated");

    assert!(
        matches!(error, SurfaceError::AmbiguousState { count: 2, .. }),
        "{error}"
    );
    assert_eq!(server.hits(&key), 0);
}
