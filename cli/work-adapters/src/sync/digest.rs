//! The two digest recipes the sync baseline stores. Port of
//! `work-item-normalise.sh:113-116` (`local`) and the `hash-common.sh`
//! sha256 recipe applied to a normalised remote body (`remote_body`).

use sha2::Digest as _;
use sha2::Sha256;

/// True iff `line` opens with a top-level fence — `^---[[:space:]]*$` —
/// matching `config_extract_frontmatter`/`config_extract_body`'s awk
/// exactly.
///
/// This is the digest path's own recogniser, not `document::fence::split`:
/// `document` compares the exact bytes `---` (a trailing space fails it) and
/// caps its scan at 1 MiB, whereas awk has neither restriction. A fence
/// carrying a trailing space would make `document` treat the whole file as
/// body, silently promoting a routine `last_updated` restamp into a content
/// change.
fn is_fence(line: &str) -> bool {
    line.starts_with("---") && line[3..].chars().all(char::is_whitespace)
}

/// Splits `content` into its raw frontmatter and body at the two fence
/// lines.
///
/// Errors, rather than treating the content as body-only, when the
/// content does not open with a fence or the frontmatter is never closed —
/// bash aborts non-zero in both cases (`set -euo pipefail` under
/// `config_extract_frontmatter`'s `NR == 1 && !/^---[[:space:]]*$/ { exit }`),
/// and returning `Ok` here would silently accept input the oracle rejects.
///
/// # Errors
///
/// When `content` does not open with a `---` fence, or the frontmatter is
/// never closed with a second one.
pub fn split_frontmatter_and_body(
    content: &str,
) -> Result<(String, String), kernel::Error> {
    let mut lines = content.lines();
    let first = lines.next().ok_or_else(|| {
        kernel::Error::Failed(
            "content is empty: no opening frontmatter fence".to_owned(),
        )
    })?;
    if !is_fence(first) {
        return Err(kernel::Error::Failed(
            "content does not open with a '---' frontmatter fence".to_owned(),
        ));
    }

    let mut frontmatter = Vec::new();
    for line in lines.by_ref() {
        if is_fence(line) {
            let body: Vec<&str> = lines.collect();
            return Ok((frontmatter.join("\n"), body.join("\n")));
        }
        frontmatter.push(line);
    }
    Err(kernel::Error::Failed(
        "frontmatter opened but never closed with a second '---' fence"
            .to_owned(),
    ))
}

#[must_use]
fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// The local-side hash: `filter_frontmatter_keys` over the frontmatter,
/// `trim_lines` over the body, joined and hashed.
///
/// Both separators are emitted unconditionally, whichever side is empty —
/// matching bash's `fm=$(…); body=$(…); printf '%s\n%s\n' "$fm" "$body"`,
/// where command substitution strips each side's trailing newlines and
/// `printf` adds exactly one back to each. The digest is opaque and only
/// ever compared against a digest from the same recipe, so the resulting
/// blank line for an empty body makes the hash arbitrary, not wrong — a
/// conditional separator would instead change every affected item's hash
/// and mass-reclassify at cutover.
///
/// # Errors
///
/// When `file_content` does not open with a frontmatter fence, or the
/// frontmatter is never closed.
pub fn local(file_content: &str) -> Result<String, kernel::Error> {
    let (frontmatter, body) = split_frontmatter_and_body(file_content)?;
    let filtered = work::normalise::filter_frontmatter_keys(&frontmatter);
    let trimmed_body = work::normalise::trim_lines(&body);
    let hashed = format!(
        "{}\n{}\n",
        filtered.trim_end_matches('\n'),
        trimmed_body.trim_end_matches('\n')
    );
    Ok(sha256_hex(hashed.as_bytes()))
}

/// The remote-side hash: `trim_lines` over the already-projected body, then
/// hashed. `projected_body` is `RemoteIssue.body` — un-normalised, since
/// normalising before hashing is this recipe's own job.
#[must_use]
pub fn remote_body(projected_body: &str) -> String {
    let trimmed = work::normalise::trim_lines(projected_body);
    sha256_hex(trimmed.as_bytes())
}

/// A `work::sync::ItemDigests` backed by a file on disk.
///
/// Takes an optional already-fetched remote body, and memoises each hash
/// and the mtime in a `RefCell` so a caller pays the stat and each hash
/// cost at most once — and only if `classify` actually asks.
///
/// The outer `Option` on `mtime`/`remote_hash` distinguishes "not yet
/// computed" from the inner `Option`'s own "computed as absent", which
/// `classify` depends on: a real three-state memo, not two collapsed into
/// one.
#[allow(clippy::option_option)]
pub struct LazyItemDigests<'a> {
    path: &'a std::path::Path,
    remote_content: Option<String>,
    mtime: std::cell::RefCell<Option<Option<u64>>>,
    local_hash: std::cell::RefCell<Option<String>>,
    remote_hash: std::cell::RefCell<Option<Option<String>>>,
}

impl<'a> LazyItemDigests<'a> {
    /// `remote_content` is the un-normalised projected body from a `show`
    /// call, when one was made this run — `None` when the two-tier read
    /// rule decided this item's stamp proves it unchanged.
    #[must_use]
    pub const fn new(
        path: &'a std::path::Path,
        remote_content: Option<String>,
    ) -> Self {
        Self {
            path,
            remote_content,
            mtime: std::cell::RefCell::new(None),
            local_hash: std::cell::RefCell::new(None),
            remote_hash: std::cell::RefCell::new(None),
        }
    }
}

impl work::sync::ItemDigests for LazyItemDigests<'_> {
    fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
        if let Some(cached) = *self.mtime.borrow() {
            return Ok(cached);
        }
        let value = std::fs::metadata(self.path)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| {
                modified.duration_since(std::time::UNIX_EPOCH).ok()
            })
            .map(|duration| duration.as_secs());
        *self.mtime.borrow_mut() = Some(value);
        Ok(value)
    }

    fn local(&self) -> Result<String, kernel::Error> {
        if let Some(cached) = self.local_hash.borrow().as_ref() {
            return Ok(cached.clone());
        }
        let content = std::fs::read_to_string(self.path).map_err(|error| {
            kernel::Error::Failed(format!(
                "could not read {}: {error}",
                self.path.display()
            ))
        })?;
        let hash = local(&content)?;
        *self.local_hash.borrow_mut() = Some(hash.clone());
        Ok(hash)
    }

    fn remote_body(&self) -> Result<Option<String>, kernel::Error> {
        if let Some(cached) = self.remote_hash.borrow().clone() {
            return Ok(cached);
        }
        let hash = self.remote_content.as_deref().map(remote_body);
        *self.remote_hash.borrow_mut() = Some(hash.clone());
        Ok(hash)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::local;
    use super::remote_body;

    #[test]
    fn local_hashes_the_composed_frontmatter_and_body() {
        let content = "---\nstatus: ready\n---\n\nBody text\n";
        let hash = local(content).expect("well-formed content hashes");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn local_ignores_provenance_fields() {
        let baseline = "---\nstatus: ready\n---\n\nBody text\n";
        let restamped = "---\nstatus: ready\nlast_updated: \"2026-01-01\"\n---\n\nBody text\n";
        assert_eq!(
            local(baseline).expect("baseline hashes"),
            local(restamped).expect("restamped hashes")
        );
    }

    #[test]
    fn local_errors_without_an_opening_fence() {
        assert!(local("no fence here\n").is_err());
    }

    #[test]
    fn local_errors_with_an_unclosed_frontmatter() {
        assert!(local("---\nstatus: ready\n").is_err());
    }

    #[test]
    fn local_errors_on_empty_content() {
        assert!(local("").is_err());
    }

    #[test]
    fn remote_body_is_deterministic_and_hex() {
        let hash = remote_body("Some projected body\n");
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, remote_body("Some projected body\n"));
    }

    #[test]
    fn remote_body_ignores_trailing_whitespace_differences() {
        assert_eq!(
            remote_body("Body text\n"),
            remote_body("Body text   \n\n\n")
        );
    }

    mod lazy {
        use work::sync::ItemDigests as _;

        use super::super::LazyItemDigests;

        #[test]
        fn local_reads_the_file_and_matches_the_pure_recipe(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let dir = tempfile::tempdir()?;
            let path = dir.path().join("item.md");
            let content = "---\nstatus: ready\n---\n\nBody text\n";
            std::fs::write(&path, content)?;

            let digests = LazyItemDigests::new(&path, None);

            assert_eq!(digests.local()?, super::local(content)?);
            Ok(())
        }

        #[test]
        fn remote_body_is_none_when_no_content_was_supplied(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let dir = tempfile::tempdir()?;
            let path = dir.path().join("item.md");
            std::fs::write(&path, "---\nstatus: ready\n---\n\nBody\n")?;

            let digests = LazyItemDigests::new(&path, None);

            assert_eq!(digests.remote_body()?, None);
            Ok(())
        }

        #[test]
        fn remote_body_hashes_the_supplied_content(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let dir = tempfile::tempdir()?;
            let path = dir.path().join("item.md");
            std::fs::write(&path, "---\nstatus: ready\n---\n\nBody\n")?;

            let digests =
                LazyItemDigests::new(&path, Some("Remote body\n".to_owned()));

            assert_eq!(
                digests.remote_body()?,
                Some(super::super::remote_body("Remote body\n"))
            );
            Ok(())
        }

        #[test]
        fn mtime_reflects_the_real_file(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let dir = tempfile::tempdir()?;
            let path = dir.path().join("item.md");
            std::fs::write(&path, "---\nstatus: ready\n---\n\nBody\n")?;

            let digests = LazyItemDigests::new(&path, None);

            assert!(digests.mtime()?.is_some());
            Ok(())
        }

        #[test]
        fn mtime_is_none_for_a_missing_file(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let dir = tempfile::tempdir()?;
            let path = dir.path().join("missing.md");

            let digests = LazyItemDigests::new(&path, None);

            assert_eq!(digests.mtime()?, None);
            Ok(())
        }
    }
}
