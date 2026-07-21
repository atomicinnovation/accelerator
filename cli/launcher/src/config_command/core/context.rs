//! The `context` and `instructions` view assembly.
//!
//! Assembles the project-context body joined across levels, and a skill's
//! context/instructions content, each trimmed of surrounding blank lines. The
//! `--skill` name is validated here, inside the fail-safe boundary, never by a
//! clap value parser.

use config::{ConfigError, Level, ReadContent};

/// Which per-skill customisation file a skill section reads.
#[derive(Clone, Copy)]
pub enum SkillFile {
    Context,
    Instructions,
}

/// The project-context body: the trimmed markdown bodies of the team then
/// personal config files, joined by one blank line; `None` when both are empty.
///
/// # Errors
///
/// A [`ConfigError`] when a config level's body cannot be read.
pub fn project_body(
    content: &dyn ReadContent,
) -> Result<Option<String>, ConfigError> {
    let mut parts = Vec::new();
    for level in [Level::Team, Level::Personal] {
        if let Some(raw) = content.config_body(level)? {
            let trimmed = trim_body(&raw);
            if !trimmed.is_empty() {
                parts.push(trimmed);
            }
        }
    }
    Ok(if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    })
}

/// A skill's trimmed customisation content; `None` when the file is absent or
/// whitespace-only.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when the skill name is not a valid identifier, or a
/// [`ConfigError`] when the file cannot be read.
pub fn skill_body(
    content: &dyn ReadContent,
    skill: &str,
    file: SkillFile,
) -> Result<Option<String>, ConfigError> {
    validate_skill_name(skill)?;
    let raw = match file {
        SkillFile::Context => content.skill_context(skill)?,
        SkillFile::Instructions => content.skill_instructions(skill)?,
    };
    Ok(raw
        .map(|content| trim_body(&content))
        .filter(|trimmed| !trimmed.is_empty()))
}

/// Strips leading and trailing blank lines, preserving interior blanks, matching
/// bash `config_trim_body`.
#[must_use]
pub fn trim_body(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let Some(start) = lines.iter().position(|line| !line.trim().is_empty())
    else {
        return String::new();
    };
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .unwrap_or(start);
    lines[start..=end].join("\n")
}

/// Refuses a skill name that is not `^[a-z0-9][a-z0-9-]*$`, so a traversing name
/// can never reach a path.
fn validate_skill_name(skill: &str) -> Result<(), ConfigError> {
    let mut chars = skill.chars();
    let head_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let tail_ok =
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if head_ok && tail_ok {
        Ok(())
    } else {
        Err(ConfigError::Invalid {
            detail: format!("invalid skill name '{skill}'"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::trim_body;

    #[test]
    fn trims_leading_and_trailing_blank_lines_keeping_interior() {
        assert_eq!(trim_body("\n\n  \nfirst\n\nlast\n \n\n"), "first\n\nlast");
    }

    #[test]
    fn a_whitespace_only_body_trims_to_empty() {
        assert_eq!(trim_body("\n  \n\t\n"), "");
    }
}
