//! Resolution of configured directory paths: the doc-type → directory mapping
//! and the safety/normalisation its consumers share.
//!
//! Pure string logic over the config ports — no filesystem, serde, or `PathBuf`
//! enters here. Consumers prepend a project root and build paths themselves.

use crate::catalogue;
use crate::error::ConfigError;
use crate::key::Key;
use crate::render::render_value;
use crate::service::ConfigAccess;

/// A resolved doc-type directory: the type, its `paths.<path_key>`, the
/// normalised safe relative directory, and whether a blank config value fell
/// back to the catalogue default.
pub struct DocTypeDir {
    pub doc_type: &'static str,
    pub path_key: &'static str,
    pub dir: String,
    pub blank_fallback: bool,
}

/// Resolve every doc-type's configured directory, coercing a blank value to the
/// catalogue default and refusing an unsafe or tab/newline-bearing value.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when a resolved directory is unsafe or carries a
/// tab or newline; a [`ConfigError`] when a config level cannot be read.
pub fn doc_type_dirs(
    config: &dyn ConfigAccess,
) -> Result<Vec<DocTypeDir>, ConfigError> {
    let mut resolved = Vec::new();
    for &(doc_type, path_key) in catalogue::DOC_TYPES {
        let full_key = format!("paths.{path_key}");
        let default = catalogue::default_for(&full_key)
            .map(|value| render_value(&value))
            .unwrap_or_default();
        let mut raw =
            config.effective(&Key::parse(&full_key)?, None)?.rendered();
        let blank_fallback = raw.is_empty();
        if blank_fallback {
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
        resolved.push(DocTypeDir {
            doc_type,
            path_key,
            dir: normalise(&raw),
            blank_fallback,
        });
    }
    Ok(resolved)
}

/// Whether a configured directory is unsafe: empty, `.`, `..`, absolute, or
/// carrying a `..`/interior-`.` segment. A leading `./` alone is safe — it is
/// normalised away by [`normalise`].
#[must_use]
pub fn is_unsafe(dir: &str) -> bool {
    dir.is_empty()
        || dir == "."
        || dir == ".."
        || dir.starts_with('/')
        || dir.ends_with("/..")
        || dir.starts_with("../")
        || dir.contains("/../")
        || dir.contains("/./")
}

/// Collapse repeated slashes, strip a leading `./` and a trailing `/`.
#[must_use]
pub fn normalise(dir: &str) -> String {
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
