//! Content normalisation for the sync change-detection contract. Port of
//! `_win_trim` and the `<file>`-mode `IGNORE_KEYS` filter
//! (`work-item-normalise.sh:49-80`).
//!
//! Trimming is ASCII-whitespace-only (`char::is_ascii_whitespace`), not
//! Rust's Unicode-aware `str::trim` — the shell runs under `LANG=C
//! LC_ALL=C`, so `[[:space:]]` means ASCII whitespace exactly. A
//! Unicode-aware trim would normalise non-ASCII-whitespace content
//! differently across machines, silently breaking the cross-machine
//! baseline guarantee.

/// Top-level frontmatter keys the sync engine's change-detection ignores as
/// provenance rather than authored content.
pub const IGNORE_KEYS: &[&str] = &[
    "last_updated",
    "last_updated_by",
    "id",
    "external_id",
    "updated_at",
    "revision",
];

const fn is_ascii_space(c: char) -> bool {
    c.is_ascii_whitespace()
}

fn trimmed_lines(content: &str) -> Vec<String> {
    let mut lines: Vec<String> = content
        .lines()
        .map(|line| line.trim_matches(is_ascii_space).to_owned())
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

fn join_with_trailing_newlines(lines: &[String]) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Per-line ASCII-whitespace trim plus trailing-blank-line strip. Port of
/// `_win_trim` (`work-item-normalise.sh:49-57`) — the `--stdin` mode.
#[must_use]
pub fn trim_lines(content: &str) -> String {
    join_with_trailing_newlines(&trimmed_lines(content))
}

/// True iff `line` opens with a top-level (column-0, unindented) `key:`
/// whose name is `key`.
fn top_level_key(line: &str) -> Option<&str> {
    let mut end = 0;
    for (index, c) in line.char_indices() {
        if index == 0 {
            if !(c.is_ascii_alphabetic() || c == '_') {
                return None;
            }
        } else if !(c.is_ascii_alphanumeric() || c == '_') {
            break;
        }
        end = index + c.len_utf8();
    }
    if end == 0 || !line[end..].starts_with(':') {
        return None;
    }
    Some(&line[..end])
}

/// Drops ignored top-level frontmatter keys, then trims as [`trim_lines`]
/// does.
///
/// Port of `_win_filter_frontmatter` (`work-item-normalise.sh:62-80`) — the
/// `<file>`-mode normaliser. Consumed by none of the five user-facing
/// commands (0194's sync-engine change-detection is its only future
/// caller); characterization-tested per the AC regardless.
#[must_use]
pub fn filter_frontmatter_keys(frontmatter_raw: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in frontmatter_raw.lines() {
        if let Some(key) = top_level_key(line) {
            if IGNORE_KEYS.contains(&key) {
                continue;
            }
        }
        lines.push(line.trim_matches(is_ascii_space).to_owned());
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    join_with_trailing_newlines(&lines)
}

#[cfg(test)]
mod tests {
    use super::{filter_frontmatter_keys, trim_lines};

    #[test]
    fn trim_lines_trims_each_line_and_drops_trailing_blanks() {
        assert_eq!(
            trim_lines("  leading and trailing   \nline2\n\n\n"),
            "leading and trailing\nline2\n"
        );
    }

    #[test]
    fn trim_lines_does_not_strip_a_trailing_nbsp() {
        // U+00A0 is not ASCII whitespace under [[:space:]] in the C locale.
        let input = "trailing-nbsp\u{00A0}\n";
        assert_eq!(trim_lines(input), input);
    }

    #[test]
    fn filter_frontmatter_keys_drops_only_top_level_ignore_keys() {
        let input = "status: draft   \n\
                     last_updated: \"2026-01-01\"\n\
                     revision: abc123\n\
                     nested:\n  last_updated: not-top-level\n";
        // Every kept line is trimmed at both ends unconditionally (matching
        // the shell's own gsub calls), so the nested line's indentation is
        // lost too — a real, unsurprising-once-verified quirk of the
        // <file>-mode normaliser, not something this port should "fix".
        assert_eq!(
            filter_frontmatter_keys(input),
            "status: draft\nnested:\nlast_updated: not-top-level\n"
        );
    }
}
