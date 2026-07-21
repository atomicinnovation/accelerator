//! The `summary` view assembly: a brief description of the active
//! configuration for the `SessionStart` hook.
//!
//! Returns `None` when there is no config and the repo is already initialised —
//! the state the hook injects nothing for. The init sentinel resolves against
//! the project root, not the caller's CWD (a bash defect this fixes).

use config::{
    ConfigAccess, ConfigError, Key, Level, Node, ReadConfigLevel, ReadContent,
    ReadLensCatalogue, Resolved,
};

use crate::config_command::core::context::trim_body;

const INIT_HINT: &str = "Accelerator has not been initialised in this \
repository. Type /accelerator:init at the prompt to set up the expected \
directory structure and gitignore entries.";

const TRAILER: &str = "Skills will read this configuration at invocation \
time. To view or edit configuration, use /accelerator:configure.";

/// # Errors
///
/// A [`ConfigError`] when a config level, body, or customisation directory
/// cannot be read.
pub fn assemble(
    config: &dyn ConfigAccess,
    levels: &dyn ReadConfigLevel,
    content: &dyn ReadContent,
    enumeration: &dyn ReadLensCatalogue,
) -> Result<Option<String>, ConfigError> {
    let team = levels.read(Level::Team)?;
    let personal = levels.read(Level::Personal)?;
    let initialised = enumeration.init_sentinel_present(&tmp_dir(config)?)?;

    if team.is_none() && personal.is_none() {
        return Ok(if initialised {
            None
        } else {
            Some(INIT_HINT.to_owned())
        });
    }

    let mut summary =
        String::from("Accelerator plugin configuration detected:");
    if team.is_some() {
        summary.push_str("\n- Team config: .accelerator/config.md");
    }
    if personal.is_some() {
        summary.push_str("\n- Personal config: .accelerator/config.local.md");
    }

    let sections = configured_sections(team.as_ref(), personal.as_ref());
    if !sections.is_empty() {
        summary.push_str("\n- Configured sections:");
        for section in sections {
            summary.push(' ');
            summary.push_str(&section);
        }
    }

    if has_project_context(content)? {
        summary.push_str(
            "\n- Project context: provided (will be injected into skills)",
        );
    }

    let customisations = skill_customisations(content, enumeration)?;
    if !customisations.is_empty() {
        summary.push_str("\n- Per-skill customisations:");
        for line in customisations {
            summary.push_str("\n    - ");
            summary.push_str(&line);
        }
    }

    summary.push_str("\n\n");
    summary.push_str(TRAILER);
    if !initialised {
        summary.push_str("\n\n");
        summary.push_str(INIT_HINT);
    }
    Ok(Some(summary))
}

fn tmp_dir(config: &dyn ConfigAccess) -> Result<String, ConfigError> {
    let key = Key::parse("paths.tmp")?;
    Ok(match config.get(&key, None)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => ".accelerator/tmp".to_owned(),
    })
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
) -> Result<Vec<String>, ConfigError> {
    let mut lines = Vec::new();
    for name in enumeration.skill_names()? {
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
