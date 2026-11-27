//! Tells subscribers what changed in the index: the SSE stream, the activity
//! feed and the lifecycle clusters all learn of a change through here.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::activity_feed::{
    ActivityEvent, ActivityRingBuffer, SetEventContext,
};
use crate::clusters::{recompute_clusters, LifecycleCluster};
use crate::indexer::{
    AssetRecord, Change, ChangeSink, IndexDiff, IndexEntry, Indexer,
    SetDocumentRole, SetPlacement, FRONTMATTER_MALFORMED,
};
use crate::server::AppState;
use crate::sse_hub::{ActionKind, SseHub, SsePayload};
use crate::write_coordinator::WriteCoordinator;

/// The server's change sink: recomputes the clusters a change may move,
/// then emits an event for each change the application did not make
/// itself.
pub struct ChangeBroadcast {
    indexer: Arc<Indexer>,
    clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
    hub: Arc<SseHub>,
    activity_feed: Arc<ActivityRingBuffer>,
    write_coordinator: Arc<WriteCoordinator>,
}

impl ChangeBroadcast {
    pub const fn new(
        indexer: Arc<Indexer>,
        clusters: Arc<RwLock<Vec<LifecycleCluster>>>,
        hub: Arc<SseHub>,
        activity_feed: Arc<ActivityRingBuffer>,
        write_coordinator: Arc<WriteCoordinator>,
    ) -> Self {
        Self {
            indexer,
            clusters,
            hub,
            activity_feed,
            write_coordinator,
        }
    }

    pub fn of(state: &AppState) -> Self {
        Self::new(
            state.indexer.clone(),
            state.clusters.clone(),
            state.sse_hub.clone(),
            state.activity_feed.clone(),
            state.write_coordinator.clone(),
        )
    }

    async fn broadcast(&self, diff: &IndexDiff) {
        if !diff.is_empty() {
            recompute_clusters(&self.indexer, &self.clusters).await;
        }
        for change in
            rescan_changes(diff, Utc::now())
                .into_iter()
                .filter(|change| {
                    !self.write_coordinator.should_suppress(&change.path)
                })
        {
            emit(change.payload, change.set, &self.hub, &self.activity_feed);
        }
    }
}

impl ChangeSink for ChangeBroadcast {
    fn changed<'a>(
        &'a self,
        diff: &'a IndexDiff,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(self.broadcast(diff))
    }
}

struct RescanChange {
    path: PathBuf,
    payload: SsePayload,
    set: Option<SetEventContext>,
}

/// The broadcasts a rescan owes its subscribers.
fn rescan_changes(diff: &IndexDiff, now: DateTime<Utc>) -> Vec<RescanChange> {
    diff.documents
        .iter()
        .filter_map(|change| document_change(change, now))
        .chain(
            diff.assets
                .iter()
                .filter_map(|change| asset_change(change, now)),
        )
        .collect()
}

fn document_change(
    change: &Change<IndexEntry>,
    now: DateTime<Utc>,
) -> Option<RescanChange> {
    let (path, payload) = match (&change.before, &change.after) {
        (before, Some(after)) => (
            after.path.clone(),
            payload_for_entry(
                after,
                after.served_rel_path(),
                before.as_ref(),
                now,
            ),
        ),
        (Some(before), None) => (
            before.path.clone(),
            SsePayload::DocChanged {
                action: ActionKind::Deleted,
                doc_type: before.r#type,
                path: before.served_rel_path(),
                etag: None,
                timestamp: now,
            },
        ),
        (None, None) => return None,
    };
    let set = change
        .after
        .as_ref()
        .or(change.before.as_ref())
        .and_then(|entry| entry.set.as_ref())
        .map(|set| set_event_context(&set.placement, set.role));
    Some(RescanChange { path, payload, set })
}

fn asset_change(
    change: &Change<AssetRecord>,
    now: DateTime<Utc>,
) -> Option<RescanChange> {
    let (asset, action) = match (&change.before, &change.after) {
        (None, Some(after)) => (after, ActionKind::Created),
        (Some(_), Some(after)) => (after, ActionKind::Edited),
        (Some(before), None) => (before, ActionKind::Deleted),
        (None, None) => return None,
    };
    Some(RescanChange {
        path: asset.path.clone(),
        payload: SsePayload::DocChanged {
            action,
            doc_type: asset.doc_type,
            path: served(&asset.rel_path),
            etag: None,
            timestamp: now,
        },
        set: Some(set_event_context(&asset.placement, SetDocumentRole::Member)),
    })
}

fn set_event_context(
    placement: &SetPlacement,
    role: SetDocumentRole,
) -> SetEventContext {
    SetEventContext {
        root_rel_path: served(&placement.context.root_rel_path),
        role,
    }
}

fn served(rel_path: &std::path::Path) -> String {
    rel_path.to_string_lossy().replace('\\', "/")
}

/// Single fan-out point for SSE events: pushes a derived `ActivityEvent`
/// into the ring buffer (if applicable — `DocInvalid` does not surface),
/// then broadcasts the payload. Ring-buffer push is **synchronous and
/// before** the broadcast, so a buffered event is always available via
/// `/api/activity` once a subscriber sees it on the live stream.
pub fn emit(
    payload: SsePayload,
    set: Option<SetEventContext>,
    hub: &SseHub,
    activity_feed: &ActivityRingBuffer,
) {
    if let Some(activity_event) = ActivityEvent::from_payload(&payload) {
        activity_feed.record(activity_event, set);
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
