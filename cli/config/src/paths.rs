//! Resolution of configured directory paths: the doc-type → directory mapping
//! and the safety/normalisation its consumers share.
//!
//! Pure logic over the config ports — no filesystem or serde enters here.
//! Consumers prepend a project root and build paths themselves.

use std::path::Path;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocTypeDir {
    pub doc_type: &'static str,
    pub path_key: &'static str,
    pub dir: String,
    pub blank_fallback: bool,
}

/// Resolve every doc-type's configured directory.
///
/// A blank value is coerced to the catalogue default. An unsafe or
/// tab/newline-bearing value is refused, and so is a set of directories in
/// which one equals or contains another.
///
/// # Errors
///
/// [`ConfigError::Invalid`] when a resolved directory is unsafe or carries a
/// tab or newline; [`ConfigError::OverlappingDocRoots`] when two resolved
/// directories overlap; a [`ConfigError`] when a config level cannot be read.
pub fn doc_type_dirs(
    config: &dyn ConfigAccess,
) -> Result<Vec<DocTypeDir>, ConfigError> {
    doc_type_dirs_at(config, None)
}

/// As [`doc_type_dirs`], reading `level` alone when one is given.
///
/// # Errors
///
/// As [`doc_type_dirs`].
pub fn doc_type_dirs_at(
    config: &dyn ConfigAccess,
    level: Option<Level>,
) -> Result<Vec<DocTypeDir>, ConfigError> {
    let mut resolved = Vec::new();
    for &(doc_type, path_key) in catalogue::DOC_TYPES {
        let full_key = format!("paths.{path_key}");
        let default = catalogue::default_for(&full_key)
            .map(|value| render_value(&value))
            .unwrap_or_default();
        let mut raw =
            config.effective(&Key::parse(&full_key)?, level)?.rendered();
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
    refuse_overlapping_roots(&resolved)?;
    Ok(resolved)
}

/// Two roots, by their positions in the list compared, of which `container`
/// equals or contains `contained`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overlap {
    pub container: usize,
    pub contained: usize,
}

/// The first pair of `roots`, in input order, of which one equals or contains
/// the other segment by segment. Of two equal roots the earlier contains the
/// later.
#[must_use]
pub fn first_overlap(roots: &[&Path]) -> Option<Overlap> {
    roots
        .iter()
        .enumerate()
        .find_map(|(earlier, earlier_root)| {
            roots.iter().enumerate().skip(earlier + 1).find_map(
                |(later, later_root)| {
                    if earlier_root.starts_with(later_root)
                        && !later_root.starts_with(earlier_root)
                    {
                        Some(Overlap {
                            container: later,
                            contained: earlier,
                        })
                    } else if later_root.starts_with(earlier_root) {
                        Some(Overlap {
                            container: earlier,
                            contained: later,
                        })
                    } else {
                        None
                    }
                },
            )
        })
}

fn refuse_overlapping_roots(dirs: &[DocTypeDir]) -> Result<(), ConfigError> {
    let roots: Vec<&Path> =
        dirs.iter().map(|dir| Path::new(&dir.dir)).collect();
    first_overlap(&roots).map_or(Ok(()), |overlap| {
        Err(ConfigError::OverlappingDocRoots {
            container: Box::new(dirs[overlap.container].clone()),
            contained: Box::new(dirs[overlap.contained].clone()),
        })
    })
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
/// happens to be set for it in config. A doc type's key is resolved through
/// [`doc_type_dirs_at`] at the same level, so it is refused, as by
/// [`doc_type_dirs`], whenever the doc roots overlap. A [`ConfigError`] when
/// `path_key` doesn't parse as a key segment or a config level cannot be
/// read.
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
    if names_a_doc_type(path_key) {
        return doc_type_dirs_at(config, level)?
            .into_iter()
            .find(|resolved| resolved.path_key == path_key)
            .map(|doc_root| doc_root.dir)
            .ok_or_else(|| unknown_path_key_error(path_key));
    }
    Ok(match config.get(&key, level)? {
        Resolved::Found(value) => render_value(&value),
        Resolved::Absent => render_value(&catalogue_default),
    })
}

fn names_a_doc_type(path_key: &str) -> bool {
    catalogue::DOC_TYPES
        .iter()
        .any(|&(_, doc_type_key)| doc_type_key == path_key)
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
    use std::path::Path;

    use super::{
        doc_type_dirs, doc_type_dirs_at, first_overlap, is_unsafe, normalise,
        resolve_with_fallback, Overlap,
    };
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

    struct TeamOnlyReader(Node);

    impl ReadConfigLevel for TeamOnlyReader {
        fn read(&self, level: Level) -> Result<Option<Node>, ConfigError> {
            Ok(match level {
                Level::Personal => None,
                Level::Team => Some(self.0.clone()),
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

    fn configured_paths(
        entries: &[(&str, &str)],
    ) -> ConfigService<FakeReader, NullWriter> {
        ConfigService::new(FakeReader(Some(paths_node(entries))), NullWriter)
    }

    fn paths_node(entries: &[(&str, &str)]) -> Node {
        mapping(vec![(
            "paths",
            mapping(
                entries
                    .iter()
                    .map(|(key, value)| {
                        (
                            *key,
                            Node::Scalar(Scalar::String((*value).to_owned())),
                        )
                    })
                    .collect(),
            ),
        )])
    }

    fn overlapping_keys(error: &ConfigError) -> Option<(&str, &str)> {
        match error {
            ConfigError::OverlappingDocRoots {
                container,
                contained,
            } => Some((container.path_key, contained.path_key)),
            _ => None,
        }
    }

    fn refused_overlap(
        entries: &[(&str, &str)],
    ) -> Option<(&'static str, &'static str)> {
        let service = configured_paths(entries);
        match doc_type_dirs(&service) {
            Err(ConfigError::OverlappingDocRoots {
                container,
                contained,
            }) => Some((container.path_key, contained.path_key)),
            _ => None,
        }
    }

    #[test]
    fn a_root_containing_another_is_refused_naming_the_first_pair() {
        assert_eq!(
            refused_overlap(&[("research_codebase", "meta/research")]),
            Some(("research_codebase", "research_issues"))
        );
    }

    #[test]
    fn equal_roots_are_refused() {
        assert_eq!(
            refused_overlap(&[("notes", "meta/plans")]),
            Some(("plans", "notes"))
        );
    }

    #[test]
    fn roots_equal_after_normalisation_are_refused() {
        assert_eq!(
            refused_overlap(&[("notes", "meta/plans/")]),
            Some(("plans", "notes"))
        );
        assert_eq!(
            refused_overlap(&[("notes", "./meta/plans")]),
            Some(("plans", "notes"))
        );
    }

    #[test]
    fn the_containing_root_is_named_as_container_whatever_its_order() {
        assert_eq!(
            refused_overlap(&[("plans", "meta/notes/plans")]),
            Some(("notes", "plans"))
        );
    }

    #[test]
    fn a_shared_string_prefix_without_a_shared_segment_is_accepted(
    ) -> Result<(), ConfigError> {
        let service = configured_paths(&[("notes", "meta/plansx")]);
        assert_eq!(doc_type_dirs(&service)?.len(), 14);
        Ok(())
    }

    #[test]
    fn the_default_roots_are_accepted() -> Result<(), ConfigError> {
        let service = ConfigService::new(FakeReader(None), NullWriter);
        assert_eq!(doc_type_dirs(&service)?.len(), 14);
        Ok(())
    }

    #[test]
    fn the_overlap_refusal_names_both_keys_and_dirs() {
        let service =
            configured_paths(&[("research_codebase", "meta/research")]);
        let refusal = doc_type_dirs(&service).err();
        assert!(refusal.as_ref().is_some_and(ConfigError::is_refusal));
        assert_eq!(
            refusal.map(|error| error.to_string()).unwrap_or_default(),
            "paths.research_codebase (meta/research) overlaps \
             paths.research_issues (meta/research/issues); doc roots may not \
             equal or contain one another"
        );
    }

    #[test]
    fn first_overlap_names_the_earlier_of_two_equal_roots_as_container() {
        let roots = [Path::new("a/b"), Path::new("c"), Path::new("a/b")];
        assert_eq!(
            first_overlap(&roots),
            Some(Overlap {
                container: 0,
                contained: 2
            })
        );
    }

    #[test]
    fn first_overlap_compares_absolute_roots_by_segment() {
        let roots = [Path::new("/r/meta/notes"), Path::new("/r/meta/note")];
        assert_eq!(first_overlap(&roots), None);
        let nested =
            [Path::new("/r/meta/topics/x"), Path::new("/r/meta/topics")];
        assert_eq!(
            first_overlap(&nested),
            Some(Overlap {
                container: 1,
                contained: 0
            })
        );
    }

    #[test]
    fn a_doc_type_lookup_over_an_overlapping_config_is_refused() {
        let service =
            configured_paths(&[("research_codebase", "meta/research")]);
        let result = resolve_with_fallback(&service, "research_topics", None);
        assert_eq!(
            result.as_ref().err().and_then(overlapping_keys),
            Some(("research_codebase", "research_issues"))
        );
    }

    #[test]
    fn a_doc_type_lookup_at_a_level_checks_that_level_alone(
    ) -> Result<(), ConfigError> {
        let service = ConfigService::new(
            TeamOnlyReader(paths_node(&[(
                "research_codebase",
                "meta/research",
            )])),
            NullWriter,
        );
        assert_eq!(
            resolve_with_fallback(&service, "plans", Some(Level::Personal))?,
            "meta/plans"
        );
        let refused =
            resolve_with_fallback(&service, "plans", Some(Level::Team));
        assert_eq!(
            refused.as_ref().err().and_then(overlapping_keys),
            Some(("research_codebase", "research_issues"))
        );
        assert!(doc_type_dirs_at(&service, Some(Level::Personal)).is_ok());
        Ok(())
    }

    #[test]
    fn a_lookup_of_a_key_that_is_not_a_doc_root_ignores_an_overlap(
    ) -> Result<(), ConfigError> {
        let service = configured_paths(&[
            ("research_codebase", "meta/research"),
            ("tmp", "scratch"),
        ]);
        assert_eq!(resolve_with_fallback(&service, "tmp", None)?, "scratch");
        Ok(())
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
