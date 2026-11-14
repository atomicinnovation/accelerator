//! CLI-boundary tests for `work create`.

#![allow(clippy::literal_string_with_formatting_args)]

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
    install_work_item_template(dir.path())?;
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

fn install_work_item_template(dir: &Path) -> Result<(), TestError> {
    fs::create_dir_all(dir.join("meta/work"))?;
    let templates_dir = dir.join("templates");
    fs::create_dir_all(&templates_dir)?;
    fs::copy(
        repo_root()?.join("templates/work-item.md"),
        templates_dir.join("work-item.md"),
    )?;
    Ok(())
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
fn work_create_accepts_a_template_declaring_aliases() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let template =
        fs::read_to_string(repo.path().join("templates/work-item.md"))?;
    assert!(template.contains("\naliases: []"), "{template}");

    let output = run(repo.path(), &["create", "Test item", "task", "medium"])?;
    assert!(output.status.success(), "{output:?}");
    let path = String::from_utf8(output.stdout)?.trim().to_owned();
    let content = fs::read_to_string(&path)?;
    assert!(!content.contains("aliases:"), "{content}");
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

#[cfg(unix)]
#[test]
fn a_read_only_work_directory_refuses_the_creation_lock(
) -> Result<(), TestError> {
    use std::os::unix::fs::PermissionsExt as _;

    let repo = scratch_repo()?;
    let work_dir = repo.path().join("meta/work").canonicalize()?;
    fs::set_permissions(&work_dir, fs::Permissions::from_mode(0o555))?;

    let output = run(repo.path(), &["create", "Blocked", "task", "high"]);
    fs::set_permissions(&work_dir, fs::Permissions::from_mode(0o755))?;
    let output = output?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        format!(
            "Error: could not acquire the work-item creation lock: cannot \
             write under '{}': not writable: Permission denied (os error \
             13)\n",
            work_dir.join(".accelerator-work-create.lockdir").display()
        )
    );
    assert!(fs::read_dir(&work_dir)?.next().is_none());
    Ok(())
}

#[test]
fn the_author_comes_from_the_jj_identity_when_no_flag_is_given(
) -> Result<(), TestError> {
    use vcs_test_support::hermetic::Hermetic;

    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let work = tempfile::Builder::new()
        .prefix("work-cli-create-jj-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(&root)?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    install_work_item_template(&root)?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .args(["create", "Fresh item", "task", "high"])
        .current_dir(&root)
        .env("ACCELERATOR_PLUGIN_ROOT", &root);
    env.apply(&mut command);

    let output = command.output()?;

    assert!(output.status.success(), "{output:?}");
    let path = String::from_utf8(output.stdout)?.trim().to_owned();
    let content = fs::read_to_string(&path)?;
    assert!(content.contains("author: \"Fixture\""), "{content}");
    assert!(
        content.contains("last_updated_by: \"Fixture\""),
        "{content}"
    );
    Ok(())
}

const KEYED: &str = "  id_pattern: \"{key}-{number:04d}\"\n  key: PP\n";

fn created_path(output: &std::process::Output) -> Result<String, TestError> {
    assert!(output.status.success(), "{output:?}");
    Ok(String::from_utf8(output.stdout.clone())?.trim().to_owned())
}

fn work_items(repo: &Path) -> Result<Vec<String>, TestError> {
    let mut names: Vec<String> = fs::read_dir(repo.join("meta/work"))?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    Ok(names)
}

#[test]
fn create_counts_a_lower_case_key_in_the_configured_keys_sequence(
) -> Result<(), TestError> {
    let repo = scratch_repo_with_work_config(KEYED)?;
    for (number, slug) in (1..=10).zip('a'..='j') {
        fs::write(
            repo.path()
                .join(format!("meta/work/PP-{number:04}-{slug}.md")),
            "",
        )?;
    }
    fs::write(repo.path().join("meta/work/pp-0500-b.md"), "")?;

    let output = run(repo.path(), &["create", "Probe", "task", "low"])?;

    assert!(created_path(&output)?.ends_with("meta/work/PP-0501-probe.md"));
    Ok(())
}

#[test]
fn create_under_a_project_counts_that_keys_sequence_ignoring_case(
) -> Result<(), TestError> {
    let repo = scratch_repo_with_work_config(KEYED)?;
    fs::write(repo.path().join("meta/work/xy-0400-a.md"), "")?;

    let output = run(
        repo.path(),
        &["create", "Probe", "task", "low", "--project", "XY"],
    )?;

    assert!(created_path(&output)?.ends_with("meta/work/XY-0401-probe.md"));
    Ok(())
}

#[test]
fn a_hostile_project_or_pattern_is_refused_before_anything_is_written(
) -> Result<(), TestError> {
    let refusals = [
        (
            KEYED,
            vec!["--project", "../x"],
            "Error: E_WORK_KEY_INVALID: --project '../x': key value '../x' \
             must match [A-Za-z][A-Za-z0-9]* (rule 5)",
        ),
        (
            "  id_pattern: \"../{number:04d}\"\n",
            vec![],
            "Error: E_WORK_ID_PATTERN_INVALID: work.id_pattern \
             '../{number:04d}': literal '/' is forbidden in patterns (rule 2)",
        ),
        (
            "  id_pattern: \"x'{number:04d}\"\n",
            vec![],
            "Error: E_WORK_ID_PATTERN_INVALID: work.id_pattern \
             'x\\'{number:04d}': literal '\\'' is forbidden in patterns \
             (rule 2)",
        ),
    ];
    for (config, extra, reason) in refusals {
        let repo = scratch_repo_with_work_config(config)?;
        let mut args = vec!["create", "Probe", "task", "low"];
        args.extend(extra);

        let output = run(repo.path(), &args)?;

        assert_eq!(output.status.code(), Some(1), "{config}");
        let stderr = String::from_utf8(output.stderr)?;
        assert!(stderr.starts_with(reason), "{config}: {stderr}");
        assert!(work_items(repo.path())?.is_empty(), "{config}");
    }
    Ok(())
}

#[test]
fn allocation_under_a_key_followed_by_a_letter_continues_the_sequence(
) -> Result<(), TestError> {
    let repo = scratch_repo_with_work_config(
        "  id_pattern: \"{key}v{number:03d}\"\n  key: PP\n",
    )?;
    fs::write(repo.path().join("meta/work/PPv001-a.md"), "")?;

    let output = run(repo.path(), &["create", "Probe", "task", "low"])?;

    assert!(created_path(&output)?.ends_with("meta/work/PPv002-probe.md"));
    Ok(())
}

#[test]
fn allocation_keeps_the_patterns_literal_text() -> Result<(), TestError> {
    let repo =
        scratch_repo_with_work_config("  id_pattern: \"v{number:03d}\"\n")?;

    let first = run(repo.path(), &["create", "First", "task", "low"])?;
    let second = run(repo.path(), &["create", "Second", "task", "low"])?;

    assert!(created_path(&first)?.ends_with("meta/work/v001-first.md"));
    assert!(created_path(&second)?.ends_with("meta/work/v002-second.md"));
    Ok(())
}

#[test]
fn a_malformed_pattern_refuses_before_the_author_or_directory(
) -> Result<(), TestError> {
    let bare = tempfile::tempdir()?;
    fs::create_dir_all(bare.path().join(".accelerator"))?;
    fs::write(
        bare.path().join(".accelerator/config.md"),
        "---\nwork:\n  id_pattern: \"{number:4d}\"\n---\n",
    )?;

    let output = run(bare.path(), &["create", "Probe", "task", "low"])?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "E_WORK_ID_PATTERN_INVALID: work.id_pattern '{number:4d}':"
        ),
        "{stderr}"
    );
    assert!(!bare.path().join("meta/work").exists());
    Ok(())
}

const DEPRECATION: &str = "'work.default_project_code' is deprecated";

#[test]
fn the_deprecation_notice_is_printed_once_per_command() -> Result<(), TestError>
{
    let repo = scratch_repo_with_work_config(
        "  id_pattern: \"{project}-{number:04d}\"\n  \
         default_project_code: PP\n",
    )?;

    let created = run(repo.path(), &["create", "Probe", "task", "low"])?;
    let listed = run(repo.path(), &["list"])?;

    for output in [created, listed] {
        assert!(output.status.success(), "{output:?}");
        let stderr = String::from_utf8(output.stderr)?;
        assert_eq!(stderr.matches(DEPRECATION).count(), 1, "{stderr}");
    }
    Ok(())
}

fn work_dir_listing(repo: &Path) -> Result<Vec<PathBuf>, TestError> {
    let mut entries: Vec<PathBuf> = fs::read_dir(repo.join("meta/work"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    Ok(entries)
}

#[test]
fn a_refused_id_exits_1_with_its_code_on_stderr_and_writes_nothing(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let before = work_dir_listing(repo.path())?;

    let output = run(
        repo.path(),
        &["create", "Probe", "task", "low", "--id", "400"],
    )?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.starts_with("Error: E_ID_PATTERN_MISMATCH:"),
        "{stderr}"
    );
    assert!(stderr.contains("{number:04d}"), "{stderr}");
    assert_eq!(work_dir_listing(repo.path())?, before);
    Ok(())
}

#[test]
fn a_taken_id_exits_1_naming_the_holder() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    fs::write(repo.path().join("meta/work/0400-other.md"), "")?;

    let output = run(
        repo.path(),
        &["create", "Probe", "task", "low", "--id", "0400"],
    )?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("E_ID_TAKEN"), "{stderr}");
    assert!(stderr.contains("0400-other.md"), "{stderr}");
    Ok(())
}

#[test]
fn create_batch_with_an_id_is_an_unrecognised_argument() -> Result<(), TestError>
{
    let repo = scratch_repo()?;
    let before = work_dir_listing(repo.path())?;

    let output = run(
        repo.path(),
        &["create-batch", "--manifest", "m.json", "--id", "0400"],
    )?;

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("--id"), "{stderr}");
    assert_eq!(work_dir_listing(repo.path())?, before);
    Ok(())
}
