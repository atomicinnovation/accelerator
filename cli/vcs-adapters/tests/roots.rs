//! `vcs_adapters::repository_roots` against real git and jj repositories: the
//! definition of "inside the repository" the command runner and the path
//! checks rest on.
#![cfg(feature = "bash-parity")]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use vcs::tracking::RootsAnswer;
use vcs_adapters::repository_roots;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

struct Scratch {
    _dir: tempfile::TempDir,
    base: PathBuf,
    env: Hermetic,
}

impl Scratch {
    fn new(tag: &str) -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix(&format!("vcs-roots-{tag}-"))
            .tempdir()?;
        let base = dir.path().canonicalize()?;
        let env = Hermetic::rooted_at(&base)?;
        Ok(Self {
            _dir: dir,
            base,
            env,
        })
    }

    fn directory(&self, name: &str) -> Result<PathBuf, TestError> {
        let root = self.base.join(name);
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn git_repo(&self) -> Result<PathBuf, TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let root = self.directory("repo")?;
        self.env.git(&["init", "--quiet"], &root)?;
        Ok(root)
    }

    fn jj_repo(&self, colocate: bool) -> Result<PathBuf, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let root = self.directory("repo")?;
        let flag = if colocate {
            "--colocate"
        } else {
            "--no-colocate"
        };
        self.env.jj(&["git", "init", flag], &root)?;
        Ok(root)
    }
}

fn roots_of(answer: &RootsAnswer) -> BTreeSet<PathBuf> {
    answer.roots.iter().cloned().collect()
}

#[test]
fn a_git_linked_worktree_yields_the_worktree_and_the_main_worktree(
) -> Result<(), TestError> {
    let scratch = Scratch::new("git-worktree")?;
    let main = scratch.git_repo()?;
    scratch
        .env
        .git(&["commit", "--allow-empty", "-m", "base"], &main)?;
    let linked = scratch.base.join("linked");
    scratch.env.git(
        &["worktree", "add", "--quiet", "--detach", "../linked"],
        &main,
    )?;

    let answer = repository_roots(&linked);

    assert_eq!(roots_of(&answer), BTreeSet::from([linked, main]));
    assert!(answer.complete);
    Ok(())
}

#[test]
fn a_jj_secondary_workspace_yields_the_workspace_and_the_repository(
) -> Result<(), TestError> {
    let scratch = Scratch::new("jj-workspace")?;
    let main = scratch.jj_repo(false)?;
    let secondary = scratch.base.join("secondary");
    scratch
        .env
        .jj(&["workspace", "add", "../secondary"], &main)?;

    let answer = repository_roots(&secondary);

    assert_eq!(roots_of(&answer), BTreeSet::from([secondary, main]));
    assert!(answer.complete);
    Ok(())
}

#[test]
fn a_subdirectory_of_a_checkout_yields_the_checkout_root(
) -> Result<(), TestError> {
    let scratch = Scratch::new("git-sub")?;
    let root = scratch.git_repo()?;
    let nested = root.join("nested/deeper");
    fs::create_dir_all(&nested)?;

    let answer = repository_roots(&nested);

    assert_eq!(roots_of(&answer), BTreeSet::from([root]));
    assert!(answer.complete);
    Ok(())
}

#[test]
fn no_vcs_yields_no_roots() -> Result<(), TestError> {
    let scratch = Scratch::new("none")?;
    let loose = scratch.directory("loose")?;

    let answer = repository_roots(&loose);

    assert!(answer.roots.is_empty());
    assert!(answer.complete);
    Ok(())
}

#[test]
fn a_crafted_jj_marker_cannot_hide_the_git_root() -> Result<(), TestError> {
    for crafted in [".jj/repo/store/type", ".jj/placeholder"] {
        let scratch = Scratch::new("crafted-jj")?;
        let root = scratch.git_repo()?;
        let marker = root.join(crafted);
        fs::create_dir_all(marker.parent().ok_or("no parent")?)?;
        fs::write(&marker, "git\n")?;
        scratch.env.git(&["add", "--force", "."], &root)?;

        let answer = repository_roots(&root);

        assert!(answer.roots.contains(&root), "{crafted}: {answer:?}");
    }
    Ok(())
}

#[test]
fn a_crafted_jj_marker_with_no_repository_marks_the_roots_incomplete(
) -> Result<(), TestError> {
    let scratch = Scratch::new("crafted-jj-empty")?;
    let root = scratch.git_repo()?;
    let marker = root.join(".jj/placeholder");
    fs::create_dir_all(marker.parent().ok_or("no parent")?)?;
    fs::write(&marker, "")?;
    scratch.env.git(&["add", "--force", "."], &root)?;

    let answer = repository_roots(&root);

    assert!(!answer.complete, "{answer:?}");
    Ok(())
}

#[test]
fn a_genuine_colocated_checkout_yields_complete_roots() -> Result<(), TestError>
{
    let scratch = Scratch::new("colocated")?;
    let root = scratch.jj_repo(true)?;

    let answer = repository_roots(&root);

    assert_eq!(roots_of(&answer), BTreeSet::from([root]));
    assert!(answer.complete);
    Ok(())
}

#[test]
fn a_workspace_whose_repository_cannot_be_found_yields_incomplete_roots(
) -> Result<(), TestError> {
    let scratch = Scratch::new("jj-orphan")?;
    let main = scratch.jj_repo(false)?;
    let secondary = scratch.base.join("secondary");
    scratch
        .env
        .jj(&["workspace", "add", "../secondary"], &main)?;
    fs::write(
        secondary.join(".jj/repo"),
        scratch
            .base
            .join("gone/.jj/repo")
            .to_string_lossy()
            .as_bytes(),
    )?;

    let answer = repository_roots(&secondary);

    assert!(answer.roots.contains(&secondary), "{answer:?}");
    assert!(!answer.complete);
    Ok(())
}
