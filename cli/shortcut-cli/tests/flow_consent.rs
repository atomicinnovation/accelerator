//! Consent through the binary: a token command set at the team level is
//! refused, and the refusal maps to its exit code.
#![cfg(all(feature = "test-loopback", unix))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::RequestKey;
use support::Token;

const TEAM_COMMAND: &str = "---\nwork:\n  integration: shortcut\nshortcut:\n  \
    team_key: team-1\n  token_cmd: printf team\n---\n";

#[test]
fn a_team_token_command_alone_exits_no_token_and_sends_nothing() {
    let server = support::serving(&["whoami-200"]);
    let dir = support::scratch(TEAM_COMMAND);

    let output = support::run_with(
        dir.path(),
        &["init", "verify"],
        Some(&server.base_url()),
        &Token::Absent,
    );

    assert_eq!(
        output.status.code(),
        Some(24),
        "{}",
        support::stderr(&output)
    );
    assert!(
        support::stderr(&output)
            .contains("E_CONSENT_KEY_TEAM_LEVEL: shortcut.token_cmd"),
        "{}",
        support::stderr(&output)
    );
    assert_eq!(server.hits(&RequestKey::get("/api/v4/whoami")), 0);
}
