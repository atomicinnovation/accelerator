//! Bounded in-memory record of recent file-change events.
//!
//! The activity feed is a domain producer/consumer pair separate from the
//! SSE broadcast hub: the watcher pushes here synchronously before
//! broadcasting, and `GET /api/activity` reads the most recent N events
//! out. Buffer state is per-process — clean restart yields an empty feed.
//!
//! **Mutex discipline** (module invariant): the `std::sync::Mutex` is
//! acceptable inside the async watcher coroutine and HTTP handler **only**
//! because critical sections are O(1) and never await. The push site holds
//! the lock for `pop_back` + `push_front` over a `VecDeque<ActivityEvent>`
//! — no allocation that can fail, no I/O, no `.await`. The read site
//! (`recent`) clones up to 50 small structs under the lock; this is
//! acceptable at the expected GET-rate (single UI client, refetched on
//! reconnect only).

use std::collections::VecDeque;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::docs::DocTypeKey;
use crate::sse_hub::{ActionKind, SsePayload};

pub const CAPACITY: usize = 50;

#[derive(Debug, Clone, Serialize)]
pub struct ActivityEvent {
    pub action: ActionKind,
    #[serde(rename = "docType")]
    pub doc_type: DocTypeKey,
    pub path: String,
    pub timestamp: DateTime<Utc>,
}

impl ActivityEvent {
    /// Projects an SSE wire-format payload into the ring-buffer's narrower
    /// shape. `DocInvalid` events do not surface in the Activity feed —
    /// returns `None`. The filter semantic (some inputs project to `None`)
    /// is intentionally visible at the call site rather than hidden inside
    /// a `From::from` invocation.
    pub fn from_payload(payload: &SsePayload) -> Option<Self> {
        match payload {
            SsePayload::DocChanged {
                action,
                doc_type,
                path,
                timestamp,
                ..
            } => Some(ActivityEvent {
                action: *action,
                doc_type: *doc_type,
                path: path.clone(),
                timestamp: *timestamp,
            }),
            SsePayload::DocInvalid { .. } => None,
        }
    }
}

/// Bounded ring buffer of recent file-change events, newest-first on read.
///
/// Critical sections must remain O(1); do not extend operations under the
/// lock to include allocation, I/O, or `.await` (see "Mutex discipline" in
/// the module docs).
pub struct ActivityRingBuffer {
    inner: Mutex<VecDeque<ActivityEvent>>,
}

impl ActivityRingBuffer {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::with_capacity(CAPACITY)),
        }
    }

    /// Pushes `event` onto the front (newest-first ordering on read).
    /// If the buffer is at `CAPACITY`, the oldest event (back) is evicted.
    pub fn push(&self, event: ActivityEvent) {
        let mut buf = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if buf.len() == CAPACITY {
            buf.pop_back();
        }
        buf.push_front(event);
    }

    /// Returns up to `limit` events, newest-first. Clones under the lock —
    /// see module-level Mutex discipline.
    pub fn recent(&self, limit: usize) -> Vec<ActivityEvent> {
        let buf = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        buf.iter().take(limit).cloned().collect()
    }
}

impl Default for ActivityRingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;

    fn ev(seconds: i64) -> ActivityEvent {
        ActivityEvent {
            action: ActionKind::Edited,
            doc_type: DocTypeKey::Plans,
            path: format!("meta/plans/t{seconds}.md"),
            timestamp: Utc.timestamp_opt(seconds, 0).unwrap(),
        }
    }

    #[test]
    fn empty_buffer_returns_empty_vec() {
        let buf = ActivityRingBuffer::new();
        assert!(buf.recent(5).is_empty());
    }

    #[test]
    fn recent_returns_newest_first() {
        let buf = ActivityRingBuffer::new();
        for i in 1..=6_i64 {
            buf.push(ev(i));
        }
        let out = buf.recent(5);
        assert_eq!(out.len(), 5);
        assert_eq!(out[0].timestamp.timestamp(), 6);
        assert_eq!(out[4].timestamp.timestamp(), 2);
        for w in out.windows(2) {
            assert!(w[0].timestamp > w[1].timestamp);
        }
    }

    #[test]
    fn at_capacity_returns_all_events() {
        let buf = ActivityRingBuffer::new();
        for i in 1..=(CAPACITY as i64) {
            buf.push(ev(i));
        }
        let out = buf.recent(CAPACITY);
        assert_eq!(out.len(), CAPACITY);
    }

    #[test]
    fn over_capacity_evicts_oldest() {
        let buf = ActivityRingBuffer::new();
        for i in 1..=(CAPACITY as i64 + 1) {
            buf.push(ev(i));
        }
        let out = buf.recent(CAPACITY);
        assert_eq!(out.len(), CAPACITY);
        assert_eq!(out[0].timestamp.timestamp(), (CAPACITY as i64) + 1);
        assert_eq!(out.last().unwrap().timestamp.timestamp(), 2);
    }

    #[test]
    fn limit_truncates_response() {
        let buf = ActivityRingBuffer::new();
        for i in 1..=10_i64 {
            buf.push(ev(i));
        }
        assert_eq!(buf.recent(3).len(), 3);
        assert_eq!(buf.recent(100).len(), 10);
        assert_eq!(buf.recent(0).len(), 0);
    }

    #[test]
    fn doc_changed_payload_projects_to_activity_event() {
        let payload = SsePayload::DocChanged {
            action: ActionKind::Created,
            doc_type: DocTypeKey::Plans,
            path: "meta/plans/x.md".into(),
            etag: Some("sha256-abc".into()),
            timestamp: Utc.timestamp_opt(42, 0).unwrap(),
        };
        let projected = ActivityEvent::from_payload(&payload).expect("DocChanged must project");
        assert_eq!(projected.action, ActionKind::Created);
        assert_eq!(projected.path, "meta/plans/x.md");
        assert_eq!(projected.timestamp.timestamp(), 42);
    }

    #[test]
    fn doc_invalid_payload_projects_to_none() {
        let payload = SsePayload::DocInvalid {
            doc_type: DocTypeKey::Plans,
            path: "meta/plans/broken.md".into(),
        };
        assert!(
            ActivityEvent::from_payload(&payload).is_none(),
            "DocInvalid must not surface in the activity feed",
        );
    }

    #[test]
    fn concurrent_pushes_from_many_threads_preserve_all_events() {
        const THREADS: i64 = 4;
        const PER_THREAD: i64 = 10; // total 40 << CAPACITY=50
        let buf = Arc::new(ActivityRingBuffer::new());
        let mut handles = Vec::new();
        for t in 0..THREADS {
            let buf = buf.clone();
            handles.push(thread::spawn(move || {
                for i in 0..PER_THREAD {
                    buf.push(ev(t * 1_000 + i));
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let out = buf.recent(CAPACITY);
        assert_eq!(out.len() as i64, THREADS * PER_THREAD);
        let unique: HashSet<i64> = out.iter().map(|e| e.timestamp.timestamp()).collect();
        assert_eq!(
            unique.len() as i64,
            THREADS * PER_THREAD,
            "every push must survive"
        );
    }
}
