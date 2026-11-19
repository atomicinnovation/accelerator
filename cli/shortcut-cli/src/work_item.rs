//! The work item behind `create`'s file mode: its title, kind and body, and
//! the `external_id` written back after a create lands.

use std::path::Path;

use corpus_adapters::FileCorpusStore;
use corpus_adapters::LockdirLock;
use document::Mapping;
use document::Scalar;
use document::Yaml;
use tracker::ExternalId;
use work_adapters::work_item_link;

pub struct WorkItem {
    pub title: String,
    pub kind: String,
    pub body: String,
    pub already_synced: bool,
}

fn text<'a>(frontmatter: &'a Mapping, name: &str) -> Option<&'a str> {
    match frontmatter.get(name) {
        Some(Yaml::Scalar(Scalar::String(value))) => Some(value),
        _ => None,
    }
}

/// # Errors
///
/// A message when the file is unreadable, its frontmatter is not a YAML
/// mapping, or it names no title.
pub fn read(path: &Path) -> Result<WorkItem, String> {
    let content = std::fs::read_to_string(path).map_err(|error| {
        format!("could not read {}: {error}", path.display())
    })?;
    let Yaml::Mapping(frontmatter) =
        document::parse(&content).map_err(|error| error.to_string())?
    else {
        return Err("the frontmatter is not a mapping".to_owned());
    };
    let title = text(&frontmatter, "title")
        .ok_or_else(|| "the frontmatter names no title".to_owned())?;
    let body = document::split(&content)
        .map_err(|error| error.to_string())?
        .body;
    Ok(WorkItem {
        title: title.to_owned(),
        kind: text(&frontmatter, "kind").unwrap_or("story").to_owned(),
        body: body.trim_start_matches('\n').to_owned(),
        already_synced: text(&frontmatter, "external_id")
            .is_some_and(|value| !value.is_empty()),
    })
}

/// Sets `external_id` in the file's frontmatter, under the file's lock.
///
/// # Errors
///
/// A message when the lock cannot be taken, or the file cannot be read,
/// parsed or written.
pub fn link(path: &Path, external_id: &str) -> Result<(), String> {
    let work_dir = path.parent().unwrap_or_else(|| Path::new("."));
    work_item_link::set_external_id(
        &LockdirLock::new(work_dir),
        &FileCorpusStore::new(work_dir),
        path,
        &ExternalId::new(external_id.to_owned()),
    )
    .map_err(|error| kernel::Error::from(error).to_string())
}
