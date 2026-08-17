//! End-to-end tests of the compiled `accelerator-collaboration` binary
//! against a local mock GitHub API server: the only tests exercising
//! `main.rs`'s error-rendering match arms and the `BlockingGitHubClient`
//! shim itself, plus the three CLI-layer characterization branches
//! deferred from `collaboration`'s own unit tests.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use http_test_support::{MockServer, RequestKey, Route};

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("collaboration-cli-e2e-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    let remote_status = Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/candidate-owner/candidate-repo.git",
        ])
        .current_dir(dir.path())
        .status()?;
    assert!(remote_status.success(), "git remote add failed");
    Ok(dir)
}

fn run(
    dir: &Path,
    server: &MockServer,
    args: &[&str],
) -> Result<Output, TestError> {
    Ok(
        Command::new(env!("CARGO_BIN_EXE_accelerator-collaboration"))
            .args(args)
            .current_dir(dir)
            .env("GH_TOKEN", "test-token")
            .env("GITHUB_TOKEN", "")
            .env(
                "ACCELERATOR_COLLABORATION_GITHUB_API_URL",
                server.base_url(),
            )
            .output()?,
    )
}

fn repository_json(owner: &str, name: &str) -> String {
    format!(
        "{{\"id\":1,\"name\":\"{name}\",\
         \"url\":\"https://api.github.com/repos/{owner}/{name}\"}}"
    )
}

fn pull_request_json(number: u64) -> String {
    format!(
        "{{\"id\":1,\"number\":{number},\
         \"url\":\"https://api.github.com/repos/owner/repo/pulls/{number}\",\
         \"head\":{{\"ref\":\"feature\",\"sha\":\"abc123\"}},\
         \"base\":{{\"ref\":\"main\",\"sha\":\"def456\"}},\
         \"locked\":false}}"
    )
}

#[test]
fn base_repo_prints_owner_slash_repo_on_success() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/candidate-owner/candidate-repo"),
        Route::Json {
            status: 200,
            body: repository_json("candidate-owner", "candidate-repo"),
        },
    );
    server.route(
        RequestKey::new(
            "GET",
            "/repos/candidate-owner/candidate-repo/pulls/42",
        ),
        Route::Json {
            status: 200,
            body: pull_request_json(42),
        },
    );

    let output = run(repo.path(), &server, &["pr", "base-repo", "42"])?;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "candidate-owner/candidate-repo\n"
    );
    Ok(())
}

#[test]
fn base_repo_reports_a_repository_lookup_failure() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/candidate-owner/candidate-repo"),
        Route::Json {
            status: 404,
            body: "{\"message\":\"Not Found\"}".to_owned(),
        },
    );

    let output = run(repo.path(), &server, &["pr", "base-repo", "42"])?;
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("404"), "{stderr}");
    assert!(stderr.contains("Not Found"), "{stderr}");
    Ok(())
}

#[test]
fn update_body_succeeds() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/candidate-owner/candidate-repo"),
        Route::Json {
            status: 200,
            body: repository_json("candidate-owner", "candidate-repo"),
        },
    );
    server.route(
        RequestKey::new(
            "GET",
            "/repos/candidate-owner/candidate-repo/pulls/42",
        ),
        Route::Json {
            status: 200,
            body: pull_request_json(42),
        },
    );
    server.route(
        RequestKey::new(
            "PATCH",
            "/repos/candidate-owner/candidate-repo/pulls/42",
        ),
        Route::Json {
            status: 200,
            body: pull_request_json(42),
        },
    );
    let body_file = repo.path().join("body.md");
    fs::write(&body_file, "new body")?;

    let output = run(
        repo.path(),
        &server,
        &[
            "pr",
            "update-body",
            "42",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn update_body_reports_a_patch_failure() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let server = MockServer::start();
    server.route(
        RequestKey::new("GET", "/repos/candidate-owner/candidate-repo"),
        Route::Json {
            status: 200,
            body: repository_json("candidate-owner", "candidate-repo"),
        },
    );
    server.route(
        RequestKey::new(
            "GET",
            "/repos/candidate-owner/candidate-repo/pulls/42",
        ),
        Route::Json {
            status: 200,
            body: pull_request_json(42),
        },
    );
    server.route(
        RequestKey::new(
            "PATCH",
            "/repos/candidate-owner/candidate-repo/pulls/42",
        ),
        Route::Json {
            status: 422,
            body: "{\"message\":\"Validation Failed\"}".to_owned(),
        },
    );
    let body_file = repo.path().join("body.md");
    fs::write(&body_file, "new body")?;

    let output = run(
        repo.path(),
        &server,
        &[
            "pr",
            "update-body",
            "42",
            "--body-file",
            body_file.to_str().expect("utf8 path"),
        ],
    )?;
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("422"), "{stderr}");
    assert!(stderr.contains("Validation Failed"), "{stderr}");
    Ok(())
}

#[test]
fn update_body_with_a_missing_body_file_exits_two() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let server = MockServer::start();
    let missing = repo.path().join("does-not-exist.md");

    let output = run(
        repo.path(),
        &server,
        &[
            "pr",
            "update-body",
            "42",
            "--body-file",
            missing.to_str().expect("utf8 path"),
        ],
    )?;
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("body-file"), "{stderr}");
    Ok(())
}

#[test]
fn base_repo_with_no_pull_number_is_a_clap_usage_error() -> Result<(), TestError>
{
    let repo = scratch_repo()?;
    let server = MockServer::start();

    let output = run(repo.path(), &server, &["pr", "base-repo"])?;
    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(0));
    Ok(())
}

#[test]
fn update_body_with_no_body_file_flag_is_a_clap_usage_error(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let server = MockServer::start();

    let output = run(repo.path(), &server, &["pr", "update-body", "42"])?;
    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(0));
    Ok(())
}

#[test]
fn no_origin_remote_configured_exits_two() -> Result<(), TestError> {
    let dir = tempfile::Builder::new()
        .prefix("collaboration-cli-e2e-no-origin-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success());
    let server = MockServer::start();

    let output = run(dir.path(), &server, &["pr", "base-repo", "42"])?;
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("origin"), "{stderr}");
    Ok(())
}
