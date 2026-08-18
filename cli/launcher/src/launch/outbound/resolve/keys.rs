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

    /// Whether `signature` verifies the bytes streamed from `reader` under any
    /// trusted key, without ever holding the whole payload in memory.
    ///
    /// The signature must be prehashed — which the release path always produces
    /// (`allow_legacy` is false everywhere) — since incremental verification is
    /// only possible in that mode. Each key whose id matches gets its own
    /// verifier fed from the one pass, so a ~120MB archive is read once and
    /// buffered never.
    ///
    /// # Errors
    ///
    /// An I/O error from `reader`. A signature that does not verify is
    /// `Ok(false)`, not an error.
    pub fn verifies_stream<R: std::io::Read>(
        &self,
        mut reader: R,
        signature: &str,
    ) -> std::io::Result<bool> {
        let Ok(parsed) = Signature::decode(signature) else {
            return Ok(false);
        };
        let mut verifiers: Vec<_> = self
            .keys
            .iter()
            .filter_map(|key| key.verify_stream(&parsed).ok())
            .collect();
        if verifiers.is_empty() {
            return Ok(false);
        }
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            for verifier in &mut verifiers {
                verifier.update(&buffer[..read]);
            }
        }
        Ok(verifiers
            .iter_mut()
            .any(|verifier| verifier.finalize().is_ok()))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
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

    // A throwaway keypair over the exact bytes below, produced with the pinned
    // `minisign -S` (prehashed). It exists only to prove the streaming verify
    // agrees with the contiguous one; the release trust root is unaffected.
    const TEST_PUB: &str = "untrusted comment: test key\n\
        RWQpaBIoB4DruHSaRS4vyYpvUh7YxGji4HFHW3Jz2QHFOb65wMAvoMt5";
    const TEST_BODY: &[u8] = b"the streamed payload\n";
    const TEST_SIG: &str = "untrusted comment: signature\n\
        RUQpaBIoB4DruIMjFSks1XCFFaJuAYD6bUdxzQb0T3YrXpHzQwvodLXfz74hUNhGZeaAp04OLoNWlb5Rb2b0yngeUHMWIG2gWQo=\n\
        trusted comment: timestamp:1787096787\tfile:body.bin\thashed\n\
        0wBq8BTGHRTEsNJEFRFr1erd6cjJZAI52lkfkQM49Em+Bf054aLtCpcrk6VSCJF7hyEjbXCBFVgmEx5hZgdRDA==\n";

    fn test_keys() -> TrustedKeys {
        TrustedKeys::from_public_key_files(&[TEST_PUB])
            .expect("the test key parses")
    }

    #[test]
    fn the_streaming_verify_accepts_a_valid_prehashed_signature() {
        let verified = test_keys()
            .verifies_stream(TEST_BODY, TEST_SIG)
            .expect("no io error");
        assert!(verified);
        // And agrees with the contiguous path over the same bytes.
        assert!(test_keys().verifies(TEST_BODY, TEST_SIG));
    }

    #[test]
    fn the_streaming_verify_rejects_tampered_bytes() {
        let mut tampered = TEST_BODY.to_vec();
        tampered[0] ^= 0xff;
        let verified = test_keys()
            .verifies_stream(tampered.as_slice(), TEST_SIG)
            .expect("no io error");
        assert!(!verified, "a tampered stream must not verify");
    }

    #[test]
    fn the_streaming_verify_rejects_an_untrusted_signer() {
        // The embedded release key did not sign the test body.
        let verified = TrustedKeys::embedded()
            .expect("embedded")
            .verifies_stream(TEST_BODY, TEST_SIG)
            .expect("no io error");
        assert!(!verified);
    }
}
