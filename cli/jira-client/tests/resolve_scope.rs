//! `resolve_scope`: Jira's discovery-path refusal, run before `search`.
//! An unscoped scope is refused with `E_JQL_NO_PROJECT`; a scoped one passes
//! through unchanged (Jira's JQL uses the project key directly).

#![allow(clippy::expect_used)]

mod support;

use http_test_support::MockServer;
use support::client::client_for;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker_support::TransportConfig;

fn scope(project: Option<&str>, all_projects: bool) -> SearchScope {
    SearchScope {
        project: project.map(str::to_owned),
        all_projects,
        filters: Vec::new(),
    }
}

#[test]
fn a_scoped_project_passes_through_unchanged() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let resolved = client
        .resolve_scope(&scope(Some("OPS"), false))
        .expect("a project-scoped run resolves");

    assert_eq!(resolved.project.as_deref(), Some("OPS"));
    assert_eq!(
        server.hits(&http_test_support::RequestKey::post(
            "/rest/api/3/search/jql"
        )),
        0,
        "resolve_scope makes no network call"
    );
}

#[test]
fn all_projects_passes_through_unchanged() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let resolved = client
        .resolve_scope(&scope(None, true))
        .expect("an all-projects run resolves");

    assert!(resolved.all_projects);
}

#[test]
fn an_unscoped_run_is_refused_with_e_jql_no_project() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .resolve_scope(&scope(None, false))
        .expect_err("an unscoped run is refused");

    assert!(
        error.detail.contains("E_JQL_NO_PROJECT"),
        "{}",
        error.detail
    );
}
