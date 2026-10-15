//! Both digest recipes' fingerprints of a local file and a projected remote,
//! as every baseline writer records them.

use corpus::body::BodyParser;
use tracker::BodyDialect;
use work::sync::DualRecipeFingerprints;
use work::sync::LegacyHashes;
use work::sync::LocalFingerprints;
use work::sync::RemoteFingerprints;
use work::sync::SplitFingerprints;

use crate::sync::body_comparison::BodyComparison;
use crate::sync::digest;
use crate::sync::synced_fields;

pub struct BaselineDigester<'a> {
    comparison: BodyComparison<'a>,
}

impl<'a> BaselineDigester<'a> {
    #[must_use]
    pub fn new(
        dialect: &'a dyn BodyDialect,
        parser: &'a dyn BodyParser,
    ) -> Self {
        Self {
            comparison: BodyComparison::new(dialect, parser),
        }
    }

    #[must_use]
    pub const fn comparison(&self) -> &BodyComparison<'a> {
        &self.comparison
    }

    /// The legacy hash and the split fingerprints of one local file.
    ///
    /// # Errors
    ///
    /// When `content`'s frontmatter is missing, unterminated or not YAML.
    pub fn local(
        &self,
        content: &str,
    ) -> Result<(String, LocalFingerprints), kernel::Error> {
        let (_, body) = digest::split_frontmatter_and_body(content)?;
        let fields = synced_fields::local_fields(content)
            .map_err(|error| kernel::Error::Failed(error.to_string()))?;
        Ok((
            digest::local(content)?,
            LocalFingerprints {
                frontmatter: digest::frontmatter(content)?,
                fields: digest::fields(&fields),
                body: self.comparison.local_body_hash(&body),
            },
        ))
    }

    /// The legacy hash and the split fingerprints of one projected remote.
    #[must_use]
    pub fn remote(&self, projected: &str) -> (String, RemoteFingerprints) {
        (
            digest::remote_body(projected),
            RemoteFingerprints {
                body: self.comparison.remote_body_hash(projected),
                fields: digest::fields(&synced_fields::remote_fields(
                    projected,
                )),
            },
        )
    }

    /// Both recipes' fingerprints of the bodies a baseline entry records;
    /// `remote_projection` is `None` when the remote could not be read.
    ///
    /// # Errors
    ///
    /// As [`Self::local`].
    pub fn digests(
        &self,
        local_content: &str,
        remote_projection: Option<&str>,
    ) -> Result<DualRecipeFingerprints, kernel::Error> {
        let (legacy_local, local) = self.local(local_content)?;
        let (legacy_remote, remote) = remote_projection
            .map(|projected| self.remote(projected))
            .unzip();
        Ok(DualRecipeFingerprints {
            split: SplitFingerprints {
                local: Some(local),
                remote,
            },
            legacy: LegacyHashes {
                local: Some(legacy_local),
                remote: legacy_remote,
            },
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use corpus_adapters::CommonMarkBody;
    use tracker::PlainDialect;
    use work::sync::LegacyHashes;

    use super::BaselineDigester;
    use crate::sync::body_comparison::BodyComparison;
    use crate::sync::digest;

    const CONTENT: &str =
        "---\ntitle: \"Add search\"\nstatus: ready\n---\n\n# Add search\n\n* one\n";
    const PROJECTED: &str = "Add search\n# Add search\n\n- one\n";

    fn digester() -> BaselineDigester<'static> {
        BaselineDigester::new(&PlainDialect, &CommonMarkBody)
    }

    #[test]
    fn a_recipe_one_entry_carries_legacy_hashes_of_the_recorded_bodies(
    ) -> Result<(), kernel::Error> {
        let recorded = digester().digests(CONTENT, Some(PROJECTED))?;

        assert_eq!(
            recorded.legacy,
            LegacyHashes {
                local: Some(digest::local(CONTENT)?),
                remote: Some(digest::remote_body(PROJECTED)),
            }
        );
        Ok(())
    }

    #[test]
    fn the_split_fingerprints_digest_each_part_on_its_own(
    ) -> Result<(), kernel::Error> {
        let comparison = BodyComparison::new(&PlainDialect, &CommonMarkBody);
        let recorded = digester().digests(CONTENT, Some(PROJECTED))?;
        let local = recorded.split.local.expect("local fingerprints");
        let remote = recorded.split.remote.expect("remote fingerprints");

        assert_eq!(local.frontmatter, digest::frontmatter(CONTENT)?);
        assert_eq!(
            local.body,
            comparison.local_body_hash("\n# Add search\n\n* one")
        );
        assert_eq!(remote.body, comparison.remote_body_hash(PROJECTED));
        assert_eq!(local.fields, remote.fields);
        assert_eq!(local.body, remote.body);
        Ok(())
    }

    #[test]
    fn an_unread_remote_records_no_remote_fingerprints(
    ) -> Result<(), kernel::Error> {
        let recorded = digester().digests(CONTENT, None)?;

        assert_eq!(recorded.split.remote, None);
        assert_eq!(recorded.legacy.remote, None);
        assert!(recorded.split.local.is_some());
        Ok(())
    }

    #[test]
    fn malformed_local_content_is_an_error() {
        assert!(digester().digests("no frontmatter\n", None).is_err());
    }
}
