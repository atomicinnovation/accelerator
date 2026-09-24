//! `resolve_scope`: the pre-flight step that substitutes a base-only scope's
//! team key for its UUID, validates every filter, and refuses a value the
//! covering base entry lacks before any request.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::MockServer;
use linear_client::resolution::ResolverSet;
use serde_json::json;
use support::catalogue::{
    complete_entry, label, member, project, resolvers, state,
};
use support::client::{
    brief, client_for, client_with_resolvers, TEAM_ID, TEAM_KEY,
};
use tracker::EntityScope;
use tracker::RemoteTracker;
use tracker::SearchScope;

fn keyed(project: Option<&str>) -> SearchScope {
    SearchScope {
        entities: tracker::EntityScope::Keyed {
            base: project.map(str::to_owned),
            additional: Vec::new(),
        },
        filters: Vec::new(),
    }
}

fn resolved_base(scope: &SearchScope) -> Option<String> {
    match &scope.entities {
        tracker::EntityScope::Keyed { base, .. } => base.clone(),
        tracker::EntityScope::WholeWorkspace => None,
    }
}

#[test]
fn a_matching_team_key_resolves_to_the_uuid() {
    let server = MockServer::start();
    let client = client_for(&server, brief());

    let resolved = client
        .resolve_scope(&keyed(Some(TEAM_KEY)))
        .expect("a known team key resolves");

    assert_eq!(
        resolved_base(&resolved).as_deref(),
        Some(TEAM_ID),
        "the resolved scope carries the UUID, not the raw key"
    );
}

#[test]
fn an_unknown_team_key_is_refused_naming_it() {
    let server = MockServer::start();
    let client = client_for(&server, brief());

    let error = client
        .resolve_scope(&keyed(Some("ZZ")))
        .expect_err("a key matching no team is refused");

    assert!(
        error.detail.contains("E_SEARCH_UNKNOWN_TEAM"),
        "{}",
        error.detail
    );
    assert!(error.detail.contains("ZZ"), "{}", error.detail);
}

#[test]
fn a_missing_team_key_is_refused() {
    let server = MockServer::start();
    let client = client_for(&server, brief());

    let error = client
        .resolve_scope(&keyed(None))
        .expect_err("an unkeyed scope is refused");

    assert!(
        error.detail.contains("E_SEARCH_NO_TEAM"),
        "{}",
        error.detail
    );
}

const OPS_ID: &str = "ops-uuid";

fn pairs(entries: &[(&str, &str)]) -> Vec<(String, String)> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn complete_catalogue() -> ResolverSet {
    resolvers(&json!({
        "baseTeam": TEAM_ID,
        "labels": [],
        "teams": [
            complete_entry(TEAM_ID, TEAM_KEY, &json!({
                "states": [state("s-todo", "Todo")],
                "labels": [label("l-bug", "Bug")],
                "members": [member("u-ann", "Ann", "ann@x.io")],
                "projects": [project("p-alpha", "Alpha")]
            })),
            complete_entry(OPS_ID, "OPS", &json!({
                "states": [state("s-ops-triage", "Triage")]
            }))
        ]
    }))
}

fn scope(entities: EntityScope, filters: &[(&str, &str)]) -> SearchScope {
    SearchScope {
        entities,
        filters: pairs(filters),
    }
}

fn base_only(key: &str) -> EntityScope {
    EntityScope::Keyed {
        base: Some(key.to_owned()),
        additional: Vec::new(),
    }
}

#[test]
fn a_base_only_scope_resolves_its_team_and_filters() {
    let server = MockServer::start();
    let client = client_with_resolvers(&server, complete_catalogue());

    let resolved = client
        .resolve_scope(&scope(base_only(TEAM_KEY), &[("state", "Todo")]))
        .expect("a known value resolves");

    assert_eq!(resolved_base(&resolved).as_deref(), Some(TEAM_ID));
    assert_eq!(resolved.filters, pairs(&[("validated:state", "Todo")]));
}

#[test]
fn a_base_plus_additional_scope_leaves_entities_as_keys() {
    let server = MockServer::start();
    let client = client_with_resolvers(&server, complete_catalogue());
    let entities = EntityScope::Keyed {
        base: Some(TEAM_KEY.to_owned()),
        additional: vec!["OPS".to_owned()],
    };

    let resolved = client
        .resolve_scope(&scope(entities.clone(), &[("label", "Bug")]))
        .expect("a broadened scope passes");

    assert_eq!(resolved.entities, entities, "enumeration resolves the keys");
    assert_eq!(resolved.filters, pairs(&[("validated:label", "Bug")]));
}

#[test]
fn an_additional_only_scope_needs_no_base() {
    let server = MockServer::start();
    let client = client_with_resolvers(&server, complete_catalogue());
    let entities = EntityScope::Keyed {
        base: None,
        additional: vec!["OPS".to_owned()],
    };

    let resolved = client
        .resolve_scope(&scope(entities.clone(), &[]))
        .expect("an additional-only scope passes");

    assert_eq!(resolved.entities, entities);
}

#[test]
fn a_whole_workspace_scope_carries_validated_names() {
    let server = MockServer::start();
    let client = client_with_resolvers(&server, complete_catalogue());

    let resolved = client
        .resolve_scope(&scope(
            EntityScope::WholeWorkspace,
            &[("assignee", "someone"), ("text", "needle")],
        ))
        .expect("a whole-workspace scope passes");

    assert_eq!(resolved.entities, EntityScope::WholeWorkspace);
    assert_eq!(
        resolved.filters,
        pairs(&[("validated:assignee", "someone"), ("text", "needle")])
    );
}

#[test]
fn a_value_the_covering_base_entry_lacks_refuses_before_any_request() {
    for (family, code) in [
        ("state", "E_SEARCH_UNKNOWN_STATE"),
        ("label", "E_SEARCH_UNKNOWN_LABEL"),
        ("assignee", "E_SEARCH_UNKNOWN_ASSIGNEE"),
        ("project", "E_SEARCH_UNKNOWN_PROJECT"),
    ] {
        let server = MockServer::start();
        let client = client_with_resolvers(&server, complete_catalogue());

        let error = client
            .resolve_scope(&scope(base_only(TEAM_KEY), &[(family, "Nope")]))
            .expect_err("the covering entry lacks the value");

        assert!(
            error
                .detail
                .starts_with("pull filters could not be resolved:"),
            "{}",
            error.detail
        );
        assert!(error.detail.contains(code), "{}", error.detail);
        assert!(
            error.detail.contains(&format!("--team-id {TEAM_ID}")),
            "{}",
            error.detail
        );
        assert!(server.unmatched().is_empty());
    }
}

#[test]
fn a_base_entry_that_does_not_cover_the_families_defers_to_completion() {
    let server = MockServer::start();
    let client = client_with_resolvers(
        &server,
        resolvers(&json!({
            "team": { "id": TEAM_ID, "key": TEAM_KEY, "name": "Engineering" },
            "workflowStates": []
        })),
    );

    let resolved = client
        .resolve_scope(&scope(base_only(TEAM_KEY), &[("label", "Anything")]))
        .expect("an uncovered family is carried forward");

    assert_eq!(resolved.filters, pairs(&[("validated:label", "Anything")]));
}

#[test]
fn pre_flight_resolves_against_the_team_the_key_names() {
    let server = MockServer::start();
    let client = client_with_resolvers(&server, complete_catalogue());

    client
        .resolve_scope(&scope(base_only("OPS"), &[("state", "Triage")]))
        .expect("OPS carries Triage");
    let error = client
        .resolve_scope(&scope(base_only("OPS"), &[("state", "Todo")]))
        .expect_err("Todo is the base team's, not OPS's");

    assert!(
        error.detail.contains("E_SEARCH_UNKNOWN_STATE"),
        "{}",
        error.detail
    );
}

#[test]
fn a_broadened_scope_never_refuses_a_value_in_pre_flight() {
    let server = MockServer::start();
    let client = client_with_resolvers(&server, complete_catalogue());

    let resolved = client.resolve_scope(&scope(
        EntityScope::Keyed {
            base: Some(TEAM_KEY.to_owned()),
            additional: vec!["FAR".to_owned()],
        },
        &[("state", "Only In FAR")],
    ));

    assert!(resolved.is_ok(), "{resolved:?}");
}

#[test]
fn the_remedy_names_a_placeholder_when_there_is_no_base_team() {
    let server = MockServer::start();
    let client = client_with_resolvers(
        &server,
        resolvers(&json!({
            "labels": [],
            "teams": [complete_entry(TEAM_ID, TEAM_KEY, &json!({}))]
        })),
    );

    let error = client
        .resolve_scope(&scope(base_only(TEAM_KEY), &[("state", "Nope")]))
        .expect_err("unknown");

    assert!(
        error.detail.contains("--team-id <uuid>"),
        "{}",
        error.detail
    );
}
