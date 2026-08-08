//! Migration 0007 (`unify-meta-corpus-frontmatter`) driven end to end
//! against the compiled binary — the mechanical path only (a
//! resolved-band body reference applying without a prompt, and an
//! unresolvable resolved-band reference silently dropped). The ambiguous
//! prompt path is exercised generically against `FixtureMigration` in
//! `cli/migrate/tests/engine.rs`; `--list`/`--decisions-file` are not yet
//! wired (Phase 5's own deferred scope — see this phase's plan
//! deviations), so a real ambiguous-band scenario against the compiled
//! binary is not driven here.

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

fn work_item(id: &str, title: &str, body: &str) -> String {
    format!(
        "---\ntype: work-item\nid: \"{id}\"\ntitle: {title}\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
         kind: task\nstatus: draft\npriority: medium\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
         schema_version: 1\n---\n\n# {id}: {title}\n{body}"
    )
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
         0006-canonicalise-work-item-id-and-author\n",
    )
}

#[test]
fn a_resolved_band_reference_applies_mechanically_and_an_unresolvable_one_is_dropped(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/work/0042-target.md",
        &work_item("0042", "Target", ""),
    )?;
    write(
        root,
        "meta/work/0001-source.md",
        &work_item(
            "0001",
            "Source",
            "\n## Dependencies\n\n- Blocks: 0042\n- Blocks: 0099\n",
        ),
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join(".accelerator/state/migrations-applied"))?,
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n"
    );

    let source = fs::read_to_string(root.join("meta/work/0001-source.md"))?;
    assert!(
        source.contains("blocks: [\"work-item:0042\"]\n"),
        "expected a merged blocks: line in {source}"
    );
    assert!(
        !source.contains("work-item:0099"),
        "an unresolvable resolved-band target must not be written: {source}"
    );

    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "0007-DIVERGE[reverse-orphan]: meta/work/0001-source.md — \
             resolved blocks target 'work-item:0099' resolves to no \
             artifact; skipped"
        ),
        "{stderr}"
    );

    // The target file itself is untouched — only the referencing file's
    // frontmatter gains the linkage.
    assert_eq!(
        fs::read_to_string(root.join("meta/work/0042-target.md"))?,
        work_item("0042", "Target", "")
    );
    Ok(())
}

#[test]
fn a_work_item_missing_kind_refuses_with_zero_mutations(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    let no_kind = "---\ntype: work-item\nid: \"0042\"\ntitle: t\n\
        date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
        status: draft\npriority: medium\n\
        last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
        schema_version: 1\n---\n\nbody\n";
    write(root, "meta/work/0042-foo.md", no_kind)?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(root.join("meta/work/0042-foo.md"))?,
        no_kind
    );
    assert!(
        !root.join(".accelerator/state/migrations-applied").exists()
            || !fs::read_to_string(
                root.join(".accelerator/state/migrations-applied")
            )?
            .contains("0007")
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains(
        "0007-REFUSE: meta/work/0042-foo.md — work-item missing kind:"
    ));
    Ok(())
}
