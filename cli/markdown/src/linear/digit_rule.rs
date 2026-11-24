//! A lone number opening a continuation line, which Linear reads as an
//! ordered-list item rather than as the paragraph's text.

use document::markdown::{MarkdownEvent, MarkdownTag};

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
    let mut link_depth = 0_usize;
    let mut lines = Vec::new();
    for (index, (event, range)) in events.iter().enumerate() {
        match event {
            MarkdownEvent::Start(
                MarkdownTag::Link { .. } | MarkdownTag::Image { .. },
            ) => link_depth += 1,
            MarkdownEvent::End(
                MarkdownTag::Link { .. } | MarkdownTag::Image { .. },
            ) => link_depth -= 1,
            MarkdownEvent::SoftBreak | MarkdownEvent::HardBreak
                if link_depth == 0 =>
            {
                if let Some((_, next)) = events.get(index + 1) {
                    lines.push(continuation_start(
                        markdown, range.end, next.start,
                    ));
                }
            }
            _ => {}
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
