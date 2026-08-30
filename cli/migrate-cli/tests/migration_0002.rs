//! Migration 0002 (`rename-work-items-with-project-prefix`) driven end to
//! end against the compiled binary, asserted against a bash golden captured
//! in isolation (`ACCELERATOR_MIGRATIONS_DIR` scoped to just 0002's script)
//! against the tree `regenerate.sh`'s `setup_0002_repo` builds. Replayed
//! standalone because the fixture captured at `tests/fixtures/0002/` chains
//! 0002 with 0003/0005/0006.
#![allow(clippy::literal_string_with_formatting_args)]

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

/// Mirrors the retired `skills/config/migrate/scripts/test-fixtures/0002/`
/// tree, with 0001 pre-applied (via the ledger) so only 0002 is pending.
fn setup_repo() -> Result<TempDir, TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        ".claude/accelerator.md",
        "---\nwork:\n  id_pattern: \"{project}-{number:04d}\"\n  \
         default_project_code: PROJ\n---\n",
    )?;
    write(
        root,
        "meta/plans/2026-04-01-some-plan.md",
        "---\ntitle: Some plan\n---\n\n# Some plan\n\n## References\n\n\
         See [the foo work item](../work/0001-add-foo.md) for context.\n\n\
         Also check [bar with anchor](../work/0042-add-bar.md#section) for \
         details.\n\n## Summary of #0042\n\nThis section discusses the bar \
         feature.\n\n# closes #0001 and #0042\n",
    )?;
    write(
        root,
        "meta/research/2026-04-02-research.md",
        "---\ntitle: Research doc\nrelated: [\"0001\", \"0042\"]\n---\n\n\
         # Research\n\nReferences:\n\n```bash\ncat meta/work/0042-add-bar.md\n\
         ls meta/work/0001-add-foo.md\n```\n\nSee the related work items \
         above.\n",
    )?;
    write(
        root,
        "meta/research/2026-04-03-history.md",
        "---\ntitle: Historical notes\n---\n\n# Historical notes\n\n\
         Plain prose mentions of 0042 should not be rewritten.\n\nA \
         timestamp 2026-04-15 is safe.\n\nWe use port 0042 for the \
         service.\n\nThere were 0042 occurrences of the bug.\n\n```\n\
         meta/work/0042-add-bar.md\n```\n\nThe above bare fenced block (no \
         language tag) should not be rewritten.\n\n```bash\nRun: foo --id \
         0042\n```\n\nNon-path numeric usage in tagged blocks should not be \
         rewritten.\n",
    )?;
    write(
        root,
        "meta/work/0001-add-foo.md",
        "---\nwork_item_id: \"0001\"\ntitle: Add foo feature\n\
         status: in-progress\n---\n\n# 0001 Add foo feature\n\nThis is the \
         foo work item.\n",
    )?;
    write(
        root,
        "meta/work/0042-add-bar.md",
        "---\nwork_item_id: \"0042\"\ntitle: Add bar feature\nstatus: \
         done\nparent: \"0001\"\n---\n\n# 0042 Add bar feature\n\nThis is \
         the bar work item, child of foo.\n",
    )?;
    write(
        root,
        "meta/work/0099-bare-frontmatter.md",
        "---\nwork_item_id: \"0099\"\ntitle: Bare frontmatter test\n\
         status: backlog\nparent: 0042\nrelated: [0001, 0099]\n---\n\n\
         # 0099 Bare frontmatter test\n\nThis item tests bare (unquoted) \
         frontmatter values.\n",
    )?;
    write(
        root,
        "meta/work/notes.md",
        "# Work notes\n\nThis is a non-work-item file under meta/work/. It \
         has no frontmatter\nand should not be renamed or modified by the \
         migration.\n\nReferences to 0001 and 0042 in this file should be \
         left alone.\n",
    )?;
    write(
        root,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
    )?;
    Ok(dir)
}

#[test]
fn matches_the_isolated_bash_golden() -> Result<(), TestError> {
    let dir = setup_repo()?;
    let root = dir.path();

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "About to apply 1 migration(s):\n\
         \x20 0002-rename-work-items-with-project-prefix — Rename legacy \
         NNNN-*.md work items to the configured project-prefix pattern\n\
         \x20   To skip: accelerator migrate --skip \
         0002-rename-work-items-with-project-prefix\n\
         \n\
         Migrations rewrite files and may make repo-wide changes; commit\n\
         your working tree before running so VCS revert is available as\n\
         rollback. The pre-flight will refuse to run on a dirty tree\n\
         unless ACCELERATOR_MIGRATE_FORCE=1 is set.\n\
         \n\
         \n\
         Migration complete. applied: 1.\n"
    );

    assert!(!root.join("meta/work/0001-add-foo.md").exists());
    assert!(!root.join("meta/work/0042-add-bar.md").exists());
    assert!(!root.join("meta/work/0099-bare-frontmatter.md").exists());

    assert_eq!(
        fs::read_to_string(root.join("meta/work/PROJ-0001-add-foo.md"))?,
        "---\nwork_item_id: \"PROJ-0001\"\ntitle: Add foo feature\n\
         status: in-progress\n---\n\n# 0001 Add foo feature\n\nThis is the \
         foo work item.\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/work/PROJ-0042-add-bar.md"))?,
        "---\nwork_item_id: \"PROJ-0042\"\ntitle: Add bar feature\n\
         status: done\nparent: \"PROJ-0001\"\n---\n\n# 0042 Add bar \
         feature\n\nThis is the bar work item, child of foo.\n"
    );
    assert_eq!(
        fs::read_to_string(
            root.join("meta/work/PROJ-0099-bare-frontmatter.md")
        )?,
        "---\nwork_item_id: \"PROJ-0099\"\ntitle: Bare frontmatter test\n\
         status: backlog\nparent: \"PROJ-0042\"\nrelated: [\"PROJ-0001\", \
         \"PROJ-0099\"]\n---\n\n# 0099 Bare frontmatter test\n\nThis item \
         tests bare (unquoted) frontmatter values.\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/work/notes.md"))?,
        "# Work notes\n\nThis is a non-work-item file under meta/work/. It \
         has no frontmatter\nand should not be renamed or modified by the \
         migration.\n\nReferences to 0001 and 0042 in this file should be \
         left alone.\n"
    );

    assert_eq!(
        fs::read_to_string(root.join("meta/plans/2026-04-01-some-plan.md"))?,
        "---\ntitle: Some plan\n---\n\n# Some plan\n\n## References\n\n\
         See [the foo work item](../work/PROJ-0001-add-foo.md) for \
         context.\n\nAlso check [bar with anchor]\
         (../work/PROJ-0042-add-bar.md#section) for details.\n\n## \
         Summary of #PROJ-0042\n\nThis section discusses the bar \
         feature.\n\n# closes #PROJ-0001 and #PROJ-0042\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/research/2026-04-02-research.md"))?,
        "---\ntitle: Research doc\nrelated: [\"PROJ-0001\", \
         \"PROJ-0042\"]\n---\n\n# Research\n\nReferences:\n\n```bash\ncat \
         meta/work/PROJ-0042-add-bar.md\nls \
         meta/work/PROJ-0001-add-foo.md\n```\n\nSee the related work items \
         above.\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/research/2026-04-03-history.md"))?,
        "---\ntitle: Historical notes\n---\n\n# Historical notes\n\nPlain \
         prose mentions of 0042 should not be rewritten.\n\nA timestamp \
         2026-04-15 is safe.\n\nWe use port 0042 for the service.\n\nThere \
         were 0042 occurrences of the bug.\n\n```\nmeta/work/0042-add-bar.md\n\
         ```\n\nThe above bare fenced block (no language tag) should not \
         be rewritten.\n\n```bash\nRun: foo --id 0042\n```\n\nNon-path \
         numeric usage in tagged blocks should not be rewritten.\n"
    );
    Ok(())
}

#[test]
fn a_second_run_against_the_now_migrated_tree_is_byte_identical(
) -> Result<(), TestError> {
    let dir = setup_repo()?;
    let root = dir.path();

    Command::new(BIN).current_dir(root).output()?;
    let after_first =
        fs::read_to_string(root.join("meta/work/PROJ-0001-add-foo.md"))?;
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

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "No pending migrations.\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/work/PROJ-0001-add-foo.md"))?,
        after_first
    );
    Ok(())
}

#[test]
fn an_already_prefixed_tree_forced_pending_again_does_not_double_prefix(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        ".claude/accelerator.md",
        "---\nwork:\n  id_pattern: \"{project}-{number:04d}\"\n  \
         default_project_code: PROJ\n---\n",
    )?;
    let already_prefixed = "---\nwork_item_id: \"PROJ-0001\"\ntitle: Add \
         foo feature\nstatus: in-progress\n---\n\n# 0001 Add foo \
         feature\n\nThis is the foo work item.\n";
    write(root, "meta/work/PROJ-0001-add-foo.md", already_prefixed)?;
    write(
        root,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
    )?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!root.join("meta/work/PROJ-PROJ-0001-add-foo.md").exists());
    assert_eq!(
        fs::read_to_string(root.join("meta/work/PROJ-0001-add-foo.md"))?,
        already_prefixed
    );
    Ok(())
}

#[test]
fn no_project_token_in_pattern_is_a_no_op_pending() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
    )?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("Migration complete. applied: 0; pending (no-op): 1."),
        "{stdout}"
    );
    Ok(())
}

#[test]
fn missing_default_project_code_is_a_fatal_error() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        ".claude/accelerator.md",
        "---\nwork:\n  id_pattern: \"{project}-{number:04d}\"\n---\n",
    )?;
    write(
        dir.path(),
        "meta/work/0001-add-foo.md",
        "---\nwork_item_id: \"0001\"\n---\n",
    )?;
    write(
        dir.path(),
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
    )?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("requires a value for work.default_project_code"),
        "{stderr}"
    );
    assert!(!dir.path().join("meta/work/PROJ-0001-add-foo.md").exists());
    Ok(())
}

#[test]
fn a_rename_collision_refuses_without_mutating() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        ".claude/accelerator.md",
        "---\nwork:\n  id_pattern: \"{project}-{number:04d}\"\n  \
         default_project_code: PROJ\n---\n",
    )?;
    write(
        dir.path(),
        "meta/work/0001-add-foo.md",
        "---\nwork_item_id: \"0001\"\n---\n\nold\n",
    )?;
    write(
        dir.path(),
        "meta/work/PROJ-0001-add-foo.md",
        "---\nwork_item_id: \"PROJ-0001\"\n---\n\nalready here\n",
    )?;
    write(
        dir.path(),
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
    )?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("rename collision detected"), "{stderr}");
    assert_eq!(
        fs::read_to_string(dir.path().join("meta/work/0001-add-foo.md"))?,
        "---\nwork_item_id: \"0001\"\n---\n\nold\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("meta/work/PROJ-0001-add-foo.md"))?,
        "---\nwork_item_id: \"PROJ-0001\"\n---\n\nalready here\n"
    );
    Ok(())
}
