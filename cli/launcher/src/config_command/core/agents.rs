//! The `agents` view: the nine agent names resolved to their override or
//! `accelerator:<key>` default, plus any unrecognised keys the config carries.
//!
//! An empty configured value falls back to the default, matching the bash
//! reader; unrecognised keys are collected for a stderr warning and skipped.

use config::{
    catalogue, ConfigAccess, ConfigError, Key, Level, Node, ReadConfigLevel,
};

use crate::config_command::core::ScalarView;

/// The assembled agents view: the resolved names in catalogue order, and the
/// unrecognised keys found under `agents:`.
pub struct AgentsView {
    pub agents: Vec<Agent>,
    pub unknown: Vec<String>,
}

/// One resolved agent: its raw catalogue name (the renderer turns hyphens into
/// spaces for display) and its resolved value.
pub struct Agent {
    pub name: String,
    pub value: String,
}

/// # Errors
///
/// A [`ConfigError`] when a config level cannot be read or parsed.
pub fn assemble(
    config: &dyn ConfigAccess,
    levels: &dyn ReadConfigLevel,
) -> Result<AgentsView, ConfigError> {
    let mut agents = Vec::new();
    for name in catalogue::AGENT_KEYS {
        let key = Key::parse(&format!("agents.{name}"))?;
        agents.push(Agent {
            name: (*name).to_owned(),
            value: config.effective_nonempty(&key, None)?.rendered(),
        });
    }
    Ok(AgentsView {
        agents,
        unknown: unknown_keys(levels)?,
    })
}

/// Resolves a single `agents.<name>` to its config override.
///
/// An explicit-empty value coalesces to the prefixed default, as does any name
/// with no config override — including one outside `AGENT_KEYS`, which carries
/// no catalogue default.
///
/// # Errors
///
/// A [`ConfigError`] when the name is malformed or a config level cannot be
/// read.
pub fn resolve(
    config: &dyn ConfigAccess,
    name: &str,
) -> Result<ScalarView, ConfigError> {
    let key = Key::parse(&format!("agents.{name}"))?;
    let value = config
        .effective_nonempty(&key, None)?
        .configured_value()
        .unwrap_or_else(|| format!("{}{name}", catalogue::AGENT_PREFIX));
    Ok(ScalarView {
        value,
        warnings: Vec::new(),
    })
}

fn unknown_keys(
    levels: &dyn ReadConfigLevel,
) -> Result<Vec<String>, ConfigError> {
    let mut seen: Vec<String> = Vec::new();
    for level in [Level::Team, Level::Personal] {
        let Some(Node::Mapping(root)) = levels.read(level)? else {
            continue;
        };
        let Some(Node::Mapping(section)) = root.get("agents") else {
            continue;
        };
        for (key, _) in section.entries() {
            if !catalogue::AGENT_KEYS.contains(&key.as_str())
                && !seen.contains(key)
            {
                seen.push(key.clone());
            }
        }
    }
    Ok(seen)
}
