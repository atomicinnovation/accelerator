---
type: "plan-review"
id: "2026-11-24-PP-996-library-links-by-document-id-review-1"
title: "Plan Review: Library Links by Document ID Implementation Plan"
date: "2026-11-24T09:49:18+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-24-PP-996-library-links-by-document-id"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "standards"]
review_number: 1
review_pass: 3
tags: ["visualiser", "library-links"]
last_updated: "2026-11-24T09:56:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Library Links by Document ID Implementation Plan

**Verdict:** COMMENT

The plan puts library addressing in one type-agnostic module with a
builder, a string form and a three-pass resolver. Its server change adds
fields only, and its four phases are ordered so that no intermediate state
emits dead links. Four lenses converged on one weakness, the Phase 4
boundary test: as written, its patterns match existing code (Sidebar's
type path and the test router helper), and nothing ever shows it going
red. The remaining findings are local: how the plan names its terms,
accessibility and styling for unlinked rows, typing of the activity
lookup, and a few gaps in test cases.

Plan is acceptable but could be improved — see major findings below.

### Cross-Cutting Themes

- **Boundary test is imprecise** (flagged by: architecture, test-coverage,
  correctness, code-quality) — the template-literal pattern matches
  `Sidebar.tsx`'s type path, the "non-test" scope takes in `src/test/**`,
  the comparison patterns are easy to get around, and the test is never
  seen to fail.
- **Display id reached through link params** (flagged by: architecture,
  code-quality) — the lifecycle display reads
  `libraryLink(entry).params.documentId`; a named `documentIdOf` is wanted.
- **Work-item `raw_id` refactor changes empty-id behaviour** (flagged by:
  code-quality, architecture, test-coverage).
- **Id-first precedence can shadow old slug/stem URLs** (flagged by:
  architecture, compatibility) — the Migration Notes promise more than the
  resolver guarantees.
- **`missingSlug` vocabulary** (flagged by: code-quality, standards).

### Tradeoff Analysis

- **Boundary strictness vs. false positives**: test-coverage wants stronger
  checks for segment matching, while code-quality wants the brittle
  comparison patterns dropped. Resolution: keep narrowed textual patterns,
  name them in the plan as an approximation, and leave the behaviour to the
  resolver's unit tests.

### Findings

#### Major

- 🟡 **Architecture / Test Coverage / Correctness**: Boundary test patterns
  match the Sidebar type path and the test router helper
  **Location**: Phase 4, Section 5: Boundary test
  `` `/library/${dt.id}` `` matches the literal-prefix rule, and
  `src/test/router-helpers.tsx` is not a `*.test.*` file.
- 🟡 **Test Coverage**: Boundary test is never seen to fail, and its
  resolver patterns are easy to get around
  **Location**: Phase 4, Section 5: Boundary test
  The test is written after the code is clean, and an empty glob or a typo
  would leave it silently green.

#### Minor

- 🔵 **Code Quality / Architecture**: Display id is read out of a router
  link object
  **Location**: Phase 3, Section 3: Lifecycle
- 🔵 **Code Quality**: Two surfaces show different identifier concepts
  **Location**: Phase 3, Section 3
- 🔵 **Code Quality**: Unlinked-row styling is left open, though the CSS
  already settles it
  **Location**: Phase 4, Section 3
- 🔵 **Code Quality / Correctness**: The activity lookup's `indexOf` over
  parallel arrays does not type-check
  **Location**: Phase 4, Section 2
- 🔵 **Code Quality / Architecture / Test Coverage**: The `raw_id` refactor
  changes empty-id behaviour without a test
  **Location**: Phase 1, Section 2
- 🔵 **Code Quality**: Text-pattern boundary rules are brittle
  **Location**: Phase 4, Section 5
- 🔵 **Standards**: A bare `id` field sits next to `workItemId`
  **Location**: Desired End State / Phase 1
- 🔵 **Standards / Code Quality**: The `missingSlug` prop keeps the old
  vocabulary
  **Location**: Phase 2, Section 4
- 🔵 **Standards**: The unlinked row cannot carry an `aria-label` on a
  plain `<div>`
  **Location**: Phase 4, Section 3
- 🔵 **Standards**: The manual step names a `mise run dev:up` task that
  does not exist
  **Location**: Manual Testing Steps
- 🔵 **Architecture / Compatibility**: Id-first resolution can change what
  an old slug or stem URL opens
  **Location**: Phase 2 resolver / Migration Notes
- 🔵 **Compatibility**: Search rows are not normalised for the new fields;
  `SearchResult` lives in `fetch.ts`
  **Location**: Phase 2, Section 1
- 🔵 **Test Coverage**: No test pins slug-before-stem precedence against
  path order
  **Location**: Phase 2, Section 2
- 🔵 **Test Coverage**: Shared slugs are tested at only some call sites
  **Location**: Phase 3, Section 3
- 🔵 **Test Coverage**: No wiki-link case for an ADR that has an `id`
  **Location**: Phase 3, Section 3
- 🔵 **Test Coverage**: New on-disk fixture edits are not covered by the
  global snapshot
  **Location**: Phase 4, Section 6
- 🔵 **Correctness**: The fixture validation criterion cannot be met by
  minimal fixtures
  **Location**: Phase 3, Success Criteria

#### Suggestions

- 🔵 **Standards**: The boundary-test file suffix is new to `api/`
- 🔵 **Architecture**: `relPath` is added to search rows only for the stem
  fallback
- 🔵 **Compatibility**: `interpolatePath` is a low-level router utility
  under a `^1` range
- 🔵 **Test Coverage**: Sidebar tests will trigger unmocked `fetchDocs`
- 🔵 **Correctness**: The builder-to-resolver round trip is not guaranteed
  for stem links

### Strengths

- ✅ One cohesive, type-agnostic address module owns both directions of the
  mapping (entry to URL, URL to entry), and a test enforces its
  type-agnosticism.
- ✅ The resolver's three explicit passes make the precedence readable, and
  the id-over-earlier-slug test guards the original bug.
- ✅ The server adds fields only, normalising them the established way, and
  no client parses frontmatter.
- ✅ `interpolatePath` keeps the module pure and free of import cycles.
- ✅ The activity lookup reuses the shared `docs(type)` cache and the SSE
  invalidation already in place.
- ✅ Each phase can be merged on its own, in a safe order, and every phase
  opens test-first.
- ✅ Stale comments are removed, in line with the comment policy.

### Recommended Changes

1. **Specify the boundary test precisely and make it fail first**
   (addresses: both majors, brittle patterns). Extract the rules into a
   function tested on synthetic sources; use a two-interpolation pattern;
   exclude `src/test/**` and `test-fixtures.ts`; assert that the glob found
   modules; write the test at the start of Phase 4.
2. **Add `documentIdOf`** (addresses: display id, two id concepts).
3. **Keep the work-item `raw_id` reading the raw value** (addresses: the
   refactor behaviour change).
4. **Decide the unlinked-row styling and accessible name** (addresses:
   styling, `aria-label`).
5. **Use a `Map` for the activity lookup** (addresses: `indexOf` typing).
6. **Add the missing test cases** (slug-before-stem, shared slugs at search,
   not-found and lifecycle, ADR wiki link, round trip, `libraryHref`/`<Link>`
   parity, Sidebar mock).
7. **Extend the e2e snapshot; narrow the fixture criterion; fix `dev:up`;
   reword the Migration Notes; rename `missingSlug`; normalise search
   rows.**

## Per-Lens Results

### Architecture

**Summary**: A cohesive address module with the server supplying identity
once and a sound phase order. The main weakness is that the boundary
test's patterns contradict "allow-list nothing else".

**Findings**: major (boundary test patterns, high); minor (display id via
link params); suggestion (search `relPath` splits the identity rule);
minor (shared namespace precedence); minor (`raw_id` helper coupling, low).

### Code Quality

**Summary**: A clear maintainability improvement. The concerns are local:
display id via link params, index-coupled lookup, the `raw_id` refactor,
the open styling decision, and brittle text patterns.

**Findings**: minor x6 (display id; two id concepts; unlinked styling;
parallel arrays; `raw_id`; brittle patterns); suggestion (`missingSlug`).

### Test Coverage

**Summary**: Test-first throughout, with good balance across the pyramid.
The boundary test is the main gap; there are smaller case gaps.

**Findings**: major (boundary test fails on existing code); major (never
seen to fail, evadable); minor x5 (slug-before-stem; shared slugs; ADR
wiki link; e2e snapshot; `raw_id`); suggestion (Sidebar `fetchDocs`).

### Correctness

**Summary**: The core logic holds. The defects are in the boundary test's
scope, a type error in the lookup, and an unattainable validation
criterion.

**Findings**: major (template literal matches Sidebar); minor (`src/test`
exclusion); minor (`indexOf` type); minor (fixture validation); suggestion
(round trip).

### Compatibility

**Summary**: Low risk and additive. The URL shape is kept and old URLs
resolve.

**Findings**: minor (id-first shadowing); minor (search-row
normalisation); suggestion (`interpolatePath` drift).

### Standards

**Summary**: Fits the conventions well. There are small gaps in
vocabulary, accessibility and task naming.

**Findings**: minor (bare `id` naming); minor (`missingSlug`); minor
(`aria-label` on `div`); minor (`dev:up`); suggestion (file suffix).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-24

**Verdict:** COMMENT

### Previously Identified Issues
- 🟡 **Architecture / Test Coverage / Correctness**: Boundary test patterns match Sidebar and the test router helper — Resolved
- 🟡 **Test Coverage**: Boundary test never seen to fail — Resolved
- 🔵 **Code Quality / Architecture**: Display id via link params — Resolved (`documentIdOf`)
- 🔵 **Code Quality**: Two identifier concepts — Resolved
- 🔵 **Code Quality**: Unlinked-row styling left open — Partially resolved (glyph rule missed; fixed after this pass)
- 🔵 **Code Quality / Correctness**: `indexOf` lookup — Resolved (Map)
- 🔵 **Code Quality / Architecture / Test Coverage**: `raw_id` refactor — Resolved (not folded)
- 🔵 **Standards / Code Quality**: `missingSlug` — Resolved
- 🔵 **Standards**: `aria-label` on div — Resolved
- 🔵 **Standards**: `dev:up` — Resolved
- 🔵 **Architecture / Compatibility**: Id-first shadowing — Resolved
- 🔵 **Compatibility**: Search-row normalisation — Partially resolved (relPath; resolved after this pass by stating same-build)
- 🔵 **Test Coverage**: slug-before-stem, shared slugs, ADR wiki link, e2e snapshot, Sidebar mock — Resolved
- 🔵 **Correctness**: Fixture validation criterion — Resolved

### New Issues Introduced
- 🟡 **Architecture / Code Quality / Test Coverage / Correctness / Standards**: The boundary rules module sits inside its own production scan and flags itself. Fixed by moving it to `src/api/testing/` and excluding `**/testing/**`
- 🟡 **Correctness**: `LibraryTypeView` still reads `params.fileSlug` behind a cast (pre-existing gap). Fixed by adding it to the Phase 2 rename and widening the `rg` check to `fileSlug`
- 🔵 **Code Quality / Standards**: `.unlinkedRow` misses the `.row [data-doc-type]` glyph rule. Fixed: `.row` on both, hover rules on `.linkedRow`
- 🔵 **Standards**: Accessible-name assertion is impossible for a generic div. Fixed: assert the list item's text and the absence of a link
- 🔵 **Compatibility**: Search `relPath` not tolerant of an older server. Fixed: same-build stated, normalisation dropped
- 🔵 **Compatibility**: Migration Notes omit slug-over-stem shadowing. Fixed
- 🔵 **Correctness**: `SlugCandidate.id` never filled. Fixed: `id: e.id`, test driven through `fetchDocs`
- 🔵 **Test Coverage**: Live-row test should use the real SSE path. Fixed

### Assessment
Two majors remain (under the threshold of 3), and both were fixed after this pass. The plan is in good shape.

## Re-Review (Pass 3) — 2026-11-24

**Verdict:** COMMENT

### Previously Identified Issues
- 🟡 **Architecture / Code Quality / Test Coverage / Correctness / Standards**: Boundary rules module flags itself — Resolved
- 🟡 **Correctness**: `LibraryTypeView` `params.fileSlug` — Resolved (rename covered; test pinning made explicit after this pass)
- 🔵 **Code Quality / Standards**: Unlinked-row glyph rule — Resolved
- 🔵 **Standards**: Impossible accessible-name assertion — Resolved
- 🔵 **Compatibility**: Search `relPath` tolerance — Resolved
- 🔵 **Compatibility**: Slug-over-stem shadowing — Resolved
- 🔵 **Correctness**: `SlugCandidate.id` — Resolved
- 🔵 **Test Coverage**: Live-row SSE path — Resolved

### New Issues Introduced
- 🟡 **Correctness**: The widened `rg fileSlug` check collides with the rules module, which must contain `fileSlugFromRelPath`, and with two leftover comments. Fixed: the check excludes `**/testing/**`, and the comments are dropped
- 🔵 **Correctness**: The `cand()` test builder needs `id: null`. Fixed
- 🔵 **Test Coverage**: No unit test pins the `<Outlet />` gate. Fixed: the child-document test is extended
- 🔵 **Standards**: The production scan belongs beside the code, not in `testing/`. Fixed: the scan is split into `src/api/library-addressing-boundary.test.ts`
- 🔵 **Compatibility**: Inconsistent normalisation rationale. Fixed: the plan now says `IndexEntry` normalisation is for consistency
- 🔵 **Architecture**: The `**/testing/**` exclusion is broad. Accepted as the repo convention

### Assessment
One major issue (under the threshold), fixed after this pass. Every lens now reports only small, local items, and code quality reported none. The plan is ready for implementation.
