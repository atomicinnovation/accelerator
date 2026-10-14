//! Text transformations for GitHub-flavoured Markdown that preserve every
//! byte they do not deliberately change.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser};

#[cfg(test)]
mod tests;

/// Replaces each GFM soft line break with a single space, leaving every
/// other byte as written.
///
/// A replaced span runs from any trailing whitespace before the line ending
/// to the start of the continuation line's content, past its `>` markers and
/// indentation.
#[must_use]
pub fn unwrap_soft_line_breaks(markdown: &str) -> String {
    let mut unwrapped = String::with_capacity(markdown.len());
    let mut preserved_from = 0;
    for span in soft_line_break_spans(markdown) {
        unwrapped.push_str(&markdown[preserved_from..span.start]);
        unwrapped.push(' ');
        preserved_from = span.end;
    }
    unwrapped.push_str(&markdown[preserved_from..]);
    unwrapped
}

fn github_extensions() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
}

fn soft_line_break_spans(markdown: &str) -> Vec<Range<usize>> {
    let events: Vec<_> = Parser::new_ext(markdown, github_extensions())
        .into_offset_iter()
        .collect();
    events
        .windows(2)
        .filter_map(|pair| match pair {
            [(Event::SoftBreak, line_ending), (_, continuation)] => Some(
                trailing_whitespace_start(markdown, line_ending.start)
                    ..continuation_start(
                        markdown,
                        line_ending.end,
                        continuation.start,
                    ),
            ),
            _ => None,
        })
        .collect()
}

fn trailing_whitespace_start(markdown: &str, line_ending: usize) -> usize {
    line_ending
        - markdown[..line_ending]
            .bytes()
            .rev()
            .take_while(|byte| matches!(byte, b' ' | b'\t' | b'\x0B' | b'\x0C'))
            .count()
}

fn continuation_start(
    markdown: &str,
    line_start: usize,
    next_event_start: usize,
) -> usize {
    let container_prefix_end = line_start
        + markdown[line_start..]
            .bytes()
            .take_while(|byte| matches!(byte, b'>' | b' ' | b'\t'))
            .count();
    let next_event_is_on_continuation_line = next_event_start >= line_start;
    if next_event_is_on_continuation_line {
        container_prefix_end.min(next_event_start)
    } else {
        container_prefix_end
    }
}
