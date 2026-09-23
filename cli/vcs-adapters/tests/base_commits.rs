//! `InProcessProbe::working_copy_state`'s base commits against the commits
//! `jj log -r 'parents(@)'` names, across the jj operations that do and do not
//! move them, and against `HEAD` on git.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;
use vcs::VcsKind;
use vcs_adapters::library::InProcessProbe;
use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::jj_status::parent_commit_ids;

type TestError = Box<dyn std::error::Error>;

const COLOCATIONS: [&str; 2] = ["--no-colocate", "--colocate"];

struct JjRepo {
    _work: TempDir,
    env: Hermetic,
    root: PathBuf,
}

impl JjRepo {
    fn new(colocation: &str) -> Result<Self, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let work = tempfile::Builder::new()
            .prefix("vcs-base-commits-")
            .tempdir()?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(&root)?;
        env.jj(&["git", "init", colocation], &root)?;
        Ok(Self {
            _work: work,
            env,
            root,
        })
    }

    fn jj(&self, args: &[&str]) -> Result<String, TestError> {
        Ok(self.env.jj(args, &self.root)?)
    }

    fn write(&self, relative: &str, content: &str) -> Result<(), TestError> {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn commit(&self, file: &str, message: &str) -> Result<String, TestError> {
        self.write(file, message)?;
        self.jj(&["commit", "-m", message])?;
        self.change_id("@-")
    }

    fn change_id(&self, revision: &str) -> Result<String, TestError> {
        self.jj(&["log", "--no-graph", "-r", revision, "-T", "change_id"])
    }

    fn base_commits(&self) -> Result<Vec<String>, TestError> {
        let mut commits = InProcessProbe
            .working_copy_state(&self.root, VcsKind::Jj)?
            .base_commits;
        commits.sort_unstable();
        Ok(commits)
    }

    fn oracle(&self) -> Result<Vec<String>, TestError> {
        Ok(parent_commit_ids(&self.env, &self.root)?)
    }
}

fn for_each_colocation(
    case: impl Fn(&JjRepo) -> Result<(), TestError>,
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        case(&JjRepo::new(colocation)?)?;
    }
    Ok(())
}

#[test]
fn a_single_parent_is_the_base() -> Result<(), TestError> {
    for_each_colocation(|repo| {
        repo.commit("meta/a.md", "a")?;

        let base = repo.base_commits()?;

        assert_eq!(base.len(), 1);
        assert_eq!(base, repo.oracle()?);
        Ok(())
    })
}

#[test]
fn the_root_commit_is_forty_zeros() -> Result<(), TestError> {
    for_each_colocation(|repo| {
        assert_eq!(repo.base_commits()?, vec!["0".repeat(40)]);
        Ok(())
    })
}

#[test]
fn a_merge_is_based_on_every_parent_whatever_their_order(
) -> Result<(), TestError> {
    for_each_colocation(|repo| {
        let a = repo.commit("meta/a.md", "a")?;
        repo.jj(&["new", "root()"])?;
        let b = repo.commit("meta/b.md", "b")?;
        repo.jj(&["new", "root()"])?;
        let c = repo.commit("meta/c.md", "c")?;

        repo.jj(&["new", &a, &b])?;
        let a_then_b = repo.base_commits()?;
        assert_eq!(a_then_b.len(), 2);
        assert_eq!(a_then_b, repo.oracle()?);

        repo.jj(&["new", &b, &a])?;
        assert_eq!(repo.base_commits()?, a_then_b);

        repo.jj(&["new", &a, &b, &c])?;
        let three = repo.base_commits()?;
        assert_eq!(three.len(), 3);
        assert_eq!(three, repo.oracle()?);
        Ok(())
    })
}

#[test]
fn edits_status_and_describing_the_working_copy_keep_the_base(
) -> Result<(), TestError> {
    for_each_colocation(|repo| {
        repo.commit("meta/a.md", "a")?;
        let before = repo.base_commits()?;

        repo.write("meta/a.md", "edited")?;
        repo.jj(&["status"])?;
        assert_eq!(repo.base_commits()?, before);

        repo.jj(&["describe", "-m", "x"])?;
        assert_eq!(repo.base_commits()?, before);
        Ok(())
    })
}

#[test]
fn rewriting_or_moving_off_the_parent_changes_the_base() -> Result<(), TestError>
{
    for_each_colocation(|repo| {
        let a = repo.commit("meta/a.md", "a")?;
        let mut previous = repo.base_commits()?;
        let mut assert_moved = |step: &str| -> Result<(), TestError> {
            let now = repo.base_commits()?;
            assert_ne!(now, previous, "{step} kept the base");
            assert_eq!(now, repo.oracle()?, "{step}");
            previous = now;
            Ok(())
        };

        repo.jj(&["describe", "@-", "-m", "y"])?;
        assert_moved("describe @-")?;
        repo.jj(&["new"])?;
        assert_moved("new")?;
        repo.write("meta/z.md", "z")?;
        repo.jj(&["commit", "-m", "z"])?;
        assert_moved("commit")?;
        repo.jj(&["rebase", "-r", "@", "-d", &a])?;
        assert_moved("rebase")?;
        repo.jj(&["new", "root()"])?;
        assert_moved("new root()")?;
        Ok(())
    })
}

#[test]
fn editing_a_sibling_with_the_same_parent_keeps_the_base(
) -> Result<(), TestError> {
    for_each_colocation(|repo| {
        repo.commit("meta/a.md", "a")?;
        repo.write("meta/sibling.md", "sibling")?;
        let sibling = repo.change_id("@")?;
        repo.jj(&["new", "@-"])?;
        let before = repo.base_commits()?;

        repo.jj(&["edit", &sibling])?;

        assert_eq!(repo.base_commits()?, before);
        Ok(())
    })
}

#[test]
fn describing_one_parent_of_a_merge_changes_the_base() -> Result<(), TestError>
{
    for_each_colocation(|repo| {
        let a = repo.commit("meta/a.md", "a")?;
        repo.jj(&["new", "root()"])?;
        let b = repo.commit("meta/b.md", "b")?;
        repo.jj(&["new", &a, &b])?;
        let before = repo.base_commits()?;

        repo.jj(&["describe", &b, "-m", "b2"])?;

        let after = repo.base_commits()?;
        assert_ne!(after, before);
        assert_eq!(after, repo.oracle()?);
        Ok(())
    })
}

/// The current op head, read directly off disk: any real `jj` command
/// snapshots the working copy first, so it cannot serve as the probe.
fn op_heads(root: &Path) -> Result<Vec<String>, TestError> {
    let mut heads: Vec<String> =
        fs::read_dir(root.join(".jj/repo/op_heads/heads"))?
            .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
            .collect::<Result<_, TestError>>()?;
    heads.sort();
    Ok(heads)
}

#[test]
fn reading_the_base_writes_no_operation() -> Result<(), TestError> {
    for_each_colocation(|repo| {
        repo.commit("meta/a.md", "a")?;
        repo.write("meta/b.md", "b")?;
        let before = op_heads(&repo.root)?;

        repo.base_commits()?;

        assert_eq!(op_heads(&repo.root)?, before);
        Ok(())
    })
}

#[test]
fn a_jj_working_copy_state_lists_the_same_dirty_paths() -> Result<(), TestError>
{
    for_each_colocation(|repo| {
        repo.commit("meta/a.md", "a")?;
        repo.write("meta/a.md", "edited")?;
        repo.write("meta/new.md", "new")?;

        let state =
            InProcessProbe.working_copy_state(&repo.root, VcsKind::Jj)?;

        let mut listed = state.dirty_paths;
        listed.sort();
        let mut expected =
            InProcessProbe.dirty_paths(&repo.root, VcsKind::Jj)?;
        expected.sort();
        assert_eq!(listed, expected);
        Ok(())
    })
}

#[test]
fn a_corrupt_operation_store_is_an_error() -> Result<(), TestError> {
    let repo = JjRepo::new("--no-colocate")?;
    repo.commit("meta/a.md", "a")?;
    let heads = repo.root.join(".jj/repo/op_heads/heads");
    fs::remove_dir_all(&heads)?;
    fs::create_dir_all(&heads)?;

    assert!(InProcessProbe
        .working_copy_state(&repo.root, VcsKind::Jj)
        .is_err());
    Ok(())
}

fn git_repo(work: &Path) -> Result<(Hermetic, PathBuf), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let env = Hermetic::rooted_at(work)?;
    let root = work.join("repo");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    Ok((env, root))
}

#[test]
fn a_git_working_copy_is_based_on_head_with_its_dirty_paths(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let (env, root) = git_repo(work.path())?;
    fs::write(root.join("a.md"), "one\n")?;
    env.git(&["add", "a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;
    fs::write(root.join("a.md"), "two\n")?;
    fs::write(root.join("b.md"), "new\n")?;
    let head = env.git(&["rev-parse", "HEAD"], &root)?;

    let state = InProcessProbe.working_copy_state(&root, VcsKind::Git)?;

    assert_eq!(state.base_commits, vec![head]);
    let mut listed = state.dirty_paths;
    listed.sort();
    let mut expected = InProcessProbe.dirty_paths(&root, VcsKind::Git)?;
    expected.sort();
    assert_eq!(listed, expected);
    Ok(())
}

#[test]
fn an_unborn_git_head_has_no_base() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let (_env, root) = git_repo(work.path())?;

    let state = InProcessProbe.working_copy_state(&root, VcsKind::Git)?;

    assert!(state.base_commits.is_empty());
    Ok(())
}

#[test]
fn no_vcs_has_no_base_and_no_dirty_paths() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;

    let state =
        InProcessProbe.working_copy_state(work.path(), VcsKind::None)?;

    assert!(state.base_commits.is_empty());
    assert!(state.dirty_paths.is_empty());
    Ok(())
}
