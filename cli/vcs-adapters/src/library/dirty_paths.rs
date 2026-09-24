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

use crate::library::is_unborn_head;
use crate::library::snapshot;
use crate::library::Error;
use crate::library::WorkingCopyState;

pub(super) fn git_working_copy_state(
    root: &Path,
) -> Result<WorkingCopyState, Error> {
    let repository = gix::open(root).map_err(|error| Error::Git {
        path: root.to_path_buf(),
        source: Box::new(error),
    })?;
    let base_commits = match repository.head_commit() {
        Ok(commit) => vec![commit.id().to_string()],
        Err(error) if is_unborn_head(&error) => Vec::new(),
        Err(error) => {
            return Err(Error::Git {
                path: root.to_path_buf(),
                source: Box::new(error),
            })
        }
    };
    Ok(WorkingCopyState {
        base_commits,
        dirty_paths: git_dirty_paths(root, &repository)?,
    })
}

fn git_dirty_paths(
    root: &Path,
    repository: &gix::Repository,
) -> Result<Vec<String>, Error> {
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
/// predicate, and the parents they are diffed against. Empty when the
/// workspace has no working-copy commit.
pub(super) fn jj_working_copy_state(
    root: &Path,
) -> Result<WorkingCopyState, Error> {
    let Some(snapshot) = snapshot::working_copy_diff(root)? else {
        return Ok(WorkingCopyState::default());
    };
    Ok(WorkingCopyState {
        base_commits: snapshot.base_commits,
        dirty_paths: snapshot
            .changes
            .into_iter()
            .map(|entry| entry.path)
            .collect(),
    })
}
