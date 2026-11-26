//! The ladder a plain, non-secret setting climbs.
//!
//! The key's environment overrides come first, then `config.local.md`, then
//! `config.md`. The first non-blank value wins, and the shared file is read
//! whenever no higher rung holds one. Unlike the credential ladder, a chosen
//! value is never refused and never falls through: judging it is the
//! caller's business.

use std::fmt;

use crate::catalogue;
use crate::catalogue::ExtraKey;
use crate::catalogue::Trust;
use crate::consent;
use crate::consent::Environment;
use crate::error::ConfigError;
use crate::service::ConfigAccess;
use crate::Level;

/// A key anyone may set and that holds no secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenKey(&'static ExtraKey);

impl OpenKey {
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] unless the catalogue declares `name` as an
    /// open key that is not a credential.
    pub fn declared(name: &str) -> Result<Self, ConfigError> {
        catalogue::declared(name)
            .filter(|key| key.trust == Trust::Open && !key.credential)
            .map(Self)
            .ok_or_else(|| ConfigError::Invalid {
                detail: format!("{name} is not a declared open setting"),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingSource {
    Environment(&'static str),
    Level(Level),
}

impl fmt::Display for SettingSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Environment(variable) => variable,
            Self::Level(Level::Personal) => "config.local.md",
            Self::Level(Level::Team) => "config.md",
        })
    }
}

/// An open setting's trimmed value and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingValue {
    pub value: String,
    pub source: SettingSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableLevel {
    pub level: Level,
    pub reason: ConfigError,
}

/// The first non-blank value of `key`, or `None` when no rung holds one.
///
/// # Errors
///
/// [`UnreadableLevel`] when a level the ladder reaches cannot be read.
pub fn open_value(
    environment: &dyn Environment,
    config: &dyn ConfigAccess,
    key: &OpenKey,
) -> Result<Option<SettingValue>, UnreadableLevel> {
    if let Some((variable, value)) =
        consent::environment_candidate(environment, key.0)
    {
        return Ok(Some(SettingValue {
            value,
            source: SettingSource::Environment(variable),
        }));
    }
    for level in [Level::Personal, Level::Team] {
        let value = consent::raw_value(config, key.0, level)
            .map_err(|reason| UnreadableLevel { level, reason })?;
        if let Some(value) = value {
            return Ok(Some(SettingValue {
                value,
                source: SettingSource::Level(level),
            }));
        }
    }
    Ok(None)
}
