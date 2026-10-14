//! Text transformations for Markdown, one module per dialect, that preserve
//! every byte they do not deliberately change.

pub mod github;

#[cfg(test)]
mod tests;

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
