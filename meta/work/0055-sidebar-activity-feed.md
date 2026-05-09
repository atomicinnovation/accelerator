---
work_item_id: "0055"
title: "Sidebar Activity Feed and SSE Action Discriminator"
date: "2026-05-11T12:11:50+00:00"
author: Toby Clemson
type: story
status: ready
priority: high
parent: "0036"
tags: [design, frontend, chrome, navigation]
---

# 0055: Sidebar Activity Feed and SSE Action Discriminator

**Type**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a user navigating the visualisation app, I want a rolling Activity feed in the Sidebar that shows the most recent file-change events with action labels and relative timestamps, so I can notice ongoing activity in the system without leaving my current view.

Extend `SsePayload::DocChanged` (the Rust variant that serialises to a `doc-changed` SSE event) with an `action` discriminator (`created` | `edited` | `deleted`) and a `timestamp` field; add a new `GET /api/activity?limit=N` route backed by an in-memory ring buffer alongside the existing SSE broadcast hub. Build an ActivityFeed component inside the Sidebar slot delivered by 0053: row prepend on SSE event, LIVE badge bound to `connectionState`, `<n>s/m/h/d ago` relative-timestamp (seconds/minutes/hours/days) refreshed by a 60 s ticker, initial-history fetch + empty-state for clean checkouts.

## Context

Child of 0036 — Sidebar Redesign. 0036 is the bundled epic; this story owns the Activity feed plus the supporting server-side changes (SSE wire-format extension and the new history endpoint). The Sidebar layout that hosts the feed is delivered by 0053 (sibling). Search behaviour is delivered by 0054 (sibling).

The current `SsePayload` in `server/src/sse_hub.rs:6-21` exposes only `DocChanged { docType, path, etag }` and `DocInvalid { docType, path }` — there is no action discriminator and no per-event timestamp. The watcher (`server/src/watcher.rs:139-150`) emits a single `DocChanged` for any FS event after debounce, with deletions encoded as `etag: None`. `SseHub` is a thin wrapper around a Tokio broadcast channel (`sse_hub.rs:23-40`); late subscribers miss everything emitted before they connected — there is no event history or replay buffer today.

## Requirements

This story owns the Activity feed frontend plus two server-side changes: extending the SSE wire format and adding a history endpoint.

- Extend `SsePayload::DocChanged` (`server/src/sse_hub.rs:6-21`) with two new fields:
  - `action: 'created' | 'edited' | 'deleted'` (serialised lowercase via a per-enum `rename_all = "lowercase"`). Populated by the watcher (`server/src/watcher.rs:139-150`) from the pre/post rescan comparison: `pre.is_none() && post.is_some()` → `'created'`; `pre.is_some() && post.is_some()` → `'edited'`; `pre.is_some() && post.is_none()` → `'deleted'`. Renames surface as create+delete pairs under this mapping; a dedicated `'moved'` action is intentionally not introduced in this story.
  - `timestamp: ISO-8601 string` — wall-clock time at which the watcher emits the broadcast, captured via `Utc::now()` immediately before `hub.broadcast(...)`.
- Encode `etag` as `Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]` so the field is omitted from the JSON entirely on delete events. `action === 'deleted'` is the canonical detector for deletions; the absence of `etag` remains a legacy fallback for consumers that pre-date the discriminator.
- Add a new server route `GET /api/activity?limit=N` registered in `server/src/api/mod.rs:22-42`, returning recent events from an in-memory ring buffer maintained alongside the broadcast channel. Response shape: `{ events: Array<{ action: 'created' | 'edited' | 'deleted', docType: DocTypeKey, path: string, timestamp: ISO-8601-string }> }`, ordered newest-first (most recent event at index 0). `DocTypeKey` is the doc-type identifier defined in `frontend/src/api/types.ts`.
- Ring buffer capacity: at least 50 events. Push events into the buffer synchronously immediately before `hub.broadcast(...)` (the broadcast channel is lossy; subscribing the buffer to the channel would drop events under load). Persistence across server restarts is out of scope.
- Build an ActivityFeed component inside the Sidebar slot delivered by 0053. The component:
  - Renders a heading containing the title "Activity" and the LIVE badge (see below). All other structural elements (rows, empty state) sit beneath the heading.
  - Seeds initial state from `GET /api/activity?limit=5`.
  - Subscribes to the SSE stream via `useDocEventsContext` and prepends each `doc-changed` event (the JSON form of `SsePayload::DocChanged`) to the top of the feed.
  - Renders each row with the per-doc-type Glyph (delivered by 0037 — per-doc-type variants keyed by `DocTypeKey`), the action label (`created` | `edited` | `deleted` rendered verbatim, i.e. the discriminator string with no transformation), and a `<n>s/m/h/d ago` relative-timestamp + filename line.
  - Shows a `LIVE` badge element (an element with text content `LIVE` and `data-testid="activity-live-badge"`) inside the heading while the SSE `connectionState` is `'open'`, and removes that element when `connectionState` is not `'open'` (`'connecting'`, `'reconnecting'`, `'closed'`).
  - Re-renders visible relative-timestamps on a fixed 60 s cadence while mounted, driven by a single `setInterval(..., 60_000)` that triggers a state update on each tick.
  - Renders an empty-state element with text content "No recent activity" and `data-testid="activity-empty"` when the initial-history response returns `events: []`.
  - Self-cause filter: by default, `doc-changed` events flagged as self-caused by `api/self-cause.ts` (i.e. events the helper attributes to the user's own PATCH edits in the same browser session) are included in the feed. In practice today, PATCH-driven writes are suppressed upstream by `WriteCoordinator` and do not reach the live SSE stream, so this default-include behaviour is observable only via test-harness events; it is documented here so the include semantics are pinned for any future code path that bypasses `WriteCoordinator`.

## Acceptance Criteria

- [ ] Given the ActivityFeed component is mounted and an SSE event with shape `{ type: 'doc-changed', action: 'created' | 'edited' | 'deleted', docType, path, etag?, timestamp }` is dispatched through the test harness's `DocEventsContext`, when the component renders on the next render tick, then a new row appears at the top of the feed displaying the per-doc-type Glyph, the action label (the `action` value rendered verbatim, with no transformation), and a `<n>s/m/h/d ago` relative-timestamp + filename line.
- [ ] Given the SSE `connectionState` exposed by `useDocEventsContext` is `'open'`, when the ActivityFeed heading renders, then an element with text content `LIVE` and `data-testid="activity-live-badge"` is present inside the heading. Given `connectionState` is not `'open'` (`'connecting'`, `'reconnecting'`, or `'closed'`), when the heading renders, then no element with `data-testid="activity-live-badge"` is present.
- [ ] Given the ActivityFeed is mounted with at least one row visible, when controlled time advances by 60 seconds under a fake timer, then each visible relative-timestamp re-renders exactly once (the 60 s `setInterval` fires once and triggers one state update; advancing by 120 s triggers exactly two re-renders).
- [ ] Given the ActivityFeed is mounted with rows visible, when the relative-timestamp formatter is invoked with elapsed times of 30s, 90s, 3700s and 90000s, then the rendered strings are `30s ago`, `1m ago`, `1h ago` and `1d ago` respectively (unit boundaries: elapsed < 60s → `<n>s ago`; 60s ≤ elapsed < 3600s → `<n>m ago`; 3600s ≤ elapsed < 86400s → `<n>h ago`; elapsed ≥ 86400s → `<n>d ago`; `<n>` is computed as `Math.floor(elapsed_seconds / unit_size_in_seconds)`).
- [ ] Given the ActivityFeed is mounted with the SSE `connectionState` initially `'open'` (the `data-testid="activity-live-badge"` element is present), when `connectionState` transitions to `'closed'`, then on the next render the badge element is removed; when `connectionState` transitions back to `'open'`, the badge element reappears.
- [ ] Given the user opens the app, when the ActivityFeed component first mounts, then it issues `GET /api/activity?limit=5`, and once the response resolves (response shape: `{ events: Array<{ action: 'created' | 'edited' | 'deleted', docType: DocTypeKey, path: string, timestamp: ISO-8601-string }> }` ordered newest-first), the rendered feed contains a row for each event in the response in the order returned. Verified with a test harness whose SSE stream is suppressed until after the initial-history response resolves.
- [ ] Given `GET /api/activity?limit=5` returns `events: []` (clean checkout), when the ActivityFeed renders, then an element with text content `No recent activity` and `data-testid="activity-empty"` is present and no row elements are rendered.
- [ ] Given a new file is created on disk in a doc-type root, when the watcher emits the corresponding SSE event, then the event's `action` field equals `'created'` and `timestamp` is within 1 second of the wall-clock time at which `hub.broadcast(...)` is invoked (i.e. the watcher's emit time, captured immediately before broadcast).
- [ ] Given an existing file's contents are modified on disk in a doc-type root, when the watcher emits the corresponding SSE event after debounce + rescan, then the event's `action` field equals `'edited'`.
- [ ] Given a file is deleted on disk in a doc-type root, when the watcher emits the corresponding SSE event, then the event's `action` field equals `'deleted'` and the `etag` key is omitted from the serialised JSON entirely (not present, not `null`).
- [ ] Given 6 file-change events have been recorded in known order T1 < T2 < ... < T6, when `GET /api/activity?limit=5` is called, then the response contains exactly 5 events, `events[0].timestamp === T6`, and `events[i].timestamp` is strictly greater than `events[i+1].timestamp` for all i.
- [ ] Given 50 file-change events have been recorded since server start, when `GET /api/activity?limit=50` is called, then the response contains 50 events.
- [ ] Given 51 file-change events have been recorded in known order T1 < T2 < ... < T51, when `GET /api/activity?limit=50` is called, then the response contains exactly 50 events, `events[0].timestamp === T51`, and `events[49].timestamp === T2` (i.e. T1 has been evicted; the ring buffer drops the oldest event, not the newest).
- [ ] Given a server-originated edit (PATCH) produces a `doc-changed` SSE event flagged as self-caused by `api/self-cause.ts`, when the ActivityFeed receives the event, then a row for that event is rendered (the default include-self-caused behaviour).

## Open Questions

- Should clicking an Activity feed row navigate to the affected document, or only highlight it in the nav?

Resolved: pagination/expand to history beyond the initial five events is out of scope for this story. The feed remains a rolling-five view; any pagination/retention model is deferred to a follow-on story (see Dependencies → Blocks).

## Dependencies

- Parent: 0036 — Sidebar Redesign (this story is one of three deliverable units decomposed from 0036; closing it closes part of 0036's scope).
- Blocked by:
  - 0037 — per-doc-type Glyph variants keyed by `DocTypeKey` (not merely the Glyph component existing). Partial coverage of the doc types referenced by feed events would break row rendering.
  - 0053 — both the Sidebar slot that hosts the feed *and* the `DocEventsContext` Provider mounted in `RootLayout` (without the Provider, `_defaultHandle` is inert and the LIVE-badge / SSE subscription paths do not work).
- Coordinates with: 0053's unseen-changes tracker — this story's wire-format extension assumes the tracker ignores unknown SSE fields and does not consume the new `action` discriminator. See Assumptions.
- Blocks:
  - No work items today. Pagination/retention beyond the rolling-five view is out of scope (see Open Questions → Resolved); when a follow-on is filed it will depend on this story's ring-buffer capacity decision (≥50) but filing that follow-on is not a close criterion for 0055.
  - Any future SSE consumer that wants to read `action` or `timestamp` (none named today) is implicitly unblocked by this story.

## Assumptions

- The new `action` and `timestamp` fields on `SsePayload::DocChanged` are additive — existing frontend consumers of the SSE stream (specifically `use-doc-events.ts`) ignore unknown fields and continue to function unchanged after the wire-format extension.
- An in-memory ring buffer is sufficient for the initial history endpoint; persistence across server restarts is not required (a clean restart triggers the empty-state in the feed, which is acceptable behaviour).
- The unseen-changes tracker delivered by 0053 does not consume the new `action` discriminator — it only cares that a `doc-changed` event arrived for a given doc type. This story does not modify the tracker.
- The existing `frontend/src/api/self-cause.ts` helper's flagging contract (which events count as self-caused) is stable. AC11's verification depends on this helper continuing to flag PATCH-driven events as self-caused; a future change to the helper would invalidate the AC's premise.

## Technical Notes

**Size**: M — server-side wire-format extension (sse_hub.rs + watcher.rs + 6 existing tests) plus a small ring-buffer module and new `/api/activity` route; frontend ActivityFeed component, DocEventsHandle extension, relative-timestamp formatter + ticker, fetch/queryKey scaffolding. Touches both halves of the stack but every change slots into an established pattern — no new infrastructure, no migrations.

- **SSE wire-format change**:
  - `SsePayload::DocChanged` in `server/src/sse_hub.rs:6-21`: add `action: ActionKind` and `timestamp: chrono::DateTime<Utc>` (serialises to ISO-8601 by default).
  - `ActionKind` enum with variants `Created`, `Edited`, `Deleted`. Annotate the enum with `#[serde(rename_all = "lowercase")]` so values serialise as `"created"` / `"edited"` / `"deleted"` (the container's `kebab-case` rename applies to variant *names*, not field values, so the nested rename is required).
  - Populate from the watcher's existing pre/post comparison (`server/src/watcher.rs:137-152`): `pre.is_none() && post.is_some()` → `Created`; `pre.is_some() && post.is_some()` → `Edited`; `pre.is_some() && post.is_none()` → `Deleted`. The watcher does *not* inspect `notify::EventKind` for this story — renames surface as create+delete pairs, and the codebase has no `'moved'` discriminator value to handle.
  - `timestamp` is captured via `Utc::now()` in the watcher debounce closure immediately before `hub.broadcast(...)`, so AC6's tolerance is measured against the broadcast wall-clock, not the FS mtime.
  - Encode `etag` as `Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]` so the field is omitted entirely on deletes. `action === 'deleted'` is the canonical detector; the existing "absent `etag`" signal stays as a legacy fallback.
- **Activity history endpoint**:
  - Add `ActivityRingBuffer` adjacent to `SseHub` (`server/src/sse_hub.rs:23-40`); push each event before broadcast.
  - Buffer capacity at least 50; expose `recent(limit: usize) -> Vec<Event>`.
  - Register `GET /api/activity` in `server/src/api/mod.rs:22-42`; handler reads the buffer and returns newest-first.
- **Frontend ActivityFeed component**:
  - Mount inside the Sidebar slot delivered by 0053.
  - Seed initial state via `useQuery({ queryKey: queryKeys.activity(5), queryFn: () => fetchActivity(5) })` on mount.
  - Subscribe to SSE via `useDocEventsContext` — extend the hook to expose a rolling event observable, or tap the dispatcher (`use-doc-events.ts:136-150`) with a sibling subscriber.
  - Self-cause filter (`api/self-cause.ts`) — decide whether to include or exclude self-caused events in the feed; default to include (the user wants to see all activity, including their own).
  - LIVE badge bound to `connectionState` from `useDocEventsContext`.
  - Relative-timestamp formatter: small helper in `frontend/src/api/format.ts`; ticker via `useEffect` with `setInterval(..., 60_000)` forcing a re-render every 60 s while mounted.

### Implementation gotchas (from codebase analysis)

- **Naming collision with existing `server/src/activity.rs`** — that module is the
  idle-timeout HTTP-activity tracker (`AtomicI64`, request-middleware), unrelated
  to file-change activity. The new ring buffer + route must live in a separate
  module (e.g. `server/src/activity_feed.rs` or `server/src/api/activity.rs`) to
  avoid conflating the two `Activity` concepts.
- **Ring buffer must be populated synchronously, not via subscription**
  (`sse_hub.rs:80-93`). The Tokio broadcast channel is lossy — slow consumers
  recover via `RecvError::Lagged`. If the ring buffer subscribed to the channel
  for backfill, it would silently drop events. Push events into the buffer
  immediately before `hub.broadcast(...)` in the watcher path
  (`watcher.rs:137-152`).
- **Serde discriminator strategy is mixed** (`sse_hub.rs:7`). The container uses
  `#[serde(tag = "type", rename_all = "kebab-case")]` for variant names, but
  field renames are applied per-field (`#[serde(rename = "docType")]` at `:10`,
  `:17`). Adding `action: ActionKind` requires a nested enum with its own
  `#[serde(rename_all = "lowercase")]` so values serialise as `"created"` /
  `"edited"` / `"deleted"` — the container `rename_all` applies to variant
  *names*, not field values.
- **Watcher does not currently inspect `notify::EventKind`** (`watcher.rs:26-94`).
  Today create/update/delete is inferred only by comparing `pre: Option<IndexEntry>`
  captured at `:68` against the post-rescan `indexer.get(&path)` at `:137-152`.
  This story reuses that pre/post comparison: `pre.is_none() && post.is_some()`
  → `Created`; `pre.is_some() && post.is_some()` → `Edited`;
  `pre.is_some() && post.is_none()` → `Deleted`. Renames produce a
  create+delete pair under this scheme, which is acceptable for the rolling
  Activity feed and aligns with the dropped `'moved'` discriminator.
- **`WriteCoordinator` self-write suppression** (`watcher.rs:118-121`) drops
  events triggered by the server's own writes, so PATCH-driven writes do not
  reach the live SSE stream today. AC11 (self-cause include behaviour) is
  therefore *not* tested against the live stream — it is verified by
  dispatching a synthetic `doc-changed` event flagged as self-caused (by
  `api/self-cause.ts`) through the test harness's `DocEventsContext`,
  asserting the ActivityFeed renders a row for it. A separate channel from
  `WriteCoordinator` that would allow server-originated edits onto the live
  SSE stream is out of scope for 0055.
- **Wire-format tests pin the contract** — adding `action` and `timestamp`
  fields requires updating `sse_payload_json_wire_format` (`sse_hub.rs:95-124`)
  and the five watcher tests at `watcher.rs:236-447` (`file_change_produces_*`,
  `rapid_writes_coalesce_*`, `malformed_frontmatter_*`,
  `new_file_in_watched_dir_*`, `file_deletion_produces_doc_changed_without_etag`).
- **Frontend dispatch ignores unknown event types** (`use-doc-events.ts:56`).
  Adding `action` and `timestamp` as new *fields* on `doc-changed` is
  transparent to the existing dispatcher (TS structural typing accepts extra
  fields). The TS twin types `SseDocChangedEvent` (`api/types.ts:87-92`) must
  be extended to expose them to the new ActivityFeed consumer. The
  unseen-changes tracker (delivered by 0053) only checks that a `doc-changed`
  event arrived for a given doc type; it does not read the new `action` /
  `timestamp` fields, so this change is additive from its perspective.
- **`useDocEvents` is a hook, not a singleton** (`use-doc-events.ts:90`).
  Calling it twice opens two `EventSource` connections. ActivityFeed must
  consume `useDocEventsContext` (`:171-175`). The current default handle
  (`_defaultHandle`) is inert — a Provider must wrap the tree, which 0053's
  Technical Notes already commit to mounting in `RootLayout`.
- **`DocEventsHandle` exposes `connectionState`** today (`use-doc-events.ts:15-19`,
  `:94`, `:114`) — LIVE-badge wiring is direct. What the handle does *not*
  expose is a way to observe individual events; extending it with a small
  subscribe/event-list API is the cleanest path. Update `_defaultHandle` with
  a no-op stub so the default Context value stays inert.
- **`SESSION_STABLE_QUERY_ROOTS`** (`query-keys.ts:24-27`) — the query-key
  roots that are exempt from invalidation on SSE reconnect (everything else
  is invalidated to force a refetch). `activity` should NOT be added: on
  reconnect the feed should refetch via `GET /api/activity` so the rolling
  buffer is rebuilt from that response plus subsequent live events.
- **`api/mod.rs` route convention** (`api/mod.rs:22-42`) — each handler lives
  in its own module file (`mod events; mod docs; mod types; …` at `:1-9`).
  Add `mod activity_feed;` (or similar) plus a
  `.route("/api/activity", get(activity_feed::handler))` line alongside
  `/api/events` at `:24`. Return `Result<Json<T>, ApiError>` (`:44-69`,
  `:71-148`) to reuse the shared error machinery.
- **Fetch convention** (`fetch.ts:1-113`) — no `?limit=N` helper exists yet;
  closest analog is `fetchDocs` using `?type=...` at `:64`. Mirror that shape:
  bare `fetch('/api/activity?limit=' + n)`, unwrap `{ events: [...] }`,
  throw `FetchError(status, message)` on non-ok.
- **`queryKeys` convention** (`query-keys.ts:3-22`) — add
  `activity: (limit: number) => ['activity', limit] as const` (parameterised
  variant). No prefix-invalidate variant needed; the feed has one query at a
  time.

## Drafting Notes

- Extracted from 0036 (Sidebar Redesign) as part of decomposing that bundled story into three deliverable units. See 0036 for the full design rationale and parent-level Technical Notes.

## References

- Parent: `meta/work/0036-sidebar-redesign.md`
- Source: `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Related: 0037, 0053, 0054
