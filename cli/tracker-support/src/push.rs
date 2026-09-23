//! The `<tracker>.push` write-bound config block: its typed model and the
//! parser that reads a resolved config mapping into it.
//!
//! Push has one knob — the write bound on remote issues a run may create or
//! replace — so the block is a single `max_items` key. The pull block's scope,
//! filters, and page caps have no push analogue, so this block is deliberately
//! narrower than [`crate::pull`].

use config::render_value;
use config::ConfigAccess;
use config::Level;
use config::Value;
use tracker::Ceiling;
use tracker::DEFAULT_MAX_ITEMS;

use crate::pull::Tracker;

/// A configured `<tracker>.push` block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PushConfig {
    /// The push-direction write bound, held as a raw token for later
    /// interpretation.
    pub max_items: Option<String>,
    /// Top-level keys the parser did not recognise, retained rather than
    /// dropped so validation can reject them.
    pub unknown_keys: Vec<String>,
}

/// Why a `push` block is invalid. Each carries the data its message names;
/// [`PushConfigError::detail`] renders the operator message, naming the config
/// file the effective block resolved from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushConfigError {
    /// The block itself is not a mapping — a scalar or sequence where a block
    /// was expected.
    NotAMapping,
    /// A `max_items` token that is neither a non-negative integer nor
    /// `unlimited`.
    BadCeiling { value: String },
    /// A top-level key the block does not accept.
    UnrecognisedKey { key: String },
}

impl PushConfigError {
    /// The full operator message, naming the offending value and the config
    /// file the effective block resolved from.
    #[must_use]
    pub fn detail(&self, level: Level) -> String {
        let file = level.filename();
        let fix = format!("Fix the `push` block in {file}.");
        match self {
            Self::NotAMapping => format!(
                "the `push` value must be a block of settings, not a scalar \
                 or list. {fix}"
            ),
            Self::BadCeiling { value } => format!(
                "push `max_items` must be a non-negative integer (0 refuses \
                 all) or `unlimited` (got `{value}`). {fix}"
            ),
            Self::UnrecognisedKey { key } => format!(
                "the push block key `{key}` is not recognised (accepted: \
                 max_items). {fix}"
            ),
        }
    }
}

/// Reads a resolved config block into a [`PushConfig`].
///
/// # Errors
///
/// [`PushConfigError::NotAMapping`] when the block is not a mapping.
pub fn parse(block: &Value) -> Result<PushConfig, PushConfigError> {
    let Value::Mapping(entries) = block else {
        return Err(PushConfigError::NotAMapping);
    };
    let mut config = PushConfig::default();
    for (key, value) in entries {
        match key.as_str() {
            "max_items" => config.max_items = Some(render_value(value)),
            other => config.unknown_keys.push(other.to_owned()),
        }
    }
    Ok(config)
}

/// Validates a parsed `push` block, structural only.
///
/// # Errors
///
/// A [`PushConfigError`] for an unrecognised top-level key or a malformed
/// `max_items` token.
pub fn validate(config: &PushConfig) -> Result<(), PushConfigError> {
    if let Some(key) = config.unknown_keys.first() {
        return Err(PushConfigError::UnrecognisedKey { key: key.clone() });
    }
    config.max_items().map(|_| ())
}

impl PushConfig {
    /// Resolves the write bound, applying [`DEFAULT_MAX_ITEMS`] when unset.
    ///
    /// # Errors
    ///
    /// [`PushConfigError::BadCeiling`] for a malformed token. [`validate`]
    /// rejects the same tokens at configure time, so this only fires on a
    /// hand-edited config that reaches interpretation unvalidated.
    pub fn max_items(&self) -> Result<Ceiling, PushConfigError> {
        self.max_items
            .as_ref()
            .map_or(Ok(DEFAULT_MAX_ITEMS), |token| {
                crate::ceiling::from_token(token, true).ok_or_else(|| {
                    PushConfigError::BadCeiling {
                        value: token.clone(),
                    }
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
    if Tracker::from_integration(integration).is_none() {
        return Ok(None);
    }
    let Some((value, level)) =
        crate::block::read_block(config, integration, "push")?
    else {
        return Ok(None);
    };
    let parsed = parse(&value).map_err(|error| error.detail(level))?;
    Ok(Some((parsed, level)))
}

#[cfg(test)]
mod tests {
    use super::{parse, validate, PushConfig, PushConfigError};
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

    #[test]
    fn a_max_items_scalar_is_held_as_a_raw_token() {
        assert_eq!(
            parse(&block(vec![("max_items", scalar("10"))])),
            Ok(PushConfig {
                max_items: Some("10".to_owned()),
                ..PushConfig::default()
            })
        );
    }

    #[test]
    fn an_unknown_key_is_retained_for_validation() {
        assert_eq!(
            parse(&block(vec![("max_pages", scalar("5"))])),
            Ok(PushConfig {
                unknown_keys: vec!["max_pages".to_owned()],
                ..PushConfig::default()
            })
        );
    }

    #[test]
    fn a_scalar_block_is_not_a_mapping() {
        assert_eq!(parse(&scalar("oops")), Err(PushConfigError::NotAMapping));
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
    fn validate_rejects_a_bad_ceiling_and_an_unknown_key() {
        let bad_ceiling = PushConfig {
            max_items: Some("2.5".to_owned()),
            ..PushConfig::default()
        };
        assert_eq!(
            validate(&bad_ceiling),
            Err(PushConfigError::BadCeiling {
                value: "2.5".to_owned()
            })
        );
        let unknown = PushConfig {
            unknown_keys: vec!["filters".to_owned()],
            ..PushConfig::default()
        };
        assert_eq!(
            validate(&unknown),
            Err(PushConfigError::UnrecognisedKey {
                key: "filters".to_owned()
            })
        );
    }

    #[test]
    fn validate_accepts_a_well_formed_block() {
        let unlimited = PushConfig {
            max_items: Some("unlimited".to_owned()),
            ..PushConfig::default()
        };
        assert_eq!(validate(&unlimited), Ok(()));
    }
}
