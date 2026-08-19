//! Pins `work::normalise` against
//! `tests/fixtures/work-item-normalise/case-*/`.
//!
//! `case-file-mode` exercises `filter_frontmatter_keys` and `trim_lines`
//! together, with the separator between them emitted unconditionally
//! whichever side is empty. `case-stdin-mode` and `case-nbsp-not-trimmed`
//! exercise `trim_lines` alone.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;
use std::path::PathBuf;

use work::normalise::filter_frontmatter_keys;
use work::normalise::trim_lines;

type TestError = Box<dyn std::error::Error>;

fn case_dir(name: &str) -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/work-item-normalise/{name}"))
        .canonicalize()?)
}

fn is_fence(line: &str) -> bool {
    line.starts_with("---") && line[3..].chars().all(char::is_whitespace)
}

/// Splits a `<file>`-mode input into its raw frontmatter and body at the
/// `^---[[:space:]]*$` fences.
fn split_frontmatter_and_body(content: &str) -> Option<(String, String)> {
    let mut lines = content.lines();
    if !is_fence(lines.next()?) {
        return None;
    }
    let mut frontmatter = Vec::new();
    for line in lines.by_ref() {
        if is_fence(line) {
            let body: Vec<&str> = lines.collect();
            let body_text = body.join("\n");
            return Some((frontmatter.join("\n"), body_text));
        }
        frontmatter.push(line);
    }
    None
}

fn file_mode_output(content: &str) -> String {
    let (frontmatter, body) = split_frontmatter_and_body(content)
        .expect("fixture has a closed fence");
    let filtered = filter_frontmatter_keys(&frontmatter);
    let trimmed_body = trim_lines(&body);
    format!(
        "{}\n{}\n",
        filtered.trim_end_matches('\n'),
        trimmed_body.trim_end_matches('\n')
    )
}

#[test]
fn case_file_mode_matches_filter_frontmatter_keys_and_trim_lines(
) -> Result<(), TestError> {
    let dir = case_dir("case-file-mode")?;
    let input = std::fs::read_to_string(dir.join("input.md"))?;
    let expected = std::fs::read_to_string(dir.join("expected.txt"))?;

    assert_eq!(file_mode_output(&input), expected);
    Ok(())
}

#[test]
fn case_stdin_mode_matches_trim_lines() -> Result<(), TestError> {
    let dir = case_dir("case-stdin-mode")?;
    let input = std::fs::read_to_string(dir.join("input.txt"))?;
    let expected = std::fs::read_to_string(dir.join("expected.txt"))?;

    assert_eq!(trim_lines(&input), expected);
    Ok(())
}

#[test]
fn case_nbsp_not_trimmed_matches_trim_lines() -> Result<(), TestError> {
    let dir = case_dir("case-nbsp-not-trimmed")?;
    let input = std::fs::read_to_string(dir.join("input.txt"))?;
    let expected = std::fs::read_to_string(dir.join("expected.txt"))?;

    assert_eq!(trim_lines(&input), expected);
    Ok(())
}

#[test]
fn every_committed_case_directory_has_a_test_arm() -> Result<(), TestError> {
    let root = case_dir("")?;
    let mut names: Vec<String> = std::fs::read_dir(root)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();

    assert_eq!(
        names,
        vec![
            "case-file-mode".to_owned(),
            "case-nbsp-not-trimmed".to_owned(),
            "case-stdin-mode".to_owned(),
        ],
        "a fixture case was added or removed without a matching test arm here"
    );
    Ok(())
}
