//! The library-backed probe against real repository shapes, alongside the
//! subprocess pair it will eventually replace.
//!
//! Two properties are under test. The boundary rule: `RepoRoot::discover`
//! reports the start path's nearest marked ancestor and never one above it,
//! paired with the negative assertion that an unbounded `gix::discover` on the
//! same fixture *does* escape — without which the containment claim could hold
//! vacuously. And parity: `kind`, `repository_root` and the git half of
//! `revision` agree with `CommandProbe`, so the swap 0185 makes is behaviour-
//! preserving on the shapes pinned here.
//!
//! These need real `jj` and `git` binaries and hard-fail when one is absent —
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
use vcs_adapters::CommandProbe;
use vcs_adapters::MarkerWalkRoot;

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

    // A jj secondary workspace planted inside an unrelated git repository: the
    // inner directory carries `.jj` only, so anything walking for `.git` alone
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

    // The paired negative: gix's own walk escapes the boundary on this very
    // fixture, so the containment above is the adapter's doing and not an
    // accident of the shape.
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

/// Asserts the two implementations agree on everything the library-backed probe
/// claims to answer. `revision` is compared for `VcsKind::Git` only: jj-lib
/// 0.43 exposes no read-only, settings-free route to the working-copy commit
/// id, so the jj half is `None` by design and is asserted as such.
fn assert_parity(root: &Path) {
    let kind = InProcessProbe.kind(root);
    assert_eq!(kind, CommandProbe::new().kind(root), "kind disagreed");
    assert_eq!(
        InProcessProbe.repository_root(root),
        MarkerWalkRoot.repository_root(root),
        "repository_root disagreed"
    );

    let subprocess = CommandProbe::new().revision(root, kind);
    let in_process = InProcessProbe.revision(root, kind);
    match kind {
        VcsKind::Git => {
            assert_eq!(in_process, subprocess, "git revision disagreed");
        }
        VcsKind::Jj => {
            assert_eq!(
                in_process, None,
                "the jj revision mechanism is descoped and must report absence"
            );
            assert!(
                subprocess.is_some(),
                "the subprocess probe should still answer, or this fixture \
                 proves nothing about the descope"
            );
        }
        VcsKind::None => assert_eq!(in_process, subprocess),
    }
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
