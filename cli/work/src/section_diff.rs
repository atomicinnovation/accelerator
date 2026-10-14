//! Section comparison for conflict-resolution diffing.
//!
//! Sections pair and compare by meaning, over each side's `compared` body,
//! and display the text each side's `shown` body holds.

use corpus::body::Body;
use corpus::body::CanonicalBody;
use corpus::body::Section;

use crate::normalise::trim_lines;

/// One side of a comparison: its raw frontmatter, the body a reader sees,
/// and the body whose meaning decides whether a section differs.
#[derive(Debug, Clone, Copy)]
pub struct ComparableItem<'a> {
    pub frontmatter: &'a str,
    pub shown: &'a Body,
    pub compared: &'a Body,
}

/// A section whose meaning differs between local and remote, carrying each
/// side's displayed text, heading included; empty where a side lacks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionDiff {
    pub name: String,
    pub local: String,
    pub remote: String,
}

/// The raw frontmatter and body of a work-item-shaped document.
///
/// Content not opening with `---` is all body; frontmatter that never closes
/// leaves both empty.
#[must_use]
pub fn frontmatter_and_body(content: &str) -> (String, String) {
    let mut lines = content.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return (String::new(), content.to_owned());
    }
    let mut frontmatter = String::new();
    for line in lines.by_ref() {
        if line.trim_end() == "---" {
            let mut body = String::new();
            for line in lines {
                body.push_str(line);
                body.push('\n');
            }
            return (frontmatter, body);
        }
        frontmatter.push_str(line);
        frontmatter.push('\n');
    }
    (String::new(), String::new())
}

/// The sections that differ between `local` and `remote`: the frontmatter
/// by trimmed text, then the preamble and each level-2 section by meaning,
/// in local then remote order.
///
/// A change limited to `last_updated` or `revision` still shows as a
/// differing frontmatter.
#[must_use]
pub fn differing_sections(
    local: &ComparableItem<'_>,
    remote: &ComparableItem<'_>,
) -> Vec<SectionDiff> {
    let mut out = Vec::new();
    let local_frontmatter = trim_lines(local.frontmatter);
    let remote_frontmatter = trim_lines(remote.frontmatter);
    if local_frontmatter != remote_frontmatter {
        out.push(SectionDiff {
            name: "frontmatter".to_owned(),
            local: local_frontmatter,
            remote: remote_frontmatter,
        });
    }

    let local_sections = KeyedSection::all(local);
    let remote_sections = KeyedSection::all(remote);
    for section in &local_sections {
        let counterpart = remote_sections
            .iter()
            .find(|candidate| candidate.pairs_with(section));
        if counterpart.is_some_and(|other| other.meaning == section.meaning) {
            continue;
        }
        out.push(SectionDiff {
            name: section.name(),
            local: section.displayed_text(),
            remote: counterpart
                .map(KeyedSection::displayed_text)
                .unwrap_or_default(),
        });
    }
    for section in &remote_sections {
        if !local_sections
            .iter()
            .any(|candidate| candidate.pairs_with(section))
        {
            out.push(SectionDiff {
                name: section.name(),
                local: String::new(),
                remote: section.displayed_text(),
            });
        }
    }
    out
}

/// A section identified by its heading's meaning and its occurrence among
/// sections sharing that heading.
struct KeyedSection<'a> {
    heading: Option<CanonicalBody>,
    occurrence: usize,
    meaning: CanonicalBody,
    displayed: Section<'a>,
}

impl<'a> KeyedSection<'a> {
    /// The `compared` body's sections, each displayed by its `shown`
    /// counterpart when both bodies hold the same number of sections.
    fn all(item: &ComparableItem<'a>) -> Vec<Self> {
        let compared = item.compared.sections();
        let shown = item.shown.sections();
        let mut keyed: Vec<Self> = Vec::with_capacity(compared.len());
        for (index, section) in compared.iter().enumerate() {
            let heading = section.canonical_heading();
            let occurrence = keyed
                .iter()
                .filter(|earlier| earlier.heading == heading)
                .count();
            let displayed = if shown.len() == compared.len() {
                shown.get(index).copied().unwrap_or(*section)
            } else {
                *section
            };
            keyed.push(Self {
                heading,
                occurrence,
                meaning: section.canonical(),
                displayed,
            });
        }
        keyed
    }

    fn pairs_with(&self, other: &Self) -> bool {
        self.heading == other.heading && self.occurrence == other.occurrence
    }

    fn name(&self) -> String {
        self.displayed.heading.map_or_else(
            || "(preamble)".to_owned(),
            |heading| {
                heading
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .trim_matches('#')
                    .trim()
                    .to_owned()
            },
        )
    }

    fn displayed_text(&self) -> String {
        trim_lines(self.displayed.source)
            .trim_start_matches('\n')
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use corpus::body::Block;
    use corpus::body::Body;
    use corpus::body::Inline;
    use corpus::body::SourcedBlock;

    use super::differing_sections;
    use super::frontmatter_and_body;
    use super::ComparableItem;
    use super::SectionDiff;

    fn text(text: &str) -> Inline {
        Inline::Text(text.to_owned())
    }

    fn heading(inlines: Vec<Inline>, source: &str) -> (Block, String) {
        (Block::Heading { level: 2, inlines }, source.to_owned())
    }

    fn titled(title: &str) -> (Block, String) {
        heading(vec![text(title)], &format!("## {title}\n"))
    }

    fn paragraph(content: &str) -> (Block, String) {
        (
            Block::Paragraph(vec![text(content)]),
            format!("{content}\n"),
        )
    }

    fn body(blocks: Vec<(Block, String)>) -> Body {
        let mut source = String::new();
        let mut sourced = Vec::new();
        for (block, text) in blocks {
            let start = source.len();
            source.push_str(&text);
            sourced.push(SourcedBlock {
                block,
                span: start..source.len(),
            });
        }
        Body::new(source, sourced)
    }

    fn item<'a>(frontmatter: &'a str, body: &'a Body) -> ComparableItem<'a> {
        ComparableItem {
            frontmatter,
            shown: body,
            compared: body,
        }
    }

    fn names(diffs: &[SectionDiff]) -> Vec<&str> {
        diffs.iter().map(|d| d.name.as_str()).collect()
    }

    #[test]
    fn a_document_splits_into_its_frontmatter_and_body() {
        assert_eq!(
            frontmatter_and_body("---\nstatus: draft\n---\n# Title\n"),
            ("status: draft\n".to_owned(), "# Title\n".to_owned())
        );
    }

    #[test]
    fn a_document_without_frontmatter_is_all_body() {
        assert_eq!(
            frontmatter_and_body("# Title\ntext"),
            (String::new(), "# Title\ntext".to_owned())
        );
    }

    #[test]
    fn an_unclosed_frontmatter_leaves_both_empty() {
        assert_eq!(
            frontmatter_and_body("---\nstatus: draft\n# Title\n"),
            (String::new(), String::new())
        );
    }

    #[test]
    fn frontmatter_only_change_is_reported() {
        let same = body(vec![titled("Summary"), paragraph("text")]);
        assert_eq!(
            names(&differing_sections(
                &item("status: draft\n", &same),
                &item("status: ready\n", &same)
            )),
            vec!["frontmatter"]
        );
    }

    #[test]
    fn frontmatter_only_last_updated_change_is_still_reported() {
        let same = body(vec![paragraph("text")]);
        assert_eq!(
            names(&differing_sections(
                &item("last_updated: \"2026-01-01T00:00:00Z\"\n", &same),
                &item("last_updated: \"2026-01-02T00:00:00Z\"\n", &same)
            )),
            vec!["frontmatter"]
        );
    }

    #[test]
    fn frontmatter_differing_only_in_trailing_whitespace_is_not_reported() {
        let same = body(vec![paragraph("text")]);
        assert!(differing_sections(
            &item("status: draft   \n", &same),
            &item("status: draft\n", &same)
        )
        .is_empty());
    }

    #[test]
    fn preamble_change_is_reported() {
        let local = body(vec![paragraph("local"), titled("Summary")]);
        let remote = body(vec![paragraph("remote"), titled("Summary")]);
        assert_eq!(
            names(&differing_sections(&item("", &local), &item("", &remote))),
            vec!["(preamble)"]
        );
    }

    #[test]
    fn a_displayed_section_drops_its_leading_blank_lines() {
        let local = body(vec![(
            Block::Paragraph(vec![text("local")]),
            "\n\nlocal\n".to_owned(),
        )]);
        let remote = body(vec![paragraph("remote")]);
        let diffs = differing_sections(&item("", &local), &item("", &remote));
        assert_eq!(diffs[0].local, "local\n");
    }

    #[test]
    fn an_equivalent_section_is_omitted() {
        let local = body(vec![
            titled("Summary"),
            (Block::Paragraph(vec![text("a b")]), "a b\n".to_owned()),
            titled("Notes"),
            paragraph("local"),
        ]);
        let remote = body(vec![
            titled("Summary"),
            (
                Block::Paragraph(vec![text("a"), Inline::SoftBreak, text("b")]),
                "a\nb\n".to_owned(),
            ),
            titled("Notes"),
            paragraph("remote"),
        ]);
        let diffs = differing_sections(&item("", &local), &item("", &remote));
        assert_eq!(names(&diffs), vec!["Notes"]);
        assert_eq!(diffs[0].local, "## Notes\nlocal\n");
        assert_eq!(diffs[0].remote, "## Notes\nremote\n");
    }

    #[test]
    fn a_section_absent_on_one_side_is_listed() {
        let local = body(vec![
            titled("Summary"),
            paragraph("same"),
            titled("Local only"),
            paragraph("mine"),
        ]);
        let remote = body(vec![
            titled("Summary"),
            paragraph("same"),
            titled("Remote only"),
            paragraph("theirs"),
        ]);
        let diffs = differing_sections(&item("", &local), &item("", &remote));
        assert_eq!(names(&diffs), vec!["Local only", "Remote only"]);
        assert_eq!(diffs[0].remote, "");
        assert_eq!(diffs[1].local, "");
        assert_eq!(diffs[1].remote, "## Remote only\ntheirs\n");
    }

    #[test]
    fn sections_whose_headings_differ_only_cosmetically_pair_and_show_their_own_headings(
    ) {
        let local = body(vec![
            heading(
                vec![Inline::Strong(vec![text("Open questions")])],
                "## **Open questions**\n",
            ),
            paragraph("local"),
        ]);
        let remote = body(vec![
            heading(
                vec![
                    Inline::Strong(vec![text("Open")]),
                    text(" "),
                    Inline::Strong(vec![text("questions")]),
                ],
                "## **Open** **questions**\n",
            ),
            paragraph("remote"),
        ]);
        let diffs = differing_sections(&item("", &local), &item("", &remote));
        assert_eq!(names(&diffs), vec!["**Open questions**"]);
        assert_eq!(diffs[0].local, "## **Open questions**\nlocal\n");
        assert_eq!(diffs[0].remote, "## **Open** **questions**\nremote\n");
    }

    #[test]
    fn the_compared_body_decides_and_the_shown_body_is_displayed() {
        let shown = body(vec![
            titled("Table"),
            paragraph("| a |\n|:-|"),
            titled("Notes"),
            paragraph("local as written"),
        ]);
        let compared = body(vec![
            titled("Table"),
            paragraph("| a |\n|-|"),
            titled("Notes"),
            paragraph("local as pushed"),
        ]);
        let remote = body(vec![
            titled("Table"),
            paragraph("| a |\n|-|"),
            titled("Notes"),
            paragraph("remote"),
        ]);
        let local = ComparableItem {
            frontmatter: "",
            shown: &shown,
            compared: &compared,
        };
        let diffs = differing_sections(&local, &item("", &remote));
        assert_eq!(names(&diffs), vec!["Notes"]);
        assert_eq!(diffs[0].local, "## Notes\nlocal as written\n");
    }
}
