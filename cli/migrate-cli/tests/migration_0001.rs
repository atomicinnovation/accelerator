//! Migration 0001 (`rename-tickets-to-work`) driven end to end against the
//! compiled binary, asserted against the Phase-0-captured bash golden at
//! `tests/fixtures/0001/` (after the invocation-path normalisation
//! `masks.toml` documents: `bash .../run-migrations.sh` → `accelerator
//! migrate`).

use std::fs;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

/// Mirrors `regenerate.sh`'s `setup_old_repo`: the pre-0001 legacy ticket
/// structure, no VCS directory. The compiled binary always runs the full
/// registry (unlike bash's `ACCELERATOR_MIGRATIONS_DIR` isolation), so every
/// other real migration is pre-marked applied here to keep this fixture
/// scoped to 0001's own observable behaviour — update this list as later
/// migrations are registered.
fn setup_old_repo() -> Result<TempDir, TestError> {
    let dir = TempDir::new()?;
    fs::create_dir_all(dir.path().join("meta/tickets"))?;
    fs::write(
        dir.path().join("meta/tickets/0001-foo.md"),
        "---\nticket_id: 0001\n---\n\n# 0001: Foo\n",
    )?;
    fs::create_dir_all(dir.path().join("meta/reviews/tickets"))?;
    fs::write(
        dir.path().join("meta/reviews/tickets/foo-review-1.md"),
        "---\ntype: work-item-review\n---\n\n# foo-review-1\n",
    )?;
    fs::create_dir_all(dir.path().join(".claude"))?;
    fs::write(
        dir.path().join(".claude/accelerator.md"),
        "---\npaths:\n  tickets: meta/tickets\n---\n",
    )?;
    fs::create_dir_all(dir.path().join(".accelerator/state"))?;
    fs::write(
        dir.path().join(".accelerator/state/migrations-applied"),
        "0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n",
    )?;
    Ok(dir)
}

#[test]
fn matches_the_bash_golden_byte_for_byte() -> Result<(), TestError> {
    let dir = setup_old_repo()?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "About to apply 1 migration(s):\n\
         \x20 0001-rename-tickets-to-work — Rename tickets/work-item terminology in meta/ and config files\n\
         \x20   To skip: accelerator migrate --skip 0001-rename-tickets-to-work\n\
         \n\
         Migrations rewrite files and may make repo-wide changes; commit\n\
         your working tree before running so VCS revert is available as\n\
         rollback. The pre-flight will refuse to run on a dirty tree\n\
         unless ACCELERATOR_MIGRATE_FORCE=1 is set.\n\
         \n\
         \n\
         Migration complete. applied: 1.\n"
    );
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "[0001-rename-tickets-to-work] running\n\
         [0001-rename-tickets-to-work] applied\n"
    );

    assert_eq!(
        fs::read_to_string(dir.path().join(".claude/accelerator.md"))?,
        "---\npaths:\n  work: meta/work\n---\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("meta/work/0001-foo.md"))?,
        "---\nwork_item_id: 0001\n---\n\n# 0001: Foo\n"
    );
    assert_eq!(
        fs::read_to_string(
            dir.path().join("meta/reviews/work/foo-review-1.md")
        )?,
        "---\ntype: work-item-review\n---\n\n# foo-review-1\n"
    );
    assert_eq!(
        fs::read_to_string(
            dir.path().join(".accelerator/state/migrations-applied")
        )?,
        "0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0001-rename-tickets-to-work\n"
    );
    assert!(!dir.path().join("meta/tickets").exists());
    assert!(!dir.path().join("meta/reviews/tickets").exists());
    Ok(())
}

fn already_applied_except_0001(dir: &std::path::Path) -> Result<(), TestError> {
    fs::create_dir_all(dir.join(".accelerator/state"))?;
    fs::write(
        dir.join(".accelerator/state/migrations-applied"),
        "0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n",
    )?;
    Ok(())
}

#[test]
fn a_truly_empty_repo_still_applies_and_records_0001() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    fs::create_dir_all(dir.path().join("meta"))?;
    already_applied_except_0001(dir.path())?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        fs::read_to_string(
            dir.path().join(".accelerator/state/migrations-applied")
        )?
        .contains("0001-rename-tickets-to-work"),
        "0001 must be recorded applied even with nothing to rewrite"
    );
    Ok(())
}

#[test]
fn a_colliding_filename_across_tickets_and_work_is_source_wins(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    fs::create_dir_all(root.join("meta/tickets"))?;
    fs::write(
        root.join("meta/tickets/shared.md"),
        "---\nticket_id: 0001\n---\n\nsource\n",
    )?;
    fs::write(root.join("meta/tickets/only-in-tickets.md"), "new\n")?;
    fs::create_dir_all(root.join("meta/work"))?;
    fs::write(root.join("meta/work/shared.md"), "dest\n")?;
    fs::write(root.join("meta/work/only-in-work.md"), "kept\n")?;
    fs::create_dir_all(root.join("meta/reviews/tickets"))?;
    fs::write(
        root.join("meta/reviews/tickets/shared-review.md"),
        "source review\n",
    )?;
    fs::create_dir_all(root.join("meta/reviews/work"))?;
    fs::write(
        root.join("meta/reviews/work/shared-review.md"),
        "dest review\n",
    )?;
    already_applied_except_0001(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("meta/work/shared.md"))?,
        "---\nwork_item_id: 0001\n---\n\nsource\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/work/only-in-tickets.md"))?,
        "new\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/work/only-in-work.md"))?,
        "kept\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/reviews/work/shared-review.md"))?,
        "source review\n"
    );
    Ok(())
}

#[test]
fn malformed_config_frontmatter_aborts_with_zero_partial_writes(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    fs::create_dir_all(root.join(".claude"))?;
    fs::write(
        root.join(".claude/accelerator.md"),
        "---\npaths:\n  tickets: meta/tickets\n",
    )?;
    fs::create_dir_all(root.join("meta/tickets"))?;
    let ticket_content = "---\nticket_id: 0001\n---\n\n# 0001: Foo\n";
    fs::write(root.join("meta/tickets/0001-foo.md"), ticket_content)?;
    already_applied_except_0001(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("malformed frontmatter"), "{stderr}");
    assert!(!root.join("meta/work").exists());
    assert_eq!(
        fs::read_to_string(root.join("meta/tickets/0001-foo.md"))?,
        ticket_content
    );
    assert!(
        !fs::read_to_string(
            root.join(".accelerator/state/migrations-applied")
        )?
        .contains("0001"),
        "a refused run must not record 0001 as applied"
    );
    Ok(())
}

#[test]
fn a_filename_containing_spaces_renames_correctly() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    fs::create_dir_all(root.join("meta/tickets"))?;
    fs::write(
        root.join("meta/tickets/0001-with space.md"),
        "---\nticket_id: 0001\n---\n",
    )?;
    already_applied_except_0001(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("meta/work/0001-with space.md"))?,
        "---\nwork_item_id: 0001\n---\n"
    );
    Ok(())
}

#[test]
fn preserves_a_pinned_non_default_tickets_directory() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    fs::create_dir_all(dir.path().join("meta/custom-tickets"))?;
    fs::write(
        dir.path().join("meta/custom-tickets/0001-foo.md"),
        "---\nticket_id: 0001\n---\n",
    )?;
    fs::create_dir_all(dir.path().join(".claude"))?;
    fs::write(
        dir.path().join(".claude/accelerator.md"),
        "---\npaths:\n  tickets: meta/custom-tickets\n---\n",
    )?;
    fs::create_dir_all(dir.path().join(".accelerator/state"))?;
    fs::write(
        dir.path().join(".accelerator/state/migrations-applied"),
        "0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n",
    )?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(0));
    assert!(dir.path().join("meta/custom-tickets").is_dir());
    assert!(!dir.path().join("meta/work").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("meta/custom-tickets/0001-foo.md"))?,
        "---\nwork_item_id: 0001\n---\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(".claude/accelerator.md"))?,
        "---\npaths:\n  work: meta/custom-tickets\n---\n"
    );
    Ok(())
}

#[test]
fn running_it_twice_converges_and_the_second_run_reports_no_op(
) -> Result<(), TestError> {
    let dir = setup_old_repo()?;
    Command::new(BIN).current_dir(dir.path()).output()?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "No pending migrations.\n"
    );
    Ok(())
}

/// The transform's own idempotency, independent of the ledger filter: drop
/// just 0001's own ledger entry between two runs (leaving 0002/0003 marked
/// applied, so only 0001's `apply()` body actually executes again) and
/// confirm it converges (no duplicated/mangled output) rather than relying
/// solely on the ledger to prevent a second pass.
#[test]
fn the_transform_itself_converges_without_the_ledger_filter(
) -> Result<(), TestError> {
    let dir = setup_old_repo()?;
    Command::new(BIN).current_dir(dir.path()).output()?;
    fs::write(
        dir.path().join(".accelerator/state/migrations-applied"),
        "0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n",
    )?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(dir.path().join(".claude/accelerator.md"))?,
        "---\npaths:\n  work: meta/work\n---\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("meta/work/0001-foo.md"))?,
        "---\nwork_item_id: 0001\n---\n\n# 0001: Foo\n"
    );
    Ok(())
}
