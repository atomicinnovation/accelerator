use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;

use crate::docs::DocTypeKey;
use crate::file_driver::{FileDriver, FileDriverError};
use crate::frontmatter::{self, FrontmatterState};
use crate::slug;

pub const FRONTMATTER_MALFORMED: &str = "malformed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub r#type: DocTypeKey,
    pub path: PathBuf,
    pub rel_path: PathBuf,
    pub slug: Option<String>,
    pub title: String,
    pub frontmatter: serde_json::Value,
    pub frontmatter_state: String,
    pub ticket: Option<String>,
    pub mtime_ms: i64,
    pub size: u64,
    pub etag: String,
    pub body_preview: String,
}

pub struct Indexer {
    driver: Arc<dyn FileDriver>,
    project_root: PathBuf,
    entries: Arc<RwLock<HashMap<PathBuf, IndexEntry>>>,
    adr_by_id: Arc<RwLock<HashMap<u32, PathBuf>>>,
    ticket_by_number: Arc<RwLock<HashMap<u32, PathBuf>>>,
}

impl Indexer {
    pub async fn build(
        driver: Arc<dyn FileDriver>,
        project_root: PathBuf,
    ) -> Result<Self, FileDriverError> {
        let me = Self {
            driver,
            project_root,
            entries: Arc::new(RwLock::new(HashMap::new())),
            adr_by_id: Arc::new(RwLock::new(HashMap::new())),
            ticket_by_number: Arc::new(RwLock::new(HashMap::new())),
        };
        me.rescan().await?;
        Ok(me)
    }

    pub async fn rescan(&self) -> Result<(), FileDriverError> {
        let mut entries = HashMap::new();
        let mut adr_by_id = HashMap::new();
        let mut ticket_by_number = HashMap::new();

        for kind in DocTypeKey::all() {
            if kind == DocTypeKey::Templates {
                continue;
            }
            let paths = match self.driver.list(kind).await {
                Ok(p) => p,
                Err(FileDriverError::TypeNotConfigured { .. }) => continue,
                Err(e) => return Err(e),
            };
            for path in paths {
                let content = match self.driver.read(&path).await {
                    Ok(c) => c,
                    Err(FileDriverError::NotFound { .. }) => continue,
                    Err(e) => return Err(e),
                };
                let parsed = frontmatter::parse(&content.bytes);
                let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let filename_stem = filename.strip_suffix(".md").unwrap_or(filename);
                let slug_val = slug::derive(kind, filename);
                let title = frontmatter::title_from(&parsed.state, &parsed.body, filename_stem);
                let body_preview = frontmatter::body_preview_from(&parsed.body);
                let ticket = frontmatter::ticket_of(&parsed.state);

                let (state_str, fm_json) = match &parsed.state {
                    FrontmatterState::Parsed(m) => {
                        let mut o = serde_json::Map::new();
                        for (k, v) in m {
                            o.insert(k.clone(), v.clone());
                        }
                        ("parsed".to_string(), serde_json::Value::Object(o))
                    }
                    FrontmatterState::Absent => ("absent".to_string(), serde_json::Value::Null),
                    FrontmatterState::Malformed => {
                        ("malformed".to_string(), serde_json::Value::Null)
                    }
                };

                if kind == DocTypeKey::Decisions {
                    if let Some(id) = parse_adr_id(&fm_json, filename) {
                        adr_by_id.insert(id, path.clone());
                    }
                }
                if kind == DocTypeKey::Tickets {
                    if let Some(n) = parse_ticket_number(filename) {
                        ticket_by_number.insert(n, path.clone());
                    }
                }

                let rel_path = path
                    .strip_prefix(&self.project_root)
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|_| path.clone());

                let entry = IndexEntry {
                    r#type: kind,
                    path: path.clone(),
                    rel_path,
                    slug: slug_val,
                    title,
                    frontmatter: fm_json,
                    frontmatter_state: state_str,
                    ticket,
                    mtime_ms: content.mtime_ms,
                    size: content.size,
                    etag: content.etag,
                    body_preview,
                };
                entries.insert(path, entry);
            }
        }

        *self.entries.write().await = entries;
        *self.adr_by_id.write().await = adr_by_id;
        *self.ticket_by_number.write().await = ticket_by_number;
        Ok(())
    }

    pub async fn all_by_type(&self, kind: DocTypeKey) -> Vec<IndexEntry> {
        self.entries
            .read()
            .await
            .values()
            .filter(|e| e.r#type == kind)
            .cloned()
            .collect()
    }

    pub async fn all(&self) -> Vec<IndexEntry> {
        self.entries.read().await.values().cloned().collect()
    }

    pub async fn get(&self, path: &Path) -> Option<IndexEntry> {
        let guard = self.entries.read().await;
        if let Some(e) = guard.get(path) {
            return Some(e.clone());
        }
        if let Ok(canon) = std::fs::canonicalize(path) {
            return guard.get(&canon).cloned();
        }
        None
    }

    pub async fn adr_by_id(&self, id: u32) -> Option<IndexEntry> {
        let path = { self.adr_by_id.read().await.get(&id).cloned()? };
        self.get(&path).await
    }

    pub async fn ticket_by_number(&self, n: u32) -> Option<IndexEntry> {
        let path = { self.ticket_by_number.read().await.get(&n).cloned()? };
        self.get(&path).await
    }
}

fn parse_adr_id(fm: &serde_json::Value, filename: &str) -> Option<u32> {
    if let Some(s) = fm.get("adr_id").and_then(|v| v.as_str()) {
        if let Some(rest) = s.strip_prefix("ADR-") {
            if let Ok(n) = rest.parse::<u32>() {
                return Some(n);
            }
        }
    }
    let rest = filename.strip_prefix("ADR-")?;
    let dash = rest.find('-')?;
    rest[..dash].parse().ok()
}

fn parse_ticket_number(filename: &str) -> Option<u32> {
    let dash = filename.find('-')?;
    filename[..dash].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_driver::LocalFileDriver;
    use std::sync::Arc;

    fn seed(tmp: &Path) -> (PathBuf, std::collections::HashMap<String, PathBuf>) {
        let dec = tmp.join("meta/decisions");
        let plans = tmp.join("meta/plans");
        let reviews = tmp.join("meta/reviews/plans");
        let notes = tmp.join("meta/notes");
        for d in [&dec, &plans, &reviews, &notes] {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(
            dec.join("ADR-0001-foo.md"),
            "---\nadr_id: ADR-0001\ntitle: Foo\n---\n# Body\n",
        )
        .unwrap();
        std::fs::write(
            plans.join("2026-04-18-hello.md"),
            "---\ntitle: Hello Plan\nstatus: draft\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            plans.join("2026-03-22-no-fm.md"),
            "# Ancient plan with no frontmatter\nbody\n",
        )
        .unwrap();
        std::fs::write(
            plans.join("2026-04-01-malformed.md"),
            "---\ntitle: \"unclosed\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            reviews.join("2026-04-18-hello-review-1.md"),
            "---\ntarget: \"meta/plans/2026-04-18-hello.md\"\n---\n",
        )
        .unwrap();
        std::fs::write(
            reviews
                .join("2026-03-28-initialise-skill-and-review-pr-ephemeral-migration-review-1.md"),
            "---\ntitle: review\n---\n",
        )
        .unwrap();
        std::fs::write(notes.join("2026-03-30-no-fm.md"), "# A bare note\n").unwrap();

        let mut map = HashMap::new();
        map.insert("decisions".into(), dec);
        map.insert("plans".into(), plans);
        map.insert("review_plans".into(), reviews);
        map.insert("notes".into(), notes);
        (tmp.to_path_buf(), map)
    }

    async fn build_indexer(tmp: &Path) -> Indexer {
        let (root, map) = seed(tmp);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![]));
        Indexer::build(driver, root).await.unwrap()
    }

    #[tokio::test]
    async fn scan_populates_entries_for_configured_types() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let decisions = idx.all_by_type(DocTypeKey::Decisions).await;
        assert_eq!(decisions.len(), 1);
        let plans = idx.all_by_type(DocTypeKey::Plans).await;
        assert_eq!(plans.len(), 3);
        let reviews = idx.all_by_type(DocTypeKey::PlanReviews).await;
        assert_eq!(reviews.len(), 2);
        let notes = idx.all_by_type(DocTypeKey::Notes).await;
        assert_eq!(notes.len(), 1);
    }

    #[tokio::test]
    async fn etag_is_content_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let decs = idx.all_by_type(DocTypeKey::Decisions).await;
        let adr = &decs[0];
        let bytes = std::fs::read(&adr.path).unwrap();
        let expected = crate::file_driver::etag_of(&bytes);
        assert_eq!(adr.etag, expected);
    }

    #[tokio::test]
    async fn frontmatter_state_distinguishes_absent_malformed_parsed() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let plans = idx.all_by_type(DocTypeKey::Plans).await;
        let by_name: HashMap<String, IndexEntry> = plans
            .into_iter()
            .map(|e| (e.path.file_name().unwrap().to_string_lossy().to_string(), e))
            .collect();
        assert_eq!(by_name["2026-04-18-hello.md"].frontmatter_state, "parsed");
        assert_eq!(by_name["2026-03-22-no-fm.md"].frontmatter_state, "absent");
        assert_eq!(
            by_name["2026-04-01-malformed.md"].frontmatter_state,
            "malformed"
        );
    }

    #[tokio::test]
    async fn slug_stripped_per_type_and_review_suffix_edge_case() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let revs = idx.all_by_type(DocTypeKey::PlanReviews).await;
        let slugs: Vec<String> = revs.iter().filter_map(|e| e.slug.clone()).collect();
        assert!(slugs.contains(&"hello".to_string()));
        assert!(
            slugs.contains(&"initialise-skill-and-review-pr-ephemeral-migration".to_string()),
            "internal -review- must be preserved in slug; got {slugs:?}",
        );
    }

    #[tokio::test]
    async fn title_fallback_to_first_h1_when_fm_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let notes = idx.all_by_type(DocTypeKey::Notes).await;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "A bare note");
    }

    #[tokio::test]
    async fn adr_by_id_is_populated_from_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let adr = idx.adr_by_id(1).await.unwrap();
        assert_eq!(adr.r#type, DocTypeKey::Decisions);
    }

    #[tokio::test]
    async fn rescan_picks_up_filesystem_mutations() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let before = idx.all_by_type(DocTypeKey::Plans).await.len();
        std::fs::write(
            tmp.path().join("meta/plans/2026-05-01-new.md"),
            "---\ntitle: New\n---\n",
        )
        .unwrap();
        idx.rescan().await.unwrap();
        let after = idx.all_by_type(DocTypeKey::Plans).await.len();
        assert_eq!(after, before + 1);
    }

    #[tokio::test]
    async fn malformed_entry_is_still_addressable_by_path() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let path = tmp.path().join("meta/plans/2026-04-01-malformed.md");
        let entry = idx.get(&path).await.expect("malformed entry still indexed");
        assert_eq!(entry.frontmatter_state, "malformed");
        assert!(entry.etag.starts_with("sha256-"));
    }

    #[tokio::test]
    async fn index_entry_carries_body_preview() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::write(
            plans.join("2026-04-25-foo.md"),
            "---\ntitle: Foo\n---\n# Foo\n\nFirst paragraph of the body.\n",
        )
        .unwrap();

        let mut paths = std::collections::HashMap::new();
        paths.insert("plans".to_string(), plans);
        let driver: Arc<dyn FileDriver> =
            Arc::new(crate::file_driver::LocalFileDriver::new(&paths, vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf())
            .await
            .unwrap();
        let entries = idx.all().await;
        let foo = entries.iter().find(|e| e.title == "Foo").unwrap();
        assert_eq!(foo.body_preview, "First paragraph of the body.");
    }

    #[tokio::test]
    async fn scan_2000_files_completes_within_one_second() {
        let tmp = tempfile::tempdir().unwrap();
        let body = "---\ntitle: Filler\n---\n".to_string() + &"x".repeat(10 * 1024);

        let dirs = [
            ("decisions", "meta/decisions", "0001"),
            ("plans", "meta/plans", "2026-01-01"),
            ("review_plans", "meta/reviews/plans", "2026-01-01"),
            ("notes", "meta/notes", "2026-01-01"),
        ];
        let mut map = HashMap::new();
        for (key, rel, prefix) in &dirs {
            let dir_path = tmp.path().join(rel);
            std::fs::create_dir_all(&dir_path).unwrap();
            for i in 0..500 {
                let name = format!("{}-filler-{i:04}.md", prefix);
                std::fs::write(dir_path.join(name), &body).unwrap();
            }
            map.insert(key.to_string(), dir_path);
        }

        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![]));
        let start = std::time::Instant::now();
        let idx = Indexer::build(driver, tmp.path().to_path_buf())
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_secs() < 1,
            "scan took {elapsed:?}, expected < 1 s",
        );
        assert_eq!(idx.all().await.len(), 2000);
    }
}
