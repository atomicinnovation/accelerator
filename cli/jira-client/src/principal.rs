//! Resolving an assignee or reporter token to a Jira accountId, the Rust twin
//! of the retiring flows' principal resolution.
//!
//! `@me` resolves through the cached accountId (`site.json`); a raw accountId
//! passes through if it is safe. An email — anything carrying `@` — is refused
//! rather than resolved: Jira's accountId is opaque, and silently querying by
//! email would be a different, unaudited lookup.

/// Why a principal token could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalError {
    /// `@me` was named but no cached accountId is available (`site.json`
    /// absent).
    NoSiteCache,
    /// The token is an email or otherwise not a bare accountId.
    BadPrincipal { token: String },
}

/// Resolves a principal token to an accountId. `@me` yields the cached id (or
/// [`PrincipalError::NoSiteCache`] when there is none); a safe raw id passes
/// through; an email or malformed token is refused.
///
/// # Errors
///
/// [`PrincipalError`] for an unresolvable `@me` or a non-accountId token.
pub fn resolve(
    token: &str,
    account_id: Option<&str>,
) -> Result<String, PrincipalError> {
    if token == "@me" {
        return account_id
            .map(str::to_owned)
            .ok_or(PrincipalError::NoSiteCache);
    }
    if is_account_id(token) {
        Ok(token.to_owned())
    } else {
        Err(PrincipalError::BadPrincipal {
            token: token.to_owned(),
        })
    }
}

/// The bash `^[A-Za-z0-9:_-]+$` shape — an opaque Jira accountId, never an
/// email.
fn is_account_id(token: &str) -> bool {
    !token.is_empty()
        && token.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-')
        })
}
