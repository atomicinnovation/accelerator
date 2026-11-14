//! A minimal work-item reader and linker for `create`'s file mode: the title,
//! kind and body from the file, and the `external_id` written back after a
//! create lands.

use std::path::Path;

pub struct WorkItem {
    pub title: String,
    pub kind: String,
    pub body: String,
    pub already_synced: bool,
}

struct Split<'a> {
    frontmatter: &'a str,
    body: &'a str,
}

fn split(content: &str) -> Result<Split<'_>, String> {
    let rest = content
        .strip_prefix("---\n")
        .ok_or_else(|| "the file has no `---` frontmatter".to_owned())?;
    let (frontmatter, body) = rest
        .split_once("\n---\n")
        .ok_or_else(|| "the frontmatter is not closed with `---`".to_owned())?;
    Ok(Split { frontmatter, body })
}

fn field<'a>(frontmatter: &'a str, name: &str) -> Option<&'a str> {
    frontmatter.lines().find_map(|line| {
        line.strip_prefix(name)
            .and_then(|rest| rest.strip_prefix(':'))
            .map(|value| unquote(value.trim()))
    })
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

/// # Errors
///
/// A message when the file is unreadable, has no frontmatter, or names no
/// title.
pub fn read(path: &Path) -> Result<WorkItem, String> {
    let content = std::fs::read_to_string(path).map_err(|error| {
        format!("could not read {}: {error}", path.display())
    })?;
    let parts = split(&content)?;
    let title = field(parts.frontmatter, "title")
        .ok_or_else(|| "the frontmatter names no title".to_owned())?;
    Ok(WorkItem {
        title: title.to_owned(),
        kind: field(parts.frontmatter, "kind")
            .unwrap_or("story")
            .to_owned(),
        body: parts.body.trim_start_matches('\n').to_owned(),
        already_synced: field(parts.frontmatter, "external_id")
            .is_some_and(|value| !value.is_empty()),
    })
}

/// Sets `external_id` in the file's frontmatter, replacing an empty one.
///
/// # Errors
///
/// A message when the file cannot be read, parsed or written.
pub fn link(path: &Path, external_id: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(path).map_err(|error| {
        format!("could not read {}: {error}", path.display())
    })?;
    let parts = split(&content)?;
    let line = format!("external_id: \"{external_id}\"");
    let mut replaced = false;
    let mut lines: Vec<String> = parts
        .frontmatter
        .lines()
        .map(|existing| {
            if existing.starts_with("external_id:") {
                replaced = true;
                line.clone()
            } else {
                existing.to_owned()
            }
        })
        .collect();
    if !replaced {
        lines.push(line);
    }
    let linked = format!("---\n{}\n---\n{}", lines.join("\n"), parts.body);
    std::fs::write(path, linked)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}
