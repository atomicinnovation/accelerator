//! The `template` / `templates list` / `templates show` view assembly.
//!
//! Resolves a template name through the three tiers (configured path, user
//! override directory, plugin default), validating the name as an identifier
//! before it reaches a path.

use config::{ConfigAccess, ConfigError, Key, ReadTemplate, ResolvedTemplate};

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
    let dir = templates_dir(config)?;
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

/// The comma-joined available names, or `(none found)` when there are none —
/// the form the `eject`/`diff`/`reset` error messages carry.
#[must_use]
pub fn available_or_none(templates: &dyn ReadTemplate) -> String {
    let names = available(templates);
    if names.is_empty() {
        "(none found)".to_owned()
    } else {
        names
    }
}

/// The configured user templates directory, or the built-in default.
///
/// # Errors
///
/// A [`ConfigError`] when the `paths.templates` value cannot be read.
pub fn templates_dir(config: &dyn ConfigAccess) -> Result<String, ConfigError> {
    Ok(config
        .effective_nonempty(&Key::parse("paths.templates")?, None)?
        .rendered())
}

/// Validates a template name as a lowercase identifier before it reaches a
/// filesystem path.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when the name is not an identifier.
pub fn validate(name: &str) -> Result<(), ConfigError> {
    validate_name(name)
}

fn scalar(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Option<String>, ConfigError> {
    Ok(config
        .effective_nonempty(&Key::parse(key)?, None)?
        .configured_value())
}

/// The line-diff between a plugin default and a user override.
pub enum TemplateDiff {
    /// The two contents are byte-identical.
    Identical,
    /// A line-oriented unified diff body.
    Unified(String),
}

/// Computes the diff between a plugin default and a user override.
#[must_use]
pub fn diff(default: &str, user: &str) -> TemplateDiff {
    if default == user {
        TemplateDiff::Identical
    } else {
        TemplateDiff::Unified(unified_diff(default, user))
    }
}

/// A line-oriented unified diff of `old` against `new`, common lines prefixed
/// with a space and changes with `-`/`+`, computed from the longest common
/// subsequence.
fn unified_diff(old: &str, new: &str) -> String {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let lcs = lcs_lengths(&a, &b);
    let mut out = String::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            push_line(&mut out, ' ', a[i]);
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            push_line(&mut out, '-', a[i]);
            i += 1;
        } else {
            push_line(&mut out, '+', b[j]);
            j += 1;
        }
    }
    while i < a.len() {
        push_line(&mut out, '-', a[i]);
        i += 1;
    }
    while j < b.len() {
        push_line(&mut out, '+', b[j]);
        j += 1;
    }
    out
}

fn push_line(out: &mut String, prefix: char, line: &str) {
    out.push(prefix);
    out.push_str(line);
    out.push('\n');
}

fn lcs_lengths(a: &[&str], b: &[&str]) -> Vec<Vec<usize>> {
    let mut lengths = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lengths[i][j] = if a[i] == b[j] {
                lengths[i + 1][j + 1] + 1
            } else {
                lengths[i + 1][j].max(lengths[i][j + 1])
            };
        }
    }
    lengths
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

#[cfg(test)]
mod tests {
    use super::{diff, TemplateDiff};

    #[test]
    fn identical_content_yields_identical() {
        assert!(matches!(diff("a\nb\n", "a\nb\n"), TemplateDiff::Identical));
    }

    #[test]
    fn a_changed_line_is_a_deletion_then_an_addition() {
        let body = match diff("a\nb\nc\n", "a\nx\nc\n") {
            TemplateDiff::Unified(body) => body,
            TemplateDiff::Identical => String::new(),
        };
        assert_eq!(body, " a\n-b\n+x\n c\n");
    }
}
