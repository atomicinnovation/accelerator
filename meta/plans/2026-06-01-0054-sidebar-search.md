---
date: "2026-06-01T20:45:00+01:00"
type: plan
skill: create-plan
work_item_id: "0054"
status: accepted
---

# 0054 Sidebar Search Input and API Search Endpoint — Implementation Plan

## Overview

Wire the temporary sidebar search input slot (delivered by 0053) to a new
backend `GET /api/search` endpoint, behind a `/` global keybind, with a
200 ms client-side debounce and a 2-character minimum. Introduce two
single-consumer frontend primitives — `useDebouncedValue` and the
RootLayout `/` keydown listener — and a new `useSearch` hook that returns
ranked results from the indexer snapshot.

The plan is structured around **test-driven development** and
**independent phases**: the server endpoint, the debounce primitive, the
keybind primitive, and the api-layer hooks can all be built and merged
independently (each pinned to the work-item contract). A final Sidebar
phase composes them into the rendered results panel.

## Current State Analysis

- The Sidebar search input slot is rendered at
  `frontend/src/components/Sidebar/Sidebar.tsx:22-33` as an uncontrolled
  `<input type="search">` with a `TEMPORARY` comment naming this work
  item (delivered by 0053).
- No `/api/search` route exists in `server/src/api/mod.rs:24-46`; no
  `fetchSearch` helper, no `queryKeys.search` entry, no `useSearch` hook,
  no `useDebouncedValue` helper, no global `/` keybind.
- `RootLayout` has zero `useEffect`s and zero event listeners
  (`frontend/src/components/RootLayout/RootLayout.tsx:19-75`).
- `Indexer::all()` at `server/src/indexer.rs:617-619` yields a
  `Vec<IndexEntry>` clone of the global entries map; Templates are
  structurally absent (asserted by `indexer.rs:1444`).
- `IndexEntry` (`indexer.rs:162-192`) carries `title`, `slug:
  Option<String>`, `path`, `rel_path`, `mtime_ms`, `body_preview` —
  every field the AC's ranking needs is reachable without extending the
  shape.
- `<Link to="/library/$type/$fileSlug">` from Tanstack Router v1
  renders an `<a href>` element (verified by
  `test/router-helpers.test.tsx:14-18`) — the AC's `<a href>`
  requirement is satisfied by `<Link>`.
- Doc-type labels live in `DOC_TYPE_LABELS`
  (`frontend/src/api/types.ts:49-63`).
- `QueryClient` defaults: `staleTime: Infinity, retry: 1`, no `gcTime`
  override (React Query default 5 min)
  (`frontend/src/api/query-client.ts:3-13`).
- Testing convention: per-suite provider stacks, `vi.spyOn` over
  `../../api/fetch`, no MSW, no global `renderWithProviders`.

For full file/line evidence and a deep dive into every precedent, see
the companion research at
`meta/research/codebase/2026-06-01-0054-sidebar-search.md`. The plan
deliberately points back to that document rather than re-listing every
reference inline.

## Desired End State

Pressing `/` (outside of any text field) focuses the sidebar search
input. Typing 2+ characters issues a debounced `GET /api/search?q=...`
request 200 ms after the last keystroke. Results render inline beneath
the input as `<Link>` rows showing per-doc-type Glyph + title + label,
in the server's four-bucket case-insensitive ranking order. An empty
result set shows a `role="status"` element with the literal text "No
matches"; a sub-2-character settled query renders nothing. Fetch errors
clear the panel and log a `FetchError` (whose message contains
`/api/search`) via `console.error`. All thirteen acceptance criteria
pass with deterministic tests.

### Key Discoveries

- **`<Link>` satisfies the `<a href>` requirement** —
  `frontend/src/test/router-helpers.test.tsx:14-18` proves it; using
  `<Link>` instead of a raw anchor also gets type-safe params and
  client-side navigation interception for free.
- **Templates exclusion is structural** —
  `server/src/indexer.rs:293-300` skips Templates at enumeration, so
  `Indexer::all()` never yields them. The AC's defensive assertion (no
  `docType === "templates"` in results) holds without a handler-level
  predicate.
- **`ApiError` needs no new variant** — the AC requires HTTP 200 +
  empty results for absent/empty `q`. There is no 4xx path on the
  spec'd happy path.
- **Doc-type label mapping is `DOC_TYPE_LABELS`** — there is no
  `docTypeMeta` module; the story's "docTypeMeta-style mapping" should
  be read as `DOC_TYPE_LABELS` from `frontend/src/api/types.ts:49-63`.
- **The work item's path references need correcting** —
  `frontend/src/Sidebar/...` should be `frontend/src/components/Sidebar/...`;
  `Indexer::all()` is at lines 617-619, not 570-572. The plan uses the
  correct paths.

## What We're NOT Doing

- **Match highlighting on result rows** — the prototype's `Highlight`
  component is out of scope; the AC requires only Glyph + title +
  doc-type label.
- **Arrow-key navigation between results and Enter-to-follow-first** —
  not in any AC. Standard for `/`-driven palettes (GitHub, DocSearch,
  VS Code Quick Open) and the natural next-step UX, but adding it
  expands scope (new ACs, new tests, focus management). Deferred to
  a follow-up story; v1 relies on native Tab traversal + Enter on a
  focused link row (AC6's "Enter while keyboard-focused triggers the
  same navigation as a plain click" already covers the Enter path).
- **User-visible error UI on fetch failure** — the AC pins
  `console.error` + cleared panel only. The blank-panel UX is a
  conscious v1 trade-off; if production usage shows users hit this
  state without diagnosis, file a follow-up to add a minimal
  `role="status"` "Search unavailable" affordance mirroring the empty
  state.
- **Live-region announcement throttling** — the empty-state
  `role="status" aria-live="polite"` element will re-announce on every
  settled empty result. Acceptable for v1; if production AT users
  complain, a follow-up can debounce the announcement separately from
  visual rendering.
- **Server-side fuzzy matching, weight tuning, or recency
  re-ranking** — the four-bucket case-insensitive strategy is the
  pinned first cut; revisit only if production usage surfaces issues.
- **Extracting a shared `useHotkey` / `useKeybind` primitive** — the
  `/` listener stays inline in `RootLayout` per "Infrastructure
  follow-up posture" in the work item. Same posture for
  `useDebouncedValue` (kept as a small, single-consumer helper).
- **A new React Context, module-level singleton, or other shared
  object for the search input ref** — `RootLayout` passes the ref via
  prop into `Sidebar`.
- **User-visible error banner / toast on fetch failure** — the AC
  requires only `console.error` logging plus a cleared panel.
- **Re-styling the existing `.searchRow`, `.searchInput`,
  `.searchIcon`, or `.kbd`** — those are owned by 0053. The plan adds
  new sibling-block CSS for the results panel only.
- **Modifying the `/` hint chip behaviour** — owned by 0053; this
  story makes no changes to it.
- **Server-side rate limiting** — the 200 ms debounce + 2-char minimum
  is sufficient per the story's Assumptions.
- **Index restructuring for search performance** — `Indexer::all()`'s
  O(N) snapshot is acceptable given the debounce; a sharded inverted
  index is future work.
- **Slug-less entry rendering** — slug-less LIBRARY entries are
  filtered server-side (see Design Decisions). They do not surface in
  any bucket.

## Implementation Approach

Each phase ships independently with its own TDD cycle. Phases 1–4 are
pairwise non-blocking (they touch disjoint files); Phase 5 integrates
them in the Sidebar.

```
[Phase 1: /api/search backend]   [Phase 2: useDebouncedValue]
        |                                 |
        v                                 v
[Phase 4: useSearch + fetchSearch + queryKeys.search]
        |
        +--<-- [Phase 3: RootLayout / keybind + searchInputRef plumbing]
        |
        v
[Phase 5: Sidebar results wiring + empty/error states]
```

**Dependency notes**:

- Phase 4 (api-layer) consumes Phase 2 (`useDebouncedValue`) and pins
  to Phase 1's response shape — but the shape is fully specified by
  the work item, so Phases 1, 2, 4 can land in any order in practice
  (Phase 4 unit-tests mock `fetchSearch`'s return type).
- Phase 3 (keybind + ref plumbing) is fully independent of 1, 2, 4 —
  it adds a ref attachment to the existing temporary input and a
  global listener. Sidebar gains a new prop but no state changes.
- Phase 5 (Sidebar integration) is the only phase that depends on the
  others; it composes them into the rendered panel.

### Design Decisions

1. **Doc-type label casing**: sentence case via `DOC_TYPE_LABELS[result.docType]`
   directly (matches `LibraryTypeView` page-title precedent).
2. **Results panel framing**: results render as a separate block
   sibling to `.searchRow` inside `.sidebar` (uses existing `gap:
   var(--sp-4)`). No joined-frame popover.
3. **Slug-less entry handling**: filtered server-side before
   bucketing. The work item AC line 60 says "Rows where `slug` is
   `null` are not rendered as result links (such entries cannot
   satisfy the slug-based ranking buckets either)" — admits two
   interpretations: filter entirely, or render as non-link rows. The
   plan picks filter-entirely because (a) slug-less LIBRARY entries
   would otherwise produce unclickable rows with no recovery path,
   and (b) in practice the indexer populates `slug` from filename for
   every LIBRARY entry so the `Option<String>` is type-system
   defensive. The filter is enforced through a single typed
   constructor (see Design Decision 6) so future bucketing changes
   cannot silently bypass it.
4. **Result row component**: `<Link to="/library/$type/$fileSlug"
   params={{ type, fileSlug }}>` (renders `<a href>` via Tanstack
   Router) rather than a raw `<a href={template}>`. Type-safe, matches
   the canonical app pattern (kanban, library, lifecycle, activity
   feed). Slugs are filename-derived kebab-case `[a-z0-9-]` in
   practice; Tanstack Router URL-encodes per-segment, so non-ASCII or
   path-separator-bearing slugs would round-trip safely but not match
   any server doc.
5. **`searchInputRef` ownership**: created in `RootLayout` (alongside
   the keydown effect) and passed into `<Sidebar>` by required prop.
   No new context, no module singleton. The prop is required (not
   optional) on `Sidebar` once Phase 5 lands so the type system
   guarantees the keybind has a focus target in every render path.
6. **`SearchResultRow` server struct + typed projection**: minimal
   projection of `IndexEntry` (`docType`, `title`, `slug`,
   `mtimeMs`) with `#[serde(rename_all = "camelCase")]`. `path` is
   deliberately omitted from the wire payload — it discloses the
   server's absolute filesystem path (canonicalised via
   `tokio::fs::canonicalize`) and is unused on the client (React row
   keys derive from `${docType}/${slug}`). The wire field `mtimeMs`
   is the camelCase serialisation of the Rust field `mtime_ms` under
   `rename_all = "camelCase"`; the work item's literal `mtime_ms`
   notation in §Requirements/§AC is the schema field, the wire form
   is `mtimeMs`. `SearchResultRow` is constructed only via a private
   `fn project(entry: &IndexEntry) -> Option<SearchResultRow>` that
   returns `None` for slug-less entries — every bucketing path
   funnels through this projector so the slug invariant is enforced
   at the type level.
7. **`q` query extractor shape and length cap**: `SearchQuery {
   #[serde(default)] q: String }` — tolerates missing `q` (returns
   `""`) per the AC's "200 + empty results for absent or empty `q`".
   No new `ApiError` variant needed. A server-side length cap
   (`const MAX_Q_LEN: usize = 128`) short-circuits over-length input
   to empty results, defending against amplification attacks that
   bypass the client debounce.
8. **In-flight rendering uses `placeholderData: keepPreviousData`**:
   `useSearch` configures `placeholderData: keepPreviousData` so the
   previous query's results stay visible while the next request is
   in flight, with `search.isPlaceholderData === true` flagging the
   transitional state. This avoids the visible flicker that a
   clear-to-empty transition produces during typing — the next
   query's results swap in when they resolve, no blank intermediate.
   Empty-state is rendered only when the *current* query has
   resolved to an empty array: `search.isSuccess === true &&
   search.data?.length === 0 && !search.isPlaceholderData`.

   **AC9 alignment**: AC9 was amended (work item edit 2026-06-01) to
   permit two transitional shapes: (a) an empty results area, or
   (b) the rows from the immediately prior settled query held as
   React Query placeholder data, distinguishable via
   `search.isPlaceholderData === true`. The plan implements shape
   (b) via `keepPreviousData`. The AC still forbids `No matches`
   flashing mid-transition and rows from an arbitrarily-older
   settled query — both prohibitions hold because the empty-state
   condition above only fires for the resolved current query, and
   the placeholder is *immediately prior* (one key back), not
   arbitrary stale.

   This is also more precise than the imprecise "React Query
   auto-clears `data` for the new key" reasoning in the original
   plan: React Query returns the cache slot for the new key, which
   may be cached data, `undefined`, or — with `keepPreviousData` —
   the prior key's data as a placeholder.
9. **`AbortSignal` plumbing**: `fetchSearch(q, signal?)` accepts an
   optional `AbortSignal`; `useSearch` threads React Query's
   `queryFn` context signal so a key change cancels the in-flight
   request rather than letting an overtaken query write to the
   cache. Closes the out-of-order-completion observation under fast
   typing on a slow link.
10. **Error logging path**: `console.error` is called from
    `fetchSearch`'s catch path (re-throws after logging) rather than
    from a `useEffect` in Sidebar. Keeps logging coupled to the
    request lifecycle (exactly one log per failed network round-trip)
    rather than the render lifecycle (potential duplicate logs across
    retries/remounts). Sidebar/SearchResultsPanel only consumes
    `search.isError` for UI.
11. **Second-hotkey refactor target**: the inline `/` keydown listener
    in `RootLayout` is the deliberate v1 shape — net-new infra, single
    consumer. When a second hotkey lands (e.g. `?` for help, `g l`
    for go-to-library), the expected refactor is a
    `HotkeyRegistryContext` with `useEffect`-based registration. The
    listener body is structured around two extracted pure helpers
    (`isPlainSlashKey`, `isEditableTarget`) to keep the predicate
    portable.
12. **`useSearch` returns the raw `useQuery` result**: matches
    `useRelated`/`useTypes`. Consumers (currently only
    `<SearchResultsPanel>`) destructure the fields they need
    (`data`, `isFetching`, `isError`, `isSuccess`, `isPlaceholderData`).
    The hook does not override `retry` or `staleTime`; both inherit
    the app-wide `QueryClient` defaults (`staleTime: Infinity, retry:
    1`) consistent with the work item's Technical Note "Use the
    global QueryClient defaults". Tests pin `retry: false, staleTime:
    Infinity` on their per-suite `QueryClient`.

---

## Phase 1: Server endpoint `GET /api/search`

### Overview

Add the `/api/search` route, handler, response types, and bucket-and-rank
logic. Templates are excluded structurally via `Indexer::all()`;
slug-less entries are filtered server-side before bucketing. Empty or
missing `q` short-circuits to a 200 with empty results.

### Changes Required

#### 1. Handler module — new file

**File**: `skills/visualisation/visualise/server/src/api/search.rs`
**Changes**: New module with `SearchQuery`, `SearchResultRow`,
`SearchResponse`, `Bucket` enum, `classify`, `project`, and `search`
handler.

- `const MAX_Q_LEN: usize = 128;` — server-side hard cap on trimmed
  `q` length; over-cap input short-circuits to empty results
  (Design Decision 7).
- `SearchQuery { #[serde(default)] q: String }` (private struct).
- `SearchResultRow { #[serde(rename_all = "camelCase")] doc_type:
  DocTypeKey, title: String, slug: String, mtime_ms: i64 }` — `path`
  is omitted from the wire payload (Design Decision 6).
- `SearchResponse { results: Vec<SearchResultRow> }`.
- `#[repr(u8)] enum Bucket { ExactSlug = 0, Prefix = 1, Interior = 2,
  Body = 3 }` — total order via discriminant.
- `fn classify(entry: &IndexEntry, q_lc: &str) -> Option<Bucket>` —
  pure function. Returns `Some(ExactSlug)` if the entry has a slug
  equal to `q_lc` (ASCII-lowercase comparison); else `Some(Prefix)`
  if title or slug starts with `q_lc`; else `Some(Interior)` if
  title or slug contains `q_lc`; else `Some(Body)` if `body_preview`
  contains `q_lc`; else `None`. Slug-less entries (`slug == None`)
  are treated as substring-free of `q_lc` in slug for buckets
  ExactSlug/Prefix/Interior; they may still match `Body`, but
  `project` filters them out downstream (Design Decision 3 + 6).
  Short-circuits on title/slug before lowercasing `body_preview` to
  avoid the body allocation for the common case (see Performance
  Considerations).
- `fn project(entry: &IndexEntry) -> Option<SearchResultRow>` —
  private constructor. Returns `None` if `entry.slug` is `None`;
  otherwise builds the projection. Only path that constructs
  `SearchResultRow` so the slug invariant is type-enforced.
- Handler signature: `pub(crate) async fn search(State(state):
  State<Arc<AppState>>, Query(q): Query<SearchQuery>) -> Result<Response, ApiError>`.
- Body:
  1. Length-cap check BEFORE trim: if `q.len() > MAX_Q_LEN`, return
     `Ok(Json(SearchResponse { results: vec![] }).into_response())`
     immediately. Trim `q` afterwards; if empty, same short-circuit.
     The cap is applied to the raw input so an attacker cannot pad
     a long string with leading/trailing whitespace to bypass the
     amplification defence (Security finding from review pass 2);
     `axum`'s default URI/query-string limits remain the
     transport-layer cap on the very largest inputs.
  2. Lowercase the trimmed `q` for case-insensitive comparison.
  3. Call `state.indexer.all().await`.
  4. For each entry: skip if `entry.r#type == DocTypeKey::Templates`
     (defence-in-depth — mirrors `docs.rs:35-37`, even though
     `Indexer::all()` already excludes Templates structurally);
     otherwise `classify(&entry, &q_lc)` and pair the entry with its
     bucket. Entries where `classify` returns `None` are excluded.
  5. Group by `Bucket` into four `Vec<IndexEntry>`s.
  6. Sort each bucket using `sort_by_cached_key`:
     `bucket.sort_by_cached_key(|e| (std::cmp::Reverse(e.mtime_ms),
     e.rel_path.to_string_lossy().into_owned()))` — `rel_path`
     string-encoded for platform-independent lexicographic tiebreak
     (avoids the `PathBuf` OS-encoding hazard on cross-platform
     dev environments). `sort_by_cached_key` computes each entry's
     sort key once and reuses it across comparisons, avoiding the
     O(M log M) String allocations a naive `sort_by` closure would
     incur (Performance finding from review pass 2).
  7. Concatenate buckets in `Bucket` discriminant order; pipe through
     `filter_map(|e| project(&e))` to enforce the slug invariant and
     produce `Vec<SearchResultRow>`.
  8. Return `Ok(Json(SearchResponse { results }).into_response())`.

Precedent: `server/src/api/docs.rs:19-41` (the `State + Query →
Result<Response, ApiError>` shape; `r#type` raw-identifier escape on
`IndexEntry` is the camelCase pattern; the Templates defence-in-depth
predicate at `docs.rs:35-37` is the precedent for step 4; see research
§"Handler precedent" for the exact code excerpt).

#### 2. Route registration

**File**: `skills/visualisation/visualise/server/src/api/mod.rs`
**Changes**:

- Add `mod search;` between `mod related;` (line 8) and `mod templates;`
  (line 9). (Module list is alphabetical.)
- Append `.route("/api/search", get(search::search))` to the router
  chain (lines 24-46). Position: alongside related routes; placement
  within the chain is feature-grouped, not alphabetised, so attach
  next to `related`.

#### 3. Integration tests — new file

**File**: `skills/visualisation/visualise/server/tests/api_search.rs`
**Changes**: New file modelled on `tests/api_docs.rs:12-73`. Uses
`common::seeded_cfg` + `AppState::build` + `build_router(state)` +
`oneshot(Request::builder()...)`. Write tests FIRST; they all fail
until the handler is implemented.

Tests to write (each maps to one or more ACs):

1. `empty_q_returns_200_with_empty_results` — `?q=` and no `q` param
   both return 200 + `{ "results": [] }` (AC: missing/empty `q`).
2. `returns_matches_across_library_doc_types` — seed entries across
   multiple LIBRARY doc types whose titles substring-match `q`;
   assert the response contains the expected doc types with kebab-case
   `docType` field (AC: indexer snapshot → results).
3. `excludes_templates_by_indexer_structure` — assert no row has
   `docType === "templates"` even after seeding template files; relies
   on `Indexer::all()`'s structural exclusion (AC: Templates).
4. `case_insensitive_matching` — seed entry with title "Foo Bar";
   query `foo`, `FOO`, `BaR` all match (AC: case-insensitive).
5. `bucket_1_exact_slug_first` — seed entries where one entry's slug
   equals `q` case-insensitively with low `mtime_ms`, and another
   entry has a more recent `mtime_ms` and matching title;
   slug-equal-q comes first (AC: exact-slug bucket).
6. `bucket_2_prefix_before_interior` — seed entry whose title starts
   with `q` (older `mtime_ms`) and another whose title contains `q`
   interior-only (newer `mtime_ms`); prefix entry comes first
   regardless of mtime (AC: prefix before interior).
7. `bucket_3_title_slug_before_bucket_4_body_preview` — title-only
   match (older mtime) vs body-preview-only match (newer mtime);
   title-match wins (AC: title/slug before body_preview).
8. `mtime_desc_within_bucket` — two title-prefix matches with
   different mtimes; newer first (AC: mtime desc within bucket).
9. `path_asc_breaks_mtime_ties` — two title-prefix matches with equal
   mtime; path-ascending breaks tie deterministically (AC: tie
   break).
10. `slugless_entries_filtered` — seed an entry with `slug = None`
    that would match by title; assert it does not appear (Design
    Decision 3).
11. `does_not_search_path_or_relpath` — seed an entry whose `path`
    string contains `q` but title, slug, and body_preview do not;
    assert it does not appear (AC: searched-field set).
12. `body_preview_substring_match` — entry matches only via
    body_preview; appears in bucket 4 (AC: body_preview membership).
13. `whitespace_only_q_returns_200_with_empty_results` — `q=%20%20%20`
    (URL-encoded whitespace) decodes to `"   "`, trims to `""`,
    short-circuits to `{ "results": [] }` (locks the server-side
    trim into the contract).
14. `over_length_q_returns_200_with_empty_results` — `q` of length
    `MAX_Q_LEN + 1` short-circuits to empty results (locks Design
    Decision 7's amplification defence).
15. `wire_field_is_mtimeMs_camelcase` — assert response JSON contains
    the field `mtimeMs` (camelCase serialisation of `mtime_ms`),
    matching the frontend `SearchResult` interface; this is a
    contract-pin for the work item's wire-shape reconciliation
    (Design Decision 6).
16. `non_matching_entries_are_excluded` — seed at least one entry
    whose title, slug, AND body_preview are all case-insensitive
    substring-free of `q`, plus at least one positive-control
    matching entry; assert the non-matching entry's slug is absent
    from the response while the matching entry is present (AC10 —
    the general "no field matches → excluded" path; complements
    test 11 which only covers the narrower path/rel_path-only
    case).
17. `mixed_case_query_and_field_classify_correctly` — seed an entry
    with title `"Foo Bar"` and slug `"foo-bar"`; query with mixed
    case `"FoO"`; assert the entry classifies into the Prefix
    bucket (locks case-folded comparison across both fields and
    catches mutations where only one comparison is case-folded).

**mtime seeding mechanism (pinned)**: `IndexEntry.mtime_ms` is sourced
from filesystem `metadata().modified()` (file_driver.rs:354-359), so
ranking tests must set fixture mtimes deterministically. MSRV is
1.85 per server `Cargo.toml`, so `std::fs::File::set_modified` (stable
since 1.75) is available without a new dependency. Add a helper to
`server/tests/common/mod.rs`:

```rust
pub fn set_mtime_ms(path: &Path, ms: i64) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::time::{SystemTime, Duration};
    let f = OpenOptions::new().write(true).open(path)?;
    f.set_modified(SystemTime::UNIX_EPOCH + Duration::from_millis(ms as u64))
}
```

Fixtures call `set_mtime_ms(&path, 1_000_000)` etc. to pin per-file
mtimes before the indexer scans. The six ranking tests
(`bucket_1_*`, `bucket_2_*`, `bucket_3_*`, `mtime_desc_*`,
`path_asc_*`, `slugless_*`) use this helper to lock the expected
ordering.

**Templates fixture for AC11 (test 3)**: the existing `seeded_cfg`
helper does NOT write templates into `meta/templates/` where the
indexer would surface them; templates are managed by `templates.rs`
in a separate store. To make `excludes_templates_by_indexer_structure`
test a real-world contract (rather than a structural tautology), the
fixture writes a templates entry whose title would match `q` *if* it
were enumerated, then independently asserts (a) the response contains
zero `docType === "templates"` rows AND (b) `state.indexer.all().await`
returns zero `Templates` entries (locking the structural-omission
invariant). If `Indexer::all()` ever weakens, assertion (b) fails
first and signals the regression before (a) silently leaks.

### TDD Sequence

1. Create `tests/api_search.rs` with all tests above. Run `cargo test
   --test api_search`; expect every test to fail with "cannot find
   route" / "404".
2. Add the `mod search;` declaration + route registration (mod.rs).
   Tests now hit the handler but fail because the handler doesn't
   exist — wire a minimal handler returning `{ "results": [] }` so the
   first test passes.
3. Implement the bucketing + ranking logic incrementally to turn each
   remaining test green.
4. Run `cargo clippy --tests` and fix any lints.

### Success Criteria

#### Automated Verification

- [ ] All `api_search` tests pass: `cd skills/visualisation/visualise/server && cargo test --test api_search`
- [ ] Full server test suite passes: `cd skills/visualisation/visualise/server && cargo test`
- [ ] No clippy warnings: `cd skills/visualisation/visualise/server && cargo clippy --tests -- -D warnings`
- [ ] Server builds: `cd skills/visualisation/visualise/server && cargo build`

#### Manual Verification

- [ ] `curl 'http://localhost:8787/api/search?q=test'` returns
  `{ "results": [...] }` with the expected camelCase fields against
  a running dev server.
- [ ] `curl 'http://localhost:8787/api/search'` (no `q`) returns
  `{ "results": [] }` with HTTP 200.

---

## Phase 2: `useDebouncedValue` primitive

### Overview

Add the trailing-edge debounce hook. ~10 lines; fully independent of
every other phase. Co-located with the rest of the api-layer hooks
under `frontend/src/api/`.

### Changes Required

#### 1. Hook module — new file

**File**: `skills/visualisation/visualise/frontend/src/api/use-debounced-value.ts`
**Changes**: Implement `useDebouncedValue<T>(value: T, delayMs:
number): T` using `useState` + `useEffect` + `setTimeout` /
`clearTimeout`. Trailing-edge by construction.

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

#### 2. Hook tests — new file

**File**: `skills/visualisation/visualise/frontend/src/api/use-debounced-value.test.ts`
**Changes**: New file with `vi.useFakeTimers` / `vi.useRealTimers`
and `@testing-library/react`'s `renderHook` +
`vi.advanceTimersByTimeAsync` (or sync `vi.advanceTimersByTime` with
`act`). Tests written FIRST.

Tests:

1. `returns_initial_value_synchronously` — first render returns
   `value` unchanged.
2. `updates_after_delay_with_no_intervening_changes` —
   `rerender({ value: 'b' })`, advance 200 ms, returned value is
   `'b'`.
3. `resets_timer_on_change_within_delay_window` — `rerender('b')`,
   advance 50, `rerender('c')`, advance 50, `rerender('b')`, advance
   200; final returned value is `'b'` (the AC's `ab → abc → ab`
   pattern at the primitive level).
4. `respects_custom_delayMs` — pass `delayMs: 50`; settles in 50.
5. `cleans_up_pending_timer_on_unmount` — unmount before delay
   elapses; no setState-after-unmount warning.

### TDD Sequence

1. Write all 5 tests. Run `npm test use-debounced-value` (from the
   frontend dir) — expect all to fail (file doesn't exist).
2. Implement `use-debounced-value.ts`. Tests turn green.

### Success Criteria

#### Automated Verification

- [ ] All 5 `useDebouncedValue` tests pass: `cd skills/visualisation/visualise/frontend && npm test -- use-debounced-value`
- [ ] Typecheck passes: `cd skills/visualisation/visualise/frontend && npm run typecheck`
- [ ] Lint passes: `cd skills/visualisation/visualise/frontend && npm run lint`

#### Manual Verification

- (None — pure primitive, exercised by Phase 4 + Phase 5 tests.)

---

## Phase 3: Global `/` keybind + `searchInputRef` plumbing in `RootLayout`

### Overview

Add the first `useEffect` and `useRef` in `RootLayout`, registering a
global `keydown` listener that focuses the sidebar search input on `/`
with no modifiers and no editable-element focus. Thread
`searchInputRef` through to `Sidebar` as a new optional prop; the
existing input picks it up via `ref={searchInputRef}`. No new context.
Independent of the api layer.

### Changes Required

#### 1. `RootLayout` — add ref + effect

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx`
**Changes**:

- Import `useEffect`, `useRef` from `react`.
- Declare `const searchInputRef = useRef<HTMLInputElement>(null)`.
- Extract two module-local pure helpers above the component:
  ```tsx
  function isPlainSlashKey(event: KeyboardEvent): boolean {
    return event.key === '/' && !event.metaKey && !event.ctrlKey
      && !event.altKey && !event.shiftKey
  }
  function isEditableTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false
    if (target instanceof HTMLInputElement) return true
    if (target instanceof HTMLTextAreaElement) return true
    return target.isContentEditable
  }
  ```
  `isPlainSlashKey` also guards `shiftKey` because Shift+`/` is `?`
  on US layouts but the key check covers that; the shift-guard is
  belt-and-braces against layouts where `key === '/'` arrives with
  Shift held. The `event.key === '/'` check (not `event.code ===
  'Slash'`) follows the work-item conventional notation.
- Add `useEffect` registering `document.addEventListener('keydown',
  onKeyDown)`:
  ```tsx
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!isPlainSlashKey(e)) return
      if (isEditableTarget(e.target)) return
      const input = searchInputRef.current
      if (!input) return  // ref-null guard: don't swallow the keystroke
      e.preventDefault()
      input.focus()
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
  }, [])
  ```
  Note: `preventDefault` and `focus()` are guarded together on
  `input !== null` — if the input is ever not mounted, the `/`
  keystroke proceeds with native behaviour instead of being silently
  swallowed.
- Pass `searchInputRef={searchInputRef}` to `<Sidebar>` (lines 52-56).

#### 2. `Sidebar` — accept and forward the ref

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`
**Changes**: This phase only attaches the ref; controlled value and
results panel are Phase 5.

- Import `RefObject` from `react`.
- Add `searchInputRef?: RefObject<HTMLInputElement>` to `Props`.
  **Optional in Phase 3** to keep existing Sidebar tests passing
  without ref-plumbing changes; Phase 5 promotes the prop to
  **required** once `<SearchResultsPanel>` lands and every render
  path has a ref source (Design Decision 5).
- Attach `ref={searchInputRef}` to the existing `<input
  type="search">` at lines 22-33.
- No other changes; the `TEMPORARY` comment stays until Phase 5.

**Cross-coupling note**: the keybind test (and the user-facing
`/` flow) relies on the search input carrying `aria-label="Search"`
(set by 0053 at `Sidebar.tsx:28`). The test query
`screen.getByRole('searchbox', { name: /search/i })` is coupled to
that label staying in place; if a future 0053 follow-up changes the
label, the keybind test and AC1 verification must be updated in
lock-step.

#### 3. Tests — RootLayout keybind

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.test.tsx`
(create if not present — check first; if present, extend)
**Changes**: Tests written FIRST. Use
`@testing-library/react`'s `render` + `userEvent.keyboard('/')`,
plus DOM helpers to set `document.activeElement`.

Tests:

1. `slash_focuses_sidebar_search_when_no_field_focused` — render the
   layout, press `/`, assert
   `document.activeElement === screen.getByRole('searchbox', { name:
   /search/i })`.
2. `slash_does_not_focus_when_input_focused` — focus another input
   first, press `/`, assert focus does not move and the `/` character
   is inserted into the focused input.
3. `slash_does_not_focus_when_textarea_focused` — same with a
   textarea.
4. `slash_does_not_focus_when_contenteditable_focused` — same with a
   `contenteditable` div.
5. `slash_with_meta_modifier_does_not_activate` — press Cmd+`/`;
   sidebar search not focused; `preventDefault` not called.
6. `slash_with_ctrl_modifier_does_not_activate` — Ctrl+`/`.
7. `slash_with_alt_modifier_does_not_activate` — Alt+`/`.
8. `listener_cleanup_on_unmount` — unmount the layout; press `/`;
   focus unchanged (no listener leaks).
9. `slash_with_shift_modifier_does_not_activate` — Shift+`/` (which
   produces `?` on US layouts but on some EU layouts can produce
   `/` with `shiftKey: true`); `isPlainSlashKey` rejects via the
   `shiftKey` guard.
10. `preventDefault_not_called_when_input_focused` — pair with test
    2: explicitly assert `preventDefault` was not called on the
    fired keydown event (spy on the synthetic event), in addition
    to focus-unchanged. Decouples the SUT contract from JSDOM
    keystroke-fidelity behaviour.

These tests will need a `renderRootLayout` helper that wraps
`<RootLayout>` (an `<Outlet />`-bearing route component) in a
`MemoryRouter` + `QueryClientProvider` stack and provides the routes
the `<Sidebar>` queries (`useTypes`, `useLibraryStructure`). Pattern A
from research §"Testing patterns" is the right fit — mock
`fetchTypes` and `fetchLibraryStructure` at module level.

### TDD Sequence

1. Write all 8 keybind tests; run them, expect failure (no keydown
   listener exists).
2. Add the `searchInputRef` prop to `Sidebar` + attach to the
   existing input. (Type-check passes; Sidebar tests still pass
   because the prop is optional.)
3. Add the `useRef` + `useEffect` in `RootLayout`, pass the ref to
   `<Sidebar>`. Tests turn green.

### Success Criteria

#### Automated Verification

- [ ] All RootLayout keybind tests pass: `cd skills/visualisation/visualise/frontend && npm test -- RootLayout`
- [ ] Existing Sidebar tests still pass (ref is optional prop): `cd skills/visualisation/visualise/frontend && npm test -- Sidebar`
- [ ] Typecheck passes: `cd skills/visualisation/visualise/frontend && npm run typecheck`
- [ ] Lint passes: `cd skills/visualisation/visualise/frontend && npm run lint`

#### Manual Verification

- [ ] In the dev server: clicking outside any input, pressing `/`
  moves cursor focus to the sidebar search input; the `/` character
  is not inserted anywhere.
- [ ] Focused inside the temporary `<input type="search">` (or any
  other input), pressing `/` inserts `/` as text and does not
  re-focus.
- [ ] Cmd+`/` / Ctrl+`/` does nothing (browser default unchanged).

---

## Phase 4: API layer — `queryKeys.search`, `fetchSearch`, `useSearch`

### Overview

Add the React Query plumbing that the Sidebar consumes in Phase 5.
Each piece is tested in isolation; `useSearch` is tested with a mocked
`fetchSearch`. Depends on Phase 2's `useDebouncedValue`. The wire
shape is pinned to the work item — Phase 4 does not need Phase 1 to
run, only the contract.

### Changes Required

#### 1. `queryKeys.search`

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.ts`
**Changes**: Add `search: (q: string) => ['search', q] as const` to
the `queryKeys` object (lines 40-62), placed before `disabled` (which
stays last as the sentinel).

#### 2. `queryKeys` test extension

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.test.ts`
**Changes**: Add a test that `queryKeys.search('foo')` returns
`['search', 'foo']` and that distinct queries produce distinct
key tuples (one test, two assertions).

#### 3. `SearchResult` type + `fetchSearch` helper

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.ts`
**Changes**:

- Add an exported `SearchResult` interface near the existing types
  (note: `path` is deliberately absent per Design Decision 6):

  ```ts
  export interface SearchResult {
    docType: DocTypeKey
    title: string
    slug: string
    mtimeMs: number
  }
  ```
- Add `fetchSearch(q: string, signal?: AbortSignal): Promise<SearchResult[]>`
  modelled on `fetchTypes` (line 59-64). The signal is forwarded to
  `fetch`, allowing React Query to cancel an in-flight request when
  the query key changes (Design Decision 9). The `FetchError`
  message omits raw `q` to avoid log injection / disclosure
  (Design Decision 6 / security finding); the AC requires only that
  the message contain the literal `/api/search`, which it does.
  `console.error` is called at this layer (Design Decision 10) and
  the error is re-thrown so React Query records `isError: true`.

  ```ts
  export async function fetchSearch(
    q: string,
    signal?: AbortSignal,
  ): Promise<SearchResult[]> {
    try {
      const r = await fetch(`/api/search?q=${encodeURIComponent(q)}`, { signal })
      if (!r.ok) throw new FetchError(r.status, `GET /api/search: ${r.status}`)
      const body: { results: SearchResult[] } = await r.json()
      return body.results
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') throw err
      console.error(err)
      throw err
    }
  }
  ```

  The `AbortError` short-circuit avoids logging cancellations as
  errors (cancellation is normal during fast typing); React Query
  treats abort errors as non-error transitions when the signal it
  passed was the one that aborted.

#### 4. `fetch.ts` tests extension

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.test.ts`
**Changes**: Add tests modelled on existing `fetchTypes` /
`fetchDocs` test patterns:

1. `fetchSearch_returns_results_on_2xx` — mock global `fetch` to
   return `{ ok: true, json: async () => ({ results: [...] }) }`;
   assert the resolved value equals the inner `results` array.
2. `fetchSearch_throws_FetchError_with_path_in_message_on_non_2xx` —
   mock fetch to return `{ ok: false, status: 500 }`; spy on
   `console.error`; assert thrown error is `FetchError`,
   `status === 500`, `err.message.includes('/api/search')`,
   `err.message` does NOT include the raw `q` (verifies the
   log-injection mitigation from Design Decision 6), and
   `console.error` was called with the `FetchError` instance.
3. `fetchSearch_encodes_query` — pass `q = "foo bar/baz"`; assert
   fetch was called with URL containing `q=foo%20bar%2Fbaz`.
4. `fetchSearch_forwards_abort_signal` — pass an `AbortController.signal`;
   call `controller.abort()`; assert `fetch` was invoked with
   `{ signal }` and that the resulting `AbortError` propagates
   without being logged via `console.error` (verifies the
   abort-short-circuit from Design Decision 9).

#### 5. `useSearch` hook

**File**: `skills/visualisation/visualise/frontend/src/api/use-search.ts`
**Changes**: New file.

```ts
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { fetchSearch } from './fetch'
import { queryKeys } from './query-keys'
import { useDebouncedValue } from './use-debounced-value'

export function useSearch(query: string) {
  const debounced = useDebouncedValue(query.trim(), 200)
  return useQuery({
    queryKey: debounced.length >= 2
      ? queryKeys.search(debounced)
      : queryKeys.disabled('search'),
    queryFn: ({ signal }) => fetchSearch(debounced, signal),
    enabled: debounced.length >= 2,
    placeholderData: keepPreviousData,
  })
}
```

`placeholderData: keepPreviousData` makes `search.data` return the
previous query's results during the in-flight transition, with
`search.isPlaceholderData === true` flagging the transitional state
so consumers can distinguish stale-but-shown rows from fresh data
(Design Decision 8). `queryFn`'s `signal` is forwarded to
`fetchSearch` to cancel in-flight requests on key change (Design
Decision 9). `retry` and `staleTime` are deliberately not overridden;
both inherit the global `QueryClient` defaults (`staleTime: Infinity,
retry: 1`) per Design Decision 12.

#### 6. `useSearch` tests

**File**: `skills/visualisation/visualise/frontend/src/api/use-search.test.tsx`
**Changes**: New file using Pattern B (`Wrapper` component) from
research §"Testing patterns". `vi.useFakeTimers` and
`vi.advanceTimersByTimeAsync`; `vi.spyOn(fetchModule, 'fetchSearch')`.
**Test `QueryClient` is pinned** to mirror production semantics on
the dimensions the tests actually depend on:

```ts
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,         // deterministic — no retry on rejection
      staleTime: Infinity,  // mirror production so cache hits don't refetch
      gcTime: Infinity,     // keep cache entries live across rerenders
    },
  },
})
```

The `staleTime: Infinity` pin is load-bearing for the dedup test
(test 3 below): without it, returning to a previously-resolved key
would refetch on remount and break the cache-hit assertion.

Tests:

1. `does_not_request_below_2_chars` — render with `q = 'a'`, advance
   200 ms; `fetchSearch` not called; `useQuery` is disabled.
2. `requests_once_after_settle_for_2_plus_chars` — render with `q =
   'ab'`, advance 200 ms; assert `fetchSearch` called exactly once
   AND with first arg `'ab'`. Use both
   `toHaveBeenCalledTimes(1)` and `toHaveBeenCalledWith('ab', expect.anything())`
   so a spurious extra call is detected.
3. `dedupes_via_react_query_cache_within_gctime` — render `'ab'`,
   advance 200 ms, wait for resolved, rerender `'a'`, advance 200 ms,
   rerender `'ab'`; assert
   `expect(fetchSearch).toHaveBeenCalledTimes(1)` total (cache hit on
   the second `ab`). Relies on the test client's `staleTime: Infinity`
   pinning above — AC debounce-dedup criterion (a).
4. `intermediate_keystrokes_do_not_settle` — render `'ab'`, advance
   100, rerender `'abc'`, advance 100, rerender `'ab'`, advance
   200; assert `toHaveBeenCalledTimes(1)`,
   `toHaveBeenCalledWith('ab', expect.anything())`, AND
   `expect(fetchSearch).not.toHaveBeenCalledWith('abc', expect.anything())`
   (AC criterion (b) — the "zero for abc" half is load-bearing).
5. `trims_input_before_debounce` — render `'  ab  '`; spy called with
   `'ab'` (the trim is applied before debounce).
6. `query_key_uses_settled_trimmed_value` — render `'  ab  '`,
   advance 200 ms; assert
   `expect(queryClient.getQueryState(['search', 'ab'])).toBeDefined()`
   AND `expect(queryClient.getQueryState(['search', '  ab  '])).toBeUndefined()`.
   Direct introspection (not the parallel-useQuery pattern, which
   conflates "key was used" with "data was cached").
7. `aborts_in_flight_request_on_key_change` — render `'abcd'` with a
   never-resolving fetch mock; advance 200 ms; rerender `'abcde'`;
   advance 200 ms; assert the first call's `signal.aborted === true`
   (the `signal` passed via `queryFn`'s context is the one React
   Query aborts when the key changes — verifies Design Decision 9
   end-to-end through `useSearch`).

### TDD Sequence

1. Write all `queryKeys.test.ts` assertions (1 test). Run; fails (no
   `search` key). Add the key; turns green.
2. Write all `fetch.test.ts` extension tests for `fetchSearch` (3
   tests). Run; fails. Add `SearchResult` + `fetchSearch`; turns green.
3. Write all `use-search.test.tsx` tests (6 tests). Run; fails (file
   doesn't exist). Add `use-search.ts`; turns green.

### Success Criteria

#### Automated Verification

- [ ] `queryKeys.search` test passes: `cd skills/visualisation/visualise/frontend && npm test -- query-keys`
- [ ] All `fetchSearch` tests pass: `cd skills/visualisation/visualise/frontend && npm test -- fetch.test`
- [ ] All `useSearch` tests pass: `cd skills/visualisation/visualise/frontend && npm test -- use-search`
- [ ] Typecheck passes: `cd skills/visualisation/visualise/frontend && npm run typecheck`
- [ ] Lint passes: `cd skills/visualisation/visualise/frontend && npm run lint`

#### Manual Verification

- (None — hook behaviour is exercised by Phase 5 tests + dev-server
  smoke.)

---

## Phase 5: Sidebar integration — controlled input + results panel

### Overview

Wire the temporary input into a controlled component, extract a new
`<SearchResultsPanel>` component owning the four-branch results state
machine, render the results panel as a sibling block beneath
`.searchRow`, render rows as `<Link>` elements with framed Glyphs +
titles + sentence-case doc-type labels, render the `role="status"
aria-live="polite"` "No matches" empty state, hold previous results
during in-flight via `placeholderData: keepPreviousData`, and add the
Escape key handler on the input. This is the only phase that depends
on prior phases — it composes Phases 2, 3, and 4.

`console.error` logging lives in `fetchSearch`'s catch path (Design
Decision 10), so Sidebar/SearchResultsPanel only consumes
`search.isError` for UI and does not run a logging side effect.

### Changes Required

#### 1. `Sidebar` component wiring

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`
**Changes**:

- Import `useState` from `react`.
- Import `SearchResultsPanel` from `./SearchResultsPanel`.
- Make the `searchInputRef` prop **required** (`searchInputRef:
  RefObject<HTMLInputElement>` — no longer optional, per Design
  Decision 5).
- Introduce `const [query, setQuery] = useState('')` as the first
  state hook in the component.
- Replace the uncontrolled `<input>` (current lines 22-33) with a
  controlled one: `value={query}`, `onChange={e =>
  setQuery(e.target.value)}`, plus an Escape handler
  (`onKeyDown={e => { if (e.key === 'Escape') { setQuery(''); e.currentTarget.blur() } }}`),
  plus the existing `ref={searchInputRef}` attachment from Phase 3.
- Remove the `TEMPORARY` comment block at lines 22-23.
- Below `.searchRow` (still inside `.sidebar`), render
  `<SearchResultsPanel query={query} />`. SearchResultsPanel owns
  the `useSearch` call and the rendering state machine; Sidebar only
  passes the controlled query string. Sidebar does NOT call
  `useSearch` directly — keeps the four-branch state machine out of
  Sidebar (addressing the god-component concern).

#### 1a. `SearchResultsPanel` — new component

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/SearchResultsPanel.tsx`
**Changes**: New file. Imports `Link` from `@tanstack/react-router`;
`Glyph` from `../Glyph/Glyph`; `DOC_TYPE_LABELS` from
`../../api/types`; `useSearch` from `../../api/use-search`; styles
from `./Sidebar.module.css`.

Render state machine (truth table in a JSDoc block at the top of the
file):

```text
/**
 * Renders the search results panel beneath the sidebar search input.
 *
 * Render state machine (rows = mutually exclusive branches, top-down
 * precedence; each row evaluated only if all above are false):
 *
 *   query.trim().length < 2                            → render nothing (panel hidden)
 *   search.isError                                     → render nothing (panel cleared; fetch.ts logged)
 *   search.isPlaceholderData && data?.length > 0       → render prior rows (in-flight, keep stale visible)
 *   search.isPlaceholderData                           → render nothing (in-flight, no prior data)
 *   search.isSuccess && data?.length === 0             → render "No matches" status
 *   data && data.length > 0                            → render results list
 *   otherwise                                          → render nothing (first-load pending)
 */
```

Component body:

```tsx
export function SearchResultsPanel({ query }: { query: string }) {
  const search = useSearch(query)
  if (query.trim().length < 2) return null
  if (search.isError) return null
  if (search.isPlaceholderData && !(search.data && search.data.length > 0)) return null
  if (search.isSuccess && search.data?.length === 0) {
    return (
      <p role="status" aria-live="polite" className={styles.resultsEmpty}>
        No matches
      </p>
    )
  }
  if (!search.data || search.data.length === 0) return null
  return (
    <section aria-label="Search results">
      <ul className={styles.results}>
        {search.data.map(r => (
          <li key={`${r.docType}/${r.slug}`}>
            <Link
              to="/library/$type/$fileSlug"
              params={{ type: r.docType, fileSlug: r.slug }}
              className={styles.resultRow}
            >
              <Glyph docType={r.docType} size={16} framed />
              <span className={styles.resultTitle}>{r.title}</span>
              <span className={styles.resultLabel}>
                {DOC_TYPE_LABELS[r.docType]}
              </span>
            </Link>
          </li>
        ))}
      </ul>
    </section>
  )
}
```

The list `<key>` is `${r.docType}/${r.slug}` rather than `r.path`,
because `path` is no longer in the wire payload (Design Decision 6).
The combination is unique within a response (no two LIBRARY docs of
the same type can share a slug — slugs are filename-derived).

#### 2. CSS — results panel styles

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.module.css`
**Changes**: Append new classes consuming existing `--ac-*` design
tokens. No raw hex / px values.

- `.results` — `display: flex; flex-direction: column; gap:
  var(--sp-1)` (or token-equivalent for compact list spacing); reset
  `<ul>` defaults (`list-style: none; margin: 0; padding: 0`).
- `.resultRow` — `display: flex; align-items: center; gap:
  var(--sp-2); padding: var(--sp-2); border-radius:
  var(--radius-md); text-decoration: none; color: var(--ac-fg-default)`
  (or equivalent); hover state via `--ac-bg-raised` (match existing
  sidebar item hover precedent); `:focus-visible` ring matching the
  existing `.link` focus convention (outline + offset using
  `--ac-focus-ring` / equivalent) so keyboard Tab traversal is
  clearly indicated.
- `.resultTitle` — `flex: 1; font: var(--ac-font-body-sm)` (or
  token-equivalent); truncate with `text-overflow: ellipsis`.
- `.resultLabel` — `font: var(--ac-font-eyebrow-sm)` or sentence-case
  body-xs token; muted via `color: var(--ac-fg-muted)`.
- `.resultsEmpty` — `padding: var(--sp-2); color:
  var(--ac-fg-muted); font: var(--ac-font-body-sm); margin: 0`.

(Read the file before editing to discover the exact token names in
use; the research notes `--sp-*` and `--radius-md` are present.
Other tokens: take cues from `LifecycleIndex.module.css` empty-state
class and other muted-text rows in `LibraryTypeView.module.css`.)

#### 3. `RootLayout` / `Sidebar` — promote ref prop to required

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`
**Changes**: Make `searchInputRef: RefObject<HTMLInputElement>` a
required prop on `Sidebar` (Design Decision 5). All existing
`<Sidebar ... />` test usages must be updated to pass a ref
(typically `useRef<HTMLInputElement>(null)` inside the test wrapper);
this is a small ergonomic cost in tests but eliminates the silent
miswiring class where a parent forgets to pass the ref and the `/`
keybind becomes a no-op.

#### 4. Sidebar tests

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx`
(and `SearchResultsPanel.test.tsx` co-located with the new component
for the rendering tests — Sidebar tests just verify Sidebar still
renders the panel; SearchResultsPanel tests own the state-machine
verification)
**Changes**: Tests written FIRST for the new behaviour. The existing
search-row presence test (`Sidebar.test.tsx:253-258`) must continue
to pass — keep `<input type="search">` and `<kbd>/</kbd>` queryable.
Update the test description (the `0054 will wire behaviour` note is
no longer true).

**Per-suite `QueryClient`** for both files mirrors Phase 4:
`{ defaultOptions: { queries: { retry: false, staleTime: Infinity,
gcTime: Infinity } } }`. Production `retry: 1` would otherwise double
the rejection-path behaviour under fake timers.

Tests on `SearchResultsPanel.test.tsx` (each maps to an AC):

1. `typing_2_chars_issues_one_request_after_200ms` — mock
   `fetchSearch` to resolve with one result; render panel with
   `query='ab'`; advance 200 ms; assert `fetchSearch` called once
   with first arg `'ab'`.
2. `does_not_request_below_2_chars` — render with `query='a'`;
   advance 200 ms; assert `fetchSearch` not called.
3. `renders_one_link_per_result_in_response_order_and_each_is_an_anchor` —
   mock returns three results with distinct doc-types/slugs/titles;
   await results; `screen.findAllByRole('link')` returns three
   elements; for each row assert `row.tagName === 'A'` AND
   `row.hasAttribute('href')` AND `row.getAttribute('href')` matches
   `/library/<docType>/<slug>` in response order (lock AC6's native
   link semantics — modifier-click / middle-click / Enter all rely
   on the element being a real `<a href>`). Use `MemoryRouter` + the
   test-only `libraryDocRoute` from `frontend/src/test/router-helpers.tsx`.
3a. `enter_on_focused_result_row_navigates` — focus the first
   rendered result row via `.focus()`; press Enter via
   `userEvent.keyboard('{Enter}')`; assert the `MemoryRouter`'s
   history advances to `/library/<docType>/<slug>` matching the
   first result. Exercises AC11's Enter-key clause directly rather
   than relying on the platform anchor semantics, so a regression
   that wraps the row in a non-anchor element with `onClick` is
   detected.
4. `each_result_row_renders_glyph_title_and_label` — assert each
   `<a>` contains the title text and the sentence-case
   `DOC_TYPE_LABELS[docType]` text.
5. `glyph_is_framed` — assert each row contains a
   `span[data-doc-type]` element (the `Glyph` framed-mode wrapper at
   `Glyph.tsx:93-95`). A bare unframed Glyph renders the SVG
   directly without the wrapping `data-doc-type` span, so this
   selector discriminates framed vs. unframed unambiguously.
6. `empty_results_shows_no_matches_status_with_aria_live` — mock
   returns `[]`; await settle; assert
   `screen.getByRole('status').textContent === 'No matches'` AND
   the element has `aria-live="polite"` attribute (matches the
   project's Toaster/KanbanBoard/RelatedArtifacts convention for
   dynamic live regions).
7. `results_list_has_accessible_name` — assert the results region is
   queryable via `screen.getByRole('region', { name: /search results/i })`
   (the `<section aria-label="Search results">` wrapper).
8. `below_threshold_after_results_clears_panel` — render `query='abc'`,
   get results; rerender `query='a'`; advance 200 ms; assert no
   result rows, no `No matches` element (panel hidden entirely
   because `query.trim().length < 2`).
9. `in_flight_keeps_prior_results_visible_via_placeholder_data` —
   render `query='ab'`; advance 200 ms (debounce); resolve fetch to
   two rows; await render. Rerender `query='cd'` with a separate
   `fetchSearch` mock for `'cd'` returning a never-resolving promise;
   advance 200 ms so the `'cd'` debounce fires and the `'cd'` fetch
   is observed pending via
   `queryClient.getQueryState(['search', 'cd'])?.fetchStatus === 'fetching'`.
   At this moment, assert: (a) the two `'ab'` rows are STILL visible
   in the DOM, (b) `screen.queryByRole('status')` is `null` (no
   `No matches` mid-transition), and (c) the underlying placeholder
   signal is active — verified via a test-only consumer that surfaces
   `search.isPlaceholderData` as a `data-placeholder` attribute on
   the wrapping element OR via
   `queryClient.getQueryState(['search', 'cd'])?.data === undefined`
   while UI still renders `'ab'` rows (the data slot for `'cd'` is
   empty but UI shows prior rows ⇒ placeholder mechanism is
   load-bearing). This implements the amended AC9 (work item edit
   2026-06-01), which explicitly permits the immediately-prior
   settled query's rows as transitional content distinguishable via
   `search.isPlaceholderData === true`, while still forbidding
   `No matches` mid-transition and rows from an arbitrarily-older
   settled query.
10. `fetch_error_clears_panel` — mock `fetchSearch` to reject with
    `new FetchError(500, 'GET /api/search: 500')`; spy on
    `console.error` (mockImplementation(() => {})); render
    `query='foo'`; advance 200 ms; await the rejection; assert no
    result rows AND no `No matches` element AND `console.error`
    was called (note: the call originates from `fetchSearch` per
    Design Decision 10, not a Sidebar effect — but the user-facing
    AC15 behaviour is identical: cleared panel + logged error
    containing `/api/search`). Assert
    `(console.error.mock.calls[0][0] as Error).message.includes('/api/search')`.
11. `aborts_in_flight_request_on_unmount` — render panel with
    `query='abcd'`, never-resolving fetch; unmount the panel before
    the fetch resolves; assert the abort signal passed to
    `fetchSearch` was aborted (no in-flight request left dangling
    after unmount).

Test on `Sidebar.test.tsx`:

12. `existing_search_row_markup_still_present` — keep the prior test
    at lines 253-258 passing (update its description text).
13. `escape_clears_query_and_blurs_input` — focus the input; type
    `'ab'`; press Escape; assert input value is `''` AND input is
    no longer the active element.
14. `sidebar_renders_search_results_panel` — assert the
    `<SearchResultsPanel>` is rendered as a sibling to `.searchRow`
    (presence-only structural check; behaviour tested in
    `SearchResultsPanel.test.tsx`).

### TDD Sequence

1. Update / add all Sidebar and SearchResultsPanel tests (split
   across `Sidebar.test.tsx` and the new
   `SearchResultsPanel.test.tsx`). Run; expect failures on every
   new assertion.
2. Wire `useState` + `useSearch` + controlled input. Some tests pass
   (typing-triggers, no-request-below-2-chars).
3. Add the results panel + Link rendering. Result-row tests pass.
4. Add the empty-state branch. Empty-state test passes.
5. Add the in-flight/below-threshold clearing logic. In-flight tests
   pass.
6. Error path: with `fetchSearch`'s catch-path logging already in
   place from Phase 4 (Design Decision 10), the SearchResultsPanel
   error test should pass without any additional component-side
   effect. Verify no `useEffect` for logging is introduced in this
   phase.
7. Add CSS module classes. Visually verify in the dev server.

### Success Criteria

#### Automated Verification

- [ ] All new Sidebar tests pass: `cd skills/visualisation/visualise/frontend && npm test -- Sidebar`
- [ ] Full frontend test suite passes: `cd skills/visualisation/visualise/frontend && npm test`
- [ ] Typecheck passes: `cd skills/visualisation/visualise/frontend && npm run typecheck`
- [ ] Lint passes: `cd skills/visualisation/visualise/frontend && npm run lint`
- [ ] Build passes: `cd skills/visualisation/visualise/frontend && npm run build`

#### Manual Verification

- [ ] Dev server: type `/` from anywhere outside an input — focus
  moves to the sidebar search.
- [ ] Type a short string (≥ 2 chars) — results panel appears 200 ms
  after the last keystroke; rows show framed Glyph + title +
  sentence-case label.
- [ ] Click a result row — navigates to `/library/<docType>/<slug>`
  via the canonical route.
- [ ] Cmd+click a row — opens in a new tab.
- [ ] Type a query with no matches — `No matches` empty state
  renders.
- [ ] Backspace below 2 chars — panel disappears (no rows, no empty
  state).
- [ ] Edit query while a previous response is shown — previous rows
  remain visible until the new response arrives (placeholder-data
  behaviour, Design Decision 8); `No matches` does not flicker in
  mid-transition.
- [ ] Focus the search input via `/`; press Tab — focus lands on
  the first result row (verifies Tab-navigation deferral fallback
  from "What We're NOT Doing").
- [ ] Stop the dev server's API (or trigger a server error) — panel
  clears; `console.error` shows a `FetchError` for `/api/search`.
- [ ] Type `0054` — work item with slug starting `0054-` appears
  first (exact-slug bucket).
- [ ] Type a string that matches multiple buckets — order reflects
  bucket 1 → 2 → 3 → 4 by inspection.

---

## Testing Strategy

### Unit Tests

- `useDebouncedValue`: trailing-edge timing, custom delay, cleanup.
- `queryKeys.search`: tuple identity.
- `fetchSearch`: 2xx → results, non-2xx → `FetchError` with
  `/api/search` in message (and `q` NOT in message — log-injection
  defence), query encoding, AbortSignal forwarding.
- `useSearch`: 2-char gate, debounce-dedup (both AC examples), trim,
  query-key uses settled value, in-flight abort on key change.

### Integration Tests

- Server: `tests/api_search.rs` exercises the full handler against
  `AppState::build` with a seeded `tempdir` filesystem — covers every
  ranking AC plus Templates exclusion (with a defence-in-depth handler
  filter on top of the structural omission), slug-less filtering via
  the typed `project` constructor, searched fields, missing/empty/
  whitespace-only/over-length `q`, and the `mtimeMs` wire-shape
  contract.
- RootLayout keybind: `RootLayout.test.tsx` exercises the global
  listener via `userEvent.keyboard('/')` against rendered providers,
  including Shift+`/` and the explicit `preventDefault not called`
  contract.
- Sidebar / SearchResultsPanel: `SearchResultsPanel.test.tsx` exercises
  typing → debounce → mocked `fetchSearch` → rendered rows or empty
  state or in-flight placeholder-data rendering or error-cleared
  panel or unmount abort. `Sidebar.test.tsx` verifies the Escape
  handler and that the search row + results panel are wired
  together.

### Manual Testing Steps

See per-phase "Manual Verification" checklists above. The end-to-end
manual smoke at the end of Phase 5 is the authoritative pre-merge
test.

## Performance Considerations

- `Indexer::all()` clones the full entry vector under a tokio
  `RwLock`. The cloned `IndexEntry` carries `frontmatter:
  serde_json::Value` (kB-scale nested allocations possible),
  `work_item_refs: Vec<String>`, `body_preview: String`, `title:
  String`, `etag: String`. At 10²–10³ entries this is dozens of MB of
  per-request allocation, not microseconds — the honest bound is
  "bounded N heavy struct clones, acceptable at expected scale and
  debounce-protected rate", not "microseconds". The 200 ms debounce
  + 2-char minimum bounds request rate, so the absolute volume is
  acceptable for v1; future work that needs lower per-request cost
  should introduce an `all_search_projection()` method on `Indexer`
  that, under the read lock, projects to a lightweight `Vec<(DocTypeKey,
  Option<String>, String, String, PathBuf, i64)>` (slug, title,
  body_preview, rel_path, mtime) — avoiding the heavy field clones
  and shortening the critical section.
- The read lock on `entries` is held across the full vector clone.
  Pending writers (`refresh_one`, `rescan`, the kanban PATCH path
  in `docs.rs`) block until the clone completes; subsequent readers
  queue behind that writer. Under bursty indexer activity (a large
  `git pull`, branch switch triggering many file changes) a search
  request that lands mid-burst can experience writer-priority
  queueing latency well above the average case. The
  `all_search_projection()` follow-up above would also shorten this
  contention window.
- Per-bucket sort cost is O(M log M) where M is the matching subset
  size; bounded by N total entries. The `classify` short-circuit
  (lowercase title/slug first, only lowercase `body_preview` when
  bucketing reaches that path) avoids the largest allocation in
  the common case where matches come from title/slug.
- React Query's default `gcTime` of 5 minutes is unchanged. The
  combination `staleTime: Infinity` + `gcTime: 5min` + no SSE
  invalidation for `queryKeys.search` means a user who types `'foo'`,
  edits a file, and re-types `'foo'` within 5 minutes will see stale
  cached results. Acceptable for v1 (file edits affect titles
  rarely; the bigger UX risk is the absence of feedback, not the
  staleness itself). Future work can either (a) override `gcTime`
  to ~30 s for search specifically (search is cheap to refetch), or
  (b) add a `searchPrefix` invalidation entry to `useDocEvents` (see
  `api/use-doc-events.ts:99-189` for the existing invalidation
  list — search is currently absent).

## Migration Notes

None. This is additive: a new route, a new hook, a new CSS class set,
new state in `Sidebar`, and the first `useEffect`/`useRef` in
`RootLayout`. No data migration, no API contract changes to existing
endpoints.

## References

- Work item: `meta/work/0054-sidebar-search.md`
- Research: `meta/research/codebase/2026-06-01-0054-sidebar-search.md`
  (the canonical source of file/line references and code excerpts;
  this plan deliberately points back to it rather than duplicating
  every reference)
- Parent epic: `meta/work/0036-sidebar-redesign.md`
- Sibling deliverables: `meta/work/0053-*.md` (sidebar layout +
  search slot), `meta/work/0055-*.md` (activity feed),
  `meta/work/0037-*.md` (Glyph component)
- Handler precedent: `skills/visualisation/visualise/server/src/api/docs.rs:19-41`
- Indexer entry point: `skills/visualisation/visualise/server/src/indexer.rs:617-619`
- Fetch helper precedent: `skills/visualisation/visualise/frontend/src/api/fetch.ts:59-64` (`fetchTypes`)
- React Query setup: `skills/visualisation/visualise/frontend/src/api/query-client.ts:3-13`
- Doc-type label mapping: `skills/visualisation/visualise/frontend/src/api/types.ts:49-63`
- Library doc route: `skills/visualisation/visualise/frontend/src/router.ts:97-113`
- `<Link>` renders `<a href>` evidence: `skills/visualisation/visualise/frontend/src/test/router-helpers.test.tsx:14-18`
