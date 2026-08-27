//! The behavioural exit-code contract: drive the binary into each reachable
//! error class and assert the *observed* exit code equals the constant
//! `exit_codes.rs` declares for it. The mapping functions there are exhaustive
//! `match`es with no wildcard, so a new error variant is already a compile
//! error at the mapping; this test proves the binary routes each class to the
//! code the fixture pins, not only that the constant exists.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::MockServer;
use support::Token;

fn install(server: &MockServer, name: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios")
        .join(format!("{name}.json"));
    Scenario::load(&path).expect("scenario").install(server);
}

/// Drives `args` against a mock serving `scenario` and returns the exit code.
fn code_for(scenario: &str, args: &[&str]) -> i32 {
    let server = MockServer::start();
    install(&server, scenario);
    let dir = support::scratch(support::CONFIG);
    support::run(dir.path(), &server, args)
        .status
        .code()
        .expect("a coded exit")
}

#[test]
fn network_classes_route_to_their_shared_codes() {
    assert_eq!(
        code_for("bearer-401", &["search", "--text", "x"]),
        11,
        "401 → UNAUTHORIZED"
    );
    assert_eq!(
        code_for("bad-request-400", &["search", "--text", "x"]),
        34,
        "400 validation error → BAD_REQUEST"
    );
}

#[test]
fn show_not_found_routes_to_its_own_code() {
    assert_eq!(code_for("show-issue-404", &["show", "BLA-1"]), 82);
}

#[test]
fn argument_validation_classes_route_before_the_wire() {
    // A mock is unnecessary — these refuse before any request. Reuse a benign
    // scenario so the harness shape stays uniform.
    assert_eq!(code_for("viewer-200", &["comment", "add", "BLA-1"]), 91);
    assert_eq!(
        code_for(
            "viewer-200",
            &["attach", "BLA-1", "--url", "u", "--file", "f"]
        ),
        132
    );
    assert_eq!(code_for("viewer-200", &["attach", "BLA-1"]), 131);
    assert_eq!(code_for("viewer-200", &["transition", "BLA-1"]), 121);
    assert_eq!(code_for("viewer-200", &["update", "BLA-1"]), 111);
}

#[test]
fn a_missing_token_routes_to_no_token() {
    let server = MockServer::start();
    install(&server, "viewer-200");
    let dir = support::scratch(support::CONFIG);
    let output = support::run_with(
        dir.path(),
        &["init", "verify"],
        Some(&format!("{}/graphql", server.base_url())),
        &Token::Absent,
    );
    assert_eq!(output.status.code(), Some(24), "no token → NO_TOKEN");
}
