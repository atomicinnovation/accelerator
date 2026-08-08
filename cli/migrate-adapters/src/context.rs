//! The composed `MigrationContext`: legacy-layout config access for
//! doc-type directories, VCS revision, and the bounded atomic write every
//! migration's mutation routes through.

use std::path::Path;
use std::path::PathBuf;

use config::ConfigService;
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
}
