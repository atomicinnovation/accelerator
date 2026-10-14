//! Markdown as GitHub renders it in pull request bodies.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser};

use crate::{continuation_start, trailing_whitespace_start};

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

pub(crate) fn extensions() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
}

fn soft_line_break_spans(markdown: &str) -> Vec<Range<usize>> {
    let events: Vec<_> = Parser::new_ext(markdown, extensions())
        .into_offset_iter()
        .collect();
    events
        .windows(2)
        .filter_map(|pair| match pair {
            [(line_break, line_break_range), (_, continuation)]
                if renders_as_soft(markdown, line_break, line_break_range) =>
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

pub(crate) fn renders_as_soft(
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
