use std::process::Command;
use std::process::Output;

use vcs_test_support::fixture_log::FIXTURE_LOG_VARIABLE;

type TestError = Box<dyn std::error::Error>;

const FORCED_FAILURE_VARIABLE: &str = "ACCELERATOR_SHARED_FIXTURES_FAIL";

fn shared_fixtures() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_shared-fixtures"));
    command
        .env_remove(FIXTURE_LOG_VARIABLE)
        .env_remove(FORCED_FAILURE_VARIABLE);
    command
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn naming_no_family_is_a_usage_error() -> Result<(), TestError> {
    let output = shared_fixtures().output()?;

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("usage"), "{}", stderr(&output));
    Ok(())
}

#[test]
fn a_forced_failure_fails_its_family_without_building() -> Result<(), TestError>
{
    let directory = tempfile::tempdir()?;
    let tree = directory.path().join("matrix");

    let output = shared_fixtures()
        .env(FORCED_FAILURE_VARIABLE, "matrix")
        .arg("--matrix")
        .arg(&tree)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(&output);
    assert!(stderr.starts_with("matrix: "), "{stderr}");
    assert!(stderr.contains(FORCED_FAILURE_VARIABLE), "{stderr}");
    assert!(!tree.exists());
    Ok(())
}

#[test]
fn a_forced_status_log_failure_fails_that_family_and_builds_nothing(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let matrix = directory.path().join("matrix");
    let status_log = directory.path().join("status-log");

    let output = shared_fixtures()
        .env(FORCED_FAILURE_VARIABLE, "status-log")
        .arg("--matrix")
        .arg(&matrix)
        .arg("--status-log")
        .arg(&status_log)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(&output);
    assert!(stderr.starts_with("status-log: "), "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(!matrix.exists());
    assert!(!status_log.exists());
    Ok(())
}

#[test]
fn a_forced_failure_naming_no_family_is_refused() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;

    let output = shared_fixtures()
        .env(FORCED_FAILURE_VARIABLE, "matrx")
        .arg("--matrix")
        .arg(directory.path().join("matrix"))
        .output()?;

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains(
            "ACCELERATOR_SHARED_FIXTURES_FAIL='matrx' is not a family; \
             expected matrix or status-log"
        ),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn without_git_every_requested_family_fails_unbuilt() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let nothing_on_path = tempfile::tempdir()?;
    let tree = directory.path().join("matrix");

    let output = shared_fixtures()
        .env("PATH", nothing_on_path.path())
        .arg("--matrix")
        .arg(&tree)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(&output);
    assert!(stderr.starts_with("matrix: git: "), "{stderr}");
    assert!(!tree.exists());
    Ok(())
}
