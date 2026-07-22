//! The `config work` view: a `work.<key>` value resolved to config, the
//! catalogue default, or empty (with a stderr warning for an unknown key).
//!
//! `work.integration` is validated fail-closed: a non-empty value outside the
//! allow-set is a [`ConfigError::Invalid`] refusal, never a degraded empty.

use config::{catalogue, ConfigAccess, ConfigError, Key, Resolved};

use crate::config_command::core::ScalarView;

/// Resolves `work.<key>`, refusing an out-of-set `work.integration` value.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when `work.integration` carries an unrecognised
/// value; a [`ConfigError`] when the key is malformed or a level cannot be read.
pub fn resolve(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<ScalarView, ConfigError> {
    let full = format!("work.{key}");
    let parsed = Key::parse(&full)?;
    let mut warnings = Vec::new();
    let fallback = work_fallback(key, &full, &mut warnings);
    let value = match config.get(&parsed, None)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => fallback,
    };
    if key == "integration" && !catalogue::is_valid_work_integration(&value) {
        return Err(bad_integration(&value));
    }
    Ok(ScalarView { value, warnings })
}

/// The default a `work` miss falls back to: the catalogue default, else empty
/// with a stderr warning naming the unrecognised key.
fn work_fallback(
    key: &str,
    full_key: &str,
    warnings: &mut Vec<String>,
) -> String {
    if let Some(value) = catalogue::default_for(full_key) {
        return config::render_value(&value);
    }
    warnings.push(unknown_work_key_warning(key));
    String::new()
}

fn unknown_work_key_warning(key: &str) -> String {
    format!("Warning: unknown key 'work.{key}' — no centralized default")
}

fn bad_integration(value: &str) -> ConfigError {
    let allowed = catalogue::WORK_INTEGRATION_VALUES.join(", ");
    ConfigError::Invalid {
        detail: format!(
            "work.integration must be one of: {allowed} (got '{value}'). \
             Update work.integration in .accelerator/config.md or run \
             '/accelerator:configure view' to inspect the current value."
        ),
    }
}
