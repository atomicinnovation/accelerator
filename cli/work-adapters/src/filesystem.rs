//! The `std::fs::read_dir`-backed [`work::resolve::DirectoryLister`] and
//! [`work::work_item_files::WorkItemFiles`] implementations.
//!
//! This crate's pure-read adapter module, isolated (via `cli/pup.ron`'s
//! `work_adapters_filesystem_reads_in_process` rule) from the
//! subprocess-spawning modules later phases add.

use std::path::Path;
use std::path::PathBuf;

use work::resolve::DirectoryLister;
use work::work_item_files::ReadFailure;
use work::work_item_files::UnreadableFile;
use work::work_item_files::WorkItemFile;
use work::work_item_files::WorkItemFiles;
use work::work_item_files::WorkItemListing;
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

/// The `*.md` files directly inside `dir`, sorted by path.
fn markdown_files_in(dir: &Path) -> Result<WorkItemListing, kernel::Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(WorkItemListing::default());
        }
        Err(error) => {
            return Err(kernel::Error::Failed(format!(
                "could not list {}: {error}",
                dir.display()
            )));
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| {
                kernel::Error::Failed(format!(
                    "could not list {}: {error}",
                    dir.display()
                ))
            })?
            .path();
        if is_markdown_file_or_dangling_link(&path) {
            paths.push(path);
        }
    }
    paths.sort();
    let mut listing = WorkItemListing::default();
    for path in paths {
        match std::fs::read_to_string(&path) {
            Ok(content) => listing.files.push(WorkItemFile { path, content }),
            Err(error) => listing.unreadable.push(UnreadableFile {
                path,
                reason: if error.kind() == std::io::ErrorKind::InvalidData {
                    ReadFailure::Undecodable
                } else {
                    ReadFailure::Io(error.to_string())
                },
            }),
        }
    }
    Ok(listing)
}

/// A dangling link is kept so its read fails visibly: it may be a work
/// item whose file was moved away.
fn is_markdown_file_or_dangling_link(path: &Path) -> bool {
    let is_markdown =
        path.extension().and_then(std::ffi::OsStr::to_str) == Some("md");
    let is_dangling_link = path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
        && std::fs::metadata(path).is_err();
    is_markdown && (path.is_file() || is_dangling_link)
}

impl WorkItemFiles for FilesystemWorkItemFiles {
    fn listing(&self) -> Result<WorkItemListing, kernel::Error> {
        let mut listing = markdown_files_in(&self.work_dir)?;
        let drafts = markdown_files_in(&drafts_dir(&self.work_dir))?;
        listing.files.extend(drafts.files);
        listing.unreadable.extend(drafts.unreadable);
        Ok(listing)
    }

    fn canonical(&self) -> Result<Vec<WorkItemFile>, kernel::Error> {
        markdown_files_in(&self.work_dir).map(|listing| listing.files)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::FilesystemLister;
    use super::FilesystemWorkItemFiles;
    use work::resolve::DirectoryLister;
    use work::work_item_files::ReadFailure;
    use work::work_item_files::WorkItemFiles;

    #[test]
    fn an_unreadable_markdown_file_is_listed_as_unreadable(
    ) -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let sealed = dir.path().join("0002-sealed.md");
        std::fs::write(&sealed, "---\nid: \"0002\"\n---\n")?;
        std::fs::write(
            dir.path().join("0001-open.md"),
            "---\nid: \"0001\"\n---\n",
        )?;
        let Some(_guard) = store::test_support::UnreadableFile::new(&sealed)?
        else {
            return Ok(());
        };

        let listing = FilesystemWorkItemFiles::new(dir.path())
            .listing()
            .expect("the directory lists");

        assert_eq!(listing.files.len(), 1);
        assert_eq!(listing.unreadable.len(), 1);
        assert_eq!(listing.unreadable[0].path, sealed);
        assert!(matches!(listing.unreadable[0].reason, ReadFailure::Io(_)));
        Ok(())
    }

    #[test]
    fn a_dangling_markdown_link_is_listed_as_unreadable(
    ) -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let dangling = dir.path().join("0005-moved.md");
        std::os::unix::fs::symlink(dir.path().join("gone.md"), &dangling)?;
        std::fs::create_dir(dir.path().join("archive.md"))?;

        let listing = FilesystemWorkItemFiles::new(dir.path())
            .listing()
            .expect("the directory lists");

        assert!(listing.files.is_empty());
        assert_eq!(listing.unreadable.len(), 1);
        assert_eq!(listing.unreadable[0].path, dangling);
        assert!(matches!(listing.unreadable[0].reason, ReadFailure::Io(_)));
        Ok(())
    }

    #[test]
    fn an_invalid_utf8_markdown_file_is_listed_as_undecodable(
    ) -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        let garbled = dir.path().join("0002-garbled.md");
        std::fs::write(&garbled, [0xff, 0xfe, b'\n'])?;

        let listing = FilesystemWorkItemFiles::new(dir.path())
            .listing()
            .expect("the directory lists");

        assert!(listing.files.is_empty());
        assert_eq!(listing.unreadable.len(), 1);
        assert_eq!(listing.unreadable[0].reason, ReadFailure::Undecodable);
        Ok(())
    }

    #[test]
    fn files_still_omits_an_unreadable_markdown_file(
    ) -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        std::fs::write(
            dir.path().join("0001-open.md"),
            "---\nid: \"0001\"\n---\n",
        )?;
        std::fs::write(dir.path().join("0002-garbled.md"), [0xff, 0xfe])?;
        let drafts = dir.path().join("drafts");
        std::fs::create_dir(&drafts)?;
        std::fs::write(drafts.join("draft-k7mq3x-x.md"), [0xff])?;

        let source = FilesystemWorkItemFiles::new(dir.path());
        let files = source.files().expect("the directory lists");
        let listing = source.listing().expect("the directory lists");

        assert_eq!(files.len(), 1);
        assert_eq!(listing.unreadable.len(), 2);
        Ok(())
    }

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
