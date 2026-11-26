//! A configuration whose doc roots overlap stops every `accelerator-migrate`
//! run and listing before any migration is applied or listed.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

const LEDGER: &str = ".accelerator/state/migrations-applied";

const EVERY_MIGRATION: &str = "0001-rename-tickets-to-work\n\
     0002-rename-work-items-with-project-prefix\n\
     0003-relocate-accelerator-state\n\
     0004-restructure-meta-research-into-subject-subcategories\n\
     0005-rename-work-item-type-to-kind\n\
     0006-canonicalise-work-item-id-and-author\n\
     0007-unify-meta-corpus-frontmatter\n\
     0008-canonical-frontmatter-quoting\n\
     0009-split-work-key-from-tracker-scope-key\n\
     0010-strip-research-title-prefix\n\
     0011-ignore-machine-local-sync-state\n";

fn overlapping_project() -> Result<TempDir, TestError> {
    let dir = TempDir::new()?;
    fs::create_dir_all(dir.path().join(".git"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(
        dir.path().join(".accelerator/config.md"),
        "---\npaths:\n  research_codebase: meta/research\n---\n",
    )?;
    Ok(dir)
}

fn fully_applied_overlapping_project() -> Result<TempDir, TestError> {
    let dir = overlapping_project()?;
    fs::create_dir_all(dir.path().join(".accelerator/state"))?;
    fs::write(dir.path().join(LEDGER), EVERY_MIGRATION)?;
    Ok(dir)
}

fn migrate(root: &Path, args: &[&str]) -> Result<Output, TestError> {
    Ok(Command::new(BIN)
        .args(args)
        .current_dir(root)
        .env_remove("ACCELERATOR_FORCE")
        .env_remove("ACCELERATOR_MIGRATE_FORCE")
        .env_remove("ACCELERATOR_MIGRATE_DECISIONS_FILE")
        .output()?)
}

fn assert_refused(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(output.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains("paths.research_codebase")
            && stderr.contains("paths.research_issues"),
        "the refusal does not name both keys: {stderr}"
    );
}

#[test]
fn a_run_over_overlapping_doc_roots_applies_and_records_nothing(
) -> Result<(), TestError> {
    let dir = overlapping_project()?;

    let output = migrate(dir.path(), &[])?;

    assert_refused(&output);
    let ledger =
        fs::read_to_string(dir.path().join(LEDGER)).unwrap_or_default();
    assert_eq!(ledger, "");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Applied"));
    Ok(())
}

#[test]
fn a_listing_over_overlapping_doc_roots_is_refused() -> Result<(), TestError> {
    let dir = overlapping_project()?;

    let output = migrate(dir.path(), &["--list"])?;

    assert_refused(&output);
    Ok(())
}

#[test]
fn an_up_to_date_run_over_overlapping_doc_roots_is_refused(
) -> Result<(), TestError> {
    let dir = fully_applied_overlapping_project()?;

    let output = migrate(dir.path(), &[])?;

    assert_refused(&output);
    assert_eq!(
        fs::read_to_string(dir.path().join(LEDGER))?,
        EVERY_MIGRATION
    );
    Ok(())
}

#[test]
fn a_listing_with_no_interactive_migration_pending_is_refused(
) -> Result<(), TestError> {
    let dir = fully_applied_overlapping_project()?;

    let output = migrate(dir.path(), &["--list"])?;

    assert_refused(&output);
    Ok(())
}
