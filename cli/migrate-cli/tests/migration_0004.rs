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
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
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
    // A mechanical migration's own combined stdout+stderr is relayed
    // through this binary's stderr, so its plain progress lines are
    // observable on stderr, never stdout — verified against a live
    // combined-chain run, not assumed from reading the migration script
    // alone.
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
fn ds_store_is_swept_and_a_research_override_targets_the_codebase_subdir(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        ".accelerator/config.md",
        "---\npaths:\n  research: docs/research\n---\n",
    )?;
    write(root, "docs/research/a.md", "codebase research a\n")?;
    write(root, "docs/research/.DS_Store", "junk\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("docs/research/codebase/a.md"))?,
        "codebase research a\n"
    );
    assert!(!root.join("docs/research/codebase/.DS_Store").exists());
    assert!(!root.join("meta/research").exists());
    Ok(())
}

#[test]
fn design_inventory_and_gap_overrides_suppress_those_specific_moves(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        ".accelerator/config.md",
        "---\npaths:\n  design_inventories: assets/inventories\n  \
         design_gaps: assets/gaps\n---\n",
    )?;
    write(root, "assets/inventories/di1/note.md", "inv content\n")?;
    write(root, "assets/gaps/2026-01-01-gap.md", "gap a\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("assets/inventories/di1/note.md"))?,
        "inv content\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("assets/gaps/2026-01-01-gap.md"))?,
        "gap a\n"
    );
    assert!(!root.join("meta/research/design-inventories").exists());
    assert!(!root.join("meta/research/design-gaps").exists());
    Ok(())
}

#[test]
fn a_destination_collision_on_move_is_source_wins() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(root, "meta/research/a.md", "SRC\n")?;
    write(root, "meta/research/codebase/a.md", "DST\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("meta/research/codebase/a.md"))?,
        "SRC\n"
    );
    Ok(())
}

#[test]
fn a_second_run_against_an_already_migrated_tree_makes_zero_further_changes(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/research/2026-01-01-a.md",
        "codebase research a\n",
    )?;
    write(root, "meta/design-inventories/di1/note.md", "inv content\n")?;
    write(root, "meta/design-gaps/2026-01-01-gap.md", "gap a\n")?;
    already_applied(root)?;

    Command::new(BIN).current_dir(root).output()?;
    let after_first = read_tree(root)?;
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
        String::from_utf8(output.stdout)?,
        "No pending migrations.\n"
    );
    let mut after_second = read_tree(root)?;
    after_second.remove(".accelerator/state/migrations-applied");
    let mut expected = after_first;
    expected.remove(".accelerator/state/migrations-applied");
    assert_eq!(after_second, expected);
    Ok(())
}

#[test]
fn a_local_config_only_override_is_honoured_independently_of_config_md(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(root, ".accelerator/config.md", "---\nauthor: Toby\n---\n")?;
    write(
        root,
        ".accelerator/config.local.md",
        "---\npaths:\n  research: docs/research\n---\n",
    )?;
    // The permission guard refuses to read a personal-level config file
    // whose mode grants any group/other access, so the fixture must match
    // the mode a real `accelerator config set --local` always produces.
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(
            root.join(".accelerator/config.local.md"),
            fs::Permissions::from_mode(0o600),
        )?;
    }
    write(root, "docs/research/a.md", "x\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("docs/research/codebase/a.md"))?,
        "x\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/config.md"))?,
        "---\nauthor: Toby\n---\n"
    );
    // Both existing config files are unconditionally backed up once a
    // research override is in play anywhere, regardless of whether each
    // individual file actually contains the overridden key — preserving a
    // historical bash quirk rather than tightening the condition.
    assert!(root.join(".accelerator/config.local.md.0004.bak").exists());
    assert!(root.join(".accelerator/config.md.0004.bak").exists());
    let rewritten_local =
        fs::read_to_string(root.join(".accelerator/config.local.md"))?;
    assert!(
        rewritten_local.contains("research_codebase: docs/research/codebase"),
        "{rewritten_local}"
    );
    Ok(())
}

#[test]
fn an_empty_paths_block_is_left_byte_unchanged() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    let config = "---\npaths:\n---\n";
    write(root, ".accelerator/config.md", config)?;
    write(root, "meta/research/a.md", "x\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/config.md"))?,
        config
    );
    Ok(())
}

#[test]
fn the_rename_notification_fires_only_when_a_config_key_actually_rewrites(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        ".accelerator/config.md",
        "---\npaths:\n  research: docs/research\n---\n",
    )?;
    write(root, "docs/research/a.md", "x\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "0004: renamed paths.research → paths.research_codebase \
             (value: docs/research → docs/research/codebase)"
        ),
        "{stderr}"
    );

    let dir2 = TempDir::new()?;
    let root2 = dir2.path();
    write(root2, "meta/research/a.md", "x\n")?;
    already_applied(root2)?;
    let output2 = Command::new(BIN).current_dir(root2).output()?;
    assert_eq!(output2.status.code(), Some(0), "{output2:?}");
    let stderr2 = String::from_utf8(output2.stderr)?;
    assert!(!stderr2.contains("0004: renamed"), "{stderr2}");
    Ok(())
}

#[test]
fn a_moved_files_own_cross_link_to_a_sibling_moved_file_is_rewritten(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/research/2026-01-01-a.md",
        "---\ntitle: A\n---\n\nSee meta/research/2026-01-02-b.md.\n",
    )?;
    write(
        root,
        "meta/research/2026-01-02-b.md",
        "---\ntitle: B\n---\n\nSee meta/research/2026-01-01-a.md.\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(
            root.join("meta/research/codebase/2026-01-01-a.md")
        )?,
        "---\ntitle: A\n---\n\nSee \
         meta/research/codebase/2026-01-02-b.md.\n"
    );
    assert_eq!(
        fs::read_to_string(
            root.join("meta/research/codebase/2026-01-02-b.md")
        )?,
        "---\ntitle: B\n---\n\nSee \
         meta/research/codebase/2026-01-01-a.md.\n"
    );
    Ok(())
}

fn read_tree(
    root: &std::path::Path,
) -> Result<std::collections::BTreeMap<String, String>, TestError> {
    fn walk(
        dir: &std::path::Path,
        root: &std::path::Path,
        out: &mut std::collections::BTreeMap<String, String>,
    ) -> Result<(), TestError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                walk(&path, root, out)?;
            } else {
                let relative =
                    path.strip_prefix(root)?.to_string_lossy().into_owned();
                out.insert(relative, fs::read_to_string(&path)?);
            }
        }
        Ok(())
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, root, &mut out)?;
    Ok(out)
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
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n\
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
