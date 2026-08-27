//! The `update` flow: a plain field update posts one `issueUpdate` mutation and
//! emits the `updated\t<identifier>` text discriminant.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::{MockServer, RequestKey};

fn install(server: &MockServer) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios/issue-update-200.json");
    Scenario::load(&path).expect("scenario").install(server);
}

#[test]
fn update_posts_the_fields_and_reports_the_updated_keyword() {
    let server = MockServer::start();
    install(&server);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &[
            "update",
            "BLA-1",
            "--title",
            "Renamed",
            "--description",
            "Rewritten body",
        ],
    );

    assert!(
        output.status.success(),
        "update exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"updated\tBLA-1\n");

    let key = RequestKey::post("/graphql");
    assert_eq!(server.hits(&key), 1);
    let sent = String::from_utf8_lossy(&server.last_body(&key).expect("body"))
        .into_owned();
    assert!(sent.contains("issueUpdate"), "sent: {sent}");
    assert!(sent.contains("Renamed") && sent.contains("Rewritten body"));
}

#[test]
fn update_with_no_fields_is_a_usage_error() {
    let server = MockServer::start();
    install(&server);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["update", "BLA-1"]);

    assert_eq!(output.status.code(), Some(111));
    assert_eq!(
        server.hits(&RequestKey::post("/graphql")),
        0,
        "a no-op update never reaches the wire"
    );
}
