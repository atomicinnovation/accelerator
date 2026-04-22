use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinHandle;

use crate::clusters::{compute_clusters, LifecycleCluster};
use crate::indexer::{IndexEntry, Indexer, FRONTMATTER_MALFORMED};
use crate::sse_hub::{SseHub, SsePayload};

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub debounce: Duration,
}

impl Settings {
    pub const DEFAULT: Settings = Settings {
        debounce: Duration::from_millis(100),
    };
}

pub fn spawn(
    dirs: Vec<PathBuf>,
    project_root: PathBuf,
    indexer: Arc<Indexer>,
    clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    hub: Arc<SseHub>,
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

    for dir in &dirs {
        if dir.exists() {
            if let Err(e) = watcher.watch(dir, RecursiveMode::NonRecursive) {
                tracing::warn!(dir = %dir.display(), error = %e, "failed to watch dir");
            } else {
                tracing::debug!(dir = %dir.display(), "watching");
            }
        }
    }

    let rescan_lock = Arc::new(Semaphore::new(1));

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
                            settings.debounce,
                            pre,
                            rescan_lock.clone(),
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

async fn on_path_changed_debounced(
    path: PathBuf,
    project_root: PathBuf,
    indexer: Arc<Indexer>,
    clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    hub: Arc<SseHub>,
    debounce: Duration,
    pre: Option<IndexEntry>,
    rescan_lock: Arc<Semaphore>,
) {
    tokio::time::sleep(debounce).await;

    let _permit = rescan_lock.acquire().await.unwrap();

    if let Err(e) = indexer.rescan().await {
        tracing::warn!(path = %path.display(), error = %e, "rescan failed after watch event");
        return;
    }

    let new_clusters = compute_clusters(&indexer.all().await);
    *clusters.write().await = new_clusters;

    let rel = path
        .strip_prefix(&project_root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");

    match indexer.get(&path).await {
        Some(entry) => {
            hub.broadcast(payload_for_entry(&entry, rel));
            tracing::debug!(file = %path.display(), "SSE event broadcast");
        }
        None => {
            if let Some(pre_entry) = pre {
                hub.broadcast(SsePayload::DocChanged {
                    doc_type: pre_entry.r#type,
                    path: rel,
                    etag: None,
                });
                tracing::debug!(file = %path.display(), "SSE doc-changed broadcast for deleted file");
            }
        }
    }
}

fn payload_for_entry(entry: &IndexEntry, rel: String) -> SsePayload {
    if entry.frontmatter_state == FRONTMATTER_MALFORMED {
        SsePayload::DocInvalid {
            doc_type: entry.r#type,
            path: rel,
        }
    } else {
        SsePayload::DocChanged {
            doc_type: entry.r#type,
            path: rel,
            etag: Some(entry.etag.clone()),
        }
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "md")
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

    async fn setup(tmp: &std::path::Path) -> (
        HashMap<String, std::path::PathBuf>,
        Arc<Indexer>,
        Arc<SseHub>,
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
            Arc::new(LocalFileDriver::new(&doc_paths, vec![]));
        let indexer = Arc::new(
            Indexer::build(driver, tmp.to_path_buf()).await.unwrap(),
        );
        let hub = Arc::new(SseHub::new(64));
        let clusters = Arc::new(RwLock::new(
            crate::clusters::compute_clusters(&indexer.all().await),
        ));
        (doc_paths, indexer, hub, clusters)
    }

    #[tokio::test]
    async fn file_change_produces_doc_changed_event() {
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, clusters) = setup(tmp.path()).await;
        let mut rx = hub.subscribe();

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            Settings { debounce: Duration::from_millis(5) },
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

        assert!(
            matches!(event, SsePayload::DocChanged { .. }),
            "expected DocChanged, got {event:?}",
        );
    }

    #[tokio::test]
    async fn rapid_writes_coalesce_to_one_event() {
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, clusters) = setup(tmp.path()).await;
        let mut rx = hub.subscribe();

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            Settings { debounce: Duration::from_millis(50) },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        let path = tmp.path().join("meta/plans/2026-01-01-foo.md");
        for i in 0..5u32 {
            std::fs::write(&path, format!("---\ntitle: v{i}\n---\n")).unwrap();
            tokio::time::sleep(Duration::from_millis(2)).await;
        }

        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert!(matches!(event, SsePayload::DocChanged { .. }));

        assert!(
            tokio::time::timeout(Duration::from_millis(200), rx.recv())
                .await
                .is_err(),
            "expected no second event but got one",
        );
    }

    #[tokio::test]
    async fn malformed_frontmatter_produces_doc_invalid_event() {
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, clusters) = setup(tmp.path()).await;
        let mut rx = hub.subscribe();

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            Settings { debounce: Duration::from_millis(5) },
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
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, clusters) = setup(tmp.path()).await;
        let mut rx = hub.subscribe();

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            Settings { debounce: Duration::from_millis(5) },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        std::fs::write(
            tmp.path().join("meta/plans/2026-05-01-new.md"),
            "---\ntitle: New\n---\n",
        )
        .unwrap();

        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");

        assert!(matches!(event, SsePayload::DocChanged { .. }));
    }

    #[tokio::test]
    async fn file_deletion_produces_doc_changed_without_etag() {
        let tmp = tempfile::tempdir().unwrap();
        let (doc_paths, indexer, hub, clusters) = setup(tmp.path()).await;
        let mut rx = hub.subscribe();
        let path = tmp.path().join("meta/plans/2026-01-01-foo.md");

        spawn(
            doc_paths.values().cloned().collect(),
            tmp.path().to_path_buf(),
            indexer,
            clusters,
            hub,
            Settings { debounce: Duration::from_millis(5) },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        std::fs::remove_file(&path).unwrap();

        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out waiting for deletion SSE event")
            .expect("channel closed");

        assert!(
            matches!(event, SsePayload::DocChanged { .. }),
            "expected DocChanged for deletion, got {event:?}",
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(
            !json.contains("etag"),
            "etag must be absent for deleted files: {json}",
        );
    }
}
