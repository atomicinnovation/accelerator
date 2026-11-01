//! The identity scheme a workspace configures for its work items: its
//! validated pattern and key, and every reading of an ID under them.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use crate::draft_id::DraftId;
use crate::id_pattern::IdKey;
use crate::id_pattern::IdParts;
use crate::id_pattern::IdPattern;
use crate::id_pattern::MissingKey;
pub use crate::id_pattern::TRACKER_TOKEN;

/// Who sets a work item's `id`: the repository, from its pattern, or the
/// tracker, whose key the `id` then equals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdOwnership {
    Local,
    Tracker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicaliseError {
    EmptyInput,
    MissingKey,
    UnrecognisedIdShape(String),
    NoMatch,
}

impl Display for CanonicaliseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(formatter, "empty input"),
            Self::MissingKey => MissingKey.fmt(formatter),
            Self::UnrecognisedIdShape(input) => write!(
                formatter,
                "input '{input}' is not a recognised ID shape"
            ),
            Self::NoMatch => {
                write!(formatter, "input does not match the pattern")
            }
        }
    }
}

impl std::error::Error for CanonicaliseError {}

impl From<MissingKey> for CanonicaliseError {
    fn from(_: MissingKey) -> Self {
        Self::MissingKey
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerIntegrationError {
    TrackerNeedsJiraOrLinear,
}

impl Display for TrackerIntegrationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TrackerNeedsJiraOrLinear => formatter.write_str(
                "`{tracker}` requires `work.integration` to be `jira` or \
                 `linear`",
            ),
        }
    }
}

impl std::error::Error for TrackerIntegrationError {}

/// # Errors
///
/// [`TrackerIntegrationError::TrackerNeedsJiraOrLinear`] when `pattern` is
/// `{tracker}` under an integration whose keys cannot serve as `id`s.
pub fn check_tracker_integration(
    pattern: &IdPattern,
    supports_tracker_ids: bool,
) -> Result<(), TrackerIntegrationError> {
    if pattern.is_tracker_owned() && !supports_tracker_ids {
        Err(TrackerIntegrationError::TrackerNeedsJiraOrLinear)
    } else {
        Ok(())
    }
}

/// Whether `token` is an ID only a tracker can own: a draft ID or a tracker
/// key.
#[must_use]
pub fn is_tracker_owned_token(token: &str) -> bool {
    DraftId::parse(token).is_some() || tracker::is_tracker_key(token)
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// A `<key>-<digits>` value whose key is a letter then letters and digits.
fn keyed_digits(value: &str) -> Option<(&str, &str)> {
    let (key, digits) = value.split_once('-')?;
    (IdKey::parse(key).is_ok() && is_digits(digits)).then_some((key, digits))
}

/// The tracker key leading `filename`, followed by the `-` that starts the
/// slug, with where the slug starts.
fn tracker_key_prefix(filename: &str) -> Option<(&str, &str, usize)> {
    let (key, rest) = filename.split_once('-')?;
    let digit_count = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (digits, after) = rest.split_at(digit_count);
    let token = &filename[..=key.len() + digit_count];
    (tracker::is_tracker_key(token) && after.starts_with('-'))
        .then(|| (key, digits, token.len() + 1))
}

fn legacy_prefix(filename: &str) -> Option<(&str, usize)> {
    let (digits, _) = filename.split_once('-')?;
    is_digits(digits).then(|| (digits, digits.len() + 1))
}

fn draft_prefix(filename: &str) -> Option<(DraftId, usize)> {
    let draft_length = DraftId::PREFIX.len() + DRAFT_SUFFIX_LENGTH;
    let candidate = filename.get(..draft_length)?;
    let followed_by_slug = filename[draft_length..].starts_with('-');
    DraftId::parse(candidate)
        .filter(|_| followed_by_slug)
        .map(|draft| (draft, draft_length + 1))
}

const DRAFT_SUFFIX_LENGTH: usize = 6;

/// The identity scheme a workspace configures for its work items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemIdScheme {
    pattern: IdPattern,
    key: Option<IdKey>,
}

impl WorkItemIdScheme {
    /// The scheme of `pattern`, holding `key` only when the pattern has a
    /// key token.
    #[must_use]
    pub fn new(pattern: IdPattern, key: Option<IdKey>) -> Self {
        let key = key.filter(|_| pattern.references_key());
        Self { pattern, key }
    }

    #[must_use]
    pub fn numeric() -> Self {
        let pattern = IdPattern::parse("{number:04d}")
            .unwrap_or_else(|_| unreachable!("the default pattern parses"));
        Self::new(pattern, None)
    }

    #[must_use]
    pub const fn pattern(&self) -> &IdPattern {
        &self.pattern
    }

    #[must_use]
    pub const fn key(&self) -> Option<&IdKey> {
        self.key.as_ref()
    }

    #[must_use]
    pub fn ownership(&self) -> IdOwnership {
        if self.pattern.is_tracker_owned() {
            IdOwnership::Tracker
        } else {
            IdOwnership::Local
        }
    }

    fn width(&self) -> usize {
        self.pattern.width().unwrap_or(LEGACY_WIDTH)
    }

    fn is_own_key(&self, key: &str) -> bool {
        self.key
            .as_ref()
            .is_some_and(|own| own.as_str().eq_ignore_ascii_case(key))
    }

    /// `key` in the spelling a join key uses: `work.key`'s when it names the
    /// same key, otherwise upper case.
    fn folded(&self, key: &str) -> String {
        match &self.key {
            Some(own) if own.as_str().eq_ignore_ascii_case(key) => {
                own.as_str().to_owned()
            }
            _ => key.to_ascii_uppercase(),
        }
    }

    /// Whether `token` is an ID in the pattern's exact shape under this
    /// scheme's own key, ignoring the key's letter case.
    #[must_use]
    pub fn is_canonical_id_token(&self, token: &str) -> bool {
        self.pattern.read_exact(token).is_some_and(|parts| {
            parts.key().is_none_or(|key| self.is_own_key(key))
        })
    }

    /// Whether `token` is an ID in the pattern's exact shape under any key.
    #[must_use]
    pub fn is_any_key_id_token(&self, token: &str) -> bool {
        self.pattern.read_exact(token).is_some()
    }

    /// A work-item ID from any source, as written: a `<key>-<digits>` value
    /// verbatim, bare digits under `work.key`, and under `{tracker}` a
    /// tracker key or a draft ID.
    #[must_use]
    pub fn normalise_id(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if self.ownership() == IdOwnership::Tracker {
            return DraftId::parse(trimmed)
                .map(|draft| draft.as_str().to_owned())
                .or_else(|| {
                    tracker::is_tracker_key(trimmed).then(|| trimmed.to_owned())
                });
        }
        if trimmed.contains('-') {
            return keyed_digits(trimmed).map(|_| trimmed.to_owned());
        }
        if !is_digits(trimmed) {
            return None;
        }
        Some(self.key.as_ref().map_or_else(
            || trimmed.to_owned(),
            |key| format!("{}-{trimmed}", key.as_str()),
        ))
    }

    /// `input`, a full ID or a bare number, rendered through the pattern; a
    /// full ID keeps its key. Surrounding quotes are stripped, and under
    /// `{tracker}` any input passes through.
    ///
    /// # Errors
    ///
    /// [`CanonicaliseError`] when `input` is empty, neither a full ID nor a
    /// bare number, or a bare number under a key pattern with no key.
    pub fn canonicalise(
        &self,
        input: &str,
    ) -> Result<String, CanonicaliseError> {
        let input = without_surrounding_quotes(input);
        if input.is_empty() {
            return Err(CanonicaliseError::EmptyInput);
        }
        if self.pattern.is_tracker_owned() {
            return Ok(input.to_owned());
        }
        if let Some(parts) = IdParts::parse(&self.pattern, input) {
            return Ok(self.render_keeping_key(&parts)?);
        }
        if is_digits(input) {
            let number =
                input.parse().map_err(|_| CanonicaliseError::NoMatch)?;
            return Ok(self.pattern.render(self.key.as_ref(), number)?);
        }
        Err(CanonicaliseError::UnrecognisedIdShape(input.to_owned()))
    }

    fn render_keeping_key(
        &self,
        parts: &IdParts,
    ) -> Result<String, MissingKey> {
        let key = parts.key().and_then(|key| IdKey::parse(key).ok());
        self.pattern.render(key.as_ref(), parts.number())
    }

    /// The key every spelling of one item joins on: the number padded, and
    /// the key in `work.key`'s spelling when it names `work.key`, otherwise
    /// in upper case. A bare number reads under `work.key`.
    #[must_use]
    pub fn identity_key(&self, reference: &str) -> Option<String> {
        if self.pattern.is_tracker_owned() {
            return Self::tracker_identity_key(reference);
        }
        if is_digits(reference) {
            let number: u64 = reference.parse().ok()?;
            return Some(
                self.pattern
                    .render(self.key.as_ref(), number)
                    .unwrap_or_else(|_| self.padded(number)),
            );
        }
        if let Some(parts) = IdParts::parse(&self.pattern, reference) {
            return self.joined(&parts);
        }
        let (key, digits) = keyed_digits(reference)?;
        let number: u64 = digits.parse().ok()?;
        Some(format!("{}-{}", self.folded(key), self.padded(number)))
    }

    fn tracker_identity_key(reference: &str) -> Option<String> {
        if let Some(draft) = DraftId::parse(reference) {
            return Some(draft.as_str().to_owned());
        }
        if tracker::is_tracker_key(reference) {
            let (key, digits) = reference.rsplit_once('-')?;
            return Some(format!("{}-{digits}", key.to_ascii_uppercase()));
        }
        Self::pad_legacy_number(reference)
    }

    fn padded(&self, number: u64) -> String {
        let width = self.width();
        format!("{number:0width$}")
    }

    fn joined(&self, parts: &IdParts) -> Option<String> {
        if self.pattern.is_tracker_owned() {
            return Some(match parts.key() {
                Some(key) => {
                    format!("{}-{}", key.to_ascii_uppercase(), parts.digits())
                }
                None => Self::pad_legacy_number(parts.digits())?,
            });
        }
        let key = parts
            .key()
            .map(|key| IdKey::parse(&self.folded(key)))
            .transpose()
            .ok()?;
        self.pattern.render(key.as_ref(), parts.number()).ok()
    }

    /// A held `id`, alias or `external_id` read as an ID under this scheme:
    /// through the pattern, or as a bare legacy number under `work.key`;
    /// under `{tracker}`, a tracker key as written.
    #[must_use]
    pub fn read_held(&self, value: &str) -> Option<IdParts> {
        if self.pattern.is_tracker_owned() {
            let (key, digits) = value.rsplit_once('-')?;
            return tracker::is_tracker_key(value)
                .then(|| IdParts::of(Some(key), digits))
                .flatten();
        }
        IdParts::parse(&self.pattern, value)
            .or_else(|| self.legacy_under_own_key(value))
    }

    fn legacy_under_own_key(&self, digits: &str) -> Option<IdParts> {
        let key = self.key.as_ref()?;
        is_digits(digits)
            .then(|| IdParts::of(Some(key.as_str()), digits))
            .flatten()
    }

    /// The ID leading `filename`, read as [`Self::read_held`] reads a value,
    /// with where the slug starts; under `{tracker}`, a bare legacy number
    /// too.
    #[must_use]
    pub fn read_held_prefix(&self, filename: &str) -> Option<(IdParts, usize)> {
        if self.pattern.is_tracker_owned() {
            if let Some((key, digits, slug_start)) =
                tracker_key_prefix(filename)
            {
                return Some((IdParts::of(Some(key), digits)?, slug_start));
            }
            let (digits, slug_start) = legacy_prefix(filename)?;
            return Some((IdParts::of(None, digits)?, slug_start));
        }
        IdParts::parse_prefix(&self.pattern, filename).or_else(|| {
            let (digits, slug_start) = legacy_prefix(filename)?;
            Some((self.legacy_under_own_key(digits)?, slug_start))
        })
    }

    /// Whether `held` names `id`: it reads as an ID with the same key,
    /// ignoring case, and the same number. Under `{tracker}`, a tracker key
    /// holds only the same key and digits as written.
    #[must_use]
    pub fn holds(&self, held: &str, id: &IdParts) -> bool {
        let Some(reading) = self.read_held(held) else {
            return false;
        };
        if self.pattern.is_tracker_owned() {
            reading.same_id(id) && reading.digits() == id.digits()
        } else {
            reading.same_id(id)
        }
    }

    /// True iff `id` is a legacy bare-number ID: 1 to 4 ASCII digits with at
    /// least one non-zero digit.
    #[must_use]
    pub fn is_legacy_id(id: &str) -> bool {
        (1..=LEGACY_WIDTH).contains(&id.len())
            && is_digits(id)
            && id.bytes().any(|byte| byte != b'0')
    }

    /// `input` zero-padded to 4 digits; `None` when it is not all digits.
    #[must_use]
    pub fn pad_legacy_number(input: &str) -> Option<String> {
        if !is_digits(input) {
            return None;
        }
        let number: u64 = input.parse().ok()?;
        Some(format!("{number:0LEGACY_WIDTH$}"))
    }

    fn filename_reading(&self, filename: &str) -> Option<(String, usize)> {
        if self.pattern.is_tracker_owned() {
            if let Some((draft, slug_start)) = draft_prefix(filename) {
                return Some((draft.as_str().to_owned(), slug_start));
            }
        }
        let (parts, slug_start) = self.read_held_prefix(filename)?;
        Some((self.joined(&parts)?, slug_start))
    }
}

const LEGACY_WIDTH: usize = 4;

fn without_surrounding_quotes(input: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|quote| {
            input
                .strip_prefix(*quote)
                .and_then(|rest| rest.strip_suffix(*quote))
        })
        .unwrap_or(input)
}

impl Default for WorkItemIdScheme {
    fn default() -> Self {
        Self::numeric()
    }
}

impl corpus::IdConvention for WorkItemIdScheme {
    fn slug_start(&self, stem: &str) -> Option<usize> {
        self.filename_reading(stem)
            .map(|(_, slug_start)| slug_start)
    }

    fn is_id(&self, token: &str) -> bool {
        self.is_canonical_id_token(token)
    }

    fn identity_key(&self, reference: &str) -> Option<String> {
        Self::identity_key(self, reference)
    }

    fn id_in_filename(&self, filename: &str) -> Option<String> {
        self.filename_reading(filename)
            .map(|(identity, _)| identity)
    }
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::unwrap_used, clippy::literal_string_with_formatting_args)]
mod tests {
    use corpus::IdConvention;

    use super::check_tracker_integration;
    use super::is_tracker_owned_token;
    use super::CanonicaliseError;
    use super::IdOwnership;
    use super::TrackerIntegrationError;
    use super::WorkItemIdScheme;
    use super::TRACKER_TOKEN;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdParts;
    use crate::id_pattern::IdPattern;

    fn scheme(pattern: &str, key: Option<&str>) -> WorkItemIdScheme {
        WorkItemIdScheme::new(
            IdPattern::parse(pattern).unwrap(),
            key.map(|key| IdKey::parse(key).unwrap()),
        )
    }

    fn keyed(key: &str) -> WorkItemIdScheme {
        scheme("{key}-{number:04d}", Some(key))
    }

    fn project(key: &str) -> WorkItemIdScheme {
        scheme("{project}-{number:04d}", Some(key))
    }

    fn tracker() -> WorkItemIdScheme {
        scheme(TRACKER_TOKEN, None)
    }

    fn identity(scheme: &WorkItemIdScheme, reference: &str) -> Option<String> {
        scheme.identity_key(reference)
    }

    fn canonical(
        scheme: &WorkItemIdScheme,
        input: &str,
    ) -> Result<String, CanonicaliseError> {
        scheme.canonicalise(input)
    }

    #[test]
    fn tracker_alone_with_jira_or_linear_is_valid() {
        let pattern = IdPattern::parse(TRACKER_TOKEN).unwrap();
        assert_eq!(check_tracker_integration(&pattern, true), Ok(()));
    }

    #[test]
    fn tracker_without_jira_or_linear_fails_naming_the_integration_rule() {
        let pattern = IdPattern::parse(TRACKER_TOKEN).unwrap();
        assert_eq!(
            check_tracker_integration(&pattern, false),
            Err(TrackerIntegrationError::TrackerNeedsJiraOrLinear)
        );
        assert_eq!(
            TrackerIntegrationError::TrackerNeedsJiraOrLinear.to_string(),
            "`{tracker}` requires `work.integration` to be `jira` or `linear`"
        );
    }

    #[test]
    fn a_pattern_without_tracker_is_valid_under_any_integration() {
        let pattern = IdPattern::parse("{number:04d}").unwrap();
        assert_eq!(check_tracker_integration(&pattern, false), Ok(()));
    }

    #[test]
    fn a_tracker_scheme_reports_tracker_ownership() {
        assert_eq!(tracker().ownership(), IdOwnership::Tracker);
        assert_eq!(WorkItemIdScheme::numeric().ownership(), IdOwnership::Local);
    }

    #[test]
    fn a_scheme_drops_a_key_its_pattern_does_not_reference() {
        assert_eq!(scheme("{number:04d}", Some("PP")).key(), None);
        assert_eq!(keyed("PP").key().map(IdKey::as_str), Some("PP"));
    }

    #[test]
    fn a_tracker_scheme_normalises_tracker_keys_and_draft_ids() {
        let tracker = tracker();

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
    fn a_tracker_owned_token_is_a_draft_id_or_a_tracker_key() {
        assert!(is_tracker_owned_token("draft-k7mq3x"));
        assert!(is_tracker_owned_token("PROJ-123"));
        assert!(!is_tracker_owned_token("0400"));
        assert!(!is_tracker_owned_token("PP_0400"));
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
        let scheme = project("PROJ");
        assert!(scheme.is_canonical_id_token("PROJ-0040"));
        assert!(!scheme.is_canonical_id_token("0040"));
        assert!(!scheme.is_canonical_id_token("PROJ-40"));
        assert!(!scheme.is_canonical_id_token("OTHER-0040"));
        assert!(project("PP").is_canonical_id_token("pp-0040"));
    }

    #[test]
    fn canonical_tokens_take_the_configured_key_in_any_case() {
        let scheme = keyed("PP");
        assert!(scheme.is_canonical_id_token("PP-0400"));
        assert!(scheme.is_canonical_id_token("pp-0400"));
        assert!(!scheme.is_canonical_id_token("XY-0400"));
    }

    #[test]
    fn a_bare_number_pattern_strips_only_four_digit_ids() {
        let scheme = scheme("{number}", None);
        assert!(scheme.is_canonical_id_token("0042"));
        assert!(!scheme.is_canonical_id_token("42"));
    }

    #[test]
    fn any_key_tokens_take_every_rule_five_key() {
        let scheme = keyed("PP");
        assert!(scheme.is_any_key_id_token("PP-0400"));
        assert!(scheme.is_any_key_id_token("xy-0400"));
        assert!(scheme.is_any_key_id_token("X1-0400"));
        assert_eq!(scheme.normalise_id("X1-0400").as_deref(), Some("X1-0400"));
    }

    #[test]
    fn normalise_id_handles_prefixed_and_bare_forms() {
        let numeric = WorkItemIdScheme::numeric();
        assert_eq!(
            numeric.normalise_id("ENG-0042").as_deref(),
            Some("ENG-0042")
        );
        assert_eq!(numeric.normalise_id("X1-0400").as_deref(), Some("X1-0400"));
        assert_eq!(numeric.normalise_id("0042").as_deref(), Some("0042"));
        assert_eq!(numeric.normalise_id("ENG0042"), None);
        assert_eq!(numeric.normalise_id("PROJ-1.2"), None);
        assert_eq!(numeric.normalise_id("  0042  ").as_deref(), Some("0042"));

        let eng = project("ENG");
        assert_eq!(eng.normalise_id("42").as_deref(), Some("ENG-42"));
        assert_eq!(eng.normalise_id("OPS-7").as_deref(), Some("OPS-7"));
    }

    #[test]
    fn canonicalise_pads_and_prefixes_per_scheme() {
        let numeric = WorkItemIdScheme::numeric();
        assert_eq!(canonical(&numeric, "40").as_deref(), Ok("0040"));
        assert_eq!(canonical(&numeric, "00040").as_deref(), Ok("0040"));
        assert_eq!(canonical(&numeric, "12345").as_deref(), Ok("12345"));
        for foreign in ["ENG-0042", "PR1-7", "AB-12-34", "1A-2"] {
            assert_eq!(
                canonical(&numeric, foreign),
                Err(CanonicaliseError::UnrecognisedIdShape(foreign.to_owned())),
                "{foreign}"
            );
        }
        assert_eq!(canonical(&numeric, ""), Err(CanonicaliseError::EmptyInput));

        let proj = project("PROJ");
        assert_eq!(canonical(&proj, "40").as_deref(), Ok("PROJ-0040"));
        assert_eq!(canonical(&proj, "OTHER-9").as_deref(), Ok("OTHER-0009"));
    }

    #[test]
    fn canonicalise_treats_key_pattern_as_prefixed() {
        let scheme = keyed("PP");
        assert_eq!(canonical(&scheme, "40").as_deref(), Ok("PP-0040"));
        assert_eq!(canonical(&scheme, "OTHER-9").as_deref(), Ok("OTHER-0009"));
    }

    #[test]
    fn canonicalise_needs_a_key_only_for_a_bare_number() {
        let keyless = scheme("{key}-{number:04d}", None);
        assert_eq!(
            canonical(&keyless, "42"),
            Err(CanonicaliseError::MissingKey)
        );
        assert_eq!(canonical(&keyless, "PROJ-42").as_deref(), Ok("PROJ-0042"));
    }

    #[test]
    fn one_canonicaliser_renders_a_full_id_through_the_pattern() {
        let scheme = keyed("PP");
        assert_eq!(canonical(&scheme, "PROJ-42").as_deref(), Ok("PROJ-0042"));
        assert_eq!(canonical(&scheme, "'0042'").as_deref(), Ok("PP-0042"));
        assert_eq!(canonical(&tracker(), "PROJ-42").as_deref(), Ok("PROJ-42"));
        let versioned = super::tests::scheme("v{number:03d}", None);
        assert_eq!(canonical(&versioned, "1").as_deref(), Ok("v001"));
        assert_eq!(canonical(&versioned, "v1").as_deref(), Ok("v001"));
    }

    #[test]
    fn identity_key_pads_and_folds_per_scheme() {
        let numeric = WorkItemIdScheme::numeric();
        assert_eq!(identity(&numeric, "40").as_deref(), Some("0040"));
        assert_eq!(identity(&numeric, "00040").as_deref(), Some("0040"));
        assert_eq!(identity(&numeric, "12345").as_deref(), Some("12345"));
        assert_eq!(identity(&numeric, "ENG-0042").as_deref(), Some("ENG-0042"));
        assert_eq!(identity(&numeric, "PR1-7").as_deref(), Some("PR1-0007"));
        assert_eq!(identity(&numeric, ""), None);
        assert_eq!(identity(&numeric, "AB-12-34"), None);
        assert_eq!(identity(&numeric, "1A-2"), None);

        for scheme in [project("PROJ"), keyed("PROJ")] {
            assert_eq!(identity(&scheme, "40").as_deref(), Some("PROJ-0040"));
            assert_eq!(
                identity(&scheme, "OTHER-9").as_deref(),
                Some("OTHER-0009")
            );
        }
    }

    #[test]
    fn one_identity_key_for_every_spelling_of_an_item() {
        let scheme = keyed("PP");
        for spelling in ["PP-42", "pp-0042", "PP-0042"] {
            assert_eq!(
                identity(&scheme, spelling).as_deref(),
                Some("PP-0042"),
                "{spelling}"
            );
        }
        assert_eq!(identity(&scheme, "xy-7").as_deref(), Some("XY-0007"));
        assert_eq!(identity(&tracker(), "TBD"), None);
        assert_eq!(
            identity(&tracker(), "PROJ-123").as_deref(),
            Some("PROJ-123")
        );
        assert_eq!(
            identity(&WorkItemIdScheme::numeric(), "ENG-42").as_deref(),
            Some("ENG-0042")
        );
    }

    #[test]
    fn a_bare_legacy_id_joins_under_the_configured_key() {
        assert_eq!(
            identity(&keyed("PROJ"), "42").as_deref(),
            Some("PROJ-0042")
        );
        assert_eq!(
            identity(&scheme("{key}_{number:04d}", Some("PROJ")), "42")
                .as_deref(),
            Some("PROJ_0042")
        );
        assert_eq!(
            identity(&WorkItemIdScheme::numeric(), "42").as_deref(),
            Some("0042")
        );
    }

    #[test]
    fn a_filename_id_is_its_identity_key() {
        let cases = [
            (keyed("PP"), "pp-0500-x.md", Some("PP-0500")),
            (
                scheme("{key}_{number:04d}", Some("PP")),
                "PP_0042-x.md",
                Some("PP_0042"),
            ),
            (scheme("v{number:03d}", None), "v001-x.md", Some("v001")),
            (WorkItemIdScheme::numeric(), "42-x.md", Some("0042")),
            (WorkItemIdScheme::numeric(), "0042-foo.md", Some("0042")),
            (WorkItemIdScheme::numeric(), "0042-bar-baz.md", Some("0042")),
            (WorkItemIdScheme::numeric(), "malformed.md", None),
            (WorkItemIdScheme::numeric(), "ADR-0001-foo.md", None),
            (project("PROJ"), "PROJ-0042-foo.md", Some("PROJ-0042")),
            (project("PROJ"), "PROJ-1-short.md", Some("PROJ-0001")),
            (project("PROJ"), "PROJ-0042.md", None),
            (keyed("PROJ"), "42-x.md", Some("PROJ-0042")),
            (project("PROJ"), "0042-legacy.md", Some("PROJ-0042")),
            (tracker(), "proj-123-x.md", Some("PROJ-123")),
        ];
        for (scheme, filename, expected) in cases {
            assert_eq!(
                scheme.id_in_filename(filename).as_deref(),
                expected,
                "{filename}"
            );
        }
    }

    #[test]
    fn a_held_value_holds_an_id_when_key_and_number_agree() {
        let scheme = keyed("PP");
        let id = |text: &str| IdParts::parse(scheme.pattern(), text).unwrap();
        assert!(scheme.holds("pp-42", &id("PP-0042")));
        assert!(!scheme.holds("PP-0042", &id("XY-0042")));
        assert!(scheme.holds("0042", &id("PP-0042")));
        assert!(!scheme.holds("0042", &id("XY-0042")));
        assert!(!scheme.holds("draft-k7mq3x", &id("PP-0042")));
    }

    #[test]
    fn a_tracker_key_is_held_only_as_written_ignoring_case() {
        let scheme = tracker();
        let id = scheme.read_held("PP-760").unwrap();
        assert!(scheme.holds("pp-760", &id));
        assert!(!scheme.holds("PP-0760", &id));
        assert_eq!(scheme.read_held("draft-k7mq3x"), None);
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
}
