//! Which repositories enclose a directory, answered so that a crafted marker
//! can only widen the set or mark it incomplete, never narrow it.
//!
//! Each repository contributes every root a checkout of it may live under: a
//! jj workspace and its repository, a git worktree and its main worktree. A
//! repository whose roots cannot all be determined leaves the answer
//! incomplete rather than silently smaller.

use std::path::Path;
use std::path::PathBuf;

use vcs::tracking::RootsAnswer;
use vcs::VcsKind;

use crate::library::InProcessProbe;
use crate::tracking::enclosing_repositories;
use crate::tracking::Enclosing;
use crate::tracking::FilesystemMarkers;

/// The roots of every repository enclosing `directory`.
pub fn repository_roots(directory: &Path) -> RootsAnswer {
    let mut answer = RootsAnswer {
        roots: Vec::new(),
        complete: true,
    };
    let Ok(directory) = dunce::canonicalize(directory) else {
        answer.complete = false;
        return answer;
    };
    for repository in enclosing_repositories(&directory, &FilesystemMarkers) {
        match repository {
            Enclosing::Repository {
                root,
                kind: VcsKind::Jj,
            } => {
                add(&mut answer, Some(root));
                add(&mut answer, jj_repository_root(&directory));
            }
            Enclosing::Repository {
                root,
                kind: VcsKind::Git | VcsKind::None,
            } => {
                add(&mut answer, Some(root));
                add(&mut answer, git_worktree_root(&directory));
                add(&mut answer, git_main_worktree_root(&directory));
            }
            Enclosing::Unusable => answer.complete = false,
        }
    }
    answer
}

fn add(answer: &mut RootsAnswer, root: Option<PathBuf>) {
    match root {
        Some(root) if answer.roots.contains(&root) => {}
        Some(root) => answer.roots.push(root),
        None => answer.complete = false,
    }
}

fn jj_repository_root(directory: &Path) -> Option<PathBuf> {
    InProcessProbe
        .jj_repository(directory)
        .ok()
        .flatten()
        .map(|facts| facts.main_root)
}

fn git_worktree_root(directory: &Path) -> Option<PathBuf> {
    InProcessProbe.dual_roots(directory).git.ok().flatten()
}

fn git_main_worktree_root(directory: &Path) -> Option<PathBuf> {
    InProcessProbe
        .worktree(directory)
        .ok()
        .flatten()
        .and_then(|facts| facts.main_worktree_root)
}
