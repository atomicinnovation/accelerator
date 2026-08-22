//! Full `work diff` CLI path against the `work-item-section-diff` golden
//! fixtures, gated on the real `diff` binary being on `PATH`.
#![cfg(feature = "bash-parity")]

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

fn fixture(case: &str, file: &str) -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!(
            "../work-adapters/tests/fixtures/work-item-section-diff/{case}/{file}"
        ))
        .canonicalize()?)
}

fn run(local: &Path, remote: &Path) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .arg("diff")
        .arg(local)
        .arg(remote)
        .output()?)
}

#[test]
fn frontmatter_only_case() -> Result<(), TestError> {
    let local = fixture("case-frontmatter-only", "local.md")?;
    let remote = fixture("case-frontmatter-only", "remote.md")?;
    let output = run(&local, &remote)?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.starts_with("=== frontmatter (- LOCAL / + REMOTE) ===\n"));
    Ok(())
}

#[test]
fn no_diff_after_normalisation_case() -> Result<(), TestError> {
    let local = fixture("case-no-diff-after-normalisation", "local.md")?;
    let remote = fixture("case-no-diff-after-normalisation", "remote.md")?;
    let output = run(&local, &remote)?;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)?.trim(),
        "(no differing sections after normalisation)"
    );
    Ok(())
}

#[test]
fn frontmatter_only_last_updated_change_still_shows_as_differing(
) -> Result<(), TestError> {
    let local = fixture("case-frontmatter-only-last-updated", "local.md")?;
    let remote = fixture("case-frontmatter-only-last-updated", "remote.md")?;
    let output = run(&local, &remote)?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.starts_with("=== frontmatter (- LOCAL / + REMOTE) ===\n"));
    assert!(stdout.contains("last_updated"));
    Ok(())
}
