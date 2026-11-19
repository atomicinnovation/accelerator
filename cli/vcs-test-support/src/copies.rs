//! Copying part of a shared fixture tree into a test's own directory, for a
//! test that changes the states it reads.

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
    let mut named_by = Vec::new();
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
        named_by.extend(files_naming(&copy, shared_tree)?);
    }
    Ok(TreeCopy {
        root: destination.to_path_buf(),
        named_by,
    })
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
