//! Repository-root discovery.
//!
//! Ports `find_repo_root` from
//! [`scripts/vcs-common.sh`](../../../../scripts/vcs-common.sh) (lines 8-18)
//! and `config_project_root` from
//! [`scripts/config-common.sh`](../../../../scripts/config-common.sh)
//! (lines 16-18). No realpath normalisation, matching bash `$PWD`.

use std::path::{Path, PathBuf};

/// Walk `start` and its ancestors looking for a `.git` or `.jj` directory,
/// returning the nearest match. Returns `None` if none is found.
///
/// Mirrors the bash loop exactly: the marker check runs `while [ "$dir" !=
/// "/" ]`, so the filesystem root `/` is **never** checked — a repo located
/// at `/` (i.e. `/.git`) is not matched. A `Path::ancestors()` walk that
/// included `/` would diverge here, so this is pinned by a test.
pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut dir: &Path = start;
    let root = Path::new("/");
    while dir != root {
        if dir.join(".jj").is_dir() || dir.join(".git").is_dir() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
    None
}

/// The project root: the nearest VCS root at or above `start`, else `start`
/// itself (the `$PWD` fallback in `config_project_root`).
pub fn project_root(start: &Path) -> PathBuf {
    find_repo_root(start).unwrap_or_else(|| start.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_git_marker_at_start() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        // No realpath normalisation: the start path is returned verbatim.
        assert_eq!(find_repo_root(tmp.path()).as_deref(), Some(tmp.path()));
    }

    #[test]
    fn finds_jj_marker_in_ancestor() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".jj")).unwrap();
        let nested = tmp.path().join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(find_repo_root(&nested).as_deref(), Some(tmp.path()));
    }

    #[test]
    fn marker_must_be_a_directory_not_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        // A regular file named .git (e.g. a gitlink) is not matched: bash
        // uses `[ -d ]`.
        fs::write(tmp.path().join(".git"), "gitdir: /elsewhere").unwrap();
        assert_eq!(find_repo_root(tmp.path()), None);
    }

    #[test]
    fn no_marker_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("x/y");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(find_repo_root(&nested), None);
    }

    #[test]
    fn project_root_falls_back_to_start_when_no_marker() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(project_root(tmp.path()), tmp.path());
    }

    #[test]
    fn marker_at_filesystem_root_is_not_matched() {
        // The bash walk stops before `/`, so a `/.git` (if it existed) would
        // not be matched and the search falls through to None. We can't
        // create `/.git`, but we can assert the walk never *returns* `/`:
        // starting from `/` directly yields None because the loop body never
        // runs.
        assert_eq!(find_repo_root(Path::new("/")), None);
    }
}
