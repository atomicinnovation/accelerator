//! CLI-boundary tests for `create --push`.
//!
//! `accelerator-work` is bin-only, so a subprocess always resolves through
//! `ConfiguredTrackers`, which has no client wired for any provider yet.
//! `tracker.create`'s success and failure branches are therefore
//! unreachable here and are unit-tested against `work::sync::push_decide`
//! instead; what this suite drives end to end is the pending-push marker
//! state machine and the `SelectionError` paths.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use work::sync::PendingPush;
use work::sync::RequestFingerprint;
use work_adapters::sync::pending_push;

mod common;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-create-push-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    let templates_dir = dir.path().join("templates");
    fs::create_dir_all(&templates_dir)?;
    fs::copy(
        repo_root()?.join("templates/work-item.md"),
        templates_dir.join("work-item.md"),
    )?;
    for (key, value) in [("user.email", "t@e.x"), ("user.name", "Test User")] {
        let status = Command::new("git")
            .args(["config", key, value])
            .current_dir(dir.path())
            .status()?;
        assert!(status.success());
    }
    Ok(dir)
}

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
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

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(std::process::Stdio::null());
    common::scrub_provider_env(&mut command);
    Ok(command.output()?)
}

const BODY: &str = "Just a body.\n";

fn marker_path(dir: &Path, integration: &str, slug: &str) -> PathBuf {
    pending_push::path(
        &dir.join(".accelerator/state/integrations"),
        integration,
        slug,
    )
}

fn write_marker(path: &Path, marker: &PendingPush) -> Result<(), TestError> {
    fs::create_dir_all(path.parent().expect("marker path has a parent"))?;
    fs::write(path, pending_push::render(marker))?;
    Ok(())
}

fn work_item_files(dir: &Path) -> Result<Vec<PathBuf>, TestError> {
    let mut entries: Vec<_> = fs::read_dir(dir.join("meta/work"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(std::ffi::OsStr::to_str) == Some("md")
        })
        .collect();
    entries.sort();
    Ok(entries)
}

#[test]
fn unset_work_integration_local_saves_and_still_writes_the_file(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let body_file = repo.path().join("body.txt");
    fs::write(&body_file, BODY)?;
    let output = run(
        repo.path(),
        &[
            "create",
            "Push me",
            "task",
            "medium",
            "--push",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let mut lines = stdout.lines();
    let path = lines.next().expect("path line");
    let row = lines.next().expect("outcome row");
    assert_eq!(row, "local-save\t");
    let content = fs::read_to_string(path)?;
    assert!(!content.contains("external_id"), "{content}");
    Ok(())
}

#[test]
fn an_unrecognised_tracker_also_local_saves() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    configure_integration(repo.path(), "bogus-tracker")?;
    let body_file = repo.path().join("body.txt");
    fs::write(&body_file, BODY)?;
    let output = run(
        repo.path(),
        &[
            "create",
            "Push me",
            "task",
            "medium",
            "--push",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(stdout.lines().nth(1), Some("local-save\t"));
    Ok(())
}

#[test]
fn a_wired_tracker_without_credentials_also_local_saves(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    configure_integration(repo.path(), "jira")?;
    let body_file = repo.path().join("body.txt");
    fs::write(&body_file, BODY)?;
    let output = run(
        repo.path(),
        &[
            "create",
            "Push me",
            "task",
            "medium",
            "--push",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(stdout.lines().nth(1), Some("local-save\t"));
    Ok(())
}

#[test]
fn a_push_ignores_its_marker_directory_config_independently(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    configure_integration(repo.path(), "jira")?;
    let body_file = repo.path().join("body.txt");
    fs::write(&body_file, BODY)?;
    let output = run(
        repo.path(),
        &[
            "create",
            "Push me",
            "task",
            "medium",
            "--push",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let ignore = marker_path(repo.path(), "jira", "unused")
        .parent()
        .expect("marker path has a parent")
        .join(".gitignore");
    let contents = fs::read_to_string(&ignore)?;
    assert!(contents.contains('*'), "{contents}");
    Ok(())
}

#[test]
fn a_matching_created_marker_with_the_id_absent_from_the_corpus_is_reused(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let title = "Push me";
    let slug = "push-me";
    let digest = pending_push::request_digest(title, BODY, "task");
    let marker = PendingPush::Created {
        request: RequestFingerprint {
            title: title.to_owned(),
            digest,
            attempted_at: 1_700_000_000,
            failure: None,
        },
        external_id: tracker::ExternalId::new("ENG-1".to_owned()),
    };
    write_marker(&marker_path(repo.path(), "", slug), &marker)?;

    let body_file = repo.path().join("body.txt");
    fs::write(&body_file, BODY)?;
    let output = run(
        repo.path(),
        &[
            "create",
            title,
            "task",
            "medium",
            "--push",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let mut lines = stdout.lines();
    let path = lines.next().expect("path line");
    assert_eq!(lines.next(), Some("write-once\tENG-1"));
    let content = fs::read_to_string(path)?;
    assert!(content.contains("external_id: ENG-1"), "{content}");
    assert!(
        !marker_path(repo.path(), "", slug).exists(),
        "the marker should be deleted after the reused write"
    );
    Ok(())
}

#[test]
fn a_fingerprint_mismatch_refuses_and_writes_nothing() -> Result<(), TestError>
{
    let repo = scratch_repo()?;
    let title = "Push me";
    let slug = "push-me";
    let marker = PendingPush::Created {
        request: RequestFingerprint {
            title: title.to_owned(),
            digest: "not-the-real-digest".to_owned(),
            attempted_at: 1_700_000_000,
            failure: None,
        },
        external_id: tracker::ExternalId::new("ENG-1".to_owned()),
    };
    let path = marker_path(repo.path(), "", slug);
    write_marker(&path, &marker)?;

    let body_file = repo.path().join("body.txt");
    fs::write(&body_file, BODY)?;
    let output = run(
        repo.path(),
        &[
            "create",
            title,
            "task",
            "medium",
            "--push",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_PUSH_FINGERPRINT_MISMATCH"), "{stderr}");
    assert!(work_item_files(repo.path())?.is_empty());
    assert!(path.exists(), "the marker must not be adopted or removed");
    Ok(())
}

#[test]
fn an_already_written_external_id_refuses_and_writes_no_second_file(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let title = "Push me";
    let slug = "push-me";
    let digest = pending_push::request_digest(title, BODY, "task");
    let marker = PendingPush::Created {
        request: RequestFingerprint {
            title: title.to_owned(),
            digest,
            attempted_at: 1_700_000_000,
            failure: None,
        },
        external_id: tracker::ExternalId::new("ENG-9".to_owned()),
    };
    write_marker(&marker_path(repo.path(), "", slug), &marker)?;
    fs::write(
        repo.path().join("meta/work/0001-existing.md"),
        "---\nexternal_id: \"ENG-9\"\n---\nBody\n",
    )?;

    let body_file = repo.path().join("body.txt");
    fs::write(&body_file, BODY)?;
    let output = run(
        repo.path(),
        &[
            "create",
            title,
            "task",
            "medium",
            "--push",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_PUSH_ALREADY_WRITTEN"), "{stderr}");
    assert!(stderr.contains("ENG-9"), "{stderr}");
    let files = work_item_files(repo.path())?;
    assert_eq!(files.len(), 1, "no second file should have been written");
    Ok(())
}

#[test]
fn a_prior_attempt_of_unknown_outcome_refuses_with_no_second_create(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let title = "Push me";
    let slug = "push-me";
    let marker = PendingPush::Attempted {
        request: RequestFingerprint {
            title: title.to_owned(),
            digest: pending_push::request_digest(title, BODY, "task"),
            attempted_at: 1_700_000_000,
            failure: Some("timed out".to_owned()),
        },
    };
    write_marker(&marker_path(repo.path(), "", slug), &marker)?;

    let body_file = repo.path().join("body.txt");
    fs::write(&body_file, BODY)?;
    let output = run(
        repo.path(),
        &[
            "create",
            title,
            "task",
            "medium",
            "--push",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_PUSH_PENDING"), "{stderr}");
    assert!(stderr.contains("timed out"), "{stderr}");
    assert!(work_item_files(repo.path())?.is_empty());
    Ok(())
}

#[test]
fn a_deliberately_truncated_marker_fails_closed() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let title = "Push me";
    let slug = "push-me";
    let path = marker_path(repo.path(), "", slug);
    fs::create_dir_all(path.parent().expect("has a parent"))?;
    fs::write(&path, "{\"kind\": \"created\", \"title\":")?;

    let body_file = repo.path().join("body.txt");
    fs::write(&body_file, BODY)?;
    let output = run(
        repo.path(),
        &[
            "create",
            title,
            "task",
            "medium",
            "--push",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_PUSH_MARKER_UNREADABLE"), "{stderr}");
    assert!(work_item_files(repo.path())?.is_empty());
    Ok(())
}

#[test]
fn without_push_stdout_is_unchanged_single_line() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output =
        run(repo.path(), &["create", "No push here", "task", "medium"])?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(stdout.lines().count(), 1);
    Ok(())
}
