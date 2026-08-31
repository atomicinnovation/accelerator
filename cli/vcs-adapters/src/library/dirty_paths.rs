//! In-process enumeration of every repo-relative path that differs from the
//! last committed tree — git via `gix::Repository::status`, jj via the shared
//! working-copy snapshot in [`crate::library::snapshot`].
//!
//! Both sides count untracked files and exclude ignored ones: git's dirwalk is
//! asked for `UntrackedFiles::Files`, and jj auto-tracks. `.gitignore` is
//! honoured by each backend's own walk, so neither reports build output.
//!
//! Neither side shells out.

use std::path::Path;

use crate::library::snapshot;
use crate::library::Error;

pub(super) fn git_dirty_paths(root: &Path) -> Result<Vec<String>, Error> {
    let repository = gix::open(root).map_err(|error| Error::Git {
        path: root.to_path_buf(),
        source: Box::new(error),
    })?;
    let status = repository
        .status(gix::progress::Discard)
        .map_err(|error| Error::Git {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?
        .untracked_files(gix::status::UntrackedFiles::Files);
    let iter = status.into_iter(Vec::<gix::bstr::BString>::new()).map_err(
        |error| Error::Git {
            path: root.to_path_buf(),
            source: Box::new(error),
        },
    )?;

    let mut paths = Vec::new();
    for item in iter {
        let item = item.map_err(|error| Error::Git {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;
        paths.push(String::from_utf8_lossy(item.location()).into_owned());
    }
    Ok(paths)
}

/// The paths the jj working-copy snapshot reports as changed, tree-valued
/// entries (gitlinks, submodules) already excluded by the snapshot's keep
/// predicate. `Ok(Vec::new())` when the workspace has no working-copy commit.
pub(super) fn jj_dirty_paths(root: &Path) -> Result<Vec<String>, Error> {
    let Some(snapshot) = snapshot::working_copy_diff(root)? else {
        return Ok(Vec::new());
    };
    Ok(snapshot
        .changes
        .into_iter()
        .map(|entry| entry.path)
        .collect())
}
