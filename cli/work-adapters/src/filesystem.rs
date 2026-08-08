//! The `std::fs::read_dir`-backed [`work::resolve::DirectoryLister`]
//! implementation.
//!
//! This crate's pure-read adapter module, isolated (via `cli/pup.ron`'s
//! `work_adapters_filesystem_reads_in_process` rule) from the
//! subprocess-spawning modules later phases add.

use std::path::Path;
use std::path::PathBuf;

use work::resolve::DirectoryLister;

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
