use crate::config::WorkItemConfig;
use crate::indexer::{IndexEntry, SetContext, SetDocument};
use corpus::cluster::ClusterKeyRule;
use corpus::doc_type::{DocType, Facet, Listing};
use corpus::DocTypeKey;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Test-only `IndexEntry` factory. New required fields default here in
/// one place; callers override only what they care about.
pub fn entry_for_test(
    doc_type: DocTypeKey,
    slug: &str,
    mtime_ms: i64,
    title: &str,
) -> IndexEntry {
    IndexEntry {
        r#type: doc_type,
        path: PathBuf::from(format!("/x/{slug}.md")),
        rel_path: PathBuf::from(format!("{slug}.md")),
        slug: Some(slug.to_string()),
        id: None,
        frontmatter_id: None,
        work_item_id: None,
        work_item_key: None,
        identity_key: None,
        draft: false,
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
        cluster_key: None,
        set: None,
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
    let slug = corpus::slug::derive(doc_type, filename, cfg.convention());
    let dir = doc_type.config_path_key().unwrap_or("misc");
    let path = PathBuf::from(format!("/repo/meta/{dir}/{filename}"));
    let title = slug.as_deref().unwrap_or("untitled").to_string();
    IndexEntry {
        r#type: doc_type,
        path: path.clone(),
        rel_path: PathBuf::from(format!("meta/{dir}/{filename}")),
        slug,
        id: None,
        frontmatter_id: None,
        work_item_id: None,
        work_item_key: None,
        identity_key: None,
        draft: false,
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
        cluster_key: None,
        set: None,
    }
}

pub fn assembled_catalogue() -> std::sync::Arc<corpus::DocTypeCatalogue> {
    declarations::catalogue().expect("the assembled catalogue builds")
}

pub fn set_root_for_test(
    doc_type: DocTypeKey,
    set_directory: &str,
    root: &str,
    set_id: &str,
) -> IndexEntry {
    let directory = PathBuf::from(format!("/x/{set_directory}"));
    let context = Arc::new(SetContext {
        set_id: set_id.to_string(),
        set_slug: Some(set_directory.to_string()),
        set_directory: directory.clone(),
        root_rel_path: PathBuf::from(format!("{set_directory}/{root}")),
    });
    IndexEntry {
        path: directory.join(root),
        rel_path: PathBuf::from(format!("{set_directory}/{root}")),
        id: Some(set_id.to_string()),
        frontmatter_id: Some(set_id.to_string()),
        frontmatter: serde_json::json!({}),
        set: Some(SetDocument::root(context, root.to_string())),
        ..entry_for_test(doc_type, set_directory, 0, set_directory)
    }
}

/// A member that shares every key with its root but its path, title and
/// role, so a test can tell the role apart from any missing field.
pub fn set_member_for_test(
    root: &IndexEntry,
    set_relative_path: &str,
) -> IndexEntry {
    let context = root
        .set
        .as_ref()
        .expect("a set root")
        .placement
        .context
        .clone();
    let stem = Path::new(set_relative_path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    IndexEntry {
        path: context.set_directory.join(set_relative_path),
        rel_path: root
            .rel_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(set_relative_path),
        title: stem,
        set: Some(SetDocument::member(context, set_relative_path.to_string())),
        ..root.clone()
    }
}

static DOSSIERS: DocType = DocType {
    wire_token: "dossiers",
    config_path_key: Some("dossiers"),
    linkage_type_name: Some("dossier"),
    label: "Dossiers",
    carries_target_frontmatter: true,
    participates_in_lifecycle: true,
    cluster_key_rule: ClusterKeyRule::Parent,
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Set,
    ..DocType::FLAT_DEFAULTS
};

/// A set type only tests declare, which carries every key the index's
/// maps and facets read.
pub fn dossiers() -> DocTypeKey {
    DocTypeKey::declared(&DOSSIERS)
}

pub fn dossier(set_directory: &str) -> IndexEntry {
    set_root_for_test(dossiers(), set_directory, "cover.md", set_directory)
}

/// A change sink that keeps every diff it is handed.
#[derive(Default)]
pub struct RecordingSink {
    diffs: std::sync::Mutex<Vec<crate::indexer::IndexDiff>>,
}

impl RecordingSink {
    pub fn diffs(&self) -> Vec<crate::indexer::IndexDiff> {
        self.diffs.lock().unwrap().clone()
    }
}

impl crate::indexer::ChangeSink for RecordingSink {
    fn changed<'a>(
        &'a self,
        diff: &'a crate::indexer::IndexDiff,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>
    {
        self.diffs.lock().unwrap().push(diff.clone());
        Box::pin(std::future::ready(()))
    }
}
