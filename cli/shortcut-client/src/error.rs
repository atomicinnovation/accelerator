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
