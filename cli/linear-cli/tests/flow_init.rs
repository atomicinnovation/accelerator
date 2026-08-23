//! `init verify` validates credentials without ever printing the token
//! (Decision 3). The `Secret`-redaction invariant is why the guarantee holds;
//! this pins it observably across the success path.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::MockServer;

#[test]
fn init_verify_never_emits_the_token_on_stdout_or_stderr() {
    let server = MockServer::start();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios/viewer-200.json");
    Scenario::load(&path).expect("scenario").install(&server);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "verify"]);

    assert!(output.status.success(), "init verify exited non-zero");
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
    assert!(
        stdout.starts_with("verified\t"),
        "init verify emits the verified keyword: {stdout}"
    );
}
