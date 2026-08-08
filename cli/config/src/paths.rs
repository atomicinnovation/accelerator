//! Resolution of configured directory paths: the doc-type → directory mapping
//! and the safety/normalisation its consumers share.
//!
//! Pure string logic over the config ports — no filesystem, serde, or `PathBuf`
//! enters here. Consumers prepend a project root and build paths themselves.

use crate::catalogue;
use crate::error::ConfigError;
use crate::key::Key;
use crate::level::Level;
use crate::render::render_value;
use crate::service::ConfigAccess;
use crate::service::Resolved;

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

/// Resolves `paths.<path_key>`, falling back to the catalogue default.
///
/// Falls back when the key is absent at the requested level (or, for
/// `level: None`, absent from both config levels). The core `Key::parse` →
/// `config.get` → catalogue-default-on-absent sequence shared by
/// `accelerator config path` and a one-shot CLI command that needs a
/// configured directory with no further layering (no explicit-default
/// override, no legacy-alias warning, no `--explain`) — callers wanting that
/// layering build it on top, as
/// `cli/launcher/src/config_command/core/paths.rs`'s `resolve` does.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when `path_key` has no catalogue entry — a path
/// key is only ever one the catalogue recognises, whether or not a value
/// happens to be set for it in config. A [`ConfigError`] when `path_key`
/// doesn't parse as a key segment or a config level cannot be read.
pub fn resolve_with_fallback(
    config: &dyn ConfigAccess,
    path_key: &str,
    level: Option<Level>,
) -> Result<String, ConfigError> {
    let full = format!("paths.{path_key}");
    let key = Key::parse(&full)?;
    let Some(catalogue_default) = catalogue::default_for(&full) else {
        return Err(unknown_path_key_error(path_key));
    };
    Ok(match config.get(&key, level)? {
        Resolved::Found(value) => render_value(&value),
        Resolved::Absent => render_value(&catalogue_default),
    })
}

/// The refusal for a `paths.<key>` lookup the catalogue has no entry for.
#[must_use]
pub fn unknown_path_key_error(key: &str) -> ConfigError {
    ConfigError::Invalid {
        detail: format!(
            "unknown path key 'paths.{key}' — no centralized default"
        ),
    }
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
    use super::{is_unsafe, normalise, resolve_with_fallback};
    use crate::error::ConfigError;
    use crate::level::Level;
    use crate::node::{Node, Scalar};
    use crate::service::{ConfigService, ReadConfigLevel, WriteConfigLevel};

    struct FakeReader(Option<Node>);

    impl ReadConfigLevel for FakeReader {
        fn read(&self, level: Level) -> Result<Option<Node>, ConfigError> {
            Ok(match level {
                Level::Personal => self.0.clone(),
                Level::Team => None,
            })
        }
    }

    struct NullWriter;

    impl WriteConfigLevel for NullWriter {
        fn write(
            &self,
            _level: Level,
            _document: &Node,
        ) -> Result<(), ConfigError> {
            Ok(())
        }
    }

    fn mapping(entries: Vec<(&str, Node)>) -> Node {
        Node::Mapping(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
        )
    }

    #[test]
    fn resolve_with_fallback_prefers_a_configured_value(
    ) -> Result<(), ConfigError> {
        let service = ConfigService::new(
            FakeReader(Some(mapping(vec![(
                "paths",
                mapping(vec![(
                    "decisions",
                    Node::Scalar(Scalar::String("adr".to_owned())),
                )]),
            )]))),
            NullWriter,
        );
        assert_eq!(resolve_with_fallback(&service, "decisions", None)?, "adr");
        Ok(())
    }

    #[test]
    fn resolve_with_fallback_falls_back_to_the_catalogue_default_on_absence(
    ) -> Result<(), ConfigError> {
        let service = ConfigService::new(FakeReader(None), NullWriter);
        assert_eq!(
            resolve_with_fallback(&service, "decisions", None)?,
            "meta/decisions"
        );
        Ok(())
    }

    #[test]
    fn resolve_with_fallback_refuses_an_unknown_key_even_when_set_in_config() {
        let service = ConfigService::new(
            FakeReader(Some(mapping(vec![(
                "paths",
                mapping(vec![(
                    "no-such-key",
                    Node::Scalar(Scalar::String("somewhere".to_owned())),
                )]),
            )]))),
            NullWriter,
        );
        let result = resolve_with_fallback(&service, "no-such-key", None);
        assert!(matches!(&result, Err(error) if error.is_refusal()));
    }

    #[test]
    fn resolve_with_fallback_refuses_an_unknown_key() {
        let service = ConfigService::new(FakeReader(None), NullWriter);
        let result = resolve_with_fallback(&service, "no-such-key", None);
        assert!(matches!(
            &result,
            Err(error)
                if error.is_refusal()
                    && error.to_string()
                        == "unknown path key 'paths.no-such-key' — no \
                            centralized default"
        ));
    }

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
