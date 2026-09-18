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
use config::Value;

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

/// Why a `pull` block could not be parsed into a [`PullConfig`].
///
/// Only structural shape faults — a non-mapping block, or a sub-key that must
/// be a mapping but is not. Acceptance faults (unsupported filter key, bad
/// ceiling token, `all`/`additional` exclusivity) are carried in the parsed
/// fields for a separate validation step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullConfigError {
    /// The block itself is not a mapping — a scalar or sequence where a block
    /// was expected (e.g. a typo'd `jira.pull: "oops"`). Never silently an
    /// empty config, so a malformed personal block cannot shadow a valid team
    /// block under whole-block replacement.
    NotAMapping,
    /// A sub-key that must carry a mapping (`filters`) does not.
    SubBlockNotAMapping { key: String },
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
            key if ADDITIONAL_KEYS.contains(&key) => {
                config
                    .additional_entities
                    .extend(value.as_string_sequence());
            }
            key if ALL_KEYS.contains(&key) => {
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

#[cfg(test)]
mod tests {
    use super::{parse, CeilingToken, PageCaps, PullConfig, PullConfigError};
    use config::{Scalar, Value};

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
            Ok(PullConfig::default())
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
                ..PullConfig::default()
            })
        );
    }
}
