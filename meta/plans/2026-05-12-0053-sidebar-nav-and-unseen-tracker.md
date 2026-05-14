---
date: "2026-05-12T20:28:19+00:00"
type: plan
skill: create-plan
work-item: "0053"
status: draft
---

# Sidebar Nav with Per-Type Change Indicators — Implementation Plan

## Overview

Replace the current three-flat-group Sidebar (Documents / Views / Meta) with a
single LIBRARY heading partitioned into five lifecycle phases (Define / Discover
/ Build / Ship / Remember). Decorate each item with a Glyph, a count badge
driven by a new `count` field on `GET /api/types`, and an unseen-changes dot
driven by a per-browser tracker fed by SSE `doc-changed` events. Slot a
non-functional search-input container and `/` keybind chip for 0054 to wire
behaviour into.

The implementation is decomposed into five phases that proceed test-first: a
backend `count` extension, an SSE plumbing extension for event subscription,
a localStorage-backed unseen-tracker hook, a `LibraryTypeView` mount-effect
that bumps T, and the Sidebar rewrite that consumes everything above.

## Current State Analysis

### Frontend Sidebar (84 lines, `frontend/src/components/Sidebar/Sidebar.tsx`)

- Partitions `docTypes` into three flat groups via inline filters
  (`Sidebar.tsx:20-21`) and a hard-coded `VIEW_TYPES` array (`Sidebar.tsx:5-8`).
- Renders plain text TanStack Router `<Link>` items with no Glyph, no badge,
  and no dot.
- No search input, no keybind chip, no footer.
- `Sidebar.test.tsx:7-21` carries vestigial mocks for `useServerInfo`,
  `useDocEventsContext`, `useOrigin` — only the second is repurposeable.

### Backend `/api/types` (19 lines, `server/src/api/types.rs`)

- Handler currently returns `describe_types(&state.cfg)` directly — no index
  data is consulted.
- `DocType` struct at `server/src/docs.rs:90-99` is `Serialize`-only with
  `#[serde(rename_all = "camelCase")]` container-level; adding a new field is
  wire-additive.
- `Indexer` (`server/src/indexer.rs:48-76`) holds entries in
  `HashMap<PathBuf, IndexEntry>` (line 58). No per-type bucketing exists;
  `all_by_type` (lines 316-324) clones entries every call. Templates is
  excluded from the index entirely (`indexer.rs:126-129`).

### SSE plumbing (`frontend/src/api/use-doc-events.ts`, 160 lines)

- `dispatchSseEvent` (`use-doc-events.ts:48-78`) self-cause-filters then
  invalidates queries; **no consumer-subscription API exists** on the returned
  `DocEventsHandle` (`use-doc-events.ts:15-19`).
- Factory `makeUseDocEvents(createSource, registry)` (lines 86-160) already
  supports two test-injection seams.
- `onmessage` handler at lines 136-150; self-cause guard at line 139; drag
  deferral at lines 140-143; reconnect handling at lines 118-133.

### `LibraryTypeView` (`frontend/src/routes/library/LibraryTypeView.tsx`)

- Narrows `params.type` to `DocTypeKey | undefined` at lines 52-54.
- Renders `<Outlet />` at line 83 when `params.fileSlug` is set.
- **TanStack Router does not remount on `:type` change** — the same instance
  serves multiple types via prop-driven re-renders. Effects bumping T must
  depend on `[type]`, not `[]`.

### Local-storage state pattern

Canonical reference: `use-font-mode.ts` (plain owning hook, no factory) and
`use-theme.ts` (with factory test seam for `prefersDark` predicate). Both use
`safeGetItem` / `safeSetItem` from `api/safe-storage.ts`. The
`SEEN_DOC_TYPES_STORAGE_KEY` will be the third entry in `storage-keys.ts`.

### Desired End State

- `GET /api/types` returns `DocType` objects each carrying `count: number`
  reflecting the indexed entry count for that type (Templates → `0`).
- The Sidebar renders a LIBRARY heading containing five phase subheadings,
  each populated with its assigned doc types in canonical display order. Each
  item shows a Glyph, the type's label, an integer count badge when `count
  > 0`, and an unseen dot when activity has occurred since the type was last
  viewed.
- A search-input container and `/` keybind hint chip are present in the
  Sidebar chrome (inert).
- An unseen tracker, scoped per-browser via `localStorage` key
  `'ac-seen-doc-types'` (JSON object of `DocTypeKey → epoch-ms`), tracks
  per-type seen-times keyed off external SSE `doc-changed` receipt and
  `LibraryTypeView` list-view mount bumps. The transient unseen state is
  in-memory only (resets on tab reload — accepted trade-off). Self-cause
  `doc-changed` echoes never raise the dot (filtered at the SSE plumbing
  layer); `doc-invalid` events are ignored entirely by the tracker (they
  have no etag and so cannot be filtered upstream).

#### Verification

- `make test` passes (server + frontend).
- `curl /api/types | jq '.types[] | {key, count}'` shows each type's count.
- Manual: navigating between types in the Sidebar clears the dot for the
  visited type. Triggering an external edit (write to a file outside the app)
  raises the dot for the corresponding type.

### Key discoveries

- The `DocType` struct's `#[serde(rename_all = "camelCase")]` means
  `count: usize` serialises as `"count"` with no extra attributes
  (`server/src/docs.rs:90-99`).
- `dispatchSseEvent` already branches on event type — the natural callback
  insertion site is `use-doc-events.ts:136-150`, after the self-cause guard
  at line 139.
- TanStack Router's component reuse across param changes
  (`router.ts:96-106`) means `useMarkDocTypeSeen` must use `[type, markSeen]`
  deps, and its caller in `LibraryTypeView` must additionally key on
  `hasFileSlug` so deep links to a child doc don't fire the effect.
- The Glyph component (work item 0037) is already shipped with all 12
  non-Templates icons and an `isGlyphDocTypeKey` narrowing helper. 0053 is its
  first production consumer.
- `Indexer` skips Templates entirely (`indexer.rs:126-129`); `counts_by_type`
  will yield no entry for Templates, and the handler folds `unwrap_or(0)` so
  the wire value is `0`.

## What We're NOT Doing

- **Search behaviour** — the input + kbd chip DOM lands hidden via the
  `hidden` attribute; 0054 removes the `hidden` flag and wires handlers.
- **Activity feed** — 0055 owns the feed and the `action` / `timestamp`
  additions to `SseEvent`.
- **Server-side seen-state persistence** — local-storage only, per the
  `use-theme` / `use-font-mode` precedent.
- **Templates count from `state.templates.list()`** — Templates is excluded
  from LIBRARY and yields `count = 0` via `unwrap_or(0)`. The trade-off is
  briefly noted in the research; revisit when a non-Sidebar consumer needs
  Templates counts.
- **A shared `Badge` / `Pill` / `Kbd` component library** — 0053 introduces
  Sidebar-local DOM with CSS-module classes for the badge / dot / kbd chip.
  Promote to shared primitives only when a second consumer appears.
- **Promoting `useMarkDocTypeSeen` and `PHASE_DOC_TYPES` to a shared
  cross-skill surface** — keep them frontend-local until a sibling story
  declares the need.
- **`BOOT_SCRIPT_SOURCE` extension** — unseen state has no pre-paint render
  path.
- **Multi-tab `storage`-event propagation** — `markSeen` in tab A does not
  clear the dot in tab B's in-memory `unseenSet` until tab B receives a
  later event for the same type. Accepted trade-off; would require a
  `window.addEventListener('storage', ...)` and additional reconciliation
  logic. Revisit if multi-tab usage becomes common.
- **First-time-user "new to you" dots** — the dot is a *"since last visit"*
  indicator, not a *"new to you"* indicator. A fresh install with empty
  storage shows no dots; first events per type are silently absorbed.
  Discoverability for first-time users falls on the count badges and
  labels. Revisit if first-run UX needs a "fresh install" signal.
- **Persistence of the transient `unseenSet`** — dots clear across tab
  reloads. Accepted trade-off: keeps event ingestion zero-write and avoids
  a second persisted data structure that would need its own corruption
  tolerance. The next event after reload re-raises the dot if appropriate.
- **Collapsible / hide-empty phase subheadings** — all five phase
  subheadings render unconditionally; vertical-space budget for 0055's
  Activity feed assumes this. Revisit when 0055 lands if it competes for
  space.
- **Sidebar entry point for Templates** — Templates is reachable via
  `/library/templates` only; no nav affordance. Discoverability is
  deliberately deprioritised for this story.

## Implementation Approach

Test-driven: each phase begins with a failing test that pins the externally
observable behaviour, then implements the smallest change to turn it green.
Phases progress strictly bottom-up so the rewriting Phase 5 consumes already-
landed primitives.

Phase 1 lands on its own (backend + frontend type-only mirror) and is shippable
in isolation because consumers tolerate absent / zero counts. Phases 2-4 form
a chain (SSE callback → tracker → view bump) and should land in order. Phase
5 consumes all prior work plus the Glyph component already in `main`.

---

## Phase 1: Backend `count` field on `/api/types`

### Overview

Extend the `DocType` JSON shape with a `count: number` field reflecting the
indexed entry count for that type. Implement on the backend without
restructuring `describe_types`, then mirror the type addition into the
frontend `DocType` interface so subsequent phases can consume it.

### Changes required

#### 1. Failing contract test — `server/tests/api_types.rs`

**File**: `skills/visualisation/visualise/server/tests/api_types.rs`
**Changes**: Add per-type `count` assertions matching `seeded_cfg` cardinalities
(1 decision, 1 plan, 1 plan-review, all others 0).

```rust
let decisions = arr.iter().find(|t| t["key"] == "decisions").unwrap();
assert_eq!(decisions["count"].as_u64().unwrap(), 1);
let plans = arr.iter().find(|t| t["key"] == "plans").unwrap();
assert_eq!(plans["count"].as_u64().unwrap(), 1);
let plan_reviews = arr.iter().find(|t| t["key"] == "plan-reviews").unwrap();
assert_eq!(plan_reviews["count"].as_u64().unwrap(), 1);
let work_items = arr.iter().find(|t| t["key"] == "work-items").unwrap();
assert_eq!(work_items["count"].as_u64().unwrap(), 0);
let templates = arr.iter().find(|t| t["key"] == "templates").unwrap();
assert_eq!(templates["count"].as_u64().unwrap(), 0);
```

#### 2. Failing indexer unit test — `server/src/indexer.rs`

**File**: `skills/visualisation/visualise/server/src/indexer.rs` (inside the
existing async `mod tests` block, near `scan_populates_entries_for_configured_types`
at line 998)
**Changes**: Add a `counts_by_type_returns_entry_count_per_configured_type`
test mirroring the existing `build_indexer(tmp).await` + cardinality
assertions, but calling `counts_by_type` instead of repeated `all_by_type`.

```rust
#[tokio::test]
async fn counts_by_type_returns_entry_count_per_configured_type() {
    let tmp = tempfile::tempdir().unwrap();
    let idx = build_indexer(tmp.path()).await;
    let counts = idx.counts_by_type().await;
    assert_eq!(counts.get(&DocTypeKey::Plans).copied().unwrap_or(0), 3);
    assert_eq!(counts.get(&DocTypeKey::Decisions).copied().unwrap_or(0), 1);
    // Templates is excluded from the index entirely (see indexer.rs:126-129);
    // assert absence, not zero, so a future change that accidentally
    // indexes Templates entries with count 0 still fails this test.
    assert!(!counts.contains_key(&DocTypeKey::Templates));
}
```

#### 3. Add `count` field — `server/src/docs.rs:90-99`

**File**: `skills/visualisation/visualise/server/src/docs.rs`
**Changes**: Append `pub count: usize` to `DocType` struct. Update the
single construction site at `docs.rs:107` (inside `describe_types`) to
include `count: 0`. The doc-comment on the new field leads with the wire
guarantee (this is what downstream readers most need to know):

```rust
/// Number of indexed entries of this doc type as of the API call.
///
/// On the JSON wire, this field is always populated by the
/// `api::types::types` handler from the live indexer state. Templates
/// is excluded from the index and so observes `count = 0` via
/// `unwrap_or(0)` in the handler.
///
/// In-process, `describe_types` constructs `DocType` values with
/// `count: 0` as a placeholder — the API handler MUST overwrite this
/// before serialisation. A non-handler consumer of `describe_types`
/// (e.g., a future CLI introspector) would observe the placeholder
/// directly and SHOULD NOT trust this field; consider splitting the
/// type if a second consumer appears.
pub count: usize,
```

```rust
pub struct DocType {
    pub key: DocTypeKey,
    pub label: String,
    pub dir_path: Option<PathBuf>,
    pub in_lifecycle: bool,
    pub in_kanban: bool,
    pub r#virtual: bool,
    pub count: usize,
}
```

#### 4. Add `Indexer::counts_by_type` — `server/src/indexer.rs`

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: Add the new method alongside `all_by_type` (lines 316-324). Single
read-lock pass over `self.entries`, no cloning.

```rust
pub async fn counts_by_type(&self) -> HashMap<DocTypeKey, usize> {
    let mut out: HashMap<DocTypeKey, usize> = HashMap::new();
    for entry in self.entries.read().await.values() {
        *out.entry(entry.r#type).or_insert(0) += 1;
    }
    out
}
```

#### 5. Wire counts into the handler — `server/src/api/types.rs:14-18`

**File**: `skills/visualisation/visualise/server/src/api/types.rs`
**Changes**: Fetch counts and populate each `DocType` before serialisation.
Templates is absent from the map; `unwrap_or(0)` yields `0`.

```rust
pub(crate) async fn types(State(state): State<Arc<AppState>>) -> Json<TypesResponse> {
    let mut types = describe_types(&state.cfg);
    let counts = state.indexer.counts_by_type().await;
    for t in &mut types {
        t.count = counts.get(&t.key).copied().unwrap_or(0);
    }
    Json(TypesResponse { types })
}
```

#### 6. Mirror onto frontend `DocType` interface

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts:26-36`
**Changes**: Add `count?: number` to the interface as **optional**. No
consumer yet; Phase 5 reads it via `t.count !== undefined && t.count > 0`.

The optional declaration is deliberate: the work item's AC2 says "absent
count → no badge". Declaring the field as required would make the rendering
guard purely defensive (a required field is always present in TS) and would
also break the standalone-Phase-1 claim — the existing `mockDocTypes`
literal in `Sidebar.test.tsx:23-28` does not include `count`, and would
fail `tsc` until Phase 5 lands. With `count?: number`, Phase 1 ships cleanly
without touching Sidebar.test.tsx fixtures.

```ts
export interface DocType {
  key: DocTypeKey
  label: string
  dirPath: string | null
  inLifecycle: boolean
  inKanban: boolean
  virtual: boolean
  count?: number
}
```

#### 7. Wire `/api/types` into the SSE invalidation cascade

**File**: `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`
**Changes**: Add `queryClient.invalidateQueries({ queryKey: queryKeys.types() })`
to the existing invalidation cascade triggered by `doc-changed` events (the
section at `use-doc-events.ts:21-34` that today invalidates `docs`,
`docContent`, `lifecycle`, `lifecycle-cluster`, `related`, and `kanban`).
Without this, count badges silently go stale after the first SSE event —
the core value proposition of the badge.

**Failing test first**: add a case to `use-doc-events.test.ts` that fires a
`doc-changed` event for `'decisions'` and asserts
`queryClient.invalidateQueries` was called with `{ queryKey:
queryKeys.types() }`. Place alongside the existing invalidation-cascade
assertions.

Note: this slightly raises the cost of `/api/types` — from once-per-page-
load to once-per-doc-changed-event. With `Indexer::counts_by_type` as a
single read-lock O(N) pass over a typical few-thousand-entry map, the
per-event cost is microseconds and is dominated by the lock-acquisition
itself. If profiling later shows contention with `rescan`/`refresh_one`
write paths, swap the implementation to an incremental `counts` field
maintained inside the existing `entries.write()` critical section.

### Success criteria

#### Automated verification

- [ ] `server/tests/api_types.rs` per-type `count` assertions pass:
      `cargo test -p visualise-server --test api_types`
- [ ] `counts_by_type_returns_entry_count_per_configured_type` passes:
      `cargo test -p visualise-server indexer::tests::counts_by_type`
- [ ] Full server test suite remains green: `make test-server` (or
      `cargo test -p visualise-server`)
- [ ] Frontend type check passes: `make typecheck-frontend` (or
      `npm --prefix skills/visualisation/visualise/frontend run typecheck`)
- [ ] No clippy regressions: `make lint-server` / `cargo clippy`

#### Manual verification

- [ ] `curl http://localhost:8765/api/types | jq '.types[] | {key, count}'`
      shows non-zero counts for types with seeded entries in the dev
      environment and `0` for Templates.

---

## Phase 2: SSE `onEvent` callback in `makeUseDocEvents`

### Overview

Extend `makeUseDocEvents` so consumers can subscribe to per-event signals
without piggy-backing on query invalidation. The callback fires for both
`doc-changed` and `doc-invalid` events, **after** the self-cause guard so
local-mutation `doc-changed` echoes never reach the tracker.

**Reconnect contract (pinned):** the hook accepts a separate
`onReconnect?: () => void` callback that fires once each time the
EventSource transitions from a disconnected state to `'open'` after the
initial connection. There is no `'reset'` sentinel through `onEvent` —
`SseEvent` stays a faithful mirror of the server wire protocol. The
unseen-tracker's `onReconnect` handler is a no-op (Phase 3); a future
consumer can opt in.

**Callback stability (pinned):** consumers MAY pass non-memoised callbacks.
The hook internally stores them in a `useRef` updated on every render, so
the long-lived `onmessage` closure always reaches the latest callback
without re-creating the EventSource. The existing `useEffect` dependency
array (`[queryClient, createSource, registry]`) is **not** extended — the
options object is read via refs, not deps.

### Changes required

#### 1. Failing tests — `use-doc-events.test.ts`

**File**: `skills/visualisation/visualise/frontend/src/api/use-doc-events.test.ts`
**Changes**: Add a new `describe` block exercising the `onEvent` callback via
the existing `FakeEventSource` harness.

Test cases to add:
- **`onEvent` fires for `doc-changed`**: factory built with `(createSource, registry, { onEvent })`; `fakes[0].onmessage?.(new MessageEvent('message', { data: JSON.stringify({ type: 'doc-changed', docType: 'decisions', path: '/x', etag: 'abc' }) }))`; assert `onEvent` called once with the event.
- **`onEvent` fires for `doc-invalid`**: same setup, push a `doc-invalid` event; assert call.
- **`onEvent` does NOT fire for self-cause `doc-changed` echo**: pre-register the etag in the injected `registry`, fire matching `doc-changed`; assert `onEvent` was not called. (Note: `doc-invalid` has no `etag` so the self-cause guard cannot filter it — the unseen tracker handles this concern in Phase 3 by ignoring `doc-invalid` events entirely.)
- **`onEvent` forwards events with unknown `docType` verbatim**: fire `doc-changed` with `docType: 'made-up-type'`; assert `onEvent` is called **exactly once** with the event as-is. The hook does not validate `docType` — consumers filter. Call-count assertion guards against a future regression that double-fires for unknown types.
- **`onReconnect` fires once on reconnect**: drive the source into reconnect (close + retry via `vi.advanceTimersByTime`); assert `onReconnect` was called exactly once. Assert `onEvent` is NOT called with any synthesised reset/sentinel event.
- **Callback stability — EventSource is constructed once across re-renders with new options**: render the hook with `{ onEvent: cb1 }`, re-render with a freshly-allocated `{ onEvent: cb2 }`, fire one event; assert (a) `createSource` was called exactly once total, (b) `cb2` (not `cb1`) received the event. This pins the ref-based callback storage and prevents EventSource thrash.

#### 2. Extend `makeUseDocEvents` signature — `use-doc-events.ts:86-160`

**File**: `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`
**Changes**: The `options` object is accepted at the **inner hook**, not
the factory — this matches the `makeUseTheme(prefersDark)` precedent where
the factory binds infrastructure (test seams) and the call site supplies
per-render configuration. The factory signature is unchanged
(`makeUseDocEvents(createSource, registry?)`).

```ts
export interface UseDocEventsOptions {
  onEvent?: (event: SseEvent) => void
  onReconnect?: () => void
}

// Owning-hook pattern split: the factory binds infrastructure + test
// seams (createSource, registry), and the inner hook accepts per-render
// consumer callbacks via `options`. This differs from `makeUseTheme`,
// whose inner hook takes no runtime parameters — the divergence is
// intentional: `useDocEvents` has multiple potential subscribers
// (unseen tracker now, activity feed in 0055), each with its own
// callbacks; binding them all at factory time would force a single
// global subscriber set.
export function makeUseDocEvents(
  createSource: EventSourceFactory,
  registry: SelfCauseRegistry = defaultSelfCauseRegistry,
) {
  return function useDocEvents(
    options?: UseDocEventsOptions,
  ): DocEventsHandle {
    // Store callbacks in refs so the long-lived onmessage / reconnect
    // closures always read the latest values WITHOUT extending the
    // useEffect deps array. This keeps the EventSource singleton across
    // re-renders even when consumers pass freshly-allocated callbacks.
    const onEventRef = useRef(options?.onEvent)
    const onReconnectRef = useRef(options?.onReconnect)
    onEventRef.current = options?.onEvent
    onReconnectRef.current = options?.onReconnect

    // ... existing setup ...

    useEffect(() => {
      // ... existing EventSource construction ...

      source.onmessage = (msg) => {
        // ... existing parse + self-cause guard at line 139 ...
        onEventRef.current?.(event)                    // <- new
        // ... existing drag-deferral + invalidation cascade ...
      }

      // In the existing reconnect handler (lines 118-133), invoke the
      // consumer callback at the END of the block — AFTER `registry.reset()`,
      // AFTER the pending-invalidation drain, and AFTER the predicate-based
      // `invalidateQueries`. This ordering guarantees the consumer observes
      // a fully-recovered state (no stale self-cause entries, all queries
      // marked stale). The unseen tracker (Phase 3) is a no-op here, but
      // future consumers may depend on the ordering.
      onReconnectRef.current?.()                        // <- new (at end of existing reconnect block)

      return cleanup
    }, [queryClient, createSource, registry]) // <- UNCHANGED deps
  }
}
```

#### 3. Plumb the production hook — `use-doc-events.ts`

**File**: `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`
**Changes**: The exported production `useDocEvents` retains its existing
shape: `export const useDocEvents = makeUseDocEvents(defaultCreateSource)`.
Because the inner hook now accepts `options?`, call sites like
`useDocEvents({ onEvent: unseen.onEvent })` work without re-plumbing.
Production identity, React DevTools display name, and module-load-once
factory binding are all preserved.

### Success criteria

#### Automated verification

- [ ] New `use-doc-events.test.ts` cases pass:
      `npm --prefix skills/visualisation/visualise/frontend test -- use-doc-events`
- [ ] No regression in the rest of the frontend suite:
      `make test-frontend` (or `npm --prefix … test`)
- [ ] Type check passes: `make typecheck-frontend`

#### Manual verification

- [ ] Open the running app with DevTools → Network → "Manifest" filter
      on. Trigger an external edit (e.g. `touch
      meta/decisions/ADR-9999.md`). Confirm the SSE handler runs and the
      `/api/types` request is re-issued in the Network tab, proving the
      new invalidation wiring without needing Phase 5's UI.
- [ ] Reconnect verification (no UI required): with DevTools console
      open, disconnect (toggle browser offline), reconnect; confirm a
      single console log / breakpoint hit in the reconnect path — no
      synthesised `'reset'` event in the SSE message stream.

---

## Phase 3: `useUnseenDocTypes` hook + Context

### Overview

Introduce a per-browser tracker that records the wall-clock time T at which
each doc type was last seen, raises a "has unseen changes" flag when an
**external** SSE `doc-changed` event for that type arrives whose receipt
time is strictly later than T, and exposes a `markSeen(type)` mutator.

**Persistence:** `localStorage` under key `'ac-seen-doc-types'` storing a
JSON object whose values are millisecond-epoch numbers (`Date.now()`
directly — no ISO-8601 parse round-trip; aligns with how `mtimeMs` is
already represented on `IndexEntry`).

**Storage write policy:** `markSeen` is the **only** code path that writes
to `localStorage`. The transient `unseenSet` is in-memory only and resets
to empty on a fresh mount — accepted trade-off so SSE event bursts (e.g.,
a `jj checkout` touching dozens of files) do not produce N synchronous
`localStorage.setItem` calls. Dots clear on tab reload; this is documented
behaviour.

**API shape:** the handle exposes `unseenSet: ReadonlySet<DocTypeKey>` as
the **primary and only** read surface (a stable identity that can be used
as a memo dep and supports `React.memo` boundaries around per-item
components). Consumers read membership via `unseenSet.has(type)` directly
— no `unseen(type)` wrapper, to avoid two ways of asking the same
question. `markSeen(type)` is the mutator; `onEvent` and `onReconnect`
are wired into `useDocEvents` from `RootLayout`.

**Event filtering:** `useUnseenDocTypes.onEvent` ignores `doc-invalid`
events entirely. The self-cause guard at the SSE plumbing layer (Phase 2)
cannot filter `doc-invalid` echoes — `SseDocInvalidEvent` has no `etag`
field — so a user's own save triggering a validation error would otherwise
raise a spurious dot on the type they just edited. The tracker treats
`doc-invalid` as "operational signal, not new content".

**Reconnect:** `onReconnect` is a no-op. Real changes that occurred during
a disconnect are reflected by the replayed `doc-changed` events on
reconnect, which are correctly classified as activity. (Manual verification
step 8 is rewritten accordingly.)

### Changes required

#### 1. Add storage key — `storage-keys.ts`

**File**: `skills/visualisation/visualise/frontend/src/api/storage-keys.ts`
**Changes**: Append `export const SEEN_DOC_TYPES_STORAGE_KEY = 'ac-seen-doc-types'`. Leave `BOOT_SCRIPT_SOURCE` untouched.

#### 2. Failing hook tests — `use-unseen-doc-types.test.ts`

**File**: `skills/visualisation/visualise/frontend/src/api/use-unseen-doc-types.test.ts` (new)
**Changes**: Mirror the test scaffolding from `use-font-mode.test.ts` (real
jsdom `localStorage`, `resetDom()` in before/afterEach). Cover:

- **Initial render with empty storage**: `result.current.unseenSet.size === 0`. No write to `localStorage` (verify `localStorage['ac-seen-doc-types']` is null).
- **First `doc-changed` event with no stored T → no dot, no write**: feed a `doc-changed` event for `work-items` via `onEvent`; assert `unseenSet` is empty (first event for never-visited type is silently absorbed); assert `localStorage['ac-seen-doc-types']` is still null (no write on event ingestion).
- **Event after `markSeen` and time advance → dot raised**: call `markSeen('work-items')` at t=1000, advance time to t=2000 via `vi.setSystemTime`, fire a `doc-changed` event; assert `unseenSet.has('work-items') === true`.
- **Equal-T → no dot (strict gt)**: call `markSeen('work-items')` at t=1000, fire event at t=1000; assert `unseenSet.has('work-items') === false`.
- **`markSeen(type)` bumps T, clears the dot, and writes once**: with the dot already raised, call `markSeen('work-items')`; assert (a) `unseenSet` no longer contains `'work-items'`, (b) stored value is the current `Date.now()` as a **number** (not a string), (c) `safeSetItem` was invoked exactly once.
- **`onEvent` does not write to storage under any condition**: spy on `safeSetItem`; fire 50 `doc-changed` events across multiple types; assert `safeSetItem` was called 0 times.
- **`doc-invalid` events are ignored**: pre-`markSeen('work-items')` at t=1000, advance to t=2000, fire `doc-invalid` event for `'work-items'`; assert `unseenSet.has('work-items') === false`. This pins the doc-invalid-bypass fix from review #1.
- **`onReconnect` is a no-op**: pre-`markSeen('work-items')` at t=1000, advance to t=2000, call `result.current.onReconnect?.()`; assert `unseenSet` unchanged and no write occurred.
- **Persistence round-trip of `markSeen` values**: call `markSeen('decisions')` at t=1000, unmount; assert directly that `JSON.parse(localStorage.getItem(SEEN_DOC_TYPES_STORAGE_KEY))['decisions'] === 1000` (unambiguous wire-format check). Remount via fresh `renderHook`; verify via strict-gt boundary: event at t=999 → no dot, event at t=1001 → dot. Both layers are pinned — the storage shape AND the comparison semantics — so a future refactor of either fails its own test rather than silently passing the other.
- **Transient state does NOT survive remount**: raise a dot via the event sequence above, then unmount and remount; assert `unseenSet.size === 0` after remount (transient state is intentionally not persisted; documented behaviour).
- **Malformed storage — `"not json"`**: pre-populate the key with `"not json"`; mount the hook; assert no throw and `unseenSet.size === 0`.
- **Malformed storage — JSON array instead of object**: pre-populate with `"[1,2,3]"`; assert treated as empty, no throw.
- **Malformed storage — non-numeric / NaN values**: pre-populate with `'{"work-items":"banana"}'`; mount; fire any event for `'work-items'`; assert no throw and `unseenSet.has('work-items') === false` (NaN comparisons must not silently lock the dot off — the parser drops the bad entry and treats `'work-items'` as never-seen, so first-event-seeds-T absorbs correctly).
- **Malformed storage — unknown DocTypeKey filtered**: pre-populate with `'{"made-up-type":1000,"work-items":1000}'`; assert the valid `'work-items'` entry survives (verified via strict-gt boundary) and no error is thrown about the unknown key.
- **`safeSetItem` failure (private mode / quota)**: spy on `Storage.prototype.setItem` to throw; call `markSeen('work-items')`; assert no throw propagates AND in-memory `unseenSet` was still cleared (in-memory state must update even when persistence fails — matches `use-font-mode.test.ts:59-69` precedent).
- **`unseenSet` identity stability across repeat events**: call `markSeen('decisions')` at t=1000; advance to t=2000; fire a `doc-changed` for `'decisions'` (raises the dot); capture `result.current.unseenSet` as `setA`. Fire a SECOND `doc-changed` for `'decisions'` (already-unseen — should be a no-op via the reducer's `if (prev.has(type)) return prev` early-return); capture `result.current.unseenSet` as `setB`; assert `Object.is(setA, setB)`. Pins the early-return path explicitly — a bare double-`rerender()` would pass trivially against a buggy implementation that returns a fresh Set on every event.
- **Reactivity through Context**: render a tiny child component wrapped in `<UnseenDocTypesContext.Provider value={hookHandle}>` that reads `unseenSet` and renders `'yes'`/`'no'`; fire an event that raises a dot; assert the child re-renders and shows the new value. Pins the contract that flipping membership propagates through context.
- **`markSeen` → synchronous `onEvent` sees the updated T**: at t=1000, call `markSeen('decisions')` then synchronously (same JS turn) invoke `result.current.onEvent` with a `doc-changed` for `'decisions'` at t=1000. Assert `unseenSet.has('decisions') === false` — proves that `onEvent` reads the freshly-written `seenRef.current[type]` (strict-gt against equal-T resolves to 'seen') rather than a stale snapshot. A future refactor that moves `markSeen` into a `useEffect` or state-updater would break the synchronous-visibility guarantee and fail this test.
- **Reconnect followed by replayed `doc-changed` raises the dot**: call `markSeen('decisions')` at t=1000; advance time to t=2000; call `result.current.onReconnect?.()` (no-op); fire a `doc-changed` event for `'decisions'` (the replayed event after reconnect); assert `unseenSet.has('decisions') === true`. Pins that `onReconnect` does NOT clear `seenRef` — a future "reset state on reconnect" regression would silently break the documented "real changes during disconnect produce dots" behaviour without this composed-sequence test.

#### 3. Implement `useUnseenDocTypes` — `use-unseen-doc-types.ts` (new)

**File**: `skills/visualisation/visualise/frontend/src/api/use-unseen-doc-types.ts` (new)
**Changes**: Mirror `use-font-mode.ts` shape (plain owning function, no
factory — `Date.now()` is sufficient and tests use `vi.useFakeTimers()`).

```ts
type SeenMap = Partial<Record<DocTypeKey, number>>  // epoch ms

export interface UnseenDocTypesHandle {
  unseenSet: ReadonlySet<DocTypeKey>          // primary read surface
  markSeen: (type: DocTypeKey) => void
  onEvent: (event: SseEvent) => void
  onReconnect: () => void
}

export function useUnseenDocTypes(): UnseenDocTypesHandle {
  // Persisted: per-type last-seen epoch ms. Holds the canonical T values
  // that survive across mounts. Updated only by markSeen.
  const seenRef = useRef<SeenMap>(parseStored())

  // Transient: which types have unseen activity right now. Empty on mount,
  // never persisted — N events do not produce N localStorage writes.
  const [unseenSet, setUnseenSet] = useState<Set<DocTypeKey>>(
    () => new Set(),
  )

  const onEvent = useCallback((event: SseEvent) => {
    // doc-invalid events bypass the self-cause guard (no etag) — ignore
    // them entirely; they signal an operational issue, not new content.
    if (event.type !== 'doc-changed') return
    if (!isDocTypeKey(event.docType)) return

    const type = event.docType as DocTypeKey
    const stored = seenRef.current[type]

    // First event for a never-visited type is silently absorbed. The user
    // has no T to compare against; we choose "since last visit" semantics
    // (the dot is not a 'new to you' indicator). Documented in NOT Doing.
    if (stored === undefined) return

    // Strict greater-than: same-millisecond markSeen+event resolves in
    // favour of 'seen' (the moment the user was last looking).
    if (Date.now() > stored) {
      setUnseenSet((prev) => {
        if (prev.has(type)) return prev          // stable identity
        const next = new Set(prev)
        next.add(type)
        return next
      })
    }
  }, [])

  const markSeen = useCallback((type: DocTypeKey) => {
    seenRef.current = { ...seenRef.current, [type]: Date.now() }
    safeSetItem(SEEN_DOC_TYPES_STORAGE_KEY, JSON.stringify(seenRef.current))
    setUnseenSet((prev) => {
      if (!prev.has(type)) return prev           // stable identity
      const next = new Set(prev)
      next.delete(type)
      return next
    })
  }, [])

  const onReconnect = useCallback(() => {
    // No-op. Real changes during a disconnect are reflected by replayed
    // doc-changed events, which onEvent classifies correctly.
  }, [])

  // No useMemo wrapper — markSeen / onEvent / onReconnect are stable
  // useCallback identities and unseenSet is the only state-derived field.
  // The handle's identity changes precisely when unseenSet changes
  // membership, which is the desired reactivity signal through context.
  return { unseenSet, markSeen, onEvent, onReconnect }
}
```

`parseStored()` reads `SEEN_DOC_TYPES_STORAGE_KEY` via `safeGetItem`, does
a `JSON.parse` inside a `try/catch`, asserts the parsed value is a non-null
non-array object, then iterates entries filtering out: keys not in
`DOC_TYPE_KEYS`, and values that are not finite numbers. Returns `{}` on
any failure. Pure function — exported only for testing.

#### 4. Context + consumer hooks

**File**: same module
**Changes**: Add `UnseenDocTypesContext` with a stable noop default handle:

```ts
const noopHandle: UnseenDocTypesHandle = {
  unseenSet: new Set(),
  markSeen: () => {},
  onEvent: () => {},
  onReconnect: () => {},
}
export const UnseenDocTypesContext = createContext(noopHandle)
export const useUnseenDocTypesContext = () => useContext(UnseenDocTypesContext)
```

Also export a `useMarkDocTypeSeen(type: DocTypeKey | undefined)` consumer
helper (renamed from `useMarkSeen` per standards review: the `use<Thing>Context` /
`use<Action><Thing>` naming convention makes it clear the hook depends on
the provider). The helper wraps:

```ts
export function useMarkDocTypeSeen(type: DocTypeKey | undefined): void {
  const { markSeen } = useUnseenDocTypesContext()
  useEffect(() => {
    if (type) markSeen(type)
  }, [type, markSeen])
}
```

Phase 4 consumes this helper from `LibraryTypeView`. The helper is a
CONSUMER hook — requires `<UnseenDocTypesContext.Provider>` ancestor.

#### 5. Wire into `RootLayout` — `RootLayout.tsx:13-25`

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx`
**Changes**: Add `const unseen = useUnseenDocTypes()` next to the existing
owning hooks (`RootLayout.tsx:13-15`). Pass `onEvent` and `onReconnect`
into `useDocEvents`. Wrap children in
`<UnseenDocTypesContext.Provider value={unseen}>` inside the existing
provider chain.

```tsx
const unseen = useUnseenDocTypes()
const docEvents = useDocEvents({
  onEvent: unseen.onEvent,
  onReconnect: unseen.onReconnect,
})
// ...
<ThemeContext.Provider value={theme}>
  <FontModeContext.Provider value={fontMode}>
    <DocEventsContext.Provider value={docEvents}>
      <UnseenDocTypesContext.Provider value={unseen}>
        {/* tree */}
      </UnseenDocTypesContext.Provider>
    </DocEventsContext.Provider>
  </FontModeContext.Provider>
</ThemeContext.Provider>
```

Callback stability: the `useDocEvents` ref pattern (Phase 2) reads the
latest `onEvent` / `onReconnect` on every event without re-creating the
EventSource, so passing a fresh `{ onEvent, onReconnect }` object literal
here on every render is safe.

### Success criteria

#### Automated verification

- [ ] All `use-unseen-doc-types.test.ts` cases pass:
      `npm --prefix … test -- use-unseen-doc-types`
- [ ] No regressions across the frontend suite: `make test-frontend`
- [ ] Type check passes: `make typecheck-frontend`
- [ ] Lint passes: `make lint-frontend`

#### Manual verification

- [ ] In DevTools → Application → Local Storage, observe the
      `ac-seen-doc-types` key is **absent** after Phase 3 lands but
      before navigating anywhere (transient state is not persisted on
      mount). Phase 4 will populate it on type navigation.

---

## Phase 4: `useMarkDocTypeSeen` wired into `LibraryTypeView`

### Overview

Bump T to the current time whenever the user lands on the **list view** of
a type (`:type` set, no `:fileSlug`). Deep links to a child document
(`/library/<type>/<slug>`) do NOT bump T — the user hasn't actually seen
the list and any dot on the parent type should persist until they visit
the list. Because TanStack Router reuses the component across `:type`
changes, the effect must depend on `[type, hasFileSlug]`, not `[]`.

### Changes required

#### 1. Failing test — `LibraryTypeView.test.tsx`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.test.tsx`
(extend if exists, otherwise create following the `Sidebar.test.tsx`
mock-and-render shape).
**Changes**: Pass a mock `UnseenDocTypesHandle` via a real
`<UnseenDocTypesContext.Provider value={mockHandle}>` so the test observes
`markSeen` calls directly — no production-only-for-tests seam, no
`propType` prop. Cases:

- **List view bumps T**: render via `renderWithRouterAt(<LibraryTypeView />, '/library/work-items')`; assert `markSeen` called once with `'work-items'`.
- **`:type` change re-fires**: re-render the same router at `/library/research`; assert `markSeen` called with `'research'` (proves `[type, hasFileSlug]` deps).
- **Child-doc URL does NOT bump T**: render at `/library/work-items/some-slug`; assert `markSeen` was **not** called. This pins the "deep link doesn't silently clear the dot" semantics — a user arriving at a child doc has not seen the list and the dot on the parent type should persist.
- **Navigating from child-doc back to list view bumps T**: render first at `/library/work-items/some-slug` (no call), then at `/library/work-items` (one call with `'work-items'`); proves the `hasFileSlug` boundary triggers correctly.
- **Navigating between two fileSlugs of the same type does NOT re-fire**: render at `/library/work-items/a`, then `/library/work-items/b`; assert `markSeen` was called 0 times across both renders (both have `hasFileSlug === true`, so the dep tuple is stable).
- **Invalid type via router does not bump T**: render at `/library/not-a-real-type`; assert `markSeen` not called. Drive through the actual router — there is no `propType` prop in production.
- **StrictMode double-effect is harmless and idempotent**: wrap the render in `<React.StrictMode>` at `/library/work-items`; assert `markSeen` was called **exactly twice** with `'work-items'` (StrictMode double-invokes the effect) AND that the stored T equals the most recent `Date.now()`. The companion non-StrictMode case in this file already asserts a single call — together they pin both "the dep tuple fires once per mount" AND "double-invoke is idempotent".

The test will need an extension to `test/router-helpers.tsx` so the test
router tree registers `/library/$type` (currently only
`/library/$type/$fileSlug` is registered at line 14 of the helper). Add
the parent route alongside it.

#### 2. Insert `useMarkDocTypeSeen` call — `LibraryTypeView.tsx`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx`
**Changes**: Insert after the `type` narrowing block and before the first
conditional return. The hook must run on every render, so it sits with the
other top-of-component hooks. Pass `undefined` when on a child-doc path so
the helper no-ops — `useMarkDocTypeSeen` reads its argument as the effect
dep, which means swapping it between a real type and `undefined` correctly
flips the effect on/off.

```tsx
const type: DocTypeKey | undefined =
  rawType && isDocTypeKey(rawType) ? rawType : undefined
const hasFileSlug = Boolean(params.fileSlug)

// Pass `undefined` to opt out of the mark-seen effect on child-doc
// paths. Deep links to /library/<type>/<slug> must NOT silently clear
// the parent type's unseen dot — the user hasn't actually seen the
// list. `useMarkDocTypeSeen` treats `undefined` as "do nothing".
const typeToMarkSeen = hasFileSlug ? undefined : type
useMarkDocTypeSeen(typeToMarkSeen)
```

`useMarkDocTypeSeen` is the consumer hook exported in Phase 3 and
internally runs `useEffect(() => { if (type) markSeen(type) }, [type, markSeen])`.

### Success criteria

#### Automated verification

- [ ] New `LibraryTypeView.test.tsx` cases pass:
      `npm --prefix … test -- LibraryTypeView`
- [ ] No regression in router-helpers consumers:
      `npm --prefix … test`
- [ ] Type check passes: `make typecheck-frontend`

#### Manual verification

- [ ] In DevTools → Application → Local Storage, observe `ac-seen-doc-types`
      gain a numeric entry each time the user lands on a list view
      (`/library/<type>` with no slug). Visit
      `/library/decisions/0001` directly — the `decisions` entry must
      **not** update (deep links to a child doc do not bump T).
- [ ] Navigate from `/library/decisions/0001` to `/library/decisions` —
      the `decisions` entry is now updated to the current `Date.now()`.

---

## Phase 5: Sidebar restructure with Glyph, badge, dot, search slot, kbd chip

### Overview

Rewrite the Sidebar to render a single LIBRARY heading partitioned by
lifecycle phase. Each nav item carries a Glyph, label, count badge, and
unseen-changes dot. Add a non-functional search input container and a `/`
keybind hint chip. Drop the three flat sections, drop `VIEW_TYPES`, drop the
`virtual`-based partition.

### Changes required

#### 1. Add `PHASE_DOC_TYPES` — `api/types.ts:134-169`

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`
**Changes**: Add a new constant alongside `LIFECYCLE_PIPELINE_STEPS` codifying
the canonical phase-to-doc-type mapping from work item 0036.

```ts
/**
 * Canonical mapping from lifecycle phase to doc types, used by the
 * Sidebar to partition LIBRARY into Define / Discover / Build / Ship /
 * Remember. The nested shape (`phase → docTypes[]`) expresses the
 * one-to-many phase→types relation, unlike `LIFECYCLE_PIPELINE_STEPS`
 * which is one-to-one (each step has a single docType + placeholder).
 * The canonical taxonomy is owned by work item 0036; promote to a
 * server-side definition only when a second consumer appears (see
 * "What We're NOT Doing"). Templates is intentionally omitted.
 */
export const PHASE_DOC_TYPES = [
  {
    phase: 'define',
    label: 'Define',
    docTypes: ['work-items', 'work-item-reviews'] as const,
  },
  {
    phase: 'discover',
    label: 'Discover',
    docTypes: ['design-inventories', 'design-gaps', 'research'] as const,
  },
  {
    phase: 'build',
    label: 'Build',
    docTypes: ['plans', 'plan-reviews', 'validations'] as const,
  },
  {
    phase: 'ship',
    label: 'Ship',
    docTypes: ['prs', 'pr-reviews'] as const,
  },
  {
    phase: 'remember',
    label: 'Remember',
    docTypes: ['decisions', 'notes'] as const,
  },
] as const
```

Twelve doc types total; Templates omitted by design.

#### 2. Failing tests — `Sidebar.test.tsx`

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx`
**Changes**: Rewrite the suite for the new shape. Drop the vestigial
`useServerInfo` / `useOrigin` mocks; keep `useDocEventsContext` mock; add a
real `<UnseenDocTypesContext.Provider value={mockHandle}>` wrapper. The
mock handle exposes `unseenSet: ReadonlySet<DocTypeKey>` whose membership
the test controls per-case (e.g., `new Set(['research'])` for "dot on for
research"); `markSeen` / `onEvent` / `onReconnect` are `vi.fn()` stubs.

Cases:
- **Renders LIBRARY heading**: `await screen.findByText('LIBRARY')`.
- **Renders five phase subheadings in canonical order**: assert "Define", "Discover", "Build", "Ship", "Remember" appear in DOM order.
- **Renders each phase's doc types in canonical display order**: for each phase, assert the labels appear in `PHASE_DOC_TYPES[phase].docTypes` order.
- **Templates does not appear**: `expect(screen.queryByText('Templates')).toBeNull()`.
- **Glyph rendered with correct `docType` prop**: render the real `Glyph` component (already shipped — no mock) and assert its accessible name appears per item.
- **Count badge present when `count > 0`**: mock `DocType` fixture has e.g. `decisions: { count: 12 }`; assert `screen.getByText('12')` is within the decisions nav item.
- **Count badge absent when `count === 0`**: assert badge element not present for zero-count types.
- **Count badge absent when `count` is missing (undefined)**: feed a `DocType` payload without `count` (the TS interface declares it optional); assert no badge.
- **Unseen dot present when context flags the type**: configure the mock context's `unseenSet` to contain `'research'`; assert dot element present in the research item.
- **Link `aria-label` reflects unseen state**: when `unseenSet.has('research')`, the research link's `aria-label` equals `'Research (unseen changes)'`; otherwise it equals the bare label. This is the channel screen readers actually pick up.
- **No dot when context's `unseenSet` does not contain the key**: default case (empty `unseenSet`); assert dot absent, link `aria-label` is the bare label, and the link has no `title` attribute (sighted mouse hover shows nothing).
- **Sighted tooltip via `title`**: when `unseenSet.has('research')`, assert the research link's `title` equals `'Unseen changes since your last visit'`. Pins the mouse-hover affordance contract that mirrors the AT channel.
- **Dot and badge co-exist for `count > 0 && unseenSet.has(key)`**: render `decisions` with `count: 12` and `'decisions'` in `unseenSet`; assert both the badge text `12` and the dot element are present in the same nav item, and that the dot is rendered before the badge in DOM order (dot adjacent to label, badge right-aligned).
- **Search row is rendered but hidden from a11y tree and tab order**: assert the input exists in the DOM (`container.querySelector('input[type="search"]')` is non-null) AND `screen.queryByRole('searchbox')` is `null` (because `hidden` removes it from the accessibility tree). This pins the "implemented but hidden" contract for 0054 to flip later.
- **`/` kbd chip exists in DOM but not in a11y tree**: `container.querySelector('kbd')?.textContent === '/'` AND `screen.queryByText('/')` is `null` (hidden ancestor).
- **Active state for `/library/<type>`**: render via `renderWithRouterAt(<Sidebar … />, '/library/work-items')`; assert the work-items nav item carries an active class. (Requires extending `test/router-helpers.tsx` to register `/library/$type`.)
- **Active state for child-doc URL `/library/<type>/<slug>`**: render at `/library/work-items/0099`; assert work-items IS active (proves the `pathname === X || startsWith(X + '/')` boundary).
- **Active state does NOT collide across prefix-sharing keys**: render at `/library/plan-reviews`; assert the `plans` nav item is **not** active (regression test for the `plans` ↔ `plan-reviews`, `prs` ↔ `pr-reviews`, `work-items` ↔ `work-item-reviews`, `design-gaps` ↔ `design-inventories` collision class).
- **Empty docTypes (loading state)**: render with `docTypes={[]}`; assert the LIBRARY heading and all five phase subheadings still render, every nav `<ul>` is empty, and no errors are thrown.
- **DocTypeKey listed in `PHASE_DOC_TYPES` but absent from server payload**: spy on `console.warn`; render `docTypes` missing the `plans` entry; assert (a) the plans phase still renders its heading, (b) the Plans nav item is skipped, AND (c) `console.warn` was called once with a message mentioning `'plans'` (pins the dev-warn diagnostic).
- **DocTypeKey not in `PHASE_DOC_TYPES`**: render `docTypes` including a hypothetical `key: 'orphan-type'`; assert the orphan does not appear anywhere in the sidebar (Templates is the canonical instance, this case generalises it).
- **Count badge updates after SSE invalidation (end-to-end)**: mount
  `<Sidebar>` inside a real `<QueryClientProvider>` whose `queryFn` for
  `queryKeys.types()` is a controllable mock. Seed the first response so
  `decisions` returns `count: 1`. Wait for the badge to render `"1"`. Then
  swap the mock to return `count: 2`, fire a `doc-changed` event through
  `useDocEvents` (or directly invoke
  `queryClient.invalidateQueries({ queryKey: queryKeys.types() })`), and
  assert the badge transitions to `"2"`. This pins the full chain — SSE
  event → invalidation → refetch → cache update → selector → render — in
  a single test, closing the gap that unit tests at the invalidation-call
  level cannot catch (a regression where the invalidation fires but the
  new count never reaches the rendered badge).

#### 3. Rewrite `Sidebar.tsx`

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`
**Changes**: Replace the three-section render with a phase-grouped render.

Skeleton:

```tsx
import { PHASE_DOC_TYPES, type DocType, type DocTypeKey } from '../../api/types'
import { Glyph, isGlyphDocTypeKey } from '../Glyph/Glyph'
import { useUnseenDocTypesContext } from '../../api/use-unseen-doc-types'

export function Sidebar({ docTypes }: { docTypes: DocType[] }) {
  const pathname = useRouterState({ select: s => s.location.pathname })
  const { unseenSet } = useUnseenDocTypesContext()
  const byKey = useMemo(() => new Map(docTypes.map(t => [t.key, t])), [docTypes])

  return (
    <aside className={styles.sidebar}>
      {/*
        Search row is implemented but hidden via the HTML `hidden`
        attribute until work item 0054 wires behaviour. `hidden` applies
        the UA-default `display: none`, removes the element from the
        accessibility tree, and removes it from the tab order — so
        sighted and AT users see no affordance for the unwired feature
        (avoiding "coming soon" confusion). 0054 only needs to remove
        the `hidden` flag and bind handlers; the DOM, classes, and
        tokens are already in place.

        This is the only `hidden`-attribute usage in the frontend; the
        pattern is intentional, not unprecedented elsewhere. See the
        "Search behaviour" entry in the plan's "What We're NOT Doing"
        section for the deferral rationale.
      */}
      <div className={styles.searchRow} hidden>
        <input
          type="search"
          aria-label="Search"
          className={styles.searchInput}
          tabIndex={-1}
        />
        <kbd className={styles.kbd}>/</kbd>
      </div>

      <nav aria-labelledby="library-heading">
        <h2 id="library-heading" className={styles.libraryHeading}>LIBRARY</h2>
        {PHASE_DOC_TYPES.map(phase => (
          <section key={phase.phase} className={styles.phase}>
            <h3 className={styles.phaseHeading}>{phase.label}</h3>
            <ul className={styles.list}>
              {phase.docTypes.map(key => {
                const t = byKey.get(key)
                if (!t) {
                  if (import.meta.env.DEV) {
                    // PHASE_DOC_TYPES references a key the server payload
                    // did not include — likely a typo or a config drift.
                    // Loud in dev, silent in prod (the user still gets a
                    // working sidebar minus the missing item).
                    console.warn(
                      `[Sidebar] PHASE_DOC_TYPES key '${key}' missing from /api/types payload — nav item will not render.`,
                    )
                  }
                  return null
                }
                const active =
                  pathname === `/library/${key}` ||
                  pathname.startsWith(`/library/${key}/`)
                const hasUnseen = unseenSet.has(key)
                const linkLabel = hasUnseen
                  ? `${t.label} (unseen changes)`
                  : t.label
                return (
                  <li key={key}>
                    <Link
                      to="/library/$type"
                      params={{ type: key }}
                      aria-label={linkLabel}
                      // Mirror the unseen state into `title` so sighted
                      // mouse users get a hover tooltip matching what AT
                      // users hear via aria-label.
                      title={hasUnseen ? 'Unseen changes since your last visit' : undefined}
                      className={`${styles.link} ${active ? styles.active : ''}`}
                    >
                      {isGlyphDocTypeKey(key) && (
                        <Glyph docType={key} size={16} />
                      )}
                      <span className={styles.label}>{t.label}</span>
                      {hasUnseen && (
                        <span className={styles.dot} aria-hidden="true" />
                      )}
                      {t.count !== undefined && t.count > 0 && (
                        <span className={styles.badge}>{t.count}</span>
                      )}
                    </Link>
                  </li>
                )
              })}
            </ul>
          </section>
        ))}
      </nav>
    </aside>
  )
}
```

#### 4. Update `Sidebar.module.css`

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.module.css`
**Changes**: Drop unused `.section` / `.sectionHeading` / `.muted` classes if
they exist only for the dropped Meta/Views sections. Add:

- `.searchRow` — flex row, `gap: var(--sp-2)`, `align-items: center`,
  `padding: var(--sp-3) var(--sp-2)`. The row carries the `hidden`
  attribute initially (handled by the user-agent default `display: none`);
  no additional `[hidden]` rule needed.
- `.searchInput` — flex-1, `background: var(--ac-bg-raised)`, `border: 1px
  solid var(--ac-stroke)`, `border-radius: var(--radius-md)`, `font:
  var(--size-sm)/1 var(--ac-font-body)`, `color: var(--ac-fg)`. No
  disabled-state styling needed — the input is `hidden`, not `disabled`.
- `.kbd` — small chip, `background: var(--ac-bg-sunken)`, `border: 1px
  solid var(--ac-stroke)`, `border-radius: var(--radius-sm)`, `font:
  var(--size-xxs)/1 var(--ac-font-mono)`, `color: var(--ac-fg-muted)`,
  `padding: 2px var(--sp-1)`.
- `.libraryHeading` — `text-transform: uppercase`, `font: 600
  var(--size-xxs)/1 var(--ac-font-display)`, `letter-spacing: 0.08em`,
  `color: var(--ac-fg-faint)`, `padding: var(--sp-2)`.
- `.phase` — `display: flex`, `flex-direction: column`, `margin-block:
  var(--sp-2)`.
- `.phaseHeading` — `font: 500 var(--size-xs)/1 var(--ac-font-display)`,
  `color: var(--ac-fg-muted)`, `padding: var(--sp-1) var(--sp-2)`.
- `.link` — flex row with `gap: var(--sp-2)`, `align-items: center`,
  `padding: var(--sp-1) var(--sp-2)`, `border-radius: var(--radius-sm)`,
  `color: var(--ac-fg)`. On `:hover`, `background: var(--ac-bg-hover)`.
- `.active` — `background: var(--ac-bg-active)`, `color: var(--ac-fg-strong)`.
- `.label` — `flex: 1` (pushes badge to the right edge).
- `.dot` — 8px circle, `background: var(--ac-accent)`, `border-radius:
  var(--radius-pill)`, `flex-shrink: 0`. Rendered **before** the badge in
  JSX so the dot sits immediately after the label (visually adjacent to
  the type name) and the badge floats to the right.
- `.badge` — inline pill, `background: var(--ac-accent-faint)`, `color:
  var(--ac-fg-muted)`, `border-radius: var(--radius-pill)`, `padding: 0
  var(--sp-2)`, `font: var(--size-xxs)/1.6 var(--ac-font-body)`,
  `margin-left: auto`, `flex-shrink: 0`.

All values reference the existing `--ac-*` / `--sp-*` / `--size-*` /
`--radius-*` token vocabulary defined in `frontend/src/styles/global.css`.
No new tokens are introduced.

#### 5. Extend `test/router-helpers.tsx`

**File**: `skills/visualisation/visualise/frontend/src/test/router-helpers.tsx`
**Changes**: Register `/library/$type` as a test route so the Sidebar's
`<Link>` can resolve during tests. Add an `Outlet`-only component the same
way `/library/$type/$fileSlug` is registered today.

### Success criteria

#### Automated verification

- [ ] All new `Sidebar.test.tsx` cases pass:
      `npm --prefix … test -- Sidebar`
- [ ] No regression elsewhere in the frontend suite: `make test-frontend`
- [ ] Type check passes: `make typecheck-frontend`
- [ ] Lint passes: `make lint-frontend`

#### Manual verification

- [ ] Sidebar renders the five phases under LIBRARY in canonical order.
- [ ] Each nav item shows a Glyph, label, count badge (when count > 0), and
      links to `/library/<type>`. When an item has both a dot and a badge,
      the dot sits adjacent to the label and the badge floats to the
      right.
- [ ] Active state highlights the current type when at `/library/<type>` or
      `/library/<type>/<doc>`. Visiting `/library/plan-reviews` highlights
      only Plan reviews, not Plans (prefix-collision regression check).
- [ ] Search input and `/` kbd chip exist in the DOM but are hidden from
      the user (verify via DevTools: the row carries the `hidden`
      attribute and is not in the tab order). Verify with a screen
      reader that the search field is not announced.
- [ ] Visiting the **list view** of a type via the Sidebar clears its
      unseen dot. (Verify by triggering an external edit — e.g. `touch
      meta/decisions/ADR-XXXX.md` — confirming the dot appears for
      `decisions`, then clicking the Decisions nav item and confirming
      the dot disappears.)
- [ ] Visiting a **deep link** to a child doc (e.g.,
      `/library/decisions/ADR-0001`) does NOT clear the parent type's
      dot. Navigate away and back to the list view to clear.
- [ ] Verify with a screen reader (VoiceOver / NVDA) that an unseen
      sidebar item is announced as "Decisions (unseen changes), link"
      when the user navigates over it, via the link's `aria-label`.
- [ ] Self-cause `doc-changed` echo (creating a doc via the UI) does not
      raise the dot. A `doc-invalid` event (UI save that triggers
      validation) also does not raise the dot.
- [ ] Templates does not appear in LIBRARY.
- [ ] Behaviour matches both light and dark themes (tokens are theme-aware).

---

## Testing Strategy

### Unit tests

- **`use-doc-events.test.ts`** (Phase 2): `onEvent` fires for `doc-changed`
  and `doc-invalid`, skipped for self-cause `doc-changed` echoes, forwards
  events with unknown `docType` verbatim. `onReconnect` fires exactly once
  per reconnection. Callback ref stability: re-rendering with a fresh
  options object does not re-create the EventSource and routes events to
  the latest callback. Also: invalidation cascade now includes
  `queryKeys.types()` so count badges stay live (Phase 1 §7).
- **`use-unseen-doc-types.test.ts`** (Phase 3): first-event-absorbs-
  silently semantics, strict-greater-than comparison, `markSeen`
  idempotence and clearing, `doc-invalid` ignored, `onReconnect` no-op,
  persistence round-trip of T values, transient `unseenSet` does not
  survive remount, multi-shape malformed-storage tolerance (not-json,
  array, NaN values, unknown DocTypeKey), `safeSetItem` failure does not
  propagate, `unseenSet` identity stability, reactivity through context.
- **`indexer.rs` inline tests** (Phase 1): `counts_by_type` returns correct
  cardinalities; Templates exclusion pinned via `!contains_key`.

### Integration tests

- **`api_types.rs`** (Phase 1): full `/api/types` response carries `count`
  per doc type matching fixture cardinalities; Templates count is `0`.
- **`Sidebar.test.tsx`** (Phase 5): render-level integration — phase
  grouping, Glyph rendering, badge/dot visibility, combined badge+dot
  layout, active-state correctness across `/library/<type>`,
  `/library/<type>/<slug>`, and prefix-collision boundaries, link
  `aria-label` reflects unseen state, hidden-but-present search row and
  kbd chip, empty-docTypes and missing-PHASE-entry edge cases.
- **`LibraryTypeView.test.tsx`** (Phase 4): list view bumps T, `:type`
  change re-fires, child-doc URL does NOT bump T, navigation between
  fileSlugs does not re-fire, invalid type does not bump T, StrictMode
  double-effect is harmless.

### Manual testing

1. Start the dev environment: `make dev` (or equivalent).
2. Navigate to `/`; confirm LIBRARY with five phases, twelve doc types, no
   Templates.
3. Click each phase's first item; confirm correct route, Glyph, badge.
4. Open DevTools → Application → Local Storage; confirm
   `ac-seen-doc-types` gains an entry per visit.
5. From a separate terminal, create or modify a doc type's file outside the
   app (`touch meta/decisions/ADR-0099.md`); confirm the unseen dot appears
   on the Decisions nav item within a few seconds.
6. Click the Decisions nav item; confirm the dot disappears and the stored T
   updates.
7. Create or modify a doc via the app UI; confirm no dot appears on the
   creating user's Sidebar (self-cause for `doc-changed`). Trigger a
   validation error on save (if achievable) and confirm no dot appears
   for the type — the tracker ignores `doc-invalid` events entirely.
8. Disconnect / reconnect SSE (toggle network, restart server). Real
   changes that occurred during the disconnect window will produce dots
   on reconnect via the replayed `doc-changed` events — this is intended
   behaviour (the changes are real activity the user has not seen). Pure
   reconnect with no intervening file changes should not raise any dots.

## Performance Considerations

- `Indexer::counts_by_type` is O(N) over the entries map under a single
  read lock with no cloning. With `/api/types` now invalidated on every
  `doc-changed` event (Phase 1 §7), the call frequency rises from
  once-per-page-load to once-per-event. React Query dedupes concurrent
  invalidations of the same query key into a single in-flight fetch, so
  a burst of N events under a filesystem-watcher storm (e.g. `jj
  checkout` touching dozens of files) produces 1 (not N) actual server
  request — only the per-invalidation cache-bookkeeping cost scales with
  event count, and that is microseconds. For typical entry counts (low
  thousands) the per-event server cost is microseconds and dominated by
  the lock acquisition itself. If profiling later shows contention
  against the existing `rescan` / `refresh_one` write paths, swap to an
  incremental `counts: HashMap<DocTypeKey, usize>` field maintained
  inside the existing `entries.write()` critical sections (decrement on
  remove, increment on insert). A useful trigger threshold for the swap:
  `/api/types` p99 latency under a synthetic watcher-storm fixture
  exceeding ~5ms.
- The unseen tracker persists at most 13 entries; storage operations are
  O(1) per write and trivial in size. **Only `markSeen` writes to
  storage** — `onEvent` mutates an in-memory `Set` only — so a watcher
  storm producing N events triggers 0 storage writes. The trade-off is
  that the transient `unseenSet` does not survive tab reload (documented).
- The unseen tracker exposes `unseenSet: ReadonlySet<DocTypeKey>` as its
  primary surface with stable identity across renders that don't change
  membership. Per-item Sidebar components can wrap in `React.memo` with
  primitive props (`unseen: boolean`) so only the items whose dot state
  actually flips re-render, not the whole Sidebar. If profiling shows the
  Sidebar re-rendering excessively in practice, extract `SidebarItem` as
  a memoised subcomponent.

## Migration Notes

- **`count` rollout**: server change ships first within Phase 1; the
  frontend `DocType` interface declares `count?: number` (optional), so
  AC2's "absent count → no badge" path means clients on older wire formats
  (or `DocType` fixtures that don't include `count`) continue to render
  correctly. No feature flag is required.
- **Storage schema**: `ac-seen-doc-types` is a fresh key holding a JSON
  object whose values are epoch-millisecond numbers (`Date.now()`). No
  migration of existing values. Future schema changes can layer an
  `_schemaVersion` field inside the JSON value if needed; the parser
  already drops non-finite values and unknown keys.
- **Drop unused `Sidebar.module.css` classes** along with the rewrite — no
  external consumers.

## References

- Work item: `meta/work/0053-sidebar-nav-and-unseen-tracker.md`
- Research: `meta/research/codebase/2026-05-12-0053-sidebar-nav-and-unseen-tracker.md`
- Parent epic: `meta/work/0036-sidebar-redesign.md`
- Related work items: `0033`, `0034`, `0035`, `0037`, `0054`, `0055`
- Review: `meta/reviews/work/0053-sidebar-nav-and-unseen-tracker-review-1.md`
- Source design gap: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `main-dark.png`, `library-view.png`
- Owning-hook pattern: `frontend/src/api/use-font-mode.ts`, `use-theme.ts`
- SSE plumbing: `frontend/src/api/use-doc-events.ts:86-160`
- LibraryTypeView insertion point: `frontend/src/routes/library/LibraryTypeView.tsx:52-54`
- Backend wiring: `server/src/api/types.rs:14-18`, `server/src/docs.rs:90-99`, `server/src/indexer.rs:316-324`
