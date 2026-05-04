use crate::docs::DocTypeKey;
use crate::indexer::IndexEntry;
use std::path::PathBuf;

/// Test-only `IndexEntry` factory. New required fields default here in
/// one place; callers override only what they care about.
pub fn entry_for_test(doc_type: DocTypeKey, slug: &str, mtime_ms: i64, title: &str) -> IndexEntry {
    IndexEntry {
        r#type: doc_type,
        path: PathBuf::from(format!("/x/{slug}.md")),
        rel_path: PathBuf::from(format!("{slug}.md")),
        slug: Some(slug.to_string()),
        title: title.to_string(),
        frontmatter: serde_json::Value::Null,
        frontmatter_state: "parsed".to_string(),
        work_item_refs: Vec::new(),
        mtime_ms,
        size: 0,
        etag: "sha256-x".to_string(),
        body_preview: String::new(),
    }
}
