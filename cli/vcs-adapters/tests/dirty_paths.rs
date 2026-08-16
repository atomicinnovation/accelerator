//! `InProcessProbe::dirty_paths` against real jj/git repositories, compared
//! against the shape `git status --porcelain`/`jj diff --name-only` reports.
#![cfg(feature = "bash-parity")]

use std::fs;

use vcs::VcsKind;
use vcs_adapters::library::InProcessProbe;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("vcs-dirty-paths-{tag}-"))
        .tempdir()?)
}

/// An untracked file counts as dirty because it is the least recoverable
/// thing in the tree: overwriting one destroys content no commit holds. Both
/// idioms report it, so the two backends return the same list.
#[test]
fn git_reports_a_modified_tracked_file_and_an_untracked_one(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    fs::write(root.join("meta/a.md"), "two\n")?;
    fs::write(root.join("meta/untracked.md"), "x\n")?;

    let probe = InProcessProbe;
    let mut paths = probe.dirty_paths(&root, VcsKind::Git)?;
    paths.sort();

    assert_eq!(
        paths,
        vec!["meta/a.md".to_owned(), "meta/untracked.md".to_owned()]
    );
    Ok(())
}

#[test]
fn git_excludes_an_ignored_file() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-ignored")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join(".gitignore"), "build/\n*.log\n")?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "."], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    fs::create_dir_all(root.join("build"))?;
    fs::write(root.join("build/artifact.md"), "x\n")?;
    fs::write(root.join("noisy.log"), "x\n")?;

    let probe = InProcessProbe;
    let paths = probe.dirty_paths(&root, VcsKind::Git)?;

    assert!(paths.is_empty(), "{paths:?}");
    Ok(())
}

#[test]
fn jj_excludes_an_ignored_file() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let work = tempdir("jj-ignored")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join(".gitignore"), "build/\n*.log\n")?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.jj(&["commit", "-m", "init"], &root)?;

    fs::create_dir_all(root.join("build"))?;
    fs::write(root.join("build/artifact.md"), "x\n")?;
    fs::write(root.join("noisy.log"), "x\n")?;

    let probe = InProcessProbe;
    let paths = probe.dirty_paths(&root, VcsKind::Jj)?;

    assert!(paths.is_empty(), "{paths:?}");
    Ok(())
}

#[test]
fn git_on_a_clean_tree_reports_nothing() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-clean")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    let probe = InProcessProbe;
    let paths = probe.dirty_paths(&root, VcsKind::Git)?;

    assert!(paths.is_empty());
    Ok(())
}

/// The current op head, read directly off disk rather than via `jj op log` —
/// any real `jj` command auto-snapshots the working copy before it runs, so
/// using the CLI itself as a before/after probe would record a head change
/// caused by the probe, not by the code under test.
fn op_heads(root: &std::path::Path) -> Result<Vec<String>, TestError> {
    let mut heads: Vec<String> =
        fs::read_dir(root.join(".jj/repo/op_heads/heads"))?
            .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
            .collect::<Result<_, TestError>>()?;
    heads.sort();
    Ok(heads)
}

#[test]
fn jj_reports_a_new_file_via_auto_track_and_never_writes_a_new_operation(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let work = tempdir("jj")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.jj(&["commit", "-m", "init"], &root)?;

    let heads_before = op_heads(&root)?;

    fs::write(root.join("meta/b.md"), "two\n")?;

    let probe = InProcessProbe;
    let mut paths = probe.dirty_paths(&root, VcsKind::Jj)?;
    paths.sort();

    assert_eq!(paths, vec!["meta/b.md".to_owned()]);
    assert_eq!(
        op_heads(&root)?,
        heads_before,
        "the in-process snapshot must not persist a new jj operation"
    );
    Ok(())
}

#[test]
fn jj_on_a_clean_tree_reports_nothing() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let work = tempdir("jj-clean")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.jj(&["commit", "-m", "init"], &root)?;

    let probe = InProcessProbe;
    let paths = probe.dirty_paths(&root, VcsKind::Jj)?;

    assert!(paths.is_empty());
    Ok(())
}
