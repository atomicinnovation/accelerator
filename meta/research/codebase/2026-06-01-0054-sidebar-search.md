---
type: codebase-research
id: "2026-06-01-0054-sidebar-search"
title: "Research: Sidebar Search Input and API Search Endpoint (work item 0054)"
date: "2026-06-01T20:31:09+01:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0054"
topic: "Wire the sidebar search input slot, add a /api/search endpoint, and introduce the supporting useDebouncedValue and `/` keybind primitives"
tags: [research, codebase, sidebar, search, keybind, visualiser, api]
revision: "ba1d8a25116abda4fdfedfc2fb8f8c4a33c97c4a"
repository: "accelerator"
last_updated: "2026-06-01T20:31:09+01:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Sidebar Search Input and API Search Endpoint (work item 0054)

**Date**: 2026-06-01T20:31:09+01:00
**Author**: Toby Clemson
**Git Commit**: ba1d8a25116abda4fdfedfc2fb8f8c4a33c97c4a
**Branch**: wqxvqnsuylzz (no bookmark)
**Repository**: accelerator (workspace: visualisation-system)

## Research Question

For work item [0054 Sidebar Search Input and API Search Endpoint](../../work/0054-sidebar-search.md):
locate and analyse every part of the visualise codebase that the story
touches — server route registration, the indexer snapshot path, the
frontend fetch/queryKeys/hooks layer, the existing Sidebar search input
slot, RootLayout, Glyph, the doc-type label mapping, the library-doc
route shape, and the testing conventions. Surface the precedents,
verify the story's claims (no existing debounce / hotkey infrastructure;
zero effects in RootLayout; Templates excluded by Indexer), and capture
exact line numbers / excerpts so implementation can proceed without
re-greps.

## Summary

The story's claims about the codebase are accurate but a handful of
details need adjustment when planning. The most important to act on:

1. **`Glyph` has no `framed` default** — per-doc-type contexts pass
   `framed` explicitly (e.g. `EyebrowLabel.tsx:13` uses `<Glyph
   docType={type} size={16} framed />`). Search result rows must pass
   `framed` themselves to match the user's "framed everywhere"
   convention.
2. **Sidebar lives at `frontend/src/components/Sidebar/`**, not
   `frontend/src/Sidebar/`. The story's path needs updating; the
   markup, classes (`searchRow`, `searchInput`, `searchIcon`, `kbd`),
   and the `TEMPORARY` comment from 0053 are all there as described.
3. **RootLayout lives at `frontend/src/components/RootLayout/`** (same
   path correction). It has zero `useEffect`s today, confirming the
   story's claim.
4. **The doc-type label mapping is `DOC_TYPE_LABELS` in
   `frontend/src/api/types.ts`** — no separate `docTypeMeta` module
   exists. `LibraryTypeView` and `EyebrowLabel` both consume it.
5. **`Indexer::all()` is at `src/indexer.rs:617-619`** (not 570-572 as
   the story says; the file has grown). The Templates-absence
   assertion is at line **1444** (not 1282). Logic unchanged.
6. **`ApiError` has no dedicated `InvalidQuery` variant.** The story's
   AC says `q` absent or empty returns `{ results: [] }` with 200, so
   no new variant is needed; axum's `Query<T>` extractor handles
   missing-param 400s if needed, and we can model an empty-`q`
   short-circuit directly in the handler.
7. **`<Link>` from Tanstack Router v1 already renders an `<a href>`
   element** (verified in `test/router-helpers.test.tsx:14-18`), so the
   story's `<a href>` requirement is satisfied by using `<Link
   to="/library/$type/$fileSlug" params={{ type, fileSlug }}>` — the
   canonical pattern used by every other doc-row in the app (kanban,
   library, lifecycle, activity feed).
8. **`QueryClient` defaults are `staleTime: Infinity, retry: 1`** with
   no `gcTime` override (`api/query-client.ts:3-13`). React Query's
   default `gcTime` of 5 minutes therefore applies — the story's
   "within ~1 second" debounce-dedup test window sits comfortably
   inside it.
9. **Testing uses Vitest + RTL with per-suite provider stacks** (no
   `renderWithProviders`, no MSW). `Sidebar.test.tsx` already has a
   test confirming the temporary search markup at lines 253-258 —
   wiring 0054 must keep both `<input type="search">` and the `<kbd>`
   present.

The remaining sections give the precise file paths, line numbers, and
code excerpts behind each of these points and the secondary findings
(query-key insertion order, route-tree implications for `templates`,
prototype-source absence on disk, testing patterns for hooks that wrap
`useQuery`, etc.).

## Detailed Findings

### Server: where to register `/api/search`

#### Route registration — `server/src/api/mod.rs:1-46`

Module declarations are alphabetised (lines 1-11). `mod search;` slots
between `mod related;` (line 8) and `mod templates;` (line 9).

The router chain (lines 24-46) is **not** alphabetised — it's grouped
by feature. The conventional shape to append:

```rust
.route("/api/search", get(search::search))
```

The router type is `Router<Arc<AppState>>`, so state injection works
via `State(state): State<Arc<AppState>>` in the handler. The
`pub fn mount(_state: Arc<AppState>) -> Router<Arc<AppState>>`
signature does not need to change.

#### Handler precedent — `server/src/api/docs.rs:19-41`

`docs_list` is the closest precedent for `State + Query → Result<Response, ApiError>`:

```rust
#[derive(Debug, Deserialize)]
pub(crate) struct DocsListQuery {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Serialize)]
struct DocsListResponse {
    docs: Vec<IndexEntry>,
}

pub(crate) async fn docs_list(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DocsListQuery>,
) -> Result<Response, ApiError> {
    let kind = parse_kind(&q.type_).ok_or(ApiError::InvalidDocType(q.type_.clone()))?;
    if kind == DocTypeKey::Templates {
        return Err(ApiError::InvalidDocType(q.type_.clone()));
    }
    let mut entries: Vec<IndexEntry> = state.indexer.all_by_type(kind).await;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Json(DocsListResponse { docs: entries }).into_response())
}
```

Pattern points for `search::search`:
- `Query` must be `Option<String>` or `Default`-ed because the AC
  requires `q` absent OR empty string to return `{ results: [] }`
  with HTTP 200. Either:
  - `pub(crate) struct SearchQuery { #[serde(default)] q: String }`
    (deserialiser tolerates missing `q`); or
  - take `Query<HashMap<String, String>>` and pull `q` manually.
  The first is more idiomatic.
- Return `Json(SearchResponse { results }).into_response()` — the
  serde camelCase wrapping happens on the row struct itself (see
  `IndexEntry` precedent below), not on the response wrapper.

#### Indexer snapshot — `server/src/indexer.rs`

Method `Indexer::all()` (lines 617-619, not 570-572 as the story
states):

```rust
pub async fn all(&self) -> Vec<IndexEntry> {
    self.entries.read().await.values().cloned().collect()
}
```

`async` because the lock is a tokio `RwLock::read().await`. Read lock
guard drops at end of statement; every value is cloned. The handler
must `.await` it. O(N) per request, acceptable given the 200 ms
client-side debounce and 2-character minimum.

`entries` field (line 216):
```rust
entries: Arc<RwLock<HashMap<PathBuf, IndexEntry>>>,
```
Keyed by canonicalised absolute path. There is no
`entries_by_type` map; `all_by_type` (lines 532-540) is itself a
linear filter and would not benefit a global-search handler.

`IndexEntry` struct (lines 162-192):

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub r#type: DocTypeKey,
    pub path: PathBuf,
    pub rel_path: PathBuf,
    pub slug: Option<String>,
    pub work_item_id: Option<String>,
    pub title: String,
    pub frontmatter: serde_json::Value,
    pub frontmatter_state: String,
    pub work_item_refs: Vec<String>,
    pub mtime_ms: i64,
    pub size: u64,
    pub etag: String,
    pub body_preview: String,
    pub completeness: Option<Completeness>,
    pub linked_count: usize,
}
```

The camelCase precedent is the `#[serde(rename_all = "camelCase")]`
attribute combined with `r#type: DocTypeKey` (raw-identifier escape).
On the wire: `type`, `relPath`, `mtimeMs`, `bodyPreview`,
`workItemId`. Search rows can either:
- return a subset of `IndexEntry` fields by projection into a small
  `SearchResultRow` struct (preferred — keeps the wire payload minimal
  and decoupled from the indexer's internal fields), or
- return `IndexEntry` directly (matches `DocsListResponse`).

The story's response row is the minimal projection
`{ docType, title, slug, path, mtimeMs }`. A `SearchResultRow` struct
is therefore the right shape:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResultRow {
    pub doc_type: DocTypeKey,
    pub title: String,
    pub slug: Option<String>,
    pub path: PathBuf,
    pub mtime_ms: i64,
}
```

#### Templates exclusion — confirmed structural

`server/src/indexer.rs:293-300` skips Templates at enumeration time:
```rust
for kind in DocTypeKey::all() {
    if kind == DocTypeKey::Templates {
        continue;
    }
    ...
}
```

`Indexer::all()` therefore yields zero Templates entries by
construction. The assertion confirming this is in
`indexer.rs:1444` (not 1282 — file has grown):
```rust
assert!(!counts.contains_key(&DocTypeKey::Templates));
```

Templates are managed by `server/src/templates.rs` independently —
`AppState.templates` is `Arc<arc_swap::ArcSwap<TemplateResolver>>`
(see `server/src/server.rs:45`), wholly separate from
`AppState.indexer`. A search endpoint that reads `state.indexer.all()`
will not see templates regardless of any future indexer change to
include them; the AC's defensive "zero entries with `docType ===
"templates"`" assertion guards that future-proof contract.

#### `DocTypeKey` enum — `server/src/docs.rs:4-20`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocTypeKey {
    Decisions, WorkItems, Plans, Research,
    PlanReviews, PrReviews, WorkItemReviews,
    Validations, Notes, PrDescriptions,
    DesignGaps, DesignInventories,
    Templates,
}
```

13 variants, kebab-case wire form. Twelve LIBRARY types (everything
except `Templates`) are what the AC enumerates.

#### `ApiError` — `server/src/api/mod.rs:48-153`

Variants relevant to search:
- `InvalidDocType(String)` — 400
- `NotFound(String)` — 404
- `Internal(String)` — 500
- (no `InvalidQuery`, no `BadRequest` variant)

Every variant body is `Json(serde_json::json!({ "error": "..." }))`.

The story does **not** require returning a 4xx for any input; empty
or missing `q` returns 200 + empty results. So **no new `ApiError`
variant is needed**. Bad `q` parsing (impossible for `String`) and
extractor rejection paths aren't on the spec'd happy path.

#### `AppState` — `server/src/server.rs:40-52`

```rust
pub struct AppState {
    pub cfg: Arc<Config>,
    pub kanban_columns: Arc<Vec<crate::config::KanbanColumn>>,
    pub file_driver: Arc<crate::file_driver::LocalFileDriver>,
    pub indexer: Arc<crate::indexer::Indexer>,
    pub templates: Arc<arc_swap::ArcSwap<crate::templates::TemplateResolver>>,
    pub template_change_handler: Arc<crate::watcher::TemplateChangeHandler>,
    pub clusters: Arc<RwLock<Vec<crate::clusters::LifecycleCluster>>>,
    pub http_activity: Arc<crate::activity::Activity>,
    pub activity_feed: Arc<crate::activity_feed::ActivityRingBuffer>,
    pub sse_hub: Arc<crate::sse_hub::SseHub>,
    pub write_coordinator: Arc<crate::write_coordinator::WriteCoordinator>,
}
```

The handler accesses `state.indexer` (line 44). Constructed by
`AppState::build(cfg, http_activity).await` (line 55) — also used in
the integration-test crate.

#### Server tests — `tests/api_docs.rs`

API tests live in the sibling integration-test crate, not inline.
Convention is one file per route family (`tests/api_smoke.rs`,
`tests/api_related.rs`, `tests/api_work_item_pattern.rs`,
`tests/api_docs_patch.rs`). Search would conventionally get
`tests/api_search.rs`.

Per-test pattern (`tests/api_docs.rs:12-35`):

```rust
#[tokio::test]
async fn docs_list_returns_index_entries_for_decisions() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=decisions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["docs"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["type"], "decisions");
}
```

`mod common;` at line 10 — fixtures live in `tests/common/`
(`common::seeded_cfg(tmp.path())`). 400-path tests at lines 37-54 and
56-73 are direct models for `?q=` validation paths if we add any.

### Frontend: fetch / query keys / hooks / router

#### `fetch.ts` — `FetchError` and the helper pattern

`frontend/src/api/fetch.ts:11-19`:

```ts
export class FetchError extends Error {
  constructor(public readonly status: number, message: string) {
    super(message)
    this.name = 'FetchError'
  }
}
```

The class does not auto-include the URL — call sites pass the full
message. Every helper uses the pattern:

```ts
export async function fetchTypes(): Promise<DocType[]> {
  const r = await fetch('/api/types')
  if (!r.ok) throw new FetchError(r.status, `GET /api/types: ${r.status}`)
  const body: { types: DocType[] } = await r.json()
  return body.types
}
```
(`fetch.ts:59-64`)

For 0054, `fetchSearch(q)` must include the literal `/api/search` in
the error message so the AC's `err.message.includes('/api/search')`
holds:

```ts
export async function fetchSearch(q: string): Promise<SearchResult[]> {
  const r = await fetch(`/api/search?q=${encodeURIComponent(q)}`)
  if (!r.ok) throw new FetchError(r.status, `GET /api/search?q=${q}: ${r.status}`)
  const body: { results: SearchResult[] } = await r.json()
  return body.results
}
```

Existing helpers using `encodeURIComponent`: `fetchDocs` (87-92),
`fetchActivity` (162-167), `fetchRelated` (169-174). All use
`URLSearchParams` for multi-param requests (`fetchLibraryStructure`,
106-123) — single-param is fine inline.

#### `query-keys.ts` — `frontend/src/api/query-keys.ts:40-62`

```ts
export const queryKeys = {
  serverInfo: () => ['server-info'] as const,
  workItemConfig: () => ['work-item-config'] as const,
  types: () => ['types'] as const,
  libraryStructure: (selection?: LibrarySelection) =>
    ['library-structure', normaliseSelection(selection)] as const,
  docs: (type: DocTypeKey) => ['docs', type] as const,
  docContent: (relPath: string) => ['doc-content', relPath] as const,
  templates: () => ['templates'] as const,
  templateDetail: (name: string) => ['template-detail', name] as const,
  lifecycle: () => ['lifecycle'] as const,
  lifecycleClusterPrefix: () => ['lifecycle-cluster'] as const,
  lifecycleCluster: (slug: string) => ['lifecycle-cluster', slug] as const,
  kanban: () => ['kanban'] as const,
  related: (relPath: string) => ['related', relPath] as const,
  relatedPrefix: () => ['related'] as const,
  activity: (limit: number) => ['activity', limit] as const,
  disabled: (prefix: string) => [prefix, '__disabled__'] as const,
} as const
```

Observations:
- Order is by feature grouping, not alphabetical. A new
  `search: (q: string) => ['search', q] as const` slots most
  naturally before `disabled` (which is the sentinel and stays last).
- First tuple element is kebab-case (`'server-info'`,
  `'lifecycle-cluster'`). For search, single-word `'search'` is
  consistent with `'types'`, `'docs'`, `'lifecycle'`, etc.
- The `disabled(prefix)` sentinel (lines 57-61) is the codebase
  idiom for gated queries; mirrors `useRelated` / `useDocContent`
  patterns.

`SESSION_STABLE_QUERY_ROOTS` (lines 64-67) is a `ReadonlySet` of keys
excluded from session-level cache invalidation. `'search'` is **not**
a candidate — results are file-content-dependent, so it stays out of
the set and inherits the default session-invalidation behaviour.

#### `QueryClient` setup — `frontend/src/api/query-client.ts:1-14`

```ts
import { QueryClient } from '@tanstack/react-query'

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: Infinity,
      retry: 1,
    },
  },
})
```

No `gcTime` override — React Query's default of 5 minutes applies.
The story's debounce-dedup AC uses a "within ~1 second" verification
window, which sits well inside `gcTime`.

`QueryClientProvider` wraps `<RouterProvider>` in
`frontend/src/main.tsx:11-17`. Tanstack React Query v5
(`package.json:24`).

#### `useSearch` hook pattern — model on `use-related.ts`

Existing pattern (`frontend/src/api/use-related.ts`):

```ts
export function useRelated(relPath: string | undefined) {
  return useQuery({
    queryKey: relPath ? queryKeys.related(relPath) : queryKeys.disabled('related'),
    queryFn: () => fetchRelated(relPath!),
    enabled: !!relPath,
  })
}
```

For 0054, the hook contract is:

```ts
// frontend/src/api/use-search.ts
import { useQuery } from '@tanstack/react-query'
import { fetchSearch } from './fetch'
import { queryKeys } from './query-keys'
import { useDebouncedValue } from './use-debounced-value'

export function useSearch(query: string) {
  const debounced = useDebouncedValue(query.trim(), 200)
  return useQuery({
    queryKey: debounced.length >= 2
      ? queryKeys.search(debounced)
      : queryKeys.disabled('search'),
    queryFn: () => fetchSearch(debounced),
    enabled: debounced.length >= 2,
  })
}
```

Naming: file is `use-search.ts` (kebab-case file name, camelCase
export — matches every other hook in `frontend/src/api/`).
**`useDebouncedValue` is co-located**: place it at
`frontend/src/api/use-debounced-value.ts`. The story says ~10 lines
using `useState` + `useEffect` + `setTimeout`/`clearTimeout`:

```ts
import { useEffect, useState } from 'react'

export function useDebouncedValue<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delayMs)
    return () => clearTimeout(id)
  }, [value, delayMs])
  return debounced
}
```

Trailing-edge by construction: every change to `value` clears the
prior timer and schedules a fresh one. The AC's
`ab → abc → ab` example yields exactly one request for `q=ab` because
the intermediate `abc` never lives 200 ms.

#### `router.ts` — `libraryDocRoute` and the `/library/$type/$fileSlug` shape

`frontend/src/router.ts:109-113`:

```ts
const libraryDocRoute = withCrumb(({ params }) => params.fileSlug, {
  getParentRoute: () => libraryTypeRoute,
  path: '/$fileSlug',
  component: LibraryDocView,
})
```

The parent `libraryTypeRoute` (lines 97-107) enforces
`$type ∈ DocTypeKey`:

```ts
parseParams: (raw: Record<string, string>): { type: DocTypeKey } => {
  if (!isDocTypeKey(raw.type)) {
    throw redirect({ to: '/library' })
  }
  return { type: raw.type }
},
```

`isDocTypeKey` (`api/types.ts:22-24`) accepts only the 13 kebab-case
wire values. **Any other string redirects to `/library`** — so
search result rows must pass `type` as one of those kebab forms (e.g.
`'work-items'`, `'plan-reviews'`).

Sibling route to watch for: `libraryTemplateDetailRoute`
(`/library/templates/$name`) takes precedence over `/library/$type/$fileSlug`
for the literal `templates` segment. Since `Indexer::all()` never
yields Templates entries, search results never hit this route — but
worth flagging for future-proofing if Templates are ever indexed.

#### `<Link>` renders an `<a>` — verified

Tanstack React Router v1 (`package.json:25`). `<Link to="..." params={...}>` renders an
implicit anchor element. Verified by `test/router-helpers.test.tsx:14-18`:

```tsx
<Link to="/library/$type/$fileSlug" params={{ type: 'work-items', fileSlug: '0001-x' }}>
  work item link
</Link>,
```

followed by `screen.findByRole('link', { name: /work item link/i })`
— `role="link"` is the implicit ARIA role of `<a href>`, which is the
only HTML element that exposes it.

The story's "result rows as `<a href>` elements" requirement is
therefore satisfied by using `<Link>`. Using `<Link>` rather than a
raw `<a href={template}>` also gets:
- type-safe params (TS catches a wrong `type` value at compile time)
- client-side navigation interception (no full reload)
- consistency with every other doc-row in the app

Every existing consumer uses `<Link>`:
- `routes/library/LibraryTypeView.tsx:235-240`
- `routes/lifecycle/LifecycleClusterView.tsx:137-143`
- `routes/kanban/WorkItemCard.tsx:36-37`
- `components/ActivityFeed/ActivityFeed.tsx:117-119`

`useNavigate`/`router.navigate` is reserved for non-link click
handlers (only `components/Breadcrumbs/Breadcrumbs.tsx:21,42`).

### Frontend: Sidebar, RootLayout, Glyph, LifecycleIndex precedent

#### Sidebar path correction

The actual paths are under `frontend/src/components/Sidebar/`, not
`frontend/src/Sidebar/` as the story says. Files:

- `frontend/src/components/Sidebar/Sidebar.tsx`
- `frontend/src/components/Sidebar/Sidebar.module.css`
- `frontend/src/components/Sidebar/Sidebar.test.tsx`

Same correction applies to RootLayout:
`frontend/src/components/RootLayout/RootLayout.tsx`.

#### Sidebar current shape

`frontend/src/components/Sidebar/Sidebar.tsx:1-13`:

```tsx
import { Link, useRouterState } from '@tanstack/react-router'
import type { DocType, LibraryDocType, LibraryPhase } from '../../api/types'
import { useUnseenDocTypesContext } from '../../api/use-unseen-doc-types'
import { ActivityFeed } from '../ActivityFeed/ActivityFeed'
import styles from './Sidebar.module.css'

interface Props {
  docTypes: DocType[]
  phases: LibraryPhase[]
  templates: LibraryDocType | null
}

export function Sidebar({ docTypes, phases, templates }: Props) {
```

Returns `<nav className={styles.sidebar} aria-label="Site navigation">`
(line 21).

Temporary search input slot (`Sidebar.tsx:22-33`):

```tsx
{/* TEMPORARY: search row visible for design review. Wire behaviour
    in work item 0054 (search submission, `/` keybind focus). */}
<div className={styles.searchRow}>
  <SearchIcon />
  <input
    type="search"
    aria-label="Search"
    placeholder="Search meta/..."
    className={styles.searchInput}
  />
  <kbd className={styles.kbd}>/</kbd>
</div>
```

`SearchIcon` is an inline SVG defined at lines 132-145 (not the
shared `Glyph` component — leave it alone unless replacing).

CSS classes available on the slot: `searchRow`, `searchIcon`,
`searchInput`, `kbd`. The results panel CSS should hang off
`searchRow`'s sibling position.

#### Sidebar.module.css search-row classes

`frontend/src/components/Sidebar/Sidebar.module.css`:

- `.sidebar` (1-16): 240px width, flex column, `gap: var(--sp-4)`,
  `padding: var(--sp-4) var(--sp-3)`.
- TEMPORARY block (23-35): `.searchRow` is 36px high, `gap: var(--sp-2)`,
  `padding: 0 10px`, raised background, stroked border, `border-radius:
  var(--radius-md)`, `margin-bottom: var(--sp-2)`.
- `.searchIcon` (37-40), `.searchInput` (42-60), `.kbd` (71-90).

The results panel naturally sits below `.searchRow` as a sibling
inside `.sidebar` (which has its own `gap: var(--sp-4)` separator) —
or could be wrapped with `.searchRow` in an extra container that
provides shared 10px horizontal padding.

#### RootLayout — zero effects today (confirmed)

`frontend/src/components/RootLayout/RootLayout.tsx` does NOT import
`useEffect`, `useState`, or `useRef`. The body (lines 19-75) is
provider composition + two `useQuery` calls. The story's claim is
verified — the new `/` keybind effect is the first effect added at
this layer.

Composition pattern at lines 52-56:

```tsx
<Sidebar
  docTypes={docTypes}
  phases={libraryStructure?.phases ?? []}
  templates={libraryStructure?.templates ?? null}
/>
```

For 0054, this becomes:

```tsx
<Sidebar
  docTypes={docTypes}
  phases={libraryStructure?.phases ?? []}
  templates={libraryStructure?.templates ?? null}
  searchInputRef={searchInputRef}
/>
```

with `const searchInputRef = useRef<HTMLInputElement>(null)` plus a
`useEffect` registering the global `keydown` listener. The story
correctly mandates ref-by-prop (no new context, no module singleton).

The `/` listener body, per the AC:

```ts
useEffect(() => {
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== '/') return
    if (e.ctrlKey || e.metaKey || e.altKey) return
    const active = document.activeElement
    if (active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        (active instanceof HTMLElement && active.isContentEditable)) {
      return
    }
    e.preventDefault()
    searchInputRef.current?.focus()
  }
  document.addEventListener('keydown', onKeyDown)
  return () => document.removeEventListener('keydown', onKeyDown)
}, [])
```

Note: the AC explicitly mentions Cmd/Ctrl/Alt/Meta. The
`KeyboardEvent` API gives `metaKey` (Cmd on macOS / Win key on
Windows) and `ctrlKey` separately; `e.metaKey || e.ctrlKey ||
e.altKey` covers all four story-named modifiers (shift `/` is
typically how you type `?` on many keyboards but `e.key` would
already be `?` in that case, so we don't need to check shift). The
guard is "no modifiers" — `e.key === '/'` is sufficient to disambiguate
on US keyboards; international layouts may produce `/` via shift on
some keys but `e.key` reflects the actual typed character.

#### Glyph — no `framed` default

`frontend/src/components/Glyph/Glyph.tsx:39-49, 68`:

```ts
export interface GlyphProps {
  docType: DocTypeKey
  size: 16 | 24 | 32
  ariaLabel?: string
  framed?: boolean
}

export function Glyph({ docType, size, ariaLabel, framed }: GlyphProps): ReactElement | null
```

`framed` has no destructuring default. Effective default is
`undefined → unframed`. Per the user-profile memory note that
per-doc-type contexts should always be framed, search result rows
must pass `framed` explicitly. Match `EyebrowLabel.tsx:13`:

```tsx
<Glyph docType={type} size={16} framed />
```

Named export only (`export function Glyph(...)`), so import as
`import { Glyph } from '../Glyph/Glyph'`.

#### Doc-type labels — `DOC_TYPE_LABELS`

`frontend/src/api/types.ts:49-63`:

```ts
export const DOC_TYPE_LABELS: Readonly<Record<DocTypeKey, string>> = {
  'decisions': 'Decisions',
  'work-items': 'Work items',
  'plans': 'Plans',
  'research': 'Research',
  'plan-reviews': 'Plan reviews',
  'pr-reviews': 'PR reviews',
  'work-item-reviews': 'Work item reviews',
  'validations': 'Validations',
  'notes': 'Notes',
  'pr-descriptions': 'PR descriptions',
  'design-gaps': 'Design gaps',
  'design-inventories': 'Design inventories',
  'templates': 'Templates',
}
```

This is the mapping `LibraryTypeView` uses (`LibraryTypeView.tsx:13`):

```ts
import { isDocTypeKey, DOC_TYPE_LABELS } from '../../api/types'
```

and renders as `DOC_TYPE_LABELS[type]` at lines 160, 171, 190, 202.
`EyebrowLabel` internally uppercases it. Search rows can use
`DOC_TYPE_LABELS[result.docType]` directly (sentence case) or
uppercase to match eyebrows — designer's call; the AC just says
"doc-type label".

The story's mention of "the same `docTypeMeta`-style mapping" should
be read as `DOC_TYPE_LABELS`. There is no `docTypeMeta` module on
disk — `frontend/src/components/ActivityFeed/ActivityFeed.tsx:57` has
an inline `labelFor` helper that prefers server-emitted
`DocType.label` (via `useTypes()`) with `DOC_TYPE_LABELS` fallback,
but that's a private helper, not a reusable export. For consistency
with `LibraryTypeView` (the story's named precedent),
`DOC_TYPE_LABELS` is the right source.

#### LifecycleIndex search input — precedent for markup, not behaviour

`frontend/src/routes/lifecycle/LifecycleIndex.tsx:62-69`:

```tsx
<input
  type="search"
  aria-label="Filter clusters"
  placeholder="Filter…"
  className={styles.filterInput}
  value={filter}
  onChange={e => setFilter(e.target.value)}
/>
```

Confirmed purely client-side: `useQuery` loads clusters once
(`LifecycleIndex.tsx:50-53`); `filter` state lives in `useState`
(line 48); `useMemo` filters in-memory (lines 55-58). The substring
matcher (lines 37-44) does case-insensitive `String.includes` on
`title || slug`, with empty/whitespace short-circuit to "no filter".

Empty-state markup (lines 93-97) — useful precedent for the AC's
`No matches` `role="status"`:
```tsx
<p role="status">No clusters match "{filter}".</p>
```

For 0054 the literal text is `No matches` (per the AC's exact-string
requirement).

#### Kebab-case `DocTypeKey` in URLs — confirmed

`routes/kanban/WorkItemCard.tsx:36-37` uses the literal
`'work-items'` (kebab) as the `type` param:

```tsx
to="/library/$type/$fileSlug"
params={{ type: 'work-items', fileSlug }}
```

Other consumers (`LibraryTypeView.tsx:236-240`,
`LifecycleClusterView.tsx:137-143`, `ActivityFeed.tsx:117-119`) all
pass kebab-case `DocTypeKey` values. `Sidebar.test.tsx:166-167`
asserts `href === '/library/templates'`. Search result links must
pass the row's `docType` (which is already the kebab wire form from
the server) directly to `params.type`.

### Testing patterns

#### Vitest + RTL + per-suite providers (no MSW)

`vite.config.ts` holds the Vitest config (no separate
`vitest.config.*`). Setup file: `frontend/src/test/setup.ts`. Shared
helpers: `frontend/src/test/router-helpers.tsx` (includes a test-only
`libraryDocRoute` for assertions on link `href`).

There is **no global `renderWithProviders`** — each suite assembles
its own provider stack. Two predominant patterns:

**Pattern A — module-level mock + custom render helper** (used in
`Sidebar.test.tsx:18-28, 98-121`):
```tsx
vi.mock('../../api/fetch', async () => {
  const actual = await vi.importActual<typeof import('../../api/fetch')>('../../api/fetch')
  return { ...actual, fetchActivity: vi.fn() }
})

function renderSidebar(...args) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={queryClient}>
      <UnseenDocTypesContext.Provider value={...}>
        <MemoryRouter>
          <Sidebar {...} />
        </MemoryRouter>
      </UnseenDocTypesContext.Provider>
    </QueryClientProvider>
  )
}
```

**Pattern B — per-test `vi.spyOn` + `Wrapper` component** (used in
`LifecycleIndex.test.tsx:33-42`):
```tsx
function Wrapper({ children }: { children: ReactNode }) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
}
// ... in each test:
vi.spyOn(fetchModule, 'fetchLifecycleClusters').mockResolvedValue(clusters)
render(<LifecycleIndex />, { wrapper: Wrapper })
```

For 0054, Pattern B is the better fit for `useSearch`-driven tests
(we want to vary the mock per test for ranking / empty / failure /
in-flight scenarios). Pattern A may suit `Sidebar.test.tsx` updates
where many existing tests already mock `fetchActivity`.

#### Existing search-input test in `Sidebar.test.tsx`

`Sidebar.test.tsx:253-258`:
```tsx
it('search row is rendered (temporary; 0054 will wire behaviour)', async () => {
  const { container } = renderSidebar()
  await screen.findByText('LIBRARY')
  expect(container.querySelector('input[type="search"]')).not.toBeNull()
  expect(container.querySelector('kbd')?.textContent).toBe('/')
})
```

This test must continue passing — keep the `<input type="search">`
and the `<kbd>/</kbd>`. The test description text will need updating
when 0054 wires real behaviour (or replace with a stronger
assertion).

#### Hook tests that wrap `useQuery`

Closest models:
- `frontend/src/api/use-related.test.tsx`
- `frontend/src/api/use-doc-page-data.test.tsx`
- `frontend/src/api/use-server-info.test.tsx`
- `frontend/src/api/use-deferred-fetching-hint.test.tsx` (closest in
  spirit — tests a timing-based hook)
- `frontend/src/api/use-unseen-doc-types.test.ts`

For `useSearch` tests specifically, the AC's debounce-dedup criterion
needs **fake timers** (`vi.useFakeTimers()` + `vi.advanceTimersByTime(200)`)
plus React Query's request count assertion. Pattern:

```ts
beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

it('only requests once for the settled query', async () => {
  const spy = vi.spyOn(fetchModule, 'fetchSearch').mockResolvedValue([])
  const { rerender } = renderHook(({ q }) => useSearch(q), {
    initialProps: { q: 'ab' },
    wrapper: Wrapper,
  })
  await vi.advanceTimersByTimeAsync(50)
  rerender({ q: 'abc' })
  await vi.advanceTimersByTimeAsync(50)
  rerender({ q: 'ab' })
  await vi.advanceTimersByTimeAsync(200)
  expect(spy).toHaveBeenCalledTimes(1)
  expect(spy).toHaveBeenCalledWith('ab')
})
```

(`vi.useFakeTimers` interacts with React Query's internal microtasks;
expect to need `vi.advanceTimersByTimeAsync` rather than the sync
form, and to flush promises with `await waitFor(...)` for assertions
on the hook output.)

#### Failure-path testing

The AC requires `console.error` to receive a `FetchError` whose
message includes `/api/search`. Pattern:

```ts
it('logs FetchError on failure', async () => {
  const errSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
  vi.spyOn(fetchModule, 'fetchSearch').mockRejectedValue(
    new FetchError(500, 'GET /api/search?q=foo: 500'),
  )
  // ... render, advance timers, await query settle ...
  expect(errSpy).toHaveBeenCalledWith(
    expect.objectContaining({
      message: expect.stringContaining('/api/search'),
    }),
  )
})
```

React Query v5 logs `queryFn` rejections via the global
`QueryCache.onError` by default — but with `retry: 1` (the global
default), a 500 will retry once before the rejection surfaces. For
deterministic test timing, configure `retry: false` on the test
`QueryClient`.

There is no precedent for the AC's exact `console.error` contract in
the codebase — `fetchModule.fetchActivity` failures are surfaced via
toasts, not `console.error`. Implementation will need to add an
explicit `useEffect(() => { if (error) console.error(error) }, [error])`
in the `Sidebar`'s search consumer (or wherever the query result is
read) to satisfy the AC literally.

### Prototype reference — what's actually on disk

The story warns that `src/search.jsx` doesn't exist on disk. Verified:
the inventory directory contains only `inventory.md`,
`prototype-standalone.html`, and `screenshots/` — no `src/`. The
standalone HTML does not embed a readable JSX source either.

Inventory references for context (not authoritative — the AC is):
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md:392-400`
  (`Highlight`, `SearchBox`, `useSearch`)
- Line 406 (`Sidebar` integration)
- Lines 781-786 (`omnibar-search` capability)

The prototype includes a `Highlight` component for match highlighting,
but the 0054 AC does not require highlighting — only Glyph + title +
doc-type label per row. We can defer highlighting to a future
enhancement.

### Implementation sketch

Below is a tactical sketch synthesising the findings — not a plan
document, just enough to seed implementation.

#### Server side

1. New file `skills/visualisation/visualise/server/src/api/search.rs`:
   - `SearchQuery { q: Option<String> }` extractor.
   - Handler: lowercase the query, short-circuit to
     `Json(SearchResponse { results: vec![] })` when `q` is None or
     empty after trim.
   - Call `state.indexer.all().await`, filter to matching entries,
     classify into 4 buckets (exact-slug → title/slug prefix →
     title/slug interior → body-preview interior), sort each bucket by
     `(-mtime_ms, path)`, concatenate, project to `SearchResultRow`.
   - Return `Ok(Json(SearchResponse { results }).into_response())`.
2. Register route in `server/src/api/mod.rs`: add `mod search;`
   (alphabetically between `related` and `templates`) and
   `.route("/api/search", get(search::search))` in the chain.
3. New file `skills/visualisation/visualise/server/tests/api_search.rs`:
   - Empty `q`: 200 + empty results.
   - Multi-bucket ordering: stage entries with controlled mtimes /
     titles / slugs / body previews and assert response array order.
   - Case-insensitive matching.
   - Templates not in results regardless of `Indexer::all()` output.
   - Slugless entries do not appear in slug-keyed buckets.

#### Frontend — `frontend/src/api/`

1. `query-keys.ts`: add `search: (q: string) => ['search', q] as const`
   before `disabled`.
2. `fetch.ts`: add `fetchSearch(q: string): Promise<SearchResult[]>`.
   Define `SearchResult` interface with `docType: DocTypeKey, title:
   string, slug: string | null, path: string, mtimeMs: number`.
3. New file `use-debounced-value.ts`: ~10-line generic hook.
4. New file `use-search.ts`: wraps `useDebouncedValue` +
   `useQuery({ queryKey, queryFn, enabled })`.
5. Tests:
   - `use-debounced-value.test.ts` (timer-based unit test).
   - `use-search.test.tsx` (debounce-dedup AC, ≥2 char AC, query-key
     identity, in-flight clearing AC).
   - Extend `fetch.test.ts` with `fetchSearch` success / failure
     coverage (message includes `/api/search`).

#### Frontend — `frontend/src/components/`

1. `Sidebar/Sidebar.tsx`:
   - Add `searchInputRef: RefObject<HTMLInputElement>` to `Props`.
   - Replace the uncontrolled `<input>` with a controlled one:
     `value={query}`, `onChange`, plus `ref={searchInputRef}`.
   - Remove the `TEMPORARY` comment.
   - Below `.searchRow` (still inside `.sidebar`), render a results
     panel when `query.trim().length >= 2` after debounce — uses
     `useSearch(query)` and branches on `data` / `isPending` / `isError`.
   - Inside results: map `data` to `<Link to="/library/$type/$fileSlug"
     params={{ type: result.docType, fileSlug: result.slug! }}>` rows.
     Skip rows where `slug === null` (the AC permits this; the slug
     buckets already exclude them).
   - Empty-state: `<p role="status">No matches</p>` when
     `data?.length === 0`.
   - On error: also clear results, log via `useEffect` →
     `console.error`.
2. `RootLayout/RootLayout.tsx`:
   - Add `const searchInputRef = useRef<HTMLInputElement>(null)`.
   - Add `useEffect` registering the `/` keydown listener (the body
     sketched in "RootLayout — zero effects today" above).
   - Pass `searchInputRef` to `<Sidebar>`.
3. `Sidebar/Sidebar.module.css`: add `.results`, `.resultRow`,
   `.resultEmpty` (and any minor adjustments to `.searchRow` if a
   wrapper is needed for shared padding).
4. Tests:
   - Extend `Sidebar.test.tsx`: typing triggers search after 200 ms,
     results render as `<a href>`s with correct paths, empty state
     shows `No matches`, in-flight state clears prior results.
   - Add (or extend existing) RootLayout tests: pressing `/` focuses
     the sidebar input when no input/textarea/contenteditable has
     focus; the modifier-key variants don't activate; the
     input-focused variant doesn't activate.

## Code References

### Server

- `skills/visualisation/visualise/server/src/api/mod.rs:1-46` — module
  declarations + router chain. Insert `mod search;` between lines 8/9
  and `.route("/api/search", get(search::search))` in the chain.
- `skills/visualisation/visualise/server/src/api/mod.rs:48-153` —
  `ApiError` enum + `IntoResponse`. No new variant needed for 0054.
- `skills/visualisation/visualise/server/src/api/mod.rs:155-157` —
  `parse_kind` (string → `Option<DocTypeKey>`); not needed for `/api/search`
  but documents the kebab-case parser if we ever need it.
- `skills/visualisation/visualise/server/src/api/docs.rs:19-41` —
  handler precedent.
- `skills/visualisation/visualise/server/src/docs.rs:4-20` —
  `DocTypeKey` enum (kebab-case wire).
- `skills/visualisation/visualise/server/src/docs.rs:134-157` —
  `DocType` struct (camelCase precedent).
- `skills/visualisation/visualise/server/src/indexer.rs:162-192` —
  `IndexEntry`.
- `skills/visualisation/visualise/server/src/indexer.rs:216` —
  `entries: Arc<RwLock<HashMap<PathBuf, IndexEntry>>>`.
- `skills/visualisation/visualise/server/src/indexer.rs:293-300` —
  Templates skipped at enumeration.
- `skills/visualisation/visualise/server/src/indexer.rs:532-540` —
  `all_by_type` (linear filter; not used by search).
- `skills/visualisation/visualise/server/src/indexer.rs:617-619` —
  `Indexer::all()` (story said 570-572).
- `skills/visualisation/visualise/server/src/indexer.rs:1444` —
  test asserting Templates absent from index counts (story said 1282).
- `skills/visualisation/visualise/server/src/server.rs:40-52` —
  `AppState`.
- `skills/visualisation/visualise/server/src/server.rs:77-79` —
  `Indexer::build` invocation.
- `skills/visualisation/visualise/server/tests/api_docs.rs:12-73` —
  integration-test pattern + 400-path examples.

### Frontend — api layer

- `skills/visualisation/visualise/frontend/src/api/fetch.ts:11-19` —
  `FetchError`.
- `skills/visualisation/visualise/frontend/src/api/fetch.ts:59-64` —
  `fetchTypes` (exemplar).
- `skills/visualisation/visualise/frontend/src/api/fetch.ts:87-92` —
  `fetchDocs` (`encodeURIComponent` exemplar).
- `skills/visualisation/visualise/frontend/src/api/query-keys.ts:40-62` —
  `queryKeys` object.
- `skills/visualisation/visualise/frontend/src/api/query-keys.ts:64-67` —
  `SESSION_STABLE_QUERY_ROOTS`.
- `skills/visualisation/visualise/frontend/src/api/query-client.ts:3-13` —
  `QueryClient` defaults (`staleTime: Infinity`, `retry: 1`).
- `skills/visualisation/visualise/frontend/src/api/use-related.ts:1-12` —
  hook pattern.
- `skills/visualisation/visualise/frontend/src/api/use-doc-content.ts` —
  alt hook pattern.
- `skills/visualisation/visualise/frontend/src/api/types.ts:4-8` —
  `DocTypeKey` union.
- `skills/visualisation/visualise/frontend/src/api/types.ts:14-19` —
  `DOC_TYPE_KEYS`.
- `skills/visualisation/visualise/frontend/src/api/types.ts:22-24` —
  `isDocTypeKey`.
- `skills/visualisation/visualise/frontend/src/api/types.ts:49-63` —
  `DOC_TYPE_LABELS`.

### Frontend — router

- `skills/visualisation/visualise/frontend/src/router.ts:97-107` —
  `libraryTypeRoute` + `parseParams` narrowing.
- `skills/visualisation/visualise/frontend/src/router.ts:109-113` —
  `libraryDocRoute`.
- `skills/visualisation/visualise/frontend/src/main.tsx:11-17` —
  `QueryClientProvider` wraps `RouterProvider`.

### Frontend — UI components

- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx:1-13` —
  current Sidebar shape.
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx:22-33` —
  TEMPORARY search input slot.
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx:132-145` —
  inline `SearchIcon` SVG.
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.module.css:23-90` —
  `.searchRow`, `.searchIcon`, `.searchInput`, `.kbd` styles.
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx:19-75` —
  current RootLayout body (no effects).
- `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx:52-56` —
  `<Sidebar>` composition.
- `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx:39-49,68` —
  `GlyphProps`, no `framed` default.
- `skills/visualisation/visualise/frontend/src/components/EyebrowLabel/EyebrowLabel.tsx:13` —
  framed-Glyph usage in per-doc-type context.
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.tsx:37-44` —
  client-side substring matcher.
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.tsx:62-69` —
  `<input type="search">` precedent.
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.tsx:93-97` —
  `role="status"` empty-state precedent.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx:13` —
  `DOC_TYPE_LABELS` import (the "docTypeMeta-style mapping" the story
  refers to).
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx:235-240` —
  `<Link to="/library/$type/$fileSlug" params={{ type, fileSlug }}>`.
- `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx:36-37` —
  kebab-case `'work-items'` as `type` param.
- `skills/visualisation/visualise/frontend/src/components/ActivityFeed/ActivityFeed.tsx:57` —
  inline `labelFor` helper (alt label source — not used by 0054 but
  documented for future consolidation).
- `skills/visualisation/visualise/frontend/src/components/ActivityFeed/ActivityFeed.tsx:117-122` —
  `<Link>` row pattern.

### Frontend — tests

- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx:18-28` —
  `vi.mock('../../api/fetch', ...)` pattern.
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx:98-121` —
  `renderSidebar` helper.
- `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx:253-258` —
  existing search-row assertion that must keep passing.
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.test.tsx:33-42` —
  `Wrapper` + `vi.spyOn` pattern.
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.test.tsx:163-168` —
  pending-state mock (never-resolving promise).
- `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleIndex.test.tsx:188-206` —
  substring-filter test (template for search behavioural tests).
- `skills/visualisation/visualise/frontend/src/test/router-helpers.tsx` —
  `MemoryRouter`, `renderWithRouterAt`, test-only `libraryDocRoute`.
- `skills/visualisation/visualise/frontend/src/test/router-helpers.test.tsx:14-18` —
  evidence that `<Link>` renders an `<a>` (role `link`).
- `skills/visualisation/visualise/frontend/src/api/use-related.test.tsx`,
  `use-doc-page-data.test.tsx`,
  `use-deferred-fetching-hint.test.tsx` — hook-test templates.

## Architecture Insights

### Indexer is the global-search backbone

The visualise indexer maintains `Vec<IndexEntry>` in memory under a
tokio `RwLock`, refreshed by a per-driver rescan plus a file watcher
that pushes incremental updates. Templates are excluded at enumeration
time and live in a separate `TemplateResolver` swapped via
`arc_swap::ArcSwap`. Any global query (search, activity, library
structure) reads from the same `entries` map, so `state.indexer.all()`
is the canonical entry point. The store does not maintain
type-indexed maps; `all_by_type` is a linear filter. For search, the
O(N) cost over a 200 ms client-side debounce is acceptable; a
sharded inverted index is future work, not 0054 scope.

### Server-side serde conventions

The wire-form contract is settled by two attribute patterns that
appear on every API DTO:
- enums: `#[serde(rename_all = "kebab-case")]` on `DocTypeKey`.
- structs: `#[serde(rename_all = "camelCase")]` on `IndexEntry`,
  `DocType`, and (by convention for 0054) `SearchResultRow`.

The Rust-keyword fields use raw-identifier escapes: `r#type` →
`"type"`, `r#virtual` → `"virtual"`. New row structs should follow
the same pattern.

### `ApiError` is a flat enum, one variant per failure mode

`ApiError` (`api/mod.rs:48-153`) is the single failure type for the
HTTP layer. Each variant emits a `Json({"error": "..."})` body and a
specific status code. The 0054 endpoint doesn't introduce a new
failure mode — empty `q` returns 200 with empty results — so we leave
the enum unchanged.

### Frontend: React Query is the cache, SSE is the invalidator

The README-grade rationale lives in
`frontend/src/api/query-client.ts:6-12` as a comment: SSE
(`useDocEvents`) invalidates `docs`, `docContent`, `lifecycle`,
`kanban` caches on real file changes; therefore the default
`staleTime: Infinity` makes all reads cache-permanent until SSE pokes
them. Search results don't need a dedicated SSE invalidation channel
— a 5-minute `gcTime` (the React Query default) is fine because (a)
the dedup AC's window is ~1 second; (b) old result sets falling out
of cache after 5 minutes is the right behaviour anyway.

### Frontend: `<Link>` is the canonical navigator

Every doc-row in the app uses `<Link to="/library/$type/$fileSlug">`
with kebab-case `$type`. `useNavigate`/`router.navigate` is reserved
for breadcrumbs (the only consumer). For search results, the
type-safe `<Link>` is the obvious choice and produces the `<a href>`
the AC requires.

### Frontend: hooks live under `frontend/src/api/`

Every fetch helper and `useX`-style hook is colocated in
`frontend/src/api/` (kebab-case filenames, camelCase exports). There
is no `frontend/src/hooks/` directory. `useDebouncedValue`,
`useSearch`, and `fetchSearch` should all land there.

### Frontend: testing has no global render helper

Each test suite assembles its own provider stack — sometimes via a
`renderSidebar()` helper, sometimes via a `Wrapper` component.
Per-test `vi.spyOn` over `../../api/fetch` is the dominant mocking
strategy (no MSW). For hook tests that involve timers (debounce),
`vi.useFakeTimers()` + `vi.advanceTimersByTimeAsync(...)` is required.

### CSS uses design tokens; no bespoke values in 0054 territory

Sidebar styles compose `--ac-*` tokens (sp, radius, fg, bg, stroke,
font-family, font-size, line-height). The results panel CSS for
0054 should consume the same tokens — no raw hex / px in
content-layout positions.

## Historical Context

### Sibling deliverables — the bundled epic 0036

- **0036 (parent epic)** — sidebar redesign rationale and parent-level
  technical notes.
- **0053 (done)** — Sidebar layout + the unwired search input slot
  + the `/` hint chip. Created the `searchRow` / `searchInput` / `kbd`
  CSS classes 0054 wires.
- **0055 (done)** — Activity feed. Already shipped — no merge-window
  coordination concern with 0054.
- **0037 (done)** — Glyph component. The result-row icon source.

Research backing these:
- `meta/research/codebase/2026-05-12-0053-sidebar-nav-and-unseen-tracker.md` —
  decisions taken when the search slot was introduced.
- `meta/research/codebase/2026-05-13-0055-sidebar-activity-feed.md` —
  reference for an in-Sidebar dynamic-content panel.
- `meta/research/codebase/2026-05-12-0037-glyph-component.md` —
  Glyph API rationale.
- `meta/research/codebase/2026-05-07-0035-topbar-component.md` —
  ambient topbar context referenced in the parent epic.

### Coordinates with 0083 (DevDesignSystem keybind)

0083 (draft) plans to register `Cmd/Ctrl+Shift+D` globally — no
conflict with 0054's `/` (different key and modifier-state). If a
second consumer ever justifies a shared `useHotkey` primitive, 0083
is the natural collaborator. Not in 0054 scope.

### Design source / gap document

- `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
  identifies the persistent search as a gap relative to the
  prototype.
- A more recent companion exists at
  `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`.

### Prototype on disk

The Claude design prototype's inventory references `src/search.jsx`
but no such file is committed. Only `inventory.md`,
`prototype-standalone.html`, and `screenshots/` are present at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/`.
The inventory description (lines 392-400 for `Highlight`, `SearchBox`,
`useSearch`; line 406 for `Sidebar` integration) plus the rendered
prototype HTML are the design reference; the AC supersedes the
prototype where they disagree.

### ADRs — none directly applicable

A pass through `meta/decisions/` found no ADR scoping sidebar
behaviour, search, or keybinds. Incidental string matches in
ADR-0026 (CSS token application) and ADR-0038 (validation params) are
unrelated.

## Related Research

- [meta/research/codebase/2026-05-12-0053-sidebar-nav-and-unseen-tracker.md](2026-05-12-0053-sidebar-nav-and-unseen-tracker.md) — Sidebar layout + search input slot
- [meta/research/codebase/2026-05-13-0055-sidebar-activity-feed.md](2026-05-13-0055-sidebar-activity-feed.md) — Sibling activity feed
- [meta/research/codebase/2026-05-12-0037-glyph-component.md](2026-05-12-0037-glyph-component.md) — Glyph dependency
- [meta/research/codebase/2026-05-07-0035-topbar-component.md](2026-05-07-0035-topbar-component.md) — Topbar / surrounding chrome
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/inventory.md` — Prototype inventory (search section lines 392-400)
- `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md` — Source design-gap doc

## Open Questions

- **Doc-type label casing for result rows**: `EyebrowLabel`
  uppercases `DOC_TYPE_LABELS[type]`; `LibraryTypeView` page titles
  use sentence case. The AC says "the doc-type label" without
  pinning casing. Default to sentence case (matches the page-title
  precedent) unless a design review prefers uppercase eyebrows for
  consistency with library cards.
- **Result-panel visual relationship to `.searchRow`**: 0053's
  `.searchRow` has `border-radius: var(--radius-md)` and a stroked
  border. Should the results panel join the same rounded frame
  (creating a popover-like attached panel), or stand below as a
  separate block inside the sidebar gap? Design call — both
  satisfy the AC.
- **Behaviour when results contain a slugless entry**: the AC
  permits omitting slugless rows from results. Implementer may
  choose between (a) the server filtering them out before the
  buckets, or (b) the client skipping them at render. Server-side
  is simpler — title/slug/body-preview match buckets already exclude
  the slug-keyed buckets for `slug = None`, and the body-preview
  bucket can be made to drop slugless rows too. Either is consistent
  with the AC; the choice affects whether `mtimeMs` ordering reflects
  non-rendered entries.
- **Test-clock granularity for the debounce-dedup AC**: with
  `vi.useFakeTimers()`, advancing by exactly 200 ms hits the
  trailing edge — but verifying "exactly one network request for
  `q=ab`" across an `ab → abc → ab` sequence requires careful
  timer-advance pacing. The implementer should make sure the
  hook's `useEffect([value, delayMs])` cleanup truly cancels the
  prior `setTimeout` before scheduling a new one (it does — `return
  () => clearTimeout(id)` runs before the next effect).
