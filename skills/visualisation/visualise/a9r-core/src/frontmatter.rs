//! YAML frontmatter extraction with the bash parser's exact three-state
//! semantics.
//!
//! Ports `config_extract_frontmatter` and the `_read_from_file` warning gate
//! from
//! [`scripts/config-common.sh`](../../../../scripts/config-common.sh) (lines
//! 73-85) and
//! [`scripts/config-read-value.sh`](../../../../scripts/config-read-value.sh)
//! (lines 44-54).
//!
//! There are **three** outcomes, not two:
//!   - [`Frontmatter::Absent`] — line 1 is not a delimiter → silent not-found.
//!   - [`Frontmatter::Closed`] — opened and closed; the text may be empty
//!     (`---\n---`), which is also a silent not-found, distinct from unclosed.
//!   - [`Frontmatter::Unclosed`] — opened, never closed → not-found, and the
//!     caller emits a stderr warning if [`opens_loosely`] matches line 1.
//!
//! The parser opens/closes only on the **strict** delimiter
//! `^---[[:space:]]*$`, but the unclosed-warning *gate* re-reads line 1 with
//! an **unanchored** `grep '^---'` (fires on `---foo`, `----`). The two are
//! modelled separately so stderr matches bash exactly.

#[derive(Debug, PartialEq, Eq)]
pub enum Frontmatter {
    /// No frontmatter (line 1 not a strict delimiter, or empty input).
    Absent,
    /// Closed frontmatter; the text between the delimiters (may be empty).
    Closed(String),
    /// Opened with a strict delimiter but never closed.
    Unclosed,
}

/// A strict frontmatter delimiter: `---` followed only by trailing
/// whitespace. Mirrors awk `^---[[:space:]]*$`.
fn is_strict_delim(line: &str) -> bool {
    match line.strip_prefix("---") {
        Some(rest) => rest.chars().all(|c| c.is_ascii_whitespace()),
        None => false,
    }
}

/// Extract frontmatter from raw file `contents`.
///
/// Lines are split on `\n` (matching awk's default record separator); a
/// trailing `\r` is intentionally **not** stripped, so a CRLF file is
/// handled the same way awk handles it.
pub fn extract(contents: &str) -> Frontmatter {
    let mut lines = contents.split('\n');
    let Some(first) = lines.next() else {
        return Frontmatter::Absent;
    };
    if !is_strict_delim(first) {
        return Frontmatter::Absent;
    }
    let mut collected: Vec<&str> = Vec::new();
    for line in lines {
        if is_strict_delim(line) {
            return Frontmatter::Closed(collected.join("\n"));
        }
        collected.push(line);
    }
    Frontmatter::Unclosed
}

/// The loose warning gate: does line 1 start with `---`? Mirrors
/// `head -1 "$file" | grep -q '^---'` (unanchored at the end).
pub fn opens_loosely(contents: &str) -> bool {
    contents
        .split('\n')
        .next()
        .is_some_and(|l| l.starts_with("---"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_when_line1_is_not_a_delimiter() {
        assert_eq!(extract("key: value\nother: thing\n"), Frontmatter::Absent);
        assert_eq!(extract(""), Frontmatter::Absent);
        // Four dashes: not a strict delimiter (trailing `-` is not space).
        assert_eq!(extract("----\nkey: v\n"), Frontmatter::Absent);
        // `---foo`: strict open requires only-whitespace after `---`.
        assert_eq!(extract("---foo\nkey: v\n"), Frontmatter::Absent);
    }

    #[test]
    fn closed_extracts_inner_lines() {
        assert_eq!(
            extract("---\nkey: value\nother: thing\n---\nbody\n"),
            Frontmatter::Closed("key: value\nother: thing".to_string()),
        );
    }

    #[test]
    fn empty_but_closed_is_distinct_from_unclosed() {
        // `---\n---` → Closed(""), a silent not-found with NO warning.
        assert_eq!(extract("---\n---\n"), Frontmatter::Closed(String::new()));
        assert_eq!(extract("---\n---"), Frontmatter::Closed(String::new()));
    }

    #[test]
    fn unclosed_when_no_closing_delimiter() {
        assert_eq!(extract("---\nkey: value\n"), Frontmatter::Unclosed);
        assert_eq!(extract("---\n"), Frontmatter::Unclosed);
        assert_eq!(extract("---"), Frontmatter::Unclosed);
    }

    #[test]
    fn delimiter_with_trailing_whitespace_is_recognised() {
        assert_eq!(
            extract("---  \nkey: value\n---  \n"),
            Frontmatter::Closed("key: value".to_string()),
        );
    }

    #[test]
    fn body_horizontal_rule_does_not_reopen() {
        // The first closing `---` ends the frontmatter; a later `---` in the
        // body is irrelevant to extraction.
        assert_eq!(
            extract("---\nkey: value\n---\nbody\n---\nmore\n"),
            Frontmatter::Closed("key: value".to_string()),
        );
    }

    #[test]
    fn loose_gate_fires_on_unanchored_dashes() {
        assert!(opens_loosely("---\n"));
        assert!(opens_loosely("---foo\n"));
        assert!(opens_loosely("----\n"));
        assert!(!opens_loosely("key: v\n"));
        assert!(!opens_loosely(""));
    }
}
