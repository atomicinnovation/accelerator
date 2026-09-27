//! The client's error taxonomy, and how it converts outward.
//!
//! Four error types are in play across the three layers:
//! `config::credentials::CredentialError`, this crate's [`ClientError`], the
//! port's `tracker::TrackerError`, and the composition root's
//! `SelectionError`. [`ClientError`] deliberately keeps its structure rather
//! than flattening into a string, so a caller can still tell a missing site
//! from a failed credential helper from a consent refusal.

use config::consent::Refusal;
use config::consent::Rejection;
use config::credentials::CredentialError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("E_AUTH_NO_SITE: jira.site is not configured")]
    NoSite,
    #[error("E_BAD_SITE: jira.site {site:?} is refused — {reason}")]
    BadSite { site: String, reason: String },
    #[error(
        "E_NO_PROJECT: jira.project_key is not configured — a Jira create \
         needs a project key"
    )]
    NoProject,
    #[error("E_AUTH_NO_EMAIL: jira.email is not configured")]
    NoEmail,
    #[error("{}", .0.fatal)]
    Consent(Rejection),
    /// A failure met after consent refusals that were only warnings, which
    /// the caller still reports before the failure itself.
    #[error("{error}")]
    WithWarnings {
        error: Box<ClientError>,
        warnings: Vec<Refusal>,
    },
    #[error("{0}")]
    Credential(#[from] CredentialError),
    #[error("{reason}")]
    BadJql { reason: String },
    #[error("E_BAD_IDENTIFIER: {identifier:?} is refused — {reason}")]
    BadIdentifier { identifier: String, reason: String },
    #[error("E_REQ_BAD_PATH: {path:?} rejected — {reason}")]
    BadPath { path: String, reason: String },
    #[error("E_REQ_CONNECT: {detail}")]
    Transport { detail: String },
    #[error(
        "E_REQ_OVERSIZED: the response exceeded the {limit}-byte bound \
         before it could be parsed"
    )]
    OversizedResponse { limit: usize },
    #[error("{key} could not be read: {detail}")]
    ConfigUnreadable { key: String, detail: String },
    #[error("the TLS stack could not be initialised: {detail}")]
    TlsUnavailable { detail: String },
}

impl ClientError {
    /// This failure, carrying `warnings` for the caller to report first.
    #[must_use]
    pub fn with_warnings(self, warnings: Vec<Refusal>) -> Self {
        if warnings.is_empty() {
            return self;
        }
        Self::WithWarnings {
            error: Box::new(self),
            warnings,
        }
    }

    /// The failure itself, beneath any warnings it carries.
    #[must_use]
    pub fn cause(&self) -> &Self {
        match self {
            Self::WithWarnings { error, .. } => error.cause(),
            other => other,
        }
    }

    /// Every refusal to report as a warning before this failure.
    #[must_use]
    pub fn warnings(&self) -> &[Refusal] {
        match self {
            Self::WithWarnings { warnings, .. } => warnings,
            Self::Consent(rejection) => &rejection.warnings,
            _ => &[],
        }
    }
}
