//! The project's scratch directory for research state that must outlive one
//! call: the arXiv lock, its pacing, its logs, and confirmed withdrawals.

use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;

use store::atomic_write;
use store::NewFileMode;
use store::WriteBounds;

const STATE_MODE: u32 = 0o644;

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

    pub fn path(&self, name: &str) -> PathBuf {
        self.directory.join(name)
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
            &WriteBounds {
                permitted_root: &self.directory,
                project_root: &self.project_root,
            },
            NewFileMode::PreserveOr(STATE_MODE),
        )
        .map_err(|error| format!("could not write {}: {error}", path.display()))
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
