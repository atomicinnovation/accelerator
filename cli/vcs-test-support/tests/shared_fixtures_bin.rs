use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;
use std::process::Output;

use vcs_test_support::family::SharedFamily as _;
use vcs_test_support::fixture_log::FIXTURE_LOG_VARIABLE;
use vcs_test_support::stalled_migration::StallKey;
use vcs_test_support::stalled_migration::StalledMigrations;
use vcs_test_support::stalled_migration::MIGRATE_VARIABLE;

type TestError = Box<dyn std::error::Error>;

const FORCED_FAILURE_VARIABLE: &str = "ACCELERATOR_SHARED_FIXTURES_FAIL";

const STALLING: &str = "#!/bin/sh
echo 'MIGRATION STALLED' >&2
exit 1
";

const STALLING_EXCEPT_IN_ONE_STALL: &str = "#!/bin/sh
case \"$(basename \"$PWD\")\" in
pure-merge-with-sibling) echo 'no stall here' >&2; exit 0 ;;
esac
echo 'MIGRATION STALLED' >&2
exit 1
";

fn shared_fixtures() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_shared-fixtures"));
    command
        .env_remove(FIXTURE_LOG_VARIABLE)
        .env_remove(FORCED_FAILURE_VARIABLE)
        .env_remove(MIGRATE_VARIABLE);
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
             expected matrix or status-log or stalled-migration"
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

#[test]
fn the_stalled_migration_set_needs_the_migrate_executable(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let tree = directory.path().join("stalled-migration");

    let output = shared_fixtures()
        .arg("--stalled-migration")
        .arg(&tree)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains(
            "building the stalled-migration set needs \
             ACCELERATOR_SHARED_FIXTURES_MIGRATE"
        ),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn a_stall_that_fails_to_build_is_named_and_the_others_still_build(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let tree = directory.path().join("stalled-migration");
    let migrate = directory.path().join("migrate");
    fs::write(&migrate, STALLING_EXCEPT_IN_ONE_STALL)?;
    fs::set_permissions(&migrate, fs::Permissions::from_mode(0o755))?;

    let output = shared_fixtures()
        .env(MIGRATE_VARIABLE, &migrate)
        .arg("--stalled-migration")
        .arg(&tree)
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(&output);
    assert!(stderr.starts_with("stalled-migration: "), "{stderr}");
    assert!(stderr.contains("pure-merge-with-sibling"), "{stderr}");
    assert!(stderr.contains("no stall here"), "{stderr}");
    for built in [
        "pure-single-parent",
        "colocated-single-parent",
        "colocated-merge-with-sibling",
    ] {
        assert!(tree.join(built).join(".jj").is_dir(), "{built}: {stderr}");
    }
    Ok(())
}

#[test]
fn a_build_in_which_every_stall_stalls_is_adopted_whole(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let tree = directory.path().join("stalled-migration");
    let migrate = directory.path().join("migrate");
    fs::write(&migrate, STALLING)?;
    fs::set_permissions(&migrate, fs::Permissions::from_mode(0o755))?;

    let output = shared_fixtures()
        .env(MIGRATE_VARIABLE, &migrate)
        .arg("--stalled-migration")
        .arg(&tree)
        .output()?;

    assert!(output.status.success(), "{}", stderr(&output));
    let adopted = StalledMigrations::adopt(&tree)?;
    let keys: Vec<StallKey> = adopted.stalls.keys().copied().collect();
    assert_eq!(keys, StallKey::ALL);
    for stall in adopted.stalls.values() {
        assert!(stall.root.join(".jj").is_dir(), "{}", stall.root.display());
    }
    Ok(())
}
