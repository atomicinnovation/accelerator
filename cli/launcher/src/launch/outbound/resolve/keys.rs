//! The trusted release public key(s) and in-process minisign verification.
//!
//! Verify-any-of over a small key set, so rotation has an overlap window.

use minisign_verify::{PublicKey, Signature};

use crate::launch::core::ResolutionError;

/// The release public key `build.rs` copies from the one committed
/// `keys/accelerator-release.pub` (the same file the bootstrap ships).
pub const EMBEDDED_RELEASE_KEY: &str =
    include_str!(concat!(env!("OUT_DIR"), "/release.pub"));

/// A set of trusted public keys; a signature is accepted if any key verifies it.
pub struct TrustedKeys {
    keys: Vec<PublicKey>,
}

impl TrustedKeys {
    /// Parse minisign `.pub` file contents (comment line + base64 line each).
    ///
    /// # Errors
    ///
    /// [`ResolutionError::CacheRootUnavailable`] if a key cannot be parsed.
    pub fn from_public_key_files(
        contents: &[&str],
    ) -> Result<Self, ResolutionError> {
        let mut keys = Vec::with_capacity(contents.len());
        for content in contents {
            let base64 = content
                .lines()
                .find(|line| {
                    !line.trim_start().starts_with("untrusted comment")
                })
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .ok_or_else(|| ResolutionError::CacheRootUnavailable {
                    detail: "trusted public key has no key line".to_owned(),
                })?;
            let key = PublicKey::from_base64(base64).map_err(|error| {
                ResolutionError::CacheRootUnavailable {
                    detail: format!("invalid trusted public key: {error}"),
                }
            })?;
            keys.push(key);
        }
        Ok(Self { keys })
    }

    /// The production trust root: just the embedded release key.
    ///
    /// # Errors
    ///
    /// If the embedded key cannot be parsed.
    pub fn embedded() -> Result<Self, ResolutionError> {
        Self::from_public_key_files(&[EMBEDDED_RELEASE_KEY])
    }

    /// Whether `signature` verifies `data` under any trusted key; any
    /// parse/verify failure is a non-match, never a panic.
    #[must_use]
    pub fn verifies(&self, data: &[u8], signature: &str) -> bool {
        let Ok(parsed) = Signature::decode(signature) else {
            return false;
        };
        self.keys
            .iter()
            .any(|key| key.verify(data, &parsed, false).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::{TrustedKeys, EMBEDDED_RELEASE_KEY};

    #[test]
    fn the_embedded_release_key_parses() {
        assert!(TrustedKeys::embedded().is_ok());
    }

    #[test]
    fn the_embedded_key_is_not_a_bare_placeholder() {
        assert!(EMBEDDED_RELEASE_KEY.contains("untrusted comment"));
        assert!(EMBEDDED_RELEASE_KEY.lines().count() >= 2);
    }
}
