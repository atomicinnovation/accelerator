//! Assignee/reporter principal resolution: `@me`, raw accountId, email refusal.

#![allow(clippy::expect_used, clippy::panic)]

use jira_client::principal::resolve;
use jira_client::principal::PrincipalError;

#[test]
fn at_me_resolves_through_the_cached_account_id() {
    assert_eq!(resolve("@me", Some("5b10a2")), Ok("5b10a2".to_owned()));
}

#[test]
fn at_me_without_a_cache_is_a_missing_site_error() {
    assert_eq!(resolve("@me", None), Err(PrincipalError::NoSiteCache));
}

#[test]
fn a_raw_account_id_passes_through_unchanged() {
    assert_eq!(
        resolve("5b10ac8d82e05b22cc7d4ef5", None),
        Ok("5b10ac8d82e05b22cc7d4ef5".to_owned())
    );
    assert_eq!(resolve("qm:abc-123_x", None), Ok("qm:abc-123_x".to_owned()));
}

#[test]
fn an_email_is_refused_never_resolved() {
    let error = resolve("toby@example.com", Some("5b10a2"))
        .expect_err("emails are not resolved");
    assert!(
        matches!(error, PrincipalError::BadPrincipal { .. }),
        "{error:?}"
    );
}

#[test]
fn an_empty_token_is_refused() {
    assert!(matches!(
        resolve("", None),
        Err(PrincipalError::BadPrincipal { .. })
    ));
}
