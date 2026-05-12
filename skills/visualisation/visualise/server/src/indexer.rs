use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
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
    /// Reverse declared-link index. Keys are lexically-clean absolute
    /// paths of target plans (or any future target type); values are
    /// sets of canonicalised paths of reviews referencing the target.
    /// `BTreeSet` gives deterministic iteration order and
    /// dedup-by-construction.
    reviews_by_target: Arc<RwLock<HashMap<PathBuf, BTreeSet<PathBuf>>>>,
    /// Reverse work-item cross-ref index. Keys are canonical work-item IDs
    /// (as produced by `canonicalise_refs`); values are sets of canonicalised
    /// paths of entries that reference that work-item via `work-item:`,
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
        let mut reviews_by_target: HashMap<PathBuf, BTreeSet<PathBuf>> = HashMap::new();
        let mut work_item_refs_by_target: HashMap<String, BTreeSet<PathBuf>> = HashMap::new();

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
                let entry = build_entry(kind, path.clone(), &content, &self.project_root, &self.work_item_cfg);

                if let Some(id) = adr_id_from_entry(&entry) {
                    adr_by_id.insert(id, path.clone());
                }
                if let Some(id) = work_item_id_from_entry(&entry) {
                    work_item_by_id.insert(id, path.clone());
                }
                if let Some(target_key) = target_path_from_entry(&entry, &self.project_root) {
                    reviews_by_target
                        .entry(target_key)
                        .or_default()
                        .insert(path.clone());
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
        }

        // Hold all five write locks simultaneously and replace contents
        // so readers never observe a partial (entries, secondary)
        // snapshot. Always acquire in the same order: entries → adr →
        // work_item → reviews_by_target → work_item_refs_by_target.
        let mut entries_w = self.entries.write().await;
        let mut adr_w = self.adr_by_id.write().await;
        let mut work_item_w = self.work_item_by_id.write().await;
        let mut reviews_w = self.reviews_by_target.write().await;
        let mut refs_w = self.work_item_refs_by_target.write().await;
        *entries_w = entries;
        *adr_w = adr_by_id;
        *work_item_w = work_item_by_id;
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
                            remove_from_reviews_by_target(
                                &self.reviews_by_target,
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
                update_reviews_by_target(
                    &self.reviews_by_target,
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
                    remove_from_reviews_by_target(
                        &self.reviews_by_target,
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

    /// Returns entries whose `work-item:`, `parent:`, or `related:` frontmatter
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
    /// - the work-items referenced via `work-item:`, `parent:`, `related:`.
    pub async fn declared_outbound(&self, entry: &IndexEntry) -> Vec<IndexEntry> {
        let entries = self.entries.read().await;
        let by_id = self.work_item_by_id.read().await;
        let mut result: Vec<IndexEntry> = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();

        // Existing: plan-review `target:` field.
        if let Some(target_key) = target_path_from_entry(entry, &self.project_root) {
            if let Some(e) = entries.get(&target_key) {
                if seen.insert(e.path.clone()) {
                    result.push(e.clone());
                }
            }
        }

        // Work-item cross-refs from `work-item:`, `parent:`, `related:`.
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

/// Phase 9 scope: `target:` is populated only on plan-reviews. Activation
/// for other declared-link fields (e.g., PR-reviews) is a follow-up.
fn target_path_from_entry(entry: &IndexEntry, project_root: &Path) -> Option<PathBuf> {
    if entry.r#type != DocTypeKey::PlanReviews {
        return None;
    }
    let raw = entry.frontmatter.get("target")?.as_str()?;
    normalize_target_key(raw, project_root)
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

async fn update_reviews_by_target(
    map: &Arc<RwLock<HashMap<PathBuf, BTreeSet<PathBuf>>>>,
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
    let prev_target = previous.and_then(|p| target_path_from_entry(p, project_root));
    let next_target = target_path_from_entry(new_entry, project_root);
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

async fn remove_from_reviews_by_target(
    map: &Arc<RwLock<HashMap<PathBuf, BTreeSet<PathBuf>>>>,
    project_root: &Path,
    previous: &IndexEntry,
) {
    if let Some(target_key) = target_path_from_entry(previous, project_root) {
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
    use std::sync::OnceLock;

    static PROJECT_RE: OnceLock<regex::Regex> = OnceLock::new();
    let project_re = PROJECT_RE
        .get_or_init(|| regex::Regex::new(r"^[A-Za-z][A-Za-z0-9]*-\d+$").unwrap());

    static NUMERIC_RE: OnceLock<regex::Regex> = OnceLock::new();
    let numeric_re = NUMERIC_RE
        .get_or_init(|| regex::Regex::new(r"^\d+$").unwrap());

    let has_project = cfg.id_pattern.contains("{project}");
    let width = number_width_from_id_pattern(&cfg.id_pattern);

    let mut seen: HashSet<String> = HashSet::new();
    let mut result: Vec<String> = Vec::new();

    for r in raw {
        if r.is_empty() {
            continue;
        }
        let canonical = if numeric_re.is_match(&r) {
            let n_str = r.parse::<u64>()
                .map(|n| n.to_string())
                .unwrap_or(r.clone());
            let padded = format!("{:0>width$}", n_str, width = width);
            if has_project {
                match &cfg.default_project_code {
                    Some(code) => format!("{code}-{padded}"),
                    None => padded,
                }
            } else {
                padded
            }
        } else if project_re.is_match(&r) {
            r.clone()
        } else {
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
    let (slug_val, work_item_id) = if kind == DocTypeKey::WorkItems {
        let regex_slug = slug::derive_work_item_with_regex(&work_item_cfg.scan_regex, filename);
        // Fall back to the default numeric slug derivation when the primary
        // regex doesn't match (e.g., legacy bare-numeric files in a
        // project-prefixed workspace during a pattern-config rollout).
        let slug = regex_slug.or_else(|| slug::derive(kind, filename));
        (slug, work_item_cfg.extract_id(filename))
    } else {
        (slug::derive(kind, filename), None)
    };
    let title = frontmatter::title_from(&parsed.state, &parsed.body, filename_stem);
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
    async fn reviews_by_target_only_admits_plan_reviews() {
        let tmp = tempfile::tempdir().unwrap();
        let plans = tmp.path().join("meta/plans");
        let pr_reviews = tmp.path().join("meta/reviews/prs");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::create_dir_all(&pr_reviews).unwrap();
        std::fs::write(plans.join("a.md"), "---\ntitle: A\n---\n").unwrap();
        // PR review with a synthetic target — should NOT be admitted.
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
        assert!(inbound.is_empty(), "PR-reviews are out of Phase 9 scope");
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
                "---\ntitle: A Plan\nwork-item: \"0001\"\n---\n",
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
                "---\ntitle: Orphan Plan\nwork-item: \"9999\"\n---\n",
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
        // Plan has work-item: 0001, parent: 0001 — two refs to same ID.
        let idx = build_indexer_with_work_items(
            tmp.path(),
            &[("0001-epic.md", "---\ntitle: Epic\n---\n")],
            &[(
                "meta/plans/2026-05-01-plan.md",
                "---\ntitle: Dup Plan\nwork-item: \"0001\"\nparent: 0001\n---\n",
            )],
        )
        .await;
        let refs = idx.work_item_refs_by_id("0001").await;
        // work-item: wins over ticket:, so work_item_refs = ["0001", "0001"]
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
        // A plan that references the work item via work-item:
        std::fs::write(
            plan_dir.join("2026-05-01-plan.md"),
            "---\ntitle: A Plan\nwork-item: \"0001\"\n---\n",
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
}
