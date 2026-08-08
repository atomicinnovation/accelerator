//! CLI-boundary tests for `work next-number`: display-only allocation
//! preview, matching `work-item-next-number.sh --project PROJ --count N`.

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
