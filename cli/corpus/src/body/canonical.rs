//! The canonical form bodies are compared and digested by.
//!
//! Soft breaks read as spaces; inline marks apply per character, never to
//! whitespace, and emphasis never to code; a link whose text is its
//! destination reads as that text; list tightness and bullet characters are
//! forgotten. Code-span and code-block text is kept exactly as parsed.

use crate::body::Alignment;
use crate::body::Block;
use crate::body::Body;
use crate::body::Inline;
use crate::body::ListItem;
use crate::body::TableRow;
use crate::body::TaskMarker;

/// The meaning of a body, with every difference that is only notation
/// removed. Two bodies are equivalent exactly when their canonical forms are
/// equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalBody(Vec<CanonicalBlock>);

#[derive(Debug, Clone, PartialEq, Eq)]
enum CanonicalBlock {
    Heading {
        level: u8,
        segments: Vec<Segment>,
    },
    Paragraph(Vec<Segment>),
    List {
        start: Option<u64>,
        items: Vec<CanonicalItem>,
    },
    CodeBlock {
        info: String,
        literal: String,
    },
    BlockQuote(Vec<CanonicalBlock>),
    Table {
        alignments: Vec<Alignment>,
        rows: Vec<Vec<Vec<Segment>>>,
    },
    ThematicBreak,
    Html(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalItem {
    task: Option<TaskMarker>,
    blocks: Vec<CanonicalBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Text {
        text: String,
        marks: Marks,
    },
    Code {
        text: String,
        marks: Marks,
    },
    Link {
        destination: String,
        title: String,
        children: Vec<Segment>,
    },
    Image {
        destination: String,
        title: String,
        alt: Vec<Segment>,
    },
    HardBreak,
    Html(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Marks {
    emphasis: bool,
    strong: bool,
    strikethrough: bool,
}

impl Marks {
    const fn without_emphasis(self) -> Self {
        Self {
            emphasis: false,
            strong: false,
            ..self
        }
    }

    const fn applied_to(self, character: char) -> Self {
        if character.is_whitespace() {
            self.without_emphasis()
        } else {
            self
        }
    }
}

impl Body {
    #[must_use]
    pub fn canonical(&self) -> CanonicalBody {
        CanonicalBody(
            self.blocks()
                .iter()
                .map(|sourced| CanonicalBlock::of(&sourced.block))
                .collect(),
        )
    }
}

impl CanonicalBody {
    /// A serialisation that differs whenever the canonical forms differ.
    ///
    /// Baseline digests hash it, so it is a persisted format: a change that
    /// moves any digest needs a new digest recipe.
    #[must_use]
    pub fn digest_input(&self) -> String {
        let mut input = DigestInput::default();
        input.sequence(&self.0);
        input.0
    }
}

/// Whether two bodies mean the same, by [`CanonicalBody`].
#[must_use]
pub fn equivalent(left: &Body, right: &Body) -> bool {
    left.canonical() == right.canonical()
}

impl CanonicalBlock {
    fn of(block: &Block) -> Self {
        match block {
            Block::Heading { level, inlines } => Self::Heading {
                level: *level,
                segments: Segments::of(inlines),
            },
            Block::Paragraph(inlines) => Self::Paragraph(Segments::of(inlines)),
            Block::List { start, items, .. } => Self::List {
                start: *start,
                items: items.iter().map(CanonicalItem::of).collect(),
            },
            Block::CodeBlock { info, literal } => Self::CodeBlock {
                info: info.clone(),
                literal: literal.clone(),
            },
            Block::BlockQuote(blocks) => Self::BlockQuote(Self::all(blocks)),
            Block::Table { alignments, rows } => Self::Table {
                alignments: alignments.clone(),
                rows: rows.iter().map(canonical_row).collect(),
            },
            Block::ThematicBreak => Self::ThematicBreak,
            Block::Html(html) => Self::Html(html.clone()),
        }
    }

    fn all(blocks: &[Block]) -> Vec<Self> {
        blocks.iter().map(Self::of).collect()
    }
}

impl CanonicalItem {
    fn of(item: &ListItem) -> Self {
        Self {
            task: item.task,
            blocks: CanonicalBlock::all(&item.blocks),
        }
    }
}

fn canonical_row(row: &TableRow) -> Vec<Vec<Segment>> {
    row.0.iter().map(|cell| Segments::of(&cell.0)).collect()
}

#[derive(Default)]
struct Segments(Vec<Segment>);

impl Segments {
    fn of(inlines: &[Inline]) -> Vec<Segment> {
        Self::marked(inlines, Marks::default()).0
    }

    fn marked(inlines: &[Inline], marks: Marks) -> Self {
        let mut segments = Self::default();
        for inline in inlines {
            segments.push_inline(inline, marks);
        }
        segments
    }

    fn push_inline(&mut self, inline: &Inline, marks: Marks) {
        match inline {
            Inline::Text(text) => self.push_text(text, marks),
            Inline::SoftBreak => self.push_text(" ", marks),
            Inline::Code(text) => self.0.push(Segment::Code {
                text: text.clone(),
                marks: marks.without_emphasis(),
            }),
            Inline::Emphasis(children) => self.push_all(
                children,
                Marks {
                    emphasis: true,
                    ..marks
                },
            ),
            Inline::Strong(children) => self.push_all(
                children,
                Marks {
                    strong: true,
                    ..marks
                },
            ),
            Inline::Strikethrough(children) => self.push_all(
                children,
                Marks {
                    strikethrough: true,
                    ..marks
                },
            ),
            Inline::Link {
                destination,
                title,
                children,
            } => self.push_link(
                destination,
                title,
                Self::marked(children, marks),
            ),
            Inline::Image {
                destination,
                title,
                alt,
            } => self.0.push(Segment::Image {
                destination: destination.clone(),
                title: title.clone(),
                alt: Self::of(alt),
            }),
            Inline::HardBreak => self.0.push(Segment::HardBreak),
            Inline::Html(html) => self.0.push(Segment::Html(html.clone())),
        }
    }

    fn push_all(&mut self, inlines: &[Inline], marks: Marks) {
        for inline in inlines {
            self.push_inline(inline, marks);
        }
    }

    fn push_text(&mut self, text: &str, marks: Marks) {
        for character in text.chars() {
            let marks = marks.applied_to(character);
            match self.0.last_mut() {
                Some(Segment::Text {
                    text,
                    marks: previous,
                }) if *previous == marks => text.push(character),
                _ => self.0.push(Segment::Text {
                    text: character.to_string(),
                    marks,
                }),
            }
        }
    }

    fn push_link(&mut self, destination: &str, title: &str, children: Self) {
        if title.is_empty()
            && children.plain_text().as_deref() == Some(destination)
        {
            for segment in children.0 {
                self.push_segment(segment);
            }
        } else {
            self.0.push(Segment::Link {
                destination: destination.to_owned(),
                title: title.to_owned(),
                children: children.0,
            });
        }
    }

    fn push_segment(&mut self, segment: Segment) {
        match segment {
            Segment::Text { text, marks } => self.push_text(&text, marks),
            other => self.0.push(other),
        }
    }

    fn plain_text(&self) -> Option<String> {
        self.0
            .iter()
            .map(|segment| match segment {
                Segment::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// Writes each value as a tag, then its fields; strings carry their byte
/// length and sequences their count, so the output parses back uniquely.
#[derive(Default)]
struct DigestInput(String);

trait Digestible {
    fn write(&self, input: &mut DigestInput);
}

impl DigestInput {
    fn tag(&mut self, tag: char) {
        self.0.push(tag);
    }

    fn number(&mut self, number: &impl ToString) {
        self.0.push_str(&number.to_string());
        self.0.push(';');
    }

    fn text(&mut self, text: &str) {
        self.number(&text.len());
        self.0.push_str(text);
    }

    fn flag(&mut self, flag: bool) {
        self.tag(if flag { '1' } else { '0' });
    }

    fn sequence<T: Digestible>(&mut self, values: &[T]) {
        self.number(&values.len());
        for value in values {
            value.write(self);
        }
    }
}

impl<T: Digestible> Digestible for Vec<T> {
    fn write(&self, input: &mut DigestInput) {
        input.sequence(self);
    }
}

impl Digestible for CanonicalBlock {
    fn write(&self, input: &mut DigestInput) {
        match self {
            Self::Heading { level, segments } => {
                input.tag('H');
                input.number(level);
                input.sequence(segments);
            }
            Self::Paragraph(segments) => {
                input.tag('P');
                input.sequence(segments);
            }
            Self::List { start, items } => {
                input.tag('L');
                match start {
                    Some(start) => {
                        input.tag('o');
                        input.number(start);
                    }
                    None => input.tag('b'),
                }
                input.sequence(items);
            }
            Self::CodeBlock { info, literal } => {
                input.tag('C');
                input.text(info);
                input.text(literal);
            }
            Self::BlockQuote(blocks) => {
                input.tag('Q');
                input.sequence(blocks);
            }
            Self::Table { alignments, rows } => {
                input.tag('T');
                input.sequence(alignments);
                input.sequence(rows);
            }
            Self::ThematicBreak => input.tag('R'),
            Self::Html(html) => {
                input.tag('X');
                input.text(html);
            }
        }
    }
}

impl Digestible for CanonicalItem {
    fn write(&self, input: &mut DigestInput) {
        input.tag(match self.task {
            None => '-',
            Some(TaskMarker::Open) => ' ',
            Some(TaskMarker::Done) => 'x',
        });
        input.sequence(&self.blocks);
    }
}

impl Digestible for Segment {
    fn write(&self, input: &mut DigestInput) {
        match self {
            Self::Text { text, marks } => {
                input.tag('t');
                marks.write(input);
                input.text(text);
            }
            Self::Code { text, marks } => {
                input.tag('c');
                marks.write(input);
                input.text(text);
            }
            Self::Link {
                destination,
                title,
                children,
            } => {
                input.tag('a');
                input.text(destination);
                input.text(title);
                input.sequence(children);
            }
            Self::Image {
                destination,
                title,
                alt,
            } => {
                input.tag('i');
                input.text(destination);
                input.text(title);
                input.sequence(alt);
            }
            Self::HardBreak => input.tag('n'),
            Self::Html(html) => {
                input.tag('h');
                input.text(html);
            }
        }
    }
}

impl Digestible for Marks {
    fn write(&self, input: &mut DigestInput) {
        input.flag(self.emphasis);
        input.flag(self.strong);
        input.flag(self.strikethrough);
    }
}

impl Digestible for Alignment {
    fn write(&self, input: &mut DigestInput) {
        input.tag(match self {
            Self::None => '-',
            Self::Left => 'l',
            Self::Center => 'c',
            Self::Right => 'r',
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::body::{
        Alignment, Block, Body, Inline, ListItem, SourcedBlock, TableCell,
        TableRow, TaskMarker,
    };

    fn text(text: &str) -> Inline {
        Inline::Text(text.to_owned())
    }

    fn body(blocks: Vec<Block>) -> Body {
        Body::new(
            "",
            blocks
                .into_iter()
                .map(|block| SourcedBlock { block, span: 0..0 })
                .collect(),
        )
    }

    fn paragraph(inlines: Vec<Inline>) -> Body {
        body(vec![Block::Paragraph(inlines)])
    }

    fn every_kind_of_block() -> Body {
        body(vec![
            Block::Heading {
                level: 1,
                inlines: vec![text("Title")],
            },
            Block::Paragraph(vec![
                Inline::Strong(vec![text("bold "), Inline::Code("c".into())]),
                Inline::SoftBreak,
                Inline::Link {
                    destination: "u".into(),
                    title: String::new(),
                    children: vec![text("t")],
                },
                Inline::HardBreak,
                Inline::Image {
                    destination: "i".into(),
                    title: "x".into(),
                    alt: vec![text("alt")],
                },
                Inline::Html("<br>".into()),
            ]),
            Block::List {
                start: Some(3),
                loose: true,
                items: vec![ListItem {
                    task: Some(TaskMarker::Done),
                    blocks: vec![Block::Paragraph(vec![text("item")])],
                }],
            },
            Block::CodeBlock {
                info: "rust".into(),
                literal: "fn main() {}\n".into(),
            },
            Block::BlockQuote(vec![Block::Paragraph(vec![text("quoted")])]),
            Block::Table {
                alignments: vec![Alignment::Left],
                rows: vec![TableRow(vec![TableCell(vec![text("cell")])])],
            },
            Block::ThematicBreak,
            Block::Html("<div></div>".into()),
        ])
    }

    #[test]
    fn canonical_form_is_deterministic() {
        let body = every_kind_of_block();
        assert_eq!(body.canonical(), every_kind_of_block().canonical());
        assert_eq!(
            body.canonical().digest_input(),
            every_kind_of_block().canonical().digest_input()
        );
    }

    #[test]
    fn distinct_canonical_forms_have_distinct_digest_inputs() {
        let bodies = [
            paragraph(vec![text("ab")]),
            body(vec![
                Block::Paragraph(vec![text("a")]),
                Block::Paragraph(vec![text("b")]),
            ]),
            paragraph(vec![Inline::Code("ab".into())]),
            paragraph(vec![Inline::Strong(vec![text("ab")])]),
            paragraph(vec![text("a"), Inline::HardBreak, text("b")]),
            body(vec![Block::Heading {
                level: 2,
                inlines: vec![text("ab")],
            }]),
            body(vec![Block::CodeBlock {
                info: "a".into(),
                literal: "b".into(),
            }]),
            body(vec![Block::CodeBlock {
                info: String::new(),
                literal: "ab".into(),
            }]),
            body(vec![Block::Html("ab".into())]),
            body(vec![Block::BlockQuote(vec![Block::Paragraph(vec![text(
                "ab",
            )])])]),
            body(vec![Block::List {
                start: None,
                loose: false,
                items: vec![ListItem {
                    task: None,
                    blocks: vec![Block::Paragraph(vec![text("ab")])],
                }],
            }]),
            body(vec![Block::List {
                start: Some(1),
                loose: false,
                items: vec![ListItem {
                    task: None,
                    blocks: vec![Block::Paragraph(vec![text("ab")])],
                }],
            }]),
            every_kind_of_block(),
        ];
        for (index, left) in bodies.iter().enumerate() {
            for right in bodies.iter().skip(index + 1) {
                assert_ne!(left.canonical(), right.canonical());
                assert_ne!(
                    left.canonical().digest_input(),
                    right.canonical().digest_input(),
                    "{left:?} and {right:?} share a digest input"
                );
            }
        }
    }

    #[test]
    fn emphasis_on_whitespace_is_ignored() {
        let per_word = paragraph(vec![
            Inline::Emphasis(vec![text("a")]),
            text(" "),
            Inline::Emphasis(vec![text("b")]),
        ]);
        let spanning = paragraph(vec![Inline::Emphasis(vec![text("a b")])]);
        assert_eq!(per_word.canonical(), spanning.canonical());
    }
}
