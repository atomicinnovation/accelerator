//! Jira needs three values, not one: a site, an account email and a token.
//!
//! The token climbs `config::credentials`'s five-rung ladder; this module
//! supplies the keys and validates the other two. `jira.site` is where the
//! token is *sent*, so it is validated as a credential destination: absolute
//! `https`, no userinfo, no query, no fragment, default port, and a host
//! matching `*.atlassian.net` at a label boundary or listed exactly in
//! `jira.allowed_sites`. The repository proposes the site; only the user may
//! widen where the token goes, so the allowlist is a consent key.
//!
//! Suffix matching would accept `atlassian.net.evil.com` and
//! `evil-atlassian.net`, so the host must match at a label boundary. Hosts
//! are compared after `url`'s own IDNA normalisation, so a homoglyph form is
//! compared in its punycode shape.
//!
//! A bare Cloud subdomain is accepted as `jira.site`, expanded to
//! `https://<site>.atlassian.net` — every already-working configuration keeps
//! working — and the absolute-URL form exists for the self-hosted tenants
//! `jira.allowed_sites` is for.

use config::consent;
use config::consent::ConsentKey;
use config::consent::Notice;
use config::consent::Refusal;
use config::consent::Usable;
use config::credentials::CredentialContext;
use config::credentials::Secret;
use config::credentials::TokenKeys;
use config::credentials::TokenSource;
use config::ConfigAccess;
use config::Key;
use config::Resolved;
use reqwest::Url;

use crate::error::ClientError;

const CLOUD_SUFFIX: &str = ".atlassian.net";

const ALLOWED_SITES: &str = "jira.allowed_sites";

/// Everything one authenticated Jira request needs, and the consent refusals
/// and notice met while resolving it, for the caller to report.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub base: Url,
    pub email: String,
    pub token: Secret,
    pub source: TokenSource,
    pub refusals: Vec<Refusal>,
    pub notice: Option<Notice>,
}

/// The environment names and config keys Jira's token climbs.
///
/// # Errors
///
/// [`ClientError::ConfigUnreadable`] if a key spelling stops parsing.
pub fn token_keys() -> Result<TokenKeys, ClientError> {
    Ok(TokenKeys {
        env: "ACCELERATOR_JIRA_TOKEN",
        env_command: "ACCELERATOR_JIRA_TOKEN_CMD",
        value: key("jira.token")?,
        command: key("jira.token_cmd")?,
    })
}

/// Resolves the site, email and token a Jira client authenticates with.
///
/// # Errors
///
/// [`ClientError`] naming the value that is missing or refused.
pub fn resolve_credentials(
    context: &CredentialContext<'_>,
) -> Result<Credentials, ClientError> {
    let site =
        configured(context.config, "jira.site")?.ok_or(ClientError::NoSite)?;
    let destination = admitted_destination(context, &site)?;
    let warned =
        |error: ClientError| error.with_warnings(destination.warnings.clone());
    let email = configured(context.config, "jira.email")
        .and_then(|email| email.ok_or(ClientError::NoEmail))
        .map_err(warned)?;
    let resolved = token_keys()
        .and_then(|keys| {
            Ok(config::credentials::resolve_token(context, &keys)?)
        })
        .map_err(warned)?;

    Ok(Credentials {
        base: destination.base,
        email,
        token: resolved.value,
        source: resolved.source,
        refusals: destination.warnings,
        notice: destination.notice,
    })
}

/// The base URL the token may be sent to, and the allowlist's warnings.
struct Destination {
    base: Url,
    warnings: Vec<Refusal>,
    notice: Option<Notice>,
}

/// Admits `site` by the default `*.atlassian.net` rule or a consented
/// allowlist. A refused allowlist is fatal only when the default rule does
/// not admit the site.
fn admitted_destination(
    context: &CredentialContext<'_>,
    site: &str,
) -> Result<Destination, ClientError> {
    let key = ConsentKey::declared(ALLOWED_SITES)
        .map_err(|error| unreadable(ALLOWED_SITES, &error))?;
    let consented = consent::resolve(&context.provenance(), &key)
        .map_err(|aborted| {
            unreadable(ALLOWED_SITES, &aborted.error)
                .with_warnings(aborted.warnings)
        })?
        .map(|rendered| allowlist(&rendered));
    let default_admits = base_url(site, &[]).is_ok();
    match consented.or_fallback(default_admits.then(Vec::new)) {
        Usable::Value {
            value,
            warnings,
            notice,
        } => match base_url(site, &value) {
            Ok(base) => Ok(Destination {
                base,
                warnings,
                notice,
            }),
            Err(error) => Err(error.with_warnings(warnings)),
        },
        Usable::Refused(rejection) => Err(ClientError::Consent(rejection)),
        Usable::Absent => base_url(site, &[]).map(|base| Destination {
            base,
            warnings: Vec::new(),
            notice: None,
        }),
    }
}

fn allowlist(rendered: &str) -> Vec<String> {
    rendered
        .trim_matches(|character| character == '[' || character == ']')
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// Resolves `jira.site` to the base URL requests are sent to.
///
/// # Errors
///
/// [`ClientError::BadSite`] when the value is not a Cloud subdomain and not
/// an admissible absolute `https` URL.
pub fn base_url(site: &str, allowed: &[String]) -> Result<Url, ClientError> {
    if is_cloud_subdomain(site) {
        return Url::parse(&format!("https://{site}{CLOUD_SUFFIX}")).map_err(
            |error| ClientError::BadSite {
                site: site.to_owned(),
                reason: error.to_string(),
            },
        );
    }

    let refuse = |reason: &str| ClientError::BadSite {
        site: site.to_owned(),
        reason: reason.to_owned(),
    };
    let url = Url::parse(site).map_err(|_| {
        refuse("neither a Cloud subdomain nor an absolute https URL")
    })?;
    if url.scheme() != "https" {
        return Err(refuse("the scheme is not https"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(refuse("the URL carries userinfo"));
    }
    if url.query().is_some() {
        return Err(refuse("the URL carries a query string"));
    }
    if url.fragment().is_some() {
        return Err(refuse("the URL carries a fragment"));
    }
    if url.port().is_some() {
        return Err(refuse("only the default https port is accepted"));
    }
    let host = url
        .host_str()
        .ok_or_else(|| refuse("no host"))?
        .to_ascii_lowercase();
    if !host_is_admissible(&host, allowed) {
        return Err(refuse(
            "the host is outside *.atlassian.net and not listed in \
             jira.allowed_sites",
        ));
    }
    Ok(url)
}

/// Whether a host may receive the token: a label-boundary match on
/// `*.atlassian.net`, or an exact entry in the allowlist.
fn host_is_admissible(host: &str, allowed: &[String]) -> bool {
    if let Some(tenant) = host.strip_suffix(CLOUD_SUFFIX) {
        if !tenant.is_empty() && !tenant.ends_with('.') {
            return true;
        }
    }
    allowed.iter().any(|entry| entry == host)
}

fn is_cloud_subdomain(site: &str) -> bool {
    let length = site.len();
    (3..=63).contains(&length)
        && site.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '-'
        })
        && !site.starts_with('-')
        && !site.ends_with('-')
}

/// The Jira project a `create` targets, from the integration-owned
/// `jira.project_key` scope key.
///
/// The deprecated `work.default_project_code` still resolves through the alias
/// during the removal window, gated on `work.integration: jira`, emitting a
/// deprecation warning.
///
/// # Errors
///
/// [`ClientError::NoProject`] when the scope key is unset;
/// [`ClientError::ConfigUnreadable`] when a config level cannot be read.
pub fn project_code(config: &dyn ConfigAccess) -> Result<String, ClientError> {
    let resolved = config::resolve_with_deprecated_fallback(
        config,
        "jira.project_key",
        "work.default_project_code",
        Some("jira"),
    )
    .map_err(|error| ClientError::ConfigUnreadable {
        key: "jira.project_key".to_owned(),
        detail: error.to_string(),
    })?;
    config::emit_deprecation_once(
        "work.default_project_code",
        resolved.deprecation.as_deref(),
    );
    resolved.value.ok_or(ClientError::NoProject)
}

fn configured(
    config: &dyn ConfigAccess,
    name: &str,
) -> Result<Option<String>, ClientError> {
    let key = key(name)?;
    let resolved = config.get(&key, None).map_err(|error| {
        ClientError::ConfigUnreadable {
            key: name.to_owned(),
            detail: error.to_string(),
        }
    })?;
    Ok(rendered(&resolved))
}

fn rendered(resolved: &Resolved) -> Option<String> {
    match resolved {
        Resolved::Found(value) => {
            let rendered = config::render_value(value);
            (!rendered.is_empty()).then_some(rendered)
        }
        Resolved::Absent => None,
    }
}

fn key(name: &str) -> Result<Key, ClientError> {
    Key::parse(name).map_err(|error| unreadable(name, &error))
}

fn unreadable(name: &str, error: &config::ConfigError) -> ClientError {
    ClientError::ConfigUnreadable {
        key: name.to_owned(),
        detail: error.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod project_code_tests {
    use std::collections::HashMap;

    use config::{
        ConfigAccess, ConfigError, Key, Level, Resolved, Scalar, Value,
    };

    use super::project_code;
    use crate::error::ClientError;

    struct FakeConfig(HashMap<String, String>);

    impl FakeConfig {
        fn with(pairs: &[(&str, &str)]) -> Self {
            Self(
                pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
            )
        }
    }

    impl ConfigAccess for FakeConfig {
        fn get(
            &self,
            key: &Key,
            _level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            Ok(self
                .0
                .get(&key.to_string())
                .map_or(Resolved::Absent, |value| {
                    Resolved::Found(Value::Scalar(Scalar::String(
                        value.clone(),
                    )))
                }))
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            Err(ConfigError::Invalid {
                detail: "set unsupported in the fake config".to_owned(),
            })
        }
    }

    #[test]
    fn resolves_from_jira_project_key_with_no_work_config() {
        let config = FakeConfig::with(&[("jira.project_key", "OPS")]);
        assert_eq!(project_code(&config).unwrap(), "OPS");
    }

    #[test]
    fn falls_back_to_the_legacy_key_when_integration_is_jira() {
        let config = FakeConfig::with(&[
            ("work.default_project_code", "PP"),
            ("work.integration", "jira"),
        ]);
        assert_eq!(project_code(&config).unwrap(), "PP");
    }

    #[test]
    fn does_not_claim_the_legacy_key_when_integration_is_linear() {
        let config = FakeConfig::with(&[
            ("work.default_project_code", "PP"),
            ("work.integration", "linear"),
        ]);
        assert!(matches!(project_code(&config), Err(ClientError::NoProject)));
    }
}
