//! The deprecation-alias resolver: a scoped `canonical ?? deprecated` fallback
//! that carries a legacy config value across a one-release deprecation window.
//!
//! It is not a generic config-layer feature — it exists only to migrate a
//! renamed key. The removal release is a single internal constant, so no call
//! site can pass a divergent value.

use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::error::ConfigError;
use crate::key::Key;
use crate::service::ConfigAccess;

/// The release in which the deprecated aliases and the `{project}` token are
/// removed. Named explicitly in every deprecation warning.
pub const REMOVAL_RELEASE: &str = "1.25.0";

/// A value resolved through the deprecation alias, plus the deprecation warning
/// its resolution earned (when a legacy key was involved).
pub struct AliasedScalar {
    pub value: Option<String>,
    pub deprecation: Option<String>,
}

fn deprecation_warning(deprecated: &str) -> String {
    format!(
        "Warning: '{deprecated}' is deprecated and will be removed in \
         {REMOVAL_RELEASE}. Run /accelerator:migrate to materialise the \
         replacement configuration keys."
    )
}

fn read_nonempty(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<String, ConfigError> {
    let parsed = Key::parse(key)?;
    Ok(config.effective_nonempty(&parsed, None)?.rendered())
}

/// Resolves `canonical`, falling back to `deprecated` across the deprecation
/// window.
///
/// A non-empty `canonical` always wins; a `deprecated` value present alongside
/// it still earns a warning telling the user to remove it. When `canonical` is
/// absent, `deprecated` is claimed only when it is this key's to claim:
/// `expected_integration` is `None` (a tracker-independent key such as the ID
/// prefix), or it equals the effective `work.integration` (a gated scope key).
/// A mismatched integration yields no value and no warning — the legacy value
/// belongs to a different key.
///
/// # Errors
///
/// A [`ConfigError`] when any of the keys read fails to resolve.
pub fn resolve_with_deprecated_fallback(
    config: &dyn ConfigAccess,
    canonical: &str,
    deprecated: &str,
    expected_integration: Option<&str>,
) -> Result<AliasedScalar, ConfigError> {
    let canonical_value = read_nonempty(config, canonical)?;
    let deprecated_value = read_nonempty(config, deprecated)?;
    let deprecated_present = !deprecated_value.is_empty();

    let claimable = match expected_integration {
        None => true,
        Some(expected) => {
            read_nonempty(config, "work.integration")? == expected
        }
    };

    if !canonical_value.is_empty() {
        let deprecation = (deprecated_present && claimable)
            .then(|| deprecation_warning(deprecated));
        return Ok(AliasedScalar {
            value: Some(canonical_value),
            deprecation,
        });
    }

    if deprecated_present && claimable {
        return Ok(AliasedScalar {
            value: Some(deprecated_value),
            deprecation: Some(deprecation_warning(deprecated)),
        });
    }

    Ok(AliasedScalar {
        value: None,
        deprecation: None,
    })
}

/// Emits a deprecation warning to stderr at most once per process, deduped on
/// the deprecated key name.
///
/// A single legacy key read at several sites within one command yields one
/// warning. A `None` deprecation is a no-op.
pub fn emit_deprecation_once(deprecated_key: &str, deprecation: Option<&str>) {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let Some(message) = deprecation else {
        return;
    };
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut set) = seen.lock() {
        if set.insert(deprecated_key.to_owned()) {
            eprintln!("{message}");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashMap;

    use super::{resolve_with_deprecated_fallback, REMOVAL_RELEASE};
    use crate::error::ConfigError;
    use crate::key::Key;
    use crate::level::Level;
    use crate::node::Scalar;
    use crate::service::{ConfigAccess, Resolved, Value};

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
    fn canonical_wins_with_no_warning_when_no_legacy_present() {
        let config = FakeConfig::with(&[("work.key", "PP")]);
        let resolved = resolve_with_deprecated_fallback(
            &config,
            "work.key",
            "work.default_project_code",
            None,
        )
        .unwrap();
        assert_eq!(resolved.value.as_deref(), Some("PP"));
        assert_eq!(resolved.deprecation, None);
    }

    #[test]
    fn canonical_wins_but_a_lingering_legacy_key_still_warns() {
        let config = FakeConfig::with(&[
            ("work.key", "PP"),
            ("work.default_project_code", "PP"),
        ]);
        let resolved = resolve_with_deprecated_fallback(
            &config,
            "work.key",
            "work.default_project_code",
            None,
        )
        .unwrap();
        assert_eq!(resolved.value.as_deref(), Some("PP"));
        let warning = resolved.deprecation.expect("a warning is expected");
        assert!(warning.contains(REMOVAL_RELEASE), "{warning}");
        assert!(warning.contains("work.default_project_code"), "{warning}");
    }

    #[test]
    fn legacy_falls_back_with_a_removal_warning_for_a_prefix_key() {
        let config = FakeConfig::with(&[("work.default_project_code", "PP")]);
        let resolved = resolve_with_deprecated_fallback(
            &config,
            "work.key",
            "work.default_project_code",
            None,
        )
        .unwrap();
        assert_eq!(resolved.value.as_deref(), Some("PP"));
        assert!(resolved
            .deprecation
            .expect("a warning is expected")
            .contains(REMOVAL_RELEASE));
    }

    #[test]
    fn a_gated_scope_key_claims_the_legacy_value_on_a_matching_integration() {
        let config = FakeConfig::with(&[
            ("work.default_project_code", "PP"),
            ("work.integration", "jira"),
        ]);
        let resolved = resolve_with_deprecated_fallback(
            &config,
            "jira.project_key",
            "work.default_project_code",
            Some("jira"),
        )
        .unwrap();
        assert_eq!(resolved.value.as_deref(), Some("PP"));
        assert!(resolved.deprecation.is_some());
    }

    #[test]
    fn a_gated_scope_key_ignores_the_legacy_value_on_a_mismatch() {
        let config = FakeConfig::with(&[
            ("work.default_project_code", "PP"),
            ("work.integration", "linear"),
        ]);
        let resolved = resolve_with_deprecated_fallback(
            &config,
            "jira.project_key",
            "work.default_project_code",
            Some("jira"),
        )
        .unwrap();
        assert_eq!(resolved.value, None);
        assert_eq!(resolved.deprecation, None);
    }

    #[test]
    fn a_gated_scope_key_ignores_the_legacy_value_when_integration_absent() {
        let config = FakeConfig::with(&[("work.default_project_code", "PP")]);
        let resolved = resolve_with_deprecated_fallback(
            &config,
            "jira.project_key",
            "work.default_project_code",
            Some("jira"),
        )
        .unwrap();
        assert_eq!(resolved.value, None);
    }

    #[test]
    fn neither_key_present_yields_nothing() {
        let config = FakeConfig::with(&[]);
        let resolved = resolve_with_deprecated_fallback(
            &config,
            "work.key",
            "work.default_project_code",
            None,
        )
        .unwrap();
        assert_eq!(resolved.value, None);
        assert_eq!(resolved.deprecation, None);
    }

    #[test]
    fn removal_release_is_a_later_minor_than_the_crate_version() {
        fn minor(version: &str) -> (u64, u64) {
            let core = version.split('-').next().unwrap_or(version);
            let mut parts = core.split('.');
            let major = parts.next().unwrap().parse().unwrap();
            let minor = parts.next().unwrap().parse().unwrap();
            (major, minor)
        }
        let package = minor(env!("CARGO_PKG_VERSION"));
        let removal = minor(REMOVAL_RELEASE);
        assert!(
            removal > package,
            "REMOVAL_RELEASE {REMOVAL_RELEASE} must be a later minor than \
             {}",
            env!("CARGO_PKG_VERSION")
        );
    }
}
