//! Markdown as Linear's API parser reads it.

mod digit_rule;
mod dotted_tokens;
mod link_nesting;
mod scheme_urls;
mod text_runs;
mod top_level_domains;

use std::ops::Range;

use document::markdown::{events, MarkdownEvent};

use crate::{spliced, Splice};

pub use top_level_domains::is_top_level_domain;

use self::text_runs::TextRun;

/// `markdown` with a backslash before each `.` or `)` that Linear's parser
/// would read as an ordered-list marker or part of a link.
///
/// Only backslashes are inserted, and none that would change how `document`
/// reads the body.
#[must_use]
pub fn escape_for_push(markdown: &str) -> String {
    escaped_or_original(markdown, proposed_escapes(markdown))
}

fn proposed_escapes(markdown: &str) -> Vec<Escape> {
    let events = events(markdown);
    let mut escapes = digit_rule::escapes(markdown, &events);
    escapes.extend(dotted_tokens::escapes(markdown, &events));
    escapes
}

/// The byte ranges of `text`, read as plain text rather than Markdown, that
/// Linear's parser would link as bare host names, each with its port.
pub fn dotted_tokens(text: &str) -> impl Iterator<Item = Range<usize>> {
    let run = TextRun::plain(text);
    dotted_tokens::linkable_tokens(&run)
        .map(|token| token.source())
        .collect::<Vec<_>>()
        .into_iter()
}

fn escaped_or_original(markdown: &str, escapes: Vec<Escape>) -> String {
    let escapes = in_source_order(escapes);
    let candidate = with_escapes(markdown, escapes.clone());
    if reads_alike(markdown, &candidate) {
        return candidate;
    }
    let mut kept: Vec<Escape> = Vec::new();
    for escape in escapes {
        kept.push(escape);
        if !reads_alike(markdown, &with_escapes(markdown, kept.clone())) {
            kept.pop();
        }
    }
    with_escapes(markdown, kept)
}

fn in_source_order(mut escapes: Vec<Escape>) -> Vec<Escape> {
    escapes.sort_unstable();
    escapes.dedup();
    escapes
}

fn with_escapes(markdown: &str, escapes: Vec<Escape>) -> String {
    spliced(
        markdown,
        in_source_order(escapes).into_iter().map(Escape::splice),
    )
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
