//! The state a transition targets: its name resolves over the base team's
//! catalogued states, and an update carries the resolved `stateId`.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::collections::BTreeMap;

use http_test_support::{MockHTTPServer, RequestKey, Route};
use linear_client::catalogue::{Catalogue, TeamEntries};
use linear_client::resolution::{
    FixedNames, NameResolver, NameUnresolved, ResolverSet, SingleResolution,
};
use linear_client::{LinearClient, SurfaceError, UpdateFields};
use serde_json::{json, Value};
use support::client::{client_with_resolvers, TEAM_ID, TEAM_KEY};
use tracker::ExternalId;

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

fn refusal(client: &LinearClient, state: &str) -> SurfaceError {
    client
        .resolve_state(state)
        .expect_err("the state name is refused")
}

fn state_id_an_update_carries(
    server: &MockHTTPServer,
    client: &LinearClient,
    state: &str,
) -> Value {
    let key = RequestKey::post(GRAPHQL);
    let state_id = client.resolve_state(state).expect("the state resolves");
    client
        .update_op(
            &ExternalId::new("ENG-1".to_owned()),
            &UpdateFields {
                state_id: Some(&state_id),
                ..UpdateFields::default()
            },
        )
        .expect("the update succeeds");
    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    sent["variables"]["input"]["stateId"].clone()
}

#[test]
fn a_known_state_resolves_to_its_uuid_and_an_update_carries_it() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::post(GRAPHQL), updated_route());
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

    assert_eq!(
        state_id_an_update_carries(&server, &client, "in progress"),
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
        let server = MockHTTPServer::start();
        let key = RequestKey::post(GRAPHQL);
        server.route(key.clone(), Route::Status(200));
        let client = client_with_resolvers(&server, resolvers);

        let error = refusal(&client, "Nonexistent");

        assert!(
            matches!(error, SurfaceError::UnknownState { .. }),
            "{case}: {error}"
        );
        assert_eq!(server.hits(&key), 0, "{case}: no request is made");
    }
}

#[test]
fn an_ambiguous_state_refuses_naming_the_count() {
    let server = MockHTTPServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(200));
    let client = client_with_resolvers(
        &server,
        with_team_states(Box::new(AmbiguousStates)),
    );

    let error = refusal(&client, "Review");

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
fn a_state_resolves_against_the_base_team_only() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::post(GRAPHQL), updated_route());
    let client = client_with_resolvers(&server, catalogued(&two_teams()));

    let error = refusal(&client, "Blocked");
    assert!(
        matches!(error, SurfaceError::UnknownState { .. }),
        "{error}"
    );

    assert_eq!(
        state_id_an_update_carries(&server, &client, "In Progress"),
        "st-eng-ip"
    );
}

#[test]
fn an_archived_state_is_never_a_transition_target() {
    let server = MockHTTPServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(200));
    let client = client_with_resolvers(&server, catalogued(&two_teams()));

    let error = refusal(&client, "Shelved");

    assert!(
        matches!(error, SurfaceError::UnknownState { .. }),
        "{error}"
    );
    assert_eq!(server.hits(&key), 0);
}
