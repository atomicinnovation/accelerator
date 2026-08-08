//! CLI-boundary tests for `work canonicalise-id`, cross-checked against the
//! `work-item-canonicalise-id.golden` fixture.

// The `{project}`/`{number:04d}` DSL tokens in test literals are not
// formatting args.
#![allow(clippy::literal_string_with_formatting_args)]

use std::fs;
use std::path::Path;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo(
    pattern: &str,
    project: &str,
) -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-canonicalise-id-")
        .tempdir()?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    let config = format!(
        "---\nwork:\n  id_pattern: \"{pattern}\"\n  default_project_code: \"{project}\"\n---\n"
    );
    fs::write(dir.path().join(".accelerator/config.md"), config)?;
    Ok(dir)
}

fn run(dir: &Path, input: &str) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(["canonicalise-id", input])
        .current_dir(dir)
        .output()?)
}

#[test]
fn bare_number_under_a_project_less_pattern() -> Result<(), TestError> {
    let repo = scratch_repo("{number:04d}", "")?;
    let output = run(repo.path(), "42")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "0042");
    Ok(())
}

#[test]
fn already_zero_padded_number() -> Result<(), TestError> {
    let repo = scratch_repo("{number:04d}", "")?;
    let output = run(repo.path(), "0042")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "0042");
    Ok(())
}

#[test]
fn double_quoted_number_is_unwrapped() -> Result<(), TestError> {
    let repo = scratch_repo("{number:04d}", "")?;
    let output = run(repo.path(), "\"0042\"")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "0042");
    Ok(())
}

#[test]
fn single_quoted_number_is_unwrapped() -> Result<(), TestError> {
    let repo = scratch_repo("{number:04d}", "")?;
    let output = run(repo.path(), "'0042'")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "0042");
    Ok(())
}

#[test]
fn bare_number_under_a_project_pattern_with_project_configured(
) -> Result<(), TestError> {
    let repo = scratch_repo("{project}-{number:04d}", "PROJ")?;
    let output = run(repo.path(), "42")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "PROJ-0042");
    Ok(())
}

#[test]
fn bare_number_under_a_project_pattern_with_no_project_configured(
) -> Result<(), TestError> {
    let repo = scratch_repo("{project}-{number:04d}", "")?;
    let output = run(repo.path(), "42")?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(
        stderr.trim(),
        "E_PATTERN_MISSING_PROJECT: bare number '42' under pattern \
         '{project}-{number:04d}' requires a project value"
    );
    Ok(())
}

#[test]
fn already_canonical_full_id() -> Result<(), TestError> {
    let repo = scratch_repo("{project}-{number:04d}", "PROJ")?;
    let output = run(repo.path(), "PROJ-0042")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "PROJ-0042");
    Ok(())
}

#[test]
fn full_id_with_an_unpadded_number() -> Result<(), TestError> {
    let repo = scratch_repo("{project}-{number:04d}", "PROJ")?;
    let output = run(repo.path(), "PROJ-42")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "PROJ-0042");
    Ok(())
}

#[test]
fn empty_input_is_rejected() -> Result<(), TestError> {
    let repo = scratch_repo("{number:04d}", "")?;
    let output = run(repo.path(), "")?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(stderr.trim(), "E_PATTERN_BAD_FORMAT_SPEC: empty input");
    Ok(())
}

#[test]
fn unrecognised_id_shape_is_rejected() -> Result<(), TestError> {
    let repo = scratch_repo("{number:04d}", "")?;
    let output = run(repo.path(), "not-an-id")?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(
        stderr.trim(),
        "E_PATTERN_BAD_FORMAT_SPEC: input 'not-an-id' is not a recognised \
         ID shape"
    );
    Ok(())
}
