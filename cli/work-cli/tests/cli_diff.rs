//! CLI-boundary tests for `work diff`.

use std::fs;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

fn run(args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(args)
        .output()?)
}

#[test]
fn no_differences_after_normalisation() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let local = dir.path().join("local.md");
    let remote = dir.path().join("remote.md");
    fs::write(&local, "---\nstatus: draft   \n---\n## Summary\ntext   \n")?;
    fs::write(&remote, "---\nstatus: draft\n---\n## Summary\ntext\n")?;
    let output = run(&[
        "diff",
        local.to_str().ok_or("non-utf8")?,
        remote.to_str().ok_or("non-utf8")?,
    ])?;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)?.trim(),
        "(no differing sections after normalisation)"
    );
    Ok(())
}

#[test]
fn frontmatter_only_change_is_shown() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let local = dir.path().join("local.md");
    let remote = dir.path().join("remote.md");
    fs::write(&local, "---\nstatus: draft\n---\n## Summary\ntext\n")?;
    fs::write(&remote, "---\nstatus: ready\n---\n## Summary\ntext\n")?;
    let output = run(&[
        "diff",
        local.to_str().ok_or("non-utf8")?,
        remote.to_str().ok_or("non-utf8")?,
    ])?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.starts_with("=== frontmatter (- LOCAL / + REMOTE) ===\n"));
    assert!(stdout.contains("-status: draft"));
    assert!(stdout.contains("+status: ready"));
    Ok(())
}

#[test]
fn one_differing_section_renders_a_unified_body_with_the_frozen_framing(
) -> Result<(), TestError> {
    const EXPECTED: &str = concat!(
        "=== Summary (- LOCAL / + REMOTE) ===\n",
        "@@ -1 +1 @@\n",
        "-local text\n",
        "+remote text\n",
        "\n",
    );
    let dir = tempfile::tempdir()?;
    let local = dir.path().join("local.md");
    let remote = dir.path().join("remote.md");
    fs::write(
        &local,
        "---\nstatus: draft\n---\n## Summary\nlocal text\n## Notes\nsame\n",
    )?;
    fs::write(
        &remote,
        "---\nstatus: draft\n---\n## Summary\nremote text\n## Notes\nsame\n",
    )?;
    let output = run(&[
        "diff",
        local.to_str().ok_or("non-utf8")?,
        remote.to_str().ok_or("non-utf8")?,
    ])?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(stdout, EXPECTED);
    Ok(())
}

#[test]
fn a_last_updated_only_frontmatter_change_renders_into_the_body(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let local = dir.path().join("local.md");
    let remote = dir.path().join("remote.md");
    fs::write(
        &local,
        "---\nstatus: draft\nlast_updated: \"2026-01-01\"\n---\ntext\n",
    )?;
    fs::write(
        &remote,
        "---\nstatus: draft\nlast_updated: \"2026-01-02\"\n---\ntext\n",
    )?;
    let output = run(&[
        "diff",
        local.to_str().ok_or("non-utf8")?,
        remote.to_str().ok_or("non-utf8")?,
    ])?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.starts_with("=== frontmatter (- LOCAL / + REMOTE) ===\n"));
    assert!(stdout.contains("-last_updated: \"2026-01-01\""));
    assert!(stdout.contains("+last_updated: \"2026-01-02\""));
    Ok(())
}

#[test]
fn missing_argument_exits_two() -> Result<(), TestError> {
    let output = run(&["diff", "one.md"])?;
    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[test]
fn non_file_argument_exits_two() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let local = dir.path().join("local.md");
    fs::write(&local, "---\nstatus: draft\n---\nbody\n")?;
    let output = run(&[
        "diff",
        local.to_str().ok_or("non-utf8")?,
        "/nonexistent/definitely/not-here.md",
    ])?;
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("both arguments must be files"));
    Ok(())
}
