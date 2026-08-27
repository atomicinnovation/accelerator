//! Parses `pub const NAME: u8 = N;` declarations out of an `exit_codes.rs`.
//!
//! A parity test compares the constants against a captured fixture without
//! importing them, since a bin crate exposes no library surface.
//!
//! The parser is deliberately textual: a parity oracle re-derived from the
//! constants it guards would be a tautology no accidental renumbering could red.

/// Every `pub const NAME: u8 = N;` in `source`, as `(name, value)` in file
/// order. A line that is not exactly that shape is skipped.
#[must_use]
pub fn parse_u8_consts(source: &str) -> Vec<(String, u8)> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("pub const ")?;
            let (name, rest) = rest.split_once(':')?;
            let (kind, value) = rest.trim().split_once('=')?;
            if kind.trim() != "u8" {
                return None;
            }
            let value = value.trim().trim_end_matches(';').trim();
            Some((name.trim().to_owned(), value.parse().ok()?))
        })
        .collect()
}
