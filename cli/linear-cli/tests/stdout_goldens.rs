//! Byte-exact stdout goldens for the JSON-emitting subcommands. The mock
//! responses are deterministic and `serde_json` orders keys stably, so the
//! whole
//! rendered document — including the top-level `outcome` discriminant — is
//! pinned as raw bytes (never `from_utf8_lossy`), catching a shape change a
//! field-wise assertion would miss.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::MockServer;

fn drive(scenario: &str, args: &[&str]) -> Vec<u8> {
    let server = MockServer::start();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios")
        .join(format!("{scenario}.json"));
    Scenario::load(&path).expect("scenario").install(&server);
    let dir = support::scratch(support::CONFIG);
    let output = support::run(dir.path(), &server, args);
    assert!(
        output.status.success(),
        "{args:?} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn show_stdout_matches_the_golden() {
    let stdout = drive("show-issue-200", &["show", "BLA-42"]);
    support::assert_golden("show.golden", &stdout);
}

#[test]
fn search_stdout_matches_the_golden() {
    let stdout = drive("search-filter-state-200", &["search", "--text", "bug"]);
    support::assert_golden("search.golden", &stdout);
}
