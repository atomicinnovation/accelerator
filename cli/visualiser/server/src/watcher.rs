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

pub trait DirectoryWatcher: Send + 'static {
    fn watch_recursively(&mut self, dir: &Path) -> notify::Result<()>;
}

impl DirectoryWatcher for RecommendedWatcher {
    fn watch_recursively(&mut self, dir: &Path) -> notify::Result<()> {
        // Recursive so editor atomic-rename patterns and nested tier layouts
        // produce events. Scope is preserved by the is_markdown filter and the
        // canonical-path index inside the handler.
        Watcher::watch(self, dir, RecursiveMode::Recursive)
    }
}

pub struct EventSink(tokio::sync::mpsc::Sender<notify::Result<Event>>);

impl notify::EventHandler for EventSink {
    fn handle_event(&mut self, event: notify::Result<Event>) {
        if self.0.try_send(event).is_err() {
            tracing::warn!("filesystem event channel full; dropping event");
        }
    }
}

pub fn os_watcher(sink: EventSink) -> notify::Result<RecommendedWatcher> {
    RecommendedWatcher::new(sink, notify::Config::default())
}

pub struct Watching {
    registered: tokio::sync::watch::Receiver<bool>,
    events: JoinHandle<()>,
}

impl Watching {
    /// Resolves once every watch directory is registered with the OS, or
    /// once registration has given up.
    pub async fn registered(&mut self) {
        let _ = self.registered.wait_for(|done| *done).await;
    }

    pub async fn finished(self) -> Result<(), tokio::task::JoinError> {
        self.events.await
    }
}

// Each arg is a distinct shared-state handle or config value the watcher
// task captures; bundling them into a struct would only rename the wiring.
#[allow(clippy::too_many_arguments)]
pub fn spawn<W: DirectoryWatcher>(
    open: impl FnOnce(EventSink) -> notify::Result<W> + Send + 'static,
    dirs: Vec<PathBuf>,
    indexer: Arc<Indexer>,
    clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    hub: Arc<SseHub>,
    activity_feed: Arc<ActivityRingBuffer>,
    write_coordinator: Arc<WriteCoordinator>,
    template_change_handler: Option<Arc<TemplateChangeHandler>>,
    settings: Settings,
) -> Watching {
    let (tx, mut rx) =
        tokio::sync::mpsc::channel::<notify::Result<Event>>(1024);
    let (registered_tx, registered) = tokio::sync::watch::channel(false);

    // Registering a recursive watch can block for tens of seconds when the
    // OS notification service is saturated, so it must not hold up serving.
    let registration = tokio::task::spawn_blocking(move || {
        // Watcher construction fails only if the platform backend is
        // unavailable; there is no recovery path, so the panic surfaces
        // through `Watching::finished` and live updates stay off.
        #[allow(clippy::expect_used)]
        let mut watcher =
            open(EventSink(tx)).expect("failed to create filesystem watcher");
        register(&mut watcher, &dirs);
        let _ = registered_tx.send(true);
        watcher
    });

    let events = tokio::spawn(async move {
        let _watcher = match registration.await {
            Ok(watcher) => watcher,
            Err(e) => std::panic::resume_unwind(e.into_panic()),
        };
        let mut pending: HashMap<PathBuf, JoinHandle<()>> = HashMap::new();

        while let Some(result) = rx.recv().await {
            match result {
                Ok(event) => {
                    for path in event.paths {
                        if !is_markdown(&path) {
                            continue;
                        }
                        if let Some(h) = pending.remove(&path) {
                            h.abort();
                        }
                        pending.retain(|_, h| !h.is_finished());

                        let h = tokio::spawn(on_path_changed_debounced(
                            path.clone(),
                            indexer.clone(),
                            clusters.clone(),
                            hub.clone(),
                            activity_feed.clone(),
                            write_coordinator.clone(),
                            template_change_handler.clone(),
                            settings.debounce,
                        ));
                        pending.insert(path, h);
                    }
                }
                Err(e) => {
                    tracing::warn!("notify watcher error: {e}");
                }
            }
        }
    });
    Watching { registered, events }
}

fn register(watcher: &mut impl DirectoryWatcher, dirs: &[PathBuf]) {
    for dir in dirs {
        if dir.exists() {
            if let Err(e) = watcher.watch_recursively(dir) {
                tracing::warn!(dir = %dir.display(), error = %e, "failed to watch dir");
            } else {
                tracing::debug!(dir = %dir.display(), "watching");
            }
        }
    }
}

// Forwards the same set of narrow shared-state handles `spawn` holds, plus
// the changed path; a param struct would not aid clarity.
#[allow(clippy::too_many_arguments)]
pub async fn on_path_changed_debounced(
    path: PathBuf,
    indexer: Arc<Indexer>,
    clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    hub: Arc<SseHub>,
    activity_feed: Arc<ActivityRingBuffer>,
    write_coordinator: Arc<WriteCoordinator>,
    template_change_handler: Option<Arc<TemplateChangeHandler>>,
    debounce: Duration,
) {
    tokio::time::sleep(debounce).await;

    // Detached: a later event for the same path aborts only the debounce
    // above. Aborting after the rescan would leave its changes in the index
    // but never broadcast, and no later rescan would see them as changes.
    let _ = tokio::spawn(on_path_changed(
        path,
        indexer,
        clusters,
        hub,
        activity_feed,
        write_coordinator,
        template_change_handler,
    ))
    .await;
}

async fn on_path_changed(
    path: PathBuf,
    indexer: Arc<Indexer>,
    clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    hub: Arc<SseHub>,
    activity_feed: Arc<ActivityRingBuffer>,
    write_coordinator: Arc<WriteCoordinator>,
    template_change_handler: Option<Arc<TemplateChangeHandler>>,
) {
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

    let before = indexer.all().await;
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

    // The rescan reindexes every file, not just `path`, so it broadcasts every
    // change it applied: a change another file's rescan absorbs is otherwise
    // indistinguishable from a no-op by the time its own event arrives.
    for payload in rescan_changes(&before, &snapshot, Utc::now())
        .into_iter()
        .filter(|change| !write_coordinator.should_suppress(&change.path))
        .map(|change| change.payload)
    {
        emit(payload, hub.as_ref(), activity_feed.as_ref());
    }
}

struct RescanChange {
    path: PathBuf,
    payload: SsePayload,
}

/// The broadcasts a rescan owes its subscribers, from the index before and
/// after it.
///
/// An entry whose etag and frontmatter state are unchanged is not a change:
/// macOS `FSEvents` replays coalesced historical events and multi-event bursts
/// per write, and broadcasting those would only churn SSE clients.
fn rescan_changes(
    before: &[IndexEntry],
    after: &[IndexEntry],
    now: DateTime<Utc>,
) -> Vec<RescanChange> {
    let before_by_path: HashMap<&Path, &IndexEntry> = before
        .iter()
        .map(|entry| (entry.path.as_path(), entry))
        .collect();
    let after_paths: std::collections::HashSet<&Path> =
        after.iter().map(|entry| entry.path.as_path()).collect();

    let edited_or_created = after.iter().filter_map(|entry| {
        let pre = before_by_path.get(entry.path.as_path()).copied();
        let unchanged = pre.is_some_and(|p| {
            p.etag == entry.etag
                && p.frontmatter_state == entry.frontmatter_state
        });
        (!unchanged).then(|| RescanChange {
            path: entry.path.clone(),
            payload: payload_for_entry(entry, rel_of(entry), pre, now),
        })
    });
    let deleted = before
        .iter()
        .filter(|entry| !after_paths.contains(entry.path.as_path()))
        .map(|entry| RescanChange {
            path: entry.path.clone(),
            payload: SsePayload::DocChanged {
                action: ActionKind::Deleted,
                doc_type: entry.r#type,
                path: rel_of(entry),
                etag: None,
                timestamp: now,
            },
        });
    edited_or_created.chain(deleted).collect()
}

fn rel_of(entry: &IndexEntry) -> String {
    entry.rel_path.to_string_lossy().replace('\\', "/")
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

/// How long a test waits for an event it *expects* to arrive.
///
/// This is a hang detector, not a latency budget: cargo runs these tests
/// concurrently and `mise run` schedules them beside cargo builds, so a
/// delivery that settles in milliseconds idle can take far longer loaded.
///
/// The "expected *no* event" waits deliberately do **not** use this: there
/// the wait *is* the assertion, so a short quiet window keeps the suite
/// fast without weakening it. Do not unify the two.
#[cfg(test)]
const EVENT_BUDGET: Duration = Duration::from_secs(5);

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

    const REGISTRATION_HOLD: Duration = Duration::from_secs(2);

    struct HeldRegistration {
        release: std::sync::mpsc::Receiver<()>,
    }

    impl DirectoryWatcher for HeldRegistration {
        fn watch_recursively(&mut self, _dir: &Path) -> notify::Result<()> {
            let _ = self.release.recv_timeout(REGISTRATION_HOLD);
            Ok(())
        }
    }

    #[tokio::test]
    async fn starting_does_not_wait_for_watch_registration() {
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, activity_feed, clusters) =
            setup(tmp.path()).await;
        let (release, held) = std::sync::mpsc::channel();

        let started = std::time::Instant::now();
        let mut watching = spawn(
            move |_sink| Ok(HeldRegistration { release: held }),
            doc_paths.values().cloned().collect(),
            indexer,
            clusters,
            hub,
            activity_feed,
            Arc::new(WriteCoordinator::new()),
            None,
            Settings::DEFAULT,
        );
        let start_took = started.elapsed();
        let pending_while_held = tokio::time::timeout(
            Duration::from_millis(100),
            watching.registered(),
        )
        .await;
        drop(release);

        assert!(
            start_took < REGISTRATION_HOLD / 2,
            "spawn waited {start_took:?} for watch registration",
        );
        assert!(
            pending_while_held.is_err(),
            "registered() resolved before its watches were registered",
        );
        tokio::time::timeout(Duration::from_secs(5), watching.registered())
            .await
            .expect("registration never completed once released");
    }

    #[tokio::test]
    async fn a_watcher_that_cannot_open_gives_up_registering_and_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, activity_feed, clusters) =
            setup(tmp.path()).await;

        let mut watching = spawn(
            |_sink| -> notify::Result<HeldRegistration> {
                Err(notify::Error::generic("backend unavailable"))
            },
            doc_paths.values().cloned().collect(),
            indexer,
            clusters,
            hub,
            activity_feed,
            Arc::new(WriteCoordinator::new()),
            None,
            Settings::DEFAULT,
        );

        tokio::time::timeout(Duration::from_secs(5), watching.registered())
            .await
            .expect("registration never gave up");
        assert!(watching.finished().await.is_err());
    }

    struct ScriptedWatcher;

    impl DirectoryWatcher for ScriptedWatcher {
        fn watch_recursively(&mut self, _dir: &Path) -> notify::Result<()> {
            Ok(())
        }
    }

    struct ScriptedEvents {
        sink: EventSink,
    }

    impl ScriptedEvents {
        fn changed(&mut self, path: &Path) {
            use notify::EventHandler;
            self.sink
                .handle_event(Ok(Event::new(notify::EventKind::Modify(
                    notify::event::ModifyKind::Any,
                ))
                .add_path(path.to_path_buf())));
        }
    }

    async fn spawn_scripted(
        indexer: Arc<Indexer>,
        hub: Arc<SseHub>,
        activity_feed: Arc<ActivityRingBuffer>,
        clusters: Arc<RwLock<Vec<crate::clusters::LifecycleCluster>>>,
    ) -> ScriptedEvents {
        spawn_scripted_debouncing(
            indexer,
            hub,
            activity_feed,
            clusters,
            Duration::from_millis(5),
        )
        .await
    }

    async fn spawn_scripted_debouncing(
        indexer: Arc<Indexer>,
        hub: Arc<SseHub>,
        activity_feed: Arc<ActivityRingBuffer>,
        clusters: Arc<RwLock<Vec<crate::clusters::LifecycleCluster>>>,
        debounce: Duration,
    ) -> ScriptedEvents {
        let (sink_tx, sink_rx) = std::sync::mpsc::channel();
        spawn(
            move |sink| {
                sink_tx.send(sink).unwrap();
                Ok(ScriptedWatcher)
            },
            Vec::new(),
            indexer,
            clusters,
            hub,
            activity_feed,
            Arc::new(WriteCoordinator::new()),
            None,
            Settings { debounce },
        )
        .registered()
        .await;
        ScriptedEvents {
            sink: sink_rx.recv().unwrap(),
        }
    }

    /// A watcher fed by scripted events over `setup`'s corpus, and the
    /// broadcasts it makes.
    struct Scripted {
        tmp: tempfile::TempDir,
        events: ScriptedEvents,
        rx: tokio::sync::broadcast::Receiver<SsePayload>,
        activity_feed: Arc<ActivityRingBuffer>,
    }

    impl Scripted {
        async fn start(debounce: Duration) -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let (_, indexer, hub, activity_feed, clusters) =
                setup(tmp.path()).await;
            let rx = hub.subscribe();
            let events = spawn_scripted_debouncing(
                indexer,
                hub,
                activity_feed.clone(),
                clusters,
                debounce,
            )
            .await;
            Self {
                tmp,
                events,
                rx,
                activity_feed,
            }
        }

        fn plan(&self, name: &str) -> PathBuf {
            self.tmp.path().join("meta/plans").join(name)
        }

        fn write(&mut self, name: &str, content: &str) {
            let path = self.plan(name);
            std::fs::write(&path, content).unwrap();
            self.events.changed(&path);
        }

        fn remove(&mut self, name: &str) {
            let path = self.plan(name);
            std::fs::remove_file(&path).unwrap();
            self.events.changed(&path);
        }

        async fn next(&mut self) -> SsePayload {
            tokio::time::timeout(EVENT_BUDGET, self.rx.recv())
                .await
                .expect("timed out waiting for SSE event")
                .expect("channel closed")
        }

        async fn assert_quiet(&mut self, window: Duration) {
            assert!(
                tokio::time::timeout(window, self.rx.recv()).await.is_err(),
                "expected no broadcast",
            );
        }
    }

    fn doc_changed(event: &SsePayload) -> (ActionKind, DateTime<Utc>) {
        match event {
            SsePayload::DocChanged {
                action, timestamp, ..
            } => (*action, *timestamp),
            other => panic!("expected DocChanged, got {other:?}"),
        }
    }

    const FOO: &str = "2026-01-01-foo.md";

    #[tokio::test]
    async fn file_change_produces_doc_changed_event() {
        let mut watcher = Scripted::start(Duration::from_millis(5)).await;

        watcher.write(FOO, "---\ntitle: Foo updated\n---\n");

        // Setup indexed the file before the watcher started -> Edited.
        assert_eq!(doc_changed(&watcher.next().await).0, ActionKind::Edited);
    }

    #[tokio::test]
    async fn unchanged_content_rewrite_is_suppressed() {
        // macOS `FSEvents` replays coalesced historical events and per-write
        // bursts for files whose content never changed; broadcasting those
        // would churn SSE clients.
        let mut watcher = Scripted::start(Duration::from_millis(5)).await;
        let original = std::fs::read_to_string(watcher.plan(FOO)).unwrap();

        watcher.write(FOO, &original);

        watcher.assert_quiet(Duration::from_millis(400)).await;
        assert_eq!(
            watcher.activity_feed.recent(5).len(),
            0,
            "suppressed re-broadcast must not push an activity event",
        );
    }

    #[tokio::test]
    async fn rapid_writes_coalesce_to_one_event() {
        let debounce = Duration::from_millis(100);
        let mut watcher = Scripted::start(debounce).await;

        for i in 0..5u32 {
            watcher.write(FOO, &format!("---\ntitle: v{i}\n---\n"));
        }

        assert_eq!(doc_changed(&watcher.next().await).0, ActionKind::Edited);
        watcher.assert_quiet(debounce * 3).await;
    }

    #[tokio::test]
    async fn malformed_frontmatter_produces_doc_invalid_event() {
        let mut watcher = Scripted::start(Duration::from_millis(5)).await;

        watcher.write(FOO, "---\ntitle: \"unclosed\n---\nbody\n");

        let event = watcher.next().await;
        assert!(
            matches!(event, SsePayload::DocInvalid { .. }),
            "expected DocInvalid for malformed frontmatter, got {event:?}",
        );
    }

    #[tokio::test]
    async fn new_file_in_watched_dir_produces_doc_changed_event() {
        let mut watcher = Scripted::start(Duration::from_millis(5)).await;

        let before = Utc::now();
        watcher.write("2026-05-01-new.md", "---\ntitle: New\n---\n");
        let (action, timestamp) = doc_changed(&watcher.next().await);
        let after = Utc::now();

        assert_eq!(action, ActionKind::Created);
        assert!(
            timestamp >= before && timestamp <= after,
            "timestamp {timestamp} not within [{before}, {after}]"
        );
        // The ring buffer is pushed before the broadcast, so the event is
        // readable from the feed by the time a subscriber sees it.
        let recent = watcher.activity_feed.recent(5);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].action, ActionKind::Created);
        assert!(
            recent[0].path.ends_with("meta/plans/2026-05-01-new.md"),
            "ring-buffer path mismatch: {}",
            recent[0].path,
        );
    }

    #[tokio::test]
    async fn file_deletion_produces_doc_changed_without_etag() {
        let mut watcher = Scripted::start(Duration::from_millis(5)).await;

        watcher.remove(FOO);

        let event = watcher.next().await;
        assert_eq!(doc_changed(&event).0, ActionKind::Deleted);
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            !json.contains("etag"),
            "etag must be absent for deleted files: {json}",
        );
        let recent = watcher.activity_feed.recent(5);
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
        let mut watcher = Scripted::start(Duration::from_millis(5)).await;
        let chain = "2026-05-13-chain.md";

        watcher.write(chain, "---\ntitle: Chain v1\n---\n");
        let (created, t0) = doc_changed(&watcher.next().await);
        watcher.write(chain, "---\ntitle: Chain v2\n---\n");
        let (edited, t1) = doc_changed(&watcher.next().await);
        watcher.remove(chain);
        let (deleted, t2) = doc_changed(&watcher.next().await);

        assert_eq!(created, ActionKind::Created);
        assert_eq!(edited, ActionKind::Edited);
        assert_eq!(deleted, ActionKind::Deleted);
        assert!(t0 <= t1, "timestamp ordering created <= edited");
        assert!(t1 <= t2, "timestamp ordering edited <= deleted");
        let recent = watcher.activity_feed.recent(10);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].action, ActionKind::Deleted);
        assert_eq!(recent[1].action, ActionKind::Edited);
        assert_eq!(recent[2].action, ActionKind::Created);
    }

    /// The one test of the OS adapter itself; every other test feeds the
    /// watcher scripted events.
    #[tokio::test]
    #[ignore = "waits on the OS's file notifications, which a loaded \
                fseventsd delays by minutes; run with \
                `mise run test:integration:visualiser-os-watcher`"]
    async fn the_os_watcher_reports_a_write_under_a_registered_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().canonicalize().unwrap();
        let probe = dir.join("probe.md");
        std::fs::write(&probe, "a").unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
        let mut watcher = os_watcher(EventSink(tx)).unwrap();
        register(&mut watcher, std::slice::from_ref(&dir));

        let reports_the_probe = async {
            loop {
                let event = rx.recv().await.unwrap().unwrap();
                if event.paths.iter().any(|path| path.ends_with("probe.md")) {
                    return;
                }
            }
        };
        let rewrites = async {
            loop {
                std::fs::write(&probe, "b").unwrap();
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        };

        // A loaded fseventsd can coalesce or drop the first write, so the
        // probe is rewritten until a report arrives.
        tokio::time::timeout(Duration::from_secs(600), async {
            tokio::select! {
                () = reports_the_probe => {}
                () = rewrites => {}
            }
        })
        .await
        .expect("the OS never reported a write under the watched directory");
    }

    #[tokio::test]
    async fn a_change_another_files_rescan_absorbed_still_broadcasts() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let (_, indexer, hub, activity_feed, clusters) = setup(&root).await;
        let foo = root.join("meta/plans/2026-01-01-foo.md");
        let bar = root.join("meta/plans/2026-01-02-bar.md");
        std::fs::write(&bar, "---\ntitle: Bar\n---\n# Body\n").unwrap();
        indexer.rescan().await.unwrap();
        let mut rx = hub.subscribe();
        let mut events =
            spawn_scripted(indexer, hub, activity_feed, clusters).await;

        std::fs::write(&foo, "---\ntitle: Foo updated\n---\n").unwrap();
        events.changed(&bar);
        tokio::time::sleep(Duration::from_millis(300)).await;
        events.changed(&foo);

        let event = tokio::time::timeout(EVENT_BUDGET, rx.recv())
            .await
            .expect("the absorbed change was never broadcast")
            .expect("channel closed");
        match event {
            SsePayload::DocChanged { path, .. } => {
                assert_eq!(path, "meta/plans/2026-01-01-foo.md");
            }
            other => panic!("expected DocChanged, got {other:?}"),
        }
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

        let event = tokio::time::timeout(EVENT_BUDGET, rx.recv())
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

        let event = tokio::time::timeout(EVENT_BUDGET, rx.recv())
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
            let event = tokio::time::timeout(EVENT_BUDGET, rx.recv())
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
