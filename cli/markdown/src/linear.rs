//! Markdown as Linear's API parser reads it.

mod digit_rule;
mod top_level_domains;

use std::ops::Range;

use document::markdown::{events, MarkdownEvent};

use crate::{spliced, Splice};

pub use top_level_domains::is_top_level_domain;

/// `markdown` with a backslash before each `.` or `)` that Linear's parser
/// would read as an ordered-list marker or part of a link.
///
/// Only backslashes are inserted. If any would change how `document` reads
/// the body, `markdown` is returned unchanged.
#[must_use]
pub fn escape_for_push(markdown: &str) -> String {
    escaped_or_original(markdown, proposed_escapes(markdown))
}

fn proposed_escapes(markdown: &str) -> Vec<Escape> {
    let events = events(markdown);
    digit_rule::escapes(markdown, &events)
}

fn escaped_or_original(markdown: &str, escapes: Vec<Escape>) -> String {
    let candidate = with_escapes(markdown, escapes);
    if reads_alike(markdown, &candidate) {
        candidate
    } else {
        markdown.to_owned()
    }
}

fn with_escapes(markdown: &str, mut escapes: Vec<Escape>) -> String {
    escapes.sort_unstable();
    escapes.dedup();
    spliced(markdown, escapes.into_iter().map(Escape::splice))
}

fn reads_alike(original: &str, escaped: &str) -> bool {
    reading(original) == reading(escaped)
}

fn reading(markdown: &str) -> Vec<MarkdownEvent> {
    let mut reading: Vec<MarkdownEvent> = Vec::new();
    for (event, _) in events(markdown) {
        match (reading.last_mut(), event) {
            (
                Some(MarkdownEvent::Text(preceding)),
                MarkdownEvent::Text(text),
            ) => preceding.push_str(&text),
            (_, event) => reading.push(event),
        }
    }
    reading
}

type Parsed = [(MarkdownEvent, Range<usize>)];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Escape {
    before: usize,
}

impl Escape {
    const fn splice(self) -> Splice {
        Splice {
            replaced: self.before..self.before,
            with: "\\",
        }
    }
}

#[cfg(test)]
mod tests;
