---
type: plan-review
id: "2026-06-12-0087-error-screen-affordances-review-1"
title: "Plan Review: 404 / Error Screen with Affordances"
date: "2026-06-12T22:13:17+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
parent: "plan:2026-06-12-0087-error-screen-affordances"
target: "plan:2026-06-12-0087-error-screen-affordances"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, usability, performance, standards]
review_number: 1
review_pass: 2
tags: [design, frontend, error-states, routing, search, suggestions]
last_updated: "2026-06-12T23:07:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: 404 / Error Screen with Affordances

**Verdict:** REVISE

This is a well-researched, well-decomposed plan: it isolates the ranking
logic as a pure function, separates fetch aggregation into a thin hook,
splits the conflated three-branch `Document not found` rendering into two
purpose-built surfaces, and sequences the work into independently-mergeable
phases with TDD ordering. Every lens credited its grounding in verified
codebase facts. It needs revision, however, before implementation: two
**guaranteed-failure mechanical defects** (JSX in the `.ts` `router.ts`, and
an ESLint-style lint suppression in a Biome-only project) would break the
plan's own success criteria, the Phase 4 catch-all is **effectively untested**
because the test fixture builds a router without the new `notFoundComponent`,
and the asynchronous suggestion experience (cold-cache pop-in, mid-flight
re-ranking, missing loading/live-region feedback) is underspecified for what
is fundamentally a UX recovery surface.

### Cross-Cutting Themes

- **JSX in `router.ts` will not compile** (flagged by: correctness,
  code-quality, standards) — three lenses independently confirmed that
  `router.ts` is a `.ts` file containing zero JSX, so the Phase 4 snippet
  `defaultNotFoundComponent: () => <NotFoundSurface />` fails `tsc`/build. This
  breaks Phase 4's own `types:frontend:check` and `frontend:check` criteria.
- **Phase 4 catch-all ships untested** (flagged by: architecture,
  test-coverage) — the `defaultNotFoundComponent` lives on the exported
  `router` instance, but the `buildRouter()` fixture constructs its own
  `createRouter({ routeTree, history })` without it, so `buildRouter("/garbage")`
  would render the framework default, not `NotFoundSurface`. The Phase 4 test
  cannot pass as written.
- **Asynchronous suggestion UX is underspecified** (flagged by: usability,
  performance) — the 12-query `useQueries` fan-out resolves piecemeal on a cold
  cache, so the `Did you mean…` list can pop in and visibly re-rank, with no
  loading feedback and no screen-reader live region. Both lenses converge on
  gating the rendered block until the fan-out settles.
- **Catch-all hero duplicates `BigGlyph` internals** (flagged by: architecture,
  code-quality) — the resolved decision to render `DefaultBigGlyph` directly
  inside a hand-rolled `<svg viewBox="0 0 80 80">` shell re-implements
  `BigGlyph`'s render contract in a second place, prone to silent visual drift.
- **`not-found/` module placement and naming** (flagged by: architecture,
  standards) — `LoadErrorSurface` (an error state, explicitly *not* a
  not-found state) lives under `not-found/`, and the router-global surface is
  nested under a library-scoped route folder.
- **Min-length gate / normalisation duplicated** (flagged by: code-quality,
  correctness) — the `>= MIN_SUGGESTION_LEN` rule is expressed three times with
  slightly different normalisation (`trim` vs `trim().toLowerCase()`).

### Tradeoff Analysis

- **Type honesty vs single render authority (the catch-all hero)**: the plan
  rejected an optional `docType?` on `BigGlyph` to "keep types honest" and
  instead renders `DefaultBigGlyph` directly. Architecture and code-quality
  both prefer extending `BigGlyph` (it already falls back to `DefaultBigGlyph`
  at `DEFAULT_BIG_HUE` internally) so the SVG shell stays owned in one place.
  Recommendation: make `docType` optional with a documented default — this is
  more type-honest than casting *and* avoids the duplicated shell.
- **Suggestion responsiveness vs stability**: usability wants the block gated
  until all enabled queries settle (no re-rank, with a loading hint);
  performance independently notes that gating collapses the up-to-12 redundant
  `useMemo` recomputations into one. These align — gating until settled serves
  both, at the cost of suggestions appearing slightly later. Recommended.

### Findings

#### Critical

- (none at critical severity — but see the two "build-breaker" majors below,
  which will fail the plan's stated success criteria)

#### Major

- 🟡 **Correctness / Code-Quality / Standards**: JSX in `router.ts` will not compile
  **Location**: Phase 4 §1 — createRouter configuration
  `router.ts` is a `.ts` module with no JSX today (routes are wired by
  identifier, e.g. `component: LibraryTypeView`). The snippet
  `defaultNotFoundComponent: () => <NotFoundSurface />` is a hard compile error.
  Fix by extracting a named wrapper in a `.tsx` file and passing it by
  reference, or renaming `router.ts` → `router.tsx` (note the import/fixture
  ripple).

- 🟡 **Standards**: ESLint-style suppression in a Biome-only project
  **Location**: Phase 1 §2 — aggregation hook, line 301
  The hook uses `// eslint-disable-next-line react-hooks/exhaustive-deps`, but
  the frontend lints with Biome. The canonical form is
  `// biome-ignore lint/correctness/useExhaustiveDependencies: …` (see
  `KanbanBoard.tsx:132`). Biome ignores the ESLint comment, the warning fires,
  and `lint:frontend:check` (warnings-as-errors) fails the `frontend:check` gate.

- 🟡 **Architecture / Test-Coverage**: Phase 4 test fixture omits the `notFoundComponent`
  **Location**: Phase 4 §Tests; Phase 4 §1
  `buildRouter()` (`router-fixtures.ts`) builds its own `createRouter` from the
  shared `routeTree` without `defaultNotFoundComponent`, which lives only on the
  exported `router`. `buildRouter("/garbage")` therefore renders the framework
  default, so the Phase 4 success criterion cannot pass. Move the not-found
  config into a shared router-options factory consumed by both `router.ts` and
  `buildRouter`, or inject it into the fixture.

- 🟡 **Test-Coverage**: `renderWithRouterAt` + `QueryClient` do not compose as specified
  **Location**: Phase 2 §Tests (NotFoundSurface header)
  `renderWithRouterAt` provides only a `RouterProvider` (no `QueryClientProvider`),
  but `NotFoundSurface` calls a `useQueries` hook that throws without a client in
  context. Specify the concrete wrapper (e.g. a `QueryClientProvider` whose
  children pass through `renderWithRouterAt`), ideally as a shared
  `renderWithRouterAndQueryAt` helper, mirroring the existing `LibraryDocView`
  test setup.

- 🟡 **Usability / Performance**: Cold-cache suggestions pop in and re-rank mid-flight
  **Location**: Phase 1 §2; Performance Considerations
  The `useMemo` re-runs the full aggregate-and-rank as each of the 12 queries
  resolves, so a fast lower-ranked match can render above a slower higher-ranked
  one and the list shuffles under the cursor — and performance-wise this is up
  to 12 redundant O(total entries) recomputations. Gate the rendered block on
  the enabled fan-out having settled so the list appears once, in final order.

- 🟡 **Usability**: No loading feedback for suggestions during the cold-cache window
  **Location**: Phase 2 §1 — `Did you mean…` block
  The sibling `SearchResultsPanel` shows an explicit loading bar during its
  fetch; here the user sees a bare 404 then silent materialisation, unable to
  distinguish "still loading" from "no matches". Add a lightweight working hint
  while suggestion queries are pending.

- 🟡 **Usability**: Async suggestion block lacks a screen-reader live region
  **Location**: Phase 2 §1 — suggestion-list block
  Both precedents (`EmptyState` `role="status"`, `SearchResultsPanel`
  `role="status" aria-live="polite"`) announce dynamic state. Suggestions
  resolve *after* the H1 is announced, so without a polite live region a
  screen-reader user never learns the recovery links appeared.

- 🟡 **Code-Quality**: `...results.map(r => r.data)` spread-into-deps is fragile
  **Location**: Phase 1 §2 — aggregation hook
  The variable-length spread plus a silenced exhaustive-deps rule loses static
  protection for future edits and encodes intent implicitly. Derive a single
  stable dependency (or at minimum expand the suppression comment to justify the
  granularity). (Correctness confirmed the array is *currently* correct; this is
  a maintainability concern.)

- 🟡 **Code-Quality**: Shared chrome risks duplication across the two surfaces
  **Location**: Phase 2 §1-§2
  The plan states `LoadErrorSurface` has "identical chrome" to `NotFoundSurface`
  but defines them as two independent components, so the eyebrow/hero/back-link
  logic (incl. the catch-all fallback) is liable to be implemented twice.
  Extract a shared `RecoverySurface` shell that both compose.

#### Minor

- 🔵 **Code-Quality / Architecture**: Catch-all hero re-implements `BigGlyph`'s SVG shell
  **Location**: Phase 2 §1 — Hero fallback decision
  Rendering `DefaultBigGlyph` in a hand-rolled `<svg viewBox="0 0 80 80">` with
  `bigPalette(215)` duplicates `BigGlyph`'s contract; prefer an optional
  `docType?` on `BigGlyph` so the shell stays owned in one place.

- 🔵 **Architecture / Standards**: `not-found/` module placement and naming
  **Location**: Phase 1-2 — new `routes/library/not-found/`
  `LoadErrorSurface` is not a not-found state yet lives under `not-found/`, the
  shared CSS is `NotFoundSurface.module.css`, and the router-global surface is
  nested under a library-scoped route folder. Consider a neutral container name
  and/or `src/components/` placement for the shared surfaces.

- 🔵 **Standards**: `.ac-topbar__btn` ghost back-link has no real class in the app
  **Location**: Phase 2 §1 Affordances / §3 Styles
  `.ac-topbar__btn` is a prototype literal; the shipped idiom is the
  `HeaderActionButton.module.css` `.btn` class. Name the concrete source so the
  implementer reuses it rather than authoring a divergent class.

- 🔵 **Test-Coverage**: Null-slug candidate path is untested
  **Location**: Phase 1 §Tests; Phase 3 §Tests
  `IndexEntry.slug` is nullable and both matching and the suggestion href rely on
  the `slug ?? fileSlugFromRelPath(relPath)` fallback, but every planned test
  uses slug-present entries. Add a `slug: null` case asserting it is ranked and
  its link href uses the relPath stem.

- 🔵 **Test-Coverage**: Partial-resolution / mixed-failure aggregation untested
  **Location**: Phase 1 §Tests — hook
  No test covers some queries resolved while others are pending or rejected (the
  realistic cold-cache path). A bug in the `r.data ?? []` guard or the memo deps
  could crash or fail to update with no failing test. Add a mixed-resolution
  hook test.

- 🔵 **Test-Coverage**: Catch-all-in-chrome contract only manually verified
  **Location**: Phase 4 §1 / §Tests
  Whether `defaultNotFoundComponent` renders inside `RootLayout` is deferred to
  manual checking. Add a `buildRouter("/garbage")` assertion that a chrome
  landmark is present alongside the `Page not found` H1.

- 🔵 **Test-Coverage**: Copy-voice and `error.message` assertions under-specified
  **Location**: Phase 2 §Tests
  "Sentence-case + terminal period" has no defined assertion shape (risking
  trivial or absent coverage), and `LoadErrorSurface`'s optional `error.message`
  line (with `error: unknown`) has no test for non-Error values — an unguarded
  `error.message` on a string/null would throw. Pin both.

- 🔵 **Correctness**: Exact-slug exclusion can hide a same-slug doc under another type
  **Location**: Phase 1 — `bucket()`
  Candidates are pooled across all 12 types, but any candidate whose slug equals
  the missing slug is dropped — including a genuinely-reachable same-slug doc
  under a *different* type. Consider excluding the exact match only when its
  `type` equals the 404's `knownType`.

- 🔵 **Correctness / Code-Quality**: Min-length gate normalises inconsistently
  **Location**: Phase 1 §2 (hook `enabled`) vs `rankSlugSuggestions`
  `enabled` measures `trim().length`; the pure function measures
  `trim().toLowerCase().length`. Equal for ASCII slugs (the real domain) but can
  diverge for some Unicode. Extract one `isSuggestible()`/`normaliseSlug` helper
  used by both.

- 🔵 **Performance**: `useQueries` fetches full heavy `IndexEntry` payloads for 4 fields
  **Location**: Phase 1 §2
  `fetchDocs` returns full entries (frontmatter, bodyPreview, workItemRefs, …)
  while the engine reads only `type/slug/title/mtimeMs/relPath`. Acceptable
  (only endpoint available, cache-shared, `staleTime: Infinity`), but note the
  cost is bounded by corpus metadata size and flag a future slim endpoint if
  repos grow large.

- 🔵 **Code-Quality**: `error: unknown` propagates the ad-hoc message-unwrap idiom
  **Location**: Phase 2 §2 — `LoadErrorSurface`
  Carrying `unknown` into a presentational component repeats the inline
  `instanceof Error ? … : String(err)` pattern. Pass a resolved `string` (or
  extract an `errorMessage(e: unknown)` helper) to keep the surface presentational.

- 🔵 **Architecture**: 12-type fan-out has no partial-failure degradation model
  **Location**: Phase 1 §2
  `r.data ?? []` silently drops failed types, so a partial 5xx yields a
  confidently-ranked but silently-incomplete list. Confirm and document this as
  the intended degradation contract.

#### Suggestions

- 🔵 **Correctness**: `localeCompare` relPath tiebreak diverges from server byte order only on exact ties — acceptable; no change required (note for strict parity only).
- 🔵 **Correctness**: Add the Phase 3 test asserting a matched-entry-with-rejected-content renders `LoadErrorSurface` (pins the branch mutual-exclusion invariant) — already listed; keep it.
- 🔵 **Code-Quality**: Hook returns `[]` indistinguishably for "loading" and "no matches"; the wiki-link precedent tracks `isPending`. Consider returning `{ suggestions, isPending }` or documenting the conflation (ties into the loading-feedback major).
- 🔵 **Performance**: Document the one-time-burst guarantee — `staleTime: Infinity` + SSE invalidation means the 12-request burst is one-time per type until the next `doc-changed` event or `gcTime` (~5 min) eviction.
- 🔵 **Test-Coverage**: Add a ranking unit case with a whitespace-padded missing slug (e.g. `"  er"`) to pin the `trim()` normalisation.

### Strengths

- ✅ Exemplary functional-core / imperative-shell separation: a pure,
  fully-unit-testable `rankSlugSuggestions` (no I/O, no React) split from a thin
  `useQueries` aggregation hook split from presentational surfaces.
- ✅ Fixes a genuine latent defect — three semantically-distinct branches (one
  true-404, two fetch errors) sharing one misleading `Document not found`
  rendering — by splitting into `NotFoundSurface` / `LoadErrorSurface` with
  distinct headings and affordance rules.
- ✅ The worked example is verifiably correct and asserted as an exact ordered
  array `[error-screen-v2, error-screens, legacy-error-screen]`, giving strong
  mutation resistance; field names (`slug` nullable, `mtimeMs`, `relPath`) are
  confirmed against `types.ts`.
- ✅ Faithfully reuses shipped precedents: the `Page` composition pattern, the
  `EmptyState` `.ac-empty-page` hero layout and `--ac-empty-page-hue` token, the
  `SearchResultsPanel` Link `to`/`params` shapes, and the nullable-slug link
  convention.
- ✅ Phased delivery is genuinely incremental (Phases 1-2 tested-but-unwired,
  3-4 activations), each phase compiling, passing tests, and mergeable alone.
- ✅ Production caching model is favourable — `staleTime: Infinity` + SSE
  invalidation makes the cold-cache fan-out a one-time-per-type cost; warm
  entries from library views / the wiki-link resolver are reused via the shared
  `queryKeys.docs(type)`.
- ✅ Genuinely-open decisions (templates exclusion, `useQueries` vs fixed hooks,
  the null-rendering `Glyph` catch-all gotcha) are explicitly resolved with
  rationale rather than left ambiguous.

### Recommended Changes

1. **Make the router catch-all compile and be testable** (addresses: "JSX in
   `router.ts`", "Phase 4 fixture omits notFoundComponent"). Extract a named
   `NotFoundRoute` wrapper in a `.tsx` file and reference it by identifier; move
   the not-found config into a shared router-options factory consumed by both
   `router.ts` and `buildRouter` so the Phase 4 test exercises the real surface.

2. **Fix the lint suppression for Biome** (addresses: "ESLint-style suppression").
   Replace with `// biome-ignore lint/correctness/useExhaustiveDependencies: …`
   and confirm Biome accepts the spread; add `mise run frontend:check` to
   Phase 1's success criteria (the hook with the suppression lands there).

3. **Specify the Phase 2 test wrapper concretely** (addresses: "renderWithRouterAt
   + QueryClient do not compose"). Define a `QueryClientProvider` +
   `renderWithRouterAt` composition (ideally a shared helper) so the
   suggestion-bearing surface mounts.

4. **Gate the suggestion block until the fan-out settles, with loading + live
   region** (addresses: "pop in and re-rank", "no loading feedback", "no live
   region"). Render the block once enabled queries are non-pending, show a
   working hint while pending, and wrap it in `role="status" aria-live="polite"`;
   this also collapses the redundant `useMemo` recomputations.

5. **Own the hero in one place** (addresses: "re-implements BigGlyph's SVG
   shell"). Add an optional `docType?` to `BigGlyph` (defaulting to its existing
   `DefaultBigGlyph`/`DEFAULT_BIG_HUE` fallback) instead of a hand-rolled SVG.

6. **Extract a shared surface shell** (addresses: "shared chrome duplication").
   A `RecoverySurface` owning the eyebrow/hero/back-link row, with each surface
   supplying only its H1, copy, and (NotFound only) the suggestion block.

7. **Close the test gaps** (addresses: null-slug, partial-resolution,
   catch-all-in-chrome, copy-voice, `error: unknown`). Add the enumerated cases.

8. **Tidy naming and single-source the gate** (addresses: "`not-found/`
   placement", "`.ac-topbar__btn` literal", "min-length gate duplication"). A
   neutral container/CSS name, the concrete `HeaderActionButton` `.btn` source,
   and one `isSuggestible()`/`normaliseSlug` helper.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound and well-grounded in verified
codebase facts. It correctly separates concerns (pure ranking function /
fetch-aggregation hook / presentational surfaces / wiring), follows the
established Page-composition pattern, and properly splits the conflated
404-vs-fetch-error branches into distinct surfaces — a genuine structural
correctness improvement. The main concerns are a Phase 4 testability gap (the
test fixture builds its own router without the new notFoundComponent), some
cohesion/naming friction in the shared not-found/ module, and a deliberate
coupling to BigGlyph internals and the server's classify() ranking convention
that is acknowledged but worth tracking.

**Strengths**:
- Clean functional-core / imperative-shell separation (pure ranking vs useQueries aggregation vs surfaces).
- Correctly identifies and fixes the latent structural defect of three branches sharing one rendering.
- Follows the established Page-shell composition pattern, preserving Page's presentation-only role.
- Phased delivery is genuinely incremental and independently mergeable.
- Preserves the existing true-404 gate and unknown-type redirect rather than restructuring routing.

**Findings**:
- 🟡 (major, high) **Phase 4 test fixture builds a router without the new notFoundComponent** — Phase 4 / Tests. `buildRouter` constructs a separate `createRouter({ routeTree, history })` without `defaultNotFoundComponent` (which lives on the exported `router`), so `/garbage` renders the framework default in the test and the success criterion cannot pass. Move the not-found config into a shared factory or inject into the fixture.
- 🔵 (minor, high) **Catch-all hero couples the surface to BigGlyph's internal palette/viewBox** — Phase 2 hero fallback. Rendering `DefaultBigGlyph` in a hand-rolled `<svg>` with `bigPalette(215)` reimplements BigGlyph's contract. Prefer an optional `docType?` on BigGlyph.
- 🔵 (minor, medium) **LoadErrorSurface housed under not-found/ blurs the module boundary** — Phase 1 & 2. An error state lives under a directory named for the opposite concern; the shared CSS is `NotFoundSurface.module.css`. Consider a neutral container/stylesheet name.
- 🔵 (minor, medium) **Ranking convention duplicated client-side with no shared contract or drift guard** — Phase 1. Add a bidirectional breadcrumb comment in `search.rs` pointing to the client engine.
- 🔵 (minor, medium) **Cold-cache 12-request fan-out has no degradation or failure model** — Phase 1. `r.data ?? []` silently drops failed types; confirm and document the intended partial-failure degradation contract.

### Code Quality

**Summary**: The plan is unusually well-structured for code quality: it isolates
the ranking logic as a pure, fully-specified function, separates presentation
from data via a thin hook, and decomposes work into independently-mergeable
phases with TDD ordering. The main concerns are localized: the dependency-array
workaround is fragile and hides a known React-hooks footgun, the two surface
components risk duplicated chrome without an extracted shared shell, and the
DefaultBigGlyph-direct-render reimplements BigGlyph's SVG shell. The JSX-in-.ts
router issue is a hard compile error.

**Strengths**:
- `rankSlugSuggestions` is an exemplary pure-function design (no I/O, typed contract, guard clause, trivially testable).
- Clean layer separation, each testable with the right tools.
- The fetch-error / 404 split is a real maintainability and correctness improvement.
- Phase decomposition is genuinely incremental with sound TDD ordering.
- Genuinely-open decisions are explicitly resolved with rationale.

**Findings**:
- 🟡 (major, high) **Spread-into-deps + exhaustive-deps disable is a fragile memo dependency** — Phase 1 §2. The `[..., ...results.map(r => r.data)]` spread plus a silenced rule loses static protection; derive a single stable dependency or at minimum justify the granularity in the comment.
- 🟡 (major, medium) **Shared chrome between the two surfaces risks duplication without an extracted shell** — Phase 2 §1-2. Extract a `RecoverySurface` both compose, supplying only the distinct H1/copy/suggestion block.
- 🟡 (major, high) **JSX added to a .ts file will not compile without a rename or createElement** — Phase 4 §1. `router.ts` is `.ts`; use a named `.tsx` wrapper by reference or `createElement`, not inline JSX.
- 🔵 (minor, high) **Rendering DefaultBigGlyph directly reimplements BigGlyph's SVG shell** — Phase 2 §1. Prefer extending BigGlyph with an optional `docType`.
- 🔵 (minor, medium) **Min-length gate evaluated in three places with slightly different normalisation** — Phase 1 §2 / Phase 2 §1. Extract one `isSuggestible()`/`normaliseSlug` helper.
- 🔵 (minor, medium) **`error?: unknown` plus inline instanceof unwrapping repeats an ad-hoc idiom** — Phase 2 §2. Pass a resolved `string` or extract `errorMessage(e: unknown)`.
- 🔵 (suggestion, low) **Hook returns [] indistinguishably for "still loading" and "genuinely no matches"** — Phase 1 §2. Consider returning `{ suggestions, isPending }` or documenting the conflation.

### Test Coverage

**Summary**: The plan is test-first and unusually thorough: a pure ranking
function with eight enumerated unit cases, hook tests with spy-based fetch
assertions, component tests per surface, and integration tests at both the
LibraryDocView and router boundaries. Coverage maps cleanly to nearly every
acceptance criterion, with the worked example asserted as an exact ordered list.
The main risks are mechanical fixture mismatches, a few under-specified
assertions, and one untested branch (the null-slug / relPath-stem path).

**Strengths**:
- Phase 1 ranking unit tests enumerate every ordering rule independently — genuine mutation resistance.
- The worked example is an exact ordered-array assertion, not a loose contains check.
- Hook tests assert negative behaviour with spies (no fetch below 2 chars; never `templates`).
- The fetch-error / 404 split is tested across all three live branches plus the loading-vs-404 gate.
- Test pyramid balance is sound (pure logic at unit; hook/surface at component; thin router/view integration).

**Findings**:
- 🟡 (major, high) **`QueryClient(retry:false) + renderWithRouterAt` do not compose** — Phase 2 / Tests. `renderWithRouterAt` provides no QueryClientProvider; the useQueries hook throws. Specify the concrete wrapper or a shared `renderWithRouterAndQueryAt`.
- 🟡 (major, medium) **Null-slug entry match + suggestion link path is untested** — Phase 3 / Tests. Both match and link href rely on `slug ?? fileSlugFromRelPath(relPath)`, but tests only use slug-present entries. Add a `slug: null` case.
- 🔵 (minor, medium) **"Sentence-case + terminal period" assertion shape undefined** — Phase 2 & AC. Pin to the mono element plus a stable copy anchor; treat full wording as an intentionally-exact assertion.
- 🔵 (minor, medium) **Catch-all-inside-RootLayout only manually verified** — Phase 4. Add a `buildRouter("/garbage")` assertion that a chrome landmark is present alongside the H1.
- 🔵 (minor, high) **trim() normalisation un-mutation-tested** — Phase 1 / Tests. Add a whitespace-padded missing-slug case.
- 🔵 (minor, medium) **Partial-resolution / mixed-failure aggregation untested** — Phase 1 hook tests. Add a test where some queries resolve and others reject/pend.
- 🔵 (minor, low) **`error.message` supplementary detail untested for non-Error values** — Phase 2 LoadErrorSurface. Pass a string and `undefined`; assert no throw.

### Correctness

**Summary**: The core ranking logic is sound: rankSlugSuggestions faithfully
reproduces the slug-relevant subset of classify() (prefix-before-interior,
mtimeMs-desc then relPath-asc, exact-match exclusion), and the worked example
genuinely produces [error-screen-v2, error-screens, legacy-error-screen]. The
useQueries hook's enabled gating, useMemo dependency array, and in-flight
partial-resolution behaviour are all logically correct, and the preserved
true-404 gate is reproduced exactly. The one concrete defect is that Phase 4
places JSX in router.ts (a .ts file with no JSX today), which will not compile.

**Strengths**:
- The worked example is verifiably correct (both prefix matches outrank the interior match; sort yields the claimed order).
- Field names confirmed against types.ts (slug nullable, mtimeMs, relPath present on IndexEntry).
- The bucket() exact-match short-circuit correctly mirrors "exact slug = found doc".
- The useMemo deps correctly track per-query data identity; the early !enabled return prevents stale cached data leaking below the gate.
- Preserves the `!entry && entries.length > 0` true-404 gate exactly.

**Findings**:
- 🟡 (major, high — body marked 🔴) **JSX in router.ts will not compile** — Phase 4. `router.ts` is `.ts` with zero JSX. Rename to `.tsx` or pass a named component identifier. Fails its own `types:frontend:check`.
- 🔵 (minor, medium) **Exact-slug exclusion can hide a genuine same-slug document under a different type** — Phase 1 bucket(). Exclude the exact match only when its `type` equals the 404's `knownType`.
- 🔵 (minor, medium) **Length gate normalises differently in hook vs pure function** — Phase 1. Gate both on the same normalised value; negligible for ASCII slugs.
- 🔵 (suggestion, high) **localeCompare diverges from server byte order only on exact ties** — Phase 1. Acceptable; no change required.
- 🔵 (suggestion, medium) **Branch order keeps content-error from pre-empting the 404 correctly** — Phase 3. Confirms behaviour-preserving; keep the planned test that pins it.

### Usability

**Summary**: From an end-user recovery-experience perspective the plan is strong
on copy voice, affordance rules, and the 404-vs-load-error distinction, and it
correctly reuses shipped precedents. The main gaps are in the asynchronous
suggestion experience: the cold-cache window has no loading feedback and the 12
piecemeal-resolving queries can make the list pop in and re-rank, and the
accessibility semantics of the async block and the suggestion rows are
underspecified (live region, heading level, listbox/option misuse).

**Strengths**:
- The 404-vs-load-error split is a genuine UX correctness fix.
- Affordance rules are precise and consistent across both surfaces.
- Copy voice is well-specified and grounded in existing precedents.
- The empty-block-omission rule and graceful DefaultBigGlyph fallback avoid confusing states.
- Suggestion links reuse the canonical Link shape (middle-click/keyboard behave normally).

**Findings**:
- 🟡 (major, high) **Cold-cache suggestions pop in and visibly re-rank as 12 queries resolve** — Phase 1 §2 / Performance. Gate the block on the enabled fan-out settling so it appears once in final order.
- 🟡 (major, high) **No loading/pending feedback during the cold-cache window** — Phase 2 §1. Add a working hint consistent with `SearchResultsPanel.searchLoading`.
- 🟡 (major, medium) **Async suggestion block lacks a screen-reader live region** — Phase 2 §1. Wrap in `role="status"`/`aria-live="polite"`.
- 🔵 (minor, medium) **Copying SearchResultsPanel's listbox/option ARIA into a static page is the wrong pattern** — Phase 2 §1. Reuse the visual layout but render plain `<Link>`s, not `role="listbox"`/`role="option"`.
- 🔵 (minor, medium) **Suggestion-block heading level unspecified, risking a hierarchy skip** — Phase 2 §1. Specify `<h2>` matching EmptyState; assert in the test.
- 🔵 (minor, low) **Over-deep valid-type URLs fall to the catch-all and lose back-to-type** — Phase 4. Either document the limitation or derive `knownType` from the first `/library/` segment in the catch-all.

### Performance

**Summary**: The performance profile is fundamentally bounded and acceptable: a
low-frequency recovery screen, production `staleTime: Infinity` with SSE-driven
invalidation so the cold-cache 12-request burst is one-time per type, and an
O(total entries) ranking over in-memory data. The two real concerns are the
full heavy IndexEntry payload fetched for four fields, and the useMemo re-running
the aggregate-and-rank up to 12 times as queries resolve piecemeal — both bounded
and minor.

**Strengths**:
- `staleTime: Infinity` + SSE invalidation makes the 12-request burst one-time per type; warm caches reused with zero refetch.
- Reuses the existing `queryKeys.docs(type)` cache key, sharing with library views and the wiki-link resolver.
- Correctly gated (`enabled`) so the fan-out never fires for sub-2-char slugs.
- Ranking is O(n) with early null-bucket rejection — appropriate complexity.
- The plan documents the fan-out cost in a dedicated section.

**Findings**:
- 🔵 (minor, high) **useQueries fetches full heavy IndexEntry payloads across 12 types for 4 fields** — Phase 1 §2. Acceptable (only endpoint, cache-shared, one-time), but note the cost is bounded by corpus metadata size; flag a future slim endpoint for large repos.
- 🔵 (minor, high) **useMemo keyed on per-query data re-runs the full aggregate+rank up to 12 times on cold cache** — Phase 1 §2. Gating the memo body on all queries being non-pending collapses it to one computation.
- 🔵 (suggestion, medium) **Document the one-time-burst guarantee from staleTime:Infinity + SSE** — Performance Considerations. Note gcTime default (~5 min) eviction re-pays the burst once after that window.

### Standards

**Summary**: The plan is strongly grounded in real codebase conventions — it
mirrors the EmptyState/Page hero composition, the SearchResultsPanel Link shape,
the nullable-slug link convention, CSS-module + `--ac-*` token reuse, and the
PascalCase / `use-` / kebab-case naming. Two concrete, verifiable conformance
defects will break the stated `frontend:check` gate: the hook uses an ESLint-style
suppression in a Biome-only project, and Phase 4 puts JSX into the JSX-free
`router.ts`. A few softer points (component placement, the `.ac-topbar__btn`
literal) are worth tightening.

**Strengths**:
- File/module naming is convention-correct (PascalCase components, kebab-case modules, `use-` hook, co-located `.module.css`/`.test.tsx`).
- TanStack Link usage faithfully reuses the canonical typed-route shapes.
- CSS approach reuses the EmptyState `.ac-empty-page` precedent and `--ac-empty-page-hue`.
- Accessibility handling is idiomatic (`role="alert"`, single H1 via Page, DefaultBigGlyph to avoid the null-rendering Glyph).
- TypeScript discipline is sound (no `any`, real interface, type-only imports, no invalid DocTypeKey cast).

**Findings**:
- 🟡 (major, high) **ESLint-style suppression comment in a Biome-only project** — Phase 1 §2, line 301. Use `// biome-ignore lint/correctness/useExhaustiveDependencies: …`; otherwise `lint:frontend:check` (warnings-as-errors) fails.
- 🟡 (major, high) **JSX added to router.ts, a .ts (JSX-free) module** — Phase 4 §1. Rename to `.tsx` or extract a `.tsx` wrapper passed by reference.
- 🔵 (minor, medium) **Router-global surface placed under routes/library/** — Phase 2. Consider `src/components/` placement; or note the cross-route import is by design.
- 🔵 (minor, medium) **'ghost .ac-topbar__btn' back-links have no real class in the app** — Phase 2 §1/§3. The shipped idiom is `HeaderActionButton.module.css` `.btn`; name the concrete source.
- 🔵 (minor, high) **frontend:check does run lint (Biome), contrary to the no-lint assumption** — Overview / Phase 1. `frontend:check` runs format + lint + types; add it to Phase 1's criteria since the suppression lands there.

---

## Re-Review (Pass 2) — 2026-06-12T22:47:33+00:00

**Verdict:** COMMENT

The revision resolves every major finding from pass 1 and nearly all minors;
the remaining items are polish. The two build-breakers are fixed (`router.ts`
stays JSX-free via a `CatchAllNotFound` `.tsx` wrapper + shared `routerOptions`
factory; the hook uses the verified Biome suppression `lint/correctness/useExhaustiveDependencies`),
the Phase 4 catch-all is now testable (the fixture shares `routerOptions`), the
surface tests get a real `renderWithRouterAndQueryAt` wrapper, the suggestion
fan-out gates on a single `isPending` settle (no mid-flight re-rank, ranks once,
working hint in a `role="status"` live region, plain links, `<h2>` heading), the
chrome is shared via `RecoverySurface`, and the gate/error-message logic is
single-sourced. The "extend `BigGlyph`" decision introduced one new actionable
item (a DEV `console.warn` that misfires on the intended `undefined` docType) —
now folded into the plan. Below the REVISE threshold (1 major, no critical), so
COMMENT: acceptable, and the remaining minors are worth folding in but not
blocking.

### Previously Identified Issues

- 🟡 **Correctness/Code-Quality/Standards**: JSX in `router.ts` won't compile — **Resolved** (`CatchAllNotFound` `.tsx` wrapper referenced by identifier; `router.ts` stays JSX-free).
- 🟡 **Standards**: ESLint suppression in a Biome project — **Resolved** (now `// biome-ignore lint/correctness/useExhaustiveDependencies: …`; rule id verified against `KanbanBoard.tsx:132`).
- 🟡 **Architecture/Test-Coverage**: Phase 4 fixture omits `notFoundComponent` — **Resolved** (shared `routerOptions` consumed by both `createRouter` and `buildRouter`).
- 🟡 **Test-Coverage**: `renderWithRouterAt` + `QueryClient` don't compose — **Resolved** (shared `renderWithRouterAndQueryAt` helper; re-review confirms it composes against the stub-route fixture).
- 🟡 **Usability/Performance**: Cold-cache pop-in / mid-flight re-rank — **Resolved** (hook gates on `isPending`, ranks once when the fan-out settles; perf re-review confirms the heavy O(n) pass now runs exactly once).
- 🟡 **Usability**: No loading feedback for suggestions — **Resolved** (working hint while pending). New minor: no debounce → can flash on warm cache (see below).
- 🟡 **Usability**: Async block lacks a live region — **Resolved** (`role="status" aria-live="polite"`). New minor: region wraps the whole link list (see below).
- 🟡 **Code-Quality**: Fragile spread-into-deps suppression — **Resolved/justified** (inline rationale; correctness re-review confirms the fixed-length `PHYSICAL_KEYS` invariant makes the dep-array arity stable). Downgraded to a documentation suggestion.
- 🟡 **Code-Quality**: Shared chrome duplication — **Resolved** (`RecoverySurface` shell composed by both surfaces).
- 🔵 **Architecture/Code-Quality**: Hand-rolled SVG hero shell — **Resolved** (optional `docType` on `BigGlyph`); **but introduced** the DEV-warn misfire below.
- 🔵 **Architecture/Standards**: `not-found/` placement/naming — **Resolved** (`recovery/` folder). Residual minors on CSS-module casing and wrapper placement (see below).
- 🔵 **Standards**: `.ac-topbar__btn` literal — **Resolved** (`HeaderActionButton` `.btn`, corroborated by that file's own header comment).
- 🔵 **Code-Quality/Correctness**: Min-length gate duplicated/divergent — **Resolved** (`isSuggestible`/`normaliseMissingSlug` single source; correctness re-review confirms the hook's `enabled` and the ranker's guard now provably agree).
- 🔵 **Code-Quality**: `error: unknown` ad-hoc unwrap — **Resolved** (`errorMessage()` helper; surface takes a resolved string).
- 🔵 **Test-Coverage** (null-slug, partial-resolution, whitespace, `errorMessage` non-Error, in-chrome, copy-voice) — **Resolved** (cases added). Residual: tighten a few assertions (see below).
- 🔵 **Correctness/Performance** (cross-type exact-slug, one-time burst) — **Resolved** (documented as accepted tradeoffs).

### New Issues Introduced

- 🟡 **Architecture / Code-Quality** (major/minor): **`BigGlyph` DEV `console.warn` misfires on the new `undefined` docType path** (`BigGlyph.tsx:77`) — the optional-`docType` extension makes `undefined` a legitimate input, but the existing DEV guard warns whenever `BIG_GLYPHS[docType]` is falsy, firing a misleading "Unknown docType" warning on every catch-all/load-error render. **Folded into the plan**: narrow the guard to `docType !== undefined && !BIG_GLYPHS[docType]`.
- 🔵 **Standards** (minor, high): CSS module named `recovery-surface.module.css` breaks the universal PascalCase-matches-component convention. **Folded in**: renamed to `RecoverySurface.module.css`.
- 🔵 **Standards** (minor, high): `router-not-found.tsx` placed at `src/` root (no component lives there; filename ≠ export). **Folded in**: relocated to `routes/library/recovery/CatchAllNotFound.tsx`.
- 🔵 **Usability** (minor): working hint has no debounce → may flash on the common warm-cache path. **Applied** — the plan now defers the hint ~250ms (the `useDeferredFetchingHint` convention, keyed on the hook's initial-load `isPending`, since that hook itself gates on refetch-not-initial-load and can't be reused verbatim).
- 🔵 **Usability** (minor): the `role="status"` region wrapped the entire `<h2>`+link list. **Applied** — the live region now carries only a concise status string; the heading + links render outside it.
- 🔵 **Test-Coverage** (minor): assertions pinned harder. **Applied** — dedicated equal-bucket+equal-mtime `relPath` tiebreak case, a staggered single-pass (no-re-rank) gate assertion, fake-timer deferred-hint timing, `getByRole("main")` in-chrome check (verified `RootLayout.tsx:94` renders `<main>`), and a distinct content-error message threaded through `errorMessage()`.
- 🔵 **Architecture/Correctness** (minor): confirm up front that TanStack's `defaultNotFoundComponent` mounts inside the root `RootLayout`/`<Outlet>` so the catch-all keeps app chrome and `<Link>` context. *Open verification item* — now backed by the `getByRole("main")` automated guard; still resolve against the fixture during Phase 4.

### Assessment

The plan is in good shape and ready to implement. All new findings have been
folded directly into the plan: the `BigGlyph` DEV-warn guard, the CSS-module
rename, the wrapper relocation, the deferred-hint debounce, the scoped live
region, and the tightened test assertions. The only open item is a one-line
verification (that TanStack mounts `defaultNotFoundComponent` inside `RootLayout`),
now guarded by an automated `getByRole("main")` assertion in Phase 4. No critical
findings; the single major (the `BigGlyph` DEV-warn) is resolved in-plan, so the
plan is implementation-ready.

**Verdict set to APPROVE** by the reviewer after the pass-2 polish (deferred
hint, scoped live region, tightened tests, plus the three folded-in new findings)
was applied — no residual blocking items remain. The lone open verification (the
TanStack `defaultNotFoundComponent` mount position) is guarded by the Phase 4
`getByRole("main")` assertion and resolved during implementation.

---
*Re-review generated by /accelerator:review-plan*
