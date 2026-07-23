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
/// On a miss an explicit non-empty `--default` wins over the catalogue, which
/// wins over empty-plus-warning. The fallback is computed eagerly, so an
/// unknown-key warning can accompany a value that resolves from config.
///
/// # Errors
///
/// A [`ConfigError`] when the key is malformed or a config level cannot be read.
pub fn resolve(
    config: &dyn ConfigAccess,
    raw_key: &str,
    default: Option<&str>,
    level: Option<Level>,
    explain: bool,
) -> Result<ScalarView, ConfigError> {
    let full = format!("paths.{raw_key}");
    let key = Key::parse(&full)?;
    let mut warnings = Vec::new();
    warnings.extend(legacy_alias_warning(config, raw_key)?);
    let fallback = path_fallback(default, raw_key, &full, &mut warnings);
    let value = match config.get(&key, level)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => fallback,
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

/// The value a `path` miss falls back to: an explicit non-empty default wins,
/// else the catalogue default, else empty with a stderr warning naming the key.
fn path_fallback(
    default: Option<&str>,
    raw_key: &str,
    full_key: &str,
    warnings: &mut Vec<String>,
) -> String {
    if let Some(explicit) = default.filter(|value| !value.is_empty()) {
        return explicit.to_owned();
    }
    if let Some(value) = catalogue::default_for(full_key) {
        return config::render_value(&value);
    }
    warnings.push(unknown_path_key_warning(raw_key));
    String::new()
}

fn unknown_path_key_warning(key: &str) -> String {
    match key {
        "design_inventories" | "design_gaps" => format!(
            "Warning: key '{key}' was renamed by migration 0004 to \
             'research_{key}'; run /accelerator:migrate"
        ),
        _ => format!(
            "Warning: unknown key 'paths.{key}' — no centralized default"
        ),
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
    for (doc_type, path_key) in catalogue::DOC_TYPES {
        let full_key = format!("paths.{path_key}");
        let default = catalogue::default_for(&full_key)
            .map(|value| config::render_value(&value))
            .unwrap_or_default();
        let mut raw = resolve_or_default(config, &full_key)?;
        if raw.is_empty() {
            blanks.push(BlankDefault {
                path_key: (*path_key).to_owned(),
                default: default.clone(),
            });
            raw = default;
        }
        if raw.contains('\t') || raw.contains('\n') {
            return Err(ConfigError::Invalid {
                detail: format!(
                    "paths.{path_key} value contains a tab or newline"
                ),
            });
        }
        if is_unsafe(&raw) {
            return Err(ConfigError::Invalid {
                detail: format!(
                    "paths.{path_key} resolves to an unsafe path: {raw}"
                ),
            });
        }
        rows.push(((*doc_type).to_owned(), normalise(&raw)));
    }
    Ok(DocTypes { rows, blanks })
}

fn resolve_or_default(
    config: &dyn ConfigAccess,
    full_key: &str,
) -> Result<String, ConfigError> {
    Ok(config.effective(&Key::parse(full_key)?, None)?.rendered())
}

/// Whether a directory is unsafe, matching the bash `config-read-doc-type-paths`
/// case: empty, `.`, `..`, absolute, or carrying a `..`/interior-`.` segment. A
/// leading `./` alone is safe — it is normalised away.
fn is_unsafe(dir: &str) -> bool {
    dir.is_empty()
        || dir == "."
        || dir == ".."
        || dir.starts_with('/')
        || dir.ends_with("/..")
        || dir.starts_with("../")
        || dir.contains("/../")
        || dir.contains("/./")
}

/// Collapses repeated slashes, strips a leading `./` and a trailing `/`.
fn normalise(dir: &str) -> String {
    let mut collapsed = String::with_capacity(dir.len());
    let mut previous_slash = false;
    for character in dir.chars() {
        let slash = character == '/';
        if !(slash && previous_slash) {
            collapsed.push(character);
        }
        previous_slash = slash;
    }
    collapsed
        .strip_prefix("./")
        .unwrap_or(&collapsed)
        .trim_end_matches('/')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{is_unsafe, normalise};

    #[test]
    fn normalise_collapses_slashes_and_strips_dot_slash_and_trailing() {
        assert_eq!(normalise("./meta//work/"), "meta/work");
        assert_eq!(normalise("meta/work"), "meta/work");
    }

    #[test]
    fn unsafe_paths_are_rejected() {
        assert!(is_unsafe(""));
        assert!(is_unsafe("."));
        assert!(is_unsafe(".."));
        assert!(is_unsafe("/abs"));
        assert!(is_unsafe("../b"));
        assert!(is_unsafe("a/.."));
        assert!(is_unsafe("a/../b"));
        assert!(is_unsafe("a/./b"));
        assert!(!is_unsafe("meta/work"));
        assert!(!is_unsafe("./meta/work"));
    }
}
