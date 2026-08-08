//! The dirty-tree pre-flight driven end to end against the compiled binary
//! over real git/jj repositories: foreign dirt refuses, `FORCE` bypasses.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::process::Command;

use tempfile::TempDir;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

fn tempdir(tag: &str) -> Result<TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("migrate-preflight-{tag}-"))
        .tempdir()?)
}

/// The compiled binary always runs the full registry (unlike bash's
/// `ACCELERATOR_MIGRATIONS_DIR` isolation), so every real migration is
/// pre-marked applied here to reach the same "nothing pending" state these
/// tests were written against when the registry was still empty.
fn mark_all_migrations_applied(
    root: &std::path::Path,
) -> Result<(), TestError> {
    fs::create_dir_all(root.join(".accelerator/state"))?;
    fs::write(
        root.join(".accelerator/state/migrations-applied"),
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n",
    )?;
    Ok(())
}

fn run(
    root: &std::path::Path,
    env_extra: &[(&str, &str)],
) -> Result<(String, String, i32), TestError> {
    let mut command = Command::new(BIN);
    command.current_dir(root);
    for (key, value) in env_extra {
        command.env(key, value);
    }
    let output = command.output()?;
    Ok((
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?,
        output.status.code().unwrap_or(-1),
    ))
}

#[test]
fn a_foreign_dirty_git_file_refuses() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-refuse")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    fs::write(root.join("meta/a.md"), "two\n")?;

    let (stdout, stderr, code) = run(&root, &[])?;

    assert_eq!(code, 1);
    assert_eq!(stdout, "");
    assert!(stderr.contains("dirty working tree"), "{stderr}");
    Ok(())
}

#[test]
fn force_bypasses_the_refusal_and_reaches_the_empty_registry_sentinel(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-force")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    fs::write(root.join("meta/a.md"), "two\n")?;
    mark_all_migrations_applied(&root)?;

    let (stdout, _stderr, code) =
        run(&root, &[("ACCELERATOR_MIGRATE_FORCE", "1")])?;

    assert_eq!(code, 0);
    assert_eq!(stdout, "No pending migrations.\n");
    Ok(())
}

#[test]
fn a_clean_git_tree_proceeds_to_the_empty_registry_sentinel(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-clean")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;
    mark_all_migrations_applied(&root)?;

    let (stdout, stderr, code) = run(&root, &[])?;

    assert_eq!(code, 0);
    assert_eq!(stdout, "No pending migrations.\n");
    assert_eq!(stderr, "");
    Ok(())
}

#[test]
fn a_foreign_dirty_jj_file_refuses() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let work = tempdir("jj-refuse")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.jj(&["commit", "-m", "init"], &root)?;

    fs::write(root.join("meta/b.md"), "two\n")?;

    let (stdout, stderr, code) = run(&root, &[])?;

    assert_eq!(code, 1);
    assert_eq!(stdout, "");
    assert!(stderr.contains("dirty working tree"), "{stderr}");
    Ok(())
}
