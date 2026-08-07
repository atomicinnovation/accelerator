//! The composition-root helper every dispatch branch that needs
//! configuration calls, rather than re-deriving `config_adapters::compose`
//! and its error mapping independently.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use config::ConfigAccess;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use corpus::DocTypeKey;

/// The composed configuration service plus the discovered project root.
pub struct Composed {
    pub service: config::ConfigService<FileConfigStore, FileConfigStore>,
    pub project_root: PathBuf,
}

/// Wires the configuration ports at `cwd`'s project root, failing closed on
/// the legacy `.claude/accelerator.md` layout — matching bash's uniform
/// treatment of config/schema-resolution failures as ordinary (exit 1)
/// failures, not usage errors.
///
/// # Errors
///
/// A [`kernel::Error`] when the discovered root carries the legacy layout, or
/// a config level cannot be read.
pub fn compose(cwd: &Path) -> Result<Composed, kernel::Error> {
    let project_root = FileConfigStore::discover_root(cwd);
    let composed = config_adapters::compose(cwd, LegacyPolicy::Reject)?;
    Ok(Composed {
        service: composed.service,
        project_root,
    })
}

/// Resolves `paths.decisions`, absolute paths used as-is and relative paths
/// resolved against the discovered project root — matching
/// `adr-next-number.sh`'s own resolution rule.
///
/// # Errors
///
/// A [`kernel::Error`] when the key cannot be resolved.
pub fn resolve_decisions_dir(
    composed: &Composed,
) -> Result<PathBuf, kernel::Error> {
    let raw = config::paths::resolve_with_fallback(
        &composed.service,
        "decisions",
        None,
    )?;
    let path = PathBuf::from(raw);
    Ok(if path.is_absolute() {
        path
    } else {
        composed.project_root.join(path)
    })
}

/// Resolves the `(DocTypeKey, dir)` table from configured doc-type paths,
/// reusing `corpus_adapters::doc_type::table_from_paths` for the final keyed
/// lookup rather than reimplementing it against `linkage_type_name()`.
///
/// Deliberately **not** joined against a project root, unlike
/// `cli/visualiser/server/src/compose.rs`'s `resolve_doc_paths` (which this
/// otherwise mirrors): `corpus::linkage::path_roots` derives its scan roots
/// from each directory's leading path segment
/// (`dir.split('/').next()`), which is empty for any absolute path — an
/// project-root-joined table silently makes every path-based linkage
/// reference unextractable (`extract_doc_paths` finds nothing to scan for).
/// `corpus::doc_type::infer`'s own matcher tolerates either form (it checks
/// a leading-prefix match *and* a `/dir/` embedded-anywhere match), so
/// keeping the table in the same project-root-relative form
/// `config::paths::doc_type_dirs` already returns costs nothing there. A
/// future caller that needs to *walk* these directories on disk (e.g.
/// `frontmatter validate`'s `CorpusWalker`) must join the project root
/// itself, at the point it actually touches the filesystem.
///
/// # Errors
///
/// A [`config::ConfigError`] when a doc-type directory is unsafe or a config
/// level cannot be read.
pub fn table_from_config(
    config: &dyn ConfigAccess,
) -> Result<Vec<(DocTypeKey, PathBuf)>, config::ConfigError> {
    let mut doc_paths = HashMap::new();
    for resolved in config::paths::doc_type_dirs(config)? {
        doc_paths
            .insert(resolved.path_key.to_owned(), PathBuf::from(resolved.dir));
    }
    Ok(corpus_adapters::doc_type::table_from_paths(&doc_paths))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use config::ConfigAccess;
    use config::ConfigError;
    use config::Key;
    use config::Level;
    use config::Resolved;
    use config::Scalar;
    use config::Value;
    use corpus::DocTypeKey;

    use super::table_from_config;

    struct FakeConfig(HashMap<String, String>);

    impl ConfigAccess for FakeConfig {
        fn get(
            &self,
            key: &Key,
            level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            if level == Some(Level::Team) {
                return Ok(Resolved::Absent);
            }
            Ok(self
                .0
                .get(&key.to_string())
                .map_or(Resolved::Absent, |value| {
                    Resolved::Found(Value::Scalar(Scalar::String(
                        value.clone(),
                    )))
                }))
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            Ok(())
        }
    }

    fn dir_for(
        table: &[(DocTypeKey, PathBuf)],
        kind: DocTypeKey,
    ) -> Option<PathBuf> {
        table
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .map(|(_, dir)| dir.clone())
    }

    #[test]
    fn resolves_a_configured_doc_type_directory() -> Result<(), ConfigError> {
        let mut values = HashMap::new();
        values.insert("paths.work".to_owned(), "tickets".to_owned());
        let config = FakeConfig(values);

        let table = table_from_config(&config)?;

        assert_eq!(
            dir_for(&table, DocTypeKey::WorkItems),
            Some(PathBuf::from("tickets")),
            "the directory must stay project-root-relative, not be joined \
             against any root here"
        );
        Ok(())
    }

    #[test]
    fn falls_back_to_the_catalogue_default_when_nothing_is_configured(
    ) -> Result<(), ConfigError> {
        let config = FakeConfig(HashMap::new());

        let table = table_from_config(&config)?;

        assert_eq!(
            dir_for(&table, DocTypeKey::WorkItems),
            Some(PathBuf::from("meta/work"))
        );
        Ok(())
    }
}
