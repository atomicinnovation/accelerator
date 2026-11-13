//! Credential resolution: the shared ladder through Shortcut's keys, the
//! header-safety check, and the team key read from config.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod support;

use config::credentials::CredentialError;
use config::credentials::TokenSource;
use shortcut_client::auth::resolve_credentials;
use shortcut_client::auth::team_key;
use shortcut_client::auth::token_keys;
use shortcut_client::auth::validate_token;
use shortcut_client::ClientError;
use support::context;
use support::FixedConfig;
use support::FixedEnvironment;
use support::FixedTracking;
use tempfile::TempDir;

fn workspace() -> TempDir {
    TempDir::new().expect("a scratch workspace")
}

#[test]
fn the_token_resolves_from_the_environment_first() {
    let root = workspace();
    let environment = FixedEnvironment::empty()
        .with("ACCELERATOR_SHORTCUT_TOKEN", "sct_rw_ws_from_env");
    let config =
        FixedConfig::new().with_personal("shortcut.token", "sct_rw_ws_local");
    let tracking = FixedTracking::nothing_tracked();

    let credentials = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
        root.path(),
    ))
    .expect("the token resolves");

    assert_eq!(credentials.token.expose(), "sct_rw_ws_from_env");
    assert_eq!(credentials.source, TokenSource::Env);
}

#[test]
fn the_token_resolves_from_the_personal_file() {
    let root = workspace();
    let environment = FixedEnvironment::empty();
    let config =
        FixedConfig::new().with_personal("shortcut.token", "sct_rw_ws_local");
    let tracking = FixedTracking::nothing_tracked();

    let credentials = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
        root.path(),
    ))
    .expect("the token resolves");

    assert_eq!(credentials.token.expose(), "sct_rw_ws_local");
    assert_eq!(credentials.source, TokenSource::Personal);
}

#[test]
fn a_team_level_token_command_is_refused_with_its_diagnostic() {
    let root = workspace();
    let environment = FixedEnvironment::empty();
    let config =
        FixedConfig::new().with_team("shortcut.token_cmd", "printf 'no'");
    let tracking = FixedTracking::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
        root.path(),
    ))
    .expect_err("a shared token_cmd is refused");

    assert!(matches!(
        error,
        ClientError::Credential(CredentialError::Consent(_))
    ));
    assert!(
        error
            .to_string()
            .starts_with("E_CONSENT_KEY_TEAM_LEVEL: shortcut.token_cmd"),
        "{error}"
    );
}

#[test]
fn no_token_names_every_source_in_order_and_where_to_create_one() {
    let root = workspace();
    let environment = FixedEnvironment::empty();
    let config = FixedConfig::new();
    let tracking = FixedTracking::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
        root.path(),
    ))
    .expect_err("no token is a refusal");

    assert!(matches!(error, ClientError::NoToken), "{error:?}");
    let message = error.to_string();
    let positions: Vec<usize> = [
        "ACCELERATOR_SHORTCUT_TOKEN",
        "ACCELERATOR_SHORTCUT_TOKEN_CMD",
        "shortcut.token",
        "shortcut.token_cmd",
    ]
    .iter()
    .map(|source| message.find(source).expect(source))
    .collect();
    assert!(positions.is_sorted(), "{message}");
    assert!(
        message
            .contains("https://app.shortcut.com/settings/account/api-tokens"),
        "{message}"
    );
}

#[test]
fn a_token_a_header_cannot_carry_is_refused() {
    for (token, found) in [
        ("has space", "a space"),
        ("has\ttab", "a control character"),
        ("has\u{e9}accent", "a non-ASCII character"),
    ] {
        let error = validate_token(token).expect_err("refused");
        assert!(
            matches!(error, ClientError::MalformedToken { .. }),
            "{token:?}"
        );
        assert!(error.to_string().contains(found), "{error}");
    }
    assert!(validate_token("sct_rw_workspace_abc123").is_ok());
}

#[test]
fn a_token_pasted_with_its_bearer_prefix_is_refused_naming_the_prefix() {
    let error = validate_token("Bearer sct_rw_ws_abc").expect_err("refused");

    assert!(matches!(error, ClientError::MalformedToken { .. }));
    assert!(error.to_string().contains("Bearer "), "{error}");
}

#[test]
fn a_malformed_token_from_the_environment_is_refused_by_resolution() {
    let root = workspace();
    let environment = FixedEnvironment::empty()
        .with("ACCELERATOR_SHORTCUT_TOKEN", "Bearer sct_rw_ws_abc");
    let config = FixedConfig::new();
    let tracking = FixedTracking::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
        root.path(),
    ))
    .expect_err("refused");

    assert!(
        matches!(error, ClientError::MalformedToken { .. }),
        "{error}"
    );
}

#[test]
fn neither_the_token_nor_any_error_renders_the_secret() {
    let root = workspace();
    let environment = FixedEnvironment::empty()
        .with("ACCELERATOR_SHORTCUT_TOKEN", "sct_rw_ws_topsecret");
    let config = FixedConfig::new();
    let tracking = FixedTracking::nothing_tracked();

    let credentials = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
        root.path(),
    ))
    .expect("the token resolves");
    let malformed =
        validate_token("sct_rw_ws_topsecret extra").expect_err("refused");

    assert!(!format!("{credentials:?}").contains("topsecret"));
    assert!(!malformed.to_string().contains("topsecret"));
    assert!(!format!("{malformed:?}").contains("topsecret"));
}

#[test]
fn the_token_climbs_shortcuts_own_keys() {
    let keys = token_keys().expect("the keys parse");

    assert_eq!(keys.plaintext.name, "shortcut.token");
    assert_eq!(keys.command.descriptor().name, "shortcut.token_cmd");
}

#[test]
fn the_team_key_is_read_from_config() {
    let config = FixedConfig::new().with_team("shortcut.team_key", "mobile");

    assert_eq!(
        team_key(&config).expect("resolves").as_deref(),
        Some("mobile")
    );
}

#[test]
fn an_absent_team_key_is_none() {
    assert_eq!(team_key(&FixedConfig::new()).expect("resolves"), None);
}
