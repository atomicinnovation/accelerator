---
date: "2026-05-12T20:55:11+01:00"
researcher: Toby Clemson
git_commit: 4396ef3e3300789e6ecb80c36bac17ec2f2ac2d2
branch: main
repository: accelerator
topic: "Sidebar Nav with Per-Type Change Indicators (work item 0053)"
tags: [research, codebase, frontend, sidebar, navigation, sse, localstorage, doc-types, backend, types-endpoint, indexer]
status: complete
last_updated: 2026-05-12
last_updated_by: Toby Clemson
---

# Research: Sidebar Nav with Per-Type Change Indicators (work item 0053)

**Date**: 2026-05-12T20:55:11+01:00
**Researcher**: Toby Clemson
**Git Commit**: 4396ef3e3300789e6ecb80c36bac17ec2f2ac2d2
**Branch**: main
**Repository**: accelerator

## Research Question

Ground-truth the implementation surface for work item 0053 — replacing the three flat Sidebar groups with a single LIBRARY heading partitioned by five lifecycle phases, decorating each item with a Glyph + count badge + unseen-changes dot, extending `GET /api/types` with a per-type `count`, and slotting an inert search-input scaffold for 0054.

The work item author already enumerated file:line references throughout; the goal of this research is to verify those references, surround them with context, and surface anything the work item missed before planning starts.

## Summary

All references in work item 0053 ground-truth cleanly. The implementation surface decomposes into five concrete deltas:

- **Frontend nav rebuild** in `Sidebar.tsx` — replace the inline `mainTypes`/`metaTypes`/`VIEW_TYPES` partition (currently using `t.virtual` as the membership signal) with a five-phase grouping driven by a new `PHASE_DOC_TYPES` constant in `api/types.ts` (alongside `LIFECYCLE_PIPELINE_STEPS`). Each row gains the existing `Glyph` component (already shipped, 12 icons, props match the work item's needs), a new count-badge element driven by `DocType.count`, and a new unseen-dot element driven by a new consumer hook.
- **New `useUnseenDocTypes` hook** in `frontend/src/api/use-unseen-doc-types.ts`, following the `use-font-mode.ts` shape exactly (plain function — no factory needed since there is no test-injectable predicate). Mounted alongside `useTheme`/`useFontMode`/`useDocEvents` in `RootLayout`, wrapped in its own Context Provider. No `BOOT_SCRIPT_SOURCE` changes required (no pre-paint CSS dependency).
- **SSE plumbing extension** in `use-doc-events.ts` — path (a) from the work item's analysis is mechanically clean: the `makeUseDocEvents` factory already supports parameter injection (`createSource`, `registry`), so adding an `onEvent` callback (or accepting an `UnseenTrackerHandle`) is additive. The callback must fire after the self-cause check (`use-doc-events.ts:139`) so suppressed echoes do not raise the dot, and likely on both `doc-changed` and `doc-invalid`. A new `unseen-on-reconnect` reset path mirrors the existing post-reconnect mass-invalidation.
- **`LibraryTypeView` bump-on-view** — insert `useMarkSeen(type)` between line 54 (end of `isDocTypeKey` narrowing) and line 59 (`useQuery`). Critically, TanStack Router reuses the component across `:type` param changes (no remount), so the implementation must depend on `[type]`, not `[]`. Placing it before the `params.fileSlug` early return (line 83) ensures it also fires when viewing a child doc within the type.
- **Backend `count` extension** — add `count: usize` to the `DocType` struct (`server/src/docs.rs:90-99`); the container-level `#[serde(rename_all = "camelCase")]` already in place serialises it as `"count"`. Wire counts in at the handler boundary (`server/src/api/types.rs:14`) using `state.indexer` (already accessible via `State<Arc<AppState>>`), not inside `describe_types` (which is config-only and synchronous). A new `Indexer::counts_by_type() -> HashMap<DocTypeKey, usize>` is cleaner than 13 calls to `all_by_type`. **Templates is excluded from the indexer entirely** (`indexer.rs:126-129`); its count must come from `state.templates.list().len()` or be defined as `0`.

The Glyph component (work item 0037) is shipped, with the 12 doc-type icons and the `isGlyphDocTypeKey` narrowing utility, but has no production consumer outside its showcase route — 0053 is its first real consumer. The Badge / dot indicator primitives the redesign needs do **not** exist yet (no generic `Badge`/`Pill`/`Tag`; no `UnseenIndicator`; no keybind-chip / `kbd` component); each will be created fresh either as Sidebar-local DOM or as small co-located components.

The 0053 work-item review (three passes) closed all majors. The clock source is pinned to `Date.now()` evaluated synchronously in the SSE `onmessage` handler (client receipt time) and at the call site that writes T; comparisons use strict greater-than. The "T equals client receipt time on first event" rule means the seeding event deterministically yields no dot.

## Detailed Findings

### Frontend chrome — current state

#### Sidebar.tsx (84 lines)

`skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`

- **Props** (`Sidebar.tsx:10-12`): single prop `{ docTypes: DocType[] }`. Confirmed.
- **Three-section partition**:
  - `VIEW_TYPES` constant at `Sidebar.tsx:5-8` — two entries: `{ path: '/lifecycle', label: 'Lifecycle' }`, `{ path: '/kanban', label: 'Kanban' }`.
  - `mainTypes` / `metaTypes` inline filters at `Sidebar.tsx:20-21` — split on `!t.virtual` vs `t.virtual`.
  - **Documents** section (`Sidebar.tsx:25-42`): renders `mainTypes` via `<Link to="/library/$type" params={{ type: t.key }}>`. Active state via `location.pathname.startsWith('/library/${t.key}')` (`Sidebar.tsx:33-35`).
  - **Views** section (`Sidebar.tsx:44-60`): renders `VIEW_TYPES` with exact-match active state (`Sidebar.tsx:51-53`).
  - **Meta** section (`Sidebar.tsx:62-79`): renders `metaTypes` with additional `styles.muted` class.
- Uses `useRouterState({ select: s => s.location })` (`Sidebar.tsx:15`) for active-state computation.
- `nav` carries `aria-label="Site navigation"` (`Sidebar.tsx:24`).
- **No footer, no version label, no search input, no keybind hints** today. The absence of version label is asserted by `Sidebar.test.tsx:52-56`.

#### Sidebar.module.css (38 lines)

`skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.module.css`

- `.sidebar`: `width: 220px`, `min-height: 100vh`, flex column, right border, `--ac-bg-sunken` background.
- `.section` / `.sectionHeading` / `.list` / `.link` / `.active` follow `--ac-*` / `--sp-*` / `--size-*` / `--radius-*` token vocabulary established by 0033 (now `done`).
- All styling is CSS Modules — no Tailwind, no styled-components.

#### Sidebar.test.tsx (57 lines)

`skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx`

- **Vestigial mocks** at `Sidebar.test.tsx:7-21`:
  - `useServerInfo` returns `{ data: undefined }`.
  - `useDocEventsContext` returns `{ setDragInProgress: vi.fn(), connectionState: 'open', justReconnected: false }`.
  - `useOrigin` returns `'localhost'`.
  - **None of these are imported by the current `Sidebar.tsx`** — pre-emptive mocks for transitive routes. Work item directs reusing the `useDocEventsContext` mock for the new tracker; the other two can be dropped.
- Mock data at `Sidebar.test.tsx:23-28`: 4 `DocType` entries — 3 non-virtual (`decisions`, `work-items`, `plans`), 1 virtual (`templates`).
- 4 test cases using `MemoryRouter` from `test/router-helpers`, asserting via async `findByText` (router settles inside `React.startTransition`).
- Test stack: vitest + `@testing-library/react` + internal `MemoryRouter` helper.

#### RootLayout.tsx (39 lines)

`skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx`

- Owns three hooks (`RootLayout.tsx:13-15`):
  - `const docEvents = useDocEvents()`
  - `const theme = useTheme()`
  - `const fontMode = useFontMode()`
- Fetches doc types at `RootLayout.tsx:17-20` — `useQuery({ queryKey: queryKeys.types(), queryFn: fetchTypes })`. Default `[]`. Confirmed exactly as work item describes.
- Provider nesting at `RootLayout.tsx:23-25` — Theme outermost, FontMode, DocEvents innermost. New `UnseenDocTypesContext.Provider` slots into this chain.
- DOM shape: `<div.root><Topbar/><div.body><Sidebar docTypes={docTypes}/><main><Outlet/></main></div></div>`.

#### Topbar.tsx (28 lines)

`skills/visualisation/visualise/frontend/src/components/Topbar/Topbar.tsx`

- `<SseIndicator />` confirmed mounted at `Topbar.tsx:17`. Reads `useDocEventsContext().connectionState`, renders 12px lightning-bolt SVG plus "SSE" text, with `data-state` / `data-animated` attributes driving CSS.
- Otherwise unrelated to this work item.

#### Query key factory

`skills/visualisation/visualise/frontend/src/api/query-keys.ts:6` — `types: () => ['types'] as const`. `types` is not in `SESSION_STABLE_QUERY_ROOTS`, so it follows normal invalidation rules.

### Frontend types model

`skills/visualisation/visualise/frontend/src/api/types.ts`

- **`DocTypeKey`** (`types.ts:4-8`): 13 string-literal variants. Order: `decisions, work-items, plans, research, plan-reviews, pr-reviews, work-item-reviews, validations, notes, prs, design-gaps, design-inventories, templates`. Mirrored at runtime by `DOC_TYPE_KEYS` (`types.ts:14-19`) and used by `isDocTypeKey` (`types.ts:22-24`).
- **`DocType` interface** (`types.ts:26-36`): `{ key, label, dirPath, inLifecycle, inKanban, virtual }` — `virtual` (camelCase, not `is_virtual`) is required. Adding `count: number` is straightforward.
- **`LIFECYCLE_PIPELINE_STEPS`** (`types.ts:134-169`) — readonly array of 11 step descriptors `{ key, docType, label, placeholder, longTail? }`. Derived `WORKFLOW_PIPELINE_STEPS` (8 main) and `LONG_TAIL_PIPELINE_STEPS` (3 long-tail) at `types.ts:171-177`.
- **`PHASE_DOC_TYPES` does not exist yet** — must be added in this file. The shape inherits the work item's table from 0036:

  | Phase    | Doc types (in display order) |
  |----------|------------------------------|
  | Define   | WorkItems, WorkItemReviews |
  | Discover | DesignInventories, DesignGaps, Research |
  | Build    | Plans, PlanReviews, Validations |
  | Ship     | Prs, PrReviews |
  | Remember | Decisions, Notes |

  Twelve doc types total — Templates excluded.
- No existing display-name / ordering / phase map exists beyond `LIFECYCLE_PIPELINE_STEPS`.

### Local-storage state pattern (factory + owning hook + Context + consumer)

Two reference implementations:

#### use-theme.ts (canonical with factory test seam)

`skills/visualisation/visualise/frontend/src/api/use-theme.ts`

- **Factory** (`use-theme.ts:26-50`): `makeUseTheme(prefersDark: () => boolean)` returns the hook closure. The `prefersDark` argument is the test-injection seam.
- **Owning hook** (`use-theme.ts:52-61`): production `export const useTheme = makeUseTheme(() => window.matchMedia(...).matches)`. Called exactly once at `RootLayout`.
- **Context** (`use-theme.ts:69`): `ThemeContext = createContext<ThemeHandle>(_defaultHandle)` — the default uses no-op functions so unprovided consumers still have a callable shape.
- **Consumer hook** (`use-theme.ts:76-78`): `useThemeContext()` returns the handle.
- **Persistence**: synchronous `safeGetItem` inside `useState` lazy initialiser; immediate `safeSetItem` inside `setTheme`'s `useCallback`. Comment at lines 39-43 explains why persistence is kept outside the state-updater (StrictMode double-invoke).
- **DOM mirror** (`use-theme.ts:30-32`): `useEffect` writes `data-theme` to `<html>` on every change. **Not needed for the unseen tracker** (no CSS dependency).

#### use-font-mode.ts (no factory)

`skills/visualisation/visualise/frontend/src/api/use-font-mode.ts`

- Same shape as `use-theme.ts` but **no factory** — `useFontMode` is a plain exported function (`use-font-mode.ts:32`). No injectable predicate (default is the constant `'display'`).
- This is the shape the unseen tracker should mirror — no test-injectable clock source is strictly necessary (`Date.now()` is fine; tests can use `vi.useFakeTimers()`).

#### safe-storage.ts (16 lines)

`skills/visualisation/visualise/frontend/src/api/safe-storage.ts`

- Exports `safeGetItem(key) => string | null` and `safeSetItem(key, value) => void`. Bare `try/catch` for private-mode `SecurityError` / quota errors.
- **No `safeRemoveItem`** — if the tracker needs deletion semantics, add it.

#### storage-keys.ts (17 lines)

`skills/visualisation/visualise/frontend/src/api/storage-keys.ts`

- Two keys today: `THEME_STORAGE_KEY = 'ac-theme'` and `FONT_MODE_STORAGE_KEY = 'ac-font-mode'`.
- **Convention**: kebab-case strings prefixed with `ac-`. Work item's proposed `'ac-seen-doc-types'` matches exactly.
- `BOOT_SCRIPT_SOURCE` (`storage-keys.ts:11-17`) — pre-paint IIFE that reads `localStorage` and sets `data-theme`/`data-font` on `<html>` before React mounts. **Not relevant for the unseen tracker** — no CSS depends on the value pre-paint.

#### Consumer pattern (where the hooks are used)

- Owning hooks live at `RootLayout.tsx:13-15`; Providers at `RootLayout.tsx:23-25`.
- Consumers: `ThemeToggle.tsx:5`, `FontModeToggle.tsx:5`, plus test mocks at `Topbar.test.tsx:19+`.

### SSE event-delivery plumbing

`skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`

#### Current dispatch model (no consumer subscription API)

- `dispatchSseEvent(event, queryClient, registry?)` at `use-doc-events.ts:48-78`:
  - Self-cause early-return for `doc-changed` events with `registry.has(event.etag)` (`use-doc-events.ts:53-55`).
  - Both `doc-changed` and `doc-invalid` invalidate the same five keys: `docs(docType)`, `docContent(path)`, `lifecycle()`, `lifecycleClusterPrefix()`, `relatedPrefix()` (with `refetchType: 'all'`).
  - `kanban()` only when `docType === 'work-items'`.
- `queryKeysForEvent(event)` (`use-doc-events.ts:21-34`) returns the same key set without invalidating, used by drag-deferral.

#### `SseEvent` union (`types.ts:87-100`)

```ts
SseDocChangedEvent  { type: 'doc-changed', docType: DocTypeKey, path: string, etag?: string }
SseDocInvalidEvent  { type: 'doc-invalid', docType: DocTypeKey, path: string }
```

**Neither variant carries `timestamp` or `action`** — both added later by 0055. The 0053 tracker uses client receipt time (`Date.now()` inside `onmessage`), not server timestamps.

#### Factory and EventSource wiring

- `makeUseDocEvents(createSource: EventSourceFactory, registry: SelfCauseRegistry = defaultSelfCauseRegistry)` at `use-doc-events.ts:86-160` — two test-injection seams.
- Returns a hook that creates a `ReconnectingEventSource` against `/api/events` (`use-doc-events.ts:112`).
- Returns a `DocEventsHandle` (`use-doc-events.ts:15-19`): `{ setDragInProgress, connectionState, justReconnected }`. **No `subscribe`, `addListener`, or `on` method exists on the handle today** — confirming the work item's claim.
- Drag deferral: while `isDraggingRef.current`, events are enqueued (as JSON-stringified query keys) into `pendingRef` (`use-doc-events.ts:140-143`), invalidation deferred until `setDragInProgress(false)`.
- Reconnect path (`use-doc-events.ts:118-133`): resets `defaultSelfCauseRegistry`, flushes drag-pending keys, runs mass invalidation excluding `SESSION_STABLE_QUERY_ROOTS = {'server-info','work-item-config'}`, sets `justReconnected: true` for 3s.

#### Extension paths for 0053

- **Path (a) — preferred**: extend `makeUseDocEvents` with a third parameter (e.g. `onEvent?: (event: SseEvent) => void`, or a typed `UnseenTrackerHandle`). Call site is `use-doc-events.ts:136-150`. Insertion must be after the self-cause check (line 139) — self-cause echoes should NOT raise the dot — and likely fire for both `doc-changed` and `doc-invalid`. Drag deferral (lines 140-143) is independent of the tracker callback (`pendingRef` only stores invalidation keys); the tracker can fire eagerly even mid-drag. Reconnect handling at lines 118-133 should mirror to the tracker (e.g. a `tracker.reset()` call) because the post-reconnect mass invalidation can trigger many cascading events — but the tracker's first-event-seeds-T behaviour may absorb this naturally.
- **Path (b)**: second `EventSource` consumer — workable (`ReconnectingEventSource` is exported), but doubles the SSE connection and complicates ordering guarantees against the existing stream. Not preferred.

#### Self-cause registry

`skills/visualisation/visualise/frontend/src/api/self-cause.ts` — `Map<etag, registeredTimestampMs>` with FIFO eviction (256 max) and TTL pruning (5s). `defaultSelfCauseRegistry` is the module singleton.

### LibraryTypeView — consumer side

`skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx`

#### Hook order on every render

1. `useParams({ strict: false })` (line 46)
2. `useState(sortKey)` (line 49)
3. `useState(sortDir)` (line 50)
4. `useQuery` (line 59)
5. `useMemo` (line 72)

**Insert `useMarkSeen(type)` between line 54 (end of narrowing) and line 59** — keeps hook order stable, runs unconditionally with `type: DocTypeKey | undefined`, hook is a no-op when `type === undefined` (the prop-render test path).

#### Narrowing

`LibraryTypeView.tsx:52-54`:
```tsx
const type: DocTypeKey | undefined =
  rawType && isDocTypeKey(rawType) ? rawType : undefined
```

Router's `parseParams` at `router.ts:99-104` already redirects unknown values to `/library`, so for URL-driven mounts `type` is always defined. The local guard exists for the direct-render prop path used by tests (lines 42-45).

#### Mount semantics — critical finding

**TanStack Router does NOT remount when only `:type` changes.** Navigating `/library/work-items` → `/library/research` reuses the same component instance:
- `useState` values survive (sort key/dir persist).
- `useEffect(() => {...}, [])` fires only on first entry to the route.
- `useEffect(() => {...}, [type])` fires on every `type` change.

So `useMarkSeen` must internally `useEffect(() => { if (type) writeT(type) }, [type])` — NOT `[]`. Under React StrictMode the effect runs twice in dev (mount + simulated unmount + remount); naturally idempotent since both writes set `Date.now()` to nearly-equal values.

#### Outlet branch interaction

`LibraryTypeView.tsx:83`: when `params.fileSlug` is set, the parent route returns `<Outlet />` (delegates to `LibraryDocView`). A `useMarkSeen(type)` placed before line 83 will fire even when the user is viewing a child doc within the type — likely desired (the user has seen the type's content).

#### Sibling routes

- `LibraryLayout.tsx` — five lines, just `<Outlet />`.
- `LibraryTemplatesIndex.tsx` mounted at `/library/templates` (`router.ts:81-85`) — literal path **preempts** the generic `/$type` route. Since `templates` is in `DocTypeKey`, AC behaviour for the Templates "T" depends on whether the work item considers Templates part of LIBRARY. **It does not** — `Templates` is excluded from LIBRARY by phase-table omission (work item AC1). No `useMarkSeen('templates')` needed.
- `LibraryTemplatesView.tsx` mounted at `/library/templates/$name`.
- `LibraryDocView.tsx` mounted at `/library/$type/$fileSlug` (child of `libraryTypeRoute`). Uses identical narrowing.

#### No existing `useEffect` in any library route

Greppable: zero `useEffect` calls under `frontend/src/routes/library/`. `useMarkSeen` will be the first.

### Backend — DocType, types handler, indexer

#### docs.rs (284 lines)

`skills/visualisation/visualise/server/src/docs.rs`

- **`DocType` struct** at `docs.rs:90-99`:
  ```rust
  #[derive(Debug, Clone, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DocType {
      pub key: DocTypeKey,
      pub label: String,
      pub dir_path: Option<PathBuf>,
      pub in_lifecycle: bool,
      pub in_kanban: bool,
      pub r#virtual: bool,
  }
  ```
  - `Serialize`-only — never deserialised in this crate, so the field addition is wire-additive only.
  - Container-level `#[serde(rename_all = "camelCase")]` applies — `count: usize` serialises as `"count"` (single word, unchanged).
  - Only three references to the `DocType` struct in the workspace: definition (`docs.rs:92`), construction (`docs.rs:101,107`), and `TypesResponse` field (`api/types.rs:11`). Safe to extend.
- **`is_virtual`** at `docs.rs:85-87` — `matches!(self, DocTypeKey::Templates)`. Confirms Templates is the sole virtual type. `in_lifecycle` at `docs.rs:77-79` is `!matches!(self, DocTypeKey::Templates)`.
- **`DocTypeKey` enum** at `docs.rs:4-20`: 13 variants, `#[serde(rename_all = "kebab-case")]`. Registry methods: `all()` (line 23), `config_path_key()` (line 41), `label()` (line 59), `in_lifecycle()`, `in_kanban()`, `is_virtual()`.
- **`describe_types(cfg: &Config) -> Vec<DocType>`** at `docs.rs:101-117` — pure, synchronous, config-only. Has no `Indexer` reference; cannot see entry counts. **Do not touch this signature.** Wire `count` in at the handler boundary instead.

#### api/types.rs (19 lines)

`skills/visualisation/visualise/server/src/api/types.rs`

```rust
pub(crate) async fn types(State(state): State<Arc<AppState>>) -> Json<TypesResponse> {
    Json(TypesResponse {
        types: describe_types(&state.cfg),
    })
}
```

- Handler is already `async` and already receives `State<Arc<AppState>>`. `state.indexer: Arc<Indexer>` is in scope (`server.rs:44`). Wiring `count` requires no state-plumbing changes.
- Route registration: `api/mod.rs:25` — `.route("/api/types", get(types::types))`.
- `TypesResponse { types: Vec<DocType> }` envelope at `api/types.rs:9-12`.

**Recommended handler shape** (additive):

```rust
pub(crate) async fn types(State(state): State<Arc<AppState>>) -> Json<TypesResponse> {
    let mut types = describe_types(&state.cfg);
    let counts = state.indexer.counts_by_type().await; // new method
    for t in &mut types {
        t.count = counts.get(&t.key).copied().unwrap_or(0);
        // Special-case Templates if its count should reflect state.templates.list().len()
    }
    Json(TypesResponse { types })
}
```

#### indexer.rs

`skills/visualisation/visualise/server/src/indexer.rs`

- **`Indexer` struct** at `indexer.rs:48-76`. Entries live in `entries: Arc<RwLock<HashMap<PathBuf, IndexEntry>>>` (line 58), keyed by canonical absolute path. Each `IndexEntry` carries `r#type: DocTypeKey`.
- **No per-type bucketed map exists.** Counting requires iterating the entries map.
- **`all_by_type(kind: DocTypeKey) -> Vec<IndexEntry>`** at `indexer.rs:316-324` — clones every entry to a Vec. Calling this 13 times to get counts is wasteful (clones N×13 entries).
- **Recommended new method**:
  ```rust
  pub async fn counts_by_type(&self) -> HashMap<DocTypeKey, usize> {
      let mut out = HashMap::new();
      for e in self.entries.read().await.values() {
          *out.entry(e.r#type).or_insert(0) += 1;
      }
      out
  }
  ```
  Single read-lock, no clones, O(N) once.
- **Templates is excluded from the index entirely** (`indexer.rs:126-129` inside `rescan`): `for kind in DocTypeKey::all() { if kind == DocTypeKey::Templates { continue; } ... }`. So `counts_by_type()` will return no entry for `Templates`. The handler must decide: `0`, the `state.templates.list().len()` value, or simply omit (`unwrap_or(0)` yields `0`).

#### Contract test — server/tests/api_types.rs (47 lines)

`skills/visualisation/visualise/server/tests/api_types.rs`

- Setup: `common::seeded_cfg(tmp.path())` (`tests/common/mod.rs:6-76`) seeds 1 decision, 1 plan, 1 plan-review, 0 of everything else.
- Transport: `tower::ServiceExt::oneshot` against the in-process Axum app.
- Parsing: `serde_json::Value` — additive field changes do **not** cause type-driven failures.
- Existing per-entry assertions: `arr.len() == 13`; `templates.virtual = true, dirPath = null`; `decisions.virtual = false, dirPath` is string; `design-gaps.inLifecycle = true`; `design-inventories.inLifecycle = true`.
- **New assertions to add**: per-entry `count` values matching the seeded fixtures (`decisions: 1`, `plans: 1`, `plan-reviews: 1`, all others: `0`).
- Second touch point: `tests/api_smoke.rs:91-100` — asserts only the 13-length envelope; no break from the field addition.

### Component primitives — what exists vs what to build

#### Already shipped (work item 0037 — Glyph)

`skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx`

- Exports: `Glyph` (component), `GlyphProps`, `GlyphDocTypeKey`, `GLYPH_DOC_TYPE_KEYS`, `isGlyphDocTypeKey`.
- Prop shape (`Glyph.tsx:60-67`): `{ docType: GlyphDocTypeKey, size: 16 | 24 | 32, ariaLabel?: string }`. `ariaLabel` presence toggles `role="img"` vs `aria-hidden`.
- 12 icon files in `components/Glyph/icons/` — one per non-Templates `DocTypeKey`.
- Test at `Glyph.test.tsx`. Showcase route at `routes/glyph-showcase/`.
- **Only consumer in production today**: the showcase route itself. 0053 is the first real consumer.
- No CSS module — icons are inline SVGs.

#### Other existing primitives

- `OriginPill` (`components/OriginPill/OriginPill.tsx`) — single-purpose origin-host pill. Not reusable.
- `FrontmatterChips` (`components/FrontmatterChips/FrontmatterChips.tsx`) — frontmatter rendering. Not reusable.
- `SseIndicator` (`components/SseIndicator/SseIndicator.tsx`) — animated live-status indicator. Pattern reusable for dot styling but not the component itself.
- `PipelineDots` (`components/PipelineDots/PipelineDots.tsx`) — lifecycle pipeline dots. CSS pattern reusable.

#### Not yet existing (must be built for 0053)

- Generic `Badge` / `CountBadge` / `Pill` / `Tag` / `Chip` component — none exists.
- `UnseenIndicator` / `LiveIndicator` dot — only `SseIndicator` exists.
- `useSeen` / `useUnseen` / `useMarkSeen` / `useTracker` hooks — none exist.
- `SearchInput` / `Search` / `LibrarySearch` component — none exists.
- `Kbd` / `Keybind` / `Shortcut` / keybind-chip component — none exists.

**Recommendation**: keep the count badge and unseen dot as Sidebar-local DOM (small CSS module classes), not new shared components — the 0036 epic only consumes them in one place. 0054 will introduce the search input as a shared piece.

### Work-item review history

`meta/reviews/work/0053-sidebar-nav-and-unseen-tracker-review-1.md` ran three passes (verdicts: REVISE → REVISE → COMMENT). All majors resolved before the work item was promoted to `ready`. Key Pass-2/Pass-3 decisions now in the work item:

- **Clock source pinned to `Date.now()`** — evaluated synchronously inside the SSE `onmessage` handler for "client receipt time" and at the call site that writes T for "current time". ISO-8601 strings produced via `new Date(Date.now()).toISOString()` and compared by re-parsing to milliseconds.
- **Strict-greater-than comparison** — if a `doc-changed` event's client receipt time equals stored T (same `Date.now()` reading), no dot is shown (treated as already-seen).
- **Mount-window definition (AC5)** — "from first render through unmount, regardless of whether the AC7 mount-effect bump has yet completed". An event arriving while `LibraryTypeView` is mounted always bumps T and clears the dot.
- **First-event seeding** — on the very first event for a doc type with no stored T, T is seeded to the current time and no dot is shown.

These remove the AC ambiguities the work-item author would otherwise re-litigate during planning.

## Code References

### Frontend

- `frontend/src/components/Sidebar/Sidebar.tsx:5-8` — `VIEW_TYPES` constant (Lifecycle / Kanban)
- `frontend/src/components/Sidebar/Sidebar.tsx:10-12` — `Props { docTypes: DocType[] }`
- `frontend/src/components/Sidebar/Sidebar.tsx:15` — `useRouterState({ select: s => s.location })`
- `frontend/src/components/Sidebar/Sidebar.tsx:20-21` — inline `!t.virtual` / `t.virtual` partition
- `frontend/src/components/Sidebar/Sidebar.tsx:25-79` — three `<section>` blocks (Documents / Views / Meta)
- `frontend/src/components/Sidebar/Sidebar.module.css:1-37` — token-driven styling
- `frontend/src/components/Sidebar/Sidebar.test.tsx:7-21` — vestigial mocks (repurpose `useDocEventsContext`)
- `frontend/src/components/Sidebar/Sidebar.test.tsx:52-56` — version-label-absent regression assertion
- `frontend/src/components/RootLayout/RootLayout.tsx:13-15` — three owning hooks
- `frontend/src/components/RootLayout/RootLayout.tsx:17-20` — `useQuery(queryKeys.types(), fetchTypes)`
- `frontend/src/components/RootLayout/RootLayout.tsx:23-25` — provider nesting
- `frontend/src/components/Topbar/Topbar.tsx:17` — `<SseIndicator />` mount (confirms 0035 footer migration done)
- `frontend/src/api/types.ts:4-8` — `DocTypeKey` union (13 variants)
- `frontend/src/api/types.ts:14-19` — `DOC_TYPE_KEYS` runtime mirror
- `frontend/src/api/types.ts:22-24` — `isDocTypeKey` narrowing guard
- `frontend/src/api/types.ts:26-36` — `DocType` interface (add `count: number` here)
- `frontend/src/api/types.ts:87-100` — `SseEvent` union — no `timestamp`/`action` today
- `frontend/src/api/types.ts:134-169` — `LIFECYCLE_PIPELINE_STEPS` (add `PHASE_DOC_TYPES` alongside)
- `frontend/src/api/use-theme.ts:26-50` — `makeUseTheme` factory
- `frontend/src/api/use-theme.ts:39-43` — comment block on why persistence is outside the state-updater (StrictMode)
- `frontend/src/api/use-theme.ts:63-78` — Context, default handle, consumer hook
- `frontend/src/api/use-font-mode.ts:23-67` — plain-function variant (no factory) — the closer template for the unseen tracker
- `frontend/src/api/safe-storage.ts:1-15` — `safeGetItem` / `safeSetItem` (no `safeRemoveItem`)
- `frontend/src/api/storage-keys.ts:1-2` — `THEME_STORAGE_KEY`, `FONT_MODE_STORAGE_KEY` (add `SEEN_DOC_TYPES_STORAGE_KEY = 'ac-seen-doc-types'`)
- `frontend/src/api/use-doc-events.ts:48-78` — `dispatchSseEvent` (the place that already branches on `event.type`)
- `frontend/src/api/use-doc-events.ts:86-160` — `makeUseDocEvents` factory (test injection seam)
- `frontend/src/api/use-doc-events.ts:118-133` — reconnect path (consider mirroring to tracker)
- `frontend/src/api/use-doc-events.ts:136-150` — `onmessage` handler — primary insertion point for path (a) tracker callback
- `frontend/src/api/use-doc-events.ts:139` — self-cause early return — callback must fire AFTER this
- `frontend/src/api/query-keys.ts:6` — `queryKeys.types()` factory
- `frontend/src/api/fetch.ts:56-61` — `fetchTypes()` GET helper
- `frontend/src/components/Glyph/Glyph.tsx:28-67` — `Glyph` component exports and prop shape
- `frontend/src/router.ts:96-106` — `libraryTypeRoute` definition (component reuse semantics)
- `frontend/src/router.ts:99-104` — `parseParams` redirects unknown `:type` to `/library`
- `frontend/src/routes/library/LibraryTypeView.tsx:46-54` — narrowing → insertion point on line 55
- `frontend/src/routes/library/LibraryTypeView.tsx:59-63` — `useQuery` with `enabled`-style gate
- `frontend/src/routes/library/LibraryTypeView.tsx:83` — `<Outlet />` early return when viewing child doc
- `frontend/src/routes/library/LibraryTemplatesIndex.tsx` — literal `/library/templates` route (preempts `/$type`)

### Backend

- `server/src/docs.rs:4-20` — `DocTypeKey` enum
- `server/src/docs.rs:23-87` — `DocTypeKey` registry methods
- `server/src/docs.rs:85-87` — `is_virtual` confirms only Templates
- `server/src/docs.rs:90-99` — `DocType` struct (add `pub count: usize`)
- `server/src/docs.rs:101-117` — `describe_types(cfg)` — leave untouched
- `server/src/api/types.rs:9-12` — `TypesResponse { types: Vec<DocType> }`
- `server/src/api/types.rs:14-18` — handler (extend here)
- `server/src/api/mod.rs:25` — route registration
- `server/src/server.rs:44` — `AppState.indexer: Arc<Indexer>` exposure
- `server/src/server.rs:45` — `AppState.templates: Arc<TemplateResolver>`
- `server/src/indexer.rs:48-76` — `Indexer` struct shape
- `server/src/indexer.rs:58` — entries `HashMap<PathBuf, IndexEntry>` (no per-type bucketing)
- `server/src/indexer.rs:126-129` — Templates excluded from `rescan`
- `server/src/indexer.rs:316-324` — `all_by_type` (add `counts_by_type` alongside)
- `server/src/templates.rs:109,151` — `list()` / `names()` for the Templates count source if needed

### Tests

- `server/tests/api_types.rs:30-46` — contract test for `/api/types` (add `count` assertions here)
- `server/tests/common/mod.rs:6-76` — `seeded_cfg` builds 1 decision, 1 plan, 1 plan-review, 0 of everything else
- `server/tests/api_smoke.rs:91-100` — envelope-length only assertion
- `frontend/src/components/Sidebar/Sidebar.test.tsx` — existing tests; will need re-shaping for the new phase grouping + tracker mock

## Architecture Insights

- **Component reuse across param changes is invisible at the consumer level.** TanStack Router does not remount `LibraryTypeView` on `:type` change; this is essential to know for the unseen-tracker insertion point but is undocumented in code. The implementation of `useMarkSeen` must use `[type]` deps; the test plan should cover the type-change-no-remount path explicitly.
- **The chrome owns three persistence channels** (theme, font-mode, doc-events) through a uniform owning-hook + Context pattern. Adding a fourth (unseen tracker) does not disturb the shape; it just slots into `RootLayout.tsx:13-25`. No new architectural layer is introduced.
- **The SSE plumbing has one consumer, by design.** `ReconnectingEventSource.onmessage` is a single field. Path (a) — adding a callback to the existing hook — preserves this. Path (b) — opening a second `EventSource` — would break the single-consumer invariant and double the server's connection count. Path (a) is unambiguous.
- **The indexer's flat `HashMap` storage is the right shape for occasional per-type counts**, but a dedicated `counts_by_type()` method is materially cheaper than 13 `all_by_type` calls because it avoids cloning every entry. Calling code that needs both counts AND entries should use `all_by_type` once per type instead.
- **The DocType struct is `Serialize`-only and used in exactly three places.** The 0053 schema extension is contractually small — wire-additive, no breakage risk, no consumer-side ripples.
- **Templates is a second-class doc type by design**: virtual flag on the frontend, excluded from the indexer on the backend, has its own dedicated route on the frontend. The 0053 redesign extends this — Templates is excluded from LIBRARY entirely (by phase-table omission, not by `virtual: true` filter). The implication: do not put a count badge on Templates in the new nav, and don't try to thread `useMarkSeen('templates')` from `LibraryTemplatesIndex` either.
- **Self-cause registry must not be bypassed by the unseen tracker.** Local mutations register an etag in `defaultSelfCauseRegistry` before issuing the PATCH; the server echoes back a `doc-changed` with that etag; `dispatchSseEvent` swallows it (`use-doc-events.ts:53-55`). The tracker callback must fire AFTER this filter — otherwise the user would see their own edit raise a dot.

## Historical Context

- **0033 — Design Token System** (`status: done`): ships `--ac-*` CSS custom properties for both light and dark themes. The Sidebar redesign consumes these wholesale. ADR-0026 (`meta/decisions/ADR-0026-css-design-token-application-conventions.md`) records the `--ac-*` conventions.
- **0034 — Theme and Font-Mode Toggles** (`status: done`): the canonical implementation of the owning-hook + Context + consumer pattern used by `use-theme.ts` and `use-font-mode.ts`. The unseen tracker mirrors this shape.
- **0035 — Topbar Component** (`status: done`): relocated `SseIndicator` from the Sidebar footer to the Topbar; removed the version label. `Sidebar.test.tsx:52-56` regression assertion documents this. Nothing more to do on the chrome footer migration.
- **0036 — Sidebar Redesign** (`status: draft`): parent epic. Holds the canonical phase-to-doc-type table and the action-verb canonical set (`created` | `edited` | `moved` | `deleted` — relevant to 0055, not 0053).
- **0037 — Glyph Component** (`status: draft`): the Glyph implementation 0053 consumes. Already shipped at `frontend/src/components/Glyph/` (12 icons + showcase route + tests). Mature enough that 0053 can consume directly.
- **0054 — Sidebar Search** (`status: ready`): downstream sibling. Consumes the inert search-input slot 0053 ships. Will wire `/api/search` plus `/` keybind handling.
- **0055 — Sidebar Activity Feed** (`status: ready`): downstream sibling. Adds `SsePayload::DocChanged.action` + `timestamp`, `GET /api/activity?limit=N`, the ActivityFeed component. The work item's "0055 may later promote the comparison to a server timestamp without breaking the AC" Assumption captures the forward-compat path for the unseen tracker.
- **0039 — Toaster and External-Edit Notifications** (`status: draft`): another consumer of the same `doc-changed` event stream. If/when 0039 plans, it will face the same "consumer can't subscribe to the event stream" problem; the path-(a) extension this work item makes is reusable.
- **Phase 4 SSE plan** (`meta/plans/2026-04-22-meta-visualiser-phase-4-sse-hub-and-notify-watcher.md`): historical context on the original SSE hub and watcher implementation.

Key meta-directory documents:

- `meta/work/0036-sidebar-redesign.md` — parent epic with the canonical phase-to-doc-type table
- `meta/work/0033-design-token-system.md` (`done`)
- `meta/work/0034-theme-and-font-mode-toggles.md` (`done`)
- `meta/work/0035-topbar-component.md` (`done`)
- `meta/work/0037-glyph-component.md` (`draft`, but implementation already in code)
- `meta/work/0054-sidebar-search.md` (`ready`)
- `meta/work/0055-sidebar-activity-feed.md` (`ready`)
- `meta/reviews/work/0053-sidebar-nav-and-unseen-tracker-review-1.md` — three-pass review, all majors closed
- `meta/reviews/work/0036-sidebar-redesign-review-1.md` — parent epic review
- `meta/research/2026-05-12-0037-glyph-component.md` — Glyph research (companion)
- `meta/research/2026-05-08-0034-theme-and-font-mode-toggles.md` — owning-hook pattern research
- `meta/research/2026-05-06-0033-design-token-system.md` — token vocabulary
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` — `--ac-*` conventions
- `meta/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md` — source design-gap doc
- `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`, `main-dark.png`, `library-view.png` — reference screenshots

## Related Research

- `meta/research/2026-05-12-0037-glyph-component.md` — Glyph component (consumed by this work item; first real consumer)
- `meta/research/2026-05-08-0034-theme-and-font-mode-toggles.md` — owning-hook + Context pattern (template for `useUnseenDocTypes`)
- `meta/research/2026-05-07-0035-topbar-component.md` — chrome restructure history; documents the SseIndicator relocation
- `meta/research/2026-05-06-0033-design-token-system.md` — `--ac-*` token vocabulary the redesign consumes

## Open Questions

- **Templates count semantics**. The indexer skips Templates entirely (`indexer.rs:126-129`), so `counts_by_type()` yields no entry for it. Three options: (a) `count = 0` for Templates (simplest; matches the `unwrap_or(0)` natural fallback); (b) `count = state.templates.list().len()` (truthful, but couples the handler to `TemplateResolver`); (c) omit `count` from the Templates entry (frontend already handles absent `count` as no-badge per AC2). 0053 excludes Templates from LIBRARY rendering, so the value is observed only via `/api/types` consumers other than the Sidebar (currently none). Defer to planning.
- **Initial unseen state on a fresh browser**. The tracker keys T off `localStorage[ac-seen-doc-types]`. AC4 covers the first-event-with-no-T case (seed to now, no dot). What about the inverse — fresh browser, no events yet, user visits the app? Sidebar renders with no dots for any doc type. This is implicit in the work item but worth pinning in the plan.
- **Reconnect-storm behaviour**. When `ReconnectingEventSource` reconnects, the server may emit a burst of catch-up `doc-changed` events for changes that occurred during the disconnect. The tracker will faithfully raise the dot for every such event whose client receipt time > stored T. This is correct behaviour ("new activity since you last visited"), but worth a brief mention in the plan so reviewers are not surprised. The existing post-reconnect mass invalidation at `use-doc-events.ts:122-131` is independent of the tracker.
- **`useMarkSeen` and `LibraryDocView`**. When the user navigates `/library/work-items/some-slug`, the parent `LibraryTypeView` renders `<Outlet />` (line 83). A `useMarkSeen(type)` placed before line 83 fires regardless. Is that the desired semantics — "seeing a child doc within a type counts as seeing the type"? The work item's AC7 mentions `LibraryTypeView` mount specifically; AC5 says "while `LibraryTypeView` is mounted". Both ACs are satisfied by the proposed insertion point. But document the choice to avoid drift.
- **Sidebar nav membership for non-canonical types**. The 13 `DocTypeKey` variants split cleanly into the phase table (12 LIBRARY entries) plus Templates. Future doc-type additions would need explicit phase placement in `PHASE_DOC_TYPES`. The work item doesn't specify what happens to a `DocType` returned by the server but missing from `PHASE_DOC_TYPES` — silently dropped from the nav? Logged as an error? Defer to planning.
- **Storage value validation**. The work item specifies `Record<DocTypeKey, ISO-8601 string>` as the localStorage shape. The hook needs to validate the parsed shape (defend against tampering, schema drift, old keys from removed doc types). Simplest defence: on `JSON.parse` failure or shape mismatch, treat as empty object. Pin during planning.
