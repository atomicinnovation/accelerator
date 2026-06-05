---
date: "2026-06-06T01:00:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-06-05-0079-aside-region-redesign.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, performance, usability]
review_pass: 3
status: complete
---

## Plan Review: Detail-Page Aside Region Redesign

**Verdict:** REVISE

This is a well-researched, tightly-scoped, genuinely test-driven plan whose core
theses are sound: Option B is correctly identified as a pure render-layer change
needing no server/API/type edits, the work item's wrong data source
(`cluster-via-label.ts` debug string) is correctly replaced with a robust
path-matched cluster resolution over the shared lifecycle cache, and the three
phases are cleanly sequenced to avoid rework. The reasons to revise before
implementation are not structural flaws but a cluster of under-specified
decisions and unexamined blast radius: the new cluster query has no
loading/error handling and silently conflates "loading", "failed", and
"no cluster"; the Phase 3 `.label` change restyles a surface (lifecycle index
cards) the plan doesn't acknowledge; the stated negative case is almost
unreachable because nearly every document lands in at least a singleton cluster;
and the test plan leans on a router route and a token export that don't exist as
described.

### Cross-Cutting Themes

- **Cluster-query loading/error/null conflation** (flagged by: Test Coverage,
  Usability, Correctness) — `useDocCluster` returns `cluster: null` during
  loading, on fetch error, *and* when the doc is genuinely in no cluster.
  `LibraryDocView` renders `{cluster && …}`, so a real cluster block silently
  vanishes while `/api/lifecycle` loads on a cold cache and never appears if the
  fetch fails — with no feedback, unlike the sibling Related-artifacts section.
  This is simultaneously a missing test case, a UX degradation, and a
  correctness ambiguity. **This is the most important finding.**

- **Unresolved `font-weight` parity** (flagged by: Code Quality, Correctness,
  Standards, Usability — four lenses) — `.aside h3` declares `font-weight: 600`;
  `.eyebrow` declares none (inherits). Phase 3 leaves this as an either/or ("drop
  if it diverges, or confirm parity"), and `font-weight` is *not* among the five
  properties the equality spec asserts — so the labels can ship visibly bolder
  than the eyebrow while every automated check passes.

- **Legend removal strips the declared/inferred explanation** (flagged by:
  Usability, Standards) — removing `<Legend>` deletes the only on-screen
  definition of the two terms, leaving bare `(declared)`/`(inferred)` tags
  distinguished partly by a faint, possibly sub-contrast colour.

### Findings

#### Major

- 🟡 **Test Coverage / Usability / Correctness**: Cluster block has no
  loading/error handling and conflates three distinct states
  **Location**: Phase 2 — Changes Required #2 (`useDocCluster`) and #4 (wiring)
  `useDocCluster` derives `cluster = query.data?.find(...) ?? null`, so a
  document that genuinely belongs to a cluster shows no block while the query is
  in flight and never shows one if the fetch rejects — neither tested nor given
  user feedback, breaking parity with the Related-artifacts section's
  `Loading…`/`role="alert"` branches.

- 🟡 **Architecture**: Phase 3 `.label` change restyles every lifecycle-index
  cluster card, not just the detail rail
  **Location**: Phase 3 — Changes Required #3 (Promote the rail label)
  `Pipeline .label` is one shared rule applied to both the `card` variant
  (rendered per cluster card in `LifecycleIndex`) and the `panel` variant (the
  detail rail). The unconditional edit changes index-card stage labels too — a
  surface neither "What We're NOT Doing" nor the eyebrow-equality spec accounts
  for, so a regression there would go uncaught.

- 🟡 **Test Coverage**: Shared test router lacks a `/lifecycle/$slug` route, so
  the planned link-href assertion cannot pass
  **Location**: Phase 2 — Changes Required #5 (RelatedCluster/LibraryDocView tests)
  `src/test/router-helpers.tsx` registers only `/`, `/library/$type`, and
  `/library/$type/$fileSlug`. A TanStack `<Link to="/lifecycle/$slug">` to an
  unregistered route won't resolve to the expected href, so the only automated
  guard on the cluster block's navigation target fails or weakens silently.

- 🟡 **Correctness**: The negative case is mischaracterised — nearly every
  document is in at least a singleton cluster
  **Location**: Current State Analysis ("Cluster block data source") and Phase 2
  The server places every slug-bearing entry into at least a singleton/per-path
  bucket; only `slug == null` entries are excluded from clustering. So the
  path-match returns non-null for virtually every detail page (often a self-only
  "1 artifact" cluster), and the planned negative-case fixture ("orphan with
  `completeness == null`") can't be built from a slug-bearing doc.

- 🟡 **Standards**: Promoting the rail stage labels (navigational content text)
  to uppercase mono is a WCAG legibility/screen-reader concern
  **Location**: Phase 3 — Changes Required #3
  Unlike the decorative page eyebrow, the rail `.label` is content (the stage
  name). All-caps via `text-transform` plus wide tracking can reduce low-vision
  readability and change how some assistive tech announces the text.

- 🟡 **Usability / Standards**: Removing the legend strips the only explanation
  of declared vs inferred; the remaining distinction leans on colour
  **Location**: Phase 1 — Changes Required #2/#3
  The deleted `<Legend>` defined the terms; the replacement is terse
  `(declared)`/`(inferred)` text differentiated by accent vs `--ac-fg-faint`.
  The faint tag may fall below the 4.5:1 text-contrast minimum now that it is
  meaning-bearing text rather than a decorative badge.

#### Minor

- 🔵 **Code Quality / Correctness / Standards / Usability**: `font-weight`
  parity for `.aside h3` left as an unresolved either/or and excluded from the
  equality spec — see Cross-Cutting Themes.
  **Location**: Phase 3 — Changes Required #2

- 🔵 **Test Coverage**: The new eyebrow spec references a `tokens['ac-fg-faint']`
  export that doesn't exist; `src/styles/tokens.ts` exports per-theme
  `LIGHT_COLOR_TOKENS`/`DARK_COLOR_TOKENS` (as `detail-eyebrow-resolved-colours.spec.ts`
  uses).
  **Location**: Phase 3 — Changes Required #4

- 🔵 **Test Coverage**: The inactive rail-label locator can match zero elements
  and pass vacuously; pin to `/lifecycle/first-plan` and assert a count before
  reading computed styles.
  **Location**: Phase 3 — Changes Required #4

- 🔵 **Test Coverage**: After Option B merges the lists, the coverage-guard count
  (`PHYSICAL_DOC_TYPE_KEYS.length - 1` inferred rows) must filter on the
  `(inferred)` tag, not the whole `<ul>`, or it over-counts.
  **Location**: Phase 1 — Changes Required #4

- 🔵 **Test Coverage**: The extracted shared `pluralise` gets no direct unit test
  (only indirect via RelatedCluster); add n=0/1/>1/explicit-plural cases in
  `format.test.ts`.
  **Location**: Phase 2 — Changes Required #1

- 🔵 **Correctness**: Ensure the rewritten empty-state branch derives from the
  combined `rows.length === 0` (all three arrays empty), not just `declared`, so
  inferred-only / declared-only docs render correctly.
  **Location**: Phase 1 — Changes Required #2

- 🔵 **Architecture / Performance**: Memoise the `query.data?.find(...)`
  derivation (`useMemo([query.data, entry?.path])`) to match the existing
  `LifecycleIndex` idiom and avoid re-scanning on unrelated re-renders.
  **Location**: Phase 2 — Changes Required #2

- 🔵 **Architecture**: Consider composing `useDocCluster` into `useDocPageData`,
  whose docstring names it as the join point for read-side fanout, rather than
  wiring a second read query directly into the view.
  **Location**: Phase 2 — Changes Required #4

- 🔵 **Code Quality / Standards**: Make the class/testid renames non-optional and
  concrete (`groupList`→`list`, `badge*`→`tag*`, keep a greppable `related-`
  testid prefix) so names match the single-list reality.
  **Location**: Phase 1 — Changes Required #2/#3

- 🔵 **Code Quality**: Render the visible tag text from an explicit map rather
  than `({kind})`, decoupling display copy from the discriminant used for the
  CSS class.
  **Location**: Phase 1 — Changes Required #2

- 🔵 **Code Quality**: Token-sharing unifies values but leaves three duplicated
  five-property eyebrow rules to drift; the cross-element identity spec is the
  right mitigation — note the tradeoff.
  **Location**: Phase 3 (overall)

- 🔵 **Usability**: `pluralise` lands in `format.ts` (otherwise all date/time
  helpers); a string utility there is hard to discover. Consider a
  `text.ts`/`strings.ts` home.
  **Location**: Phase 2 — Changes Required #1

- 🔵 **Architecture**: Document the whole-list-fetch tradeoff (robust
  path-matching + cache sharing vs whole-collection retrieval) so a future scale
  change has a recorded decision point.
  **Location**: Performance Considerations / Phase 2

- 🔵 **Architecture**: `Pipeline .label` is owned by work item 0040
  (functionally complete, not transitioned); the coordination note is correct —
  consider whether label typography should be a shared token/class to reduce the
  cross-work-item ownership collision.
  **Location**: Coordination Notes

#### Suggestions

- 🔵 **Performance**: On a cold cache, a direct detail-page load fetches
  `/api/lifecycle` — the heaviest list endpoint (every cluster's full
  `entries[]` with frontmatter) — to surface one small block. Acceptable at this
  scale; note the targeted single-cluster fetch as the escape hatch if the
  corpus grows.
  **Location**: Performance Considerations / Phase 2

- 🔵 **Performance**: Once mounted, the detail page becomes an observer of
  `queryKeys.lifecycle()`, so every `doc-changed`/`doc-invalid` SSE event now
  refetches the full list while the user sits on one document. Fine for a
  single-user local tool; worth a one-line acknowledgement.
  **Location**: Phase 2 — Changes Required #4

### Strengths

- ✅ Correctly establishes Option B as a pure render-layer change — server-encoded
  declared/inferred provenance (mutually exclusive by server dedup) is preserved,
  so no API/wire/type change is needed.
- ✅ Replaces the work item's incorrect data source with a robust path-matched
  resolution that sidesteps the representative-slug ambiguity, and explains *why*.
- ✅ Reuses the existing `queryKeys.lifecycle()` cache shared with
  `LifecycleIndex`, avoiding a divergent key and a duplicate fetch path.
- ✅ Phase sequencing (1 establishes `.aside h3` → 2 adds a third such section →
  3 unifies the eyebrow in one pass) genuinely avoids rework; each phase is
  shippable.
- ✅ Extracts `pluralise` at the moment a second consumer appears — pragmatic DRY,
  not speculative.
- ✅ Every phase mandates red-first authoring with explicit "fails before, passes
  after" criteria, and correctly identifies the existing tests as asserting the
  *old* structure (to be rewritten, not silently broken).
- ✅ Negative/absence cases are called out specifically (no legend, no level-4
  headings, no block when no cluster); the visual spec uses cross-element
  identity in addition to canonical literals.
- ✅ Preserves existing a11y affordances (decorative `Glyph` `aria-hidden`,
  `aria-live` updating hint) in the rewrite.

### Recommended Changes

1. **Handle the cluster query's loading and error states explicitly**
   (addresses: Cluster block loading/error/null conflation). Surface the hook's
   `isPending`/`isError` in `LibraryDocView` and decide a policy: either render
   the `Cluster` section shell with a loading placeholder + `role="alert"` error
   mirroring the Related-artifacts branch, or state in the plan that silent
   absence during load/error is the intended tradeoff. Add the corresponding
   loading-state and rejected-fetch test cases, and make the negative-case test
   assert absence only after the query settles.

2. **Resolve the Phase 3 `.label` blast radius** (addresses: `.label` restyles
   index cards). Either scope the rule to the panel variant
   (`[data-variant='panel'] .label`) if only the detail rail should change, or
   explicitly declare the index-card restyle in-scope and add an index-card
   assertion (or deliberate exclusion note) to the spec.

3. **Correct the negative-case characterisation** (addresses: singleton-cluster
   negative case). State that the block renders on essentially every detail page
   (often as a self-only "1 artifact" cluster) and confirm that's intended, or
   add a `cluster.entries.length > 1` render guard so it signals a real
   multi-artifact lifecycle. Pin the negative-case fixture to a `slug: null`
   entry (or a list deliberately missing the doc's path).

4. **Add the missing test infrastructure** (addresses: router route, token
   export, vacuous rail assertion, coverage-guard count, `pluralise` test).
   Register `/lifecycle/$slug` in `router-helpers.tsx`; use
   `LIGHT/DARK_COLOR_TOKENS` by theme in the eyebrow spec; pin the rail check to
   `/lifecycle/first-plan` and assert the inactive-label count; filter the
   coverage guard on the `(inferred)` tag; add a direct `pluralise` unit test.

5. **Decide the `font-weight` question now** (addresses: cross-lens font-weight
   parity). Drop the explicit `font-weight: 600` from `.aside h3` so it inherits
   like the eyebrow, and add `fontWeight` to the equality spec's asserted
   properties so the "identical weight" criterion is machine-checked.

6. **Preserve the declared/inferred meaning and check contrast** (addresses:
   legend removal + colour distinction). Carry the term definitions in a
   lightweight affordance (a `title`/`aria-label` per tag or a one-line caption)
   and confirm the `(inferred)` faint colour meets the 4.5:1 text-contrast
   minimum now that it is meaning-bearing text.

7. **Confirm the rail-label a11y tradeoff** (addresses: uppercase content
   label). Note in the success criteria that the rail label is navigational
   content and verify it still reads as a stage name to a screen reader after the
   uppercase promotion.

8. **Apply the minor tightenings** (addresses: remaining minors). Make the
   class/testid renames non-optional, memoise the cluster derivation, key the
   empty-state off combined `rows.length`, render tag text from an explicit map,
   and record the whole-list-fetch and token-duplication tradeoffs.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound in its core thesis: it correctly
identifies Option B as a pure render-layer change, and corrects the work item's
wrong data-source assumption by introducing a path-matched cluster-resolution
hook over the shared lifecycle cache. Phase boundaries are clean, independently
mergeable, and well-sequenced. The principal architectural risk is that Phase 3's
eyebrow change targets the shared `.label` rule in the `Pipeline` component,
rendered in two distinct contexts (lifecycle index cards and the cluster detail
panel), so the change has a broader blast radius than the plan's "rail label"
framing acknowledges.

**Strengths**:
- Correctly establishes Option B as a pure render-layer transformation;
  server-encoded provenance preserved, no API/wire/type change needed.
- Replaces the work item's incorrect data source with robust path-matched
  resolution, avoiding coupling to server-internal slug-selection heuristics.
- `useDocCluster` reuses `queryKeys.lifecycle()` shared with `LifecycleIndex`.
- Phase sequencing genuinely avoids rework; each phase ships working.
- `pluralise` extraction removes copy-paste between `LifecycleIndex` and the new
  block.
- Adopting `--tracking-caps` creates a single source of truth for caps tracking.

**Findings**:
- 🟡 MAJOR (high) — Phase 3 step 3: `Pipeline .label` is one shared rule applied
  to both `card` (index cards, `LifecycleIndex.tsx:118`) and `panel`
  (`LifecycleClusterView.tsx:92`) variants. The unconditional edit restyles
  index-card stage labels too — a surface the plan's "NOT Doing" and the
  eyebrow-equality spec don't account for. Scope to the panel variant or
  acknowledge and cover the index-card surface.
- 🔵 MINOR (medium) — Phase 2 steps 2/4: `useDocCluster` is wired directly into
  `LibraryDocView` rather than composed into `useDocPageData`, whose docstring
  names it as the read-side fanout join point. Consider consolidating.
- 🔵 MINOR (medium) — Performance Considerations / Phase 2: whole-list
  `/api/lifecycle` fetch to find one cluster by `path`, where a per-slug endpoint
  exists. Acceptable tradeoff; record it explicitly.
- 🔵 MINOR (high) — Phase 2 step 3 / Desired End State: nav uses `cluster.slug`
  (server representative slug), correctly read off the resolved object rather than
  derived. The one client dependency on that value; track from both call sites on
  any future slug migration.
- 🔵 MINOR (medium) — Coordination Notes: Phase 3 edits `Pipeline.module.css
  .label`, owned by work item 0040. Coordination acknowledged; consider a shared
  token/class to reduce the ownership collision.

### Code Quality

**Summary**: A well-researched, tightly-scoped render-layer plan with strong
code-quality instincts: it extracts a shared `pluralise` helper, isolates cluster
resolution into a testable `useDocCluster` hook, and keeps each phase small and
independently shippable. The main maintainability concerns are an
under-specified font-weight decision, the persistence of three near-identical CSS
rules sharing values via tokens but not structure, and a couple of
naming/data-shape choices worth tightening. None blocking.

**Strengths**:
- `pluralise` extracted into `format.ts` and original caller rewired — textbook,
  pragmatic DRY.
- `useDocCluster` cleanly separates resolution (query + path-match) from
  presentational `RelatedCluster`.
- Path-matching by `entry.path` is the robust, self-documenting choice, with the
  why explained.
- Phases sequenced to avoid rework; "What We're NOT Doing" fences scope.
- Correctly identifies existing tests assert the old structure and must be
  rewritten red-first.

**Findings**:
- 🔵 MINOR (high) — Phase 3 §2: under-specified font-weight ("drop 600 if it
  diverges … or confirm parity"). The eyebrow declares no weight (inherits), so
  they genuinely diverge; font-weight is omitted from the asserted properties.
  Resolve now (drop the explicit 600) and add `fontWeight` to the spec.
- 🔵 MINOR (medium) — Phase 3 overall: token-sharing unifies values but leaves
  three duplicated five-property rules to drift. Acceptable given CSS-module
  scoping; note the tradeoff; keep the cross-element identity assertion.
- 🔵 MINOR (medium) — Phase 1 §2: row mapping `{ entry, kind }` derives both tag
  text and class from one literal (`({kind})`). Render display copy from an
  explicit map to decouple copy from styling.
- 🔵 MINOR (low) — Phase 1 §3: optional class renames risk misleading vestigial
  names (`groupList`, `badge`) post-Option B. Make renames non-optional and
  concrete.
- 🔵 MINOR (low) — Phase 2 §2: `useDocCluster` recomputes `find` every render
  without memoisation, inconsistent with `LifecycleIndex`'s `useMemo`. Optional.

### Test Coverage

**Summary**: Unusually test-disciplined: every phase is red-first, each
enumerates the specific assertions to add/remove, and the existing test inventory
is correctly identified as asserting the old structure and slated for rewrite.
Coverage is well-matched to risk. The main gaps are test-architecture mechanics:
the shared router lacks a `/lifecycle/$slug` route (so the link-href assertion
can't pass), the hook's loading/error states are under-specified for tests, and
the eyebrow spec references a token-export shape that doesn't match the codebase.

**Strengths**:
- Every phase mandates red-first with explicit "fails before, passes after".
- Correctly identifies stale old-structure assertions to rewrite.
- Coverage rigour proportional to risk (focused row assertions vs dedicated hook
  positive/negative cases).
- Negative/absence cases called out specifically.
- Visual strategy uses cross-element identity plus canonical literals.

**Findings**:
- 🟡 MAJOR (high) — Phase 2 §5: `src/test/router-helpers.tsx` registers no
  `/lifecycle/$slug` route; a `<Link>` to an unregistered route won't produce the
  expected resolved href. Extend the test router before authoring the tests.
- 🟡 MAJOR (medium) — Phase 2 §2/§4: only positive and no-match cluster cases
  planned; loading (data undefined) and error (fetch rejects) states untested,
  unlike the sibling related/content queries. Add both.
- 🔵 MINOR (high) — Phase 3 §4: spec references a theme-agnostic
  `tokens['ac-fg-faint']`; the module exports `LIGHT_COLOR_TOKENS`/
  `DARK_COLOR_TOKENS` by theme. Update the spec description.
- 🔵 MINOR (medium) — Phase 3 §4: inactive rail-label locator can match zero
  elements → vacuous pass. Pin to `/lifecycle/first-plan` and assert count ≥ 1.
- 🔵 MINOR (medium) — Phase 1 §4: post-merge, a whole-list selector over-counts;
  count guard must filter on the `(inferred)` tag.
- 🔵 MINOR (medium) — Phase 2 §1: extracted `pluralise` has no direct test. Add
  `format.test.ts` cases (0/1/>1/explicit plural).

### Correctness

**Summary**: Logically sound at the data-flow level: the path-based match is
robust (both docs and clusters carry the identical server-canonical `path`
through the same normalisation), declared-before-inferred concatenation is
well-defined, and the eyebrow CSS consolidation is property-correct. The one
genuine gap is that the stated negative case (`completeness == null` / "in no
cluster") is far narrower than the prose implies: the server places virtually
every slug-bearing document into at least a singleton/per-path bucket, so
`cluster` is non-null for nearly all docs. Secondary: the loading window returns
`cluster: null`, render-correct but conflating "loading" with "no cluster".

**Strengths**:
- Path-based match is the correct invariant; both lists serialise the same
  canonical `path` through the same boundary.
- Declared-before-inferred ordering is total; server dedup guarantees mutually
  exclusive tag sets.
- Correctly rejects `cluster-via-label.ts` and slug-derivation in favour of
  membership-by-path.
- Shared cache key gives one coherent cluster snapshot.

**Findings**:
- 🟡 MAJOR (high) — Current State Analysis / Phase 2: nearly every slug-bearing
  doc is in at least a singleton cluster (`plan_without_typed_linkage_falls_back_to_slug_bucket`,
  `orphan_types_with_colliding_slugs_do_not_merge`); only `slug == null` entries
  are excluded. So the block renders almost always (often self-only "1 artifact"),
  and the negative-case fixture needs a `slug: null` entry. Confirm intent or add
  an `entries.length > 1` guard.
- 🔵 MINOR (high) — Phase 2 §2: `cluster = data?.find(...) ?? null` makes loading
  indistinguishable from no-cluster; block pops in after fetch and absence tests
  can pass spuriously against the loading state. Expose `isPending` and gate
  absence assertions.
- 🔵 MINOR (medium) — Phase 1 §2: ensure the empty-state branch derives from
  combined `rows.length === 0` (all three arrays), not just `declared`, so
  inferred-only/declared-only docs render correctly.
- 🔵 MINOR (medium) — Phase 3 §2: font-weight parity left as either/or and
  excluded from the five-property spec; could render bolder undetected. Resolve.
- 🔵 MINOR (low) — Phase 1 §4 / Phase 2: coverage-guard `length - 1` assumes one
  of every type and self-exclusion, while the cluster `<n>` includes self.
  Document the differing membership semantics and assert against actual fixture
  size if not pinned.

### Standards

**Summary**: Strongly aligned with codebase conventions: new files land where
expected (hook in `src/api/`, component in its own PascalCase dir with co-located
`.module.css`/`.test.tsx`, helper in `format.ts`), and it correctly adopts the
modern typed `<Link>` for new wiring while deferring `<a href>` modernisation.
The main concerns are accessibility-related: Phase 3 promotes navigational stage
labels to uppercase mono, and Phase 1 leaves the declared/inferred distinction
partly reliant on colour. A few convention details (font-weight, testid naming)
are left open.

**Strengths**:
- New files follow established layout precisely (kebab-case `use-*.ts`, PascalCase
  component dirs).
- Adopts modern typed `<Link>` and reuses `queryKeys.lifecycle()`.
- Extracts `pluralise` into shared `format.ts`, rewiring the original caller.
- Adopts `--tracking-caps` as a single source for tracking.
- Preserves `Glyph` `aria-hidden` + `aria-live` updating hint.

**Findings**:
- 🟡 MAJOR (medium) — Phase 3 §3: rail `.label` is content text (stage name);
  uppercasing + wide tracking is a WCAG legibility/screen-reader concern.
  Confirm intent and make the tradeoff explicit in success criteria.
- 🔵 MINOR (medium) — Phase 1 §2/§3: declared/inferred distinguished by
  parenthesised text + accent-vs-faint colour; legend removed. WCAG 1.4.1 met by
  the literal word, but verify `--ac-fg-faint` meets 1.4.3 text contrast and
  consider a gloss replacing the legend.
- 🔵 MINOR (high) — Phase 3 §2: font-weight reconciliation undecided; not among
  the five equality properties. Pin the value in the plan.
- 🔵 MINOR (medium) — Phase 1 §2/§4: `related-list` testid and "rename if
  preferred" diverge from the `related-group-*` family. Decide renames; keep a
  greppable `related-` prefix.

### Performance

**Summary**: Predominantly render-layer with small per-render data sets; the
dominant question is `useDocCluster` fetching the full `/api/lifecycle` list on
every detail page. That cost is well-mitigated by the existing `staleTime:
Infinity` + SSE-invalidation cache shared under `queryKeys.lifecycle()`, so a
warm cache makes it free; residual concerns are the cold-cache payload (the
heaviest list endpoint, embedding every cluster's full `entries[]`) and the
linear membership scan. Acceptable for a local tool over a small meta directory.

**Strengths**:
- Reuses the exact `queryKeys.lifecycle()` cache key, so index↔detail navigation
  incurs zero extra round-trips.
- Correctly identifies Option B and eyebrow unification as pure render-layer.
- Performance Considerations section explicitly reasons about the fetch and
  concludes no pagination needed.
- Path-matching avoids a fragile per-page server round-trip.

**Findings**:
- 🔵 MINOR (high) — Performance Considerations / Phase 2 §2: cold-cache direct
  load fetches `/api/lifecycle` (heaviest endpoint, full `entries[]` with
  frontmatter) for one small block. Accept now; note the targeted single-cluster
  fetch as the scale escape hatch.
- 🔵 SUGGESTION (high) — Phase 2 §2: nested linear scan recomputed every render,
  unmemoised (unlike `LifecycleIndex`). Wrap in `useMemo([query.data,
  entry?.path])`.
- 🔵 SUGGESTION (medium) — Phase 2 §4: once mounted, every `doc-changed`/
  `doc-invalid` SSE event refetches the full list while on one document.
  Acceptable for single-user; note it.

### Usability

**Summary**: Well-scoped with strong developer ergonomics: `useDocCluster`
reuses the shared cache and enabled-gating convention, `RelatedCluster` mirrors
`LifecycleIndex` card idioms, and shared `pluralise` removes real duplication.
The main gaps are end-user UX: the Cluster block silently vanishes (rather than
degrading visibly) while its query loads or errors, and removing the legend
strips the only explanation of declared/inferred. Smaller DX wrinkles (pluralise
placement, unresolved font-weight) are minor.

**Strengths**:
- `useDocCluster` follows the established hook convention (shared cache key,
  enabled gating) — predictable.
- Path-matching is the least-surprise data path, with the why explained.
- Shared `pluralise` keeps "1 artifact"/"N artifacts" consistent across surfaces.
- `RelatedCluster` reuses existing meta idioms (`formatMtime`, ` · `, typed
  `<Link>`).
- Adopts `--tracking-caps` as one shared knob.

**Findings**:
- 🔴/🟡 MAJOR (high) — Phase 2: block renders only `{cluster && …}` and the hook
  returns `null` during loading and on error alike, so a real cluster affordance
  vanishes with no feedback, unlike the Related-artifacts loading/`role=alert`
  branches. Consume `isPending`/`isError` and decide an explicit policy.
- 🟡 MAJOR (medium) — Phase 1: removing `<Legend>` deletes the only on-screen
  definition of declared/inferred; bare tags differ only by colour, invisible to
  colour-blind users. Preserve definitions via `title`/`aria-label` or a caption.
- 🔵 MINOR (high) — Phase 2 §1: `pluralise` in `format.ts` (all date/time
  helpers) hurts discoverability. Consider a `text.ts`/`strings.ts` home.
- 🔵 MINOR (medium) — Phase 3 §2: font-weight left as either/or; could ship
  bolder than the eyebrow while the five-property spec passes. State the decision.

## Re-Review (Pass 2) — 2026-06-06

**Verdict:** REVISE

All seven lenses were re-run against the revised plan. **Every major and
cross-cutting finding from Pass 1 is resolved.** The re-run surfaced six new
major findings — three were introduced by the Pass 1 edits themselves (the hook
return shape, the `RelatedCluster` null-narrowing, and a terse error copy), one
was a genuine latent bug exposed by deeper review (declared-list duplication),
and two were test-infrastructure gaps (unmocked fetch in existing view tests, no
cluster fixture factory). **All six new majors, plus the notable new minors, have
since been addressed by a follow-up round of edits** (see "New Issues — and
their resolution" below). The plan is now substantially ready; a confirmatory
Pass 3 is optional.

### Previously Identified Issues
- 🟡 **Test Coverage / Usability / Correctness**: Cluster block loading/error/null
  conflation — **Resolved**. Hook now exposes query status; the view degrades
  visibly (`Loading…` / `role="alert"`) mirroring the related-artifacts section;
  loading, error, and settled-no-match are all tested. Praised by Architecture,
  Correctness, and Usability re-runs.
- 🟡 **Architecture**: Phase 3 `.label` blast radius — **Resolved**. Scoped to
  `.chain[data-variant='panel'] .label`; card-variant index labels untouched.
  Confirmed correct by Architecture, Correctness, and Code Quality re-runs.
- 🟡 **Test Coverage**: Test router missing `/lifecycle/$slug` — **Resolved**.
  Added as an explicit prerequisite step before the `<Link>` href tests.
- 🟡 **Correctness**: Negative case / singleton clusters — **Resolved**. Plan now
  states the block is near-ubiquitous and pins the negative-case fixture to a
  `slug == null` entry; Correctness re-run verified the reasoning against the
  server.
- 🟡 **Standards**: Rail-label uppercase a11y — **Resolved** (downgraded to a
  suggestion). The content-vs-chrome trade-off is acknowledged with a manual
  screen-reader/legibility check.
- 🟡 **Usability / Standards**: Legend removal + contrast — **Resolved**. Contrast
  verification added; **broadened this round** to cover all newly-faint surfaces
  (aside `<h3>`, cluster meta, rail label), not just the `(inferred)` tag.
- 🔵 **Cross-lens**: `font-weight` parity — **Resolved**. `.aside h3` drops the
  explicit `600`; `fontWeight` added as a machine-checked sixth property in the
  equality spec.
- 🔵 Remaining Pass 1 minors (token export, vacuous rail assertion,
  coverage-guard count, `pluralise` test, empty-state keying, naming,
  memoisation, performance tradeoffs) — **Resolved** in the Pass 1 edits.

### New Issues Introduced (Pass 2) — and their resolution
- 🔴 **Correctness**: Declared-list duplication — flattening
  `[...declaredOutbound, ...declaredInbound]` double-renders any artifact with a
  bidirectional declared relationship (server does not dedup across the two
  arrays), with a colliding React `key`. **Fixed**: plan now dedups the declared
  list by `path`, keys rows by `entry.path`, and adds a bidirectional-duplicate
  test fixture. *(Latent bug exposed by deeper review, not introduced by edits.)*
- 🟡 **Correctness**: `RelatedCluster cluster={cluster}` passed `LifecycleCluster
  | null` into a non-nullable prop without narrowing (typecheck failure).
  **Fixed**: inner render branches `cluster ? <RelatedCluster/> : null`.
  *(Introduced by the Pass 1 visible-degradation edit.)*
- 🟡 **Usability**: `useDocCluster` returned a bespoke `{cluster, isPending,
  isError, error}` shape diverging from sibling hooks. **Fixed**: reverted to
  `return { ...query, cluster }` so the full `UseQueryResult` surface is
  preserved. *(Introduced by the Pass 1 edit; original plan had `{...query,
  cluster}`.)*
- 🟡 **Test Coverage**: Wiring `useDocCluster` into the view fires an unmocked
  `fetchLifecycleClusters` in every existing `LibraryDocView` test. **Fixed**:
  added a prerequisite to stub it suite-wide (default `[]`).
- 🟡 **Test Coverage**: No `LifecycleCluster` test fixture/factory. **Fixed**:
  added a `makeLifecycleCluster` factory prerequisite.
- 🔵 New minors — also addressed: `format.test.ts` is *new* not "extend";
  loading test must use a never-resolving enabled query (the `isPending`-on-idle
  caveat); positional ordering assertion; coverage-guard scoped to inferred-row
  icons (`:has(.tagInferred) svg[data-doc-type]`); cluster error copy aligned
  with `LifecycleIndex`; CSS comment on the shared base `.label` rule;
  `fetchLifecycleCluster`-not-used comment.

### Remaining (deferred by design / low priority)
- Compose `useDocCluster` into `useDocPageData` (the documented read-fanout join
  point) — deferred with a reasoned note; revisit on a third read query.
- Extract a shared `QueryState`/`AsideSection` degradation helper — noted as a
  consideration (two call sites).
- Optional automated contrast guard and `staleTime`-parity verification —
  captured as validation-time notes.

### Assessment
The revision successfully closed every Pass 1 concern. The Pass 2 re-run did its
job — it caught a real latent duplication bug and the regressions the Pass 1
edits introduced — and those have now been fixed in turn. No open major findings
remain in the plan; the residual items are explicitly deferred or low-priority.
The plan is ready for implementation; a final confirmatory pass is optional
rather than required.

## Re-Review (Pass 3, confirmatory: correctness + test-coverage) — 2026-06-06

**Verdict:** REVISE (2 majors found, both since fixed → plan now ready)

A focused confirmatory pass on the two lenses where the Pass 2 fixes landed.
**All five core logic fixes were confirmed sound** (declared-list dedup against
the verified `related.rs:53-81` non-dedup behaviour; path-based match; the
tri-state hook + `isPending`-while-idle handling; the inner `cluster ?`
narrowing; `{ ...query, cluster }`). Test-coverage confirmed all six of its
prior concerns soundly addressed and codebase-accurate. The pass caught two
**new** majors — both in the Playwright selectors the Pass 1/2 edits introduced —
plus minor refinements. All have been fixed.

### Previously Identified Issues (the Pass 2 fixes)
- 🔴 Correctness — declared-list dedup — **Confirmed sound** (grounded in verified
  server non-dedup; first-occurrence-wins by `path` resolves the colliding key).
- 🟡 Correctness — `RelatedCluster` null narrowing — **Confirmed sound** (the
  inner `cluster ?` branch is genuinely required; outer guard does not narrow).
- 🟡 Correctness — loading vs no-cluster + `isPending`-while-idle — **Confirmed
  sound** (masked in the view; never-resolving enabled-query test mandated).
- 🟡 Usability/Correctness — hook returns `{ ...query, cluster }` — **Confirmed**.
- 🟡 Test Coverage — suite-wide `fetchLifecycleClusters` stub — **Confirmed**
  (no global fetch stub exists; spy-based default is the right shape).
- 🟡 Test Coverage — `makeLifecycleCluster` factory — **Confirmed** (no cluster
  factory exists today; centralises shape churn).
- 🔵 Test Coverage — `format.test.ts` new / never-resolving loading test /
  positional ordering / theme-specific tokens / non-vacuous locator guard /
  machine-checked `fontWeight` — **All confirmed** addressed and accurate.

### New Issues Introduced (Pass 3) — and their resolution
- 🔴 **Correctness**: Coverage-guard selector `:has(.tagInferred)` targets a
  **hashed CSS-module class** unselectable in the built bundle (the existing spec
  selects only by `data-testid`/`data-*`/stable globals; verified). **Fixed**:
  added `data-kind={kind}` to the `related-row` `<li>`; the spec now scopes via
  `[data-testid="related-row"][data-kind="inferred"] svg[data-doc-type]`.
- 🔴 **Correctness**: Eyebrow-equality rail locator
  `.chain[data-variant='panel'] … .label` targets hashed module classes — the
  label span (`Pipeline.tsx:64`) carries only `styles.label`, no stable hook
  (verified). **Fixed**: Phase 3 §3 now adds a global `ac-stagechain__label`
  class to the span; §4 selects via
  `.ac-stagechain[data-variant='panel'] .ac-stagechain__stage[data-active='false'] .ac-stagechain__label`.
  *(The CSS-module selectors inside the `.module.css` files were always fine —
  module scoping resolves them; only the Playwright spec selectors were broken.)*
- 🔵 **Correctness**: Negative-case precision — a slug-less **orphan-by-design**
  type still buckets; only a slug-less **lifecycle-participating** type is
  excluded. **Fixed**: fixture guidance tightened (prefer a list missing the
  doc's `path`; if no-slug, use a `Plan`).
- 🔵 **Test Coverage**: suite-wide stub must be a `beforeEach` `mockResolvedValue`
  with per-test overrides after it; settled-no-match test needs a settle anchor
  (`waitFor` non-pending before asserting absence); ordering fixture must use
  distinct `path`s disjoint from the dedup fixture. **All three folded in.**

### Assessment
Three passes converged cleanly. The logic is confirmed correct; the only Pass 3
findings were test-selector mechanics (hashed module classes vs stable hooks),
now resolved with verified stable DOM hooks (`data-kind`, `ac-stagechain__label`).
No open findings remain across any lens. The plan is **ready for
implementation** — no further review pass is needed.

## Final Verdict — APPROVE (2026-06-06)

After three review passes (initial + two re-reviews), every finding across all
seven lenses is resolved. The logic is confirmed correct, the test strategy is
sound and codebase-accurate, and the only late findings were test-selector
mechanics now fixed with verified stable DOM hooks. No open findings remain.

**Review approved. Plan accepted and ready for implementation.**
