//! The `std::fs::read_dir`-backed [`work::resolve::DirectoryLister`] and
//! [`work::work_item_files::WorkItemFiles`] implementations.
//!
//! This crate's pure-read adapter module, isolated (via `cli/pup.ron`'s
//! `work_adapters_filesystem_reads_in_process` rule) from the
//! subprocess-spawning modules later phases add.

use std::path::Path;
use std::path::PathBuf;

use work::resolve::DirectoryLister;
use work::work_item_files::WorkItemFile;
use work::work_item_files::WorkItemFiles;
use work::work_item_files::DRAFTS_DIRECTORY;

/// Lists the filenames directly inside `dir` (no recursion). A missing or
/// unreadable directory yields an empty list, matching the shell's own
/// `shopt -s nullglob` behaviour when `WORK_DIR` does not exist.
pub struct FilesystemLister {
    dir: PathBuf,
}

impl FilesystemLister {
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }
}

impl DirectoryLister for FilesystemLister {
    fn filenames(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect()
    }
}

#[must_use]
pub fn drafts_dir(work_dir: &Path) -> PathBuf {
    work_dir.join(DRAFTS_DIRECTORY)
}

pub struct FilesystemWorkItemFiles {
    work_dir: PathBuf,
}

impl FilesystemWorkItemFiles {
    #[must_use]
    pub fn new(work_dir: &Path) -> Self {
        Self {
            work_dir: work_dir.to_path_buf(),
        }
    }
}

/// The `*.md` files directly inside `dir`, sorted by path. A missing
/// directory holds none; an unreadable file is skipped.
fn markdown_files_in(dir: &Path) -> Result<Vec<WorkItemFile>, kernel::Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => {
            return Err(kernel::Error::Failed(format!(
                "could not list {}: {error}",
                dir.display()
            )));
        }
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path.extension().and_then(std::ffi::OsStr::to_str)
                    == Some("md")
        })
        .collect();
    paths.sort();
    Ok(paths
        .into_iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(&path).ok()?;
            Some(WorkItemFile { path, content })
        })
        .collect())
}

impl WorkItemFiles for FilesystemWorkItemFiles {
    fn files(&self) -> Result<Vec<WorkItemFile>, kernel::Error> {
        let mut files = self.canonical()?;
        files.extend(markdown_files_in(&drafts_dir(&self.work_dir))?);
        Ok(files)
    }

    fn canonical(&self) -> Result<Vec<WorkItemFile>, kernel::Error> {
        markdown_files_in(&self.work_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::FilesystemLister;
    use work::resolve::DirectoryLister;

    #[test]
    fn lists_filenames_in_a_real_directory() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("0001-foo.md"), "")?;
        std::fs::write(dir.path().join("0002-bar.md"), "")?;
        let lister = FilesystemLister::new(dir.path());
        let mut names = lister.filenames();
        names.sort();
        assert_eq!(names, vec!["0001-foo.md", "0002-bar.md"]);
        Ok(())
    }

    #[test]
    fn a_missing_directory_yields_an_empty_list() {
        let lister = FilesystemLister::new(std::path::Path::new(
            "/nonexistent/definitely/not/here",
        ));
        assert!(lister.filenames().is_empty());
    }
}
