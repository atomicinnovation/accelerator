//! Black-box tests that built-in commands stay decoupled from the release
//! manifest: `version` runs without it, and top-level `--help` degrades to the
//! built-in help when the manifest cannot be fetched.
//!
//! The manifest-derived external-subcommands section itself is unit-tested in
//! `launch::help` (`external_subcommands_section`): the binary pins the embedded
//! release key, and a test cannot sign a manifest under it, so the
//! rendered-section assertion belongs at the unit level, not here.

use std::error::Error;
use std::process::Command;

const LAUNCHER: &str = env!("CARGO_BIN_EXE_accelerator");

// An https endpoint that refuses connections, so a manifest fetch fails fast
// (the production fetcher pins https, so the URL must use that scheme).
const DEAD_RELEASE_URL: &str = "https://127.0.0.1:1";

#[test]
fn version_succeeds_with_no_manifest_available() -> Result<(), Box<dyn Error>> {
    // `version` is a built-in and must never touch the manifest or network;
    // pointing the release URL at a dead endpoint proves it is not consulted.
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
    // The manifest read that augments `--help` is best-effort: when it cannot be
    // fetched, `--help` must still print the built-in help and exit 0.
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
