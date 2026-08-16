//! `VcsDirtyPathScanner` against a real repository: what the migrate
//! preflight's foreign-dirt gate actually sees.
#![cfg(feature = "bash-parity")]

use std::fs;

use migrate::ports::DirtyPathScanner;
use migrate::preflight::SCOPES;
use migrate_adapters::dirty_path_scanner::VcsDirtyPathScanner;
use vcs::VcsKind;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

fn committed_repo(
    tag: &str,
) -> Result<(tempfile::TempDir, Hermetic), TestError> {
    let work = tempfile::Builder::new()
        .prefix(&format!("migrate-dirty-{tag}-"))
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta/work"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join(".gitignore"), "build/\n")?;
    fs::write(root.join("meta/work/0001-a.md"), "one\n")?;
    env.git(&["add", "."], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;
    Ok((work, env))
}

/// An uncommitted document under a scope is foreign dirt: the migration would
/// rewrite content no commit holds, and no revert could bring it back.
#[test]
fn an_untracked_document_in_scope_is_reported() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let (work, _env) = committed_repo("untracked")?;
    let root = work.path().join("repo");
    fs::write(root.join("meta/work/0002-new.md"), "new\n")?;

    let scanner = VcsDirtyPathScanner::new(&root, VcsKind::Git);

    assert_eq!(
        scanner.dirty_paths(&SCOPES)?,
        vec!["meta/work/0002-new.md".to_owned()]
    );
    Ok(())
}

#[test]
fn an_untracked_file_outside_every_scope_is_not_reported(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let (work, _env) = committed_repo("out-of-scope")?;
    let root = work.path().join("repo");
    fs::write(root.join("notes.md"), "loose\n")?;

    let scanner = VcsDirtyPathScanner::new(&root, VcsKind::Git);

    assert!(scanner.dirty_paths(&SCOPES)?.is_empty());
    Ok(())
}

#[test]
fn an_ignored_file_in_scope_is_not_reported() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let (work, _env) = committed_repo("ignored")?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta/build"))?;
    fs::write(root.join("meta/build/artifact.md"), "x\n")?;
    fs::write(root.join(".gitignore"), "build/\nmeta/build/\n")?;
    let env = Hermetic::rooted_at(work.path())?;
    env.git(&["add", ".gitignore"], &root)?;
    env.git(&["commit", "--quiet", "-m", "ignore"], &root)?;

    let scanner = VcsDirtyPathScanner::new(&root, VcsKind::Git);

    assert!(scanner.dirty_paths(&SCOPES)?.is_empty());
    Ok(())
}

#[test]
fn a_clean_tree_reports_nothing() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let (work, _env) = committed_repo("clean")?;
    let root = work.path().join("repo");

    let scanner = VcsDirtyPathScanner::new(&root, VcsKind::Git);

    assert!(scanner.dirty_paths(&SCOPES)?.is_empty());
    Ok(())
}
