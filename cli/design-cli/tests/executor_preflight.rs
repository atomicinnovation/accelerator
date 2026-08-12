//! `accelerator design executor`'s pre-flight, through the compiled binary.
//!
//! Everything asserted here happens *before* Node is reached, so none of it
//! needs a Playwright runtime, a browser or a network. The properties that only
//! exist once the client is exec'd — the pass-through of its exit status,
//! streams and signal death — belong to the opt-in lane that provisions a real
//! runtime, and are not faked here.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-design");

/// Runs the executor with a controlled environment, so a developer's own
/// `ACCELERATOR_*` settings cannot change the outcome.
fn executor(
    arguments: &[&str],
    cwd: &Path,
    environment: &[(&str, &str)],
) -> Output {
    let mut command = Command::new(BIN);
    command.arg("executor");
    command.args(arguments);
    command.current_dir(cwd);
    command.env_clear();
    command.env("PATH", std::env::var("PATH").unwrap_or_default());
    command.env("HOME", cwd);
    for (name, value) in environment {
        command.env(name, value);
    }
    command
        .output()
        .unwrap_or_else(|error| unreachable!("the binary should run: {error}"))
}

fn code_of(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The one that would start a second foreground daemon over the live one's
/// state files. Rejected by argument validation, before anything is resolved.
#[test]
fn the_internal_daemon_command_is_refused() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let output = executor(&["daemon"], work.path(), &[]);

    assert_eq!(code_of(&output), 2);
    assert!(stderr_of(&output).contains("internal executor command"));
    Ok(())
}

#[test]
fn an_unknown_command_is_refused_and_lists_what_is_valid(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let output = executor(&["rm-rf"], work.path(), &[]);

    assert_eq!(code_of(&output), 2);
    let message = stderr_of(&output);
    for valid in ["ping", "navigate", "links", "daemon-stop"] {
        assert!(message.contains(valid), "{message}");
    }
    Ok(())
}

/// Validation happens before resolution, so a refused command leaves no state
/// directory behind — it cannot create one in a repository it never looked for.
#[test]
fn a_refused_command_resolves_nothing() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    executor(&["daemon"], work.path(), &[]);

    let stray: Vec<_> = fs::read_dir(work.path())?.collect();
    assert!(stray.is_empty(), "a refused command created {stray:?}");
    Ok(())
}

/// The `no-repo` envelope: three keys on stderr, exit 2.
#[test]
fn invoking_outside_a_repository_emits_the_no_repo_envelope(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let output = executor(&["ping"], work.path(), &[]);

    assert_eq!(code_of(&output), 2);
    let message = stderr_of(&output);
    assert!(message.contains(r#""error":"no-repo""#), "{message}");
    assert!(message.contains(r#""category":"usage""#), "{message}");
    assert!(
        !message.contains("protocol") && !message.contains("retryable"),
        "launcher envelopes stay three-key: {message}"
    );
    Ok(())
}

#[test]
fn a_missing_command_is_a_usage_error() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let mut command = Command::new(BIN);
    command.arg("executor");
    command.current_dir(work.path());
    command.env_clear();
    let output = command.output()?;

    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

/// A bootstrapped-looking repository with an empty namespace: exit 3, naming
/// the directory the surviving bootstrap script would populate.
#[test]
fn an_unpopulated_namespace_reports_playwright_not_installed(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let repo = work.path().join("repo");
    fs::create_dir_all(repo.join(".git"))?;
    let cache = work.path().join("cache");
    fs::create_dir_all(&cache)?;

    let output = executor(
        &["ping"],
        &repo,
        &[
            ("ACCELERATOR_PLAYWRIGHT_CACHE", &cache.display().to_string()),
            (
                "ACCELERATOR_PLUGIN_ROOT",
                &plugin_root().display().to_string(),
            ),
        ],
    );

    assert_eq!(code_of(&output), 3);
    let message = stderr_of(&output);
    assert!(
        message.contains(r#""error":"playwright-not-installed""#),
        "{message}"
    );
    assert!(message.contains(r#""category":"bootstrap""#), "{message}");
    assert!(
        message.contains(&cache.display().to_string()),
        "the envelope names the namespace it looked in: {message}"
    );
    Ok(())
}

/// The launcher cannot derive its own location, so an unset plugin root must
/// say so rather than failing somewhere further in.
#[test]
fn an_unset_plugin_root_is_reported_rather_than_guessed(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let repo = work.path().join("repo");
    fs::create_dir_all(repo.join(".git"))?;

    let output = executor(&["ping"], &repo, &[]);

    assert_eq!(code_of(&output), 1);
    assert!(
        stderr_of(&output).contains("ACCELERATOR_PLUGIN_ROOT"),
        "{}",
        stderr_of(&output)
    );
    Ok(())
}

fn plugin_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default()
}
