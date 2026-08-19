//! Verified fetches for the cold path: the large archive streamed to disk, and
//! the small attestation pair buffered.
//!
//! The archive is never held in memory — it is streamed to a temp file and its
//! sha256 computed in the same pass, then its prehashed signature is verified by
//! reading the file back rather than by buffering it. Two disk reads of a
//! ~120MB file cost tens of milliseconds; a RAM buffer of it is the OOM the
//! whole streaming path exists to avoid.

use std::fs::File;
use std::path::Path;

use sha2::{Digest as _, Sha256};

use crate::launch::core::tree::TreeError;

use super::super::fetcher::{Fetcher, StreamLimits, StreamSink};
use super::super::keys::TrustedKeys;

/// The two small documents fetched alongside an archive.
pub struct AttestationBytes {
    pub document: Vec<u8>,
    pub signature: String,
}

/// Stream the archive at `url` into `dest`, resuming from any bytes already on
/// disk and returning the sha256 of the whole file.
///
/// `max_bytes` is the artifact's `archive_size`, so a body larger than the
/// manifest promised is refused rather than filling the disk. A partial `dest`
/// from an interrupted run makes this issue a `Range` request from the bytes
/// already present, so a link too slow to finish one crawl still converges
/// rather than restarting from zero each time. The digest is recomputed over the
/// whole file, so a resumed transfer is verified exactly as a fresh one is.
///
/// # Errors
///
/// [`TreeError::Unreachable`] wrapping the transport cause — a bare fetch
/// failure is an availability failure, not tampering, so a crawl degrades.
pub fn stream_archive(
    fetcher: &Fetcher,
    url: &str,
    dest: &Path,
    max_bytes: u64,
) -> Result<[u8; 32], TreeError> {
    let existing = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    let range_from = (existing > 0).then_some(existing);
    let limits = StreamLimits::for_archive(max_bytes);
    let mut open_dest =
        |partial: bool| -> std::io::Result<Box<dyn StreamSink>> {
            // A 206 appends after the prefix; a 200 (server ignored the range)
            // truncates and takes the whole body.
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(partial)
                .truncate(!partial)
                .open(dest)?;
            Ok(Box::new(FileSink { file }))
        };
    let body = fetcher
        .get_streaming(url, &limits, range_from, &mut open_dest)
        .map_err(|error| TreeError::Unreachable {
            detail: format!(
                "could not fetch the archive from {url}: {error:?}"
            ),
        })?;
    if body.partial {
        // The streamed digest covered only the appended suffix, so re-hash the
        // whole file — a local re-read of at most ~120MB, not a second network
        // pass.
        digest_file(dest)
    } else {
        Ok(body.sha256)
    }
}

fn digest_file(path: &Path) -> Result<[u8; 32], TreeError> {
    use std::io::Read as _;
    let mut file = File::open(path).map_err(|error| TreeError::Extraction {
        detail: format!("cannot re-read the resumed archive: {error}"),
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read =
            file.read(&mut buffer)
                .map_err(|error| TreeError::Extraction {
                    detail: format!(
                        "cannot re-read the resumed archive: {error}"
                    ),
                })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

struct FileSink {
    file: File,
}

impl StreamSink for FileSink {
    fn accept(&mut self, chunk: &[u8]) -> std::io::Result<()> {
        std::io::Write::write_all(&mut self.file, chunk)
    }
}

/// Fetch the attestation document and its detached signature.
///
/// # Errors
///
/// [`TreeError::Unreachable`] if either small asset cannot be fetched — a
/// transport failure, distinct from the signature failing to verify.
pub fn fetch_attestation(
    fetcher: &Fetcher,
    document_url: &str,
    signature_url: &str,
) -> Result<AttestationBytes, TreeError> {
    let document =
        fetcher
            .get(document_url)
            .map_err(|error| TreeError::Unreachable {
                detail: format!(
                "could not fetch the attestation from {document_url}: {error:?}"
            ),
            })?;
    let signature =
        fetcher
            .get(signature_url)
            .map_err(|error| TreeError::Unreachable {
                detail: format!(
                    "could not fetch the attestation signature from \
                 {signature_url}: {error:?}"
                ),
            })?;
    Ok(AttestationBytes {
        document,
        signature: String::from_utf8_lossy(&signature).into_owned(),
    })
}

/// Verify the archive on disk against its expected sha256 and its prehashed
/// signature, reading the file rather than a buffer.
///
/// # Errors
///
/// [`TreeError::UnexpectedDigest`] if the streamed digest disagrees with the
/// expected one, or [`TreeError::Attestation`] if the signature does not verify.
pub fn verify_archive_file(
    path: &Path,
    streamed_sha256: &[u8; 32],
    expected_sha256: &str,
    signature: &str,
    keys: &TrustedKeys,
) -> Result<(), TreeError> {
    let streamed_hex = hex(streamed_sha256);
    if streamed_hex != expected_sha256 {
        return Err(TreeError::UnexpectedDigest {
            artifact: "archive".to_owned(),
            expected: expected_sha256.to_owned(),
            found: streamed_hex,
        });
    }
    let file = File::open(path).map_err(|error| TreeError::Extraction {
        detail: format!("cannot reopen the archive to verify it: {error}"),
    })?;
    let verified = keys.verifies_stream(file, signature).map_err(|error| {
        TreeError::Attestation {
            detail: format!("cannot read the archive to verify it: {error}"),
        }
    })?;
    if !verified {
        return Err(TreeError::Attestation {
            detail: "the archive's signature does not verify under the \
                     embedded release key"
                .to_owned(),
        });
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use sha2::{Digest as _, Sha256};

    use crate::launch::core::tree::TreeError;

    use super::super::super::keys::TrustedKeys;
    use super::verify_archive_file;

    // The same throwaway keypair the keys tests use, over the same body.
    const TEST_PUB: &str = "untrusted comment: test key\n\
        RWQpaBIoB4DruHSaRS4vyYpvUh7YxGji4HFHW3Jz2QHFOb65wMAvoMt5";
    const TEST_BODY: &[u8] = b"the streamed payload\n";
    const TEST_SIG: &str = "untrusted comment: signature\n\
        RUQpaBIoB4DruIMjFSks1XCFFaJuAYD6bUdxzQb0T3YrXpHzQwvodLXfz74hUNhGZeaAp04OLoNWlb5Rb2b0yngeUHMWIG2gWQo=\n\
        trusted comment: timestamp:1787096787\tfile:body.bin\thashed\n\
        0wBq8BTGHRTEsNJEFRFr1erd6cjJZAI52lkfkQM49Em+Bf054aLtCpcrk6VSCJF7hyEjbXCBFVgmEx5hZgdRDA==\n";

    fn keys() -> TrustedKeys {
        TrustedKeys::from_public_key_files(&[TEST_PUB]).expect("test key")
    }

    fn write_body() -> (tempfile::TempDir, std::path::PathBuf, [u8; 32]) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("archive.tar.gz");
        std::fs::write(&path, TEST_BODY).expect("write");
        let digest: [u8; 32] = Sha256::digest(TEST_BODY).into();
        (dir, path, digest)
    }

    #[test]
    fn a_matching_digest_and_a_valid_signature_verify() {
        let (_dir, path, digest) = write_body();
        let expected = super::hex(&digest);
        verify_archive_file(&path, &digest, &expected, TEST_SIG, &keys())
            .expect("verifies");
    }

    #[test]
    fn a_digest_disagreement_is_an_unexpected_digest() {
        let (_dir, path, digest) = write_body();
        let wrong = "f".repeat(64);
        assert!(matches!(
            verify_archive_file(&path, &digest, &wrong, TEST_SIG, &keys()),
            Err(TreeError::UnexpectedDigest { .. })
        ));
    }

    #[test]
    fn a_signature_that_does_not_verify_is_an_attestation_failure() {
        let (_dir, path, digest) = write_body();
        let expected = super::hex(&digest);
        let untrusted = TrustedKeys::embedded().expect("embedded");
        assert!(matches!(
            verify_archive_file(
                &path, &digest, &expected, TEST_SIG, &untrusted
            ),
            Err(TreeError::Attestation { .. })
        ));
    }
}
