//! `ACCELERATOR_LOG` reaches the work binary, so a tracker client's request
//! traces can be turned on for a sync.

use std::process::Command;

type TestError = Box<dyn std::error::Error>;

#[test]
fn a_malformed_accelerator_log_is_reported_and_the_command_still_runs(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join(".accelerator"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(["id", "canonicalise", "42"])
        .current_dir(dir.path())
        .env("ACCELERATOR_LOG", "bad=notalevel")
        .output()?;

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid log filter"), "{stderr}");
    Ok(())
}
