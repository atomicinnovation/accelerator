//! `vcs_adapters::file_tracking` against real git and jj repositories: the
//! fail-closed answer the consent policy's tracked-file refusal rests on.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use vcs::tracking::FileTracking;
use vcs_adapters::file_tracking;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const PERSONAL: &str = ".accelerator/config.local.md";

struct Scratch {
    _dir: tempfile::TempDir,
    base: PathBuf,
    env: Hermetic,
}

impl Scratch {
    fn new(tag: &str) -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix(&format!("vcs-file-tracking-{tag}-"))
            .tempdir()?;
        let base = dir.path().canonicalize()?;
        let env = Hermetic::rooted_at(&base)?;
        Ok(Self {
            _dir: dir,
            base,
            env,
        })
    }

    fn repo(&self) -> Result<PathBuf, TestError> {
        let root = self.base.join("repo");
        fs::create_dir_all(root.join(".accelerator"))?;
        Ok(root)
    }

    fn git_repo(&self) -> Result<PathBuf, TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let root = self.repo()?;
        self.env.git(&["init", "--quiet"], &root)?;
        Ok(root)
    }

    fn jj_repo(&self, colocate: bool) -> Result<PathBuf, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let root = self.repo()?;
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

#[test]
fn git_answers_untracked_then_tracked_once_staged() -> Result<(), TestError> {
    let scratch = Scratch::new("git")?;
    let root = scratch.git_repo()?;
    let path = personal(&root)?;

    assert_eq!(file_tracking(&path), FileTracking::Untracked);

    scratch.env.git(&["add", PERSONAL], &root)?;
    assert_eq!(file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn jj_answers_tracked_for_a_committed_file() -> Result<(), TestError> {
    let scratch = Scratch::new("jj")?;
    let root = scratch.jj_repo(false)?;
    let path = personal(&root)?;
    scratch.env.jj(&["commit", "-m", "track it"], &root)?;

    assert_eq!(file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn no_vcs_is_untracked() -> Result<(), TestError> {
    let scratch = Scratch::new("none")?;
    let path = personal(&scratch.repo()?)?;

    assert_eq!(file_tracking(&path), FileTracking::Untracked);
    Ok(())
}

#[test]
fn a_config_root_in_a_subdirectory_of_a_git_checkout_is_still_checked(
) -> Result<(), TestError> {
    let scratch = Scratch::new("git-sub")?;
    let root = scratch.git_repo()?;
    let path = personal(&root.join("nested"))?;
    scratch
        .env
        .git(&["add", "nested/.accelerator/config.local.md"], &root)?;

    assert_eq!(file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn a_config_root_in_a_subdirectory_of_a_jj_checkout_is_still_checked(
) -> Result<(), TestError> {
    let scratch = Scratch::new("jj-sub")?;
    let root = scratch.jj_repo(false)?;
    let path = personal(&root.join("nested"))?;
    scratch.env.jj(&["commit", "-m", "track it"], &root)?;

    assert_eq!(file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn a_corrupted_git_index_is_unknown() -> Result<(), TestError> {
    let scratch = Scratch::new("git-corrupt")?;
    let root = scratch.git_repo()?;
    let path = personal(&root)?;
    scratch.env.git(&["add", PERSONAL], &root)?;
    fs::write(root.join(".git/index"), b"DIRC not really an index")?;

    assert_eq!(file_tracking(&path), FileTracking::Unknown);
    Ok(())
}

#[test]
fn a_crafted_jj_marker_cannot_hide_a_git_tracked_file() -> Result<(), TestError>
{
    for crafted in [".jj/repo/store/type", ".accelerator/.jj/repo/store/type"] {
        let scratch = Scratch::new("crafted-jj")?;
        let root = scratch.git_repo()?;
        let path = personal(&root)?;
        let marker = root.join(crafted);
        fs::create_dir_all(marker.parent().ok_or("no parent")?)?;
        fs::write(&marker, "git\n")?;
        scratch.env.git(&["add", "--force", "."], &root)?;

        assert_eq!(
            file_tracking(&path),
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
    let root = scratch.git_repo()?;
    let path = personal(&root)?;
    let marker = root.join(".jj/placeholder");
    fs::create_dir_all(marker.parent().ok_or("no parent")?)?;
    fs::write(&marker, "")?;
    scratch
        .env
        .git(&["add", "--force", ".jj/placeholder"], &root)?;

    assert_eq!(file_tracking(&path), FileTracking::Unknown);
    Ok(())
}

#[test]
fn git_matches_case_insensitively_under_core_ignorecase(
) -> Result<(), TestError> {
    let scratch = Scratch::new("icase")?;
    let root = scratch.git_repo()?;
    scratch
        .env
        .git(&["config", "core.ignorecase", "true"], &root)?;
    let staged = root.join(".accelerator/CONFIG.LOCAL.md");
    fs::write(&staged, "x\n")?;
    scratch
        .env
        .git(&["add", ".accelerator/CONFIG.LOCAL.md"], &root)?;
    fs::remove_file(&staged)?;
    let path = personal(&root)?;

    assert_eq!(file_tracking(&path), FileTracking::Tracked);
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

    assert_eq!(file_tracking(&path), FileTracking::Tracked);
    Ok(())
}

#[test]
fn an_ignored_file_is_untracked_in_a_genuine_jj_store() -> Result<(), TestError>
{
    for colocate in [false, true] {
        let scratch = Scratch::new("ignored")?;
        let root = scratch.jj_repo(colocate)?;
        fs::write(root.join(".accelerator/.gitignore"), "config.local.md\n")?;
        let path = personal(&root)?;
        scratch.env.jj(&["commit", "-m", "ignore it"], &root)?;

        assert_eq!(
            file_tracking(&path),
            FileTracking::Untracked,
            "colocated: {colocate}"
        );
    }
    Ok(())
}
