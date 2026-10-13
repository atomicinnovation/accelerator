//! A Markdown body as the corpus sees it, and the port that parses one.
//!
//! A tight list item holds its inline content as a `Paragraph`, as a loose
//! one does, so tightness lives only in [`Block::List`]'s `loose`.

use std::ops::Range;

/// A parsed Markdown body: its source text and its top-level blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    source: String,
    blocks: Vec<SourcedBlock>,
}

/// A top-level block and the byte range of the source it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcedBlock {
    pub block: Block,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    List {
        start: Option<u64>,
        loose: bool,
        items: Vec<ListItem>,
    },
    CodeBlock {
        info: String,
        literal: String,
    },
    BlockQuote(Vec<Block>),
    Table {
        alignments: Vec<Alignment>,
        rows: Vec<TableRow>,
    },
    ThematicBreak,
    Html(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

/// A table row; the header row is the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow(pub Vec<TableCell>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCell(pub Vec<Inline>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub task: Option<TaskMarker>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskMarker {
    Open,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Code(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Link {
        destination: String,
        title: String,
        children: Vec<Inline>,
    },
    Image {
        destination: String,
        title: String,
        alt: Vec<Inline>,
    },
    SoftBreak,
    HardBreak,
    Html(String),
}

/// A run of top-level blocks opened by a level-2 heading, or the preamble
/// before the first one. Consecutive sections tile the body's source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section<'a> {
    /// The opening heading's source text; `None` for the preamble.
    pub heading: Option<&'a str>,
    pub source: &'a str,
    pub blocks: &'a [SourcedBlock],
}

/// Parses Markdown into a [`Body`].
pub trait BodyParser {
    fn parse(&self, markdown: &str) -> Body;
}

impl Body {
    #[must_use]
    pub fn new(source: impl Into<String>, blocks: Vec<SourcedBlock>) -> Self {
        Self {
            source: source.into(),
            blocks,
        }
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn blocks(&self) -> &[SourcedBlock] {
        &self.blocks
    }

    /// The preamble, always first and possibly empty, then one section per
    /// level-2 heading.
    #[must_use]
    pub fn sections(&self) -> Vec<Section<'_>> {
        let openings: Vec<usize> = self
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, sourced)| sourced.opens_a_section())
            .map(|(index, _)| index)
            .collect();
        let mut sections = Vec::with_capacity(openings.len() + 1);
        let mut first_block = 0;
        let mut heading = None;
        for opening in openings.iter().copied().chain([self.blocks.len()]) {
            let start = self.source_start_of(first_block);
            let end = self.source_start_of(opening);
            sections.push(Section {
                heading,
                source: self.slice(start..end),
                blocks: self
                    .blocks
                    .get(first_block..opening)
                    .unwrap_or_default(),
            });
            heading = self
                .blocks
                .get(opening)
                .map(|sourced| self.slice(sourced.span.clone()));
            first_block = opening;
        }
        sections
    }

    fn source_start_of(&self, block: usize) -> usize {
        match block {
            0 => 0,
            _ => self
                .blocks
                .get(block)
                .map_or(self.source.len(), |sourced| sourced.span.start),
        }
    }

    fn slice(&self, range: Range<usize>) -> &str {
        self.source.get(range).unwrap_or_default()
    }
}

impl SourcedBlock {
    const fn opens_a_section(&self) -> bool {
        matches!(self.block, Block::Heading { level: 2, .. })
    }
}

#[cfg(test)]
mod tests {
    use super::{Block, Body, Inline, SourcedBlock};

    fn heading(level: u8, text: &str, source: &str) -> SourcedBlock {
        sourced(
            Block::Heading {
                level,
                inlines: vec![Inline::Text(text.to_owned())],
            },
            source,
            &format!("{} {text}", "#".repeat(usize::from(level))),
        )
    }

    fn paragraph(text: &str, source: &str) -> SourcedBlock {
        sourced(
            Block::Paragraph(vec![Inline::Text(text.to_owned())]),
            source,
            text,
        )
    }

    fn sourced(block: Block, source: &str, line: &str) -> SourcedBlock {
        let start = source.find(line).unwrap_or_default();
        SourcedBlock {
            block,
            span: start..start + line.len() + 1,
        }
    }

    fn body(source: &str, blocks: Vec<SourcedBlock>) -> Body {
        Body::new(source, blocks)
    }

    fn two_sections() -> Body {
        let source = "# Title\nintro\n## One\nfirst\n## Two\nsecond\n";
        body(
            source,
            vec![
                heading(1, "Title", source),
                paragraph("intro", source),
                heading(2, "One", source),
                paragraph("first", source),
                heading(2, "Two", source),
                paragraph("second", source),
            ],
        )
    }

    #[test]
    fn sections_split_on_level_two_headings() {
        let body = two_sections();
        let headings: Vec<_> = body
            .sections()
            .iter()
            .map(|section| section.heading)
            .collect();
        assert_eq!(headings, vec![None, Some("## One\n"), Some("## Two\n")]);
        let block_counts: Vec<_> = body
            .sections()
            .iter()
            .map(|section| section.blocks.len())
            .collect();
        assert_eq!(block_counts, vec![2, 2, 2]);
    }

    #[test]
    fn text_before_the_first_level_two_heading_is_the_preamble() {
        let body = two_sections();
        let sections = body.sections();
        let preamble = sections.first();
        assert_eq!(preamble.map(|section| section.heading), Some(None));
        assert_eq!(
            preamble.map(|section| section.source),
            Some("# Title\nintro\n")
        );
    }

    #[test]
    fn a_section_keeps_its_source_text() {
        let body = two_sections();
        let sources: Vec<_> = body
            .sections()
            .iter()
            .map(|section| section.source)
            .collect();
        assert_eq!(
            sources,
            vec!["# Title\nintro\n", "## One\nfirst\n", "## Two\nsecond\n"]
        );
    }

    #[test]
    fn a_body_without_level_two_headings_is_all_preamble() {
        let source = "# Title\n### Deeper\ntext\n";
        let body = body(
            source,
            vec![
                heading(1, "Title", source),
                heading(3, "Deeper", source),
                paragraph("text", source),
            ],
        );
        let sections = body.sections();
        assert_eq!(sections.len(), 1);
        assert_eq!(
            sections.first().map(|section| section.source),
            Some(source)
        );
        assert_eq!(
            sections.first().map(|section| section.blocks.len()),
            Some(3)
        );
    }
}
