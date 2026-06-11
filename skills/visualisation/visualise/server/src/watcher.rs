use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use chrono::{DateTime, Utc};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;

use crate::activity_feed::{ActivityEvent, ActivityRingBuffer};
use crate::clusters::{compute_clusters_with_backfill, LifecycleCluster};
use crate::config::TemplateTiers;
use crate::file_driver::FileDriver;
use crate::indexer::{IndexEntry, Indexer, FRONTMATTER_MALFORMED};
use crate::sse_hub::{ActionKind, SseHub, SsePayload};
use crate::templates::TemplateResolver;
use crate::write_coordinator::WriteCoordinator;

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub debounce: Duration,
}

impl Settings {
    pub const DEFAULT: Settings = Settings {
        debounce: Duration::from_millis(100),
    };
}

// Each arg is a distinct shared-state handle or config value the watcher
// task captures; bundling them into a struct would only rename the wiring.
#[allow(clippy::too_many_arguments)]
pub fn spawn(
    dirs: Vec<PathBuf>,
    project_root: PathBuf,
    indexer: Arc<Indexer>,
    clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    hub: Arc<SseHub>,
    activity_feed: Arc<ActivityRingBuffer>,
    write_coordinator: Arc<WriteCoordinator>,
    template_change_handler: Option<Arc<TemplateChangeHandler>>,
    settings: Settings,
) -> JoinHandle<()> {
    let (tx, mut rx) =
        tokio::sync::mpsc::channel::<notify::Result<Event>>(1024);

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if tx.try_send(res).is_err() {
                tracing::warn!("filesystem event channel full; dropping event");
            }
        },
        notify::Config::default(),
    )
    .expect("failed to create filesystem watcher");

    // Use recursive watches so editor atomic-rename patterns and nested
    // tier layouts produce events. Scope is preserved by the is_markdown
    // filter and the canonical-path index inside the handler.
    for dir in &dirs {
        if dir.exists() {
            if let Err(e) = watcher.watch(dir, RecursiveMode::Recursive) {
                tracing::warn!(dir = %dir.display(), error = %e, "failed to watch dir");
            } else {
                tracing::debug!(dir = %dir.display(), "watching");
            }
        }
    }

    tokio::spawn(async move {
        let _watcher = watcher;
        let mut pending: HashMap<PathBuf, JoinHandle<()>> = HashMap::new();

        while let Some(result) = rx.recv().await {
            match result {
                Ok(event) => {
                    for path in event.paths {
                        if !is_markdown(&path) {
                            continue;
                        }
                        let pre = indexer.get(&path).await;

                        if let Some(h) = pending.remove(&path) {
                            h.abort();
                        }
                        pending.retain(|_, h| !h.is_finished());

                        let h = tokio::spawn(on_path_changed_debounced(
                            path.clone(),
                            project_root.clone(),
                            indexer.clone(),
                            clusters.clone(),
                            hub.clone(),
                            activity_feed.clone(),
                            write_coordinator.clone(),
                            template_change_handler.clone(),
                            settings.debounce,
                            pre,
                        ));
                        pending.insert(path, h);
                    }
                }
                Err(e) => {
                    tracing::warn!("notify watcher error: {e}");
                }
            }
        }
    })
}

// Forwards the same set of narrow shared-state handles `spawn` holds, plus
// the changed path and pre-change entry; a param struct would not aid clarity.
#[allow(clippy::too_many_arguments)]
pub async fn on_path_changed_debounced(
    path: PathBuf,
    project_root: PathBuf,
    indexer: Arc<Indexer>,
    clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    hub: Arc<SseHub>,
    activity_feed: Arc<ActivityRingBuffer>,
    write_coordinator: Arc<WriteCoordinator>,
    template_change_handler: Option<Arc<TemplateChangeHandler>>,
    debounce: Duration,
    pre: Option<IndexEntry>,
) {
    tokio::time::sleep(debounce).await;

    // Walk-up canonicalisation so that delete events (where canonicalize
    // fails because the inode is gone) still match the canonical paths
    // stored by the WriteCoordinator and the TierPathIndex.
    let canonical = canonicalise_path_or_ancestor(&path).await;

    // Additive: a tier-file change is signalled for rebuild + broadcast
    // independently of the doc-changed flow. Templates have no frontend
    // write path today, so we skip WriteCoordinator suppression here.
    let is_template = template_change_handler
        .as_ref()
        .is_some_and(|h| h.try_handle(&canonical));
    tracing::debug!(
        file = %canonical.display(),
        is_template,
        "watcher dispatched fs event",
    );

    if write_coordinator.should_suppress(&canonical) {
        tracing::debug!(file = %path.display(), "watcher suppressed self-write broadcast");
        return;
    }

    if let Err(e) = indexer.rescan().await {
        tracing::warn!(path = %path.display(), error = %e, "rescan failed after watch event");
        return;
    }

    let snapshot = indexer.all().await;
    let work_item_by_id = indexer.work_item_by_id_snapshot().await;
    let plans_by_id = indexer.plans_by_id_snapshot().await;
    let cluster_ctx = crate::clusters::ClusterContext::from_entries(
        &snapshot,
        &work_item_by_id,
        &plans_by_id,
        indexer.project_root(),
        indexer.work_item_cfg(),
    );
    let (new_clusters, completeness_backfill, cluster_key_backfill) =
        compute_clusters_with_backfill(&snapshot, &cluster_ctx);
    let linked_counts = crate::related::collect_linked_counts(
        &indexer,
        &new_clusters,
        &snapshot,
    )
    .await;
    indexer
        .apply_completeness_backfill(completeness_backfill)
        .await;
    indexer
        .apply_cluster_key_backfill(cluster_key_backfill)
        .await;
    indexer.apply_linked_count_backfill(linked_counts).await;
    *clusters.write().await = new_clusters;

    let rel = path
        .strip_prefix(&project_root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");

    let now = Utc::now();

    match indexer.get(&path).await {
        Some(entry) => {
            // Suppress no-op re-broadcasts. A filesystem event does not imply
            // the indexed content changed: editors touch metadata, and macOS
            // FSEvents replays coalesced historical events for files that
            // existed before the watch began plus multi-event bursts per
            // write. When the post-rescan content hash matches the hash
            // captured before the event, nothing the frontend renders has
            // changed, so broadcasting would only churn SSE clients. Both
            // states must be well-formed — any transition into or out of the
            // malformed state is a real change the frontend must observe.
            let unchanged = pre.as_ref().is_some_and(|p| {
                entry.frontmatter_state != FRONTMATTER_MALFORMED
                    && p.frontmatter_state != FRONTMATTER_MALFORMED
                    && p.etag == entry.etag
            });
            if unchanged {
                tracing::debug!(
                    file = %path.display(),
                    "watcher suppressed no-op re-broadcast (content unchanged)",
                );
                return;
            }
            emit(
                payload_for_entry(&entry, rel, pre.as_ref(), now),
                hub.as_ref(),
                activity_feed.as_ref(),
            );
            tracing::debug!(file = %path.display(), "SSE event broadcast");
        }
        None => {
            if let Some(pre_entry) = pre {
                emit(
                    SsePayload::DocChanged {
                        action: ActionKind::Deleted,
                        doc_type: pre_entry.r#type,
                        path: rel,
                        etag: None,
                        timestamp: now,
                    },
                    hub.as_ref(),
                    activity_feed.as_ref(),
                );
                tracing::debug!(file = %path.display(), "SSE doc-changed broadcast for deleted file");
            }
        }
    }
}

/// Single fan-out point for SSE events: pushes a derived `ActivityEvent`
/// into the ring buffer (if applicable — `DocInvalid` does not surface),
/// then broadcasts the payload. Ring-buffer push is **synchronous and
/// before** the broadcast, so a buffered event is always available via
/// `/api/activity` once a subscriber sees it on the live stream.
fn emit(payload: SsePayload, hub: &SseHub, activity_feed: &ActivityRingBuffer) {
    if let Some(activity_event) = ActivityEvent::from_payload(&payload) {
        activity_feed.push(activity_event);
    }
    hub.broadcast(payload);
}

/// Map an `IndexEntry` (post-rescan) plus optional `pre` (pre-rescan) plus a
/// captured `now` into the SSE wire-format envelope to broadcast.
///
/// - Malformed entries produce `DocInvalid` (no action; not surfaced in the
///   Activity feed).
/// - `pre.is_some()` -> `Edited`; `pre.is_none()` -> `Created`.
fn payload_for_entry(
    entry: &IndexEntry,
    rel: String,
    pre: Option<&IndexEntry>,
    now: DateTime<Utc>,
) -> SsePayload {
    if entry.frontmatter_state == FRONTMATTER_MALFORMED {
        SsePayload::DocInvalid {
            doc_type: entry.r#type,
            path: rel,
        }
    } else {
        let action = if pre.is_some() {
            ActionKind::Edited
        } else {
            ActionKind::Created
        };
        SsePayload::DocChanged {
            action,
            doc_type: entry.r#type,
            path: rel,
            etag: Some(entry.etag.clone()),
            timestamp: now,
        }
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "md")
}

/// Canonicalise a path that may not exist yet by walking up to the
/// nearest existing ancestor, canonicalising it, and re-appending the
/// descendant components. Used by both `TierPathIndex::build` (to
/// canonicalise absent-at-startup tier paths) and by the watcher's
/// per-event path resolution (so delete events — where canonicalize
/// fails because the inode is gone — still match the index).
pub(crate) async fn canonicalise_path_or_ancestor(raw: &Path) -> PathBuf {
    if let Ok(c) = tokio::fs::canonicalize(raw).await {
        return c;
    }
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = raw.to_path_buf();
    while let Some(parent) = cursor.parent().map(Path::to_path_buf) {
        if let Some(name) = cursor.file_name() {
            tail.push(name.to_os_string());
        }
        cursor = parent;
        if let Ok(canonical_ancestor) = tokio::fs::canonicalize(&cursor).await {
            let mut out = canonical_ancestor;
            for name in tail.iter().rev() {
                out.push(name);
            }
            return out;
        }
        if cursor.as_os_str().is_empty() {
            break;
        }
    }
    raw.to_path_buf()
}

/// O(1) lookup from canonical tier-file path → list of template names
/// that reference it. Built once at startup. Multiple templates may
/// share a tier file (e.g. a common plugin-default), so the value is
/// `Vec<String>`.
pub struct TierPathIndex {
    by_canonical_path: HashMap<PathBuf, Vec<String>>,
}

impl TierPathIndex {
    pub async fn build(templates: &HashMap<String, TemplateTiers>) -> Self {
        let mut by_canonical_path: HashMap<PathBuf, Vec<String>> =
            HashMap::new();
        for (name, t) in templates {
            for raw in t.iter_paths() {
                let canon = canonicalise_path_or_ancestor(&raw).await;
                by_canonical_path
                    .entry(canon)
                    .or_default()
                    .push(name.clone());
            }
        }
        Self { by_canonical_path }
    }

    pub fn names_for(&self, canonical: &Path) -> &[String] {
        self.by_canonical_path
            .get(canonical)
            .map_or(&[], Vec::as_slice)
    }

    pub fn has_any(&self, canonical: &Path) -> bool {
        self.by_canonical_path.contains_key(canonical)
    }
}

/// Owns the resolver `ArcSwap` and the canonical-path index. All
/// template-tier change handling goes through `try_handle`. A single
/// background task coalesces all pending changes into one rebuild via
/// `tokio::sync::Notify`.
pub struct TemplateChangeHandler {
    notify: Arc<Notify>,
    index: Arc<TierPathIndex>,
}

impl TemplateChangeHandler {
    // Distinct template-resolution collaborators (resolver, config, driver,
    // index, hub, roots) captured by the spawned task; a struct adds no clarity.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        templates: Arc<ArcSwap<TemplateResolver>>,
        cfg_templates: Arc<HashMap<String, TemplateTiers>>,
        driver: Arc<dyn FileDriver>,
        index: TierPathIndex,
        hub: Arc<SseHub>,
        project_root: Arc<PathBuf>,
        plugin_root: Arc<PathBuf>,
    ) -> Self {
        let index = Arc::new(index);
        let notify = Arc::new(Notify::new());
        let consumer_notify = notify.clone();

        // Initial sha256 snapshot per template — used to suppress no-op
        // broadcasts and to dedup across-tier multiplicity.
        let mut previous: HashMap<String, Option<String>> = cfg_templates
            .keys()
            .map(|name| {
                (
                    name.clone(),
                    templates.load().detail(name).and_then(|d| d.sha256),
                )
            })
            .collect();

        let cfg_keys: Vec<String> = cfg_templates.keys().cloned().collect();

        tokio::spawn(async move {
            loop {
                consumer_notify.notified().await;

                // Isolate build from the loop: a panic inside build
                // surfaces as a JoinError so the consumer logs and
                // continues rather than silently disabling all future
                // broadcasts.
                let cfg_for_build = cfg_templates.clone();
                let driver_for_build = driver.clone();
                let project_root_for_build = project_root.clone();
                let plugin_root_for_build = plugin_root.clone();
                let build_result = tokio::spawn(async move {
                    TemplateResolver::build(
                        &cfg_for_build,
                        driver_for_build.as_ref(),
                        project_root_for_build.as_ref(),
                        plugin_root_for_build.as_ref(),
                    )
                    .await
                })
                .await;

                let new_resolver = match build_result {
                    Ok(r) => Arc::new(r),
                    Err(join_err) => {
                        tracing::error!(
                            error = ?join_err,
                            "TemplateResolver::build panicked or was cancelled; \
                             consumer skipping this rebuild and remaining alive",
                        );
                        continue;
                    }
                };

                let mut to_broadcast: Vec<(String, Option<String>)> =
                    Vec::new();
                for name in &cfg_keys {
                    let new_sha =
                        new_resolver.detail(name).and_then(|d| d.sha256);
                    let prev = previous.get(name).cloned().flatten();
                    if new_sha != prev {
                        previous.insert(name.clone(), new_sha.clone());
                        to_broadcast.push((name.clone(), new_sha));
                    }
                }

                templates.store(new_resolver);

                for (template, sha256) in to_broadcast {
                    hub.broadcast(SsePayload::TemplateChanged {
                        template,
                        sha256,
                        timestamp: Utc::now(),
                    });
                }
            }
        });

        Self { notify, index }
    }

    /// Returns `true` when the path was claimed as a template-tier
    /// change.
    pub fn try_handle(&self, canonical_path: &Path) -> bool {
        if !self.index.has_any(canonical_path) {
            return false;
        }
        self.notify.notify_one();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_driver::LocalFileDriver;
    use crate::indexer::Indexer;
    use crate::sse_hub::{SseHub, SsePayload};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::RwLock;

    async fn watcher_fires_in_this_env() -> bool {
        use notify::Watcher;
        let tmp = tempfile::tempdir().unwrap();
        let probe = tmp.path().join("probe.txt");
        std::fs::write(&probe, "a").unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut watcher = notify::recommended_watcher(move |_| {
            let _ = tx.try_send(());
        })
        .unwrap();
        watcher
            .watch(tmp.path(), notify::RecursiveMode::NonRecursive)
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        std::fs::write(&probe, "b").unwrap();

        tokio::time::timeout(Duration::from_millis(300), rx.recv())
            .await
            .is_ok()
    }

    async fn setup(
        tmp: &std::path::Path,
    ) -> (
        HashMap<String, std::path::PathBuf>,
        Arc<Indexer>,
        Arc<SseHub>,
        Arc<ActivityRingBuffer>,
        Arc<RwLock<Vec<crate::clusters::LifecycleCluster>>>,
    ) {
        let plans = tmp.join("meta/plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::write(
            plans.join("2026-01-01-foo.md"),
            "---\ntitle: Foo\n---\n# Body\n",
        )
        .unwrap();
        let mut doc_paths = HashMap::new();
        doc_paths.insert("plans".into(), plans);
        let driver: Arc<dyn crate::file_driver::FileDriver> =
            Arc::new(LocalFileDriver::new(&doc_paths, vec![], vec![]));
        let work_item_cfg =
            Arc::new(crate::config::WorkItemConfig::default_numeric());
        let indexer = Arc::new(
            Indexer::build(driver, tmp.to_path_buf(), work_item_cfg)
                .await
                .unwrap(),
        );
        let hub = Arc::new(SseHub::new(64));
        let activity_feed = Arc::new(ActivityRingBuffer::new());
        let snapshot = indexer.all().await;
        let work_item_by_id = indexer.work_item_by_id_snapshot().await;
        let plans_by_id = indexer.plans_by_id_snapshot().await;
        let ctx = crate::clusters::ClusterContext::from_entries(
            &snapshot,
            &work_item_by_id,
            &plans_by_id,
            indexer.project_root(),
            indexer.work_item_cfg(),
        );
        let clusters = Arc::new(RwLock::new(
            crate::clusters::compute_clusters(&snapshot, &ctx),
        ));
        (doc_paths, indexer, hub, activity_feed, clusters)
    }

    #[tokio::test]
    async fn file_change_produces_doc_changed_event() {
        if !watcher_fires_in_this_env().await {
            eprintln!("SKIP: notify watcher not firing in this environment");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, activity_feed, clusters) =
            setup(tmp.path()).await;
        let mut rx = hub.subscribe();

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            activity_feed.clone(),
            Arc::new(WriteCoordinator::new()),
            None,
            Settings {
                debounce: Duration::from_millis(5),
            },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        std::fs::write(
            tmp.path().join("meta/plans/2026-01-01-foo.md"),
            "---\ntitle: Foo updated\n---\n",
        )
        .unwrap();

        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out waiting for SSE event")
            .expect("channel closed");

        match event {
            SsePayload::DocChanged { action, .. } => {
                // Setup wrote the file before the indexer was built, so `pre`
                // is populated when the modify event fires -> Edited.
                assert_eq!(action, ActionKind::Edited);
            }
            other => panic!("expected DocChanged, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unchanged_content_rewrite_is_suppressed() {
        // A filesystem event whose rescan yields content identical to what was
        // already indexed must not broadcast. This guards the macOS FSEvents
        // behaviour where coalesced historical events and per-write multi-event
        // bursts re-fire for files whose content never changed — broadcasting
        // those would churn SSE clients with no-op doc-changed events.
        if !watcher_fires_in_this_env().await {
            eprintln!("SKIP: notify watcher not firing in this environment");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, activity_feed, clusters) =
            setup(tmp.path()).await;
        let mut rx = hub.subscribe();
        let path = tmp.path().join("meta/plans/2026-01-01-foo.md");
        let original = std::fs::read_to_string(&path).unwrap();

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            activity_feed.clone(),
            Arc::new(WriteCoordinator::new()),
            None,
            Settings {
                debounce: Duration::from_millis(5),
            },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Rewrite byte-for-byte identical content: the rescan etag matches the
        // pre-event etag, so the broadcast must be suppressed.
        std::fs::write(&path, &original).unwrap();

        assert!(
            tokio::time::timeout(Duration::from_millis(400), rx.recv())
                .await
                .is_err(),
            "no-op rewrite of unchanged content must not broadcast",
        );
        assert_eq!(
            activity_feed.recent(5).len(),
            0,
            "suppressed re-broadcast must not push an activity event",
        );
    }

    #[tokio::test]
    async fn rapid_writes_coalesce_to_one_event() {
        if !watcher_fires_in_this_env().await {
            eprintln!("SKIP: notify watcher not firing in this environment");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, activity_feed, clusters) =
            setup(tmp.path()).await;
        let mut rx = hub.subscribe();

        // Use a generous debounce so the coalescing assertion is robust to
        // CI scheduler / inotify-delivery jitter — on a slow runner the gap
        // between consecutive events arriving at the consumer can exceed a
        // tight debounce, splitting the burst into two broadcasts.
        let debounce = Duration::from_millis(300);
        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            activity_feed.clone(),
            Arc::new(WriteCoordinator::new()),
            None,
            Settings { debounce },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Write back-to-back without yielding to the runtime — under a
        // current_thread test runtime an inter-write `sleep().await` lets the
        // consumer start scheduling debounces mid-burst, which fragments the
        // coalescing window.
        let path = tmp.path().join("meta/plans/2026-01-01-foo.md");
        for i in 0..5u32 {
            std::fs::write(&path, format!("---\ntitle: v{i}\n---\n")).unwrap();
        }

        let event = tokio::time::timeout(
            debounce + Duration::from_millis(500),
            rx.recv(),
        )
        .await
        .expect("timed out")
        .expect("channel closed");
        match event {
            SsePayload::DocChanged { action, .. } => {
                assert_eq!(action, ActionKind::Edited);
            }
            other => panic!("expected DocChanged, got {other:?}"),
        }

        assert!(
            tokio::time::timeout(Duration::from_millis(300), rx.recv())
                .await
                .is_err(),
            "expected no second event but got one",
        );
    }

    #[tokio::test]
    async fn malformed_frontmatter_produces_doc_invalid_event() {
        if !watcher_fires_in_this_env().await {
            eprintln!("SKIP: notify watcher not firing in this environment");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, activity_feed, clusters) =
            setup(tmp.path()).await;
        let mut rx = hub.subscribe();

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            activity_feed.clone(),
            Arc::new(WriteCoordinator::new()),
            None,
            Settings {
                debounce: Duration::from_millis(5),
            },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        std::fs::write(
            tmp.path().join("meta/plans/2026-01-01-foo.md"),
            "---\ntitle: \"unclosed\n---\nbody\n",
        )
        .unwrap();

        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");

        assert!(
            matches!(event, SsePayload::DocInvalid { .. }),
            "expected DocInvalid for malformed frontmatter, got {event:?}",
        );
    }

    #[tokio::test]
    async fn new_file_in_watched_dir_produces_doc_changed_event() {
        if !watcher_fires_in_this_env().await {
            eprintln!("SKIP: notify watcher not firing in this environment");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, activity_feed, clusters) =
            setup(tmp.path()).await;
        let mut rx = hub.subscribe();

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            activity_feed.clone(),
            Arc::new(WriteCoordinator::new()),
            None,
            Settings {
                debounce: Duration::from_millis(5),
            },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        let before = Utc::now();
        std::fs::write(
            tmp.path().join("meta/plans/2026-05-01-new.md"),
            "---\ntitle: New\n---\n",
        )
        .unwrap();

        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        let after = Utc::now();

        match event {
            SsePayload::DocChanged {
                action, timestamp, ..
            } => {
                assert_eq!(action, ActionKind::Created);
                // Tighter-than-AC bound: timestamp is captured between `before`
                // and `after` from the test's perspective.
                assert!(
                    timestamp >= before && timestamp <= after,
                    "timestamp {timestamp} not within [{before}, {after}]"
                );
                // AC6 verbatim — 1-second tolerance against broadcast wall-clock.
                assert!(
                    (after - timestamp).num_milliseconds().abs() < 1_000,
                    "AC6: timestamp {timestamp} not within 1s of broadcast time {after}"
                );
            }
            other => panic!("expected DocChanged, got {other:?}"),
        }

        // Ring-buffer push: the emit helper pushes synchronously before the
        // broadcast, so by the time the subscriber observed the event the
        // ActivityEvent must already be readable from the feed.
        let recent = activity_feed.recent(5);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].action, ActionKind::Created);
        // On macOS the notify-delivered path may live under /private/var
        // while project_root points at /var/folders, breaking strip_prefix;
        // assert the suffix rather than the full rel so the test is robust.
        assert!(
            recent[0].path.ends_with("meta/plans/2026-05-01-new.md"),
            "ring-buffer path mismatch: {}",
            recent[0].path,
        );
    }

    #[tokio::test]
    async fn file_deletion_produces_doc_changed_without_etag() {
        if !watcher_fires_in_this_env().await {
            eprintln!("SKIP: notify watcher not firing in this environment");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, activity_feed, clusters) =
            setup(tmp.path()).await;
        let mut rx = hub.subscribe();
        let path = tmp.path().join("meta/plans/2026-01-01-foo.md");

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            activity_feed.clone(),
            Arc::new(WriteCoordinator::new()),
            None,
            Settings {
                debounce: Duration::from_millis(5),
            },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        std::fs::remove_file(&path).unwrap();

        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out waiting for deletion SSE event")
            .expect("channel closed");

        match &event {
            SsePayload::DocChanged { action, .. } => {
                assert_eq!(*action, ActionKind::Deleted);
            }
            other => panic!("expected DocChanged for deletion, got {other:?}"),
        }
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            !json.contains("etag"),
            "etag must be absent for deleted files: {json}",
        );

        let recent = activity_feed.recent(5);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].action, ActionKind::Deleted);
        assert!(
            recent[0].path.ends_with("meta/plans/2026-01-01-foo.md"),
            "ring-buffer path mismatch: {}",
            recent[0].path,
        );
    }

    #[tokio::test]
    async fn create_edit_delete_chain_emits_three_distinct_events() {
        if !watcher_fires_in_this_env().await {
            eprintln!("SKIP: notify watcher not firing in this environment");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, activity_feed, clusters) =
            setup(tmp.path()).await;
        let mut rx = hub.subscribe();

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            activity_feed.clone(),
            Arc::new(WriteCoordinator::new()),
            None,
            Settings {
                debounce: Duration::from_millis(20),
            },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        let chain_path = tmp.path().join("meta/plans/2026-05-13-chain.md");

        // 1. Create.
        std::fs::write(&chain_path, "---\ntitle: Chain v1\n---\n").unwrap();
        let created =
            tokio::time::timeout(Duration::from_millis(800), rx.recv())
                .await
                .expect("timed out waiting for created event")
                .expect("channel closed");

        // 2. Edit. Pause longer than debounce so the events do not coalesce.
        tokio::time::sleep(Duration::from_millis(150)).await;
        std::fs::write(&chain_path, "---\ntitle: Chain v2\n---\n").unwrap();
        let edited =
            tokio::time::timeout(Duration::from_millis(800), rx.recv())
                .await
                .expect("timed out waiting for edited event")
                .expect("channel closed");

        // 3. Delete.
        tokio::time::sleep(Duration::from_millis(150)).await;
        std::fs::remove_file(&chain_path).unwrap();
        let deleted =
            tokio::time::timeout(Duration::from_millis(800), rx.recv())
                .await
                .expect("timed out waiting for deleted event")
                .expect("channel closed");

        let (a0, t0) = match &created {
            SsePayload::DocChanged {
                action, timestamp, ..
            } => (*action, *timestamp),
            other => panic!("expected DocChanged for created, got {other:?}"),
        };
        let (a1, t1) = match &edited {
            SsePayload::DocChanged {
                action, timestamp, ..
            } => (*action, *timestamp),
            other => panic!("expected DocChanged for edited, got {other:?}"),
        };
        let (a2, t2) = match &deleted {
            SsePayload::DocChanged {
                action, timestamp, ..
            } => (*action, *timestamp),
            other => panic!("expected DocChanged for deleted, got {other:?}"),
        };

        assert_eq!(a0, ActionKind::Created);
        assert_eq!(a1, ActionKind::Edited);
        assert_eq!(a2, ActionKind::Deleted);
        assert!(t0 < t1, "timestamp ordering created < edited");
        assert!(t1 < t2, "timestamp ordering edited < deleted");

        // Ring buffer captured all three events (newest-first).
        let recent = activity_feed.recent(10);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].action, ActionKind::Deleted);
        assert_eq!(recent[1].action, ActionKind::Edited);
        assert_eq!(recent[2].action, ActionKind::Created);
    }
}

#[cfg(test)]
mod tier_path_index_tests {
    use super::*;
    use crate::config::TemplateTiers;

    #[tokio::test]
    async fn path_referenced_by_two_templates_returns_both_names() {
        let tmp = tempfile::tempdir().unwrap();
        let shared = tmp.path().join("shared.md");
        std::fs::write(&shared, "x").unwrap();
        let mut map: HashMap<String, TemplateTiers> = HashMap::new();
        map.insert(
            "a".into(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.path().join("user-a.md"),
                plugin_default: shared.clone(),
                config_override_source: None,
            },
        );
        map.insert(
            "b".into(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.path().join("user-b.md"),
                plugin_default: shared.clone(),
                config_override_source: None,
            },
        );
        let idx = TierPathIndex::build(&map).await;
        let canon = canonicalise_path_or_ancestor(&shared).await;
        let mut names = idx.names_for(&canon).to_vec();
        names.sort();
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn unrelated_path_returns_empty_slice() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin = tmp.path().join("p.md");
        std::fs::write(&plugin, "x").unwrap();
        let mut map: HashMap<String, TemplateTiers> = HashMap::new();
        map.insert(
            "a".into(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.path().join("u.md"),
                plugin_default: plugin,
                config_override_source: None,
            },
        );
        let idx = TierPathIndex::build(&map).await;
        let unrelated =
            canonicalise_path_or_ancestor(&tmp.path().join("nope.md")).await;
        assert!(idx.names_for(&unrelated).is_empty());
        assert!(!idx.has_any(&unrelated));
    }

    #[tokio::test]
    async fn absent_at_startup_tier_file_is_keyed_by_canonical_ancestor() {
        let tmp = tempfile::tempdir().unwrap();
        // user-override path does not exist
        let user_path = tmp.path().join("user-future.md");
        let plugin = tmp.path().join("p.md");
        std::fs::write(&plugin, "x").unwrap();
        let mut map: HashMap<String, TemplateTiers> = HashMap::new();
        map.insert(
            "a".into(),
            TemplateTiers {
                config_override: None,
                user_override: user_path.clone(),
                plugin_default: plugin,
                config_override_source: None,
            },
        );
        let idx = TierPathIndex::build(&map).await;
        // Look up via what event-time canonicalisation would produce.
        let canon_at_lookup = canonicalise_path_or_ancestor(&user_path).await;
        let names = idx.names_for(&canon_at_lookup);
        assert_eq!(names, &["a".to_string()]);
    }
}

#[cfg(test)]
mod template_change_handler_tests {
    use super::*;
    use crate::config::TemplateTiers;
    use crate::file_driver::LocalFileDriver;
    use crate::sse_hub::SseHub;
    use sha2::Digest as _;
    use std::time::Duration;

    async fn build_handler(
        tmp: &std::path::Path,
        templates_map: HashMap<String, TemplateTiers>,
    ) -> (
        Arc<TemplateChangeHandler>,
        Arc<ArcSwap<TemplateResolver>>,
        Arc<SseHub>,
    ) {
        let driver: Arc<dyn FileDriver> = Arc::new(LocalFileDriver::new(
            &HashMap::new(),
            vec![tmp.to_path_buf()],
            vec![],
        ));
        let resolver =
            TemplateResolver::build(&templates_map, driver.as_ref(), tmp, tmp)
                .await;
        let templates = Arc::new(ArcSwap::from_pointee(resolver));
        let hub = Arc::new(SseHub::new(64));
        let index = TierPathIndex::build(&templates_map).await;
        let handler = Arc::new(TemplateChangeHandler::spawn(
            templates.clone(),
            Arc::new(templates_map),
            driver,
            index,
            hub.clone(),
            Arc::new(tmp.to_path_buf()),
            Arc::new(tmp.to_path_buf()),
        ));
        (handler, templates, hub)
    }

    #[tokio::test]
    async fn winning_tier_change_broadcasts_with_new_sha() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin = tmp.path().join("p.md");
        std::fs::write(&plugin, "v1").unwrap();
        let mut map: HashMap<String, TemplateTiers> = HashMap::new();
        map.insert(
            "a".into(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.path().join("user-missing.md"),
                plugin_default: plugin.clone(),
                config_override_source: None,
            },
        );
        let (handler, _resolver, hub) = build_handler(tmp.path(), map).await;
        let mut rx = hub.subscribe();

        std::fs::write(&plugin, "v2").unwrap();
        let canon = canonicalise_path_or_ancestor(&plugin).await;
        assert!(handler.try_handle(&canon));

        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        match event {
            SsePayload::TemplateChanged {
                template, sha256, ..
            } => {
                assert_eq!(template, "a");
                let expected = format!(
                    "sha256-{}",
                    hex::encode(sha2::Sha256::digest(b"v2"))
                );
                assert_eq!(sha256.as_deref(), Some(expected.as_str()));
            }
            other => panic!("expected TemplateChanged, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn empty_content_transition_broadcasts_without_sha256() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin = tmp.path().join("p.md");
        std::fs::write(&plugin, "v1").unwrap();
        let mut map: HashMap<String, TemplateTiers> = HashMap::new();
        map.insert(
            "a".into(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.path().join("user-missing.md"),
                plugin_default: plugin.clone(),
                config_override_source: None,
            },
        );
        let (handler, _resolver, hub) = build_handler(tmp.path(), map).await;
        let mut rx = hub.subscribe();

        std::fs::write(&plugin, "").unwrap();
        let canon = canonicalise_path_or_ancestor(&plugin).await;
        assert!(handler.try_handle(&canon));

        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        match event {
            SsePayload::TemplateChanged {
                template, sha256, ..
            } => {
                assert_eq!(template, "a");
                assert!(sha256.is_none());
            }
            other => panic!("expected TemplateChanged, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unchanged_winning_content_produces_no_broadcast() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin = tmp.path().join("p.md");
        std::fs::write(&plugin, "v1").unwrap();
        let mut map: HashMap<String, TemplateTiers> = HashMap::new();
        map.insert(
            "a".into(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.path().join("user-missing.md"),
                plugin_default: plugin.clone(),
                config_override_source: None,
            },
        );
        let (handler, _resolver, hub) = build_handler(tmp.path(), map).await;
        let mut rx = hub.subscribe();

        // Re-write identical bytes — should produce no broadcast.
        std::fs::write(&plugin, "v1").unwrap();
        let canon = canonicalise_path_or_ancestor(&plugin).await;
        assert!(handler.try_handle(&canon));

        assert!(
            tokio::time::timeout(Duration::from_millis(200), rx.recv())
                .await
                .is_err(),
            "expected no broadcast for unchanged winning content",
        );
    }

    #[tokio::test]
    async fn unrelated_path_is_not_handled() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin = tmp.path().join("p.md");
        std::fs::write(&plugin, "v1").unwrap();
        let mut map: HashMap<String, TemplateTiers> = HashMap::new();
        map.insert(
            "a".into(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.path().join("user-missing.md"),
                plugin_default: plugin.clone(),
                config_override_source: None,
            },
        );
        let (handler, _resolver, _hub) = build_handler(tmp.path(), map).await;
        let unrelated =
            canonicalise_path_or_ancestor(&tmp.path().join("other.md")).await;
        assert!(!handler.try_handle(&unrelated));
    }

    #[tokio::test]
    async fn shared_tier_file_broadcasts_per_template() {
        let tmp = tempfile::tempdir().unwrap();
        let shared = tmp.path().join("shared.md");
        std::fs::write(&shared, "v1").unwrap();
        let mut map: HashMap<String, TemplateTiers> = HashMap::new();
        map.insert(
            "a".into(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.path().join("u-a.md"),
                plugin_default: shared.clone(),
                config_override_source: None,
            },
        );
        map.insert(
            "b".into(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.path().join("u-b.md"),
                plugin_default: shared.clone(),
                config_override_source: None,
            },
        );
        let (handler, _resolver, hub) = build_handler(tmp.path(), map).await;
        let mut rx = hub.subscribe();

        std::fs::write(&shared, "v2").unwrap();
        let canon = canonicalise_path_or_ancestor(&shared).await;
        assert!(handler.try_handle(&canon));

        let mut got: Vec<String> = Vec::new();
        for _ in 0..2 {
            let event =
                tokio::time::timeout(Duration::from_millis(500), rx.recv())
                    .await
                    .expect("timed out")
                    .expect("channel closed");
            if let SsePayload::TemplateChanged { template, .. } = event {
                got.push(template);
            }
        }
        got.sort();
        assert_eq!(got, vec!["a".to_string(), "b".to_string()]);
    }
}
