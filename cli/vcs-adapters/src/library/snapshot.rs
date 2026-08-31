//! The shared jj working-copy access, and the one place a `UserSettings` is
//! constructed.
//!
//! Snapshotting is not a read of already-recorded state — it is jj-lib
//! re-deriving on-disk changes since the last operation — and that requires a
//! real `TreeStateSettings` (conflict-marker style, eol/exec-bit handling,
//! fsmonitor backend) that only a `UserSettings` supplies. Both `dirty_paths`
//! (the change list) and `status_log` (status and the log walk) reach the
//! working copy through here, so the snapshot path and the `UserSettings` it
//! unavoidably constructs live in exactly one module.
//!
//! Runs with a `UserSettings` carrying jj-lib's own bundled defaults
//! (`StackedConfig::with_defaults`) rather than the user's config, which forces
//! `fsmonitor.backend = "none"` — sidestepping a hard failure on a Watchman
//! config this build was not compiled to support — at the cost of ignoring the
//! user's own `snapshot.max-new-file-size`/`snapshot.auto-track`, both set
//! explicitly below instead.
//!
//! The snapshot writes tree/blob objects into the backend as an unavoidable
//! consequence of computing the new tree's id (exactly as `jj diff` does) but
//! never calls `LockedWorkspace::finish` — no operation or working-copy state is
//! persisted, so `jj op log` and `@` are unchanged afterward.

use std::path::Path;
use std::sync::Arc;

use jj_lib::commit::Commit;
use jj_lib::config::StackedConfig;
use jj_lib::gitignore::GitIgnoreFile;
use jj_lib::matchers::EverythingMatcher;
use jj_lib::matchers::NothingMatcher;
use jj_lib::merge::MergedTreeValue;
use jj_lib::merged_tree::MergedTree;
use jj_lib::merged_tree::TreeDiffIterator;
use jj_lib::ref_name::WorkspaceNameBuf;
use jj_lib::repo::ReadonlyRepo;
use jj_lib::repo::Repo as _;
use jj_lib::repo::StoreFactories;
use jj_lib::settings::UserSettings;
use jj_lib::working_copy::SnapshotOptions;
use jj_lib::workspace::default_working_copy_factories;
use jj_lib::workspace::DefaultWorkspaceLoaderFactory;
use jj_lib::workspace::Workspace;
use jj_lib::workspace::WorkspaceLoaderFactory as _;

use crate::library::Error;

/// One path in the working-copy diff, with the before/after presence the status
/// classifier needs to tell Added from Deleted from Modified.
///
/// Presence encodes the `is_present() && !is_tree()` keep predicate, so a
/// tree-valued entry (gitlink, submodule) is `false` on both sides and excluded
/// exactly as before.
pub(super) struct DiffEntry {
    pub path: String,
    pub before_present: bool,
    pub after_present: bool,
}

/// The working-copy diff against the parent tree, the snapshot tree itself
/// (status reads conflicts from the tree, which the diff cannot express), and
/// the bookmarks on the working-copy commit (byte-sorted).
pub(super) struct WorkingCopySnapshot {
    pub branch: Vec<String>,
    pub changes: Vec<DiffEntry>,
    pub tree: MergedTree,
}

/// The settings-loaded repository and the working-copy commit, for the jj log
/// walk. Holding the repository keeps its store alive for the ancestry peel.
pub(super) struct HeadCommit {
    pub repo: Arc<ReadonlyRepo>,
    pub commit: Commit,
}

struct Loaded {
    workspace: Workspace,
    repo: Arc<ReadonlyRepo>,
    name: WorkspaceNameBuf,
}

fn err_jj<E>(root: &Path) -> impl Fn(E) -> Error + '_
where
    E: std::error::Error + Send + Sync + 'static,
{
    move |source| Error::Jj {
        path: root.to_path_buf(),
        source: Box::new(source),
    }
}

fn err_wc_diff<E>(root: &Path) -> impl Fn(E) -> Error + '_
where
    E: std::error::Error + Send + Sync + 'static,
{
    move |source| Error::JjWorkingCopyDiff {
        path: root.to_path_buf(),
        source: Box::new(source),
    }
}

fn load(root: &Path) -> Result<Loaded, Error> {
    let settings = UserSettings::from_config(StackedConfig::with_defaults())
        .map_err(err_jj(root))?;
    let loader = DefaultWorkspaceLoaderFactory
        .create(root)
        .map_err(err_jj(root))?;
    let workspace = loader
        .load(
            &settings,
            &StoreFactories::default(),
            &default_working_copy_factories(),
        )
        .map_err(err_jj(root))?;
    let repo = pollster::block_on(workspace.repo_loader().load_at_head())
        .map_err(err_wc_diff(root))?;
    let name = workspace.workspace_name().to_owned();
    Ok(Loaded {
        workspace,
        repo,
        name,
    })
}

/// Snapshots the jj working copy and diffs it against the working-copy commit's
/// parent tree — the two trees `jj diff` compares. `Ok(None)` when the workspace
/// has no working-copy commit, preserving the old empty-list behaviour.
pub(super) fn working_copy_diff(
    root: &Path,
) -> Result<Option<WorkingCopySnapshot>, Error> {
    let Loaded {
        mut workspace,
        repo,
        name,
    } = load(root)?;

    let Some(wc_commit_id) = repo.view().get_wc_commit_id(&name).cloned()
    else {
        return Ok(None);
    };
    let mut branch: Vec<String> = repo
        .view()
        .local_bookmarks_for_commit(&wc_commit_id)
        .map(|(bookmark, _)| bookmark.as_str().to_owned())
        .collect();
    branch.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));

    let wc_commit = repo
        .store()
        .get_commit(&wc_commit_id)
        .map_err(err_wc_diff(root))?;
    let parent_tree = pollster::block_on(wc_commit.parent_tree(repo.as_ref()))
        .map_err(err_wc_diff(root))?;

    let mut locked_ws =
        pollster::block_on(workspace.start_working_copy_mutation())
            .map_err(err_wc_diff(root))?;
    let options = SnapshotOptions {
        base_ignores: GitIgnoreFile::empty(),
        progress: None,
        start_tracking_matcher: &EverythingMatcher,
        force_tracking_matcher: &NothingMatcher,
        max_new_file_size: u64::MAX,
    };
    let (new_tree, _stats) =
        pollster::block_on(locked_ws.locked_wc().snapshot(&options))
            .map_err(err_wc_diff(root))?;
    drop(locked_ws);

    let present =
        |value: &MergedTreeValue| value.is_present() && !value.is_tree();
    let mut changes = Vec::new();
    for entry in
        TreeDiffIterator::new(&parent_tree, &new_tree, &EverythingMatcher)
    {
        let values = entry.values.map_err(err_wc_diff(root))?;
        let before_present = present(&values.before);
        let after_present = present(&values.after);
        if before_present || after_present {
            changes.push(DiffEntry {
                path: entry.path.as_internal_file_string().to_owned(),
                before_present,
                after_present,
            });
        }
    }

    Ok(Some(WorkingCopySnapshot {
        branch,
        changes,
        tree: new_tree,
    }))
}

/// The settings-loaded repository and working-copy commit for the jj log walk.
/// `Ok(None)` when the workspace has no working-copy commit.
pub(super) fn head_commit(root: &Path) -> Result<Option<HeadCommit>, Error> {
    let Loaded { repo, name, .. } = load(root)?;
    let Some(wc_commit_id) = repo.view().get_wc_commit_id(&name).cloned()
    else {
        return Ok(None);
    };
    let commit = repo
        .store()
        .get_commit(&wc_commit_id)
        .map_err(err_jj(root))?;
    Ok(Some(HeadCommit { repo, commit }))
}
