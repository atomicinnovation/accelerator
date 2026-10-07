//! The `<tracker>.pull` discovery-scope config block: its typed model and the
//! parser that reads a resolved config mapping into it.
//!
//! The model is entity- and tracker-neutral: each tracker's own noun
//! (`additional_projects` for Jira, `additional_teams` for Linear) folds into
//! `additional_entities`, and `all_projects` / `all_teams` into `all_entities`.
//! The catalogue owns the block's structure; parsing checks only its shape,
//! leaving acceptance to [`validate`], so one parser serves both `configure`
//! and the sync path.

use config::catalogue::TRACKERS;
use config::render_value;
use config::tracker_block::read_block;
use config::tracker_block::BlockError;
use config::tracker_block::BlockFault;
use config::tracker_block::BlockName;
use config::tracker_block::FieldKind;
use config::tracker_block::TrackerCatalogue;
use config::ConfigAccess;
use config::Level;
use config::Value;
use tracker::Ceiling;
use tracker::DEFAULT_MAX_ITEMS;
use tracker::DEFAULT_MAX_PAGES;

/// A configured discovery-scope block, normalised to the port's entity-neutral
/// vocabulary.
#[derive(Debug, Clone, PartialEq, Default)]
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
    /// Catalogue fields with no typed slot here, in field order.
    pub extensions: Vec<(&'static str, Value)>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullConfigError {
    Structure(BlockError),
}

impl PullConfigError {
    /// The full operator message, naming the offending value, the accepted set,
    /// and the config file the effective block resolved from.
    #[must_use]
    pub fn detail(&self, level: Level) -> String {
        match self {
            Self::Structure(error) => error.detail(level),
        }
    }
}

/// Reads a resolved `<scope>.pull` block into a [`PullConfig`] against the
/// platform catalogue.
///
/// # Errors
///
/// A [`PullConfigError::Structure`] when the block, or a filters field in it,
/// is not a mapping.
pub fn parse(
    scope: &str,
    block: &Value,
) -> Result<PullConfig, PullConfigError> {
    parse_with(TRACKERS, scope, block)
}

/// [`parse`] against an injected catalogue. A key outside the scope's block
/// is left for [`validate_with`] to refuse.
///
/// # Errors
///
/// As [`parse`].
pub fn parse_with(
    catalogue: TrackerCatalogue,
    scope: &str,
    block: &Value,
) -> Result<PullConfig, PullConfigError> {
    let mut config = PullConfig::default();
    let Some(definition) = catalogue.block(scope, BlockName::Pull) else {
        return Ok(config);
    };
    let entries = definition
        .entries(block)
        .map_err(PullConfigError::Structure)?;
    for (key, value) in entries {
        let Some(field) = definition.field(key) else {
            continue;
        };
        match (field.kind, field.name) {
            (FieldKind::EntityList, _) => config
                .additional_entities
                .extend(value.as_string_sequence()),
            (FieldKind::ScopeFlag, _) => {
                config.all_entities |= is_truthy(value);
            }
            (FieldKind::Filters { .. }, "filters") => {
                parse_filters(value, &mut config);
            }
            (FieldKind::Ceiling { .. }, "max_items") => {
                config.max_items = Some(CeilingToken(render_value(value)));
            }
            (FieldKind::PageCaps { .. }, "max_pages") => {
                parse_page_caps(value, &mut config);
            }
            (_, name) => config.extensions.push((name, value.clone())),
        }
    }
    Ok(config)
}

fn parse_filters(value: &Value, config: &mut PullConfig) {
    if let Value::Mapping(fields) = value {
        for (field, field_value) in fields {
            config
                .filters
                .push((field.clone(), field_value.as_string_sequence()));
        }
    }
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
                    _ => {}
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

/// Validates a `<scope>.pull` block's structure against the platform
/// catalogue — remote existence of named entities is out of scope.
///
/// # Errors
///
/// The first structural fault, as a [`PullConfigError::Structure`].
pub fn validate(scope: &str, block: &Value) -> Result<(), PullConfigError> {
    validate_with(TRACKERS, scope, block)
}

/// [`validate`] against an injected catalogue.
///
/// # Errors
///
/// As [`validate`].
pub fn validate_with(
    catalogue: TrackerCatalogue,
    scope: &str,
    block: &Value,
) -> Result<(), PullConfigError> {
    catalogue
        .validate(scope, BlockName::Pull, block)
        .map_err(PullConfigError::Structure)
}

fn bad_ceiling(
    key: &str,
    token: &CeilingToken,
    allow_zero: bool,
) -> PullConfigError {
    PullConfigError::Structure(BlockError {
        block: BlockName::Pull,
        fault: BlockFault::BadCeiling {
            key: key.to_owned(),
            value: token.0.clone(),
            allow_zero,
        },
    })
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
    /// A [`BlockFault::BadCeiling`] for a malformed token. [`validate`]
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
    if TRACKERS.block(integration, BlockName::Pull).is_none() {
        return Ok(None);
    }
    let Some((value, level)) = read_block(config, integration, BlockName::Pull)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let parsed =
        parse(integration, &value).map_err(|error| error.detail(level))?;
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
        PullConfigError,
    };
    use config::tracker_block::{BlockError, BlockFault, BlockName};
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

    fn structure(fault: BlockFault) -> PullConfigError {
        PullConfigError::Structure(BlockError {
            block: BlockName::Pull,
            fault,
        })
    }

    #[test]
    fn a_jira_block_maps_additional_projects_to_additional_entities() {
        assert_eq!(
            parse(
                "jira",
                &block(vec![("additional_projects", seq(&["PP", "XX"]))])
            ),
            Ok(PullConfig {
                additional_entities: owned(&["PP", "XX"]),
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn a_linear_block_maps_additional_teams_to_additional_entities() {
        assert_eq!(
            parse(
                "linear",
                &block(vec![("additional_teams", seq(&["core", "ops"]))])
            ),
            Ok(PullConfig {
                additional_entities: owned(&["core", "ops"]),
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn an_absent_block_is_the_empty_config() {
        assert_eq!(
            parse("jira", &Value::Mapping(Vec::new())),
            Ok(PullConfig::default())
        );
    }

    #[test]
    fn each_trackers_all_noun_sets_all_entities() {
        for (scope, noun) in [("jira", "all_projects"), ("linear", "all_teams")]
        {
            assert_eq!(
                parse(
                    scope,
                    &block(vec![(noun, Value::Scalar(Scalar::Bool(true)))])
                ),
                Ok(PullConfig {
                    all_entities: true,
                    ..PullConfig::default()
                })
            );
        }
    }

    #[test]
    fn all_entities_is_false_when_the_flag_is_false() {
        assert_eq!(
            parse(
                "linear",
                &block(vec![("all_teams", Value::Scalar(Scalar::Bool(false)))])
            ),
            Ok(PullConfig::default())
        );
    }

    #[test]
    fn another_trackers_noun_is_left_for_validation() {
        assert_eq!(
            parse("jira", &block(vec![("additional_teams", seq(&["core"]))])),
            Ok(PullConfig::default())
        );
    }

    #[test]
    fn filters_carry_keys_and_their_value_lists() {
        assert_eq!(
            parse(
                "jira",
                &block(vec![(
                    "filters",
                    block(vec![
                        ("label", seq(&["a", "b"])),
                        ("state", scalar("open")),
                    ]),
                )])
            ),
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
            parse(
                "jira",
                &block(vec![("max_items", Value::Scalar(Scalar::Int(3)))])
            ),
            Ok(PullConfig {
                max_items: Some(token("3")),
                ..PullConfig::default()
            })
        );
    }

    #[test]
    fn a_scalar_max_pages_sets_the_general_default_cap() {
        assert_eq!(
            parse(
                "jira",
                &block(vec![("max_pages", Value::Scalar(Scalar::Int(50)))])
            ),
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
            parse(
                "linear",
                &block(vec![(
                    "max_pages",
                    block(vec![
                        ("discovery", Value::Scalar(Scalar::Int(20))),
                        ("keyed_read", scalar("unlimited")),
                    ]),
                )])
            ),
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
    fn a_non_mapping_block_is_a_shape_error() {
        for value in [scalar("oops"), seq(&["a", "b"])] {
            assert_eq!(
                parse("jira", &value),
                Err(structure(BlockFault::NotAMapping))
            );
        }
    }

    #[test]
    fn a_non_mapping_filters_is_a_shape_error() {
        assert_eq!(
            parse("jira", &block(vec![("filters", scalar("oops"))])),
            Err(structure(BlockFault::SubBlockNotAMapping {
                key: "filters".to_owned()
            }))
        );
    }

    #[test]
    fn a_scope_with_no_pull_block_parses_to_the_empty_config() {
        assert_eq!(parse("trello", &scalar("oops")), Ok(PullConfig::default()));
    }

    #[test]
    fn validate_wraps_the_catalogues_first_fault() {
        assert_eq!(
            validate(
                "jira",
                &block(vec![
                    ("all_projects", Value::Scalar(Scalar::Bool(true))),
                    ("additional_projects", seq(&["PP"])),
                ])
            ),
            Err(structure(BlockFault::MutuallyExclusive {
                first: "all_projects".to_owned(),
                second: "additional_projects".to_owned(),
            }))
        );
        assert_eq!(
            validate("linear", &block(vec![("all_teams", seq(&[]))])),
            Ok(())
        );
    }

    #[allow(clippy::expect_used)]
    fn ceilings(entries: Vec<(&str, Value)>) -> Ceilings {
        parse("jira", &block(entries))
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
            Err(structure(BlockFault::BadCeiling {
                key: "max_pages".to_owned(),
                value: "0".to_owned(),
                allow_zero: false,
            }))
        );
    }

    #[test]
    fn detail_names_the_resolving_config_file() {
        let error = structure(BlockFault::NotAMapping);
        assert!(error
            .detail(config::Level::Personal)
            .contains(".accelerator/config.local.md"));
        assert!(error
            .detail(config::Level::Team)
            .contains(".accelerator/config.md"));
    }
}
