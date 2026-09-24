//! The `search` flow: the JSON envelope the repointed body renders (nodes with
//! state and assignee from the read-side projection, plus the truncated flag
//! and the outcome keyword), and the composed-filter stderr audit line.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::{MockServer, RequestKey, Route};
use serde_json::Value;

fn install(server: &MockServer) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios/search-filter-state-200.json");
    Scenario::load(&path)
        .expect("a valid scenario")
        .install(server);
}

#[test]
fn search_renders_the_envelope_with_state_and_assignee_and_audits_the_filter() {
    let server = MockServer::start();
    install(&server);
    let dir = support::scratch(support::CONFIG);

    let output =
        support::run(dir.path(), &server, &["search", "--text", "bug"]);

    assert!(output.status.success(), "search exited non-zero");
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    let nodes = stdout
        .pointer("/data/issues/nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    assert_eq!(nodes.len(), 2);
    assert_eq!(
        nodes[0].pointer("/state/name").and_then(Value::as_str),
        Some("In Progress"),
        "the projection carries the state name the port search drops"
    );
    assert_eq!(
        nodes[0].pointer("/assignee/name").and_then(Value::as_str),
        Some("Alice")
    );
    assert_eq!(
        stdout
            .pointer("/data/issues/truncated")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        stdout.get("outcome").and_then(Value::as_str),
        Some("results")
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("INFO: composed IssueFilter:"),
        "the composed filter is audited to stderr: {stderr}"
    );
}

#[test]
fn search_cap_hit_exits_nonzero_with_the_truncated_envelope() {
    // Every page reports a next page, so the walk cap-hits: the search fails
    // loud with the dedicated code, still emitting the truncated envelope.
    let server = MockServer::start();
    server.route(
        RequestKey::post("/graphql"),
        Route::Json {
            status: 200,
            body: "{\"data\":{\"issues\":{\"nodes\":[{\"identifier\":\
                   \"ENG-1\",\"title\":\"t\",\"updatedAt\":\
                   \"2026-01-01T00:00:00.000Z\"}],\"pageInfo\":\
                   {\"hasNextPage\":true,\"endCursor\":\"c\"}}}}"
                .to_owned(),
        },
    );
    let dir = support::scratch(support::CONFIG);

    let output =
        support::run(dir.path(), &server, &["search", "--text", "bug"]);

    assert_eq!(
        output.status.code(),
        Some(79),
        "a cap-hit exits with the dedicated SEARCH_CAP_HIT code"
    );
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        stdout.get("outcome").and_then(Value::as_str),
        Some("cap-hit"),
        "a cap-hit is its own outcome, distinct from a transient cutoff"
    );
    assert_eq!(
        stdout
            .pointer("/data/issues/truncated")
            .and_then(Value::as_bool),
        Some(true),
        "the truncated field rides the envelope alongside the non-zero exit"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E_SEARCH_CAP_HIT") && stderr.contains("max_pages"),
        "the cap-hit names the max_pages remedy: {stderr}"
    );
}

#[test]
fn quiet_suppresses_the_composed_filter_audit() {
    let server = MockServer::start();
    install(&server);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["search", "--text", "bug", "--quiet"],
    );

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("INFO: composed IssueFilter:"),
        "--quiet suppresses the audit line: {stderr}"
    );
}

fn serve_issues(server: &MockServer) {
    server.route(
        RequestKey::graphql("issues"),
        Route::Json {
            status: 200,
            body: "{\"data\":{\"issues\":{\"nodes\":[],\"pageInfo\":\
                   {\"hasNextPage\":false,\"endCursor\":null}}}}"
                .to_owned(),
        },
    );
}

fn sent_filter(server: &MockServer) -> Value {
    let body = server
        .last_body(&RequestKey::graphql("issues"))
        .expect("an issues request");
    let sent: Value = serde_json::from_slice(&body).expect("JSON");
    sent["variables"]["filter"].clone()
}

fn connection(root: &str, nodes: &str) -> Route {
    Route::Json {
        status: 200,
        body: format!(
            "{{\"data\":{{\"{root}\":{{\"nodes\":{nodes},\"pageInfo\":\
             {{\"hasNextPage\":false,\"endCursor\":null}}}}}}}}"
        ),
    }
}

#[test]
fn search_by_label_sends_label_ids() {
    let server = MockServer::start();
    serve_issues(&server);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output =
        support::run(dir.path(), &server, &["search", "--label", "infra"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let filter = sent_filter(&server);
    assert_eq!(filter["labels"]["id"]["eq"], "label-infra-uuid");
    assert_eq!(filter["team"]["id"]["eq"], "team-uuid");
}

#[test]
fn search_by_state_stays_scoped_to_the_init_team() {
    let server = MockServer::start();
    serve_issues(&server);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(
        dir.path(),
        &server,
        &["search", "--state", "In Progress"],
    );

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let filter = sent_filter(&server);
    assert_eq!(filter["state"]["id"]["eq"], "state-ip-uuid");
    assert_eq!(filter["team"]["id"]["eq"], "team-uuid");
}

#[test]
fn search_by_unknown_assignee_refuses_without_a_request() {
    let server = MockServer::start();
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output =
        support::run(dir.path(), &server, &["search", "--assignee", "nobody"]);

    assert_eq!(output.status.code(), Some(89));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E_SEARCH_UNKNOWN_ASSIGNEE"), "{stderr}");
    assert_eq!(server.hits(&RequestKey::post("/graphql")), 0);
}

#[test]
fn search_against_a_legacy_catalogue_fetches_and_never_writes() {
    let server = MockServer::start();
    serve_issues(&server);
    server.route(
        RequestKey::graphql("TeamIdentities"),
        connection(
            "teams",
            "[{\"id\":\"team-uuid\",\"key\":\"BLA\",\"name\":\"Bla\"}]",
        ),
    );
    server.route(
        RequestKey::graphql("TeamLabels"),
        connection(
            "issueLabels",
            "[{\"id\":\"label-infra-uuid\",\"name\":\"infra\",\
              \"team\":{\"id\":\"team-uuid\"}}]",
        ),
    );
    server.route(
        RequestKey::graphql("WorkspaceLabels"),
        connection("issueLabels", "[]"),
    );
    let dir = support::scratch(support::CONFIG);
    support::seed_legacy_catalogue(dir.path());
    let before = std::fs::read(dir.path().join(support::CATALOGUE))
        .expect("the seeded catalogue");

    let output =
        support::run(dir.path(), &server, &["search", "--label", "infra"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        sent_filter(&server)["labels"]["id"]["eq"],
        "label-infra-uuid"
    );
    assert_eq!(server.hits(&RequestKey::graphql("TeamLabels")), 1);
    assert_eq!(
        std::fs::read(dir.path().join(support::CATALOGUE)).expect("catalogue"),
        before,
        "search never writes the catalogue"
    );
}

#[test]
fn search_by_state_without_a_catalogued_team_refuses_as_no_team() {
    let server = MockServer::start();
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["search", "--state", "In Progress"],
    );

    assert_eq!(output.status.code(), Some(77));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E_SEARCH_NO_TEAM"), "{stderr}");
    assert_eq!(server.hits(&RequestKey::post("/graphql")), 0);
}

#[test]
fn a_text_only_search_without_a_catalogued_team_runs_workspace_wide() {
    let server = MockServer::start();
    serve_issues(&server);
    let dir = support::scratch(support::CONFIG);

    let output =
        support::run(dir.path(), &server, &["search", "--text", "bug"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let filter = sent_filter(&server);
    assert!(filter.get("team").is_none(), "no team clause: {filter}");
    assert_eq!(filter["title"]["containsIgnoreCase"], "bug");
}

#[test]
fn search_without_any_team_exits_105() {
    let server = MockServer::start();
    let dir = support::scratch(support::TEAMLESS_CONFIG);

    let output =
        support::run(dir.path(), &server, &["search", "--text", "bug"]);

    assert_eq!(output.status.code(), Some(105));
}
