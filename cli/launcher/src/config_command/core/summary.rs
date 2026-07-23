//! The `summary` view assembly: a brief description of the active
//! configuration for the `SessionStart` hook.
//!
//! Returns `None` when there is no config and the repo is already initialised —
//! the state the hook injects nothing for. The init sentinel resolves against
//! the project root, not the caller's CWD (a bash defect this fixes).

use config::{
    ConfigAccess, ConfigError, Key, Level, Node, ReadConfigLevel, ReadContent,
    ReadLensCatalogue,
};

use crate::config_command::core::context::trim_body;

/// The summary state the renderer turns into text (or nothing).
pub enum Summary {
    /// No config and the repo is initialised — the hook injects nothing.
    Nothing,
    /// No config and the repo is not initialised — the init hint.
    NotInitialised,
    /// Config is present.
    Configured(SummaryView),
}

/// The facts a configured-summary body is built from.
pub struct SummaryView {
    pub present_levels: Vec<Level>,
    pub configured_sections: Vec<String>,
    pub has_project_context: bool,
    pub customisations: Vec<String>,
    pub initialised: bool,
}

/// # Errors
///
/// A [`ConfigError`] when a config level, body, or customisation directory
/// cannot be read.
pub fn assemble(
    config: &dyn ConfigAccess,
    levels: &dyn ReadConfigLevel,
    content: &dyn ReadContent,
    enumeration: &dyn ReadLensCatalogue,
) -> Result<(Summary, Vec<String>), ConfigError> {
    let team = levels.read(Level::Team)?;
    let personal = levels.read(Level::Personal)?;
    let initialised = enumeration.init_sentinel_present(&tmp_dir(config)?)?;

    if team.is_none() && personal.is_none() {
        let summary = if initialised {
            Summary::Nothing
        } else {
            Summary::NotInitialised
        };
        return Ok((summary, Vec::new()));
    }
    let mut warnings = Vec::new();
    let mut present_levels = Vec::new();
    if team.is_some() {
        present_levels.push(Level::Team);
    }
    if personal.is_some() {
        present_levels.push(Level::Personal);
    }
    let configured_sections =
        configured_sections(team.as_ref(), personal.as_ref());
    let has_project_context = has_project_context(content)?;
    let customisations =
        skill_customisations(content, enumeration, &mut warnings)?;
    Ok((
        Summary::Configured(SummaryView {
            present_levels,
            configured_sections,
            has_project_context,
            customisations,
            initialised,
        }),
        warnings,
    ))
}

fn tmp_dir(config: &dyn ConfigAccess) -> Result<String, ConfigError> {
    Ok(config
        .effective(&Key::parse("paths.tmp")?, None)?
        .rendered())
}

fn configured_sections(
    team: Option<&Node>,
    personal: Option<&Node>,
) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for document in [team, personal] {
        for key in top_level_keys(document) {
            if !seen.contains(&key) {
                seen.push(key);
            }
        }
    }
    seen
}

fn top_level_keys(document: Option<&Node>) -> Vec<String> {
    let Some(Node::Mapping(mapping)) = document else {
        return Vec::new();
    };
    let mut keys: Vec<String> = mapping
        .entries()
        .iter()
        .map(|(key, _)| key.clone())
        .filter(|key| is_section_key(key))
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

fn is_section_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn has_project_context(content: &dyn ReadContent) -> Result<bool, ConfigError> {
    for level in [Level::Team, Level::Personal] {
        if let Some(body) = content.config_body(level)? {
            if !trim_body(&body).is_empty() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn skill_customisations(
    content: &dyn ReadContent,
    enumeration: &dyn ReadLensCatalogue,
    warnings: &mut Vec<String>,
) -> Result<Vec<String>, ConfigError> {
    let known = enumeration.known_skill_names()?;
    let mut lines = Vec::new();
    for name in enumeration.skill_names()? {
        if !known.is_empty() && !known.contains(&name) {
            warnings.push(format!(
                "Warning: .accelerator/skills/{name}/ does not match any \
                 known skill name. Valid names: {}",
                known.join(" ")
            ));
        }
        let has_context = content
            .skill_context(&name)?
            .is_some_and(|body| !trim_body(&body).is_empty());
        let has_instructions = content
            .skill_instructions(&name)?
            .is_some_and(|body| !trim_body(&body).is_empty());
        let types = match (has_context, has_instructions) {
            (true, true) => "context + instructions",
            (true, false) => "context",
            (false, true) => "instructions",
            (false, false) => continue,
        };
        lines.push(format!("{name} ({types})"));
    }
    Ok(lines)
}
