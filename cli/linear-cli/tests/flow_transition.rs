//! The `transition` flow: the catalogue resolves the state name to a UUID
//! locally, then one update carries it as the `stateId`.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;
use std::process::Output;

use cli_test_support::Scenario;
use http_test_support::{MockHTTPServer, RequestKey, Route};
use serde_json::{json, Value};

fn install(server: &MockHTTPServer, scenario: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/scenarios/{scenario}.json"));
    Scenario::load(&path).expect("scenario").install(server);
}

fn graphql() -> RequestKey {
    RequestKey::post("/graphql")
}

fn run_against(
    server: &MockHTTPServer,
    seed: fn(&Path),
    args: &[&str],
) -> Output {
    let dir = support::scratch(support::CONFIG);
    seed(dir.path());
    support::run(dir.path(), server, args)
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn transition_resolves_the_state_and_posts_the_stateid() {
    for seed in [support::seed_catalogue, support::seed_legacy_catalogue] {
        transitions_over(seed);
    }
}

fn transitions_over(seed: fn(&Path)) {
    let server = MockHTTPServer::start();
    install(&server, "transition-update-200");

    let output =
        run_against(&server, seed, &["transition", "BLA-9", "In Progress"]);

    assert!(
        output.status.success(),
        "transition exited {:?}: {}",
        output.status.code(),
        stderr_of(&output)
    );
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(stdout["outcome"], "transitioned");
    let issue = &stdout["data"]["issueUpdate"]["issue"];
    assert_eq!(issue["identifier"], "BLA-9");
    assert_eq!(issue["state"]["name"], "In Progress");

    assert_eq!(server.hits(&graphql()), 1);
    let sent: Value =
        serde_json::from_slice(&server.last_body(&graphql()).expect("a body"))
            .expect("a JSON request");
    assert_eq!(
        sent["variables"],
        json!({"id": "BLA-9", "input": {"stateId": "state-ip-uuid"}})
    );
    let document = sent["query"].as_str().expect("a document");
    assert!(document.contains("issueUpdate"), "sent: {document}");
    assert!(
        document.contains("state { name }"),
        "the echoed state name is asked for: {document}"
    );
}

#[test]
fn a_failed_transition_reports_as_a_failed_update() {
    let server = MockHTTPServer::start();
    install(&server, "bad-request-400");

    let output = run_against(
        &server,
        support::seed_catalogue,
        &["transition", "BLA-9", "In Progress"],
    );

    assert_eq!(output.status.code(), Some(34));
    assert_eq!(stderr_of(&output), "linear update failed\n");
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(server.hits(&graphql()), 1);
}

fn refused_with_a_bare_401(args: &[&str]) -> (Option<i32>, String) {
    let server = MockHTTPServer::start();
    server.route(graphql(), Route::Status(401));
    let output = run_against(&server, support::seed_catalogue, args);
    (output.status.code(), stderr_of(&output))
}

#[test]
fn a_failed_transition_exits_as_the_same_state_update_would() {
    let transition =
        refused_with_a_bare_401(&["transition", "BLA-9", "In Progress"]);
    let update =
        refused_with_a_bare_401(&["update", "BLA-9", "--state", "In Progress"]);

    assert_eq!(transition.0, Some(20));
    assert_eq!(transition, update);
}

#[test]
fn a_transition_of_a_refused_identifier_sends_nothing() {
    let server = MockHTTPServer::start();
    install(&server, "transition-update-200");

    let output = run_against(
        &server,
        support::seed_catalogue,
        &["transition", "#BLA-9", "In Progress"],
    );

    assert_eq!(output.status.code(), Some(34));
    assert_eq!(stderr_of(&output), "linear update failed\n");
    assert_eq!(server.hits(&graphql()), 0);
}

fn assert_refused_before_sending(seed: fn(&Path), state: &str, code: i32) {
    let server = MockHTTPServer::start();
    install(&server, "transition-update-200");

    let output = run_against(&server, seed, &["transition", "BLA-9", state]);

    assert_eq!(output.status.code(), Some(code));
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(server.hits(&graphql()), 0);
}

#[test]
fn an_unknown_state_is_refused_before_sending() {
    assert_refused_before_sending(support::seed_catalogue, "Nonexistent", 122);
}

#[test]
fn an_ambiguous_state_is_refused_before_sending() {
    assert_refused_before_sending(
        support::seed_ambiguous_catalogue,
        "Done",
        123,
    );
}
