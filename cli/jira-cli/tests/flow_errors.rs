//! The behavioural exit-code contract: drive the binary into each reachable
//! error class and assert the *observed* exit code equals the constant
//! `exit_codes.rs` declares for it. The mapping functions there are exhaustive
//! `match`es with no wildcard, so a new error variant is already a compile error
//! at the mapping; this test proves the binary routes each class to the pinned
//! code, not only that the constant exists.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};

/// Drives `show ENG-1` against a mock returning `status` and returns the exit
/// code — the surface-flow status→code routing.
fn show_status_code(status: u16) -> i32 {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/rest/api/3/issue/ENG-1"),
        Route::Status(status),
    );
    let dir = support::scratch(support::CONFIG);
    support::run(dir.path(), &server, &["show", "ENG-1"])
        .status
        .code()
        .expect("a coded exit")
}

#[test]
fn http_status_classes_route_to_their_shared_codes() {
    assert_eq!(show_status_code(401), 11, "401 → UNAUTHORIZED");
    assert_eq!(show_status_code(403), 12, "403 → FORBIDDEN");
    assert_eq!(show_status_code(404), 13, "404 → NOT_FOUND");
    assert_eq!(show_status_code(410), 14, "410 → GONE");
    assert_eq!(show_status_code(400), 34, "400 → REQ_BAD_REQUEST");
    // The retrying classes (429 → RATELIMITED, 5xx → SERVER_ERROR) are left to
    // `exit_codes_parity.rs` and the `jira-client` transport tests: driving them
    // here would exhaust the transport's real backoff retries, slowing the suite
    // and starving the parallel runner for no extra routing coverage.
}

/// Drives `args` against a benign mock and returns the exit code — the pre-wire
/// argument-validation classes, which refuse before any request.
fn arg_code(args: &[&str]) -> i32 {
    let server = MockServer::start();
    server.route(RequestKey::get("/rest/api/3/myself"), Route::Status(200));
    let dir = support::scratch(support::CONFIG);
    support::run(dir.path(), &server, args)
        .status
        .code()
        .expect("a coded exit")
}

#[test]
fn argument_validation_classes_route_before_the_wire() {
    assert_eq!(arg_code(&["create"]), 102, "no summary → CREATE_NO_SUMMARY");
    assert_eq!(
        arg_code(&["update", "ENG-1"]),
        112,
        "no ops → UPDATE_NO_OPS"
    );
    assert_eq!(
        arg_code(&["comment", "add", "ENG-1"]),
        94,
        "no body → COMMENT_NO_BODY"
    );
    assert_eq!(
        arg_code(&["transition", "ENG-1"]),
        121,
        "no state → TRANSITION_NO_STATE"
    );
}
