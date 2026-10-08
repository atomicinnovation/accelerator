//! Black-box coverage for migration 0008 (canonical frontmatter quoting),
//! isolated by pre-marking every other real migration applied.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

fn write(dir: &Path, relative: &str, content: &str) -> Result<(), TestError> {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

/// A work item with bare title/author/status/kind/priority — the re-render's
/// input, which the canonical emitter must quote.
fn bare_work_item(id: &str, title: &str) -> String {
    bare_work_item_with(id, title, "")
}

fn bare_work_item_with(id: &str, title: &str, extra: &str) -> String {
    format!(
        "---\ntype: work-item\nid: \"{id}\"\ntitle: {title}\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
         kind: task\nstatus: draft\npriority: medium\n{extra}\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
         schema_version: 1\n---\n\n# {id}: {title}\nBody\n"
    )
}

fn already_applied_except_0008(dir: &Path) -> Result<(), TestError> {
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
         0010-strip-research-title-prefix\n",
    )
}

fn run(dir: &Path) -> std::io::Result<std::process::Output> {
    Command::new(BIN).current_dir(dir).output()
}

fn baseline_local_hash(dir: &Path, id: &str) -> Option<String> {
    let content = fs::read_to_string(
        dir.join(".accelerator/state/integrations/linear/last-sync.json"),
    )
    .ok()?;
    let (baseline, _) =
        work_adapters::sync::baseline::Baseline::read(Some(&content));
    baseline.get(id).map(|entry| entry.local_hash.clone())
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
fn bare_frontmatter_is_canonicalised() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/work/0001-x.md",
        &bare_work_item("0001", "Bare Title"),
    )?;
    already_applied_except_0008(root)?;

    let output = run(root)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    let content = fs::read_to_string(root.join("meta/work/0001-x.md"))?;
    assert!(content.contains("title: \"Bare Title\""), "{content}");
    assert!(content.contains("status: \"draft\""), "{content}");
    assert!(content.contains("schema_version: 1"), "{content}");
    Ok(())
}

#[test]
fn a_second_run_reports_no_pending() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(root, "meta/work/0001-x.md", &bare_work_item("0001", "Bare"))?;
    already_applied_except_0008(root)?;

    run(root)?;
    let second = run(root)?;
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    assert!(
        String::from_utf8(second.stdout)?.contains("No pending migrations."),
        "the second run must be a no-op"
    );
    Ok(())
}

#[test]
fn a_pre_migration_synced_baseline_is_realigned_not_flagged(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    let file = bare_work_item("0001", "Synced");
    write(root, "meta/work/0001-x.md", &file)?;
    already_applied_except_0008(root)?;
    let before_hash = work_adapters::sync::digest::local(&file)?;
    seed_baseline(root, "0001", &before_hash)?;

    let output = run(root)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    let canonical = fs::read_to_string(root.join("meta/work/0001-x.md"))?;
    let after_hash = work_adapters::sync::digest::local(&canonical)?;
    assert_ne!(before_hash, after_hash, "re-render must change the digest");
    assert_eq!(
        baseline_local_hash(root, "0001").as_deref(),
        Some(after_hash.as_str()),
        "a synced item's baseline must advance to the re-rendered digest"
    );
    Ok(())
}

#[test]
fn a_pre_migration_modified_baseline_is_left_flagged() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/work/0001-x.md",
        &bare_work_item("0001", "Modified"),
    )?;
    already_applied_except_0008(root)?;
    let stale = "0".repeat(64);
    seed_baseline(root, "0001", &stale)?;

    let output = run(root)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    assert_eq!(
        baseline_local_hash(root, "0001").as_deref(),
        Some(stale.as_str()),
        "a locally-modified item's baseline must be preserved so its \
         pending push survives"
    );
    Ok(())
}

fn only_0008_pending(dir: &Path) -> Result<(), TestError> {
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
         0009-split-work-key-from-tracker-scope-key\n\
         0010-strip-research-title-prefix\n",
    )
}

fn run_quietly(dir: &Path) -> std::io::Result<std::process::Output> {
    Command::new(BIN)
        .current_dir(dir)
        .env("ACCELERATOR_LOG", "off")
        .output()
}

const RE_RENDER_FAILURE: &str = "re-rendered frontmatter still violates the \
     standard: INVALID-TYPE — type: '<absent>' is not a schema type — revert \
     this migration commit to recover";

#[test]
fn a_baseline_under_absolute_integrations_outside_the_repo_is_realigned(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path().canonicalize()?;
    let outside = TempDir::new()?;
    let integrations = outside.path().canonicalize()?;
    let synced =
        bare_work_item_with("0001", "Synced item", "external_id: ENG-1\n");
    write(&root, "meta/work/0001-synced-item.md", &synced)?;
    write(
        &root,
        "meta/work/0002-modified.md",
        &bare_work_item_with("0002", "Modified", "external_id: ENG-2\n"),
    )?;
    write(
        &root,
        "meta/work/0003-commented.md",
        &bare_work_item_with("0003", "Commented", "# a frontmatter comment\n"),
    )?;
    write(
        &root,
        ".accelerator/config.md",
        &format!(
            "---\npaths:\n  integrations: {}\n---\n",
            integrations.display()
        ),
    )?;
    let synced_hash = work_adapters::sync::digest::local(&synced)?;
    let modified_hash = "0".repeat(64);
    write(
        &integrations,
        "jira/last-sync.json",
        &format!(
            "{{\"timestamp\": 1, \"items\": {{\"0001\": {{\
             \"remote_updated_at\": \"2026-06-01T00:00:00Z\", \
             \"remote_hash\": \"rh\", \"local_hash\": \"{synced_hash}\"}}, \
             \"0002\": {{\"remote_updated_at\": \"2026-06-01T00:00:00Z\", \
             \"remote_hash\": \"rh\", \
             \"local_hash\": \"{modified_hash}\"}}}}}}\n"
        ),
    )?;
    only_0008_pending(&root)?;

    let output = run_quietly(&root)?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        format!(
            "[0008-canonical-frontmatter-quoting] running\n\
             0008-LOSSY {}/meta/work/0003-commented.md: inline frontmatter \
             comment\n\
             0008: 4 file(s) re-rendered, 1 with dropped comments/CRLF, 1 \
             sync baseline(s) realigned — revert this migration commit to \
             recover\n\
             [0008-canonical-frontmatter-quoting] applied\n",
            root.display()
        )
    );
    let realigned_hash = work_adapters::sync::digest::local(
        &fs::read_to_string(root.join("meta/work/0001-synced-item.md"))?,
    )?;
    assert_eq!(
        fs::read_to_string(integrations.join("jira/last-sync.json"))?,
        format!(
            "{{\"timestamp\":1,\"items\":{{\"0001\":{{\
             \"remote_updated_at\":\"2026-06-01T00:00:00Z\",\
             \"remote_hash\":\"rh\",\"local_hash\":\"{realigned_hash}\",\
             \"local_synced_at\":1}},\"0002\":{{\
             \"remote_updated_at\":\"2026-06-01T00:00:00Z\",\
             \"remote_hash\":\"rh\",\"local_hash\":\"{modified_hash}\",\
             \"local_synced_at\":1}}}}}}\n"
        )
    );
    assert_ne!(realigned_hash, synced_hash);
    Ok(())
}

#[test]
fn a_document_without_a_mapping_frontmatter_root_aborts_the_whole_re_render(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path().canonicalize()?;
    let scalar_root = "---\njust text\n---\nbody\n";
    write(&root, "meta/plans/odd.md", scalar_root)?;
    let renderable = bare_work_item("0001", "Bare");
    write(&root, "meta/work/0001-x.md", &renderable)?;
    only_0008_pending(&root)?;

    let output = run_quietly(&root)?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        !String::from_utf8(output.stdout)?.contains("Migration complete."),
        "the run must not report completion"
    );
    let failure = format!(
        "0008: {}/meta/plans/odd.md: {RE_RENDER_FAILURE}\n",
        root.display()
    );
    assert_eq!(
        String::from_utf8(output.stderr)?,
        format!(
            "[0008-canonical-frontmatter-quoting] running\n{failure}\
             [0008-canonical-frontmatter-quoting] failed\n{failure}"
        )
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/plans/odd.md"))?,
        scalar_root
    );
    assert_eq!(
        fs::read_to_string(root.join("meta/work/0001-x.md"))?,
        renderable
    );
    Ok(())
}
