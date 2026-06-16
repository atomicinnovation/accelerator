//! Markdown body extraction and blank-line trimming.
//!
//! Ports `config_extract_body` and `config_trim_body` from
//! [`scripts/config-common.sh`](../../../../scripts/config-common.sh) (lines
//! 96-139). These back the `config-read-context` /
//! `config-read-skill-context` / `config-read-skill-instructions` family.

/// Extract the markdown body (everything after the closing `---`).
///
/// Mirrors `config_extract_body`:
///   - No frontmatter (line 1 is not a strict delimiter) → the **entire**
///     file is the body.
///   - Closed frontmatter → the lines after the closing delimiter.
///   - Unclosed frontmatter → empty (the file is treated as malformed).
///
/// The returned string preserves the body's internal line structure but
/// carries no trailing newline beyond what the source lines imply; callers
/// pass it through [`trim_body`].
pub fn extract_body(contents: &str) -> String {
    let mut lines = contents.split('\n');
    let Some(first) = lines.next() else {
        return String::new();
    };
    if !is_strict_delim(first) {
        // No frontmatter: the whole file is the body. Reconstruct verbatim.
        return contents.to_string();
    }
    // Frontmatter opened. Find the closing delimiter; the body is what
    // follows. If never closed, the body is empty.
    let mut closed = false;
    let mut body: Vec<&str> = Vec::new();
    for line in lines {
        if !closed {
            if is_strict_delim(line) {
                closed = true;
            }
            continue;
        }
        body.push(line);
    }
    if closed {
        body.join("\n")
    } else {
        String::new()
    }
}

/// Trim leading and trailing blank lines (a blank line is empty or
/// whitespace-only). Mirrors `config_trim_body`'s awk: skip leading lines
/// until the first with a non-whitespace field, then drop trailing
/// whitespace-only lines. Interior blank lines are preserved.
pub fn trim_body(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let is_blank = |l: &str| l.split_whitespace().next().is_none();
    let start = lines.iter().position(|l| !is_blank(l));
    let Some(start) = start else {
        return String::new();
    };
    let mut end = lines.len();
    while end > start && is_blank(lines[end - 1]) {
        end -= 1;
    }
    lines[start..end].join("\n")
}

/// A strict frontmatter delimiter: `---` then only trailing whitespace.
/// Mirrors awk `^---[[:space:]]*$`.
fn is_strict_delim(line: &str) -> bool {
    line.strip_prefix("---")
        .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_whitespace()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_frontmatter_returns_whole_file() {
        assert_eq!(extract_body("line one\nline two\n"), "line one\nline two\n");
    }

    #[test]
    fn closed_frontmatter_returns_body_after_close() {
        assert_eq!(
            extract_body("---\nkey: v\n---\n\nbody line\n"),
            "\nbody line\n",
        );
    }

    #[test]
    fn unclosed_frontmatter_returns_empty() {
        assert_eq!(extract_body("---\nkey: v\nno close\n"), "");
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(extract_body(""), "");
    }

    #[test]
    fn trim_strips_leading_and_trailing_blank_lines() {
        assert_eq!(trim_body("\n\n  \nhello\nworld\n\n  \n"), "hello\nworld");
    }

    #[test]
    fn trim_preserves_interior_blanks() {
        assert_eq!(trim_body("a\n\nb"), "a\n\nb");
    }

    #[test]
    fn trim_whitespace_only_is_empty() {
        assert_eq!(trim_body("   \n\n  \n"), "");
        assert_eq!(trim_body(""), "");
    }

    #[test]
    fn extract_then_trim_matches_context_pipeline() {
        // `---\nkey: v\n---\n\nThis is the project context.\n`
        let body = extract_body("---\nkey: v\n---\n\nThis is the project context.\n");
        assert_eq!(trim_body(&body), "This is the project context.");
    }
}
