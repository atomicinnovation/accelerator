//! The `ACCELERATOR_JIRA_API_URL` seam (Decision 10). A set-but-inadmissible
//! override is a hard usage error before any credential attaches; a plain-http
//! or loopback override is refused in a build without `test-loopback`. The
//! no-override path takes the `from_config` branch, exercised here through its
//! deterministic missing-credential failure so the real-user path is not left
//! to manual verification alone.
//!
//! Ungated on the feature so it runs in the default build too — that is where
//! loopback admission must stay off.
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use support::Token;

fn code(args: &[&str], api_url: Option<&str>, token: &Token) -> Option<i32> {
    let dir = support::scratch(support::CONFIG);
    support::run_with(dir.path(), args, api_url, token)
        .status
        .code()
}

#[test]
fn a_non_admissible_host_is_a_usage_error_before_credentials() {
    assert_eq!(
        code(
            &["init", "verify"],
            Some("http://evil.example.com"),
            &Token::Present,
        ),
        Some(2),
        "a non-*.atlassian.net http host is refused as a usage error"
    );
}

#[test]
fn an_unparseable_override_is_a_usage_error() {
    assert_eq!(
        code(&["init", "verify"], Some("::: not a url"), &Token::Present),
        Some(2)
    );
}

#[test]
fn the_from_config_branch_resolves_config_without_an_override() {
    // No override → the `from_config` branch runs; with no token it fails
    // deterministically at credential resolution, before any network, proving
    // the branch is wired without a live call.
    assert_eq!(
        code(&["init", "verify"], None, &Token::Absent),
        Some(24),
        "from_config resolves config then reports the missing token"
    );
}

#[test]
fn bare_init_without_a_default_project_refuses_rather_than_blocking() {
    // A config with no `work.default_project_code`, and the harness runs with
    // stdin closed (never a TTY), so bare `jira init` must refuse with the
    // needs-config code instead of blocking on an interactive prompt.
    let config = "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
        email: toby@example.com\n---\n";
    let dir = support::scratch(config);
    let output =
        support::run_with(dir.path(), &["init"], None, &Token::Present);
    assert_eq!(
        output.status.code(),
        Some(60),
        "no default project + no TTY → INIT_NEEDS_CONFIG, not a hang"
    );
}

#[cfg(not(feature = "test-loopback"))]
#[test]
fn a_loopback_override_is_refused_without_the_test_feature() {
    assert_eq!(
        code(
            &["init", "verify"],
            Some("http://127.0.0.1:9"),
            &Token::Present,
        ),
        Some(2),
        "loopback is admitted only under test-loopback"
    );
}
