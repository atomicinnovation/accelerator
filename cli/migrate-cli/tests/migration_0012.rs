//! Migration 0012 (`backfill-work-item-tags`) driven end to end against the
//! compiled binary, with every other migration pre-seeded in the ledger so
//! 0012 is the sole pending migration.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;

use corpus::frontmatter_validation::pipeline::validate_text;
use corpus_adapters::YamlFrontmatter;
use store::test_support::ReadOnlyDir;
use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

const MIGRATION: &str = "0012-backfill-work-item-tags";

const UNTAGGED: &str = "---\n\
    type: \"work-item\"\n\
    id: \"0001\"\n\
    title: \"Untagged\"\n\
    date: \"2026-01-01T00:00:00+00:00\"\n\
    author: \"Toby\"\n\
    producer: \"create-work-item\"\n\
    status: \"draft\"\n\
    kind: \"story\"\n\
    priority: \"medium\"\n\
    last_updated: \"2026-01-01T00:00:00+00:00\"\n\
    last_updated_by: \"Toby\"\n\
    schema_version: 1\n\
    ---\n\
    \n\
    # 0001: Untagged\n";

const UNTAGGED_REPAIRED: &str = "---\n\
    type: \"work-item\"\n\
    id: \"0001\"\n\
    title: \"Untagged\"\n\
    date: \"2026-01-01T00:00:00+00:00\"\n\
    author: \"Toby\"\n\
    producer: \"create-work-item\"\n\
    status: \"draft\"\n\
    kind: \"story\"\n\
    priority: \"medium\"\n\
    tags: []\n\
    last_updated: \"2026-01-01T00:00:00+00:00\"\n\
    last_updated_by: \"Toby\"\n\
    schema_version: 1\n\
    ---\n\
    \n\
    # 0001: Untagged\n";

const UNTAGGED_DRAFT: &str = "---\n\
    type: \"work-item\"\n\
    id: \"draft-a1b2c3\"\n\
    title: \"Draft\"\n\
    last_updated: \"2026-01-01T00:00:00+00:00\"\n\
    ---\n\
    \n\
    # Draft\n";

const UNTAGGED_DRAFT_REPAIRED: &str = "---\n\
    type: \"work-item\"\n\
    id: \"draft-a1b2c3\"\n\
    title: \"Draft\"\n\
    tags: []\n\
    last_updated: \"2026-01-01T00:00:00+00:00\"\n\
    ---\n\
    \n\
    # Draft\n";

fn write(dir: &Path, relative: &str, content: &str) -> Result<(), TestError> {
    write_bytes(dir, relative, content.as_bytes())
}

fn write_bytes(
    dir: &Path,
    relative: &str,
    content: &[u8],
) -> Result<(), TestError> {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

fn read(dir: &Path, relative: &str) -> Result<String, TestError> {
    Ok(fs::read_to_string(dir.join(relative))?)
}

fn already_applied_except_0012(dir: &Path) -> Result<(), TestError> {
    write(
        dir,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n\
         0009-split-work-key-from-tracker-scope-key\n\
         0010-strip-research-title-prefix\n\
         0011-ignore-machine-local-sync-state\n",
    )
}

fn run(dir: &Path) -> std::io::Result<Output> {
    Command::new(BIN).current_dir(dir).output()
}

fn succeeded(dir: &Path) -> Result<String, TestError> {
    let output = run(dir)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    Ok(String::from_utf8(output.stderr)?)
}

fn applied(dir: &Path) -> Result<String, TestError> {
    read(dir, ".accelerator/state/migrations-applied")
}

fn baseline_local_hash(dir: &Path, id: &str) -> Option<String> {
    let content = fs::read_to_string(
        dir.join(".accelerator/state/integrations/linear/last-sync.json"),
    )
    .ok()?;
    let (baseline, _) =
        work_adapters::sync::baseline::Baseline::read(Some(&content));
    baseline.get(id).map(|entry| {
        entry
            .legacy()
            .and_then(|hashes| hashes.local.clone())
            .unwrap_or_default()
    })
}

fn seed_baseline(
    dir: &Path,
    id: &str,
    local_hash: &str,
) -> Result<(), TestError> {
    write(
        dir,
        ".accelerator/state/integrations/linear/last-sync.json",
        &format!(
            "{{\"timestamp\":1,\"items\":{{\"{id}\":{{\
             \"remote_updated_at\":\"2026-06-01T00:00:00Z\",\
             \"remote_hash\":\"rh\",\"local_hash\":\"{local_hash}\"}}}}}}\n"
        ),
    )
}

#[test]
fn backfills_an_item_missing_tags() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, "meta/work/0001-untagged.md", UNTAGGED)?;
    write(
        root,
        "meta/work/drafts/draft-a1b2c3-draft.md",
        UNTAGGED_DRAFT,
    )?;
    already_applied_except_0012(root)?;

    let stderr = succeeded(root)?;

    let repaired = read(root, "meta/work/0001-untagged.md")?;
    assert_eq!(repaired, UNTAGGED_REPAIRED);
    assert_eq!(validate_text(&repaired, &YamlFrontmatter), Vec::new());
    assert_eq!(
        read(root, "meta/work/drafts/draft-a1b2c3-draft.md")?,
        UNTAGGED_DRAFT_REPAIRED
    );
    assert!(applied(root)?.contains(MIGRATION));
    assert!(
        stderr.contains(
            "0012: added tags to 2 work item(s) under meta/work, 0 sync \
             baseline(s) realigned"
        ),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn backfills_a_crlf_item_through_the_real_yaml_guard() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/work/0001-crlf.md",
        "---\r\ntype: \"work-item\"\r\nid: \"0001\"\r\ntitle: \"Crlf\"\r\n\
         last_updated: \"2026-01-01T00:00:00+00:00\"\r\n---\r\n\r\n\
         # Crlf\r\n",
    )?;
    already_applied_except_0012(root)?;

    succeeded(root)?;

    assert_eq!(
        read(root, "meta/work/0001-crlf.md")?,
        "---\r\ntype: \"work-item\"\r\nid: \"0001\"\r\ntitle: \"Crlf\"\r\n\
         tags: []\r\nlast_updated: \"2026-01-01T00:00:00+00:00\"\r\n---\r\n\
         \r\n# Crlf\r\n"
    );
    Ok(())
}

#[test]
fn warns_when_the_work_directory_is_absent() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    already_applied_except_0012(root)?;

    let stderr = succeeded(root)?;

    assert!(
        stderr.contains("Warning: 0012: no work directory at meta/work"),
        "{stderr}"
    );
    assert!(
        stderr.contains("0012: added tags to 0 work item(s) under meta/work"),
        "{stderr}"
    );
    assert!(applied(root)?.contains(MIGRATION));
    Ok(())
}

#[test]
fn applies_when_the_work_path_is_a_file() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, "meta/work", "not a directory\n")?;
    already_applied_except_0012(root)?;

    let stderr = succeeded(root)?;

    assert!(
        stderr.contains("Warning: 0012: no work directory at meta/work"),
        "{stderr}"
    );
    assert_eq!(read(root, "meta/work")?, "not a directory\n");
    assert!(applied(root)?.contains(MIGRATION));
    Ok(())
}

#[test]
fn backfills_an_item_without_last_updated_at_the_end_of_its_frontmatter(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/work/0001-legacy.md",
        "---\ntype: \"work-item\"\nid: \"0001\"\ntitle: \"Legacy\"\n\
         status: \"draft\"\n---\n\n# Legacy\n",
    )?;
    already_applied_except_0012(root)?;

    succeeded(root)?;

    assert_eq!(
        read(root, "meta/work/0001-legacy.md")?,
        "---\ntype: \"work-item\"\nid: \"0001\"\ntitle: \"Legacy\"\n\
         status: \"draft\"\ntags: []\n---\n\n# Legacy\n"
    );
    Ok(())
}

#[test]
fn leaves_tagged_items_untouched_and_ignores_body_tags() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    let root = dir.path();
    let flow = "---\ntype: \"work-item\"\nid: \"0001\"\ntags: [\"api\"]\n\
                last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n# Flow\n";
    let block = "---\ntype: \"work-item\"\nid: \"0002\"\ntags:\n  - api\n\
                 last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n# Block\n";
    write(root, "meta/work/0001-flow.md", flow)?;
    write(root, "meta/work/0002-block.md", block)?;
    write(
        root,
        "meta/work/0003-body.md",
        "---\ntype: \"work-item\"\nid: \"0003\"\n\
         last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\ntags: body\n",
    )?;
    already_applied_except_0012(root)?;

    succeeded(root)?;

    assert_eq!(read(root, "meta/work/0001-flow.md")?, flow);
    assert_eq!(read(root, "meta/work/0002-block.md")?, block);
    assert_eq!(
        read(root, "meta/work/0003-body.md")?,
        "---\ntype: \"work-item\"\nid: \"0003\"\ntags: []\n\
         last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\ntags: body\n"
    );
    Ok(())
}

#[test]
fn a_backfilled_work_directory_is_unchanged_on_a_second_run(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, "meta/work/0001-untagged.md", UNTAGGED)?;
    already_applied_except_0012(root)?;

    succeeded(root)?;
    let after_first = read(root, "meta/work/0001-untagged.md")?;
    assert_eq!(after_first, UNTAGGED_REPAIRED);
    assert!(applied(root)?.contains(MIGRATION));

    let second = run(root)?;
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    assert!(
        String::from_utf8(second.stdout)?.contains("No pending migrations."),
        "the second run must be a no-op"
    );
    assert_eq!(read(root, "meta/work/0001-untagged.md")?, after_first);
    Ok(())
}

#[test]
fn a_backfilled_work_directory_is_byte_stable_on_re_application(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, "meta/work/0001-untagged.md", UNTAGGED)?;
    already_applied_except_0012(root)?;

    succeeded(root)?;
    let after_first = read(root, "meta/work/0001-untagged.md")?;
    assert_eq!(after_first, UNTAGGED_REPAIRED);
    assert!(applied(root)?.contains(MIGRATION));

    already_applied_except_0012(root)?;
    succeeded(root)?;

    assert_eq!(read(root, "meta/work/0001-untagged.md")?, after_first);
    Ok(())
}

#[test]
fn a_synced_item_s_baseline_is_realigned() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    let synced = "---\ntype: \"work-item\"\nid: \"0001\"\ntitle: \"Synced\"\n\
                  external_id: \"ENG-1\"\n\
                  last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n\
                  # Synced\n";
    write(root, "meta/work/0001-synced.md", synced)?;
    already_applied_except_0012(root)?;
    let before = work_adapters::sync::digest::local(synced)?;
    seed_baseline(root, "0001", &before)?;

    let stderr = succeeded(root)?;

    let after = work_adapters::sync::digest::local(&read(
        root,
        "meta/work/0001-synced.md",
    )?)?;
    assert_ne!(after, before);
    assert_eq!(baseline_local_hash(root, "0001"), Some(after));
    assert!(
        stderr.contains(
            "0012: added tags to 1 work item(s) under meta/work, 1 sync \
             baseline(s) realigned"
        ),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn names_and_leaves_alone_an_item_that_is_not_utf8() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    let bytes = b"---\ntype: \"work-item\"\nid: \"0001\"\n\
                  last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n\
                  # Caf\xFF\n";
    write_bytes(root, "meta/work/0001-latin1.md", bytes)?;
    already_applied_except_0012(root)?;

    let stderr = succeeded(root)?;

    assert_eq!(fs::read(root.join("meta/work/0001-latin1.md"))?, bytes);
    assert!(
        stderr.contains("\n  meta/work/0001-latin1.md: not UTF-8"),
        "{stderr}"
    );
    assert!(applied(root)?.contains(MIGRATION));
    Ok(())
}

#[test]
fn names_and_leaves_alone_an_item_with_a_quoted_tags_key(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    let quoted =
        "---\ntype: \"work-item\"\nid: \"0001\"\n\"tags\": [\"api\"]\n\
                  last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n\
                  # Quoted\n";
    write(root, "meta/work/0001-quoted.md", quoted)?;
    already_applied_except_0012(root)?;

    let stderr = succeeded(root)?;

    assert_eq!(read(root, "meta/work/0001-quoted.md")?, quoted);
    assert!(
        stderr.contains(
            "\n  meta/work/0001-quoted.md: its tags key is spelled \
             non-canonically; rewrite the key as `tags:`"
        ),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn names_and_leaves_alone_an_item_whose_frontmatter_does_not_parse(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    let broken = "---\ntype: \"work-item\"\nid: \"0001\"\n\
                  title: \"A title that\ncontinues: unindented\"\n\
                  last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n\
                  # Broken\n";
    write(root, "meta/work/0001-broken.md", broken)?;
    already_applied_except_0012(root)?;

    let stderr = succeeded(root)?;

    assert_eq!(read(root, "meta/work/0001-broken.md")?, broken);
    assert!(
        stderr.contains(
            "\n  meta/work/0001-broken.md: frontmatter does not parse as YAML ("
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains("fix the frontmatter, then add `tags: []`"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_failed_write_leaves_the_work_directory_as_it_found_it(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    let first = "---\ntype: \"work-item\"\nid: \"0001\"\n\
                 last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n# A\n";
    let second = "---\ntype: \"work-item\"\nid: \"0002\"\n\
                  last_updated: \"2026-01-01T00:00:00+00:00\"\n---\n\n# B\n";
    write(root, "meta/work/0001-a.md", first)?;
    write(root, "meta/work/locked/0002-b.md", second)?;
    already_applied_except_0012(root)?;

    {
        let Some(_sealed) = ReadOnlyDir::new(&root.join("meta/work/locked"))?
        else {
            return Ok(());
        };

        let output = run(root)?;

        assert_ne!(output.status.code(), Some(0), "{output:?}");
        assert_eq!(read(root, "meta/work/0001-a.md")?, first);
        assert_eq!(read(root, "meta/work/locked/0002-b.md")?, second);
        let stderr = String::from_utf8(output.stderr)?;
        assert!(stderr.contains("Left the 2 work item(s)"), "{stderr}");
        assert!(!applied(root)?.contains(MIGRATION));
    }

    succeeded(root)?;
    assert!(read(root, "meta/work/0001-a.md")?.contains("tags: []\n"));
    assert!(read(root, "meta/work/locked/0002-b.md")?.contains("tags: []\n"));
    Ok(())
}
