//! The composed `MigrationContext`: legacy-layout config access for
//! doc-type directories, VCS revision, and the bounded atomic write every
//! migration's mutation routes through.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use config::ConfigAccess as _;
use config::ConfigService;
use config::Key;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use corpus::doc_type::DocTypeKey;
use migrate::ports::CorpusIndex;
use migrate::ports::DocTypeDir;
use migrate::ports::ManifestStore as _;
use migrate::ports::MigrationContext;
use migrate::ports::MigrationError;
use store::NewFileMode;
use store::WriteBounds;
use vcs_adapters::library::InProcessProbe;

use crate::corpus_index::FileCorpusIndex;
use crate::manifest_store::FileManifestStore;
use crate::merge_move::merge_move;

pub struct FileMigrationContext {
    root: PathBuf,
    config: ConfigService<FileConfigStore, FileConfigStore>,
    fresh_mode: u32,
    index: OnceCell<FileCorpusIndex>,
    manifest: FileManifestStore,
}

impl FileMigrationContext {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let store =
            FileConfigStore::at(&root).with_legacy_policy(LegacyPolicy::Allow);
        Self {
            config: ConfigService::new(store.clone(), store),
            fresh_mode: 0o666 & !store::current_umask(),
            index: OnceCell::new(),
            manifest: FileManifestStore::new(&root),
            root,
        }
    }

    fn bounds(&self) -> WriteBounds<'_> {
        WriteBounds {
            permitted_root: &self.root,
            project_root: &self.root,
        }
    }

    /// The `corpus::linkage`-shaped doc-type table: every configured
    /// doc-type directory, keyed by its [`DocTypeKey`] rather than its bare
    /// linkage-type-name string.
    fn linkage_table(&self) -> Vec<(DocTypeKey, PathBuf)> {
        MigrationContext::doc_type_dirs(self)
            .into_iter()
            .filter_map(|dir| {
                DocTypeKey::from_linkage_type_name(&dir.doc_type)
                    .map(|key| (key, dir.dir))
            })
            .collect()
    }
}

impl MigrationContext for FileMigrationContext {
    fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
        config::paths::doc_type_dirs(&self.config)
            .map(|dirs| {
                dirs.into_iter()
                    .map(|dir| DocTypeDir {
                        doc_type: dir.doc_type.to_owned(),
                        dir: self.root.join(dir.dir),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn revision(&self) -> Option<String> {
        let probe = InProcessProbe;
        vcs::facts(&self.root, &probe, &probe).and_then(|facts| facts.revision)
    }

    fn corpus_index(&self) -> &dyn CorpusIndex {
        self.index
            .get_or_init(|| FileCorpusIndex::build(&self.linkage_table()))
    }

    fn write(&self, path: &Path, content: &str) -> Result<(), MigrationError> {
        store::atomic_write(
            path,
            content.as_bytes(),
            &self.bounds(),
            NewFileMode::PreserveOr(self.fresh_mode),
        )
        .map_err(|error| MigrationError::new(error.to_string()))?;
        if let Ok(relative) = path.strip_prefix(&self.root) {
            self.manifest
                .append_manifest_path(&relative.to_string_lossy())?;
        }
        Ok(())
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn config_value(
        &self,
        key: &str,
    ) -> Result<Option<String>, MigrationError> {
        let key = Key::parse(key)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        let resolution = self
            .config
            .effective(&key, None)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        Ok(Some(resolution.rendered()))
    }

    fn configured_path_override(
        &self,
        key: &str,
    ) -> Result<Option<String>, MigrationError> {
        let key = Key::parse(key)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        let resolution = self
            .config
            .effective(&key, None)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        Ok(resolution.configured_value())
    }

    fn read(&self, path: &Path) -> Result<Option<String>, MigrationError> {
        let bytes = store::read_within(path, &self.bounds())
            .map_err(|error| MigrationError::new(error.to_string()))?;
        Ok(bytes.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn dir_exists(&self, path: &Path) -> bool {
        fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())
    }

    fn remove_file(&self, path: &Path) -> Result<(), MigrationError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(())
            }
            Err(error) => Err(MigrationError::new(error.to_string())),
        }
    }

    fn remove_dir_if_empty(&self, path: &Path) -> Result<bool, MigrationError> {
        match fs::remove_dir(path) {
            Ok(()) => Ok(true),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound
                        | std::io::ErrorKind::DirectoryNotEmpty
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(MigrationError::new(error.to_string())),
        }
    }

    fn list_md_files(
        &self,
        dir: &Path,
    ) -> Result<Vec<PathBuf>, MigrationError> {
        let mut files = Vec::new();
        walk(dir, &mut files, true)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        files.sort();
        Ok(files)
    }

    fn list_all_under(
        &self,
        dir: &Path,
    ) -> Result<Vec<PathBuf>, MigrationError> {
        let mut entries = Vec::new();
        walk(dir, &mut entries, false)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        entries.sort();
        Ok(entries)
    }

    fn merge_move(&self, src: &Path, dst: &Path) -> Result<(), MigrationError> {
        merge_move(src, dst, &self.root)
    }

    fn canonicalise_work_item_id(
        &self,
        bare_number: &str,
    ) -> Result<String, MigrationError> {
        let pattern = MigrationContext::config_value(self, "work.id_pattern")?
            .unwrap_or_else(|| "{number:04d}".to_owned());
        let project =
            MigrationContext::config_value(self, "work.default_project_code")?
                .unwrap_or_default();
        corpus_adapters::work_item_pattern::canonicalise_id(
            bare_number,
            &pattern,
            &project,
        )
        .map_err(|error| MigrationError::new(error.to_string()))
    }

    fn validate_frontmatter(
        &self,
        files: &[PathBuf],
    ) -> Result<(), MigrationError> {
        let table = self.linkage_table();
        let walker = corpus_adapters::fs::RealFs;
        let target = if files.is_empty() {
            corpus_adapters::frontmatter_validation::corpus_files(
                &table, &walker,
            )
            .map_err(|error| MigrationError::new(error.to_string()))?
        } else {
            files.to_vec()
        };
        let index = corpus_adapters::frontmatter_validation::build_index(
            &table, &walker,
        )
        .map_err(|error| MigrationError::new(error.to_string()))?;
        let checks = corpus_adapters::frontmatter_validation::Checks {
            structure: true,
            references: true,
            canonical: false,
        };
        let results =
            corpus_adapters::frontmatter_validation::validate_targets(
                &target, &table, &index, checks, &walker,
            )
            .map_err(|error| MigrationError::new(error.to_string()))?;

        let mut messages = Vec::new();
        for (path, outcome) in &results {
            if let corpus_adapters::frontmatter_validation::TargetOutcome::Violations(violations) = outcome {
                for violation in violations {
                    messages.push(format!("{}: {violation}", path.display()));
                }
            }
        }
        if messages.is_empty() {
            Ok(())
        } else {
            Err(MigrationError::new(messages.join("\n")))
        }
    }

    fn realign_sync_baseline(
        &self,
        pre_migration: &[(PathBuf, String)],
    ) -> Result<usize, MigrationError> {
        let Some(relative) =
            MigrationContext::config_value(self, "paths.integrations")?
                .filter(|value| !value.is_empty())
        else {
            return Ok(0);
        };
        let integrations_root = {
            let path = Path::new(&relative);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.root.join(path)
            }
        };
        if !integrations_root.is_dir() {
            return Ok(0);
        }

        let mut by_id: HashMap<String, (&Path, &str)> = HashMap::new();
        for (path, original) in pre_migration {
            if let Some(id) = frontmatter_id(original) {
                by_id.insert(id, (path.as_path(), original.as_str()));
            }
        }

        let mut realigned = 0;
        let entries = fs::read_dir(&integrations_root)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        for entry in entries {
            let entry = entry
                .map_err(|error| MigrationError::new(error.to_string()))?;
            if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                continue;
            }
            let integration = entry.file_name();
            let integration = integration.to_string_lossy();
            let baseline_path = work_adapters::sync::baseline::path(
                &integrations_root,
                &integration,
            );
            if baseline_path.exists() {
                realigned += realign_one_baseline(&baseline_path, &by_id)?;
            }
        }
        Ok(realigned)
    }
}

/// The `id` field's stripped value from `content`'s frontmatter, if present.
fn frontmatter_id(content: &str) -> Option<String> {
    let (frontmatter, _) =
        work_adapters::sync::digest::split_frontmatter_and_body(content)
            .ok()?;
    let entries = corpus::frontmatter_validation::parse_entries(&frontmatter);
    let raw = corpus::frontmatter_validation::raw_value(&entries, "id")?;
    let id = corpus::frontmatter_validation::strip_surrounding_quote(raw);
    (!id.is_empty()).then(|| id.to_owned())
}

/// Realigns one integration's baseline: an entry whose pre-migration digest
/// matched its stored `local_hash` (it was `Synced`) is advanced to the
/// re-rendered file's digest; a diverged entry (it was `LocallyModified`) is
/// left untouched so its pending push survives.
fn realign_one_baseline(
    baseline_path: &Path,
    by_id: &HashMap<String, (&Path, &str)>,
) -> Result<usize, MigrationError> {
    let reader = corpus_adapters::fs::RealFs;
    let store_dir = baseline_path.parent().unwrap_or_else(|| Path::new("."));
    let writer = corpus_adapters::FileCorpusStore::new(store_dir);
    let mut store = work_adapters::sync::baseline_store::BaselineStore::new(
        baseline_path.to_path_buf(),
        &reader,
        &writer,
    );
    let (baseline, _) = store
        .load()
        .map_err(|error| MigrationError::new(error.to_string()))?;

    let mut realigned = 0;
    for (id, &(path, original)) in by_id {
        let Some(entry) = baseline.get(id) else {
            continue;
        };
        let Ok(pre_hash) = work_adapters::sync::digest::local(original) else {
            continue;
        };
        if pre_hash != entry.local_hash {
            continue;
        }
        let Ok(new_content) = fs::read_to_string(path) else {
            continue;
        };
        let Ok(new_hash) = work_adapters::sync::digest::local(&new_content)
        else {
            continue;
        };
        if new_hash == entry.local_hash {
            continue;
        }
        let updated = work_adapters::sync::baseline::Entry {
            local_hash: new_hash,
            remote_hash: entry.remote_hash.clone(),
            remote_updated_at: entry.remote_updated_at.clone(),
        };
        store
            .set(id, updated)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        realigned += 1;
    }
    Ok(realigned)
}

/// Recursively collects entries under `dir`. `md_only` selects between
/// [`MigrationContext::list_md_files`]'s `.md`-file-only listing and
/// [`MigrationContext::list_all_under`]'s unfiltered file-and-directory
/// listing.
fn walk(
    dir: &Path,
    out: &mut Vec<PathBuf>,
    md_only: bool,
) -> std::io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(())
        }
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !md_only {
                out.push(path.clone());
            }
            walk(&path, out, md_only)?;
        } else if file_type.is_file()
            && (!md_only || path.extension().is_some_and(|ext| ext == "md"))
        {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::FileMigrationContext;
    use crate::manifest_store::FileManifestStore;
    use migrate::ports::ManifestStore as _;
    use migrate::ports::MigrationContext as _;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn a_write_records_the_path_once_even_across_two_write_points(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let ctx = FileMigrationContext::new(dir.path());

        ctx.write(&dir.path().join("meta/work/a.md"), "one")?;
        ctx.write(&dir.path().join("meta/work/a.md"), "two")?;
        ctx.write(&dir.path().join("meta/work/b.md"), "three")?;

        let manifest = FileManifestStore::new(dir.path());
        let mut recorded = manifest.manifest()?.unwrap_or_default();
        recorded.sort();
        assert_eq!(
            recorded,
            vec!["meta/work/a.md".to_owned(), "meta/work/b.md".to_owned()]
        );
        Ok(())
    }
}
