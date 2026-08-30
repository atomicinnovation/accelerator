//! Migration 0005 (`rename-work-item-type-to-kind`) driven end to end
//! against the compiled binary, asserted against a bash golden captured in
//! isolation (`ACCELERATOR_MIGRATIONS_DIR` scoped to just 0005's script).

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
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
    )
}

#[test]
fn matches_the_isolated_bash_golden() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/work/a.md",
        "---\ntype: work-item\ntitle: A\n---\n\n**Type**: work-item\n\n\
         Body.\n",
    )?;
    write(
        root,
        "meta/work/b.md",
        "---\ntype: work-item\nkind: bug\n---\n\n**Type**: work-item\n\
         **Kind**: bug\n",
    )?;
    write(
        root,
        "meta/work/c.md",
        "---\nkind: work-item\n---\n\nAlready migrated.\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.ends_with("Migration complete. applied: 1.\n"),
        "{stdout}"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("Warning: 0005: divergent type/kind in")
            && stderr.contains("kept kind=bug, dropped type=work-item"),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "Warning: 0005: divergent **Type**/**Kind** body label in"
        ) && stderr.contains("kept Kind=bug, dropped Type=work-item"),
        "{stderr}"
    );
    // The driver relays a mechanical migration's own combined
    // stdout+stderr through *its own* stderr, so this progress line is
    // observable on stderr, never stdout.
    assert!(
        stderr.contains("0005: rewrote 2 file(s) under meta/work"),
        "{stderr}"
    );

    assert_eq!(
        fs::read_to_string(root.join("meta/work/a.md"))?,
        "---\nkind: work-item\ntitle: A\n---\n\n**Kind**: work-item\n\n\
         Body.\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/work/b.md"))?,
        "---\nkind: bug\n---\n\n**Kind**: bug\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/work/c.md"))?,
        "---\nkind: work-item\n---\n\nAlready migrated.\n"
    );
    Ok(())
}

#[test]
fn a_type_key_with_no_type_body_label_renames_without_inserting_one(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/work/a.md",
        "---\ntype: adr-creation-task\ntitle: A\n---\n\nNo body label \
         here at all.\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let rewritten = fs::read_to_string(root.join("meta/work/a.md"))?;
    assert!(rewritten.contains("kind: adr-creation-task"), "{rewritten}");
    assert!(!rewritten.contains("**Type**:"), "{rewritten}");
    assert!(!rewritten.contains("**Kind**:"), "{rewritten}");
    Ok(())
}

#[test]
fn a_custom_work_path_is_honoured_and_the_default_never_created(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        ".claude/accelerator.md",
        "---\npaths:\n  work: docs/work\n---\n",
    )?;
    write(
        root,
        "docs/work/a.md",
        "---\ntype: story\ntitle: A\n---\n\n**Type**: story\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("docs/work/a.md"))?,
        "---\nkind: story\ntitle: A\n---\n\n**Kind**: story\n"
    );
    assert!(!root.join("meta/work").exists());
    Ok(())
}

#[test]
fn a_missing_custom_work_path_is_named_in_the_warning() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        ".claude/accelerator.md",
        "---\npaths:\n  work: docs/work\n---\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "Warning: 0005: work directory does not exist: docs/work"
        ),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn an_empty_work_dir_rewrites_zero_with_no_absence_warning(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    fs::create_dir_all(root.join("meta/work"))?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("0005: rewrote 0 file(s) under meta/work"),
        "{stderr}"
    );
    assert!(!stderr.contains("does not exist"), "{stderr}");
    Ok(())
}

#[test]
fn a_second_run_against_the_now_migrated_tree_is_byte_identical(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/work/a.md",
        "---\ntype: work-item\ntitle: A\n---\n\n**Type**: work-item\n\n\
         Body.\n",
    )?;
    write(
        root,
        "meta/work/b.md",
        "---\ntype: work-item\nkind: bug\n---\n\n**Type**: work-item\n\
         **Kind**: bug\n",
    )?;
    write(
        root,
        "meta/work/c.md",
        "---\nkind: work-item\n---\n\nAlready migrated.\n",
    )?;
    already_applied(root)?;

    Command::new(BIN).current_dir(root).output()?;
    let after_first = [
        fs::read_to_string(root.join("meta/work/a.md"))?,
        fs::read_to_string(root.join("meta/work/b.md"))?,
        fs::read_to_string(root.join("meta/work/c.md"))?,
    ];
    write(
        root,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
    )?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        [
            fs::read_to_string(root.join("meta/work/a.md"))?,
            fs::read_to_string(root.join("meta/work/b.md"))?,
            fs::read_to_string(root.join("meta/work/c.md"))?,
        ],
        after_first
    );
    Ok(())
}

#[test]
fn refuses_a_dangerous_configured_work_path() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        ".claude/accelerator.md",
        "---\npaths:\n  work: ../escape\n---\n",
    )?;
    already_applied(dir.path())?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("refusing dangerous paths.work value: ../escape"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn warns_and_counts_zero_when_the_work_dir_is_absent() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    already_applied(dir.path())?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("0005: rewrote 0 file(s) under meta/work"),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "Warning: 0005: work directory does not exist: meta/work"
        ),
        "{stderr}"
    );
    Ok(())
}
