use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{RwLock, Semaphore};

use crate::docs::DocTypeKey;
use crate::file_driver::{FileContent, FileDriver, FileDriverError};
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
    // Serialises rescan() against refresh_one() so they cannot interleave.
    rescan_lock: Arc<Semaphore>,
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
            rescan_lock: Arc::new(Semaphore::new(1)),
        };
        me.rescan().await?;
        Ok(me)
    }

    pub fn rescan_lock(&self) -> Arc<Semaphore> {
        self.rescan_lock.clone()
    }

    pub async fn rescan(&self) -> Result<(), FileDriverError> {
        let _permit = self.rescan_lock.acquire().await.unwrap();

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
                let entry = build_entry(kind, path.clone(), &content, &self.project_root);

                if let Some(id) = adr_id_from_entry(&entry) {
                    adr_by_id.insert(id, path.clone());
                }
                if let Some(n) = ticket_number_from_entry(&entry) {
                    ticket_by_number.insert(n, path.clone());
                }

                entries.insert(path, entry);
            }
        }

        *self.entries.write().await = entries;
        *self.adr_by_id.write().await = adr_by_id;
        *self.ticket_by_number.write().await = ticket_by_number;
        Ok(())
    }

    /// Refreshes a single index entry for the given path without a full rescan.
    ///
    /// Acquires the same `rescan_lock` that `rescan()` uses, so the two cannot
    /// interleave. If the file no longer exists, its entry is removed and
    /// `Ok(None)` is returned.
    pub async fn refresh_one(&self, path: &Path) -> Result<Option<IndexEntry>, FileDriverError> {
        let _permit = self.rescan_lock.acquire().await.unwrap();

        match self.driver.read(path).await {
            Ok(content) => {
                // The driver's read() canonicalized the path internally; use
                // tokio::fs::canonicalize to get the same form as the stored key.
                let canonical = tokio::fs::canonicalize(path)
                    .await
                    .unwrap_or_else(|_| path.to_path_buf());

                let kind = match self.driver.kind_for_canonical_path(&canonical) {
                    Some(k) => k,
                    None => {
                        // Path is not under any known root; remove if present and bail
                        self.entries.write().await.remove(&canonical);
                        return Ok(None);
                    }
                };

                let entry = build_entry(kind, canonical.clone(), &content, &self.project_root);

                if let Some(id) = adr_id_from_entry(&entry) {
                    self.adr_by_id.write().await.insert(id, canonical.clone());
                }
                if let Some(n) = ticket_number_from_entry(&entry) {
                    self.ticket_by_number
                        .write()
                        .await
                        .insert(n, canonical.clone());
                }

                self.entries.write().await.insert(canonical, entry.clone());
                Ok(Some(entry))
            }
            Err(FileDriverError::NotFound { .. }) => {
                // Find the existing entry. When the file is deleted we can no
                // longer canonicalize it (the inode is gone), so fall back to
                // matching by canonical parent + filename. This handles the
                // macOS /var → /private/var symlink and similar indirections.
                let existing = self.find_entry_for_deleted(path).await;

                if let Some(entry) = &existing {
                    let key = entry.path.clone();
                    self.entries.write().await.remove(&key);
                    if let Some(id) = adr_id_from_entry(entry) {
                        self.adr_by_id.write().await.remove(&id);
                    }
                    if let Some(n) = ticket_number_from_entry(entry) {
                        self.ticket_by_number.write().await.remove(&n);
                    }
                }
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Finds an existing index entry for a path whose file has been deleted.
    ///
    /// Direct map lookup is tried first. When that misses (e.g., the path
    /// contains a symlink that `canonicalize` could resolve while the file
    /// existed but can no longer resolve), the parent directory is
    /// canonicalized (it still exists) and used together with the filename to
    /// find the entry — catching macOS `/var` ↔ `/private/var` indirection.
    async fn find_entry_for_deleted(&self, path: &Path) -> Option<IndexEntry> {
        let guard = self.entries.read().await;

        // Fast path: direct lookup (works when path is already canonical)
        if let Some(e) = guard.get(path) {
            return Some(e.clone());
        }

        // Canonicalize only the parent directory (the file itself is gone)
        let filename = path.file_name()?;
        let canonical_parent = path.parent().and_then(|p| std::fs::canonicalize(p).ok())?;

        guard
            .values()
            .find(|e| {
                e.path.file_name() == Some(filename)
                    && e.path.parent() == Some(canonical_parent.as_path())
            })
            .cloned()
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

fn build_entry(
    kind: DocTypeKey,
    path: PathBuf,
    content: &FileContent,
    project_root: &Path,
) -> IndexEntry {
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
        FrontmatterState::Malformed => ("malformed".to_string(), serde_json::Value::Null),
    };

    let rel_path = path
        .strip_prefix(project_root)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| path.clone());

    IndexEntry {
        r#type: kind,
        path,
        rel_path,
        slug: slug_val,
        title,
        frontmatter: fm_json,
        frontmatter_state: state_str,
        ticket,
        mtime_ms: content.mtime_ms,
        size: content.size,
        etag: content.etag.clone(),
        body_preview,
    }
}

fn adr_id_from_entry(entry: &IndexEntry) -> Option<u32> {
    if entry.r#type != DocTypeKey::Decisions {
        return None;
    }
    let filename = entry.path.file_name()?.to_str()?;
    parse_adr_id(&entry.frontmatter, filename)
}

fn ticket_number_from_entry(entry: &IndexEntry) -> Option<u32> {
    if entry.r#type != DocTypeKey::Tickets {
        return None;
    }
    let filename = entry.path.file_name()?.to_str()?;
    parse_ticket_number(filename)
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
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
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
        let driver: Arc<dyn FileDriver> = Arc::new(crate::file_driver::LocalFileDriver::new(
            &paths,
            vec![],
            vec![],
        ));
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

        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
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

#[cfg(test)]
mod refresh_tests {
    use super::*;
    use crate::file_driver::{etag_of, LocalFileDriver};
    use std::sync::Arc;

    async fn build_refresh_indexer(tmp: &Path) -> (Indexer, PathBuf) {
        let tickets = tmp.join("meta/tickets");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(
            tickets.join("0001-foo.md"),
            "---\ntitle: Foo\nstatus: todo\n---\n# body\n",
        )
        .unwrap();
        std::fs::write(
            tickets.join("0002-bar.md"),
            "---\ntitle: Bar\nstatus: done\n---\n# body\n",
        )
        .unwrap();
        std::fs::write(
            tickets.join("0003-baz.md"),
            "---\ntitle: Baz\nstatus: in-progress\n---\n# body\n",
        )
        .unwrap();

        let dec = tmp.join("meta/decisions");
        std::fs::create_dir_all(&dec).unwrap();
        std::fs::write(
            dec.join("ADR-0001-foo.md"),
            "---\nadr_id: ADR-0001\ntitle: Foo Decision\n---\n",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert("tickets".into(), tickets.clone());
        map.insert("decisions".into(), dec);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.to_path_buf()).await.unwrap();
        (idx, tickets)
    }

    // ── Step 2.14 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_picks_up_external_edit() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, tickets) = build_refresh_indexer(tmp.path()).await;

        // Write a new ticket file that didn't exist at build time
        let new_path = tickets.join("0004-new.md");
        std::fs::write(&new_path, "---\ntitle: New\nstatus: todo\n---\n# body\n").unwrap();

        let entry = idx.refresh_one(&new_path).await.unwrap();
        assert!(
            entry.is_some(),
            "new file should be indexed after refresh_one"
        );
        let entry = entry.unwrap();
        assert_eq!(entry.title, "New");

        let raw = std::fs::read(&new_path).unwrap();
        assert_eq!(entry.etag, etag_of(&raw));

        let tickets_entries = idx.all_by_type(DocTypeKey::Tickets).await;
        assert!(
            tickets_entries.iter().any(|e| e.title == "New"),
            "all_by_type should include the new entry"
        );
    }

    // ── Step 2.15 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_updates_etag_on_change() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, tickets) = build_refresh_indexer(tmp.path()).await;

        let path = tickets.join("0001-foo.md");
        let old_entry = idx.get(&path).await.unwrap();

        // Edit the file out-of-band
        std::fs::write(&path, "---\ntitle: Foo\nstatus: in-progress\n---\n# body\n").unwrap();

        idx.refresh_one(&path).await.unwrap();
        let new_entry = idx.get(&path).await.unwrap();

        assert_ne!(
            new_entry.etag, old_entry.etag,
            "etag must change after file edit"
        );
        let raw = std::fs::read(&path).unwrap();
        assert_eq!(new_entry.etag, etag_of(&raw));
    }

    // ── Step 2.16 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_removes_deleted_file() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, tickets) = build_refresh_indexer(tmp.path()).await;

        let path = tickets.join("0001-foo.md");
        assert!(
            idx.get(&path).await.is_some(),
            "entry should exist before deletion"
        );

        std::fs::remove_file(&path).unwrap();
        let result = idx.refresh_one(&path).await.unwrap();

        assert!(
            result.is_none(),
            "refresh_one should return None for deleted file"
        );
        assert!(
            idx.get(&path).await.is_none(),
            "entry should be gone from index"
        );
        assert!(
            idx.ticket_by_number(1).await.is_none(),
            "ticket_by_number index must also be cleaned up"
        );
    }

    // ── Step 2.17 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_does_not_disturb_unrelated_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, tickets) = build_refresh_indexer(tmp.path()).await;

        let path1 = tickets.join("0001-foo.md");
        let path2 = tickets.join("0002-bar.md");
        let path3 = tickets.join("0003-baz.md");

        let before2 = idx.get(&path2).await.unwrap();
        let before3 = idx.get(&path3).await.unwrap();

        idx.refresh_one(&path1).await.unwrap();

        let after2 = idx.get(&path2).await.unwrap();
        let after3 = idx.get(&path3).await.unwrap();

        assert_eq!(before2.etag, after2.etag, "ticket 2 etag must not change");
        assert_eq!(
            before2.mtime_ms, after2.mtime_ms,
            "ticket 2 mtime must not change"
        );
        assert_eq!(before3.etag, after3.etag, "ticket 3 etag must not change");
    }

    // ── Step 2.18 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_rebuilds_secondary_indexes_for_tickets_and_decisions() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, tickets) = build_refresh_indexer(tmp.path()).await;

        // Refresh ticket #1 — ticket_by_number must still work
        let path1 = tickets.join("0001-foo.md");
        idx.refresh_one(&path1).await.unwrap();
        assert!(
            idx.ticket_by_number(1).await.is_some(),
            "ticket_by_number(1) must still resolve after refresh_one"
        );

        // Refresh ADR-0001 — adr_by_id must still work
        let adr_path = tmp.path().join("meta/decisions/ADR-0001-foo.md");
        idx.refresh_one(&adr_path).await.unwrap();
        assert!(
            idx.adr_by_id(1).await.is_some(),
            "adr_by_id(1) must still resolve after refresh_one"
        );
    }

    // ── Step 2.19 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_serialises_with_concurrent_rescan() {
        use std::sync::Arc as StdArc;
        use tokio::sync::Barrier;

        let tmp = tempfile::tempdir().unwrap();
        let (idx, tickets) = build_refresh_indexer(tmp.path()).await;
        let idx = StdArc::new(idx);

        let path = tickets.join("0001-foo.md");
        // Out-of-band edit so refresh_one picks up a different etag
        std::fs::write(&path, "---\ntitle: Foo\nstatus: done\n---\n# body\n").unwrap();
        let expected_etag = etag_of(&std::fs::read(&path).unwrap());

        let barrier = StdArc::new(Barrier::new(2));
        let idx2 = idx.clone();
        let path2 = path.clone();
        let b2 = barrier.clone();
        let rescan_handle = tokio::spawn(async move {
            b2.wait().await;
            idx2.rescan().await.unwrap();
        });

        let b3 = barrier.clone();
        let idx3 = idx.clone();
        let refresh_handle = tokio::spawn(async move {
            b3.wait().await;
            idx3.refresh_one(&path2).await.unwrap();
        });

        rescan_handle.await.unwrap();
        refresh_handle.await.unwrap();

        // After both complete, the entry for path must be present
        // (either rescan or refresh_one produced it — both include the edited content)
        let entry = idx
            .get(&path)
            .await
            .expect("entry must exist after concurrent ops");
        // The entry's etag must match the edited file
        assert_eq!(
            entry.etag, expected_etag,
            "final entry must reflect the edited file content"
        );
    }
}
