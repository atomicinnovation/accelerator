//! The project's scratch directory for research state that must outlive one
//! call: the arXiv lock, its pacing, its logs, and confirmed withdrawals.

use std::fs::File;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use store::atomic_write;
use store::ensure_contained;
use store::replace_without_sync;
use store::NewFileMode;
use store::WriteBounds;

const STATE_MODE: u32 = 0o644;

#[derive(Clone)]
pub struct ScratchDir {
    project_root: PathBuf,
    directory: PathBuf,
}

impl ScratchDir {
    /// `directory` must lie inside `project_root`; a write anywhere else is
    /// refused.
    pub fn new(project_root: &Path, directory: &Path) -> Self {
        Self {
            project_root: project_root.to_owned(),
            directory: directory.to_owned(),
        }
    }

    /// A directory within this one, bounded by the same project root.
    #[must_use]
    pub fn nested(&self, name: &str) -> Self {
        Self {
            project_root: self.project_root.clone(),
            directory: self.directory.join(name),
        }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }

    /// The names of every entry, or none while the directory does not exist
    /// yet.
    ///
    /// # Errors
    ///
    /// A one-line description naming the directory when it cannot be listed.
    pub fn names(&self) -> Result<Vec<String>, String> {
        let entries = match std::fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Vec::new())
            }
            Err(error) => return Err(self.unlistable(&error)),
        };
        entries
            .map(|entry| {
                entry
                    .map(|entry| {
                        entry.file_name().to_string_lossy().into_owned()
                    })
                    .map_err(|error| self.unlistable(&error))
            })
            .collect()
    }

    pub fn modified(&self, name: &str) -> Option<SystemTime> {
        std::fs::metadata(self.path(name))
            .and_then(|metadata| metadata.modified())
            .ok()
    }

    /// Removes a file, counting one already gone as removed.
    ///
    /// # Errors
    ///
    /// A one-line description naming the file when it lies outside this
    /// directory or cannot be removed.
    pub fn remove(&self, name: &str) -> Result<(), String> {
        let path = self.path(name);
        let unremovable = |detail: &dyn std::fmt::Display| {
            format!("could not remove {}: {detail}", path.display())
        };
        ensure_contained(&path, &self.bounds())
            .map_err(|error| unremovable(&error))?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(unremovable(&error)),
        }
    }

    fn unlistable(&self, error: &std::io::Error) -> String {
        format!("could not list {}: {error}", self.directory.display())
    }

    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(name)).ok()
    }

    /// # Errors
    ///
    /// A one-line description naming the file when it cannot be replaced.
    pub fn replace(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.path(name);
        atomic_write(
            &path,
            bytes,
            &self.bounds(),
            NewFileMode::PreserveOr(STATE_MODE),
        )
        .map_err(|error| format!("could not write {}: {error}", path.display()))
    }

    /// Like [`Self::replace`], but an OS crash may lose the write: for state
    /// that is rebuilt or outlived by any call it could affect.
    ///
    /// # Errors
    ///
    /// A one-line description naming the file when it cannot be replaced.
    pub fn replace_without_sync(
        &self,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let path = self.path(name);
        replace_without_sync(
            &path,
            bytes,
            &self.bounds(),
            NewFileMode::PreserveOr(STATE_MODE),
        )
        .map_err(|error| format!("could not write {}: {error}", path.display()))
    }

    fn bounds(&self) -> WriteBounds<'_> {
        WriteBounds {
            permitted_root: &self.directory,
            project_root: &self.project_root,
        }
    }

    /// # Errors
    ///
    /// A one-line description naming the file when it cannot be appended to.
    pub fn append_line(&self, name: &str, line: &str) -> Result<(), String> {
        let path = self.path(name);
        self.created()
            .and_then(|()| {
                OpenOptions::new().create(true).append(true).open(&path)
            })
            .and_then(|mut log| writeln!(log, "{line}"))
            .map_err(|error| {
                format!("could not append to {}: {error}", path.display())
            })
    }

    /// # Errors
    ///
    /// A one-line description naming the file when it cannot be opened.
    pub fn open(&self, name: &str) -> Result<File, String> {
        let path = self.path(name);
        self.created()
            .and_then(|()| {
                OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .write(true)
                    .open(&path)
            })
            .map_err(|error| {
                format!("could not open {}: {error}", path.display())
            })
    }

    fn created(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.directory)
    }
}
