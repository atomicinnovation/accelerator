---
work_item_id: "0054"
title: "Sidebar Search Input and API Search Endpoint"
date: "2026-05-11T12:11:50+00:00"
author: Toby Clemson
type: story
status: ready
priority: high
parent: "0036"
tags: [design, frontend, chrome, navigation]
---

# 0054: Sidebar Search Input and API Search Endpoint

**Type**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a user navigating the visualisation app, I want a persistent search box in the Sidebar with a `/` keybind, so I can find documents across all twelve LIBRARY doc types without leaving my current view.

This story adds a new server endpoint `GET /api/search?q=<query>` returning matches across the twelve LIBRARY doc types, and wires the Sidebar search input slot created by 0053: a `/` keybind activator (early-returning while a text input, textarea, or contenteditable element has focus), a 200 ms debounced `useSearch` hook with 2-character minimum, inline result rendering, and a no-match empty state.

## Context

Child of 0036 — Sidebar Redesign. 0036 is the bundled epic; this story owns the search input behaviour and the `/api/search` backend. The Sidebar layout, search input slot, and `/` hint chip are delivered by 0053 (sibling). The activity feed lands in 0055 (sibling).

The "twelve LIBRARY doc types" referenced throughout this work item are the twelve non-`Templates` variants of `DocTypeKey`, defined in `server/src/doc_type.rs` (the canonical enum used across the indexer, API, and frontend type bridge). Any change to that enum's variant set must also revisit this story's endpoint contract.

There is no existing `/api/search` route in `server/src/api/mod.rs:22-42`, no client fetch helper in `api/fetch.ts`, and no query-key entry in `api/query-keys.ts`. The pre-existing `<input type="search">` in `LifecycleIndex.tsx:75-82` is a pure client-side substring filter over already-loaded data and is unrelated to the new Sidebar search feature introduced by this story.

## Requirements

This story owns the search backend endpoint and the Sidebar search input behaviour.

- Add a new server route `GET /api/search?q=<query>` registered in `server/src/api/mod.rs:22-42`, returning matches across the twelve LIBRARY doc types (every `DocTypeKey` variant except `Templates`). Response shape: `{ results: Array<{ docType: DocTypeKey, title: string, slug: string, path: string, mtime_ms: number }> }`. Results are ordered server-side by `mtime_ms` descending (most recent first); the field is returned so clients can verify ordering and so future stories can introduce client-side re-ranking without a contract change.
- Wire the Sidebar search input slot (delivered by 0053) with a global `/` keydown listener in `RootLayout.tsx` (hereafter "the keybind listener") that focuses the Sidebar search input. The keybind listener early-returns (does not steal focus or call `preventDefault`) while a text input, textarea, or contenteditable element has focus.
- Implement a `useSearch(query)` hook with 200 ms client-side debounce and 2-character minimum length. The React Query cache key is `queryKeys.search(<settled-query>)`, where `<settled-query>` is the trimmed query value present 200 ms after the most recent keystroke (not the raw input value).
- Add a new query-key `queryKeys.search(query)` entry in `frontend/src/api/query-keys.ts` and a new fetch helper `fetchSearch(query)` in `frontend/src/api/fetch.ts`.
- Render results inline beneath the search input as `<a href>` link elements (one per result), each row showing the per-doc-type Glyph, the document title, and the doc-type label. Native browser link semantics apply — modifier-click / middle-click open in a new tab, keyboard Enter on a focused row navigates.
- Render a no-match empty state when `results` is an empty array, as an element with `role="status"` containing the literal text `No matches` (see Acceptance Criteria for the queryable signal).
- State ownership: `Sidebar.tsx` owns the controlled query input state and the results panel state. `RootLayout.tsx` owns the global `keydown` listener and the `searchInputRef`, passed into `Sidebar` by prop (no new context).

## Acceptance Criteria

- [ ] Given the user presses `/` and no text input, textarea, or contenteditable element currently has focus, when the keydown fires, then focus moves to the Sidebar search input and the `/` character is not inserted into any field.
- [ ] Given a text input, textarea, or contenteditable element currently has focus, when the user presses `/`, then the `/` character is inserted normally into that element and Sidebar search focus does not change.
- [ ] Given the user types into the search input, when the trimmed query value present 200 ms after the most recent keystroke (the "settled query") has length ≥ 2, then `GET /api/search?q=<settled-query>` is called exactly once per distinct settled-query value. The debounce is trailing-edge: every keystroke resets the 200 ms timer, and the request fires only once no further keystrokes have arrived for 200 ms. Concrete examples: (a) typing `ab`, deleting to `a`, then retyping `ab` while the React Query cache entry for `queryKeys.search('ab')` is still live yields exactly one network request for `q=ab` (the second occurrence is a cache hit); (b) typing `ab`→`abc`→`ab` with each keystroke ≤ 100 ms after the prior yields exactly one network request for `q=ab` (and zero requests for `q=abc`, because `abc` was never the settled value for 200 ms).
- [ ] Given `GET /api/search` responds with `{ results: Array<{ docType: DocTypeKey, title: string, slug: string, path: string, mtime_ms: number }> }` and the response is non-empty, when the response arrives, then results are rendered inline beneath the input within the same React render in which the response resolves, with each row showing the per-doc-type Glyph, the document title, and the doc-type label.
- [ ] Given the response contains matches with different `mtime_ms` values, when results render, then rows appear in the same order as the response array (which is ordered by `mtime_ms` descending server-side). For deterministic verification, ties on `mtime_ms` are broken by `path` ascending (server-side); fixtures may rely on this secondary key.
- [ ] Given a rendered result row, when the user clicks it (plain click), then the browser navigates to the document detail view for that row's `docType` + `slug`. The row is rendered as an `<a href>` element so that native link semantics apply: modifier-click and middle-click open a new tab, and pressing Enter while the row is keyboard-focused triggers the same navigation as a plain click.
- [ ] Given the user types fewer than 2 characters into the search input, when 200 ms elapses, then no `/api/search` request is issued and no results are rendered.
- [ ] Given the user types a query that returns no matches, when `/api/search` responds with `results: []`, then the results area shows an empty-state element with `role="status"` containing the literal text `No matches`, queryable via `getByRole('status')`.
- [ ] Given a settled query of length ≥ 2 has triggered a request that has not yet resolved, when the results area renders, then it does not show the `No matches` element and does not show stale rows from a previous query; the previous results are cleared on every new settled query, and the only allowed transitional content is an empty results area.
- [ ] Given `GET /api/search` responds with a non-2xx status or the network request fails, when the failure is observed, then the results area is cleared (no stale rows, no `No matches` element) and the failure is logged via `console.error` with a `FetchError` instance whose message includes the request URL (consistent with the existing `FetchError` handling in `frontend/src/api/fetch.ts`). No user-visible error banner is required by this story.
- [ ] Given the indexer contains entries across LIBRARY doc types plus Templates, when `GET /api/search?q=<title-substring>` is called server-side with a substring matching at least one entry in every LIBRARY type and at least one Templates entry, then the response is `{ results: [...] }` containing matches from the LIBRARY types and zero entries with `docType: Templates`.
- [ ] Given `q` is absent or an empty string, when `GET /api/search` is called, then the response is `{ results: [] }` with HTTP 200 (no error, no 4xx).

## Open Questions

- What ranking/matching strategy should `/api/search` use across doc types (substring, fuzzy, full-text rank, recency boost), and does any one type — e.g. work items by ID — need an exact-match shortcut? (Endpoint URL, request, and response shape are pinned; only the internal ranking algorithm remains open.)

## Dependencies

- Part of: 0036 (Sidebar Redesign epic) — one of three child deliverables (alongside 0053 layout and 0055 activity feed).
- Blocked by: 0037 (Glyph component used in result rows), 0053 (Sidebar layout and search input slot).
- Coordinates with: 0055 (Activity Feed) — both stories add features to `Sidebar.tsx`. This story introduces the controlled search input state and results panel; 0055 introduces the activity feed section. No hard ordering constraint, but mergers should expect minor `Sidebar.tsx` integration if both stories land in the same window.
- Blocks: none currently tracked. The endpoint contract is coupled to the `DocTypeKey` enum (see Context) — any future story that extends, renames, or restructures `DocTypeKey` must revisit `/api/search` filtering and response typing. No follow-up story is currently scheduled to revisit the deferred ranking algorithm (see Open Questions); if the open question resolves into work that warrants its own story, it will be tracked at that point. No current backlog item depends on the net-new `useDebouncedValue` helper or global `/` keybind infrastructure introduced by this story.

## Assumptions

- The `/api/search` endpoint is implemented within this story (not coordinated as parallel backend work).
- A client-side debounce of 200 ms with a 2-character minimum is sufficient to keep request rate acceptable without a dedicated rate limiter on the server.

## Technical Notes

**Size**: M — server route + handler (small, well-precedented by api/docs.rs), useSearch + fetchSearch + queryKeys.search (small, well-precedented), useDebouncedValue helper (net-new but ~10 lines), global `/` keybind via RootLayout's first useEffect (net-new infra, small), Sidebar gains its first local state + results panel (small-medium). Each piece is contained; the M rating reflects that two pieces of frontend infrastructure are net-new (debounce, hotkey) rather than any single piece being large.

- **Frontend wiring**:
  - New `useSearch(query: string)` hook: internally pipes the raw `query` argument through `useDebouncedValue(query.trim(), 200)`, then uses `useQuery` with `queryKey: queryKeys.search(debouncedQuery)` and `queryFn: () => fetchSearch(debouncedQuery)`, `enabled: debouncedQuery.length >= 2`. The cache key is always the settled (trimmed, debounced) value — not the raw input — so the dedup behaviour in AC3 follows directly from React Query's default key-based caching. Add `search: (q: string) => ['search', q] as const` to `queryKeys` in `skills/visualisation/visualise/frontend/src/api/query-keys.ts:3-22` and `fetchSearch(q: string): Promise<SearchResult[]>` (where `SearchResult` matches the response row shape including `mtime_ms`) in `frontend/src/api/fetch.ts` following the `fetchTypes` exemplar (lines 56-61): bare `fetch()`, throw `FetchError` on `!r.ok`, parse inline-typed body, return inner field.
  - **Debounce: net-new infrastructure.** No `useDebounce` / `useDebouncedValue` exists anywhere in `frontend/src`. Introduce a named `useDebouncedValue(value, delayMs)` helper (≈10 lines using `useState` + `useEffect` + `setTimeout`/`clearTimeout`) co-located with `useSearch`. Keep it as a named exported helper rather than inlined — this single shape pre-empts confusion in review.
  - **Infrastructure follow-up posture.** Both `useDebouncedValue` and the global `/` keybind are introduced as small, single-consumer primitives in service of this feature (not as a generic platform layer). If a second consumer for either appears during implementation, split the generalisation out as a follow-up chore rather than expanding scope inside this story.
  - **`/` keybind listener: net-new infrastructure.** No `useHotkey`, `useKeybind`, or global `keydown` listener exists anywhere in `frontend/src`. Add the keybind listener as a global `keydown` `useEffect` in `RootLayout.tsx` — this will be the **first `useEffect` in that file** (RootLayout currently has zero effects and zero event listeners). The keybind listener checks `document.activeElement` against `{ INPUT, TEXTAREA }` tags and `isContentEditable` before stealing focus; on `/` (and no modifier keys — Cmd, Ctrl, Alt, or Meta held suppresses activation and lets the default browser behaviour proceed), `preventDefault()` and focus the Sidebar search input via a shared ref. **Plumbing decision**: `RootLayout` owns the `searchInputRef` and the keybind listener, and passes the ref into `<Sidebar>` by prop. No new context is introduced (deferred until a second consumer exists).
- **Sidebar input host**: `Sidebar.tsx` is purely presentational today with no local state. This story introduces the first state hook in this component — `Sidebar` owns the controlled query string and the rendered results panel state; `RootLayout` retains ownership of the focus ref and global listener (see plumbing decision above). The `LifecycleIndex.tsx:75-82` precedent uses `<input type="search">` with `aria-label` and `placeholder` but is purely synchronous (no debounce, no 2-char minimum) — reuse the markup conventions, not the filter loop.
- **`/` hint chip**: the chip rendered in the search input slot is delivered and owned by 0053. This story does not modify the chip's appearance, visibility, or dynamic behaviour — the chip is purely informational visual state owned by 0053.
- **Server route**:
  - Register `GET /api/search` in `skills/visualisation/visualise/server/src/api/mod.rs:22-42` — add `mod search;` and `.route("/api/search", get(search::search))` to the existing chain. The Router is `Router<Arc<AppState>>` (line 22).
  - **Handler precedent**: model on `api/docs.rs:30-33` — `pub(crate) async fn search(State(state): State<Arc<AppState>>, Query(q): Query<SearchQuery>) -> Result<Response, ApiError>` — `docs_list` is the closest precedent because it combines `State` + `Query` and returns `Result<Response, ApiError>`. `SearchQuery` is a private struct with `q: String`.
  - **Index access**: read `state.indexer.all()` (`indexer.rs:326`) — the natural entry point for global search. There is **no `entries_by_type` map** on the Indexer; the primary store is `entries: HashMap<PathBuf, IndexEntry>` keyed by path (`indexer.rs:58`), and `all_by_type` (`indexer.rs:316-324`) is itself a linear filter. `IndexEntry` (`indexer.rs:17-34`) carries `title`, `slug: Option<String>`, `path`, `rel_path`, `mtime_ms`, and `body_preview` — all reachable as ranking inputs without extending the entry shape.
  - **Templates exclusion**: Templates are owned by `AppState.templates` (separate store, not the regular Indexer), so `Indexer::all()` already excludes them by structural omission rather than an explicit filter. No `kind != Templates` predicate is required.
  - **Initial ranking strategy** (open question scope): case-insensitive substring match on `title` and `slug`, ordered by `mtime_ms` descending. `body_preview` matching is out of scope at first cut; revisit in the Open Question resolution.
- **Search input slot**: delivered by 0053 — `<input type="search">` slot with `/` hint chip already in DOM. This story wires `onChange`, `onKeyDown`, the global `/` listener, and the results panel beneath it.
- **No `api/activity.rs`**: if you look for sibling handler precedents, `api/lifecycle.rs` and `api/docs.rs` are the right examples — there is no `server/src/api/activity.rs` (activity is a non-HTTP module at `server/src/activity.rs`).

## Drafting Notes

- Extracted from 0036 (Sidebar Redesign) as part of decomposing that bundled story into three deliverable units. See 0036 for the full design rationale and parent-level Technical Notes.

## References

- Parent: `meta/work/0036-sidebar-redesign.md`
- Source: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Related: 0037, 0053, 0055
