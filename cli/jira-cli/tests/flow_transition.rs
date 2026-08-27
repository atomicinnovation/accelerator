//! The `transition` flow: a numeric `--transition-id` posts directly (no
//! lookup) and stamps the `transitioned` text discriminant; a missing target is
//! a pre-wire usage refusal.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};

const TRANSITIONS: &str = "/rest/api/3/issue/ENG-1/transitions";

#[test]
fn a_transition_id_posts_directly_and_stamps_transitioned() {
    let server = MockServer::start();
    server.route(RequestKey::post(TRANSITIONS), Route::Status(204));
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["transition", "ENG-1", "--transition-id", "31"],
    );
    assert!(
        output.status.success(),
        "transition exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"transitioned\tENG-1\n");
    assert_eq!(server.hits(&RequestKey::post(TRANSITIONS)), 1);
}

#[test]
fn a_missing_target_exits_before_the_wire() {
    let server = MockServer::start();
    let dir = support::scratch(support::CONFIG);
    let output = support::run(dir.path(), &server, &["transition", "ENG-1"]);
    assert_eq!(
        output.status.code(),
        Some(121),
        "no state → TRANSITION_NO_STATE"
    );
}
