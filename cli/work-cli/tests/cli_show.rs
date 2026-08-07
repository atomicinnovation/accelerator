//! CLI-boundary tests for `work show`.

use std::fs;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

fn run(args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(args)
        .output()?)
}

#[test]
fn shows_the_full_file_verbatim() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("0001-foo.md");
    let content = "---\nstatus: draft\n---\nbody\n";
    fs::write(&path, content)?;
    let output = run(&["show", path.to_str().ok_or("non-utf8 path")?])?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, content);
    Ok(())
}

#[test]
fn shows_a_single_field() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("0001-foo.md");
    fs::write(&path, "---\nstatus: draft\n---\nbody\n")?;
    let output = run(&[
        "show",
        path.to_str().ok_or("non-utf8 path")?,
        "--field",
        "status",
    ])?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "draft");
    Ok(())
}

#[test]
fn own_identity_alias_fallback() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("0001-foo.md");
    fs::write(&path, "---\nwork_item_id: 0099\n---\nbody\n")?;
    let output = run(&[
        "show",
        path.to_str().ok_or("non-utf8 path")?,
        "--field",
        "id",
    ])?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "0099");
    Ok(())
}

#[test]
fn missing_field_exits_one() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("0001-foo.md");
    fs::write(&path, "---\nstatus: draft\n---\nbody\n")?;
    let output = run(&[
        "show",
        path.to_str().ok_or("non-utf8 path")?,
        "--field",
        "priority",
    ])?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("No 'priority' field found"));
    Ok(())
}

#[test]
fn missing_file_exits_one() -> Result<(), TestError> {
    let output = run(&["show", "/nonexistent/definitely/not-here.md"])?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("File not found"));
    Ok(())
}

#[test]
fn no_frontmatter_exits_one() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("plain.md");
    fs::write(&path, "no frontmatter here\n")?;
    let output = run(&[
        "show",
        path.to_str().ok_or("non-utf8 path")?,
        "--field",
        "status",
    ])?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("No YAML frontmatter"));
    Ok(())
}

#[test]
fn unclosed_frontmatter_exits_one() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("unclosed.md");
    fs::write(&path, "---\nstatus: draft\nbody without closing\n")?;
    let output = run(&[
        "show",
        path.to_str().ok_or("non-utf8 path")?,
        "--field",
        "status",
    ])?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("opened but not closed"));
    Ok(())
}
