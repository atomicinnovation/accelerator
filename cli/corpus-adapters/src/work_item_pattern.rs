//! Compiles a work-item `id_pattern` (the token DSL) into the ERE scan regex
//! whose first capture group is the id number run.
//!
//! This is a Rust port of `_wip_compile` (scan mode) in
//! `skills/work/scripts/work-item-common.sh`; a parity test cross-checks the
//! output against that script so the two implementations cannot drift. It lives
//! beside [`RegexScanner`](crate::RegexScanner) — the adapter that compiles the
//! scan-regex string this produces — so the whole `work.id_pattern` → scanner
//! pipeline sits in the corpus adapter layer, shared by every Rust consumer.

use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    Empty,
    UnmatchedBrace(usize),
    NestedBrace(usize),
    UnclosedToken(usize),
    AdjacentTokens,
    MissingProject,
    BadProjectValue(String),
    BadFormatSpec(String),
    UnknownToken(String),
    HostileChar(char),
    NoNumberToken,
}

impl Display for PatternError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "pattern is empty"),
            Self::UnmatchedBrace(offset) => {
                write!(formatter, "unmatched '}}' at offset {offset}")
            }
            Self::NestedBrace(offset) => write!(
                formatter,
                "nested '{{' in token starting at offset {offset}"
            ),
            Self::UnclosedToken(offset) => {
                write!(formatter, "unclosed token starting at offset {offset}")
            }
            Self::AdjacentTokens => write!(
                formatter,
                "dynamic tokens must be separated by literal text (rule 3)"
            ),
            Self::MissingProject => write!(
                formatter,
                "pattern contains {{project}} but no value supplied"
            ),
            Self::BadProjectValue(value) => write!(
                formatter,
                "project value '{value}' must match [A-Za-z][A-Za-z0-9]* \
                 (rule 5)"
            ),
            Self::BadFormatSpec(spec) => write!(
                formatter,
                "{{number}} format spec '{spec}' must match 0Nd (rule 4)"
            ),
            Self::UnknownToken(token) => {
                write!(formatter, "unknown token '{{{token}}}' in pattern")
            }
            Self::HostileChar(character) => write!(
                formatter,
                "literal '{character}' is forbidden in patterns (rule 2)"
            ),
            Self::NoNumberToken => write!(
                formatter,
                "pattern must contain at least one {{number}} token (rule 1)"
            ),
        }
    }
}

impl std::error::Error for PatternError {}

fn push_escaped(c: char, out: &mut String) {
    if matches!(
        c,
        '.' | '^'
            | '$'
            | '*'
            | '+'
            | '?'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '|'
            | '\\'
    ) {
        out.push('\\');
    }
    out.push(c);
}

fn is_valid_project(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {
            chars.all(|c| c.is_ascii_alphanumeric())
        }
        _ => false,
    }
}

fn is_valid_number_spec(spec: &str) -> bool {
    let bytes = spec.as_bytes();
    bytes.len() >= 3
        && bytes[0] == b'0'
        && (b'1'..=b'9').contains(&bytes[1])
        && *bytes.last().unwrap_or(&0) == b'd'
        && bytes[2..bytes.len() - 1].iter().all(u8::is_ascii_digit)
}

/// Compile `pattern` into its ERE scan regex. `project_value` supplies the
/// substitution for a `{project}` token (empty when the pattern has none).
///
/// # Errors
///
/// A [`PatternError`] when the pattern is empty, malformed, uses an unknown or
/// adjacent token, carries a hostile literal, lacks a `{number}` token, or the
/// project value is required but absent or invalid.
pub fn compile_scan_regex(
    pattern: &str,
    project_value: &str,
) -> Result<String, PatternError> {
    if pattern.is_empty() {
        return Err(PatternError::Empty);
    }

    let chars: Vec<char> = pattern.chars().collect();
    let len = chars.len();
    let mut out = String::new();
    let mut saw_number = false;
    let mut last_was_dynamic = false;
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();

        if ch == '{' && next == Some('{') {
            out.push_str("\\{");
            i += 2;
            last_was_dynamic = false;
            continue;
        }
        if ch == '}' && next == Some('}') {
            out.push_str("\\}");
            i += 2;
            last_was_dynamic = false;
            continue;
        }
        if ch == '}' {
            return Err(PatternError::UnmatchedBrace(i));
        }

        if ch == '{' {
            let mut j = i + 1;
            let mut close = None;
            while j < len {
                match chars[j] {
                    '}' => {
                        close = Some(j);
                        break;
                    }
                    '{' => return Err(PatternError::NestedBrace(i)),
                    _ => j += 1,
                }
            }
            let close = close.ok_or(PatternError::UnclosedToken(i))?;
            let token: String = chars[i + 1..close].iter().collect();
            let token_total = close - i + 1;

            if last_was_dynamic {
                return Err(PatternError::AdjacentTokens);
            }

            if token == "project" {
                if project_value.is_empty() {
                    return Err(PatternError::MissingProject);
                }
                if !is_valid_project(project_value) {
                    return Err(PatternError::BadProjectValue(
                        project_value.to_string(),
                    ));
                }
                for c in project_value.chars() {
                    push_escaped(c, &mut out);
                }
                last_was_dynamic = true;
            } else if token == "number"
                || (token.starts_with("number:")
                    && token.len() > "number:".len())
            {
                let spec = if token == "number" {
                    "04d"
                } else {
                    &token["number:".len()..]
                };
                if !is_valid_number_spec(spec) {
                    return Err(PatternError::BadFormatSpec(spec.to_string()));
                }
                out.push_str("([0-9]+)");
                saw_number = true;
                last_was_dynamic = true;
            } else {
                return Err(PatternError::UnknownToken(token));
            }

            i += token_total;
            continue;
        }

        if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '<' | '>' | '|' | '"') {
            return Err(PatternError::HostileChar(ch));
        }
        push_escaped(ch, &mut out);
        i += 1;
        last_was_dynamic = false;
    }

    if !saw_number {
        return Err(PatternError::NoNumberToken);
    }
    Ok(format!("^{out}-"))
}

#[cfg(test)]
// Test inputs are DSL pattern strings whose `{…}` tokens are not format args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::*;

    #[test]
    fn numeric_default_pattern() -> Result<(), PatternError> {
        assert_eq!(compile_scan_regex("{number:04d}", "")?, "^([0-9]+)-");
        Ok(())
    }

    #[test]
    fn bare_number_token_defaults_spec() -> Result<(), PatternError> {
        assert_eq!(compile_scan_regex("{number}", "")?, "^([0-9]+)-");
        Ok(())
    }

    #[test]
    fn project_prefixed_pattern_escapes_and_substitutes(
    ) -> Result<(), PatternError> {
        assert_eq!(
            compile_scan_regex("{project}-{number:04d}", "PROJ")?,
            "^PROJ-([0-9]+)-"
        );
        Ok(())
    }

    #[test]
    fn literal_prefix_is_escaped() -> Result<(), PatternError> {
        assert_eq!(compile_scan_regex("v{number:03d}", "")?, "^v([0-9]+)-");
        // A regex metachar in the literal text is escaped.
        assert_eq!(compile_scan_regex("a.{number}", "")?, "^a\\.([0-9]+)-");
        Ok(())
    }

    #[test]
    fn escaped_braces_are_literal() -> Result<(), PatternError> {
        assert_eq!(
            compile_scan_regex("{{x}}{number}", "")?,
            "^\\{x\\}([0-9]+)-"
        );
        Ok(())
    }

    #[test]
    fn missing_number_token_is_rejected() {
        assert!(matches!(
            compile_scan_regex("{project}-", "PROJ"),
            Err(PatternError::NoNumberToken)
        ));
    }

    #[test]
    fn adjacent_dynamic_tokens_are_rejected() {
        assert!(matches!(
            compile_scan_regex("{project}{number}", "PROJ"),
            Err(PatternError::AdjacentTokens)
        ));
    }

    #[test]
    fn hostile_literal_is_rejected() {
        assert!(matches!(
            compile_scan_regex("a/b{number}", ""),
            Err(PatternError::HostileChar('/'))
        ));
    }

    #[test]
    fn bad_number_spec_is_rejected() {
        assert!(matches!(
            compile_scan_regex("{number:9x}", ""),
            Err(PatternError::BadFormatSpec(_))
        ));
    }

    #[test]
    fn unknown_token_is_rejected() {
        assert!(matches!(
            compile_scan_regex("{bogus}{number}", ""),
            Err(PatternError::UnknownToken(_))
        ));
        // `number:` with nothing after the colon is an unknown token, not a
        // bad-spec, matching the shell's `^number(:(.+))?$` match.
        assert!(matches!(
            compile_scan_regex("{number:}", ""),
            Err(PatternError::UnknownToken(_))
        ));
    }

    #[test]
    fn project_token_without_value_is_rejected() {
        assert!(matches!(
            compile_scan_regex("{project}-{number}", ""),
            Err(PatternError::MissingProject)
        ));
    }

    #[test]
    fn bad_project_value_is_rejected() {
        assert!(matches!(
            compile_scan_regex("{project}-{number}", "1PROJ"),
            Err(PatternError::BadProjectValue(_))
        ));
    }

    #[test]
    fn empty_pattern_is_rejected() {
        assert!(matches!(
            compile_scan_regex("", ""),
            Err(PatternError::Empty)
        ));
    }
}
