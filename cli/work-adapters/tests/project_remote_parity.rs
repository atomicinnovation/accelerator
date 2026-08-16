//! Pins `work_adapters::project_remote` against the committed
//! `test-fixtures/work-item-project-remote/case-*` fixtures.
//!
//! Each `expected.txt` is four lines: `integration=<name>`,
//! `updated=<value>`, then the projected two-line body verbatim. Any
//! further lines are human commentary and ignored.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;
use std::path::PathBuf;

use work_adapters::project_remote::parse_integration;
use work_adapters::project_remote::project;
use work_adapters::project_remote::Op;

type TestError = Box<dyn std::error::Error>;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn case_dir(name: &str) -> Result<PathBuf, TestError> {
    Ok(repo_root()?.join(format!(
        "skills/work/scripts/test-fixtures/work-item-project-remote/{name}"
    )))
}

struct Expected {
    integration: String,
    updated: String,
    body: String,
}

fn read_expected(dir: &Path) -> Result<Expected, TestError> {
    let raw = std::fs::read_to_string(dir.join("expected.txt"))?;
    let lines: Vec<&str> = raw.lines().collect();
    let integration = lines[0]
        .strip_prefix("integration=")
        .expect("line 1 is integration=<name>")
        .to_owned();
    let updated = lines[1]
        .strip_prefix("updated=")
        .expect("line 2 is updated=<value>")
        .to_owned();
    let body_first = lines[2]
        .strip_prefix("body=")
        .expect("line 3 is body=<first line>");
    let body_second = lines[3];
    Ok(Expected {
        integration,
        updated,
        body: format!("{body_first}\n{body_second}"),
    })
}

fn run_case(name: &str) -> Result<(), TestError> {
    let dir = case_dir(name)?;
    let remote: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("remote.json"))?,
    )?;
    let expected = read_expected(&dir)?;
    let integration = parse_integration(&expected.integration)
        .expect("expected.txt names a recognised integration");

    assert_eq!(
        project(integration, Op::Updated, &remote),
        expected.updated,
        "case {name}: updated"
    );
    assert_eq!(
        project(integration, Op::Body, &remote),
        expected.body,
        "case {name}: body"
    );
    Ok(())
}

#[test]
fn case_jira_matches_expected() -> Result<(), TestError> {
    run_case("case-jira")
}

#[test]
fn case_jira_reordered_matches_expected() -> Result<(), TestError> {
    run_case("case-jira-reordered")
}

#[test]
fn case_linear_matches_expected() -> Result<(), TestError> {
    run_case("case-linear")
}

#[test]
fn jira_reordered_and_jira_project_identical_bodies() -> Result<(), TestError> {
    let plain: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(case_dir("case-jira")?.join("remote.json"))?,
    )?;
    let reordered: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(
            case_dir("case-jira-reordered")?.join("remote.json"),
        )?)?;

    assert_eq!(
        project(
            parse_integration("jira").expect("jira is recognised"),
            Op::Body,
            &plain
        ),
        project(
            parse_integration("jira").expect("jira is recognised"),
            Op::Body,
            &reordered
        )
    );
    Ok(())
}
