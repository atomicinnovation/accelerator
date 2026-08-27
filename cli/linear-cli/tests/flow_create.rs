//! The `create` flow: the `created\t<identifier>` text discriminant on success,
//! and the fail-closed `writeback-failed` orphan case when the remote returns an
//! unusable identifier.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::{MockServer, RequestKey};

fn install(server: &MockServer, name: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios")
        .join(format!("{name}.json"));
    Scenario::load(&path).expect("scenario").install(server);
}

#[test]
fn create_emits_the_created_keyword_and_identifier() {
    let server = MockServer::start();
    install(&server, "create-201");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["create", "--title", "A brand new issue"],
    );

    assert!(
        output.status.success(),
        "create exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"created\tBLA-123\n");

    let key = RequestKey::post("/graphql");
    assert_eq!(server.hits(&key), 1);
    let sent = String::from_utf8_lossy(&server.last_body(&key).expect("body"))
        .into_owned();
    assert!(sent.contains("issueCreate"), "sent: {sent}");
    assert!(
        sent.contains("A brand new issue"),
        "the title passes through"
    );
}

#[test]
fn create_with_an_unusable_identifier_fails_closed() {
    let server = MockServer::start();
    install(&server, "create-malformed-identifier-201");
    let dir = support::scratch(support::CONFIG);

    let output =
        support::run(dir.path(), &server, &["create", "--title", "Injected"]);

    // Created remotely but unwritable: exit 107, the writeback-failed keyword,
    // and a reconcile-manually diagnostic naming the orphaned key.
    assert_eq!(output.status.code(), Some(107));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("writeback-failed\t"), "stdout: {stdout}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E_CREATE_WRITEBACK_FAILED")
            && stderr.contains("reconcile manually"),
        "stderr: {stderr}"
    );
}
