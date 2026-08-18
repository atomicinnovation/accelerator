//! The client's error taxonomy, and how it converts outward.
//!
//! Four error types are in play across the three layers:
//! `tracker_support::CredentialError`, this crate's [`ClientError`], the
//! port's `tracker::TrackerError`, and the composition root's
//! `SelectionError`. [`ClientError`] deliberately keeps its structure rather
//! than flattening into a string, so a caller can still tell a missing site
//! from a failed credential helper from a shared-config refusal.

use thiserror::Error;
use tracker_support::CredentialError;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("E_AUTH_NO_SITE: jira.site is not configured")]
    NoSite,
    #[error("E_BAD_SITE: jira.site {site:?} is refused — {reason}")]
    BadSite { site: String, reason: String },
    #[error("E_AUTH_NO_EMAIL: jira.email is not configured")]
    NoEmail,
    #[error(
        "E_ALLOWED_SITES_FROM_SHARED_CONFIG: jira.allowed_sites in \
         config.md is refused — a credential destination a clone would \
         inherit; move it to config.local.md"
    )]
    AllowlistFromSharedConfig,
    #[error("{0}")]
    Credential(#[from] CredentialError),
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
