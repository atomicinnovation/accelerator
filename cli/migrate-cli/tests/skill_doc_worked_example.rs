//! Doctests `skills/config/migrate/SKILL.md`'s "Worked example" against the
//! compiled binary, so the example cannot silently drift once its bash-era
//! verification (`test-migrate-interactive.sh`'s `extract_block` scraping)
//! is gone. Ports that scraping technique — marker-delimited fenced blocks —
//! to Rust, then drives the exact scenario the doc describes: an ambiguous
//! `## References` body linkage in `meta/work/0001-improve-startup-time.md`
//! pointing at `meta/work/0042-add-a-caching-layer.md`, migrations 0001-0006
//! already applied, `0007-unify-meta-corpus-frontmatter` pending.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");
const SKILL_MD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../skills/config/migrate/SKILL.md"
);

/// Mirrors bash's `extract_block()`: the exact text between a
/// `<!-- @{name}-start -->`/`<!-- @{name}-end -->` marker pair, minus the
/// fenced-code-block delimiters immediately inside it.
fn extract_block(doc: &str, name: &str) -> Result<String, TestError> {
    let start_marker = format!("<!-- @{name}-start -->");
    let end_marker = format!("<!-- @{name}-end -->");
    let start = doc.find(&start_marker).ok_or_else(|| {
        format!("SKILL.md is missing the {start_marker} marker")
    })?;
    let end = doc.find(&end_marker).ok_or_else(|| {
        format!("SKILL.md is missing the {end_marker} marker")
    })?;
    let block = &doc[start + start_marker.len()..end];
    let fenced = block
        .trim()
        .strip_prefix("```")
        .and_then(|rest| rest.split_once('\n'))
        .map_or_else(
            || block.trim(),
            |(_lang, rest)| rest.strip_suffix("```").unwrap_or(rest).trim_end(),
        );
    Ok(fenced.to_owned())
}

fn write(dir: &Path, relative: &str, content: &str) -> Result<(), TestError> {
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

fn seed_worked_example(root: &Path) -> Result<(), TestError> {
    write(
        root,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n",
    )?;
    write(
        root,
        "meta/work/0042-add-a-caching-layer.md",
        &work_item("0042", "Add a caching layer", ""),
    )?;
    write(
        root,
        "meta/work/0001-improve-startup-time.md",
        &work_item(
            "0001",
            "Improve startup time",
            "\n## References\n- `meta/work/0042-add-a-caching-layer.md`\n",
        ),
    )?;
    Ok(())
}

#[test]
fn the_list_output_in_the_worked_example_matches_the_real_binary(
) -> Result<(), TestError> {
    let doc = fs::read_to_string(SKILL_MD)?;
    let expected = extract_block(&doc, "list-output")?;

    let dir = TempDir::new()?;
    let root = dir.path();
    seed_worked_example(root)?;

    let output = Command::new(BIN).arg("--list").current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout)?.trim_end(), expected);
    Ok(())
}

#[test]
fn the_decisions_file_worked_example_applies_and_matches_the_documented_after_state(
) -> Result<(), TestError> {
    let doc = fs::read_to_string(SKILL_MD)?;
    let decisions_content = extract_block(&doc, "decisions-file")?;
    let expected_frontmatter_line = extract_block(&doc, "after-frontmatter")?;
    let expected_session_log_shape = extract_block(&doc, "session-log")?;

    let dir = TempDir::new()?;
    let root = dir.path();
    seed_worked_example(root)?;
    let decisions_path = root.join("decisions.txt");
    fs::write(&decisions_path, format!("{decisions_content}\n"))?;

    let output = Command::new(BIN)
        .args(["--decisions-file", decisions_path.to_str().ok_or("path")?])
        .current_dir(root)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");

    let source = fs::read_to_string(
        root.join("meta/work/0001-improve-startup-time.md"),
    )?;
    assert!(
        source.contains(&expected_frontmatter_line),
        "expected {source} to contain {expected_frontmatter_line}"
    );

    let session_log = fs::read_to_string(root.join(
        ".accelerator/state/migrations-0007-unify-meta-corpus-frontmatter-session.jsonl",
    ))?;
    let (documented_prefix, documented_suffix) = expected_session_log_shape
        .split_once("\"timestamp\":\"<REDACTED>\"")
        .ok_or(
            "documented session-log shape is missing the timestamp placeholder",
        )?;
    assert!(
        session_log.starts_with(documented_prefix),
        "expected {session_log:?} to start with {documented_prefix:?}"
    );
    let after_prefix = &session_log[documented_prefix.len()..];
    let timestamp_field_end =
        after_prefix.find(documented_suffix).ok_or_else(|| {
            format!(
                "expected {after_prefix:?} to contain the documented suffix \
             {documented_suffix:?} after a real timestamp field"
            )
        })?;
    let timestamp_field = &after_prefix[..timestamp_field_end];
    assert!(
        timestamp_field.starts_with("\"timestamp\":\"")
            && timestamp_field.ends_with('"')
            && timestamp_field.len() > "\"timestamp\":\"\"".len(),
        "expected a real timestamp field, got {timestamp_field:?}"
    );
    Ok(())
}
