//! The real, full 7-migration registry chained through one invocation
//! against a pristine pre-0001 layout with an empty ledger — every other
//! black-box test pre-marks all-but-one migration applied to isolate it;
//! this is the one place the whole chain runs together.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

#[test]
fn a_pristine_legacy_repo_applies_every_real_migration_in_order(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    fs::create_dir_all(root.join("meta/tickets"))?;
    fs::write(
        root.join("meta/tickets/0001-foo.md"),
        "---\nticket_id: 0001\ntitle: Foo\ntype: story\n\
         priority: medium\nstatus: draft\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
         schema_version: 1\n---\n\n# 0001: Foo\n",
    )?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let applied =
        fs::read_to_string(root.join(".accelerator/state/migrations-applied"))?;
    // 0002 has no project-token pattern configured in this fixture, so it
    // legitimately stays a no-op-pending (never recorded applied) — every
    // other migration in the chain applies for real.
    for id in [
        "0001-rename-tickets-to-work",
        "0003-relocate-accelerator-state",
        "0004-restructure-meta-research-into-subject-subcategories",
        "0005-rename-work-item-type-to-kind",
        "0006-canonicalise-work-item-id-and-author",
        "0007-unify-meta-corpus-frontmatter",
    ] {
        assert!(applied.contains(id), "missing {id} in {applied}");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("applied: 6; pending (no-op): 1."),
        "{stdout}"
    );

    // 0001 moved meta/tickets/ -> meta/work/; 0003 relocated the
    // config/state scaffold onto .accelerator/.
    assert!(!root.join("meta/tickets").exists());
    assert!(root.join(".accelerator").is_dir());
    let work_file = fs::read_to_string(root.join("meta/work/0001-foo.md"))?;
    assert!(
        work_file.contains("id: \"0001\""),
        "0007 should have canonicalised the id: {work_file}"
    );

    let second = Command::new(BIN).current_dir(root).output()?;
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    let second_stdout = String::from_utf8(second.stdout)?;
    assert!(
        second_stdout.contains("applied: 0; pending (no-op): 1."),
        "0002 stays pending on every run without a project-token \
         pattern configured: {second_stdout}"
    );
    Ok(())
}
