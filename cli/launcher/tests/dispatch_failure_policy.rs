//! How a hook's dispatch fails when the sub-binary cannot be produced:
//! `--non-blocking` must turn an integrity refusal into a non-blocking exit,
//! since a `PreToolUse` hook reads exit `2` as a block.

use std::error::Error;
use std::path::Path;
use std::process::Command;
use std::process::Output;

const LAUNCHER: &str = env!("CARGO_BIN_EXE_accelerator");
const VERSION: &str = env!("CARGO_PKG_VERSION");

// https (the production fetcher pins it) but refusing connections.
const DEAD_RELEASE_URL: &str = "https://127.0.0.1:1";

const JUNK_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

fn research_guard(
    cache: &Path,
    flags: &[&str],
) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(LAUNCHER)
        .args(["research", "guard"])
        .args(flags)
        .env("ACCELERATOR_RELEASE_BASE_URL", DEAD_RELEASE_URL)
        .env("ACCELERATOR_CACHE_DIR", cache)
        .env_remove("ACCELERATOR_RESEARCH_BIN")
        .env_remove("ACCELERATOR_LOG")
        .output()?)
}

/// A cached research binary that fails re-verification, so the launcher
/// refetches from the dead URL and refuses.
fn corrupt_cache() -> Result<(tempfile::TempDir, String), Box<dyn Error>> {
    let cache = tempfile::Builder::new().prefix("acc-policy-").tempdir()?;
    let cached = cache
        .path()
        .join(format!("research-{VERSION}-{JUNK_SHA256}"));
    std::fs::write(&cached, b"not the signed binary")?;
    std::fs::write(
        cache
            .path()
            .join(format!("research-{VERSION}-{JUNK_SHA256}.minisig")),
        b"junk",
    )?;
    Ok((cache, cached.display().to_string()))
}

#[test]
fn a_non_blocking_refusal_exits_one_with_its_recovery_step(
) -> Result<(), Box<dyn Error>> {
    let (cache, cached) = corrupt_cache()?;
    let output =
        research_guard(cache.path(), &["--fail-safe", "--non-blocking"])?;
    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains(&cached),
        "the cached path is named: {stderr}"
    );
    assert!(
        stderr.contains(
            "so the next call fetches a verified copy, or set \
             ACCELERATOR_RESEARCH_BIN"
        ),
        "the recovery step is given: {stderr}"
    );
    Ok(())
}

#[test]
fn a_fail_safe_refusal_still_blocks() -> Result<(), Box<dyn Error>> {
    let (cache, _) = corrupt_cache()?;
    let output = research_guard(cache.path(), &["--fail-safe"])?;
    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[test]
fn an_availability_failure_is_swallowed_only_under_fail_safe(
) -> Result<(), Box<dyn Error>> {
    let cache = tempfile::Builder::new().prefix("acc-policy-").tempdir()?;
    let non_blocking = research_guard(cache.path(), &["--non-blocking"])?;
    assert_eq!(non_blocking.status.code(), Some(1));
    let both =
        research_guard(cache.path(), &["--fail-safe", "--non-blocking"])?;
    assert_eq!(both.status.code(), Some(0));
    Ok(())
}
