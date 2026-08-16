//! CLI-boundary tests for `update --push`.
//!
//! Same constraint as `cli_create_push.rs`: `accelerator-work` is bin-only,
//! so `tracker.update`'s branches are unreachable from a subprocess. What
//! is reachable — the unsynced refusal and the `SelectionError` exit codes
//! — is covered here.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-update-push-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    Ok(dir)
}

fn configure_integration(
    dir: &Path,
    integration: &str,
) -> Result<(), TestError> {
    fs::create_dir_all(dir.join(".accelerator"))?;
    fs::write(
        dir.join(".accelerator/config.md"),
        format!("---\nwork:\n  integration: {integration}\n---\n"),
    )?;
    Ok(())
}

fn write_item(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join("meta/work").join(name);
    fs::write(&path, content).expect("write fixture");
    path
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .arg("update")
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null())
        .output()?)
}

const SYNCED_ITEM: &str =
    "---\nid: \"0001\"\ntitle: Synced item\nexternal_id: ENG-1\n---\nBody\n";
const UNSYNCED_ITEM: &str =
    "---\nid: \"0002\"\ntitle: Unsynced item\n---\nBody\n";

#[test]
fn an_unsynced_target_is_refused_with_a_named_error() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_item(repo.path(), "0002-unsynced.md", UNSYNCED_ITEM);
    let output =
        run(repo.path(), &[path.to_str().expect("utf8 path"), "--push"])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_PUSH_UNSYNCED"), "{stderr}");
    let content = fs::read_to_string(&path)?;
    assert_eq!(content, UNSYNCED_ITEM, "the local file must be untouched");
    Ok(())
}

#[test]
fn an_empty_external_id_is_treated_as_unsynced() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let content =
        "---\nid: \"0003\"\ntitle: Blank id\nexternal_id: \"\"\n---\nBody\n";
    let path = write_item(repo.path(), "0003-blank.md", content);
    let output =
        run(repo.path(), &[path.to_str().expect("utf8 path"), "--push"])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_PUSH_UNSYNCED"), "{stderr}");
    Ok(())
}

#[test]
fn unset_work_integration_exits_73() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_item(repo.path(), "0001-synced.md", SYNCED_ITEM);
    let output =
        run(repo.path(), &[path.to_str().expect("utf8 path"), "--push"])?;
    assert_eq!(output.status.code(), Some(73));
    let content = fs::read_to_string(&path)?;
    assert_eq!(content, SYNCED_ITEM, "the local file must be untouched");
    Ok(())
}

#[test]
fn an_unrecognised_tracker_exits_73() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    configure_integration(repo.path(), "bogus-tracker")?;
    let path = write_item(repo.path(), "0001-synced.md", SYNCED_ITEM);
    let output =
        run(repo.path(), &[path.to_str().expect("utf8 path"), "--push"])?;
    assert_eq!(output.status.code(), Some(73));
    Ok(())
}

#[test]
fn a_recognised_but_unbuilt_tracker_exits_72() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    configure_integration(repo.path(), "jira")?;
    let path = write_item(repo.path(), "0001-synced.md", SYNCED_ITEM);
    let output =
        run(repo.path(), &[path.to_str().expect("utf8 path"), "--push"])?;
    assert_eq!(output.status.code(), Some(72));
    let content = fs::read_to_string(&path)?;
    assert_eq!(content, SYNCED_ITEM, "the local file must be untouched");
    Ok(())
}

#[test]
fn without_push_the_existing_update_behaviour_is_unchanged(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_item(repo.path(), "0001-synced.md", SYNCED_ITEM);
    let output = run(
        repo.path(),
        &[
            path.to_str().expect("utf8 path"),
            "--set",
            "status=in-progress",
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("status: in-progress"));
    Ok(())
}
