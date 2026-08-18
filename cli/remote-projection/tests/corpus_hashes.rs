//! The raw-token reader must produce the same `remote_hash` the committed
//! sync-baseline corpus records, or the first live sync reclassifies every
//! item.

#![allow(clippy::expect_used)]

use std::path::Path;
use std::path::PathBuf;

use remote_projection::{project, project_raw, Integration, Op};
use sha2::Digest as _;
use sha2::Sha256;

type TestError = Box<dyn std::error::Error>;

fn corpus() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../skills/work/scripts/test-fixtures/work-item-sync-baseline")
        .canonicalize()?)
}

/// The digest the sync engine takes: normalise line-wise, then sha256.
fn remote_hash(projected: &str) -> String {
    let trimmed = work::normalise::trim_lines(projected);
    let mut hasher = Sha256::new();
    hasher.update(trimmed.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn case(name: &str) -> Result<(String, serde_json::Value), TestError> {
    let dir = corpus()?.join(name);
    let raw = std::fs::read_to_string(dir.join("remote.json"))?;
    let expected: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("expected.json"))?,
    )?;
    Ok((raw, expected))
}

#[test]
fn the_raw_reader_reproduces_every_jira_remote_hash() -> Result<(), TestError> {
    let mut checked = 0;
    for name in ["case-jira-adf", "case-jira-no-description"] {
        let (raw, expected) = case(name)?;
        let projected = project_raw(Integration::Jira, Op::Body, &raw)?;
        assert_eq!(
            remote_hash(&projected),
            expected["remote_hash"].as_str().expect("remote_hash"),
            "case {name}"
        );
        checked += 1;
    }
    assert_eq!(checked, 2);
    Ok(())
}

#[test]
fn the_raw_reader_reproduces_every_linear_remote_hash() -> Result<(), TestError>
{
    for name in ["case-linear-empty-description", "case-linear-markdown"] {
        let (raw, expected) = case(name)?;
        let projected = project_raw(Integration::Linear, Op::Body, &raw)?;
        assert_eq!(
            remote_hash(&projected),
            expected["remote_hash"].as_str().expect("remote_hash"),
            "case {name}"
        );
    }
    Ok(())
}

#[test]
fn the_absent_description_case_projects_the_literal_null(
) -> Result<(), TestError> {
    let (raw, _) = case("case-jira-no-description")?;
    let projected = project_raw(Integration::Jira, Op::Body, &raw)?;

    assert!(
        projected.ends_with("\nnull"),
        "the description key is absent, not null and not empty: {projected:?}"
    );
    assert!(
        !projected.contains("\n\nnull"),
        "no blank line before the description"
    );
    Ok(())
}

#[test]
fn the_empty_description_case_projects_an_empty_line() -> Result<(), TestError>
{
    let (raw, _) = case("case-linear-empty-description")?;
    let projected = project_raw(Integration::Linear, Op::Body, &raw)?;

    assert!(
        projected.ends_with('\n'),
        "an empty-string description projects as an empty line: {projected:?}"
    );
    Ok(())
}

#[test]
fn both_projection_entry_points_agree_on_the_whole_corpus(
) -> Result<(), TestError> {
    for name in [
        "case-jira-adf",
        "case-jira-no-description",
        "case-linear-empty-description",
        "case-linear-markdown",
    ] {
        let (raw, expected) = case(name)?;
        let integration = remote_projection::parse_integration(
            expected["integration"].as_str().expect("integration"),
        )
        .expect("a recognised integration");
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        assert_eq!(
            project_raw(integration, Op::Body, &raw)?,
            project(integration, Op::Body, &parsed),
            "case {name}: the two entry points must agree where no numeric \
             literal is involved"
        );
        assert_eq!(
            project_raw(integration, Op::Updated, &raw)?,
            project(integration, Op::Updated, &parsed),
            "case {name}: updated"
        );
    }
    Ok(())
}
