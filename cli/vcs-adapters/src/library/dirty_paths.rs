//! In-process enumeration of every repo-relative path that differs from the
//! last committed tree — git via `gix::Repository::status`, jj via a real
//! `jj-lib` snapshot (matching what the `jj` binary itself does before a
//! diff) followed by a tree-diff against the working-copy commit's parent.
//!
//! Neither side shells out. The jj side writes tree/blob objects into the
//! backend as an unavoidable consequence of computing the new tree's id
//! (exactly as `jj diff` does) but never calls `LockedWorkspace::finish` —
//! no operation or working-copy state is persisted, so `jj op log` and `@`
//! are unchanged afterward.

use std::path::Path;

use jj_lib::config::StackedConfig;
use jj_lib::gitignore::GitIgnoreFile;
use jj_lib::matchers::EverythingMatcher;
use jj_lib::matchers::NothingMatcher;
use jj_lib::merge::MergedTreeValue;
use jj_lib::merged_tree::TreeDiffIterator;
use jj_lib::repo::Repo as _;
use jj_lib::repo::StoreFactories;
use jj_lib::settings::UserSettings;
use jj_lib::working_copy::SnapshotOptions;
use jj_lib::workspace::default_working_copy_factories;
use jj_lib::workspace::DefaultWorkspaceLoaderFactory;
use jj_lib::workspace::WorkspaceLoaderFactory as _;

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
        .untracked_files(gix::status::UntrackedFiles::None);
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

/// Snapshots the jj working copy (detecting on-disk changes since the last
/// recorded operation, honouring auto-track exactly as `jj snapshot.auto-track
/// = "all()"` does) and diffs the result against the working-copy commit's
/// parent tree — the same two trees `jj diff` compares.
///
/// Runs with a settings-free-of-user-config `UserSettings` (jj-lib's own
/// bundled defaults only), which is deliberate: it forces
/// `fsmonitor.backend = "none"`, sidestepping a hard failure on a Watchman
/// config this build was not compiled to support, at the cost of ignoring the
/// user's own `snapshot.max-new-file-size`/`snapshot.auto-track` — both are
/// set explicitly below instead.
pub(super) fn jj_dirty_paths(root: &Path) -> Result<Vec<String>, Error> {
    let settings = UserSettings::from_config(StackedConfig::with_defaults())
        .map_err(|error| Error::Jj {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;
    let loader =
        DefaultWorkspaceLoaderFactory
            .create(root)
            .map_err(|error| Error::Jj {
                path: root.to_path_buf(),
                source: Box::new(error),
            })?;
    let mut workspace = loader
        .load(
            &settings,
            &StoreFactories::default(),
            &default_working_copy_factories(),
        )
        .map_err(|error| Error::Jj {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;

    let repo = pollster::block_on(workspace.repo_loader().load_at_head())
        .map_err(|error| Error::JjDirtyPaths {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;

    let name = workspace.workspace_name().to_owned();
    let Some(wc_commit_id) = repo.view().get_wc_commit_id(&name).cloned()
    else {
        return Ok(Vec::new());
    };
    let wc_commit =
        repo.store().get_commit(&wc_commit_id).map_err(|error| {
            Error::JjDirtyPaths {
                path: root.to_path_buf(),
                source: Box::new(error),
            }
        })?;
    let parent_tree = pollster::block_on(wc_commit.parent_tree(repo.as_ref()))
        .map_err(|error| Error::JjDirtyPaths {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;

    let mut locked_ws = pollster::block_on(
        workspace.start_working_copy_mutation(),
    )
    .map_err(|error| Error::JjDirtyPaths {
        path: root.to_path_buf(),
        source: Box::new(error),
    })?;
    let options = SnapshotOptions {
        base_ignores: GitIgnoreFile::empty(),
        progress: None,
        start_tracking_matcher: &EverythingMatcher,
        force_tracking_matcher: &NothingMatcher,
        max_new_file_size: u64::MAX,
    };
    let (new_tree, _stats) = pollster::block_on(
        locked_ws.locked_wc().snapshot(&options),
    )
    .map_err(|error| Error::JjDirtyPaths {
        path: root.to_path_buf(),
        source: Box::new(error),
    })?;
    // Deliberately not `locked_ws.finish(...)` — see the module doc comment.
    drop(locked_ws);

    let keep = |value: &MergedTreeValue| value.is_present() && !value.is_tree();
    let mut paths = Vec::new();
    for entry in
        TreeDiffIterator::new(&parent_tree, &new_tree, &EverythingMatcher)
    {
        let values = entry.values.map_err(|error| Error::JjDirtyPaths {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;
        if keep(&values.before) || keep(&values.after) {
            paths.push(entry.path.as_internal_file_string().to_owned());
        }
    }
    Ok(paths)
}
