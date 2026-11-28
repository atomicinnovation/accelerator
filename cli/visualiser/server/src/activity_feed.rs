//! Bounded in-memory record of recent file-change events.
//!
//! The activity feed is a domain producer/consumer pair separate from the
//! SSE broadcast hub: each change event is recorded here synchronously
//! before it is broadcast, and `GET /api/activity` reads the most recent N
//! events out. Buffer state is per-process — clean restart yields an empty
//! feed.
//!
//! **Mutex discipline** (module invariant): the `std::sync::Mutex` is
//! acceptable inside the async watcher coroutine and HTTP handler **only**
//! because critical sections are bounded by `CAPACITY` and never await. The
//! record site holds the lock to find and remove a set's row among at most
//! 50, then `pop_back` + `push_front` over a `VecDeque<ActivityEvent>` — no
//! allocation that can fail, no I/O, no `.await`. The read site (`recent`)
//! clones up to 50 small structs under the lock; this is acceptable at the
//! expected GET-rate (single UI client, refetched on each change).

use std::collections::VecDeque;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::indexer::SetDocumentRole;
use crate::sse_hub::{ActionKind, SsePayload};
use corpus::DocTypeKey;

pub const CAPACITY: usize = 50;

#[derive(Debug, Clone, Serialize)]
pub struct ActivityEvent {
    pub action: ActionKind,
    #[serde(rename = "docType", with = "crate::doc_type_serde")]
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
            SsePayload::DocInvalid { .. }
            | SsePayload::TemplateChanged { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEventContext {
    pub root_rel_path: String,
    pub role: SetDocumentRole,
}

/// Bounded ring buffer of recent file-change events, newest-first on read.
///
/// Critical sections must stay bounded by `CAPACITY`; do not extend
/// operations under the lock to include allocation, I/O, or `.await` (see
/// "Mutex discipline" in the module docs).
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
        let mut buf = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if buf.len() == CAPACITY {
            buf.pop_back();
        }
        buf.push_front(event);
    }

    /// Records `event`. A change inside a set replaces that set's row with
    /// one for its root, which keeps the root's last action: a set reads as
    /// created, edited or deleted by what happened to its root, or edited
    /// when no root action is on record.
    pub fn record(&self, event: ActivityEvent, set: Option<SetEventContext>) {
        let Some(set) = set else {
            self.push(event);
            return;
        };
        let mut buf = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = buf
            .iter()
            .position(|row| row.path == set.root_rel_path)
            .and_then(|at| buf.remove(at));
        let action = match set.role {
            SetDocumentRole::Root => event.action,
            SetDocumentRole::Member => {
                previous.map_or(ActionKind::Edited, |row| row.action)
            }
        };
        if buf.len() == CAPACITY {
            buf.pop_back();
        }
        buf.push_front(ActivityEvent {
            action,
            path: set.root_rel_path,
            ..event
        });
    }

    /// Returns up to `limit` events, newest-first. Clones under the lock —
    /// see module-level Mutex discipline.
    pub fn recent(&self, limit: usize) -> Vec<ActivityEvent> {
        let buf = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
            doc_type: DocTypeKey::PLANS,
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
            doc_type: DocTypeKey::PLANS,
            path: "meta/plans/x.md".into(),
            etag: Some("sha256-abc".into()),
            timestamp: Utc.timestamp_opt(42, 0).unwrap(),
        };
        let projected = ActivityEvent::from_payload(&payload)
            .expect("DocChanged must project");
        assert_eq!(projected.action, ActionKind::Created);
        assert_eq!(projected.path, "meta/plans/x.md");
        assert_eq!(projected.timestamp.timestamp(), 42);
    }

    #[test]
    fn doc_invalid_payload_projects_to_none() {
        let payload = SsePayload::DocInvalid {
            doc_type: DocTypeKey::PLANS,
            path: "meta/plans/broken.md".into(),
        };
        assert!(
            ActivityEvent::from_payload(&payload).is_none(),
            "DocInvalid must not surface in the activity feed",
        );
    }

    fn set_event(
        action: ActionKind,
        set: &str,
        role: SetDocumentRole,
        member: &str,
        seconds: i64,
    ) -> (ActivityEvent, Option<SetEventContext>) {
        let root = format!("meta/research/topics/{set}/manifest.md");
        let path = match role {
            SetDocumentRole::Root => root.clone(),
            SetDocumentRole::Member => {
                format!("meta/research/topics/{set}/{member}")
            }
        };
        (
            ActivityEvent {
                action,
                doc_type: DocTypeKey::TOPIC_RESEARCH,
                path,
                timestamp: Utc.timestamp_opt(seconds, 0).unwrap(),
            },
            Some(SetEventContext {
                root_rel_path: root,
                role,
            }),
        )
    }

    fn record_all(
        buf: &ActivityRingBuffer,
        events: Vec<(ActivityEvent, Option<SetEventContext>)>,
    ) {
        for (event, set) in events {
            buf.record(event, set);
        }
    }

    fn rows(buf: &ActivityRingBuffer) -> Vec<(ActionKind, String, i64)> {
        buf.recent(CAPACITY)
            .into_iter()
            .map(|e| (e.action, e.path, e.timestamp.timestamp()))
            .collect()
    }

    const A_ROOT: &str = "meta/research/topics/a/manifest.md";
    const B_ROOT: &str = "meta/research/topics/b/manifest.md";

    #[test]
    fn a_sets_changes_roll_up_into_one_row_for_its_root() {
        use SetDocumentRole::{Member, Root};
        let buf = ActivityRingBuffer::new();

        record_all(
            &buf,
            vec![
                set_event(ActionKind::Edited, "a", Member, "brief.md", 1),
                set_event(ActionKind::Created, "a", Member, "outline.md", 2),
                set_event(ActionKind::Edited, "b", Member, "brief.md", 3),
                set_event(ActionKind::Edited, "a", Root, "", 4),
                set_event(ActionKind::Edited, "a", Member, "brief.md", 5),
            ],
        );

        assert_eq!(
            rows(&buf),
            [
                (ActionKind::Edited, A_ROOT.to_owned(), 5),
                (ActionKind::Edited, B_ROOT.to_owned(), 3),
            ]
        );
    }

    #[test]
    fn a_set_whose_root_was_edited_reads_edited_after_member_changes() {
        use SetDocumentRole::{Member, Root};
        let buf = ActivityRingBuffer::new();

        record_all(
            &buf,
            vec![
                set_event(ActionKind::Edited, "a", Root, "", 1),
                set_event(ActionKind::Created, "a", Member, "outline.md", 2),
                set_event(ActionKind::Deleted, "a", Member, "brief.md", 3),
            ],
        );

        assert_eq!(rows(&buf), [(ActionKind::Edited, A_ROOT.to_owned(), 3)]);
    }

    #[test]
    fn a_created_set_still_reads_created_after_member_changes() {
        use SetDocumentRole::{Member, Root};
        let buf = ActivityRingBuffer::new();

        record_all(
            &buf,
            vec![
                set_event(ActionKind::Created, "a", Root, "", 1),
                set_event(ActionKind::Edited, "a", Member, "brief.md", 7),
            ],
        );

        assert_eq!(rows(&buf), [(ActionKind::Created, A_ROOT.to_owned(), 7)]);
    }

    #[test]
    fn a_set_with_no_recorded_root_action_reads_edited() {
        use SetDocumentRole::Member;
        let buf = ActivityRingBuffer::new();

        record_all(
            &buf,
            vec![set_event(ActionKind::Created, "a", Member, "brief.md", 1)],
        );

        assert_eq!(rows(&buf), [(ActionKind::Edited, A_ROOT.to_owned(), 1)]);
    }

    #[test]
    fn a_deleted_root_reads_deleted_after_its_members_go() {
        use SetDocumentRole::{Member, Root};
        let buf = ActivityRingBuffer::new();

        record_all(
            &buf,
            vec![
                set_event(ActionKind::Deleted, "a", Root, "", 1),
                set_event(ActionKind::Deleted, "a", Member, "brief.md", 1),
            ],
        );

        assert_eq!(rows(&buf), [(ActionKind::Deleted, A_ROOT.to_owned(), 1)]);
    }

    #[test]
    fn changes_outside_sets_each_take_a_row() {
        let buf = ActivityRingBuffer::new();

        for seconds in 1..=3 {
            let mut event = ev(seconds);
            event.path = "meta/plans/same.md".into();
            buf.record(event, None);
        }

        assert_eq!(rows(&buf).len(), 3);
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
        let unique: HashSet<i64> =
            out.iter().map(|e| e.timestamp.timestamp()).collect();
        assert_eq!(
            unique.len() as i64,
            THREADS * PER_THREAD,
            "every push must survive"
        );
    }
}
