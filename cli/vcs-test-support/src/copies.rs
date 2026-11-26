//! Copying part of a shared fixture tree into a test's own directory, for a
//! test that changes the states it reads.

use std::fmt;
use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use filetime::FileTime;

use crate::path_scan::files_naming;
use crate::Error;

const OWNER_WRITE: u32 = 0o200;

/// A copy of part of a shared tree, with every copied file that still names
/// the shared tree it came from.
#[derive(Debug)]
pub struct TreeCopy {
    pub root: PathBuf,
    pub named_by: Vec<PathBuf>,
}

impl TreeCopy {
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.named_by.is_empty()
    }

    /// Fails unless the copy is clean, naming every file that still names
    /// its source.
    ///
    /// # Errors
    ///
    /// When any copied file still names the source.
    pub fn require_clean(
        &self,
        copied: impl fmt::Display,
        source: impl fmt::Display,
    ) -> Result<(), Error> {
        if self.is_clean() {
            return Ok(());
        }
        Err(Error::message(format!(
            "{copied} in {} still names {source} in: {}",
            self.root.display(),
            self.named_by
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

/// Copies each of `entries` from `shared_tree` into `destination`.
///
/// Every copied entry keeps its source's contents and times to the
/// nanosecond, so jj and git see their recorded working-copy state as
/// unchanged. Each keeps its source's mode too, plus its owner's write
/// permission: a shared tree is sealed read-only, and the copy exists to be
/// written.
///
/// # Errors
///
/// When an entry is missing, or the copy cannot be written or scanned.
pub fn copy_part(
    shared_tree: &Path,
    entries: &[&str],
    destination: &Path,
) -> Result<TreeCopy, Error> {
    for entry in entries {
        let source = shared_tree.join(entry);
        let copy = destination.join(entry);
        copy_entry(&source, &copy).map_err(|error| {
            Error::message(format!(
                "copying {} to {}: {error}",
                source.display(),
                copy.display()
            ))
        })?;
    }
    let mut named_by = Vec::new();
    for entry in entries {
        named_by.extend(files_naming(&destination.join(entry), shared_tree)?);
    }
    Ok(TreeCopy {
        root: destination.to_path_buf(),
        named_by,
    })
}

/// Copies every top-level entry of `template`, a tree the test built for
/// itself, into `destination`, as [`copy_part`] copies a shared tree's.
///
/// Copying logs nothing, so the fixture-build guard,
/// `tests/unit/tasks/test_fixture_builds.py`, permits a call only from the
/// files it lists, each copying a template its own test built.
///
/// # Errors
///
/// When the template cannot be listed or copied, or a copied file still
/// names the template.
pub fn copy_private_tree(
    template: &Path,
    destination: &Path,
) -> Result<PathBuf, Error> {
    let entries = top_level_entries(template)?;
    let names: Vec<&str> = entries.iter().map(String::as_str).collect();
    let copy = copy_part(template, &names, destination)?;
    copy.require_clean(
        "the copy",
        format_args!("its template {}", template.display()),
    )?;
    Ok(copy.root)
}

fn top_level_entries(tree: &Path) -> Result<Vec<String>, Error> {
    let mut names = Vec::new();
    for entry in fs::read_dir(tree)? {
        let name = entry?.file_name();
        names.push(name.into_string().map_err(|name| {
            Error::message(format!(
                "non-utf8 entry {} in {}",
                name.display(),
                tree.display()
            ))
        })?);
    }
    names.sort();
    Ok(names)
}

fn copy_entry(source: &Path, copy: &Path) -> Result<(), Error> {
    let metadata = source.symlink_metadata()?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        symlink(fs::read_link(source)?, copy)?;
        filetime::set_symlink_file_times(
            copy,
            FileTime::from_last_access_time(&metadata),
            FileTime::from_last_modification_time(&metadata),
        )?;
        return Ok(());
    }
    if file_type.is_dir() && cloned_whole(source, copy)? {
        return finish_a_clone_as_a_copy(source, copy);
    }
    if file_type.is_dir() {
        fs::create_dir(copy)?;
        for child in fs::read_dir(source)? {
            let name = child?.file_name();
            copy_entry(&source.join(&name), &copy.join(&name))?;
        }
    } else {
        fs::copy(source, copy)?;
    }
    fs::set_permissions(
        copy,
        fs::Permissions::from_mode(metadata.permissions().mode() | OWNER_WRITE),
    )?;
    filetime::set_file_times(
        copy,
        FileTime::from_last_access_time(&metadata),
        FileTime::from_last_modification_time(&metadata),
    )?;
    Ok(())
}

/// Leaves a directory clone as an entry-by-entry copy would: every entry
/// writable by its owner, and every directory back at its source's times.
///
/// A clone keeps each entry's contents, mode and times except a directory's
/// modification time, which it sets to the moment of the clone.
fn finish_a_clone_as_a_copy(source: &Path, copy: &Path) -> Result<(), Error> {
    for entry in walkdir::WalkDir::new(copy) {
        let entry = entry.map_err(|error| Error::message(error.to_string()))?;
        if entry.path_is_symlink() {
            continue;
        }
        let mode = entry
            .metadata()
            .map_err(|error| Error::message(error.to_string()))?
            .permissions()
            .mode();
        if mode & OWNER_WRITE == 0 {
            fs::set_permissions(
                entry.path(),
                fs::Permissions::from_mode(mode | OWNER_WRITE),
            )?;
        }
        if entry.file_type().is_dir() {
            let relative = entry
                .path()
                .strip_prefix(copy)
                .map_err(|error| Error::message(error.to_string()))?;
            let original = source.join(relative).symlink_metadata()?;
            filetime::set_file_times(
                entry.path(),
                FileTime::from_last_access_time(&original),
                FileTime::from_last_modification_time(&original),
            )?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn cloned_whole(source: &Path, copy: &Path) -> Result<bool, Error> {
    use rustix::fs::fclonefileat;
    use rustix::fs::CloneFlags;
    use rustix::fs::Mode;
    use rustix::fs::OFlags;
    use rustix::io::Errno;

    let (Some(parent), Some(name)) = (copy.parent(), copy.file_name()) else {
        return Ok(false);
    };
    let open = |path: &Path| {
        rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY,
            Mode::empty(),
        )
        .map_err(std::io::Error::from)
    };
    match fclonefileat(open(source)?, open(parent)?, name, CloneFlags::NOFOLLOW)
    {
        Ok(()) => Ok(true),
        Err(Errno::XDEV | Errno::NOTSUP) => Ok(false),
        Err(error) => Err(std::io::Error::from(error).into()),
    }
}

#[cfg(not(target_os = "macos"))]
fn cloned_whole(_source: &Path, _copy: &Path) -> Result<bool, Error> {
    Ok(false)
}
