//! The client's error taxonomy.
//!
//! Linear's auth band differs from Jira's: there is no site and no email, so
//! no `E_AUTH_NO_SITE` (27) and no `E_AUTH_NO_EMAIL` (28). Linear's **27 is
//! `E_TOKEN_MALFORMED`** — a token that would corrupt the `curl --config -`
//! directive the bash writes. The check is reproduced because codes 25, 27 and
//! 29 are re-exited verbatim by `linear-graphql.sh:481-489` and appear in
//! `_wiur_map_linear`'s retryable clause.

use thiserror::Error;
use tracker_support::CredentialError;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("{0}")]
    Credential(#[from] CredentialError),
    #[error("E_TOKEN_MALFORMED: the token contains {found}")]
    MalformedToken { found: String },
    #[error(
        "E_CREATE_NO_CATALOGUE: no Linear team configured — set \
         linear.team_id, or run /init-linear to write catalogue.json"
    )]
    NoTeam,
    #[error(
        "E_SEARCH_UNKNOWN_STATE: no workflow state named {name:?} in the \
         catalogue — run /init-linear to refresh it"
    )]
    UnknownState { name: String },
    #[error("E_BAD_IDENTIFIER: {identifier:?} is refused — {reason}")]
    BadIdentifier { identifier: String, reason: String },
    #[error("E_GQL_CONNECT: {detail}")]
    Transport { detail: String },
    #[error(
        "E_GQL_OVERSIZED: the response exceeded the {limit}-byte bound before \
         it could be parsed"
    )]
    OversizedResponse { limit: usize },
    #[error("{key} could not be read: {detail}")]
    ConfigUnreadable { key: String, detail: String },
    #[error("the TLS stack could not be initialised: {detail}")]
    TlsUnavailable { detail: String },
}
