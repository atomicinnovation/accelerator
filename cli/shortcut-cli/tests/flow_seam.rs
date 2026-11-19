//! The `ACCELERATOR_SHORTCUT_API_URL` seam. An override that is set but not
//! admissible is a usage error before any credential attaches. A loopback
//! override is refused in a build without `test-loopback`.
//!
//! Ungated on the feature, so it runs in the default build too, where
//! loopback admission must stay off.
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::process::Output;

use support::Token;

fn run(api_url: Option<&str>, token: &Token) -> Output {
    let dir = support::scratch(support::CONFIG);
    support::run_with(dir.path(), &["init", "verify"], api_url, token)
}

#[test]
fn a_foreign_host_is_a_usage_error_naming_the_override() {
    let output = run(Some("https://evil.example.com"), &Token::Present);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("E_BAD_API_URL:"), "{stderr}");
    assert!(stderr.contains("ACCELERATOR_SHORTCUT_API_URL"), "{stderr}");
}

#[test]
fn an_unparseable_override_is_a_usage_error() {
    assert_eq!(
        run(Some("::: not a url"), &Token::Present).status.code(),
        Some(2)
    );
}

#[test]
fn without_an_override_the_missing_token_is_reported() {
    let output = run(None, &Token::Absent);

    assert_eq!(output.status.code(), Some(24));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("E_NO_TOKEN"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(not(feature = "test-loopback"))]
#[test]
fn a_loopback_override_is_refused_without_the_test_feature() {
    assert_eq!(
        run(Some("http://127.0.0.1:9"), &Token::Present)
            .status
            .code(),
        Some(2)
    );
}
