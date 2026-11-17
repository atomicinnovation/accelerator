//! CLI-boundary tests for the refusal every command that reads or writes
//! IDs gives over a corpus holding an item re-key would move.

#![allow(clippy::literal_string_with_formatting_args)]

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

mod common;

type TestError = Box<dyn std::error::Error>;

const OFF_PATTERN_ITEM: &str = "meta/work/PP-0001-earlier.md";

fn diverged_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-rekey-required-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(
        dir.path().join(".accelerator/config.md"),
        "---\nwork:\n  id_pattern: \"{key}_{number:04d}\"\n  key: PP\n---\n",
    )?;
    fs::create_dir_all(dir.path().join("meta/work"))?;
    fs::write(
        dir.path().join(OFF_PATTERN_ITEM),
        "---\nid: \"PP-0001\"\ntitle: \"Earlier\"\nstatus: \"draft\"\n---\n",
    )?;
    let templates = dir.path().join("templates");
    fs::create_dir_all(&templates)?;
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../templates/work-item.md"),
        templates.join("work-item.md"),
    )?;
    fs::write(
        dir.path().join("manifest.json"),
        "{\"items\": [{\"title\": \"Batched\", \"kind\": \"task\"}]}",
    )?;
    Ok(dir)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null());
    common::scrub_provider_env(&mut command);
    Ok(command.output()?)
}

fn work_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir.join("meta/work"))
        .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
        .unwrap_or_default();
    files.sort();
    files
}

const GATED_COMMANDS: &str = include_str!("fixtures/gated-commands.txt");

fn command_name(args: &[&str]) -> String {
    match args {
        ["id", verb, ..] => format!("id {verb}"),
        [command, ..] => (*command).to_owned(),
        [] => String::new(),
    }
}

fn assert_every_gated_command_is_run(commands: &[&[&str]]) {
    let run: std::collections::BTreeSet<String> =
        commands.iter().map(|args| command_name(args)).collect();
    let gated: std::collections::BTreeSet<String> =
        GATED_COMMANDS.lines().map(str::to_owned).collect();
    assert_eq!(
        run, gated,
        "the gated commands are listed in {GATED_COMMANDS}"
    );
}

#[test]
fn every_id_command_refuses_a_diverged_corpus() -> Result<(), TestError> {
    let commands: &[&[&str]] = &[
        &["create", "Fresh item", "task", "medium"],
        &["create", "Fresh item", "task", "medium", "--id", "PP_0400"],
        &[
            "create",
            "Fresh item",
            "task",
            "medium",
            "--dry-run",
            "--id",
            "PP_0400",
        ],
        &[
            "create-batch",
            "--manifest",
            "manifest.json",
            "--author",
            "A",
        ],
        &["id", "next"],
        &["id", "check", "PP_0400"],
        &["resolve", "PP_0001"],
        &["promote", "draft-k7mq3x"],
    ];
    assert_every_gated_command_is_run(commands);
    for args in commands {
        let repo = diverged_repo()?;
        let before = work_files(repo.path());

        let output = run(repo.path(), args)?;

        let stderr = String::from_utf8(output.stderr)?;
        assert_eq!(output.status.code(), Some(4), "{args:?}: {stderr}");
        assert!(
            stderr.contains(&format!(
                "E_REKEY_REQUIRED: 1 work item is not under the ID \
                 `work.id_pattern` '{{key}}_{{number:04d}}' gives them, such \
                 as {OFF_PATTERN_ITEM} with `id` 'PP-0001'."
            )),
            "{args:?}: {stderr}"
        );
        assert_eq!(work_files(repo.path()), before, "{args:?}");
    }
    Ok(())
}

#[test]
fn list_warns_over_a_diverged_corpus_and_still_lists_it(
) -> Result<(), TestError> {
    let repo = diverged_repo()?;

    let output = run(repo.path(), &["list"])?;

    let stderr = String::from_utf8(output.stderr)?;
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains(&format!(
            "warning: E_REKEY_REQUIRED: 1 work item is not under the ID \
             `work.id_pattern` '{{key}}_{{number:04d}}' gives them, such as \
             {OFF_PATTERN_ITEM} with `id` 'PP-0001'."
        )),
        "{stderr}"
    );
    assert!(String::from_utf8(output.stdout)?.contains("PP-0001"));
    Ok(())
}

#[test]
fn list_warns_over_a_malformed_item_and_still_lists_the_rest(
) -> Result<(), TestError> {
    let repo = diverged_repo()?;
    fs::remove_file(repo.path().join(OFF_PATTERN_ITEM))?;
    fs::write(
        repo.path().join("meta/work/PP_0001.md"),
        "---\nid: \"PP_0001\"\ntitle: \"Slugless\"\n---\n",
    )?;

    let output = run(repo.path(), &["list"])?;

    let stderr = String::from_utf8(output.stderr)?;
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains("warning: E_WORK_ITEM_MALFORMED: meta/work/PP_0001.md"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn canonicalise_reads_no_corpus_and_is_not_refused() -> Result<(), TestError> {
    let repo = diverged_repo()?;

    let output = run(repo.path(), &["id", "canonicalise", "7"])?;

    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "PP_0007");
    Ok(())
}

#[test]
fn sync_is_not_refused_over_a_diverged_corpus() -> Result<(), TestError> {
    let repo = diverged_repo()?;
    fs::write(
        repo.path().join(".accelerator/config.md"),
        "---\nwork:\n  id_pattern: \"{key}_{number:04d}\"\n  key: PP\n  \
         integration: linear\n---\n",
    )?;

    let output = run(repo.path(), &["sync"])?;

    let stderr = String::from_utf8(output.stderr)?;
    assert!(!stderr.contains("E_REKEY_REQUIRED"), "{stderr}");
    assert_eq!(
        output.status.code(),
        Some(74),
        "sync reaches its tracker check, past the corpus: {stderr}"
    );
    assert!(stderr.contains("linear"), "{stderr}");
    Ok(())
}

#[test]
fn rekey_converges_a_diverged_corpus() -> Result<(), TestError> {
    let repo = diverged_repo()?;
    for args in [&["add", "-A"][..], &["commit", "-qm", "seed"][..]] {
        let status = Command::new("git")
            .args(["-c", "user.email=t@e.x", "-c", "user.name=T"])
            .args(args)
            .current_dir(repo.path())
            .status()?;
        assert!(status.success(), "git {args:?}");
    }

    let output = run(repo.path(), &["rekey"])?;

    assert!(output.status.success(), "{output:?}");
    let listed = run(repo.path(), &["list"])?;
    assert!(listed.status.success(), "{listed:?}");
    Ok(())
}

#[test]
fn every_id_command_refuses_a_malformed_item() -> Result<(), TestError> {
    let commands: &[&[&str]] = &[
        &["create", "Fresh item", "task", "medium"],
        &[
            "create-batch",
            "--manifest",
            "manifest.json",
            "--author",
            "A",
        ],
        &["id", "next"],
        &["id", "check", "PP_0400"],
        &["resolve", "PP_0001"],
        &["promote", "draft-k7mq3x"],
    ];
    assert_every_gated_command_is_run(commands);
    for args in commands {
        let repo = diverged_repo()?;
        fs::remove_file(repo.path().join(OFF_PATTERN_ITEM))?;
        fs::write(
            repo.path().join("meta/work/PP_0001.md"),
            "---\nid: \"PP_0001\"\ntitle: \"Slugless\"\n---\n",
        )?;
        let before = work_files(repo.path());

        let output = run(repo.path(), args)?;

        let stderr = String::from_utf8(output.stderr)?;
        assert_eq!(output.status.code(), Some(4), "{args:?}: {stderr}");
        assert!(
            stderr.contains(
                "E_WORK_ITEM_MALFORMED: meta/work/PP_0001.md has `id` \
                 'PP_0001' but its filename does not start with its `id` and \
                 a slug."
            ),
            "{args:?}: {stderr}"
        );
        assert_eq!(work_files(repo.path()), before, "{args:?}");
    }
    Ok(())
}

#[test]
fn a_malformed_key_the_pattern_ignores_is_warned_about_once(
) -> Result<(), TestError> {
    let repo = diverged_repo()?;
    fs::write(
        repo.path().join(".accelerator/config.md"),
        "---\nwork:\n  id_pattern: \"{number:04d}\"\n  key: 1PP\n---\n",
    )?;
    fs::remove_file(repo.path().join(OFF_PATTERN_ITEM))?;

    let output = run(
        repo.path(),
        &["create", "Fresh item", "task", "medium", "--author", "A"],
    )?;

    let stderr = String::from_utf8(output.stderr)?;
    assert!(output.status.success(), "{stderr}");
    assert_eq!(
        stderr.matches("warning: work.key '1PP' is unused").count(),
        1,
        "{stderr}"
    );
    Ok(())
}
