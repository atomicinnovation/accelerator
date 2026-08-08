//! `--skip`/`--unskip`/`--unapply` driven end to end against the compiled
//! binary: exact stdout strings, exit codes, and ledger file mutation.

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

fn run(
    dir: &TempDir,
    args: &[&str],
) -> Result<(String, String, i32), TestError> {
    let output = Command::new(BIN)
        .args(args)
        .current_dir(dir.path())
        .output()?;
    Ok((
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?,
        output.status.code().unwrap_or(-1),
    ))
}

#[test]
fn skip_appends_the_id_and_prints_confirmation() -> Result<(), TestError> {
    let dir = project()?;

    let (stdout, stderr, code) = run(&dir, &["--skip", "0001-example"])?;

    assert_eq!(stdout, "Skipped migration: 0001-example\n");
    assert_eq!(stderr, "");
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(
            dir.path().join(".accelerator/state/migrations-skipped")
        )?,
        "0001-example\n"
    );
    Ok(())
}

#[test]
fn skip_is_idempotent() -> Result<(), TestError> {
    let dir = project()?;
    run(&dir, &["--skip", "0001-example"])?;
    run(&dir, &["--skip", "0001-example"])?;

    assert_eq!(
        fs::read_to_string(
            dir.path().join(".accelerator/state/migrations-skipped")
        )?,
        "0001-example\n"
    );
    Ok(())
}

#[test]
fn unskip_removes_the_id_and_prints_confirmation() -> Result<(), TestError> {
    let dir = project()?;
    run(&dir, &["--skip", "0001-example"])?;

    let (stdout, _stderr, code) = run(&dir, &["--unskip", "0001-example"])?;

    assert_eq!(stdout, "Unskipped migration: 0001-example\n");
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(
            dir.path().join(".accelerator/state/migrations-skipped")
        )?,
        ""
    );
    Ok(())
}

#[test]
fn unskip_of_an_absent_id_is_a_no_op_that_still_succeeds(
) -> Result<(), TestError> {
    let dir = project()?;

    let (stdout, _stderr, code) = run(&dir, &["--unskip", "9999-nope"])?;

    assert_eq!(stdout, "Unskipped migration: 9999-nope\n");
    assert_eq!(code, 0);
    Ok(())
}

#[test]
fn unapply_removes_the_id_from_the_applied_ledger() -> Result<(), TestError> {
    let dir = project()?;
    fs::create_dir_all(dir.path().join(".accelerator/state"))?;
    fs::write(
        dir.path().join(".accelerator/state/migrations-applied"),
        "0001-example\n0002-other\n",
    )?;

    let (stdout, _stderr, code) = run(&dir, &["--unapply", "0001-example"])?;

    assert_eq!(stdout, "Unapplied migration: 0001-example\n");
    assert_eq!(code, 0);
    assert_eq!(
        fs::read_to_string(
            dir.path().join(".accelerator/state/migrations-applied")
        )?,
        "0002-other\n"
    );
    Ok(())
}

#[test]
fn no_id_validation_against_the_registry_matches_bash() -> Result<(), TestError>
{
    let dir = project()?;

    let (_stdout, _stderr, code) =
        run(&dir, &["--skip", "not-a-real-migration"])?;

    assert_eq!(code, 0);
    Ok(())
}
