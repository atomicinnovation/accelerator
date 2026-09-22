//! Reading a `<tracker>.<sub>` config block down to the value that configured
//! it and the level it resolved from — the shared front half of every
//! per-tracker block reader.

use config::ConfigAccess;
use config::Key;
use config::Level;
use config::Resolved;
use config::Source;
use config::Value;

/// The configured value of the `<integration>.<sub_key>` block and the config
/// level it resolved from, or `None` when the block is unset or an empty
/// mapping — each meaning the built-in defaults apply.
///
/// Reads the already-composed config representation, so it performs no
/// filesystem or subprocess access.
///
/// # Errors
///
/// A config-access failure, rendered as its display string.
pub fn read_block(
    config: &dyn ConfigAccess,
    integration: &str,
    sub_key: &str,
) -> Result<Option<(Value, Level)>, String> {
    let key = Key::parse(&format!("{integration}.{sub_key}"))
        .map_err(|error| error.to_string())?;
    let Resolved::Found(value) =
        config.get(&key, None).map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    if matches!(&value, Value::Mapping(entries) if entries.is_empty()) {
        return Ok(None);
    }
    let level = match config
        .effective(&key, None)
        .map_err(|error| error.to_string())?
        .source()
    {
        Source::Personal => Level::Personal,
        _ => Level::Team,
    };
    Ok(Some((value, level)))
}
