//! `init verify` validates credentials without ever printing the token
//! and persists `viewer.json`; `init discover` persists
//! `catalogue.json`. The `Secret`-redaction invariant is why the no-token
//! guarantee holds; the binary owns cache production so the repointed skill
//! needs no `Write` grant.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::MockServer;
use serde_json::Value;

fn install(server: &MockServer, name: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios")
        .join(format!("{name}.json"));
    Scenario::load(&path).expect("scenario").install(server);
}

fn state_file(dir: &Path, name: &str) -> std::path::PathBuf {
    dir.join(".accelerator/state/integrations/linear")
        .join(name)
}

#[test]
fn init_verify_persists_the_viewer_without_leaking_the_token() {
    let server = MockServer::start();
    install(&server, "viewer-200");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "verify"]);

    assert!(
        output.status.success(),
        "init verify exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains(support::TOKEN_SENTINEL),
        "the token must never reach stdout: {stdout}"
    );
    assert!(
        !stderr.contains(support::TOKEN_SENTINEL),
        "the token must never reach stderr: {stderr}"
    );
    let doc: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(doc.get("outcome").and_then(Value::as_str), Some("verified"));
    assert_eq!(doc.get("name").and_then(Value::as_str), Some("Test User"));

    // The binary owns cache production: viewer.json is written and the token is
    // not in it either.
    let viewer = std::fs::read_to_string(state_file(dir.path(), "viewer.json"))
        .expect("viewer.json written");
    assert!(viewer.contains("Test User"));
    assert!(!viewer.contains(support::TOKEN_SENTINEL));
}

#[test]
fn init_discover_persists_the_catalogue() {
    let server = MockServer::start();
    install(&server, "team-states-200");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["init", "discover", "--team-id", "team-x-uuid"],
    );

    assert!(
        output.status.success(),
        "init discover exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let doc: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        doc.get("outcome").and_then(Value::as_str),
        Some("discovered")
    );

    let catalogue =
        std::fs::read_to_string(state_file(dir.path(), "catalogue.json"))
            .expect("catalogue.json written");
    let parsed: Value =
        serde_json::from_str(&catalogue).expect("catalogue is JSON");
    assert_eq!(
        parsed.pointer("/team/key").and_then(Value::as_str),
        Some("BLA")
    );
    assert_eq!(
        parsed
            .pointer("/workflowStates/1/name")
            .and_then(Value::as_str),
        Some("In Progress")
    );
    // The catalogue is committed, so it is not gitignored; viewer.json is.
    let gitignore =
        std::fs::read_to_string(state_file(dir.path(), ".gitignore"))
            .expect(".gitignore scaffolded");
    assert!(gitignore.lines().any(|line| line == "viewer.json"));
    assert!(!gitignore.lines().any(|line| line == "catalogue.json"));
}

#[test]
fn init_list_teams_renders_the_teams_with_the_listed_keyword() {
    let server = MockServer::start();
    install(&server, "teams-200");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "list-teams"]);

    assert!(
        output.status.success(),
        "init list-teams exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        stdout.get("outcome").and_then(Value::as_str),
        Some("listed")
    );
    assert!(
        stdout.pointer("/teams").is_some(),
        "the team list is rendered: {stdout}"
    );
}
