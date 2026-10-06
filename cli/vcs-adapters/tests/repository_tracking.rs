//! `InProcessTracking` through the `vcs::tracking::RepositoryTracking` port,
//! against real git and jj repositories: the fail-closed answers the consent
//! policy's tracked-file refusal and its repository-root checks rest on.
#![cfg(feature = "bash-parity")]

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use vcs::tracking::RootsAnswer;
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

    fn directory(&self, name: &str) -> Result<PathBuf, TestError> {
        let root = self.base.join(name);
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn git_repo(&self, name: &str) -> Result<PathBuf, TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let root = self.directory(name)?;
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

fn personal(root: &Path) -> Result<PathBuf, TestError> {
    let path = root.join(PERSONAL);
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(&path, "---\njira:\n  token_cmd: x\n---\n")?;
    Ok(path)
}

fn roots_of(answer: &RootsAnswer) -> BTreeSet<PathBuf> {
    answer.roots.iter().cloned().collect()
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
    let root = scratch.jj_repo(false)?;
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
fn no_vcs_is_untracked() -> Result<(), TestError> {
    let scratch = Scratch::new("none")?;
    let path = personal(&scratch.directory("repo")?)?;

    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Untracked);
    Ok(())
}

#[test]
fn a_config_root_in_a_subdirectory_of_a_git_checkout_is_still_checked(
) -> Result<(), TestError> {
    let scratch = Scratch::new("git-sub")?;
    let root = scratch.git_repo("repo")?;
    let path = personal(&root.join("nested"))?;
    scratch
        .env
        .git(&["add", "nested/.accelerator/config.local.md"], &root)?;

    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn a_config_root_in_a_subdirectory_of_a_jj_checkout_is_still_checked(
) -> Result<(), TestError> {
    let scratch = Scratch::new("jj-sub")?;
    let root = scratch.jj_repo(false)?;
    let path = personal(&root.join("nested"))?;
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
fn a_crafted_jj_marker_cannot_hide_a_git_tracked_file() -> Result<(), TestError>
{
    for crafted in [".jj/repo/store/type", ".accelerator/.jj/repo/store/type"] {
        let scratch = Scratch::new("crafted-jj")?;
        let root = scratch.git_repo("repo")?;
        let path = personal(&root)?;
        let marker = root.join(crafted);
        fs::create_dir_all(marker.parent().ok_or("no parent")?)?;
        fs::write(&marker, "git\n")?;
        scratch.env.git(&["add", "--force", "."], &root)?;

        assert_eq!(
            TRACKING.file_tracking(&path),
            FileTracking::Tracked,
            "a .jj committed at {crafted} must not mask git's answer"
        );
    }
    Ok(())
}

#[test]
fn a_jj_directory_git_tracks_trips_the_answer_to_unknown(
) -> Result<(), TestError> {
    let scratch = Scratch::new("tripwire")?;
    let root = scratch.git_repo("repo")?;
    let path = personal(&root)?;
    let marker = root.join(".jj/placeholder");
    fs::create_dir_all(marker.parent().ok_or("no parent")?)?;
    fs::write(&marker, "")?;
    scratch
        .env
        .git(&["add", "--force", ".jj/placeholder"], &root)?;

    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Unknown);
    Ok(())
}

#[test]
fn git_matches_case_insensitively_under_core_ignorecase(
) -> Result<(), TestError> {
    let scratch = Scratch::new("icase")?;
    let root = scratch.git_repo("repo")?;
    scratch
        .env
        .git(&["config", "core.ignorecase", "true"], &root)?;
    fs::create_dir_all(root.join(".accelerator"))?;
    let staged = root.join(".accelerator/CONFIG.LOCAL.md");
    fs::write(&staged, "x\n")?;
    scratch
        .env
        .git(&["add", ".accelerator/CONFIG.LOCAL.md"], &root)?;
    fs::remove_file(&staged)?;
    let path = personal(&root)?;

    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn a_colocated_checkout_where_only_jj_tracks_the_file_is_tracked(
) -> Result<(), TestError> {
    let scratch = Scratch::new("colocated-tracked")?;
    let root = scratch.jj_repo(true)?;
    let path = personal(&root)?;
    scratch.env.jj(&["commit", "-m", "track it"], &root)?;
    let index = root.join(".git/index");
    if index.exists() {
        fs::remove_file(index)?;
    }

    assert_eq!(TRACKING.file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn an_ignored_file_is_untracked_in_a_genuine_jj_store() -> Result<(), TestError>
{
    for colocate in [false, true] {
        let scratch = Scratch::new("ignored")?;
        let root = scratch.jj_repo(colocate)?;
        fs::create_dir_all(root.join(".accelerator"))?;
        fs::write(root.join(".accelerator/.gitignore"), "config.local.md\n")?;
        let path = personal(&root)?;
        scratch.env.jj(&["commit", "-m", "ignore it"], &root)?;

        assert_eq!(
            TRACKING.file_tracking(&path),
            FileTracking::Untracked,
            "colocated: {colocate}"
        );
    }
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
    let root = scratch.jj_repo(false)?;

    let answer = TRACKING.repository_roots(&root);

    assert_eq!(answer.roots, [root]);
    assert!(answer.complete);
    Ok(())
}

#[test]
fn a_git_linked_worktree_yields_the_worktree_and_the_main_worktree(
) -> Result<(), TestError> {
    let scratch = Scratch::new("git-worktree")?;
    let main = scratch.git_repo("repo")?;
    scratch
        .env
        .git(&["commit", "--allow-empty", "-m", "base"], &main)?;
    let linked = scratch.base.join("linked");
    scratch.env.git(
        &["worktree", "add", "--quiet", "--detach", "../linked"],
        &main,
    )?;

    let answer = TRACKING.repository_roots(&linked);

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

    let answer = TRACKING.repository_roots(&secondary);

    assert_eq!(roots_of(&answer), BTreeSet::from([secondary, main]));
    assert!(answer.complete);
    Ok(())
}

#[test]
fn a_subdirectory_of_a_checkout_yields_the_checkout_root(
) -> Result<(), TestError> {
    let scratch = Scratch::new("git-sub-roots")?;
    let root = scratch.git_repo("repo")?;
    let nested = root.join("nested/deeper");
    fs::create_dir_all(&nested)?;

    let answer = TRACKING.repository_roots(&nested);

    assert_eq!(roots_of(&answer), BTreeSet::from([root]));
    assert!(answer.complete);
    Ok(())
}

#[test]
fn no_vcs_yields_no_roots() -> Result<(), TestError> {
    let scratch = Scratch::new("none-roots")?;
    let loose = scratch.directory("loose")?;

    let answer = TRACKING.repository_roots(&loose);

    assert!(answer.roots.is_empty());
    assert!(answer.complete);
    Ok(())
}

#[test]
fn a_crafted_jj_marker_cannot_hide_the_git_root() -> Result<(), TestError> {
    for crafted in [".jj/repo/store/type", ".jj/placeholder"] {
        let scratch = Scratch::new("crafted-jj-roots")?;
        let root = scratch.git_repo("repo")?;
        let marker = root.join(crafted);
        fs::create_dir_all(marker.parent().ok_or("no parent")?)?;
        fs::write(&marker, "git\n")?;
        scratch.env.git(&["add", "--force", "."], &root)?;

        let answer = TRACKING.repository_roots(&root);

        assert!(answer.roots.contains(&root), "{crafted}: {answer:?}");
    }
    Ok(())
}

#[test]
fn a_crafted_jj_marker_with_no_repository_marks_the_roots_incomplete(
) -> Result<(), TestError> {
    let scratch = Scratch::new("crafted-jj-empty")?;
    let root = scratch.git_repo("repo")?;
    let marker = root.join(".jj/placeholder");
    fs::create_dir_all(marker.parent().ok_or("no parent")?)?;
    fs::write(&marker, "")?;
    scratch.env.git(&["add", "--force", "."], &root)?;

    let answer = TRACKING.repository_roots(&root);

    assert!(!answer.complete, "{answer:?}");
    Ok(())
}

#[test]
fn a_genuine_colocated_checkout_yields_complete_roots() -> Result<(), TestError>
{
    let scratch = Scratch::new("colocated")?;
    let root = scratch.jj_repo(true)?;

    let answer = TRACKING.repository_roots(&root);

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

    let answer = TRACKING.repository_roots(&secondary);

    assert!(answer.roots.contains(&secondary), "{answer:?}");
    assert!(!answer.complete);
    Ok(())
}
