//! The signed document binding a tree's identity to its content.
//!
//! Without it every check on the hit path is local and self-referential: a
//! digest matching the digest in its own directory name proves nothing about
//! provenance. It is a producer-side artifact rather than the manifest's
//! archive signature reused, because that signature covers the archive *file's*
//! bytes and the archive is deleted after extraction — leaving nothing on disk
//! for it to verify against, and a signature the consumer cannot check is not a
//! control.
//!
//! It binds artifact identity and content, and deliberately neither the plugin
//! release version nor the launcher's layout version. The first is unknowable
//! in the job that assembles, which runs before the version is chosen and whose
//! one archive set serves two cuts; binding it would also make cross-version
//! adoption impossible. The second is consumer-owned policy, and a signed copy
//! could never be rewritten by the launcher that owns it — a policy bump would
//! miss, re-materialise, fetch the same producer document still carrying the old
//! value, and miss again.

use serde::Deserialize;

use crate::launch::core::tree::TreeError;

use super::super::keys::TrustedKeys;
use super::layout::is_wellformed_digest;

/// The document shape this launcher reads.
pub const ATTESTATION_FORMAT_VERSION: u32 = 1;

/// A producer-signed statement about one artifact on one platform.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Attestation {
    pub attestation_format_version: u32,
    pub artifact: String,
    pub platform: String,
    pub archive_sha256: String,
    pub uncompressed_size: u64,
    pub entry_count: u64,
    pub table_sha256: String,
}

impl Attestation {
    /// Verify the detached signature over `document`, then read it.
    ///
    /// The signature is checked over the raw bytes before anything is parsed,
    /// so a malformed document can never be interpreted on the strength of
    /// having arrived alongside a signature.
    ///
    /// # Errors
    ///
    /// [`TreeError::Attestation`] if the signature does not verify under a
    /// trusted key, if the document is unreadable, or if it carries a format
    /// version this launcher does not read.
    pub fn verified(
        document: &[u8],
        signature: &str,
        keys: &TrustedKeys,
    ) -> Result<Self, TreeError> {
        if !keys.verifies(document, signature) {
            return Err(untrusted(
                "the signature does not verify under the embedded release key",
            ));
        }
        let attestation: Self = serde_json::from_slice(document)
            .map_err(|error| untrusted(&format!("unreadable: {error}")))?;
        if attestation.attestation_format_version > ATTESTATION_FORMAT_VERSION {
            return Err(untrusted(&format!(
                "format version {} is newer than this launcher reads ({})",
                attestation.attestation_format_version,
                ATTESTATION_FORMAT_VERSION
            )));
        }
        if !is_wellformed_digest(&attestation.archive_sha256)
            || !is_wellformed_digest(&attestation.table_sha256)
        {
            return Err(untrusted("a recorded digest is not lowercase hex"));
        }
        Ok(attestation)
    }

    /// Check every field against what is actually being resolved.
    ///
    /// Artifact identity and platform would otherwise live only in this
    /// document and in an unsigned pointer filename, so any process able to
    /// write the trees directory could repoint at another artifact's or another
    /// platform's generation whose signature is entirely valid.
    ///
    /// `table_sha256` is deliberately not checked here: it is read by
    /// verification and repair, which is what keeps the hit path's cost
    /// independent of a tree's file count.
    ///
    /// # Errors
    ///
    /// [`TreeError::Attestation`] naming the field that disagreed, or
    /// [`TreeError::UnexpectedDigest`] when the content is not the one this
    /// launcher was built to resolve.
    pub fn matches(
        &self,
        artifact: &str,
        platform: &str,
        expected_digest: &str,
    ) -> Result<(), TreeError> {
        if self.attestation_format_version != ATTESTATION_FORMAT_VERSION {
            return Err(untrusted("the format version is not this launcher's"));
        }
        if self.artifact != artifact {
            return Err(untrusted(&format!(
                "it describes the {} artifact, not {artifact}",
                self.artifact
            )));
        }
        if self.platform != platform {
            return Err(untrusted(&format!(
                "it describes the {} platform, not {platform}",
                self.platform
            )));
        }
        if self.archive_sha256 != expected_digest {
            return Err(TreeError::UnexpectedDigest {
                artifact: artifact.to_owned(),
                expected: expected_digest.to_owned(),
                found: self.archive_sha256.clone(),
            });
        }
        Ok(())
    }
}

fn untrusted(detail: &str) -> TreeError {
    TreeError::Attestation {
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::launch::core::tree::TreeError;

    use super::super::super::keys::TrustedKeys;
    use super::{Attestation, ATTESTATION_FORMAT_VERSION};

    const ARCHIVE: &str =
        "abc0000000000000000000000000000000000000000000000000000000000123";
    const TABLE: &str =
        "def0000000000000000000000000000000000000000000000000000000000456";

    /// A key set that accepts nothing, so the signature gate is exercised
    /// without a signing step. The signature-positive cases live in the
    /// integration suite, where a real keypair exists.
    fn no_trusted_keys() -> TrustedKeys {
        TrustedKeys::from_public_key_files(&[])
            .expect("an empty key set parses")
    }

    fn document(version: u32) -> String {
        format!(
            "{{\"attestation_format_version\":{version},\
             \"artifact\":\"browser\",\"platform\":\"linux-x64\",\
             \"archive_sha256\":\"{ARCHIVE}\",\
             \"uncompressed_size\":185790464,\"entry_count\":14,\
             \"table_sha256\":\"{TABLE}\"}}"
        )
    }

    fn parsed() -> Attestation {
        serde_json::from_str(&document(ATTESTATION_FORMAT_VERSION))
            .expect("the fixture document parses")
    }

    #[test]
    fn a_signature_no_trusted_key_verifies_is_refused_before_parsing() {
        let outcome = Attestation::verified(
            document(ATTESTATION_FORMAT_VERSION).as_bytes(),
            "not a signature",
            &no_trusted_keys(),
        );
        assert!(matches!(outcome, Err(TreeError::Attestation { .. })));
    }

    #[test]
    fn an_unknown_additive_field_still_reads() {
        let with_future_field = document(ATTESTATION_FORMAT_VERSION)
            .replace("{\"attestation", "{\"future_field\":42,\"attestation");
        let attestation: Attestation =
            serde_json::from_str(&with_future_field).expect("still reads");
        assert_eq!(attestation.artifact, "browser");
    }

    #[test]
    fn each_field_is_checked_against_what_is_being_resolved() {
        let attestation = parsed();
        attestation
            .matches("browser", "linux-x64", ARCHIVE)
            .expect("the fixture describes exactly this");

        assert!(matches!(
            attestation.matches("driver", "linux-x64", ARCHIVE),
            Err(TreeError::Attestation { .. })
        ));
        assert!(matches!(
            attestation.matches("browser", "darwin-arm64", ARCHIVE),
            Err(TreeError::Attestation { .. })
        ));
    }

    #[test]
    fn a_digest_other_than_the_compiled_in_one_is_the_rollback_refusal() {
        let superseded = "f".repeat(64);
        let outcome = parsed().matches("browser", "linux-x64", &superseded);
        assert!(
            matches!(
                outcome,
                Err(TreeError::UnexpectedDigest { ref expected, .. })
                    if *expected == superseded
            ),
            "a superseded generation must be refused, not adopted"
        );
    }

    #[test]
    fn a_format_version_this_launcher_does_not_read_is_refused() {
        let ahead: Attestation =
            serde_json::from_str(&document(ATTESTATION_FORMAT_VERSION + 1))
                .expect("parses");
        assert!(matches!(
            ahead.matches("browser", "linux-x64", ARCHIVE),
            Err(TreeError::Attestation { .. })
        ));
    }
}
