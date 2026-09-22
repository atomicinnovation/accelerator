//! The `config get` view.
//!
//! A raw key resolves personal-over-team, then a non-empty `--default`, then —
//! only when resolving across levels — the built-in catalogue default, else
//! empty. A single-level read applies no built-in default.

use config::{catalogue, ConfigAccess, ConfigError, Key, Level, Resolved};

use crate::config_command::core::{explain_lines, ScalarView};

/// Resolves a raw key verbatim (no section prefix).
///
/// On a miss a non-empty `--default` wins; otherwise, when resolving across
/// levels, the built-in catalogue default applies, and a single-level read
/// yields empty.
///
/// # Errors
///
/// A [`ConfigError`] when the key is malformed or a config level cannot be read.
pub fn resolve(
    config: &dyn ConfigAccess,
    raw_key: &str,
    default: Option<&str>,
    level: Option<Level>,
    explain: bool,
) -> Result<ScalarView, ConfigError> {
    let key = Key::parse(raw_key)?;
    let value = match config.get(&key, level)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => match default.filter(|value| !value.is_empty()) {
            Some(explicit) => explicit.to_owned(),
            None if level.is_none() => catalogue::default_for(raw_key)
                .map(|value| config::render_value(&value))
                .unwrap_or_default(),
            None => String::new(),
        },
    };
    Ok(ScalarView {
        value,
        warnings: explain_lines(config, &key, level, explain)?,
    })
}
