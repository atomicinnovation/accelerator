//! The ancestor walk and marker reading both adapters share. Each delegates to
//! these rather than to the other, so retiring either strands nothing.

use std::path::Path;
use std::path::PathBuf;

use vcs::VcsKind;

/// The nearest ancestor of `start`, `start` included, that
/// `marks_a_repository` accepts. The filesystem root is never tested.
pub fn walk_up(
    start: &Path,
    marks_a_repository: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let mut dir = start;
    while dir.parent().is_some() {
        if marks_a_repository(dir) {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
    None
}

/// Whether `dir` carries a `.jj` or `.git` marker of any shape.
pub fn carries_any_marker(dir: &Path) -> bool {
    dir.join(".jj").exists() || dir.join(".git").exists()
}

/// Whether `dir` is a jj workspace root.
///
/// The jj queries need this rather than the combined test: jj-lib's loader does
/// not walk, so a boundary found by the `.git`-inclusive walk makes it report
/// absence on a git checkout nested inside a jj workspace.
pub fn carries_jj_marker(dir: &Path) -> bool {
    dir.join(".jj").exists()
}

/// The idiom the markers at `root` call for. `Jj` wins in a colocated checkout,
/// because git's index lags the jj working-copy commit and a git-shaped probe
/// would read live edits as clean.
pub fn marker_kind(root: &Path) -> VcsKind {
    if root.join(".jj").exists() {
        VcsKind::Jj
    } else if root.join(".git").exists() {
        VcsKind::Git
    } else {
        VcsKind::None
    }
}
