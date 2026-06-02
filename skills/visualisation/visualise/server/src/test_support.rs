use crate::config::WorkItemConfig;
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
        work_item_id: None,
        title: title.to_string(),
        frontmatter: serde_json::Value::Null,
        frontmatter_state: "parsed".to_string(),
        work_item_refs: Vec::new(),
        mtime_ms,
        size: 0,
        etag: "sha256-x".to_string(),
        body_preview: String::new(),
        completeness: None,
        linked_count: 0,
    }
}

/// Build a test `IndexEntry` whose slug is computed from `filename` via
/// `slug::derive` under the supplied config. Useful for tests that need
/// to exercise the slug derivation end-to-end without hand-computing.
pub fn entry_for_test_with_filename(
    doc_type: DocTypeKey,
    filename: &str,
    cfg: &WorkItemConfig,
) -> IndexEntry {
    let slug = crate::slug::derive(doc_type, filename, cfg);
    let dir = doc_type
        .config_path_key()
        .unwrap_or("misc");
    let path = PathBuf::from(format!("/repo/meta/{dir}/{filename}"));
    let title = slug.as_deref().unwrap_or("untitled").to_string();
    IndexEntry {
        r#type: doc_type,
        path: path.clone(),
        rel_path: PathBuf::from(format!("meta/{dir}/{filename}")),
        slug,
        work_item_id: None,
        title,
        frontmatter: serde_json::Value::Null,
        frontmatter_state: "parsed".to_string(),
        work_item_refs: Vec::new(),
        mtime_ms: 0,
        size: 0,
        etag: "sha256-x".to_string(),
        body_preview: String::new(),
        completeness: None,
        linked_count: 0,
    }
}
