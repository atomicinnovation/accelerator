//! Credential failures: each gives its own message, and none prints the
//! token.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use support::Token;

fn verify(scenarios: &[&str], token: &Token) -> std::process::Output {
    let server = support::serving(scenarios);
    let dir = support::scratch(support::CONFIG);
    support::run_with(
        dir.path(),
        &["init", "verify"],
        Some(&server.base_url()),
        token,
    )
}

fn leaks(output: &std::process::Output, token: &str) -> bool {
    String::from_utf8_lossy(&output.stdout).contains(token)
        || String::from_utf8_lossy(&output.stderr).contains(token)
}

#[test]
fn no_token_names_where_to_set_one() {
    let output = verify(&["whoami-200"], &Token::Absent);

    assert_eq!(output.status.code(), Some(24));
    assert!(support::stderr(&output).contains("ACCELERATOR_SHORTCUT_TOKEN"));
}

#[test]
fn a_rejected_token_is_unauthorised_and_unprinted() {
    let output = verify(&["whoami-401"], &Token::Present);

    assert_eq!(output.status.code(), Some(11));
    assert!(!leaks(&output, support::TOKEN_SENTINEL));
}

#[test]
fn a_bearer_prefixed_token_is_malformed_and_unprinted() {
    let token = "Bearer sct_rw_sentinel_do_not_leak";
    let output = verify(&["whoami-200"], &Token::Is(token));

    assert_eq!(output.status.code(), Some(27));
    assert!(support::stderr(&output).contains("Bearer"));
    assert!(!leaks(&output, support::TOKEN_SENTINEL));
}

#[test]
fn a_verified_token_is_never_printed() {
    let output = verify(&["whoami-200"], &Token::Present);

    assert!(output.status.success());
    assert!(!leaks(&output, support::TOKEN_SENTINEL));
}
