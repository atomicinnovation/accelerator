//! The `<tracker>.pull` discovery-scope config block: its typed model and the
//! parser that reads a resolved config mapping into it.
//!
//! The model is entity- and tracker-neutral: each tracker's own noun
//! (`additional_projects` for Jira, `additional_teams` for Linear) folds into
//! `additional_entities`, and `all_projects` / `all_teams` into `all_entities`.
//! Parsing stays total over shape — it accepts any structurally-valid mapping
//! and carries unresolved concerns (filter-key acceptance, ceiling-token
//! validity, `all`/`additional` exclusivity) into typed fields for a separate
//! validation step to reject — so one parser serves both `configure` and the
//! sync path.

use config::render_value;
use config::ConfigAccess;
use config::Level;
use config::Value;
use tracker::Ceiling;
use tracker::FilterSchema;
use tracker::DEFAULT_MAX_ITEMS;
use tracker::DEFAULT_MAX_PAGES;

/// A configured discovery-scope block, normalised to the port's entity-neutral
/// vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PullConfig {
    /// Entities to broaden discovery onto beyond the keyed base entity.
    pub additional_entities: Vec<String>,
    /// Whether to discover across every entity the credential can see.
    pub all_entities: bool,
    /// Field filters, each key AND'd against the others and its values OR'd.
    pub filters: Vec<(String, Vec<String>)>,
    /// The discovered-write ceiling, held as a raw token for later
    /// interpretation.
    pub max_items: Option<CeilingToken>,
    /// The transport page caps, held as raw tokens for later interpretation.
    pub max_pages: PageCaps,
    /// The tracker-specific scope nouns (`additional_projects`, `all_teams`, …)
    /// as written, retained after normalisation so per-tracker validation can
    /// reject a noun belonging to the other tracker.
    pub scope_nouns: Vec<String>,
    /// Top-level keys the parser did not recognise, retained rather than
    /// dropped so validation can reject them.
    pub unknown_keys: Vec<String>,
}

/// A ceiling value as written in config, held verbatim.
///
/// Parsing keeps the token raw; classifying it (a non-negative integer, the
/// `unlimited` sentinel, or a reject) is a separate concern so parse and
/// validate stay independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CeilingToken(pub String);

/// The `max_pages` caps: a general default plus optional per-operation
/// overrides. A scalar `max_pages` sets `default`; a block sets any of the
/// three.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PageCaps {
    pub default: Option<CeilingToken>,
    pub discovery: Option<CeilingToken>,
    pub keyed_read: Option<CeilingToken>,
}

/// Why a `pull` block is invalid.
///
/// The first two are structural shape faults raised by [`parse`]; the rest are
/// acceptance faults raised by [`validate`]. Each carries the data its message
/// names; [`PullConfigError::detail`] renders the full operator message,
/// naming the config file the effective block resolved from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullConfigError {
    /// The block itself is not a mapping — a scalar or sequence where a block
    /// was expected (e.g. a typo'd `jira.pull: "oops"`). Never silently an
    /// empty config, so a malformed personal block cannot shadow a valid team
    /// block under whole-block replacement.
    NotAMapping,
    /// A sub-key that must carry a mapping (`filters`) does not.
    SubBlockNotAMapping { key: String },
    /// A filter field key outside the tracker's accepted set.
    UnsupportedFilterKey { key: String, accepted: Vec<String> },
    /// A reserved grouping key (`all` / `any`) used as a filter field.
    NestedFiltersUnsupported { key: String },
    /// A ceiling token that is neither the right kind of integer nor
    /// `unlimited`. `allow_zero` distinguishes `max_items` (0 refuses all) from
    /// `max_pages` (a positive cap only).
    BadCeiling {
        key: String,
        value: String,
        allow_zero: bool,
    },
    /// `all_*` set together with a non-empty `additional_*`.
    MutuallyExclusiveScope,
    /// A top-level key the tracker does not accept — an unknown key, or the
    /// other tracker's scope noun (then `hint` names the intended key).
    UnrecognisedKey {
        key: String,
        accepted: Vec<String>,
        hint: Option<String>,
    },
}

impl PullConfigError {
    /// The full operator message, naming the offending value, the accepted set,
    /// and the config file the effective block resolved from.
    #[must_use]
    pub fn detail(&self, level: Level) -> String {
        let file = level.filename();
        let fix = format!("Fix the `pull` block in {file}.");
        match self {
            Self::NotAMapping => format!(
                "the `pull` value must be a block of settings, not a scalar \
                 or list. {fix}"
            ),
            Self::SubBlockNotAMapping { key } => format!(
                "the pull `{key}` value must be a block of settings. {fix}"
            ),
            Self::UnsupportedFilterKey { key, accepted } => format!(
                "the pull filter key `{key}` is not supported (accepted: {}). \
                 {fix}",
                accepted.join(", ")
            ),
            Self::NestedFiltersUnsupported { key } => format!(
                "the pull filter key `{key}` is reserved: nested filters not \
                 yet supported. {fix}"
            ),
            Self::BadCeiling {
                key,
                value,
                allow_zero: true,
            } => format!(
                "pull `{key}` must be a non-negative integer (0 refuses all) \
                 or `unlimited` (got `{value}`). {fix}"
            ),
            Self::BadCeiling {
                key,
                value,
                allow_zero: false,
            } => format!(
                "pull `{key}` must be a positive integer or `unlimited` \
                 (got `{value}`). {fix}"
            ),
            Self::MutuallyExclusiveScope => format!(
                "a pull block sets both `all_*` and `additional_*` — remove \
                 one of `all_*`/`additional_*`. {fix}"
            ),
            Self::UnrecognisedKey {
                key,
                accepted,
                hint,
            } => format!(
                "the pull block key `{key}` is not recognised (accepted: {}).{} \
                 {fix}",
                accepted.join(", "),
                hint.as_deref().unwrap_or("")
            ),
        }
    }
}

const ADDITIONAL_KEYS: &[&str] = &["additional_projects", "additional_teams"];
const ALL_KEYS: &[&str] = &["all_projects", "all_teams"];

/// Reads a resolved config block into a [`PullConfig`].
///
/// # Errors
///
/// [`PullConfigError::NotAMapping`] when the block is not a mapping, and
/// [`PullConfigError::SubBlockNotAMapping`] when `filters` is present but not a
/// mapping.
pub fn parse(block: &Value) -> Result<PullConfig, PullConfigError> {
    let Value::Mapping(entries) = block else {
        return Err(PullConfigError::NotAMapping);
    };
    let mut config = PullConfig::default();
    for (key, value) in entries {
        match key.as_str() {
            noun if ADDITIONAL_KEYS.contains(&noun) => {
                config.scope_nouns.push(noun.to_owned());
                config
                    .additional_entities
                    .extend(value.as_string_sequence());
            }
            noun if ALL_KEYS.contains(&noun) => {
                config.scope_nouns.push(noun.to_owned());
                config.all_entities |= is_truthy(value);
            }
            "filters" => parse_filters(value, &mut config)?,
            "max_items" => {
                config.max_items = Some(CeilingToken(render_value(value)));
            }
            "max_pages" => parse_page_caps(value, &mut config),
            other => config.unknown_keys.push(other.to_owned()),
        }
    }
    Ok(config)
}

fn parse_filters(
    value: &Value,
    config: &mut PullConfig,
) -> Result<(), PullConfigError> {
    let Value::Mapping(fields) = value else {
        return Err(PullConfigError::SubBlockNotAMapping {
            key: "filters".to_owned(),
        });
    };
    for (field, field_value) in fields {
        config
            .filters
            .push((field.clone(), field_value.as_string_sequence()));
    }
    Ok(())
}

fn parse_page_caps(value: &Value, config: &mut PullConfig) {
    match value {
        Value::Mapping(caps) => {
            for (cap_key, cap_value) in caps {
                let token = CeilingToken(render_value(cap_value));
                match cap_key.as_str() {
                    "default" => config.max_pages.default = Some(token),
                    "discovery" => config.max_pages.discovery = Some(token),
                    "keyed_read" => config.max_pages.keyed_read = Some(token),
                    other => {
                        config.unknown_keys.push(format!("max_pages.{other}"));
                    }
                }
            }
        }
        scalar_or_sequence => {
            config.max_pages.default =
                Some(CeilingToken(render_value(scalar_or_sequence)));
        }
    }
}

fn is_truthy(value: &Value) -> bool {
    render_value(value) == "true"
}

/// A tracker whose `pull` block can be validated: its scope-noun vocabulary and
/// its accepted filter keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tracker {
    Jira,
    Linear,
}

/// Which scope a noun broadens: the additional-entity list or the whole
/// workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NounKind {
    Additional,
    All,
}

const NOUN_OWNERS: &[(&str, Tracker, NounKind)] = &[
    ("additional_projects", Tracker::Jira, NounKind::Additional),
    ("all_projects", Tracker::Jira, NounKind::All),
    ("additional_teams", Tracker::Linear, NounKind::Additional),
    ("all_teams", Tracker::Linear, NounKind::All),
];

const JIRA_FILTERS: FilterSchema = FilterSchema {
    accepted: &["label", "state", "assignee"],
};
const LINEAR_FILTERS: FilterSchema = FilterSchema {
    accepted: &["label", "state", "assignee", "project"],
};

impl Tracker {
    /// The tracker for a `work.integration` value, or `None` for one with no
    /// pull-scope surface.
    #[must_use]
    pub fn from_integration(integration: &str) -> Option<Self> {
        match integration {
            "jira" => Some(Self::Jira),
            "linear" => Some(Self::Linear),
            _ => None,
        }
    }

    const fn filter_schema(self) -> FilterSchema {
        match self {
            Self::Jira => JIRA_FILTERS,
            Self::Linear => LINEAR_FILTERS,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Jira => "Jira",
            Self::Linear => "Linear",
        }
    }

    const fn noun(self, kind: NounKind) -> &'static str {
        match (self, kind) {
            (Self::Jira, NounKind::Additional) => "additional_projects",
            (Self::Jira, NounKind::All) => "all_projects",
            (Self::Linear, NounKind::Additional) => "additional_teams",
            (Self::Linear, NounKind::All) => "all_teams",
        }
    }

    fn accepts_noun(self, key: &str) -> bool {
        key == self.noun(NounKind::Additional)
            || key == self.noun(NounKind::All)
    }

    fn accepted_top_level_keys(self) -> Vec<String> {
        vec![
            self.noun(NounKind::Additional).to_owned(),
            self.noun(NounKind::All).to_owned(),
            "filters".to_owned(),
            "max_items".to_owned(),
            "max_pages".to_owned(),
        ]
    }
}

/// The "did you mean" hint for a scope noun that belongs to the other tracker.
fn wrong_noun_hint(tracker: Tracker, key: &str) -> Option<String> {
    NOUN_OWNERS.iter().find(|(noun, ..)| *noun == key).map(
        |(_, owner, kind)| {
            format!(
                " `{key}` is a {} key; this integration is {} — did you mean \
                 `{}`?",
                owner.label(),
                tracker.label(),
                tracker.noun(*kind)
            )
        },
    )
}

/// Validates a parsed `pull` block against a tracker's vocabulary, structural
/// only — remote existence of named entities is out of scope.
///
/// # Errors
///
/// A [`PullConfigError`] for a wrong-tracker or unknown top-level key, `all_*`
/// with `additional_*`, an unsupported or reserved filter key, or a malformed
/// ceiling token.
pub fn validate(
    config: &PullConfig,
    tracker: Tracker,
) -> Result<(), PullConfigError> {
    for noun in &config.scope_nouns {
        if !tracker.accepts_noun(noun) {
            return Err(PullConfigError::UnrecognisedKey {
                key: noun.clone(),
                accepted: tracker.accepted_top_level_keys(),
                hint: wrong_noun_hint(tracker, noun),
            });
        }
    }
    if let Some(key) = config.unknown_keys.first() {
        return Err(PullConfigError::UnrecognisedKey {
            key: key.clone(),
            accepted: tracker.accepted_top_level_keys(),
            hint: None,
        });
    }
    if config.all_entities && !config.additional_entities.is_empty() {
        return Err(PullConfigError::MutuallyExclusiveScope);
    }
    let accepted = tracker.filter_schema().accepted;
    for (field, _) in &config.filters {
        if field == "all" || field == "any" {
            return Err(PullConfigError::NestedFiltersUnsupported {
                key: field.clone(),
            });
        }
        if !accepted.contains(&field.as_str()) {
            return Err(PullConfigError::UnsupportedFilterKey {
                key: field.clone(),
                accepted: accepted.iter().map(|k| (*k).to_owned()).collect(),
            });
        }
    }
    if let Some(token) = &config.max_items {
        if !ceiling_ok(&token.0, true) {
            return Err(bad_ceiling("max_items", token, true));
        }
    }
    for (key, token) in page_cap_tokens(&config.max_pages) {
        if !ceiling_ok(&token.0, false) {
            return Err(bad_ceiling(key, token, false));
        }
    }
    Ok(())
}

fn bad_ceiling(
    key: &str,
    token: &CeilingToken,
    allow_zero: bool,
) -> PullConfigError {
    PullConfigError::BadCeiling {
        key: key.to_owned(),
        value: token.0.clone(),
        allow_zero,
    }
}

fn page_cap_tokens(caps: &PageCaps) -> Vec<(&'static str, &CeilingToken)> {
    [
        ("max_pages", &caps.default),
        ("max_pages.discovery", &caps.discovery),
        ("max_pages.keyed_read", &caps.keyed_read),
    ]
    .into_iter()
    .filter_map(|(key, cap)| cap.as_ref().map(|token| (key, token)))
    .collect()
}

/// Whether a ceiling token is a valid bound, deferring to
/// [`crate::ceiling::from_token`] so validation and interpretation can never
/// disagree.
fn ceiling_ok(token: &str, allow_zero: bool) -> bool {
    crate::ceiling::from_token(token, allow_zero).is_some()
}

/// The bounds a pull runs under, resolved from a `<tracker>.pull` block with
/// the built-in defaults applied to any unset key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ceilings {
    /// The pull-direction write bound: tracked-item updates plus newly
    /// discovered creates, counted after dedup and local subtraction.
    pub max_items: Ceiling,
    /// The page cap for unkeyed discovery searches.
    pub discovery_pages: Ceiling,
    /// The page cap for keyed reconcile reads.
    pub keyed_read_pages: Ceiling,
}

impl PullConfig {
    /// Resolves the ceilings, applying the built-in defaults for any unset key:
    /// `max_items` defaults to [`DEFAULT_MAX_ITEMS`]; each page cap resolves as
    /// its per-operation override, then the general `max_pages`, then
    /// [`DEFAULT_MAX_PAGES`].
    ///
    /// # Errors
    ///
    /// [`PullConfigError::BadCeiling`] for a malformed token. [`validate`]
    /// rejects the same tokens at configure time, so this only fires on a
    /// hand-edited config that reaches interpretation unvalidated.
    pub fn ceilings(&self) -> Result<Ceilings, PullConfigError> {
        let max_items = match &self.max_items {
            Some(token) => interpret(token, "max_items", true)?,
            None => DEFAULT_MAX_ITEMS,
        };
        Ok(Ceilings {
            max_items,
            discovery_pages: self.page_cap(
                self.max_pages.discovery.as_ref(),
                "max_pages.discovery",
            )?,
            keyed_read_pages: self.page_cap(
                self.max_pages.keyed_read.as_ref(),
                "max_pages.keyed_read",
            )?,
        })
    }

    /// Resolves one page cap: its per-operation override, else the general
    /// `max_pages` default, else the built-in default.
    fn page_cap(
        &self,
        override_token: Option<&CeilingToken>,
        override_key: &str,
    ) -> Result<Ceiling, PullConfigError> {
        if let Some(token) = override_token {
            return interpret(token, override_key, false);
        }
        if let Some(token) = &self.max_pages.default {
            return interpret(token, "max_pages", false);
        }
        Ok(DEFAULT_MAX_PAGES)
    }
}

fn interpret(
    token: &CeilingToken,
    key: &str,
    allow_zero: bool,
) -> Result<Ceiling, PullConfigError> {
    crate::ceiling::from_token(&token.0, allow_zero)
        .ok_or_else(|| bad_ceiling(key, token, allow_zero))
}

/// Reads and parses the active tracker's `<tracker>.pull` block from resolved
/// config, paired with the config level it resolved from.
///
/// Returns `None` when the integration has no pull surface, no block is
/// configured, or the block is empty — each meaning base-only discovery under
/// the built-in ceilings. Reads the already-composed config representation, so
/// it performs no filesystem or subprocess access.
///
/// # Errors
///
/// A config-access failure, or a structural parse fault ([`PullConfigError`])
/// rendered against the resolving config file.
pub fn read(
    config: &dyn ConfigAccess,
    integration: &str,
) -> Result<Option<(PullConfig, Level)>, String> {
    if Tracker::from_integration(integration).is_none() {
        return Ok(None);
    }
    let Some((value, level)) =
        crate::block::read_block(config, integration, "pull")?
    else {
        return Ok(None);
    };
    let parsed = parse(&value).map_err(|error| error.detail(level))?;
    Ok(Some((parsed, level)))
}

/// Resolves the pull ceilings for the active tracker from config, applying the
/// built-in defaults when no `<tracker>.pull` block is configured.
///
/// The single entry point every client-construction site and the sync path
/// share, so the transport caps and the reconcile bound resolve from one place.
///
/// # Errors
///
/// A config-access failure or an invalid block, rendered against the resolving
/// config file.
pub fn resolve_ceilings(
    config: &dyn ConfigAccess,
    integration: &str,
) -> Result<Ceilings, String> {
    match read(config, integration)? {
        Some((pull, level)) => {
            pull.ceilings().map_err(|error| error.detail(level))
        }
        None => Ok(Ceilings {
            max_items: DEFAULT_MAX_ITEMS,
            discovery_pages: DEFAULT_MAX_PAGES,
            keyed_read_pages: DEFAULT_MAX_PAGES,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse, validate, CeilingToken, Ceilings, PageCaps, PullConfig,
        PullConfigError, Tracker,
    };
    use config::{Scalar, Value};
    use tracker::Ceiling;

    fn scalar(text: &str) -> Value {
        Value::Scalar(Scalar::String(text.to_owned()))
    }

    fn seq(items: &[&str]) -> Value {
        Value::Sequence(
            items
                .iter()
                .map(|item| Scalar::String((*item).to_owned()))
                .collect(),
        )
    }

    fn block(entries: Vec<(&str, Value)>) -> Value {
        Value::Mapping(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    fn owned(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    fn token(text: &str) -> CeilingToken {
        CeilingToken(text.to_owned())
    }

    #[test]
    fn a_jira_block_maps_additional_projects_to_additional_entities() {
        assert_eq!(
            parse(&block(vec![("additional_projects", seq(&["PP", "XX"]))])),
            Ok(PullConfig {
                additional_entities: owned(&["PP", "XX"]),
                scope_nouns: owned(&["additional_projects"]),
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn a_linear_block_maps_additional_teams_to_additional_entities() {
        assert_eq!(
            parse(&block(vec![("additional_teams", seq(&["core", "ops"]))])),
            Ok(PullConfig {
                additional_entities: owned(&["core", "ops"]),
                scope_nouns: owned(&["additional_teams"]),
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn an_absent_block_is_the_empty_config() {
        assert_eq!(
            parse(&Value::Mapping(Vec::new())),
            Ok(PullConfig::default())
        );
    }

    #[test]
    fn all_projects_and_all_teams_both_set_all_entities() {
        for noun in ["all_projects", "all_teams"] {
            assert_eq!(
                parse(&block(vec![(noun, Value::Scalar(Scalar::Bool(true)))])),
                Ok(PullConfig {
                    all_entities: true,
                    scope_nouns: owned(&[noun]),
                    ..PullConfig::default()
                })
            );
        }
    }

    #[test]
    fn all_entities_is_false_when_the_flag_is_false() {
        assert_eq!(
            parse(&block(vec![(
                "all_teams",
                Value::Scalar(Scalar::Bool(false))
            )])),
            Ok(PullConfig {
                all_entities: false,
                scope_nouns: owned(&["all_teams"]),
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn filters_carry_keys_and_their_value_lists() {
        assert_eq!(
            parse(&block(vec![(
                "filters",
                block(vec![
                    ("label", seq(&["a", "b"])),
                    ("state", scalar("open")),
                ]),
            )])),
            Ok(PullConfig {
                filters: vec![
                    ("label".to_owned(), owned(&["a", "b"])),
                    ("state".to_owned(), owned(&["open"])),
                ],
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn max_items_is_kept_as_a_raw_token() {
        assert_eq!(
            parse(&block(vec![("max_items", Value::Scalar(Scalar::Int(3)))])),
            Ok(PullConfig {
                max_items: Some(token("3")),
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn a_scalar_max_pages_sets_the_general_default_cap() {
        assert_eq!(
            parse(&block(vec![("max_pages", Value::Scalar(Scalar::Int(50)))])),
            Ok(PullConfig {
                max_pages: PageCaps {
                    default: Some(token("50")),
                    discovery: None,
                    keyed_read: None,
                },
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn a_max_pages_block_sets_per_operation_overrides() {
        assert_eq!(
            parse(&block(vec![(
                "max_pages",
                block(vec![
                    ("discovery", Value::Scalar(Scalar::Int(20))),
                    ("keyed_read", scalar("unlimited")),
                ]),
            )])),
            Ok(PullConfig {
                max_pages: PageCaps {
                    default: None,
                    discovery: Some(token("20")),
                    keyed_read: Some(token("unlimited")),
                },
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn an_unrecognised_top_level_key_lands_in_unknown_keys() {
        assert_eq!(
            parse(&block(vec![("bogus", scalar("x"))])),
            Ok(PullConfig {
                unknown_keys: owned(&["bogus"]),
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn an_unrecognised_max_pages_sub_key_lands_in_unknown_keys() {
        assert_eq!(
            parse(&block(vec![(
                "max_pages",
                block(vec![("sideways", Value::Scalar(Scalar::Int(5)))]),
            )])),
            Ok(PullConfig {
                unknown_keys: owned(&["max_pages.sideways"]),
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn a_non_mapping_block_is_a_shape_error() {
        assert_eq!(parse(&scalar("oops")), Err(PullConfigError::NotAMapping));
        assert_eq!(parse(&seq(&["a", "b"])), Err(PullConfigError::NotAMapping));
    }

    #[test]
    fn a_non_mapping_filters_is_a_shape_error() {
        assert_eq!(
            parse(&block(vec![("filters", scalar("oops"))])),
            Err(PullConfigError::SubBlockNotAMapping {
                key: "filters".to_owned()
            })
        );
    }

    #[test]
    fn a_personal_only_block_carries_no_team_fields() {
        assert_eq!(
            parse(&block(vec![("additional_teams", seq(&["Y"]))])),
            Ok(PullConfig {
                additional_entities: owned(&["Y"]),
                scope_nouns: owned(&["additional_teams"]),
                ..PullConfig::default()
            })
        );
    }

    fn check(
        entries: Vec<(&str, Value)>,
        tracker: Tracker,
    ) -> Result<(), PullConfigError> {
        validate(&parse(&block(entries))?, tracker)
    }

    #[test]
    fn a_well_formed_jira_block_validates() {
        assert_eq!(
            check(
                vec![
                    ("additional_projects", seq(&["PP"])),
                    (
                        "filters",
                        block(vec![
                            ("label", seq(&["a", "b"])),
                            ("state", scalar("open")),
                            ("assignee", scalar("me")),
                        ]),
                    ),
                    ("max_items", scalar("unlimited")),
                    ("max_pages", Value::Scalar(Scalar::Int(50))),
                ],
                Tracker::Jira,
            ),
            Ok(())
        );
    }

    #[test]
    fn an_unsupported_filter_key_is_rejected() {
        for (tracker, accepted) in [
            (Tracker::Jira, owned(&["label", "state", "assignee"])),
            (
                Tracker::Linear,
                owned(&["label", "state", "assignee", "project"]),
            ),
        ] {
            assert_eq!(
                check(
                    vec![("filters", block(vec![("colour", seq(&["red"]))]))],
                    tracker,
                ),
                Err(PullConfigError::UnsupportedFilterKey {
                    key: "colour".to_owned(),
                    accepted,
                }),
                "{tracker:?}"
            );
        }
    }

    #[test]
    fn a_project_filter_is_accepted_under_linear() {
        assert_eq!(
            check(
                vec![("filters", block(vec![("project", seq(&["Alpha"]))]))],
                Tracker::Linear,
            ),
            Ok(())
        );
    }

    #[test]
    fn a_project_filter_is_rejected_under_jira_listing_the_jira_set() {
        assert_eq!(
            check(
                vec![("filters", block(vec![("project", seq(&["Alpha"]))]))],
                Tracker::Jira,
            ),
            Err(PullConfigError::UnsupportedFilterKey {
                key: "project".to_owned(),
                accepted: owned(&["label", "state", "assignee"]),
            })
        );
    }

    #[test]
    fn a_reserved_grouping_key_is_rejected() {
        for reserved in ["all", "any"] {
            assert_eq!(
                check(
                    vec![("filters", block(vec![(reserved, seq(&["x"]))]))],
                    Tracker::Linear,
                ),
                Err(PullConfigError::NestedFiltersUnsupported {
                    key: reserved.to_owned(),
                })
            );
        }
    }

    #[test]
    fn all_with_additional_is_mutually_exclusive() {
        assert_eq!(
            check(
                vec![
                    ("all_projects", Value::Scalar(Scalar::Bool(true))),
                    ("additional_projects", seq(&["PP"])),
                ],
                Tracker::Jira,
            ),
            Err(PullConfigError::MutuallyExclusiveScope)
        );
    }

    #[test]
    fn an_unrecognised_top_level_key_is_rejected() {
        assert_eq!(
            check(vec![("bogus", scalar("x"))], Tracker::Jira),
            Err(PullConfigError::UnrecognisedKey {
                key: "bogus".to_owned(),
                accepted: owned(&[
                    "additional_projects",
                    "all_projects",
                    "filters",
                    "max_items",
                    "max_pages",
                ]),
                hint: None,
            })
        );
    }

    #[test]
    fn an_unrecognised_max_pages_sub_key_is_rejected() {
        assert_eq!(
            check(
                vec![(
                    "max_pages",
                    block(vec![("sideways", Value::Scalar(Scalar::Int(5)))]),
                )],
                Tracker::Jira,
            ),
            Err(PullConfigError::UnrecognisedKey {
                key: "max_pages.sideways".to_owned(),
                accepted: owned(&[
                    "additional_projects",
                    "all_projects",
                    "filters",
                    "max_items",
                    "max_pages",
                ]),
                hint: None,
            })
        );
    }

    #[test]
    fn a_wrong_tracker_noun_is_rejected_with_a_targeted_hint() {
        let result =
            check(vec![("additional_teams", seq(&["core"]))], Tracker::Jira);
        assert!(
            matches!(
                &result,
                Err(PullConfigError::UnrecognisedKey {
                    key,
                    hint: Some(hint),
                    ..
                }) if key == "additional_teams"
                    && hint.contains("Linear")
                    && hint.contains("Jira")
                    && hint.contains("additional_projects")
            ),
            "{result:?}"
        );
    }

    #[test]
    fn max_items_accepts_zero_but_max_pages_does_not() {
        assert_eq!(
            check(
                vec![("max_items", Value::Scalar(Scalar::Int(0)))],
                Tracker::Jira,
            ),
            Ok(())
        );
        assert_eq!(
            check(
                vec![("max_pages", Value::Scalar(Scalar::Int(0)))],
                Tracker::Jira,
            ),
            Err(PullConfigError::BadCeiling {
                key: "max_pages".to_owned(),
                value: "0".to_owned(),
                allow_zero: false,
            })
        );
    }

    #[test]
    fn ceilings_accept_unlimited_and_reject_negative_float_and_text() {
        assert_eq!(
            check(vec![("max_items", scalar("unlimited"))], Tracker::Jira),
            Ok(())
        );
        assert_eq!(
            check(vec![("max_pages", scalar("unlimited"))], Tracker::Jira),
            Ok(())
        );
        for bad in [
            Value::Scalar(Scalar::Int(-1)),
            Value::Scalar(Scalar::Float(1.5)),
            scalar("lots"),
        ] {
            assert!(matches!(
                check(vec![("max_items", bad)], Tracker::Jira),
                Err(PullConfigError::BadCeiling { .. })
            ));
        }
    }

    #[test]
    fn a_keyed_read_override_ceiling_is_validated() {
        assert_eq!(
            check(
                vec![(
                    "max_pages",
                    block(vec![("keyed_read", Value::Scalar(Scalar::Int(0)))]),
                )],
                Tracker::Linear,
            ),
            Err(PullConfigError::BadCeiling {
                key: "max_pages.keyed_read".to_owned(),
                value: "0".to_owned(),
                allow_zero: false,
            })
        );
    }

    #[allow(clippy::expect_used)]
    fn ceilings(entries: Vec<(&str, Value)>) -> Ceilings {
        parse(&block(entries))
            .expect("parse")
            .ceilings()
            .expect("ceilings")
    }

    #[test]
    fn an_empty_block_resolves_to_the_built_in_ceilings() {
        assert_eq!(
            ceilings(Vec::new()),
            Ceilings {
                max_items: Ceiling::Bounded(25),
                discovery_pages: Ceiling::Bounded(50),
                keyed_read_pages: Ceiling::Bounded(50),
            }
        );
    }

    #[test]
    fn max_items_resolves_its_configured_bound_zero_and_unlimited() {
        assert_eq!(
            ceilings(vec![("max_items", Value::Scalar(Scalar::Int(3)))])
                .max_items,
            Ceiling::Bounded(3)
        );
        assert_eq!(
            ceilings(vec![("max_items", Value::Scalar(Scalar::Int(0)))])
                .max_items,
            Ceiling::Bounded(0)
        );
        assert_eq!(
            ceilings(vec![("max_items", scalar("unlimited"))]).max_items,
            Ceiling::Unlimited
        );
    }

    #[test]
    fn a_scalar_max_pages_sets_both_page_caps() {
        let resolved =
            ceilings(vec![("max_pages", Value::Scalar(Scalar::Int(7)))]);
        assert_eq!(resolved.discovery_pages, Ceiling::Bounded(7));
        assert_eq!(resolved.keyed_read_pages, Ceiling::Bounded(7));
    }

    #[test]
    fn per_operation_overrides_resolve_independently_over_the_default() {
        let resolved = ceilings(vec![(
            "max_pages",
            block(vec![
                ("default", Value::Scalar(Scalar::Int(9))),
                ("keyed_read", scalar("unlimited")),
            ]),
        )]);
        assert_eq!(resolved.discovery_pages, Ceiling::Bounded(9));
        assert_eq!(resolved.keyed_read_pages, Ceiling::Unlimited);
    }

    #[test]
    fn a_hand_edited_bad_page_cap_is_rejected_at_interpretation() {
        let config = PullConfig {
            max_pages: PageCaps {
                default: Some(CeilingToken("0".to_owned())),
                ..PageCaps::default()
            },
            ..PullConfig::default()
        };
        assert_eq!(
            config.ceilings(),
            Err(PullConfigError::BadCeiling {
                key: "max_pages".to_owned(),
                value: "0".to_owned(),
                allow_zero: false,
            })
        );
    }

    #[test]
    fn detail_names_the_resolving_config_file() {
        let error = PullConfigError::MutuallyExclusiveScope;
        assert!(error
            .detail(config::Level::Personal)
            .contains(".accelerator/config.local.md"));
        assert!(error
            .detail(config::Level::Team)
            .contains(".accelerator/config.md"));
    }
}
