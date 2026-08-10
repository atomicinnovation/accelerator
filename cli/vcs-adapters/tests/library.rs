//! The library-backed probe against real repository shapes, alongside the
//! subprocess pair.
//!
//! Two properties. The boundary rule: `RepoRoot::discover` reports the start
//! path's nearest marked ancestor and never one above it, paired with the
//! negative assertion that an unbounded `gix::discover` on the same fixture
//! *does* escape — without which the containment claim holds vacuously. And
//! parity with `CommandProbe` on every port method.
//!
//! These need real `jj` and `git` binaries and hard-fail when one is absent:
//! Rust's harness has no skip primitive, so an early return would register as a
//! green PASS.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;
use vcs::RepoRoot;
use vcs::VcsKind;
use vcs::VcsProbe;
use vcs_adapters::library::InProcessProbe;
use vcs_adapters::subprocess::CommandProbe;
use vcs_adapters::subprocess::MarkerWalkRoot;

type TestError = Box<dyn std::error::Error>;

fn require(binary: &str) -> Result<(), TestError> {
    let found = Command::new(binary)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if found {
        return Ok(());
    }
    Err(format!("`{binary}` is required by the library fixtures").into())
}

fn tempdir(label: &str) -> Result<TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("vcs-library-{label}-"))
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

/// Identity is passed per-invocation so the fixture does not depend on the
/// developer's global git config, and so a hostname with no domain cannot make
/// git refuse the commit.
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

fn colocated_repo(label: &str) -> Result<TempDir, TestError> {
    let dir = git_repo_with_a_commit(label)?;
    run("jj", &["git", "init", "--colocate"], dir.path())?;
    Ok(dir)
}

fn path_of(dir: &TempDir) -> Result<PathBuf, TestError> {
    Ok(dir.path().canonicalize()?)
}

// --- The boundary rule ---

#[test]
fn the_boundary_is_the_nearest_marker_not_an_ancestor() -> Result<(), TestError>
{
    require("jj")?;
    require("git")?;

    // The inner directory carries `.jj` only, so anything walking for `.git`
    // sails past it into the outer repository.
    let main = colocated_repo("nested-main")?;
    let main = path_of(&main)?;
    let outer = git_repo_with_a_commit("nested-outer")?;
    let outer = path_of(&outer)?;
    let inner = outer.join("sub");
    run(
        "jj",
        &[
            "workspace",
            "add",
            inner.to_str().ok_or("non-UTF-8 workspace path")?,
        ],
        &main,
    )?;

    assert!(inner.join(".jj").exists(), "expected a .jj marker");
    assert!(
        !inner.join(".git").exists(),
        "the inner workspace must carry .jj only, or the fixture proves nothing"
    );

    assert_eq!(
        InProcessProbe.discover(&inner),
        Some(inner.clone()),
        "discover must report the boundary, not the enclosing git repository"
    );

    // The paired negative: gix's own walk escapes on this very fixture, so the
    // containment above is not an accident of the shape.
    let escaped = gix::discover(&inner)?;
    let escaped = escaped
        .workdir()
        .ok_or("expected the escaped repository to have a workdir")?
        .canonicalize()?;
    assert_eq!(
        escaped, outer,
        "the control must escape, or the containment assertion is vacuous"
    );

    Ok(())
}

#[test]
fn a_colocated_checkout_roots_at_its_own_markers() -> Result<(), TestError> {
    require("jj")?;
    require("git")?;
    let root = colocated_repo("colocated")?;
    let root = path_of(&root)?;

    assert_eq!(InProcessProbe.discover(&root), Some(root.clone()));
    Ok(())
}

#[test]
fn a_worktree_whose_git_marker_is_a_file_roots_at_itself(
) -> Result<(), TestError> {
    require("git")?;
    let primary = git_repo_with_a_commit("worktree-primary")?;
    let primary = path_of(&primary)?;
    let worktree_root = tempdir("worktree")?;
    let worktree = path_of(&worktree_root)?.join("checkout");
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
    assert_eq!(InProcessProbe.discover(&worktree), Some(worktree));
    Ok(())
}

#[test]
fn a_tree_with_no_marker_has_no_boundary() -> Result<(), TestError> {
    let loose = tempdir("loose")?;
    let loose = path_of(&loose)?;
    assert_eq!(InProcessProbe.discover(&loose), None);
    Ok(())
}

#[test]
fn the_walk_finds_the_boundary_from_a_nested_directory() -> Result<(), TestError>
{
    require("git")?;
    let root = git_repo_with_a_commit("nested-dir")?;
    let root = path_of(&root)?;
    let nested = root.join("meta/work");
    fs::create_dir_all(&nested)?;

    assert_eq!(InProcessProbe.discover(&nested), Some(root));
    Ok(())
}

// --- Parity with the subprocess pair ---

/// Asserts the two implementations agree on every port method, both idioms
/// included.
///
/// `revision` is compared unconditionally. The fixtures are built and then read
/// with no intervening edit, so the jj working copy's recorded operation is
/// current and the snapshot the `jj` binary takes is a no-op; the one shape
/// where the two legitimately differ is pinned separately by
/// `an_unsnapshotted_edit_is_the_one_documented_divergence`.
fn assert_parity(root: &Path) {
    let kind = InProcessProbe.kind(root);
    assert_eq!(kind, CommandProbe::new().kind(root), "kind disagreed");
    assert_eq!(
        InProcessProbe.repository_root(root),
        MarkerWalkRoot.repository_root(root),
        "repository_root disagreed"
    );
    assert_eq!(
        InProcessProbe.revision(root, kind),
        CommandProbe::new().revision(root, kind),
        "revision disagreed"
    );
}

#[test]
fn a_plain_git_repository_agrees_with_the_subprocess_pair(
) -> Result<(), TestError> {
    require("git")?;
    let root = git_repo_with_a_commit("parity-git")?;
    let root = path_of(&root)?;

    assert_eq!(InProcessProbe.kind(&root), VcsKind::Git);
    assert_parity(&root);
    Ok(())
}

#[test]
fn a_commitless_repository_agrees_and_reports_no_revision(
) -> Result<(), TestError> {
    require("git")?;
    let root = tempdir("parity-commitless")?;
    let root = path_of(&root)?;
    run("git", &["init", "--quiet"], &root)?;

    assert_eq!(
        InProcessProbe.revision(&root, VcsKind::Git),
        None,
        "a commitless repository has no revision, and that is not a failure"
    );
    assert_parity(&root);
    Ok(())
}

#[test]
fn a_colocated_repository_agrees_and_is_driven_as_jj() -> Result<(), TestError>
{
    require("jj")?;
    require("git")?;
    let root = colocated_repo("parity-colocated")?;
    let root = path_of(&root)?;

    assert_eq!(
        InProcessProbe.kind(&root),
        VcsKind::Jj,
        "jj must win over git in a colocated checkout"
    );
    assert_parity(&root);
    Ok(())
}

#[test]
fn a_secondary_workspace_resolves_to_the_repository_it_shares(
) -> Result<(), TestError> {
    require("jj")?;
    require("git")?;
    let main = colocated_repo("parity-main")?;
    let main = path_of(&main)?;
    let secondary_root = tempdir("parity-secondary")?;
    let secondary = path_of(&secondary_root)?.join("workspace");
    run(
        "jj",
        &[
            "workspace",
            "add",
            secondary.to_str().ok_or("non-UTF-8 workspace path")?,
        ],
        &main,
    )?;

    assert_eq!(
        InProcessProbe.repository_root(&secondary),
        main,
        "a secondary workspace resolves through the loader to its main \
         repository, not to its own working copy"
    );
    assert_parity(&secondary);
    Ok(())
}

#[test]
fn a_main_workspace_resolves_to_itself() -> Result<(), TestError> {
    require("jj")?;
    require("git")?;
    let root = colocated_repo("parity-main-workspace")?;
    let root = path_of(&root)?;

    assert_eq!(InProcessProbe.repository_root(&root), root);
    assert_parity(&root);
    Ok(())
}

// --- The jj revision route ---

/// The commit id the `jj` binary reports for `@`.
fn jj_revision_oracle(root: &Path) -> Result<String, TestError> {
    let output = Command::new("jj")
        .args(["log", "-r", "@", "--no-graph", "-T", "commit_id"])
        .current_dir(root)
        .output()?;
    assert!(output.status.success(), "the jj oracle failed: {output:?}");
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[test]
fn a_pure_jj_workspace_reports_its_working_copy_commit() -> Result<(), TestError>
{
    require("jj")?;
    let dir = tempdir("revision-pure-jj")?;
    let root = path_of(&dir)?;
    run("jj", &["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join("file"), "content")?;
    run("jj", &["commit", "-m", "one"], &root)?;

    // No git HEAD to fall back to, so this can only have come from the jj side.
    assert!(!root.join(".git").exists(), "the fixture must be pure jj");
    assert_eq!(
        InProcessProbe.revision(&root, VcsKind::Jj),
        Some(jj_revision_oracle(&root)?),
    );
    Ok(())
}

#[test]
fn each_workspace_reports_its_own_commit_not_its_neighbour_s(
) -> Result<(), TestError> {
    require("jj")?;
    require("git")?;
    let main = colocated_repo("revision-ws-main")?;
    let main = path_of(&main)?;
    let secondary_root = tempdir("revision-ws-secondary")?;
    let secondary = path_of(&secondary_root)?.join("workspace");
    run(
        "jj",
        &[
            "workspace",
            "add",
            "--name",
            "inner",
            secondary.to_str().ok_or("non-UTF-8 workspace path")?,
        ],
        &main,
    )?;

    let main_revision = InProcessProbe.revision(&main, VcsKind::Jj);
    let secondary_revision = InProcessProbe.revision(&secondary, VcsKind::Jj);

    assert_eq!(main_revision, Some(jj_revision_oracle(&main)?));
    assert_eq!(secondary_revision, Some(jj_revision_oracle(&secondary)?));
    // The load-bearing one: taking the view's sole entry would pass every
    // single-workspace test and answer for the wrong workspace here.
    assert_ne!(
        main_revision, secondary_revision,
        "two workspaces of one repository must not report the same commit"
    );
    Ok(())
}

#[test]
fn an_unreadable_checkout_state_reports_absence_rather_than_a_wrong_commit(
) -> Result<(), TestError> {
    require("jj")?;
    let dir = tempdir("revision-broken-checkout")?;
    let root = path_of(&dir)?;
    run("jj", &["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join("file"), "content")?;
    run("jj", &["commit", "-m", "one"], &root)?;

    let checkout = root.join(".jj/working_copy/checkout");
    assert!(
        InProcessProbe.revision(&root, VcsKind::Jj).is_some(),
        "the fixture must answer before it is broken, or this proves nothing"
    );
    fs::write(&checkout, b"not a protobuf")?;

    // The port carries no error channel, so a failure is warn-logged absence.
    // What must not happen is a panic or a stale answer.
    assert_eq!(InProcessProbe.revision(&root, VcsKind::Jj), None);
    Ok(())
}

#[test]
fn unreadable_git_ref_data_reports_absence_rather_than_a_wrong_commit(
) -> Result<(), TestError> {
    require("git")?;
    let dir = git_repo_with_a_commit("revision-broken-head")?;
    let root = path_of(&dir)?;

    assert!(
        InProcessProbe.revision(&root, VcsKind::Git).is_some(),
        "the fixture must answer before it is broken, or this proves nothing"
    );
    fs::write(root.join(".git/HEAD"), b"\xff\xfe truncated")?;

    assert_eq!(InProcessProbe.revision(&root, VcsKind::Git), None);
    Ok(())
}

#[test]
fn reading_the_revision_writes_nothing() -> Result<(), TestError> {
    require("jj")?;
    let dir = tempdir("revision-readonly")?;
    let root = path_of(&dir)?;
    run("jj", &["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join("file"), "content")?;
    run("jj", &["commit", "-m", "one"], &root)?;

    // One jj store loader in this area does create directories on load, so
    // fingerprint the whole tree rather than spot-checking.
    let before = fingerprint(&root.join(".jj"))?;
    assert!(InProcessProbe.revision(&root, VcsKind::Jj).is_some());
    assert_eq!(
        before,
        fingerprint(&root.join(".jj"))?,
        "the probe wrote to .jj"
    );
    Ok(())
}

#[test]
fn an_unsnapshotted_edit_is_the_one_documented_divergence(
) -> Result<(), TestError> {
    require("jj")?;
    let dir = tempdir("revision-snapshot")?;
    let root = path_of(&dir)?;
    run("jj", &["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join("file"), "content")?;
    run("jj", &["commit", "-m", "one"], &root)?;

    let recorded = InProcessProbe.revision(&root, VcsKind::Jj);
    assert_eq!(
        recorded,
        Some(jj_revision_oracle(&root)?),
        "in sync to begin"
    );

    // No jj command, so the recorded operation falls behind the working copy.
    fs::write(root.join("untracked"), "edited outside jj")?;

    // Still the last recorded commit: this route does not snapshot.
    assert_eq!(
        InProcessProbe.revision(&root, VcsKind::Jj),
        recorded,
        "the in-process route must not snapshot"
    );

    // The subprocess probe snapshots first, so it reports a different commit —
    // and creates it. That write is what the in-process route removes.
    let snapshotted = CommandProbe::new().revision(&root, VcsKind::Jj);
    assert!(snapshotted.is_some());
    assert_ne!(
        snapshotted, recorded,
        "asking the jj binary snapshots, so it should report a new commit here"
    );

    // The recorded state has now moved, and the two agree again.
    assert_eq!(
        InProcessProbe.revision(&root, VcsKind::Jj),
        snapshotted,
        "after the binary snapshots, the recorded commit is the new one"
    );
    Ok(())
}

/// Sorted `path:len` for every file under `dir`, so a create, delete or resize
/// anywhere shows up. Lengths rather than mtimes: mtime granularity varies by
/// filesystem, and a rewrite that preserves length is not a shape jj produces.
fn fingerprint(dir: &Path) -> Result<Vec<String>, TestError> {
    let mut entries = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else {
                entries.push(format!(
                    "{}:{}",
                    path.display(),
                    entry.metadata()?.len()
                ));
            }
        }
    }
    entries.sort();
    Ok(entries)
}
