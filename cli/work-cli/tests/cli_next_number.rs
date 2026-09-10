//! CLI-boundary tests for `work next-number`: a display-only allocation
//! preview.

#![allow(clippy::literal_string_with_formatting_args)]

use std::fs;
use std::path::Path;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-next-number-")
        .tempdir()?;
    fs::create_dir_all(dir.path().join(".git"))?;
    fs::create_dir_all(dir.path().join("meta/work"))?;
    Ok(dir)
}

fn scratch_repo_with_work_config(
    work_body: &str,
) -> Result<tempfile::TempDir, TestError> {
    let dir = scratch_repo()?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(
        dir.path().join(".accelerator/config.md"),
        format!("---\nwork:\n{work_body}---\n"),
    )?;
    Ok(dir)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(["next-number"])
        .args(args)
        .current_dir(dir)
        .output()?)
}

#[test]
fn three_sequential_ids_from_an_empty_corpus() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), &["--count", "3"])?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "0001\n0002\n0003\n");
    Ok(())
}

#[test]
fn a_repeated_invocation_prints_the_same_starting_number(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let first = run(repo.path(), &["--count", "2"])?;
    assert!(first.status.success());
    let second = run(repo.path(), &["--count", "2"])?;
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    let entries: Vec<_> = fs::read_dir(repo.path().join("meta/work"))?
        .filter_map(Result::ok)
        .collect();
    assert!(entries.is_empty(), "next-number must never write a file");
    Ok(())
}

#[test]
fn allocates_after_the_highest_existing_number() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    fs::write(repo.path().join("meta/work/0007-existing.md"), "")?;
    let output = run(repo.path(), &["--count", "1"])?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "0008\n");
    Ok(())
}

#[test]
fn a_key_pattern_mints_from_work_key() -> Result<(), TestError> {
    let repo = scratch_repo_with_work_config(
        "  id_pattern: \"{key}-{number:04d}\"\n  key: \"PP\"\n",
    )?;
    let output = run(repo.path(), &["--count", "1"])?;
    assert!(output.status.success(), "{:?}", output.status);
    assert_eq!(String::from_utf8(output.stdout)?, "PP-0001\n");
    Ok(())
}

#[test]
fn a_legacy_tracker_less_config_mints_and_warns_once() -> Result<(), TestError>
{
    let repo = scratch_repo_with_work_config(
        "  id_pattern: \"{project}-{number:04d}\"\n  \
         default_project_code: \"PP\"\n",
    )?;
    let output = run(repo.path(), &["--count", "1"])?;
    assert!(output.status.success(), "{:?}", output.status);
    assert_eq!(String::from_utf8(output.stdout)?, "PP-0001\n");
    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(
        stderr.matches("1.25.0").count(),
        1,
        "expected exactly one deprecation warning, got: {stderr}"
    );
    assert!(stderr.contains("work.default_project_code"), "{stderr}");
    Ok(())
}

#[test]
fn a_key_pattern_without_work_key_is_a_config_error() -> Result<(), TestError> {
    let repo = scratch_repo_with_work_config(
        "  id_pattern: \"{key}-{number:04d}\"\n",
    )?;
    let output = run(repo.path(), &["--count", "1"])?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("work.key"), "{stderr}");
    Ok(())
}

#[test]
fn overflow_prints_the_partial_ids_that_fit_then_exits_one(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    fs::write(repo.path().join("meta/work/9998-near-cap.md"), "")?;
    let output = run(repo.path(), &["--count", "3"])?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8(output.stdout)?, "9999\n");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_PATTERN_OVERFLOW"));
    assert!(stderr.contains("number space exhausted"));
    Ok(())
}
