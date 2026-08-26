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
    // Coverage instrumentation writes to the path this names; clearing it
    // makes the child litter its profile into the directory under test.
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
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

    // Coverage artefacts are the harness's, not the binary's — the claim is
    // that no *state* was resolved.
    let stray: Vec<_> = fs::read_dir(work.path())?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.ends_with(".profraw"))
        .collect();
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

/// With the runtime not yet materialised and no launcher reachable, the
/// executor downgrades to `artifact-unavailable` rather than failing hard, so
/// the caller falls back to the code-only crawler.
#[test]
fn an_unresolvable_launcher_downgrades_to_artifact_unavailable(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let repo = work.path().join("repo");
    fs::create_dir_all(repo.join(".git"))?;
    // A plugin root with no launcher under `bin`, and a PATH carrying none
    // either, so discovery exhausts every source.
    let empty_bin = work.path().join("bin");
    fs::create_dir_all(&empty_bin)?;

    let output = executor(
        &["ping"],
        &repo,
        &[
            (
                "ACCELERATOR_PLUGIN_ROOT",
                &work.path().display().to_string(),
            ),
            ("PATH", &empty_bin.display().to_string()),
        ],
    );

    assert_eq!(code_of(&output), 3);
    let message = stderr_of(&output);
    assert!(message.contains(r#""error":"downgrade""#), "{message}");
    assert!(
        message.contains(r#""reason":"artifact-unavailable""#),
        "{message}"
    );
    Ok(())
}

/// An unset plugin root is no longer a pre-flight hard error: the runtime check
/// runs first, and with no launcher reachable it downgrades rather than naming
/// the unset variable — a diagnostic only a resolved runtime reaches.
#[test]
fn an_unset_plugin_root_downgrades_rather_than_hard_failing(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let repo = work.path().join("repo");
    fs::create_dir_all(repo.join(".git"))?;
    let empty_bin = work.path().join("bin");
    fs::create_dir_all(&empty_bin)?;

    let output = executor(
        &["ping"],
        &repo,
        &[("PATH", &empty_bin.display().to_string())],
    );

    assert_eq!(code_of(&output), 3);
    let message = stderr_of(&output);
    assert!(
        message.contains(r#""reason":"artifact-unavailable""#),
        "{message}"
    );
    assert!(
        !message.contains("ACCELERATOR_PLUGIN_ROOT"),
        "an unset plugin root must not surface as a hard error here: {message}"
    );
    Ok(())
}
