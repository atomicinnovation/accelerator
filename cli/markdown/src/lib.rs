//! Text transformations for Markdown, one module per dialect, that preserve
//! every byte they do not deliberately change.

use std::ops::Range;

pub mod github;
pub mod linear;

#[cfg(test)]
mod tests;

pub(crate) struct Splice {
    pub(crate) replaced: Range<usize>,
    pub(crate) with: &'static str,
}

/// `markdown` with each splice applied, in ascending, non-overlapping order.
pub(crate) fn spliced(
    markdown: &str,
    splices: impl IntoIterator<Item = Splice>,
) -> String {
    let mut result = String::with_capacity(markdown.len());
    let mut preserved_from = 0;
    for splice in splices {
        result.push_str(&markdown[preserved_from..splice.replaced.start]);
        result.push_str(splice.with);
        preserved_from = splice.replaced.end;
    }
    result.push_str(&markdown[preserved_from..]);
    result
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
