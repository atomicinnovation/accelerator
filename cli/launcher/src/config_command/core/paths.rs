//! The `paths` view assembly: the configured path keys, and the 13 doc-type →
//! directory mappings with their fail-closed value hardening.

use config::{catalogue, ConfigAccess, ConfigError, Key, Level, Resolved};

use crate::config_command::core::ScalarView;

/// A configured path row: the bare key and its resolved (or default) value.
pub struct ConfiguredPath {
    pub key: String,
    pub value: String,
}

/// A doc-type path key that was blank in config and fell back to its default.
pub struct BlankDefault {
    pub path_key: String,
    pub default: String,
}

/// The doc-type resolutions and the blank-coercion facts the renderer turns
/// into stderr notes.
pub struct DocTypes {
    pub rows: Vec<(String, String)>,
    pub blanks: Vec<BlankDefault>,
}

/// Non-document keys excluded from the configured-paths block unless `all`.
const EXCLUDED: &[&str] = &["tmp", "templates", "integrations"];

/// # Errors
///
/// A [`ConfigError`] when a config level cannot be read.
pub fn configured(
    config: &dyn ConfigAccess,
    all: bool,
) -> Result<Vec<ConfiguredPath>, ConfigError> {
    let mut paths = Vec::new();
    for (full_key, _) in catalogue::PATH_KEYS {
        let key = full_key.strip_prefix("paths.").unwrap_or(full_key);
        if !all && EXCLUDED.contains(&key) {
            continue;
        }
        paths.push(ConfiguredPath {
            key: key.to_owned(),
            value: resolve_or_default(config, full_key)?,
        });
    }
    Ok(paths)
}

/// Resolves a single `paths.<key>`, prefixing the `paths.` section.
///
/// `raw_key` must name a path the catalogue recognises — a key with no
/// catalogue entry is refused, whether or not it happens to carry a value in
/// config. On a miss for a recognised key, an explicit non-empty `--default`
/// wins over the catalogue default.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when `raw_key` has no catalogue entry. A
/// [`ConfigError`] when the key is malformed or a config level cannot be read.
pub fn resolve(
    config: &dyn ConfigAccess,
    raw_key: &str,
    default: Option<&str>,
    level: Option<Level>,
    explain: bool,
) -> Result<ScalarView, ConfigError> {
    let full = format!("paths.{raw_key}");
    let key = Key::parse(&full)?;
    if catalogue::default_for(&full).is_none() {
        return Err(unknown_path_key_error(raw_key));
    }
    let mut warnings = Vec::new();
    warnings.extend(legacy_alias_warning(config, raw_key)?);
    let value =
        if let Some(explicit) = default.filter(|value| !value.is_empty()) {
            match config.get(&key, level)? {
                Resolved::Found(value) => config::render_value(&value),
                Resolved::Absent => explicit.to_owned(),
            }
        } else {
            config::paths::resolve_with_fallback(config, raw_key, level)?
        };
    warnings.extend(super::explain_lines(config, &key, level, explain)?);
    Ok(ScalarView { value, warnings })
}

/// The migration-0004 nudge when a canonical `research_design_*` key is read
/// while its pre-rename alias carries a value in config that is being ignored.
fn legacy_alias_warning(
    config: &dyn ConfigAccess,
    raw_key: &str,
) -> Result<Option<String>, ConfigError> {
    let legacy = match raw_key {
        "research_design_inventories" => "design_inventories",
        "research_design_gaps" => "design_gaps",
        _ => return Ok(None),
    };
    let key = Key::parse(&format!("paths.{legacy}"))?;
    let set = match config.get(&key, None)? {
        Resolved::Found(value) => !config::render_value(&value).is_empty(),
        Resolved::Absent => false,
    };
    Ok(set.then(|| {
        format!(
            "Warning: your config sets 'paths.{legacy}' (renamed by migration \
             0004 to 'paths.{raw_key}'); the legacy override is being \
             ignored. Run /accelerator:migrate"
        )
    }))
}

/// The refusal for an unrecognised `paths.<key>` lookup — a distinct,
/// actionable message for the two keys migration 0004 renamed, else the
/// generic catalogue-membership refusal.
fn unknown_path_key_error(key: &str) -> ConfigError {
    match key {
        "design_inventories" | "design_gaps" => ConfigError::Invalid {
            detail: format!(
                "key '{key}' was renamed by migration 0004 to \
                 'research_{key}'; run /accelerator:migrate"
            ),
        },
        _ => config::paths::unknown_path_key_error(key),
    }
}

/// Resolves each doc-type's configured directory.
///
/// Coerces a blank value to the registry default (with a note) and refuses an
/// unsafe or tab/newline-bearing value. Buffers every row: on a refusal,
/// nothing is emitted.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when a resolved directory is unsafe or carries a
/// tab or newline; a [`ConfigError`] when a config level cannot be read.
pub fn doc_types(config: &dyn ConfigAccess) -> Result<DocTypes, ConfigError> {
    let mut rows = Vec::new();
    let mut blanks = Vec::new();
    for resolved in config::paths::doc_type_dirs(config)? {
        if resolved.blank_fallback {
            blanks.push(BlankDefault {
                path_key: resolved.path_key.to_owned(),
                default: resolved.dir.clone(),
            });
        }
        rows.push((resolved.doc_type.to_owned(), resolved.dir));
    }
    Ok(DocTypes { rows, blanks })
}

fn resolve_or_default(
    config: &dyn ConfigAccess,
    full_key: &str,
) -> Result<String, ConfigError> {
    Ok(config.effective(&Key::parse(full_key)?, None)?.rendered())
}
