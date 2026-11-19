use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use filetime::FileTime;
use vcs_test_support::copies::copy_part;
use walkdir::WalkDir;

type TestError = Box<dyn std::error::Error>;

const OWNER_WRITE: u32 = 0o200;

struct Source {
    directory: tempfile::TempDir,
}

impl Source {
    fn new() -> Result<Self, TestError> {
        Ok(Self {
            directory: tempfile::tempdir()?,
        })
    }

    fn tree(&self) -> Result<PathBuf, TestError> {
        Ok(self.directory.path().canonicalize()?)
    }

    fn write(&self, relative: &str, contents: &str) -> Result<(), TestError> {
        let path = self.tree()?.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
        Ok(())
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        for entry in WalkDir::new(self.directory.path()).into_iter().flatten() {
            if entry.file_type().is_dir() {
                let _ = fs::set_permissions(
                    entry.path(),
                    fs::Permissions::from_mode(0o755),
                );
            }
        }
    }
}

fn set_mode(path: &Path, mode: u32) -> Result<(), TestError> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

fn stamp_every_entry_distinctly(tree: &Path) -> Result<(), TestError> {
    let entries: Vec<_> = WalkDir::new(tree)
        .contents_first(true)
        .into_iter()
        .collect::<Result<_, _>>()?;
    for (index, entry) in entries.iter().enumerate() {
        let offset = i64::try_from(index)?;
        let time =
            FileTime::from_unix_time(1_600_000_000 + offset, 123_456_789);
        if entry.file_type().is_symlink() {
            filetime::set_symlink_file_times(entry.path(), time, time)?;
        } else {
            filetime::set_file_times(entry.path(), time, time)?;
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Observed {
    relative: PathBuf,
    kind: &'static str,
    contents: Vec<u8>,
    mode: Option<u32>,
    modified: FileTime,
}

fn observe(tree: &Path) -> Result<Vec<Observed>, TestError> {
    let mut observed = Vec::new();
    for entry in WalkDir::new(tree).sort_by_file_name() {
        let entry = entry?;
        let metadata = entry.path().symlink_metadata()?;
        let file_type = metadata.file_type();
        let (kind, contents, mode) = if file_type.is_symlink() {
            let target = fs::read_link(entry.path())?;
            (
                "symlink",
                target.into_os_string().into_encoded_bytes(),
                None,
            )
        } else if file_type.is_dir() {
            ("directory", Vec::new(), Some(metadata.permissions().mode()))
        } else {
            (
                "file",
                fs::read(entry.path())?,
                Some(metadata.permissions().mode()),
            )
        };
        observed.push(Observed {
            relative: entry.path().strip_prefix(tree)?.to_path_buf(),
            kind,
            contents,
            mode: mode.map(|mode| mode & 0o7777),
            modified: FileTime::from_last_modification_time(&metadata),
        });
    }
    Ok(observed)
}

fn with_owner_write(observed: Vec<Observed>) -> Vec<Observed> {
    observed
        .into_iter()
        .map(|entry| Observed {
            mode: entry.mode.map(|mode| mode | OWNER_WRITE),
            ..entry
        })
        .collect()
}

#[test]
fn a_copy_keeps_each_entry_s_contents_times_and_mode_and_lets_its_owner_write(
) -> Result<(), TestError> {
    let source = Source::new()?;
    let tree = source.tree()?;
    source.write("part/top.txt", "top\n")?;
    source.write("part/nested/deeper/file.txt", "deep\n")?;
    source.write("part/run.sh", "#!/bin/sh\n")?;
    set_mode(&tree.join("part/run.sh"), 0o750)?;
    symlink("nested/deeper/file.txt", tree.join("part/link"))?;
    source.write("part/sealed/inside.txt", "inside\n")?;
    set_mode(&tree.join("part/sealed/inside.txt"), 0o444)?;
    stamp_every_entry_distinctly(&tree.join("part"))?;
    set_mode(&tree.join("part/sealed"), 0o555)?;
    let destination = tempfile::tempdir()?;

    copy_part(&tree, &["part"], destination.path())?;

    assert_eq!(
        observe(&destination.path().join("part"))?,
        with_owner_write(observe(&tree.join("part"))?)
    );
    Ok(())
}

#[test]
fn a_copy_of_a_sealed_tree_can_be_written() -> Result<(), TestError> {
    let source = Source::new()?;
    let tree = source.tree()?;
    source.write("part/sealed/inside.txt", "inside\n")?;
    set_mode(&tree.join("part/sealed/inside.txt"), 0o444)?;
    set_mode(&tree.join("part/sealed"), 0o555)?;
    set_mode(&tree.join("part"), 0o555)?;
    let destination = tempfile::tempdir()?;

    copy_part(&tree, &["part"], destination.path())?;

    let copied = destination.path().join("part/sealed");
    fs::write(copied.join("inside.txt"), "changed\n")?;
    fs::write(copied.join("added.txt"), "added\n")?;
    Ok(())
}

#[test]
fn only_the_named_entries_are_copied() -> Result<(), TestError> {
    let source = Source::new()?;
    let tree = source.tree()?;
    source.write("wanted/a.txt", "a\n")?;
    source.write("unwanted/b.txt", "b\n")?;
    let destination = tempfile::tempdir()?;

    copy_part(&tree, &["wanted"], destination.path())?;

    assert!(destination.path().join("wanted/a.txt").is_file());
    assert!(!destination.path().join("unwanted").exists());
    Ok(())
}

#[test]
fn a_missing_entry_fails_naming_it() -> Result<(), TestError> {
    let source = Source::new()?;
    let destination = tempfile::tempdir()?;

    let error = copy_part(&source.tree()?, &["absent"], destination.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("absent"), "{error}");
    Ok(())
}

#[test]
fn a_planted_path_is_reported() -> Result<(), TestError> {
    let source = Source::new()?;
    let tree = source.tree()?;
    source.write("part/config", &format!("url = {}\n", tree.display()))?;
    source.write("part/innocent", "nothing here\n")?;
    let destination = tempfile::tempdir()?;

    let copy = copy_part(&tree, &["part"], destination.path())?;

    assert_eq!(copy.named_by, vec![destination.path().join("part/config")]);
    assert!(!copy.is_clean());
    Ok(())
}

#[test]
fn a_copy_naming_no_path_is_clean() -> Result<(), TestError> {
    let source = Source::new()?;
    let tree = source.tree()?;
    source.write("part/innocent", "nothing here\n")?;
    let destination = tempfile::tempdir()?;

    let copy = copy_part(&tree, &["part"], destination.path())?;

    assert_eq!(copy.root, destination.path());
    assert!(copy.named_by.is_empty());
    assert!(copy.is_clean());
    Ok(())
}
