//! The out-of-band create paths: authoring a local file for a discovered
//! remote issue, and canonicalising external ids so a cosmetic spelling
//! difference never re-discovers an already-tracked issue.
//!
//! The decision table never produces a create action; the sync run drives
//! both creates outside the id-keyed plan loop. The filesystem authoring that
//! needs config, an id scheme, and frontmatter composition lives behind the
//! [`LocalAuthor`] port, whose implementation sits in the binary layer — the
//! marker-and-`create` orchestration that the crash-recovery invariant
//! depends on stays in [`crate::sync::run`], where a fake tracker can drive
//! it.

use std::path::Path;
use std::path::PathBuf;

use corpus::store::AtomicWrite;
use tracker::ExternalId;
use tracker::RemoteIssue;

/// A local work-item file just authored from a discovered remote issue.
///
/// Carries the allocated local id as well as the path, because the sync
/// baseline is keyed by local id and the create writes an entry the moment
/// the file lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredLocal {
    pub id: String,
    pub path: PathBuf,
}

/// A remote issue with no local counterpart, ready to be authored as a new
/// local work item.
///
/// Carries the full [`RemoteIssue`] — not just the discovery stamp — because
/// authoring the local file needs the projected body, and the baseline entry
/// the create writes afterwards needs both the body and the stamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredIssue {
    pub external_id: ExternalId,
    pub issue: RemoteIssue,
}

/// Authors local work-item files the sync engine cannot compose itself.
///
/// The engine holds the tracker port and the baseline; what it lacks is the
/// id scheme, the template, and the frontmatter renderer — all config-bound
/// and living in the binary layer. This port is that seam.
pub trait LocalAuthor {
    /// Authors a brand-new local work-item file for a discovered remote issue,
    /// allocating the next id and refusing when the target path already
    /// exists. Returns the path written.
    ///
    /// The write is exclusive-create: a crash mid-write must leave no partial
    /// file, and an id collision — a stray on-disk file, or a concurrent
    /// second sync — must surface as an error rather than clobber.
    ///
    /// # Errors
    ///
    /// When id allocation, frontmatter composition, or the exclusive write
    /// fails — including when the destination already exists.
    fn author_from_remote(
        &self,
        issue: &DiscoveredIssue,
    ) -> Result<AuthoredLocal, kernel::Error>;

    /// Links an already-created remote id into an existing local draft's
    /// frontmatter, as one atomic write.
    ///
    /// # Errors
    ///
    /// When the file cannot be read, parsed, or rewritten.
    fn link_external_id(
        &self,
        path: &Path,
        external_id: &ExternalId,
    ) -> Result<(), kernel::Error>;
}

/// Writes `bytes` to `path` only when nothing is already there.
///
/// Refuses rather than clobbering — the exclusive-create semantics the
/// create-from-remote author needs so an id collision (a stray on-disk file,
/// or a concurrent second sync) surfaces as an error. The write itself stays
/// atomic through the injected store.
///
/// # Errors
///
/// When `path` already exists, or the underlying write fails.
pub fn exclusive_write(
    store: &dyn AtomicWrite,
    path: &Path,
    bytes: &[u8],
) -> Result<(), kernel::Error> {
    if path.exists() {
        return Err(kernel::Error::Failed(format!(
            "refusing to overwrite an existing file: {}",
            path.display()
        )));
    }
    store
        .write(path, bytes)
        .map_err(|error| kernel::Error::Failed(error.to_string()))
}

/// Folds an external id to a canonical comparison key.
///
/// `tracker::ExternalId` derives `Eq`/`Hash` over its raw bytes, so a stored
/// id differing from a search result only by case, surrounding whitespace, or
/// spacing around the project-number separator would compare unequal and
/// re-discover an already-tracked issue. This fold is the untracked-set
/// difference's single definition of "the same id": upper-case, whitespace
/// stripped, so `" eng - 12 "` and `"ENG-12"` collapse together.
#[must_use]
pub fn canonical_external_key(id: &ExternalId) -> String {
    id.as_str()
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::canonical_external_key;
    use tracker::ExternalId;

    fn key(raw: &str) -> String {
        canonical_external_key(&ExternalId::new(raw.to_owned()))
    }

    #[test]
    fn case_and_whitespace_differences_fold_together() {
        assert_eq!(key("ENG-12"), key(" eng - 12 "));
        assert_eq!(key("eng-12"), key("ENG-12"));
    }

    #[test]
    fn distinct_ids_stay_distinct() {
        assert_ne!(key("ENG-12"), key("ENG-13"));
        assert_ne!(key("ENG-1"), key("ENG-12"));
    }

    mod exclusive {
        use std::path::Path;

        use super::super::exclusive_write;
        use corpus::store::AtomicWrite;
        use corpus::store::StoreError;

        struct RealWrite;

        impl AtomicWrite for RealWrite {
            fn write(
                &self,
                path: &Path,
                bytes: &[u8],
            ) -> Result<(), StoreError> {
                std::fs::write(path, bytes).map_err(|error| StoreError::Io {
                    path: path.display().to_string(),
                    detail: error.to_string(),
                })
            }
        }

        #[test]
        fn writes_a_fresh_path() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("new.md");
            exclusive_write(&RealWrite, &path, b"content")
                .expect("fresh write");
            assert_eq!(
                std::fs::read_to_string(&path).expect("read"),
                "content"
            );
        }

        #[test]
        fn refuses_an_existing_path_rather_than_overwriting() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("existing.md");
            std::fs::write(&path, "original").expect("seed");
            assert!(exclusive_write(&RealWrite, &path, b"replacement").is_err());
            assert_eq!(
                std::fs::read_to_string(&path).expect("read"),
                "original",
                "the existing file must be untouched"
            );
        }
    }
}
