//! The dirty-tree pre-flight driven end to end against the compiled binary
//! over real git/jj repositories: foreign dirt refuses, `FORCE` bypasses.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::process::Command;

use tempfile::TempDir;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

fn tempdir(tag: &str) -> Result<TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("migrate-preflight-{tag}-"))
        .tempdir()?)
}

/// The compiled binary always runs the full registry and has no way to
/// isolate a subset of migrations for a test, so every real migration is
/// pre-marked applied here to reach the same "nothing pending" state these
/// tests were originally written against with an empty registry.
fn mark_all_migrations_applied(
    root: &std::path::Path,
) -> Result<(), TestError> {
    fs::create_dir_all(root.join(".accelerator/state"))?;
    fs::write(
        root.join(".accelerator/state/migrations-applied"),
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
    )?;
    Ok(())
}

fn run(
    root: &std::path::Path,
    env_extra: &[(&str, &str)],
) -> Result<(String, String, i32), TestError> {
    let mut command = Command::new(BIN);
    command.current_dir(root);
    for (key, value) in env_extra {
        command.env(key, value);
    }
    let output = command.output()?;
    Ok((
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?,
        output.status.code().unwrap_or(-1),
    ))
}

#[test]
fn a_foreign_dirty_git_file_refuses() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-refuse")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    fs::write(root.join("meta/a.md"), "two\n")?;

    let (stdout, stderr, code) = run(&root, &[])?;

    assert_eq!(code, 1);
    assert_eq!(stdout, "");
    assert_eq!(
        stderr,
        "Error: dirty working tree — uncommitted changes detected in \
         meta/, .claude/accelerator*.md, or .accelerator/.\nCommit or \
         discard those changes first, or set ACCELERATOR_MIGRATE_FORCE=1 \
         to skip this check.\n"
    );
    Ok(())
}

#[test]
fn force_bypasses_the_refusal_and_reaches_the_empty_registry_sentinel(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-force")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    fs::write(root.join("meta/a.md"), "two\n")?;
    mark_all_migrations_applied(&root)?;

    let (stdout, _stderr, code) =
        run(&root, &[("ACCELERATOR_MIGRATE_FORCE", "1")])?;

    assert_eq!(code, 0);
    assert_eq!(stdout, "No pending migrations.\n");
    Ok(())
}

#[test]
fn a_clean_git_tree_proceeds_to_the_empty_registry_sentinel(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-clean")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;
    mark_all_migrations_applied(&root)?;

    let (stdout, stderr, code) = run(&root, &[])?;

    assert_eq!(code, 0);
    assert_eq!(stdout, "No pending migrations.\n");
    assert_eq!(stderr, "");
    Ok(())
}

#[test]
fn a_guarded_resume_renders_the_affordance_with_the_decision_count(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-resume")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join(".accelerator/state"))?;
    fs::create_dir_all(root.join("meta"))?;
    let session_log = root.join(
        ".accelerator/state/migrations-0007-unify-meta-corpus-frontmatter-session.jsonl",
    );
    // gix's dirty-path scan excludes untracked files (mirroring `git status
    // --porcelain` filtered to tracked changes only) — every path this test
    // needs "dirty" must first be committed, then modified.
    fs::write(root.join("meta/a.md"), "one\n")?;
    fs::write(root.join(".accelerator/state/migrations-run-paths.txt"), "")?;
    fs::write(root.join(".accelerator/state/migrations-run.id"), "")?;
    fs::write(&session_log, "")?;
    env.git(&["init", "--quiet"], &root)?;
    env.git(&["add", "-A"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;
    let head = env.git(&["rev-parse", "HEAD"], &root)?;
    let head = head.trim();
    mark_all_migrations_applied(&root)?;

    fs::write(
        root.join(".accelerator/state/migrations-run.id"),
        format!("{head}\n"),
    )?;
    fs::write(
        &session_log,
        "{\"transformation_key\":\"k1\",\"schema_version\":1,\
         \"outcome\":\"accepted\",\"proposed_value\":\"v1\",\
         \"timestamp\":\"2026-01-01T00:00:00+00:00\"}\n\
         {\"transformation_key\":\"k2\",\"schema_version\":1,\
         \"outcome\":\"skipped\",\"proposed_value\":\"v2\",\
         \"timestamp\":\"2026-01-01T00:00:00+00:00\"}\n",
    )?;

    let (stdout, stderr, code) = run(&root, &[])?;

    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "No pending migrations.\n");
    assert!(
        stderr
            .contains("Resuming over this run's own partial migration output:"),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "  .accelerator/state/migrations-0007-unify-meta-corpus-frontmatter-session.jsonl"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "interactive migration — resuming: replays 2 decided \
             transformation(s) and re-prompts only undecided ones"
        ),
        "{stderr}"
    );
    let session_log_display =
        fs::canonicalize(&session_log)?.display().to_string();
    assert!(
        stderr.contains(&format!(
            "To discard instead: rm {session_log_display}  (loses 2 decisions)"
        )),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_foreign_dirty_file_under_accelerator_refuses() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-accelerator-dirty")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join(".accelerator"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join(".accelerator/config.md"), "---\n---\n")?;
    env.git(&["add", "-A"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    fs::write(
        root.join(".accelerator/config.md"),
        "---\nchanged: yes\n---\n",
    )?;

    let (stdout, stderr, code) = run(&root, &[])?;

    assert_eq!(code, 1);
    assert_eq!(stdout, "");
    assert!(stderr.contains("dirty working tree"), "{stderr}");
    Ok(())
}

#[test]
fn a_successful_run_clears_the_manifest_and_run_id_sidecar(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-clear")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;
    mark_all_migrations_applied(&root)?;

    let (stdout, stderr, code) = run(&root, &[])?;

    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "No pending migrations.\n");
    assert!(
        !root
            .join(".accelerator/state/migrations-run-paths.txt")
            .exists(),
        "the manifest sidecar must be cleared after a successful run"
    );
    assert!(
        !root.join(".accelerator/state/migrations-run.id").exists(),
        "the run-id sidecar must be cleared after a successful run"
    );
    Ok(())
}

#[test]
fn force_bypasses_only_the_dirty_check_not_a_skipped_migration(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-force-skip")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.git(&["add", "meta/a.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;
    fs::write(root.join("meta/a.md"), "two\n")?;
    fs::create_dir_all(root.join(".accelerator/state"))?;
    fs::write(
        root.join(".accelerator/state/migrations-applied"),
        "0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
    )?;
    fs::write(
        root.join(".accelerator/state/migrations-skipped"),
        "0001-rename-tickets-to-work\n",
    )?;

    let (stdout, _stderr, code) =
        run(&root, &[("ACCELERATOR_MIGRATE_FORCE", "1")])?;

    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.starts_with("No pending migrations.\n"), "{stdout}");
    assert!(
        stdout.contains("Skipped: 0001-rename-tickets-to-work"),
        "FORCE must not silently un-skip a skipped migration: {stdout}"
    );
    Ok(())
}

#[test]
fn a_foreign_dirty_jj_file_refuses() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let work = tempdir("jj-refuse")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join("meta"))?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join("meta/a.md"), "one\n")?;
    env.jj(&["commit", "-m", "init"], &root)?;

    fs::write(root.join("meta/b.md"), "two\n")?;

    let (stdout, stderr, code) = run(&root, &[])?;

    assert_eq!(code, 1);
    assert_eq!(stdout, "");
    assert!(stderr.contains("dirty working tree"), "{stderr}");
    Ok(())
}
