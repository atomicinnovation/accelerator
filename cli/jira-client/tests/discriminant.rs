//! The structured discriminant the `*_op` methods surface: the
//! binary reads the granular bash exit code from it, and the post-create
//! "created remotely but unwritable" case is a distinct variant, not a wire
//! outcome — so it never has to be parsed back out of a `TrackerError` string.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::classify::bash_code;
use jira_client::mutation::{CreateFields, FieldEdit, IssueType, UpdateFields};
use jira_client::JiraFailure;
use serde_json::{Map, Value};
use support::client::{brief, client_for};
use tracker::ExternalId;
use tracker::RemoteTracker as _;

const ISSUE: &str = "/rest/api/3/issue";

fn id(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

const fn create<'a>(
    summary: &'a str,
    body: &'a str,
    kind: &'a str,
    custom: &'a Map<String, Value>,
) -> CreateFields<'a> {
    CreateFields {
        summary,
        body,
        issue_type: IssueType::Name(kind),
        project: None,
        assignee: None,
        reporter: None,
        priority: None,
        labels: &[],
        components: &[],
        parent: None,
        custom,
    }
}

const fn edit<'a>(
    summary: &'a str,
    body: &'a str,
    custom: &'a Map<String, Value>,
) -> UpdateFields<'a> {
    UpdateFields {
        summary: Some(summary),
        body: Some(body),
        priority: None,
        assignee: None::<FieldEdit<'a>>,
        reporter: None,
        parent: None,
        labels: None,
        add_labels: &[],
        remove_labels: &[],
        components: None,
        add_components: &[],
        remove_components: &[],
        custom,
        no_notify: false,
    }
}

fn json(status: u16, body: &str) -> Route {
    Route::Json {
        status,
        body: body.to_owned(),
    }
}

#[test]
fn a_create_wire_failure_surfaces_the_outcome_the_exit_code_reads() {
    let server = MockServer::start();
    server.route(RequestKey::post(ISSUE), json(401, "{}"));

    let client = client_for(&server, brief());
    let custom = Map::new();
    let failure = client
        .create_op(&create("t", "b", "story", &custom))
        .expect_err("401");

    match failure {
        JiraFailure::Wire { outcome, .. } => {
            assert_eq!(bash_code(outcome), 11, "401 is unauthorised");
        }
        other => panic!("a 401 is a wire failure, got {other:?}"),
    }
}

#[test]
fn a_created_but_unwritable_identifier_is_a_distinct_variant() {
    let server = MockServer::start();
    // The create succeeds, but the returned key carries a control byte the
    // frontmatter-safety check refuses — created remotely, unwritable.
    server.route(
        RequestKey::post(ISSUE),
        json(201, "{\"key\":\"ENG-1\\u0001\"}"),
    );

    let client = client_for(&server, brief());
    let custom = Map::new();
    let failure = client
        .create_op(&create("t", "b", "story", &custom))
        .expect_err("unwritable");

    assert!(
        matches!(failure, JiraFailure::UnwritableIdentifier { .. }),
        "expected the post-create case, got {failure:?}"
    );
}

#[test]
fn an_update_wire_failure_carries_the_granular_not_found_code() {
    let server = MockServer::start();
    server.route(RequestKey::put(&format!("{ISSUE}/ENG-1")), json(404, "{}"));

    let client = client_for(&server, brief());
    let custom = Map::new();
    let failure = client
        .update_op(&id("ENG-1"), &edit("t", "b", &custom))
        .expect_err("404");

    match failure {
        JiraFailure::Wire { outcome, .. } => {
            assert_eq!(bash_code(outcome), 13, "a 404 is not-found");
        }
        other => panic!("a 404 is a wire failure, got {other:?}"),
    }
}

#[test]
fn a_show_wire_failure_surfaces_the_granular_code() {
    let server = MockServer::start();
    server.route(RequestKey::get(&format!("{ISSUE}/ENG-9")), json(403, "{}"));

    let client = client_for(&server, brief());
    let failure = client.show_op(&id("ENG-9")).expect_err("403");

    match failure {
        JiraFailure::Wire { outcome, .. } => {
            assert_eq!(bash_code(outcome), 12, "a 403 is forbidden");
        }
        other => panic!("a 403 is a wire failure, got {other:?}"),
    }
}

#[test]
fn the_port_create_still_collapses_to_a_tracker_error() {
    let server = MockServer::start();
    server.route(RequestKey::post(ISSUE), json(500, "{}"));

    let client = client_for(&server, brief());
    // The `RemoteTracker` port derives its `TrackerError` from the same
    // discriminant, so its observable behaviour is unchanged.
    let error = client.create("t", "b", "story").expect_err("500");
    assert!(
        matches!(error, tracker::TrackerError::Terminal { .. }),
        "a 500 on create is terminal: {error:?}"
    );
}
