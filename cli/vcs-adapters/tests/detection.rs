//! Detection against real repository shapes: colocated, jj-secondary workspace,
//! a `.git`-file worktree, a git repo with no commits, a bare repo, and a tree
//! with no repository at all.
//!
//! These need real `jj` and `git` binaries and hard-fail when one is absent —
//! Rust's harness has no skip primitive, so an early return would register as a
//! green PASS. The marker-walk cases that need no binary are unit tests in the
//! crate, so they still run with the feature off.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use vcs::RepoFacts;
use vcs::VcsKind;
use vcs_adapters::library::InProcessProbe;

type TestError = Box<dyn std::error::Error>;

fn require(binary: &str) -> Result<(), TestError> {
    let found = Command::new(binary)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if found {
        return Ok(());
    }
    Err(format!("`{binary}` is required by the detection fixtures").into())
}

/// A temp directory guard whose removal is handled on drop. Callers canonicalize
/// its `.path()`, so a probed root compares equal on a platform whose temp dir
/// is itself a symlink.
fn tempdir(label: &str) -> Result<TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("vcs-detect-{label}-"))
        .tempdir()?)
}

fn run(binary: &str, args: &[&str], dir: &Path) -> Result<(), TestError> {
    let output = Command::new(binary)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|error| format!("could not run {binary}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{binary} {} failed in {}: {}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

/// A git repository with one commit. Identity is passed per-invocation so the
/// fixture does not depend on the developer's global git config.
fn git_repo_with_a_commit(label: &str) -> Result<TempDir, TestError> {
    let dir = tempdir(label)?;
    run("git", &["init", "--quiet"], dir.path())?;
    run(
        "git",
        &[
            "-c",
            "user.email=fixture@example.com",
            "-c",
            "user.name=Fixture",
            "commit",
            "--allow-empty",
            "--quiet",
            "-m",
            "root",
        ],
        dir.path(),
    )?;
    Ok(dir)
}

/// The probe asks for the *full* working-copy id — 40 hex digits from both jj
/// (`commit_id`) and git (`rev-parse HEAD`) — so a short or decorated id fails.
///
/// Note this rejects a **sha256** repository's 64-hex id. That is deliberate:
/// no fixture in this suite uses one, and such a repository is unsupported —
/// `gix` cannot read it at all, so `revision` reports absence rather than a
/// wider id.
fn is_full_revision_id(revision: &str) -> bool {
    revision.len() == 40 && revision.chars().all(|c| c.is_ascii_hexdigit())
}

/// The facts for `start`, composed the way the crate's own composition root
/// composes them.
fn facts(start: &Path) -> Option<RepoFacts> {
    vcs::facts(start, &InProcessProbe, &InProcessProbe)
}

#[test]
fn a_git_repository_reports_its_root_name_and_revision() -> Result<(), TestError>
{
    require("git")?;
    let root = git_repo_with_a_commit("plain-git")?;
    let root = root.path().canonicalize()?;

    let derived = facts(&root).ok_or("expected the git repo to be detected")?;

    assert_eq!(derived.root, root);
    assert_eq!(
        derived.name,
        root.file_name()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or("the temp root has no final component")?
    );
    assert_eq!(derived.kind, VcsKind::Git);
    let revision = derived.revision.ok_or("expected a revision")?;
    assert!(
        is_full_revision_id(&revision),
        "not a full revision id: {revision}"
    );
    Ok(())
}

#[test]
fn the_walk_finds_the_root_from_a_nested_directory() -> Result<(), TestError> {
    require("git")?;
    let root = git_repo_with_a_commit("nested")?;
    let root = root.path().canonicalize()?;
    let nested = root.join("meta/work");
    fs::create_dir_all(&nested)?;

    let derived = facts(&nested).ok_or("expected the walk to find the root")?;

    assert_eq!(derived.root, root);
    Ok(())
}

#[test]
fn a_git_repository_with_no_commits_has_no_revision() -> Result<(), TestError> {
    require("git")?;
    let root = tempdir("no-commits")?;
    let root = root.path().canonicalize()?;
    run("git", &["init", "--quiet"], &root)?;

    let derived = facts(&root).ok_or("expected the git repo to be detected")?;

    assert_eq!(derived.kind, VcsKind::Git);
    assert_eq!(
        derived.revision, None,
        "a commitless repo must report no revision, not an empty one"
    );
    Ok(())
}

#[test]
fn a_colocated_repository_is_driven_as_jj() -> Result<(), TestError> {
    require("jj")?;
    require("git")?;
    let root = git_repo_with_a_commit("colocated")?;
    let root = root.path().canonicalize()?;
    run("jj", &["git", "init", "--colocate"], &root)?;

    assert!(root.join(".git").exists(), "the git marker should remain");
    assert!(root.join(".jj").exists(), "the jj marker should be present");

    let derived =
        facts(&root).ok_or("expected the colocated repo to detect")?;

    assert_eq!(
        derived.kind,
        VcsKind::Jj,
        "jj must win over git in a colocated checkout"
    );
    let revision = derived.revision.ok_or("expected a revision")?;
    assert!(
        is_full_revision_id(&revision),
        "not a full revision id: {revision}"
    );
    Ok(())
}

#[test]
fn a_secondary_jj_workspace_roots_at_its_own_marker() -> Result<(), TestError> {
    require("jj")?;
    require("git")?;
    let primary = git_repo_with_a_commit("primary")?;
    let primary = primary.path().canonicalize()?;
    run("jj", &["git", "init", "--colocate"], &primary)?;

    let secondary_root = tempdir("secondary")?;
    let secondary = secondary_root.path().canonicalize()?.join("workspace");
    run(
        "jj",
        &[
            "workspace",
            "add",
            secondary.to_str().ok_or("non-UTF-8 workspace path")?,
        ],
        &primary,
    )?;

    let derived = facts(&secondary)
        .ok_or("expected the secondary workspace to detect")?;

    assert_eq!(
        derived.root, secondary,
        "a secondary workspace roots at its own .jj, not at the primary"
    );
    assert_eq!(derived.kind, VcsKind::Jj);
    assert_eq!(
        derived.name,
        primary
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or("the primary root has no final component")?,
        "a workspace's name is the repository it shares a store with, not the \
         ephemeral workspace directory"
    );
    Ok(())
}

#[test]
fn a_worktree_whose_git_marker_is_a_file_is_recognised() -> Result<(), TestError>
{
    require("git")?;
    let primary = git_repo_with_a_commit("worktree-primary")?;
    let primary = primary.path().canonicalize()?;
    let worktree_root = tempdir("worktree")?;
    let worktree = worktree_root.path().canonicalize()?.join("checkout");
    run(
        "git",
        &[
            "worktree",
            "add",
            worktree.to_str().ok_or("non-UTF-8 worktree path")?,
        ],
        &primary,
    )?;

    assert!(
        worktree.join(".git").is_file(),
        "a worktree's .git should be a file, not a directory"
    );

    let derived = facts(&worktree).ok_or("expected the worktree to detect")?;

    assert_eq!(derived.root, worktree);
    assert_eq!(
        derived.kind,
        VcsKind::Git,
        "the marker is tested by existence, so a .git file counts"
    );
    let revision = derived.revision.ok_or("expected a revision")?;
    assert!(
        is_full_revision_id(&revision),
        "not a full revision id: {revision}"
    );
    Ok(())
}

#[test]
fn a_bare_repository_has_no_facts() -> Result<(), TestError> {
    require("git")?;

    // A bare repo keeps HEAD/objects/refs at its top level with no `.git`
    // marker, so the marker walk finds nothing — the reason `facts` is an
    // Option rather than a fabricated empty root.
    let bare = tempdir("bare")?;
    let bare = bare.path().canonicalize()?;
    run(
        "git",
        &["init", "--bare", "--initial-branch=main", "."],
        &bare,
    )?;
    assert!(bare.join("HEAD").is_file(), "expected a bare layout");
    assert!(
        !bare.join(".git").exists(),
        "a bare repo has no .git marker"
    );

    assert_eq!(
        facts(&bare),
        None,
        "a bare repository must resolve to no facts, not an empty root"
    );
    Ok(())
}
