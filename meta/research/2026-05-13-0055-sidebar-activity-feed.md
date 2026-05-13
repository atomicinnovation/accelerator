---
date: "2026-05-13T09:15:21+01:00"
researcher: Toby Clemson
git_commit: 99c29ca35498adbe72fafec97a68216d6ebf6fde
branch: yptuuowrwpqzxxzrvvzmqpxwsnrtpkkt
repository: accelerator
topic: "Educate implementation of 0055 — Sidebar Activity Feed and SSE Action Discriminator"
tags: [research, codebase, sse, watcher, activity-feed, sidebar, sse-hub, frontend, server, work-item-0055]
status: complete
last_updated: 2026-05-13
last_updated_by: Toby Clemson
---

# Research: Educate implementation of 0055 — Sidebar Activity Feed and SSE Action Discriminator

**Date**: 2026-05-13 09:15:21 BST
**Researcher**: Toby Clemson
**Git Commit**: `99c29ca35498adbe72fafec97a68216d6ebf6fde`
**Branch**: anonymous change `yptuuowrwpqzxxzrvvzmqpxwsnrtpkkt` (on top of `main`)
**Repository**: accelerator (workspace `visualisation-system`)

## Research Question

What does an implementer of `meta/work/0055-sidebar-activity-feed.md` need to know about the current codebase before planning? Specifically: verify every file/line the work item cites, surface gotchas, identify silent prerequisites (deps, AppState plumbing, missing slots, claimed callback channels), and pin the test patterns and conventions the implementation must follow.

## Summary

The work item is in good shape (three review passes, verdict `COMMENT` — see `meta/reviews/work/0055-sidebar-activity-feed-review-1.md`), and most of its file:line citations are accurate to within a few lines. The codebase is ready to receive this change with **one hard prerequisite not called out in the work item**: `chrono` is not a direct server dependency today, so `Cargo.toml` must gain `chrono = { version = "0.4", features = ["serde"] }` before `Utc::now()` / ISO-8601 `DateTime<Utc>` serialisation will compile.

Two other findings change the implementation path materially:

1. **`options.onEvent` on `useDocEvents` is already claimed** by the unseen-doc-types tracker (`RootLayout.tsx:17-21`). 0055 cannot reuse that single callback slot — it must extend `DocEventsHandle` with a multi-subscriber API (a `subscribe(listener) → unsubscribe` function on the handle is the natural shape). This is consistent with the work item's "tap the dispatcher with a sibling subscriber" hint, but the work item's framing that 0053 *also* plans this extension does not match reality: the unseen-tracker already exists and already consumed the slot.
2. **The `DocEventsContext` Provider is already mounted in `RootLayout`** (`RootLayout.tsx:33`). The work item's Dependencies entry treats this as a 0053 commitment; it has already shipped. The remaining 0053 dependency is just the Sidebar slot (which does not exist today).

Other notable findings: `payload_for_entry` (`watcher.rs:155-168`) currently has no access to `pre`, so the new `Created` vs `Edited` mapping forces a signature change or inlining; the watcher does no `notify::EventKind` matching anywhere (confirmed); `frontend/src/api/format.ts` already has a `formatMtime` helper that implements *almost exactly* the same unit boundaries AC4 specifies; there is no `setInterval` anywhere in the frontend today (ActivityFeed introduces the first); the existing test-harness pattern uses `vi.mock(...)` of `useDocEventsContext`, not a `<Provider>` wrapper.

The recommended new server module is `server/src/api/activity.rs` — namespaced under `api::`, this avoids collision with the crate-root `activity` module (an unrelated HTTP-activity tracker) without resorting to `activity_feed`-style suffixing.

## Detailed Findings

### Server-side: `SsePayload` + the broadcast hub

File: `skills/visualisation/visualise/server/src/sse_hub.rs`

Current `SsePayload` definition (`sse_hub.rs:6-21`):

```rust
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
```

- The container `rename_all = "kebab-case"` renames **variant names** only (`DocChanged` → `"doc-changed"`). Field renames are per-field via `#[serde(rename = "docType")]`. A nested `ActionKind` enum will need its own `#[serde(rename_all = "lowercase")]` for the value side.
- `etag` is already `Option<String>` with `skip_serializing_if` — the work item's requirement (omit `etag` entirely on deletes) is already satisfied; only the population logic changes.
- `SseHub` (`sse_hub.rs:23-40`) is a thin `broadcast::Sender<SsePayload>` wrapper. `broadcast()` is fire-and-forget. Lag handling lives in the consumer (`api/events.rs:25-28`).
- Channel capacity is `256` — constructed in `AppState::build` at `server.rs:83`: `let sse_hub = Arc::new(crate::sse_hub::SseHub::new(256));`.

Wire-format test `sse_payload_json_wire_format` (`sse_hub.rs:95-124`) pins `type`, `docType`, `etag` for `DocChanged` and `!json.contains("etag")` for deletions. All hub tests use a `make_event` helper at `sse_hub.rs:47-53` constructing `DocChanged { doc_type, path, etag }` literals — every one will need updating once `action` and `timestamp` are added (consider a `Default` impl on `ActionKind` and a constructor helper to keep test sites short).

**`chrono` is NOT in `server/Cargo.toml`** (`Cargo.toml:23-50`). Cargo.lock lists it only as a transitive of some other dep. No production code in the server crate uses `chrono::Utc::now()` or `DateTime` (grep confirmed: zero matches across `server/src/**`). Existing timestamps use `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` directly (`activity.rs:30-35`, `server.rs:366-369`). Adding `chrono = { version = "0.4", features = ["serde"] }` (or equivalent — `default-features = false` plus `["std", "clock", "serde"]` is the minimal set) is a hard prerequisite.

### Server-side: watcher pipeline

File: `skills/visualisation/visualise/server/src/watcher.rs`

The pipeline (`watcher.rs:26-152`):

1. `spawn(...)` opens a `notify::recommended_watcher`, reads events from a Tokio mpsc channel.
2. For each FS event, the outer loop captures `pre = indexer.get(&path).await` at `watcher.rs:68` **before** launching the per-path debounce task.
3. The debounce task (`on_path_changed_debounced`, `watcher.rs:97-152`) is spawned with `pre` passed in.
4. Inside the debounce task: `tokio::time::sleep(debounce).await` → canonicalise path → `WriteCoordinator::should_suppress` early-return → `indexer.rescan().await` → recompute clusters → `match indexer.get(&path).await { Some(post) => ..., None => ... }`.
5. There are exactly two `hub.broadcast(...)` call sites: `watcher.rs:139` (post present) and `watcher.rs:144` (deletion path; only fires if `pre.is_some()`).

Key implementation points:

- **`Utc::now()` capture point**: between rescan and the match block (e.g. at `watcher.rs:136`), so both `Some(post)` and `None+Some(pre)` branches share one timestamp, and the same instant is pushed into the ring buffer immediately before broadcast.
- **`pre`/`post` mapping required**: `pre.is_none() && post.is_some()` → `Created`; `pre.is_some() && post.is_some()` → `Edited`; `pre.is_some() && post.is_none()` → `Deleted`. The fourth case (`None + None`) currently broadcasts nothing — keep it that way.
- **`payload_for_entry` constraint**: `watcher.rs:155-168` is the helper that constructs `DocChanged`/`DocInvalid` for the `Some(post)` branch. It currently does NOT receive `pre`. To map `pre.is_some() → Edited` vs `pre.is_none() → Created`, either: (a) change its signature to take `pre_present: bool`; or (b) inline the helper back into the match arm so the `Created`/`Edited` choice happens at the call site. Approach (b) is likely cleaner because the helper exists only to share the `DocChanged`/`DocInvalid` decision between two callers, but only one of them — the post-present branch — needs the action discriminator.
- **`WriteCoordinator` early-return** (`watcher.rs:118-121`) runs **after** `pre` is captured but **before** rescan, broadcast, and (in the new design) ring-buffer push. So self-caused writes drop out cleanly: neither broadcast nor ring-buffer entry. This is what makes AC11 verifiable only via a synthetic event in the test harness — the live SSE path never delivers self-caused events today.
- **No `notify::EventKind` inspection anywhere** in `server/src/**` (grep confirmed zero matches). The codebase has never branched on the FS event kind; renames will continue surfacing as create+delete pairs — consistent with the work item's drop of `'moved'` from the discriminator.

The five tests at `watcher.rs:236-447` to update:

| Test (line range) | Current assertion | Change needed |
|---|---|---|
| `file_change_produces_doc_changed_event` (236-275) | `matches!(event, SsePayload::DocChanged { .. })` | Add `action == "edited"` check (pre exists, post exists). |
| `rapid_writes_coalesce_to_one_event` (277-325) | `matches!(event, SsePayload::DocChanged { .. })` + no second event in 300 ms | Same `action == "edited"` add; coalescing assertion unchanged. |
| `malformed_frontmatter_produces_doc_invalid_event` (327-366) | `matches!(event, SsePayload::DocInvalid { .. })` | Work item does NOT extend `DocInvalid` with `action`/`timestamp`; this test stays unchanged unless the design opts to add `timestamp` to `DocInvalid` too. The work item is silent — assume no change. |
| `new_file_in_watched_dir_produces_doc_changed_event` (368-404) | `matches!(event, SsePayload::DocChanged { .. })` | Add `action == "created"` (pre is None, post is Some). |
| `file_deletion_produces_doc_changed_without_etag` (406-447) | `matches!(...)` + `!json.contains("etag")` | Add `action == "deleted"`; `!json.contains("etag")` assertion remains valid. |

### Server-side: AppState and watcher wiring

File: `skills/visualisation/visualise/server/src/server.rs`

`AppState` (`server.rs:40-50`) is the canonical Arc-shared state struct:

```rust
pub struct AppState {
    pub cfg: Arc<Config>,
    pub kanban_columns: Arc<Vec<crate::config::KanbanColumn>>,
    pub file_driver: Arc<crate::file_driver::LocalFileDriver>,
    pub indexer: Arc<crate::indexer::Indexer>,
    pub templates: Arc<crate::templates::TemplateResolver>,
    pub clusters: Arc<RwLock<Vec<crate::clusters::LifecycleCluster>>>,
    pub activity: Arc<crate::activity::Activity>,
    pub sse_hub: Arc<crate::sse_hub::SseHub>,
    pub write_coordinator: Arc<crate::write_coordinator::WriteCoordinator>,
}
```

Threading plan for the new ring buffer:
- Add field `pub activity_feed: Arc<crate::api::activity::ActivityRingBuffer>` (or whichever module name; see naming section below).
- Construct adjacent to `SseHub::new(256)` at `server.rs:83`.
- Thread into `crate::watcher::spawn(...)` at `server.rs:284-292` as a new argument; `watcher::spawn`'s signature (currently 7 args at `watcher.rs:26-34`) grows by one, and `on_path_changed_debounced` does too.
- The HTTP handler reads the buffer via `State(state): State<Arc<AppState>>` — no extra router plumbing.

### Server-side: API routing convention

File: `skills/visualisation/visualise/server/src/api/mod.rs`

Module list at `api/mod.rs:1-9` is "one file per route module" (`mod docs; mod events; mod info; mod kanban_config; mod lifecycle; mod related; mod templates; mod types; mod work_item_config;`).

Route table (`api/mod.rs:22-42`) lists routes in a single `Router::new()...route(...).route(...)` chain. New entry slot: alongside `/api/events` (line 24, both are SSE-pipeline-flavoured):

```rust
.route("/api/activity", get(activity::handler))
```

The simplest handler precedent is `api/types.rs` (full file is ~14 lines): extract `State(state): State<Arc<AppState>>`, build a response, return `Json<Response>`. The new handler is read-only and only fails on `limit` parsing; using plain `Json<...>` (not `Result<Json<...>, ApiError>`) is consistent with `events`/`types`. The `query` feature is on Axum (`Cargo.toml:24`), so `Query<LimitParam>` is available for `?limit=N` parsing.

`ApiError` machinery (`api/mod.rs:44-148`) is `thiserror`-based with nine variants and bespoke `IntoResponse` mappings. Reach for it only if `?limit` validation should return a 400.

### Naming collision: `crate::activity` vs the new module

File: `skills/visualisation/visualise/server/src/activity.rs` (29 lines) is an HTTP-activity tracker — `AtomicI64` + Tower middleware that touches the atomic on every request, consumed by the idle-timeout watch. Doc comment (`activity.rs:1-3`):

> Tracks the timestamp of the most recent HTTP activity. Consumed by the idle-timeout watch. Middleware updates the atomic on every request; no request-path changes needed.

Public API: `pub struct Activity(AtomicI64)`, `Activity::new()`, `.touch()`, `.last_millis()`, `middleware(...)`. Wired in `lib.rs:9` (`pub mod activity;`) and `server.rs:47` (the `activity: Arc<crate::activity::Activity>` field on `AppState`).

**Do NOT name the new module `activity` at crate root.** Two acceptable options:

1. **`server/src/api/activity.rs`** (recommended) — namespaced under `api::`, so the path is `crate::api::activity::handler` and `crate::api::activity::ActivityRingBuffer`. No collision with crate-root `crate::activity`. Mirrors the precedent of `api/types.rs` (`crate::api::types`) coexisting with the unrelated `crate::docs::DocType` types module. If you want the ring buffer to live outside `api/`, also legit — but pick a different name.
2. **`server/src/activity_feed.rs`** — separate crate-root module. Slightly more verbose but unambiguous.

Grep confirms: nothing in the server crate references `activity_feed`, `ActivityFeed`, `ActivityRingBuffer`, or `RingBuffer`.

### Server-side: indexer + IndexEntry

File: `skills/visualisation/visualise/server/src/indexer.rs`

`IndexEntry` (`indexer.rs:15-34`) — `Clone`, so cloning into `pre: Option<IndexEntry>` at `watcher.rs:68` works fine. Only `entry.r#type` (which gives the `DocTypeKey`) is needed from `pre_entry` in the new mapping (`watcher.rs:145` already does this). The new logic also needs only the doc type from `pre`.

`indexer.get(&Path)` (`indexer.rs:338-347`) — `async fn` that reads from `Arc<RwLock<HashMap<PathBuf, IndexEntry>>>` and returns `Option<IndexEntry>` (cloned out). This is the canonical shape for shared state; the ring buffer should follow it (`Arc<Mutex<VecDeque<...>>>` is sufficient — `std::sync::Mutex` matches `WriteCoordinator`'s precedent at `write_coordinator.rs:10`).

### Frontend: `use-doc-events.ts` — the constrained extension point

File: `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`

The work item's line citations are off by ~15-20 lines vs the current file (likely the work item was authored against an older revision). Actual locations:

- `DocEventsHandle` type: `use-doc-events.ts:15-19` — exposes `setDragInProgress`, `connectionState`, `justReconnected`. **No subscribe API today.**
- `_defaultHandle` (inert default): `use-doc-events.ts:182-186` — `connectionState: 'connecting'`. Critical: if a tree forgets to wrap in `<DocEventsContext.Provider>`, the LIVE badge will never appear (AC2 requires `'open'`).
- `makeUseDocEvents` factory: `use-doc-events.ts:87-177`.
- `dispatchSseEvent` exported pure dispatch: `use-doc-events.ts:42-85`.
- `onmessage` handler (the single per-event tap point): `use-doc-events.ts:152-167`. Drops self-caused events at line 155 (`registry.has(event.etag)`), then invokes `onEventRef.current?.(event)` at line 156 before either queuing (drag in progress) or invalidating queries.
- `DocEventsContext` + `useDocEventsContext`: `use-doc-events.ts:188-192`.

**The hard constraint**: the existing `options.onEvent` callback is **already claimed**. `RootLayout.tsx:17-21`:

```tsx
const unseen = useUnseenDocTypes()
const docEvents = useDocEvents({
  onEvent: unseen.onEvent,
  onReconnect: unseen.onReconnect,
})
```

So the unseen-doc-types tracker (`frontend/src/api/use-unseen-doc-types.ts`, already shipped) holds the single `onEvent` slot. 0055 cannot piggy-back. The natural extension is one of:

1. **Multi-subscriber API on the handle**: add `subscribe(listener: (e: SseEvent) => void): () => void` to `DocEventsHandle`. Inside `makeUseDocEvents`, keep a `useRef<Set<Listener>>()` and invoke at the same spot as line 156. `_defaultHandle.subscribe = () => () => {}`. Composes with existing `onEvent`.
2. **Rolling events array on the handle**: `recentEvents: SseEvent[]` exposed via context — readable directly by ActivityFeed without subscription.

Option 1 is the cleanest fit (smaller behavioural surface, no React state churn for non-feed consumers). It also means *the per-listener filter for self-cause is the listener's responsibility* — sibling subscribers see every parsed event from the wire, which matches AC11 ("include self-caused events by default") without code changes to the existing line-155 filter.

**Important nuance**: the work item assumes the `DocEventsContext` Provider mount is a 0053 commitment yet to land. It is **already mounted** at `RootLayout.tsx:33`: `<DocEventsContext.Provider value={docEvents}>`. The only 0053 prerequisite still genuinely outstanding is the Sidebar slot (which does not exist today — see below).

### Frontend: types

File: `skills/visualisation/visualise/frontend/src/api/types.ts`

- `DocTypeKey` (`types.ts:4-8`) — a 13-variant string union plus a runtime mirror `DOC_TYPE_KEYS` (`types.ts:14-19`) and a guard `isDocTypeKey` (`types.ts:22-24`).
- `SseDocChangedEvent` (`types.ts:113-118`) and the `SseEvent` discriminated union (`types.ts:126`). Extend `SseDocChangedEvent` with `action: 'created' | 'edited' | 'deleted'` and `timestamp: string` (ISO-8601). TypeScript structural typing makes this additive — `dispatchSseEvent` and the unseen tracker keep working unchanged.
- The work item refers to `DOC_TYPE_LABELS` (`types.ts:35-49`) implicitly via "action label rendered verbatim". The label rendering rule is "render the discriminator string with no transformation" — i.e. literal `"created"` / `"edited"` / `"deleted"`. No transformation needed.

### Frontend: self-cause helper

File: `skills/visualisation/visualise/frontend/src/api/self-cause.ts` (57 lines)

Public API:

```ts
export interface SelfCauseRegistry {
  register(etag: string): void
  has(etag: string | undefined): boolean
  reset(): void
}
export const defaultSelfCauseRegistry: SelfCauseRegistry = createSelfCauseRegistry()
export const SelfCauseContext = createContext<SelfCauseRegistry>(defaultSelfCauseRegistry)
export function useSelfCauseRegistry(): SelfCauseRegistry
```

- Flagging contract: an event is self-caused iff its `etag` was `register()`ed within 5 s (TTL) and the entry hasn't been FIFO-evicted (capacity 256). `pruneExpired` runs on each `register`/`has`.
- The registry resets on SSE reconnect (`use-doc-events.ts:134`).
- AC11 says ActivityFeed *includes* self-caused events by default. The implementation does NOT need to consult `useSelfCauseRegistry` — once the multi-subscriber API is added, sibling subscribers receive every parsed event (the line-155 drop only affects the query-invalidation dispatcher path). If a future feature wants an "exclude self-caused" toggle, it would consult `useSelfCauseRegistry()` and filter at the component level.

The work item's gotcha at lines 143-151 spells this out: `WriteCoordinator` suppresses self-writes upstream, so self-caused events don't reach the live SSE stream today; AC11 is exercisable only via a synthetic event dispatched through the test harness's mocked `useDocEventsContext`.

### Frontend: fetch convention

File: `skills/visualisation/visualise/frontend/src/api/fetch.ts`

`fetchDocs` (`fetch.ts:63-68`) is the closest analogue for `fetchActivity(limit)`:

```ts
export async function fetchDocs(type: DocTypeKey): Promise<IndexEntry[]> {
  const r = await fetch(`/api/docs?type=${encodeURIComponent(type)}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/docs?type=${type}: ${r.status}`)
  const body: DocsListResponse = await r.json()
  return body.docs
}
```

Pattern: inline URL with `encodeURIComponent`, throw `FetchError(status, msg)` on non-2xx, parse `{ events: [...] }` and unwrap. No `?limit=N` helper exists anywhere today — `fetchActivity` introduces it. `FetchError` (`fetch.ts:11-16`) carries `status: number` so callers can branch.

### Frontend: query-keys

File: `skills/visualisation/visualise/frontend/src/api/query-keys.ts`

`queryKeys` (`query-keys.ts:3-22`) — add:

```ts
activity: (limit: number) => ['activity', limit] as const,
```

This mirrors `docs: (type) => ['docs', type]` (line 7).

`SESSION_STABLE_QUERY_ROOTS` (`query-keys.ts:24-27`) currently lists only `'server-info'` and `'work-item-config'`. The work item explicitly forbids adding `'activity'` here — on SSE reconnect the feed should refetch via `GET /api/activity` so the rolling buffer is rebuilt from the response plus subsequent live events. Confirmed: current behaviour at `use-doc-events.ts:140` invalidates every query whose first key element is not in this set on reconnect, which is exactly what we want.

### Frontend: format.ts — helper already 90% there

File: `skills/visualisation/visualise/frontend/src/api/format.ts` (11 lines)

```ts
export function formatMtime(ms: number, now: number = Date.now()): string {
  if (ms <= 0) return '—'
  const diffSec = Math.floor((now - ms) / 1000)
  if (diffSec < 0)          return 'just now'
  if (diffSec < 60)         return `${diffSec}s ago`
  if (diffSec < 3600)       return `${Math.floor(diffSec / 60)}m ago`
  if (diffSec < 86400)      return `${Math.floor(diffSec / 3600)}h ago`
  if (diffSec < 7 * 86400)  return `${Math.floor(diffSec / 86400)}d ago`
  if (diffSec < 30 * 86400) return `${Math.floor(diffSec / (7 * 86400))}w ago`
  return new Date(ms).toLocaleDateString()
}
```

This already implements AC4's `<n>s` / `<n>m` / `<n>h` / `<n>d` boundaries identically for the first four branches. Where it diverges from AC4: AC4 keeps `<n>d ago` for **all** elapsed ≥86400s, but `formatMtime` flips to `<n>w ago` at 7 days and `toLocaleDateString` after 30 days.

Decision points for the implementer:
- **Add a sibling export** `formatRelative(ms, now)` (or `formatActivityTime`) that returns `<n>d ago` for everything ≥86400s. Coexists with `formatMtime`. Test sites mirror `format.test.ts`.
- **OR re-use `formatMtime` and accept the divergence** — but then AC4 with `elapsed = 90000s ≈ 1d` would still pass (`<7*86400` branch returns `1d ago`), while a `7+` day stale event would fail AC4. Since the rolling-five feed will only ever show very recent events in practice (the ring buffer holds ≥50 events at server lifetime), this might be acceptable; but the AC pinned by the work item is explicit, so the safer route is a dedicated helper.

`format.test.ts` exists as a precedent for the test shape.

### Frontend: connection state binding

File: `skills/visualisation/visualise/frontend/src/api/reconnecting-event-source.ts`

`ConnectionState = 'connecting' | 'open' | 'reconnecting' | 'closed'` re-exported at `use-doc-events.ts:11`. Driven from inside `makeUseDocEvents` (`use-doc-events.ts:129`) via the `onStateChange: setConnectionState` callback passed to `ReconnectingEventSource`.

LIVE-badge AC: render badge iff `connectionState === 'open'`. Direct conditional render in the heading. The closest existing precedent for `connectionState`-driven UI is `components/SseIndicator/SseIndicator.tsx` — also useful as a reference for test patterns (see Testing section below).

### Frontend: ticker — no existing precedent

Searched the entire `frontend/src/` for `setInterval`: zero matches. The codebase has only `setTimeout` (one-shot timers for confirm-dismiss in KanbanBoard, deferred-fetch hints, reconnect backoff, the 3 s `justReconnected` window). The ActivityFeed introduces the first repeating interval.

Recommended pattern (matches the existing `useEffect` + timer idiom in `use-deferred-fetching-hint.ts:27`):

```tsx
useEffect(() => {
  const id = setInterval(() => setTick((t) => t + 1), 60_000)
  return () => clearInterval(id)
}, [])
```

`tick` is a counter purely to force re-render; visible relative-timestamps recompute on each render against `Date.now()`. AC3 requires "exactly once" re-render per 60 s tick, which `useState` setter on identical-value-update wouldn't satisfy — increment guarantees a new state value and one render.

### Frontend: Sidebar slot — does not yet exist

File: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`

Current sections in render order: search row (placeholder for 0054) → LIBRARY (phase-grouped nav) → VIEWS (Kanban, Lifecycle) → META (Templates). No Activity slot. The component prop type is `{ docTypes: DocType[] }` only — no `children`, no named subcomponent slot.

**Implication**: 0055 cannot mount the ActivityFeed inside the Sidebar today. It depends on 0053 first carving out a slot. The work item correctly lists 0053 as a blocker — coordinate slot shape (likely a final section after META, or expose a `children`/slot prop).

### Frontend: Glyph component — already shipped

File: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx`

```ts
export interface GlyphProps {
  docType: GlyphDocTypeKey
  size: 16 | 24 | 32
  ariaLabel?: string
}
```

Already accepts per-doc-type variants. `ICON_COMPONENTS` (`Glyph.tsx:45-58`) maps 12 doc-type keys to icon components in `Glyph/icons/`.

**Type-narrowing caveat**: `GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>` (`Glyph.tsx:28`). Incoming `event.docType` in `SseDocChangedEvent` is the broader `DocTypeKey`. ActivityFeed must narrow via `isGlyphDocTypeKey` (`Glyph.tsx:37-39`) before rendering — for events from `templates` (`virtual: true` types), the Glyph row would need to fall back to no icon. In practice, `templates` is a virtual doc type that the watcher doesn't observe FS events for, so this branch shouldn't fire — but the narrowing must be in place for type-correctness.

### Frontend: test harness pattern

The established pattern is **`vi.mock` of `useDocEventsContext`**, not a `<Provider value={...}>` wrapper. See `frontend/src/components/SseIndicator/SseIndicator.test.tsx:6-18` and `Topbar.test.tsx:6-8, 31-36`:

```ts
vi.mock('../../api/use-doc-events', () => ({
  useDocEventsContext: vi.fn(),
}))

function mockState(connectionState: string) {
  vi.mocked(useDocEventsContext).mockReturnValue({
    connectionState,
    justReconnected: false,
    setDragInProgress: vi.fn(),
  } as any)
}
```

For ACs that inject an SSE event "through the test harness's `DocEventsContext`" (AC1, AC11), the mocked handle must expose a stub `subscribe` that captures the listener and returns an unsubscribe function; the test then invokes the captured listener directly:

```ts
let injectEvent: ((e: SseEvent) => void) | undefined
vi.mocked(useDocEventsContext).mockReturnValue({
  connectionState: 'open',
  justReconnected: false,
  setDragInProgress: vi.fn(),
  subscribe: (listener) => { injectEvent = listener; return () => {} },
} as any)

// in test:
injectEvent?.({ type: 'doc-changed', action: 'created', ... })
```

`RootLayout.test.tsx` additionally stubs `DocEventsContext: { Provider: ({ children }: any) => children }` for tests that need the real `useDocEventsContext` to fall through without the Provider — confirming this isn't a one-off pattern.

### Initial-history fetch + suppress-SSE-until-resolved pattern

AC6 says the initial-history `GET /api/activity?limit=5` resolves first, the feed renders the returned rows in order, and only afterwards does the SSE stream connect (the harness suppresses SSE until the initial-history response resolves). This implies the implementation should not race: ActivityFeed's `useEffect` that subscribes to the SSE stream must run after the `useQuery` for initial history settles, OR the component holds the SSE-delivered events in a pending buffer until the query resolves and then concatenates.

Two practical patterns:
1. **Block subscribe until query resolves**: gate the `subscribe(...)` call on `query.isSuccess`.
2. **Always subscribe, prepend SSE events to query data**: render-time merge — `displayed = [...sseEvents, ...query.data.events]` with de-dup if needed.

Pattern (2) avoids any ordering race and matches React-Query's idiom (the cache is the source of truth, the local subscription buffer holds the rolling-prepend). The work item's "row prepend on SSE event" implies (2).

## Code References

### Server-side

- `server/src/sse_hub.rs:6-21` — `SsePayload` enum (extend `DocChanged` with `action` + `timestamp`; add nested `ActionKind` enum).
- `server/src/sse_hub.rs:23-40` — `SseHub` wrapper around `broadcast::Sender<SsePayload>`.
- `server/src/sse_hub.rs:47-53, 85-93, 95-124` — hub tests + wire-format pin (`sse_payload_json_wire_format`).
- `server/src/watcher.rs:26-34` — `watcher::spawn` signature (grows by one `Arc<ActivityRingBuffer>` param).
- `server/src/watcher.rs:68` — `pre = indexer.get(&path).await` capture site.
- `server/src/watcher.rs:97-152` — debounce coroutine; capture `Utc::now()` between rescan and the match; push to ring buffer immediately before each `hub.broadcast(...)`.
- `server/src/watcher.rs:118-121` — `WriteCoordinator::should_suppress` early-return.
- `server/src/watcher.rs:155-168` — `payload_for_entry` helper that must either grow a `pre_present` arg or be inlined.
- `server/src/watcher.rs:236-447` — five tests to update with `action` assertions.
- `server/src/api/mod.rs:1-9` — module list (add `mod activity;` if going the `api/activity.rs` route).
- `server/src/api/mod.rs:22-42` — route table (add `.route("/api/activity", get(activity::handler))` adjacent to `/api/events`).
- `server/src/api/types.rs` (whole file) — simplest handler precedent.
- `server/src/api/events.rs:13-28` — SSE consumer + lag handling (only place `RecvError::Lagged` is handled).
- `server/src/activity.rs` (whole file) — naming collision; do NOT shadow.
- `server/src/server.rs:40-50` — `AppState` (add `activity_feed` field).
- `server/src/server.rs:83` — `SseHub::new(256)` construction; ring buffer constructs here.
- `server/src/server.rs:283-292` — `watcher::spawn(...)` call; passes new arg.
- `server/src/lib.rs:1-19` — module list at crate root.
- `server/Cargo.toml:23-50` — dependency list; **add `chrono` with `serde` feature**.
- `server/src/write_coordinator.rs:10` — `std::sync::Mutex` precedent for shared-state primitives.
- `server/src/indexer.rs:15-34` — `IndexEntry` shape.
- `server/src/indexer.rs:338-347` — `Indexer::get` signature.

### Frontend

- `frontend/src/api/use-doc-events.ts:15-19` — `DocEventsHandle` (extend with `subscribe`).
- `frontend/src/api/use-doc-events.ts:42-85` — `dispatchSseEvent`.
- `frontend/src/api/use-doc-events.ts:87-177` — `makeUseDocEvents` factory.
- `frontend/src/api/use-doc-events.ts:152-167` — `onmessage` handler; line 156 is the existing tap point (`onEventRef.current?.(event)`).
- `frontend/src/api/use-doc-events.ts:182-186` — `_defaultHandle` (extend with `subscribe: () => () => {}`).
- `frontend/src/api/use-doc-events.ts:188-192` — `DocEventsContext`, `useDocEventsContext`.
- `frontend/src/api/types.ts:4-8` — `DocTypeKey`.
- `frontend/src/api/types.ts:113-126` — `SseDocChangedEvent` (add `action`, `timestamp`), `SseEvent` union.
- `frontend/src/api/self-cause.ts:50-56` — `defaultSelfCauseRegistry`, `SelfCauseContext`, `useSelfCauseRegistry`.
- `frontend/src/api/fetch.ts:11-16, 63-68` — `FetchError` and `fetchDocs` pattern (model for `fetchActivity`).
- `frontend/src/api/query-keys.ts:3-22` — `queryKeys` (add `activity: (limit) => ['activity', limit]`).
- `frontend/src/api/query-keys.ts:24-27` — `SESSION_STABLE_QUERY_ROOTS` (do NOT add `'activity'`).
- `frontend/src/api/format.ts` (whole file) — `formatMtime`; add a sibling `formatRelative` or reuse.
- `frontend/src/api/reconnecting-event-source.ts` — `ConnectionState` type.
- `frontend/src/api/use-deferred-fetching-hint.ts:27` — `useEffect` + timer idiom.
- `frontend/src/api/use-unseen-doc-types.ts` — already-shipped consumer of `options.onEvent`.
- `frontend/src/components/Sidebar/Sidebar.tsx` — Activity slot does not exist yet (0053 dependency).
- `frontend/src/components/Glyph/Glyph.tsx:28, 37-39, 45-58, 60-67` — `GlyphDocTypeKey`, `isGlyphDocTypeKey`, `ICON_COMPONENTS`, `GlyphProps`.
- `frontend/src/components/RootLayout/RootLayout.tsx:17-33` — `useDocEvents` + `<DocEventsContext.Provider>` already wired.
- `frontend/src/components/SseIndicator/SseIndicator.test.tsx:6-18` — test harness pattern (mock the hook).
- `frontend/src/components/Topbar/Topbar.test.tsx:6-8, 31-36` — same pattern.

## Architecture Insights

- **Shared mutable state in the server is Arc-wrapped**, with `std::sync::Mutex` for short-critical-section data (`WriteCoordinator`) and `tokio::sync::RwLock` for async-held data (`Indexer`'s entries map, `clusters`). The ring buffer fits the former category — pushes and reads are O(1) on a `VecDeque` and never await.
- **No `notify::EventKind` matching anywhere in the watcher** — the codebase has always inferred create/update/delete via the pre/post comparison. This is consistent with dropping `'moved'` from the action discriminator.
- **SSE wire format uses container `tag = "type" + rename_all = "kebab-case"` for variants, then per-field `#[serde(rename = "...")]` for camelCase fields**. The `ActionKind` nested enum needs its own `rename_all = "lowercase"` because the container rename applies only to variant names — not nested-enum values.
- **Frontend SSE consumption is a query-cache invalidation model, not a pub-sub model**. The existing `dispatchSseEvent` translates each SSE event into `queryClient.invalidateQueries(...)` calls. The single `options.onEvent` callback is the only escape hatch, and it's already used. Adding a sibling `subscribe` API on `DocEventsHandle` is the cleanest extension for the activity-feed use case.
- **Glyph is a closed enum of per-doc-type icons**, excluding `templates` (virtual doc type). The activity feed inherits this constraint.
- **Tests mock at the module boundary** (`vi.mock('...path...')`) rather than wrap with a Provider. Be ready to extend the mocked `DocEventsContextHandle` shape to include the new subscribe API.
- **`format.ts` already has the relative-time logic** — duplicate at low cost, or extract a shared helper.

## Historical Context

Reviews and prior plans relevant to this work:

- `meta/reviews/work/0055-sidebar-activity-feed-review-1.md` — three review passes; verdict `COMMENT` (acceptable for implementation). Carry-over minors include: AC3 mixes observable outcome with the `setInterval` mechanism phrasing; AC1 doesn't pin filename derivation (basename vs full path); AC6 white-box clock is observer-defined; AC12/AC13 assume capacity exactly 50 while Requirements says "at least 50".
- `meta/work/0036-sidebar-redesign.md` — parent epic; phase-to-doc-type table and overall sidebar design rationale.
- `meta/work/0053-sidebar-nav-and-unseen-tracker.md` — sibling; defines the Sidebar layout + Provider mount (already shipped) + unseen-tracker (already shipped). The Sidebar Activity slot itself is **not** yet defined here — coordinate with whoever implements 0053's chrome.
- `meta/work/0037-glyph-component.md` — already complete (component shipped at `Glyph.tsx`).
- `meta/plans/2026-04-22-meta-visualiser-phase-4-sse-hub-and-notify-watcher.md` — original SSE hub + notify watcher plan; design rationale for the `pre`/`post` comparison and the broadcast channel.
- `meta/plans/2026-04-26-meta-visualiser-phase-8-kanban-write-path.md` — introduced `WriteCoordinator` and `self-cause.ts`; the basis for the AC11 self-cause behaviour.
- `meta/research/2026-05-12-0053-sidebar-nav-and-unseen-tracker.md` — research backing 0053; covers the existing `DocEventsContext` consumption surface.
- `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/sidebar-{light,dark}.png` — visual targets for the Activity feed in the redesigned sidebar.
- `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md` — gap analysis; flags activity feed as a missing surface.

## Related Research

- `meta/research/2026-05-12-0053-sidebar-nav-and-unseen-tracker.md` — sibling research on the Sidebar / DocEventsContext surface 0055 plugs into.
- `meta/research/2026-05-12-0037-glyph-component.md` — Glyph internals (already shipped).
- `meta/research/2026-04-17-meta-visualiser-implementation-context.md` — cross-phase implementation context covering SSE/notify/index design.

## Open Questions

1. **Should `DocInvalid` also gain a `timestamp` field?** The work item is silent on this. Today `DocInvalid` is emitted alongside `DocChanged` (for files that exist but have malformed frontmatter). The Activity feed only consumes `DocChanged`, so leaving `DocInvalid` unchanged is consistent — but it does mean future consumers reading the wire format will see asymmetric envelopes. Pin a decision in the plan.

2. **Subscribe vs rolling-events shape for `DocEventsHandle`?** Two viable extensions (multi-listener `subscribe(...)` vs `recentEvents: SseEvent[]` on the handle). Subscribe is more flexible; rolling-events is simpler but bakes the rolling-window decision into the handle. Default recommendation: `subscribe(...)` returning unsubscribe.

3. **Ring-buffer capacity exactly 50 vs ≥50?** Work item says "at least 50" (Requirements) but ACs assume exactly 50 (AC12, AC13 test `?limit=50` returning 50 and 51-events-evicts-T1). Picking exactly 50 keeps the ACs verifiable; an internal `pub const CAPACITY: usize = 50` clarifies intent.

4. **`formatMtime` reuse or sibling helper?** AC4 explicitly pins `<n>d ago` for all ≥86400s elapsed, but `formatMtime` flips to `<n>w` at 7 days. Recommendation: ship a sibling helper to keep AC4 verifiable, and let `formatMtime` keep serving the doc-list mtime renderings unchanged.

5. **Where does the new ring-buffer + handler live: `server/src/api/activity.rs` or `server/src/activity_feed.rs`?** Both work. `api/activity.rs` is namespaced under `api::` and keeps the route handler co-located with the ring buffer (handlers reach state via `AppState`, so this is fine). The crate-root `activity` module is a different concept (HTTP-activity tracker) and must not be shadowed.
