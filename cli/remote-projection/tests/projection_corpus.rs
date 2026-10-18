//! Pins `remote_projection` against the committed
//! `tests/fixtures/work-item-project-remote/case-*` fixtures.
//!
//! Each `expected.txt` is four lines: `integration=<name>`,
//! `updated=<value>`, then the projected two-line body verbatim. Any
//! further lines are human commentary and ignored.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;
use std::path::PathBuf;

use remote_projection::parse_integration;
use remote_projection::project;
use remote_projection::Op;

type TestError = Box<dyn std::error::Error>;

const EXPECTED_CASE_COUNT: usize = 3;

fn case_dir(name: &str) -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/work-item-project-remote/{name}"))
        .canonicalize()?)
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

fn case_names() -> Result<Vec<String>, TestError> {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/work-item-project-remote");
    let mut names: Vec<String> = std::fs::read_dir(corpus)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("case-"))
        .collect();
    names.sort();
    Ok(names)
}

#[test]
fn every_case_matches_its_expectation() -> Result<(), TestError> {
    let names = case_names()?;
    assert_eq!(
        names.len(),
        EXPECTED_CASE_COUNT,
        "a fixture case was added or removed without updating this floor"
    );
    for name in names {
        run_case(&name)?;
    }
    Ok(())
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
