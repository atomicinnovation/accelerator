//! CLI-boundary tests for `work promote`: the refusals decided before any
//! tracker is contacted.

use std::fs;
use std::path::Path;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-promote-")
        .tempdir()?;
    fs::create_dir_all(dir.path().join(".git"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(
        dir.path().join(".accelerator/config.md"),
        "---\nwork:\n  integration: jira\n  id_pattern: \"{tracker}\"\n---\n",
    )?;
    fs::create_dir_all(dir.path().join("meta/work"))?;
    fs::write(
        dir.path().join("meta/work/0042-legacy.md"),
        "---\nid: \"0042\"\n---\n\n# 0042: Legacy\n",
    )?;
    Ok(dir)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(args)
        .current_dir(dir)
        .output()?)
}

#[test]
fn promoting_a_non_draft_prints_e_promote_not_a_draft() -> Result<(), TestError>
{
    let repo = scratch_repo()?;

    let output = run(repo.path(), &["promote", "0042"])?;

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_PROMOTE_NOT_A_DRAFT"), "{stderr}");
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn adopt_with_create_is_a_usage_error() -> Result<(), TestError> {
    let repo = scratch_repo()?;

    let output = run(
        repo.path(),
        &["promote", "draft-aaaaaa", "--adopt", "PP-900", "--create"],
    )?;

    assert_eq!(output.status.code(), Some(2));
    Ok(())
}
