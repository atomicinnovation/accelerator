//! Reading a Markdown body as a faithful event stream.
//!
//! Tables, task lists and strikethrough are recognised; no other extension
//! is. Each event carries the byte range of the source it was read from. A
//! code block or HTML block arrives as one event holding its whole literal,
//! and every `End` repeats the tag its `Start` opened.

use std::ops::Range;

use pulldown_cmark::{
    CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag,
};

/// One Markdown event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownEvent {
    Start(MarkdownTag),
    End(MarkdownTag),
    Text(String),
    Code(String),
    InlineHtml(String),
    HtmlBlock(String),
    CodeBlock { info: String, literal: String },
    SoftBreak,
    HardBreak,
    Rule,
    TaskListMarker(bool),
}

/// A container a `Start` opens and its `End` closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownTag {
    Paragraph,
    Heading(u8),
    BlockQuote,
    List(Option<u64>),
    Item,
    Table(Vec<Alignment>),
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Link { destination: String, title: String },
    Image { destination: String, title: String },
}

/// A table column's alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

/// The events of `source`, each with the byte range it was read from.
#[must_use]
pub fn events(source: &str) -> Vec<(MarkdownEvent, Range<usize>)> {
    let mut reader = EventReader::default();
    for (event, span) in Parser::new_ext(source, options()).into_offset_iter() {
        reader.read(event, span);
    }
    reader.events
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
}

#[derive(Default)]
struct EventReader {
    events: Vec<(MarkdownEvent, Range<usize>)>,
    open_tags: Vec<Option<MarkdownTag>>,
    literal: Option<Literal>,
}

struct Literal {
    kind: LiteralKind,
    text: String,
    span: Range<usize>,
}

enum LiteralKind {
    Code { info: String },
    Html,
}

impl EventReader {
    fn read(&mut self, event: Event<'_>, span: Range<usize>) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                self.open_literal(
                    LiteralKind::Code { info: info(&kind) },
                    span,
                );
            }
            Event::Start(Tag::HtmlBlock) => {
                self.open_literal(LiteralKind::Html, span);
            }
            Event::End(
                pulldown_cmark::TagEnd::CodeBlock
                | pulldown_cmark::TagEnd::HtmlBlock,
            ) => self.close_literal(),
            Event::Text(text) | Event::Html(text) if self.literal.is_some() => {
                if let Some(literal) = &mut self.literal {
                    literal.text.push_str(&text);
                }
            }
            Event::Start(tag) => {
                let tag = markdown_tag(tag);
                if let Some(tag) = &tag {
                    self.events.push((MarkdownEvent::Start(tag.clone()), span));
                }
                self.open_tags.push(tag);
            }
            Event::End(_) => {
                if let Some(tag) = self.open_tags.pop().flatten() {
                    self.events.push((MarkdownEvent::End(tag), span));
                }
            }
            event => {
                if let Some(event) = leaf_event(event) {
                    self.events.push((event, span));
                }
            }
        }
    }

    fn open_literal(&mut self, kind: LiteralKind, span: Range<usize>) {
        self.literal = Some(Literal {
            kind,
            text: String::new(),
            span,
        });
    }

    fn close_literal(&mut self) {
        let Some(Literal { kind, text, span }) = self.literal.take() else {
            return;
        };
        let event = match kind {
            LiteralKind::Code { info } => MarkdownEvent::CodeBlock {
                info,
                literal: text,
            },
            LiteralKind::Html => MarkdownEvent::HtmlBlock(text),
        };
        self.events.push((event, span));
    }
}

fn info(kind: &CodeBlockKind<'_>) -> String {
    match kind {
        CodeBlockKind::Fenced(info) => info.to_string(),
        CodeBlockKind::Indented => String::new(),
    }
}

fn markdown_tag(tag: Tag<'_>) -> Option<MarkdownTag> {
    Some(match tag {
        Tag::Paragraph => MarkdownTag::Paragraph,
        Tag::Heading { level, .. } => {
            MarkdownTag::Heading(heading_level(level))
        }
        Tag::BlockQuote(_) => MarkdownTag::BlockQuote,
        Tag::List(start) => MarkdownTag::List(start),
        Tag::Item => MarkdownTag::Item,
        Tag::Table(alignments) => {
            MarkdownTag::Table(alignments.into_iter().map(alignment).collect())
        }
        Tag::TableHead => MarkdownTag::TableHead,
        Tag::TableRow => MarkdownTag::TableRow,
        Tag::TableCell => MarkdownTag::TableCell,
        Tag::Emphasis => MarkdownTag::Emphasis,
        Tag::Strong => MarkdownTag::Strong,
        Tag::Strikethrough => MarkdownTag::Strikethrough,
        Tag::Link {
            dest_url, title, ..
        } => MarkdownTag::Link {
            destination: dest_url.to_string(),
            title: title.to_string(),
        },
        Tag::Image {
            dest_url, title, ..
        } => MarkdownTag::Image {
            destination: dest_url.to_string(),
            title: title.to_string(),
        },
        Tag::CodeBlock(_)
        | Tag::HtmlBlock
        | Tag::FootnoteDefinition(_)
        | Tag::DefinitionList
        | Tag::DefinitionListTitle
        | Tag::DefinitionListDefinition
        | Tag::Superscript
        | Tag::Subscript
        | Tag::MetadataBlock(_) => return None,
    })
}

fn leaf_event(event: Event<'_>) -> Option<MarkdownEvent> {
    Some(match event {
        Event::Text(text) => MarkdownEvent::Text(text.to_string()),
        Event::Code(code) => MarkdownEvent::Code(code.to_string()),
        Event::InlineHtml(html) => MarkdownEvent::InlineHtml(html.to_string()),
        Event::Html(html) => MarkdownEvent::HtmlBlock(html.to_string()),
        Event::SoftBreak => MarkdownEvent::SoftBreak,
        Event::HardBreak => MarkdownEvent::HardBreak,
        Event::Rule => MarkdownEvent::Rule,
        Event::TaskListMarker(done) => MarkdownEvent::TaskListMarker(done),
        Event::Start(_)
        | Event::End(_)
        | Event::InlineMath(_)
        | Event::DisplayMath(_)
        | Event::FootnoteReference(_) => return None,
    })
}

const fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

const fn alignment(alignment: pulldown_cmark::Alignment) -> Alignment {
    match alignment {
        pulldown_cmark::Alignment::None => Alignment::None,
        pulldown_cmark::Alignment::Left => Alignment::Left,
        pulldown_cmark::Alignment::Center => Alignment::Center,
        pulldown_cmark::Alignment::Right => Alignment::Right,
    }
}
