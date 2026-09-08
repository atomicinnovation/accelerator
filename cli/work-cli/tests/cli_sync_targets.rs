//! CLI-boundary tests for `work sync --target`.
//!
//! `accelerator-work` is bin-only, so a subprocess cannot inject a fake
//! tracker; this suite covers exactly what target resolution guarantees from
//! outside the process: the resolution exit codes (3/6/2), that every abort is
//! credential-independent (it precedes the tracker's credential check) and
//! writes nothing, and that a valid target reaches the tracker phase.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

mod common;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-sync-targets-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(
        dir.path().join(".accelerator/config.md"),
        "---\nwork:\n  integration: jira\n---\n",
    )?;
    Ok(dir)
}

fn work_item(
    dir: &Path,
    id: &str,
    external: Option<&str>,
) -> Result<(), TestError> {
    let external_line = external
        .map(|value| format!("external_id: \"{value}\"\n"))
        .unwrap_or_default();
    fs::write(
        dir.join(format!("meta/work/{id}-title.md")),
        format!("---\nid: \"{id}\"\n{external_line}---\n\nBody\n"),
    )?;
    Ok(())
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .arg("sync")
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null());
    common::scrub_provider_env(&mut command);
    Ok(command.output()?)
}

fn baseline_exists(dir: &Path) -> bool {
    fn walk(dir: &Path) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                if walk(&path) {
                    return true;
                }
            } else if path.file_name().and_then(std::ffi::OsStr::to_str)
                == Some("last-sync.json")
            {
                return true;
            }
        }
        false
    }
    walk(dir)
}

#[test]
fn a_no_match_target_exits_three_before_the_credential_check(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    work_item(repo.path(), "0001", Some("PP-1"))?;
    let output = run(repo.path(), &["--target", "9999"])?;
    assert_eq!(
        output.status.code(),
        Some(3),
        "a no-match target aborts before the tracker credential check (74)"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("9999"), "{stderr}");
    assert!(
        !baseline_exists(repo.path()),
        "the abort writes no baseline"
    );
    Ok(())
}

#[test]
fn an_out_of_directory_path_target_exits_six() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    work_item(repo.path(), "0001", Some("PP-1"))?;
    fs::write(repo.path().join("meta/outside.md"), "---\nid: \"x\"\n---\n")?;
    let output = run(repo.path(), &["--target", "meta/outside.md"])?;
    assert_eq!(output.status.code(), Some(6));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("meta/outside.md"), "{stderr}");
    assert!(!baseline_exists(repo.path()));
    Ok(())
}

#[test]
fn an_unmanaged_in_directory_file_exits_three() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    work_item(repo.path(), "0001", Some("PP-1"))?;
    // A real .md inside the work directory that is not a discovered work item
    // (no `id` frontmatter, so `discover_items` skips it).
    fs::write(repo.path().join("meta/work/notes.md"), "not a work item\n")?;
    let output = run(repo.path(), &["--target", "meta/work/notes.md"])?;
    assert_eq!(output.status.code(), Some(3));
    assert!(!baseline_exists(repo.path()));
    Ok(())
}

#[test]
fn an_empty_target_token_is_a_usage_error() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    work_item(repo.path(), "0001", Some("PP-1"))?;
    let output = run(repo.path(), &["--target", ""])?;
    assert_eq!(output.status.code(), Some(2));
    assert!(!baseline_exists(repo.path()));
    Ok(())
}

#[test]
fn a_no_match_and_an_out_of_dir_path_exit_six_naming_both(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    work_item(repo.path(), "0001", Some("PP-1"))?;
    fs::write(repo.path().join("meta/outside.md"), "---\nid: \"x\"\n---\n")?;
    let output = run(
        repo.path(),
        &["--target", "9999", "--target", "meta/outside.md"],
    )?;
    assert_eq!(
        output.status.code(),
        Some(6),
        "the out-of-directory code outranks the no-match code"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("9999"), "{stderr}");
    assert!(stderr.contains("meta/outside.md"), "{stderr}");
    assert!(!baseline_exists(repo.path()));
    Ok(())
}

#[test]
fn a_local_local_collision_exits_two_naming_both_files() -> Result<(), TestError>
{
    let repo = scratch_repo()?;
    // File A carries local id 0001 and no external_id; file B records 0001 as
    // its external_id, so targeting "0001" is a genuine local/local collision.
    work_item(repo.path(), "0001", None)?;
    work_item(repo.path(), "0002", Some("0001"))?;
    let output = run(repo.path(), &["--target", "0001"])?;
    assert_eq!(
        output.status.code(),
        Some(2),
        "a local/local collision is a usage error, decided from the corpus \
         with no remote call"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("0001-title.md"), "names file A: {stderr}");
    assert!(stderr.contains("0002-title.md"), "names file B: {stderr}");
    assert!(
        !baseline_exists(repo.path()),
        "the abort writes no baseline"
    );
    Ok(())
}

#[test]
fn a_valid_target_resolves_then_reaches_the_credential_check(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    work_item(repo.path(), "0001", Some("PP-1"))?;
    let output = run(repo.path(), &["--target", "0001"])?;
    assert_eq!(
        output.status.code(),
        Some(74),
        "a resolved target reaches the credential-less tracker phase, not a \
         resolution code"
    );
    assert!(
        !baseline_exists(repo.path()),
        "the credential abort writes no baseline"
    );
    Ok(())
}
