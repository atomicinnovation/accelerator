//! Writes the discovered base team's key into `shortcut.team_key`.
//!
//! An absent key is written. A different key already set is kept unless the
//! caller forces it: init never prompts, so a skill asks the user and re-runs
//! with `--force`.

use config::ConfigAccess;
use config::ConfigError;
use config::Key;
use config::Level;
use config::Resolved;

const TEAM_KEY: &str = "shortcut.team_key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Writeback {
    Written,
    Unchanged,
    Overwritten,
    LeftIntact { existing: String },
}

/// # Errors
///
/// [`ConfigError`] when the team level cannot be read or written.
pub fn write_team_key(
    config: &dyn ConfigAccess,
    discovered: &str,
    force: bool,
) -> Result<Writeback, ConfigError> {
    let key = Key::parse(TEAM_KEY)?;
    let existing = match config.get(&key, Some(Level::Team))? {
        Resolved::Found(value) => Some(config::render_value(&value)),
        Resolved::Absent => None,
    };
    match existing {
        None => {
            config.set(&key, discovered, Level::Team)?;
            Ok(Writeback::Written)
        }
        Some(current) if current.is_empty() => {
            config.set(&key, discovered, Level::Team)?;
            Ok(Writeback::Written)
        }
        Some(current) if current == discovered => Ok(Writeback::Unchanged),
        Some(_) if force => {
            config.set(&key, discovered, Level::Team)?;
            Ok(Writeback::Overwritten)
        }
        Some(existing) => Ok(Writeback::LeftIntact { existing }),
    }
}
