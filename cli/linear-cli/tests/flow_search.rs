//! The `search` flow: the JSON envelope the repointed body renders (nodes with
//! state and assignee from the read-side projection, plus the truncated flag
//! and the outcome keyword), and the composed-filter stderr audit line.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::MockServer;
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
