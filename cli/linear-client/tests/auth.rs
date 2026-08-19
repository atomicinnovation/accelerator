//! Credential resolution: the shared ladder through Linear's keys, the
//! malformed-token check the bash carries, and the two sources a team id can
//! come from.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod support;

use std::path::Path;

use linear_client::auth::{
    resolve_credentials, resolve_team, token_keys, validate_token, TeamSource,
};
use linear_client::ClientError;
use support::{context, FixedConfig, FixedEnvironment, FixedProvenance};
use tempfile::TempDir;
use tracker_support::{CredentialError, TokenSource};

const TEAM: &str = "5c9f2a1b-0000-4000-8000-000000000001";

fn workspace() -> TempDir {
    TempDir::new().expect("a scratch workspace")
}

fn with_catalogue(root: &Path, team: &str) -> std::path::PathBuf {
    let integrations = root.join("integrations");
    std::fs::create_dir_all(integrations.join("linear"))
        .expect("the catalogue directory");
    std::fs::write(
        integrations.join("linear/catalogue.json"),
        format!("{{\"team\":{{\"id\":\"{team}\",\"key\":\"ENG\"}}}}"),
    )
    .expect("write the catalogue");
    integrations
}

#[test]
fn the_token_and_team_resolve_together() {
    let root = workspace();
    let integrations = with_catalogue(root.path(), TEAM);
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_LINEAR_TOKEN", "lin_api_x");
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();

    let credentials = resolve_credentials(
        &context(&environment, &config, &provenance, root.path()),
        &integrations,
    )
    .expect("both values resolve");

    assert_eq!(credentials.token.expose(), "lin_api_x");
    assert_eq!(credentials.team_id, TEAM);
    assert_eq!(credentials.source, TokenSource::Env);
}

#[test]
fn the_configured_key_outranks_the_catalogue() {
    let root = workspace();
    let integrations = with_catalogue(root.path(), "from-catalogue");
    let config = FixedConfig::new().with_team("linear.team_id", "from-key");

    let (team, source) =
        resolve_team(&config, &integrations).expect("the key resolves");

    assert_eq!(team, "from-key");
    assert_eq!(source, TeamSource::ConfiguredKey);
}

#[test]
fn an_already_onboarded_repository_resolves_from_the_catalogue() {
    // No linear.team_id set, representing an already-onboarded repository.
    // Requiring the key would report this as unconfigured.
    let root = workspace();
    let integrations = with_catalogue(root.path(), TEAM);
    let config = FixedConfig::new();

    let (team, source) =
        resolve_team(&config, &integrations).expect("the catalogue resolves");

    assert_eq!(team, TEAM);
    assert_eq!(source, TeamSource::Catalogue);
}

#[test]
fn neither_source_naming_a_team_is_a_refusal() {
    let root = workspace();
    let config = FixedConfig::new();

    let error = resolve_team(&config, &root.path().join("integrations"))
        .expect_err("no team is a refusal");

    assert!(matches!(error, ClientError::NoTeam));
    assert!(error.to_string().contains("linear.team_id"), "{error}");
}

#[test]
fn a_catalogue_with_no_team_id_falls_through_to_the_refusal() {
    let root = workspace();
    let integrations = root.path().join("integrations");
    std::fs::create_dir_all(integrations.join("linear")).expect("the dir");
    std::fs::write(integrations.join("linear/catalogue.json"), "{\"team\":{}}")
        .expect("write the catalogue");
    let config = FixedConfig::new();

    assert!(matches!(
        resolve_team(&config, &integrations),
        Err(ClientError::NoTeam)
    ));
}

#[test]
fn a_team_level_token_command_is_refused_with_its_diagnostic() {
    let root = workspace();
    let integrations = with_catalogue(root.path(), TEAM);
    let environment = FixedEnvironment::empty();
    let config =
        FixedConfig::new().with_team("linear.token_cmd", "printf 'no'");
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_credentials(
        &context(&environment, &config, &provenance, root.path()),
        &integrations,
    )
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
fn a_malformed_token_is_refused() {
    for (token, found) in [
        ("has\ttab", "a control character or newline"),
        ("has\nnewline", "a control character or newline"),
        ("has\"quote", "a double-quote"),
        ("has\\backslash", "a backslash"),
    ] {
        let error = validate_token(token)
            .expect_err("a curl-directive-corrupting token is refused");
        assert!(
            matches!(error, ClientError::MalformedToken { .. }),
            "{token:?}"
        );
        assert!(error.to_string().contains(found), "{error}");
    }
    assert!(validate_token("lin_api_abc123").is_ok());
}

#[test]
fn a_malformed_token_from_the_environment_is_refused_by_resolution() {
    let root = workspace();
    let integrations = with_catalogue(root.path(), TEAM);
    let environment = FixedEnvironment::empty()
        .with("ACCELERATOR_LINEAR_TOKEN", "quote\"inside");
    let config = FixedConfig::new();
    let provenance = FixedProvenance::nothing_tracked();

    let error = resolve_credentials(
        &context(&environment, &config, &provenance, root.path()),
        &integrations,
    )
    .expect_err("the malformed token is refused");

    assert!(
        matches!(error, ClientError::MalformedToken { .. }),
        "{error}"
    );
}

#[test]
fn there_is_no_site_or_email_in_linears_auth_band() {
    let keys = token_keys().expect("the keys parse");

    assert_eq!(keys.env, "ACCELERATOR_LINEAR_TOKEN");
    assert_eq!(keys.env_command, "ACCELERATOR_LINEAR_TOKEN_CMD");
    assert_eq!(keys.value.to_string(), "linear.token");
    assert_eq!(keys.command.to_string(), "linear.token_cmd");
}
