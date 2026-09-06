//! Black-box tests that built-ins stay decoupled from the release manifest.
//! (The rendered section itself is unit-tested in `launch::help`, since a test
//! cannot sign a manifest under the embedded key.)

use std::error::Error;
use std::process::Command;

const LAUNCHER: &str = env!("CARGO_BIN_EXE_accelerator");

// https (the production fetcher pins it) but refusing connections.
const DEAD_RELEASE_URL: &str = "https://127.0.0.1:1";

#[test]
fn version_succeeds_with_no_manifest_available() -> Result<(), Box<dyn Error>> {
    let output = Command::new(LAUNCHER)
        .arg("version")
        .env("ACCELERATOR_RELEASE_BASE_URL", DEAD_RELEASE_URL)
        .env_remove("ACCELERATOR_LOG")
        .output()?;
    assert!(output.status.success(), "version did not succeed");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("accelerator "),
        "expected version output, got: {stdout}"
    );
    Ok(())
}

#[test]
fn top_level_help_prints_builtins_when_manifest_unavailable(
) -> Result<(), Box<dyn Error>> {
    let output = Command::new(LAUNCHER)
        .arg("--help")
        .env("ACCELERATOR_RELEASE_BASE_URL", DEAD_RELEASE_URL)
        .env_remove("ACCELERATOR_LOG")
        .output()?;
    assert!(output.status.success(), "--help did not exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("version"),
        "expected built-in help listing `version`, got: {stdout}"
    );
    Ok(())
}

#[test]
fn help_subcommand_prints_builtins_when_manifest_unavailable(
) -> Result<(), Box<dyn Error>> {
    let output = Command::new(LAUNCHER)
        .arg("help")
        .env("ACCELERATOR_RELEASE_BASE_URL", DEAD_RELEASE_URL)
        .env_remove("ACCELERATOR_LOG")
        .output()?;
    assert!(output.status.success(), "help did not exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("version"),
        "expected built-in listing `version`, got: {stdout}"
    );
    Ok(())
}

#[test]
fn bare_invocation_lists_builtins_on_stdout_and_cues_on_stderr(
) -> Result<(), Box<dyn Error>> {
    let output = Command::new(LAUNCHER)
        .env("ACCELERATOR_RELEASE_BASE_URL", DEAD_RELEASE_URL)
        .env_remove("ACCELERATOR_LOG")
        .output()?;
    assert_eq!(output.status.code(), Some(1), "bare invocation must exit 1");
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stdout.contains("version"),
        "expected the listing on stdout, got: {stdout}"
    );
    assert!(
        stderr.contains("subcommand"),
        "expected the required-subcommand cue on stderr, got: {stderr}"
    );
    Ok(())
}

#[test]
fn config_help_renders_config_help_not_the_top_level_listing(
) -> Result<(), Box<dyn Error>> {
    let output = Command::new(LAUNCHER)
        .args(["config", "--help"])
        .env("ACCELERATOR_RELEASE_BASE_URL", DEAD_RELEASE_URL)
        .env_remove("ACCELERATOR_LOG")
        .output()?;
    assert!(output.status.success(), "config --help did not exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("Usage: accelerator config"),
        "expected config's own help, got: {stdout}"
    );
    assert!(
        !stdout.contains("cache"),
        "config --help leaked a sibling built-in from the top-level listing: \
         {stdout}"
    );
    Ok(())
}
