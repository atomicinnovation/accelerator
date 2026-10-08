//! End-to-end tests of the compiled `accelerator-collaboration` binary
//! against a local mock GitHub API server: the only tests exercising
//! `main.rs`'s error-rendering match arms and the `BlockingGitHubClient`
//! shim itself, plus the three CLI-layer characterization branches
//! deferred from `collaboration`'s own unit tests.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use http_test_support::{MockHTTPServer, RequestKey, Route};
use vcs_test_support::hermetic::Hermetic;

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

fn colocated_jj_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("collaboration-cli-e2e-jj-")
        .tempdir()?;
    let repo = dir.path().join("repo");
    fs::create_dir_all(&repo)?;
    let jj = Hermetic::rooted_at(dir.path())?;
    jj.jj(&["git", "init", "--colocate"], &repo)?;
    jj.jj(
        &[
            "git",
            "remote",
            "add",
            "origin",
            "https://github.com/candidate-owner/candidate-repo.git",
        ],
        &repo,
    )?;
    Ok(dir)
}

fn run(
    dir: &Path,
    server: &MockHTTPServer,
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
    let server = MockHTTPServer::start();
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
fn base_repo_resolves_the_origin_of_a_colocated_jj_repository(
) -> Result<(), TestError> {
    let dir = colocated_jj_repo()?;
    let server = MockHTTPServer::start();
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

    let output = run(
        &dir.path().join("repo"),
        &server,
        &["pr", "base-repo", "42"],
    )?;
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
    let server = MockHTTPServer::start();
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
    let server = MockHTTPServer::start();
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
    let server = MockHTTPServer::start();
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
    let server = MockHTTPServer::start();
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
    let server = MockHTTPServer::start();

    let output = run(repo.path(), &server, &["pr", "base-repo"])?;
    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(0));
    Ok(())
}

#[test]
fn update_body_with_no_body_file_flag_is_a_clap_usage_error(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let server = MockHTTPServer::start();

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
    let server = MockHTTPServer::start();

    let output = run(dir.path(), &server, &["pr", "base-repo", "42"])?;
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("origin"), "{stderr}");
    Ok(())
}

#[cfg(unix)]
fn with_insecure_personal_config(dir: &Path) -> Result<(), TestError> {
    use std::os::unix::fs::PermissionsExt as _;

    fs::create_dir_all(dir.join(".accelerator"))?;
    fs::write(
        dir.join(".accelerator/config.md"),
        "---\ngithub:\n  token: team-token\n---\n",
    )?;
    let personal = dir.join(".accelerator/config.local.md");
    fs::write(&personal, "---\ngithub:\n  token: mine\n---\n")?;
    fs::set_permissions(&personal, fs::Permissions::from_mode(0o644))?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_team_token_is_not_used_beside_an_insecure_personal_config(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    with_insecure_personal_config(repo.path())?;
    let server = MockHTTPServer::start();
    let key = RequestKey::new("GET", "/repos/candidate-owner/candidate-repo");
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: repository_json("candidate-owner", "candidate-repo"),
        },
    );

    let output = Command::new(env!("CARGO_BIN_EXE_accelerator-collaboration"))
        .args(["pr", "base-repo", "42"])
        .current_dir(repo.path())
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .env(
            "ACCELERATOR_COLLABORATION_GITHUB_API_URL",
            server.base_url(),
        )
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("E_LOCAL_PERMS_INSECURE"), "{stderr}");
    assert_eq!(server.hits(&key), 0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn gh_token_works_beside_an_insecure_personal_config() -> Result<(), TestError>
{
    let repo = scratch_repo()?;
    with_insecure_personal_config(repo.path())?;
    let server = MockHTTPServer::start();
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

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert_eq!(
        stderr.matches("warning: E_LOCAL_PERMS_INSECURE").count(),
        1,
        "{stderr}"
    );
    Ok(())
}

fn resolving_server() -> MockHTTPServer {
    let server = MockHTTPServer::start();
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
    server
}

fn base_repo(
    dir: &Path,
    server: &MockHTTPServer,
    gh_token: Option<&str>,
) -> Result<Output, TestError> {
    let mut command =
        Command::new(env!("CARGO_BIN_EXE_accelerator-collaboration"));
    command
        .args(["pr", "base-repo", "42"])
        .current_dir(dir)
        .env_remove("GITHUB_TOKEN")
        .env(
            "ACCELERATOR_COLLABORATION_GITHUB_API_URL",
            server.base_url(),
        );
    match gh_token {
        Some(token) => command.env("GH_TOKEN", token),
        None => command.env_remove("GH_TOKEN"),
    };
    Ok(command.output()?)
}

fn write_configs(
    dir: &Path,
    team: &str,
    personal: Option<&str>,
) -> Result<(), TestError> {
    fs::create_dir_all(dir.join(".accelerator"))?;
    fs::write(
        dir.join(".accelerator/config.md"),
        format!("---\ngithub:\n{team}---\n"),
    )?;
    if let Some(personal) = personal {
        let path = dir.join(".accelerator/config.local.md");
        fs::write(&path, format!("---\ngithub:\n{personal}---\n"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}

fn track_personal_config(dir: &Path) -> Result<(), TestError> {
    let status = Command::new("git")
        .args(["add", "--force", ".accelerator/config.local.md"])
        .current_dir(dir)
        .status()?;
    assert!(status.success(), "git add failed");
    Ok(())
}

#[test]
fn a_team_token_command_beside_gh_token_warns_and_runs() -> Result<(), TestError>
{
    let repo = scratch_repo()?;
    write_configs(repo.path(), "  token_cmd: printf team\n", None)?;
    let server = resolving_server();

    let output = base_repo(repo.path(), &server, Some("test-token"))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains("warning: E_CONSENT_KEY_TEAM_LEVEL: github.token_cmd"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_team_token_command_alone_is_refused() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    write_configs(repo.path(), "  token_cmd: printf team\n", None)?;
    let server = resolving_server();

    let output = base_repo(repo.path(), &server, None)?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("E_CONSENT_KEY_TEAM_LEVEL: github.token_cmd"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_tracked_personal_token_is_refused() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    write_configs(
        repo.path(),
        "  host: github.com\n",
        Some("  token: mine\n"),
    )?;
    track_personal_config(repo.path())?;
    let server = resolving_server();

    let output = base_repo(repo.path(), &server, None)?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("E_TOKEN_FROM_TRACKED_FILE: github.token"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_tracked_personal_command_beside_a_team_command_is_fatal_and_warns(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    write_configs(
        repo.path(),
        "  token_cmd: printf team\n",
        Some("  token_cmd: printf mine\n"),
    )?;
    track_personal_config(repo.path())?;
    let server = resolving_server();

    let output = base_repo(repo.path(), &server, None)?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("warning: E_CONSENT_KEY_TEAM_LEVEL: github.token_cmd"),
        "{stderr}"
    );
    assert!(
        stderr.contains("E_CONSENT_KEY_TRACKED: github.token_cmd"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_personal_token_command_runs_outside_the_project_in_a_fresh_directory(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let record = tempfile::tempdir()?;
    let recorded = record.path().join("cwd");
    write_configs(
        repo.path(),
        "  host: github.com\n",
        Some(&format!(
            "  token_cmd: pwd > {} && printf from-helper\n",
            recorded.display()
        )),
    )?;
    let server = resolving_server();

    let output = base_repo(repo.path(), &server, None)?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    let cwd = std::path::PathBuf::from(fs::read_to_string(&recorded)?.trim());
    assert!(!cwd.starts_with(repo.path().canonicalize()?));
    assert!(!cwd.exists(), "{} outlived the run", cwd.display());
    Ok(())
}
