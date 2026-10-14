//! Text transformations for GitHub-flavoured Markdown that preserve every
//! byte they do not deliberately change.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser};

#[cfg(test)]
mod tests;

/// Replaces each line break GitHub renders as soft with a single space.
///
/// The replaced span takes in the trailing whitespace before the line break
/// and the continuation line's `>` markers and indentation. Every other byte
/// is left as written.
///
/// A whitespace line break that does not end in two spaces counts as soft:
/// `pulldown-cmark` reads any two trailing whitespace bytes as a hard break,
/// but GitHub needs two spaces and renders the rest as soft.
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
            [(line_break, line_break_range), (_, continuation)]
                if renders_as_soft_on_github(
                    markdown,
                    line_break,
                    line_break_range,
                ) =>
            {
                Some(
                    trailing_whitespace_start(markdown, line_break_range.start)
                        ..continuation_start(
                            markdown,
                            line_break_range.end,
                            continuation.start,
                        ),
                )
            }
            _ => None,
        })
        .collect()
}

fn renders_as_soft_on_github(
    markdown: &str,
    line_break: &Event<'_>,
    range: &Range<usize>,
) -> bool {
    match line_break {
        Event::SoftBreak => true,
        Event::HardBreak => {
            let marker = markdown[range.clone()].trim_end_matches(['\r', '\n']);
            !(marker.ends_with("  ") || marker.ends_with('\\'))
        }
        _ => false,
    }
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
