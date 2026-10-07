//! The `<tracker>.push` write-bound config block: its typed model and the
//! parser that reads a resolved config mapping into it.
//!
//! Push has one knob — the write bound on remote issues a run may create or
//! replace — so the block is a single `max_items` key. The pull block's scope,
//! filters, and page caps have no push analogue, so this block is deliberately
//! narrower than [`crate::pull`].

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

/// A configured `<tracker>.push` block.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PushConfig {
    /// The push-direction write bound, held as a raw token for later
    /// interpretation.
    pub max_items: Option<String>,
    /// Catalogue fields with no typed slot here, in field order.
    pub extensions: Vec<(&'static str, Value)>,
}

/// Why a `push` block is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushConfigError {
    Structure(BlockError),
}

impl PushConfigError {
    /// The full operator message, naming the offending value and the config
    /// file the effective block resolved from.
    #[must_use]
    pub fn detail(&self, level: Level) -> String {
        match self {
            Self::Structure(error) => error.detail(level),
        }
    }
}

/// Reads a resolved `<scope>.push` block into a [`PushConfig`] against the
/// platform catalogue.
///
/// # Errors
///
/// A [`PushConfigError::Structure`] when the block is not a mapping.
pub fn parse(
    scope: &str,
    block: &Value,
) -> Result<PushConfig, PushConfigError> {
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
) -> Result<PushConfig, PushConfigError> {
    let mut config = PushConfig::default();
    let Some(definition) = catalogue.block(scope, BlockName::Push) else {
        return Ok(config);
    };
    let entries = definition
        .entries(block)
        .map_err(PushConfigError::Structure)?;
    for (key, value) in entries {
        let Some(field) = definition.field(key) else {
            continue;
        };
        match (field.kind, field.name) {
            (FieldKind::Ceiling { .. }, "max_items") => {
                config.max_items = Some(render_value(value));
            }
            (_, name) => config.extensions.push((name, value.clone())),
        }
    }
    Ok(config)
}

/// Validates a `<scope>.push` block's structure against the platform
/// catalogue.
///
/// # Errors
///
/// The first structural fault, as a [`PushConfigError::Structure`].
pub fn validate(scope: &str, block: &Value) -> Result<(), PushConfigError> {
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
) -> Result<(), PushConfigError> {
    catalogue
        .validate(scope, BlockName::Push, block)
        .map_err(PushConfigError::Structure)
}

impl PushConfig {
    /// Resolves the write bound, applying [`DEFAULT_MAX_ITEMS`] when unset.
    ///
    /// # Errors
    ///
    /// A [`BlockFault::BadCeiling`] for a malformed token. [`validate`]
    /// rejects the same tokens at configure time, so this only fires on a
    /// hand-edited config that reaches interpretation unvalidated.
    pub fn max_items(&self) -> Result<Ceiling, PushConfigError> {
        self.max_items
            .as_ref()
            .map_or(Ok(DEFAULT_MAX_ITEMS), |token| {
                crate::ceiling::from_token(token, true).ok_or_else(|| {
                    PushConfigError::Structure(BlockError {
                        block: BlockName::Push,
                        fault: BlockFault::BadCeiling {
                            key: "max_items".to_owned(),
                            value: token.clone(),
                            allow_zero: true,
                        },
                    })
                })
            })
    }
}

/// Reads and parses the active tracker's `<tracker>.push` block from resolved
/// config, paired with the config level it resolved from.
///
/// Returns `None` when the integration has no tracker surface, no block is
/// configured, or the block is empty — each meaning the built-in write bound.
/// Reads the already-composed config representation, so it performs no
/// filesystem or subprocess access.
///
/// # Errors
///
/// A config-access failure, or a structural parse fault ([`PushConfigError`])
/// rendered against the resolving config file.
pub fn read(
    config: &dyn ConfigAccess,
    integration: &str,
) -> Result<Option<(PushConfig, Level)>, String> {
    if TRACKERS.block(integration, BlockName::Push).is_none() {
        return Ok(None);
    }
    let Some((value, level)) = read_block(config, integration, BlockName::Push)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let parsed =
        parse(integration, &value).map_err(|error| error.detail(level))?;
    Ok(Some((parsed, level)))
}

#[cfg(test)]
mod tests {
    use super::{parse, validate, PushConfig, PushConfigError};
    use config::tracker_block::{BlockError, BlockFault, BlockName};
    use config::{Scalar, Value};
    use tracker::{Ceiling, DEFAULT_MAX_ITEMS};

    fn scalar(text: &str) -> Value {
        Value::Scalar(Scalar::String(text.to_owned()))
    }

    fn block(entries: Vec<(&str, Value)>) -> Value {
        Value::Mapping(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    fn structure(fault: BlockFault) -> PushConfigError {
        PushConfigError::Structure(BlockError {
            block: BlockName::Push,
            fault,
        })
    }

    #[test]
    fn a_max_items_scalar_is_held_as_a_raw_token() {
        assert_eq!(
            parse("jira", &block(vec![("max_items", scalar("10"))])),
            Ok(PushConfig {
                max_items: Some("10".to_owned()),
                ..PushConfig::default()
            })
        );
    }

    #[test]
    fn an_unknown_key_is_left_for_validation() {
        assert_eq!(
            parse("linear", &block(vec![("max_pages", scalar("5"))])),
            Ok(PushConfig::default())
        );
    }

    #[test]
    fn a_scalar_block_is_not_a_mapping() {
        assert_eq!(
            parse("jira", &scalar("oops")),
            Err(structure(BlockFault::NotAMapping))
        );
    }

    #[test]
    fn an_unset_max_items_resolves_to_the_built_in_default() {
        assert_eq!(PushConfig::default().max_items(), Ok(DEFAULT_MAX_ITEMS));
    }

    #[test]
    fn max_items_resolves_its_bound_zero_and_unlimited() {
        let bounded = PushConfig {
            max_items: Some("9".to_owned()),
            ..PushConfig::default()
        };
        assert_eq!(bounded.max_items(), Ok(Ceiling::Bounded(9)));
        let refuse_all = PushConfig {
            max_items: Some("0".to_owned()),
            ..PushConfig::default()
        };
        assert_eq!(refuse_all.max_items(), Ok(Ceiling::Bounded(0)));
        let unbounded = PushConfig {
            max_items: Some("unlimited".to_owned()),
            ..PushConfig::default()
        };
        assert_eq!(unbounded.max_items(), Ok(Ceiling::Unlimited));
    }

    #[test]
    fn a_hand_edited_bad_max_items_is_rejected_at_interpretation() {
        let config = PushConfig {
            max_items: Some("2.5".to_owned()),
            ..PushConfig::default()
        };
        assert_eq!(
            config.max_items(),
            Err(structure(BlockFault::BadCeiling {
                key: "max_items".to_owned(),
                value: "2.5".to_owned(),
                allow_zero: true,
            }))
        );
    }

    #[test]
    fn validate_wraps_the_catalogues_first_fault() {
        assert_eq!(
            validate("jira", &block(vec![("filters", scalar("x"))])),
            Err(structure(BlockFault::UnrecognisedKey {
                key: "filters".to_owned(),
                accepted: vec!["max_items".to_owned()],
                hint: None,
            }))
        );
        assert_eq!(
            validate(
                "linear",
                &block(vec![("max_items", scalar("unlimited"))])
            ),
            Ok(())
        );
    }
}
