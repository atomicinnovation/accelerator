//! The ancestor walk and the marker reading both probes share.
//!
//! These are the pieces the subprocess probes and the library-backed probe
//! agree on, so they live apart from either: the subprocess pair delegates to
//! them rather than the other way round, and outlives its deletion.

use std::path::Path;
use std::path::PathBuf;

use vcs::VcsKind;

/// The nearest ancestor of `start`, `start` itself included, that
/// `marks_a_repository` accepts.
///
/// The filesystem root itself is never tested, matching the bash walk.
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

/// Whether `dir` is a checkout boundary — a `.jj` or `.git` marker of any
/// shape, tested for *existence* so a `.git` file counts alongside a directory.
pub fn carries_any_marker(dir: &Path) -> bool {
    dir.join(".jj").exists() || dir.join(".git").exists()
}

/// Whether `dir` is a jj workspace root.
///
/// The jj queries need this in place of the combined boundary test: jj-lib's
/// loader performs no walk of its own, so feeding it a boundary found by the
/// `.git`-inclusive walk makes it report absence on a git checkout nested
/// inside a jj workspace — where `jj workspace root` reports a root.
pub fn carries_jj_marker(dir: &Path) -> bool {
    dir.join(".jj").exists()
}

/// The idiom the markers at `root` call for. `Jj` wins in a colocated
/// checkout, because git's index lags the jj working-copy commit and a
/// git-shaped probe would read live edits as clean.
pub fn marker_kind(root: &Path) -> VcsKind {
    if root.join(".jj").exists() {
        VcsKind::Jj
    } else if root.join(".git").exists() {
        VcsKind::Git
    } else {
        VcsKind::None
    }
}
