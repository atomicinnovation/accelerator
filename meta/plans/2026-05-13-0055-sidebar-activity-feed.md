---
date: "2026-05-13T10:30:00+01:00"
type: plan
skill: create-plan
work-item: "0055"
status: implemented
---

# 0055 — Sidebar Activity Feed and SSE Action Discriminator: Implementation Plan

## Overview

Add an Activity feed to the Sidebar that shows the most recent file-change
events with action labels and relative timestamps. The work spans both halves of
the stack:

- **Server**: extend `SsePayload::DocChanged` with `action` (`created`/`edited`/
  `deleted`) and `timestamp` (ISO-8601), and add a new
  `GET /api/activity?limit=N` route backed by an in-memory ring buffer.
- **Frontend**: extend `DocEventsHandle` with a multi-subscriber API, add the
  `ActivityFeed` component (initial-history `useQuery`, prepend-on-SSE, LIVE
  badge, 60 s ticker, empty state), and mount it as a new section in
  `Sidebar.tsx`.

Implementation is test-driven: each phase opens with failing tests and closes
once they pass plus all suites are green.

## Current State Analysis

- `SsePayload::DocChanged` (`server/src/sse_hub.rs:8-15`) currently carries only
  `docType`, `path`, and optional `etag`. No action discriminator, no timestamp.
  `etag` is already `Option<String>` with
  `#[serde(skip_serializing_if = "Option::is_none")]` — omission on delete is
  already correct.
- The watcher (`server/src/watcher.rs:62-152`) captures`pre: Option<IndexEntry>`
  at `watcher.rs:68`, then after debounce/canonicalise/`should_suppress`/rescan,
  broadcasts either`payload_for_entry(...)` (`Some(post)` branch at
  `watcher.rs:137-141`) or a manually-constructed `DocChanged` (deletion branch
  at `watcher.rs:142-152`). There are two `hub.broadcast` call sites and no
  `notify::EventKind`inspection.
- `SseHub` (`server/src/sse_hub.rs:23-40`) is a thin wrapper around
  `tokio::sync::broadcast::Sender<SsePayload>` with capacity 256 (constructed at
  `server.rs:83`). No history/replay; late subscribers miss prior events.
- `chrono` is not in `server/Cargo.toml`. Existing timestamps use
  `std::time::SystemTime` directly (`activity.rs`, `server.rs:107-120`).
- The crate root has an unrelated `server/src/activity.rs` (HTTP-activity
  tracker, `Arc<AtomicI64>`) — name collisions must be avoided.
- API routes register in `server/src/api/mod.rs:22-42`. Each handler is a
  separate module file (`mod docs; mod events; ...`). The `axum::Router` `query`
  feature is enabled (`Cargo.toml:24`), so `Query<...>` extraction is available.
- Existing api-handler test pattern (`api/events.rs:35-90`): build a minimal
  `AppState` via `tempfile::tempdir()` + `Config { ... }` defaults, then
  `build_router(state).oneshot(Request)` from `tower::ServiceExt`.
- `DocEventsHandle` (`frontend/src/api/use-doc-events.ts:15-19`) exposes
  `setDragInProgress`, `connectionState`, `justReconnected` — no event-subscribe
  API. The single `options.onEvent` callback (`:22`) is already claimed by
  `useUnseenDocTypes` at `RootLayout.tsx:17-21`.
- The SSE `onmessage` handler (`use-doc-events.ts:152-167`) drops self-caused
  events at line 155 (`registry.has(event.etag)`) **before** invoking
  `onEventRef.current?.(event)` at line 156. The activity feed needs
  `subscribe(...)` to be invoked **before** that drop so the work item's "
  default include self-caused" semantic remains meaningful for future code paths
  that bypass `WriteCoordinator`.
- `DocEventsContext.Provider` is already mounted at `RootLayout.tsx:33` (the
  work item's framing treats this as 0053-pending, but it has shipped).
- `Sidebar.tsx` (current 178 lines) renders LIBRARY → VIEWS → META in that
  order. No Activity slot exists. Component prop type is
  `{ docTypes: DocType[] }` only — no `children`. The Activity section will be
  added after META.
- `frontend/src/api/format.ts` already has `formatMtime(ms, now)` with branches
  matching AC4 for `< 7d`, but flips to `<n>w ago` at 7d and`toLocaleDateString`
  at 30d. AC4 pins `<n>d ago` for all `≥ 86400s` — a sibling helper is the
  cleaner fit.
- `Glyph` (`frontend/src/components/Glyph/Glyph.tsx`) takes
  `GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>`. Activity rows must
  narrow `event.docType` via `isGlyphDocTypeKey` before rendering. In practice
  `templates` is virtual and the watcher never fires for it.
- Frontend test pattern is
  `vi.mock('../../api/use-doc-events', () => ({ useDocEventsContext: vi.fn() }))`
  (see `SseIndicator.test.tsx`, `Topbar.test.tsx`). The mocked handle is stubbed
  per test; injecting events to ActivityFeed requires the mock to expose a stub
  `subscribe` that captures the listener for the test to invoke.

## Desired End State

When this plan is complete:

- The server emits
  `SsePayload::DocChanged { action, docType, path, etag?, timestamp }` for every
  watcher event. `action` is `created` / `edited` / `deleted`; `etag` is omitted
  for deletes.
- `GET /api/activity?limit=N` returns
  `{ events: Array<{ action, docType, path, timestamp }> }` ordered
  newest-first, drawn from an in-memory ring buffer of capacity 50. The ring
  buffer evicts the oldest event on overflow.
- The frontend `ActivityFeed` component renders inside the Sidebar (new section
  after META) with:
  - a heading "Activity" + a `data-testid="activity-live-badge"` LIVE element
    shown iff `connectionState === 'open'`,
  - up to 5 rows seeded from `GET /api/activity?limit=5`, newest first,
  - new SSE events prepended to the top,
  - per-row Glyph (narrowed by `isGlyphDocTypeKey`), action label rendered
    verbatim, and a `<n>s/m/h/d ago` + filename line,
  - relative-times refreshed every 60 s by a single `setInterval`,
  - an empty state `data-testid="activity-empty"` with text "No recent activity"
    when the initial history is empty,
  - self-caused events included (the subscriber sees events before the
    self-cause drop in the dispatch path).

### Verification

- All Rust tests pass: `make test-server` (or
  `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml`).
- All frontend tests pass:
  `npm --prefix skills/visualisation/visualise/frontend test`.
- Frontend typecheck passes:
  `npm --prefix skills/visualisation/visualise/frontend run typecheck`.
- Frontend lint clean: project's standard lint command.
- All acceptance criteria AC1–AC13 in `meta/work/0055-sidebar-activity-feed.md`
  map to at least one automated test.
- Manual smoke: `/accelerator:visualise`, edit/create/delete a file under
  `meta/plans/`, see a row appear at the top of the Activity feed within ~1
  second with the correct action label.

### Key Discoveries (from research and review)

- `chrono` must be added to `server/Cargo.toml` — it is not currently a direct
  dependency (`server/Cargo.toml:23-50`). Use
  `chrono = { version = "0.4", default-features = false, features = ["std", "clock", "serde"] }`
  (minimal).
- `payload_for_entry` (`watcher.rs:155-168`) currently has no access to `pre`.
  Rather than inline at the call site (which would fatten the debounce coroutine
  arm), **evolve the helper's signature** to
  `payload_for_entry(entry: &IndexEntry, rel: String, pre: Option<&IndexEntry>, now: DateTime<Utc>) -> SsePayload`.
  Pre/post action mapping moves *into* the helper; the malformed branch stays
  where it is; the coroutine remains a thin orchestrator. Preserves the helper's
  single-purpose seam and opens a unit-test surface for the pure mapping.
- The ring buffer must be pushed **synchronously, immediately before**
  `hub.broadcast(...)`. Subscribing the buffer to the broadcast channel would
  silently drop events under load (`RecvError::Lagged` recovery in
  `api/events.rs:25-28`).
- `ActionKind` needs a nested `#[serde(rename_all = "lowercase")]` because the
  container `rename_all = "kebab-case"` only renames variant **names**, not
  values.
- `DocEventsHandle` extension must invoke subscribers **before** the line-155
  self-cause drop (currently `onEvent` consumers don't see self-caused events at
  all — AC11's "default include" intent requires this re-ordering for the new
  subscribe API only; we keep the line-155 drop intact for the dispatch path).
- No `setInterval` exists anywhere in the frontend today — ActivityFeed
  introduces the first.
- **Module placement**: `ActivityRingBuffer` + `ActivityEvent` live at the crate
  root in `server/src/activity_feed.rs` (the `_feed` suffix disambiguates from
  the existing `crate::activity` HTTP-tracker). The HTTP handler lives in
  `server/src/api/activity.rs`. This preserves the convention that `api/`depends
  on domain modules, not the other way round.
- **`AppState` field naming**: the existing
  `activity: Arc<crate::activity::Activity>` field (HTTP-idle tracker) is
  renamed to `http_activity` to disambiguate from the new
  `activity_feed: Arc<crate::activity_feed::ActivityRingBuffer>` field. Cheap
  rename; permanent clarity dividend.
- **Watcher dual-sink consolidation**: a single emit point owns the fan-out.
  Introduce an inherent method
  `ActivityEvent::from_payload(&SsePayload) -> Option<ActivityEvent>` (returns
  `Some` for `DocChanged`, `None` for `DocInvalid`) on the local type so the
  filter semantic is visible at the call site, and a small helper
  `emit(payload: SsePayload, hub: &SseHub, activity: &ActivityRingBuffer)` that
  pushes any derived `ActivityEvent` before broadcasting. **Placement**: `emit`
  lives in `server/src/watcher.rs` (its sole caller today) rather than
  `sse_hub.rs` — this keeps the SSE hub a pure broadcast primitive and avoids
  the hub depending on the activity-feed module. The watcher constructs one
  payload per branch and the SSE/ring-buffer projection becomes a single,
  testable transformation.
- **Wire-format migration audit**: the plan's Phase 1 changes affect both test
  fixtures and a non-test production call site (`server/src/api/docs.rs:242` —
  the PATCH handler constructs `SsePayload::DocChanged`). The frontend TS
  interface change has a wider blast radius (~23 fixture sites across
  `use-doc-events.test.ts`, `LibraryDocView.smoke.test.tsx`,
  `use-unseen-doc-types.test.ts`). Both are enumerated below.
- **Subscribe/onEvent fan-out divergence is deliberate and scoped**:
  `subscribe(listener)` and the existing `onEvent` callback coexist with
  different ordering semantics. The divergence is documented on the
  `DocEventsHandle` interface (TSDoc block + Key Discoveries note in this plan),
  not implicit. Unifying the two surfaces into one`subscribe(listener, options)`
  API is a larger refactor that would also migrate `useUnseenDocTypes`; that is
  intentionally **not** in scope for 0055 (see "What We're NOT Doing").

## What We're NOT Doing

- Persisting the ring buffer across server restarts (clean restart → empty
  state, documented as acceptable).
- Adding `action` or `timestamp` to `DocInvalid` (left unchanged; consumers of
  the wire format will see asymmetric envelopes; documented).
- Adding a `'moved'` action variant — renames continue to surface as
  create+delete pairs (no change to current watcher semantics).
- **Surfacing `DocInvalid` events in the Activity feed.** Edits that produce a
  malformed-frontmatter state emit `DocInvalid` (no `action` field), and the
  feed filters those out via its `event.type !== 'doc-changed'` guard. Result:
  edits that break frontmatter do **not** appear in the Activity feed. AC8's "
  edited" wording does not carve out malformed-edit; this asymmetry is a
  deliberate trade-off documented for any consumer who expects every disk edit
  to surface here.
- **Unifying `subscribe(listener)` and `onEvent` into one fan-out API.** The two
  coexist with different ordering semantics (subscribers fire
  pre-self-cause-drop; `onEvent` fires post-drop) and different cardinality
  (multi vs single). A unified API of the shape
  `subscribe(listener, { includeSelfCaused? })` would also require migrating
  `useUnseenDocTypes`, which is out of scope for 0055. The divergence is
  documented on the `DocEventsHandle` interface (see Phase 5) so the contract is
  visible at the type level rather than buried in the onmessage handler.
- Pagination / "show more" beyond the initial 5 rows. The feed is a rolling-five
  view; any future paginated history is a follow-on story.
- Click-to-navigate on rows (open question in the work item; deferred).
- Excluding self-caused events from the feed (default-include behaviour is
  final; no toggle).
- Server-originated edits onto the live SSE stream from inside`WriteCoordinator`
  (a hypothetical bypass channel — not in scope).
- Adding `'activity'` to `SESSION_STABLE_QUERY_ROOTS` — on reconnect the feed
  correctly refetches (with deduplication, see Phase 6).

## Implementation Approach

Seven phases, TDD throughout:

1. **Server: SSE wire-format extension** — atomic change to
   `SsePayload::DocChanged` (`action` + `timestamp` fields), `payload_for_entry`
   helper evolves to take `pre` + `now`, watcher captures `Utc::now()` once per
   debounce, `api/docs.rs:242` production call site migrates. All `sse_hub` and
   `watcher` tests pinned to the new wire format (including the new
   create→edit→delete chain test); a grep checklist pins the migration audit.
2. **Server: `activity_feed` module at crate root** — `ActivityRingBuffer`
   (`std::sync::Mutex<VecDeque<…>>`, `const CAPACITY: usize = 50`),
   `From<&SsePayload> for Option<ActivityEvent>` projection, concurrent-push
   test. Module lives at `server/src/activity_feed.rs` (crate root) — the HTTP
   handler lives separately in `api/activity.rs` so transport depends on domain
   rather than the reverse.
3. **Server: `/api/activity` endpoint + AppState wiring + `emit` helper** —
   handler caps `?limit` at `CAPACITY`, AppState gains `activity_feed` and
   renames the existing `activity` field to `http_activity`, a single
   `emit(payload, hub, activity_feed)` helper owns the dual-sink fan-out.
   Watcher integration tests cover both the create-branch and the delete-branch
   ring-buffer pushes; the restart-empty-state contract is automated, not just
   manual.
4. **Frontend: foundational helpers** — `SseDocChangedEvent` extension (with
   the ~23-site TS fixture migration enumerated), `fetchActivity`,
   `queryKeys.activity`, and `formatRelative` (sharing a private
   `formatElapsedShort` ladder with `formatMtime` so future drift is
   impossible).
5. **Frontend: `DocEventsHandle.subscribe` extension** — multi-subscriber `Set`,
   invoked before line-155 drop with `console.error` on listener throws; the
   divergence from `onEvent` is documented in a TSDoc block on the interface so
   the contract is visible at the type level.
6. **Frontend: `ActivityFeed` component** — `useQuery` initial history,
   prepend-on-SSE via `subscribe`, dedup-on-reconnect, defensive coercion for
   cross-version dev drift, stable React key without index, LIVE badge, 60s
   ticker, empty state, loading state, AC3 render-count-exact test, AC11
   cross-referenced to the hook layer.
7. **Frontend: Sidebar slot + mount** — render `<ActivityFeed />` directly after
   META; the component owns its own `<section>` and no double-wrapping (avoids
   invalid ARIA).

Phases 1–3 are server-only and can land as one PR; phases 4–7 are frontend-only
and can land as a second PR. Within each phase, tests are written first and
fail, then implementation makes them pass.

---

## Phase 1: Server — SSE wire-format extension

### Overview

Extend `SsePayload::DocChanged` with `action: ActionKind` and
`timestamp: chrono::DateTime<Utc>`, evolve `payload_for_entry` to take `pre` and
`now` (preserving the pure-function seam rather than inlining), and migrate
every constructor site — including the **production** PATCH-handler call site at
`server/src/api/docs.rs:242` that the original audit missed.

This is an atomic change at the crate level: the existing wire-format test
(`sse_payload_json_wire_format`), the five watcher tests, and the
`api/docs.rs:242` call site all break the moment the type changes. Test changes,
the helper-signature evolution, and the `api/docs.rs` migration must ship
together. A grep checklist (Section 4a) verifies no constructor site was missed
before the phase is declared complete.

### Changes Required

#### 1. Cargo dependencies

**File**: `skills/visualisation/visualise/server/Cargo.toml`
**Changes**: add `chrono` with `serde` feature.

```toml
[dependencies]
# ... existing entries ...
chrono = { version = "0.4", default-features = false, features = ["std", "clock", "serde"] }
```

#### 2. `ActionKind` enum + extended `DocChanged` variant

**File**: `skills/visualisation/visualise/server/src/sse_hub.rs`
**Changes**: add `ActionKind`; extend `DocChanged` with two new fields.

```rust
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::docs::DocTypeKey;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionKind {
    Created,
    Edited,
    Deleted,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SsePayload {
    DocChanged {
        action: ActionKind,
        #[serde(rename = "docType")]
        doc_type: DocTypeKey,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        etag: Option<String>,
        timestamp: DateTime<Utc>,
    },
    DocInvalid {
        #[serde(rename = "docType")]
        doc_type: DocTypeKey,
        path: String,
    },
}
```

#### 3. Update `sse_hub` tests

**File**: `skills/visualisation/visualise/server/src/sse_hub.rs` (tests module,
`:42-125`)
**Changes**:

- Update `make_event` to set `action: ActionKind::Edited` and
  `timestamp: Utc::now()`.
- Update inline event constructions in `slow_consumer_gets_lagged_error`
  (`:84-89`) similarly.
- Extend `sse_payload_json_wire_format` to assert:
  - JSON contains `"action":"edited"` for the existing changed event, and
    `"action":"deleted"` for the deletion case.
  - JSON contains a `"timestamp"` key whose value is a string (the ISO-8601
    form; exact value not pinned beyond shape).
  - Deletion case omits `etag` (existing assertion preserved).

```rust
#[test]
fn sse_payload_json_wire_format() {
    let ts = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 5, 13, 12, 0, 0).unwrap();
    let changed = SsePayload::DocChanged {
        action: ActionKind::Edited,
        doc_type: crate::docs::DocTypeKey::Plans,
        path: "meta/plans/foo.md".into(),
        etag: Some("sha256-abc".into()),
        timestamp: ts,
    };
    let json = serde_json::to_string(&changed).unwrap();
    assert!(json.contains("\"type\":\"doc-changed\""), "json: {json}");
    assert!(json.contains("\"action\":\"edited\""), "json: {json}");
    assert!(json.contains("\"docType\":"), "json: {json}");
    assert!(json.contains("\"etag\":\"sha256-abc\""), "json: {json}");
    assert!(json.contains("\"timestamp\":\"2026-05-13T12:00:00Z\""), "json: {json}");

    let deleted = SsePayload::DocChanged {
        action: ActionKind::Deleted,
        doc_type: crate::docs::DocTypeKey::Plans,
        path: "meta/plans/foo.md".into(),
        etag: None,
        timestamp: ts,
    };
    let json = serde_json::to_string(&deleted).unwrap();
    assert!(json.contains("\"action\":\"deleted\""), "json: {json}");
    assert!(!json.contains("etag"), "etag must be absent for deletions: {json}");
}
```

#### 4. Watcher: action mapping + `Utc::now()` capture; evolve

`payload_for_entry` signature

**File**: `skills/visualisation/visualise/server/src/watcher.rs`
**Changes**: evolve `payload_for_entry` to accept `pre` and `now`, moving the
create/edit/delete decision into the helper rather than inlining at the call
site. This preserves the pure-function seam (unit-testable mapping) and keeps
the debounce coroutine a thin orchestrator. Capture `now = Utc::now()` once
between rescan and the match (so both branches share the same instant). The`now`
is captured close to broadcast — the only operation between `Utc::now()`and
`hub.broadcast(...)` is the `indexer.get(&path).await` lookup (a `HashMap`-style
read) and a small synchronous decision; under typical load drift is
sub-millisecond, well inside AC7's 1-second tolerance.

Helper signature (in `watcher.rs`, replacing the existing `payload_for_entry`):

```rust
use chrono::{DateTime, Utc};
use crate::sse_hub::{ActionKind, SsePayload};

/// Map an `IndexEntry` (post-rescan) plus optional `pre` (pre-rescan) plus a
/// captured `now` into the SSE wire-format envelope to broadcast.
///
/// - Malformed entries produce `DocInvalid` (no action; not surfaced in the
///   Activity feed — see Migration Notes).
/// - `pre.is_some()` → `Edited`; `pre.is_none()` → `Created`.
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
        let action = if pre.is_some() { ActionKind::Edited } else { ActionKind::Created };
        SsePayload::DocChanged {
            action,
            doc_type: entry.r#type,
            path: rel,
            etag: Some(entry.etag.clone()),
            timestamp: now,
        }
    }
}
```

Call-site update (in `on_path_changed_debounced`):

```rust
// ... existing logic up to `*clusters.write().await = new_clusters;` ...

let now = Utc::now();

match indexer.get( & path).await {
Some(entry) => {
let payload = payload_for_entry( & entry, rel.clone(), pre.as_ref(), now);
emit(payload, hub.as_ref(), activity_feed.as_ref());
tracing::debug ! (file = % path.display(), "SSE event broadcast");
}
None => {
if let Some(pre_entry) = pre {
let payload = SsePayload::DocChanged {
action: ActionKind::Deleted,
doc_type: pre_entry.r#type,
path: rel,
etag: None,
timestamp: now,
};
emit(payload, hub.as_ref(), activity_feed.as_ref());
tracing::debug ! (file = % path.display(), "SSE doc-changed broadcast for deleted file");
}
}
}
```

The `emit` helper consolidates ring-buffer push + broadcast — defined in Phase
3, Section 3. It uses `impl From<&SsePayload> for Option<ActivityEvent>` (also
defined in Phase 3) so the projection between wire formats lives in one place.

Note: `DocInvalid` deliberately unchanged. The existing
`malformed_frontmatter_produces_doc_invalid_event` test continues to pass as-is.

#### 4a. Production call site: `api/docs.rs:242`

**File**: `skills/visualisation/visualise/server/src/api/docs.rs:242`
**Changes**: the PATCH handler currently constructs
`SsePayload::DocChanged { doc_type, path, etag }` after a write. After the
variant gains required `action` and `timestamp` fields, this call site must be
updated:

```rust
// At server/src/api/docs.rs:242 (PATCH handler):
state.sse_hub.broadcast(SsePayload::DocChanged {
action: ActionKind::Edited,            // PATCH always edits existing docs
doc_type,
path: rel.clone(),
etag: Some(new_etag.clone()),
timestamp: chrono::Utc::now(),
});
```

The PATCH handler always targets an existing document (the route is
`PATCH /api/docs/...` against a known path), so `action: Edited` is the correct
choice. This broadcast is suppressed downstream by `WriteCoordinator` self-write
suppression — the wire-format must still be correct for the type checker and for
any future code path that bypasses `WriteCoordinator`.

**Audit checklist** (run before declaring Phase 1 complete):

```bash
# Must return only the call sites this phase touches:
grep -rn 'SsePayload::DocChanged' skills/visualisation/visualise/server/src/
# Expected matches: sse_hub.rs (definition + tests), watcher.rs (broadcast + tests),
# api/docs.rs (line 242), api/events.rs (test), api/activity.rs (test).
```

#### 5. Update four watcher tests + add chain test

**File**: `skills/visualisation/visualise/server/src/watcher.rs` (tests at
`:236-447`)
**Changes**: in each test, after the existing
`matches!(event, SsePayload::DocChanged { .. })` assertion, pattern-match the
`action` field and `timestamp` proximity. Pin the create-and-index-then-modify
ordering in the edit tests so `pre.is_some()` is guaranteed at modify-time.

- `file_change_produces_doc_changed_event` (`:236`): assert
  `action == ActionKind::Edited` (pre exists, post exists). **Pin ordering**:the
  test must `rx.recv()` the initial `Created` event after the first write
  completes (draining it from the channel) before triggering the second write.
  Without this, the test could observe a stale-`pre` state where `pre.is_none()`
  at modify-time and the watcher emits `Created` instead of `Edited`. Add an
  explicit comment in the test that documents this dependency.
- `rapid_writes_coalesce_to_one_event` (`:277`): assert
  `action == ActionKind::Edited`. Same `pre.is_some()` invariant — drain the
  initial event before the rapid-write burst.
- `new_file_in_watched_dir_produces_doc_changed_event` (`:368`): assert
  `action == ActionKind::Created`.
- `file_deletion_produces_doc_changed_without_etag` (`:406`): assert
  `action == ActionKind::Deleted`; existing `!json.contains("etag")` assertion
  preserved.

**AC6 timestamp tolerance**: in
`new_file_in_watched_dir_produces_doc_changed_event`, capture
`let before = Utc::now()` before triggering the write and
`let after = Utc::now()` after `rx.recv()`. Pattern-match `timestamp` out of the
event and assert **both** the strict containment `before <= timestamp <= after`
(the tighter property we control) **and** the AC6 wording
`(after - timestamp).num_seconds().abs() < 1` (the AC's 1-second tolerance
pinned verbatim, so a future drift in the capture point cannot silently violate
AC6 while the strict-containment assertion still passes).

Example test extension:

```rust
let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
.await.expect("timed out").expect("channel closed");

match event {
SsePayload::DocChanged { action, timestamp, ..} => {
assert_eq ! (action, ActionKind::Created);
// Tighter-than-AC bound — we control the test clock:
assert ! (timestamp > = before & & timestamp < = after,
"timestamp {timestamp} not within [{before}, {after}]");
// AC6 verbatim — 1-second tolerance against broadcast wall-clock.
// Use num_milliseconds() to avoid the truncation-toward-zero
// sensitivity of num_seconds() (e.g. a 1001ms diff truncates to 1
// and would still pass `< 1` if we used num_seconds — using ms
// expresses the contract precisely).
assert ! ((after - timestamp).num_milliseconds().abs() < 1_000,
"AC6: timestamp {timestamp} not within 1s of broadcast time {after}");
}
other => panic!("expected DocChanged, got {other:?}"),
}
```

`malformed_frontmatter_produces_doc_invalid_event` (`:327`) unchanged (no
`action` on `DocInvalid`).

#### 5a. New test: create→edit→delete chain on a single path

**File**: `skills/visualisation/visualise/server/src/watcher.rs` (new test in
the `tests` module)
**Changes**: add an integration-style test that exercises three successive
events on the same path, asserting the `pre`/`post` capture is correctly
maintained across debounce windows and that all three actions surface in their
expected order:

```rust
#[tokio::test]
async fn create_edit_delete_chain_emits_three_distinct_events() {
    // Setup: spawn watcher, create file, await Created event.
    // Modify file, await Edited event.
    // Delete file, await Deleted event.
    // Assert the three events have actions [Created, Edited, Deleted] in that order
    // and each carries a distinct timestamp (later events are strictly greater).
}
```

This test catches regressions in:

- the `pre` field being correctly populated for the second event (else it would
  be `Created` again),
- the `pre` field surviving the rescan-to-broadcast cycle for the third event,
- the ring buffer (once Phase 3 lands) recording all three.

### Success Criteria

#### Automated Verification:

- [ ] 
  `cargo build --manifest-path skills/visualisation/visualise/server/Cargo.toml`
  succeeds (including the `api/docs.rs:242` migration — see Section 4a).
- [ ] 
  `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml sse_hub`
  passes (including the extended `sse_payload_json_wire_format`).
- [ ] 
  `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml watcher`
  passes (all five existing watcher tests with new `action` and `timestamp`
  assertions, plus the new`create_edit_delete_chain_emits_three_distinct_events`
  test).
- [ ] 
  `cargo clippy --manifest-path skills/visualisation/visualise/server/Cargo.toml -- -D warnings`
  clean.
- [ ] Audit checklist (Section 4a):
  `grep -rn 'SsePayload::DocChanged' skills/visualisation/visualise/server/src/`
  returns only the call sites enumerated in this phase.

#### Manual Verification:

- [ ] Start the visualiser, open `/api/events` in a browser/curl; modify a file
  under `meta/plans/`; observe a `doc-changed` SSE frame carrying `action` and
  `timestamp` fields with the expected values.

---

## Phase 2: Server — `ActivityRingBuffer` module (crate root)

### Overview

Add the in-memory ring buffer that records the last N events. The ring buffer is
a **domain module** at the crate root (`server/src/activity_feed.rs`) — it's a
producer/consumer pair for file-change events, not an HTTP concern. The HTTP
handler that exposes it lives in `server/src/api/activity.rs` in Phase 3. This
preserves the convention that `api/` depends on domain modules, not the other
way round.

The `_feed` suffix disambiguates from the existing `crate::activity` (HTTP-idle
tracker) — the field-level rename `AppState.activity → http_activity` lands in
Phase 3.

### Changes Required

#### 1. New module `activity_feed.rs` (ring buffer at crate root)

**File**: `skills/visualisation/visualise/server/src/activity_feed.rs` (new)
**Changes**: define the entry type, the buffer, the `From<&SsePayload>`
projection, and unit tests. Use `std::sync::Mutex` (matches `WriteCoordinator`'s
pattern; critical sections are O (1) and never `await`).

**Mutex discipline (module invariant)**: the `std::sync::Mutex` is acceptable
inside the async watcher coroutine and HTTP handler **only** because critical
sections are O (1) and never await. The push site holds the lock for`pop_back` +
`push_front` over a `VecDeque<ActivityEvent>` — no allocation that can fail, no
I/O, no `.await`. The read site (`recent`) clones up to 50 small structs under
the lock; this is acceptable at the expected GET-rate (single UI client,
refetched on reconnect only). Any future change that extends operations under
the lock must be reviewed against this invariant.

**Poisoning policy**: a poisoned `Mutex` only occurs after a panic
mid-critical-section; for the current `VecDeque<ActivityEvent>` operations this
is effectively impossible. The activity feed is non-essential — propagating a
panic into the watcher or HTTP handler would be wildly disproportionate. The
buffer therefore recovers gracefully via
`unwrap_or_else(|poisoned| poisoned.into_inner())`.

```rust
use std::sync::Mutex;
use std::collections::VecDeque;

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
    /// returns `None`. Prefer this inherent method over a `From` impl: the
    /// filter semantic (some inputs project to `None`) is visible at the
    /// call site rather than hidden inside a `From::from` invocation.
    pub fn from_payload(payload: &SsePayload) -> Option<Self> {
        match payload {
            SsePayload::DocChanged { action, doc_type, path, timestamp, .. } => {
                Some(ActivityEvent {
                    action: *action,
                    doc_type: *doc_type,
                    path: path.clone(),
                    timestamp: *timestamp,
                })
            }
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
        Self { inner: Mutex::new(VecDeque::with_capacity(CAPACITY)) }
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
    fn default() -> Self { Self::new() }
}
```

#### 2. Register module at crate root

**File**: `skills/visualisation/visualise/server/src/lib.rs` (or `main.rs` —
whichever declares the crate's module list)
**Changes**: add `pub mod activity_feed;` alongside the other module
declarations.

```rust
// in server/src/lib.rs (or main.rs):
pub mod activity_feed;
// ... existing modules ...
```

The HTTP handler module (`api/activity.rs`) lands in Phase 3, Section 1.

#### 3. Unit tests (write FIRST; implementation makes them pass)

**File**: `skills/visualisation/visualise/server/src/activity_feed.rs` (tests
module)
**Changes**: cover capacity, ordering, eviction, the `From<&SsePayload>`
projection, and concurrent pushes from multiple threads.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::collections::HashSet;
    use chrono::{TimeZone, Utc};
    use crate::docs::DocTypeKey;
    use crate::sse_hub::{ActionKind, SsePayload};

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
        for i in 1..=6_i64 { buf.push(ev(i)); }
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
        for i in 1..=(CAPACITY as i64) { buf.push(ev(i)); }
        let out = buf.recent(CAPACITY);
        assert_eq!(out.len(), CAPACITY);
    }

    #[test]
    fn over_capacity_evicts_oldest() {
        let buf = ActivityRingBuffer::new();
        for i in 1..=(CAPACITY as i64 + 1) { buf.push(ev(i)); }
        let out = buf.recent(CAPACITY);
        assert_eq!(out.len(), CAPACITY);
        assert_eq!(out[0].timestamp.timestamp(), (CAPACITY as i64) + 1);
        assert_eq!(out.last().unwrap().timestamp.timestamp(), 2);
    }

    #[test]
    fn limit_truncates_response() {
        let buf = ActivityRingBuffer::new();
        for i in 1..=10_i64 { buf.push(ev(i)); }
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
        let projected = ActivityEvent::from_payload(&payload)
            .expect("DocChanged must project");
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
        // The ring buffer is hit concurrently from the watcher task and the
        // HTTP handler in production. This test fires N pushes from M threads
        // and asserts every event survives (CAPACITY is large enough to hold
        // them all) and timestamps are unique.
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
        for h in handles { h.join().unwrap(); }
        let out = buf.recent(CAPACITY);
        assert_eq!(out.len() as i64, THREADS * PER_THREAD);
        let unique: HashSet<i64> = out.iter().map(|e| e.timestamp.timestamp()).collect();
        assert_eq!(unique.len() as i64, THREADS * PER_THREAD, "every push must survive");
    }
}
```

(The capacity/ordering/eviction/limit tests directly cover AC10, AC12, AC13 once
the endpoint exists in Phase 3. The projection tests pin the `From<&SsePayload>`
contract that Phase 1's `emit` helper and Phase 3's watcher integration rely on.
The concurrent-push test validates the Mutex discipline under realistic
multi-producer load.)

### Success Criteria

#### Automated Verification:

- [ ] 
  `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml activity_feed`
  passes (capacity, ordering, eviction, limit, projection, concurrent-push
  tests).
- [ ] 
  `cargo clippy --manifest-path skills/visualisation/visualise/server/Cargo.toml -- -D warnings`
  clean.

#### Manual Verification:

- None (pure data structure).

---

## Phase 3: Server — `/api/activity` endpoint + AppState wiring + watcher push

### Overview

Thread an `Arc<ActivityRingBuffer>` through `AppState`, introduce a single`emit`
helper that owns the fan-out (ring-buffer push + broadcast), and register
`GET /api/activity?limit=N`. Rename the existing `AppState.activity` field to
`http_activity` to disambiguate from the new `activity_feed` field.

### Changes Required

#### 1. Handler

**File**: `skills/visualisation/visualise/server/src/api/activity.rs` (new)
**Changes**: HTTP handler only — the data structure lives in
`crate::activity_feed`. Cap `limit` at `CAPACITY` so a malformed or oversized
query parameter cannot trigger a larger-than-buffer clone.

```rust
use std::sync::Arc;
use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};

use crate::activity_feed::{ActivityEvent, CAPACITY};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub(crate) struct LimitParam {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ActivityResponse {
    events: Vec<ActivityEvent>,
}

/// `GET /api/activity?limit=N` — returns up to `min(N, CAPACITY)` recent
/// file-change events, newest-first. Default `limit` is `CAPACITY` (50).
///
/// **Contract** (documented for any non-UI consumer, e.g. CLI debug tools):
///
/// - `?limit=N` with `0 <= N <= CAPACITY` → returns up to N events.
/// - `?limit=N` with `N > CAPACITY` → silently clamped at `CAPACITY`. The
///   response does NOT signal the clamp (no `X-Total-Count` header, no
///   `limit` field in the response envelope). Clients reasoning about
///   response size must not assume `events.len() == requested limit`.
/// - `?limit=foo` (non-numeric) or `?limit=-1` → Axum's `Query` extractor
///   returns 400.
/// - No `?limit` → default `CAPACITY`.
pub(crate) async fn activity(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LimitParam>,
) -> Json<ActivityResponse> {
    let limit = params.limit.unwrap_or(CAPACITY).min(CAPACITY);
    let events = state.activity_feed.recent(limit);
    Json(ActivityResponse { events })
}
```

#### 2. `AppState` field + construction (with `activity` →

`http_activity` rename)

**File**: `skills/visualisation/visualise/server/src/server.rs`
**Changes**: rename the existing `activity: Arc<crate::activity::Activity>`field
to `http_activity` (the HTTP-idle tracker), and add `activity_feed`adjacent to
`SseHub`. Update every consumer of the old field name (request middleware that
reads the idle tracker, any test fixtures).

```rust
pub struct AppState {
    // ... existing fields ...
    pub sse_hub: Arc<crate::sse_hub::SseHub>,
    pub activity_feed: Arc<crate::activity_feed::ActivityRingBuffer>,
    pub http_activity: Arc<crate::activity::Activity>, // renamed from `activity`
    pub write_coordinator: Arc<crate::write_coordinator::WriteCoordinator>,
}
```

```rust
let sse_hub = Arc::new( crate::sse_hub::SseHub::new(256));
let activity_feed = Arc::new( crate::activity_feed::ActivityRingBuffer::new());
let http_activity = Arc::new( crate::activity::Activity::new());
let write_coordinator = Arc::new( crate::write_coordinator::WriteCoordinator::new());
Ok(Arc::new(Self {
// ... existing fields ...
sse_hub,
activity_feed,
http_activity,
write_coordinator,
}))
```

**Audit checklist**:
`grep -rn '\.activity\b' skills/visualisation/visualise/server/src/ skills/visualisation/visualise/server/tests/`
to find every reference to the old field name across both the source tree and
the integration-test crate. Update each to `.http_activity` (or `.activity_feed`
if the call site is intended for the new buffer — distinguishable by context).
Also rename `AppState::build`'s positional `activity: Arc<...>` parameter to
`http_activity` so the constructor signature matches the field.

#### 3. Single fan-out helper `emit` (consolidates ring-buffer push + broadcast)

**File**: `skills/visualisation/visualise/server/src/watcher.rs`
**Changes**: define a single fan-out point inside the watcher module — the
watcher is the sole caller, and placing the helper there keeps `sse_hub.rs` as a
pure broadcast primitive (no dependency on the activity-feed module). The
projection from `SsePayload` to `Option<ActivityEvent>` lives as the inherent
method `ActivityEvent::from_payload` (Phase 2, Section 1), so `emit` simply
applies that projection.

```rust
// in server/src/watcher.rs:
use crate::activity_feed::{ActivityEvent, ActivityRingBuffer};
use crate::sse_hub::{SseHub, SsePayload};

/// Single fan-out point for SSE events: pushes a derived `ActivityEvent`
/// into the ring buffer (if applicable — `DocInvalid` does not surface),
/// then broadcasts the payload. Ring-buffer push is **synchronous and
/// before** the broadcast, so a buffered event is always available via
/// `/api/activity` once a subscriber sees it on the live stream.
pub(crate) fn emit(payload: SsePayload, hub: &SseHub, activity: &ActivityRingBuffer) {
    if let Some(activity_event) = ActivityEvent::from_payload(&payload) {
        activity.push(activity_event);
    }
    hub.broadcast(payload);
}
```

The watcher's debounce coroutine (Phase 1, Section 4 call-site update) calls
`emit(payload, hub.as_ref(), activity_feed.as_ref())`. If a future producer of
`SsePayload` is added outside the watcher (e.g. a hypothetical bypass channel
from `WriteCoordinator`), promote `emit` to `pub(crate)` and call it from there
too — but for now keeping it module-private documents the watcher-is-sole-caller
invariant.

#### 3a. Thread ring buffer through `watcher::spawn`

**Files**: `skills/visualisation/visualise/server/src/watcher.rs`,
`server/src/server.rs:283-292`
**Changes**:

- Add `activity_feed: Arc<crate::activity_feed::ActivityRingBuffer>` argument to
  `watcher::spawn` (`watcher.rs:26-34`) and to `on_path_changed_debounced`
  (`watcher.rs:97-106`).
- The debounce coroutine no longer constructs `ActivityEvent` literals — it
  constructs a single `SsePayload` and calls `emit(...)` (Phase 1 Section 4
  already shows this call-site shape).
- Pass `state.activity_feed.clone()` at the call site in `server.rs:284-292`.

#### 4. Register route + module

**File**: `skills/visualisation/visualise/server/src/api/mod.rs:1-9`, `:22-42`
**Changes**: register the new handler module and add the route alongside
`/api/events`.

```rust
// at the top of api/mod.rs:
mod activity;
mod docs;
mod events;
// ...

pub fn mount(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/events", get(events::events))
        .route("/api/activity", get(activity::activity))
    // ... rest unchanged ...
}
```

#### 5. Tests (write FIRST)

**Handler tests** in `server/src/api/activity.rs` tests module (mirrors
`api/events.rs:35-90` setup):

- Build `minimal_state` (copy the helper from `events.rs`).
- **AC10**: Push 6 events with monotonically-increasing timestamps directly via
  `state.activity_feed.push(...)`. Issue `GET /api/activity?limit=5`. Assert
  status 200, content-type `application/json`, body deserialises,
  `events.len() == 5`, `events[0].timestamp` is the newest, ordering strictly
  newest-first.
- **AC13**: Push 51 events. Issue `GET /api/activity?limit=50`. Assert
  `events.len() == 50`, `events[0].timestamp == T51`,
  `events[49].timestamp == T2`.
- **Default limit**: Issue `GET /api/activity` (no `?limit`). Assert default
  behaviour returns up to `CAPACITY` events.
- **`?limit=0`**: Issue `GET /api/activity?limit=0`. Assert status 200,
  `events: []`. Documents the contract for zero-limit clients.
- **`?limit=foo` (invalid)**: Issue `GET /api/activity?limit=foo`. Assert status
  400 (Axum's `Query` extractor parse failure into `usize`). Documents the
  contract for malformed clients.
- **`?limit=999999` (oversized)**: Issue `GET /api/activity?limit=999999`against
  a buffer with 10 events. Assert `events.len() == 10` (the request is clamped
  at `CAPACITY=50` server-side but the underlying buffer only has 10 events, so
  10 is returned).
- **Restart → empty state** (automated, replaces the manual-only contract): with
  no pushes (a freshly-constructed `AppState`), issue
  `GET /api/activity?limit=5`. Assert `events: []`. Comment: this test pins the
  no-persistence contract documented in "What We're NOT Doing" — if a future
  change adds persistence, this test must change deliberately.

**Watcher integration tests** in `watcher.rs` tests module:

- **Create-branch push**: extend
  `new_file_in_watched_dir_produces_doc_changed_event` to pass an
  `Arc<ActivityRingBuffer>` into `spawn(...)` and assert
  `activity_feed.recent(1)` returns one entry whose `action == Created` and
  `path` matches the created file's relpath.
- **Delete-branch push** (new — was missing from the original plan): extend
  `file_deletion_produces_doc_changed_without_etag` (or add a sibling test) to
  also pass an `Arc<ActivityRingBuffer>` and assert `activity_feed.recent(1)`
  returns one entry whose `action == Deleted` and `path` matches the deleted
  file's relpath. This catches a regression in the deletion-branch push that the
  create-branch test would miss.
- **Chain test ring-buffer assertion**: extend the new
  `create_edit_delete_chain_emits_three_distinct_events` test (Phase 1, Section
  5a) to assert `activity_feed.recent(10)` contains three entries with actions
  `[Deleted, Edited, Created]` in newest-first order.

### Success Criteria

#### Automated Verification:

- [ ] 
  `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml api::activity`
  covers AC10, AC12, AC13, the `?limit=0`/`?limit=foo`/`?limit=999999`contracts,
  and the restart-empty-state contract.
- [ ] 
  `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml watcher::tests`
  (extended create-branch and delete-branch + chain test) passes.
- [ ] 
  `cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml` —
  full suite green.
- [ ] 
  `cargo clippy --manifest-path skills/visualisation/visualise/server/Cargo.toml -- -D warnings`
  clean.
- [ ] AppState rename audit:
  `grep -rn '\.activity\b' skills/visualisation/visualise/server/src/ skills/visualisation/visualise/server/tests/`
  returns zero unmigrated references (covers both the source tree and the
  integration-test crate).

#### Manual Verification:

- [ ] Start the visualiser; edit a file under `meta/plans/`;
  `curl http://127.0.0.1:<port>/api/activity?limit=5 | jq .` returns the event
  with the correct `action`, `docType`, `path`, `timestamp`.

---

## Phase 4: Frontend — foundational helpers (types, fetch, query-keys, format)

### Overview

Add the wire-format type extensions, the `fetchActivity` helper, the query-key,
and the `formatRelative` helper that matches AC4 exactly. All TDD'd in
`format.test.ts` and a new `fetch.test.ts` (if absent) or via direct unit-test
files.

### Changes Required

#### 1. Extend `SseDocChangedEvent` type

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts:113-118`
**Changes**:

```ts
export type ActionKind = 'created' | 'edited' | 'deleted'

export interface SseDocChangedEvent {
  type: 'doc-changed'
  action: ActionKind
  docType: DocTypeKey
  path: string
  etag?: string
  timestamp: string
}
```

Plus a new response shape and entry type for the activity endpoint:

```ts
export interface ActivityEvent {
  action: ActionKind
  docType: DocTypeKey
  path: string
  timestamp: string
}

export interface ActivityResponse {
  events: ActivityEvent[]
}
```

#### 1a. TS test-fixture migration audit (~23 sites)

**Decision**: keep `action` and `timestamp` **required** on the TS interface
(matches the Rust-side strictness; the wire format is genuinely producing these
fields). Every existing fixture site that constructs
`{ type: 'doc-changed', ... }` literals must be updated with sentinel values.
Run before this phase:

```bash
grep -rn '"type":\s*"doc-changed"\|type:\s*"doc-changed"\|type:\s*'\''doc-changed'\''' \
  skills/visualisation/visualise/frontend/src/
grep -rn 'SseDocChangedEvent' skills/visualisation/visualise/frontend/src/
```

**Expected fixture sites** (enumerated from grep — confirm before editing):

| File                                                                   | Approx. count | Sentinel migration                                                                                                                                                                                     |
|------------------------------------------------------------------------|---------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `frontend/src/api/use-doc-events.test.ts`                              | ~19           | Add `action: 'edited'` and `timestamp: '2026-05-13T00:00:00Z'` to every `doc-changed` literal; for tests where the action specifically matters (create/delete narratives), pick the appropriate value. |
| `frontend/src/components/LibraryDocView/LibraryDocView.smoke.test.tsx` | ~2            | Same sentinel pair.                                                                                                                                                                                    |
| `frontend/src/api/use-unseen-doc-types.test.ts`                        | ~1            | Same sentinel pair.                                                                                                                                                                                    |
| Any `JSON.stringify({type: 'doc-changed', ...})` sites                 | TBD via grep  | Add the two fields inside the literal before stringifying.                                                                                                                                             |

**Mitigation for cross-version-drift in dev mode**: when a new TS frontend is
rebuilt against an old server (only happens during a partial rebuild in `dev`),
the wire JSON may lack `action`/`timestamp`. The TS interface asserts they're
present; `event.action` would be `undefined` at runtime and the row would render
`undefined` verbatim. To prevent this confusing dev-mode failure (and to
diagnose any future server-side regression that emits empty strings), the
ActivityFeed component (Phase 6) defensively guards and **logs via`console.warn`
** so the drop is visible rather than silent:

```ts
// in ActivityFeed.tsx subscribe handler:
if (event.type !== 'doc-changed') return
if (typeof event.action !== 'string' || typeof event.timestamp !== 'string'
  || event.action === '' || event.timestamp === '') {
  console.warn(
    '[activity-feed] dropping doc-changed event with missing/empty action ' +
    'or timestamp — likely cross-version dev drift OR a server serialisation ' +
    'regression',
    event,
  )
  return
}
```

The `typeof` + empty-string check covers three distinct failure modes (missing
key, null/undefined value, empty string) with one branch and one diagnostic.
Aligns with the Phase 5 listener-error policy: silent swallowing here would mask
real bugs.

#### 2. `fetchActivity` helper

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.ts`
**Changes**: append a new function modelled on `fetchDocs` (`:63-68`).

```ts
import type { ActivityEvent, ActivityResponse } from './types'

export async function fetchActivity(limit: number): Promise<ActivityEvent[]> {
  const r = await fetch(`/api/activity?limit=${encodeURIComponent(String(limit))}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/activity?limit=${limit}: ${r.status}`)
  const body: ActivityResponse = await r.json()
  return body.events
}
```

#### 3. `queryKeys.activity`

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.ts:3-22`
**Changes**: add a parameterised entry.

```ts
activity: (limit: number) => [ 'activity', limit ] as const,
```

Do NOT add `'activity'` to `SESSION_STABLE_QUERY_ROOTS` (`:24-27`) — on
reconnect the feed should refetch.

#### 4. `formatRelative` helper (sharing the short-form ladder with

`formatMtime`)

**File**: `skills/visualisation/visualise/frontend/src/api/format.ts`
**Changes**: extract the shared `<n>s/m/h/d` ladder into a private helper that
both `formatMtime` and `formatRelative` call. The helper covers **only** the
in-range short-form ladder — the negative-elapsed clamp and the high-bound
fallback are each caller's responsibility, encoded at the call site rather than
inside the helper. This makes the divergent semantics (`formatMtime` clamps to
`'just now'` on negative elapsed and flips to `w`/`localeDateString` at 7d;
`formatRelative` clamps to `'0s ago'` on negative elapsed and keeps `d ago`
indefinitely) visible from the function bodies rather than buried in a shared
null-sentinel.

```ts
/**
 * Short-form elapsed-time ladder shared by `formatMtime` and `formatRelative`.
 * **Precondition**: caller must pass `diffSec >= 0`. Returns `null` only when
 * `diffSec >= 7 * 86400` (caller decides the long-form fallback). Otherwise
 * returns one of `<n>s ago` / `<n>m ago` / `<n>h ago` / `<n>d ago`.
 *
 * The negative-elapsed clamp is intentionally NOT in this helper — callers
 * have divergent semantics (`formatMtime` → 'just now', `formatRelative` →
 * '0s ago') and the divergence belongs at the call site.
 */
function formatElapsedShort(diffSec: number): string | null {
  if (diffSec < 60) return `${diffSec}s ago`
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`
  if (diffSec < 7 * 86400) return `${Math.floor(diffSec / 86400)}d ago`
  return null
}

// `formatMtime` is refactored to:
//   1. clamp negative elapsed to 'just now' (preserves existing contract);
//   2. delegate to `formatElapsedShort` for the in-range short-form ladder;
//   3. fall back to weeks / localeDateString when the helper returns null.
// The 'just now' branch and the weeks-fallback branch are unchanged from the
// existing `formatMtime` — only the s/m/h/d body is consolidated.
export function formatMtime(ms: number, now: number = Date.now()): string {
  const diffSec = Math.floor((now - ms) / 1000)
  if (diffSec < 0) return 'just now' // existing contract — preserved
  const short = formatElapsedShort(diffSec)
  if (short !== null) return short
  // existing weeks / localeDateString fallback — preserved verbatim
  // (refer to the current `formatMtime` body for the exact branch).
  return /* unchanged weeks-and-locale-date fallback */ formatMtimeWeeksOrDate(ms, now)
}

/**
 * Activity-feed-flavoured relative formatter. Diverges from `formatMtime`
 * at both ends: clamps negative elapsed to '0s ago' (vs 'just now') and
 * keeps `<n>d ago` indefinitely (vs weeks/locale flip at 7d).
 * Pinned to AC4 of work item 0055.
 */
export function formatRelative(ms: number, now: number = Date.now()): string {
  const diffSec = Math.floor((now - ms) / 1000)
  if (diffSec < 0) return '0s ago'
  return formatElapsedShort(diffSec)
    ?? `${Math.floor(diffSec / 86400)}d ago`
}
```

**Behavioural-equivalence verification**: Phase 4 Success Criteria adds an
explicit bullet
`existing formatMtime tests including the 'just now' negative-elapsed branch and the >=7d weeks/locale-date branch still pass without modification`.
The refactor must preserve every existing `formatMtime` test verbatim.

#### 5. Tests (write FIRST)

**File**: `skills/visualisation/visualise/frontend/src/api/format.test.ts`
(existing — extend) — add cases for `formatRelative`. Include explicit boundary
inputs to catch `<` vs `<=` mutations.

```ts
import { formatRelative } from './format'

describe('formatRelative', () => {
  const now = 10_000_000_000 // arbitrary fixed instant in ms
  it('renders seconds for elapsed < 60s', () => {
    expect(formatRelative(now - 30_000, now)).toBe('30s ago')
  })
  it('renders minutes for 60s <= elapsed < 3600s', () => {
    expect(formatRelative(now - 90_000, now)).toBe('1m ago')
    expect(formatRelative(now - 59_000, now)).toBe('59s ago')
  })
  it('renders hours for 3600s <= elapsed < 86400s', () => {
    expect(formatRelative(now - 3_700_000, now)).toBe('1h ago')
  })
  it('renders days for elapsed >= 86400s', () => {
    expect(formatRelative(now - 90_000_000, now)).toBe('1d ago')
    expect(formatRelative(now - 8 * 86_400_000, now)).toBe('8d ago')
  })
  it('clamps negative elapsed to 0s ago', () => {
    expect(formatRelative(now + 1000, now)).toBe('0s ago')
  })

  // Boundary cases — catch `<` vs `<=` mutations.
  it('renders boundary inputs precisely', () => {
    expect(formatRelative(now, now)).toBe('0s ago')           // diffSec = 0
    expect(formatRelative(now - 60_000, now)).toBe('1m ago')   // diffSec = 60 (minute boundary)
    expect(formatRelative(now - 3_600_000, now)).toBe('1h ago') // diffSec = 3600 (hour boundary)
    expect(formatRelative(now - 86_400_000, now)).toBe('1d ago') // diffSec = 86400 (day boundary)
  })
})
```

(Covers AC4 plus the four `<`/`<=` boundary mutations.)

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.test.ts`
(existing or new) — add `fetchActivity` cases using `vi.spyOn(global, 'fetch')`:

```ts
describe('fetchActivity', () => {
  it('GETs /api/activity?limit=N and returns events array', async () => {
    const body = {
      events: [ {
        action: 'created',
        docType: 'plans',
        path: 'a',
        timestamp: '2026-05-13T00:00:00Z'
      } ]
    }
    vi.spyOn(global, 'fetch').mockResolvedValue(new Response(JSON.stringify(body), { status: 200 }))
    const out = await fetchActivity(5)
    expect(out).toEqual(body.events)
    expect(global.fetch).toHaveBeenCalledWith('/api/activity?limit=5')
  })
  it('throws FetchError on non-2xx', async () => {
    vi.spyOn(global, 'fetch').mockResolvedValue(new Response('boom', { status: 500 }))
    await expect(fetchActivity(5)).rejects.toBeInstanceOf(FetchError)
  })
})
```

### Success Criteria

#### Automated Verification:

- [ ] `npm --prefix skills/visualisation/visualise/frontend run typecheck` clean
  (new types narrow correctly).
- [ ] 
  `npm --prefix skills/visualisation/visualise/frontend test -- format.test.ts`
  passes (AC4 + boundary cases).
- [ ] **Existing `formatMtime` tests including the `'just now'` negative-elapsed
  branch and the `>= 7d` weeks/localeDate branch pass without modification** —
  pins the behavioural-equivalence claim that the `formatElapsedShort`extraction
  makes (Section 4).
- [ ] 
  `npm --prefix skills/visualisation/visualise/frontend test -- fetch.test.ts`
  passes (when filtering, vitest CLI).
- [ ] Full frontend test suite remains green.

#### Manual Verification:

- None (helpers only).

---

## Phase 5: Frontend — `DocEventsHandle.subscribe` extension

### Overview

Extend `DocEventsHandle` with a `subscribe(listener) → unsubscribe` API, invoked
from inside the `onmessage` handler **before** the line-155 self-cause drop, so
future code paths can deliver self-caused events to subscribers without code
change. The existing dispatch path (`dispatchSseEvent` + the line-155
short-circuit) is preserved unchanged.

### Changes Required

#### 1. Extend the handle type and default

**File**:
`skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:15-19`,
`:182-186`
**Changes**: extend the handle with `subscribe`. The TSDoc block on
`DocEventsHandle` documents the deliberate divergence between `subscribe`
(pre-self-cause-drop, multi-consumer) and the existing `onEvent` callback path
(post-self-cause-drop, single-consumer) so future maintainers can read the
contract at the interface rather than reverse-engineering it from the`onmessage`
body.

```ts
/**
 * Handle returned by `useDocEvents` / `useDocEventsContext`.
 *
 * Two fan-out paths exist on this handle, with **different** ordering and
 * cardinality semantics — keep both contracts in mind when adding a new
 * consumer:
 *
 * - `subscribe(listener)` — multi-consumer; listeners fire **before** the
 *   self-cause registry check, so they observe ALL incoming events including
 *   events the application's own writes produced. The ActivityFeed
 *   (work item 0055, AC11) uses this path to surface self-caused activity.
 *
 * - `options.onEvent` (passed to `useDocEvents`) — single-consumer; fires
 *   **after** the self-cause drop, so it never sees self-caused events.
 *   `useUnseenDocTypes` is the only current consumer.
 *
 * Unifying these into one `subscribe(listener, { includeSelfCaused })` API
 * is intentionally out of scope for work item 0055 (would require migrating
 * `useUnseenDocTypes`). See "What We're NOT Doing" in the 0055 plan.
 */
export interface DocEventsHandle {
  setDragInProgress(v: boolean): void

  connectionState: ConnectionState
  justReconnected: boolean

  /**
   * Subscribe to incoming SSE events. The listener fires for every event,
   * **including self-caused ones** (this is the load-bearing difference
   * from `onEvent`; see the interface-level TSDoc). The callback should be
   * **cheap and non-blocking** — it runs synchronously inside the
   * EventSource onmessage handler, so a slow listener delays subsequent
   * listeners and the downstream dispatch.
   *
   * Returns an unsubscribe function. Safe to call multiple times.
   *
   * Naming note: if/when the unification refactor lands (see "What We're
   * NOT Doing" in the 0055 plan), this method will likely be renamed to
   * `subscribeRaw` and a sibling `subscribe(listener, { includeSelfCaused:
   * false })` will replace `onEvent`. Treat the include-self-caused
   * semantic as the load-bearing contract, not the method name.
   */
  subscribe(listener: (event: SseEvent) => void): () => void
}
```

```ts
const _defaultHandle: DocEventsHandle = {
  setDragInProgress: () => {
  },
  connectionState: 'connecting',
  justReconnected: false,
  subscribe: () => () => {
  },
}
```

#### 2. Wire subscribers into `makeUseDocEvents`

**File**:
`skills/visualisation/visualise/frontend/src/api/use-doc-events.ts:97-176`
**Changes**:

- Inside the hook body, create a stable
  `useRef<Set<(e: SseEvent) => void>>(new Set())` for listeners.
- Define
  `subscribe = useCallback((listener) => { listenersRef.current.add(listener); return () => { listenersRef.current.delete(listener) } }, [])`.
- In the `onmessage` handler at `:152-167`, parse the event, then **before** the
  line-155 self-cause check, iterate `listenersRef.current` and invoke each
  listener with the parsed event. Existing line-155 drop and downstream
  dispatcher logic stays unchanged for the query-invalidation path.
- Return `{ setDragInProgress, connectionState, justReconnected, subscribe }`.

```ts
const listenersRef = useRef(new Set<(e: SseEvent) => void>())

const subscribe = useCallback((listener: (e: SseEvent) => void) => {
  listenersRef.current.add(listener)
  return () => {
    listenersRef.current.delete(listener)
  }
}, [])

// inside onmessage, replacing lines 152-167:
reconnecting.onmessage = (e: MessageEvent) => {
  try {
    const event = JSON.parse(e.data as string) as SseEvent
    // Fan out to subscribers BEFORE the self-cause drop, so subscribers
    // observe events that the dispatcher chooses to ignore. The
    // ActivityFeed defaults to include-self-caused per work item 0055 AC11.
    // Listener errors are isolated (one bad listener does not break others)
    // but escalated to console.error so they surface in production
    // diagnostics — silent swallowing here would mask real bugs.
    for (const listener of listenersRef.current) {
      try {
        listener(event)
      } catch (err) {
        console.error('[doc-events] subscriber threw — listener faulted, others continue', err)
      }
    }
    if (event.type === 'doc-changed' && registry.has(event.etag)) return
    onEventRef.current?.(event)
    if (isDraggingRef.current) {
      for (const k of queryKeysForEvent(event)) {
        pendingRef.current.add(JSON.stringify(k))
      }
    } else {
      dispatchSseEvent(event, queryClient)
    }
  } catch (err) {
    console.warn('useDocEvents: failed to parse SSE message', {
      data: e.data,
      err
    })
  }
}

return { setDragInProgress, connectionState, justReconnected, subscribe }
```

#### 3. Tests (write FIRST)

**File**:
`skills/visualisation/visualise/frontend/src/api/use-doc-events.test.ts`
(existing — extend) or a new file alongside.

Tests to add (using the existing `makeUseDocEvents` factory + a fake EventSource
factory that exposes the `MessageEvent` injection seam — follow the established
harness in the existing `use-doc-events` tests):

1. **Subscribe + receive**: render a hook instance, subscribe a `vi.fn()`,
   dispatch a synthesized `MessageEvent` with a `doc-changed` payload, assert
   the listener was invoked with the parsed event.
2. **Unsubscribe stops delivery**: subscribe, immediately unsubscribe, dispatch
   event, assert listener was NOT called.
3. **Multiple subscribers all receive**: two `vi.fn()` listeners, one event,
   both called.
4. **Self-caused event still reaches subscribers** (the critical fix): register
   an etag in the self-cause registry, dispatch a `doc-changed` with that etag,
   assert listener was invoked exactly once (existing behaviour preserved:
   `onEvent` is NOT called and `dispatchSseEvent` is not called for this event,
   but the subscriber **is**).
5. **`_defaultHandle.subscribe` is a no-op** returning a no-op unsubscribe
   (sanity).

(Tests 1–4 collectively cover AC11 at the hook layer.)

### Success Criteria

#### Automated Verification:

- [ ] `npm --prefix skills/visualisation/visualise/frontend run typecheck` clean
  (existing `useDocEventsContext` consumers must compile — there are no
  required-method changes that break them since they all spread/destructure
  subsets).
- [ ] 
  `npm --prefix skills/visualisation/visualise/frontend test -- use-doc-events.test.ts`
  passes (new subscribe tests).
- [ ] Existing `useUnseenDocTypes` flow (via `onEvent`) still works — verify by
  running the full frontend suite.

#### Manual Verification:

- [ ] Visualiser runs; edit a file; the existing unseen-dot behaviour on the
  Sidebar still appears (no regression in `useUnseenDocTypes`).

---

## Phase 6: Frontend — `ActivityFeed` component

### Overview

Build the component itself. Initial state from
`useQuery({ queryKey: queryKeys.activity(5), queryFn: () => fetchActivity(5) })`;
live prepend via `useDocEventsContext().subscribe(...)`; LIVE badge from
`connectionState`; 60 s `setInterval` ticker; empty state; row rendering with
Glyph + verbatim action label + `formatRelative(...)` line.

### Changes Required

#### 1. Component file + CSS module

**Files**:

-

`skills/visualisation/visualise/frontend/src/components/ActivityFeed/ActivityFeed.tsx`
(new)

-

`skills/visualisation/visualise/frontend/src/components/ActivityFeed/ActivityFeed.module.css`
(new — minimal styles; design tokens follow existing Sidebar patterns)

```tsx
import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { fetchActivity } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { formatRelative } from '../../api/format'
import { useDocEventsContext } from '../../api/use-doc-events'
import type { ActivityEvent } from '../../api/types'
import { Glyph, isGlyphDocTypeKey } from '../Glyph/Glyph'
import styles from './ActivityFeed.module.css'

const LIMIT = 5

function basename(path: string): string {
  const ix = path.lastIndexOf('/')
  return ix >= 0 ? path.slice(ix + 1) : path
}

/**
 * Stable identifier for an activity row — used as both the React `key` prop
 * and the dedup key. Extracting it once locates the equality contract in
 * one place so future schema changes can't have one consumer merge what
 * the other reconciles.
 */
function activityRowId(event: ActivityEvent): string {
  return `${event.timestamp}|${event.path}|${event.action}`
}

/**
 * Dedupe by `activityRowId`, then sort newest-first by `timestamp`. After an
 * SSE reconnect the refetched `initial` history overlaps with events already
 * accumulated in `live`; without dedup we would render those events twice.
 * The sort handles the rare interleave case where a live event received
 * before `useQuery` resolves is older than the newest event in the refetched
 * history — without the sort, that live event would render above the newer
 * initial event.
 */
function dedupeAndSortRows(rows: ActivityEvent[]): ActivityEvent[] {
  const seen = new Set<string>()
  const out: ActivityEvent[] = []
  for (const r of rows) {
    const k = activityRowId(r)
    if (seen.has(k)) continue
    seen.add(k)
    out.push(r)
  }
  // RFC-3339 ISO strings sort lexicographically as time (chrono's output).
  out.sort((a, b) => b.timestamp.localeCompare(a.timestamp))
  return out
}

export function ActivityFeed() {
  const { data: initial, isSuccess } = useQuery({
    queryKey: queryKeys.activity(LIMIT),
    queryFn: () => fetchActivity(LIMIT),
  })
  const { connectionState, subscribe } = useDocEventsContext()
  const [ live, setLive ] = useState<ActivityEvent[]>([])
  // Ticker: force re-render every 60s for relative-time refresh.
  const [ , setTick ] = useState(0)
  useEffect(() => {
    const id = setInterval(() => setTick(t => t + 1), 60_000)
    return () => clearInterval(id)
  }, [])

  useEffect(() => {
    const unsub = subscribe(event => {
      // Discriminated-union narrowing on `event.type` yields
      // `SseDocChangedEvent` automatically — no `as` cast needed.
      if (event.type !== 'doc-changed') return
      // Defensive guard. Two separate failure modes, each diagnosed
      // distinctly so server-side regressions don't hide behind a
      // dev-mode-drift assumption:
      if (typeof event.action !== 'string' || typeof event.timestamp !== 'string'
        || event.action === '' || event.timestamp === '') {
        console.warn(
          '[activity-feed] dropping doc-changed event with missing/empty ' +
          'action or timestamp — likely cross-version dev drift OR a server ' +
          'serialisation regression',
          event,
        )
        return
      }
      setLive(prev => [
        {
          action: event.action,
          docType: event.docType,
          path: event.path,
          timestamp: event.timestamp
        },
        ...prev,
      ].slice(0, LIMIT))
    })
    return unsub
  }, [ subscribe ])

  // Note: `Date.parse` truncates chrono's RFC-3339 nanosecond precision to
  // milliseconds, which is irrelevant for s/m/h/d-granularity formatting.
  // The negative-elapsed clamp inside `formatRelative` handles small forward
  // clock skew between server and client.
  const rows = dedupeAndSortRows([ ...live, ...(initial ?? []) ]).slice(0, LIMIT)
  const isLive = connectionState === 'open'
  const isEmptyHistory = isSuccess && (initial?.length ?? 0) === 0 && live.length === 0

  return (
    <section aria-labelledby="activity-heading" className={styles.section}>
      <h2 id="activity-heading" className={styles.heading}>
        <span>Activity</span>
        {isLive && (
          <span data-testid="activity-live-badge"
                className={styles.liveBadge}>LIVE</span>
        )}
      </h2>
      {isEmptyHistory ? (
        <p data-testid="activity-empty" className={styles.empty}>No recent
          activity</p>
      ) : (
        <ul className={styles.list}>
          {rows.map(r => (
            // Stable key via the shared `activityRowId` helper — the same
            // identifier the dedup `Set` uses, so future schema changes
            // update both sites at once. No index in the key: array index
            // would shift on prepend and remount every row on each new
            // event, defeating reconciliation.
            <li key={activityRowId(r)} className={styles.row}>
              {isGlyphDocTypeKey(r.docType) &&
                <Glyph docType={r.docType} size={16}/>}
              <span className={styles.action}>{r.action}</span>
              <span className={styles.meta}>
                {formatRelative(Date.parse(r.timestamp))} · {basename(r.path)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
```

**Glyph silent-omission policy**: the
`isGlyphDocTypeKey(r.docType) && <Glyph ... />` guard silently omits the icon
for `DocTypeKey` variants without a Glyph variant (today only `templates`, which
the watcher never fires for). This is a deliberate trade-off — widening `Glyph`
to accept the full `DocTypeKey` with a fallback icon is deferred. If a future
virtual doc type is added, the row renders without an icon; document this in the
`ActivityFeed` component-level comment so it's not a surprise.

#### 2. Tests (write FIRST)

**File**:
`skills/visualisation/visualise/frontend/src/components/ActivityFeed/ActivityFeed.test.tsx`
(new)

Follow the test harness pattern from `SseIndicator.test.tsx` /`Topbar.test.tsx`:
`vi.mock('../../api/use-doc-events', ...)` exposing a`useDocEventsContext` mock
whose `subscribe` captures the listener. Mock`fetchActivity` via
`vi.mock('../../api/fetch', ...)` so the initial-history`useQuery` resolves to a
controllable value.

Use a `QueryClientProvider` test wrapper (mirrors any existing component test
that uses TanStack Query — search the repo if a helper already exists; otherwise
inline `new QueryClient({ defaultOptions: { queries: { retry: false } } })` per
test).

Tests to add:

1. **AC1 — prepend on SSE**: mock initial fetch → 1 existing event; mount; flush
   queries; capture `subscribe` listener; invoke listener with a `doc-changed`
   event. Assert the new event row is at the top (use
   `getAllByRole('listitem')[0]`), shows the action verbatim, and renders the
   Glyph + relative-time + basename.
2. **AC2 — LIVE badge presence**: `connectionState === 'open'` →
   `getByTestId('activity-live-badge')` present with text `LIVE`;
   `connectionState === 'closed'` → `queryByTestId('activity-live-badge')` is
   null.
3. **AC3 — 60 s ticker fires exactly once per 60s** (`vi.useFakeTimers()`):mount
   with 1 row whose `timestamp` is `now - 30s`. Use a **child-component render
   counter** (not `vi.spyOn` on the imported `formatRelative` — ES module
   bindings are live and `vi.spyOn` on the module-level reference will not
   redirect calls inside the component; only `vi.mock('../../api/format', ...)`
   would work, but the child-counter approach is cleaner and doesn't require
   remocking a tested helper). Concretely: extract the row body into a tiny
   `<ActivityRow>` subcomponent during Phase 6 §1 and have the test wrap it with
   `forwardRef`-style render counting (e.g. a
   `useEffect(() => { renderCount.current += 1 })` inside a test-only subclass,
   or `vi.fn()` as the component itself via React Testing Library's `render`with
   a counting wrapper). Advance 60_000 ms — assert renderCount incremented by
   exactly 1. Advance another 60_000 ms — exactly 2 from baseline. Pins AC3's "
   exactly one re-render per tick" wording, not just "the text changed".
4. **AC4** is covered by `format.test.ts`; here, smoke that `formatRelative` is
   applied via the row's rendered text matching `'30s ago'` / `'1m ago'` etc.
5. **AC5 — badge toggles with connection state**: re-render with
   `connectionState: 'open'` then with `connectionState: 'closed'`, assert badge
   appears then disappears.
6. **AC6 — initial-history fetch**: spy on the mocked `fetchActivity`, mount,
   assert it was called with `5`; render order matches the mocked response's
   order.
7. **AC7 — empty state**: mock `fetchActivity` to resolve `[]`; mount; assert
   `getByTestId('activity-empty')` text `No recent activity`; no `<li>` row
   elements present.
8. **AC11 — cross-reference, not duplicated**: AC11 (self-caused events reach
   the feed) is verified at the **hook layer** by Phase 5 test #4, where the
   line-155 self-cause registry is exercised end-to-end. The component-level
   test would only re-prove "any event reaches the row" (component never
   consults the registry) and adds no incremental AC11 coverage. **Action**: do
   NOT add a component-level AC11 test. Instead, add a comment in
   `ActivityFeed.test.tsx` near the AC1 case noting that AC11 is covered by
   `use-doc-events.test.ts`'s self-caused-subscriber test, so anyone reading the
   component suite knows where the AC11 evidence lives.
9. **Loading-state behaviour** (new — was missing): mount with a never-resolving
   `fetchActivity` mock (`vi.fn().mockReturnValue(new Promise(() => {}))`).
   Assert that neither `getByTestId('activity-empty')` nor any `<li>` row
   elements are present during the loading phase. Catches a future regression
   where the empty state flashes before the fetch resolves.
10. **Dedup on overlap (unit-level)**: mock initial fetch → 2 events `[A, B]`;
    mount; flush queries; invoke the captured subscribe listener with event `A`
    (same `activityRowId` as the first initial entry). Assert the rendered row
    list has 2 items (not 3) — the duplicate was deduped. Pins the dedup
    contract.
11. **Dedup on real refetch (integration-style)**: mount with
    `connectionState: 'open'` and an initial fetch resolving to `[A]`. Invoke
    the subscribe listener with two new events `[B, C]` (so `live = [C, B]`).
    Then
    `act(() => queryClient.invalidateQueries({ queryKey: queryKeys.activity(5) }))`
    and have the next `fetchActivity` resolve to `[C, B, A]` (the server's ring
    now contains the live events). Wait for the refetch to flush. Assert the
    rendered row list contains exactly `[C, B, A]` in that order — no
    duplicates, no out-of-order interleave. This exercises the actual production
    refetch path (vs the unit-level test which only synthesises duplication
    through the listener).
12. **Interleave sort**: mount with a never-resolving initial fetch; invoke the
    subscribe listener with one event `X` (timestamp `T0`); then resolve the
    initial fetch to `[Y]` (timestamp `T0 + 1s`, i.e. newer than `X`). After the
    fetch resolves, assert the rendered row order is `[Y, X]` — the
    `dedupeAndSortRows` sort step ensures the newer initial event appears above
    the stale live event, not below.

**Test harness — typed mock factory, no `as any`, no module-level state**:

```tsx
import type { SseEvent } from '../../api/types'
import type { DocEventsHandle } from '../../api/use-doc-events'

vi.mock('../../api/use-doc-events', () => ({
  useDocEventsContext: vi.fn(),
}))
vi.mock('../../api/fetch', () => ({
  fetchActivity: vi.fn(),
}))

interface MountResult {
  rendered: ReturnType<typeof render>
  getListener: () => ((e: SseEvent) => void) | undefined
}

/** Typed mock-handle factory — preserves DocEventsHandle compile-time checks. */
function mockHandle(overrides: Partial<DocEventsHandle> = {}): DocEventsHandle {
  return {
    setDragInProgress: vi.fn(),
    connectionState: 'open',
    justReconnected: false,
    subscribe: () => () => {
    },
    ...overrides,
  }
}

function mountWith({
                     initial,
                     connectionState = 'open' as const,
                   }: {
  initial: ActivityEvent[] | Promise<ActivityEvent[]>
  connectionState?: ConnectionState
}): MountResult {
  let captured: ((e: SseEvent) => void) | undefined
  vi.mocked(fetchActivity).mockReturnValue(
    initial instanceof Promise ? initial : Promise.resolve(initial),
  )
  vi.mocked(useDocEventsContext).mockReturnValue(
    mockHandle({
      connectionState,
      subscribe: (listener) => {
        captured = listener
        return () => {
        }
      },
    }),
  )
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const rendered = render(
    <QueryClientProvider client={qc}><ActivityFeed/></QueryClientProvider>,
  )
  // `captured` is local to mountWith — each test owns its own listener handle,
  // no module-level shared state, no beforeEach reset hazard.
  return { rendered, getListener: () => captured }
}
```

### Success Criteria

#### Automated Verification:

- [ ] 
  `npm --prefix skills/visualisation/visualise/frontend test -- ActivityFeed.test.tsx`
  passes — AC1, AC2, AC3 (render-count-exact), AC5, AC6, AC7 all covered at the
  component layer; AC11 cross-referenced to `use-doc-events.test.ts`; plus
  loading-state and dedup-on-reconnect cases.
- [ ] `npm --prefix skills/visualisation/visualise/frontend run typecheck` clean
  (no `as any` in the test harness).
- [ ] Full frontend suite green.

#### Manual Verification:

- [ ] Visualiser running; ActivityFeed visible on the page (use the
  `/glyph-showcase` route or a temporary mount if Sidebar slot is still pending
  in Phase 7 — otherwise wait for Phase 7). Editing a file produces a new top
  row within ~1 second.

---

## Phase 7: Frontend — Sidebar slot + mount

### Overview

Add a new `<section>` after META in `Sidebar.tsx` that renders
`<ActivityFeed />`. Add an integration test confirming the feed renders inside
the Sidebar.

### Changes Required

#### 1. Mount in `Sidebar.tsx`

**File**:
`skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`
**Changes**: after the `{templates && (<section>…META…</section>)}` block,
render `<ActivityFeed />` directly — no wrapping `<section>`. The component owns
its own `<section aria-labelledby="activity-heading">` (Phase 6);
double-wrapping would produce two landmark-style elements sharing one heading
id, which is invalid ARIA. The Sidebar's other sections (LIBRARY, VIEWS, META)
currently render their own `<section>` too, so this matches the established
rhythm.

```tsx
<ActivityFeed/>
```

Plus the import at the top:

```tsx
import { ActivityFeed } from '../ActivityFeed/ActivityFeed'
```

`ActivityFeed.module.css` (Phase 6) carries any Sidebar-style margins on its
`.section` class so the visual rhythm matches LIBRARY/VIEWS/META without Sidebar
needing to inject layout.

#### 2. Integration test

**File**:
`skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx`
(existing — extend) or `ActivityFeed.integration.test.tsx`.

Mock `useDocEventsContext`, `fetchActivity`, `useUnseenDocTypesContext`. Render
`<Sidebar docTypes={...} />` inside `QueryClientProvider`. Assert:

- The Activity heading and (when `connectionState === 'open'`)
  `data-testid="activity-live-badge"` are present.
- The ActivityFeed renders after the META section in DOM order (use
  `compareDocumentPosition` or query for both headings and assert relative
  position).
- The existing LIBRARY / VIEWS / META sections still render unchanged (no
  regression).

### Success Criteria

#### Automated Verification:

- [ ] `npm --prefix skills/visualisation/visualise/frontend test -- Sidebar`
  passes.
- [ ] `npm --prefix skills/visualisation/visualise/frontend run typecheck`clean.
- [ ] `npm --prefix skills/visualisation/visualise/frontend run build` succeeds.

#### Manual Verification:

- [ ] Visualiser running with at least one file in `meta/plans/`. Open the app;
  the Sidebar shows an Activity section after META with the LIVE badge while
  connected.
- [ ] Modify, create, and delete a markdown file under `meta/plans/`. Each
  action appears as a new top row with the correct action label and per-doc-type
  Glyph; relative-times refresh approximately every 60 s.
- [ ] Disconnect the server (`/accelerator:visualise stop`) — the LIVE badge
  disappears. Restart — `GET /api/activity?limit=5` returns `{events: []}`
  initially → empty state renders briefly, then live events repopulate.

---

## Testing Strategy

### Unit Tests

- **Rust**: `sse_hub` (wire-format), `api::activity` (ring buffer + handler),
  `watcher` (action + timestamp + ring-buffer push).
- **TS**: `format.test.ts` (`formatRelative`), `fetch.test.ts`(`fetchActivity`),
  `use-doc-events.test.ts`(subscribe/unsubscribe/multi-listener/self-cause),
  `ActivityFeed.test.tsx`(all component ACs).

### Integration Tests

- **Server**: handler test via `build_router(state).oneshot(...)` (existing
  pattern from `api/events.rs`).
- **Frontend**: Sidebar integration test verifying ActivityFeed mounts in the
  right slot.

### Manual Testing Steps

1. Start visualiser (`/accelerator:visualise`).
2. Open the URL in a browser; confirm Sidebar shows Activity section (heading +
   LIVE badge + empty state if no recent events).
3. `touch meta/plans/2026-05-13-test.md` — within ~1 s a `created` row appears
   at the top.
4. Edit the file — an `edited` row appears at the top, pushing earlier rows down
   (max 5 visible).
5. `rm meta/plans/2026-05-13-test.md` — a `deleted` row appears.
6. Wait 60 s — confirm a relative-time (`30s ago` → `1m ago`) updates.
7. Stop the server (`/accelerator:visualise stop`); confirm the LIVE badge
   disappears in the still-open browser tab.
8. Restart the server; on auto-reconnect the LIVE badge reappears and the feed
   refetches (empty initially).

## Performance Considerations

- Ring buffer push is O (1) under a `std::sync::Mutex` — held only across
  `pop_back` + `push_front`, never across an `await`.
- `setInterval(60_000)` is a single timer per `ActivityFeed` instance; only one
  instance is mounted (singleton via Sidebar).
- The subscribe set is invoked in the SSE `onmessage` path; with one subscriber
  (ActivityFeed), this adds one indirect call per event. Bounded set size in
  practice.
- The 50-entry ring buffer keeps a bounded amount of memory (50 small structs).

## Migration Notes

- No schema migrations.
- **JSON-level compatibility**: the wire-format change to
  `SsePayload::DocChanged` is **additive** at the JSON level. A new server
  emitting `action`/`timestamp` against an old frontend works (the existing TS
  dispatcher branches on `type` and ignores unknown fields). A new frontend
  against an old server is the reverse-drift case — the new TS interface marks
  the fields as required, so `event.action` would be `undefined` at runtime. The
  ActivityFeed defensively guards against this in dev (Phase 4 §1a). Production
  deploys ship server + frontend as a single unit so cross-version drift is a
  dev-only concern.
- **Rust migration audit** (must compile cleanly after Phase 1):
  `SsePayload::DocChanged` is constructed at the following call sites and every
  site adds `action` + `timestamp`:
  - `server/src/sse_hub.rs` — type definition + `sse_payload_json_wire_format`
    test
  - `server/src/watcher.rs` — production broadcast (now via the `emit` helper) +
    five existing tests + new chain test
  - `server/src/api/docs.rs:242` — **production** PATCH-handler broadcast (was
    missing from the original plan's audit; see Phase 1 §4a)
  - `server/src/api/events.rs` — existing test
  - `server/src/api/activity.rs` — new handler test
  - Phase 1 Success Criteria includes a grep checklist that pins the audit.
- **TypeScript migration audit** (must typecheck cleanly after Phase 4): the
  change to make `action`/`timestamp` required on `SseDocChangedEvent` breaks ~
  23 existing fixture sites across `use-doc-events.test.ts`,
  `LibraryDocView.smoke.test.tsx`, and `use-unseen-doc-types.test.ts`. Phase 4
  §1a enumerates the sites and pins sentinel values per fixture; the grep
  checklist pins the audit before declaring the phase complete.
- **`AppState.activity` field rename** (Phase 3 §2): the existing
  HTTP-idle-tracker field `activity` is renamed to `http_activity` to
  disambiguate from the new `activity_feed` field. Every consumer (request
  middleware, test fixtures, the `AppState::build` parameter name) must be
  updated. Phase 3 Success Criteria includes a grep checklist that pins the
  rename audit across both `server/src/` and `server/tests/`.
- **`GET /api/activity?limit=N` clamp contract** (Phase 3 §1): values of
  `N > CAPACITY` (50) are silently clamped at `CAPACITY`. The response does NOT
  signal the clamp (no `X-Total-Count`, no `limit` echo). Documented in the
  handler TSDoc so future tooling (CLI debug clients, scripts) does not assume
  `events.len() == requested N`.
- Clean restart → `GET /api/activity` returns `{ events: [] }` (no persistence;
  covered by an automated handler test in Phase 3, not just manual
  verification).

## References

- Work item: `meta/work/0055-sidebar-activity-feed.md`
- Research: `meta/research/codebase/2026-05-13-0055-sidebar-activity-feed.md`
- Parent epic: `meta/work/0036-sidebar-redesign.md`
- Sibling: `meta/work/0053-sidebar-nav-and-unseen-tracker.md` (Provider already
  mounted at `RootLayout.tsx:33`)
- Glyph (already shipped): `frontend/src/components/Glyph/Glyph.tsx`
- Server SSE hub: `server/src/sse_hub.rs:6-40`
- Server watcher: `server/src/watcher.rs:26-152`
- Server API routing: `server/src/api/mod.rs:22-42`
- Frontend SSE hook: `frontend/src/api/use-doc-events.ts:15-192`
- Frontend types: `frontend/src/api/types.ts:113-126`
- Frontend Sidebar: `frontend/src/components/Sidebar/Sidebar.tsx`
- Existing relative-time helper: `frontend/src/api/format.ts`
- Existing fetch helper precedent: `frontend/src/api/fetch.ts:63-68`
- Existing query-key precedent: `frontend/src/api/query-keys.ts:3-22`
- Existing SSE-state UI precedent:
  `frontend/src/components/SseIndicator/SseIndicator.tsx`
- Existing test harness pattern:
  `frontend/src/components/SseIndicator/SseIndicator.test.tsx`,
  `frontend/src/components/Topbar/Topbar.test.tsx`
