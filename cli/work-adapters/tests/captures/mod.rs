//! Bodies pushed to Linear and read back, from `tests/fixtures`, split into
//! the fragments each capture pairs up by name.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

pub type TestError = Box<dyn std::error::Error>;

pub type Fragments = BTreeMap<String, String>;

pub fn fixture(path: &str) -> Result<String, TestError> {
    Ok(std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(path),
    )?)
}

/// The description a Linear read-back at `path` holds.
pub fn read_back(path: &str) -> Result<String, TestError> {
    let read_back: serde_json::Value = serde_json::from_str(&fixture(path)?)?;
    Ok(read_back["data"]["issue"]["description"]
        .as_str()
        .ok_or_else(|| format!("{path}: no captured description"))?
        .to_owned())
}

/// Each section's body, keyed by the first word of the heading that opens
/// it with `heading_marker`.
pub fn fragments(markdown: &str, heading_marker: &str) -> Fragments {
    let mut sections = BTreeMap::new();
    let mut current: Option<(String, String)> = None;
    for line in markdown.lines() {
        if let Some(heading) = line.strip_prefix(heading_marker) {
            sections.extend(current.take());
            let code = heading.split_whitespace().next().unwrap_or_default();
            current = Some((code.to_owned(), String::new()));
        } else if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    sections.extend(current);
    sections
        .into_iter()
        .map(|(code, body)| (code, body.trim().to_owned()))
        .collect()
}

/// Each line that is its own paragraph, keyed by its first word.
pub fn lines(markdown: &str) -> Fragments {
    markdown
        .lines()
        .filter_map(|line| {
            let code = line.split_whitespace().next()?;
            Some((code.to_owned(), line.to_owned()))
        })
        .collect()
}
