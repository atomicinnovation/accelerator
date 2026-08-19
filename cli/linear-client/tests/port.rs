//! The four port operations, against a mock server.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use serde_json::Value;
use support::client::{brief, client_for, client_with, TEAM_ID, TEAM_KEY};
use tracker::{ExternalId, RemoteTimestamp, RemoteTracker as _, TrackerError};

const GRAPHQL: &str = "/graphql";

fn id(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

const fn json_route(body: String) -> Route {
    Route::Json { status: 200, body }
}

fn issue_body(identifier: &str, updated: &str, description: &str) -> String {
    format!(
        "{{\"data\":{{\"issue\":{{\"id\":\"uuid\",\
         \"identifier\":\"{identifier}\",\"title\":\"A title\",\
         \"updatedAt\":\"{updated}\",\"description\":{description}}}}}}}"
    )
}

fn search_body(identifiers: &[&str], next: Option<&str>) -> String {
    let nodes: Vec<String> = identifiers
        .iter()
        .map(|identifier| {
            format!(
                "{{\"id\":\"u\",\"identifier\":\"{identifier}\",\
                 \"title\":\"t\",\"updatedAt\":\"2026-01-01T00:00:00.000Z\"}}"
            )
        })
        .collect();
    let page = next.map_or_else(
        || "{\"hasNextPage\":false,\"endCursor\":null}".to_owned(),
        |cursor| format!("{{\"hasNextPage\":true,\"endCursor\":\"{cursor}\"}}"),
    );
    format!(
        "{{\"data\":{{\"issues\":{{\"nodes\":[{}],\"pageInfo\":{page}}}}}}}",
        nodes.join(",")
    )
}

#[test]
fn create_sends_the_mutation_and_returns_the_identifier() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        json_route(
            "{\"data\":{\"issueCreate\":{\"success\":true,\
             \"issue\":{\"id\":\"u\",\"identifier\":\"ENG-42\"}}}}"
                .to_owned(),
        ),
    );
    let client = client_for(&server, brief());

    let created = client
        .create("A title", "A body\n", "story")
        .expect("create succeeds");

    assert_eq!(created, id("ENG-42"));
    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert!(sent["query"]
        .as_str()
        .expect("a document")
        .contains("issueCreate"));
    assert_eq!(sent["variables"]["input"]["teamId"], TEAM_ID);
    assert_eq!(sent["variables"]["input"]["title"], "A title");
    assert_eq!(
        sent["variables"]["input"]["description"], "A body\n",
        "Linear is Markdown-native: the body passes through verbatim"
    );
}

#[test]
fn a_created_identifier_that_cannot_be_written_back_is_terminal() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(
            "{\"data\":{\"issueCreate\":{\"issue\":{\"identifier\":\"---X\"}}}}"
                .to_owned(),
        ),
    );
    let client = client_for(&server, brief());

    let error = client
        .create("A title", "A body\n", "")
        .expect_err("an unusable identifier is a failure");

    assert!(matches!(error, TrackerError::Terminal { .. }), "{error}");
}

#[test]
fn a_two_hundred_carrying_an_auth_error_is_retryable_on_create_and_update() {
    let body = "{\"errors\":[{\"message\":\"no\",\
                \"extensions\":{\"type\":\"authentication error\"}}]}";
    for expectation in ["create", "update"] {
        let server = MockServer::start();
        server.route(RequestKey::post(GRAPHQL), json_route(body.to_owned()));
        let client = client_for(&server, brief());

        let error = if expectation == "create" {
            client.create("t", "b\n", "").expect_err("it fails")
        } else {
            client
                .update(&id("ENG-1"), "t", "b\n")
                .expect_err("it fails")
        };

        assert!(
            matches!(error, TrackerError::Retryable { .. }),
            "{expectation}: a provably-unapplied auth rejection: {error}"
        );
    }
}

#[test]
fn a_two_hundred_carrying_an_unclassified_error_diverges_between_operations() {
    let body = "{\"errors\":[{\"message\":\"Field does not exist\"}]}";

    let server = MockServer::start();
    server.route(RequestKey::post(GRAPHQL), json_route(body.to_owned()));
    let created = client_for(&server, brief())
        .create("t", "b\n", "")
        .expect_err("it fails");
    assert!(
        matches!(created, TrackerError::Retryable { .. }),
        "{created}"
    );

    let server = MockServer::start();
    server.route(RequestKey::post(GRAPHQL), json_route(body.to_owned()));
    let updated = client_for(&server, brief())
        .update(&id("ENG-1"), "t", "b\n")
        .expect_err("it fails");
    assert!(
        matches!(updated, TrackerError::Terminal { .. }),
        "a 200-body error may mean the update applied: {updated}"
    );
}

#[test]
fn update_sends_the_mutation_with_the_identifier_and_input() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        json_route(
            "{\"data\":{\"issueUpdate\":{\"success\":true,\
             \"issue\":{\"identifier\":\"ENG-1\"}}}}"
                .to_owned(),
        ),
    );
    let client = client_for(&server, brief());

    client
        .update(&id("ENG-1"), "New title", "New body\n")
        .expect("update succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert!(sent["query"]
        .as_str()
        .expect("a document")
        .contains("issueUpdate"));
    assert_eq!(sent["variables"]["id"], "ENG-1");
    assert_eq!(sent["variables"]["input"]["title"], "New title");
    assert_eq!(sent["variables"]["input"]["description"], "New body\n");
}

#[test]
fn show_projects_the_body_with_exactly_one_trailing_newline() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(issue_body(
            "ENG-1",
            "2026-01-01T00:00:00.000Z",
            "\"Some *markdown*\"",
        )),
    );
    let client = client_for(&server, brief());

    let issue = client.show(&id("ENG-1")).expect("show succeeds");

    assert_eq!(issue.body, "A title\nSome *markdown*\n");
    assert!(!issue.body.ends_with("\n\n"));
    assert_eq!(
        issue.updated,
        RemoteTimestamp::Reported("2026-01-01T00:00:00.000Z".to_owned())
    );
}

#[test]
fn an_empty_string_description_projects_as_an_empty_line() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(issue_body("ENG-1", "2026-01-01T00:00:00.000Z", "\"\"")),
    );
    let client = client_for(&server, brief());

    let issue = client.show(&id("ENG-1")).expect("show succeeds");

    assert_eq!(
        issue.body, "A title\n",
        "Linear's empty description projects as an empty line, where Jira's \
         absent one projects as the literal null"
    );
}

#[test]
fn a_null_or_absent_stamp_is_not_reported() {
    for description in ["null", "\"\""] {
        let server = MockServer::start();
        server.route(
            RequestKey::post(GRAPHQL),
            json_route(format!(
                "{{\"data\":{{\"issue\":{{\"identifier\":\"ENG-1\",\
                 \"title\":\"t\",\"updatedAt\":null,\
                 \"description\":{description}}}}}}}"
            )),
        );
        let client = client_for(&server, brief());

        let issue = client.show(&id("ENG-1")).expect("show succeeds");

        assert_eq!(issue.updated, RemoteTimestamp::NotReported);
    }
}

#[test]
fn an_empty_request_makes_no_remote_call() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(500));
    let client = client_for(&server, brief());

    let outcome = client.fetch_all(&[]).expect("an empty request succeeds");

    assert_eq!(server.hits(&key), 0);
    assert!(outcome.found.is_empty());
    assert!(outcome.absent.is_empty());
    assert!(outcome.indeterminate.is_empty());
}

#[test]
fn duplicate_ids_are_deduplicated() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(search_body(&["ENG-1"], None)),
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-1")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
}

#[test]
fn every_search_request_carries_an_explicit_first() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), json_route(search_body(&["ENG-1"], None)));
    let client = client_for(&server, brief());

    client
        .fetch_all(&[id("ENG-1")])
        .expect("fetch_all succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert_eq!(
        sent["variables"]["first"], 250,
        "the API's default of 50 multiplied by complexity is what the \
         explicit value exists to bound"
    );
    assert_eq!(sent["variables"]["filter"]["team"]["id"]["eq"], TEAM_ID);
}

#[test]
fn an_unfound_in_team_id_is_absent_when_the_retrieval_completed() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(search_body(&["ENG-1"], None)),
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-2")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
    assert_eq!(outcome.absent, vec![id("ENG-2")]);
    assert!(outcome.indeterminate.is_empty());
}

#[test]
fn an_id_outside_the_configured_team_is_indeterminate_not_absent() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(search_body(&["ENG-1"], None)),
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("OPS-7")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
    assert!(
        outcome.absent.is_empty(),
        "the search never had scope to see OPS-7, so its absence is unproven"
    );
    assert_eq!(outcome.indeterminate, vec![id("OPS-7")]);
    assert_eq!(TEAM_KEY, "ENG");
}

#[test]
fn without_a_known_team_key_no_absence_can_be_proved() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(search_body(&["ENG-1"], None)),
    );
    // Only linear.team_id configured: the UUID alone cannot answer whether an
    // identifier was ever in scope.
    let client = client_with(&server.base_url(), brief(), None);

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-2")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
    assert!(outcome.absent.is_empty());
    assert_eq!(outcome.indeterminate, vec![id("ENG-2")]);
}

#[test]
fn a_failed_search_reports_every_unfound_id_indeterminate() {
    let server = MockServer::start();
    server.route(RequestKey::post(GRAPHQL), Route::Status(500));
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-2")])
        .expect("a transport-level failure is an Ok with the partition");

    assert!(outcome.absent.is_empty());
    assert_eq!(outcome.indeterminate.len(), 2);
}

#[test]
fn a_stamp_absent_from_a_bulk_row_is_still_found() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(
            "{\"data\":{\"issues\":{\"nodes\":[{\"identifier\":\"ENG-1\"}],\
             \"pageInfo\":{\"hasNextPage\":false}}}}"
                .to_owned(),
        ),
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1")])
        .expect("fetch_all succeeds");

    assert_eq!(
        outcome.found,
        vec![(id("ENG-1"), RemoteTimestamp::NotReported)],
        "dropping a null-stamped row would report a live issue as deleted"
    );
}

#[test]
fn an_unsafe_identifier_is_a_preflight_error() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(200));
    let client = client_for(&server, brief());

    let error = client
        .fetch_all(&[id("ENG-1"), id("bad\nkey")])
        .expect_err("an unsafe id fails before any request");

    assert!(matches!(error, TrackerError::Retryable { .. }), "{error}");
    assert_eq!(server.hits(&key), 0);
}

#[test]
fn a_404_shaped_read_failure_is_retryable_never_terminal() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Json {
            status: 400,
            body: "{\"errors\":[{\"message\":\"Entity not found\"}]}"
                .to_owned(),
        },
    );
    let client = client_for(&server, brief());

    let error = client.show(&id("ENG-404")).expect_err("the read fails");

    assert!(matches!(error, TrackerError::Retryable { .. }), "{error}");
}
