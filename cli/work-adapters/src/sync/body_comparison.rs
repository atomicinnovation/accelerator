//! The body rules sync and promotion apply, over the tracker's dialect.

use corpus::body::equivalent;
use corpus::body::BodyParser;
use remote_projection::split_projected;
use serde_json::Value;
use tracker::BodyDialect;
use tracker::Unrenderable;
use work::section_diff::frontmatter_and_body;
use work::section_diff::ComparableItem;
use work::section_diff::SectionDiff;
use work::sync::BodyEquivalence;
use work::sync::PullResidue;

use crate::sync::digest;

/// The single owner of how a tracker's projected body relates to a local
/// work item's body.
pub struct BodyComparison<'a> {
    dialect: &'a dyn BodyDialect,
    parser: &'a dyn BodyParser,
}

impl<'a> BodyComparison<'a> {
    #[must_use]
    pub fn new(
        dialect: &'a dyn BodyDialect,
        parser: &'a dyn BodyParser,
    ) -> Self {
        Self { dialect, parser }
    }

    /// The recipe-1 digest of a local body, taken over the body the tracker
    /// would store were it pushed.
    #[must_use]
    pub fn local_body_hash(&self, body: &str) -> String {
        self.canonical_hash(&self.dialect.as_pushed(body))
    }

    /// The recipe-1 digest of a projected remote: its rendered local body,
    /// or, when it has none, its description tagged as unrenderable.
    #[must_use]
    pub fn remote_body_hash(&self, projected: &str) -> String {
        self.local_body(projected).map_or_else(
            |_| {
                digest::sha256_hex(
                    format!(
                        "unrenderable:{}",
                        split_projected(projected).description
                    )
                    .as_bytes(),
                )
            },
            |rendered| self.canonical_hash(&rendered),
        )
    }

    /// Whether `local_body`, as the tracker would store it once pushed,
    /// means what the `projected` remote's description does.
    #[must_use]
    pub fn equivalence(
        &self,
        local_body: &str,
        projected: &str,
    ) -> BodyEquivalence {
        self.local_body(projected)
            .map_or(BodyEquivalence::RemoteUnrenderable, |rendered| {
                self.equivalence_to_rendered(local_body, &rendered)
            })
    }

    /// As [`Self::equivalence`], against a remote already rendered to its
    /// local body.
    #[must_use]
    pub fn equivalence_to_rendered(
        &self,
        local_body: &str,
        rendered_remote: &str,
    ) -> BodyEquivalence {
        if self.dialect.rejects(local_body) {
            return BodyEquivalence::Different;
        }
        let local = self.parser.parse(&self.dialect.as_pushed(local_body));
        if equivalent(&local, &self.parser.parse(rendered_remote)) {
            BodyEquivalence::Equivalent
        } else {
            BodyEquivalence::Different
        }
    }

    /// The sections of `local_content` whose meaning, once pushed, differs
    /// from `remote_content`'s, each shown as its own side wrote it.
    #[must_use]
    pub fn differing_sections(
        &self,
        local_content: &str,
        remote_content: &str,
    ) -> Vec<SectionDiff> {
        let (local_frontmatter, local_body) =
            frontmatter_and_body(local_content);
        let (remote_frontmatter, remote_body) =
            frontmatter_and_body(remote_content);
        let local_shown = self.parser.parse(&local_body);
        let local_compared =
            self.parser.parse(&self.dialect.as_pushed(&local_body));
        let remote = self.parser.parse(&remote_body);
        work::section_diff::differing_sections(
            &ComparableItem {
                frontmatter: &local_frontmatter,
                shown: &local_shown,
                compared: &local_compared,
            },
            &ComparableItem {
                frontmatter: &remote_frontmatter,
                shown: &remote,
                compared: &remote,
            },
        )
    }

    fn canonical_hash(&self, markdown: &str) -> String {
        digest::sha256_hex(
            self.parser
                .parse(markdown)
                .canonical()
                .digest_input()
                .as_bytes(),
        )
    }

    /// The local body a pull of `projected` writes: the description below
    /// the projected title line, rendered through the tracker's dialect.
    ///
    /// # Errors
    ///
    /// [`Unrenderable`] when the description has no Markdown form.
    pub fn local_body(&self, projected: &str) -> Result<String, Unrenderable> {
        self.dialect
            .local_body(split_projected(projected).description)
    }

    /// The residue `local_body` carries, against the current `projected`
    /// remote.
    ///
    /// The body is read with and without its first line, so a title line an
    /// earlier pull wrote, and a later retitle made stale, does not hide the
    /// document beneath it.
    #[must_use]
    pub fn pull_residue(
        &self,
        local_body: &str,
        projected: &str,
    ) -> Option<PullResidue> {
        let local_body = local_body.trim_start_matches('\n');
        let below_first_line =
            local_body.split_once('\n').map(|(_, rest)| rest);
        let document = [Some(local_body), below_first_line]
            .into_iter()
            .flatten()
            .find(|candidate| self.dialect.is_wire_document(candidate))?;
        let remote = split_projected(projected).description;
        Some(if same_document(document, remote) {
            PullResidue::VerbatimRemote
        } else {
            PullResidue::TrackerWireDocument
        })
    }
}

fn same_document(left: &str, right: &str) -> bool {
    match (
        serde_json::from_str::<Value>(left.trim()),
        serde_json::from_str::<Value>(right.trim()),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => left.trim() == right.trim(),
    }
}

#[cfg(test)]
mod tests {
    use corpus_adapters::CommonMarkBody;
    use tracker::BodyDialect;
    use tracker::PlainDialect;
    use work::sync::PullResidue;

    use super::BodyComparison;

    struct JsonDocumentDialect;

    impl BodyDialect for JsonDocumentDialect {
        fn is_wire_document(&self, body: &str) -> bool {
            serde_json::from_str::<serde_json::Value>(body.trim())
                .is_ok_and(|document| document["type"] == "doc")
        }
    }

    const REMOTE_DOCUMENT: &str =
        "{\"content\":[],\"type\":\"doc\",\"version\":1}";
    const REORDERED_REMOTE_DOCUMENT: &str =
        "{\"version\":1,\"type\":\"doc\",\"content\":[]}";
    const OTHER_DOCUMENT: &str =
        "{\"content\":[{\"type\":\"rule\"}],\"type\":\"doc\",\"version\":1}";

    fn remote() -> String {
        format!("Title\n{REMOTE_DOCUMENT}\n")
    }

    fn residue(
        dialect: &dyn BodyDialect,
        local_body: &str,
        projected: &str,
    ) -> Option<PullResidue> {
        BodyComparison::new(dialect, &CommonMarkBody)
            .pull_residue(local_body, projected)
    }

    #[test]
    fn a_jira_adf_document_body_is_residue() {
        assert_eq!(
            residue(
                &JsonDocumentDialect,
                &format!("{OTHER_DOCUMENT}\n"),
                &remote()
            ),
            Some(PullResidue::TrackerWireDocument)
        );
    }

    #[test]
    fn a_stale_title_line_over_an_adf_document_is_residue() {
        assert_eq!(
            residue(
                &JsonDocumentDialect,
                &format!("Old title\n{OTHER_DOCUMENT}\n"),
                &remote()
            ),
            Some(PullResidue::TrackerWireDocument)
        );
    }

    #[test]
    fn residue_below_the_blank_line_after_the_frontmatter_is_residue() {
        assert_eq!(
            residue(
                &JsonDocumentDialect,
                &format!("\nOld title\n{OTHER_DOCUMENT}\n"),
                &remote()
            ),
            Some(PullResidue::TrackerWireDocument)
        );
    }

    #[test]
    fn a_description_opening_with_its_title_is_not_residue() {
        assert_eq!(
            residue(&JsonDocumentDialect, "Title\n\n# Heading\n", &remote()),
            None
        );
    }

    #[test]
    fn a_linear_title_line_over_the_current_description_is_not_residue() {
        assert_eq!(
            residue(
                &PlainDialect,
                "Title\nRemote body\n",
                "Title\nRemote body\n"
            ),
            None
        );
    }

    #[test]
    fn a_linear_body_equal_to_the_remote_is_not_residue() {
        assert_eq!(
            residue(&PlainDialect, "Remote body\n", "Title\nRemote body\n"),
            None
        );
    }

    #[test]
    fn a_json_code_block_in_markdown_is_not_residue() {
        assert_eq!(
            residue(
                &JsonDocumentDialect,
                &format!("```json\n{REMOTE_DOCUMENT}\n```\n"),
                &remote()
            ),
            None
        );
    }

    #[test]
    fn a_title_line_and_the_current_remote_document_is_a_verbatim_remote() {
        assert_eq!(
            residue(
                &JsonDocumentDialect,
                &format!("Title\n{REORDERED_REMOTE_DOCUMENT}\n"),
                &remote()
            ),
            Some(PullResidue::VerbatimRemote)
        );
    }

    #[test]
    fn a_wire_document_differing_from_the_remote_is_not_a_verbatim_remote() {
        assert_eq!(
            residue(
                &JsonDocumentDialect,
                &format!("Title\n{OTHER_DOCUMENT}\n"),
                &remote()
            ),
            Some(PullResidue::TrackerWireDocument)
        );
    }
}
