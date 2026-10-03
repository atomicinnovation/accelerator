//! An insecure `config.local.md`: the migrating run refuses and writes
//! nothing, while the read-only surfaces the session hooks call still run.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

fn pending_repo_with_insecure_personal_config() -> Result<TempDir, TestError> {
    let dir = TempDir::new()?;
    fs::create_dir_all(dir.path().join("meta"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    let personal = dir.path().join(".accelerator/config.local.md");
    fs::write(&personal, "---\npaths:\n  work: mine\n---\n")?;
    fs::set_permissions(&personal, fs::Permissions::from_mode(0o644))?;
    Ok(dir)
}

#[test]
fn the_migrating_run_refuses_and_writes_nothing() -> Result<(), TestError> {
    let dir = pending_repo_with_insecure_personal_config()?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("E_LOCAL_PERMS_INSECURE"), "{stderr}");
    assert!(!dir.path().join(".accelerator/state").exists());
    Ok(())
}

#[test]
fn the_read_only_surfaces_still_run() -> Result<(), TestError> {
    for args in [
        &["--discoverability-hook", "--format=hook", "--fail-safe"][..],
        &["--list"][..],
    ] {
        let dir = pending_repo_with_insecure_personal_config()?;

        let output = Command::new(BIN)
            .args(args)
            .current_dir(dir.path())
            .output()?;

        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
