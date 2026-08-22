//! A minimal work-item reader for `create`'s file mode: the title from
//! frontmatter, the body after it, and whether an `external_id` already marks
//! the item as synced.

use std::path::Path;

pub struct WorkItem {
    pub title: String,
    pub body: String,
    pub already_synced: bool,
}

/// Reads a work-item file.
///
/// # Errors
///
/// A message when the file is unreadable, carries no frontmatter, or names no
/// title.
pub fn read(path: &Path) -> Result<WorkItem, String> {
    let content = std::fs::read_to_string(path).map_err(|error| {
        format!("could not read {}: {error}", path.display())
    })?;
    let rest = content
        .strip_prefix("---\n")
        .ok_or_else(|| "the file has no `---` frontmatter".to_owned())?;
    let (frontmatter, body) = rest
        .split_once("\n---\n")
        .ok_or_else(|| "the frontmatter is not closed with `---`".to_owned())?;

    let mut title = None;
    let mut already_synced = false;
    for line in frontmatter.lines() {
        if let Some(value) = line.strip_prefix("title:") {
            title = Some(unquote(value.trim()));
        } else if let Some(value) = line.strip_prefix("external_id:") {
            already_synced = !unquote(value.trim()).is_empty();
        }
    }

    let title =
        title.ok_or_else(|| "the frontmatter names no title".to_owned())?;
    Ok(WorkItem {
        title,
        body: body.to_owned(),
        already_synced,
    })
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
        .to_owned()
}
