//! The identity scheme a workspace configures for its work items: its
//! validated pattern and key, and every reading of an ID under them.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use crate::draft_id::DraftId;
use crate::id_pattern::is_id_character;
use crate::id_pattern::HeldPrefix;
use crate::id_pattern::IdKey;
use crate::id_pattern::IdParts;
use crate::id_pattern::IdPattern;
use crate::id_pattern::MissingKey;
pub use crate::id_pattern::TRACKER_TOKEN;
use tracker::TrackerKeys;

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

/// Whether `token` is an ID only a tracker can own: a draft ID or one of
/// `keys`.
#[must_use]
pub fn is_tracker_owned_token(token: &str, keys: TrackerKeys) -> bool {
    DraftId::parse(token).is_some() || keys.is_key(token)
}

mod pattern_ids;
mod tracker_ids;

use crate::id_scheme::pattern_ids::PatternIds;
use crate::id_scheme::tracker_ids::TrackerIds;

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// The digits a legacy `NNNN-slug` filename starts with.
fn legacy_prefix(filename: &str) -> Option<&str> {
    let (digits, _) = filename.split_once('-')?;
    is_digits(digits).then_some(digits)
}

const LEGACY_WIDTH: usize = 4;

/// The identity key a filename starts with, and where its ID ends.
struct FilenameIdentity {
    identity_key: String,
    id_end: usize,
}

impl FilenameIdentity {
    /// The byte offset just past the `-` after the ID.
    const fn slug_start(&self) -> usize {
        self.id_end + 1
    }
}

/// How one owner of a workspace's IDs reads them, grouped by what each
/// reading is for.
trait IdReadings {
    /// Display: the ID as a page shows it.
    fn displayed_id(&self, trimmed: &str) -> Option<String>;

    /// Join: the key every spelling of one item reads to.
    fn identity_key(&self, reference: &str) -> Option<String>;

    /// Join: the identity key a filename starts with, and where its ID
    /// ends.
    fn filename_identity(&self, filename: &str) -> Option<FilenameIdentity>;

    /// Occupancy: a held `id`, alias or `external_id` read as an ID.
    fn read_held(&self, value: &str) -> Option<IdParts>;

    /// Occupancy: the ID a filename holds.
    fn read_held_prefix(&self, filename: &str) -> Option<HeldPrefix>;

    /// Occupancy: whether `held` names `id`.
    fn holds(&self, held: &str, id: &IdParts) -> bool;

    /// Rendering: `input` in the shape the owner writes.
    fn canonicalise(&self, input: &str) -> Result<String, CanonicaliseError>;

    /// Rendering: the key the owner renders IDs under.
    fn key(&self) -> Option<&IdKey>;

    /// Linkage: whether `token`, standing alone in prose, is an ID.
    fn is_bare_id(&self, token: &str) -> bool;

    /// The rules a tracker key is read by.
    fn tracker_keys(&self) -> TrackerKeys;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Owner {
    Tracker(TrackerIds),
    Pattern(PatternIds),
}

/// The identity scheme a workspace configures for its work items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdScheme {
    owner: Owner,
}

impl IdScheme {
    /// The scheme of `pattern`, holding `key` only when the pattern has a
    /// key token, and reading any supported tracker's keys.
    #[must_use]
    pub fn new(pattern: IdPattern, key: Option<IdKey>) -> Self {
        Self::under_tracker(pattern, key, TrackerKeys::AnySupported)
    }

    /// The scheme of `pattern`, holding `key` only when the pattern has a
    /// key token, and reading tracker keys by `tracker_keys`' rules.
    #[must_use]
    pub fn under_tracker(
        pattern: IdPattern,
        key: Option<IdKey>,
        tracker_keys: TrackerKeys,
    ) -> Self {
        let owner = if pattern.is_tracker_owned() {
            Owner::Tracker(TrackerIds::new(pattern, tracker_keys))
        } else {
            Owner::Pattern(PatternIds::new(pattern, key, tracker_keys))
        };
        Self { owner }
    }

    /// The rules a tracker key is read by.
    #[must_use]
    pub fn tracker_keys(&self) -> TrackerKeys {
        self.readings().tracker_keys()
    }

    #[must_use]
    pub fn numeric() -> Self {
        let pattern = IdPattern::parse("{number:04d}")
            .unwrap_or_else(|_| unreachable!("the default pattern parses"));
        Self::new(pattern, None)
    }

    #[must_use]
    pub const fn pattern(&self) -> &IdPattern {
        match &self.owner {
            Owner::Tracker(ids) => ids.pattern(),
            Owner::Pattern(ids) => ids.pattern(),
        }
    }

    #[must_use]
    pub fn key(&self) -> Option<&IdKey> {
        self.readings().key()
    }

    #[must_use]
    pub const fn ownership(&self) -> IdOwnership {
        match self.owner {
            Owner::Tracker(_) => IdOwnership::Tracker,
            Owner::Pattern(_) => IdOwnership::Local,
        }
    }

    fn readings(&self) -> &dyn IdReadings {
        match &self.owner {
            Owner::Tracker(ids) => ids,
            Owner::Pattern(ids) => ids,
        }
    }

    pub(crate) fn is_own_key(&self, key: &str) -> bool {
        self.key()
            .is_some_and(|own| own.as_str().eq_ignore_ascii_case(key))
    }

    /// Whether `token` is an ID in the pattern's exact shape under this
    /// scheme's own key, ignoring the key's letter case.
    #[must_use]
    pub fn is_canonical_id_token(&self, token: &str) -> bool {
        self.pattern().read_exact(token).is_some_and(|parts| {
            parts.key().is_none_or(|key| self.is_own_key(key))
        })
    }

    /// Whether `id` is in the pattern's shape under any key, whatever its
    /// case.
    #[must_use]
    pub(crate) fn is_in_shape(&self, id: &str) -> bool {
        self.pattern().read_in_shape(id).is_some()
    }

    /// A work-item ID as it is shown: as written when the pattern reads it,
    /// and a bare number as the pattern renders it under `work.key`; under
    /// `{tracker}`, a tracker key as written or a draft ID in lower case.
    #[must_use]
    pub fn displayed_id(&self, raw: &str) -> Option<String> {
        self.readings().displayed_id(raw.trim())
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
        self.readings().canonicalise(input)
    }

    /// The key every spelling of one item joins on: the number padded, and
    /// the key in `work.key`'s spelling when it names `work.key`, otherwise
    /// in upper case. A bare number reads under `work.key`; a reference in
    /// no shape the pattern reads, such as one an earlier pattern wrote,
    /// joins nothing until a re-key converges it.
    #[must_use]
    pub fn identity_key(&self, reference: &str) -> Option<String> {
        self.readings().identity_key(reference)
    }

    /// A held `id`, alias or `external_id` read as an ID under this scheme:
    /// through the pattern, or as a bare number; under `{tracker}`, a
    /// tracker key as written.
    #[must_use]
    pub fn read_held(&self, value: &str) -> Option<IdParts> {
        self.readings().read_held(value)
    }

    /// The ID leading `filename`, read as [`Self::read_held`] reads a value;
    /// under `{tracker}`, a bare legacy number too.
    #[must_use]
    pub fn read_held_prefix(&self, filename: &str) -> Option<HeldPrefix> {
        self.readings().read_held_prefix(filename)
    }

    /// Whether `held` names `id`: it reads as an ID with the same key,
    /// ignoring case, and the same number. Under `{tracker}`, a tracker key
    /// holds only the same key and digits as written.
    #[must_use]
    pub fn holds(&self, held: &str, id: &IdParts) -> bool {
        self.readings().holds(held, id)
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
}

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

impl Default for IdScheme {
    fn default() -> Self {
        Self::numeric()
    }
}

impl corpus::IdConvention for IdScheme {
    fn slug_start(&self, stem: &str) -> Option<usize> {
        self.readings()
            .filename_identity(stem)
            .map(|identity| identity.slug_start())
    }

    fn is_id(&self, token: &str) -> bool {
        self.is_canonical_id_token(token)
    }

    fn identity_key(&self, reference: &str) -> Option<String> {
        Self::identity_key(self, reference)
    }

    fn id_in_filename(&self, filename: &str) -> Option<String> {
        self.readings()
            .filename_identity(filename)
            .map(|identity| identity.identity_key)
    }
}

impl corpus::LinkageIds for IdScheme {
    fn id_in_filename(&self, filename: &str) -> Option<String> {
        corpus::IdConvention::id_in_filename(self, filename)
    }

    fn bare_ids_in(&self, line: &str) -> Vec<String> {
        line.split(|c: char| !is_id_character(c))
            .map(|run| run.trim_end_matches(['.', '_', '-']))
            .filter(|run| self.readings().is_bare_id(run))
            .map(str::to_owned)
            .collect()
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
    use super::IdScheme;
    use super::TrackerIntegrationError;
    use super::TRACKER_TOKEN;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdParts;
    use crate::id_pattern::IdPattern;
    use tracker::TrackerKeys;

    fn scheme(pattern: &str, key: Option<&str>) -> IdScheme {
        IdScheme::new(
            IdPattern::parse(pattern).unwrap(),
            key.map(|key| IdKey::parse(key).unwrap()),
        )
    }

    fn keyed(key: &str) -> IdScheme {
        scheme("{key}-{number:04d}", Some(key))
    }

    fn project(key: &str) -> IdScheme {
        scheme("{project}-{number:04d}", Some(key))
    }

    fn tracker() -> IdScheme {
        scheme(TRACKER_TOKEN, None)
    }

    fn tracker_under(keys: TrackerKeys) -> IdScheme {
        IdScheme::under_tracker(
            IdPattern::parse(TRACKER_TOKEN).unwrap(),
            None,
            keys,
        )
    }

    fn identity(scheme: &IdScheme, reference: &str) -> Option<String> {
        scheme.identity_key(reference)
    }

    fn canonical(
        scheme: &IdScheme,
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
    fn a_tracker_scheme_reads_keys_by_its_trackers_rules() {
        let jira = tracker_under(TrackerKeys::Jira);
        let linear = tracker_under(TrackerKeys::Linear);
        assert_eq!(
            jira.displayed_id("MY_PROJ-7").as_deref(),
            Some("MY_PROJ-7")
        );
        assert_eq!(linear.displayed_id("MY_PROJ-7"), None);
        assert!(jira.read_held("MY_PROJ-7").is_some());
        assert!(linear.read_held("MY_PROJ-7").is_none());
        assert_eq!(linear.identity_key("eng-7").as_deref(), Some("ENG-7"));
        assert_eq!(linear.tracker_keys(), TrackerKeys::Linear);
        assert_eq!(tracker().tracker_keys(), TrackerKeys::AnySupported);
    }

    #[test]
    fn a_tracker_scheme_reports_tracker_ownership() {
        assert_eq!(tracker().ownership(), IdOwnership::Tracker);
        assert_eq!(IdScheme::numeric().ownership(), IdOwnership::Local);
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
            tracker.displayed_id("MY_PROJ-7").as_deref(),
            Some("MY_PROJ-7")
        );
        assert_eq!(
            tracker.displayed_id(" ABC2-15 ").as_deref(),
            Some("ABC2-15")
        );
        assert_eq!(
            tracker.displayed_id("Draft-K7MQ3X").as_deref(),
            Some("draft-k7mq3x")
        );
        assert_eq!(tracker.displayed_id("0042"), None);
        assert_eq!(IdScheme::numeric().displayed_id("MY_PROJ-7"), None);
        assert_eq!(IdScheme::numeric().displayed_id("draft-k7mq3x"), None);
    }

    #[test]
    fn a_tracker_owned_token_is_a_draft_id_or_a_tracker_key() {
        let any = TrackerKeys::AnySupported;
        assert!(is_tracker_owned_token("draft-k7mq3x", any));
        assert!(is_tracker_owned_token("PROJ-123", any));
        assert!(!is_tracker_owned_token("0400", any));
        assert!(!is_tracker_owned_token("PP_0400", any));
    }

    #[test]
    fn is_canonical_id_token_under_default_numeric() {
        let scheme = IdScheme::numeric();
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
    fn any_key_tokens_take_every_well_formed_key() {
        let scheme = keyed("PP");
        assert!(scheme.is_in_shape("PP-0400"));
        assert!(scheme.is_in_shape("xy-0400"));
        assert!(scheme.is_in_shape("X1-0400"));
        assert_eq!(scheme.displayed_id("X1-0400").as_deref(), Some("X1-0400"));
    }

    #[test]
    fn an_id_the_pattern_reads_is_displayed_as_written() {
        let numeric = IdScheme::numeric();
        assert_eq!(numeric.displayed_id("0042").as_deref(), Some("0042"));
        assert_eq!(numeric.displayed_id("42").as_deref(), Some("42"));
        assert_eq!(numeric.displayed_id("  0042  ").as_deref(), Some("0042"));
        for off_pattern in ["ENG-0042", "X1-0400", "ENG0042", "PROJ-1.2", ""] {
            assert_eq!(
                numeric.displayed_id(off_pattern),
                None,
                "{off_pattern}"
            );
        }

        let eng = project("ENG");
        assert_eq!(eng.displayed_id("OPS-7").as_deref(), Some("OPS-7"));
        assert_eq!(eng.displayed_id("eng-0042").as_deref(), Some("eng-0042"));

        let underscored = scheme("{key}_{number:04d}", Some("PP"));
        assert_eq!(
            underscored.displayed_id("PP_0042").as_deref(),
            Some("PP_0042")
        );
        assert_eq!(underscored.displayed_id("PP-0042"), None);
        assert_eq!(
            scheme("v{number:03d}", None)
                .displayed_id("v001")
                .as_deref(),
            Some("v001")
        );
    }

    #[test]
    fn a_bare_number_is_displayed_as_the_pattern_renders_it_under_work_key() {
        assert_eq!(
            project("ENG").displayed_id("42").as_deref(),
            Some("ENG-0042")
        );
        assert_eq!(
            scheme("{key}_{number:04d}", Some("PP"))
                .displayed_id("7")
                .as_deref(),
            Some("PP_0007")
        );
        assert_eq!(scheme("{key}-{number:04d}", None).displayed_id("7"), None);
    }

    #[test]
    fn canonicalise_pads_and_prefixes_per_scheme() {
        let numeric = IdScheme::numeric();
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
        assert_eq!(
            canonical(&numeric, "99999999999999999999"),
            Err(CanonicaliseError::NoMatch)
        );

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
        let numeric = IdScheme::numeric();
        assert_eq!(identity(&numeric, "40").as_deref(), Some("0040"));
        assert_eq!(identity(&numeric, "00040").as_deref(), Some("0040"));
        assert_eq!(identity(&numeric, "12345").as_deref(), Some("12345"));
        assert_eq!(identity(&numeric, "ENG-0042"), None);
        assert_eq!(identity(&numeric, "PR1-7"), None);
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
        assert_eq!(identity(&IdScheme::numeric(), "ENG-42"), None);
    }

    #[test]
    fn a_reference_off_the_pattern_has_no_identity_key() {
        let underscored = scheme("{key}_{number:04d}", Some("PP"));
        assert_eq!(identity(&underscored, "PP-42"), None);
        assert_eq!(identity(&underscored, "PP-0042"), None);
        assert_eq!(identity(&underscored, "pp_42").as_deref(), Some("PP_0042"));
    }

    #[test]
    fn a_bare_number_reads_alike_in_every_reading_under_a_literal_pattern() {
        let versioned = scheme("v{number:03d}", None);
        let held = versioned.read_held("1").map(|parts| parts.number());
        assert_eq!(held, Some(1));
        assert_eq!(identity(&versioned, "1").as_deref(), Some("v001"));
        assert_eq!(canonical(&versioned, "1").as_deref(), Ok("v001"));
        assert_eq!(
            versioned
                .read_held_prefix("0001-x.md")
                .map(|held| held.parts().number()),
            Some(1)
        );
        let v001 = versioned.read_held("v001").unwrap();
        assert!(versioned.holds("1", &v001));
    }

    #[test]
    fn linkage_reads_ids_through_the_configured_scheme() {
        use corpus::LinkageIds;

        let underscored = scheme("{key}_{number:04d}", Some("PP"));
        assert_eq!(
            LinkageIds::id_in_filename(&underscored, "pp_42-x.md").as_deref(),
            Some("PP_0042")
        );
        assert_eq!(
            LinkageIds::id_in_filename(&underscored, "0042-x.md").as_deref(),
            Some("PP_0042")
        );
        assert_eq!(
            LinkageIds::id_in_filename(&underscored, "PP-0042-x.md"),
            None
        );
        assert_eq!(
            underscored.bare_ids_in("- Blocks: PP_0042, XY_0042 and PP_0043."),
            ["PP_0042", "XY_0042", "PP_0043"]
        );
        assert!(underscored
            .bare_ids_in("- Blocks: 0042 and PP-0042")
            .is_empty());

        let tracker = tracker();
        assert_eq!(
            tracker.bare_ids_in("- Blocked by: ENG-7, draft-k7mq3x, 0042"),
            ["ENG-7", "draft-k7mq3x"]
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
            identity(&IdScheme::numeric(), "42").as_deref(),
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
            (IdScheme::numeric(), "42-x.md", Some("0042")),
            (IdScheme::numeric(), "0042-foo.md", Some("0042")),
            (IdScheme::numeric(), "0042-bar-baz.md", Some("0042")),
            (IdScheme::numeric(), "malformed.md", None),
            (IdScheme::numeric(), "ADR-0001-foo.md", None),
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
        assert!(IdScheme::is_legacy_id("42"));
        assert!(IdScheme::is_legacy_id("0042"));
        assert!(IdScheme::is_legacy_id("9"));
        assert!(!IdScheme::is_legacy_id("0000"));
        assert!(!IdScheme::is_legacy_id("12345"));
        assert!(!IdScheme::is_legacy_id(""));
        assert!(!IdScheme::is_legacy_id("12a4"));
    }

    #[test]
    fn pad_legacy_number_zero_pads_to_four_digits() {
        assert_eq!(IdScheme::pad_legacy_number("42").as_deref(), Some("0042"));
        assert_eq!(
            IdScheme::pad_legacy_number("12345").as_deref(),
            Some("12345")
        );
        assert_eq!(IdScheme::pad_legacy_number(""), None);
        assert_eq!(IdScheme::pad_legacy_number("12a"), None);
    }

    #[test]
    fn a_numeric_scheme_answers_as_an_id_convention() {
        let scheme = IdScheme::numeric();
        let filenames: &[(&str, Option<usize>, Option<&str>)] = &[
            ("0042-foo", Some(5), Some("0042")),
            ("42-foo", Some(3), Some("0042")),
            ("PP-0042-foo", None, None),
            ("pp-0042-foo", None, None),
            ("XY-0042-foo", None, None),
            ("MY_PROJ-7-a-title", None, None),
            ("0296-2026-review", Some(5), Some("0296")),
            ("draft-k7mq3x-title", None, None),
            ("PROJ-123-x", None, None),
            ("PRO_J-123-x", None, None),
            ("malformed", None, None),
        ];
        for (stem, slug_start, id) in filenames {
            assert_eq!(scheme.slug_start(stem), *slug_start, "{stem}");
            assert_eq!(
                scheme.id_in_filename(&format!("{stem}.md")).as_deref(),
                *id,
                "{stem}"
            );
        }
        let references: &[(&str, bool, Option<&str>)] = &[
            ("0042", true, Some("0042")),
            ("42", false, Some("0042")),
            ("PP-0042", false, None),
            ("pp-42", false, None),
            ("XY-7", false, None),
            ("ENG-0042", false, None),
            ("PROJ-123", false, None),
            ("TBD", false, None),
            ("", false, None),
        ];
        for (reference, is_id, identity) in references {
            assert_eq!(scheme.is_id(reference), *is_id, "{reference}");
            assert_eq!(
                IdConvention::identity_key(&scheme, reference).as_deref(),
                *identity,
                "{reference}"
            );
        }
    }

    #[test]
    fn a_keyed_scheme_answers_as_an_id_convention() {
        let scheme = keyed("PP");
        let filenames: &[(&str, Option<usize>, Option<&str>)] = &[
            ("0042-foo", Some(5), Some("PP-0042")),
            ("42-foo", Some(3), Some("PP-0042")),
            ("PP-0042-foo", Some(8), Some("PP-0042")),
            ("pp-0042-foo", Some(8), Some("PP-0042")),
            ("XY-0042-foo", Some(8), Some("XY-0042")),
            ("MY_PROJ-7-a-title", None, None),
            ("0296-2026-review", Some(5), Some("PP-0296")),
            ("draft-k7mq3x-title", None, None),
            ("PROJ-123-x", Some(9), Some("PROJ-0123")),
            ("PRO_J-123-x", None, None),
            ("malformed", None, None),
        ];
        for (stem, slug_start, id) in filenames {
            assert_eq!(scheme.slug_start(stem), *slug_start, "{stem}");
            assert_eq!(
                scheme.id_in_filename(&format!("{stem}.md")).as_deref(),
                *id,
                "{stem}"
            );
        }
        let references: &[(&str, bool, Option<&str>)] = &[
            ("0042", false, Some("PP-0042")),
            ("42", false, Some("PP-0042")),
            ("PP-0042", true, Some("PP-0042")),
            ("pp-42", false, Some("PP-0042")),
            ("XY-7", false, Some("XY-0007")),
            ("ENG-0042", false, Some("ENG-0042")),
            ("PROJ-123", false, Some("PROJ-0123")),
            ("TBD", false, None),
            ("", false, None),
        ];
        for (reference, is_id, identity) in references {
            assert_eq!(scheme.is_id(reference), *is_id, "{reference}");
            assert_eq!(
                IdConvention::identity_key(&scheme, reference).as_deref(),
                *identity,
                "{reference}"
            );
        }
    }

    #[test]
    fn a_tracker_scheme_answers_as_an_id_convention() {
        let scheme = tracker();
        let filenames: &[(&str, Option<usize>, Option<&str>)] = &[
            ("0042-foo", Some(5), Some("0042")),
            ("42-foo", Some(3), Some("0042")),
            ("PP-0042-foo", Some(8), Some("PP-0042")),
            ("pp-0042-foo", Some(8), Some("PP-0042")),
            ("XY-0042-foo", Some(8), Some("XY-0042")),
            ("MY_PROJ-7-a-title", Some(10), Some("MY_PROJ-7")),
            ("0296-2026-review", Some(5), Some("0296")),
            ("draft-k7mq3x-title", Some(13), Some("draft-k7mq3x")),
            ("PROJ-123-x", Some(9), Some("PROJ-123")),
            ("PRO_J-123-x", Some(10), Some("PRO_J-123")),
            ("malformed", None, None),
        ];
        for (stem, slug_start, id) in filenames {
            assert_eq!(scheme.slug_start(stem), *slug_start, "{stem}");
            assert_eq!(
                scheme.id_in_filename(&format!("{stem}.md")).as_deref(),
                *id,
                "{stem}"
            );
        }
        let references: &[(&str, bool, Option<&str>)] = &[
            ("0042", false, Some("0042")),
            ("42", false, Some("0042")),
            ("PP-0042", false, Some("PP-0042")),
            ("pp-42", false, Some("PP-42")),
            ("XY-7", false, Some("XY-7")),
            ("ENG-0042", false, Some("ENG-0042")),
            ("PROJ-123", false, Some("PROJ-123")),
            ("TBD", false, None),
            ("", false, None),
        ];
        for (reference, is_id, identity) in references {
            assert_eq!(scheme.is_id(reference), *is_id, "{reference}");
            assert_eq!(
                IdConvention::identity_key(&scheme, reference).as_deref(),
                *identity,
                "{reference}"
            );
        }
    }
}
