//! Verified fetches for the cold path: the large archive streamed to disk, and
//! the small attestation pair buffered.
//!
//! The archive is never held in memory — it is streamed to a temp file and its
//! sha256 computed in the same pass, then its prehashed signature is verified by
//! reading the file back rather than by buffering it. Two disk reads of a
//! ~120MB file cost tens of milliseconds; a RAM buffer of it is the OOM the
//! whole streaming path exists to avoid.

use std::fs::File;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::launch::core::tree::TreeError;

use super::super::fetcher::{
    FetchError, Fetcher, Resumed, SinkMode, StreamLimits, StreamSink,
};
use super::super::keys::TrustedKeys;

/// The two small documents fetched alongside an archive.
pub struct AttestationBytes {
    pub document: Vec<u8>,
    pub signature: String,
}

/// How much of the archive a partial download already holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartialArchive {
    Absent,
    Resumable(u64),
    Complete,
    OverLong,
}

impl PartialArchive {
    #[must_use]
    pub const fn assess(on_disk: Option<u64>, archive_size: u64) -> Self {
        match on_disk {
            None | Some(0) => Self::Absent,
            Some(held) if held < archive_size => Self::Resumable(held),
            Some(held) if held == archive_size => Self::Complete,
            Some(_) => Self::OverLong,
        }
    }

    /// The partial's bytes are either kept or truncated away before the
    /// download writes, so the peak need is the shortfall, never more.
    #[must_use]
    pub const fn additional_bytes(&self, archive_size: u64) -> u64 {
        match *self {
            Self::Absent => archive_size,
            Self::Resumable(held) => archive_size.saturating_sub(held),
            Self::Complete | Self::OverLong => 0,
        }
    }
}

/// A partial archive's path, assessed once against the archive size, so a
/// download cannot act on a path that disagrees with its assessment.
pub struct AssessedPartial {
    path: PathBuf,
    archive_size: u64,
    state: PartialArchive,
}

impl AssessedPartial {
    /// Anything at `path` that is not a regular file is over-long, so it is
    /// replaced rather than resumed.
    #[must_use]
    pub fn of(path: &Path, archive_size: u64) -> Self {
        let state = match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_file() => {
                PartialArchive::assess(Some(metadata.len()), archive_size)
            }
            Ok(_) => PartialArchive::OverLong,
            Err(_) => PartialArchive::Absent,
        };
        Self {
            path: path.to_owned(),
            archive_size,
            state,
        }
    }

    #[must_use]
    pub const fn additional_bytes(&self) -> u64 {
        self.state.additional_bytes(self.archive_size)
    }
}

/// Stream the archive at `url` into the partial's path, resuming from any
/// bytes already on disk and returning the sha256 of the whole file.
///
/// The archive size bounds the body, so one larger than the manifest
/// promised is refused rather than filling the disk. A partial from an
/// interrupted run makes this issue a `Range` request from the bytes already
/// present, so a link too slow to finish one crawl still converges rather
/// than restarting from zero each time. A partial at the archive size is
/// hashed without a fetch, and one longer than it is fetched again whole. The
/// digest is recomputed over the whole file, so a resumed transfer is verified
/// exactly as a fresh one is.
///
/// # Errors
///
/// [`TreeError::Unreachable`] wrapping the transport cause — a bare fetch
/// failure is an availability failure, not tampering, so a crawl degrades.
pub fn stream_archive(
    fetcher: &Fetcher,
    url: &str,
    partial: &AssessedPartial,
) -> Result<[u8; 32], TreeError> {
    let download = Download {
        fetcher,
        url,
        path: &partial.path,
        limits: StreamLimits::for_archive(partial.archive_size),
    };
    match partial.state {
        PartialArchive::Absent | PartialArchive::OverLong => {
            download.fetch_whole()
        }
        PartialArchive::Resumable(held) => download.resume_from(held),
        PartialArchive::Complete => download.digest_partial(),
    }
}

struct Download<'a> {
    fetcher: &'a Fetcher,
    url: &'a str,
    path: &'a Path,
    limits: StreamLimits,
}

impl Download<'_> {
    fn fetch_whole(&self) -> Result<[u8; 32], TreeError> {
        self.fetcher
            .get_whole(self.url, &self.limits, &mut || {
                open_archive_sink(self.path, SinkMode::Truncate)
            })
            .map(|whole| whole.sha256)
            .map_err(|error| self.unreachable(&error))
    }

    fn resume_from(&self, held: u64) -> Result<[u8; 32], TreeError> {
        let resumed = self
            .fetcher
            .resume(self.url, &self.limits, held, &mut |mode| {
                open_archive_sink(self.path, mode)
            })
            .map_err(|error| self.unreachable(&error))?;
        match resumed {
            Resumed::Whole(whole) => Ok(whole.sha256),
            Resumed::Appended | Resumed::AlreadyComplete => {
                self.digest_partial()
            }
            Resumed::RangeRejected => self.fetch_whole(),
        }
    }

    /// A local re-read of at most ~120MB, not a second network pass.
    fn digest_partial(&self) -> Result<[u8; 32], TreeError> {
        digest_file(self.path)
    }

    fn unreachable(&self, error: &FetchError) -> TreeError {
        TreeError::Unreachable {
            detail: format!(
                "could not fetch the archive from {}: {error:?}",
                self.url
            ),
        }
    }
}

fn open_archive_sink(
    path: &Path,
    mode: SinkMode,
) -> std::io::Result<Box<dyn StreamSink>> {
    let appending = matches!(mode, SinkMode::Append { .. });
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(appending)
        .truncate(!appending)
        .open(path)?;
    Ok(Box::new(FileSink { file }))
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
    use super::{verify_archive_file, PartialArchive};

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

    const ARCHIVE_SIZE: u64 = 1000;

    #[test]
    fn no_file_is_absent() {
        assert_eq!(
            PartialArchive::assess(None, ARCHIVE_SIZE),
            PartialArchive::Absent
        );
    }

    #[test]
    fn an_empty_file_is_absent() {
        assert_eq!(
            PartialArchive::assess(Some(0), ARCHIVE_SIZE),
            PartialArchive::Absent
        );
    }

    #[test]
    fn a_short_file_is_resumable_from_its_length() {
        assert_eq!(
            PartialArchive::assess(Some(400), ARCHIVE_SIZE),
            PartialArchive::Resumable(400)
        );
    }

    #[test]
    fn a_file_of_the_archive_size_is_complete() {
        assert_eq!(
            PartialArchive::assess(Some(ARCHIVE_SIZE), ARCHIVE_SIZE),
            PartialArchive::Complete
        );
    }

    #[test]
    fn a_longer_file_is_over_long() {
        assert_eq!(
            PartialArchive::assess(Some(ARCHIVE_SIZE + 1), ARCHIVE_SIZE),
            PartialArchive::OverLong
        );
    }

    #[test]
    fn an_absent_partial_needs_the_whole_archive() {
        assert_eq!(
            PartialArchive::Absent.additional_bytes(ARCHIVE_SIZE),
            ARCHIVE_SIZE
        );
    }

    #[test]
    fn a_resumable_partial_needs_the_rest_of_the_archive() {
        assert_eq!(
            PartialArchive::Resumable(400).additional_bytes(ARCHIVE_SIZE),
            600
        );
    }

    #[test]
    fn a_complete_partial_needs_nothing_more() {
        assert_eq!(PartialArchive::Complete.additional_bytes(ARCHIVE_SIZE), 0);
    }

    #[test]
    fn an_over_long_partial_needs_nothing_more_since_truncation_frees_it() {
        assert_eq!(PartialArchive::OverLong.additional_bytes(ARCHIVE_SIZE), 0);
    }
}
