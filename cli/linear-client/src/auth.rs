//! Linear resolves a **token and a team**, and nothing else.
//!
//! A personal API key is user-scoped and grants access to every team the user
//! belongs to, so there is no site and no email.
//!
//! The team is the one value that was historically not stored in config.
//! `catalogue.json` carries `.team.id`, so an already-onboarded repository can
//! have a populated catalogue and no `linear.team_id`. Requiring the key
//! outright would report such a repository as an unconfigured tracker, and
//! would leave one fact with two disagreeing sources of truth. Resolution is
//! therefore the key first, then the catalogue.

use std::path::Path;

use config::ConfigAccess;
use config::Key;
use config::Resolved;
use tracker_support::CredentialContext;
use tracker_support::Secret;
use tracker_support::TokenKeys;
use tracker_support::TokenSource;

use crate::error::ClientError;

/// Everything one authenticated Linear request needs.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub token: Secret,
    pub team_id: String,
    pub source: TokenSource,
}

/// Where a team id came from, asserted by the resolution tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamSource {
    ConfiguredKey,
    Catalogue,
}

/// The environment names and config keys Linear's token climbs.
///
/// # Errors
///
/// [`ClientError::ConfigUnreadable`] if a key spelling stops parsing.
pub fn token_keys() -> Result<TokenKeys, ClientError> {
    Ok(TokenKeys {
        env: "ACCELERATOR_LINEAR_TOKEN",
        env_command: "ACCELERATOR_LINEAR_TOKEN_CMD",
        value: key("linear.token")?,
        command: key("linear.token_cmd")?,
    })
}

/// Resolves the token and team a Linear client authenticates with.
///
/// `integrations_root` is `paths.integrations`; the catalogue is read from
/// `<root>/linear/catalogue.json`.
///
/// # Errors
///
/// [`ClientError`] naming the value that is missing or refused.
pub fn resolve_credentials(
    context: &CredentialContext<'_>,
    integrations_root: &Path,
) -> Result<Credentials, ClientError> {
    let resolved = tracker_support::resolve_token(context, &token_keys()?)?;
    validate_token(resolved.value.expose())?;
    let (team_id, _) = resolve_team(context.config, integrations_root)?;
    Ok(Credentials {
        token: resolved.value,
        team_id,
        source: resolved.source,
    })
}

/// The team id, and which source produced it.
///
/// # Errors
///
/// [`ClientError::NoTeam`] when neither the key nor the catalogue names one.
pub fn resolve_team(
    config: &dyn ConfigAccess,
    integrations_root: &Path,
) -> Result<(String, TeamSource), ClientError> {
    if let Some(configured) = configured(config, "linear.team_id")? {
        return Ok((configured, TeamSource::ConfiguredKey));
    }
    catalogue_team(integrations_root)
        .map(|team| (team, TeamSource::Catalogue))
        .ok_or(ClientError::NoTeam)
}

/// The team **key** — the `ENG` in `ENG-42` — which is what decides whether a
/// requested identifier was ever in this client's scope. Only the catalogue
/// carries it; `linear.team_id` is the UUID alone.
#[must_use]
pub fn catalogue_team_key(integrations_root: &Path) -> Option<String> {
    catalogue_field(integrations_root, "/team/key")
}

fn catalogue_team(integrations_root: &Path) -> Option<String> {
    catalogue_field(integrations_root, "/team/id")
}

fn catalogue_field(integrations_root: &Path, pointer: &str) -> Option<String> {
    let catalogue = integrations_root.join("linear/catalogue.json");
    let raw = std::fs::read_to_string(catalogue).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
    parsed
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// The malformed-token check.
///
/// A control byte is already refused by the shared ladder; the double-quote and
/// backslash are Linear's own additions. They are reproduced rather than
/// dropped: the code they raise is re-exited verbatim by the transport and
/// appears in the update mapper's retryable clause.
///
/// # Errors
///
/// [`ClientError::MalformedToken`] naming the offending byte.
pub fn validate_token(token: &str) -> Result<(), ClientError> {
    if token.chars().any(char::is_control) {
        return Err(ClientError::MalformedToken {
            found: "a control character or newline".to_owned(),
        });
    }
    if token.contains('"') {
        return Err(ClientError::MalformedToken {
            found: "a double-quote".to_owned(),
        });
    }
    if token.contains('\\') {
        return Err(ClientError::MalformedToken {
            found: "a backslash".to_owned(),
        });
    }
    Ok(())
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
    Ok(match resolved {
        Resolved::Found(value) => {
            let rendered = config::render_value(&value);
            (!rendered.is_empty()).then_some(rendered)
        }
        Resolved::Absent => None,
    })
}

fn key(name: &str) -> Result<Key, ClientError> {
    Key::parse(name).map_err(|error| ClientError::ConfigUnreadable {
        key: name.to_owned(),
        detail: error.to_string(),
    })
}

/// The frontmatter-safety check every identifier entering a request goes
/// through, wrapped into this crate's error so a caller can report which value
/// was refused.
///
/// The rule itself lives in `tracker-support`, so both providers share one
/// implementation and one fixture rather than two copies that drift.
///
/// # Errors
///
/// [`ClientError::BadIdentifier`] naming the identifier and the property that
/// failed.
pub fn check_identifier(identifier: &str) -> Result<(), ClientError> {
    tracker_support::identifier_is_safe(identifier).map_err(|refusal| {
        ClientError::BadIdentifier {
            identifier: identifier.to_owned(),
            reason: refusal.to_string(),
        }
    })
}
