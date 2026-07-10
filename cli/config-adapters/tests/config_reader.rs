//! Black-box tests of the compiled `config-adapters-fixture` composition root,
//! run with a per-test working directory carrying a VCS boundary marker so
//! discovery roots inside the fixture and never escapes into the real tree.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const READER: &str = env!("CARGO_BIN_EXE_config-adapters-fixture");

static COUNTER: AtomicU64 = AtomicU64::new(0);

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "reader-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn run(dir: &Path) -> Output {
    Command::new(READER)
        .current_dir(dir)
        .output()
        .expect("run config-adapters-fixture")
}

fn run_with_migration_mode(dir: &Path) -> Output {
    Command::new(READER)
        .current_dir(dir)
        .env("ACCELERATOR_MIGRATION_MODE", "1")
        .output()
        .expect("run config-adapters-fixture")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn seed_legacy(root: &Path) -> TestResult {
    fs::create_dir_all(root.join(".claude"))?;
    fs::write(root.join(".claude/accelerator.md"), "legacy config\n")?;
    Ok(())
}

#[test]
fn a_legacy_layout_exits_non_zero_with_the_migrate_directive() -> TestResult {
    let root = workspace();
    fs::create_dir_all(root.join(".git"))?;
    seed_legacy(&root)?;

    let output = run(&root);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("/accelerator:migrate"));
    Ok(())
}

#[test]
fn a_legacy_layout_fails_closed_under_migration_mode() -> TestResult {
    let root = workspace();
    fs::create_dir_all(root.join(".git"))?;
    seed_legacy(&root)?;

    let output = run_with_migration_mode(&root);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("/accelerator:migrate"));
    Ok(())
}

#[test]
fn a_legacy_layout_blocks_from_a_git_rooted_subdirectory() -> TestResult {
    let root = workspace();
    fs::create_dir_all(root.join(".git"))?;
    seed_legacy(&root)?;
    let sub = root.join("nested/deeper");
    fs::create_dir_all(&sub)?;

    let output = run(&sub);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("/accelerator:migrate"));
    Ok(())
}

#[test]
fn a_legacy_layout_blocks_from_a_jj_only_subdirectory() -> TestResult {
    let root = workspace();
    fs::create_dir_all(root.join(".jj"))?;
    seed_legacy(&root)?;
    let sub = root.join("nested/deeper");
    fs::create_dir_all(&sub)?;

    let output = run(&sub);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("/accelerator:migrate"));
    Ok(())
}

#[test]
fn a_normal_layout_resolves_and_exits_zero() -> TestResult {
    let root = workspace();
    fs::create_dir_all(root.join(".git"))?;
    fs::create_dir_all(root.join(".accelerator"))?;
    fs::write(
        root.join(".accelerator/config.md"),
        "---\npaths:\n  work: resolved-work\n---\n",
    )?;

    let output = run(&root);
    assert!(output.status.success());
    assert_eq!(stdout(&output), "resolved-work\n");
    Ok(())
}

#[test]
fn a_normal_layout_without_paths_work_exits_non_zero() -> TestResult {
    let root = workspace();
    fs::create_dir_all(root.join(".git"))?;
    fs::create_dir_all(root.join(".accelerator"))?;
    fs::write(
        root.join(".accelerator/config.md"),
        "---\nother: value\n---\n",
    )?;

    let output = run(&root);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("paths.work not set"));
    Ok(())
}
