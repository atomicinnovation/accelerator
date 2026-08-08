//! Migration 0007 (`unify-meta-corpus-frontmatter`) driven end to end
//! against the compiled binary — the mechanical path only (a
//! resolved-band body reference applying without a prompt, and an
//! unresolvable resolved-band reference silently dropped). The ambiguous
//! prompt path is exercised generically against `FixtureMigration` in
//! `cli/migrate/tests/engine.rs`; `--list`/`--decisions-file` are not yet
//! wired (Phase 5's own deferred scope — see this phase's plan
//! deviations), so a real ambiguous-band scenario against the compiled
//! binary is not driven here.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

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

fn copy_tree(src: &Path, dst: &Path) -> Result<(), TestError> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target)?;
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn read_tree(root: &Path) -> Result<BTreeMap<String, String>, TestError> {
    fn walk(
        dir: &Path,
        root: &Path,
        out: &mut BTreeMap<String, String>,
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
    let mut out = BTreeMap::new();
    walk(root, root, &mut out)?;
    Ok(out)
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
fn an_unsupported_session_log_schema_version_refuses_naming_the_discard_command(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    already_applied(root)?;
    let session_log_path =
        root.join(".accelerator/state/migrations-0007-unify-meta-corpus-frontmatter-session.jsonl");
    write(
        root,
        session_log_path
            .strip_prefix(root)?
            .to_str()
            .ok_or("path")?,
        "{\"transformation_key\":\"k1\",\"schema_version\":99,\
         \"outcome\":\"accepted\",\"proposed_value\":\"v1\",\
         \"timestamp\":\"2026-01-01T00:00:00+00:00\"}\n",
    )?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("[resume] unknown schema_version 99 — supported: {1}."),
        "{stderr}"
    );
    assert!(stderr.contains("[resume]   rm "), "{stderr}");
    assert!(
        !fs::read_to_string(
            root.join(".accelerator/state/migrations-applied")
        )?
        .contains("0007"),
        "a refused resume must not add 0007 to the applied ledger"
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

#[test]
fn a_multi_type_corpus_rewrites_cleanly_with_no_refusals(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/prs/240-description.md",
        "---\ntype:\nid: \"240-description\"\n\
         title: \"PR 240 Description\"\n\
         date: \"2026-06-01T00:00:00+00:00\"\nauthor: Toby\n\
         status: complete\nrelates_to: [\"PR #416\"]\npr_number: 240\n\
         tags: []\nrevision: \"abc123\"\nrepository: \"accelerator\"\n\
         last_updated: \"2026-06-01T00:00:00+00:00\"\n\
         last_updated_by: Toby\nschema_version: 1\n---\n\
         # PR 240 Description\n",
    )?;
    write(
        root,
        "meta/notes/2026-06-20-ticketed.md",
        "---\ntype: note\nid: \"2026-06-20-ticketed\"\n\
         title: \"Ticketed; with semicolon\"\n\
         date: \"2026-06-20T00:00:00+00:00\"\nauthor: Toby\n\
         producer: create-note\nstatus: captured\n\
         ticket: \"PROJ-1234\"\ntags: []\nrevision: \"abc123\"\n\
         repository: \"accelerator\"\n\
         last_updated: \"2026-06-20T00:00:00+00:00\"\n\
         last_updated_by: Toby\nschema_version: 1\n---\n\
         # Ticketed; with semicolon\n",
    )?;
    write(
        root,
        "meta/reviews/prs/2026-06-17-pr-430-review.md",
        "---\ntype: pr-review\nid: \"2026-06-17-pr-430-review\"\n\
         title: \"PR 430 Review\"\ndate: \"2026-06-17T00:00:00+00:00\"\n\
         author: Toby\nstatus: complete\ntags: []\n\
         last_updated: \"2026-06-17T00:00:00+00:00\"\n\
         last_updated_by: Toby\nschema_version: 1\n---\n\
         # PR 430 Review\n",
    )?;
    write(
        root,
        "meta/docs/logging-guide.md",
        "---\ntitle: Logging Guide\nfoo: bar\n---\n\n# Logging Guide\n\n\
         Freeform documentation the plugin does not own.\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!stderr.contains("0007-REFUSE"), "{stderr}");

    let prs = fs::read_to_string(root.join("meta/prs/240-description.md"))?;
    assert!(prs.contains("type: pr-description\n"), "{prs}");
    assert!(prs.contains("relates_to: [\"pr:416\"]\n"), "{prs}");

    let notes =
        fs::read_to_string(root.join("meta/notes/2026-06-20-ticketed.md"))?;
    assert!(
        notes.contains("topic: \"Ticketed; with semicolon\"\n"),
        "{notes}"
    );
    assert!(!notes.contains("ticket:"), "{notes}");

    let review = fs::read_to_string(
        root.join("meta/reviews/prs/2026-06-17-pr-430-review.md"),
    )?;
    assert!(review.contains("pr_number: 430\n"), "{review}");

    // meta/docs/ isn't a configured corpus directory — left byte-unchanged.
    assert_eq!(
        fs::read_to_string(root.join("meta/docs/logging-guide.md"))?,
        "---\ntitle: Logging Guide\nfoo: bar\n---\n\n# Logging Guide\n\n\
         Freeform documentation the plugin does not own.\n"
    );
    Ok(())
}

#[test]
fn an_unconfigured_subtree_is_left_byte_unchanged() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    let arbitrary = "---\nsomething: else\n---\n\n# Not ours\n";
    write(root, "meta/arbitrary/thing.md", arbitrary)?;
    write(
        root,
        "meta/work/0042-foo.md",
        "---\ntype: work-item\nid: \"0042\"\ntitle: t\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
         kind: task\nstatus: draft\npriority: medium\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
         schema_version: 1\n---\n\nbody\n",
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("meta/arbitrary/thing.md"))?,
        arbitrary
    );
    Ok(())
}

#[test]
fn an_ambiguous_band_prompt_stalls_with_no_decision_input_available(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();

    write(
        root,
        "meta/work/0001-source.md",
        &work_item("0001", "Source", "\n## Dependencies\n\n- Related: 0042\n"),
    )?;
    already_applied(root)?;

    let output = Command::new(BIN)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    let decisions_path = fs::canonicalize(root)?.join(
        ".accelerator/state/migrations-0007-unify-meta-corpus-frontmatter-decisions.txt",
    );
    let decisions_path = decisions_path.display();
    assert!(
        stderr.contains(
            "[0007-unify-meta-corpus-frontmatter] MIGRATION STALLED: no \
             decision input available"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "[0007-unify-meta-corpus-frontmatter]   pending decision: \
             meta/work/0001-source.md#body:dependencies#0"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "accelerator migrate --decisions-file {decisions_path}"
        )),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "ACCELERATOR_MIGRATE_DECISIONS_FILE={decisions_path} \
             accelerator migrate"
        )),
        "{stderr}"
    );
    assert!(
        !root.join(".accelerator/state/migrations-applied").exists()
            || !fs::read_to_string(
                root.join(".accelerator/state/migrations-applied")
            )?
            .contains("0007"),
        "a stalled run must not record 0007 as applied"
    );
    Ok(())
}

#[test]
fn matches_the_checked_in_byte_equivalence_golden() -> Result<(), TestError> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../skills/config/migrate/scripts/test-fixtures/migrate-byte-equiv",
    );
    let dir = TempDir::new()?;
    let root = dir.path();

    copy_tree(&fixture.join("input"), root)?;
    already_applied(root)?;

    let output = Command::new(BIN).current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let actual = read_tree(&root.join("meta"))?;
    let expected = read_tree(&fixture.join("golden/meta"))?;
    assert_eq!(actual, expected);
    Ok(())
}
