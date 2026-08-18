//! Jira needs three values, not one: a site, an account email and a token.
//!
//! The token climbs `tracker_support`'s five-rung ladder; this module supplies
//! the keys and validates the other two. `jira.site` is where the token is
//! *sent*, so it is validated as a credential destination: absolute `https`,
//! no userinfo, no query, no fragment, default port, and a host matching
//! `*.atlassian.net` at a label boundary or listed exactly in
//! `jira.allowed_sites`.
//!
//! Suffix matching would accept `atlassian.net.evil.com` and
//! `evil-atlassian.net`, which is the defect the Linear upload allowlist is
//! written to avoid. Hosts are compared after `url`'s own IDNA
//! normalisation, so a homoglyph form is compared in its punycode shape.
//!
//! The bash writes `jira.site` as a bare Cloud subdomain and builds
//! `https://<site>.atlassian.net` from it (`jira-request.sh:240`). That form
//! is still accepted — every already-working configuration keeps working —
//! and the absolute-URL form exists for the self-hosted tenants
//! `jira.allowed_sites` is for.

use config::ConfigAccess;
use config::Key;
use config::Level;
use config::Resolved;
use reqwest::Url;
use tracker_support::credentials::refuse_tracked_source;
use tracker_support::CredentialContext;
use tracker_support::Secret;
use tracker_support::TokenKeys;
use tracker_support::TokenSource;

use crate::error::ClientError;

const CLOUD_SUFFIX: &str = ".atlassian.net";

/// Everything one authenticated Jira request needs.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub base: Url,
    pub email: String,
    pub token: Secret,
    pub source: TokenSource,
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
    let allowed = allowed_sites(context)?;
    let base = base_url(&site, &allowed)?;
    let email = configured(context.config, "jira.email")?
        .ok_or(ClientError::NoEmail)?;
    let resolved = tracker_support::resolve_token(context, &token_keys()?)?;

    Ok(Credentials {
        base,
        email,
        token: resolved.value,
        source: resolved.source,
    })
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

/// Reads `jira.allowed_sites`, which widens the set of hosts the token may be
/// sent to and is therefore held to the same trust rules as `token_cmd`.
fn allowed_sites(
    context: &CredentialContext<'_>,
) -> Result<Vec<String>, ClientError> {
    let key = key("jira.allowed_sites")?;
    if level_value(context.config, &key, Level::Team)?.is_some() {
        return Err(ClientError::AllowlistFromSharedConfig);
    }
    let Some(rendered) = level_value(context.config, &key, Level::Personal)?
    else {
        return Ok(Vec::new());
    };
    refuse_tracked_source(
        context.provenance,
        &context.personal_config,
        "jira.allowed_sites",
    )?;
    Ok(rendered
        .trim_matches(|character| character == '[' || character == ']')
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_ascii_lowercase)
        .collect())
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

fn level_value(
    config: &dyn ConfigAccess,
    key: &Key,
    level: Level,
) -> Result<Option<String>, ClientError> {
    let resolved = config.get(key, Some(level)).map_err(|error| {
        ClientError::ConfigUnreadable {
            key: key.to_string(),
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
    Key::parse(name).map_err(|error| ClientError::ConfigUnreadable {
        key: name.to_owned(),
        detail: error.to_string(),
    })
}
