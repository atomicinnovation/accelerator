//! Text as a reader sees it, with each character traced back to its source.

use document::markdown::MarkdownEvent;

use super::link_nesting::LinkNesting;
use super::Parsed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RunCharacter {
    pub(super) character: char,
    pub(super) source: usize,
    pub(super) escaped: bool,
}

/// Text read without interruption: no break, tag, code, HTML or entity
/// falls inside it.
#[derive(Debug, Default)]
pub(super) struct TextRun {
    pub(super) characters: Vec<RunCharacter>,
    source_end: usize,
}

impl TextRun {
    pub(super) fn from_events(markdown: &str, events: &Parsed) -> Vec<Self> {
        let mut runs = Vec::new();
        let mut current = Self::default();
        let mut nesting = LinkNesting::default();
        for (event, range) in events {
            nesting.observe(event);
            match event {
                MarkdownEvent::Text(text)
                    if nesting.is_outside()
                        && markdown.get(range.clone())
                            == Some(text.as_str()) =>
                {
                    if !current.continues_at(markdown, range.start) {
                        runs.push(std::mem::take(&mut current));
                    }
                    current.read(markdown, text, range.start);
                }
                _ => runs.push(std::mem::take(&mut current)),
            }
        }
        runs.push(current);
        runs.retain(|run| !run.characters.is_empty());
        runs
    }

    pub(super) fn plain(text: &str) -> Self {
        let mut run = Self::default();
        run.read(text, text, 0);
        run
    }

    fn continues_at(&self, markdown: &str, start: usize) -> bool {
        self.characters.is_empty()
            || start == self.source_end
            || is_escape_gap(markdown, self.source_end, start)
    }

    fn read(&mut self, markdown: &str, text: &str, start: usize) {
        let escaped = start > 0
            && markdown.as_bytes()[start - 1] == b'\\'
            && (self.characters.is_empty() || self.source_end == start - 1);
        self.characters.extend(text.char_indices().map(
            |(offset, character)| RunCharacter {
                character,
                source: start + offset,
                escaped: escaped && offset == 0,
            },
        ));
        self.source_end = start + text.len();
    }
}

fn is_escape_gap(markdown: &str, end: usize, start: usize) -> bool {
    start == end + 1 && markdown.as_bytes()[end] == b'\\'
}
