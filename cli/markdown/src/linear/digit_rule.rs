//! A lone number opening a continuation line, which Linear reads as an
//! ordered-list item rather than as the paragraph's text.

use document::markdown::MarkdownEvent;

use super::link_nesting::LinkNesting;
use super::{Escape, Parsed};
use crate::continuation_start;

pub(super) fn escapes(markdown: &str, events: &Parsed) -> Vec<Escape> {
    continuation_lines(markdown, events)
        .into_iter()
        .filter_map(|line| {
            lone_number_delimiter(&markdown[line..]).map(|delimiter| Escape {
                before: line + delimiter,
            })
        })
        .collect()
}

fn continuation_lines(markdown: &str, events: &Parsed) -> Vec<usize> {
    let mut nesting = LinkNesting::default();
    let mut lines = Vec::new();
    for (index, (event, range)) in events.iter().enumerate() {
        nesting.observe(event);
        if matches!(event, MarkdownEvent::SoftBreak | MarkdownEvent::HardBreak)
            && nesting.is_outside()
        {
            if let Some((_, next)) = events.get(index + 1) {
                lines.push(continuation_start(markdown, range.end, next.start));
            }
        }
    }
    lines
}

/// The offset of the `.` or `)` in `line` when it opens with a number that
/// Linear would take for an ordered-list marker.
fn lone_number_delimiter(line: &str) -> Option<usize> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    let mut rest = line.bytes().skip(digits);
    let is_marker = digits > 0
        && matches!(rest.next(), Some(b'.' | b')'))
        && matches!(rest.next(), None | Some(b' ' | b'\t' | b'\r' | b'\n'));
    is_marker.then_some(digits)
}
