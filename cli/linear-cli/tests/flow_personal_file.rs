//! An insecure `config.local.md` through the binary: a writer refuses before
//! any request rather than take its team from the team file.

#![cfg(all(feature = "test-loopback", unix))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::os::unix::fs::PermissionsExt as _;

use http_test_support::{MockHTTPServer, RequestKey, Route};

fn insecure_scratch() -> tempfile::TempDir {
    let dir = support::scratch(support::CONFIG);
    let personal = dir.path().join(".accelerator/config.local.md");
    std::fs::write(&personal, "---\nlinear:\n  team_id: mine\n---\n")
        .expect("write the personal config");
    std::fs::set_permissions(&personal, std::fs::Permissions::from_mode(0o644))
        .expect("loosen the personal config");
    dir
}

#[test]
fn create_refuses_beside_a_team_team_id_and_sends_nothing() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::post("/graphql"), Route::Status(200));
    let dir = insecure_scratch();

    let output = support::run(
        dir.path(),
        &server,
        &["create", "--title", "A brand new issue"],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("E_LOCAL_PERMS_INSECURE"), "{stderr}");
    assert_eq!(server.hits(&RequestKey::post("/graphql")), 0);
}
