//! The empty-registry default run and the `--help` surface, driven end to
//! end against the compiled binary.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

fn project() -> Result<TempDir, TestError> {
    let dir = TempDir::new()?;
    fs::create_dir_all(dir.path().join(".git"))?;
    Ok(dir)
}

#[test]
fn an_empty_registry_prints_no_pending_migrations() -> Result<(), TestError> {
    let dir = project()?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(
        String::from_utf8(output.stdout)?,
        "No pending migrations.\n"
    );
    assert_eq!(String::from_utf8(output.stderr)?, "");
    assert_eq!(output.status.code(), Some(0));
    Ok(())
}

#[test]
fn an_empty_registry_with_a_skip_list_reports_it() -> Result<(), TestError> {
    let dir = project()?;
    Command::new(BIN)
        .args(["--skip", "0001-example"])
        .current_dir(dir.path())
        .output()?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(
        String::from_utf8(output.stdout)?,
        "No pending migrations.\nSkipped: 0001-example \n"
    );
    Ok(())
}

const EXPECTED_HELP: &str = "\
The `accelerator-migrate` command-line surface

Usage: accelerator-migrate [OPTIONS]

Options:
      --skip <id>              Mark migration <ID> skipped; do not run it
      --unskip <id>            Remove migration <ID> from the skip list
      --unapply <id>           Remove migration <ID> from the applied ledger so a half-applied migration can be re-run
      --list                   Dry-emit pending interactive transformations, one tab-delimited line each, then exit without mutating anything
      --decisions-file <path>  Scripted decisions for interactive migrations, one per line: accept | skip | edit <value>
  -h, --help                   Print help

Environment:
  ACCELERATOR_MIGRATE_DECISIONS_FILE=<path>  Same as --decisions-file.
  ACCELERATOR_MIGRATE_FORCE=1                Bypass the dirty-tree pre-flight.
";

#[test]
fn help_matches_the_committed_rust_native_snapshot_byte_for_byte(
) -> Result<(), TestError> {
    let output = Command::new(BIN).arg("--help").output()?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout)?, EXPECTED_HELP);
    Ok(())
}
