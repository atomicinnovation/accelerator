//! Transitions: state-name resolution over the base team's catalogued states,
//! and the `issueUpdate` mutation that carries the resolved `stateId`.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::collections::BTreeMap;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::catalogue::{Catalogue, TeamEntries};
use linear_client::resolution::{
    FixedNames, NameResolver, NameUnresolved, ResolverSet, SingleResolution,
};
use linear_client::SurfaceError;
use serde_json::{json, Value};
use support::client::{client_with_resolvers, TEAM_ID, TEAM_KEY};

const GRAPHQL: &str = "/graphql";

fn json_route(body: &str) -> Route {
    Route::Json {
        status: 200,
        body: body.to_owned(),
    }
}

fn updated_route() -> Route {
    json_route(
        "{\"data\":{\"issueUpdate\":{\"success\":true,\
         \"issue\":{\"id\":\"u\",\"identifier\":\"ENG-1\"}}}}",
    )
}

fn with_team_states(team_states: Box<dyn NameResolver>) -> ResolverSet {
    ResolverSet::new(team_states, TeamEntries::keyed(&[(TEAM_KEY, TEAM_ID)]))
}

fn fixed(pairs: &[(&str, &str)]) -> ResolverSet {
    let mut map = BTreeMap::new();
    for (name, id) in pairs {
        map.insert((*name).to_owned(), (*id).to_owned());
    }
    with_team_states(Box::new(FixedNames(map)))
}

fn catalogued(catalogue: &Value) -> ResolverSet {
    Catalogue::from_text(&catalogue.to_string()).resolver_set()
}

fn state(id: &str, name: &str, archived_at: Option<&str>) -> Value {
    json!({ "id": id, "name": name, "type": "started", "position": 1,
            "archivedAt": archived_at })
}

/// A resolver whose display name matches two base-team states, which only the
/// catalogue can produce — `FixedNames` has unique keys.
struct AmbiguousStates;

impl NameResolver for AmbiguousStates {
    fn resolve(&self, _value: &str) -> SingleResolution {
        SingleResolution::Unresolved(NameUnresolved::Ambiguous { count: 2 })
    }
}

fn sent_state_id(server: &MockServer, key: &RequestKey) -> Value {
    let sent: Value =
        serde_json::from_slice(&server.last_body(key).expect("a body"))
            .expect("JSON");
    assert!(sent["query"]
        .as_str()
        .expect("a document")
        .contains("issueUpdate"));
    assert_eq!(sent["variables"]["id"], "ENG-1");
    sent["variables"]["input"]["stateId"].clone()
}

#[test]
fn a_known_state_resolves_to_its_uuid_and_the_mutation_carries_it() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), updated_route());
    let client = client_with_resolvers(
        &server,
        catalogued(&json!({
            "team": { "id": TEAM_ID, "key": TEAM_KEY, "name": "Engineering" },
            "workflowStates": [
                { "id": "st-42", "name": "In Progress", "type": "started",
                  "position": 1 }
            ]
        })),
    );

    client
        .transition("ENG-1", "in progress")
        .expect("the transition succeeds");

    assert_eq!(
        sent_state_id(&server, &key),
        "st-42",
        "matching is case-insensitive and resolves to the catalogue UUID"
    );
}

#[test]
fn a_state_absent_from_the_team_refuses_as_not_in_catalogue() {
    let cases = [
        ("not found", fixed(&[("Done", "st-9")])),
        ("not catalogued", catalogued(&json!({}))),
    ];

    for (case, resolvers) in cases {
        let server = MockServer::start();
        let key = RequestKey::post(GRAPHQL);
        server.route(key.clone(), Route::Status(200));
        let client = client_with_resolvers(&server, resolvers);

        let error = client
            .transition("ENG-1", "Nonexistent")
            .expect_err("an unresolvable state is refused");

        assert!(
            matches!(error, SurfaceError::UnknownState { .. }),
            "{case}: {error}"
        );
        assert_eq!(server.hits(&key), 0, "{case}: no request is made");
    }
}

#[test]
fn an_ambiguous_state_refuses_naming_the_count() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(200));
    let client = client_with_resolvers(
        &server,
        with_team_states(Box::new(AmbiguousStates)),
    );

    let error = client
        .transition("ENG-1", "Review")
        .expect_err("a shared display name cannot be disambiguated");

    assert!(
        matches!(error, SurfaceError::AmbiguousState { count: 2, .. }),
        "{error}"
    );
    assert_eq!(server.hits(&key), 0);
}

fn two_teams() -> Value {
    json!({
        "baseTeam": TEAM_ID,
        "teams": [
            { "id": TEAM_ID, "key": TEAM_KEY, "name": "Engineering",
              "states": [state("st-eng-ip", "In Progress", None),
                         state("st-eng-old", "Shelved",
                               Some("2026-01-01T00:00:00Z"))] },
            { "id": "ops-uuid", "key": "OPS", "name": "Operations",
              "states": [state("st-ops-ip", "In Progress", None),
                         state("st-ops-blocked", "Blocked", None)] }
        ]
    })
}

#[test]
fn transition_resolves_against_the_base_team_only() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), updated_route());
    let client = client_with_resolvers(&server, catalogued(&two_teams()));

    let error = client
        .transition("ENG-1", "Blocked")
        .expect_err("another team's state is not a target");
    assert!(
        matches!(error, SurfaceError::UnknownState { .. }),
        "{error}"
    );

    client
        .transition("ENG-1", "In Progress")
        .expect("the base team's state resolves");
    assert_eq!(sent_state_id(&server, &key), "st-eng-ip");
}

#[test]
fn an_archived_state_is_never_a_transition_target() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(200));
    let client = client_with_resolvers(&server, catalogued(&two_teams()));

    let error = client
        .transition("ENG-1", "Shelved")
        .expect_err("an archived state is refused");

    assert!(
        matches!(error, SurfaceError::UnknownState { .. }),
        "{error}"
    );
    assert_eq!(server.hits(&key), 0);
}
