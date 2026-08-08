//! Migration 0006 (`canonicalise-work-item-id-and-author`) driven end to
//! end against the compiled binary, asserted against a bash golden
//! captured in isolation (`ACCELERATOR_MIGRATIONS_DIR` scoped to just
//! 0006's script).

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
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n",
    )
}

#[test]
#[allow(clippy::too_many_lines)]
fn matches_the_isolated_bash_golden() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/plans/a.md",
        "---\nwork-item: 0042\ntitle: A\n---\n\n# Plan\n\n## Section\n",
    )?;
    write(
        root,
        "meta/plans/b.md",
        "---\nwork-item: 0042\nwork_item_id: \"0099\"\n---\n\n# Plan B\n",
    )?;
    write(
        root,
        "meta/research/codebase/c.md",
        "---\nresearcher: alice\n---\n\n**Researcher**: alice\n\n\
         ## Findings\n",
    )?;
    write(
        root,
        "meta/research/codebase/d.md",
        "---\nresearcher: bob\nauthor: carol\n---\n\n**Researcher**: bob\n\
         **Author**: carol\n",
    )?;
    write(
        root,
        "meta/research/issues/e.md",
        "work-item: 0001\n\n# No frontmatter\n",
    )?;
    write(
        root,
        "meta/research/issues/f.md",
        "---\nwork-item: \"bad # value\n---\n",
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
    // stdout+stderr through *its own* stderr, so these progress lines
    // are observable on stderr, never stdout.
    let stderr = String::from_utf8(output.stderr)?;
    for line in [
        "0006: rewrote 2 file(s) under meta/plans",
        "0006: rewrote 2 file(s) under meta/research/codebase",
        "0006: rewrote 0 file(s) under meta/research/issues",
        "Warning: 0006: 0006-DIVERGE:",
        "work-item=0042 vs work_item_id=\"0099\" (kept work_item_id)",
        "researcher=bob vs author=carol (kept author)",
        "**Researcher**=bob vs **Author**=carol (kept **Author**)",
        "Warning: 0006: 0006-REFUSE:",
        "refused work-item (unsafe value shape)",
        "Warning: 0006: 0006-MALFORMED:",
        "legacy key seen but no frontmatter fence (---) detected",
    ] {
        assert!(stderr.contains(line), "missing {line:?} in {stderr}");
    }

    assert_eq!(
        fs::read_to_string(root.join("meta/plans/a.md"))?,
        "---\nwork_item_id: \"0042\"\ntitle: A\n---\n\n# Plan\n\n\
         ## Section\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/plans/b.md"))?,
        "---\nwork_item_id: \"0099\"\n---\n\n# Plan B\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/research/codebase/c.md"))?,
        "---\nauthor: alice\n---\n\n**Author**: alice\n\n## Findings\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/research/codebase/d.md"))?,
        "---\nauthor: carol\n---\n\n**Author**: carol\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/research/issues/e.md"))?,
        "work-item: 0001\n\n# No frontmatter\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/research/issues/f.md"))?,
        "---\nwork-item: \"bad # value\n---\n"
    );
    Ok(())
}

#[test]
fn skips_a_dangerous_configured_corpus_path() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        ".claude/accelerator.md",
        "---\npaths:\n  plans: ../escape\n---\n",
    )?;
    already_applied(dir.path())?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("refusing dangerous paths.plans value: ../escape"),
        "{stderr}"
    );
    assert!(
        stderr.contains("0006: rewrote 0 file(s) under <unresolved plans>"),
        "{stderr}"
    );
    Ok(())
}
