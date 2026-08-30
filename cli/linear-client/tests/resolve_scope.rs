//! `resolve_scope`: the pre-flight step that substitutes a team key for its
//! UUID and refuses a scope that names no team.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::MockServer;
use support::client::{brief, client_for, TEAM_ID, TEAM_KEY};
use tracker::RemoteTracker;
use tracker::SearchScope;

fn keyed(project: Option<&str>) -> SearchScope {
    SearchScope {
        project: project.map(str::to_owned),
        all_projects: false,
        filters: Vec::new(),
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
        resolved.project.as_deref(),
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
