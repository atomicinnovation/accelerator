//! The hash functions behind both digest recipes the sync baseline stores.

use sha2::Digest as _;
use sha2::Sha256;
use work::sync::SyncedTrackerFields;

/// Recognises `^---[[:space:]]*$`, deliberately not `document::fence::split`:
/// `document` compares the exact bytes `---` and caps its scan at 1 MiB, so
/// a fence carrying a trailing space would make it treat the whole file as
/// body, silently promoting a routine `last_updated` restamp into a content
/// change.
fn is_fence(line: &str) -> bool {
    line.starts_with("---") && line[3..].chars().all(char::is_whitespace)
}

/// Splits `content` into its raw frontmatter and body at the two fence
/// lines, erroring rather than treating malformed content as body-only.
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
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
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
/// Both separators are emitted unconditionally, whichever side is empty.
/// The digest is opaque and only ever compared against one from the same
/// recipe, so the blank line an empty body leaves is arbitrary rather than
/// wrong; making the separator conditional would instead change every
/// affected item's hash and mass-reclassify the corpus.
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

/// The remote-side hash. `projected_body` is the un-normalised
/// `RemoteIssue.body`; normalising it is this recipe's own job.
#[must_use]
pub fn remote_body(projected_body: &str) -> String {
    let trimmed = work::normalise::trim_lines(projected_body);
    sha256_hex(trimmed.as_bytes())
}

/// The recipe-1 frontmatter digest: `filter_frontmatter_keys` over the
/// frontmatter alone, so a provenance restamp moves nothing.
///
/// # Errors
///
/// As [`local`].
pub fn frontmatter(file_content: &str) -> Result<String, kernel::Error> {
    let (frontmatter, _) = split_frontmatter_and_body(file_content)?;
    let filtered = work::normalise::filter_frontmatter_keys(&frontmatter);
    Ok(sha256_hex(filtered.as_bytes()))
}

/// The recipe-1 digest of the tracker fields sync owns, the same for
/// either side's reading of them.
#[must_use]
pub fn fields(fields: &SyncedTrackerFields) -> String {
    sha256_hex(fields.digest_input().as_bytes())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use corpus_adapters::CommonMarkBody;
    use tracker::PlainDialect;

    use super::fields;
    use super::frontmatter;
    use super::local;
    use super::remote_body;
    use crate::sync::body_comparison::BodyComparison;
    use crate::sync::synced_fields::local_fields;
    use crate::sync::synced_fields::remote_fields;

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
    fn the_frontmatter_hash_ignores_provenance_keys(
    ) -> Result<(), kernel::Error> {
        let baseline = "---\nstatus: ready\n---\n\nBody text\n";
        let restamped = "---\nstatus: ready\nlast_updated: \"2026-01-01\"\n\
                         ---\n\nOther body\n";
        assert_eq!(frontmatter(baseline)?, frontmatter(restamped)?);
        assert_ne!(
            frontmatter(baseline)?,
            frontmatter("---\nstatus: done\n---\n\nBody text\n")?
        );
        Ok(())
    }

    #[test]
    fn the_remote_fields_hash_digests_the_synced_fields_in_key_order() {
        let projected = "Add search\nBody\n";
        assert_eq!(
            fields(&remote_fields(projected)),
            super::sha256_hex(b"title=10:Add search\n")
        );
    }

    #[test]
    fn local_and_remote_fields_with_the_same_title_hash_equal(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content =
            "---\ntitle: \"Add search\"\nstatus: ready\n---\n\nBody\n";
        assert_eq!(
            fields(&local_fields(content)?),
            fields(&remote_fields("Add search\nOther body\n"))
        );
        Ok(())
    }

    #[test]
    fn a_local_status_edit_leaves_the_local_fields_hash_unchanged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ready = "---\ntitle: Add search\nstatus: ready\n---\n\nBody\n";
        let done = "---\ntitle: Add search\nstatus: done\n---\n\nBody\n";
        assert_eq!(fields(&local_fields(ready)?), fields(&local_fields(done)?));
        assert_ne!(frontmatter(ready)?, frontmatter(done)?);
        Ok(())
    }

    #[test]
    fn a_remote_title_change_alone_changes_only_the_remote_fields_hash() {
        let comparison = BodyComparison::new(&PlainDialect, &CommonMarkBody);
        let before = "Add search\n# Heading\n";
        let retitled = "Add search everywhere\n# Heading\n";

        assert_eq!(
            comparison.remote_body_hash(before),
            comparison.remote_body_hash(retitled)
        );
        assert_ne!(
            fields(&remote_fields(before)),
            fields(&remote_fields(retitled))
        );
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
}
