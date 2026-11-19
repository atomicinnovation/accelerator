use config::consent::Refusal;
use config::credentials::CredentialError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("{0}")]
    Credential(CredentialError),
    #[error(
        "E_NO_TOKEN: no Shortcut token found. Set ACCELERATOR_SHORTCUT_TOKEN \
         or ACCELERATOR_SHORTCUT_TOKEN_CMD in the environment, or \
         shortcut.token or shortcut.token_cmd in \
         .accelerator/config.local.md. Create a token at \
         https://app.shortcut.com/settings/account/api-tokens"
    )]
    NoToken,
    #[error("E_TOKEN_MALFORMED: the token contains {found}")]
    MalformedToken { found: String },
    #[error("E_SC_CONNECT: {detail}")]
    NotSent { detail: String },
    #[error("E_SC_CONNECT: {detail}")]
    Transport { detail: String },
    #[error("E_SC_INVALID: {detail}")]
    RequestInvalid { detail: String },
    #[error(
        "E_SC_OVERSIZED: the response exceeded the {limit}-byte bound before \
         it could be parsed"
    )]
    OversizedResponse { limit: usize },
    #[error(
        "E_SC_FOREIGN_PAGE: Shortcut returned a next page outside its own \
         origin or the v4 API, so it was not followed: {url}"
    )]
    ForeignPage { url: String },
    #[error("the TLS stack could not be initialised: {detail}")]
    TlsUnavailable { detail: String },
    #[error(
        "E_NO_CATALOGUE: there is no Shortcut catalogue.json. Pull the one a \
         teammate committed, or run /accelerator:init-shortcut"
    )]
    NoCatalogue,
    #[error("{0}")]
    Catalogue(crate::cache::CacheError),
    #[error("{key} could not be read: {detail}")]
    ConfigUnreadable { key: String, detail: String },
}

impl ClientError {
    /// Every refusal to report as a warning before this failure.
    #[must_use]
    pub fn warnings(&self) -> &[Refusal] {
        match self {
            Self::Credential(credential) => credential.warnings(),
            _ => &[],
        }
    }
}
