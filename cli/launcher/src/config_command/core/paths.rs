//! The `paths` view assembly: the configured path keys, and the 13 doc-type →
//! directory mappings with their fail-closed value hardening.

use config::{catalogue, ConfigAccess, ConfigError, Key, Resolved};

/// A configured path row: the bare key and its resolved (or default) value.
pub struct ConfiguredPath {
    pub key: String,
    pub value: String,
}

/// The doc-type resolutions and any blank-coercion notes for stderr.
pub struct DocTypes {
    pub rows: Vec<(String, String)>,
    pub notes: Vec<String>,
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
    let mut notes = Vec::new();
    for (doc_type, path_key) in catalogue::DOC_TYPES {
        let full_key = format!("paths.{path_key}");
        let default = catalogue::default_for(&full_key)
            .map(|value| config::render_value(&value))
            .unwrap_or_default();
        let mut raw = resolve_or_default(config, &full_key)?;
        if raw.is_empty() {
            notes.push(format!(
                "paths.{path_key} is blank; using default '{default}' \
                 (blanking a path does not disable a doc-type)"
            ));
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
    Ok(DocTypes { rows, notes })
}

fn resolve_or_default(
    config: &dyn ConfigAccess,
    full_key: &str,
) -> Result<String, ConfigError> {
    let key = Key::parse(full_key)?;
    Ok(match config.get(&key, None)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => catalogue::default_for(full_key)
            .map(|value| config::render_value(&value))
            .unwrap_or_default(),
    })
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
