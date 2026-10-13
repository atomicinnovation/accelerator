//! One item's local file and fetched remote, as classification reads them.

use std::cell::OnceCell;
use std::path::Path;

use work::sync::DualRecipeFingerprints;
use work::sync::ItemComparison;
use work::sync::ItemDigests;
use work::sync::LegacyHashes;
use work::sync::LocalFingerprints;
use work::sync::PullResidue;
use work::sync::RemoteFingerprints;
use work::sync::SplitFingerprints;

use crate::sync::baseline_digester::BaselineDigester;
use crate::sync::digest;

/// The per-item memo classification and a later baseline record share, so
/// each read and digest is paid at most once, and only if asked for.
///
/// A failed read is not memoised: it is reported each time it is asked.
pub struct ItemSnapshot<'a> {
    path: &'a Path,
    remote_projection: Option<String>,
    digester: &'a BaselineDigester<'a>,
    mtime: OnceCell<Option<u64>>,
    content: OnceCell<String>,
    legacy_local: OnceCell<String>,
    local: OnceCell<LocalFingerprints>,
    legacy_remote: OnceCell<Option<String>>,
    remote: OnceCell<Option<RemoteFingerprints>>,
}

fn memoised<T: Clone>(
    cell: &OnceCell<T>,
    compute: impl FnOnce() -> Result<T, kernel::Error>,
) -> Result<T, kernel::Error> {
    if let Some(value) = cell.get() {
        return Ok(value.clone());
    }
    let value = compute()?;
    Ok(cell.get_or_init(|| value).clone())
}

impl<'a> ItemSnapshot<'a> {
    /// `remote_projection` is the un-normalised projected body from a
    /// `show` call, when one was made this run — `None` when the two-tier
    /// read rule decided this item's stamp proves it unchanged.
    #[must_use]
    pub const fn new(
        path: &'a Path,
        remote_projection: Option<String>,
        digester: &'a BaselineDigester<'a>,
    ) -> Self {
        Self {
            path,
            remote_projection,
            digester,
            mtime: OnceCell::new(),
            content: OnceCell::new(),
            legacy_local: OnceCell::new(),
            local: OnceCell::new(),
            legacy_remote: OnceCell::new(),
            remote: OnceCell::new(),
        }
    }

    fn content(&self) -> Result<String, kernel::Error> {
        memoised(&self.content, || {
            std::fs::read_to_string(self.path).map_err(|error| {
                kernel::Error::Failed(format!(
                    "could not read {}: {error}",
                    self.path.display()
                ))
            })
        })
    }

    /// Both recipes' fingerprints of what classification read, for a
    /// baseline record that must describe exactly that.
    ///
    /// # Errors
    ///
    /// When the local file cannot be read or fingerprinted.
    pub fn recorded(&self) -> Result<DualRecipeFingerprints, kernel::Error> {
        Ok(DualRecipeFingerprints {
            split: SplitFingerprints {
                local: Some(self.local()?),
                remote: self.remote()?,
            },
            legacy: LegacyHashes {
                local: Some(self.legacy_local()?),
                remote: self.legacy_remote()?,
            },
        })
    }
}

impl ItemDigests for ItemSnapshot<'_> {
    fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
        memoised(&self.mtime, || {
            Ok(std::fs::metadata(self.path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| {
                    modified.duration_since(std::time::UNIX_EPOCH).ok()
                })
                .map(|duration| duration.as_secs()))
        })
    }

    fn legacy_local(&self) -> Result<String, kernel::Error> {
        memoised(&self.legacy_local, || digest::local(&self.content()?))
    }

    fn legacy_remote(&self) -> Result<Option<String>, kernel::Error> {
        memoised(&self.legacy_remote, || {
            Ok(self.remote_projection.as_deref().map(digest::remote_body))
        })
    }

    fn local(&self) -> Result<LocalFingerprints, kernel::Error> {
        memoised(&self.local, || {
            let (legacy, fingerprints) =
                self.digester.local(&self.content()?)?;
            let _ = self.legacy_local.set(legacy);
            Ok(fingerprints)
        })
    }

    fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error> {
        memoised(&self.remote, || {
            Ok(self
                .remote_projection
                .as_deref()
                .map(|projected| self.digester.remote(projected).1))
        })
    }
}

impl ItemComparison for ItemSnapshot<'_> {
    fn pull_residue(&self) -> Option<PullResidue> {
        let projected = self.remote_projection.as_deref()?;
        let content = self.content().ok()?;
        let (_, body) = digest::split_frontmatter_and_body(&content).ok()?;
        self.digester.comparison().pull_residue(&body, projected)
    }
}

#[cfg(test)]
mod tests {
    use corpus_adapters::CommonMarkBody;
    use tracker::PlainDialect;
    use work::sync::ItemComparison as _;
    use work::sync::ItemDigests as _;

    use super::ItemSnapshot;
    use crate::sync::baseline_digester::BaselineDigester;
    use crate::sync::digest;

    type TestError = Box<dyn std::error::Error>;

    const CONTENT: &str = "---\nstatus: ready\n---\n\nBody text\n";

    fn digester() -> BaselineDigester<'static> {
        BaselineDigester::new(&PlainDialect, &CommonMarkBody)
    }

    #[test]
    fn the_legacy_local_digest_matches_the_pure_recipe() -> Result<(), TestError>
    {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("item.md");
        std::fs::write(&path, CONTENT)?;
        let digester = digester();

        let snapshot = ItemSnapshot::new(&path, None, &digester);

        assert_eq!(snapshot.legacy_local()?, digest::local(CONTENT)?);
        Ok(())
    }

    #[test]
    fn an_unfetched_remote_has_no_digests_and_no_residue(
    ) -> Result<(), TestError> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("item.md");
        std::fs::write(&path, CONTENT)?;
        let digester = digester();

        let snapshot = ItemSnapshot::new(&path, None, &digester);

        assert_eq!(snapshot.legacy_remote()?, None);
        assert_eq!(snapshot.remote()?, None);
        assert_eq!(snapshot.pull_residue(), None);
        Ok(())
    }

    #[test]
    fn the_recorded_fingerprints_are_the_digests_classification_read(
    ) -> Result<(), TestError> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("item.md");
        std::fs::write(&path, CONTENT)?;
        let digester = digester();
        let snapshot = ItemSnapshot::new(
            &path,
            Some("Title\nBody text\n".to_owned()),
            &digester,
        );
        let classified = (snapshot.local()?, snapshot.legacy_local()?);
        std::fs::write(&path, "---\nstatus: done\n---\n\nEdited\n")?;

        let recorded = snapshot.recorded()?;

        assert_eq!(recorded.split.local, Some(classified.0));
        assert_eq!(recorded.legacy.local, Some(classified.1));
        assert_eq!(
            recorded,
            digester.digests(CONTENT, Some("Title\nBody text\n"))?
        );
        Ok(())
    }

    #[test]
    fn mtime_is_none_for_a_missing_file() -> Result<(), TestError> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("missing.md");
        let digester = digester();

        let snapshot = ItemSnapshot::new(&path, None, &digester);

        assert_eq!(snapshot.mtime()?, None);
        Ok(())
    }
}
