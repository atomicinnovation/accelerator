//! The `transition` flow: the catalogue resolves the state name to a UUID
//! locally, then one `issueUpdate` mutation carries the `stateId`.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::{MockServer, RequestKey};
use serde_json::Value;

fn install(server: &MockServer) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios/transition-update-200.json");
    Scenario::load(&path).expect("scenario").install(server);
}

#[test]
fn transition_resolves_the_state_and_posts_the_stateid() {
    let server = MockServer::start();
    install(&server);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(
        dir.path(),
        &server,
        &["transition", "BLA-9", "In Progress"],
    );

    assert!(
        output.status.success(),
        "transition exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        stdout.get("outcome").and_then(Value::as_str),
        Some("transitioned")
    );

    let key = RequestKey::post("/graphql");
    assert_eq!(server.hits(&key), 1);
    let sent = String::from_utf8_lossy(&server.last_body(&key).expect("body"))
        .into_owned();
    assert!(sent.contains("issueUpdate"), "sent: {sent}");
    assert!(
        sent.contains("state-ip-uuid"),
        "the resolved stateId is on the wire: {sent}"
    );
}
