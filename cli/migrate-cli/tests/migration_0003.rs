//! Migration 0003 (`relocate-accelerator-state`) driven end to end against
//! the compiled binary, asserted against a bash golden captured in
//! isolation (`ACCELERATOR_MIGRATIONS_DIR` scoped to just 0003's script)
//! against a from-scratch fixture exercising every move pair, the
//! pinned-override warning, the root/inner `.gitignore` rewrites, and the
//! legacy state-file merge.

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
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n",
    )
}

#[test]
#[allow(clippy::too_many_lines)]
fn matches_the_isolated_bash_golden() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        ".claude/accelerator.md",
        "---\npaths:\n  templates: custom/templates\n---\n",
    )?;
    write(root, ".claude/accelerator.local.md", "---\n---\n")?;
    // The permission guard refuses to read a personal-level config file
    // whose mode grants any group/other access, so the fixture must match
    // the mode a real `accelerator config set --local` always produces.
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(
            root.join(".claude/accelerator.local.md"),
            fs::Permissions::from_mode(0o600),
        )?;
    }
    write(root, ".claude/accelerator/skills/foo.md", "team skill\n")?;
    write(root, ".claude/accelerator/lenses/bar.md", "team lens\n")?;
    write(root, "meta/templates/t.md", "tmpl\n")?;
    write(root, "meta/integrations/jira/site.json", "jira state\n")?;
    write(root, "meta/tmp/scratch.txt", "tmpfile\n")?;
    write(root, "meta/.migrations-applied", "a\nb\n")?;
    write(root, "meta/.migrations-skipped", "c\n")?;
    write(
        root,
        ".gitignore",
        "node_modules/\n.claude/accelerator.local.md\n\
         meta/integrations/jira/.lock\nfoo.txt\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "About to apply 1 migration(s):\n\
         \x20 0003-relocate-accelerator-state — Relocate Accelerator-owned \
         files from .claude/ and meta/ to .accelerator/\n\
         \x20   To skip: accelerator migrate --skip \
         0003-relocate-accelerator-state\n\
         \n\
         Migrations rewrite files and may make repo-wide changes; commit\n\
         your working tree before running so VCS revert is available as\n\
         rollback. The pre-flight will refuse to run on a dirty tree\n\
         unless ACCELERATOR_MIGRATE_FORCE=1 is set.\n\
         \n\
         \n\
         Migration complete. applied: 1.\n"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "Warning: paths.templates is explicitly set to \
             'custom/templates'. The migration\n"
        ),
        "{stderr}"
    );

    assert_eq!(
        fs::read_to_string(root.join(".accelerator/config.md"))?,
        "---\npaths:\n  templates: custom/templates\n---\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/config.local.md"))?,
        "---\n---\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/skills/foo.md"))?,
        "team skill\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/lenses/bar.md"))?,
        "team lens\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/templates/t.md"))?,
        "tmpl\n"
    );
    assert_eq!(
        fs::read_to_string(
            root.join(".accelerator/state/integrations/jira/site.json")
        )?,
        "jira state\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/tmp/scratch.txt"))?,
        "tmpfile\n"
    );

    assert!(!root.join(".claude/accelerator.md").exists());
    assert!(!root.join(".claude/accelerator.local.md").exists());
    assert!(!root.join("meta/templates").exists());
    assert!(!root.join("meta/integrations/jira").exists());
    assert!(!root.join("meta/tmp").exists());
    assert!(!root.join("meta/.migrations-applied").exists());
    assert!(!root.join("meta/.migrations-skipped").exists());

    assert_eq!(
        fs::read_to_string(root.join(".accelerator/.gitignore"))?,
        "config.local.md\n.tmp-*\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".gitignore"))?,
        "node_modules/\nfoo.txt\n.accelerator/config.local.md\n"
    );
    assert_eq!(
        fs::read_to_string(
            root.join(".accelerator/state/integrations/jira/.gitignore")
        )?,
        "site.json\n.refresh-meta.json\n.lock/\n"
    );
    assert!(root
        .join(".accelerator/state/integrations/jira/.gitkeep")
        .exists());

    assert_eq!(
        fs::read_to_string(root.join(".accelerator/state/migrations-applied"))?,
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         a\nb\n\
         0003-relocate-accelerator-state\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/state/migrations-skipped"))?,
        "c\n"
    );
    Ok(())
}

#[test]
fn a_pinned_paths_tmp_leaves_meta_tmp_untouched_in_every_shape(
) -> Result<(), TestError> {
    for pinned_value in ["custom/tmp", "meta/tmp", "meta/tmp/"] {
        let dir = TempDir::new()?;
        let root = dir.path();
        write(
            root,
            ".claude/accelerator.md",
            &format!("---\npaths:\n  tmp: {pinned_value}\n---\n"),
        )?;
        write(root, "meta/tmp/scratch.txt", "tmpfile\n")?;
        already_applied(root)?;

        let output = Command::new(BIN).current_dir(root).output()?;

        assert_eq!(output.status.code(), Some(0), "{pinned_value}: {output:?}");
        assert_eq!(
            fs::read_to_string(root.join("meta/tmp/scratch.txt"))?,
            "tmpfile\n",
            "{pinned_value}"
        );
        assert!(!root.join(".accelerator/tmp").exists(), "{pinned_value}");
    }
    Ok(())
}

#[test]
fn a_tmp_key_under_an_unrelated_block_is_not_mistaken_for_an_override(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        ".claude/accelerator.md",
        "---\nsome_section:\n  tmp: meta/tmp\n---\n",
    )?;
    write(root, "meta/tmp/scratch.txt", "tmpfile\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!root.join("meta/tmp").exists());
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/tmp/scratch.txt"))?,
        "tmpfile\n"
    );
    Ok(())
}

#[test]
fn a_second_run_reports_no_pending_and_does_not_duplicate_the_gitignore_rule(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, ".claude/accelerator.md", "---\n---\n")?;
    write(
        root,
        ".gitignore",
        "node_modules/\n.claude/accelerator.local.md\n",
    )?;
    already_applied(root)?;

    Command::new(BIN).current_dir(root).output()?;
    write(
        root,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n",
    )?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "No pending migrations.\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".gitignore"))?
            .matches(".accelerator/config.local.md")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn a_manually_moved_config_with_skills_still_pending_completes_cleanly(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, ".accelerator/config.md", "---\n---\n")?;
    write(root, ".claude/accelerator/skills/foo.md", "team skill\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/skills/foo.md"))?,
        "team skill\n"
    );
    assert!(!root.join(".claude/accelerator/skills").exists());
    Ok(())
}

#[test]
fn every_source_manually_moved_still_merges_the_dangling_legacy_state_file(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, ".accelerator/config.md", "---\n---\n")?;
    write(root, "meta/.migrations-applied", "a\nb\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!root.join("meta/.migrations-applied").exists());
    let applied =
        fs::read_to_string(root.join(".accelerator/state/migrations-applied"))?;
    assert!(applied.contains("a\n"), "{applied}");
    assert!(applied.contains("b\n"), "{applied}");
    Ok(())
}

#[test]
fn a_pre_existing_destination_config_merges_with_source_winning(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, ".claude/accelerator.md", "---\nsource: yes\n---\n")?;
    write(root, ".accelerator/config.md", "---\ndest: yes\n---\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/config.md"))?,
        "---\nsource: yes\n---\n"
    );
    Ok(())
}

#[test]
fn meta_tmp_merges_recursively_into_a_pre_existing_accelerator_tmp(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, ".claude/accelerator.md", "---\n---\n")?;
    write(root, "meta/tmp/shared.txt", "SRC")?;
    write(root, "meta/tmp/only-in-source.txt", "new")?;
    write(root, ".accelerator/tmp/shared.txt", "DST")?;
    write(root, ".accelerator/tmp/only-in-dest.txt", "kept")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!root.join("meta/tmp").exists());
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/tmp/shared.txt"))?,
        "SRC"
    );
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/tmp/only-in-source.txt"))?,
        "new"
    );
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/tmp/only-in-dest.txt"))?,
        "kept"
    );
    Ok(())
}

#[test]
fn an_absent_meta_templates_never_spuriously_creates_the_destination(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, ".claude/accelerator.md", "---\n---\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!root.join(".accelerator/templates").exists());
    Ok(())
}

#[test]
fn the_pinned_override_warning_fires_for_both_keys_together(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        ".claude/accelerator.md",
        "---\npaths:\n  templates: custom/templates\n  \
         integrations: custom/ints\n---\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "Warning: paths.templates is explicitly set to \
             'custom/templates'."
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "Warning: paths.integrations is explicitly set to \
             'custom/ints'."
        ),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn no_pinned_override_warning_when_neither_key_is_pinned(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, ".claude/accelerator.md", "---\n---\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!stderr.contains("is explicitly set to"), "{stderr}");
    Ok(())
}

#[test]
fn refuses_a_legacy_gitignore_line_with_trailing_content(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, ".claude/accelerator.md", "---\n---\n")?;
    write(
        root,
        ".gitignore",
        ".claude/accelerator.local.md # keep this note\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("refusing to rewrite")
            && stderr.contains(".claude/accelerator.local.md # keep this note"),
        "{stderr}"
    );
    assert!(root.join(".claude/accelerator.md").exists());
    Ok(())
}

#[test]
fn merges_legacy_and_existing_state_files_deduplicating_first_seen(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, ".claude/accelerator.md", "---\n---\n")?;
    write(
        root,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         x\ny\n",
    )?;
    write(root, "meta/.migrations-applied", "y\nz\n")?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/state/migrations-applied"))?,
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         x\ny\nz\n\
         0003-relocate-accelerator-state\n"
    );
    Ok(())
}
