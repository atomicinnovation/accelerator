//! The `template` / `templates list` / `templates show` view assembly.
//!
//! Resolves a template name through the three tiers (configured path, user
//! override directory, plugin default), validating the name as an identifier
//! before it reaches a path.

use config::{
    ConfigAccess, ConfigError, Key, ReadTemplate, Resolved, ResolvedTemplate,
};

/// One row of the `templates list` table.
pub struct ListRow {
    pub key: String,
    pub source: String,
    pub display_path: String,
}

/// Resolves a template by name, or `None` when it is not found in any tier.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when the name is not an identifier, or a
/// [`ConfigError`] when a config value or candidate file cannot be read.
pub fn resolve(
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
    name: &str,
) -> Result<Option<ResolvedTemplate>, ConfigError> {
    validate_name(name)?;
    let config_path = scalar(config, &format!("templates.{name}"))?;
    let dir = scalar(config, "paths.templates")?
        .unwrap_or_else(|| ".accelerator/templates".to_owned());
    templates.resolve_template(name, config_path.as_deref(), &dir)
}

/// The `templates list` rows for every available template.
///
/// # Errors
///
/// A [`ConfigError`] when a config value or candidate file cannot be read.
pub fn list(
    config: &dyn ConfigAccess,
    templates: &dyn ReadTemplate,
) -> Result<Vec<ListRow>, ConfigError> {
    let mut rows = Vec::new();
    for key in templates.template_names() {
        let (source, display_path) = match resolve(config, templates, &key)? {
            Some(resolved) => {
                (resolved.source.label().to_owned(), resolved.display_path)
            }
            None => ("not found".to_owned(), "—".to_owned()),
        };
        rows.push(ListRow {
            key,
            source,
            display_path,
        });
    }
    Ok(rows)
}

/// The comma-joined available names, for the not-found diagnostic.
#[must_use]
pub fn available(templates: &dyn ReadTemplate) -> String {
    templates.template_names().join(", ")
}

fn scalar(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Option<String>, ConfigError> {
    let parsed = Key::parse(key)?;
    Ok(match config.get(&parsed, None)? {
        Resolved::Found(value) => {
            let rendered = config::render_value(&value);
            (!rendered.is_empty()).then_some(rendered)
        }
        Resolved::Absent => None,
    })
}

fn validate_name(name: &str) -> Result<(), ConfigError> {
    let mut chars = name.chars();
    let head_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let tail_ok =
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if head_ok && tail_ok {
        Ok(())
    } else {
        Err(ConfigError::Invalid {
            detail: format!("invalid template name '{name}'"),
        })
    }
}
