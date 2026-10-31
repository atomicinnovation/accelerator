//! The work-item ID scheme read from configuration, in steps each consumer
//! composes as its job needs.

use ::config::resolve_with_deprecated_fallback;
use ::config::ConfigAccess;
use ::config::Key;
use work::id_pattern::escaped;
use work::id_pattern::IdKey;
use work::id_pattern::IdPattern;
use work::id_pattern::KeyError;
use work::id_pattern::PatternError;
use work::id_scheme::check_tracker_integration;
use work::id_scheme::TrackerIntegrationError;
use work::id_scheme::WorkItemIdScheme;

const PATTERN_SETTING: &str = "work.id_pattern";

const KEY_SETTING: &str = "work.key";

pub const LEGACY_KEY_SETTING: &str = "work.default_project_code";

pub struct SchemeReading {
    pub scheme: WorkItemIdScheme,
    pub deprecation: Option<String>,
}

#[derive(Debug)]
pub enum SchemeError {
    Config(kernel::Error),
    Pattern {
        value: String,
        error: PatternError,
    },
    Key {
        setting: String,
        value: String,
        error: KeyError,
    },
    KeyRequired {
        pattern: String,
    },
    Integration(TrackerIntegrationError),
}

impl std::fmt::Display for SchemeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::Pattern { value, error } => write!(
                formatter,
                "E_WORK_ID_PATTERN_INVALID: {PATTERN_SETTING} '{}': {error}",
                escaped(value)
            ),
            Self::Key {
                setting,
                value,
                error,
            } => write!(
                formatter,
                "E_WORK_KEY_INVALID: {setting} '{}': {error}",
                escaped(value)
            ),
            Self::KeyRequired { pattern } => write!(
                formatter,
                "E_WORK_KEY_REQUIRED: id_pattern '{}' references the \
                 {{key}} prefix but work.key is not set. The local ID prefix \
                 is the work-owned work.key; it is independent of the tracker \
                 scope key (jira.project_key / linear.team_key), never \
                 derives from it, and must be set explicitly. Set work.key \
                 in .accelerator/config.md.",
                escaped(pattern)
            ),
            Self::Integration(error) => {
                write!(formatter, "E_WORK_ID_PATTERN_INVALID: {error}")
            }
        }
    }
}

impl std::error::Error for SchemeError {}

impl From<::config::ConfigError> for SchemeError {
    fn from(error: ::config::ConfigError) -> Self {
        Self::Config(kernel::Error::Failed(error.to_string()))
    }
}

fn configured(
    config: &dyn ConfigAccess,
    setting: &str,
) -> Result<Option<String>, ::config::ConfigError> {
    Ok(config
        .effective_nonempty(&Key::parse(setting)?, None)?
        .configured_value())
}

/// The configured pattern and, only when the pattern has a key token, the
/// configured key, read from `work.key` or its deprecated alias. The alias's
/// deprecation notice is returned, never printed.
///
/// # Errors
///
/// [`SchemeError::Config`] when a setting cannot be read,
/// [`SchemeError::Pattern`] when the pattern is malformed, and
/// [`SchemeError::Key`] when the key it needs is.
pub fn read_scheme(
    config: &dyn ConfigAccess,
) -> Result<SchemeReading, SchemeError> {
    let text = config
        .effective_nonempty(&Key::parse(PATTERN_SETTING)?, None)?
        .rendered();
    let pattern =
        IdPattern::parse(&text).map_err(|error| SchemeError::Pattern {
            value: text.clone(),
            error,
        })?;
    if !pattern.references_key() {
        return Ok(SchemeReading {
            scheme: WorkItemIdScheme::new(pattern, None),
            deprecation: None,
        });
    }
    let aliased = resolve_with_deprecated_fallback(
        config,
        KEY_SETTING,
        LEGACY_KEY_SETTING,
        None,
    )?;
    let key = match aliased.value {
        Some(value) => {
            let setting = if configured(config, KEY_SETTING)?.is_some() {
                KEY_SETTING
            } else {
                LEGACY_KEY_SETTING
            };
            Some(IdKey::parse(&value).map_err(|error| SchemeError::Key {
                setting: setting.to_owned(),
                value,
                error,
            })?)
        }
        None => None,
    };
    Ok(SchemeReading {
        scheme: WorkItemIdScheme::new(pattern, key),
        deprecation: aliased.deprecation,
    })
}

/// # Errors
///
/// [`SchemeError::KeyRequired`] when the pattern has a key token and no key
/// is configured.
pub fn require_key(scheme: &WorkItemIdScheme) -> Result<(), SchemeError> {
    if scheme.pattern().references_key() && scheme.key().is_none() {
        Err(SchemeError::KeyRequired {
            pattern: scheme.pattern().as_str().to_owned(),
        })
    } else {
        Ok(())
    }
}

/// The scheme a command that writes or allocates IDs needs:
/// [`read_scheme`], then [`require_key`], then the `{tracker}` integration
/// rule.
///
/// The key is tracker-independent: it always comes from `work.key`, never
/// the scope key, and populates the scheme only when the pattern uses it.
///
/// # Errors
///
/// Any [`SchemeError`] of [`read_scheme`] or [`require_key`], and
/// [`SchemeError::Integration`] when `{tracker}` is configured under an
/// integration whose keys cannot serve as `id`s.
pub fn resolve_scheme(
    config: &dyn ConfigAccess,
) -> Result<SchemeReading, SchemeError> {
    let reading = read_scheme(config)?;
    require_key(&reading.scheme)?;
    let integration = configured(config, "work.integration")?;
    check_tracker_integration(
        reading.scheme.pattern(),
        integration
            .as_deref()
            .is_some_and(tracker::supports_tracker_owned_ids),
    )
    .map_err(SchemeError::Integration)?;
    Ok(reading)
}

#[cfg(test)]
#[allow(clippy::literal_string_with_formatting_args, clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use ::config::ConfigAccess;
    use ::config::ConfigError;
    use ::config::Key;
    use ::config::Level;
    use ::config::Resolved;
    use ::config::Scalar;
    use ::config::Value;
    use work::id_pattern::IdKey;
    use work::id_pattern::IdPattern;
    use work::id_pattern::KeyError;
    use work::id_pattern::PatternError;
    use work::id_scheme::IdOwnership;
    use work::id_scheme::TrackerIntegrationError;
    use work::id_scheme::WorkItemIdScheme;

    use super::read_scheme;
    use super::require_key;
    use super::resolve_scheme;
    use super::SchemeError;

    struct FakeConfig(HashMap<String, String>);

    impl FakeConfig {
        fn with(pairs: &[(&str, &str)]) -> Self {
            Self(
                pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
            )
        }
    }

    impl ConfigAccess for FakeConfig {
        fn get(
            &self,
            key: &Key,
            _level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            Ok(self
                .0
                .get(&key.to_string())
                .map_or(Resolved::Absent, |value| {
                    Resolved::Found(Value::Scalar(Scalar::String(
                        value.clone(),
                    )))
                }))
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            Err(ConfigError::Invalid {
                detail: "set unsupported in the fake config".to_owned(),
            })
        }
    }

    fn key_of(scheme: &WorkItemIdScheme) -> Option<&str> {
        scheme.key().map(IdKey::as_str)
    }

    #[test]
    fn resolve_scheme_sources_the_prefix_from_work_key() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{key}-{number:04d}"),
            ("work.key", "PP"),
        ]);
        let reading = resolve_scheme(&config).unwrap();
        assert_eq!(key_of(&reading.scheme), Some("PP"));
    }

    #[test]
    fn resolve_scheme_falls_back_to_the_deprecated_prefix_key() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{project}-{number:04d}"),
            ("work.default_project_code", "PP"),
        ]);
        let reading = resolve_scheme(&config).unwrap();
        assert_eq!(key_of(&reading.scheme), Some("PP"));
    }

    #[test]
    fn resolve_scheme_requires_work_key_when_the_pattern_uses_the_prefix() {
        let config =
            FakeConfig::with(&[("work.id_pattern", "{key}-{number:04d}")]);
        let error = resolve_scheme(&config).err().unwrap();
        assert!(error.to_string().contains("work.key"), "{error}");
    }

    #[test]
    fn resolve_scheme_accepts_work_key_equal_to_a_scope_key() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{key}-{number:04d}"),
            ("work.key", "PP"),
            ("jira.project_key", "PP"),
            ("work.integration", "jira"),
        ]);
        let reading = resolve_scheme(&config).unwrap();
        assert_eq!(key_of(&reading.scheme), Some("PP"));
    }

    #[test]
    fn resolve_scheme_accepts_tracker_under_jira_or_linear() {
        for integration in ["jira", "linear"] {
            let config = FakeConfig::with(&[
                ("work.id_pattern", "{tracker}"),
                ("work.integration", integration),
            ]);
            let reading = resolve_scheme(&config).unwrap();
            assert_eq!(reading.scheme.ownership(), IdOwnership::Tracker);
        }
    }

    #[test]
    fn resolve_scheme_rejects_an_invalid_tracker_pattern() {
        let cases = [
            ("{tracker}-{number:04d}", Some("linear")),
            ("{tracker}", None),
            ("{tracker}", Some("trello")),
            ("{tracker}", Some("github-issues")),
        ];
        for (pattern, integration) in cases {
            let mut pairs = vec![("work.id_pattern", pattern)];
            pairs.extend(integration.map(|name| ("work.integration", name)));
            let error =
                resolve_scheme(&FakeConfig::with(&pairs)).err().unwrap();
            assert!(
                error.to_string().starts_with("E_WORK_ID_PATTERN_INVALID: "),
                "{pattern} {integration:?}: {error}"
            );
        }
    }

    #[test]
    fn the_integration_error_display_equals_resolve_schemes_message() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{tracker}"),
            ("work.integration", "trello"),
        ]);
        let error = resolve_scheme(&config).err().unwrap();
        assert_eq!(
            error.to_string(),
            format!(
                "E_WORK_ID_PATTERN_INVALID: {}",
                TrackerIntegrationError::TrackerNeedsJiraOrLinear
            )
        );
    }

    #[test]
    fn resolve_scheme_gates_the_prefix_field_on_a_bare_numeric_pattern() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{number:04d}"),
            ("work.key", "PP"),
        ]);
        let reading = resolve_scheme(&config).unwrap();
        assert_eq!(reading.scheme.key(), None);
    }

    #[test]
    fn read_scheme_refuses_a_malformed_pattern_or_hostile_key() {
        let malformed = [
            "{key}-x",
            "x/{number:04d}",
            "{key}{number:04d}",
            "{number:4d}",
            "x'{number:04d}",
            "x\t{number:04d}",
            "{number:020d}",
        ];
        for pattern in malformed {
            let config = FakeConfig::with(&[
                ("work.id_pattern", pattern),
                ("work.key", "PP"),
            ]);
            assert!(
                matches!(
                    read_scheme(&config),
                    Err(SchemeError::Pattern { ref value, .. })
                        if value == pattern
                ),
                "{pattern:?}"
            );
        }
        for (setting, value) in [
            ("work.key", "1PP"),
            ("work.key", "../x"),
            ("work.key", "P-P"),
        ] {
            let config = FakeConfig::with(&[
                ("work.id_pattern", "{key}-{number:04d}"),
                (setting, value),
            ]);
            assert!(
                matches!(
                    read_scheme(&config),
                    Err(SchemeError::Key { setting: ref named, .. })
                        if named == setting
                ),
                "{value:?}"
            );
        }
        let legacy = FakeConfig::with(&[
            ("work.id_pattern", "{project}-{number:04d}"),
            ("work.default_project_code", "../x"),
        ]);
        assert!(matches!(
            read_scheme(&legacy),
            Err(SchemeError::Key { ref setting, .. })
                if setting == "work.default_project_code"
        ));
    }

    #[test]
    fn a_scheme_error_names_the_setting_and_value() {
        let pattern = SchemeError::Pattern {
            value: "{number:4d}".to_owned(),
            error: IdPattern::parse("{number:4d}").unwrap_err(),
        };
        assert_eq!(
            pattern.to_string(),
            format!(
                "E_WORK_ID_PATTERN_INVALID: work.id_pattern '{{number:4d}}': \
                 {}",
                PatternError::BadFormatSpec("4d".to_owned())
            )
        );
        let key = SchemeError::Key {
            setting: "work.key".to_owned(),
            value: "../x".to_owned(),
            error: KeyError::BadKeyValue("../x".to_owned()),
        };
        assert_eq!(
            key.to_string(),
            format!(
                "E_WORK_KEY_INVALID: work.key '../x': {}",
                KeyError::BadKeyValue("../x".to_owned())
            )
        );
        let hostile = SchemeError::Pattern {
            value: "x'\u{1b}{number:04d}".to_owned(),
            error: PatternError::HostileChar('\''),
        };
        assert!(
            hostile.to_string().starts_with(
                "E_WORK_ID_PATTERN_INVALID: work.id_pattern \
                 'x\\'U+001B{number:04d}': "
            ),
            "{hostile}"
        );
        let required = SchemeError::KeyRequired {
            pattern: "{key}-{number:04d}".to_owned(),
        };
        assert!(required.to_string().starts_with(
            "E_WORK_KEY_REQUIRED: id_pattern '{key}-{number:04d}' references \
             the {key} prefix but work.key is not set."
        ));
        assert_eq!(
            SchemeError::Integration(
                TrackerIntegrationError::TrackerNeedsJiraOrLinear
            )
            .to_string(),
            format!(
                "E_WORK_ID_PATTERN_INVALID: {}",
                TrackerIntegrationError::TrackerNeedsJiraOrLinear
            )
        );
    }

    #[test]
    fn read_scheme_accepts_a_key_pattern_without_a_key() {
        let config =
            FakeConfig::with(&[("work.id_pattern", "{key}-{number:04d}")]);
        let reading = read_scheme(&config).unwrap();
        assert_eq!(reading.scheme.key(), None);
    }

    #[test]
    fn require_key_refuses_it_with_e_work_key_required() {
        let config =
            FakeConfig::with(&[("work.id_pattern", "{key}-{number:04d}")]);
        let reading = read_scheme(&config).unwrap();
        assert!(matches!(
            require_key(&reading.scheme),
            Err(SchemeError::KeyRequired { ref pattern })
                if pattern == "{key}-{number:04d}"
        ));
        assert!(require_key(&WorkItemIdScheme::numeric()).is_ok());
    }

    #[test]
    fn read_scheme_ignores_an_invalid_key_under_a_keyless_pattern() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{number:04d}"),
            ("work.key", "../x"),
            ("work.default_project_code", "PP"),
        ]);
        let reading = read_scheme(&config).unwrap();
        assert_eq!(reading.scheme.key(), None);
        assert_eq!(reading.deprecation, None);
    }

    #[test]
    fn read_scheme_returns_the_deprecation_notice() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{project}-{number:04d}"),
            ("work.default_project_code", "PP"),
        ]);
        let reading = read_scheme(&config).unwrap();
        assert!(
            reading.deprecation.as_deref().is_some_and(|notice| notice
                .contains("'work.default_project_code' is deprecated")),
            "{:?}",
            reading.deprecation
        );
    }
}
