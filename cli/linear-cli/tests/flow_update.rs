//! The `update` flow: one `issueUpdate` mutation whose input holds exactly the
//! fields the flags name, with `--state` resolved through the catalogue, then
//! the `updated\t<identifier>` text discriminant.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;
use std::process::Output;

use cli_test_support::Scenario;
use http_test_support::{MockHTTPServer, RequestKey};
use serde_json::{json, Value};

fn install(server: &MockHTTPServer, scenario: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/scenarios/{scenario}.json"));
    Scenario::load(&path).expect("scenario").install(server);
}

fn graphql() -> RequestKey {
    RequestKey::post("/graphql")
}

fn sent_input(server: &MockHTTPServer) -> Value {
    let key = graphql();
    assert_eq!(server.hits(&key), 1, "exactly one issueUpdate is sent");
    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("a JSON request");
    assert!(sent["query"]
        .as_str()
        .expect("a document")
        .contains("issueUpdate"));
    sent["variables"]["input"].clone()
}

fn update(seed: Option<fn(&Path)>, args: &[&str]) -> (MockHTTPServer, Output) {
    let server = MockHTTPServer::start();
    install(&server, "issue-update-200");
    let dir = support::scratch(support::CONFIG);
    if let Some(seed) = seed {
        seed(dir.path());
    }
    let output = support::run(
        dir.path(),
        &server,
        &[&["update", "BLA-1"], args].concat(),
    );
    (server, output)
}

fn assert_updated(output: &Output) {
    assert!(
        output.status.success(),
        "update exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"updated\tBLA-1\n");
}

fn input_sent_for(args: &[&str]) -> Value {
    let (server, output) = update(None, args);
    assert_updated(&output);
    sent_input(&server)
}

fn assert_refused(output: &Output, code: u8, diagnostic: &str) {
    assert_eq!(output.status.code(), Some(i32::from(code)));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(diagnostic), "stderr: {stderr}");
}

#[test]
fn a_description_alone_is_the_whole_input() {
    assert_eq!(
        input_sent_for(&["--description", "x"]),
        json!({"description": "x"})
    );
}

#[test]
fn a_title_alone_is_the_whole_input() {
    assert_eq!(input_sent_for(&["--title", "x"]), json!({"title": "x"}));
}

#[test]
fn a_priority_alone_is_sent_as_a_number() {
    assert_eq!(input_sent_for(&["--priority", "1"]), json!({"priority": 1}));
}

#[test]
fn an_assignee_alone_is_the_whole_input() {
    assert_eq!(
        input_sent_for(&["--assignee-id", "user-alice-uuid"]),
        json!({"assigneeId": "user-alice-uuid"})
    );
}

#[test]
fn an_empty_description_is_sent_to_clear_it() {
    assert_eq!(
        input_sent_for(&["--description", ""]),
        json!({"description": ""})
    );
}

#[test]
fn a_title_and_description_are_the_whole_input() {
    assert_eq!(
        input_sent_for(&[
            "--title",
            "Renamed",
            "--description",
            "Rewritten body"
        ]),
        json!({"title": "Renamed", "description": "Rewritten body"})
    );
}

#[test]
fn a_state_joins_the_other_fields_in_one_update() {
    for seed in [support::seed_catalogue, support::seed_legacy_catalogue] {
        let (server, output) =
            update(Some(seed), &["--state", "In Progress", "--title", "x"]);

        assert_updated(&output);
        assert_eq!(
            sent_input(&server),
            json!({"stateId": "state-ip-uuid", "title": "x"})
        );
    }
}

#[test]
fn a_quiet_state_change_reports_the_updated_keyword() {
    let (server, output) = update(
        Some(support::seed_catalogue),
        &["--state", "In Progress", "--quiet"],
    );

    assert_updated(&output);
    assert_eq!(sent_input(&server), json!({"stateId": "state-ip-uuid"}));
}

#[test]
fn an_unknown_state_is_refused_before_sending() {
    let (server, output) = update(
        Some(support::seed_catalogue),
        &["--state", "Nowhere", "--title", "x"],
    );

    assert_refused(&output, 122, "E_TRANSITION_STATE_NOT_IN_CATALOGUE");
    assert_eq!(server.hits(&graphql()), 0);
}

#[test]
fn a_state_without_a_catalogue_is_refused_before_sending() {
    let (server, output) =
        update(None, &["--state", "Nowhere", "--title", "x"]);

    assert_refused(&output, 122, "E_TRANSITION_STATE_NOT_IN_CATALOGUE");
    assert_eq!(server.hits(&graphql()), 0);
}

#[test]
fn an_ambiguous_state_is_refused_before_sending() {
    let (server, output) = update(
        Some(support::seed_ambiguous_catalogue),
        &["--state", "Done", "--title", "x"],
    );

    assert_refused(&output, 123, "E_TRANSITION_STATE_AMBIGUOUS");
    assert_eq!(server.hits(&graphql()), 0);
}

#[test]
fn a_failed_state_change_reports_like_any_failed_update() {
    let server = MockHTTPServer::start();
    install(&server, "bad-request-400");
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(
        dir.path(),
        &server,
        &["update", "BLA-1", "--state", "In Progress", "--title", "x"],
    );

    assert_refused(&output, 34, "linear update failed");
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(server.hits(&graphql()), 1);
}

#[test]
fn update_with_no_fields_is_a_usage_error() {
    let server = MockHTTPServer::start();
    install(&server, "issue-update-200");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["update", "BLA-1"]);

    assert_eq!(output.status.code(), Some(111));
    assert_eq!(
        server.hits(&graphql()),
        0,
        "a no-op update never reaches the wire"
    );
}
