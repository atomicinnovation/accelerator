//! ADR numbering and status: the pure conventions behind `adr next-number`
//! and `adr read-status`.

/// The next `count` sequential ADR numbers, zero-padded to a minimum of 4
/// digits, given `existing` ADR-directory basenames already filtered to
/// `ADR-[0-9][0-9][0-9][0-9]*`.
#[must_use]
pub fn next_numbers(existing: &[&str], count: u32) -> Vec<String> {
    let highest = existing
        .iter()
        .filter_map(|name| name.strip_prefix("ADR-"))
        .filter_map(leading_number)
        .max()
        .unwrap_or(0);
    (1..=count).map(|n| format!("{:04}", highest + n)).collect()
}

fn leading_number(rest: &str) -> Option<u32> {
    let digits: String =
        rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// The `status:` value from `content`'s YAML frontmatter fence, or `None`
/// when the fence is missing or unclosed, or the fence carries no `status:`
/// line.
///
/// Replicates the bash reference's line-by-line state machine exactly: the
/// first bare `---` opens the fence, the second closes it and the scan stops
/// immediately, and value extraction runs quote-stripping before
/// whitespace-trimming — an intentional quirk, not a bug, that this port
/// preserves (see [`unwrap_status_value`]).
#[must_use]
pub fn read_status(content: &str) -> Option<String> {
    let mut in_fence = false;
    let mut closed = false;
    let mut status = None;
    for line in content.lines() {
        if line == "---" {
            if in_fence {
                closed = true;
                break;
            }
            in_fence = true;
            continue;
        }
        if in_fence {
            if let Some(rest) = line.strip_prefix("status:") {
                status = Some(unwrap_status_value(rest));
            }
        }
    }
    if closed {
        status
    } else {
        None
    }
}

/// Strips the `status:` prefix's leading whitespace, then one leading and one
/// trailing quote char (independently, and in that order — a quote-stripped
/// buffer's trailing whitespace has not been trimmed yet), then trailing
/// whitespace. Because quote-stripping runs before whitespace-trimming, a
/// value like `"foo" ` (trailing space after the closing quote) unwraps to
/// `foo"`, not `foo`: the trailing-quote check sees a trailing space, not a
/// quote, so only the leading quote is stripped, and the later
/// whitespace-trim leaves the stray closing quote behind.
fn unwrap_status_value(raw: &str) -> String {
    let mut value = raw.trim_start().to_owned();
    if value.starts_with('"') || value.starts_with('\'') {
        value.remove(0);
    }
    if value.ends_with('"') || value.ends_with('\'') {
        value.pop();
    }
    value.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::next_numbers;
    use super::read_status;

    #[test]
    fn next_numbers_starts_at_0001_when_nothing_exists() {
        assert_eq!(next_numbers(&[], 1), vec!["0001"]);
    }

    #[test]
    fn next_numbers_continues_from_the_highest_existing() {
        assert_eq!(next_numbers(&["ADR-0003-foo"], 1), vec!["0004"]);
    }

    #[test]
    fn next_numbers_ignores_gaps_and_uses_the_highest() {
        assert_eq!(
            next_numbers(&["ADR-0001-first", "ADR-0005-fifth"], 1),
            vec!["0006"]
        );
    }

    #[test]
    fn next_numbers_overflow_is_not_truncated() {
        assert_eq!(next_numbers(&["ADR-9999-overflow"], 1), vec!["10000"]);
    }

    #[test]
    fn next_numbers_parses_a_leading_zero_as_decimal_not_octal() {
        assert_eq!(next_numbers(&["ADR-0008-x"], 1), vec!["0009"]);
    }

    #[test]
    fn next_numbers_emits_a_sequential_run() {
        assert_eq!(
            next_numbers(&["ADR-0002-x"], 3),
            vec!["0003", "0004", "0005"]
        );
    }

    #[test]
    fn next_numbers_ignores_non_adr_names() {
        assert_eq!(
            next_numbers(&["ADR-0002-x", "README", "DRAFT-notes"], 1),
            vec!["0003"]
        );
    }

    #[test]
    fn next_numbers_with_zero_count_emits_nothing() {
        assert!(next_numbers(&["ADR-0002-x"], 0).is_empty());
    }

    #[test]
    fn read_status_finds_a_plain_value() {
        let content =
            "---\nadr_id: ADR-0001\nstatus: proposed\n---\n\n# Test\n";
        assert_eq!(read_status(content), Some("proposed".to_owned()));
    }

    #[test]
    fn read_status_strips_double_quotes() {
        let content = "---\nstatus: \"proposed\"\n---\n";
        assert_eq!(read_status(content), Some("proposed".to_owned()));
    }

    #[test]
    fn read_status_strips_single_quotes() {
        let content = "---\nstatus: 'proposed'\n---\n";
        assert_eq!(read_status(content), Some("proposed".to_owned()));
    }

    #[test]
    fn read_status_is_unaffected_by_quote_stripping_when_unquoted() {
        let content = "---\nstatus: proposed\n---\n";
        assert_eq!(read_status(content), Some("proposed".to_owned()));
    }

    #[test]
    fn read_status_allows_no_space_after_the_colon() {
        let content = "---\nstatus:proposed\n---\n";
        assert_eq!(read_status(content), Some("proposed".to_owned()));
    }

    #[test]
    fn read_status_a_trailing_space_after_a_closing_double_quote_leaves_a_stray_quote(
    ) {
        let content = "---\nstatus: \"foo\" \n---\n";
        assert_eq!(read_status(content), Some("foo\"".to_owned()));
    }

    #[test]
    fn read_status_a_trailing_space_after_a_closing_single_quote_leaves_a_stray_quote(
    ) {
        let content = "---\nstatus: 'foo' \n---\n";
        assert_eq!(read_status(content), Some("foo'".to_owned()));
    }

    #[test]
    fn read_status_last_status_line_inside_the_fence_wins() {
        let content = "---\nstatus: proposed\nstatus: accepted\n---\n";
        assert_eq!(read_status(content), Some("accepted".to_owned()));
    }

    #[test]
    fn read_status_ignores_a_status_like_line_in_the_body() {
        let content = "---\nstatus: proposed\n---\n\nstatus: accepted\n";
        assert_eq!(read_status(content), Some("proposed".to_owned()));
    }

    #[test]
    fn read_status_an_empty_value_still_succeeds() {
        let content = "---\nstatus: \n---\n";
        assert_eq!(read_status(content), Some(String::new()));
    }

    #[test]
    fn read_status_missing_fence_is_none() {
        assert_eq!(read_status("# Just a regular file\n"), None);
    }

    #[test]
    fn read_status_unclosed_fence_is_none() {
        assert_eq!(read_status("---\nstatus: proposed\n"), None);
    }

    #[test]
    fn read_status_closed_fence_with_no_status_key_is_none() {
        assert_eq!(read_status("---\nadr_id: ADR-0001\n---\n"), None);
    }
}
