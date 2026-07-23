//! The `config get` view: a raw key resolved to its config value, the
//! caller-supplied `--default`, or empty — never the catalogue default.

use config::{ConfigAccess, ConfigError, Key, Level, Resolved};

use crate::config_command::core::{explain_lines, ScalarView};

/// Resolves a raw key verbatim (no section prefix). On a miss it returns the
/// caller's `--default` or empty; the catalogue is never consulted.
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
        Resolved::Absent => default.unwrap_or_default().to_owned(),
    };
    Ok(ScalarView {
        value,
        warnings: explain_lines(config, &key, level, explain)?,
    })
}
