//! The filesystem adapter behind the `WorkItemFiles` port: which files under
//! the work directory it yields, and in what order.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use work::work_item_files::WorkItemFiles;
use work_adapters::filesystem::drafts_dir;
use work_adapters::filesystem::FilesystemWorkItemFiles;

type TestError = Box<dyn std::error::Error>;

fn write(path: &Path, content: &str) -> Result<(), TestError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn names(paths: impl IntoIterator<Item = PathBuf>, root: &Path) -> Vec<String> {
    paths
        .into_iter()
        .filter_map(|path| {
            path.strip_prefix(root)
                .ok()
                .map(|relative| relative.display().to_string())
        })
        .collect()
}

#[test]
fn discovery_includes_markdown_files_in_the_drafts_directory(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let work_dir = dir.path();
    write(&work_dir.join("0002-b.md"), "two")?;
    write(&work_dir.join("0001-a.md"), "one")?;
    write(&drafts_dir(work_dir).join("draft-k7mq3x-c.md"), "draft")?;

    let files = FilesystemWorkItemFiles::new(work_dir).files()?;

    assert_eq!(
        names(files.iter().map(|file| file.path.clone()), work_dir),
        vec!["0001-a.md", "0002-b.md", "drafts/draft-k7mq3x-c.md"]
    );
    assert_eq!(files[2].content, "draft");
    Ok(())
}

#[test]
fn discovery_ignores_non_markdown_and_nested_directories_below_drafts(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let work_dir = dir.path();
    let drafts = drafts_dir(work_dir);
    write(&drafts.join("draft-k7mq3x-c.md"), "draft")?;
    write(&drafts.join("notes.txt"), "not an item")?;
    write(&drafts.join("nested/draft-p4tw9z-d.md"), "too deep")?;
    write(&work_dir.join("README.txt"), "not an item")?;

    let files = FilesystemWorkItemFiles::new(work_dir).files()?;

    assert_eq!(
        names(files.into_iter().map(|file| file.path), work_dir),
        vec!["drafts/draft-k7mq3x-c.md"]
    );
    Ok(())
}

#[test]
fn canonical_files_exclude_the_drafts_directory() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let work_dir = dir.path();
    write(&work_dir.join("0001-a.md"), "one")?;
    write(&drafts_dir(work_dir).join("draft-k7mq3x-c.md"), "draft")?;

    let files = FilesystemWorkItemFiles::new(work_dir).canonical()?;

    assert_eq!(
        names(files.into_iter().map(|file| file.path), work_dir),
        vec!["0001-a.md"]
    );
    Ok(())
}

#[test]
fn a_missing_drafts_directory_yields_only_canonical_items(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let work_dir = dir.path();
    write(&work_dir.join("0001-a.md"), "one")?;

    let files = FilesystemWorkItemFiles::new(work_dir).files()?;

    assert_eq!(
        names(files.into_iter().map(|file| file.path), work_dir),
        vec!["0001-a.md"]
    );
    Ok(())
}

#[test]
fn a_missing_work_directory_yields_no_items() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;

    let files =
        FilesystemWorkItemFiles::new(&dir.path().join("absent")).files()?;

    assert!(files.is_empty());
    Ok(())
}
