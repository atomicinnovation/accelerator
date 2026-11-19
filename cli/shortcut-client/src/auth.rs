//! Shortcut resolves a **token**, and reads the base team's key from config.
//!
//! A v4 token is bound to one workspace and one member, so there is no site
//! and no email. The workspace slug comes from whoami at discovery time, never
//! from the token string, whose shape Shortcut does not promise to keep.

use config::consent::Notice;
use config::consent::Refusal;
use config::credentials::CredentialContext;
use config::credentials::CredentialError;
use config::credentials::Secret;
use config::credentials::TokenKeys;
use config::credentials::TokenSource;
use config::ConfigAccess;
use config::Key;
use config::Resolved;

use crate::error::ClientError;

const TEAM_KEY: &str = "shortcut.team_key";
const BEARER_PREFIX: &str = "Bearer ";

/// Everything one authenticated Shortcut request needs, and the consent
/// refusals and notice met while resolving it, for the caller to report.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub token: Secret,
    pub source: TokenSource,
    pub refusals: Vec<Refusal>,
    pub notice: Option<Notice>,
}

/// The config keys Shortcut's token climbs.
///
/// # Errors
///
/// [`ClientError::ConfigUnreadable`] if the catalogue stops declaring them.
pub fn token_keys() -> Result<TokenKeys, ClientError> {
    TokenKeys::declared("shortcut.token", "shortcut.token_cmd").map_err(
        |error| ClientError::ConfigUnreadable {
            key: "shortcut.token".to_owned(),
            detail: error.to_string(),
        },
    )
}

/// # Errors
///
/// [`ClientError::NoToken`] when nothing is configured,
/// [`ClientError::MalformedToken`] for a token a header cannot carry, and
/// [`ClientError::Credential`] for a consent refusal or an unreadable level.
pub fn resolve_credentials(
    context: &CredentialContext<'_>,
) -> Result<Credentials, ClientError> {
    let resolved = config::credentials::resolve_token(context, &token_keys()?)
        .map_err(|error| match error {
            CredentialError::NoToken { .. } => ClientError::NoToken,
            other => ClientError::Credential(other),
        })?;
    validate_token(resolved.value.expose())?;
    Ok(Credentials {
        token: resolved.value,
        source: resolved.source,
        refusals: resolved.refusals,
        notice: resolved.notice,
    })
}

/// Refuses a token that cannot travel as `Authorization: Bearer <token>`.
///
/// The message names what was found, never the token itself.
///
/// # Errors
///
/// [`ClientError::MalformedToken`] naming the offending character.
pub fn validate_token(token: &str) -> Result<(), ClientError> {
    let malformed = |found: &str| ClientError::MalformedToken {
        found: found.to_owned(),
    };
    if token.starts_with(BEARER_PREFIX) {
        return Err(malformed(
            "the \"Bearer \" prefix; give the token alone, and the client \
             adds the prefix",
        ));
    }
    for character in token.chars() {
        if !character.is_ascii() {
            return Err(malformed("a non-ASCII character"));
        }
        if character.is_ascii_control() {
            return Err(malformed("a control character"));
        }
        if character == ' ' {
            return Err(malformed("a space"));
        }
    }
    Ok(())
}

/// The base team's key, as `shortcut.team_key` names it.
///
/// # Errors
///
/// [`ClientError::ConfigUnreadable`] when a config level cannot be read.
pub fn team_key(
    config: &dyn ConfigAccess,
) -> Result<Option<String>, ClientError> {
    let unreadable = |detail: String| ClientError::ConfigUnreadable {
        key: TEAM_KEY.to_owned(),
        detail,
    };
    let key =
        Key::parse(TEAM_KEY).map_err(|error| unreadable(error.to_string()))?;
    let resolved = config
        .get(&key, None)
        .map_err(|error| unreadable(error.to_string()))?;
    Ok(match resolved {
        Resolved::Found(value) => {
            let rendered = config::render_value(&value);
            (!rendered.is_empty()).then_some(rendered)
        }
        Resolved::Absent => None,
    })
}
