//! The composed `MigrationContext`: legacy-layout config access for
//! doc-type directories, VCS revision, and the bounded atomic write every
//! migration's mutation routes through.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use config::ConfigAccess as _;
use config::ConfigService;
use config::Key;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use migrate::ports::CorpusIndex;
use migrate::ports::DocTypeDir;
use migrate::ports::ManifestStore as _;
use migrate::ports::MigrationContext;
use migrate::ports::MigrationError;
use store::NewFileMode;
use store::WriteBounds;
use vcs_adapters::library::InProcessProbe;

use crate::manifest_store::FileManifestStore;
use crate::merge_move::merge_move;

/// No migration built so far consults `corpus_index()` — that capability is
/// migration 0007's alone (the Migration 0007 Port phase, which builds the
/// real doc-type-scanning `CorpusIndex` adapter). Always-false is honest
/// until then, not a placeholder standing in for hidden behaviour.
struct NoIndex;

impl CorpusIndex for NoIndex {
    fn target_exists(&self, _target_type: &str, _target_id: &str) -> bool {
        false
    }
}

pub struct FileMigrationContext {
    root: PathBuf,
    config: ConfigService<FileConfigStore, FileConfigStore>,
    fresh_mode: u32,
    index: NoIndex,
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
            index: NoIndex,
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
        &self.index
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

    fn config_value(&self, key: &str) -> Option<String> {
        let key = Key::parse(key).ok()?;
        let resolution = self.config.effective(&key, None).ok()?;
        Some(resolution.rendered())
    }

    fn read(&self, path: &Path) -> Result<Option<String>, MigrationError> {
        let bytes = store::read_within(path, &self.bounds())
            .map_err(|error| MigrationError::new(error.to_string()))?;
        Ok(bytes.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn list_md_files(
        &self,
        dir: &Path,
    ) -> Result<Vec<PathBuf>, MigrationError> {
        let mut files = Vec::new();
        walk_md_files(dir, &mut files)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        files.sort();
        Ok(files)
    }

    fn merge_move(&self, src: &Path, dst: &Path) -> Result<(), MigrationError> {
        merge_move(src, dst, &self.root)
    }
}

fn walk_md_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
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
            walk_md_files(&path, out)?;
        } else if file_type.is_file()
            && path.extension().is_some_and(|ext| ext == "md")
        {
            out.push(path);
        }
    }
    Ok(())
}
