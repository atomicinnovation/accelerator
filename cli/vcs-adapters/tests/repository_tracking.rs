//! `InProcessTracking` through the `vcs::tracking::RepositoryTracking` port,
//! against real git and jj repositories.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use vcs_adapters::InProcessTracking;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const PERSONAL: &str = ".accelerator/config.local.md";

const TRACKING: &dyn RepositoryTracking = &InProcessTracking;

struct Scratch {
    _dir: tempfile::TempDir,
    base: PathBuf,
    env: Hermetic,
}

impl Scratch {
    fn new(tag: &str) -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix(&format!("vcs-repository-tracking-{tag}-"))
            .tempdir()?;
        let base = dir.path().canonicalize()?;
        let env = Hermetic::rooted_at(&base)?;
        Ok(Self {
            _dir: dir,
            base,
            env,
        })
    }

    fn git_repo(&self, name: &str) -> Result<PathBuf, TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let root = self.base.join(name);
        fs::create_dir_all(&root)?;
        self.env.git(&["init", "--quiet"], &root)?;
        Ok(root)
    }

    fn jj_repo(&self) -> Result<PathBuf, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let root = self.base.join("repo");
        fs::create_dir_all(&root)?;
        self.env.jj(&["git", "init", "--no-colocate"], &root)?;
        Ok(root)
    }
}

fn personal(root: &Path) -> Result<PathBuf, TestError> {
    let path = root.join(PERSONAL);
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(&path, "---\njira:\n  token_cmd: x\n---\n")?;
    Ok(path)
}

#[test]
fn git_answers_untracked_then_tracked_once_staged() -> Result<(), TestError> {
    let scratch = Scratch::new("git")?;
    let root = scratch.git_repo("repo")?;
    let path = personal(&root)?;

    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Untracked);

    scratch.env.git(&["add", PERSONAL], &root)?;
    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn jj_answers_untracked_then_tracked_once_committed() -> Result<(), TestError> {
    let scratch = Scratch::new("jj")?;
    let root = scratch.jj_repo()?;
    fs::create_dir_all(root.join(".accelerator"))?;
    fs::write(root.join(".accelerator/.gitignore"), "config.local.md\n")?;
    let path = personal(&root)?;
    scratch.env.jj(&["commit", "-m", "ignore it"], &root)?;

    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Untracked);

    fs::remove_file(root.join(".accelerator/.gitignore"))?;
    scratch.env.jj(&["commit", "-m", "track it"], &root)?;
    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn a_corrupted_git_index_is_unknown() -> Result<(), TestError> {
    let scratch = Scratch::new("git-corrupt")?;
    let root = scratch.git_repo("repo")?;
    let path = personal(&root)?;
    scratch.env.git(&["add", PERSONAL], &root)?;
    fs::write(root.join(".git/index"), b"DIRC not really an index")?;

    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Unknown);
    Ok(())
}

#[test]
fn a_nested_repository_yields_only_its_own_root() -> Result<(), TestError> {
    let scratch = Scratch::new("nested")?;
    let outer = scratch.git_repo("outer")?;
    let inner = scratch.git_repo("outer/inner")?;
    let deeper = inner.join("deeper");
    fs::create_dir_all(&deeper)?;

    let answer = TRACKING.repository_roots(&deeper);

    assert_eq!(answer.roots, [inner]);
    assert!(!answer.roots.contains(&outer));
    assert!(answer.complete);
    Ok(())
}

#[test]
fn a_jj_repository_yields_its_root() -> Result<(), TestError> {
    let scratch = Scratch::new("jj-roots")?;
    let root = scratch.jj_repo()?;

    let answer = TRACKING.repository_roots(&root);

    assert_eq!(answer.roots, [root]);
    assert!(answer.complete);
    Ok(())
}
