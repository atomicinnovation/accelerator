//! CLI-boundary tests for `work resolve`: exit-code mapping for all four
//! `InputClass` outcomes, exercised against a real temp directory (not
//! domain-level doubles — this is the adapter/binary boundary).

#![allow(clippy::literal_string_with_formatting_args)]

use std::fs;
use std::path::Path;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-resolve-")
        .tempdir()?;
    fs::create_dir_all(dir.path().join(".git"))?;
    fs::create_dir_all(dir.path().join("meta/work"))?;
    Ok(dir)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(args)
        .current_dir(dir)
        .output()?)
}

#[test]
fn path_class_resolved_directly_without_the_domain_search(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let target = repo.path().join("meta/work/0001-foo.md");
    fs::write(&target, "")?;
    let output = run(repo.path(), &["resolve", "meta/work/0001-foo.md"])?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(stdout.trim(), target.canonicalize()?.display().to_string());
    Ok(())
}

#[test]
fn path_class_miss_exits_three_not_one() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), &["resolve", "meta/work/nope.md"])?;
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_RESOLVE_NOT_FOUND"));
    Ok(())
}

#[test]
fn invalid_class_exits_one_immediately() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), &["resolve", "abc"])?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_RESOLVE_INVALID"));
    Ok(())
}

#[test]
fn bare_number_class_routes_through_the_domain_search() -> Result<(), TestError>
{
    let repo = scratch_repo()?;
    fs::write(repo.path().join("meta/work/0042-bar.md"), "")?;
    let output = run(repo.path(), &["resolve", "42"])?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.trim().ends_with("meta/work/0042-bar.md"));
    Ok(())
}

#[test]
fn bare_number_ambiguous_exits_two() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    fs::write(repo.path().join("meta/work/0042-a.md"), "")?;
    fs::write(repo.path().join("meta/work/0042-b.md"), "")?;
    let output = run(repo.path(), &["resolve", "42"])?;
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_RESOLVE_AMBIGUOUS"));
    Ok(())
}

#[test]
fn bare_number_not_found_exits_three() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), &["resolve", "99"])?;
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_RESOLVE_NOT_FOUND"));
    Ok(())
}

#[test]
fn path_outside_the_work_directory_exits_six() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    fs::write(repo.path().join("meta/secret.md"), "")?;
    let output = run(repo.path(), &["resolve", "meta/secret.md"])?;
    assert_eq!(output.status.code(), Some(6));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_RESOLVE_OUTSIDE_WORKDIR"));
    Ok(())
}

#[test]
fn traversal_path_escaping_the_work_directory_exits_six(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    fs::write(repo.path().join("meta/secret.md"), "")?;
    let output = run(repo.path(), &["resolve", "meta/work/../secret.md"])?;
    assert_eq!(output.status.code(), Some(6));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_RESOLVE_OUTSIDE_WORKDIR"));
    Ok(())
}

fn scratch_repo_with_work_config(
    work_body: &str,
) -> Result<tempfile::TempDir, TestError> {
    let dir = scratch_repo()?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(
        dir.path().join(".accelerator/config.md"),
        format!("---\nwork:\n{work_body}---\n"),
    )?;
    Ok(dir)
}

#[test]
fn resolve_refuses_a_malformed_pattern() -> Result<(), TestError> {
    let repo =
        scratch_repo_with_work_config("  id_pattern: \"{number:4d}\"\n")?;
    let output = run(repo.path(), &["resolve", "0042"])?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("format spec '4d' must match 0Nd (rule 4)"),
        "{stderr}"
    );
    assert!(
        !stderr
            .lines()
            .any(|line| line.starts_with("E_RESOLVE_INVALID:")),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_topic_without_a_work_directory_is_e_resolve_invalid(
) -> Result<(), TestError> {
    let repo = tempfile::Builder::new()
        .prefix("work-cli-resolve-")
        .tempdir()?;
    fs::create_dir_all(repo.path().join(".git"))?;

    let topic = run(repo.path(), &["resolve", "add dark mode"])?;
    assert_eq!(topic.status.code(), Some(1));
    let stderr = String::from_utf8(topic.stderr)?;
    assert!(
        stderr
            .lines()
            .any(|line| line.starts_with("E_RESOLVE_INVALID:")),
        "{stderr}"
    );

    let number = run(repo.path(), &["resolve", "0042"])?;
    assert_eq!(number.status.code(), Some(3));
    Ok(())
}
