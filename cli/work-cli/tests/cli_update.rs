//! CLI-boundary tests for `work update`.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use document::Scalar;
use document::Yaml;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-update-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    Command::new("git")
        .args(["config", "user.email", "t@e.x"])
        .current_dir(dir.path())
        .status()?;
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .status()?;
    Ok(dir)
}

fn write_fixture(
    dir: &Path,
    name: &str,
    content: &str,
) -> Result<std::path::PathBuf, TestError> {
    let path = dir.join(name);
    fs::write(&path, content)?;
    Ok(path)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .arg("update")
        .args(args)
        .current_dir(dir)
        .output()?)
}

const BASIC_FIXTURE: &str = "---\n\
title: Test\n\
status: draft\n\
priority: medium\n\
tags: [a, b]\n\
---\n\
body\n";

#[test]
fn set_changes_exactly_the_given_fields() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    let output = run(
        repo.path(),
        &[
            path.to_str().ok_or("non-utf8")?,
            "--set",
            "status=ready",
            "--set",
            "priority=high",
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let content = fs::read_to_string(&path)?;
    let yaml = document::parse(&content)?;
    let Yaml::Mapping(mapping) = yaml else {
        unreachable!("expected a mapping");
    };
    assert_eq!(
        mapping.get("title"),
        Some(&Yaml::Scalar(Scalar::String("Test".to_owned())))
    );
    assert_eq!(
        mapping.get("status"),
        Some(&Yaml::Scalar(Scalar::String("ready".to_owned())))
    );
    assert_eq!(
        mapping.get("priority"),
        Some(&Yaml::Scalar(Scalar::String("high".to_owned())))
    );
    Ok(())
}

#[test]
fn add_tag_appends_a_new_tag_without_losing_existing_ones(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    let output = run(
        repo.path(),
        &[path.to_str().ok_or("non-utf8")?, "--add-tag", "c"],
    )?;
    assert!(output.status.success(), "{output:?}");
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("tags: [a, b, c]"));
    Ok(())
}

#[test]
fn add_tag_repeated_across_invocations_never_loses_prior_tags(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    run(
        repo.path(),
        &[path.to_str().ok_or("non-utf8")?, "--set", "status=ready"],
    )?;
    let output = run(
        repo.path(),
        &[path.to_str().ok_or("non-utf8")?, "--add-tag", "c"],
    )?;
    assert!(output.status.success(), "{output:?}");
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("tags: [a, b, c]"));
    Ok(())
}

#[test]
fn add_tag_duplicate_is_a_no_op() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    let output = run(
        repo.path(),
        &[path.to_str().ok_or("non-utf8")?, "--add-tag", "a"],
    )?;
    assert!(output.status.success(), "{output:?}");
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("tags: [a, b]"));
    Ok(())
}

#[test]
fn remove_tag_removes_a_present_tag() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    let output = run(
        repo.path(),
        &[path.to_str().ok_or("non-utf8")?, "--remove-tag", "a"],
    )?;
    assert!(output.status.success(), "{output:?}");
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("tags: [b]"));
    Ok(())
}

#[test]
fn add_tag_against_block_style_tags_is_rejected() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let fixture = "---\ntitle: Test\ntags:\n  - a\n  - b\n---\nbody\n";
    let path = write_fixture(repo.path(), "f1.md", fixture)?;
    let output = run(
        repo.path(),
        &[path.to_str().ok_or("non-utf8")?, "--add-tag", "c"],
    )?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("tags field is in block format"));
    let unchanged = fs::read_to_string(&path)?;
    assert_eq!(unchanged, fixture, "a rejected mutation must not write");
    Ok(())
}

#[test]
fn append_on_a_missing_key_inserts_a_one_element_sequence(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    let output = run(
        repo.path(),
        &[
            path.to_str().ok_or("non-utf8")?,
            "--append",
            "blocks=work-item:0099",
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let content = fs::read_to_string(&path)?;
    let yaml = document::parse(&content)?;
    let Yaml::Mapping(mapping) = yaml else {
        unreachable!("expected a mapping");
    };
    assert_eq!(
        mapping.get("blocks"),
        Some(&Yaml::Sequence(vec![Yaml::Scalar(Scalar::String(
            "work-item:0099".to_owned()
        ))]))
    );
    Ok(())
}

#[test]
fn remove_on_an_existing_sequence_changes_only_that_field(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let fixture = "---\ntitle: Test\nblocks: [work-item:0001, work-item:0002]\n---\nbody\n";
    let path = write_fixture(repo.path(), "f1.md", fixture)?;
    let output = run(
        repo.path(),
        &[
            path.to_str().ok_or("non-utf8")?,
            "--remove",
            "blocks=work-item:0001",
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("title: Test"));
    let yaml = document::parse(&content)?;
    let Yaml::Mapping(mapping) = yaml else {
        unreachable!("expected a mapping");
    };
    assert_eq!(
        mapping.get("blocks"),
        Some(&Yaml::Sequence(vec![Yaml::Scalar(Scalar::String(
            "work-item:0002".to_owned()
        ))]))
    );
    Ok(())
}

#[test]
fn set_on_a_list_field_is_rejected_not_applied_as_a_scalar(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    let output = run(
        repo.path(),
        &[
            path.to_str().ok_or("non-utf8")?,
            "--set",
            "blocks=work-item:0099",
        ],
    )?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("--append/--remove"));
    let unchanged = fs::read_to_string(&path)?;
    assert_eq!(unchanged, BASIC_FIXTURE);
    Ok(())
}

#[test]
fn append_tags_is_rejected_use_add_tag_instead() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    let output = run(
        repo.path(),
        &[path.to_str().ok_or("non-utf8")?, "--append", "tags=c"],
    )?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("--add-tag/--remove-tag"));
    Ok(())
}

#[test]
fn set_id_is_hard_blocked_with_the_exact_message_even_with_other_valid_sets(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    let output = run(
        repo.path(),
        &[
            path.to_str().ok_or("non-utf8")?,
            "--set",
            "status=ready",
            "--set",
            "id=0099",
        ],
    )?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains(
        "Error: own-identity (id) cannot be changed — the filename prefix \
         is the authoritative work item ID."
    ));
    let unchanged = fs::read_to_string(&path)?;
    assert_eq!(
        unchanged, BASIC_FIXTURE,
        "no --set should apply when any key fails validation"
    );
    Ok(())
}

#[test]
fn a_missing_file_exits_one() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let missing = repo.path().join("does-not-exist.md");
    let output = run(
        repo.path(),
        &[missing.to_str().ok_or("non-utf8")?, "--set", "status=ready"],
    )?;
    assert_eq!(output.status.code(), Some(1));
    Ok(())
}

#[test]
fn concurrent_updates_never_lose_each_others_writes() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let fixture = "---\ntitle: Test\n---\nbody\n";
    let path = write_fixture(repo.path(), "f1.md", fixture)?;
    let mut children = Vec::new();
    for i in 0..5 {
        let child = Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
            .args([
                "update",
                path.to_str().ok_or("non-utf8")?,
                "--append",
                &format!("blocks=work-item:{i:04}"),
            ])
            .current_dir(repo.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        children.push(child);
    }
    for child in children {
        let output = child.wait_with_output()?;
        assert!(output.status.success(), "{output:?}");
    }
    let content = fs::read_to_string(&path)?;
    let yaml = document::parse(&content)?;
    let Yaml::Mapping(mapping) = yaml else {
        unreachable!("expected a mapping");
    };
    let Some(Yaml::Sequence(items)) = mapping.get("blocks") else {
        unreachable!("expected a blocks sequence");
    };
    assert_eq!(items.len(), 5, "every concurrent --append must survive");
    Ok(())
}

#[test]
fn a_comment_a_crlf_ending_and_a_flow_style_array_round_trip(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let fixture =
        "---\r\ntitle: Test\r\nstatus: draft  # trailing comment\r\ntags: [a, b]\r\n---\r\nbody\r\n";
    let path = write_fixture(repo.path(), "f1.md", fixture)?;
    let output = run(
        repo.path(),
        &[path.to_str().ok_or("non-utf8")?, "--set", "priority=high"],
    )?;
    assert!(output.status.success(), "{output:?}");
    let content = fs::read_to_string(&path)?;

    assert!(
        !content.contains("trailing comment"),
        "document::Yaml has no comment representation, so the inline \
         comment is expected to be dropped"
    );
    let split = document::split(&content)?;
    assert!(
        !split.frontmatter.contains('\r'),
        "the rewritten frontmatter is expected to normalise to LF"
    );
    assert!(
        split.body.contains('\r'),
        "the body is preserved verbatim, CRLF included — only the \
         frontmatter is re-serialised"
    );

    let yaml = document::parse(&content)?;
    let Yaml::Mapping(mapping) = yaml else {
        unreachable!("expected a mapping");
    };
    assert_eq!(
        mapping.get("status"),
        Some(&Yaml::Scalar(Scalar::String("draft".to_owned())))
    );
    assert_eq!(
        mapping.get("tags"),
        Some(&Yaml::Sequence(vec![
            Yaml::Scalar(Scalar::String("a".to_owned())),
            Yaml::Scalar(Scalar::String("b".to_owned())),
        ]))
    );
    assert_eq!(
        mapping.get("priority"),
        Some(&Yaml::Scalar(Scalar::String("high".to_owned())))
    );
    Ok(())
}

#[test]
fn a_dirty_working_copy_does_not_block_the_write() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let path = write_fixture(repo.path(), "f1.md", BASIC_FIXTURE)?;
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo.path())
        .status()?;
    Command::new("git")
        .args(["commit", "-q", "-m", "initial"])
        .current_dir(repo.path())
        .status()?;
    fs::write(&path, format!("{BASIC_FIXTURE}\nuncommitted trailer\n"))?;

    let output = run(
        repo.path(),
        &[path.to_str().ok_or("non-utf8")?, "--set", "status=ready"],
    )?;
    assert!(
        output.status.success(),
        "work update must write unconditionally, ignoring VCS dirtiness: \
         {output:?}"
    );
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("status: ready"));
    Ok(())
}
