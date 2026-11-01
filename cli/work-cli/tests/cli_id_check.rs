//! CLI-boundary tests for `work id check`: settling an ID for a new item
//! without writing anything.

#![allow(clippy::literal_string_with_formatting_args)]

use std::fs;
use std::path::Path;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-id-check-")
        .tempdir()?;
    fs::create_dir_all(dir.path().join(".git"))?;
    fs::create_dir_all(dir.path().join("meta/work"))?;
    Ok(dir)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(["id", "check"])
        .args(args)
        .current_dir(dir)
        .output()?)
}

#[test]
fn an_exact_id_exits_0_printing_exact() -> Result<(), TestError> {
    let repo = scratch_repo()?;

    let output = run(repo.path(), &["0400"])?;

    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout)?, "exact\t0400\n");
    Ok(())
}

#[test]
fn a_refused_id_exits_1_with_its_code_on_stderr() -> Result<(), TestError> {
    let repo = scratch_repo()?;

    let output = run(repo.path(), &["foo"])?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.starts_with("Error: E_ID_PATTERN_MISMATCH:"),
        "{stderr}"
    );
    assert!(stderr.contains("{number:04d}"), "{stderr}");
    Ok(())
}

#[test]
fn an_id_starting_with_a_hyphen_is_read_after_the_separator(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;

    let output = run(repo.path(), &["--", "-0400"])?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.starts_with("Error: E_ID_PATTERN_MISMATCH:"),
        "{stderr}"
    );
    Ok(())
}
