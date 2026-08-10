//! Line-oriented text helpers shared by the mechanical migrations.
//!
//! Applies `sed`/`grep`-style line transforms exactly (rather than a
//! structural YAML rewrite) so a migration's output stays byte-for-byte
//! equal to the captured golden fixtures wherever nothing changed.

/// Applies `transform` to every line, preserving whether `content` ends with
/// a trailing newline.
pub fn map_lines(content: &str, transform: impl Fn(&str) -> String) -> String {
    let lines: Vec<String> = content.lines().map(transform).collect();
    join_lines(content, &lines)
}

/// Keeps only lines for which `keep` returns `true`, preserving trailing
/// newline presence — the Rust equivalent of `grep -v`.
pub fn filter_lines(content: &str, keep: impl Fn(&str) -> bool) -> String {
    let lines: Vec<String> = content
        .lines()
        .filter(|line| keep(line))
        .map(str::to_owned)
        .collect();
    join_lines(content, &lines)
}

fn join_lines(original: &str, lines: &[String]) -> String {
    let mut result = lines.join("\n");
    if original.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// The number of leading whitespace (space/tab) characters.
fn leading_whitespace_len(line: &str) -> usize {
    line.find(|character: char| !character.is_whitespace())
        .unwrap_or(line.len())
}

/// `^([[:space:]]+)old_key:` → `\1new_key:`, everything after the colon
/// unchanged. `None` when the line has no leading whitespace or does not
/// start with `old_key:` after it.
#[must_use]
pub fn indented_rename(
    line: &str,
    old_key: &str,
    new_key: &str,
) -> Option<String> {
    let indent_len = leading_whitespace_len(line);
    if indent_len == 0 {
        return None;
    }
    let (indent, rest) = line.split_at(indent_len);
    let prefix = format!("{old_key}:");
    let after = rest.strip_prefix(prefix.as_str())?;
    Some(format!("{indent}{new_key}:{after}"))
}

/// `^([[:space:]]+)old_key:[[:space:]]*old_value[[:space:]]*$` →
/// `\1replacement` (the whole remainder is replaced, not just the key).
#[must_use]
pub fn indented_rename_default(
    line: &str,
    old_key: &str,
    old_value: &str,
    replacement: &str,
) -> Option<String> {
    let indent_len = leading_whitespace_len(line);
    if indent_len == 0 {
        return None;
    }
    let (indent, rest) = line.split_at(indent_len);
    let prefix = format!("{old_key}:");
    let after = rest.strip_prefix(prefix.as_str())?;
    if after.trim() == old_value {
        Some(format!("{indent}{replacement}"))
    } else {
        None
    }
}

/// `^old_key:` → `new_key:`, anchored at column 0 (no leading whitespace
/// required or permitted).
#[must_use]
pub fn flat_rename(line: &str, old_key: &str, new_key: &str) -> Option<String> {
    let prefix = format!("{old_key}:");
    let after = line.strip_prefix(prefix.as_str())?;
    Some(format!("{new_key}:{after}"))
}

/// `^old_key:[[:space:]]*old_value[[:space:]]*$` → `replacement`, anchored at
/// column 0.
#[must_use]
pub fn flat_rename_default(
    line: &str,
    old_key: &str,
    old_value: &str,
    replacement: &str,
) -> Option<String> {
    let prefix = format!("{old_key}:");
    let after = line.strip_prefix(prefix.as_str())?;
    if after.trim() == old_value {
        Some(replacement.to_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        filter_lines, flat_rename, flat_rename_default, indented_rename,
        indented_rename_default, map_lines,
    };

    #[test]
    fn map_lines_preserves_trailing_newline() {
        assert_eq!(map_lines("a\nb\n", str::to_uppercase), "A\nB\n");
        assert_eq!(map_lines("a\nb", str::to_uppercase), "A\nB");
    }

    #[test]
    fn filter_lines_drops_matching_lines() {
        assert_eq!(
            filter_lines("keep\ndrop\nkeep\n", |l| l != "drop"),
            "keep\nkeep\n"
        );
    }

    #[test]
    fn indented_rename_requires_leading_whitespace() {
        assert_eq!(
            indented_rename("  tickets: x", "tickets", "work"),
            Some("  work: x".to_owned())
        );
        assert_eq!(indented_rename("tickets: x", "tickets", "work"), None);
        assert_eq!(indented_rename("  other: x", "tickets", "work"), None);
    }

    #[test]
    fn indented_rename_default_matches_exact_value_only() {
        assert_eq!(
            indented_rename_default(
                "  tickets: meta/tickets",
                "tickets",
                "meta/tickets",
                "work: meta/work"
            ),
            Some("  work: meta/work".to_owned())
        );
        assert_eq!(
            indented_rename_default(
                "  tickets: meta/custom",
                "tickets",
                "meta/tickets",
                "work: meta/work"
            ),
            None
        );
    }

    #[test]
    fn flat_rename_requires_column_zero() {
        assert_eq!(
            flat_rename("paths.tickets: x", "paths.tickets", "paths.work"),
            Some("paths.work: x".to_owned())
        );
        assert_eq!(
            flat_rename("  paths.tickets: x", "paths.tickets", "paths.work"),
            None
        );
    }

    #[test]
    fn flat_rename_default_matches_exact_value_only() {
        assert_eq!(
            flat_rename_default(
                "paths.tickets: meta/tickets",
                "paths.tickets",
                "meta/tickets",
                "paths.work: meta/work"
            ),
            Some("paths.work: meta/work".to_owned())
        );
    }
}
