//! Consent through the binary's tracker resolution: a command key's
//! refusals reach stderr on both tracker paths, and a refused command leaves
//! the tracker unconfigured. No case lets a client reach the network.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

mod common;

type TestError = Box<dyn std::error::Error>;

fn repo(
    team: &str,
    personal: Option<&str>,
) -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-consent-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(dir.path().join(".accelerator/config.md"), team)?;
    if let Some(personal) = personal {
        let path = dir.path().join(".accelerator/config.local.md");
        fs::write(&path, personal)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(dir)
}

fn sync(dir: &Path) -> Result<(Option<i32>, String), TestError> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .args(["sync", "--preview"])
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null());
    common::scrub_provider_env(&mut command);
    let output = command.output()?;
    Ok((output.status.code(), String::from_utf8(output.stderr)?))
}

#[test]
fn a_team_token_command_alone_leaves_either_tracker_unconfigured(
) -> Result<(), TestError> {
    for (team, key) in [
        (
            "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
             email: a@b.c\n  project_key: ENG\n  token_cmd: printf t\n---\n",
            "jira.token_cmd",
        ),
        (
            "---\nwork:\n  integration: linear\nlinear:\n  team_id: t\n  \
             token_cmd: printf t\n---\n",
            "linear.token_cmd",
        ),
    ] {
        let repo = repo(team, None)?;

        let (code, stderr) = sync(repo.path())?;

        assert_eq!(code, Some(74), "{key}: {stderr}");
        assert!(
            stderr.contains(&format!("E_CONSENT_KEY_TEAM_LEVEL: {key}")),
            "{key}: {stderr}"
        );
    }
    Ok(())
}

#[test]
fn a_team_token_command_beside_a_personal_token_is_reported_as_a_warning(
) -> Result<(), TestError> {
    let repo = repo(
        "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
         email: a@b.c\n  token_cmd: printf t\n---\n",
        Some("---\njira:\n  token: mine\n---\n"),
    )?;

    let (code, stderr) = sync(repo.path())?;

    assert_eq!(code, Some(74), "{stderr}");
    assert!(
        stderr.contains("warning: E_CONSENT_KEY_TEAM_LEVEL: jira.token_cmd"),
        "{stderr}"
    );
    assert!(stderr.contains("E_NO_PROJECT"), "{stderr}");
    Ok(())
}

#[test]
fn a_personal_token_command_in_a_tracked_file_leaves_the_tracker_unconfigured(
) -> Result<(), TestError> {
    let repo = repo(
        "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
         email: fixture@example.com\n---\n",
        Some("---\njira:\n  token_cmd: exit 3\n---\n"),
    )?;
    let status = Command::new("git")
        .args(["add", "--force", ".accelerator/config.local.md"])
        .current_dir(repo.path())
        .status()?;
    assert!(status.success(), "git add failed");

    let (code, stderr) = sync(repo.path())?;

    assert_eq!(code, Some(74), "{stderr}");
    assert_eq!(
        stderr,
        format!(
            "work.integration names 'jira', which is wired but not usable — \
             its configuration or credentials are missing or refused: \
             E_CONSENT_KEY_TRACKED: jira.token_cmd in {} is refused — the \
             file is tracked by version control, so the repository chose \
             the value; untrack it\n",
            repo.path()
                .canonicalize()?
                .join(".accelerator/config.local.md")
                .display()
        )
    );
    Ok(())
}
