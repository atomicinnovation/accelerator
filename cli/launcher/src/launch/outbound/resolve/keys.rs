//! The trusted release public key(s) and in-process minisign verification.
//!
//! The minisign signature is the security boundary (sha256 is only a corruption
//! check). Verification supports a small set of trusted keys — verify-any-of —
//! so key rotation has an overlap window rather than a hard cutover.

use minisign_verify::{PublicKey, Signature};

use crate::launch::core::ResolutionError;

/// The release public key the launcher embeds.
///
/// Single source of truth: `build.rs` copies the one committed key
/// (`keys/accelerator-release.pub` — the same file the bootstrap ships) into
/// `OUT_DIR`, so the launcher's embedded key and the bootstrap's key cannot
/// diverge (no separate coherence check needed). In 0164 this is a placeholder
/// release key; 0165 swaps in the production key (a verify-any-of rotation
/// overlap). It is never a fixture/test key — the hermetic tests inject their
/// own freshly-generated keys via config, so a fixture key can never reach a
/// release build.
pub const EMBEDDED_RELEASE_KEY: &str =
    include_str!(concat!(env!("OUT_DIR"), "/release.pub"));

/// A set of trusted public keys; a signature is accepted if ANY key verifies it.
pub struct TrustedKeys {
    keys: Vec<PublicKey>,
}

impl TrustedKeys {
    /// Parse minisign `.pub` file contents (comment line + base64 line each).
    ///
    /// # Errors
    ///
    /// [`ResolutionError::CacheRootUnavailable`] (used as a generic
    /// startup-config error) if a key cannot be parsed — a misconfigured trust
    /// root must fail closed.
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

    /// The launcher's production trust root: just the embedded release key.
    ///
    /// # Errors
    ///
    /// If the embedded key cannot be parsed (a build-time misconfiguration).
    pub fn embedded() -> Result<Self, ResolutionError> {
        Self::from_public_key_files(&[EMBEDDED_RELEASE_KEY])
    }

    /// Whether `signature` (a `.minisig` file's contents) verifies `data` under
    /// any trusted key. Any parse/verify failure is a non-match, never a panic.
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
        // Guards the build.rs copy → include_str!(OUT_DIR) → parse chain: the
        // committed key must produce a usable trust root.
        assert!(TrustedKeys::embedded().is_ok());
    }

    #[test]
    fn the_embedded_key_is_not_a_bare_placeholder() {
        // A crude but real guard that build.rs embedded an actual minisign key
        // (comment + base64 line), not an empty/garbage file.
        assert!(EMBEDDED_RELEASE_KEY.contains("untrusted comment"));
        assert!(EMBEDDED_RELEASE_KEY.lines().count() >= 2);
    }
}
