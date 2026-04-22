---
date: "2026-04-22T00:00:00+01:00"
type: plan
skill: create-plan
status: draft
---

# Meta visualiser Phase 4 — SSE hub and notify watcher

## Overview

Phase 4 adds real-time file-change notifications to the visualiser server. It
implements two new modules (`sse_hub`, `watcher`), the `GET /api/events` SSE
endpoint, and wires everything into `AppState` and `server::run()`. The goal
is: edit a file on disk, listen on the SSE endpoint, see a `doc-changed`
event arrive within ~100ms.

The approach is test-driven throughout: tests are written before each
implementation step. Every module has co-located `#[cfg(test)]` tests;
the integration tests in `server/tests/` exercise the full router.

## Current state

Phases 1-3 are complete. The server (`skills/visualisation/visualise/server/`)
has:

- `file_driver.rs` — `FileDriver` trait + `LocalFileDriver` impl. **No watch
  method exists** — watching is not mediated by `FileDriver`.
- `indexer.rs` — `Indexer` with `rescan()` (full re-index, `&self`, safe to
  call from any task that holds `Arc<Indexer>`). No incremental update.
- `server.rs` — `AppState` struct + `run()`. Existing background-task pattern
  in `lifecycle::spawn()` is the canonical model to follow.
- `api/mod.rs` — `mount()` wires all current routes. SSE route is absent.
- `Cargo.toml` — no `notify` crate; no `tokio-stream` crate. `tokio` already
  has the `sync` feature (covers `broadcast`, `mpsc`, `RwLock`).

## Desired end state

- `GET /api/events` streams `text/event-stream` to any subscriber.
- Each SSE data payload is a JSON object: `{ "type": "doc-changed",
  "docType": "<key>", "path": "<rel-path>", "etag": "<sha256-hex>" }` on a
  file add or modify, or `{ "type": "doc-invalid", "docType": "<key>",
  "path": "<rel-path>" }` when the changed file has malformed frontmatter.
- File deletions emit `{ "type": "doc-changed", "docType": "<key>",
  "path": "<rel-path>" }` with no `etag` field (signals removal to the
  client).
- Watching is non-recursive, one watcher per configured doc-type directory.
  (Template directory watching is deferred — see "What we are NOT doing".)
- Per-path debounce is 100ms in production; tests pass a shorter value via a
  `Settings` parameter analogous to `lifecycle::Settings`.
- `AppState` gains a new public field: `sse_hub: Arc<SseHub>`.
- `cargo test` passes with all new unit and integration tests green.

### Verification

```bash
cd skills/visualisation/visualise/server
cargo test
```

Manual verification:
1. Start server against a real meta directory.
2. `curl -N http://127.0.0.1:<port>/api/events` in one terminal.
3. Edit any `.md` file in a watched directory.
4. Within ~200ms a `doc-changed` event appears in the curl terminal.
5. Introduce malformed frontmatter (unclosed YAML string). A `doc-invalid`
   event appears.
6. Delete the file. A `doc-changed` event appears with no `etag`.

## What we are NOT doing

- Watching `notify` events recursively (each dir walk is flat).
- Adding a `watch` method to the `FileDriver` trait (the watcher reads
  `cfg.doc_paths` directly).
- Implementing incremental per-file index updates — `rescan()` is a full
  re-scan; that is the intended design for v1.
- A `CancellationToken` for the watcher task — the task exits naturally
  when the process is killed (SIGTERM/SIGINT) since tokio drops spawned
  tasks on runtime shutdown.
- Implementing the client-side SSE listener — that is Phase 5.
- Watching template tier-1 and tier-2 directories (per spec D9) — deferred to a
  follow-on phase. The `watch_dirs` collection in Step 5c covers only
  `cfg.doc_paths`; template paths live in `cfg.templates` and are not included.

---

## Implementation approach

Phase 4 follows TDD in five ordered steps:

1. **`sse_hub` module** — write tests, implement, verify.
2. **`watcher` module** — write tests with configurable debounce, implement,
   verify.
3. **`GET /api/events` route** — write integration test, implement, verify.
4. **Wire into `AppState` and `run()`** — add the hub field, spawn the
   watcher, register the SSE route.
5. **End-to-end integration test** — filesystem mutation → SSE event via
   the full router.

---

## Step 1: Dependencies

### File: `skills/visualisation/visualise/server/Cargo.toml`

Add two dependencies under `[dependencies]`:

```toml
notify = "6"
tokio-stream = "0.1"
```

`notify = "6"` provides `RecommendedWatcher` (FSEvents on macOS, inotify on
Linux, ReadDirectoryChangesW on Windows — backend selected automatically).
No feature flags are required. Note: this project targets macOS and Linux
only (the existing codebase uses `nix` and `tokio::signal::unix`).

`tokio-stream = "0.1"` provides `BroadcastStream<T>`, which adapts a
`tokio::sync::broadcast::Receiver<T>` into a `futures::Stream`. This is
required by the axum `Sse` response type.

### Success criteria

```bash
cargo build 2>&1 | grep -c error
# must output 0
```

---

## Step 2: `sse_hub` module (TDD)

### File: `skills/visualisation/visualise/server/src/sse_hub.rs` (new)

#### 2a. Write the tests first

Write the `#[cfg(test)] mod tests` block below. Tests drive the public API
shape:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast::error::RecvError;

    fn make_event(path: &str) -> SsePayload {
        SsePayload::DocChanged {
            doc_type: crate::docs::DocTypeKey::Plans,
            path: path.to_string(),
            etag: Some("sha256-abc".to_string()),
        }
    }

    #[tokio::test]
    async fn single_subscriber_receives_broadcast() {
        let hub = SseHub::new(16);
        let mut rx = hub.subscribe();
        hub.broadcast(make_event("meta/plans/foo.md"));
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, SsePayload::DocChanged { .. }));
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive_broadcast() {
        let hub = SseHub::new(16);
        let mut rx1 = hub.subscribe();
        let mut rx2 = hub.subscribe();
        hub.broadcast(make_event("meta/plans/foo.md"));
        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }

    #[tokio::test]
    async fn send_with_no_subscribers_does_not_panic() {
        let hub = SseHub::new(16);
        // No subscribers — must not panic.
        hub.broadcast(make_event("meta/plans/foo.md"));
    }

    #[tokio::test]
    async fn slow_consumer_gets_lagged_error() {
        let hub = SseHub::new(2); // tiny capacity to force lag
        let mut rx = hub.subscribe();
        // Overflow the channel.
        for i in 0..10u32 {
            hub.broadcast(SsePayload::DocChanged {
                doc_type: crate::docs::DocTypeKey::Plans,
                path: format!("meta/plans/{i}.md"),
                etag: Some("sha256-x".into()),
            });
        }
        // First recv should return Lagged; subsequent recv recovers.
        assert!(matches!(rx.recv().await, Err(RecvError::Lagged(_))));
        hub.broadcast(make_event("meta/plans/recovery.md"));
        assert!(rx.recv().await.is_ok(), "expected recovery after lag");
    }

    #[test]
    fn sse_payload_json_wire_format() {
        // Normal doc-changed: etag present.
        let changed = SsePayload::DocChanged {
            doc_type: crate::docs::DocTypeKey::Plans,
            path: "meta/plans/foo.md".into(),
            etag: Some("sha256-abc".into()),
        };
        let json = serde_json::to_string(&changed).unwrap();
        assert!(json.contains("\"type\":\"doc-changed\""), "json: {json}");
        assert!(json.contains("\"docType\":"), "json: {json}");
        assert!(json.contains("\"etag\":\"sha256-abc\""), "json: {json}");

        // Deletion: etag absent.
        let deleted = SsePayload::DocChanged {
            doc_type: crate::docs::DocTypeKey::Plans,
            path: "meta/plans/foo.md".into(),
            etag: None,
        };
        let json = serde_json::to_string(&deleted).unwrap();
        assert!(!json.contains("etag"), "etag must be absent for deletions: {json}");

        // Doc-invalid.
        let invalid = SsePayload::DocInvalid {
            doc_type: crate::docs::DocTypeKey::Plans,
            path: "meta/plans/bad.md".into(),
        };
        let json = serde_json::to_string(&invalid).unwrap();
        assert!(json.contains("\"type\":\"doc-invalid\""), "json: {json}");
    }
}
```

#### 2b. Implement `sse_hub.rs`

```rust
//! Broadcast hub for SSE doc-changed / doc-invalid events.

use serde::Serialize;
use tokio::sync::broadcast;

use crate::docs::DocTypeKey;

/// Wire-format payload for every SSE event the server emits.
///
/// Serialises with `#[serde(tag = "type")]` so the JSON includes a
/// `"type"` discriminant matching the spec's `doc-changed` /
/// `doc-invalid` strings.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SsePayload {
    DocChanged {
        #[serde(rename = "docType")]
        doc_type: DocTypeKey,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        etag: Option<String>,
    },
    DocInvalid {
        #[serde(rename = "docType")]
        doc_type: DocTypeKey,
        path: String,
    },
}

/// Shared broadcast hub. Stored in `AppState` as `Arc<SseHub>`.
/// Slow consumers receive `RecvError::Lagged` — the channel drops
/// the oldest messages (broadcast semantics).
pub struct SseHub {
    tx: broadcast::Sender<SsePayload>,
}

impl SseHub {
    /// `capacity` is the broadcast channel buffer size. Production: 256.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Subscribe. The returned receiver must be converted to a stream
    /// by the SSE handler via `tokio_stream::wrappers::BroadcastStream`.
    pub fn subscribe(&self) -> broadcast::Receiver<SsePayload> {
        self.tx.subscribe()
    }

    /// Broadcast an event to all current subscribers.
    /// Returns silently if there are zero subscribers.
    pub fn broadcast(&self, payload: SsePayload) {
        let _ = self.tx.send(payload);
    }
}
```

`SsePayload::DocChanged.etag` is `Option<String>`. Normal events pass `etag: Some(entry.etag.clone())`; deletion events pass `etag: None`. The field is omitted from JSON when `None` via `skip_serializing_if = "Option::is_none"`.

#### Success criteria

```bash
cd skills/visualisation/visualise/server
cargo test sse_hub
# all 4 tests must pass
```

---

## Step 3: `watcher` module (TDD)

### File: `skills/visualisation/visualise/server/src/watcher.rs` (new)

#### 3a. Write the tests first

The key testing challenge is the 100ms debounce. Mirror the `lifecycle::Settings`
pattern — pass a `Settings` struct with configurable `debounce` duration so tests
can use 5ms without slowing the suite.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::DocTypeKey;
    use crate::file_driver::LocalFileDriver;
    use crate::indexer::Indexer;
    use crate::sse_hub::{SseHub, SsePayload};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::RwLock;

    /// Seed a temp directory with a single plans dir containing one file.
    /// Returns (tmp dir, doc_paths, Arc<Indexer>, Arc<SseHub>, clusters lock).
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

        // Give the watcher a moment to register with the OS.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Mutate a file.
        std::fs::write(
            tmp.path().join("meta/plans/2026-01-01-foo.md"),
            "---\ntitle: Foo updated\n---\n",
        )
        .unwrap();

        // Expect event within 500ms.
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
            // 50ms debounce + 2ms write spacing: the debounce timer resets on
            // each event, so all five writes are coalesced on both Linux (inotify
            // delivers individual events) and macOS (FSEvents may batch them, but
            // the debounce is still the coalescing mechanism under test).
            Settings { debounce: Duration::from_millis(50) },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;

        let path = tmp.path().join("meta/plans/2026-01-01-foo.md");
        for i in 0..5u32 {
            std::fs::write(&path, format!("---\ntitle: v{i}\n---\n")).unwrap();
            tokio::time::sleep(Duration::from_millis(2)).await;
        }

        // Wait for the debounce to fire.
        let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert!(matches!(event, SsePayload::DocChanged { .. }));

        // No second event should arrive within 200ms.
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

        // Deletion must emit DocChanged with no etag.
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
```

#### 3b. Implement `watcher.rs`

Add `pub const FRONTMATTER_MALFORMED: &str = "malformed";` to `src/indexer.rs` (next to the `FrontmatterState` string definitions). The watcher references this constant rather than a bare string literal.

```rust
//! Filesystem watcher: wraps `notify` with per-path debounce,
//! calls `Indexer::rescan()` on change, recomputes clusters, and
//! broadcasts SSE events via `SseHub`.

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

/// Spawn the filesystem watcher. Returns the task handle — callers should
/// supervise it and log or abort on unexpected exit.
///
/// `dirs` — absolute paths to watch (non-recursive). Missing directories
///   are skipped with a warning.
/// `project_root` — used to compute `rel_path` in SSE payloads.
pub fn spawn(
    dirs: Vec<PathBuf>,
    project_root: PathBuf,
    indexer: Arc<Indexer>,
    clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    hub: Arc<SseHub>,
    settings: Settings,
) -> JoinHandle<()> {
    // Bounded channel: if the OS delivers events faster than we can process
    // them (e.g. a large git checkout), we drop and warn rather than
    // exhausting heap memory.
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

    // Semaphore(1): serialise concurrent rescans so that a burst of file
    // changes does not stack multiple full-corpus reads under the index
    // write lock simultaneously.
    let rescan_lock = Arc::new(Semaphore::new(1));

    tokio::spawn(async move {
        let _watcher = watcher; // keep alive for the task lifetime
        let mut pending: HashMap<PathBuf, JoinHandle<()>> = HashMap::new();

        while let Some(result) = rx.recv().await {
            match result {
                Ok(event) => {
                    for path in event.paths {
                        if !is_markdown(&path) {
                            continue;
                        }
                        // Capture the pre-event index state *now*, before the
                        // debounce sleep. A concurrent rescan triggered by a
                        // different path could otherwise sweep this entry from
                        // the index before the debounce fires, silently
                        // dropping deletion events.
                        let pre = indexer.get(&path).await;

                        // Abort any pending debounce for this path and evict
                        // completed handles to prevent unbounded map growth.
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

    // Serialise rescans: at most one full-corpus read under the index write
    // lock at a time.
    let _permit = rescan_lock.acquire().await.unwrap();

    if let Err(e) = indexer.rescan().await {
        tracing::warn!(path = %path.display(), error = %e, "rescan failed after watch event");
        return;
    }

    // Recompute clusters outside the write lock, then swap in atomically.
    let new_clusters = compute_clusters(&indexer.all().await);
    *clusters.write().await = new_clusters;

    // Forward-slash normalisation ensures SSE payloads use consistent
    // separators regardless of host OS.
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
            // File was deleted. Use the pre-event entry for doc_type; if
            // pre is also None (entry was never indexed) we cannot
            // determine the type and skip broadcasting.
            if let Some(pre_entry) = pre {
                hub.broadcast(SsePayload::DocChanged {
                    doc_type: pre_entry.r#type,
                    path: rel,
                    etag: None, // absent etag signals deletion to the client
                });
                tracing::debug!(file = %path.display(), "SSE doc-changed broadcast for deleted file");
            }
        }
    }
}

/// Build the SSE payload for a file that exists in the post-rescan index.
fn payload_for_entry(entry: &IndexEntry, rel: String) -> SsePayload {
    if entry.frontmatter_state == FRONTMATTER_MALFORMED {
        SsePayload::DocInvalid {
            doc_type: entry.r#type.clone(),
            path: rel,
        }
    } else {
        SsePayload::DocChanged {
            doc_type: entry.r#type.clone(),
            path: rel,
            etag: Some(entry.etag.clone()),
        }
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "md")
}
```

#### Success criteria

```bash
cd skills/visualisation/visualise/server
cargo test watcher
# all 4 watcher tests must pass
```

---

## Step 4: `GET /api/events` SSE route (TDD)

### File: `skills/visualisation/visualise/server/src/api/events.rs` (new)

#### 4a. Write the integration test first

Add to `skills/visualisation/visualise/server/src/api/events.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::AppState;
    use crate::sse_hub::{SseHub, SsePayload};
    use crate::docs::DocTypeKey;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;
    use http_body_util::BodyExt;

    async fn minimal_state() -> Arc<AppState> {
        // Use the same helper pattern as server.rs tests.
        let tmp = tempfile::tempdir().unwrap();
        let cfg = crate::config::Config {
            plugin_root: tmp.path().to_path_buf(),
            plugin_version: "test".into(),
            project_root: tmp.path().to_path_buf(),
            tmp_path: tmp.path().to_path_buf(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: tmp.path().join("server.log"),
            doc_paths: Default::default(),
            templates: Default::default(),
        };
        let activity = Arc::new(crate::activity::Activity::new());
        AppState::build(cfg, activity).await.unwrap()
    }

    #[tokio::test]
    async fn events_endpoint_returns_text_event_stream() {
        let state = minimal_state().await;
        let app = crate::server::build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/events")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let ct = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.starts_with("text/event-stream"),
            "expected text/event-stream, got {ct}",
        );
    }

    #[tokio::test]
    async fn hub_event_arrives_on_sse_stream() {
        use http_body_util::BodyExt as _;

        let state = minimal_state().await;
        let hub = state.sse_hub.clone();
        let app = crate::server::build_router(state);

        // Open the SSE connection first. The handler subscribes to the
        // broadcast channel during request processing, so the receiver
        // exists once oneshot resolves.
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/events")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Broadcast *after* the handler has subscribed. tokio broadcast
        // channels only deliver messages to receivers that already exist
        // at send time — broadcasting before subscribe would silently
        // discard the event.
        hub.broadcast(SsePayload::DocChanged {
            doc_type: DocTypeKey::Plans,
            path: "meta/plans/foo.md".into(),
            etag: Some("sha256-abc".into()),
        });

        // Read exactly one SSE frame. The stream is infinite, so we must
        // NOT call collect() — it would block forever.
        let frame = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            response.into_body().frame(),
        )
        .await
        .expect("timed out waiting for first SSE frame")
        .expect("body error")
        .expect("stream ended before first frame");

        let bytes = frame.into_data().expect("expected data frame");
        let text = std::str::from_utf8(&bytes).unwrap();

        assert!(
            text.contains("doc-changed"),
            "SSE frame did not contain 'doc-changed': {text}",
        );
        assert!(text.contains("sha256-abc"), "frame: {text}");
    }
}
```

**Note**: The second test broadcasts *after* the SSE handler has subscribed
(i.e. after `oneshot` resolves). `tokio::sync::broadcast` only delivers
messages to receivers that exist at send time — broadcasting before the
handler subscribes would silently discard the event. Never use `collect()`
on an SSE body — the stream is infinite and `collect()` blocks forever.

#### 4b. Implement `events.rs`

```rust
//! `GET /api/events` — SSE stream of doc-changed / doc-invalid events.

use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;

use crate::server::AppState;
use crate::sse_hub::SsePayload;

pub async fn events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.sse_hub.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| {
        match msg {
            Ok(payload) => {
                let data = serde_json::to_string(&payload).ok()?;
                Some(Ok::<Event, Infallible>(Event::default().data(data)))
            }
            // Lagged — the subscriber fell behind; skip the dropped messages.
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                tracing::warn!(dropped = n, "SSE subscriber lagged; messages dropped");
                None
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
```

`KeepAlive::default()` sends a `: keep-alive` comment every 15 seconds to
prevent proxies and load balancers from closing idle SSE connections.

#### 4c. Wire `events.rs` into `api/mod.rs`

**File**: `skills/visualisation/visualise/server/src/api/mod.rs`

Add `mod events;` at the top and add the route:

```rust
mod events;
// ... (existing mod declarations)

pub fn mount(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        // ... existing routes ...
        .route("/api/events", get(events::events))
}
```

#### Success criteria

```bash
cd skills/visualisation/visualise/server
cargo test api::events
# both SSE route tests must pass
```

---

## Step 5: Wire into `AppState` and `server::run()`

### File: `skills/visualisation/visualise/server/src/server.rs`

#### 5a. Add `sse_hub` field to `AppState`

```rust
pub struct AppState {
    pub cfg: Arc<Config>,
    pub file_driver: Arc<crate::file_driver::LocalFileDriver>,
    pub indexer: Arc<crate::indexer::Indexer>,
    pub templates: Arc<crate::templates::TemplateResolver>,
    pub clusters: Arc<RwLock<Vec<crate::clusters::LifecycleCluster>>>,
    pub activity: Arc<crate::activity::Activity>,
    pub sse_hub: Arc<crate::sse_hub::SseHub>, // NEW
}
```

#### 5b. Initialise `sse_hub` in `AppState::build()`

```rust
pub async fn build(
    cfg: Config,
    activity: Arc<crate::activity::Activity>,
) -> Result<Arc<Self>, AppStateError> {
    // ... existing code up to clusters ...
    let sse_hub = Arc::new(crate::sse_hub::SseHub::new(256));
    Ok(Arc::new(Self {
        cfg,
        file_driver: driver,
        indexer,
        templates,
        clusters,
        activity,
        sse_hub, // NEW
    }))
}
```

#### 5c. Spawn the watcher in `server::run()`

After `AppState::build()` and before `axum::serve(...)`, add:

```rust
// Collect watchable directories: all configured doc-type dirs.
// Template directories are not included (deferred to a future phase).
let watch_dirs: Vec<std::path::PathBuf> =
    state.cfg.doc_paths.values().cloned().collect();

let watcher_handle = crate::watcher::spawn(
    watch_dirs,
    state.cfg.project_root.clone(),
    state.indexer.clone(),
    state.clusters.clone(),
    state.sse_hub.clone(),
    crate::watcher::Settings::DEFAULT,
);

// Supervise the watcher task. If it exits (e.g. due to a panic), log a
// prominent error so operators know file-change notifications are disabled.
tokio::spawn(async move {
    if let Err(e) = watcher_handle.await {
        tracing::error!(
            error = %e,
            "filesystem watcher task exited unexpectedly; \
             file-change notifications are disabled until the server restarts",
        );
    }
});
```

Place this block immediately after the `lifecycle::spawn(...)` call so the
ordering is: signals → lifecycle → watcher → axum serve.

#### 5d. Update `lib.rs` and `indexer.rs`

**File**: `skills/visualisation/visualise/server/src/lib.rs`

Add:
```rust
pub mod sse_hub;
pub mod watcher;
```

**File**: `skills/visualisation/visualise/server/src/indexer.rs`

Add a public constant next to the `FrontmatterState` string definitions so the
watcher can reference it without a magic string:
```rust
pub const FRONTMATTER_MALFORMED: &str = "malformed";
```

#### Success criteria

```bash
cd skills/visualisation/visualise/server
cargo test
# full suite must pass with 0 failures
cargo build 2>&1 | grep -c error
# must output 0
```

---

## Step 6: End-to-end integration test (file mutation → SSE event)

### File: `skills/visualisation/visualise/server/tests/sse_e2e.rs` (new)

This test spins up the full server (as in `server.rs`'s `serves_placeholder_root_and_writes_info` test), opens an SSE connection via `reqwest`, mutates a file in a watched directory, and asserts the event arrives.

```rust
//! End-to-end test: file mutation → SSE event via the full server.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::test]
async fn file_mutation_arrives_as_sse_event() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(plans.join("2026-01-01-test.md"), "---\ntitle: T\n---\n").unwrap();

    let mut doc_paths = HashMap::new();
    doc_paths.insert("plans".into(), plans.clone());

    let cfg = accelerator_visualiser::config::Config {
        plugin_root: tmp.path().to_path_buf(),
        plugin_version: "test".into(),
        project_root: tmp.path().to_path_buf(),
        tmp_path: tmp.path().to_path_buf(),
        host: "127.0.0.1".into(),
        owner_pid: 0,
        owner_start_time: None,
        log_path: tmp.path().join("server.log"),
        doc_paths,
        templates: HashMap::new(),
    };

    let info_path = tmp.path().join("server-info.json");
    let info_path_clone = info_path.clone();

    // Spawn server.
    let _handle = tokio::spawn(async move {
        accelerator_visualiser::server::run(cfg, &info_path_clone)
            .await
            .unwrap();
    });

    // Wait for server-info.json.
    let start = std::time::Instant::now();
    let port = loop {
        if let Ok(bytes) = std::fs::read(&info_path) {
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            break v["port"].as_u64().unwrap() as u16;
        }
        if start.elapsed().as_secs() > 5 {
            panic!("server-info.json did not appear in 5s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    // Open SSE stream. reqwest streams are lazy — we read frame-by-frame.
    // NOTE: this test requires kernel filesystem notifications (inotify on
    // Linux, FSEvents on macOS). In containerised CI environments using
    // overlayfs or with exhausted inotify watch quotas, the test may be
    // skipped or flaky. Ensure `fs.inotify.max_user_watches` is sufficient
    // (≥ 8192 recommended) on Linux CI runners.
    let url = format!("http://127.0.0.1:{port}/api/events");
    let client = reqwest::Client::new();
    let mut sse_response = client
        .get(&url)
        .send()
        .await
        .expect("GET /api/events failed");
    assert_eq!(sse_response.status(), 200);

    // Give the watcher time to register with the OS.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Mutate a watched file.
    std::fs::write(plans.join("2026-01-01-test.md"), "---\ntitle: Updated\n---\n").unwrap();

    // Read SSE frames until we see "doc-changed" or time out.
    // 2000ms deadline: 100ms debounce + OS notification latency + reqwest
    // round-trip, with generous headroom for slow CI runners.
    let deadline = tokio::time::Instant::now() + Duration::from_millis(2000);
    let mut found = false;
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(300), sse_response.chunk()).await {
            Ok(Ok(Some(chunk))) => {
                let text = std::str::from_utf8(&chunk).unwrap_or("");
                if text.contains("doc-changed") {
                    found = true;
                    break;
                }
            }
            Ok(Ok(None)) => break, // stream closed
            Ok(Err(e)) => panic!("reqwest error reading SSE stream: {e}"),
            Err(_) => break, // timeout — no more chunks arriving
        }
    }
    assert!(found, "expected doc-changed SSE event within 2000ms");
}
```

**Note on `watcher::Settings` in `run()`**: the watcher is spawned with
`Settings::DEFAULT` (100ms debounce). This test allows 2000ms total — enough
headroom for the debounce plus OS notification latency on any reasonable CI
runner. The E2E test requires kernel filesystem notifications (inotify/FSEvents)
to be functional on the test host; see the inline comment for CI requirements.

#### Success criteria

```bash
cd skills/visualisation/visualise/server
cargo test --test sse_e2e
# must pass
```

---

## Full success criteria

### Automated verification

- [ ] No compilation errors: `cargo build`
- [ ] All unit tests pass: `cargo test`
- [ ] All integration tests pass: `cargo test --tests`
- [ ] Specific suites:
  - `cargo test sse_hub` — 6 tests (4 original + lagged recovery + wire-format)
  - `cargo test watcher` — 5 tests (4 original + deletion)
  - `cargo test api::events` — 2 tests
  - `cargo test --test sse_e2e` — 1 test

### Manual verification

- [ ] Start server with `ACCELERATOR_VISUALISER_BIN=<dev binary> ./scripts/launch-server.sh`.
- [ ] `curl -N http://127.0.0.1:<port>/api/events` streams `text/event-stream`.
- [ ] Editing a `.md` file in any watched directory produces a `doc-changed`
      event in the curl output within ~200ms.
- [ ] Introducing malformed frontmatter produces `doc-invalid`.
- [ ] Deleting a file produces `doc-changed` with no `etag` field.
- [ ] Second browser tab connected to `/api/events` receives the same events.

---

## Implementation sequence

Implement in this order within a single session:

1. [x] `Cargo.toml` — add `notify` + `tokio-stream`.
2. [x] `src/sse_hub.rs` — tests (6) then implementation.
3. [x] `src/indexer.rs` — add `pub const FRONTMATTER_MALFORMED`.
4. [x] `src/lib.rs` — add `pub mod sse_hub; pub mod watcher;`.
5. [x] `src/server.rs` — add `sse_hub` field to `AppState` + `AppState::build()`.
6. [x] `src/watcher.rs` — tests (5) then implementation.
7. [x] `src/api/events.rs` — tests then implementation.
8. [x] `src/api/mod.rs` — add `mod events;` + route.
9. [x] `src/server.rs` — `watcher::spawn(...)` + supervisor in `run()`.
10. [x] `tests/sse_e2e.rs` — end-to-end integration test.
11. [x] Run full suite: `cargo test`.

Stop after each step to verify the new tests pass before proceeding.

---

## References

- Spec: `meta/specs/2026-04-17-meta-visualisation-design.md` §§ Server
  components, SSE hub, watcher, Live updates, Failure modes.
- Research: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
  §§ Phase 4, D3, D5.
- Canonical background-task pattern: `src/lifecycle.rs` (`spawn` + `Settings`).
- `AppState` construction: `src/server.rs:50-75`.
- `Indexer::rescan()`: `src/indexer.rs:50-128`.
- `compute_clusters`: `src/clusters.rs` (`compute_clusters`).
- `DocTypeKey`: `src/docs.rs`.
