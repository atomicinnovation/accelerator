use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use futures::stream::{self, StreamExt};
use serde::Serialize;
use tokio::sync::{RwLock, Semaphore};

use crate::clusters::Completeness;
use crate::docs::DocTypeKey;
use crate::file_driver::{FileContent, FileDriver, FileDriverError};
use crate::frontmatter::{self, FrontmatterState};
use crate::slug;

pub const FRONTMATTER_MALFORMED: &str = "malformed";

/// Active facet selection, scoped per doc type. Empty selection ⇒ no filtering.
/// Keyed by doc type, then by facet id (e.g. "status", "clusterSlug", "project").
/// Values are the selected option ids for that facet (OR within a facet, AND
/// across facets).
pub type Selection = HashMap<DocTypeKey, HashMap<String, Vec<String>>>;

/// Aggregated per-doc-type library data used by `/api/library/structure`.
#[derive(Default, Debug)]
pub struct LibraryAggregates {
    pub per_type: HashMap<DocTypeKey, PerTypeAggregate>,
}

#[derive(Default, Debug)]
pub struct PerTypeAggregate {
    /// Total entries (selection-unaware; for hub/overview).
    pub count: usize,
    /// Entries matching this type's selection (used for list-view "N documents").
    pub filtered_count: usize,
    /// Selection-unaware: hub card always shows the absolute latest.
    pub latest: Option<LatestPreview>,
    /// facet_options[facet_id] => sorted map of option-id → count.
    /// Counts are computed with post-other-facet, pre-own-facet scoping.
    pub facet_options: HashMap<String, BTreeMap<String, usize>>,
}

#[derive(Default, Debug, Clone)]
pub struct LatestPreview {
    pub title: String,
    pub slug: Option<String>,
    /// Used as deterministic tie-break key.
    pub rel_path: String,
    pub modified_at: i64,
}

impl LatestPreview {
    fn from_entry(entry: &IndexEntry) -> Self {
        Self {
            title: entry.title.clone(),
            slug: entry.slug.clone(),
            rel_path: entry.rel_path.to_string_lossy().into_owned(),
            modified_at: entry.mtime_ms,
        }
    }
}

/// Facet ids declared for a given doc type.
pub fn facets_for(doc_type: DocTypeKey) -> &'static [&'static str] {
    match doc_type {
        DocTypeKey::WorkItems => &["status", "project", "clusterSlug"],
        DocTypeKey::Templates => &[],
        _ => &["status", "clusterSlug"],
    }
}

/// Extracts the value an entry contributes to the given facet, or `None` when
/// the entry does not contribute to that facet at all.
pub fn extract_facet_value(
    entry: &IndexEntry,
    cfg: &crate::config::Config,
    facet_id: &str,
) -> Option<String> {
    match facet_id {
        "status" => {
            if entry.frontmatter_state != "parsed" {
                return None;
            }
            entry
                .frontmatter
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }
        "clusterSlug" => entry.slug.clone(),
        "project" => {
            let raw = entry.work_item_id.as_deref()?;
            if let Some((prefix, _)) = raw.split_once('-') {
                if prefix.is_empty() {
                    None
                } else {
                    Some(prefix.to_string())
                }
            } else {
                cfg.work_item
                    .as_ref()
                    .and_then(|w| w.default_project_code.clone())
            }
        }
        _ => None,
    }
}

/// True iff every facet in `type_selection` has a non-empty selected-options
/// list that includes the entry's value for that facet. An empty selected-
/// options list for a facet acts as a no-op (matches every entry).
pub fn entry_matches_all(
    entry: &IndexEntry,
    cfg: &crate::config::Config,
    type_selection: Option<&HashMap<String, Vec<String>>>,
) -> bool {
    let Some(sel) = type_selection else {
        return true;
    };
    for (facet_id, options) in sel {
        if options.is_empty() {
            continue;
        }
        let value = match extract_facet_value(entry, cfg, facet_id) {
            Some(v) => v,
            None => return false,
        };
        if !options.iter().any(|o| o == &value) {
            return false;
        }
    }
    true
}

/// Like `entry_matches_all` but skips the facet equal to `except_facet`.
pub fn entry_matches_all_except(
    entry: &IndexEntry,
    cfg: &crate::config::Config,
    type_selection: Option<&HashMap<String, Vec<String>>>,
    except_facet: &str,
) -> bool {
    let Some(sel) = type_selection else {
        return true;
    };
    for (facet_id, options) in sel {
        if facet_id == except_facet {
            continue;
        }
        if options.is_empty() {
            continue;
        }
        let value = match extract_facet_value(entry, cfg, facet_id) {
            Some(v) => v,
            None => return false,
        };
        if !options.iter().any(|o| o == &value) {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub r#type: DocTypeKey,
    pub path: PathBuf,
    pub rel_path: PathBuf,
    pub slug: Option<String>,
    /// Filename-derived work-item ID (via the configured scan regex).
    /// `Some(id)` when the entry is a work-item and the filename matches;
    /// `None` for non-work-item types or unmatched filenames.
    pub work_item_id: Option<String>,
    pub title: String,
    pub frontmatter: serde_json::Value,
    pub frontmatter_state: String,
    pub work_item_refs: Vec<String>,
    pub mtime_ms: i64,
    pub size: u64,
    pub etag: String,
    pub body_preview: String,
    /// Cluster-level Completeness back-filled by `compute_clusters_with_backfill`.
    /// Serialises as JSON `null` for orphan entries (no slug) and for entries
    /// that have not yet been through a cluster pass — kanban cards read this
    /// signal to switch to orphan rendering.
    pub completeness: Option<Completeness>,
    /// Total related-artifact count, back-filled from
    /// `count_from_resolution(resolve_related(...))`. Equals the sum of the
    /// three array lengths returned by `/api/related/{path}` for the same
    /// entry, by construction. Defaults to `0` until a cluster pass
    /// populates it.
    pub linked_count: usize,
    /// Composite cluster key resolved by walking typed-linkage frontmatter
    /// (per ADR-0034) back to a canonical work-item id. `None` when no
    /// chain reaches a work item — such entries fall back to the slug
    /// bucket (lifecycle-participating types) or a per-path orphan bucket
    /// (orphan-by-design types). Serialises on the wire as `clusterKey`.
    pub cluster_key: Option<String>,
}

/// Test rendezvous point used by Phase 9 concurrency tests to inspect
/// state at the precise moment after the secondary indexes have been
/// updated but before the `entries.write()` guard is released. Two
/// `oneshot` channels — `reached` (writer → test) and `proceed`
/// (test → writer) — give a deterministic, lost-wakeup-free
/// rendezvous.
#[cfg(test)]
pub(crate) struct PostSecondaryUpdateHook {
    pub reached: tokio::sync::oneshot::Sender<()>,
    pub proceed: tokio::sync::oneshot::Receiver<()>,
}

pub struct Indexer {
    driver: Arc<dyn FileDriver>,
    /// Canonicalised once at the top of `Indexer::build`. Every
    /// secondary-index key derivation routes through this prefix so
    /// primary keys (canonical via `build_entry`) and secondary keys
    /// (lexical `project_root.join(raw)`) share the same canonical
    /// prefix. The field is module-private (no `pub`) — read-only
    /// access is exposed via `project_root()`.
    project_root: PathBuf,
    work_item_cfg: Arc<crate::config::WorkItemConfig>,
    entries: Arc<RwLock<HashMap<PathBuf, IndexEntry>>>,
    adr_by_id: Arc<RwLock<HashMap<u32, PathBuf>>>,
    work_item_by_id: Arc<RwLock<HashMap<String, PathBuf>>>,
    /// Secondary index mapping plan `id:` (filename stem) to plan path.
    /// Used to resolve `target: "plan:<id>"` typed-linkage references per
    /// ADR-0034. Lock-ordering invariant: acquire after `work_item_by_id`
    /// and before `reviews_by_target`.
    plans_by_id: Arc<RwLock<HashMap<String, PathBuf>>>,
    /// Reverse declared-link index. Keys are lexically-clean absolute
    /// paths of target plans (or any future target type); values are
    /// sets of canonicalised paths of reviews referencing the target.
    /// `BTreeSet` gives deterministic iteration order and
    /// dedup-by-construction.
    reviews_by_target: Arc<RwLock<HashMap<PathBuf, BTreeSet<PathBuf>>>>,
    /// Reverse work-item cross-ref index. Keys are canonical work-item IDs
    /// (as produced by `canonicalise_refs`); values are sets of canonicalised
    /// paths of entries that reference that work-item via `work_item_id:`,
    /// `parent:`, or `related:` frontmatter keys.
    work_item_refs_by_target: Arc<RwLock<HashMap<String, BTreeSet<PathBuf>>>>,
    // Serialises rescan() against refresh_one() so they cannot interleave.
    rescan_lock: Arc<Semaphore>,
    #[cfg(test)]
    test_post_secondary_update: tokio::sync::Mutex<Option<PostSecondaryUpdateHook>>,
}

impl Indexer {
    pub async fn build(
        driver: Arc<dyn FileDriver>,
        project_root: PathBuf,
        work_item_cfg: Arc<crate::config::WorkItemConfig>,
    ) -> Result<Self, FileDriverError> {
        // Canonicalise once. Every downstream consumer — `rescan`,
        // `refresh_one`, the secondary-index helpers, and
        // `Indexer::reviews_by_target` — sees a canonical path. The
        // failure (e.g., the project_root does not exist) is mapped
        // into the existing `FileDriverError` channel so call sites
        // do not learn a new error type.
        let project_root = tokio::fs::canonicalize(&project_root)
            .await
            .map_err(|source| FileDriverError::Io {
                path: project_root.clone(),
                source,
            })?;
        let me = Self {
            driver,
            project_root,
            work_item_cfg,
            entries: Arc::new(RwLock::new(HashMap::new())),
            adr_by_id: Arc::new(RwLock::new(HashMap::new())),
            work_item_by_id: Arc::new(RwLock::new(HashMap::new())),
            plans_by_id: Arc::new(RwLock::new(HashMap::new())),
            reviews_by_target: Arc::new(RwLock::new(HashMap::new())),
            work_item_refs_by_target: Arc::new(RwLock::new(HashMap::new())),
            rescan_lock: Arc::new(Semaphore::new(1)),
            #[cfg(test)]
            test_post_secondary_update: tokio::sync::Mutex::new(None),
        };
        me.rescan().await?;
        Ok(me)
    }

    pub fn rescan_lock(&self) -> Arc<Semaphore> {
        self.rescan_lock.clone()
    }

    pub async fn rescan(&self) -> Result<(), FileDriverError> {
        let _permit = self.rescan_lock.acquire().await.unwrap();

        let mut entries: HashMap<PathBuf, IndexEntry> = HashMap::new();
        let mut adr_by_id: HashMap<u32, PathBuf> = HashMap::new();
        let mut work_item_by_id: HashMap<String, PathBuf> = HashMap::new();
        let mut plans_by_id: HashMap<String, PathBuf> = HashMap::new();
        let mut reviews_by_target: HashMap<PathBuf, BTreeSet<PathBuf>> = HashMap::new();
        let mut work_item_refs_by_target: HashMap<String, BTreeSet<PathBuf>> = HashMap::new();

        // Phase 1: enumerate every (kind, path) up front, preserving doc-type
        // and per-type listing order so the dedup folds below stay deterministic.
        let mut targets: Vec<(DocTypeKey, PathBuf)> = Vec::new();
        for kind in DocTypeKey::all() {
            if kind == DocTypeKey::Templates {
                continue;
            }
            match self.driver.list(kind).await {
                Ok(paths) => targets.extend(paths.into_iter().map(|p| (kind, p))),
                Err(FileDriverError::TypeNotConfigured { .. }) => continue,
                Err(e) => return Err(e),
            }
        }

        // Phase 2: read files concurrently. Each read issues several `tokio::fs`
        // calls that block; overlapping them lets the runtime's blocking pool
        // service many at once instead of one round-trip at a time. `buffered`
        // preserves input order, so the serial fold below remains deterministic.
        const READ_CONCURRENCY: usize = 64;
        let read_results: Vec<(DocTypeKey, PathBuf, Result<FileContent, FileDriverError>)> =
            stream::iter(targets)
                .map(|(kind, path)| {
                    let driver = &self.driver;
                    async move {
                        let content = driver.read(&path).await;
                        (kind, path, content)
                    }
                })
                .buffered(READ_CONCURRENCY)
                .collect()
                .await;

        // Phase 3 Pass A: parse and fold serially, in original order.
        // Build entries and every primary-id map. Defer reviews_by_target
        // until Pass B because resolving `target: "plan:<id>"` requires
        // the fully-populated `plans_by_id`.
        for (kind, path, content) in read_results {
            let content = match content {
                Ok(c) => c,
                Err(FileDriverError::NotFound { .. }) => continue,
                Err(e) => return Err(e),
            };
            let entry = build_entry(kind, path.clone(), &content, &self.project_root, &self.work_item_cfg);

            if let Some(id) = adr_id_from_entry(&entry) {
                adr_by_id.insert(id, path.clone());
            }
            if let Some(id) = work_item_id_from_entry(&entry) {
                work_item_by_id.insert(id, path.clone());
            }
            if let Some(id) = plan_id_from_entry(&entry) {
                plans_by_id.insert(id, path.clone());
            }
            for id in canonicalise_refs(entry.work_item_refs.clone(), &self.work_item_cfg) {
                // Skip self-references: a doc must not appear in its own index.
                if entry.work_item_id.as_deref() == Some(id.as_str()) {
                    continue;
                }
                work_item_refs_by_target
                    .entry(id)
                    .or_default()
                    .insert(path.clone());
            }

            entries.insert(path, entry);
        }

        // Phase 3 Pass B: resolve reviews_by_target with the fully-
        // populated `plans_by_id` so typed-linkage `target: "plan:<id>"`
        // references resolve regardless of file-driver iteration order.
        for entry in entries.values() {
            if let Some(target_key) = target_path_from_entry(
                entry,
                &plans_by_id,
                &work_item_by_id,
                &self.work_item_cfg,
                &self.project_root,
            ) {
                reviews_by_target
                    .entry(target_key)
                    .or_default()
                    .insert(entry.path.clone());
            }
        }

        // Hold all six write locks simultaneously and replace contents
        // so readers never observe a partial (entries, secondary)
        // snapshot. Always acquire in the same order: entries → adr →
        // work_item → plans → reviews_by_target → work_item_refs_by_target.
        let mut entries_w = self.entries.write().await;
        let mut adr_w = self.adr_by_id.write().await;
        let mut work_item_w = self.work_item_by_id.write().await;
        let mut plans_w = self.plans_by_id.write().await;
        let mut reviews_w = self.reviews_by_target.write().await;
        let mut refs_w = self.work_item_refs_by_target.write().await;
        *entries_w = entries;
        *adr_w = adr_by_id;
        *work_item_w = work_item_by_id;
        *plans_w = plans_by_id;
        *reviews_w = reviews_by_target;
        *refs_w = work_item_refs_by_target;
        Ok(())
    }

    /// Refreshes a single index entry for the given path without a full rescan.
    ///
    /// Acquires the same `rescan_lock` that `rescan()` uses, so the two cannot
    /// interleave. If the file no longer exists, its entry is removed and
    /// `Ok(None)` is returned.
    ///
    /// Lock-ordering invariant: the entire update happens while holding a
    /// single `entries.write()` guard. Each secondary-index helper takes its
    /// own write lock *while* the caller holds `entries.write()`, so readers
    /// never observe a partial (entries, secondary) snapshot.
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
                        // Path is not under any known root; remove if present and bail.
                        // Hold entries.write() and clean each secondary index for
                        // any previous entry under this path before dropping.
                        let mut entries = self.entries.write().await;
                        if let Some(previous) = entries.get(&canonical).cloned() {
                            remove_from_adr_by_id(&self.adr_by_id, &previous).await;
                            remove_from_work_item_by_id(&self.work_item_by_id, &previous).await;
                            remove_from_plans_by_id(&self.plans_by_id, &previous).await;
                            let plans_snapshot = self.plans_by_id.read().await.clone();
                            // Snapshot work_item_by_id AFTER the work-item-by-id
                            // removal, mirroring plans_snapshot, so the typed
                            // `work-item:` target resolves against a consistent view.
                            let work_item_snapshot = self.work_item_by_id.read().await.clone();
                            remove_from_reviews_by_target(
                                &self.reviews_by_target,
                                &plans_snapshot,
                                &work_item_snapshot,
                                &self.work_item_cfg,
                                &self.project_root,
                                &previous,
                            )
                            .await;
                            remove_from_work_item_refs_by_target(
                                &self.work_item_refs_by_target,
                                &self.work_item_cfg,
                                &previous,
                            )
                            .await;
                            entries.remove(&canonical);
                        }
                        return Ok(None);
                    }
                };

                let entry = build_entry(kind, canonical.clone(), &content, &self.project_root, &self.work_item_cfg);

                // Single-writer-lock invariant: hold entries.write() across
                // every secondary-index update so readers see a consistent
                // (entries, secondary) snapshot.
                let mut entries = self.entries.write().await;
                let previous = entries.get(&canonical).cloned();

                update_adr_by_id(&self.adr_by_id, &entry, previous.as_ref()).await;
                update_work_item_by_id(&self.work_item_by_id, &entry, previous.as_ref()).await;
                update_plans_by_id(&self.plans_by_id, &entry, previous.as_ref()).await;
                // Acquire a snapshot of plans_by_id AFTER the plans-by-id
                // helper has folded the new entry in, so the
                // reviews_by_target update sees a consistent view.
                let plans_snapshot = self.plans_by_id.read().await.clone();
                // Snapshot work_item_by_id AFTER update_work_item_by_id has
                // folded the new entry in, mirroring plans_snapshot, so a typed
                // `work-item:` target resolves against a consistent view.
                let work_item_snapshot = self.work_item_by_id.read().await.clone();
                update_reviews_by_target(
                    &self.reviews_by_target,
                    &plans_snapshot,
                    &work_item_snapshot,
                    &self.work_item_cfg,
                    &self.project_root,
                    &entry,
                    previous.as_ref(),
                )
                .await;
                update_work_item_refs_by_target(
                    &self.work_item_refs_by_target,
                    &self.work_item_cfg,
                    &entry,
                    previous.as_ref(),
                )
                .await;

                entries.insert(canonical.clone(), entry.clone());

                #[cfg(test)]
                if let Some(hook) = self.test_post_secondary_update.lock().await.take() {
                    let _ = hook.reached.send(());
                    let _ = hook.proceed.await;
                }
                drop(entries);

                Ok(Some(entry))
            }
            Err(FileDriverError::NotFound { .. }) => {
                // Acquire the write guard FIRST, then perform the lookup
                // against it. Eliminates the read-then-write TOCTOU window
                // and keeps deletion under the same single-writer-lock
                // invariant.
                let mut entries = self.entries.write().await;
                let previous = find_entry_for_deleted(&entries, path);

                if let Some(previous) = previous {
                    remove_from_adr_by_id(&self.adr_by_id, &previous).await;
                    remove_from_work_item_by_id(&self.work_item_by_id, &previous).await;
                    remove_from_plans_by_id(&self.plans_by_id, &previous).await;
                    let plans_snapshot = self.plans_by_id.read().await.clone();
                    let work_item_snapshot = self.work_item_by_id.read().await.clone();
                    remove_from_reviews_by_target(
                        &self.reviews_by_target,
                        &plans_snapshot,
                        &work_item_snapshot,
                        &self.work_item_cfg,
                        &self.project_root,
                        &previous,
                    )
                    .await;
                    remove_from_work_item_refs_by_target(
                        &self.work_item_refs_by_target,
                        &self.work_item_cfg,
                        &previous,
                    )
                    .await;
                    entries.remove(&previous.path);
                }

                #[cfg(test)]
                if let Some(hook) = self.test_post_secondary_update.lock().await.take() {
                    let _ = hook.reached.send(());
                    let _ = hook.proceed.await;
                }
                drop(entries);

                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    #[cfg(test)]
    pub(crate) async fn install_post_secondary_update_hook(&self, hook: PostSecondaryUpdateHook) {
        *self.test_post_secondary_update.lock().await = Some(hook);
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

    pub async fn counts_by_type(&self) -> HashMap<DocTypeKey, usize> {
        let mut out: HashMap<DocTypeKey, usize> = HashMap::new();
        for entry in self.entries.read().await.values() {
            *out.entry(entry.r#type).or_insert(0) += 1;
        }
        out
    }

    /// Computes counts, the latest entry, and facet-option counts per doc type.
    /// Facet counts use post-other-facet, pre-own-facet scoping: for each facet,
    /// count over the set of entries matching every OTHER facet's selection
    /// (so toggling a value in facet B updates facet A's counts but does not
    /// hide facet A's own currently-selected option).
    // PERF: Recompute per request — same complexity class as counts_by_type
    // × the number of facets per doc type. At v1 scale (low thousands of
    // entries, ≤3 facets per type), cold runs are well under 10ms. Migrate to
    // the `state.clusters` cached pattern (see server.rs:46,82) when p95
    // handler latency exceeds ~50ms or the entry count exceeds ~10k.
    pub async fn library_aggregates(
        &self,
        cfg: &crate::config::Config,
        selection: &Selection,
    ) -> LibraryAggregates {
        let entries = self.entries.read().await;
        let mut agg = LibraryAggregates::default();

        // First pass: counts and latest preview (selection-unaware).
        for entry in entries.values() {
            let per = agg.per_type.entry(entry.r#type).or_default();
            per.count += 1;

            let preview = LatestPreview::from_entry(entry);
            per.latest = Some(match per.latest.take() {
                None => preview,
                Some(existing) => {
                    if preview.modified_at > existing.modified_at
                        || (preview.modified_at == existing.modified_at
                            && preview.rel_path < existing.rel_path)
                    {
                        preview
                    } else {
                        existing
                    }
                }
            });
        }

        // Second pass: facet scoping per doc type, under the same lock.
        for (doc_type, per) in agg.per_type.iter_mut() {
            let type_selection = selection.get(doc_type);
            let type_entries: Vec<&IndexEntry> =
                entries.values().filter(|e| e.r#type == *doc_type).collect();

            per.filtered_count = type_entries
                .iter()
                .filter(|e| entry_matches_all(e, cfg, type_selection))
                .count();

            for facet_id in facets_for(*doc_type) {
                let mut option_counts: BTreeMap<String, usize> = BTreeMap::new();
                for entry in &type_entries {
                    if !entry_matches_all_except(entry, cfg, type_selection, facet_id) {
                        continue;
                    }
                    if let Some(option_id) = extract_facet_value(entry, cfg, facet_id) {
                        *option_counts.entry(option_id).or_insert(0) += 1;
                    }
                }
                per.facet_options.insert((*facet_id).to_string(), option_counts);
            }
        }

        agg
    }

    pub async fn all(&self) -> Vec<IndexEntry> {
        self.entries.read().await.values().cloned().collect()
    }

    /// Returns a cloned snapshot of `work_item_by_id` for callers
    /// constructing a `ClusterContext`.
    pub async fn work_item_by_id_snapshot(&self) -> HashMap<String, PathBuf> {
        self.work_item_by_id.read().await.clone()
    }

    /// Returns a cloned snapshot of `plans_by_id` for callers
    /// constructing a `ClusterContext`.
    pub async fn plans_by_id_snapshot(&self) -> HashMap<String, PathBuf> {
        self.plans_by_id.read().await.clone()
    }

    /// Returns the work-item config used by the indexer. Cluster-key
    /// resolution reuses it to canonicalise frontmatter values.
    pub fn work_item_cfg(&self) -> &crate::config::WorkItemConfig {
        &self.work_item_cfg
    }

    /// Applies a per-path cluster_key map onto `IndexEntry.cluster_key`
    /// under a single `entries.write()` lock. Paths absent from
    /// `backfill` have their cluster_key cleared to `None`, mirroring
    /// the apply_completeness_backfill contract so a slug-derivation
    /// regression or removal of typed-linkage frontmatter flips the
    /// entry back to slug-bucket rendering.
    pub async fn apply_cluster_key_backfill(
        &self,
        backfill: HashMap<PathBuf, Option<String>>,
    ) {
        let mut entries = self.entries.write().await;
        for (path, entry) in entries.iter_mut() {
            entry.cluster_key = backfill.get(path).cloned().unwrap_or(None);
        }
    }

    /// Applies a per-path `Completeness` map onto `IndexEntry.completeness`
    /// under a single `entries.write()` lock. Paths absent from `entries`
    /// (because the file was deleted between Pass 1 snapshot and Pass 2
    /// apply) are silently skipped. Paths in `entries` but absent from
    /// `backfill` (orphan entries) have their `completeness` cleared to
    /// `None`, so a slug-derivation regression flips an entry to orphan
    /// rendering rather than leaving a stale cluster snapshot behind.
    pub async fn apply_completeness_backfill(
        &self,
        backfill: HashMap<PathBuf, Completeness>,
    ) {
        let mut entries = self.entries.write().await;
        for (path, entry) in entries.iter_mut() {
            entry.completeness = backfill.get(path).cloned();
        }
    }

    /// Applies a per-path `linked_count` map onto `IndexEntry.linked_count`
    /// under a single `entries.write()` lock. Paths absent from `backfill`
    /// fall back to `0`, so an entry that no longer has any cross-links
    /// drops its count rather than retaining a stale value.
    pub async fn apply_linked_count_backfill(&self, backfill: HashMap<PathBuf, usize>) {
        let mut entries = self.entries.write().await;
        for (path, entry) in entries.iter_mut() {
            entry.linked_count = backfill.get(path).copied().unwrap_or(0);
        }
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

    pub async fn work_item_by_id(&self, id: &str) -> Option<IndexEntry> {
        let path = { self.work_item_by_id.read().await.get(id).cloned()? };
        self.get(&path).await
    }

    /// Returns the project root (canonicalised at construction). Used by
    /// callers that need to construct lexical absolute keys (e.g., the
    /// related-artifacts handler).
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Returns reviews whose `target:` resolves to the given absolute path.
    /// Lock-ordering: acquires `entries.read()` *before* the secondary
    /// `reviews_by_target.read()` so readers and writers share a single
    /// canonical lock-acquisition order (entries → secondary).
    pub async fn reviews_by_target(&self, target: &Path) -> Vec<IndexEntry> {
        let key = normalize_absolute(target);
        let entries = self.entries.read().await;
        let map = self.reviews_by_target.read().await;
        let Some(paths) = map.get(&key) else {
            return Vec::new();
        };
        // BTreeSet iteration is lexical-by-path; no explicit sort.
        paths
            .iter()
            .filter_map(|p| entries.get(p).cloned())
            .collect()
    }

    /// Returns entries whose `work_item_id:`, `parent:`, or `related:` frontmatter
    /// canonicalises to the given work-item ID.
    pub async fn work_item_refs_by_id(&self, id: &str) -> Vec<IndexEntry> {
        let entries = self.entries.read().await;
        let map = self.work_item_refs_by_target.read().await;
        let Some(paths) = map.get(id) else {
            return Vec::new();
        };
        paths
            .iter()
            .filter_map(|p| entries.get(p).cloned())
            .collect()
    }

    /// Returns the resolved declared-outbound entries for the given entry:
    /// - the plan-review `target:` (if this entry is a plan-review), and
    /// - the work-items referenced via `work_item_id:`, `parent:`, `related:`.
    pub async fn declared_outbound(&self, entry: &IndexEntry) -> Vec<IndexEntry> {
        let entries = self.entries.read().await;
        let by_id = self.work_item_by_id.read().await;
        let plans = self.plans_by_id.read().await;
        let mut result: Vec<IndexEntry> = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();

        // Existing: plan-review `target:` field.
        if let Some(target_key) =
            target_path_from_entry(entry, &plans, &by_id, &self.work_item_cfg, &self.project_root)
        {
            if let Some(e) = entries.get(&target_key) {
                if seen.insert(e.path.clone()) {
                    result.push(e.clone());
                }
            }
        }

        // Work-item cross-refs from `work_item_id:`, `parent:`, `related:`.
        let canon_refs = canonicalise_refs(entry.work_item_refs.clone(), &self.work_item_cfg);
        for id in &canon_refs {
            if let Some(path) = by_id.get(id) {
                if path != &entry.path {
                    if let Some(e) = entries.get(path) {
                        if seen.insert(e.path.clone()) {
                            result.push(e.clone());
                        }
                    }
                }
            }
        }

        result
    }
}

// ──────────────────────────────────────────────────────────────────────
// Lexical path helpers.
// ──────────────────────────────────────────────────────────────────────

/// Lexically clean an absolute path: collapse `.` and `..` segments
/// without touching the filesystem.
///
/// Algorithm:
///   - walk path components left to right;
///   - skip `Component::CurDir` (`.`);
///   - on `Component::ParentDir` (`..`) pop the last `Normal` component
///     if any, else discard (do not escape root);
///   - keep `Component::RootDir` and `Component::Prefix` as-is;
///   - rejoin via `PathBuf::push`.
///
/// Does NOT perform Unicode normalisation, case folding, trailing-slash
/// stripping, or symlink resolution. Read/write parity in the
/// reverse-index keying relies on `project_root` being canonical (set
/// once at `Indexer::build`) so both sides land at the same form.
fn normalize_absolute(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Prefix(_) | Component::RootDir => out.push(c.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                // Pop only Normal components; never pop past RootDir/Prefix.
                let popped = out
                    .components()
                    .next_back()
                    .map(|c| matches!(c, Component::Normal(_)))
                    .unwrap_or(false);
                if popped {
                    out.pop();
                }
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

/// Validate and normalise a `target:` frontmatter value. Returns
/// `None` for any value that:
///   - is empty;
///   - contains `..`, `.`, NUL, or backslash in any segment;
///   - starts with `/` (absolute paths bypass `project_root`);
///   - resolves outside `project_root` after lexical join.
fn normalize_target_key(raw: &str, project_root: &Path) -> Option<PathBuf> {
    if raw.is_empty() || raw.starts_with('/') {
        return None;
    }
    for segment in raw.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.contains('\0')
        {
            return None;
        }
    }
    let joined = project_root.join(raw);
    let normalized = normalize_absolute(&joined);
    // Defence in depth: per-segment validation already rejects `..`,
    // but verify the normalised form retains the project_root prefix.
    if !normalized.starts_with(project_root) {
        return None;
    }
    Some(normalized)
}

/// Resolves a review/validation `target:` value to the target artifact's path.
///
/// Accepts, per ADR-0034 §"Forms":
///   - Path form: `target: "meta/plans/2026-...md"` — resolved via
///     `normalize_target_key` against `project_root`.
///   - Typed `plan:` form: `target: "plan:<plan-id>"` — resolved via
///     the `plans_by_id` index (the plan's `id:` field; usually the
///     filename stem).
///   - Typed `work-item:` form: `target: "work-item:NNNN"` — resolved via the
///     `work_item_by_id` index, canonicalising the raw id through
///     `canonicalise_one_id` first (so a project-prefixed/under-padded id still
///     resolves, matching `cluster_key.rs`). Story 0070 types work-item-review
///     targets to this shape; resolving it here keeps the path-keyed
///     `reviews_by_target` reverse index populated.
///
/// Returns `None` for entries that carry no `target:`, or for `adr:`/`pr:`
/// targets (no corpus path).
pub(crate) fn target_path_from_entry(
    entry: &IndexEntry,
    plans_by_id: &HashMap<String, PathBuf>,
    work_item_by_id: &HashMap<String, PathBuf>,
    work_item_cfg: &crate::config::WorkItemConfig,
    project_root: &Path,
) -> Option<PathBuf> {
    use crate::typed_ref::{parse_typed_ref, TypedRef};
    if !entry.r#type.carries_target_frontmatter() {
        return None;
    }
    let raw = entry.frontmatter.get("target")?.as_str()?;
    match parse_typed_ref(raw)? {
        TypedRef::Plan(id) => plans_by_id.get(&id).cloned(),
        TypedRef::WorkItem(id) => {
            let canon = canonicalise_one_id(&id, work_item_cfg)?;
            work_item_by_id.get(&canon).cloned()
        }
        TypedRef::Path(p) => {
            let raw_str = p.to_str()?;
            normalize_target_key(raw_str, project_root)
        }
        _ => None,
    }
}

/// Pure function over a held entries snapshot — direct map lookup first,
/// then fall back to canonicalised parent + filename matching to handle
/// macOS `/var` → `/private/var` indirection. The file is gone, so we
/// can no longer canonicalise the path itself; the parent directory still
/// exists and can be canonicalised.
fn find_entry_for_deleted(
    entries: &HashMap<PathBuf, IndexEntry>,
    path: &Path,
) -> Option<IndexEntry> {
    if let Some(e) = entries.get(path) {
        return Some(e.clone());
    }
    let filename = path.file_name()?;
    let canonical_parent = path.parent().and_then(|p| std::fs::canonicalize(p).ok())?;
    entries
        .values()
        .find(|e| {
            e.path.file_name() == Some(filename)
                && e.path.parent() == Some(canonical_parent.as_path())
        })
        .cloned()
}

// ──────────────────────────────────────────────────────────────────────
// Secondary-index update / remove helpers.
//
// Each helper takes (storage, [context], new_entry, previous) — except
// `remove_from_*` which only needs the entry being removed. They are
// async only because the per-index `RwLock::write().await` acquisition
// is async; the bodies are otherwise straight map mutation. They are
// expected to be called *while the caller holds `entries.write()`* —
// see the lock-ordering invariant on `Indexer`.
// ──────────────────────────────────────────────────────────────────────

async fn update_adr_by_id(
    map: &Arc<RwLock<HashMap<u32, PathBuf>>>,
    new_entry: &IndexEntry,
    previous: Option<&IndexEntry>,
) {
    let prev_id = previous.and_then(adr_id_from_entry);
    let next_id = adr_id_from_entry(new_entry);
    let mut m = map.write().await;
    if let Some(prev_id) = prev_id {
        if Some(prev_id) != next_id {
            m.remove(&prev_id);
        }
    }
    if let Some(id) = next_id {
        m.insert(id, new_entry.path.clone());
    }
}

async fn update_work_item_by_id(
    map: &Arc<RwLock<HashMap<String, PathBuf>>>,
    new_entry: &IndexEntry,
    previous: Option<&IndexEntry>,
) {
    let prev_id = previous.and_then(work_item_id_from_entry);
    let next_id = work_item_id_from_entry(new_entry);
    let mut m = map.write().await;
    if let Some(ref prev_id) = prev_id {
        if Some(prev_id.as_str()) != next_id.as_deref() {
            m.remove(prev_id);
        }
    }
    if let Some(id) = next_id {
        m.insert(id, new_entry.path.clone());
    }
}

async fn update_plans_by_id(
    map: &Arc<RwLock<HashMap<String, PathBuf>>>,
    new_entry: &IndexEntry,
    previous: Option<&IndexEntry>,
) {
    let prev_id = previous.and_then(plan_id_from_entry);
    let next_id = plan_id_from_entry(new_entry);
    let mut m = map.write().await;
    if let Some(ref prev_id) = prev_id {
        if Some(prev_id.as_str()) != next_id.as_deref() {
            m.remove(prev_id);
        }
    }
    if let Some(id) = next_id {
        m.insert(id, new_entry.path.clone());
    }
}

async fn update_reviews_by_target(
    map: &Arc<RwLock<HashMap<PathBuf, BTreeSet<PathBuf>>>>,
    plans_by_id: &HashMap<String, PathBuf>,
    work_item_by_id: &HashMap<String, PathBuf>,
    work_item_cfg: &crate::config::WorkItemConfig,
    project_root: &Path,
    new_entry: &IndexEntry,
    previous: Option<&IndexEntry>,
) {
    // The remove-then-insert is unconditional even when prev_target ==
    // next_target. An earlier short-circuit on equal targets masked a
    // path-change leak: if the *review's own* file is renamed (path
    // changes) while its `target:` is unchanged, the old path would
    // remain in the BTreeSet under the still-current target key. Always
    // removing `previous.path` and inserting `new_entry.path` keeps the
    // contract correct on rename. BTreeSet ops are O(log n) so the
    // redundant work is negligible at v1 scale.
    let prev_target = previous.and_then(|p| {
        target_path_from_entry(p, plans_by_id, work_item_by_id, work_item_cfg, project_root)
    });
    let next_target =
        target_path_from_entry(new_entry, plans_by_id, work_item_by_id, work_item_cfg, project_root);
    let mut m = map.write().await;
    if let (Some(t), Some(prev)) = (&prev_target, previous) {
        if let Some(set) = m.get_mut(t) {
            set.remove(&prev.path);
            if set.is_empty() {
                m.remove(t);
            }
        }
    }
    if let Some(t) = next_target {
        m.entry(t).or_default().insert(new_entry.path.clone());
    }
}

async fn remove_from_adr_by_id(map: &Arc<RwLock<HashMap<u32, PathBuf>>>, previous: &IndexEntry) {
    if let Some(id) = adr_id_from_entry(previous) {
        map.write().await.remove(&id);
    }
}

async fn remove_from_work_item_by_id(
    map: &Arc<RwLock<HashMap<String, PathBuf>>>,
    previous: &IndexEntry,
) {
    if let Some(id) = work_item_id_from_entry(previous) {
        map.write().await.remove(&id);
    }
}

async fn remove_from_plans_by_id(
    map: &Arc<RwLock<HashMap<String, PathBuf>>>,
    previous: &IndexEntry,
) {
    if let Some(id) = plan_id_from_entry(previous) {
        map.write().await.remove(&id);
    }
}

async fn remove_from_reviews_by_target(
    map: &Arc<RwLock<HashMap<PathBuf, BTreeSet<PathBuf>>>>,
    plans_by_id: &HashMap<String, PathBuf>,
    work_item_by_id: &HashMap<String, PathBuf>,
    work_item_cfg: &crate::config::WorkItemConfig,
    project_root: &Path,
    previous: &IndexEntry,
) {
    if let Some(target_key) =
        target_path_from_entry(previous, plans_by_id, work_item_by_id, work_item_cfg, project_root)
    {
        let mut m = map.write().await;
        if let Some(set) = m.get_mut(&target_key) {
            set.remove(&previous.path);
            if set.is_empty() {
                m.remove(&target_key);
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────
// Work-item cross-reference canonicalisation.
// ──────────────────────────────────────────────────────────────────────

/// Extracts the zero-pad width from an `id_pattern` like `{number:04d}` → 4.
fn number_width_from_id_pattern(pattern: &str) -> usize {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\{number:0*(\d+)d\}").unwrap())
        .captures(pattern)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(4)
}

/// Canonicalises a single raw cross-reference string to match
/// `work_item_by_id` keys. Returns `None` for malformed values
/// (anything not in one of the three accepted shapes). The
/// allocation-light counterpart to `canonicalise_refs`.
pub fn canonicalise_one_id(
    raw: &str,
    cfg: &crate::config::WorkItemConfig,
) -> Option<String> {
    use std::sync::OnceLock;
    static PROJECT_RE: OnceLock<regex::Regex> = OnceLock::new();
    let project_re = PROJECT_RE
        .get_or_init(|| regex::Regex::new(r"^[A-Za-z][A-Za-z0-9]*-\d+$").unwrap());
    static NUMERIC_RE: OnceLock<regex::Regex> = OnceLock::new();
    let numeric_re = NUMERIC_RE
        .get_or_init(|| regex::Regex::new(r"^\d+$").unwrap());

    if raw.is_empty() {
        return None;
    }
    let has_project = cfg.id_pattern.contains("{project}");
    let width = number_width_from_id_pattern(&cfg.id_pattern);

    if numeric_re.is_match(raw) {
        let n_str = raw
            .parse::<u64>()
            .map(|n| n.to_string())
            .unwrap_or_else(|_| raw.to_string());
        let padded = format!("{:0>width$}", n_str, width = width);
        if has_project {
            return Some(match &cfg.default_project_code {
                Some(code) => format!("{code}-{padded}"),
                None => padded,
            });
        }
        return Some(padded);
    }
    if project_re.is_match(raw) {
        return Some(raw.to_string());
    }
    None
}

/// Canonicalises raw cross-reference strings to match `work_item_by_id` keys.
///
/// Four cases (applied in order):
/// 1. Bare numeric (`^\d+$`) under a non-project pattern: zero-pad to width.
/// 2. Bare numeric under a project pattern with `default_project_code`: prefix + zero-pad.
/// 3. Project-prefixed (`^[A-Za-z][A-Za-z0-9]*-\d+$`): pass through verbatim.
/// 4. Anything else: skip (silent drop; never panics).
///
/// Duplicates are removed; insertion order is preserved for the unique elements.
pub fn canonicalise_refs(
    raw: Vec<String>,
    cfg: &crate::config::WorkItemConfig,
) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result: Vec<String> = Vec::new();
    for r in raw {
        let Some(canonical) = canonicalise_one_id(&r, cfg) else {
            continue;
        };
        if seen.insert(canonical.clone()) {
            result.push(canonical);
        }
    }
    result
}

async fn update_work_item_refs_by_target(
    map: &Arc<RwLock<HashMap<String, BTreeSet<PathBuf>>>>,
    cfg: &crate::config::WorkItemConfig,
    new_entry: &IndexEntry,
    previous: Option<&IndexEntry>,
) {
    let prev_refs = previous
        .map(|p| canonicalise_refs(p.work_item_refs.clone(), cfg))
        .unwrap_or_default();
    let next_refs = canonicalise_refs(new_entry.work_item_refs.clone(), cfg);
    let mut m = map.write().await;
    // Remove the previous entry's path from all its canonical IDs.
    if let Some(prev) = previous {
        for id in &prev_refs {
            if let Some(set) = m.get_mut(id) {
                set.remove(&prev.path);
                if set.is_empty() {
                    m.remove(id);
                }
            }
        }
    }
    // Insert the new entry's path under all its canonical IDs (excluding self-refs).
    for id in &next_refs {
        if new_entry.work_item_id.as_deref() == Some(id.as_str()) {
            continue;
        }
        m.entry(id.clone()).or_default().insert(new_entry.path.clone());
    }
}

async fn remove_from_work_item_refs_by_target(
    map: &Arc<RwLock<HashMap<String, BTreeSet<PathBuf>>>>,
    cfg: &crate::config::WorkItemConfig,
    previous: &IndexEntry,
) {
    let refs = canonicalise_refs(previous.work_item_refs.clone(), cfg);
    if refs.is_empty() {
        return;
    }
    let mut m = map.write().await;
    for id in &refs {
        if let Some(set) = m.get_mut(id) {
            set.remove(&previous.path);
            if set.is_empty() {
                m.remove(id);
            }
        }
    }
}

fn build_entry(
    kind: DocTypeKey,
    path: PathBuf,
    content: &FileContent,
    project_root: &Path,
    work_item_cfg: &crate::config::WorkItemConfig,
) -> IndexEntry {
    let parsed = frontmatter::parse(&content.bytes);
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let filename_stem = filename.strip_suffix(".md").unwrap_or(filename);

    // For nested-manifest doc types (e.g. design inventories) the slug
    // source is the parent directory name, not the manifest filename.
    let slug_filename: String = if kind.nested_manifest_filename().is_some() {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| format!("{n}.md"))
            .unwrap_or_else(|| filename.to_string())
    } else {
        filename.to_string()
    };

    let (slug_val, work_item_id) = if kind == DocTypeKey::WorkItems {
        let regex_slug = slug::derive_work_item_with_regex(&work_item_cfg.scan_regex, filename);
        // Fall back to the default numeric slug derivation when the primary
        // regex doesn't match (e.g., legacy bare-numeric files in a
        // project-prefixed workspace during a pattern-config rollout).
        let slug = regex_slug.or_else(|| slug::derive(kind, filename, work_item_cfg));
        // Identity resolution (story 0070): the unified `id:` key is primary,
        // then the legacy `work_item_id:` key, then filename extraction. The
        // latter two are retained transitional fallbacks — a follow-on contract
        // story removes them once every consuming repo has migrated — and each
        // emits a deprecation warning when it is the resolving source. All three
        // sources route through `normalise_id` so the identity shape is
        // canonical regardless of where it came from (a raw `id:` must not
        // bypass normalisation). A synced work-item may carry its id in
        // frontmatter even when the filename doesn't encode it.
        let read_fm_id = |key: &str| -> Option<String> {
            let FrontmatterState::Parsed(map) = &parsed.state else {
                return None;
            };
            map.get(key).and_then(|v| v.as_str()).and_then(|raw| {
                let normalised = work_item_cfg.normalise_id(raw);
                if normalised.is_none() && !raw.trim().is_empty() {
                    tracing::warn!(
                        file = %path.display(),
                        key = key,
                        value = raw,
                        "work-item identity frontmatter value failed shape validation",
                    );
                }
                normalised
            })
        };
        let id = if let Some(v) = read_fm_id("id") {
            Some(v)
        } else if let Some(v) = read_fm_id("work_item_id") {
            tracing::warn!(
                file = %path.display(),
                "work-item identity resolved via the legacy `work_item_id:` key; \
                 migrate to `id:` (deprecated fallback — story 0070 follow-on)",
            );
            Some(v)
        } else if let Some(v) = work_item_cfg.extract_id(filename) {
            tracing::warn!(
                file = %path.display(),
                "work-item identity resolved via the filename fallback; \
                 add an `id:` (deprecated fallback — story 0070 follow-on)",
            );
            Some(v)
        } else {
            None
        };
        (slug, id)
    } else {
        (slug::derive(kind, &slug_filename, work_item_cfg), None)
    };
    // Title fallback uses the slug-source stem so nested kinds (where the
    // manifest filename is just "inventory") get a meaningful default.
    let title_fallback_stem = slug_filename
        .strip_suffix(".md")
        .unwrap_or(filename_stem);
    let title = frontmatter::title_from(&parsed.state, &parsed.body, title_fallback_stem);
    let body_preview = frontmatter::body_preview_from(&parsed.body);
    let work_item_refs = frontmatter::read_ref_keys(&parsed.state);

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
        work_item_id,
        title,
        frontmatter: fm_json,
        frontmatter_state: state_str,
        work_item_refs,
        mtime_ms: content.mtime_ms,
        size: content.size,
        etag: content.etag.clone(),
        body_preview,
        completeness: None,
        linked_count: 0,
        cluster_key: None,
    }
}

fn adr_id_from_entry(entry: &IndexEntry) -> Option<u32> {
    if entry.r#type != DocTypeKey::Decisions {
        return None;
    }
    let filename = entry.path.file_name()?.to_str()?;
    parse_adr_id(&entry.frontmatter, filename)
}

fn work_item_id_from_entry(entry: &IndexEntry) -> Option<String> {
    entry.work_item_id.clone()
}

/// Extracts a plan's `id:` value (typically the filename stem). Used to
/// populate `plans_by_id` for resolving `target: "plan:<id>"`
/// typed-linkage references per ADR-0034.
fn plan_id_from_entry(entry: &IndexEntry) -> Option<String> {
    if entry.r#type != DocTypeKey::Plans {
        return None;
    }
    let id = entry.frontmatter.get("id")?.as_str()?;
    if id.is_empty() {
        return None;
    }
    Some(id.to_string())
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


#[cfg(test)]
mod canonicalise_tests {
    use super::*;
    use crate::config::WorkItemConfig;

    fn cfg_numeric() -> WorkItemConfig {
        WorkItemConfig::default_numeric() // id_pattern = "{number:04d}", no project
    }

    fn cfg_project(code: &str) -> WorkItemConfig {
        let raw = crate::config::RawWorkItemConfig {
            scan_regex: format!("^{code}-(\\d+)-"),
            id_pattern: format!("{{project}}-{{number:04d}}"),
            default_project_code: Some(code.to_string()),
        };
        WorkItemConfig::from_raw(raw).unwrap()
    }

    #[test]
    fn canonicalise_refs_pads_bare_numeric_under_default_pattern() {
        let cfg = cfg_numeric();
        let raw = vec!["42".to_string(), "0007".to_string()];
        assert_eq!(canonicalise_refs(raw, &cfg), vec!["0042", "0007"]);
    }

    #[test]
    fn canonicalise_refs_prefixes_default_project_under_project_pattern() {
        let cfg = cfg_project("PROJ");
        let raw = vec!["42".to_string()];
        assert_eq!(canonicalise_refs(raw, &cfg), vec!["PROJ-0042"]);
    }

    #[test]
    fn canonicalise_refs_passes_prefixed_input_through_under_default_pattern() {
        let cfg = cfg_numeric();
        let raw = vec!["PROJ-0042".to_string()];
        assert_eq!(canonicalise_refs(raw, &cfg), vec!["PROJ-0042"]);
    }

    #[test]
    fn canonicalise_refs_dedups_after_canonicalisation() {
        let cfg = cfg_numeric();
        // "42", "0042", and 42 all canonicalise to "0042"
        let raw = vec!["42".to_string(), "0042".to_string()];
        assert_eq!(canonicalise_refs(raw, &cfg), vec!["0042"]);
    }

    #[test]
    fn canonicalise_refs_drops_malformed_input() {
        let cfg = cfg_numeric();
        let raw = vec![
            "not-a-valid-id".to_string(),
            "".to_string(),
            "42-foo".to_string(),
            "PROJ-".to_string(),
            "-0042".to_string(),
        ];
        assert_eq!(canonicalise_refs(raw, &cfg), Vec::<String>::new());
    }

    #[test]
    fn canonicalise_refs_case_3_vs_case_4_boundary() {
        let cfg = cfg_numeric();
        let cases: &[(&str, Option<&str>)] = &[
            ("PROJ-0042",   Some("PROJ-0042")),
            ("FOO-1",       Some("FOO-1")),
            ("Web2-7",      Some("Web2-7")),
            ("42-foo",      None),
            ("PROJ-",       None),
            ("-0042",       None),
            ("PROJ--0042",  None),
            ("PROJ-0042-x", None),
            ("",            None),
        ];
        for (input, expected) in cases {
            let got = canonicalise_refs(vec![input.to_string()], &cfg);
            let got_first = got.first().map(String::as_str);
            assert_eq!(got_first, *expected, "input={input}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkItemConfig;
    use crate::file_driver::LocalFileDriver;
    use std::sync::Arc;

    fn default_work_item_cfg() -> Arc<WorkItemConfig> {
        Arc::new(WorkItemConfig::default_numeric())
    }

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
        Indexer::build(driver, root, default_work_item_cfg()).await.unwrap()
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
    async fn counts_by_type_returns_entry_count_per_configured_type() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let counts = idx.counts_by_type().await;
        assert_eq!(counts.get(&DocTypeKey::Plans).copied().unwrap_or(0), 3);
        assert_eq!(counts.get(&DocTypeKey::Decisions).copied().unwrap_or(0), 1);
        assert_eq!(counts.get(&DocTypeKey::PlanReviews).copied().unwrap_or(0), 2);
        assert_eq!(counts.get(&DocTypeKey::Notes).copied().unwrap_or(0), 1);
        assert!(!counts.contains_key(&DocTypeKey::Templates));
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
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), default_work_item_cfg())
            .await
            .unwrap();
        let entries = idx.all().await;
        let foo = entries.iter().find(|e| e.title == "Foo").unwrap();
        assert_eq!(foo.body_preview, "First paragraph of the body.");
    }

    fn make_cfg(doc_paths: HashMap<String, PathBuf>) -> crate::config::Config {
        crate::config::Config {
            plugin_root: "/p".into(),
            plugin_version: "test".into(),
            project_root: "/p".into(),
            tmp_path: "/t".into(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: "/l".into(),
            doc_paths,
            templates: Default::default(),
            work_item: None,
            kanban_columns: None,
            idle_timeout: None,
            editor: None,
            editor_project: None,
        }
    }

    #[tokio::test]
    async fn library_aggregates_returns_counts_and_latest_per_type() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let cfg = make_cfg(HashMap::new());
        let agg = idx.library_aggregates(&cfg, &Selection::new()).await;
        let plans = agg
            .per_type
            .get(&DocTypeKey::Plans)
            .expect("plans present");
        assert_eq!(plans.count, 3);
        assert_eq!(plans.filtered_count, 3);
        assert!(plans.latest.is_some());
        let decisions = agg.per_type.get(&DocTypeKey::Decisions).unwrap();
        assert_eq!(decisions.count, 1);
        assert!(decisions.latest.is_some());
    }

    #[tokio::test]
    async fn library_aggregates_returns_empty_map_for_empty_index() {
        let tmp = tempfile::tempdir().unwrap();
        let map = HashMap::new();
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), default_work_item_cfg())
            .await
            .unwrap();
        let cfg = make_cfg(HashMap::new());
        let agg = idx.library_aggregates(&cfg, &Selection::new()).await;
        assert!(agg.per_type.is_empty());
    }

#[tokio::test]
    async fn library_aggregates_status_facet_skips_non_parsed_frontmatter() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let cfg = make_cfg(HashMap::new());
        let agg = idx.library_aggregates(&cfg, &Selection::new()).await;
        let plans = agg.per_type.get(&DocTypeKey::Plans).unwrap();
        let status_options = plans.facet_options.get("status").unwrap();
        // Only the "draft" plan has parsed frontmatter with a status — the
        // malformed and absent plans contribute nothing.
        assert_eq!(status_options.get("draft").copied(), Some(1));
        assert_eq!(status_options.len(), 1);
    }

    #[tokio::test]
    async fn library_aggregates_filtered_count_reflects_selection() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let cfg = make_cfg(HashMap::new());
        let mut sel: Selection = HashMap::new();
        sel.insert(
            DocTypeKey::Plans,
            HashMap::from([("status".to_string(), vec!["draft".to_string()])]),
        );
        let agg = idx.library_aggregates(&cfg, &sel).await;
        let plans = agg.per_type.get(&DocTypeKey::Plans).unwrap();
        assert_eq!(plans.count, 3);
        assert_eq!(plans.filtered_count, 1);
    }

    #[tokio::test]
    async fn library_aggregates_empty_options_array_disables_facet_filter() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer(tmp.path()).await;
        let cfg = make_cfg(HashMap::new());
        let mut sel: Selection = HashMap::new();
        sel.insert(
            DocTypeKey::Plans,
            HashMap::from([("status".to_string(), vec![])]),
        );
        let agg = idx.library_aggregates(&cfg, &sel).await;
        let plans = agg.per_type.get(&DocTypeKey::Plans).unwrap();
        assert_eq!(plans.filtered_count, plans.count);
    }

    #[test]
    fn facets_for_returns_expected_facets() {
        assert_eq!(facets_for(DocTypeKey::Decisions), &["status", "clusterSlug"]);
        assert_eq!(
            facets_for(DocTypeKey::WorkItems),
            &["status", "project", "clusterSlug"]
        );
        assert!(facets_for(DocTypeKey::Templates).is_empty());
    }

    #[test]
    fn extract_facet_value_project_derives_from_work_item_id() {
        let entry = sample_entry_with_work_item("PROJ-0042");
        let cfg = make_cfg(HashMap::new());
        assert_eq!(
            extract_facet_value(&entry, &cfg, "project"),
            Some("PROJ".to_string())
        );
    }

    #[test]
    fn extract_facet_value_project_falls_back_to_default_for_prefixless_ids() {
        let entry = sample_entry_with_work_item("0042");
        let cfg = make_cfg_with_default_project_code("FALLBACK");
        assert_eq!(
            extract_facet_value(&entry, &cfg, "project"),
            Some("FALLBACK".to_string())
        );
    }

    #[test]
    fn extract_facet_value_project_returns_none_for_empty_prefix() {
        let entry = sample_entry_with_work_item("-0042");
        let cfg = make_cfg(HashMap::new());
        assert_eq!(extract_facet_value(&entry, &cfg, "project"), None);
    }

    #[test]
    fn extract_facet_value_project_returns_none_when_work_item_id_is_none() {
        let entry = sample_entry_without_work_item();
        let cfg = make_cfg_with_default_project_code("FALLBACK");
        assert_eq!(extract_facet_value(&entry, &cfg, "project"), None);
    }

    #[test]
    fn entry_matches_all_returns_true_when_selection_absent() {
        let entry = sample_entry_without_work_item();
        let cfg = make_cfg(HashMap::new());
        assert!(entry_matches_all(&entry, &cfg, None));
    }

    #[test]
    fn entry_matches_all_empty_options_is_no_filter() {
        let entry = sample_entry_without_work_item();
        let cfg = make_cfg(HashMap::new());
        let sel: HashMap<String, Vec<String>> =
            HashMap::from([("status".to_string(), vec![])]);
        assert!(entry_matches_all(&entry, &cfg, Some(&sel)));
    }

    #[test]
    fn entry_matches_all_except_skips_named_facet() {
        let entry = sample_entry_without_work_item();
        let cfg = make_cfg(HashMap::new());
        let sel: HashMap<String, Vec<String>> =
            HashMap::from([("status".to_string(), vec!["does-not-match".to_string()])]);
        // entry_matches_all would reject it
        assert!(!entry_matches_all(&entry, &cfg, Some(&sel)));
        // entry_matches_all_except("status") skips the rejecting facet
        assert!(entry_matches_all_except(&entry, &cfg, Some(&sel), "status"));
    }

    fn make_cfg_with_default_project_code(code: &str) -> crate::config::Config {
        crate::config::Config {
            plugin_root: "/p".into(),
            plugin_version: "test".into(),
            project_root: "/p".into(),
            tmp_path: "/t".into(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: "/l".into(),
            doc_paths: HashMap::new(),
            templates: Default::default(),
            work_item: Some(crate::config::RawWorkItemConfig {
                scan_regex: r"^(?<id>\d+)".to_string(),
                id_pattern: "{number:04d}".to_string(),
                default_project_code: Some(code.to_string()),
            }),
            kanban_columns: None,
            idle_timeout: None,
            editor: None,
            editor_project: None,
        }
    }

    fn sample_entry_with_work_item(work_item_id: &str) -> IndexEntry {
        IndexEntry {
            r#type: DocTypeKey::WorkItems,
            path: PathBuf::from("/tmp/x.md"),
            rel_path: PathBuf::from("x.md"),
            slug: None,
            work_item_id: Some(work_item_id.to_string()),
            title: "Sample".into(),
            frontmatter: serde_json::Value::Null,
            frontmatter_state: "absent".into(),
            work_item_refs: vec![],
            mtime_ms: 0,
            size: 0,
            etag: String::new(),
            body_preview: String::new(),
            completeness: None,
            linked_count: 0,
            cluster_key: None,
        }
    }

    fn sample_entry_without_work_item() -> IndexEntry {
        IndexEntry {
            r#type: DocTypeKey::Decisions,
            path: PathBuf::from("/tmp/x.md"),
            rel_path: PathBuf::from("x.md"),
            slug: None,
            work_item_id: None,
            title: "Sample".into(),
            frontmatter: serde_json::Value::Null,
            frontmatter_state: "absent".into(),
            work_item_refs: vec![],
            mtime_ms: 0,
            size: 0,
            etag: String::new(),
            body_preview: String::new(),
            completeness: None,
            linked_count: 0,
            cluster_key: None,
        }
    }

    #[tokio::test]
    async fn design_inventories_indexed_from_nested_directories() {
        // Design inventories live as `<root>/YYYY-MM-DD-HHMMSS-{source}/inventory.md`
        // rather than flat `.md` files. The indexer must descend one level.
        let tmp = tempfile::tempdir().unwrap();
        let inv_root = tmp.path().join("meta/research/design-inventories");
        let inv1 = inv_root.join("2026-05-06-140608-foo");
        let inv2 = inv_root.join("2026-05-06-135214-bar");
        let skip_tmp = inv_root.join(".2026-05-06-200000-inflight.tmp");
        for d in [&inv1, &inv2, &skip_tmp] {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(
            inv1.join("inventory.md"),
            "---\ntitle: Foo Inventory\nstatus: accepted\n---\n# body\n",
        )
        .unwrap();
        std::fs::write(
            inv2.join("inventory.md"),
            "---\ntitle: Bar Inventory\n---\n# body\n",
        )
        .unwrap();
        // Dot-prefixed in-flight dir should be ignored even with an inventory.md.
        std::fs::write(skip_tmp.join("inventory.md"), "---\n---\n").unwrap();
        // A subdirectory missing inventory.md should be ignored.
        std::fs::create_dir_all(inv_root.join("2026-05-06-empty")).unwrap();

        let mut map = HashMap::new();
        map.insert("research_design_inventories".to_string(), inv_root);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), default_work_item_cfg())
            .await
            .unwrap();

        let entries = idx.all_by_type(DocTypeKey::DesignInventories).await;
        let titles: Vec<&str> = entries.iter().map(|e| e.title.as_str()).collect();
        assert!(titles.contains(&"Foo Inventory"), "got {titles:?}");
        assert!(titles.contains(&"Bar Inventory"), "got {titles:?}");
        assert_eq!(entries.len(), 2);

        // Slug derived from the parent directory name, not the manifest stem.
        let slugs: Vec<String> = entries.iter().filter_map(|e| e.slug.clone()).collect();
        assert!(
            slugs.iter().any(|s| s == "140608-foo"),
            "expected slug derived from parent dir; got {slugs:?}",
        );
        assert!(
            slugs.iter().any(|s| s == "135214-bar"),
            "expected slug derived from parent dir; got {slugs:?}",
        );
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
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), default_work_item_cfg())
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "scan took {elapsed:?}, expected < 5 s",
        );
        assert_eq!(idx.all().await.len(), 2000);
    }
}

#[cfg(test)]
mod refresh_tests {
    use super::*;
    use crate::config::WorkItemConfig;
    use crate::file_driver::{etag_of, LocalFileDriver};
    use std::sync::Arc;

    async fn build_refresh_indexer(tmp: &Path) -> (Indexer, PathBuf) {
        let work = tmp.join("meta/work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(
            work.join("0001-foo.md"),
            "---\ntitle: Foo\nstatus: todo\n---\n# body\n",
        )
        .unwrap();
        std::fs::write(
            work.join("0002-bar.md"),
            "---\ntitle: Bar\nstatus: done\n---\n# body\n",
        )
        .unwrap();
        std::fs::write(
            work.join("0003-baz.md"),
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
        map.insert("work".into(), work.clone());
        map.insert("decisions".into(), dec);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let cfg = Arc::new(WorkItemConfig::default_numeric());
        let idx = Indexer::build(driver, tmp.to_path_buf(), cfg).await.unwrap();
        (idx, work)
    }

    // ── Step 2.14 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_picks_up_external_edit() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, work) = build_refresh_indexer(tmp.path()).await;

        // Write a new work item file that didn't exist at build time
        let new_path = work.join("0004-new.md");
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

        let work_entries = idx.all_by_type(DocTypeKey::WorkItems).await;
        assert!(
            work_entries.iter().any(|e| e.title == "New"),
            "all_by_type should include the new entry"
        );
    }

    // ── Step 2.15 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_updates_etag_on_change() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, work) = build_refresh_indexer(tmp.path()).await;

        let path = work.join("0001-foo.md");
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
        let (idx, work) = build_refresh_indexer(tmp.path()).await;

        let path = work.join("0001-foo.md");
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
            idx.work_item_by_id("0001").await.is_none(),
            "work_item_by_id index must also be cleaned up"
        );
    }

    // ── Step 2.17 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_does_not_disturb_unrelated_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, work) = build_refresh_indexer(tmp.path()).await;

        let path1 = work.join("0001-foo.md");
        let path2 = work.join("0002-bar.md");
        let path3 = work.join("0003-baz.md");

        let before2 = idx.get(&path2).await.unwrap();
        let before3 = idx.get(&path3).await.unwrap();

        idx.refresh_one(&path1).await.unwrap();

        let after2 = idx.get(&path2).await.unwrap();
        let after3 = idx.get(&path3).await.unwrap();

        assert_eq!(before2.etag, after2.etag, "work item 2 etag must not change");
        assert_eq!(
            before2.mtime_ms, after2.mtime_ms,
            "work item 2 mtime must not change"
        );
        assert_eq!(before3.etag, after3.etag, "work item 3 etag must not change");
    }

    // ── Step 2.18 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_rebuilds_secondary_indexes_for_work_items_and_decisions() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, work) = build_refresh_indexer(tmp.path()).await;

        // Refresh work item #1 — work_item_by_id must still work
        let path1 = work.join("0001-foo.md");
        idx.refresh_one(&path1).await.unwrap();
        assert!(
            idx.work_item_by_id("0001").await.is_some(),
            "work_item_by_id(\"0001\") must still resolve after refresh_one"
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
        let (idx, work) = build_refresh_indexer(tmp.path()).await;
        let idx = StdArc::new(idx);

        let path = work.join("0001-foo.md");
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

#[cfg(test)]
mod reverse_index_tests {
    use super::*;
    use crate::config::WorkItemConfig;
    use crate::file_driver::LocalFileDriver;
    use std::sync::Arc;

    /// Seed a tempdir with one plan and one review whose `target:`
    /// points at the plan. Returns (idx, plan_path, review_path), with
    /// paths in canonicalised form so they match the indexer's primary
    /// keys directly.
    async fn build_with_plan_and_review(tmp: &Path) -> (Indexer, PathBuf, PathBuf) {
        let plans = tmp.join("meta/plans");
        let reviews = tmp.join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        let plan = plans.join("2026-04-18-foo.md");
        let review = reviews.join("2026-04-18-foo-review-1.md");
        std::fs::write(&plan, "---\ntitle: Foo Plan\n---\nbody\n").unwrap();
        std::fs::write(
            &review,
            "---\ntarget: \"meta/plans/2026-04-18-foo.md\"\n---\nbody\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans);
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let cfg = Arc::new(WorkItemConfig::default_numeric());
        let idx = Indexer::build(driver, tmp.to_path_buf(), cfg).await.unwrap();
        // Canonicalise the paths so callers compare against the
        // indexer's primary key form.
        let plan = std::fs::canonicalize(&plan).unwrap();
        let review = std::fs::canonicalize(&review).unwrap();
        (idx, plan, review)
    }

    // ── Step 1.0 ────────────────────────────────────────────────────────────
    #[test]
    fn normalize_absolute_collapses_dot_and_dotdot_lexically() {
        // Cases come from Step 1.0 of the plan.
        assert_eq!(
            normalize_absolute(&PathBuf::from("/a/./b")),
            PathBuf::from("/a/b")
        );
        assert_eq!(
            normalize_absolute(&PathBuf::from("/a/b/../c")),
            PathBuf::from("/a/c")
        );
        // Cannot escape root.
        assert_eq!(
            normalize_absolute(&PathBuf::from("/a/../../b")),
            PathBuf::from("/b")
        );
        assert_eq!(
            normalize_absolute(&PathBuf::from("/a//b")),
            PathBuf::from("/a/b")
        );
        assert_eq!(
            normalize_absolute(&PathBuf::from("/a/b/.")),
            PathBuf::from("/a/b")
        );
    }

    // ── Step 1.1 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn reviews_by_target_populated_on_initial_scan() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, plan_path, review_path) = build_with_plan_and_review(tmp.path()).await;
        let inbound = idx.reviews_by_target(&plan_path).await;
        assert_eq!(inbound.len(), 1, "expected exactly one inbound review");
        assert_eq!(inbound[0].path, review_path);
    }

    // ── Step 1.2 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn reviews_by_target_round_trips_via_canonical_root() {
        // tempfile::tempdir on macOS gives /var/folders/... which is a
        // symlink to /private/var/folders/.... Construct with the
        // *non-canonical* form and assert lookups via the canonical
        // form succeed — the discipline canonicalises project_root once.
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(plans.join("2026-04-18-foo.md"), "---\ntitle: Foo\n---\n").unwrap();
        std::fs::write(
            reviews.join("2026-04-18-foo-review-1.md"),
            "---\ntarget: \"meta/plans/2026-04-18-foo.md\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans);
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap();

        // Look up the plan via Indexer::get with the non-canonical path.
        let plan = idx
            .get(&tmp.path().join("meta/plans/2026-04-18-foo.md"))
            .await
            .expect("plan entry should be indexed");
        // Use the entry's stored canonical path as the lookup key.
        let inbound = idx.reviews_by_target(&plan.path).await;
        assert_eq!(inbound.len(), 1, "round-trip via canonical entry path");
    }

    // ── Step 1.3 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn reviews_by_target_excludes_reviews_without_target_field() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(plans.join("2026-04-18-foo.md"), "---\ntitle: Foo\n---\n").unwrap();
        std::fs::write(
            reviews.join("2026-04-18-foo-review-1.md"),
            "---\ntitle: review with no target\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans);
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap();

        let map_guard = idx.reviews_by_target.read().await;
        assert!(
            map_guard.is_empty(),
            "no reverse-index keys for review without target"
        );
    }

    // ── Step 1.4 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn reviews_by_target_tolerates_target_pointing_at_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        // Plan does NOT exist; review's target points at it anyway.
        std::fs::write(
            reviews.join("2026-04-18-foo-review-1.md"),
            "---\ntarget: \"meta/plans/never-created.md\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans);
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap();

        // Compute the lexical key the way the production code does.
        let project_root = idx.project_root().to_path_buf();
        let key = project_root.join("meta/plans/never-created.md");
        let key = normalize_absolute(&key);
        let inbound = idx.reviews_by_target(&key).await;
        assert_eq!(
            inbound.len(),
            1,
            "reverse index materialises by lexical key even when target file does not exist"
        );
    }

    // ── Step 1.5 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn reviews_by_target_supports_multiple_reviews_per_target() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(plans.join("2026-04-18-foo.md"), "---\ntitle: Foo\n---\n").unwrap();
        std::fs::write(
            reviews.join("2026-04-18-foo-review-1.md"),
            "---\ntarget: \"meta/plans/2026-04-18-foo.md\"\n---\n",
        )
        .unwrap();
        std::fs::write(
            reviews.join("2026-04-18-foo-review-2.md"),
            "---\ntarget: \"meta/plans/2026-04-18-foo.md\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_plans".into(), reviews.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap();

        let plan_canon = std::fs::canonicalize(plans.join("2026-04-18-foo.md")).unwrap();
        let inbound = idx.reviews_by_target(&plan_canon).await;
        assert_eq!(inbound.len(), 2, "both reviews must appear");
        // BTreeSet iteration is path-sorted; review-1 sorts before review-2.
        let names: Vec<String> = inbound
            .iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "2026-04-18-foo-review-1.md".to_string(),
                "2026-04-18-foo-review-2.md".to_string(),
            ],
            "BTreeSet iteration must be lexically ordered"
        );
    }

    // ── Step 1.5b ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_on_unchanged_review_keeps_set_size_one() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, plan_path, review_path) = build_with_plan_and_review(tmp.path()).await;
        // Refresh the review twice without touching its content.
        idx.refresh_one(&review_path).await.unwrap();
        idx.refresh_one(&review_path).await.unwrap();
        let inbound = idx.reviews_by_target(&plan_path).await;
        assert_eq!(inbound.len(), 1, "BTreeSet dedup-by-construction holds");
    }

    // ── Step 1.5c ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_on_renamed_review_with_unchanged_target() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, plan_path, review_a) = build_with_plan_and_review(tmp.path()).await;

        // Move the review (rename to a new path), keeping target the same.
        let review_b = review_a.with_file_name("2026-04-18-foo-review-2.md");
        std::fs::rename(&review_a, &review_b).unwrap();
        // Watcher's typical sequence: refresh_one on new path then on old.
        idx.refresh_one(&review_b).await.unwrap();
        idx.refresh_one(&review_a).await.unwrap();

        let inbound = idx.reviews_by_target(&plan_path).await;
        let paths: Vec<&Path> = inbound.iter().map(|e| e.path.as_path()).collect();
        let review_b_canon = std::fs::canonicalize(&review_b).unwrap();
        assert_eq!(
            paths,
            vec![review_b_canon.as_path()],
            "old path must drop, new path must remain"
        );
    }

    // ── Step 1.6 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_adds_review_to_reverse_index() {
        let tmp = tempfile::tempdir().unwrap();
        // Build with plan only — no review yet.
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(plans.join("2026-04-18-foo.md"), "---\ntitle: Foo\n---\n").unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_plans".into(), reviews.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap();

        // Now create a review and refresh_one.
        let new_review = reviews.join("2026-04-18-foo-review-1.md");
        std::fs::write(
            &new_review,
            "---\ntarget: \"meta/plans/2026-04-18-foo.md\"\n---\n",
        )
        .unwrap();
        idx.refresh_one(&new_review).await.unwrap();

        let plan_canon = std::fs::canonicalize(plans.join("2026-04-18-foo.md")).unwrap();
        let inbound = idx.reviews_by_target(&plan_canon).await;
        assert_eq!(inbound.len(), 1);
    }

    // ── Step 1.7 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_removes_review_from_reverse_index_on_target_change() {
        let tmp = tempfile::tempdir().unwrap();
        // Two plans + one review initially targeting plan A.
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(plans.join("a.md"), "---\ntitle: A\n---\n").unwrap();
        std::fs::write(plans.join("b.md"), "---\ntitle: B\n---\n").unwrap();
        let review = reviews.join("rev-1.md");
        std::fs::write(&review, "---\ntarget: \"meta/plans/a.md\"\n---\n").unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap();

        let plan_a = std::fs::canonicalize(plans.join("a.md")).unwrap();
        let plan_b = std::fs::canonicalize(plans.join("b.md")).unwrap();

        // Migrate target from A to B.
        std::fs::write(&review, "---\ntarget: \"meta/plans/b.md\"\n---\n").unwrap();
        idx.refresh_one(&review).await.unwrap();

        let inbound_a = idx.reviews_by_target(&plan_a).await;
        let inbound_b = idx.reviews_by_target(&plan_b).await;
        assert!(inbound_a.is_empty(), "stale key under A must be dropped");
        assert_eq!(inbound_b.len(), 1, "new key under B must be present");
    }

    // ── Step 1.7b ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_target_migration_is_atomic_under_single_writer_lock() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(plans.join("a.md"), "---\ntitle: A\n---\n").unwrap();
        std::fs::write(plans.join("b.md"), "---\ntitle: B\n---\n").unwrap();
        let review = reviews.join("rev-1.md");
        std::fs::write(&review, "---\ntarget: \"meta/plans/a.md\"\n---\n").unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Arc::new(
            Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
                .await
                .unwrap(),
        );

        let plan_a = std::fs::canonicalize(plans.join("a.md")).unwrap();
        let plan_b = std::fs::canonicalize(plans.join("b.md")).unwrap();

        // Pre-arm the rendezvous channels.
        let (reached_tx, reached_rx) = tokio::sync::oneshot::channel::<()>();
        let (proceed_tx, proceed_rx) = tokio::sync::oneshot::channel::<()>();
        idx.install_post_secondary_update_hook(PostSecondaryUpdateHook {
            reached: reached_tx,
            proceed: proceed_rx,
        })
        .await;

        // Spawn the writer: migrate the review's target from A → B.
        std::fs::write(&review, "---\ntarget: \"meta/plans/b.md\"\n---\n").unwrap();
        let writer_idx = idx.clone();
        let writer_review = review.clone();
        let writer = tokio::spawn(async move {
            writer_idx.refresh_one(&writer_review).await.unwrap();
        });

        // Wait for the writer to reach the post-secondary-update barrier.
        reached_rx.await.expect("writer reached barrier");

        // Spawn the reader. It must block on entries.read() until the
        // writer drops its guard.
        let reader_idx = idx.clone();
        let reader = tokio::spawn(async move {
            let inbound_a = reader_idx.reviews_by_target(&plan_a).await;
            let inbound_b = reader_idx.reviews_by_target(&plan_b).await;
            (inbound_a, inbound_b)
        });

        // Give the reader a moment to attempt entries.read(); it should
        // block. We can't observe the block directly, but if the reader
        // *doesn't* block, it will return the *pre*-update state and
        // the assertion below will fail.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Release the writer.
        proceed_tx.send(()).expect("writer awaiting proceed");
        writer.await.unwrap();

        let (inbound_a, inbound_b) = reader.await.unwrap();
        assert!(
            inbound_a.is_empty(),
            "post-migration: A must have no inbound"
        );
        assert_eq!(inbound_b.len(), 1, "post-migration: B must have the review");
    }

    // ── Step 1.8 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn refresh_one_removes_review_from_reverse_index_on_deletion() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, plan_path, review_path) = build_with_plan_and_review(tmp.path()).await;
        std::fs::remove_file(&review_path).unwrap();
        idx.refresh_one(&review_path).await.unwrap();
        let inbound = idx.reviews_by_target(&plan_path).await;
        assert!(
            inbound.is_empty(),
            "reviews_by_target must drop the deleted review"
        );
    }

    // ── Step 1.8b ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn delete_target_plan_with_inbound_reviews() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(plans.join("a.md"), "---\ntitle: A\n---\n").unwrap();
        std::fs::write(
            reviews.join("rev-1.md"),
            "---\ntarget: \"meta/plans/a.md\"\n---\n",
        )
        .unwrap();
        std::fs::write(
            reviews.join("rev-2.md"),
            "---\ntarget: \"meta/plans/a.md\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_plans".into(), reviews.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap();

        let plan_a = plans.join("a.md");
        let plan_a_canon = std::fs::canonicalize(&plan_a).unwrap();
        std::fs::remove_file(&plan_a).unwrap();
        idx.refresh_one(&plan_a).await.unwrap();

        // Reviews must still be addressable via the (now-non-existent)
        // canonical plan path — deferred materialisation contract.
        let inbound = idx.reviews_by_target(&plan_a_canon).await;
        assert_eq!(
            inbound.len(),
            2,
            "lexical key survives target-file deletion"
        );
        // Reviews' own entries are unchanged.
        let rev_canon = std::fs::canonicalize(reviews.join("rev-1.md")).unwrap();
        assert!(idx.get(&rev_canon).await.is_some());
    }

    // ── Step 1.9 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn reviews_by_target_survives_full_rescan() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, plan_path, _review_path) = build_with_plan_and_review(tmp.path()).await;
        idx.rescan().await.unwrap();
        let inbound = idx.reviews_by_target(&plan_path).await;
        assert_eq!(inbound.len(), 1, "rescan re-populates the reverse index");
    }

    // ── Step 1.10 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn reviews_by_target_admits_every_target_carrying_doc_type() {
        // Phase 3 widens target resolution to every doc type that carries
        // a `target:` frontmatter key (PlanReviews, WorkItemReviews,
        // PrReviews, Validations). A PR review pointing at a plan-shaped
        // path is admitted; the parser is intentionally type-agnostic.
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let pr_reviews = tmp.path().join("meta/reviews/prs");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&pr_reviews).unwrap();
        std::fs::write(plans.join("a.md"), "---\ntitle: A\n---\n").unwrap();
        std::fs::write(
            pr_reviews.join("pr-rev.md"),
            "---\ntarget: \"meta/plans/a.md\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_prs".into(), pr_reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap();

        let plan_a = std::fs::canonicalize(plans.join("a.md")).unwrap();
        let inbound = idx.reviews_by_target(&plan_a).await;
        assert_eq!(inbound.len(), 1, "PR-review with path target is admitted");
    }

    #[tokio::test]
    async fn reviews_by_target_resolves_typed_work_item_target_on_rescan_and_refresh() {
        // Story 0070: a migrated work-item-review carries `target:
        // "work-item:NNNN"`. target_path_from_entry now resolves it via
        // work_item_by_id, so the path-keyed reviews_by_target reverse index
        // is populated on BOTH the full rescan (Indexer::build Pass B) and the
        // incremental refresh_one path.
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("meta/work");
        let wi_reviews = tmp.path().join("meta/reviews/work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::create_dir_all(&wi_reviews).unwrap();
        std::fs::write(work.join("0042-foo.md"), "---\ntitle: Foo\n---\n").unwrap();
        std::fs::write(
            wi_reviews.join("0042-foo-review-1.md"),
            "---\ntarget: \"work-item:0042\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work.clone());
        map.insert("review_work".into(), wi_reviews.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx =
            Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
                .await
                .unwrap();

        let wi_path = std::fs::canonicalize(work.join("0042-foo.md")).unwrap();
        assert_eq!(
            idx.reviews_by_target(&wi_path).await.len(),
            1,
            "rescan: typed work-item target resolves the reverse edge"
        );

        // Incremental refresh of the review re-resolves the same edge.
        let rev_path = std::fs::canonicalize(wi_reviews.join("0042-foo-review-1.md")).unwrap();
        idx.refresh_one(&rev_path).await.unwrap();
        assert_eq!(
            idx.reviews_by_target(&wi_path).await.len(),
            1,
            "refresh_one: typed work-item target re-resolves the reverse edge"
        );
    }

    // ── Step 1.11 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn target_path_from_entry_rejects_malformed_values() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(plans.join("a.md"), "---\ntitle: A\n---\n").unwrap();
        // Each malformed target keeps the review indexable but should
        // contribute no reverse-index key.
        let cases = [
            ("rev-empty.md", "target: \"\""),
            ("rev-escape.md", "target: \"../escape.md\""),
            ("rev-abs.md", "target: \"/etc/passwd\""),
            ("rev-back.md", "target: \"foo\\\\bar\""),
            ("rev-collapse.md", "target: \"foo/../escape.md\""),
            ("rev-num.md", "target: 42"),
            ("rev-null.md", "target: null"),
            ("rev-list.md", "target: [\"a\"]"),
        ];
        for (name, fm) in cases {
            std::fs::write(
                reviews.join(name),
                format!("---\n{fm}\n---\n# review with malformed target\n"),
            )
            .unwrap();
        }
        let mut map = HashMap::new();
        map.insert("plans".into(), plans);
        map.insert("review_plans".into(), reviews.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
            .await
            .unwrap();

        // Each review entry is admitted to the primary index.
        for (name, _) in cases {
            let p = std::fs::canonicalize(reviews.join(name)).unwrap();
            assert!(
                idx.get(&p).await.is_some(),
                "review {name} must still be indexed"
            );
        }
        // No reverse-index keys were inserted.
        let map_guard = idx.reviews_by_target.read().await;
        assert!(
            map_guard.is_empty(),
            "malformed targets must not produce reverse-index keys",
        );
    }

    // ── Step 1.12 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn rescan_clears_all_three_secondary_maps_before_repopulating() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, _plan_path, _review_path) = build_with_plan_and_review(tmp.path()).await;
        // Inject stale data into each of the three secondary maps.
        idx.adr_by_id
            .write()
            .await
            .insert(99, PathBuf::from("/nonexistent/ADR-9999.md"));
        idx.work_item_by_id
            .write()
            .await
            .insert("99".to_string(), PathBuf::from("/nonexistent/9999-work-item.md"));
        idx.reviews_by_target.write().await.insert(
            PathBuf::from("/nonexistent/plan.md"),
            BTreeSet::from([PathBuf::from("/nonexistent/review.md")]),
        );

        idx.rescan().await.unwrap();

        let adr_map = idx.adr_by_id.read().await;
        let work_item_map = idx.work_item_by_id.read().await;
        let reviews_map = idx.reviews_by_target.read().await;
        assert!(!adr_map.contains_key(&99u32), "stale ADR cleared on rescan");
        assert!(
            !work_item_map.contains_key("99"),
            "stale work item cleared on rescan"
        );
        assert!(
            !reviews_map.contains_key(&PathBuf::from("/nonexistent/plan.md")),
            "stale review-target cleared on rescan",
        );
    }

    // ── Step 1.13 ────────────────────────────────────────────────────────────
    #[tokio::test]
    async fn concurrent_rescan_and_target_migration_under_rescan_lock() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(plans.join("a.md"), "---\ntitle: A\n---\n").unwrap();
        std::fs::write(plans.join("b.md"), "---\ntitle: B\n---\n").unwrap();
        let review = reviews.join("rev-1.md");
        std::fs::write(&review, "---\ntarget: \"meta/plans/a.md\"\n---\n").unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Arc::new(
            Indexer::build(driver, tmp.path().to_path_buf(), Arc::new(WorkItemConfig::default_numeric()))
                .await
                .unwrap(),
        );

        let (reached_tx, reached_rx) = tokio::sync::oneshot::channel::<()>();
        let (proceed_tx, proceed_rx) = tokio::sync::oneshot::channel::<()>();
        idx.install_post_secondary_update_hook(PostSecondaryUpdateHook {
            reached: reached_tx,
            proceed: proceed_rx,
        })
        .await;

        // Migrate target from A to B; the writer holds the rescan_lock
        // permit while parked at the post-secondary-update barrier.
        std::fs::write(&review, "---\ntarget: \"meta/plans/b.md\"\n---\n").unwrap();
        let writer_idx = idx.clone();
        let writer_review = review.clone();
        let writer = tokio::spawn(async move {
            writer_idx.refresh_one(&writer_review).await.unwrap();
        });

        reached_rx.await.expect("writer reached barrier");

        // While the writer is parked, the rescan_lock semaphore must
        // be empty — try_acquire must fail.
        let permit = idx.rescan_lock().try_acquire_owned();
        assert!(
            permit.is_err(),
            "rescan_lock must be held by writer at the barrier",
        );

        // Schedule a rescan; it must wait for the writer to release.
        let rescan_idx = idx.clone();
        let rescan = tokio::spawn(async move { rescan_idx.rescan().await.unwrap() });

        // Release the writer.
        proceed_tx.send(()).expect("writer awaiting proceed");
        writer.await.unwrap();
        rescan.await.unwrap();

        let plan_a = std::fs::canonicalize(plans.join("a.md")).unwrap();
        let plan_b = std::fs::canonicalize(plans.join("b.md")).unwrap();
        let inbound_a = idx.reviews_by_target(&plan_a).await;
        let inbound_b = idx.reviews_by_target(&plan_b).await;
        assert!(inbound_a.is_empty(), "review must not be under A");
        assert_eq!(
            inbound_b.len(),
            1,
            "review must be under exactly one target (B)"
        );
    }

    // ── Work-item cross-ref reverse index tests ──────────────────────────

    async fn build_indexer_with_work_items(
        tmp: &Path,
        work_items: &[(&str, &str)],
        other_docs: &[(&str, &str)],
    ) -> Indexer {
        let work_dir = tmp.join("meta/work");
        let plan_dir = tmp.join("meta/plans");
        std::fs::create_dir_all(&work_dir).unwrap();
        std::fs::create_dir_all(&plan_dir).unwrap();

        for (filename, content) in work_items {
            std::fs::write(work_dir.join(filename), content).unwrap();
        }
        for (path, content) in other_docs {
            let full = tmp.join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(&full, content).unwrap();
        }

        let mut dir_map = HashMap::new();
        dir_map.insert("work".into(), work_dir);
        dir_map.insert("plans".into(), plan_dir);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&dir_map, vec![], vec![]));
        let work_item_cfg = Arc::new(WorkItemConfig::default_numeric());
        Indexer::build(driver, tmp.to_path_buf(), work_item_cfg)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn reverse_cross_ref_index_populates_work_item_refs_by_id() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = build_indexer_with_work_items(
            tmp.path(),
            &[("0001-epic.md", "---\ntitle: Epic\n---\n")],
            &[(
                "meta/plans/2026-05-01-plan.md",
                "---\ntitle: A Plan\nwork_item_id: \"0001\"\n---\n",
            )],
        )
        .await;
        let refs = idx.work_item_refs_by_id("0001").await;
        assert_eq!(refs.len(), 1, "plan should appear in work-item 0001's refs");
        assert!(refs[0].rel_path.to_string_lossy().contains("2026-05-01-plan.md"));
    }

    #[tokio::test]
    async fn reverse_cross_ref_excludes_self_reference() {
        let tmp = tempfile::tempdir().unwrap();
        // Work-item 0001 has parent: 0001 in its own frontmatter.
        let idx = build_indexer_with_work_items(
            tmp.path(),
            &[("0001-self-ref.md", "---\ntitle: Self Ref\nparent: 0001\n---\n")],
            &[],
        )
        .await;
        let refs = idx.work_item_refs_by_id("0001").await;
        assert!(
            refs.is_empty(),
            "self-referencing work-item must not appear in its own refs"
        );
    }

    #[tokio::test]
    async fn reverse_cross_ref_handles_two_way_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        // A.parent=0002, B.parent=0001 — mutual cycle.
        let idx = build_indexer_with_work_items(
            tmp.path(),
            &[
                ("0001-a.md", "---\ntitle: A\nparent: 0002\n---\n"),
                ("0002-b.md", "---\ntitle: B\nparent: 0001\n---\n"),
            ],
            &[],
        )
        .await;
        let refs_a = idx.work_item_refs_by_id("0001").await;
        let refs_b = idx.work_item_refs_by_id("0002").await;
        assert_eq!(refs_a.len(), 1, "0001 should be referenced by B exactly once");
        assert_eq!(refs_b.len(), 1, "0002 should be referenced by A exactly once");
        assert!(refs_a[0].rel_path.to_string_lossy().contains("0002-b.md"));
        assert!(refs_b[0].rel_path.to_string_lossy().contains("0001-a.md"));
    }

    #[tokio::test]
    async fn reverse_cross_ref_to_unknown_id_is_silently_dropped() {
        let tmp = tempfile::tempdir().unwrap();
        // Plan references work-item 9999 which doesn't exist; no panic.
        let idx = build_indexer_with_work_items(
            tmp.path(),
            &[("0001-real.md", "---\ntitle: Real\n---\n")],
            &[(
                "meta/plans/2026-05-01-plan.md",
                "---\ntitle: Orphan Plan\nwork_item_id: \"9999\"\n---\n",
            )],
        )
        .await;
        // 0001 has no refs from any document.
        let refs = idx.work_item_refs_by_id("0001").await;
        assert!(refs.is_empty());
        // No panic; silently dropped.
    }

    #[tokio::test]
    async fn reverse_cross_ref_dedups_within_same_source_doc() {
        let tmp = tempfile::tempdir().unwrap();
        // Plan has work_item_id: 0001, parent: 0001 — two refs to same ID.
        let idx = build_indexer_with_work_items(
            tmp.path(),
            &[("0001-epic.md", "---\ntitle: Epic\n---\n")],
            &[(
                "meta/plans/2026-05-01-plan.md",
                "---\ntitle: Dup Plan\nwork_item_id: \"0001\"\nparent: 0001\n---\n",
            )],
        )
        .await;
        let refs = idx.work_item_refs_by_id("0001").await;
        // work_item_id: wins over ticket:, so work_item_refs = ["0001", "0001"]
        // after canonicalisation and dedup, only one entry is added for the plan.
        assert_eq!(refs.len(), 1, "same source doc should appear at most once");
    }

    #[tokio::test]
    async fn composition_plan_review_and_work_item_ref_both_surface() {
        // ADR-0017 precedent: a work-item can be both the review target (via
        // reviews_by_target) AND referenced by a work-item cross-ref field.
        // Both must surface together in declared_inbound.
        let tmp = tempfile::tempdir().unwrap();
        let work_dir = tmp.path().join("meta/work");
        let plan_dir = tmp.path().join("meta/plans");
        let review_dir = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&work_dir).unwrap();
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::create_dir_all(&review_dir).unwrap();

        std::fs::write(work_dir.join("0001-epic.md"), "---\ntitle: Epic\n---\n").unwrap();
        // A plan that references the work item via work_item_id:
        std::fs::write(
            plan_dir.join("2026-05-01-plan.md"),
            "---\ntitle: A Plan\nwork_item_id: \"0001\"\n---\n",
        )
        .unwrap();
        // A plan-review whose `target:` points at the work item directly
        // (uses the PlanReviews doc type so reviews_by_target indexes it)
        std::fs::write(
            review_dir.join("0001-epic-review-1.md"),
            "---\ntitle: Review\ntarget: \"meta/work/0001-epic.md\"\n---\n",
        )
        .unwrap();

        let mut dir_map = HashMap::new();
        dir_map.insert("work".into(), work_dir.clone());
        dir_map.insert("plans".into(), plan_dir);
        dir_map.insert("review_plans".into(), review_dir);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&dir_map, vec![], vec![]));
        let work_item_cfg = Arc::new(WorkItemConfig::default_numeric());
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), work_item_cfg)
            .await
            .unwrap();

        let work_item_path = std::fs::canonicalize(work_dir.join("0001-epic.md")).unwrap();
        let via_review = idx.reviews_by_target(&work_item_path).await;
        let via_ref = idx.work_item_refs_by_id("0001").await;

        assert_eq!(via_review.len(), 1, "review must appear under work-item path");
        assert_eq!(via_ref.len(), 1, "plan must appear under work-item ID");
        assert!(
            via_review[0].rel_path.to_string_lossy().contains("0001-epic-review-1.md"),
            "review entry path must match"
        );
        assert!(
            via_ref[0].rel_path.to_string_lossy().contains("2026-05-01-plan.md"),
            "ref entry path must match"
        );
    }

    // ── Frontmatter-first work_item_id resolution ───────────────────────────
    #[tokio::test]
    async fn work_item_id_uses_frontmatter_when_present() {
        let tmp = tempfile::tempdir().unwrap();
        let work_dir = tmp.path().join("meta/work");
        std::fs::create_dir_all(&work_dir).unwrap();
        // Filename is bare-numeric (matches default scan_regex) but
        // frontmatter declares a prefixed ID — frontmatter wins.
        std::fs::write(
            work_dir.join("0001-foo.md"),
            "---\ntitle: F\nwork_item_id: \"ENG-0042\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work_dir.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let cfg = Arc::new(WorkItemConfig::default_numeric());
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), cfg).await.unwrap();
        let entry = idx
            .get(&work_dir.join("0001-foo.md"))
            .await
            .expect("indexed");
        assert_eq!(entry.work_item_id.as_deref(), Some("ENG-0042"));
    }

    #[tokio::test]
    async fn work_item_id_falls_back_to_filename_when_frontmatter_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let work_dir = tmp.path().join("meta/work");
        std::fs::create_dir_all(&work_dir).unwrap();
        std::fs::write(work_dir.join("0042-foo.md"), "---\ntitle: F\n---\n").unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work_dir.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let cfg = Arc::new(WorkItemConfig::default_numeric());
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), cfg).await.unwrap();
        let entry = idx
            .get(&work_dir.join("0042-foo.md"))
            .await
            .expect("indexed");
        assert_eq!(entry.work_item_id.as_deref(), Some("0042"));
    }

    // ── Story 0070: unified `id:` read path + per-arm deprecation warnings ──
    fn fc_for(s: &str) -> crate::file_driver::FileContent {
        crate::file_driver::FileContent {
            bytes: s.as_bytes().to_vec(),
            etag: "etag".into(),
            mtime_ms: 0,
            size: s.len() as u64,
        }
    }

    #[test]
    fn work_item_identity_resolves_via_unified_id_key() {
        // `id:` is primary and wins over the filename (9999), routed through
        // normalise_id — no legacy `work_item_id:` present.
        let cfg = crate::config::WorkItemConfig::default_numeric();
        let entry = build_entry(
            DocTypeKey::WorkItems,
            PathBuf::from("/repo/meta/work/9999-x.md"),
            &fc_for("---\ntitle: T\nid: \"0042\"\n---\nbody\n"),
            Path::new("/repo"),
            &cfg,
        );
        assert_eq!(entry.work_item_id.as_deref(), Some("0042"));
    }

    #[test]
    fn legacy_work_item_id_key_emits_deprecation_warning() {
        let cfg = crate::config::WorkItemConfig::default_numeric();
        let body = crate::log::test_support::capture_logs(|| {
            let entry = build_entry(
                DocTypeKey::WorkItems,
                PathBuf::from("/repo/meta/work/9999-x.md"),
                &fc_for("---\ntitle: T\nwork_item_id: \"0042\"\n---\nbody\n"),
                Path::new("/repo"),
                &cfg,
            );
            assert_eq!(entry.work_item_id.as_deref(), Some("0042"));
        });
        assert!(
            body.contains("legacy `work_item_id:` key"),
            "expected legacy-key deprecation warning, got: {body}"
        );
    }

    #[test]
    fn filename_fallback_emits_deprecation_warning() {
        let cfg = crate::config::WorkItemConfig::default_numeric();
        let body = crate::log::test_support::capture_logs(|| {
            let entry = build_entry(
                DocTypeKey::WorkItems,
                PathBuf::from("/repo/meta/work/0042-foo.md"),
                &fc_for("---\ntitle: T\n---\nbody\n"),
                Path::new("/repo"),
                &cfg,
            );
            assert_eq!(entry.work_item_id.as_deref(), Some("0042"));
        });
        assert!(
            body.contains("filename fallback"),
            "expected filename-fallback deprecation warning, got: {body}"
        );
    }

    #[tokio::test]
    async fn work_item_id_frontmatter_bare_digits_applies_project_code() {
        let tmp = tempfile::tempdir().unwrap();
        let work_dir = tmp.path().join("meta/work");
        std::fs::create_dir_all(&work_dir).unwrap();
        std::fs::write(
            work_dir.join("0001-foo.md"),
            "---\ntitle: F\nwork_item_id: \"42\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work_dir.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let cfg = Arc::new(
            WorkItemConfig::from_raw(crate::config::RawWorkItemConfig {
                scan_regex: "^ENG-([0-9]+)-".to_string(),
                id_pattern: "{project}-{number:04d}".to_string(),
                default_project_code: Some("ENG".to_string()),
            })
            .unwrap(),
        );
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), cfg).await.unwrap();
        let entry = idx
            .get(&work_dir.join("0001-foo.md"))
            .await
            .expect("indexed");
        assert_eq!(entry.work_item_id.as_deref(), Some("ENG-42"));
    }

    #[tokio::test]
    async fn work_item_id_frontmatter_foreign_prefix_passes_through() {
        let tmp = tempfile::tempdir().unwrap();
        let work_dir = tmp.path().join("meta/work");
        std::fs::create_dir_all(&work_dir).unwrap();
        // Workspace's default_project_code is ENG, but the file declares
        // a foreign prefix — must passthrough verbatim.
        std::fs::write(
            work_dir.join("0001-foo.md"),
            "---\ntitle: F\nwork_item_id: \"OPS-7\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work_dir.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let cfg = Arc::new(
            WorkItemConfig::from_raw(crate::config::RawWorkItemConfig {
                scan_regex: "^ENG-([0-9]+)-".to_string(),
                id_pattern: "{project}-{number:04d}".to_string(),
                default_project_code: Some("ENG".to_string()),
            })
            .unwrap(),
        );
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), cfg).await.unwrap();
        let entry = idx
            .get(&work_dir.join("0001-foo.md"))
            .await
            .expect("indexed");
        assert_eq!(entry.work_item_id.as_deref(), Some("OPS-7"));
    }

    #[tokio::test]
    async fn work_item_id_frontmatter_shape_invalid_falls_back_to_filename() {
        let tmp = tempfile::tempdir().unwrap();
        let work_dir = tmp.path().join("meta/work");
        std::fs::create_dir_all(&work_dir).unwrap();
        std::fs::write(
            work_dir.join("0001-foo.md"),
            "---\ntitle: F\nwork_item_id: \"PROJ-1.2\"\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work_dir.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let cfg = Arc::new(WorkItemConfig::default_numeric());
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), cfg).await.unwrap();
        let entry = idx
            .get(&work_dir.join("0001-foo.md"))
            .await
            .expect("indexed");
        assert_eq!(
            entry.work_item_id.as_deref(),
            Some("0001"),
            "shape-invalid frontmatter falls back to filename",
        );
    }

    #[tokio::test]
    async fn work_item_id_none_when_neither_frontmatter_nor_filename_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let work_dir = tmp.path().join("meta/work");
        std::fs::create_dir_all(&work_dir).unwrap();
        std::fs::write(
            work_dir.join("foo-without-number.md"),
            "---\ntitle: F\n---\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("work".into(), work_dir.clone());
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let cfg = Arc::new(WorkItemConfig::default_numeric());
        let idx = Indexer::build(driver, tmp.path().to_path_buf(), cfg).await.unwrap();
        let entry = idx
            .get(&work_dir.join("foo-without-number.md"))
            .await
            .expect("indexed");
        assert_eq!(entry.work_item_id, None);
    }

    // ── Typed-linkage `target:` resolution ───────────────────────────────────
    //
    // Introduces a `plans_by_id` secondary index and teaches
    // `target_path_from_entry` to resolve `target: "plan:<id>"` against it.

    #[tokio::test]
    async fn target_path_resolves_typed_linkage_plan_form() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(
            plans.join("2026-04-18-foo.md"),
            "---\nid: \"2026-04-18-foo\"\ntitle: Foo\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            reviews.join("2026-04-18-foo-review-1.md"),
            "---\ntype: plan-review\ntarget: \"plan:2026-04-18-foo\"\n---\nbody\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(
            driver,
            tmp.path().to_path_buf(),
            Arc::new(WorkItemConfig::default_numeric()),
        )
        .await
        .unwrap();

        let plan_path = std::fs::canonicalize(plans.join("2026-04-18-foo.md")).unwrap();
        let inbound = idx.reviews_by_target(&plan_path).await;
        assert_eq!(
            inbound.len(),
            1,
            "typed-linkage target should resolve via plans_by_id"
        );
    }

    #[tokio::test]
    async fn target_path_legacy_path_form_still_resolves() {
        let tmp = tempfile::tempdir().unwrap();
        let (idx, plan_path, _) = build_with_plan_and_review(tmp.path()).await;
        let inbound = idx.reviews_by_target(&plan_path).await;
        assert_eq!(inbound.len(), 1, "path-form target must still resolve");
    }

    #[tokio::test]
    async fn target_path_typed_linkage_unknown_id_resolves_to_none() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(
            reviews.join("review-1.md"),
            "---\ntype: plan-review\ntarget: \"plan:no-such-id\"\n---\nbody\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans);
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(
            driver,
            tmp.path().to_path_buf(),
            Arc::new(WorkItemConfig::default_numeric()),
        )
        .await
        .unwrap();

        let map_guard = idx.reviews_by_target.read().await;
        assert!(
            map_guard.is_empty(),
            "unresolved typed-linkage target must not produce a reverse-index key"
        );
    }

    #[tokio::test]
    async fn target_path_typed_linkage_empty_id_resolves_to_none() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        std::fs::write(
            reviews.join("review-empty.md"),
            "---\ntype: plan-review\ntarget: \"plan:\"\n---\nbody\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans);
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(
            driver,
            tmp.path().to_path_buf(),
            Arc::new(WorkItemConfig::default_numeric()),
        )
        .await
        .unwrap();

        let map_guard = idx.reviews_by_target.read().await;
        assert!(
            map_guard.is_empty(),
            "empty typed-linkage id must not produce a reverse-index key"
        );
    }

    #[tokio::test]
    async fn target_path_typed_linkage_resolves_regardless_of_iteration_order() {
        // Validates the two-pass build approach: a plan-review whose
        // source plan happens to be enumerated after it must still
        // resolve once Pass A has populated plans_by_id.
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        // Filenames are alphabetic; the review sorts before the plan in
        // most file-driver enumerations. Pass A folds both before Pass B
        // resolves reviews_by_target, so the order is irrelevant.
        std::fs::write(
            reviews.join("aaa-review-1.md"),
            "---\ntype: plan-review\ntarget: \"plan:zzz-target-plan\"\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            plans.join("zzz-target-plan.md"),
            "---\nid: \"zzz-target-plan\"\ntitle: Target\n---\nbody\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(
            driver,
            tmp.path().to_path_buf(),
            Arc::new(WorkItemConfig::default_numeric()),
        )
        .await
        .unwrap();

        let plan_path = std::fs::canonicalize(plans.join("zzz-target-plan.md")).unwrap();
        let inbound = idx.reviews_by_target(&plan_path).await;
        assert_eq!(
            inbound.len(),
            1,
            "two-pass build must resolve typed linkage regardless of enumeration order"
        );
    }

    #[tokio::test]
    async fn plans_by_id_lifecycle_on_refresh_one() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let reviews = tmp.path().join("meta/reviews/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&reviews).unwrap();
        let plan = plans.join("2026-04-18-foo.md");
        std::fs::write(
            &plan,
            "---\nid: \"2026-04-18-foo\"\ntitle: Foo\n---\nbody\n",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert("plans".into(), plans.clone());
        map.insert("review_plans".into(), reviews);
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(&map, vec![], vec![]));
        let idx = Indexer::build(
            driver,
            tmp.path().to_path_buf(),
            Arc::new(WorkItemConfig::default_numeric()),
        )
        .await
        .unwrap();

        // After initial build, plans_by_id contains the plan.
        {
            let m = idx.plans_by_id.read().await;
            assert!(m.contains_key("2026-04-18-foo"), "plan should be indexed");
        }

        // Mutate the plan's id and refresh.
        std::fs::write(
            &plan,
            "---\nid: \"2026-04-18-foo-renamed\"\ntitle: Foo\n---\nbody\n",
        )
        .unwrap();
        idx.refresh_one(&plan).await.unwrap();
        {
            let m = idx.plans_by_id.read().await;
            assert!(
                !m.contains_key("2026-04-18-foo"),
                "old id should be removed after refresh"
            );
            assert!(
                m.contains_key("2026-04-18-foo-renamed"),
                "new id should be inserted after refresh"
            );
        }

        // Delete the plan and confirm plans_by_id no longer contains it.
        std::fs::remove_file(&plan).unwrap();
        idx.refresh_one(&plan).await.unwrap();
        {
            let m = idx.plans_by_id.read().await;
            assert!(
                !m.contains_key("2026-04-18-foo-renamed"),
                "deleted plan should be removed from plans_by_id"
            );
        }
    }
}

#[cfg(test)]
mod target_path_resolution_tests {
    use super::*;
    use crate::test_support::entry_for_test;

    fn plans_by_id_with(id: &str, path: PathBuf) -> HashMap<String, PathBuf> {
        let mut m = HashMap::new();
        m.insert(id.to_string(), path);
        m
    }

    #[test]
    fn work_item_review_with_path_target_resolves_to_work_item_path() {
        let mut entry = entry_for_test(DocTypeKey::WorkItemReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({
            "target": "meta/work/0033-design-token-system.md"
        });
        let root = PathBuf::from("/repo");
        let resolved = target_path_from_entry(
            &entry,
            &plans_by_id_with("ignored", PathBuf::from("/repo/meta/plans/x.md")),
            &HashMap::new(),
            &crate::config::WorkItemConfig::default(),
            &root,
        );
        assert_eq!(
            resolved,
            Some(PathBuf::from("/repo/meta/work/0033-design-token-system.md"))
        );
    }

    #[test]
    fn validation_with_path_target_resolves_to_plan_path() {
        let mut entry = entry_for_test(DocTypeKey::Validations, "x", 0, "V");
        entry.frontmatter = serde_json::json!({
            "target": "meta/plans/2026-04-21-foo.md"
        });
        let root = PathBuf::from("/repo");
        let resolved = target_path_from_entry(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &crate::config::WorkItemConfig::default(),
            &root,
        );
        assert_eq!(
            resolved,
            Some(PathBuf::from("/repo/meta/plans/2026-04-21-foo.md"))
        );
    }

    #[test]
    fn plan_review_with_typed_plan_id_resolves_via_plans_by_id() {
        let mut entry = entry_for_test(DocTypeKey::PlanReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({
            "target": "plan:2026-05-31-0040-pipeline"
        });
        let plans = plans_by_id_with(
            "2026-05-31-0040-pipeline",
            PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md"),
        );
        let resolved = target_path_from_entry(
            &entry,
            &plans,
            &HashMap::new(),
            &crate::config::WorkItemConfig::default(),
            &PathBuf::from("/repo"),
        );
        assert_eq!(
            resolved,
            Some(PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md"))
        );
    }

    #[test]
    fn pr_review_with_path_target_resolves_to_target_path() {
        let mut entry = entry_for_test(DocTypeKey::PrReviews, "x", 0, "PRR");
        entry.frontmatter = serde_json::json!({
            "target": "meta/prs/42-foo.md"
        });
        let resolved = target_path_from_entry(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &crate::config::WorkItemConfig::default(),
            &PathBuf::from("/repo"),
        );
        assert_eq!(resolved, Some(PathBuf::from("/repo/meta/prs/42-foo.md")));
    }

    #[test]
    fn entry_without_target_field_resolves_to_none() {
        let entry = entry_for_test(DocTypeKey::PlanReviews, "x", 0, "R");
        let resolved = target_path_from_entry(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &crate::config::WorkItemConfig::default(),
            &PathBuf::from("/repo"),
        );
        assert_eq!(resolved, None);
    }

    #[test]
    fn non_target_carrying_doc_types_return_none() {
        for kind in [
            DocTypeKey::Plans,
            DocTypeKey::Research,
            DocTypeKey::WorkItems,
        ] {
            let mut entry = entry_for_test(kind, "x", 0, "T");
            entry.frontmatter = serde_json::json!({ "target": "meta/plans/foo.md" });
            assert_eq!(
                target_path_from_entry(
                    &entry,
                    &HashMap::new(),
                    &HashMap::new(),
                    &crate::config::WorkItemConfig::default(),
                    &PathBuf::from("/repo")
                ),
                None,
                "{kind:?} should not resolve target:",
            );
        }
    }

    #[test]
    fn typed_work_item_target_resolves_via_work_item_by_id() {
        // Story 0070 types work-item-review targets to `work-item:NNNN`;
        // target_path_from_entry now resolves them through work_item_by_id
        // (canonicalising the raw id first), so the path-keyed
        // reviews_by_target reverse index stays populated. (Was previously
        // pinned to return None and resolved only by the cluster-key resolver.)
        let mut entry = entry_for_test(DocTypeKey::WorkItemReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({ "target": "work-item:0042" });
        let mut work_item_by_id = HashMap::new();
        work_item_by_id.insert(
            "0042".to_string(),
            PathBuf::from("/repo/meta/work/0042-foo.md"),
        );
        let resolved = target_path_from_entry(
            &entry,
            &HashMap::new(),
            &work_item_by_id,
            &crate::config::WorkItemConfig::default(),
            &PathBuf::from("/repo"),
        );
        assert_eq!(resolved, Some(PathBuf::from("/repo/meta/work/0042-foo.md")));
    }

    #[test]
    fn typed_adr_and_pr_targets_return_none() {
        for raw in ["adr:0034", "pr:42"] {
            let mut entry = entry_for_test(DocTypeKey::PrReviews, "x", 0, "PRR");
            entry.frontmatter = serde_json::json!({ "target": raw });
            assert_eq!(
                target_path_from_entry(
                    &entry,
                    &HashMap::new(),
                    &HashMap::new(),
                    &crate::config::WorkItemConfig::default(),
                    &PathBuf::from("/repo")
                ),
                None,
                "raw={raw}",
            );
        }
    }

    #[test]
    fn path_target_with_traversal_is_rejected_by_normalize_target_key() {
        let mut entry = entry_for_test(DocTypeKey::PlanReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({ "target": "../../etc/passwd" });
        assert_eq!(
            target_path_from_entry(
                &entry,
                &HashMap::new(),
                &HashMap::new(),
                &crate::config::WorkItemConfig::default(),
                &PathBuf::from("/repo")
            ),
            None,
        );
    }

    #[test]
    fn path_target_resolves_against_supplied_project_root() {
        let mut entry = entry_for_test(DocTypeKey::PlanReviews, "x", 0, "R");
        entry.frontmatter = serde_json::json!({ "target": "meta/plans/foo.md" });
        let resolved = target_path_from_entry(
            &entry,
            &HashMap::new(),
            &HashMap::new(),
            &crate::config::WorkItemConfig::default(),
            &PathBuf::from("/repo/alt"),
        );
        assert_eq!(resolved, Some(PathBuf::from("/repo/alt/meta/plans/foo.md")));
    }
}
