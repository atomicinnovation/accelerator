//! An insecure `config.local.md`: `init discover`, which writes, refuses
//! before any request.
#![cfg(all(feature = "test-loopback", unix))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::os::unix::fs::PermissionsExt as _;

use http_test_support::RequestKey;

#[test]
fn discover_refuses_beside_an_insecure_personal_file_and_sends_nothing() {
    let server = support::serving(&["whoami-200", "workspace-200"]);
    let dir = support::scratch(support::CONFIG);
    let personal = dir.path().join(".accelerator/config.local.md");
    std::fs::write(&personal, "---\nshortcut:\n  team_key: mine\n---\n")
        .expect("write the personal config");
    std::fs::set_permissions(&personal, std::fs::Permissions::from_mode(0o644))
        .expect("loosen the personal config");

    let output = support::run(
        dir.path(),
        &server,
        &["init", "discover", "--team", "team-1"],
    );

    let stderr = support::stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("E_LOCAL_PERMS_INSECURE"), "{stderr}");
    assert_eq!(server.hits(&RequestKey::get("/api/v4/whoami")), 0);
}
