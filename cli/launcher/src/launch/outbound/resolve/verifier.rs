//! sha256 (corruption check) plus minisign (the security boundary: proves
//! signed-by-our-key, not merely served over TLS).

use std::fmt::Write as _;

use sha2::{Digest as _, Sha256};

use crate::launch::core::ResolutionError;

use super::keys::TrustedKeys;

/// Lowercase hex sha256 of `bytes` (the bare form the manifest carries).
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Verify a binary's sha256 then its minisign signature against a trusted key.
///
/// # Errors
///
/// [`ResolutionError::ChecksumMismatch`] or
/// [`ResolutionError::SignatureMismatch`].
pub fn verify_binary(
    asset: &str,
    bytes: &[u8],
    expected_sha256: &str,
    signature: &str,
    keys: &TrustedKeys,
) -> Result<(), ResolutionError> {
    let actual = sha256_hex(bytes);
    if actual != expected_sha256 {
        return Err(ResolutionError::ChecksumMismatch {
            asset: asset.to_owned(),
            expected: expected_sha256.to_owned(),
            actual,
        });
    }
    if !keys.verifies(bytes, signature) {
        return Err(ResolutionError::SignatureMismatch {
            asset: asset.to_owned(),
        });
    }
    Ok(())
}

/// Verify the manifest's own detached signature against a trusted key.
///
/// # Errors
///
/// [`ResolutionError::ManifestSignature`] if no trusted key verifies it.
pub fn verify_manifest(
    manifest_bytes: &[u8],
    manifest_signature: &str,
    keys: &TrustedKeys,
) -> Result<(), ResolutionError> {
    if keys.verifies(manifest_bytes, manifest_signature) {
        Ok(())
    } else {
        Err(ResolutionError::ManifestSignature)
    }
}

#[cfg(test)]
mod tests {
    use sha2::{Digest as _, Sha256};

    use super::sha256_hex;

    #[test]
    fn sha256_hex_matches_the_empty_vector() {
        // echo -n "" | sha256sum
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_hex_matches_the_abc_vector() {
        // echo -n "abc" | sha256sum
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_hex_matches_the_padding_boundary_vectors() {
        // 55 bytes is the largest message whose padding still fits the final
        // block; 56 forces a second block. Both drive the compression loop the
        // intrinsic backend replaces, across the boundary a soft-only test misses.
        assert_eq!(
            sha256_hex(&[b'a'; 55]),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 56]),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
    }

    #[test]
    fn sha256_hex_matches_a_multi_block_vector() {
        assert_eq!(
            sha256_hex(&[b'a'; 128]),
            "6836cf13bac400e9105071cd6af47084dfacad4e5e302c94bfed24e013afb73e"
        );
    }

    #[test]
    fn a_chunked_digest_agrees_with_the_one_shot() {
        let bytes = [b'a'; 130];
        let mut chunked = Sha256::new();
        chunked.update(&bytes[..64]);
        chunked.update(&bytes[64..]);
        assert_eq!(Sha256::digest(bytes), chunked.finalize());
    }
}
