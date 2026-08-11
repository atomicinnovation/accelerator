//! Compiles a work-item `id_pattern` (the token DSL) into the ERE scan regex
//! whose first capture group is the id number run.
//!
//! It lives beside [`RegexScanner`](crate::RegexScanner) — the adapter that
//! compiles the scan-regex string this produces — so the whole
//! `work.id_pattern` → scanner pipeline sits in the corpus adapter layer,
//! shared by every Rust consumer.

// DSL pattern strings such as "{project}" are literal token markers, not
// format args.
#![allow(clippy::literal_string_with_formatting_args)]

use std::fmt::{self, Display, Formatter, Write as _};

use regex::Regex;

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
    WidthOverflow(usize),
    EmptyInput,
    NoMatch,
    UnrecognisedIdShape(String),
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
            Self::WidthOverflow(width) => write!(
                formatter,
                "{{number}} width {width} overflows the allocator's numeric \
                 cap"
            ),
            Self::EmptyInput => write!(formatter, "empty input"),
            Self::NoMatch => {
                write!(formatter, "input does not match the pattern")
            }
            Self::UnrecognisedIdShape(input) => write!(
                formatter,
                "input '{input}' is not a recognised ID shape"
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Scan,
    Format,
}

/// Finds the closing `}` for a token opened at `open` (index of `{`) with no
/// nesting permitted. Returns the token text and its total width including
/// braces.
fn find_token(
    chars: &[char],
    open: usize,
) -> Result<(String, usize), PatternError> {
    let mut j = open + 1;
    loop {
        match chars.get(j) {
            Some('}') => break,
            Some('{') => return Err(PatternError::NestedBrace(open)),
            Some(_) => j += 1,
            None => return Err(PatternError::UnclosedToken(open)),
        }
    }
    let token: String = chars[open + 1..j].iter().collect();
    Ok((token, j - open + 1))
}

/// Compiles one `{project}` or `{number...}` token into `out`, mode-driven.
/// Returns whether the token was a `{number}` token (for the caller's
/// `saw_number` tracking).
fn compile_token(
    token: &str,
    project_value: &str,
    mode: Mode,
    out: &mut String,
) -> Result<bool, PatternError> {
    if token == "project" {
        if project_value.is_empty() {
            return Err(PatternError::MissingProject);
        }
        if !is_valid_project(project_value) {
            return Err(PatternError::BadProjectValue(
                project_value.to_string(),
            ));
        }
        match mode {
            Mode::Scan => {
                for c in project_value.chars() {
                    push_escaped(c, out);
                }
            }
            Mode::Format => out.push_str(&project_value.replace('%', "%%")),
        }
        return Ok(false);
    }

    if token == "number"
        || (token.starts_with("number:") && token.len() > "number:".len())
    {
        let spec = if token == "number" {
            "04d"
        } else {
            &token["number:".len()..]
        };
        if !is_valid_number_spec(spec) {
            return Err(PatternError::BadFormatSpec(spec.to_string()));
        }
        match mode {
            Mode::Scan => out.push_str("([0-9]+)"),
            Mode::Format => {
                out.push('%');
                out.push_str(spec);
            }
        }
        return Ok(true);
    }

    Err(PatternError::UnknownToken(token.to_owned()))
}

/// Internal pattern compiler shared by [`compile_scan_regex`] and
/// [`compile_format_string`], mode-driven rather than duplicating the
/// token-walking loop per mode.
fn compile(
    pattern: &str,
    project_value: &str,
    mode: Mode,
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
            match mode {
                Mode::Scan => out.push_str("\\{"),
                Mode::Format => out.push('{'),
            }
            i += 2;
            last_was_dynamic = false;
            continue;
        }
        if ch == '}' && next == Some('}') {
            match mode {
                Mode::Scan => out.push_str("\\}"),
                Mode::Format => out.push('}'),
            }
            i += 2;
            last_was_dynamic = false;
            continue;
        }
        if ch == '}' {
            return Err(PatternError::UnmatchedBrace(i));
        }

        if ch == '{' {
            let (token, token_total) = find_token(&chars, i)?;
            if last_was_dynamic {
                return Err(PatternError::AdjacentTokens);
            }
            if compile_token(&token, project_value, mode, &mut out)? {
                saw_number = true;
            }
            last_was_dynamic = true;
            i += token_total;
            continue;
        }

        if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '<' | '>' | '|' | '"') {
            return Err(PatternError::HostileChar(ch));
        }
        match mode {
            Mode::Scan => push_escaped(ch, &mut out),
            Mode::Format => {
                if ch == '%' {
                    out.push_str("%%");
                } else {
                    out.push(ch);
                }
            }
        }
        i += 1;
        last_was_dynamic = false;
    }

    if !saw_number {
        return Err(PatternError::NoNumberToken);
    }
    match mode {
        Mode::Scan => Ok(format!("^{out}-")),
        Mode::Format => Ok(out),
    }
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
    compile(pattern, project_value, Mode::Scan)
}

/// Compile `pattern` into its `printf`-style format string.
///
/// # Errors
///
/// The same [`PatternError`] cases as [`compile_scan_regex`].
pub fn compile_format_string(
    pattern: &str,
    project_value: &str,
) -> Result<String, PatternError> {
    compile(pattern, project_value, Mode::Format)
}

/// The configured `{number}` width's cap: `10^N - 1`.
///
/// # Errors
///
/// [`PatternError::Empty`] for an empty pattern, [`PatternError::NoNumberToken`]
/// when no `{number}` token is present, or [`PatternError::WidthOverflow`] when
/// the configured width's cap does not fit in a `u64`.
pub fn pattern_max_number(pattern: &str) -> Result<u64, PatternError> {
    if pattern.is_empty() {
        return Err(PatternError::Empty);
    }
    let width = if let Some(width) = explicit_number_width(pattern) {
        width
    } else if pattern.contains("{number}") {
        4
    } else {
        return Err(PatternError::NoNumberToken);
    };
    let exponent = u32::try_from(width).unwrap_or(u32::MAX);
    let cap = 10u64
        .checked_pow(exponent)
        .ok_or(PatternError::WidthOverflow(width))?;
    cap.checked_sub(1).ok_or(PatternError::WidthOverflow(width))
}

/// Finds the width `N` of a `{number:0Nd}` token anywhere in `pattern`, by
/// blind substring search rather than a full token walk.
fn explicit_number_width(pattern: &str) -> Option<usize> {
    let idx = pattern.find("{number:0")?;
    let rest = &pattern[idx + "{number:0".len()..];
    let digit_end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let digits = &rest[..digit_end];
    if digits.is_empty() || digits.starts_with('0') {
        return None;
    }
    if rest[digit_end..].starts_with("d}") {
        digits.parse().ok()
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Project,
    Number,
}

/// Builds a capturing regex for `pattern` plus the ordered list of which
/// capture group is `{project}` vs `{number...}`.
fn build_full_id_regex(
    pattern: &str,
) -> Result<(String, Vec<TokenKind>), PatternError> {
    let chars: Vec<char> = pattern.chars().collect();
    let len = chars.len();
    let mut out = String::from("^");
    let mut order = Vec::new();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        if ch == '{' {
            let mut j = i + 1;
            let mut close = None;
            while j < len {
                if chars[j] == '}' {
                    close = Some(j);
                    break;
                }
                j += 1;
            }
            let close = close.ok_or(PatternError::UnclosedToken(i))?;
            let token: String = chars[i + 1..close].iter().collect();
            if token == "project" {
                out.push_str("([A-Za-z][A-Za-z0-9]*)");
                order.push(TokenKind::Project);
            } else if token == "number" || token.starts_with("number:") {
                out.push_str("([0-9]+)");
                order.push(TokenKind::Number);
            } else {
                return Err(PatternError::UnknownToken(token));
            }
            i = close + 1;
            continue;
        }
        push_escaped(ch, &mut out);
        i += 1;
    }
    out.push('$');
    Ok((out, order))
}

/// A full ID parsed against an `id_pattern`: the project code (when the
/// pattern carries `{project}`) and the number run, unpadded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedId {
    pub project: Option<String>,
    pub number: String,
}

/// Parses `id` against `pattern`.
///
/// # Errors
///
/// [`PatternError::NoMatch`] when `id` does not match `pattern`'s shape; other
/// [`PatternError`] variants when `pattern` itself is malformed.
pub fn parse_full_id(
    id: &str,
    pattern: &str,
) -> Result<ParsedId, PatternError> {
    if pattern.contains("{project}") {
        let (regex_str, order) = build_full_id_regex(pattern)?;
        let re = Regex::new(&regex_str).map_err(|_| PatternError::NoMatch)?;
        let captures = re.captures(id).ok_or(PatternError::NoMatch)?;
        let mut project = None;
        let mut number = None;
        for (index, kind) in order.iter().enumerate() {
            let value = captures.get(index + 1).map(|m| m.as_str().to_owned());
            match kind {
                TokenKind::Project => project = value,
                TokenKind::Number => number = value,
            }
        }
        let number = number.ok_or(PatternError::NoMatch)?;
        Ok(ParsedId { project, number })
    } else {
        compile_format_string(pattern, "")?;
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
            Ok(ParsedId {
                project: None,
                number: id.to_owned(),
            })
        } else {
            Err(PatternError::NoMatch)
        }
    }
}

fn strip_surrounding_quotes(input: &str) -> &str {
    let bytes = input.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &input[1..input.len() - 1]
    } else {
        input
    }
}

/// Applies a compiled `printf`-style format string (as produced by
/// [`compile_format_string`]) to `number`: `%%` becomes a literal `%`, the
/// single `%0Nd` token becomes `number` zero-padded to width `N`.
fn apply_number_format(format: &str, number: u64) -> String {
    let chars: Vec<char> = format.chars().collect();
    let len = chars.len();
    let mut out = String::new();
    let mut i = 0;
    while i < len {
        if chars[i] == '%' {
            if chars.get(i + 1) == Some(&'%') {
                out.push('%');
                i += 2;
                continue;
            }
            let mut j = i + 1;
            while j < len && chars[j] != 'd' {
                j += 1;
            }
            let spec: String = chars[i + 1..j].iter().collect();
            let width: usize =
                spec.trim_start_matches('0').parse().unwrap_or(0);
            let _ = write!(out, "{number:0width$}");
            i = j + 1;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Canonicalises a work-item ID, composing [`parse_full_id`] and
/// [`compile_format_string`] — no new parsing logic of its own.
///
/// # Errors
///
/// [`PatternError::EmptyInput`] for an empty (or empty-after-quote-stripping)
/// input, [`PatternError::MissingProject`] for a bare number under a
/// `{project}` pattern with no `project_value` supplied,
/// [`PatternError::UnrecognisedIdShape`] when `input` is neither a full ID nor
/// a bare number, or other [`PatternError`] variants when `pattern` itself is
/// malformed.
pub fn canonicalise_id(
    input: &str,
    pattern: &str,
    project_value: &str,
) -> Result<String, PatternError> {
    let input = strip_surrounding_quotes(input);
    if input.is_empty() {
        return Err(PatternError::EmptyInput);
    }

    let has_project = pattern.contains("{project}");

    if let Ok(parsed) = parse_full_id(input, pattern) {
        let project = parsed.project.unwrap_or_default();
        let number: u64 =
            parsed.number.parse().map_err(|_| PatternError::NoMatch)?;
        let format = compile_format_string(pattern, &project)
            .or_else(|_| compile_format_string(pattern, ""))?;
        return Ok(apply_number_format(&format, number));
    }

    if input.chars().all(|c| c.is_ascii_digit()) {
        let number: u64 = input.parse().map_err(|_| PatternError::NoMatch)?;
        if has_project {
            if project_value.is_empty() {
                return Err(PatternError::MissingProject);
            }
            let format = compile_format_string(pattern, project_value)?;
            return Ok(apply_number_format(&format, number));
        }
        let format = compile_format_string(pattern, "")?;
        return Ok(apply_number_format(&format, number));
    }

    Err(PatternError::UnrecognisedIdShape(input.to_owned()))
}

#[cfg(test)]
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
        // bad-spec: a spec has to be non-empty to be judged malformed.
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

    #[test]
    fn compile_format_string_examples() -> Result<(), PatternError> {
        assert_eq!(compile_format_string("{number:04d}", "")?, "%04d");
        assert_eq!(
            compile_format_string("{project}-{number:04d}", "PROJ")?,
            "PROJ-%04d"
        );
        assert_eq!(compile_format_string("{number}", "")?, "%04d");
        assert_eq!(compile_format_string("v{number:03d}", "")?, "v%03d");
        Ok(())
    }

    #[test]
    fn compile_format_string_escapes_literal_percent(
    ) -> Result<(), PatternError> {
        assert_eq!(compile_format_string("100%-{number}", "")?, "100%%-%04d");
        Ok(())
    }

    #[test]
    fn compile_format_string_shares_validation_with_scan() {
        assert!(matches!(
            compile_format_string("{project}-{number}", ""),
            Err(PatternError::MissingProject)
        ));
        assert!(matches!(
            compile_format_string("", ""),
            Err(PatternError::Empty)
        ));
    }

    #[test]
    fn pattern_max_number_examples() -> Result<(), PatternError> {
        assert_eq!(pattern_max_number("{number:04d}")?, 9999);
        assert_eq!(pattern_max_number("{number:05d}")?, 99_999);
        assert_eq!(pattern_max_number("{number}")?, 9999);
        assert_eq!(pattern_max_number("{project}-{number:03d}")?, 999);
        Ok(())
    }

    #[test]
    fn pattern_max_number_overflows_cleanly() {
        assert!(matches!(
            pattern_max_number("{number:020d}"),
            Err(PatternError::WidthOverflow(20))
        ));
    }

    #[test]
    fn pattern_max_number_requires_a_number_token() {
        assert!(matches!(
            pattern_max_number("{project}"),
            Err(PatternError::NoNumberToken)
        ));
        assert!(matches!(pattern_max_number(""), Err(PatternError::Empty)));
    }

    #[test]
    fn parse_full_id_with_project() -> Result<(), PatternError> {
        let parsed = parse_full_id("PROJ-0042", "{project}-{number:04d}")?;
        assert_eq!(parsed.project.as_deref(), Some("PROJ"));
        assert_eq!(parsed.number, "0042");
        Ok(())
    }

    #[test]
    fn parse_full_id_without_project() -> Result<(), PatternError> {
        let parsed = parse_full_id("0042", "{number:04d}")?;
        assert_eq!(parsed.project, None);
        assert_eq!(parsed.number, "0042");
        Ok(())
    }

    #[test]
    fn parse_full_id_rejects_a_non_matching_shape() {
        assert!(matches!(
            parse_full_id("not-an-id", "{number:04d}"),
            Err(PatternError::NoMatch)
        ));
        assert!(matches!(
            parse_full_id("42", "{project}-{number:04d}"),
            Err(PatternError::NoMatch)
        ));
    }

    #[test]
    fn canonicalise_id_matches_the_phase_1_golden() -> Result<(), PatternError>
    {
        assert_eq!(canonicalise_id("42", "{number:04d}", "")?, "0042");
        assert_eq!(canonicalise_id("0042", "{number:04d}", "")?, "0042");
        assert_eq!(canonicalise_id("\"0042\"", "{number:04d}", "")?, "0042");
        assert_eq!(canonicalise_id("'0042'", "{number:04d}", "")?, "0042");
        assert_eq!(
            canonicalise_id("42", "{project}-{number:04d}", "PROJ")?,
            "PROJ-0042"
        );
        assert_eq!(
            canonicalise_id("PROJ-0042", "{project}-{number:04d}", "PROJ")?,
            "PROJ-0042"
        );
        assert_eq!(
            canonicalise_id("PROJ-42", "{project}-{number:04d}", "PROJ")?,
            "PROJ-0042"
        );
        Ok(())
    }

    #[test]
    fn canonicalise_id_matches_the_phase_1_golden_error_arms() {
        assert!(matches!(
            canonicalise_id("42", "{project}-{number:04d}", ""),
            Err(PatternError::MissingProject)
        ));
        assert!(matches!(
            canonicalise_id("", "{number:04d}", ""),
            Err(PatternError::EmptyInput)
        ));
        assert!(matches!(
            canonicalise_id("not-an-id", "{number:04d}", ""),
            Err(PatternError::UnrecognisedIdShape(ref s)) if s == "not-an-id"
        ));
    }

    #[test]
    fn canonicalise_id_parent_equality() -> Result<(), PatternError> {
        assert_eq!(
            canonicalise_id("PROJ-0042", "{project}-{number:04d}", "PROJ")?,
            canonicalise_id("42", "{project}-{number:04d}", "PROJ")?
        );
        Ok(())
    }
}
