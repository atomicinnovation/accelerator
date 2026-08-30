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
         0005-rename-work-item-type-to-kind\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n",
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
fn a_malformed_config_file_aborts_rather_than_silently_skipping(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write(
        dir.path(),
        ".claude/accelerator.md",
        "---\npaths:\n  plans: [unterminated\n---\n",
    )?;
    already_applied(dir.path())?;

    let output = Command::new(BIN).current_dir(dir.path()).output()?;

    assert_ne!(
        output.status.code(),
        Some(0),
        "stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !fs::read_to_string(
            dir.path().join(".accelerator/state/migrations-applied")
        )?
        .contains("0006"),
        "a config-read failure must not record 0006 as applied"
    );
    Ok(())
}

#[test]
fn each_configured_corpus_key_is_walked_at_its_override_path(
) -> Result<(), TestError> {
    for (key, expected_default) in [
        ("plans", "meta/plans"),
        ("research_codebase", "meta/research/codebase"),
        ("research_issues", "meta/research/issues"),
    ] {
        let dir = TempDir::new()?;
        let root = dir.path();
        write(
            root,
            ".claude/accelerator.md",
            &format!("---\npaths:\n  {key}: docs/{key}\n---\n"),
        )?;
        write(
            root,
            &format!("docs/{key}/a.md"),
            "---\nwork-item: 0042\n---\n",
        )?;
        already_applied(root)?;

        let output = Command::new(BIN).current_dir(root).output()?;

        assert_eq!(output.status.code(), Some(0), "{key}: {output:?}");
        let stderr = String::from_utf8(output.stderr)?;
        assert!(
            stderr
                .contains(&format!("0006: rewrote 1 file(s) under docs/{key}")),
            "{key}: {stderr}"
        );
        assert!(!root.join(expected_default).exists(), "{key}");
        assert_eq!(
            fs::read_to_string(root.join(format!("docs/{key}/a.md")))?,
            "---\nwork_item_id: \"0042\"\n---\n",
            "{key}"
        );
    }
    Ok(())
}

#[test]
fn a_missing_configured_corpus_path_names_itself_not_the_default(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        ".claude/accelerator.md",
        "---\npaths:\n  plans: docs/typo-plans\n---\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "Warning: 0006: plans directory does not exist: docs/typo-plans"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains("0006: rewrote 0 file(s) under docs/typo-plans"),
        "{stderr}"
    );
    assert!(
        fs::read_to_string(root.join(".accelerator/state/migrations-applied"))?
            .contains("0006"),
        "0006 must still be recorded applied"
    );
    Ok(())
}

#[test]
fn two_corpus_keys_aliased_to_the_same_path_are_walked_exactly_once(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        ".claude/accelerator.md",
        "---\npaths:\n  research_codebase: docs/shared\n  \
         research_issues: docs/shared\n---\n",
    )?;
    write(root, "docs/shared/a.md", "---\nresearcher: alice\n---\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "Warning: 0006: paths.research_issues aliases \
             paths.research_codebase"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "0006: skipping duplicate walk for paths.research_issues"
        ),
        "{stderr}"
    );
    assert_eq!(
        stderr
            .matches("0006: rewrote 1 file(s) under docs/shared")
            .count(),
        1,
        "expected exactly one rewrite pass over docs/shared: {stderr}"
    );
    assert_eq!(
        fs::read_to_string(root.join("docs/shared/a.md"))?,
        "---\nauthor: alice\n---\n"
    );
    Ok(())
}

#[test]
fn a_tier_2_userspace_template_is_rewritten_via_paths_templates(
) -> Result<(), TestError> {
    for name in ["plan", "codebase-research", "rca"] {
        let dir = TempDir::new()?;
        let root = dir.path();
        write(
            root,
            ".claude/accelerator.md",
            "---\npaths:\n  templates: custom/templates\n---\n",
        )?;
        write(
            root,
            &format!("custom/templates/{name}.md"),
            "---\nwork-item: 0042\n---\n",
        )?;
        already_applied(root)?;

        let output = Command::new(BIN).current_dir(root).output()?;

        assert_eq!(output.status.code(), Some(0), "{name}: {output:?}");
        assert_eq!(
            fs::read_to_string(
                root.join(format!("custom/templates/{name}.md"))
            )?,
            "---\nwork_item_id: \"0042\"\n---\n",
            "{name}"
        );
    }
    Ok(())
}

#[test]
fn a_tier_1_explicit_template_path_is_rewritten() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        ".claude/accelerator.md",
        "---\ntemplates:\n  plan: custom/plan-template.md\n---\n",
    )?;
    write(
        root,
        "custom/plan-template.md",
        "---\nwork-item: 0042\n---\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("custom/plan-template.md"))?,
        "---\nwork_item_id: \"0042\"\n---\n"
    );
    Ok(())
}

#[test]
fn tier_1_takes_precedence_over_tier_2_when_both_are_present(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        ".claude/accelerator.md",
        "---\npaths:\n  templates: custom/tier2\ntemplates:\n  plan: \
         custom/tier1-plan.md\n---\n",
    )?;
    write(root, "custom/tier1-plan.md", "---\nwork-item: 0042\n---\n")?;
    write(root, "custom/tier2/plan.md", "---\nwork-item: 0099\n---\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("custom/tier1-plan.md"))?,
        "---\nwork_item_id: \"0042\"\n---\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("custom/tier2/plan.md"))?,
        "---\nwork-item: 0099\n---\n",
        "tier-2 must not be touched when tier-1 resolves"
    );
    Ok(())
}

#[test]
fn a_missing_tier_1_file_warns_and_does_not_fall_through_to_tier_2(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        ".claude/accelerator.md",
        "---\npaths:\n  templates: custom/tier2\ntemplates:\n  plan: \
         custom/missing.md\n---\n",
    )?;
    write(root, "custom/tier2/plan.md", "---\nwork-item: 0099\n---\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "Warning: 0006: templates.plan points at missing file: \
             custom/missing.md"
        ),
        "{stderr}"
    );
    assert_eq!(
        fs::read_to_string(root.join("custom/tier2/plan.md"))?,
        "---\nwork-item: 0099\n---\n",
        "no fallthrough to tier-2 once tier-1 is explicitly configured"
    );
    Ok(())
}

#[test]
fn two_template_names_resolving_to_the_same_file_rewrite_once(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        ".claude/accelerator.md",
        "---\ntemplates:\n  plan: custom/shared.md\n  codebase-research: \
         custom/shared.md\n---\n",
    )?;
    write(root, "custom/shared.md", "---\nwork-item: 0042\n---\n")?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("resolve to the same file")
            && stderr.contains("skipping duplicate rewrite"),
        "{stderr}"
    );
    assert_eq!(
        stderr.matches("0006: template plan").count()
            + stderr.matches("0006: template codebase-research").count(),
        1,
        "expected exactly one template rewrite: {stderr}"
    );
    Ok(())
}

#[test]
fn a_clean_rewrite_is_byte_stable_across_three_consecutive_runs(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/plans/a.md",
        "---\nwork-item: 0042\ntitle: A\n---\n\n# Plan\n",
    )?;
    already_applied(root)?;

    Command::new(BIN).current_dir(root).output()?;
    let after_first = fs::read_to_string(root.join("meta/plans/a.md"))?;

    for _ in 0..2 {
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
            fs::read_to_string(root.join("meta/plans/a.md"))?,
            after_first
        );
    }
    Ok(())
}

#[test]
fn a_refused_line_is_stable_across_repeated_runs() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/plans/a.md",
        "---\nwork-item: 0042 # note\n---\n",
    )?;
    write(
        root,
        "meta/plans/b.md",
        "---\nwork-item: has \"embedded\" quote\n---\n",
    )?;
    already_applied(root)?;

    Command::new(BIN).current_dir(root).output()?;
    let after_first_a = fs::read_to_string(root.join("meta/plans/a.md"))?;
    let after_first_b = fs::read_to_string(root.join("meta/plans/b.md"))?;

    for _ in 0..2 {
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
            fs::read_to_string(root.join("meta/plans/a.md"))?,
            after_first_a
        );
        assert_eq!(
            fs::read_to_string(root.join("meta/plans/b.md"))?,
            after_first_b
        );
    }
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
