//! In-process answer to "is this one repo-relative path tracked?" — git via
//! the index, jj via the working-copy commit's tree, matching
//! `git ls-files --error-unmatch` / `jj file list`.
//!
//! The git side reads the index rather than a commit, so a path that is staged
//! but not yet committed reads as tracked — matching `git ls-files`. The jj
//! side reads the working-copy commit's recorded tree with no fresh snapshot,
//! so nothing is written to the backend and no operation is persisted.

use std::path::Path;

use jj_lib::config::StackedConfig;
use jj_lib::repo::Repo as _;
use jj_lib::repo::StoreFactories;
use jj_lib::repo_path::RepoPath;
use jj_lib::settings::UserSettings;
use jj_lib::workspace::default_working_copy_factories;
use jj_lib::workspace::DefaultWorkspaceLoaderFactory;
use jj_lib::workspace::WorkspaceLoaderFactory as _;

use crate::library::Error;

pub(super) fn git_is_tracked(
    root: &Path,
    relpath: &str,
) -> Result<bool, Error> {
    let repository = gix::open(root).map_err(|error| Error::Git {
        path: root.to_path_buf(),
        source: Box::new(error),
    })?;
    // `index_or_empty`, not `index`: a repository with no index file yet tracks
    // nothing, which is an answer rather than a failure.
    let index = repository.index_or_empty().map_err(|error| Error::Git {
        path: root.to_path_buf(),
        source: Box::new(error),
    })?;
    Ok(index.entry_by_path(relpath.into()).is_some())
}

pub(super) fn jj_is_tracked(root: &Path, relpath: &str) -> Result<bool, Error> {
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
    let workspace = loader
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
        .map_err(|error| Error::Jj {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;

    let name = workspace.workspace_name().to_owned();
    let Some(wc_commit_id) = repo.view().get_wc_commit_id(&name).cloned()
    else {
        return Ok(false);
    };
    let wc_commit =
        repo.store()
            .get_commit(&wc_commit_id)
            .map_err(|error| Error::Jj {
                path: root.to_path_buf(),
                source: Box::new(error),
            })?;

    let repo_path =
        RepoPath::from_internal_string(relpath).map_err(|error| Error::Jj {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;
    let value = pollster::block_on(wc_commit.tree().path_value(repo_path))
        .map_err(|error| Error::Jj {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;
    Ok(value.is_present())
}
