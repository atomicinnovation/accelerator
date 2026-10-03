//! CLI-boundary tests for `work create-batch`: the report a declined batch
//! prints, and the refusal of a manifest whose parents form a cycle.
#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

mod common;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-create-batch-")
        .tempdir()?;
    fs::create_dir_all(dir.path().join(".git"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(
        dir.path().join(".accelerator/config.md"),
        "---\nwork:\n  integration: linear\n  id_pattern: \"{tracker}\"\n---\n",
    )?;
    fs::create_dir_all(dir.path().join("meta/work"))?;
    let templates_dir = dir.path().join("templates");
    fs::create_dir_all(&templates_dir)?;
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../templates/work-item.md"),
        templates_dir.join("work-item.md"),
    )?;
    Ok(dir)
}

fn run(dir: &Path, manifest: &str) -> Result<std::process::Output, TestError> {
    fs::write(dir.join("manifest.json"), manifest)?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .args([
            "create-batch",
            "--manifest",
            "manifest.json",
            "--author",
            "A Tester",
        ])
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(std::process::Stdio::null());
    common::scrub_provider_env(&mut command);
    Ok(command.output()?)
}

fn markdown_under(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_declined_batch_prints_one_keyword_line_per_item_and_exits_zero(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;

    let output = run(
        repo.path(),
        r#"[{"ref": "epic", "title": "The epic", "kind": "epic",
             "priority": "high"},
            {"ref": "story", "title": "A story", "kind": "story",
             "priority": "low", "parent": {"ref": "epic"}}]"#,
    )?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let lines: Vec<Vec<&str>> = stdout
        .lines()
        .map(|line| line.split('\t').collect())
        .collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    for (line, reference) in lines.iter().zip(["epic", "story"]) {
        assert_eq!(line.len(), 4, "{stdout}");
        assert_eq!(line[0], reference);
        assert!(line[1].contains("/meta/work/drafts/draft-"), "{stdout}");
        assert_eq!(line[2], "declined");
        assert_eq!(line[3], "");
    }
    Ok(())
}

#[test]
fn a_cyclic_manifest_exits_two_and_writes_nothing() -> Result<(), TestError> {
    let repo = scratch_repo()?;

    let output = run(
        repo.path(),
        r#"[{"ref": "a", "title": "A", "kind": "story", "priority": "low",
             "parent": {"ref": "b"}},
            {"ref": "b", "title": "B", "kind": "story", "priority": "low",
             "parent": {"ref": "a"}}]"#,
    )?;

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_BATCH_CYCLE"), "{stderr}");
    assert!(stderr.contains("a, b"), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(markdown_under(&repo.path().join("meta/work")).is_empty());
    assert!(markdown_under(&repo.path().join("meta/work/drafts")).is_empty());
    Ok(())
}
