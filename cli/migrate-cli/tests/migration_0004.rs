//! Migration 0004 (`restructure-meta-research-into-subject-subcategories`)
//! driven end to end against the compiled binary, asserted against bash
//! goldens captured in isolation (`ACCELERATOR_MIGRATIONS_DIR` scoped to
//! just 0004's script).

use std::fs;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

fn write(
    dir: &std::path::Path,
    relative: &str,
    content: &str,
) -> Result<(), TestError> {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

fn already_applied(dir: &std::path::Path) -> Result<(), TestError> {
    write(
        dir,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n",
    )
}

#[test]
#[allow(clippy::too_many_lines)]
fn matches_the_isolated_bash_golden_for_default_layout() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/research/2026-01-01-a.md",
        "codebase research a\n",
    )?;
    write(root, "meta/research/.gitkeep", "")?;
    write(root, "meta/design-inventories/di1/note.md", "inv content\n")?;
    write(root, "meta/design-gaps/2026-01-01-gap.md", "gap a\n")?;
    write(
        root,
        "meta/plans/2026-02-01-plan.md",
        "---\ntitle: Some plan\n---\n\nSee meta/research/2026-01-01-a.md \
         and meta/design-inventories/di1 here.\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.ends_with("Migration complete. applied: 1.\n"),
        "{stdout}"
    );
    // The driver relays a mechanical migration's own combined
    // stdout+stderr through *its own* stderr (`run-migrations.sh`
    // captures `bash "$f" >"$STDOUT_FILE" 2>&1` then relays via `>&2`),
    // so bash's plain `echo` progress lines are observable on stderr,
    // never stdout — verified against a live combined-chain run, not
    // assumed from reading the migration script alone.
    let stderr = String::from_utf8(output.stderr)?;
    for line in [
        "0004: moved meta/research/2026-01-01-a.md → \
         meta/research/codebase/2026-01-01-a.md",
        "0004: moved meta/design-inventories/di1 → \
         meta/research/design-inventories/di1",
        "0004: moved meta/design-gaps/2026-01-01-gap.md → \
         meta/research/design-gaps/2026-01-01-gap.md",
        "0004: removed empty legacy directory meta/design-inventories",
        "0004: removed empty legacy directory meta/design-gaps",
        "0004: created meta/research/codebase/.gitkeep",
        "0004: created meta/research/issues/.gitkeep",
        "0004: created meta/research/design-inventories/.gitkeep",
        "0004: created meta/research/design-gaps/.gitkeep",
    ] {
        assert!(stderr.contains(line), "missing {line:?} in {stderr}");
    }

    assert_eq!(
        fs::read_to_string(
            root.join("meta/research/codebase/2026-01-01-a.md")
        )?,
        "codebase research a\n"
    );
    assert!(root.join("meta/research/.gitkeep").exists());
    assert_eq!(
        fs::read_to_string(
            root.join("meta/research/design-inventories/di1/note.md")
        )?,
        "inv content\n"
    );
    assert_eq!(
        fs::read_to_string(
            root.join("meta/research/design-gaps/2026-01-01-gap.md")
        )?,
        "gap a\n"
    );
    assert!(!root.join("meta/design-inventories").exists());
    assert!(!root.join("meta/design-gaps").exists());
    assert!(root.join("meta/research/issues/.gitkeep").exists());

    assert_eq!(
        fs::read_to_string(root.join("meta/plans/2026-02-01-plan.md"))?,
        "---\ntitle: Some plan\n---\n\nSee \
         meta/research/codebase/2026-01-01-a.md and \
         meta/research/design-inventories/di1 here.\n"
    );
    Ok(())
}

#[test]
fn is_a_true_no_op_when_no_legacy_directories_exist() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    already_applied(dir.path())?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.ends_with("Migration complete. applied: 1.\n"),
        "{stdout}"
    );
    assert_eq!(
        fs::read_to_string(
            dir.path().join(".accelerator/state/migrations-applied")
        )?,
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0004-restructure-meta-research-into-subject-subcategories\n"
    );
    Ok(())
}

#[test]
fn refuses_mixed_state_config() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        ".accelerator/config.md",
        "---\npaths:\n  research: meta/research\n  \
         research_codebase: meta/research/codebase\n---\n",
    )?;
    write(dir.path(), "meta/research/a.md", "x\n")?;
    already_applied(dir.path())?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("mixed-state config detected"), "{stderr}");
    assert!(dir.path().join("meta/research/a.md").exists());
    Ok(())
}
