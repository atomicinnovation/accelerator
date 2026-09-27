//! Credential resolution: the ladder through Jira's keys, and the two values
//! the ladder does not carry — the site and the account email.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use std::path::Path;

use config::consent::{Refusal, Rejection};
use config::credentials::{CredentialError, TokenSource};
use jira_client::auth::{
    base_url, resolve_credentials, token_keys, Credentials,
};
use jira_client::ClientError;
use support::{context, FixedConfig, FixedEnvironment, FixedTracking};
use tempfile::TempDir;

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
    let tracking = FixedTracking::nothing_tracked();

    let credentials = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
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
    let tracking = FixedTracking::nothing_tracked();

    let credentials = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
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
    let tracking = FixedTracking::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
        root.path(),
    ))
    .expect_err("a shared token_cmd is refused");

    assert!(
        error
            .to_string()
            .starts_with("E_CONSENT_KEY_TEAM_LEVEL: jira.token_cmd"),
        "{error}"
    );
    assert!(rejection(error).warnings.is_empty());
}

#[test]
fn nothing_configured_is_a_missing_site() {
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
    .expect_err("no site is a refusal");

    assert!(matches!(error, ClientError::NoSite));
}

#[test]
fn a_missing_email_is_a_refusal_of_its_own() {
    let root = workspace();
    let environment =
        FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "t");
    let config = FixedConfig::new().with_team("jira.site", "tenant");
    let tracking = FixedTracking::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
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
    let tracking = FixedTracking::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
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
    let tracking = FixedTracking::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
        root.path(),
    ))
    .expect_err("a header-injecting token is refused");

    assert!(matches!(
        rejection(error).fatal,
        Refusal::MalformedToken { .. }
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

const OUTSIDE: &str = "https://jira.internal.example.com";
const ATLASSIAN: &str = "https://tenant.atlassian.net";
const HOST: &str = "jira.internal.example.com";

fn codes(refusals: &[Refusal]) -> Vec<String> {
    refusals
        .iter()
        .map(|refusal| {
            refusal
                .to_string()
                .split(':')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

fn resolve(
    config: &FixedConfig,
    tracking: &FixedTracking,
    environment: &FixedEnvironment,
) -> Result<Credentials, ClientError> {
    let root = workspace();
    resolve_credentials(&context(environment, config, tracking, root.path()))
}

fn with_token() -> FixedEnvironment {
    FixedEnvironment::empty().with("ACCELERATOR_JIRA_TOKEN", "t")
}

fn site(site: &str) -> FixedConfig {
    FixedConfig::new()
        .with_team("jira.site", site)
        .with_team("jira.email", "a@b.c")
}

fn rejection(error: ClientError) -> Rejection {
    match error {
        ClientError::Consent(rejection) => rejection,
        other => panic!("expected a consent refusal, got {other}"),
    }
}

#[test]
fn a_team_allowlist_beside_an_atlassian_site_warns_and_succeeds() {
    let config = site(ATLASSIAN).with_team("jira.allowed_sites", HOST);

    let credentials =
        resolve(&config, &FixedTracking::nothing_tracked(), &with_token())
            .expect("the default rule admits an Atlassian site");

    assert_eq!(codes(&credentials.refusals), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    assert!(credentials.notices.is_empty());
}

#[test]
fn a_team_allowlist_beside_an_outside_site_is_fatal() {
    let config = site(OUTSIDE).with_team("jira.allowed_sites", HOST);

    let error =
        resolve(&config, &FixedTracking::nothing_tracked(), &with_token())
            .expect_err("a team allowlist widens nothing");

    assert!(
        error.to_string().starts_with("E_CONSENT_KEY_TEAM_LEVEL"),
        "{error}"
    );
    let rejection = rejection(error);
    assert!(matches!(rejection.fatal, Refusal::TeamLevel { .. }));
}

#[test]
fn a_distrusted_personal_allowlist_beside_an_outside_site_is_fatal() {
    for (provenance, code) in [
        (FixedTracking::everything_tracked(), "E_CONSENT_KEY_TRACKED"),
        (FixedTracking::unknown(), "E_CONSENT_KEY_TRACKING_UNKNOWN"),
    ] {
        let config = site(OUTSIDE).with_personal("jira.allowed_sites", HOST);

        let error = resolve(&config, &provenance, &with_token())
            .expect_err("a distrusted allowlist widens nothing");

        assert_eq!(codes(&[rejection(error).fatal]), [code]);
    }
}

#[test]
fn a_distrusted_personal_allowlist_beside_an_atlassian_site_only_warns() {
    for (provenance, code) in [
        (FixedTracking::everything_tracked(), "E_CONSENT_KEY_TRACKED"),
        (FixedTracking::unknown(), "E_CONSENT_KEY_TRACKING_UNKNOWN"),
    ] {
        let config = site(ATLASSIAN).with_personal("jira.allowed_sites", HOST);

        let credentials = resolve(&config, &provenance, &with_token())
            .expect("the default rule still admits the site");

        assert_eq!(codes(&credentials.refusals), [code]);
    }
}

#[test]
fn a_tracked_personal_allowlist_is_refused_through_the_context_path() {
    let root = workspace();
    let tracking =
        FixedTracking::tracking(&root.path().join("config.local.md"));
    let config = site(OUTSIDE).with_personal("jira.allowed_sites", HOST);

    let error = resolve_credentials(&context(
        &with_token(),
        &config,
        &tracking,
        root.path(),
    ))
    .expect_err("a committed allowlist is refused");

    assert_eq!(codes(&[rejection(error).fatal]), ["E_CONSENT_KEY_TRACKED"]);
}

#[test]
fn a_personal_allowlist_from_an_untracked_file_widens_the_host_set() {
    let config = site(OUTSIDE).with_personal("jira.allowed_sites", HOST);

    let credentials =
        resolve(&config, &FixedTracking::nothing_tracked(), &with_token())
            .expect("the self-hosted host is admitted");

    assert_eq!(
        credentials.base.as_str(),
        "https://jira.internal.example.com/"
    );
    assert!(credentials.refusals.is_empty());
}

#[test]
fn the_allowlist_override_widens_the_host_set_and_notices() {
    let environment = with_token().with("ACCELERATOR_JIRA_ALLOWED_SITES", HOST);

    let credentials = resolve(
        &site(OUTSIDE),
        &FixedTracking::nothing_tracked(),
        &environment,
    )
    .expect("the override admits the host");

    assert_eq!(
        credentials.base.as_str(),
        "https://jira.internal.example.com/"
    );
    let [notice] = &credentials.notices[..] else {
        panic!("an override is noticed once");
    };
    assert_eq!(
        notice.to_string(),
        format!(
            "notice: jira.allowed_sites taken from \
             ACCELERATOR_JIRA_ALLOWED_SITES: {HOST}"
        )
    );
}

#[test]
fn the_allowlist_override_beside_a_tracked_file_never_reads_it() {
    let environment = with_token().with("ACCELERATOR_JIRA_ALLOWED_SITES", HOST);
    let config =
        site(OUTSIDE).with_personal("jira.allowed_sites", "other.example.com");

    let credentials = resolve(&config, &FixedTracking::unknown(), &environment)
        .expect("the override wins");

    assert_eq!(
        credentials.base.as_str(),
        "https://jira.internal.example.com/"
    );
    assert!(
        credentials.refusals.is_empty(),
        "the personal level was consulted: {:?}",
        credentials.refusals
    );
}

#[test]
fn an_admitted_allowlist_that_omits_the_host_keeps_its_warnings() {
    let config = site(OUTSIDE)
        .with_team("jira.allowed_sites", HOST)
        .with_personal("jira.allowed_sites", "other.example.com");

    let error =
        resolve(&config, &FixedTracking::nothing_tracked(), &with_token())
            .expect_err("the host is not listed");

    assert!(
        matches!(error.cause(), ClientError::BadSite { .. }),
        "{error}"
    );
    assert_eq!(codes(error.warnings()), ["E_CONSENT_KEY_TEAM_LEVEL"]);
}

#[test]
fn a_later_failure_still_carries_the_allowlist_warnings() {
    let config = site(ATLASSIAN).with_team("jira.allowed_sites", HOST);

    let error = resolve(
        &config,
        &FixedTracking::nothing_tracked(),
        &FixedEnvironment::empty(),
    )
    .expect_err("no token is configured");

    assert!(matches!(
        error.cause(),
        ClientError::Credential(CredentialError::NoToken { .. })
    ));
    assert_eq!(codes(error.warnings()), ["E_CONSENT_KEY_TEAM_LEVEL"]);
}

#[test]
fn an_ignored_personal_allowlist_leaves_only_the_default_rule() {
    let root = workspace();
    let config = site(OUTSIDE)
        .with_ignored_personal_file(&root.path().join("config.local.md"));

    let error = resolve_credentials(&context(
        &with_token(),
        &config,
        &FixedTracking::nothing_tracked(),
        root.path(),
    ))
    .expect_err("nothing admits an outside host");

    assert_eq!(codes(&[rejection(error).fatal]), ["E_LOCAL_PERMS_INSECURE"]);
}

#[cfg(unix)]
#[test]
fn the_allowlist_override_works_beside_an_insecure_personal_file() {
    use std::os::unix::fs::PermissionsExt as _;

    let root = workspace();
    std::fs::create_dir_all(root.path().join(".git")).unwrap();
    std::fs::create_dir_all(root.path().join(".accelerator")).unwrap();
    std::fs::write(
        root.path().join(".accelerator/config.md"),
        format!("---\njira:\n  site: {OUTSIDE}\n  email: a@b.c\n---\n"),
    )
    .unwrap();
    let personal = root.path().join(".accelerator/config.local.md");
    std::fs::write(&personal, "---\njira:\n  allowed_sites: x\n---\n").unwrap();
    std::fs::set_permissions(&personal, std::fs::Permissions::from_mode(0o644))
        .unwrap();
    let composed = config_adapters::compose(
        root.path(),
        config_adapters::LegacyPolicy::Reject,
    )
    .unwrap();
    let environment = with_token().with("ACCELERATOR_JIRA_ALLOWED_SITES", HOST);

    let credentials = resolve_credentials(&context(
        &environment,
        &composed.service,
        &FixedTracking::nothing_tracked(),
        &root.path().join(".accelerator"),
    ))
    .expect("the team config and the overrides suffice");

    assert_eq!(
        credentials.base.as_str(),
        "https://jira.internal.example.com/"
    );
    let ignored = Refusal::for_personal_file(composed.personal_file())
        .expect("the composition root reports the ignored file");
    assert!(ignored.to_string().starts_with("E_LOCAL_PERMS_INSECURE"));
}

#[test]
fn the_keys_the_ladder_reads_are_jiras_own() {
    let keys = token_keys().expect("the keys parse");
    assert_eq!(keys.plaintext.name, "jira.token");
    assert_eq!(keys.command.descriptor().name, "jira.token_cmd");
}

#[test]
fn a_loopback_site_is_unreachable_through_config_whatever_the_environment() {
    let root = workspace();
    // No environment escape hatch exists: the test seam is the constructor,
    // so no process state can turn the site validator off.
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
    let tracking = FixedTracking::nothing_tracked();

    let error = resolve_credentials(&context(
        &environment,
        &config,
        &tracking,
        root.path(),
    ))
    .expect_err("a loopback site is refused through config");

    assert!(matches!(error, ClientError::BadSite { .. }), "{error}");
}
