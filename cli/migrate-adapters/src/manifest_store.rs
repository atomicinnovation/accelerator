//! The per-run path manifest and the recorded run base.
//!
//! `.accelerator/state/migrations-run-paths.txt` (one repo-relative path per
//! line, deduped) and `.accelerator/state/migrations-run.id` (the run base
//! the run started against; empty content records no run base, distinct
//! from the file being absent).

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use migrate::ports::ManifestStore;
use migrate::ports::MigrationError;
use migrate::run_base::RunBase;
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

    fn run_base_path(&self) -> PathBuf {
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

    fn recorded_run_base(&self) -> Result<Option<RunBase>, MigrationError> {
        Ok(self
            .read(&self.run_base_path())?
            .and_then(|text| RunBase::recorded(&text)))
    }

    fn record_run_base(
        &self,
        run_base: Option<&RunBase>,
    ) -> Result<(), MigrationError> {
        let recorded = run_base.map(ToString::to_string).unwrap_or_default();
        self.write(&self.run_base_path(), &format!("{recorded}\n"))
    }

    fn clear(&self) -> Result<(), MigrationError> {
        for path in [self.manifest_path(), self.run_base_path()] {
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|error| MigrationError::new(error.to_string()))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::FileManifestStore;
    use migrate::ports::ManifestStore;
    use migrate::run_base::RunBase;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn an_empty_run_base_record_parses_the_same_as_an_absent_one(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let store = FileManifestStore::new(dir.path());
        std::fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        std::fs::write(
            dir.path().join(".accelerator/state/migrations-run.id"),
            "",
        )?;

        assert_eq!(store.recorded_run_base()?, None);
        Ok(())
    }

    #[test]
    fn a_recorded_run_base_reads_back_unchanged() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let store = FileManifestStore::new(dir.path());
        let run_base =
            RunBase::from_base_commits(&["b".to_owned(), "a".to_owned()]);

        store.record_run_base(run_base.as_ref())?;

        assert_eq!(
            std::fs::read_to_string(
                dir.path().join(".accelerator/state/migrations-run.id")
            )?,
            "a+b\n"
        );
        assert_eq!(store.recorded_run_base()?, run_base);
        Ok(())
    }
}
