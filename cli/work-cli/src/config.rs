//! Shared config-resolution helpers used by more than one subcommand's
//! adapter wiring: the `work.id_pattern`/`work.key` scheme, and the configured
//! work-item directory.

use std::path::Path;
use std::path::PathBuf;

use ::config::resolve_with_deprecated_fallback;
use ::config::ConfigAccess;
use ::config::Key;
use corpus::references_key;
use corpus::WorkItemIdScheme;

pub const LEGACY_PREFIX_KEY: &str = "work.default_project_code";

pub fn effective_nonempty(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<String, kernel::Error> {
    let parsed = Key::parse(key)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    Ok(config
        .effective_nonempty(&parsed, None)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?
        .rendered())
}

fn work_key_required(id_pattern: &str) -> kernel::Error {
    kernel::Error::Failed(format!(
        "E_WORK_KEY_REQUIRED: id_pattern '{id_pattern}' references the \
         {{key}} prefix but work.key is not set. The local ID prefix is the \
         work-owned work.key; it is independent of the tracker scope key \
         (jira.project_key / linear.team_key), never derives from it, and \
         must be set explicitly. Set work.key in .accelerator/config.md."
    ))
}

/// Resolves the work-item ID scheme, sourcing the local prefix from `work.key`
/// (with the deprecated `work.default_project_code` alias) only when the
/// pattern references the prefix token.
///
/// The prefix is tracker-independent: it always comes from `work.key`, never
/// the scope key. When the pattern references `{key}` and no prefix resolves,
/// this is a config-validation error. The prefix populates the scheme field
/// only when the pattern uses it, so a bare-numeric pattern never inherits a
/// tracker prefix into the scheme's admission predicates.
///
/// # Errors
///
/// A [`kernel::Error`] when a key read fails, or when the pattern references
/// `{key}` but no prefix is configured.
pub fn resolve_scheme(
    config: &dyn ConfigAccess,
) -> Result<WorkItemIdScheme, kernel::Error> {
    let id_pattern = effective_nonempty(config, "work.id_pattern")?;
    let pattern_uses_key = references_key(&id_pattern);
    let prefix = resolve_with_deprecated_fallback(
        config,
        "work.key",
        LEGACY_PREFIX_KEY,
        None,
    )?;

    if pattern_uses_key && prefix.value.is_none() {
        return Err(work_key_required(&id_pattern));
    }

    if pattern_uses_key {
        ::config::emit_deprecation_once(
            LEGACY_PREFIX_KEY,
            prefix.deprecation.as_deref(),
        );
    }

    Ok(WorkItemIdScheme {
        id_pattern,
        key: pattern_uses_key.then_some(prefix.value).flatten(),
    })
}

pub fn templates_dir(
    config: &dyn ConfigAccess,
) -> Result<String, kernel::Error> {
    effective_nonempty(config, "paths.templates")
}

pub fn configured_override(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Option<String>, kernel::Error> {
    let parsed = Key::parse(key)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    Ok(config
        .effective_nonempty(&parsed, None)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?
        .configured_value())
}

pub fn resolve_work_dir(
    config: &dyn ConfigAccess,
    root: &Path,
) -> Result<PathBuf, kernel::Error> {
    let dirs = ::config::paths::doc_type_dirs(config)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    let relative = dirs
        .into_iter()
        .find(|resolved| resolved.doc_type == "work-item")
        .map(|resolved| resolved.dir)
        .ok_or_else(|| {
            kernel::Error::Failed(
                "no work-item doc-type directory configured".to_owned(),
            )
        })?;
    Ok(root.join(relative))
}

#[cfg(test)]
#[allow(clippy::literal_string_with_formatting_args, clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use ::config::{
        ConfigAccess, ConfigError, Key, Level, Resolved, Scalar, Value,
    };

    use super::resolve_scheme;

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

    #[test]
    fn resolve_scheme_sources_the_prefix_from_work_key() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{key}-{number:04d}"),
            ("work.key", "PP"),
        ]);
        let scheme = resolve_scheme(&config).unwrap();
        assert_eq!(scheme.key.as_deref(), Some("PP"));
    }

    #[test]
    fn resolve_scheme_falls_back_to_the_deprecated_prefix_key() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{project}-{number:04d}"),
            ("work.default_project_code", "PP"),
        ]);
        let scheme = resolve_scheme(&config).unwrap();
        assert_eq!(scheme.key.as_deref(), Some("PP"));
    }

    #[test]
    fn resolve_scheme_requires_work_key_when_the_pattern_uses_the_prefix() {
        let config =
            FakeConfig::with(&[("work.id_pattern", "{key}-{number:04d}")]);
        let error = resolve_scheme(&config).unwrap_err();
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
        let scheme = resolve_scheme(&config).unwrap();
        assert_eq!(scheme.key.as_deref(), Some("PP"));
    }

    #[test]
    fn resolve_scheme_gates_the_prefix_field_on_a_bare_numeric_pattern() {
        let config = FakeConfig::with(&[
            ("work.id_pattern", "{number:04d}"),
            ("work.key", "PP"),
        ]);
        let scheme = resolve_scheme(&config).unwrap();
        assert_eq!(scheme.key, None);
    }
}
