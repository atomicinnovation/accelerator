---
date: "2026-04-25T19:30:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-25-meta-visualiser-phase-6-lifecycle-clusters-and-view.md"
review_number: 1
verdict: REVISE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, performance, compatibility]
review_pass: 2
status: complete
---

## Plan Review: Phase 6 — Lifecycle clusters and view

**Verdict:** REVISE

The plan is well-structured, follows TDD throughout, and mirrors established patterns from earlier phases (Layout/Index/View routing trio, query-key factory, fetch helper conventions). It successfully avoids hot-path performance pitfalls and gives clear, independently-committable steps with explicit test counts. However, multiple lenses converge on the same handful of structural concerns: a server/client schema asymmetry around `hasNotes` and `Notes` entries that causes silent data loss in the timeline, body-preview logic that diverges from its own docstring, an SSE invalidation strategy that bypasses the central query-key factory, a developer-UX gap (no back-link, raw error strings, missing search) that limits the v1 view's usefulness, and accessibility gaps in `PipelineDots` that hide the indicator's meaning from screen readers. None are critical, but the count and breadth of major findings (12) warrants revision before implementation.

### Cross-Cutting Themes

- **Notes / `hasNotes` schema asymmetry** (flagged by: architecture, code-quality, standards, compatibility) — the server's `Completeness` requires `has_notes`, the TS interface marks it optional, the frontend's `LIFECYCLE_PIPELINE_STEPS` deliberately omits it, and `compute_clusters` happily clusters Notes entries that the timeline then silently filters out. The data flow is internally inconsistent across three layers.
- **`body_preview_from` stated intent vs implementation** (flagged by: code-quality, correctness, test-coverage) — the strategy comment says "skip a leading H1", the implementation skips every heading line and carries a dead `seen_h1` flag, and the tests don't pin the boundary cases (200-char threshold, mid-paragraph headings, H2-only first content) that would reveal the divergence.
- **SSE invalidation prefix duplicates `query-keys.ts`** (flagged by: architecture, code-quality) — `['lifecycle-cluster']` is a magic string in the dispatcher while every other invalidation goes through `queryKeys`. Single-source-of-truth on cache key shapes is broken.
- **`slug?` test-only prop creates dual routing path** (flagged by: architecture, correctness, usability) — `LifecycleClusterView` accepts both a prop and `useParams({ strict: false })` with a cast that erases route typing; tests don't exercise the production routing path.
- **Test fixture sweep relies on grep instead of a factory** (flagged by: code-quality, compatibility, usability) — adding required fields to `IndexEntry` forces a sweep of every literal constructor; a `makeIndexEntry(overrides)` helper would absorb this and every future field addition.
- **Lowercased PR labels read as typos** (flagged by: code-quality, standards, usability) — `step.label.toLowerCase()` produces "no pr yet"; an explicit `placeholder` field per stage would let copy live next to its label.
- **`bodyPreview` widens `IndexEntry` contract for one consumer** (flagged by: architecture, performance) — the field travels on every doc-list endpoint even though only the cluster timeline reads it, and the SSE broad-prefix invalidation re-fetches the inflated payload across every mounted detail view without coalescing.

### Tradeoff Analysis

- **Test isolation vs router-integration coverage**: the `slug?` prop simplifies unit tests but skips exercising the real `useParams` path. Resolution depends on whether you prefer tighter component tests with a router-bound smoke test, or splitting into `LifecycleClusterView` (router) wrapping `LifecycleClusterContent` (pure).
- **Wire-payload size vs round-trip count**: `bodyPreview` on `IndexEntry` trades larger list payloads for avoided per-card `fetchDocContent` calls. The detail timeline genuinely needs it; the index card does not. A scoped `IndexEntrySummary` for `/api/lifecycle` would keep both wins.
- **Simplicity vs accessibility on PipelineDots**: rendering as a `<ul>` of inert `<li>` is simple but hides state from screen-readers; `<div role="img" aria-label="...">` or per-dot `aria-label` adds verbosity but actually conveys the data.

### Findings

#### Major

- 🟡 **Architecture**: Notes entries silently disappear from the cluster detail view
  **Location**: Step 9: LifecycleClusterView (timeline rendering); cross-references Step 4 (LIFECYCLE_PIPELINE_STEPS) and server `clusters.rs::canonical_rank`
  Server `compute_clusters` only filters out `Templates`, so `Notes` entries are clustered alongside the eight pipeline stages and `has_notes` is part of `Completeness`. The frontend's `LIFECYCLE_PIPELINE_STEPS` deliberately omits Notes and the timeline filters by `step.docType` — any `Notes` entry is silently dropped from the rendered timeline despite being on the wire.

- 🟡 **Architecture**: SSE invalidation uses a hard-coded prefix string instead of `queryKeys`
  **Location**: Step 6: useDocEvents cluster-prefix invalidation
  `queryClient.invalidateQueries({ queryKey: ['lifecycle-cluster'] })` bypasses the `queryKeys.lifecycleCluster(slug)` helper; if the prefix is renamed in `query-keys.ts`, SSE invalidation silently breaks with no compile-time link.

- 🟡 **Code Quality**: Heading-skipping logic does not match the documented strategy
  **Location**: Step 2: `body_preview_from` in `frontmatter.rs`
  The strategy comment says "skip a leading H1", but the implementation skips every heading and carries a dead `seen_h1` flag that is set but never read. Tests pass because they happen to align with "skip all headings", but the misleading comment + dead state will trip future maintainers.

- 🟡 **Test Coverage**: No regression test pinning that `last_changed_ms` survives the alphabetic slug sort
  **Location**: Step 1: Server — `last_changed_ms`
  Both new tests use single-cluster fixtures. A bug that swaps the wrong index after the alphabetic-by-slug sort, or computes the max over all entries rather than per-bucket, would not be caught by either test.

- 🟡 **Test Coverage**: `body_preview_from` edge cases are under-tested at the boundary
  **Location**: Step 2: Server — `body_preview_from` tests
  Missing: exactly-200-char body (truncate boundary, `>` vs `>=`); H2-only first content line; multi-line first paragraph; interleaved blank lines and headings before content; lines with leading whitespace after headings.

- 🟡 **Test Coverage**: SSE cluster-prefix invalidation test only covers `doc-changed`
  **Location**: Step 6: Frontend — useDocEvents
  `dispatchSseEvent` triggers on either `doc-changed` OR `doc-invalid`. Only the first is covered. After a `doc-invalid` event, open detail views could silently keep stale data — exactly the bug the invalidation is meant to prevent.

- 🟡 **Test Coverage**: Sort tiebreaks and full-pipeline (8/8) cluster are not covered
  **Location**: Step 8: Frontend — LifecycleIndex tests
  Two clusters with identical completeness scores or identical mtimes don't appear in any fixture; tiebreak ordering is therefore not pinned. No fixture has all 8 booleans true.

- 🟡 **Standards**: TypeScript `Completeness.hasNotes` typed as optional but server emits it as required
  **Location**: Step 4: Frontend — types.ts, Completeness interface
  The Rust struct declares `pub has_notes: bool` (required); the proposed TS interface declares `hasNotes?: boolean`. Wire reality is "always present"; the loose TS type drifts the client model away from the server's serde shape.

- 🟡 **Usability**: `PipelineDots` stage state not reliably announced to screen readers
  **Location**: Step 7: PipelineDots component
  Per-stage state is conveyed only via colour and `title` on `<li>`. `title` is unreliable for assistive tech; colour alone fails WCAG 1.4.1. Keyboard/screen-reader users cannot tell which stages are present vs absent.

- 🟡 **Usability**: Detail view has no back-link or breadcrumb to `/lifecycle`
  **Location**: Step 9: LifecycleClusterView
  `LifecycleLayout` is just `<Outlet />`; nothing in the view returns to the index. Deep-linked or SSE-arrived users have only the browser back button.

- 🟡 **Usability**: Error message leaks raw URL and HTTP status to end-users
  **Location**: Step 9: LifecycleClusterView, error branch
  Renders `Failed to load cluster does-not-exist: GET /api/lifecycle/does-not-exist: 404` — internal API path, HTTP semantics, no actionable next step. Conflates "not found" with "server unreachable".

- 🟡 **Usability**: No search/filter on the lifecycle index
  **Location**: Step 8: LifecycleIndex toolbar
  Three sort modes but no text filter; once cluster count exceeds ~20–30 the index becomes a scroll-and-Ctrl-F surface. Users will URL-type slugs they happen to remember.

#### Minor

- 🔵 **Architecture**: Presentational `bodyPreview` widens the universal `IndexEntry` contract for one consumer
  **Location**: Step 2 / Step 4
  `IndexEntry` is the canonical record returned by every doc-list endpoint; adding `body_preview` couples the index contract to the lifecycle view's display optimisation. Future presentational fields will face the same precedent.

- 🔵 **Architecture**: Test-only `slug?` prop creates two routing paths
  **Location**: Step 9: LifecycleClusterView signature
  Production never passes the prop, but the dual path must be maintained; `enabled: slug.length > 0` hides bugs where the route param is genuinely missing.

- 🔵 **Code Quality**: Placeholder copy duplicates pipeline labels and risks drift
  **Location**: Step 9: LifecycleClusterView
  `no {step.label.toLowerCase()} yet` produces "no pr yet" / "no pr review yet" — acronyms read as typos. Copy is generated, not authored.

- 🔵 **Code Quality**: Sort-button selector relies on case-insensitive name match that may collide with active state
  **Location**: Step 8: LifecycleIndex.test.tsx
  `getByRole('button', { name: /completeness/i })` is permissive; PipelineDots tests already use the exact-match style (`/^Plan$/`).

- 🔵 **Code Quality**: `formatMtime` duplicates a likely-existing helper and bakes in `Date.now()` non-determinism
  **Location**: Step 8: LifecycleIndex.tsx
  Inline implementation, no test seam, plausibly duplicated in LibraryTypeView; future tests asserting the meta line will need fake timers.

- 🔵 **Code Quality**: Repeated `frontmatter as Record<string, unknown>` casts indicate primitive obsession
  **Location**: Step 9: LifecycleClusterView EntryCard
  Same idiom appears in `LibraryTypeView`. Centralise via `frontmatterField(entry, key)` (or typed accessors) before it proliferates.

- 🔵 **Code Quality**: Plan relies on grep sweeps to fix struct-extension fallout
  **Location**: Step 2d / Step 4
  Rust compiler will catch literals; `..Default::default()` or builder patterns wouldn't be flagged. A `entry_for_test(...)` helper / `makeIndexEntry(overrides)` factory would absorb this.

- 🔵 **Correctness**: Mid-paragraph headings continue rather than terminate the preview
  **Location**: Step 2: body_preview_from
  `"First para.\n## Heading\nMore text.\n"` produces `"First para. More text."`. Stated strategy says "first non-heading paragraph"; implementation splices across heading-separated paragraphs.

- 🔵 **Correctness**: `useParams({ strict: false })` cast to `{ slug?: string }` erases route typing
  **Location**: Step 9: LifecycleClusterView
  A future route under `/lifecycle` with a different param name won't be caught by the compiler; `params.slug` will silently be `undefined`.

- 🔵 **Correctness**: `isLoading` is true when `enabled: false`, showing a perpetual loading spinner for empty slug
  **Location**: Step 9: LifecycleClusterView
  Empty-slug deep-link or pre-router-resolution renders `<p>Loading…</p>` indefinitely.

- 🔵 **Correctness**: Error/empty-state ordering masks `undefined` data as the empty branch
  **Location**: Step 8: LifecycleIndex
  `data: clusters = []` default would render "No lifecycle clusters yet" if the helper ever returned undefined on a soft-failure (e.g., 204).

- 🔵 **Standards**: `<ol>` with `aria-label="Lifecycle pipeline"` may be the wrong semantic for a row of indicator dots
  **Location**: Step 7: PipelineDots
  Screen readers announce "list, 8 items" then walk anonymous items with only `title` attributes — verbose and uninformative. Consider `<div role="img" aria-label="Lifecycle pipeline: 3 of 8 stages complete">` or `<ul>` with `<span class="sr-only">` per dot.

- 🔵 **Standards**: Empty-state copy diverges from established style
  **Location**: Step 8: LifecycleIndex
  Existing views say "No documents found."; the plan introduces "No lifecycle clusters yet." Trailing "yet" is a new tone for the codebase.

- 🔵 **Standards**: `data-testid` attribute introduced where the codebase uses semantic queries
  **Location**: Step 9: LifecycleClusterView
  No existing `data-testid` precedent; existing tests use Testing Library role/text queries plus `data-stage` / `data-present` for visual state. Pick one convention.

- 🔵 **Compatibility**: Integration test only covers `/api/lifecycle`, not `/api/lifecycle/:slug` or `/api/docs`
  **Location**: Step 3: Server
  Both also serialise `LifecycleCluster`/`IndexEntry`; a future divergent serialiser could silently drop the new fields from those endpoints.

- 🔵 **Compatibility**: Default-features Rust build/test path is mentioned but not in the verification block
  **Location**: Verification + Full success criteria
  "Desired end state" promises "default-features lib pass" but the checklist only runs `--features dev-frontend`.

- 🔵 **Compatibility**: `key: keyof Completeness` widens to include the optional `hasNotes`
  **Location**: Step 4: LIFECYCLE_PIPELINE_STEPS
  Explicit annotation erases the `as const` literal-tuple guarantee; future contributor could append `{ key: 'hasNotes', ... }` without a type error.

- 🔵 **Compatibility**: `grep` sweep scope is `server/src/` only; misses `server/tests/`
  **Location**: Step 2d
  Compiler catches it eventually, but following the plan literally leaves the implementer briefly confused after a clean `src/` sweep.

- 🔵 **Compatibility**: No verification that the existing top-nav link to `/lifecycle` still resolves correctly
  **Location**: Step 10: router rewrite
  Layout-with-index reshape can break TanStack Router active-link detection depending on `activeOptions`. Sidebar active-state regression is plausible and visible.

- 🔵 **Performance**: SSE event storms cause unthrottled cluster-detail refetches
  **Location**: Step 6: useDocEvents
  Bulk filesystem ops (git checkout, formatter run) produce a burst of SSE events; each one re-fetches `/api/lifecycle` and every mounted cluster-detail view. Trailing-debounce coalescing would turn N events into one refetch.

- 🔵 **Performance**: Per-entry `bodyPreview` inflates `/api/lifecycle` payload and watcher-triggered clones
  **Location**: Step 1 + Step 2
  ~200 bytes × all entries in all clusters per fetch; cloned end-to-end on every watcher debounce. The index card never reads `bodyPreview`.

- 🔵 **Test Coverage**: `PipelineDots` tests query implementation-specific `[data-stage]` attributes
  **Location**: Step 7
  Couples tests to chosen markup; renaming the attribute or migrating to ARIA forces a rewrite even though behaviour is unchanged.

- 🔵 **Test Coverage**: Body-preview omit assertion uses `data-testid`, an implementation hook
  **Location**: Step 9
  Production attribute exists purely for tests; semantic queries (`screen.queryByText`) would be more behaviour-pinning.

- 🔵 **Test Coverage**: No test covers a cluster with multiple entries of the same stage type
  **Location**: Step 9
  Plan-review-2 after revision is realistic; ordering and rendering of multi-entry stages is not pinned.

- 🔵 **Test Coverage**: Loading/error tests do not cover the `slug=''` disabled-query branch
  **Location**: Step 9
  See correctness finding above; the resulting branch (perpetual "Loading…" or fallthrough error) is unpinned.

- 🔵 **Test Coverage**: Integration test only exercises the list endpoint, not `/api/lifecycle/:slug`
  **Location**: Step 3
  Detail-endpoint serialization regression would only surface in manual testing.

- 🔵 **Test Coverage**: No test covers deep-link to an unknown cluster slug at the router layer
  **Location**: Step 10
  Manual checklist mentions it; automated coverage missing.

- 🔵 **Test Coverage**: No coverage for `body_preview_from` interaction with code fences / lists / blockquotes
  **Location**: Step 2
  Realistic content shapes (` ```rust `, `- item`, `> quote`); contract for handling them isn't pinned.

- 🔵 **Code Quality**: Magic-string query-key prefix duplicates `query-keys.ts`
  **Location**: Step 6
  (Same root as the architecture-major finding; reinforces the recommendation to expose a `lifecycleClusterPrefix()` accessor.)

- 🔵 **Code Quality**: Optional `hasNotes` carried for compatibility creates a quiet code smell
  **Location**: Step 4
  (Same root as the standards-major finding.)

- 🔵 **Architecture**: Completeness wire shape is asymmetric: `hasNotes` required server-side, optional client-side
  **Location**: Step 4
  (Same root as standards-major; reinforced from the architecture lens.)

- 🔵 **Correctness**: `seen_h1` flag is set but never read — dead state
  **Location**: Step 2
  (Same root as the code-quality-major finding.)

- 🔵 **Correctness**: `setQueryData(key, null)` may not register a query reliably across TanStack Query versions
  **Location**: Step 6
  Test contract for `isInvalidated` on observerless cached queries varies subtly.

- 🔵 **Correctness**: `i64` type for `last_changed_ms` permits negative values that the integration test asserts against
  **Location**: Step 1 / Step 3
  `assert!(last > 0)` would fail on clock-skewed systems; use `>= 0` or clamp in the cluster step.

- 🔵 **Correctness**: `encodeURIComponent` on slugs containing slashes still hits a 404
  **Location**: Step 5
  Unlikely in practice (slugs are kebab-cased), but failure mode is opaque; either narrow slug shape or document the constraint.

- 🔵 **Usability**: `formatMtime` jumps abruptly from "23h ago" to full localised timestamp
  **Location**: Step 8
  Long localised strings compete for horizontal space in the meta row.

- 🔵 **Usability**: Sort pills use `aria-pressed`; a `radiogroup` would communicate single-choice intent better
  **Location**: Step 8
  Three mutually-exclusive toggles read as independent toggles to screen readers.

- 🔵 **Usability**: Hover affordance on cards is border-only — weak signal that the card is a link
  **Location**: Step 8
  Adding a `box-shadow` or title-colour shift on hover would match the detail-view's `entryLink:hover` pattern.

- 🔵 **Usability**: Inconsistent capitalisation: 'Plan review' vs 'PR review' compounded by `toLowerCase()`
  **Location**: Step 4 + Step 9
  (Same root as the multi-lens placeholder-text finding.)

- 🔵 **Usability**: `webkit-line-clamp` lacks a Firefox/non-webkit fallback
  **Location**: Step 9
  Add `line-clamp: 3` and a `max-height` fallback to cap card heights for non-supporting browsers.

- 🔵 **Usability**: Mixing prop-passthrough and router-params is non-idiomatic for TanStack Router
  **Location**: Step 9
  (Same root as the architecture-minor finding.)

- 🔵 **Usability**: Required `bodyPreview: string` forces a wide test-fixture sweep
  **Location**: Step 4
  (Same root as the code-quality-minor finding; recommend `makeIndexEntry(overrides)` factory.)

#### Suggestions

- 🔵 **Architecture**: 404 vs 5xx distinguished only by stringly-typed error messages — consider a typed `FetchError` class.
- 🔵 **Architecture**: Pipeline ordering knowledge duplicated across client `LIFECYCLE_PIPELINE_STEPS` and server `canonical_rank`; a `/api/lifecycle/pipeline` endpoint or shared definition would make the contract explicit.
- 🔵 **Standards**: Step 10 subsection numbering is `9a/9b/9c/9d`; renumber to `10a/...`.
- 🔵 **Standards**: Extract `formatMtime` to a shared utility shared with `LibraryTypeView`.
- 🔵 **Standards**: Sidebar `<nav>` lacks `aria-label`; out of scope for Phase 6 but worth a follow-up.
- 🔵 **Performance**: `compute_clusters` does an extra O(n) max-mtime walk per cluster; trivial at expected sizes — keep for clarity.
- 🔵 **Performance**: 8x per-step filter on `cluster.entries` in detail view; `Object.groupBy` once if entry counts ever grow.
- 🔵 **Performance**: Extend the existing 2000-file scan benchmark to assert the budget still holds with `body_preview` populated.
- 🔵 **Test Coverage**: SSE burst-of-events scenario (back-to-back `doc-changed` events) is not pinned.
- 🔵 **Usability**: Slug rendered alongside title may be visual noise; consider hover-only or `title` attribute.

### Strengths

- ✅ `LIFECYCLE_PIPELINE_STEPS` as a single source of truth (key + docType + label) — adding a stage is a one-line change and the constant is reused by `PipelineDots`, the index sort, and the timeline.
- ✅ Routing structure mirrors the library subtree (Layout + index + detail) and uses the same TanStack Router primitives, keeping the route tree self-similar.
- ✅ Test-first ordering at every step with named test counts and explicit cargo/npm verification gates between steps; failures localise to one boundary at a time.
- ✅ Functional core / imperative shell separation preserved: `compute_clusters`, `body_preview_from`, `completenessScore`, `sortClusters` are all pure and unit-testable.
- ✅ UTF-8 boundary safety in truncation via `chars().take(PREVIEW_MAX_CHARS)`; explicit test for the `é` repetition case.
- ✅ Field additions are purely additive on the wire — no field renames, removals, or type changes; both producer and consumer updated in lockstep.
- ✅ Server-side `last_changed_ms` keeps sort logic on the server's data model rather than re-walking entries client-side.
- ✅ Explicit acknowledgement of the SSE prefix-invalidation tradeoff with a code comment that lives next to the implementation.
- ✅ Plan-time integration test pins the camelCase wire contract (`lastChangedMs`, `bodyPreview`) on `/api/lifecycle` — the right level for catching serde drift.
- ✅ `body_preview` computed once at index time on already-loaded body strings, eliminating per-card `fetchDocContent` round-trips.
- ✅ `useMemo` correctly placed before conditional early returns, respecting Rules of Hooks.
- ✅ Hex-colour palette in proposed CSS modules matches the rest of the project; copy ("Loading…", "Failed to load …: {err}") matches existing views verbatim.

### Recommended Changes

Ordered by impact:

1. **Resolve the Notes / `hasNotes` schema asymmetry** (addresses: Notes silently disappear; `hasNotes` optional in TS but required server-side; `keyof Completeness` widening; carrying-for-compatibility code smell)
   Pick one of: (a) drop Notes from `compute_clusters` so the wire matches what the timeline renders; (b) extend `LIFECYCLE_PIPELINE_STEPS` with a 9th Notes stage rendered as a long-tail bucket; (c) explicitly render an "Other" group below the pipeline. Whichever you choose, declare `hasNotes: boolean` (required) on the TS side, and either narrow `LIFECYCLE_PIPELINE_STEPS`'s `key` to a literal stage-key union or drop the explicit annotation so `as const` keeps the 8-element tuple. Document the decision next to `LIFECYCLE_PIPELINE_STEPS`.

2. **Centralise the `lifecycle-cluster` prefix in `query-keys.ts`** (addresses: SSE invalidation hard-coded prefix; magic-string duplication)
   Add `lifecycleClusterPrefix: () => ['lifecycle-cluster'] as const` (or similar) to `queryKeys`. Have both `lifecycleCluster(slug)` and the SSE dispatcher consume it. Update Step 6 and the test accordingly.

3. **Align `body_preview_from` intent, code, and tests** (addresses: heading-skipping logic mismatch; mid-paragraph headings; dead `seen_h1` flag; under-tested edge cases)
   Decide whether the helper "skips all headings" (current behaviour) or "skips one leading H1 then terminates on any further heading" (stated intent). Update the docstring, drop or repurpose `seen_h1`, and add tests for: exactly-200-char body returns no ellipsis; H2-only first-content line; mid-paragraph heading; multi-line first paragraph join; code-fence/list/blockquote first content. Also broaden the integration test in Step 3 to exercise `/api/lifecycle/:slug` and add `bodyPreview` presence to a `/api/docs` test.

4. **Replace the `slug?` test escape hatch with a clean split** (addresses: dual routing path; type erasure on `useParams({ strict: false })`; perpetual-loading on empty slug)
   Split `LifecycleClusterView` into a router-bound shell (`useParams` from the strict route) wrapping a pure `LifecycleClusterContent({ slug, ... })`. Tests render the inner component directly; production never has an empty slug. Drop `enabled: slug.length > 0` and the empty-string fallback.

5. **Improve the cluster-detail view's failure UX** (addresses: error message leaks raw URL/HTTP status; no back-link to the index)
   (a) Add a `<Link to="/lifecycle">← All clusters</Link>` at the top of the detail view (or inside `LifecycleLayout`). (b) Introduce a typed `FetchError extends Error` carrying `status: number`, branch on 404 to render `'No cluster called "{slug}" exists.'` plus a back-link, and a generic message otherwise. The same `FetchError` can also be used in the index view's error branch.

6. **Strengthen test coverage for the new behavioural surfaces** (addresses: `last_changed_ms` cross-cluster pinning; SSE `doc-invalid` not covered; sort tiebreaks; full-pipeline cluster; multi-entry stages; router 404 deep-link; `data-testid` / `data-stage` query coupling)
   Add: a 2-cluster `last_changed_ms` test; a `doc-invalid` SSE invalidation case (parametrise the existing test); fixture pairs with identical completeness scores AND identical mtimes to pin tiebreak order; an all-8-true cluster fixture; a 2-plan-review fixture for stage grouping; a router test deep-linking to an unknown slug. Replace `data-testid="entry-body-preview"` with semantic `screen.queryByText` queries; replace `[data-stage]` queries with `getAllByRole('listitem')` (or the role chosen for accessibility — see #8).

7. **Add a search/filter input to the lifecycle index** (addresses: missing search/filter usability gap)
   Even a substring case-insensitive match over `title` + `slug`, controlled by a `<input type="search">` in the toolbar, materially improves the index UX. If genuinely deferred to a later phase, document it explicitly under "What we are NOT doing" with a rationale.

8. **Make `PipelineDots` accessible** (addresses: stage state not announced to screen readers; `<ol>` semantics for indicator dots)
   Either render as `<div role="img" aria-label="Lifecycle pipeline: N of 8 stages complete (...)">` with inert spans, or keep `<ul>`/`<li>` and add per-dot `aria-label={\`${step.label}: ${present ? 'present' : 'missing'}\`}` plus a non-colour signal (filled/hollow ring or check/×) so the indicator works without colour. Update the tests to query roles/labels rather than `data-stage`/`data-present`.

9. **Introduce test factories to absorb future field additions** (addresses: grep-sweep brittleness in Rust and TypeScript; compat sweep scope missing `server/tests/`)
   Add `entry_for_test(...)` in a shared Rust test helpers module (used by `clusters.rs`, `watcher.rs`, etc.) and `makeIndexEntry(overrides: Partial<IndexEntry>): IndexEntry` in `frontend/src/api/test-fixtures.ts`. Migrate fixtures opportunistically. Step 2d's grep guidance also gains `server/tests/` (or simply runs `cargo build --tests --features dev-frontend` as the authoritative check).

10. **Move shared formatting and consider scoping `bodyPreview` to the lifecycle endpoints** (addresses: `formatMtime` duplication and non-determinism; `bodyPreview` payload bloat on universal endpoints; SSE storm refetches)
    (a) Extract `formatMtime` to `frontend/src/api/format.ts` (or similar) accepting an injectable `now`, importable by `LifecycleIndex` and `LibraryTypeView`. Extend the relative ladder to `<7d` / `<30d` so the jump from "23h ago" to a full timestamp is softened. (b) Decide whether `bodyPreview` should live on every `IndexEntry` or only on cluster entries: introduce a server-side projection / `IndexEntrySummary` so `/api/docs` and the lifecycle index list don't carry a per-entry preview that nobody renders. (c) Add a trailing 100–200ms debounce in `dispatchSseEvent` so bulk filesystem operations coalesce into a single invalidation per affected query.

11. **Smaller polish items** (addresses: lowercase `pr` typo; webkit-line-clamp fallback; `aria-pressed` vs radiogroup for sort buttons; empty-state copy "yet"; subsection numbering; sidebar active-link verification; default-features build in checklist; primitive-obsession on frontmatter access)
    Apply each as a quick edit. Most live in Step 4 (add a `placeholder` field on `LIFECYCLE_PIPELINE_STEPS`), Step 8 (radiogroup or pressed-with-`role=group`; `No lifecycle clusters found.`), Step 9 (add `line-clamp: 3` and `max-height` fallback; render `placeholder` directly), Step 10 (renumber to `10a..10d`, add a sidebar-active-link assertion to one router test), Verification block (add `cargo test --lib` no-features line), and an optional `frontmatterField` helper in `api/types.ts` to absorb the cast/narrow idiom shared with `LibraryTypeView`.

---

## Per-Lens Results

### Architecture

**Summary**: The plan extends a clean, well-modularised system in a way that is largely consistent with established patterns from Phase 5. Concerns are primarily about shared-knowledge coupling between client and server domain models — particularly the lifecycle pipeline ordering, the Notes doc type that exists server-side but is intentionally absent from the frontend pipeline, and presentational state (`bodyPreview`) leaking into the universal `IndexEntry` shape.

**Strengths**: `LIFECYCLE_PIPELINE_STEPS` single source of truth; routing mirrors library subtree; explicit SSE-invalidation tradeoff comment; server-side `last_changed_ms` keeps sort on the server's data model; TDD checkpoints between steps; pure functional core preserved.

**Findings** (selected — see Findings sections above for full list):
- 🟡 Notes entries silently disappear from cluster detail view (high)
- 🟡 SSE invalidation uses hard-coded prefix string (high)
- 🔵 `bodyPreview` widens universal IndexEntry contract (medium)
- 🔵 Test-only `slug?` prop creates dual routing paths (high)
- 🔵 `Completeness.hasNotes` shape asymmetric server vs client (medium)
- 🔵 404 vs 5xx distinguished only by stringly-typed errors (medium)
- 🔵 Pipeline ordering duplicated across client/server (low)

### Code Quality

**Summary**: Plan is well-structured, follows TDD, mirrors existing patterns, and keeps each step modestly scoped. Subtle quality issues: `body_preview_from` algorithm with a confusingly-handled heading path, sort-button selector brittleness, duplication between timeline placeholders and pipeline labels.

**Strengths**: Single source of truth for stages; pure helper isolated next to existing `title_from`; independently committable steps; deliberate testability seam; Rules of Hooks discipline called out; uniform error/loading/empty handling.

**Findings**:
- 🟡 Heading-skipping doesn't match documented strategy (high)
- 🔵 Placeholder copy duplicates labels and risks drift (high)
- 🔵 Sort-button selector relies on permissive name match (medium)
- 🔵 `formatMtime` duplicates and bakes in `Date.now()` (medium)
- 🔵 Magic-string query-key prefix duplicates `query-keys.ts` (high)
- 🔵 Optional `hasNotes` is a quiet code smell (medium)
- 🔵 Repeated `frontmatter as Record<...>` casts indicate primitive obsession (high)
- 🔵 Plan relies on grep sweeps to fix struct-extension fallout (medium)

### Test Coverage

**Summary**: Admirably TDD-driven with explicit test counts at each step, covers happy paths, error paths, loading, and contract pinning at the API wire. Several risk surfaces under-tested: clusters with all 8 stages, body_preview boundary cases, sort tiebreaks, `doc-invalid` SSE path, and several behavioural assertions are coupled to implementation details.

**Strengths**: Test-first ordering with sweep-the-fixtures step; integration test pins camelCase wire contract; UTF-8 edge case explicit; uses TanStack Query's `isInvalidated` rather than spying on internals; aggregate test counts locked in.

**Findings**:
- 🟡 No regression test pinning `last_changed_ms` after alphabetic sort (high)
- 🟡 body_preview_from edge cases under-tested at boundary (high)
- 🟡 SSE invalidation only covers `doc-changed`, not `doc-invalid` (high)
- 🟡 Sort tiebreak and full-pipeline cluster not covered (medium)
- 🔵 PipelineDots tests query implementation `[data-stage]` attributes (high)
- 🔵 Body-preview omit assertion uses `data-testid` (high)
- 🔵 No multi-entry-per-stage cluster fixture (medium)
- 🔵 Loading/error tests miss `slug=''` disabled-query branch (medium)
- 🔵 Integration test only exercises the list endpoint (medium)
- 🔵 No test for router deep-link to unknown slug (low)
- 🔵 No coverage for body_preview interaction with code fences / lists / blockquotes (medium)
- 🔵 No test for SSE burst-of-events / concurrent invalidation (low)

### Correctness

**Summary**: Logic is largely sound for the stated goals, with careful UTF-8 handling. Subtle correctness concerns: `body_preview_from` diverges from its stated intent, `seen_h1` is dead state, and `LifecycleClusterView`'s slug computation has empty-slug edge cases.

**Strengths**: Truncation uses `chars().take(...)` correctly; broad-prefix SSE invalidation justified; `completenessScore` correctly excludes `hasNotes`; explicit tiebreakers; `encodeURIComponent` applied; hooks ordering preserved; `Boolean(...)` coerces optional flags.

**Findings**:
- 🔵 `seen_h1` is set but never read — dead state (high)
- 🔵 Mid-paragraph headings continue rather than terminate the preview (high)
- 🔵 `useParams({ strict: false })` cast erases route typing (medium)
- 🔵 `isLoading=true` when `enabled: false` shows perpetual loading (medium)
- 🔵 `encodeURIComponent` on slugs with slashes still hits 404 (high)
- 🔵 `setQueryData(key, null)` may not register a query reliably (medium)
- 🔵 Error/empty-state ordering masks `undefined` data (medium)
- 🔵 `i64` for `last_changed_ms` permits negatives the test asserts against (medium)

### Standards

**Summary**: Phase 6 broadly follows established conventions: file/component naming, router structure, query-key shape, fetch helpers, error/loading copy, CSS hex usage. The most concrete divergence is the server/client `Completeness.hasNotes` mismatch, plus minor inconsistencies around empty-state copy, accessibility semantics for indicator dots, and a misnumbered step heading.

**Strengths**: Naming follows Layout/Index/View suffix convention; router mirrors library subtree; fetch helpers consistent; error/loading copy matches existing views verbatim; serde camelCase + snake_case Rust fields; hex-colour palette consistent; aria-pressed on toggle buttons appropriate.

**Findings**:
- 🟡 `Completeness.hasNotes` typed optional but server emits required (high)
- 🔵 `<ol>` with aria-label may be wrong semantic for indicator dots (high)
- 🔵 Empty-state copy "...yet." diverges from "found." style (high)
- 🔵 `data-testid` introduced where codebase uses semantic queries (medium)
- 🔵 Lowercased "pr" / "pr review" placeholder copy reads as typos (medium)
- 🔵 Step 10 subsection numbering is `9a/9b/9c/9d` (high)
- 🔵 `formatMtime` duplicated rather than shared with LibraryTypeView (medium)
- 🔵 Sidebar `<nav>` lacks `aria-label` (low)

### Usability

**Summary**: Solid basic UX with sensible defaults and a well-designed `LIFECYCLE_PIPELINE_STEPS` constant. Real gaps: missing search/filter, no back-link from detail to index, error messages leak raw URLs, screen-reader users get only `title` attributes from PipelineDots, and `formatMtime` switches abruptly at 24h. The `bodyPreview` migration imposes a wide test-fixture sweep a `Partial<IndexEntry>` factory would have absorbed.

**Strengths**: Single source of truth for stages; fetch helpers mirror existing pattern; default sort matches "what changed?" intent; explicit empty/loading/error branches; faded placeholders give clear signals; SSE prefix-invalidation pragmatic and well-justified.

**Findings**:
- 🟡 PipelineDots state not announced to screen readers (high)
- 🟡 Detail view has no back-link / breadcrumb (high)
- 🟡 Error message leaks raw URL/HTTP status (high)
- 🟡 No search/filter on the index (high)
- 🔵 `formatMtime` jumps abruptly from "23h ago" to full timestamp (high)
- 🔵 Sort pills should be a radiogroup, not toggle buttons (medium)
- 🔵 Hover affordance on cards is border-only — weak signal (medium)
- 🔵 Inconsistent capitalisation 'Plan review' vs 'PR review' (high)
- 🔵 `webkit-line-clamp` lacks Firefox/non-webkit fallback (medium)
- 🔵 Mixing prop-passthrough and router-params is non-idiomatic (medium)
- 🔵 Required `bodyPreview` forces wide test-fixture sweep (high)
- 🔵 Slug rendered alongside title may be visual noise (low)

### Performance

**Summary**: Modest, well-bounded work: a string-scan body preview at index time, a max-mtime per cluster, one extra SSE invalidation prefix. No O(n^2) hot paths. Main concerns are payload growth on `/api/lifecycle`, retained-memory growth in the cluster snapshot (cloned on every watcher debounce), and broad-prefix invalidation that couples every SSE event to a refetch of every open detail without coalescing.

**Strengths**: `body_preview` computed once at index time; `lastChangedMs` hoists what would otherwise be repeated client-side mtime walks; truncation bounded; tradeoff acknowledged inline; `useMemo` placed correctly.

**Findings**:
- 🔵 Per-entry `bodyPreview` inflates `/api/lifecycle` payload and watcher-triggered clones (medium)
- 🔵 SSE event storms cause unthrottled cluster-detail refetches (high)
- 🔵 `compute_clusters` does an extra O(n) max-mtime walk per cluster (medium) — keep for clarity
- 🔵 8x per-step filter on `cluster.entries` (medium) — skip until measured
- 🔵 `body_preview_from` re-walks the body that frontmatter parsing produced (low) — extend the 2000-file benchmark to lock the budget

### Compatibility

**Summary**: Two additive wire fields (`lastChangedMs`, `bodyPreview`) plus a router restructure. With one consumer (in-repo frontend) and required-field choices on both sides, contract risk is low and compilers will surface most affected sites. Main gaps: incomplete verification coverage (default-features build, list-only integration test, no `/api/docs` field assertion).

**Strengths**: Field additions purely additive; producer + consumer updated in lockstep; required-on-both-sides eliminates optional/undefined ambiguity; `hasNotes` left optional client-side gives forward compat (but contradicts Standards lens — see findings); `/lifecycle/$slug` matches existing TanStack Router pattern; struct-literal sweep documented.

**Findings**:
- 🔵 Integration test only covers `/api/lifecycle` (high)
- 🔵 Default-features Rust build/test path missing from verification (high)
- 🔵 `key: keyof Completeness` widens to include optional `hasNotes` (medium)
- 🔵 grep sweep scope is `server/src/` only; misses `server/tests/` (medium)
- 🔵 No verification that the existing top-nav `/lifecycle` link still resolves (high)

---

## Re-Review (Pass 2) — 2026-04-25

**Verdict:** REVISE

The plan now successfully addresses every prior major finding. Notes is a 9th `longTail` step rendered in a separate "Other" section; SSE invalidation flows through `queryKeys.lifecycleClusterPrefix()`; `body_preview_from` strategy and implementation align (with mid-paragraph heading termination + boundary tests); the cluster regression, `doc-invalid` SSE, sort tiebreak, and multi-entry-per-stage tests all land; `Completeness.hasNotes` is required; PipelineDots gains per-dot `aria-label` and a non-colour signal; the detail view has a back-link reachable from both states; `FetchError` drives 404-vs-5xx branching with no URL leak in the cluster detail copy; the index has a search/filter input.

Two **new** major issues surfaced from the edits, both small and well-localised:

1. **Build-breaking** — Step 10b declares `const lifecycleClusterRoute = createRoute(...)` (no `export`), but the new `LifecycleClusterView` shell imports `lifecycleClusterRoute` from `'../../router'` to call `useParams()`. As written, the component split (which retired the `useParams({ strict: false })` cast and the empty-slug `enabled` guard) won't compile. Fix: change to `export const`.
2. **Incomplete error-leak fix** — Finding #11's URL-leak fix landed in `LifecycleClusterContent` (cluster detail) but the same problem persists in `LifecycleIndex`'s error branch, which still does `error.message` substitution. With `FetchError`'s message preserving `'GET /api/lifecycle: 500'`, the index page leaks the very URL the cluster-detail copy was rewritten to hide. Fix: branch on `error instanceof FetchError` and render generic copy, mirroring the cluster-detail treatment.

A handful of new minors and suggestions also surfaced — most are small polish items or deferred-by-rationale tradeoffs already documented under "What we are NOT doing".

### Previously Identified Issues

#### Resolved
- ✅ **Architecture/Standards**: Notes silently disappear from cluster detail view — Resolved via 9th `longTail` step + "Other" section.
- ✅ **Architecture/Code Quality**: SSE invalidation hard-codes prefix — Resolved via `queryKeys.lifecycleClusterPrefix()`.
- ✅ **Code Quality/Correctness/Test Coverage**: `body_preview_from` heading-skip mismatch + dead `seen_h1` — Resolved; new test pins mid-paragraph heading termination.
- ✅ **Test Coverage**: No regression test for `last_changed_ms` after slug sort — Resolved.
- ✅ **Test Coverage**: SSE only covers `doc-changed` — Resolved (`doc-invalid` test added).
- ✅ **Test Coverage**: `body_preview_from` boundary cases — Resolved (200/201/multi-line/mid-heading tests).
- ✅ **Test Coverage**: Sort tiebreaks not covered — Resolved.
- ✅ **Test Coverage**: PipelineDots queries `[data-stage]` — Resolved (now `getAllByRole('listitem')`).
- ✅ **Test Coverage**: `data-testid` body-preview omit assertion — Resolved (semantic queries; `data-testid` removed from prod).
- ✅ **Test Coverage**: No multi-entry-per-stage test — Resolved.
- ✅ **Test Coverage**: Integration test only on `/api/lifecycle` — Resolved (detail-endpoint test added).
- ✅ **Standards**: `Completeness.hasNotes` asymmetric — Resolved (required field, `PipelineStepKey` literal union).
- ✅ **Usability**: PipelineDots not announced to screen readers — Resolved (per-dot `aria-label` + dashed-border / inner-dot non-colour signals).
- ✅ **Usability**: Detail view has no back-link — Resolved (back-link in both success and error states).
- ✅ **Usability**: `formatMtime` jumps from 23h to full timestamp — Resolved (extended ladder).
- ✅ **Usability**: Hover affordance on cards — Resolved (box-shadow + title-colour shift).
- ✅ **Usability**: `webkit-line-clamp` lacks fallback — Resolved (standard `line-clamp` + `max-height`).
- ✅ **Usability/Architecture/Correctness**: `slug?` prop dual routing path — Resolved via `LifecycleClusterView` (shell) + `LifecycleClusterContent` (renderer) split. Subject to the new build-breaking finding below.
- ✅ **Usability/Code Quality**: `bodyPreview` test-fixture sweep churn — Resolved via `makeIndexEntry`.
- ✅ **Code Quality/Standards/Usability**: Lowercased "no pr yet" placeholder — Resolved (explicit `placeholder` field per step).
- ✅ **Standards**: `<ol>` semantic for indicator dots — Resolved (`<ul>`).
- ✅ **Standards**: Empty-state copy "...yet." — Resolved ("...found.").
- ✅ **Standards**: Step 10 subsection numbering — Resolved (10a/10b/10c/10d).
- ✅ **Compatibility**: Default-features build missing — Resolved (`cargo test --lib` no-features added).
- ✅ **Compatibility**: `keyof Completeness` widens — Resolved via `PipelineStepKey` literal union.
- ✅ **Compatibility**: grep sweep scope — Resolved (now `server/src server/tests`).
- ✅ **Compatibility**: No sidebar active-state verification — Resolved (manual checklist).

#### Partially Resolved
- 🟡 **Usability**: Error message leaks raw URL — fixed in cluster detail; **STILL PRESENT** in `LifecycleIndex` error branch (see new majors).
- 🟡 **Standards**: `formatMtime` duplication — extracted to shared module, but the duplicate inside `LibraryTypeView.tsx` not migrated; two implementations now coexist with subtly different ladders (`toLocaleString()` vs `Nd ago`/`Nw ago`).

#### Deferred (acknowledged)
- 🔵 Sort pills as `aria-pressed` vs `radiogroup` — accepted tradeoff.
- 🔵 Slug as visual noise on cards — judgement call deferred.
- 🔵 `frontmatter as Record<...>` casts — code-quality re-flagged the deferral.
- 🔵 Pipeline ordering duplicated client/server — suggestion-only, deferred.
- 🔵 SSE event storms unthrottled — acceptable at v1 scale.
- 🔵 Sidebar `<nav>` aria-label — out of scope.

### New Issues Introduced

#### Major

- 🟡 **Correctness**: `lifecycleClusterRoute` declared `const`, not exported, but the new shell imports it
  **Location**: Step 10b: `frontend/src/router.ts`
  The Step 9 shell calls `lifecycleClusterRoute.useParams()` after importing it from `'../../router'`. Step 10b shows `const lifecycleClusterRoute = createRoute(...)` without `export`. The build will fail at the import edge, regressing the prior fixes that depend on the component split (slug? prop, perpetual-loading branch). Fix is one keyword: `export const`.

- 🟡 **Usability**: Index-view error still leaks raw URL/status via `error.message`
  **Location**: Step 8: `LifecycleIndex.tsx` error branch
  The fix for #11 changed only the cluster-detail view. The index view still renders `Failed to load lifecycle clusters: {error instanceof Error ? error.message : String(error)}`, and `FetchError`'s message is `'GET /api/lifecycle: 500'`. So the index page exposes the very URL the detail view was rewritten to hide. Fix: branch on `error instanceof FetchError` and render generic copy.

#### Minor

- 🔵 **Architecture/Code Quality**: `WORKFLOW_PIPELINE_STEPS` re-derived locally in `LifecycleClusterView.tsx` instead of imported. Two `.filter()` sites for one partition. Suggest exporting `LONG_TAIL_PIPELINE_STEPS` from `types.ts` too and importing both.
- 🔵 **Code Quality**: `LifecycleClusterView.test.tsx` `entry()` helper still constructs `IndexEntry` literally instead of delegating to `makeIndexEntry` — the prior fix's intent (one place to default new fields) is partially leaked.
- 🔵 **Code Quality**: Test count inconsistency — verification block says 12 LifecycleClusterView tests; implementation-sequence step 25 says 11. (Counting the file: 12.)
- 🔵 **Code Quality**: `if (isError || !cluster)` collapses two distinct states; `cluster === undefined` without `isError` would render the error UI without an `error` to branch on.
- 🔵 **Test Coverage**: `bodyPreview` integration test asserts only `entry.get("bodyPreview").is_some()` — accepts `null` or any JSON value. Should be `as_str()` and assert the empty-string contract for the seeded heading-only body.
- 🔵 **Test Coverage**: Router `/lifecycle/foo` test does not assert the slug round-trip (e.g., `expect(fetchLifecycleCluster).toHaveBeenCalledWith('foo')`). A regression in `useParams` extraction by the shell would not be caught.
- 🔵 **Test Coverage**: Body-preview omit assertion uses text uniqueness, not structural omit — would still pass against a regression that always renders an empty `<p>`.
- 🔵 **Correctness**: `formatMtime` produces `"-Ns ago"` when `ms > now` (clock skew or fresh-mtime ahead of browser clock). Add a `< 0` guard returning `"just now"`.
- 🔵 **Standards**: `<section aria-label="Other artefacts">` uses UK spelling; the spec uses US `artifacts`. Pick one.
- 🔵 **Standards**: `formatMtime` extraction left the original duplicate in `LibraryTypeView.tsx` — two implementations diverge on the >24h branch.
- 🔵 **Standards**: `test_support` Rust module name vs. existing `tests/common/` and the more idiomatic `test_helpers` — flag the choice or rename for symmetry.
- 🔵 **Standards**: `frontend/src/api/test-fixtures.ts` placement vs the existing `components/Sidebar/test-helpers.tsx` pattern — consider `test-helpers.ts` for naming consistency.
- 🔵 **Standards**: `FetchError` adopted only by new helpers — adds two error idioms in one module without a written rule.
- 🔵 **Usability**: No-match filter "No clusters match …" is not announced to screen readers. Add `role="status"` to the `<p>`.
- 🔵 **Usability**: Long-tail "Other" `<h3>` is vague; `aria-label="Other artefacts"` carries more info than the visible heading. Use `aria-labelledby` to point at a heading that carries the same copy.
- 🔵 **Compatibility**: `WORKFLOW_PIPELINE_STEPS = LIFECYCLE_PIPELINE_STEPS.filter(...)` widens to a non-tuple `Array<...>`, losing literal-position typing. Add a `satisfies` clause or comment to flag the tradeoff.

#### Suggestions
- 🔵 Migrate the five existing `fetch.ts` helpers to `FetchError` (or add a comment establishing the rule).
- 🔵 Capture pipeline-ordering client/server coupling as a deferred ADR.
- 🔵 Document a measurable trigger for revisiting the universal `bodyPreview` decision (e.g. `/api/docs > 500KB`).
- 🔵 Add `expect(fetchLifecycleCluster).toHaveBeenCalledWith('foo')` to the router shell test.
- 🔵 Per-dot `aria-label` reads as `key: value` (`"Plan: present"`) — consider natural-English alternatives.

### Assessment

The plan now reflects substantial improvement: every prior major and most minors are addressed cleanly. **One small additional pass is needed** to land the two new majors:

1. `export const lifecycleClusterRoute` in Step 10b (one-keyword change).
2. Apply the `FetchError`-branching pattern to `LifecycleIndex`'s error branch so the URL-leak fix is consistent across both views.

The remaining new minors are polish items that can either be applied opportunistically in this same revise pass (artefacts → artifacts; LibraryTypeView formatMtime migration; tightening the integration test's `bodyPreview` assertion to `as_str()`; adding `role="status"` to the no-match copy; tightening the router-shell slug round-trip test) or deferred. After the two majors land, the plan is ready to implement.
