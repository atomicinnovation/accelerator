//! The JSON manifest `work create-batch` reads: one entry per item, each
//! named by a `ref` that other entries' `parent` may point at.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Map;
use serde_json::Value;
use sha2::Digest as _;
use sha2::Sha256;

use crate::create::CreateArgs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentLink {
    /// Another entry of the same manifest, by its `ref`.
    InBatch(String),
    /// A typed reference to an item that already exists, kept as given.
    Existing(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub reference: String,
    pub title: String,
    pub kind: String,
    pub priority: String,
    pub status: String,
    pub body_file: Option<PathBuf>,
    /// The body file's content, read while the manifest is checked so a
    /// missing file refuses the batch before anything is written.
    pub body: Option<String>,
    pub tags: Vec<String>,
    pub blocks: Vec<String>,
    pub blocked_by: Vec<String>,
    pub relates_to: Vec<String>,
    pub derived_from: Vec<String>,
    pub source: Option<String>,
    pub parent: Option<ParentLink>,
}

/// Who the batch's items are recorded as written by, and whether each is
/// pushed.
pub struct Authorship<'a> {
    pub author: Option<&'a str>,
    pub producer: &'a str,
    pub push: bool,
}

impl ManifestEntry {
    #[must_use]
    pub fn create_args(
        &self,
        parent: Option<String>,
        authorship: &Authorship<'_>,
    ) -> CreateArgs {
        CreateArgs {
            title: self.title.clone(),
            kind: self.kind.clone(),
            priority: self.priority.clone(),
            status: self.status.clone(),
            parent,
            tags: self.tags.clone(),
            blocks: self.blocks.clone(),
            blocked_by: self.blocked_by.clone(),
            derived_from: self.derived_from.clone(),
            relates_to: self.relates_to.clone(),
            source: self.source.clone(),
            project: None,
            author: authorship.author.map(str::to_owned),
            producer: authorship.producer.to_owned(),
            body_file: self.body_file.clone(),
            push: authorship.push,
            dry_run: false,
        }
    }
}

pub const E_BATCH_MANIFEST: &str = "E_BATCH_MANIFEST";

const DEFAULT_STATUS: &str = "draft";

/// What identifies a batch across regenerations of its manifest: its
/// entries' refs and titles, whatever their order, bodies or other fields.
#[must_use]
pub fn fingerprint(entries: &[ManifestEntry]) -> String {
    use std::fmt::Write as _;

    let named: BTreeSet<(&str, &str)> = entries
        .iter()
        .map(|entry| (entry.reference.as_str(), entry.title.as_str()))
        .collect();
    let mut hasher = Sha256::new();
    for (reference, title) in named {
        hasher.update(reference.as_bytes());
        hasher.update([0]);
        hasher.update(title.as_bytes());
        hasher.update([0]);
    }
    let mut hex = String::new();
    for byte in hasher.finalize() {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Reads the manifest at `path`. A relative `body_file` is resolved against
/// the manifest's own directory.
///
/// # Errors
///
/// One message per problem found, each naming the entry's `ref` and the
/// field at fault.
pub fn read(path: &Path) -> Result<Vec<ManifestEntry>, Vec<String>> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        vec![format!(
            "{E_BATCH_MANIFEST}: could not read {}: {error}",
            path.display()
        )]
    })?;
    parse(&text, path.parent().unwrap_or_else(|| Path::new(".")))
}

/// # Errors
///
/// As [`read`].
pub fn parse(
    text: &str,
    base: &Path,
) -> Result<Vec<ManifestEntry>, Vec<String>> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        vec![format!(
            "{E_BATCH_MANIFEST}: the manifest is not valid JSON: {error}"
        )]
    })?;
    let Value::Array(raw_entries) = value else {
        return Err(vec![format!(
            "{E_BATCH_MANIFEST}: the manifest must be a JSON array of entries"
        )]);
    };
    let mut problems = Vec::new();
    let entries: Vec<ManifestEntry> = raw_entries
        .iter()
        .enumerate()
        .filter_map(|(position, raw)| {
            let mut fields = EntryFields::new(position, raw, base);
            let entry = fields.entry();
            problems.append(&mut fields.problems);
            entry
        })
        .collect();
    problems.extend(link_problems(&entries));
    if problems.is_empty() {
        Ok(entries)
    } else {
        Err(problems)
    }
}

fn link_problems(entries: &[ManifestEntry]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in entries {
        if !seen.insert(entry.reference.as_str()) {
            problems.push(format!(
                "{E_BATCH_MANIFEST}: entry '{}': field 'ref' duplicates \
                 another entry's",
                entry.reference
            ));
        }
    }
    for entry in entries {
        if let Some(ParentLink::InBatch(parent)) = &entry.parent {
            if !seen.contains(parent.as_str()) {
                problems.push(format!(
                    "{E_BATCH_MANIFEST}: entry '{}': field 'parent' names \
                     ref '{parent}', which no entry has",
                    entry.reference
                ));
            }
        }
    }
    problems
}

/// Reads one entry's fields, gathering every problem rather than stopping
/// at the first.
struct EntryFields<'a> {
    name: String,
    object: Option<&'a Map<String, Value>>,
    base: &'a Path,
    problems: Vec<String>,
}

impl<'a> EntryFields<'a> {
    fn new(position: usize, raw: &'a Value, base: &'a Path) -> Self {
        let object = raw.as_object();
        let name = object
            .and_then(|fields| fields.get("ref"))
            .and_then(Value::as_str)
            .filter(|reference| !reference.is_empty())
            .map_or_else(
                || format!("#{}", position + 1),
                |reference| format!("'{reference}'"),
            );
        Self {
            name,
            object,
            base,
            problems: Vec::new(),
        }
    }

    fn problem(&mut self, field: &str, what: &str) {
        self.problems.push(format!(
            "{E_BATCH_MANIFEST}: entry {}: field '{field}' {what}",
            self.name
        ));
    }

    fn field(&self, name: &str) -> Option<&'a Value> {
        self.object
            .and_then(|fields| fields.get(name))
            .filter(|value| !value.is_null())
    }

    fn required(&mut self, name: &str) -> Option<String> {
        match self.field(name) {
            None => {
                self.problem(name, "is required");
                None
            }
            Some(Value::String(text)) if !text.trim().is_empty() => {
                Some(text.clone())
            }
            Some(_) => {
                self.problem(name, "must be a non-empty string");
                None
            }
        }
    }

    fn optional(&mut self, name: &str) -> Option<String> {
        match self.field(name) {
            None => None,
            Some(Value::String(text)) => Some(text.clone()),
            Some(_) => {
                self.problem(name, "must be a string");
                None
            }
        }
    }

    fn list(&mut self, name: &str) -> Vec<String> {
        match self.field(name) {
            None => Vec::new(),
            Some(Value::Array(items)) if items.iter().all(Value::is_string) => {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            }
            Some(_) => {
                self.problem(name, "must be an array of strings");
                Vec::new()
            }
        }
    }

    fn parent(&mut self) -> Option<ParentLink> {
        match self.field("parent") {
            None => None,
            Some(Value::String(reference)) if !reference.is_empty() => {
                Some(ParentLink::Existing(reference.clone()))
            }
            Some(Value::Object(link)) => {
                match link.get("ref").and_then(Value::as_str) {
                    Some(reference) if !reference.is_empty() => {
                        Some(ParentLink::InBatch(reference.to_owned()))
                    }
                    _ => {
                        self.problem(
                            "parent",
                            "must be {\"ref\": \"<ref>\"} or a typed \
                             reference such as \"work-item:0042\"",
                        );
                        None
                    }
                }
            }
            Some(_) => {
                self.problem(
                    "parent",
                    "must be {\"ref\": \"<ref>\"} or a typed reference such \
                     as \"work-item:0042\"",
                );
                None
            }
        }
    }

    fn body(&mut self) -> (Option<PathBuf>, Option<String>) {
        let Some(named) = self.optional("body_file") else {
            return (None, None);
        };
        let path = self.base.join(named);
        match std::fs::read_to_string(&path) {
            Ok(body) => (Some(path), Some(body)),
            Err(error) => {
                self.problem(
                    "body_file",
                    &format!("could not be read ({}): {error}", path.display()),
                );
                (None, None)
            }
        }
    }

    fn entry(&mut self) -> Option<ManifestEntry> {
        if self.object.is_none() {
            self.problems.push(format!(
                "{E_BATCH_MANIFEST}: entry {} must be a JSON object",
                self.name
            ));
            return None;
        }
        let reference = self.required("ref");
        let title = self.required("title");
        let kind = self.required("kind");
        let priority = self.required("priority");
        let status = self
            .optional("status")
            .unwrap_or_else(|| DEFAULT_STATUS.to_owned());
        let (body_file, body) = self.body();
        let tags = self.list("tags");
        let blocks = self.list("blocks");
        let blocked_by = self.list("blocked_by");
        let relates_to = self.list("relates_to");
        let derived_from = self.list("derived_from");
        let source = self.optional("source");
        let parent = self.parent();
        Some(ManifestEntry {
            reference: reference?,
            title: title?,
            kind: kind?,
            priority: priority?,
            status,
            body_file,
            body,
            tags,
            blocks,
            blocked_by,
            relates_to,
            derived_from,
            source,
            parent,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn problems(text: &str) -> Vec<String> {
        parse(text, Path::new(".")).expect_err("the manifest is refused")
    }

    #[test]
    fn an_entry_reads_every_field_and_defaults_the_status() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("epic.md"), "The body.\n")
            .expect("body");

        let entries = parse(
            r#"[{"ref": "epic", "title": "Epic", "kind": "epic",
                 "priority": "high", "body_file": "epic.md",
                 "tags": ["sync"], "derived_from": ["plan:x"],
                 "source": "doc:y", "parent": "work-item:0042"},
                {"ref": "child", "title": "Child", "kind": "story",
                 "priority": "low", "status": "ready",
                 "parent": {"ref": "epic"}}]"#,
            dir.path(),
        )
        .expect("valid");

        assert_eq!(entries[0].status, "draft");
        assert_eq!(entries[0].body.as_deref(), Some("The body.\n"));
        assert_eq!(entries[0].body_file, Some(dir.path().join("epic.md")));
        assert_eq!(entries[0].tags, vec!["sync".to_owned()]);
        assert_eq!(
            entries[0].parent,
            Some(ParentLink::Existing("work-item:0042".to_owned()))
        );
        assert_eq!(entries[1].status, "ready");
        assert_eq!(
            entries[1].parent,
            Some(ParentLink::InBatch("epic".to_owned()))
        );
    }

    #[test]
    fn a_duplicate_ref_is_rejected() {
        let found = problems(
            r#"[{"ref": "a", "title": "A", "kind": "story", "priority": "low"},
                {"ref": "a", "title": "B", "kind": "story", "priority": "low"}]"#,
        );

        assert_eq!(
            found,
            vec![
                "E_BATCH_MANIFEST: entry 'a': field 'ref' duplicates another \
                 entry's"
                    .to_owned()
            ]
        );
    }

    #[test]
    fn a_parent_ref_naming_no_entry_is_rejected() {
        let found = problems(
            r#"[{"ref": "a", "title": "A", "kind": "story", "priority": "low",
                 "parent": {"ref": "missing"}}]"#,
        );

        assert_eq!(
            found,
            vec!["E_BATCH_MANIFEST: entry 'a': field 'parent' names ref \
                 'missing', which no entry has"
                .to_owned()]
        );
    }

    #[test]
    fn a_missing_required_field_is_rejected() {
        let found = problems(
            r#"[{"ref": "a", "title": "A", "priority": "low"},
                {"title": "B", "kind": "story", "priority": "low"}]"#,
        );

        assert_eq!(
            found,
            vec![
                "E_BATCH_MANIFEST: entry 'a': field 'kind' is required"
                    .to_owned(),
                "E_BATCH_MANIFEST: entry #2: field 'ref' is required"
                    .to_owned(),
            ]
        );
    }

    #[test]
    fn malformed_json_is_rejected() {
        let found = problems("[{\"ref\": ");

        assert_eq!(found.len(), 1);
        assert!(
            found[0].starts_with(
                "E_BATCH_MANIFEST: the manifest is not valid JSON: "
            ),
            "{found:?}"
        );
    }

    #[test]
    fn an_unreadable_body_file_is_rejected_naming_the_field() {
        let found = problems(
            r#"[{"ref": "a", "title": "A", "kind": "story", "priority": "low",
                 "body_file": "no-such-body.md"}]"#,
        );

        assert!(
            found[0].starts_with(
                "E_BATCH_MANIFEST: entry 'a': field 'body_file' could not be \
                 read"
            ),
            "{found:?}"
        );
    }
}
