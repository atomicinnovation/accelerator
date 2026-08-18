//! Credential resolution: the ladder through Jira's keys, and the two values
//! the ladder does not carry — the site and the account email.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod support;

use std::path::Path;

use jira_client::auth::{base_url, resolve_credentials, token_keys};
use jira_client::ClientError;
use support::{context, FixedConfig, FixedEnvironment, FixedProvenance};
use tempfile::TempDir;
use tracker_support::{CredentialError, TokenSource};

fn workspace() -> TempDir {
    TempDir::new().expect("a scratch workspace")
}

fn personal_config(root: &Path) -> std::path::PathBuf {
    let path = root.join("config.local.md");
    std::fs::write(&path, "---\njira:\n  token: unused\n---\n")
        .expect("write the personal config");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("tighten the mode");
    }
    path
}

#[test]
fn the_three_values_resolve_together() {
    let root = workspace();
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "env-token");
    let config = FixedConfig::new()
        .with_team("jira.site", "atomic-innovation")
        .with_personal("jira.email", "toby@example.com");
    let provenance = FixedProvenance::nothing_tracked();

    let credentials = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect("all three values resolve");

    assert_eq!(
        credentials.base.as_str(),
        "https://atomic-innovation.atlassian.net/"
    );
    assert_eq!(credentials.email, "toby@example.com");
    assert_eq!(credentials.token.expose(), "env-token");
    assert_eq!(credentials.source, TokenSource::Env);
}

#[test]
fn the_environment_token_outranks_a_configured_one() {
    let root = workspace();
    personal_config(root.path());
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "env-token");
    let config = FixedConfig::new()
        .with_team("jira.site", "tenant")
        .with_team("jira.email", "a@b.c")
        .with_personal("jira.token", "file-token");
    let provenance = FixedProvenance::nothing_tracked();

    let credentials = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect("the environment wins");

    assert_eq!(credentials.token.expose(), "env-token");
}

#[test]
fn a_team_level_token_command_is_refused_with_its_diagnostic() {
    let root = workspace();
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new()
        .with_team("jira.site", "tenant")
        .with_team("jira.email", "a@b.c")
        .with_team("jira.token_cmd", "printf 'no'");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect_err("a shared token_cmd is refused");

    assert!(matches!(
        error,
        ClientError::Credential(
            CredentialError::TokenCmdFromSharedConfig { .. }
        )
    ));
    assert!(
        error.to_string().contains("move it to config.local.md"),
        "{error}"
    );
}

#[test]
fn nothing_configured_is_a_missing_site() {
    let root = workspace();
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect_err("no site is a refusal");

    assert!(matches!(error, ClientError::NoSite));
}

#[test]
fn a_missing_email_is_a_refusal_of_its_own() {
    let root = workspace();
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "t");
    let config = FixedConfig::new().with_team("jira.site", "tenant");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect_err("no email is a refusal");

    assert!(matches!(error, ClientError::NoEmail));
}

#[test]
fn a_missing_token_stays_a_structured_credential_error() {
    let root = workspace();
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new()
        .with_team("jira.site", "tenant")
        .with_team("jira.email", "a@b.c");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect_err("no token is a refusal");

    let source = std::error::Error::source(&error)
        .expect("the credential error survives as a source");
    assert!(source.to_string().contains("E_NO_TOKEN"), "{source}");
    assert!(matches!(
        error,
        ClientError::Credential(CredentialError::NoToken { .. })
    ));
}

#[test]
fn a_token_carrying_a_control_byte_is_refused() {
    let root = workspace();
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "abc\r\ndef");
    let config = FixedConfig::new()
        .with_team("jira.site", "tenant")
        .with_team("jira.email", "a@b.c");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect_err("a header-injecting token is refused");

    assert!(matches!(
        error,
        ClientError::Credential(CredentialError::MalformedToken { .. })
    ));
}

#[test]
fn the_bash_cloud_subdomain_form_still_resolves() {
    let base =
        base_url("atomic-innovation", &[]).expect("a subdomain resolves");
    assert_eq!(base.as_str(), "https://atomic-innovation.atlassian.net/");
}

#[test]
fn a_site_outside_the_allow_shape_is_refused_before_any_request() {
    for site in [
        "http://tenant.atlassian.net",
        "https://user:pass@tenant.atlassian.net",
        "https://tenant.atlassian.net?x=1",
        "https://tenant.atlassian.net#frag",
        "https://tenant.atlassian.net:8443",
        "https://atlassian.net.evil.com",
        "https://evil-atlassian.net",
        "https://jira.internal.example.com",
        "https://127.0.0.1:8080",
    ] {
        let error = base_url(site, &[]).expect_err("the site must be refused");
        assert!(
            matches!(error, ClientError::BadSite { .. }),
            "{site}: {error}"
        );
    }
}

#[test]
fn the_allowlist_admits_only_an_exact_host() {
    let allowed = vec!["jira.internal.example.com".to_owned()];

    assert!(base_url("https://jira.internal.example.com", &allowed).is_ok());
    assert!(
        base_url("https://evil.jira.internal.example.com", &allowed).is_err(),
        "an allowlist entry is an exact host, not a suffix"
    );
}

#[test]
fn an_allowlist_at_team_level_is_refused_and_widens_nothing() {
    let root = workspace();
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "t");
    let config = FixedConfig::new()
        .with_team("jira.site", "https://jira.internal.example.com")
        .with_team("jira.email", "a@b.c")
        .with_team("jira.allowed_sites", "jira.internal.example.com");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect_err("a shared allowlist is refused");

    assert!(matches!(error, ClientError::AllowlistFromSharedConfig));
}

#[test]
fn an_allowlist_from_a_tracked_file_is_refused() {
    let root = workspace();
    let personal = personal_config(root.path());
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "t");
    let config = FixedConfig::new()
        .with_team("jira.site", "https://jira.internal.example.com")
        .with_team("jira.email", "a@b.c")
        .with_personal("jira.allowed_sites", "jira.internal.example.com");
    let provenance = FixedProvenance::tracking(&personal);

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect_err("a committed allowlist is refused");

    assert!(matches!(
        error,
        ClientError::Credential(
            CredentialError::TokenCmdFromTrackedFile { .. }
        )
    ));
    assert!(error.to_string().contains("jira.allowed_sites"), "{error}");
}

#[test]
fn a_personal_allowlist_from_an_untracked_file_widens_the_host_set() {
    let root = workspace();
    personal_config(root.path());
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "t");
    let config = FixedConfig::new()
        .with_team("jira.site", "https://jira.internal.example.com")
        .with_team("jira.email", "a@b.c")
        .with_personal("jira.allowed_sites", "jira.internal.example.com");
    let provenance = FixedProvenance::nothing_tracked();

    let credentials = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect("the self-hosted host is admitted");

    assert_eq!(
        credentials.base.as_str(),
        "https://jira.internal.example.com/"
    );
}

#[test]
fn the_keys_the_ladder_reads_are_jiras_own() {
    let keys = token_keys().expect("the keys parse");
    assert_eq!(keys.env, "ACCELERATOR_JIRA_TOKEN");
    assert_eq!(keys.env_command, "ACCELERATOR_JIRA_TOKEN_CMD");
    assert_eq!(keys.value.to_string(), "jira.token");
    assert_eq!(keys.command.to_string(), "jira.token_cmd");
}

#[test]
fn a_loopback_site_is_unreachable_through_config_whatever_the_environment() {
    let root = workspace();
    // The bash implements its loopback escape hatch as ACCELERATOR_TEST_MODE=1
    // plus ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST. Neither exists here: the
    // test seam is the constructor, so no process state can turn the site
    // validator off.
    let environment = FixedEnvironment::empty()
        .with("ACCELERATOR_JIRA_TOKEN", "t")
        .with("ACCELERATOR_TEST_MODE", "1")
        .with(
            "ACCELERATOR_JIRA_BASE_URL_OVERRIDE_TEST",
            "http://127.0.0.1:9",
        )
        .with("ACCELERATOR_ALLOW_INSECURE_LOCAL", "1");
    let config = FixedConfig::new()
        .with_team("jira.site", "http://127.0.0.1:9")
        .with_team("jira.email", "a@b.c");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &provenance,
        root.path(),
    ))
    .expect_err("a loopback site is refused through config");

    assert!(matches!(error, ClientError::BadSite { .. }), "{error}");
}
