//! The in-process user-name read agrees with an explicit git config write and
//! with jj's own `[user]` config, and `JJ_USER` overrides both.
//!
//! Runs the fixture binary as a child rather than probing in-process, for the
//! same reason `scrub.rs` does: the hermetic environment is applied through
//! process env vars, and libtest runs each test on its own thread, so
//! in-process `set_var` would be racy.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::process::Command;

use vcs_test_support::fixtures::pure_jj;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const FIXTURE: &str = env!("CARGO_BIN_EXE_vcs-adapters-fixture");

fn kind_and_user_name(
    env: &Hermetic,
    dir: &Path,
    extra_env: &[(&str, &str)],
) -> Result<String, TestError> {
    let mut command = Command::new(FIXTURE);
    command.arg("only").arg("kind_and_user_name").arg(dir);
    env.apply(&mut command);
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "fixture binary failed for {}: {}",
            dir.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn reads_an_explicitly_configured_git_user_name() -> Result<(), TestError> {
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let repo = base.path().join("repo");
    fs::create_dir_all(&repo)?;
    env.git(&["init", "--quiet"], &repo)?;
    env.git(
        &["config", "--local", "user.name", "Explicit Author"],
        &repo,
    )?;

    let rendered = kind_and_user_name(&env, &repo, &[])?;
    assert!(
        rendered.contains("git Explicit Author"),
        "expected the local git config's user.name, got {rendered:?}"
    );
    Ok(())
}

#[test]
fn an_unconfigured_git_repository_has_no_user_name() -> Result<(), TestError> {
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let repo = base.path().join("repo");
    fs::create_dir_all(&repo)?;
    env.git(&["init", "--quiet"], &repo)?;

    let rendered = kind_and_user_name(&env, &repo, &[])?;
    assert!(
        rendered.contains("git absent"),
        "the hermetic environment carries no global git config: {rendered:?}"
    );
    Ok(())
}

#[test]
fn reads_the_jj_config_s_user_name() -> Result<(), TestError> {
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let repo = pure_jj(&base.path().join("repo"), &env)?;

    let rendered = kind_and_user_name(&env, &repo, &[])?;
    assert!(
        rendered.contains("jj Fixture"),
        "expected Hermetic's own [user] name, got {rendered:?}"
    );
    Ok(())
}

#[test]
fn jj_user_env_var_overrides_the_config_file() -> Result<(), TestError> {
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let repo = pure_jj(&base.path().join("repo"), &env)?;

    let rendered =
        kind_and_user_name(&env, &repo, &[("JJ_USER", "Overridden")])?;
    assert!(
        rendered.contains("jj Overridden"),
        "JJ_USER must win over the config file's user.name: {rendered:?}"
    );
    Ok(())
}
