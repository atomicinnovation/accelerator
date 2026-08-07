//! Golden-fixture comparison: `diff_shellout::render`'s output structure
//! against the Phase 1 `work-item-section-diff` fixtures, gated on the
//! real `diff` binary being on `PATH` (mirrors `vcs-adapters`' own
//! `bash-parity` convention — CI enables it, local dev does not need it by
//! default).
#![cfg(feature = "bash-parity")]

use std::path::Path;
use std::path::PathBuf;

use work::section_diff::differing_sections;
use work_adapters::diff_shellout::render;

type TestError = Box<dyn std::error::Error>;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn fixture(case: &str) -> Result<PathBuf, TestError> {
    Ok(repo_root()?.join(format!(
        "skills/work/scripts/test-fixtures/work-item-section-diff/{case}"
    )))
}

#[test]
fn frontmatter_only_case_renders_a_real_diff_u_body() -> Result<(), TestError> {
    let dir = fixture("case-frontmatter-only")?;
    let local = std::fs::read_to_string(dir.join("local.md"))?;
    let remote = std::fs::read_to_string(dir.join("remote.md"))?;

    let diffs = differing_sections(&local, &remote);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].name, "frontmatter");

    let rendered = render(&diffs[0]).map_err(|_| "diff unavailable")?;
    assert!(rendered.starts_with("=== frontmatter (- LOCAL / + REMOTE) ===\n"));
    assert!(rendered.contains("-status: draft"));
    assert!(rendered.contains("+status: ready"));
    Ok(())
}

#[test]
fn no_diff_after_normalisation_case_has_no_sections() -> Result<(), TestError> {
    let dir = fixture("case-no-diff-after-normalisation")?;
    let local = std::fs::read_to_string(dir.join("local.md"))?;
    let remote = std::fs::read_to_string(dir.join("remote.md"))?;
    assert!(differing_sections(&local, &remote).is_empty());
    Ok(())
}

#[test]
fn named_heading_case_reports_summary_and_open_questions_only(
) -> Result<(), TestError> {
    let dir = fixture("case-named-heading-and-one-sided")?;
    let local = std::fs::read_to_string(dir.join("local.md"))?;
    let remote = std::fs::read_to_string(dir.join("remote.md"))?;
    let diffs = differing_sections(&local, &remote);
    let names: Vec<&str> = diffs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, vec!["Summary", "Open Questions"]);
    for diff in &diffs {
        render(diff).map_err(|_| "diff unavailable")?;
    }
    Ok(())
}
