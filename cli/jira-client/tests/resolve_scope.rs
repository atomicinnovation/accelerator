//! `resolve_scope`: Jira's discovery-path refusal, run before `search`.
//! An unscoped scope is refused with `E_JQL_NO_PROJECT`; a scoped one passes
//! through unchanged (Jira's JQL uses the project key directly).

#![allow(clippy::expect_used)]

mod support;

use http_test_support::MockServer;
use support::client::client_for;
use tracker::EntityScope;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker_support::TransportConfig;

fn keyed(project: Option<&str>) -> SearchScope {
    SearchScope {
        entities: EntityScope::Keyed {
            base: project.map(str::to_owned),
            additional: Vec::new(),
        },
        filters: Vec::new(),
    }
}

const fn whole_workspace() -> SearchScope {
    SearchScope {
        entities: EntityScope::WholeWorkspace,
        filters: Vec::new(),
    }
}

#[test]
fn a_scoped_project_passes_through_unchanged() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let resolved = client
        .resolve_scope(&keyed(Some("OPS")))
        .expect("a project-scoped run resolves");

    assert_eq!(
        resolved.entities,
        EntityScope::Keyed {
            base: Some("OPS".to_owned()),
            additional: Vec::new(),
        }
    );
    assert_eq!(
        server.hits(&http_test_support::RequestKey::post(
            "/rest/api/3/search/jql"
        )),
        0,
        "resolve_scope makes no network call"
    );
}

#[test]
fn a_whole_workspace_scope_passes_through_unchanged() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let resolved = client
        .resolve_scope(&whole_workspace())
        .expect("a whole-workspace run resolves");

    assert_eq!(resolved.entities, EntityScope::WholeWorkspace);
}

#[test]
fn an_unscoped_run_is_refused_with_e_jql_no_project() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .resolve_scope(&keyed(None))
        .expect_err("an unscoped run is refused");

    assert!(
        error.detail.contains("E_JQL_NO_PROJECT"),
        "{}",
        error.detail
    );
}
