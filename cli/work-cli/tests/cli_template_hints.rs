//! CLI-boundary tests for `work template-hints`, cross-checked against the
//! Phase 1 `work-item-template-field-hints.golden` fixture's
//! comment-present rows (the `template-unreadable` rows exercise the same
//! hardcoded-fallback code path as `comment-absent`, per that golden's own
//! note, so are not separately re-exercised here).

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-template-hints-")
        .tempdir()?;
    let templates_dir = dir.path().join("templates");
    fs::create_dir_all(&templates_dir)?;
    fs::copy(
        repo_root()?.join("templates/work-item.md"),
        templates_dir.join("work-item.md"),
    )?;
    Ok(dir)
}

fn run(dir: &Path, field: &str) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(["template-hints", field])
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .output()?)
}

#[test]
fn kind_hints_come_from_the_trailing_comment() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), "kind")?;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "story\nepic\ntask\nbug\nspike\n"
    );
    Ok(())
}

#[test]
fn status_hints_come_from_the_trailing_comment() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), "status")?;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "draft\nready\nin-progress\nreview\ndone\nblocked\nabandoned\n"
    );
    Ok(())
}

#[test]
fn priority_hints_come_from_the_trailing_comment() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), "priority")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "high\nmedium\nlow\n");
    Ok(())
}

#[test]
fn a_field_with_no_trailing_comment_yields_nothing() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), "title")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "");
    Ok(())
}

#[test]
fn an_unknown_field_exits_zero_with_no_output() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), "not_a_real_field")?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "");
    Ok(())
}
