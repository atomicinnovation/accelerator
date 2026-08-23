//! The `show` flow end to end: the binary drives the mock through the seam and
//! renders the issue JSON with the keyword discriminant.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::MockServer;
use serde_json::Value;

fn scenario(name: &str) -> Scenario {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios")
        .join(format!("{name}.json"));
    Scenario::load(&path).expect("a valid scenario")
}

#[test]
fn show_renders_the_issue_with_the_found_keyword() {
    let server = MockServer::start();
    scenario("show-issue-200").install(&server);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["show", "BLA-42"]);

    assert!(
        output.status.success(),
        "show exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("stdout is one JSON doc");
    assert_eq!(
        stdout
            .pointer("/data/issue/identifier")
            .and_then(Value::as_str),
        Some("BLA-42")
    );
    assert_eq!(
        stdout
            .pointer("/data/issue/state/name")
            .and_then(Value::as_str),
        Some("In Review"),
        "the detailed projection carries the state name"
    );
    assert_eq!(
        stdout.get("outcome").and_then(Value::as_str),
        Some("found"),
        "the discriminant is a top-level JSON field, not a trailing line"
    );

    // The request went to the mock through the seam.
    assert_eq!(
        server.hits(&http_test_support::RequestKey::post("/graphql")),
        1
    );
}
