//! Folding the `document` Markdown event stream into a `corpus::body::Body`.

use std::ops::Range;

use corpus::body::{
    Alignment, Block, Body, BodyParser, Inline, ListItem, SourcedBlock,
    TableCell, TableRow, TaskMarker,
};
use document::markdown::{self, MarkdownEvent, MarkdownTag};

/// The [`BodyParser`] over `document`'s Markdown reader.
#[derive(Debug, Clone, Copy, Default)]
pub struct CommonMarkBody;

impl BodyParser for CommonMarkBody {
    fn parse(&self, markdown: &str) -> Body {
        let mut cursor = Cursor {
            events: markdown::events(markdown),
            position: 0,
        };
        let mut blocks = Vec::new();
        while let Some((event, span)) = cursor.next() {
            if let Some(block) = cursor.block(event) {
                blocks.push(SourcedBlock { block, span });
            }
        }
        Body::new(markdown, blocks)
    }
}

struct Cursor {
    events: Vec<(MarkdownEvent, Range<usize>)>,
    position: usize,
}

struct Contents {
    blocks: Vec<Block>,
    holds_a_paragraph: bool,
}

impl Cursor {
    fn next(&mut self) -> Option<(MarkdownEvent, Range<usize>)> {
        let event = self.events.get(self.position).cloned();
        self.position += 1;
        event
    }

    fn peek(&self, offset: usize) -> Option<&MarkdownEvent> {
        self.events
            .get(self.position + offset)
            .map(|(event, _)| event)
    }

    fn block(&mut self, event: MarkdownEvent) -> Option<Block> {
        Some(match event {
            MarkdownEvent::Start(MarkdownTag::Paragraph) => {
                Block::Paragraph(self.inlines())
            }
            MarkdownEvent::Start(MarkdownTag::Heading(level)) => {
                Block::Heading {
                    level,
                    inlines: self.inlines(),
                }
            }
            MarkdownEvent::Start(MarkdownTag::BlockQuote) => {
                Block::BlockQuote(self.contents().blocks)
            }
            MarkdownEvent::Start(MarkdownTag::List(start)) => self.list(start),
            MarkdownEvent::Start(MarkdownTag::Table(alignments)) => {
                Block::Table {
                    alignments: alignments.into_iter().map(alignment).collect(),
                    rows: self.rows(),
                }
            }
            MarkdownEvent::CodeBlock { info, literal } => {
                Block::CodeBlock { info, literal }
            }
            MarkdownEvent::HtmlBlock(html) => Block::Html(html),
            MarkdownEvent::Rule => Block::ThematicBreak,
            _ => return None,
        })
    }

    fn contents(&mut self) -> Contents {
        let mut contents = Contents {
            blocks: Vec::new(),
            holds_a_paragraph: false,
        };
        let mut loose_inlines = Vec::new();
        while let Some((event, _)) = self.next() {
            if matches!(event, MarkdownEvent::End(_)) {
                break;
            }
            if is_inline(&event) {
                self.inline(event, &mut loose_inlines);
                continue;
            }
            if !loose_inlines.is_empty() {
                contents
                    .blocks
                    .push(Block::Paragraph(std::mem::take(&mut loose_inlines)));
            }
            contents.holds_a_paragraph |=
                event == MarkdownEvent::Start(MarkdownTag::Paragraph);
            contents.blocks.extend(self.block(event));
        }
        if !loose_inlines.is_empty() {
            contents.blocks.push(Block::Paragraph(loose_inlines));
        }
        contents
    }

    fn list(&mut self, start: Option<u64>) -> Block {
        let mut items = Vec::new();
        let mut loose = false;
        while let Some((event, _)) = self.next() {
            if event != MarkdownEvent::Start(MarkdownTag::Item) {
                break;
            }
            let task = self.task_marker();
            let contents = self.contents();
            loose |= contents.holds_a_paragraph;
            items.push(ListItem {
                task,
                blocks: contents.blocks,
            });
        }
        Block::List {
            start,
            loose,
            items,
        }
    }

    fn task_marker(&self) -> Option<TaskMarker> {
        let marker = match self.peek(0) {
            Some(MarkdownEvent::Start(MarkdownTag::Paragraph)) => self.peek(1),
            first => first,
        };
        match marker {
            Some(MarkdownEvent::TaskListMarker(true)) => Some(TaskMarker::Done),
            Some(MarkdownEvent::TaskListMarker(false)) => {
                Some(TaskMarker::Open)
            }
            _ => None,
        }
    }

    fn rows(&mut self) -> Vec<TableRow> {
        let mut rows = Vec::new();
        while let Some((event, _)) = self.next() {
            match event {
                MarkdownEvent::Start(
                    MarkdownTag::TableHead | MarkdownTag::TableRow,
                ) => rows.push(TableRow(self.cells())),
                _ => break,
            }
        }
        rows
    }

    fn cells(&mut self) -> Vec<TableCell> {
        let mut cells = Vec::new();
        while let Some((event, _)) = self.next() {
            match event {
                MarkdownEvent::Start(MarkdownTag::TableCell) => {
                    cells.push(TableCell(self.inlines()));
                }
                _ => break,
            }
        }
        cells
    }

    fn inlines(&mut self) -> Vec<Inline> {
        let mut inlines = Vec::new();
        while let Some((event, _)) = self.next() {
            if matches!(event, MarkdownEvent::End(_)) {
                break;
            }
            self.inline(event, &mut inlines);
        }
        inlines
    }

    fn inline(&mut self, event: MarkdownEvent, inlines: &mut Vec<Inline>) {
        let inline = match event {
            MarkdownEvent::Text(text) => {
                if let Some(Inline::Text(previous)) = inlines.last_mut() {
                    previous.push_str(&text);
                    return;
                }
                Inline::Text(text)
            }
            MarkdownEvent::Code(code) => Inline::Code(code),
            MarkdownEvent::InlineHtml(html) => Inline::Html(html),
            MarkdownEvent::SoftBreak => Inline::SoftBreak,
            MarkdownEvent::HardBreak => Inline::HardBreak,
            MarkdownEvent::Start(MarkdownTag::Emphasis) => {
                Inline::Emphasis(self.inlines())
            }
            MarkdownEvent::Start(MarkdownTag::Strong) => {
                Inline::Strong(self.inlines())
            }
            MarkdownEvent::Start(MarkdownTag::Strikethrough) => {
                Inline::Strikethrough(self.inlines())
            }
            MarkdownEvent::Start(MarkdownTag::Link { destination, title }) => {
                Inline::Link {
                    destination,
                    title,
                    children: self.inlines(),
                }
            }
            MarkdownEvent::Start(MarkdownTag::Image { destination, title }) => {
                Inline::Image {
                    destination,
                    title,
                    alt: self.inlines(),
                }
            }
            _ => return,
        };
        inlines.push(inline);
    }
}

const fn is_inline(event: &MarkdownEvent) -> bool {
    match event {
        MarkdownEvent::Start(tag) => matches!(
            tag,
            MarkdownTag::Emphasis
                | MarkdownTag::Strong
                | MarkdownTag::Strikethrough
                | MarkdownTag::Link { .. }
                | MarkdownTag::Image { .. }
        ),
        MarkdownEvent::Text(_)
        | MarkdownEvent::Code(_)
        | MarkdownEvent::InlineHtml(_)
        | MarkdownEvent::SoftBreak
        | MarkdownEvent::HardBreak
        | MarkdownEvent::TaskListMarker(_) => true,
        MarkdownEvent::End(_)
        | MarkdownEvent::HtmlBlock(_)
        | MarkdownEvent::CodeBlock { .. }
        | MarkdownEvent::Rule => false,
    }
}

const fn alignment(alignment: markdown::Alignment) -> Alignment {
    match alignment {
        markdown::Alignment::None => Alignment::None,
        markdown::Alignment::Left => Alignment::Left,
        markdown::Alignment::Center => Alignment::Center,
        markdown::Alignment::Right => Alignment::Right,
    }
}
