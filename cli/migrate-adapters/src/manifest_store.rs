//! The per-run path manifest and run-id sidecar.
//!
//! `.accelerator/state/migrations-run-paths.txt` (one repo-relative path per
//! line, deduped) and `.accelerator/state/migrations-run.id` (the base
//! revision the run started against; empty content records a `None`
//! revision, distinct from the file being absent).

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use migrate::ports::ManifestStore;
use migrate::ports::MigrationError;
use store::NewFileMode;
use store::WriteBounds;

pub struct FileManifestStore {
    root: PathBuf,
    fresh_mode: u32,
}

impl FileManifestStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            fresh_mode: 0o666 & !store::current_umask(),
        }
    }

    fn bounds(&self) -> WriteBounds<'_> {
        WriteBounds {
            permitted_root: &self.root,
            project_root: &self.root,
        }
    }

    fn manifest_path(&self) -> PathBuf {
        self.root
            .join(".accelerator/state/migrations-run-paths.txt")
    }

    fn run_id_path(&self) -> PathBuf {
        self.root.join(".accelerator/state/migrations-run.id")
    }

    fn write(&self, path: &Path, content: &str) -> Result<(), MigrationError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| MigrationError::new(error.to_string()))?;
        }
        store::atomic_write(
            path,
            content.as_bytes(),
            &self.bounds(),
            NewFileMode::PreserveOr(self.fresh_mode),
        )
        .map_err(|error| MigrationError::new(error.to_string()))
    }

    fn read(&self, path: &Path) -> Result<Option<String>, MigrationError> {
        if !path.exists() {
            return Ok(None);
        }
        let bytes = store::read_within(path, &self.bounds())
            .map_err(|error| MigrationError::new(error.to_string()))?
            .unwrap_or_default();
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|error| MigrationError::new(error.to_string()))
    }
}

impl ManifestStore for FileManifestStore {
    fn manifest(&self) -> Result<Option<Vec<String>>, MigrationError> {
        Ok(self.read(&self.manifest_path())?.map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect()
        }))
    }

    fn write_manifest(&self, paths: &[String]) -> Result<(), MigrationError> {
        let mut content = String::new();
        for path in paths {
            content.push_str(path);
            content.push('\n');
        }
        self.write(&self.manifest_path(), &content)
    }

    fn append_manifest_path(&self, path: &str) -> Result<(), MigrationError> {
        let mut current = self.manifest()?.unwrap_or_default();
        if !current.iter().any(|existing| existing == path) {
            current.push(path.to_owned());
        }
        self.write_manifest(&current)
    }

    fn run_id(&self) -> Result<Option<String>, MigrationError> {
        Ok(self
            .read(&self.run_id_path())?
            .map(|text| text.trim().to_owned())
            .filter(|trimmed| !trimmed.is_empty()))
    }

    fn write_run_id(
        &self,
        revision: Option<&str>,
    ) -> Result<(), MigrationError> {
        self.write(
            &self.run_id_path(),
            &format!("{}\n", revision.unwrap_or("")),
        )
    }

    fn clear(&self) -> Result<(), MigrationError> {
        for path in [self.manifest_path(), self.run_id_path()] {
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|error| MigrationError::new(error.to_string()))?;
            }
        }
        Ok(())
    }
}
