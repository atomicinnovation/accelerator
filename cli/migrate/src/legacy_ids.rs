//! The legacy four-digit work-item ID rules that migrations 0007 and 0009
//! were written against, frozen so their output never moves with the live
//! ID grammar. Only migrations read them.

use corpus::LinkageIds;

pub struct LegacyNumericIds;

const BARE_ID_WIDTH: usize = 4;

impl LinkageIds for LegacyNumericIds {
    fn id_in_filename(&self, filename: &str) -> Option<String> {
        let digits: String =
            filename.chars().take_while(char::is_ascii_digit).collect();
        (!digits.is_empty()).then_some(digits)
    }

    fn bare_ids_in(&self, line: &str) -> Vec<String> {
        let characters: Vec<char> = line.chars().collect();
        let mut tokens = Vec::new();
        let mut index = 0;
        while index + BARE_ID_WIDTH <= characters.len() {
            let candidate: String =
                characters[index..index + BARE_ID_WIDTH].iter().collect();
            if candidate.bytes().all(|byte| byte.is_ascii_digit()) {
                tokens.push(candidate);
                index += BARE_ID_WIDTH;
            } else {
                index += 1;
            }
        }
        tokens
    }
}

/// Whether `pattern` names the key token in either spelling.
///
/// The scan is lenient: an escaped `{{key}}` and a malformed token never
/// count, and a pattern the live grammar refuses may still reference the key.
#[must_use]
pub fn references_key(pattern: &str) -> bool {
    tokens_of(pattern)
        .iter()
        .any(|token| matches!(token.as_str(), "key" | "project"))
}

fn tokens_of(pattern: &str) -> Vec<String> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let next = chars.get(i + 1).copied();
        if (chars[i] == '{' && next == Some('{'))
            || (chars[i] == '}' && next == Some('}'))
        {
            i += 2;
            continue;
        }
        if chars[i] == '{' {
            if let Some(offset) =
                chars[i + 1..].iter().position(|&c| c == '}' || c == '{')
            {
                let close = i + 1 + offset;
                if chars[close] == '}' {
                    tokens.push(chars[i + 1..close].iter().collect());
                    i = close + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    tokens
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use corpus::LinkageIds;

    use super::references_key;
    use super::LegacyNumericIds;

    #[test]
    fn a_legacy_path_reads_the_stems_leading_digits() {
        for (filename, expected) in [
            ("0042-foo.md", Some("0042")),
            ("0042.md", Some("0042")),
            ("0042_notes.md", Some("0042")),
            ("42a-x.md", Some("42")),
            ("PROJ-0042-x.md", None),
        ] {
            assert_eq!(
                LegacyNumericIds.id_in_filename(filename).as_deref(),
                expected,
                "{filename}"
            );
        }
    }

    #[test]
    fn a_legacy_body_id_is_any_run_of_four_digits() {
        assert_eq!(
            LegacyNumericIds.bare_ids_in("- Blocks: 0042, 0061 and 042"),
            ["0042", "0061"]
        );
        assert_eq!(LegacyNumericIds.bare_ids_in("- Blocks: 12345"), ["1234"]);
        assert!(LegacyNumericIds.bare_ids_in("- Blocks: 004a").is_empty());
        assert_eq!(
            LegacyNumericIds.bare_ids_in("12345 0042a PR0042"),
            ["1234", "0042", "0042"]
        );
    }

    #[test]
    fn references_key_scans_as_m0009_does() {
        assert!(references_key("{project}-{number:04d}"));
        assert!(references_key("{key}-{number:04d}"));
        assert!(references_key("{project}-{number:4d}"));
        assert!(!references_key("{{project}}"));
    }

    #[test]
    fn references_key_is_brace_aware() {
        assert!(!references_key("{number:04d}"));
        assert!(!references_key("{number}"));
        assert!(!references_key("{{key}}-{number:04d}"));
        assert!(!references_key("{{project}}-{number:04d}"));
        assert!(references_key("x{{key}}-{key}"));
    }

    #[test]
    fn references_key_rejects_malformed_patterns() {
        for pattern in [
            "{ke{y}",
            "{key",
            "{project",
            "{bogus}-{number}",
            "{{",
            "}}",
            "",
            "v{number:03d}",
        ] {
            assert!(!references_key(pattern), "{pattern}");
        }
    }
}
