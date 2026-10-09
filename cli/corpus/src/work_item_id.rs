//! The work-item-ID runtime predicate and the injected scan and
//! canonicalisation ports.
//!
//! The pattern-DSL compiler that turns `work.id_pattern` into a scan regex is a
//! work/config concern; this crate takes the compiled scanner and canonicaliser
//! by injection so it never depends on `regex`.

use std::fmt::Display;
use std::fmt::Formatter;

/// A match produced by an [`IdScanner`] over a filename.
pub struct IdScan {
    pub digits: String,
    pub match_end: usize,
}

/// Recognises a work-item-ID prefix in a filename. Implemented in the adapter
/// layer over the compiled scan regex.
pub trait IdScanner {
    fn scan(&self, text: &str) -> Option<IdScan>;
}

/// Why a work-item ID could not be canonicalised: a refusal of the input, or
/// a pattern that is itself malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicaliseError {
    EmptyInput,
    MissingKey,
    UnrecognisedIdShape(String),
    NoMatch,
    MalformedPattern(String),
}

impl Display for CanonicaliseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput => write!(formatter, "empty input"),
            Self::MissingKey => write!(
                formatter,
                "pattern references the {{key}} prefix but no value supplied"
            ),
            Self::UnrecognisedIdShape(input) => write!(
                formatter,
                "input '{input}' is not a recognised ID shape"
            ),
            Self::NoMatch => {
                write!(formatter, "input does not match the pattern")
            }
            Self::MalformedPattern(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for CanonicaliseError {}

/// Canonicalises a work-item ID under an `id_pattern`. Implemented in the
/// adapter layer over the compiled pattern DSL.
pub trait WorkItemIdCanonicaliser {
    /// The canonical form of `input`, a full ID or a bare number, with
    /// `key_value` supplying the `{key}` prefix for a bare number.
    ///
    /// # Errors
    ///
    /// A [`CanonicaliseError`] when `input` is not an ID under `pattern`, or
    /// `pattern` is malformed.
    fn canonicalise(
        &self,
        input: &str,
        pattern: &str,
        key_value: &str,
    ) -> Result<String, CanonicaliseError>;
}

/// The identity scheme a workspace configures for its work items.
#[derive(Debug, Clone)]
pub struct WorkItemIdScheme {
    pub id_pattern: String,
    pub key: Option<String>,
}

impl Default for WorkItemIdScheme {
    fn default() -> Self {
        Self::numeric()
    }
}

impl WorkItemIdScheme {
    #[must_use]
    pub fn numeric() -> Self {
        Self {
            id_pattern: "{number:04d}".to_owned(),
            key: None,
        }
    }

    #[must_use]
    pub fn ownership(&self) -> IdOwnership {
        if self.id_pattern == TRACKER_TOKEN {
            IdOwnership::Tracker
        } else {
            IdOwnership::Local
        }
    }

    /// True iff `token` is exactly a canonical work-item-ID under this scheme:
    /// the correct project prefix (if any) and the exact configured digit width
    /// (or any non-empty digit run when the width is unspecified).
    #[must_use]
    pub fn is_canonical_id_token(&self, token: &str) -> bool {
        let width = self.canonical_digit_width();
        let digits = match &self.key {
            Some(code) => match token.strip_prefix(&format!("{code}-")) {
                Some(rest) => rest,
                None => return false,
            },
            None => token,
        };
        if width == 0 {
            !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
        } else {
            digits.len() == width && digits.chars().all(|c| c.is_ascii_digit())
        }
    }

    /// The zero-padded digit width from the pattern's `{number:0Nd}` segment, or
    /// `0` (admit any width) when unspecified.
    #[must_use]
    pub fn canonical_digit_width(&self) -> usize {
        let pattern = &self.id_pattern;
        let Some(start) = pattern.find("{number") else {
            return 0;
        };
        let rest = &pattern[start + "{number".len()..];
        let Some(end) = rest.find('}') else {
            return 0;
        };
        let spec = rest[..end].trim_start_matches(':').trim_end_matches('d');
        if spec.is_empty() {
            return 0;
        }
        let digits = spec.trim_start_matches('0');
        if digits.is_empty() {
            return 0;
        }
        digits.parse::<usize>().unwrap_or(0)
    }

    /// Validates and normalises a work-item-ID from any source. A prefixed form
    /// passes through verbatim; bare digits gain the configured project code.
    /// Under tracker ownership only a tracker key or a draft ID is an ID.
    #[must_use]
    pub fn normalise_id(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if self.ownership() == IdOwnership::Tracker {
            return canonical_draft_id(trimmed).or_else(|| {
                is_tracker_key(trimmed).then(|| trimmed.to_owned())
            });
        }
        if let Some((prefix, digits)) = trimmed.split_once('-') {
            if prefix.is_empty()
                || !prefix.chars().all(|c| c.is_ascii_alphabetic())
                || digits.is_empty()
                || !digits.chars().all(|c| c.is_ascii_digit())
            {
                return None;
            }
            return Some(trimmed.to_owned());
        }
        if !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(self.key.as_ref().map_or_else(
            || trimmed.to_owned(),
            |code| format!("{code}-{trimmed}"),
        ))
    }

    /// Canonicalises a raw cross-reference token to the exact form used as a
    /// `work_item_by_id` key. Bare digits are stripped of leading zeros and
    /// re-padded to the scheme's width (prefixed with the project code when the
    /// scheme carries one); an already-prefixed `<alnum>-<digits>` token passes
    /// through verbatim. Returns `None` for anything else. Distinct from
    /// [`Self::normalise_id`], which preserves the raw digit run rather than
    /// re-padding it.
    #[must_use]
    pub fn canonicalise_id(&self, raw: &str) -> Option<String> {
        if raw.is_empty() {
            return None;
        }
        let has_key = references_key(&self.id_pattern);
        let width = number_width(&self.id_pattern);
        if raw.chars().all(|c| c.is_ascii_digit()) {
            let n_str = raw
                .parse::<u64>()
                .map_or_else(|_| raw.to_owned(), |n| n.to_string());
            let padded = format!("{n_str:0>width$}");
            return Some(if has_key {
                match &self.key {
                    Some(code) => format!("{code}-{padded}"),
                    None => padded,
                }
            } else {
                padded
            });
        }
        if is_project_prefixed(raw) {
            return Some(raw.to_owned());
        }
        None
    }

    /// True iff `id` is a legacy bare-number ID: 1 to 4 ASCII digits with at
    /// least one non-zero digit.
    #[must_use]
    pub fn is_legacy_id(id: &str) -> bool {
        (1..=4).contains(&id.len())
            && id.chars().all(|c| c.is_ascii_digit())
            && id.chars().any(|c| c != '0')
    }

    /// Zero-pads `input` to 4 digits. `None` when `input` is not all-ASCII-digit
    /// (including empty).
    #[must_use]
    pub fn pad_legacy_number(input: &str) -> Option<String> {
        if input.is_empty() || !input.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let number: u64 = input.parse().ok()?;
        Some(format!("{number:04}"))
    }

    /// Extracts the full work-item-ID from a filename: the injected `scanner`
    /// supplies the primary digit run; a bare-numeric fallback keys legacy files
    /// when a project code is configured.
    #[must_use]
    pub fn extract_id(
        &self,
        filename: &str,
        scanner: &dyn IdScanner,
    ) -> Option<String> {
        if let Some(scan) = scanner.scan(filename) {
            let digits = scan.digits;
            return Some(match &self.key {
                Some(code) => format!("{code}-{digits}"),
                None => digits,
            });
        }
        let code = self.key.as_deref()?;
        let dash = filename.find('-')?;
        let prefix = &filename[..dash];
        if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(format!("{code}-{prefix}"))
    }
}

/// The pattern-DSL spellings that denote the local ID prefix token.
///
/// `key` is the domain spelling; `project` is the deprecated synonym carried
/// through the migration window so legacy `{project}` patterns keep resolving.
/// The single source of truth for the accepted spellings, shared across the ID
/// pipelines' tokenisers.
const KEY_TOKEN_SPELLINGS: &[&str] = &["key", "project"];

/// True iff `token` (the inner text of a `{...}` token) spells the local ID
/// prefix in any recognised spelling.
#[must_use]
pub fn is_key_token(token: &str) -> bool {
    KEY_TOKEN_SPELLINGS.contains(&token)
}

/// The sole-token pattern under which the tracker owns each work item's `id`.
pub const TRACKER_TOKEN: &str = "{tracker}";

/// Who sets a work item's `id`: the repository, from its pattern, or the
/// tracker, whose key the `id` then equals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdOwnership {
    Local,
    Tracker,
}

const DRAFT_PREFIX: &str = "draft-";
const DRAFT_SUFFIX_LENGTH: usize = 6;

const fn is_crockford_character(c: char) -> bool {
    c.is_ascii_digit()
        || (c.is_ascii_lowercase() && !matches!(c, 'i' | 'l' | 'o' | 'u'))
}

/// The lowercase form of `token` when it is a draft ID: `draft-` and six
/// lowercase Crockford base-32 characters, at least one a letter.
#[must_use]
pub fn canonical_draft_id(token: &str) -> Option<String> {
    let canonical = token.to_ascii_lowercase();
    let suffix = canonical.strip_prefix(DRAFT_PREFIX)?;
    let well_formed = suffix.len() == DRAFT_SUFFIX_LENGTH
        && suffix.chars().all(is_crockford_character)
        && suffix.chars().any(|c| c.is_ascii_alphabetic());
    well_formed.then_some(canonical)
}

/// True iff `token` is shaped like a Jira key or Linear identifier:
/// `[A-Za-z][A-Za-z0-9_]*-[0-9]+`.
#[must_use]
pub fn is_tracker_key(token: &str) -> bool {
    let Some((prefix, number)) = token.rsplit_once('-') else {
        return false;
    };
    let mut prefix_chars = prefix.chars();
    prefix_chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && prefix_chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
}

/// True iff `pattern` references the local ID prefix token in any recognised
/// spelling.
///
/// Brace-aware: an escaped `{{key}}` / `{{project}}` literal does not count, so
/// a false positive cannot spuriously reject a valid config.
#[must_use]
pub fn references_key(pattern: &str) -> bool {
    tokens_of(pattern).iter().any(|token| is_key_token(token))
}

/// The inner text of each `{...}` token in `pattern`, skipping escaped
/// `{{` / `}}` literals and malformed tokens.
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

/// A `work.id_pattern` that breaks a rule of the `{tracker}` token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdPatternError {
    TrackerNotSoleToken,
    TrackerNeedsJiraOrLinear,
}

impl std::fmt::Display for IdPatternError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TrackerNotSoleToken => formatter.write_str(
                "`{tracker}` must be the only token in `work.id_pattern`",
            ),
            Self::TrackerNeedsJiraOrLinear => formatter.write_str(
                "`{tracker}` requires `work.integration` to be `jira` or \
                 `linear`",
            ),
        }
    }
}

impl std::error::Error for IdPatternError {}

/// Checks the `{tracker}` rules: the token stands alone, and only under an
/// integration whose keys can serve as `id`s.
///
/// # Errors
///
/// The first [`IdPatternError`] rule `pattern` breaks.
pub fn validate_id_pattern(
    pattern: &str,
    supports_tracker_ids: bool,
) -> Result<(), IdPatternError> {
    let tracker_inner = &TRACKER_TOKEN[1..TRACKER_TOKEN.len() - 1];
    let references_tracker = tokens_of(pattern)
        .iter()
        .any(|token| token == tracker_inner);
    if !references_tracker {
        return Ok(());
    }
    if pattern != TRACKER_TOKEN {
        return Err(IdPatternError::TrackerNotSoleToken);
    }
    if !supports_tracker_ids {
        return Err(IdPatternError::TrackerNeedsJiraOrLinear);
    }
    Ok(())
}

/// The zero-pad width the canonical form uses: the `{number:0Nd}` segment's
/// `N`, or `4` when the pattern carries no explicit width.
fn number_width(pattern: &str) -> usize {
    let Some(start) = pattern.find("{number:") else {
        return 4;
    };
    let rest = &pattern[start + "{number:".len()..];
    let Some(dend) = rest.find("d}") else {
        return 4;
    };
    let spec = &rest[..dend];
    if spec.is_empty() {
        return 4;
    }
    let stripped = spec.trim_start_matches('0');
    let digits = if stripped.is_empty() { "0" } else { stripped };
    digits.parse().unwrap_or(4)
}

/// True iff `token` is an already-canonical `<alnum-starting-with-letter>-<digits>`
/// reference (e.g. `PROJ-0040`).
fn is_project_prefixed(token: &str) -> bool {
    let Some((prefix, digits)) = token.split_once('-') else {
        return false;
    };
    prefix
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && prefix.chars().all(|c| c.is_ascii_alphanumeric())
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_draft_id, is_key_token, is_tracker_key, references_key,
        validate_id_pattern, IdOwnership, IdPatternError, IdScan, IdScanner,
        WorkItemIdScheme, TRACKER_TOKEN,
    };

    fn scheme_of(id_pattern: &str) -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: id_pattern.to_owned(),
            key: None,
        }
    }

    #[test]
    fn tracker_alone_with_jira_or_linear_is_valid() {
        assert_eq!(validate_id_pattern(TRACKER_TOKEN, true), Ok(()));
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn tracker_with_another_token_fails_naming_the_only_token_rule() {
        for pattern in ["{tracker}-{number:04d}", "x{tracker}", "{tracker} "] {
            assert_eq!(
                validate_id_pattern(pattern, true),
                Err(IdPatternError::TrackerNotSoleToken),
                "{pattern}"
            );
        }
        assert_eq!(
            IdPatternError::TrackerNotSoleToken.to_string(),
            "`{tracker}` must be the only token in `work.id_pattern`"
        );
    }

    #[test]
    fn tracker_without_jira_or_linear_fails_naming_the_integration_rule() {
        assert_eq!(
            validate_id_pattern(TRACKER_TOKEN, false),
            Err(IdPatternError::TrackerNeedsJiraOrLinear)
        );
        assert_eq!(
            IdPatternError::TrackerNeedsJiraOrLinear.to_string(),
            "`{tracker}` requires `work.integration` to be `jira` or `linear`"
        );
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn a_pattern_without_tracker_is_valid_under_any_integration() {
        assert_eq!(validate_id_pattern("{number:04d}", false), Ok(()));
        assert_eq!(validate_id_pattern("{{tracker}}{number}", false), Ok(()));
    }

    #[test]
    fn a_tracker_scheme_reports_tracker_ownership() {
        assert_eq!(scheme_of(TRACKER_TOKEN).ownership(), IdOwnership::Tracker);
        assert_eq!(WorkItemIdScheme::numeric().ownership(), IdOwnership::Local);
    }

    #[test]
    fn jira_and_linear_keys_are_tracker_keys_in_either_case() {
        assert!(is_tracker_key("PP-760"));
        assert!(is_tracker_key("eng-42"));
        assert!(is_tracker_key("MY_PROJ-7"));
    }

    #[test]
    fn a_non_key_token_is_not_a_tracker_key() {
        assert!(!is_tracker_key("0230"));
        assert!(!is_tracker_key("PP-"));
        assert!(!is_tracker_key("-760"));
        assert!(!is_tracker_key("1PP-760"));
        assert!(!is_tracker_key("PP-76a"));
        assert!(!is_tracker_key("draft-k7mq3x"));
    }

    #[test]
    fn a_draft_id_canonicalises_to_lowercase() {
        assert_eq!(
            canonical_draft_id("Draft-K7MQ3X").as_deref(),
            Some("draft-k7mq3x")
        );
        assert_eq!(canonical_draft_id("draft-k7mq3"), None);
        assert_eq!(canonical_draft_id("draft-123456"), None);
        assert_eq!(canonical_draft_id("draft-k7mq3i"), None);
        assert_eq!(canonical_draft_id("PP-760"), None);
    }

    #[test]
    fn a_tracker_scheme_normalises_tracker_keys_and_draft_ids() {
        let tracker = scheme_of(TRACKER_TOKEN);

        assert_eq!(
            tracker.normalise_id("MY_PROJ-7").as_deref(),
            Some("MY_PROJ-7")
        );
        assert_eq!(
            tracker.normalise_id(" ABC2-15 ").as_deref(),
            Some("ABC2-15")
        );
        assert_eq!(
            tracker.normalise_id("Draft-K7MQ3X").as_deref(),
            Some("draft-k7mq3x")
        );
        assert_eq!(tracker.normalise_id("0042"), None);
        assert_eq!(WorkItemIdScheme::numeric().normalise_id("MY_PROJ-7"), None);
        assert_eq!(
            WorkItemIdScheme::numeric().normalise_id("draft-k7mq3x"),
            None
        );
    }

    #[test]
    fn is_key_token_accepts_both_spellings() {
        assert!(is_key_token("key"));
        assert!(is_key_token("project"));
        assert!(!is_key_token("number"));
        assert!(!is_key_token("bogus"));
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn references_key_is_brace_aware() {
        assert!(references_key("{key}-{number:04d}"));
        assert!(references_key("{project}-{number:04d}"));
        assert!(!references_key("{number:04d}"));
        assert!(!references_key("{number}"));
        assert!(!references_key("{{key}}-{number:04d}"));
        assert!(!references_key("{{project}}-{number:04d}"));
        assert!(references_key("x{{key}}-{key}"));
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn references_key_rejects_malformed_patterns() {
        assert!(!references_key("{ke{y}"));
        assert!(!references_key("{key"));
        assert!(!references_key("{project"));
        assert!(!references_key("{bogus}-{number}"));
        assert!(!references_key("{{"));
        assert!(!references_key("}}"));
        assert!(!references_key(""));
        assert!(!references_key("v{number:03d}"));
    }

    struct DigitRunScanner;

    impl IdScanner for DigitRunScanner {
        fn scan(&self, text: &str) -> Option<IdScan> {
            let digits: String =
                text.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() || !text[digits.len()..].starts_with('-') {
                return None;
            }
            let match_end = digits.len() + 1;
            Some(IdScan { digits, match_end })
        }
    }

    fn project(code: &str, width: usize) -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: format!("{{project}}-{{number:0{width}d}}"),
            key: Some(code.to_owned()),
        }
    }

    #[test]
    fn canonical_digit_width_reads_the_pattern() {
        assert_eq!(WorkItemIdScheme::numeric().canonical_digit_width(), 4);
        let any = WorkItemIdScheme {
            id_pattern: "{number}".to_owned(),
            key: None,
        };
        assert_eq!(any.canonical_digit_width(), 0);
        let admit_any = WorkItemIdScheme {
            id_pattern: "{number:0d}".to_owned(),
            key: None,
        };
        assert_eq!(admit_any.canonical_digit_width(), 0);
    }

    #[test]
    fn is_canonical_id_token_under_default_numeric() {
        let scheme = WorkItemIdScheme::numeric();
        assert!(scheme.is_canonical_id_token("0040"));
        assert!(!scheme.is_canonical_id_token("40"));
        assert!(!scheme.is_canonical_id_token("00040"));
        assert!(!scheme.is_canonical_id_token("100"));
        assert!(!scheme.is_canonical_id_token("004A"));
        assert!(!scheme.is_canonical_id_token(""));
    }

    #[test]
    fn is_canonical_id_token_under_project_pattern() {
        let scheme = project("PROJ", 4);
        assert!(scheme.is_canonical_id_token("PROJ-0040"));
        assert!(!scheme.is_canonical_id_token("0040"));
        assert!(!scheme.is_canonical_id_token("PROJ-40"));
        assert!(!scheme.is_canonical_id_token("OTHER-0040"));
    }

    #[test]
    fn normalise_id_handles_prefixed_and_bare_forms() {
        let numeric = WorkItemIdScheme::numeric();
        assert_eq!(
            numeric.normalise_id("ENG-0042").as_deref(),
            Some("ENG-0042")
        );
        assert_eq!(numeric.normalise_id("0042").as_deref(), Some("0042"));
        assert_eq!(numeric.normalise_id("ENG0042"), None);
        assert_eq!(numeric.normalise_id("PROJ-1.2"), None);
        assert_eq!(numeric.normalise_id("  0042  ").as_deref(), Some("0042"));

        let eng = project("ENG", 4);
        assert_eq!(eng.normalise_id("42").as_deref(), Some("ENG-42"));
        assert_eq!(eng.normalise_id("OPS-7").as_deref(), Some("OPS-7"));
    }

    #[test]
    fn canonicalise_id_pads_and_prefixes_per_scheme() {
        let numeric = WorkItemIdScheme::numeric();
        assert_eq!(numeric.canonicalise_id("40").as_deref(), Some("0040"));
        assert_eq!(numeric.canonicalise_id("00040").as_deref(), Some("0040"));
        assert_eq!(numeric.canonicalise_id("12345").as_deref(), Some("12345"));
        assert_eq!(
            numeric.canonicalise_id("ENG-0042").as_deref(),
            Some("ENG-0042")
        );
        assert_eq!(numeric.canonicalise_id("PR1-7").as_deref(), Some("PR1-7"));
        assert_eq!(numeric.canonicalise_id(""), None);
        assert_eq!(numeric.canonicalise_id("AB-12-34"), None);
        assert_eq!(numeric.canonicalise_id("1A-2"), None);

        let proj = project("PROJ", 4);
        assert_eq!(proj.canonicalise_id("40").as_deref(), Some("PROJ-0040"));
        assert_eq!(proj.canonicalise_id("OTHER-9").as_deref(), Some("OTHER-9"));
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn canonicalise_id_treats_key_pattern_as_prefixed() {
        let scheme = WorkItemIdScheme {
            id_pattern: "{key}-{number:04d}".to_owned(),
            key: Some("PP".to_owned()),
        };
        assert_eq!(scheme.canonicalise_id("40").as_deref(), Some("PP-0040"));
        assert_eq!(
            scheme.canonicalise_id("OTHER-9").as_deref(),
            Some("OTHER-9")
        );
    }

    #[test]
    fn is_legacy_id_accepts_one_to_four_nonzero_digits() {
        assert!(WorkItemIdScheme::is_legacy_id("42"));
        assert!(WorkItemIdScheme::is_legacy_id("0042"));
        assert!(WorkItemIdScheme::is_legacy_id("9"));
        assert!(!WorkItemIdScheme::is_legacy_id("0000"));
        assert!(!WorkItemIdScheme::is_legacy_id("12345"));
        assert!(!WorkItemIdScheme::is_legacy_id(""));
        assert!(!WorkItemIdScheme::is_legacy_id("12a4"));
    }

    #[test]
    fn pad_legacy_number_zero_pads_to_four_digits() {
        assert_eq!(
            WorkItemIdScheme::pad_legacy_number("42").as_deref(),
            Some("0042")
        );
        assert_eq!(
            WorkItemIdScheme::pad_legacy_number("12345").as_deref(),
            Some("12345")
        );
        assert_eq!(WorkItemIdScheme::pad_legacy_number(""), None);
        assert_eq!(WorkItemIdScheme::pad_legacy_number("12a"), None);
    }

    #[test]
    fn extract_id_uses_the_scanner_then_the_fallback() {
        let numeric = WorkItemIdScheme::numeric();
        assert_eq!(
            numeric
                .extract_id("0042-foo.md", &DigitRunScanner)
                .as_deref(),
            Some("0042")
        );
        assert_eq!(numeric.extract_id("malformed.md", &DigitRunScanner), None);

        let proj = project("PROJ", 4);
        assert_eq!(
            proj.extract_id("0042-legacy.md", &DigitRunScanner)
                .as_deref(),
            Some("PROJ-0042")
        );
    }
}
