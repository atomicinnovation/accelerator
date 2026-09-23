//! `VcsWorkingCopy` against a real repository: the run base and the changes
//! the migrate pre-flight's unowned-changes gate actually sees.
#![cfg(feature = "bash-parity")]

use std::fs;

use migrate::ports::WorkingCopy;
use migrate::preflight::SCOPES;
use migrate::run_base::RunBase;
use migrate_adapters::working_copy::VcsWorkingCopy;
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

/// An uncommitted document under a scope is an unowned change: the migration
/// would rewrite content no commit holds, and no revert could bring it back.
#[test]
fn an_untracked_document_in_scope_is_reported() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let (work, _env) = committed_repo("untracked")?;
    let root = work.path().join("repo");
    fs::write(root.join("meta/work/0002-new.md"), "new\n")?;

    let working_copy = VcsWorkingCopy::new(&root, VcsKind::Git);

    assert_eq!(
        working_copy.observe(&SCOPES)?.dirty_paths,
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

    let working_copy = VcsWorkingCopy::new(&root, VcsKind::Git);

    assert!(working_copy.observe(&SCOPES)?.dirty_paths.is_empty());
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

    let working_copy = VcsWorkingCopy::new(&root, VcsKind::Git);

    assert!(working_copy.observe(&SCOPES)?.dirty_paths.is_empty());
    Ok(())
}

#[test]
fn a_clean_tree_reports_nothing() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let (work, _env) = committed_repo("clean")?;
    let root = work.path().join("repo");

    let working_copy = VcsWorkingCopy::new(&root, VcsKind::Git);

    assert!(working_copy.observe(&SCOPES)?.dirty_paths.is_empty());
    Ok(())
}

#[test]
fn a_git_working_copy_is_on_the_run_base_of_its_head() -> Result<(), TestError>
{
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let (work, env) = committed_repo("run-base")?;
    let root = work.path().join("repo");
    let head = env.git(&["rev-parse", "HEAD"], &root)?;

    let observation =
        VcsWorkingCopy::new(&root, VcsKind::Git).observe(&SCOPES)?;

    assert_eq!(observation.run_base, RunBase::recorded(&head));
    Ok(())
}

#[test]
fn an_unreadable_repository_has_no_run_base_and_no_changes(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let (work, _env) = committed_repo("unreadable")?;
    let root = work.path().join("repo");
    fs::write(root.join("meta/work/0002-new.md"), "new\n")?;
    fs::write(root.join(".git/HEAD"), "not a reference\n")?;

    let observation =
        VcsWorkingCopy::new(&root, VcsKind::Git).observe(&SCOPES)?;

    assert_eq!(observation.run_base, None);
    assert!(observation.dirty_paths.is_empty());
    Ok(())
}
