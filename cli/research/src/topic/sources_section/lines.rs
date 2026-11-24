//! The one rule for what a line of a document body is.

pub struct Line<'body> {
    pub number: usize,
    pub text: &'body str,
}

pub fn numbered(body: &str) -> impl Iterator<Item = Line<'_>> {
    body.split_inclusive('\n')
        .zip(1..)
        .map(|(as_written, number)| {
            let text = as_written.strip_suffix('\n').unwrap_or(as_written);
            let text = text.strip_suffix('\r').unwrap_or(text);
            Line { number, text }
        })
}

pub fn offset_of(body: &str, number: usize) -> usize {
    body.split_inclusive('\n')
        .take(number.saturating_sub(1))
        .map(str::len)
        .sum()
}
