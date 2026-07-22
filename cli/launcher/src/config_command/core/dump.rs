//! The `dump` view: every catalogue key with its effective value and source
//! attribution, plus the ad-hoc integration keys.
//!
//! Source is decided by *presence* at a level, not by value — a key set to its
//! default string still attributes to the level that set it. Credential keys
//! render as `*(set — hidden)*`.

use config::{
    catalogue, ConfigAccess, ConfigError, Key, Level, ReadConfigLevel, Resolved,
};

/// The value cell of a dump row.
pub enum Cell {
    /// A concrete value, rendered in backticks.
    Value(String),
    /// A set credential, rendered as `*(set — hidden)*`.
    Hidden,
    /// An unset key, rendered as `*(not set)*`.
    NotSet,
}

/// Where a key's value came from.
#[derive(Clone, Copy)]
pub enum Source {
    Team,
    Local,
    Default,
}

pub struct Row {
    pub key: String,
    pub cell: Cell,
    pub source: Source,
}

/// The dump rows, or `None` when no config file exists at all (the reader emits
/// nothing then).
///
/// # Errors
///
/// A [`ConfigError`] when a config level cannot be read.
pub fn assemble(
    config: &dyn ConfigAccess,
    levels: &dyn ReadConfigLevel,
) -> Result<Option<Vec<Row>>, ConfigError> {
    let has_config = levels.read(Level::Team)?.is_some()
        || levels.read(Level::Personal)?.is_some();
    if !has_config {
        return Ok(None);
    }
    let mut rows = Vec::new();
    for (key, _) in catalogue::REVIEW_KEYS {
        rows.push(defaulted_row(config, key)?);
    }
    for name in catalogue::AGENT_KEYS {
        rows.push(defaulted_row(config, &format!("agents.{name}"))?);
    }
    for (key, _) in catalogue::PATH_KEYS {
        rows.push(defaulted_row(config, key)?);
    }
    for key in catalogue::TEMPLATE_KEYS {
        rows.push(optional_row(config, key)?);
    }
    for (key, _) in catalogue::WORK_KEYS {
        rows.push(work_row(config, key)?);
    }
    for (key, _) in catalogue::VISUALISER_KEYS {
        rows.push(defaulted_row(config, key)?);
    }
    for key in catalogue::EXTRA_KEYS {
        rows.push(extra_row(config, key)?);
    }
    Ok(Some(rows))
}

fn config_get(
    config: &dyn ConfigAccess,
    key: &str,
    level: Option<Level>,
) -> Result<Option<String>, ConfigError> {
    let parsed = Key::parse(key)?;
    Ok(match config.get(&parsed, level)? {
        Resolved::Found(value) => Some(config::render_value(&value)),
        Resolved::Absent => None,
    })
}

fn source_of(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Source, ConfigError> {
    let parsed = Key::parse(key)?;
    Ok(match config.effective(&parsed, None)?.source() {
        config::Source::Personal => Source::Local,
        config::Source::Team => Source::Team,
        config::Source::Catalogue | config::Source::Unset => Source::Default,
    })
}

fn defaulted_row(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Row, ConfigError> {
    let value = config.effective(&Key::parse(key)?, None)?.rendered();
    Ok(Row {
        key: key.to_owned(),
        cell: Cell::Value(value),
        source: source_of(config, key)?,
    })
}

fn optional_row(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Row, ConfigError> {
    match config_get(config, key, None)? {
        Some(value) if !value.is_empty() => Ok(Row {
            key: key.to_owned(),
            cell: Cell::Value(value),
            source: source_of(config, key)?,
        }),
        _ => Ok(Row {
            key: key.to_owned(),
            cell: Cell::NotSet,
            source: Source::Default,
        }),
    }
}

fn work_row(config: &dyn ConfigAccess, key: &str) -> Result<Row, ConfigError> {
    let value = config.effective(&Key::parse(key)?, None)?.rendered();
    if value.is_empty() {
        return Ok(Row {
            key: key.to_owned(),
            cell: Cell::NotSet,
            source: source_of(config, key)?,
        });
    }
    let display = if key == "work.integration"
        && !catalogue::WORK_INTEGRATION_VALUES.contains(&value.as_str())
    {
        format!(
            "{value} (invalid: must be {})",
            catalogue::WORK_INTEGRATION_VALUES.join(", ")
        )
    } else {
        value
    };
    Ok(Row {
        key: key.to_owned(),
        cell: Cell::Value(display),
        source: source_of(config, key)?,
    })
}

fn extra_row(config: &dyn ConfigAccess, key: &str) -> Result<Row, ConfigError> {
    let Some(value) = config_get(config, key, None)?.filter(|v| !v.is_empty())
    else {
        return Ok(Row {
            key: key.to_owned(),
            cell: Cell::NotSet,
            source: Source::Default,
        });
    };
    let leaf = key.rsplit('.').next().unwrap_or(key);
    let cell = if leaf == "token" || leaf == "token_cmd" {
        Cell::Hidden
    } else {
        Cell::Value(value)
    };
    Ok(Row {
        key: key.to_owned(),
        cell,
        source: source_of(config, key)?,
    })
}
