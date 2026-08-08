//! `--discoverability-hook`, driven end to end against the compiled binary.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

#[test]
fn silent_and_exit_zero_on_a_non_accelerator_repo() -> Result<(), TestError> {
    let dir = TempDir::new()?;

    let output = Command::new(BIN)
        .args(["--discoverability-hook", "--format=hook", "--fail-safe"])
        .current_dir(dir.path())
        .output()?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout)?, "");
    Ok(())
}

#[test]
fn emits_the_session_start_envelope_on_a_pending_repo() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    fs::create_dir_all(dir.path().join("meta"))?;

    let output = Command::new(BIN)
        .args(["--discoverability-hook", "--format=hook", "--fail-safe"])
        .current_dir(dir.path())
        .output()?;

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("\"systemMessage\""));
    assert!(stdout.contains("is behind the plugin"));
    Ok(())
}

#[test]
fn never_mutates_or_acquires_the_run_lock() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    fs::create_dir_all(dir.path().join("meta"))?;

    Command::new(BIN)
        .args(["--discoverability-hook", "--format=hook", "--fail-safe"])
        .current_dir(dir.path())
        .output()?;

    assert!(!dir.path().join(".accelerator").exists());
    Ok(())
}
