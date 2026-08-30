//! CLI-boundary tests for `work create`.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-create-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    let templates_dir = dir.path().join("templates");
    fs::create_dir_all(&templates_dir)?;
    let repo_root = repo_root()?;
    fs::copy(
        repo_root.join("templates/work-item.md"),
        templates_dir.join("work-item.md"),
    )?;
    let email_status = Command::new("git")
        .args(["config", "user.email", "t@e.x"])
        .current_dir(dir.path())
        .status()?;
    assert!(email_status.success());
    let name_status = Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .status()?;
    assert!(name_status.success());
    Ok(dir)
}

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .output()?)
}

#[test]
fn creates_a_work_item_with_the_template_schema() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(repo.path(), &["create", "Test item", "task", "medium"])?;
    assert!(output.status.success(), "{output:?}");
    let path = String::from_utf8(output.stdout)?.trim().to_owned();
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("id: \"0001\""));
    assert!(content.contains("title: \"Test item\""));
    assert!(content.contains("kind: \"task\""));
    assert!(content.contains("priority: \"medium\""));
    assert!(content.contains("status: \"draft\""));
    assert!(content.contains("schema_version: 1"));
    assert!(content.contains("# 0001: Test item"));
    // No --author flag: author must come from work_adapters::author::
    // current_vcs_user() reading the scratch repo's own configured
    // git user.name, proving that fallback resolves end to end.
    assert!(content.contains("author: \"Test User\""));
    assert!(content.contains("last_updated_by: \"Test User\""));
    Ok(())
}

#[test]
fn a_second_invocation_allocates_the_next_sequential_id(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    run(repo.path(), &["create", "First", "task", "medium"])?;
    let output = run(repo.path(), &["create", "Second", "task", "medium"])?;
    assert!(output.status.success());
    let path = String::from_utf8(output.stdout)?.trim().to_owned();
    assert!(path.contains("0002-second.md"));
    Ok(())
}

#[test]
fn typed_linkage_and_tags_are_populated_and_omitted_when_empty(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(
        repo.path(),
        &[
            "create",
            "Linked item",
            "task",
            "medium",
            "--parent",
            "work-item:0001",
            "--block",
            "work-item:0099",
            "--source",
            "issue-research:0002",
            "--tag",
            "api",
        ],
    )?;
    assert!(output.status.success());
    let path = String::from_utf8(output.stdout)?.trim().to_owned();
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("parent: \"work-item:0001\""));
    assert!(content.contains("blocks: [\"work-item:0099\"]"));
    assert!(content.contains("source: \"issue-research:0002\""));
    assert!(content.contains("tags: [\"api\"]"));
    assert!(!content.contains("blocked_by:"));
    assert!(!content.contains("derived_from:"));
    assert!(!content.contains("relates_to:"));
    assert!(!content.contains("external_id:"));
    Ok(())
}

#[test]
fn body_file_content_has_every_nnnn_occurrence_substituted(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let body_file = repo.path().join("draft-body.md");
    fs::write(
        &body_file,
        "# NNNN: placeholder\n\n## Summary\nCustom.\n\n## References\n- Related: NNNN, NNNN\n",
    )?;
    let output = run(
        repo.path(),
        &[
            "create",
            "Body test",
            "task",
            "low",
            "--body-file",
            body_file.to_str().ok_or("non-utf8")?,
        ],
    )?;
    assert!(output.status.success());
    let path = String::from_utf8(output.stdout)?.trim().to_owned();
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("# 0001: placeholder"));
    assert!(content.contains("Custom."));
    assert!(content.contains("Related: 0001, 0001"));
    assert!(!content.contains("NNNN"));
    Ok(())
}

#[test]
fn producer_default_is_overridable() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(
        repo.path(),
        &[
            "create",
            "Producer test",
            "task",
            "medium",
            "--producer",
            "create-work-item",
        ],
    )?;
    assert!(output.status.success());
    let path = String::from_utf8(output.stdout)?.trim().to_owned();
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("producer: \"create-work-item\""));
    Ok(())
}

#[test]
fn concurrent_creates_never_collide_on_the_same_id() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let mut children = Vec::new();
    for i in 0..5 {
        let child = Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
            .args(["create", &format!("Item {i}"), "task", "medium"])
            .current_dir(repo.path())
            .env("ACCELERATOR_PLUGIN_ROOT", repo.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        children.push(child);
    }
    let mut paths = Vec::new();
    for child in children {
        let output = child.wait_with_output()?;
        assert!(output.status.success(), "{output:?}");
        let path = String::from_utf8(output.stdout)?.trim().to_owned();
        assert!(!path.is_empty(), "expected a non-empty stdout path");
        paths.push(path);
    }
    let unique: std::collections::HashSet<_> = paths.iter().collect();
    assert_eq!(unique.len(), 5, "every concurrent create got a distinct ID");
    Ok(())
}

#[test]
fn a_bad_author_environment_is_reported_clearly() -> Result<(), TestError> {
    // No `.jj`/`.git` marker at all, so `current_vcs_user`'s repo-kind
    // detection short-circuits to `None` without ever shelling `git`/`jj`
    // (and so without risking a fallback read of this machine's own
    // ambient global `~/.gitconfig` identity, which a `.git`-marked but
    // otherwise-uninitialised directory would still expose).
    let bare = tempfile::tempdir()?;
    fs::create_dir_all(bare.path().join("meta/work"))?;
    let templates_dir = bare.path().join("templates");
    fs::create_dir_all(&templates_dir)?;
    fs::copy(
        repo_root()?.join("templates/work-item.md"),
        templates_dir.join("work-item.md"),
    )?;
    let output = run(bare.path(), &["create", "No author", "task", "medium"])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("author could not be resolved"));
    Ok(())
}

#[test]
fn explicit_author_flag_overrides_vcs_identity() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let output = run(
        repo.path(),
        &[
            "create",
            "Explicit author",
            "task",
            "medium",
            "--author",
            "Someone Else",
        ],
    )?;
    assert!(output.status.success());
    let path = String::from_utf8(output.stdout)?.trim().to_owned();
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("author: \"Someone Else\""));
    Ok(())
}

#[test]
fn overflow_is_reported_and_nothing_is_written() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    fs::write(repo.path().join("meta/work/9999-last.md"), "")?;
    let output = run(repo.path(), &["create", "Overflow", "task", "medium"])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("number space exhausted")
            || stderr.contains("exceeding")
    );
    let entries: Vec<_> = fs::read_dir(repo.path().join("meta/work"))?
        .filter_map(Result::ok)
        .collect();
    assert_eq!(entries.len(), 1, "no new file should have been written");
    Ok(())
}
