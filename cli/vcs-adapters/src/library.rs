//! The library-backed probe: git through `gix`, jj through `jj-lib`, both read
//! in the calling process rather than by spawning the VCS binaries.
//!
//! Two mechanisms live here and must not be confused. `RepoRoot::discover` is
//! the marker walk followed by nothing else, because the checkout boundary is
//! the start path or its nearest marked ancestor and never an ancestor above
//! it. `gix::discover` deliberately *does* walk past that boundary and is used
//! only where following a recorded link out of the checkout is the answer being
//! asked for. A ceiling cannot enforce the boundary rule — `ceiling_dirs`
//! computes its height as `strip_prefix(ceiling).components().count()` and
//! discards height 0, so a ceiling at the boundary is silently ignored.
//!
//! Every path returned from this module is canonicalised, at the single choke
//! point below. The sources disagree otherwise: `repo_path()` arrives already
//! canonicalised from jj-lib while `workspace_root()` is whatever was passed
//! in, and a linked worktree's `workdir()` is reconstructed from the absolute
//! path git recorded at `git worktree add` time.
//!
//! A cargo-pup rule (`vcs_adapters_library_reads_in_process`) restricts this
//! module's imports to a permit list and denies `std::process`. cargo-pup
//! resolves a grouped `use a::{b, c}` to an empty module name, which the permit
//! list rejects — so every import here is single-item, and must stay that way.

use std::path::Path;
use std::path::PathBuf;

use jj_lib::workspace::DefaultWorkspaceLoaderFactory;
use jj_lib::workspace::WorkspaceLoadError;
use jj_lib::workspace::WorkspaceLoaderFactory as _;
use tracing::warn;
use vcs::RepoRoot;
use vcs::VcsKind;
use vcs::VcsProbe;

use crate::markers::carries_any_marker;
use crate::markers::marker_kind;
use crate::markers::walk_up;

/// Reads a repository's root, idiom and revision in-process.
///
/// Ships unwired: `crate::facts` still composes the subprocess pair, and no
/// feature flag or config switch routes a caller here.
#[derive(Debug, Clone, Copy, Default)]
pub struct InProcessProbe;

impl RepoRoot for InProcessProbe {
    fn discover(&self, start: &Path) -> Option<PathBuf> {
        walk_up(start, carries_any_marker).map(|root| canonical(&root))
    }

    fn repository_root(&self, working_copy_root: &Path) -> PathBuf {
        jj_repository_root(working_copy_root)
            .unwrap_or_else(|| canonical(working_copy_root))
    }
}

impl VcsProbe for InProcessProbe {
    fn kind(&self, root: &Path) -> VcsKind {
        marker_kind(root)
    }

    fn revision(&self, root: &Path, kind: VcsKind) -> Option<String> {
        match kind {
            VcsKind::Git => git_revision(root),
            VcsKind::Jj => {
                warn!(
                    vcs = "jj",
                    "jj-lib 0.43 exposes no read-only, settings-free route to \
                     the working-copy commit id, so no revision is reported"
                );
                None
            }
            VcsKind::None => None,
        }
    }
}

/// The repository a jj working copy belongs to, resolved through the loader so
/// the `.jj/repo`-file-means-secondary rule has one implementation.
///
/// The store is always `<repository>/.jj/repo`, so the repository is its
/// grandparent — for a secondary workspace that is the main repository, and for
/// a main workspace it is the workspace itself. `None` when this is not a jj
/// workspace at all, which is the one absence that does not log.
fn jj_repository_root(working_copy_root: &Path) -> Option<PathBuf> {
    let loader = match DefaultWorkspaceLoaderFactory.create(working_copy_root) {
        Ok(loader) => loader,
        Err(WorkspaceLoadError::NoWorkspaceHere(_)) => return None,
        Err(error) => {
            warn!(%error, "could not load the jj workspace");
            return None;
        }
    };

    let store = loader.repo_path();
    let Some(repository) = store.parent().and_then(Path::parent) else {
        warn!(
            store = %store.display(),
            "the jj store has no repository root above it"
        );
        return None;
    };
    Some(canonical(repository))
}

/// The full working-copy revision of a git checkout. `None` — unlogged — for a
/// repository with no commits yet, since that is a legitimate absence rather
/// than a probe that could not answer.
fn git_revision(root: &Path) -> Option<String> {
    let repository = match gix::discover(root) {
        Ok(repository) => repository,
        Err(error) => {
            warn!(
                vcs = "git",
                %error, "could not open the repository for its revision"
            );
            return None;
        }
    };

    let head = repository.head_commit();
    match head {
        Ok(commit) => Some(commit.id().to_string()),
        Err(error) if is_unborn_head(&error) => None,
        Err(error) => {
            warn!(vcs = "git", %error, "could not read the head commit");
            None
        }
    }
}

/// Whether the head could not be peeled because nothing has been committed yet,
/// as opposed to the repository being unreadable.
const fn is_unborn_head(error: &gix::reference::head_commit::Error) -> bool {
    matches!(
        error,
        gix::reference::head_commit::Error::Head(
            gix::reference::find::existing::Error::NotFound { .. }
        )
    )
}

/// The single choke point every path leaving this module passes through.
///
/// Falls back to the uncanonicalised path rather than dropping the answer: the
/// callers return `Option`/`PathBuf` and cannot carry the distinction, and the
/// paths reaching here have just been read off the filesystem.
fn canonical(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(error) => {
            warn!(
                path = %path.display(),
                %error, "could not canonicalise the path"
            );
            path.to_path_buf()
        }
    }
}
