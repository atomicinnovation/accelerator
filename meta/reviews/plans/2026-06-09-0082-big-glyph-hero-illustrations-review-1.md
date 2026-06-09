---
type: plan-review
id: "2026-06-09-0082-big-glyph-hero-illustrations-review-1"
title: "Plan Review: BigGlyph Hero Illustration Set"
date: "2026-06-10T10:45:34+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
parent: "plan:2026-06-09-0082-big-glyph-hero-illustrations"
target: "plan:2026-06-09-0082-big-glyph-hero-illustrations"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability]
review_number: 1
review_pass: 2
tags: [design, frontend, components, illustrations]
last_updated: "2026-06-10T19:53:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: BigGlyph Hero Illustration Set

**Verdict:** REVISE

This is a strong, well-grounded plan: it mirrors the proven 0037 `Glyph`
component/showcase/visual-regression structure, reuses the sanctioned `PaperFold`
runtime-hsl colour model, isolates each traced shape into one file for
reviewability, and draws clean, independently-integratable phase boundaries. The
verdict is REVISE rather than APPROVE because of four major findings — all
fixable with small, localised edits, none requiring a structural rethink. The
headline issue is a self-contradiction in the Phase 1 component snippet that, if
copied verbatim, ships a throwing off-union path and fails the very fallback AC
it is meant to satisfy. The other three concern a dark-theme polling trap
inherited from a flawed template and two test-assertion gaps where the suite
cannot distinguish a correct implementation from several plausible miswirings.

### Cross-Cutting Themes

- **The off-union hue-resolution guard is contradicted between code and prose**
  (flagged by: Architecture, Code Quality, Correctness, Usability) — The
  component snippet computes `const resolvedHue = hue ?? TYPE_COPY[docType].hue`
  (unguarded), while the prose immediately below mandates
  `TYPE_COPY[docType]?.hue ?? 215`. As written, the snippet throws on an
  off-union `docType` before the `?? DefaultBigGlyph` dispatch fallback can fire,
  defeating the `DEFAULT_BIG` fallback (AC1) and failing the planned
  "renders without throwing" unit test. Four independent lenses converged on this
  exact contradiction — it is the single most important fix.

- **Off-union handling silently diverges from the sibling `Glyph` contract**
  (flagged by: Code Quality, Standards, Usability) — `Glyph` returns `null` and
  emits a DEV `console.warn` (with the valid key list) for unknown `docType`;
  `BigGlyph` silently renders `DEFAULT_BIG` with no diagnostic. The graceful
  fallback render is AC-mandated and correct, but the divergence from the
  component this plan explicitly sets out to mirror is undocumented and removes a
  developer-facing signal that an accidental bad key was passed.

- **Key test assertions cannot distinguish correct from miswired**
  (flagged by: Test Coverage) — The EmptyState integration test asserts only
  "a decorative 80×80 svg exists" (passes for any docType, any size); the
  off-union test asserts "no throw" (passes whether the fallback fired or not);
  and per-type distinctness has no automated cross-baseline check. Each leaves a
  load-bearing behaviour effectively unverified.

### Tradeoff Analysis

- **Faithful prototype port vs. codebase consistency**: Several findings
  (uppercase `#FFFFFF` vs the codebase's `#ffffff`; `(p) => ReactElement` render
  functions vs `Glyph`'s `ComponentType`; silent fallback vs `Glyph`'s warn)
  stem from the tension between porting the prototype verbatim and matching local
  conventions. The plan's verbatim-port discipline is a genuine strength for
  tracing fidelity, so the recommendation is to keep the ported *shapes* exact
  but reconcile *surrounding scaffolding* (casing, fallback diagnostics, prop
  docs) to the codebase, with a one-line comment wherever a deliberate divergence
  remains.

- **Component reuse vs. dependency direction**: Internal hue resolution
  (`<BigGlyph docType={docType} />`) gives the cleanest call site, but couples a
  `src/components/` primitive upward to a `src/routes/library/` module. Cleaner
  call site vs. cleaner dependency graph — see the Architecture finding for the
  neutral-location option that gets both.

### Findings

#### Critical

_None._

#### Major

- 🔴 **Code Quality / Correctness**: Component snippet's off-union hue lookup
  throws, contradicting the prose guard and the `DEFAULT_BIG` fallback (AC1)
  **Location**: Phase 1, Section 3 (The component + dispatch)
  The snippet uses `hue ?? TYPE_COPY[docType].hue` (line 306) while the prose
  (lines 322–327) mandates `TYPE_COPY[docType]?.hue ?? 215`. An off-union
  `docType` with no `hue` override hits a TypeError before `?? DefaultBigGlyph`
  runs; an implementer copying the snippet verbatim fails AC1 and the Phase 1
  "renders without throwing" test.

- 🟡 **Correctness / Test Coverage**: Dark-theme polling compares an `rgb()`
  result against a hex token (and the mirrored template predicate is a no-op)
  **Location**: Phase 2, Section 3 (Visual-regression spec — dark-theme polling)
  `getComputedStyle(...).backgroundColor` returns `rgb(19, 21, 36)`, never the
  hex `#131524` the plan proposes to match, so the `waitForFunction` would hang
  to timeout. Worse, the `glyph-showcase.spec.ts:23-35` template it says to
  mirror has a tautological predicate (`colour !== '' && hex.length > 0`) that
  never compares against the dark token — mirroring it literally inherits a poll
  that confirms nothing.

- 🟡 **Test Coverage**: Per-type distinctness has no automated guard — rests
  entirely on a one-time manual sign-off
  **Location**: Phase 2, Section 4 (Baseline capture + sign-off); AC5
  AC1 requires distinctness be "verified by those baselines rather than by
  subjective judgement," but the 26 per-cell baselines only guard each cell
  against *its own* future regression. Nothing asserts the 13 illustrations
  differ *from each other*, so a dispatch copy-paste error or a duplicated trace
  would pass every baseline.

- 🟡 **Test Coverage**: EmptyState integration test does not verify the correct
  per-type hero rendered
  **Location**: Phase 3, Section 3 (Test the swapped hero)
  Asserting only that an `<svg viewBox="0 0 80 80" aria-hidden="true">` exists
  passes for any docType and any size — a regression wiring EmptyState to a
  hardcoded/wrong docType, or dropping `size={96}` back to 72, would still pass.
  The single integration point — the point of the story — is untested for
  correctness of wiring.

#### Minor

- 🔵 **Architecture**: Shared `src/components/` primitive depends upward on a
  route-area module (`src/routes/library/empty-descriptions`) for its hue source
  **Location**: Phase 1, Section 3
  Inverts the usual dependency direction and couples a reusable primitive to
  library-route internals; `PaperFold` avoids this by taking `hue` as a prop.

- 🔵 **Code Quality / Standards / Usability**: Off-union fallback is silent where
  the sibling `Glyph` emits a DEV `console.warn`
  **Location**: Phase 1, Section 3 (dispatch)
  Undocumented divergence from the component being mirrored; an off-union key
  renders a plausible-but-wrong paper sheet with no diagnostic signal.

- 🔵 **Code Quality**: `bigPalette` JSDoc forward-references `PaperFold`, which is
  deleted in the same change set
  **Location**: Phase 1, Section 1
  Anchor the formulas to the authoritative prototype (`big-glyphs.jsx:16-26`)
  instead, so the comment doesn't dangle after Phase 3.

- 🔵 **Code Quality**: PascalCase illustration exports are render functions
  (`draw(palette)`), not components — easy to mistake for JSX-renderable
  **Location**: Phase 1, Sections 2/3
  A future contributor may reach for `<XBigGlyph />` (the `Glyph` idiom); a
  one-line note on the `BIG_GLYPHS` record pre-empts it.

- 🔵 **Correctness**: Hue resolution must use `??` (not `||`) or a `hue={0}`
  override (valid red) is silently discarded for the default 215
  **Location**: Phase 1, Sections 3/4
  The prototype uses `hue || 215`; copying that idiom drops the boundary hue 0.
  Specify `??` and add a `hue={0}` case to the override test.

- 🔵 **Test Coverage**: Off-union fallback test asserts "no throw" but not that
  `DefaultBigGlyph` was actually selected
  **Location**: Phase 1, Section 4
  Passes whether the `?? DefaultBigGlyph` branch fires or not; give
  `DefaultBigGlyph` a stable marker and assert it (plus the 215 guard hue).

- 🔵 **Test Coverage**: The "no colour literals outside the sanctioned set" (AC2)
  invariant is left to manual code review, not automated
  **Location**: Phase 1, Section 4 / Testing Strategy
  The 0037 `Glyph` suite deep-walks every descendant `fill`/`stroke` against an
  allow-list; mirror that to catch a future stray `hsl(...)` literal a reviewer
  might miss.

- 🔵 **Correctness**: EmptyState re-resolves the hue independently of the
  panel-gradient hue after the swap
  **Location**: Phase 3, Section 1
  Identical today (typed `Record`), but a future panel-hue override without a
  matching hero override would desync. Either pass the resolved hue to BigGlyph
  or document the deliberate delegation.

- 🔵 **Standards**: Shared-constants modules diverge from the `Glyph.constants.ts`
  convention; the diff-tints file sits under `icons/` among per-type components
  **Location**: Phase 1, Section 1
  Consider consolidating into `BigGlyph.constants.ts` at the component root and
  keeping constant-only files out of `icons/`.

- 🔵 **Usability**: Override prop `hue?: number` diverges from sibling `Glyph`'s
  `colorVar?: string` in both name and type
  **Location**: Phase 1, Section 3 (BigGlyphProps)
  Justified by the runtime-hsl architecture, but a JSDoc note contrasting the two
  makes the deliberate divergence discoverable at the point of use.

- 🔵 **Usability**: `size` is optional/unconstrained here but required and union
  (`16|24|32`) on `Glyph`
  **Location**: Phase 1, Section 3
  The looser typing is the right call for a scalable hero; a one-line JSDoc note
  pre-empts the surprise.

#### Suggestions

- 🔵 **Architecture**: The throwaway `/big-glyph-showcase` route + its 26
  baselines become a hard dependency that 0083 must *migrate*, not just delete
  **Location**: What We're NOT Doing / Phase 2
  Ensure the 0083 `blocks` edge captures the migration obligation so VR coverage
  isn't lost in the route consolidation.

- 🔵 **Test Coverage**: Visual regression covers the throwaway showcase backdrop,
  not the real EmptyState gradient-panel surface
  **Location**: Phase 2 vs Phase 3
  Consider one representative EmptyState baseline (single docType, both themes) so
  the production composition has minimal regression protection.

- 🔵 **Usability**: The showcase exercises only the 13 union keys, never the
  shipped `DEFAULT_BIG` fallback
  **Location**: Phase 2 / Phase 1 dispatch
  Optionally add one cast off-union cell labelled "fallback / DEFAULT_BIG" so the
  fallback is visually discoverable.

- 🔵 **Standards**: `white: '#FFFFFF'` (prototype casing) diverges from the
  codebase's lowercase `#ffffff`
  **Location**: Phase 1, Section 1
  Prefer lowercase to match neighbouring code, or note the verbatim-port intent
  so reviewers don't flag it.

### Strengths

- ✅ Clean phase decomposition with an explicit dependency graph (Phase 1
  prerequisite; Phases 2 and 3 independent), each phase independently
  integratable and mergeable.
- ✅ The key divergences from `Glyph` (palette-arg dispatch vs zero-arg
  `ComponentType`; runtime-hsl vs CSS-var) are explicitly called out and
  justified against the prototype contract rather than left implicit.
- ✅ Compile-time exhaustiveness via `Record<DocTypeKey, …>`; adding/removing a
  doc type fails the typecheck rather than silently degrading.
- ✅ One-file-per-shape isolation directly serves tracing-fidelity review and
  keeps each illustration independently replaceable.
- ✅ Sanctioned colour-literal exceptions (notes shadow, pr-reviews diff tints)
  are extracted into named/commented constants and pinned by a unit test, making
  the AC2 review check mechanical rather than subjective.
- ✅ The TDD boundary is drawn honestly — mechanically-verifiable behaviour is
  test-first; the inherently-visual SVG shapes' red-green loop is correctly the
  baseline sign-off, not an overclaimed unit test.
- ✅ The `bigPalette` hue-equality test is specific and mutation-resistant (parses
  the `hsl(<hue> …)` prefix); per-cell clipped screenshots (maxDiffPixelRatio
  0.05) are the correct granularity for a 13-type set.
- ✅ Cross-platform baseline commitment correctly accounts for the known
  darwin/linux drift and the `GITHUB_TOKEN` re-trigger gotcha.
- ✅ Accessibility convention is correctly preserved and tested: `aria-hidden`,
  no `role`, accessible title/lede/footer unchanged.
- ✅ The orphaned-`PaperFold` removal is correctly scoped — deletion atomic with
  its sole consumer's swap, and the `migration.test.ts` reference is accounted
  for.
- ✅ The primary call site (`<BigGlyph docType={docType} size={96} />`) is minimal
  and removes the hue-plumbing the `PaperFold` caller previously carried.

### Recommended Changes

1. **Fix the off-union hue guard in the Phase 1 code block** (addresses: the
   throwing-snippet major flagged by Code Quality + Correctness, the
   Architecture/Usability variants). Change line 306 to
   `const resolvedHue = hue ?? TYPE_COPY[docType]?.hue ?? 215`, name `215` as the
   documented blue fallback inline, use `??` (not `||`) so `hue={0}` is honoured,
   and delete the now-redundant contradictory prose so the snippet is the single
   source of truth.

2. **Make the dark-theme poll compare resolved `rgb()` and abandon the broken
   template predicate** (addresses: the dark-theme polling major). Convert
   `DARK_COLOR_TOKENS['ac-bg-card']` to `rgb(19, 21, 36)` via the existing
   `parseRgb` helper and poll `getComputedStyle(cell).backgroundColor` for strict
   equality with that rgb string — explicitly do not inherit
   `glyph-showcase.spec.ts`'s no-op `colour !== '' && hex.length > 0` predicate.

3. **Add an automated per-type distinctness check** (addresses: the distinctness
   major). After capture, assert the 13 light-theme baseline buffers are pairwise
   non-identical (hash comparison), so an accidental shape collision or wrong
   dispatch entry fails CI rather than relying on the reviewer's eye.

4. **Strengthen the EmptyState and off-union assertions** (addresses: the
   integration-test major + the off-union-fallback minor). In the EmptyState
   test, render two different docTypes and assert distinct hue-bearing tones
   (e.g. `hsl(12 ` vs `hsl(355 `) and `width`/`height` of 96; in the off-union
   test, give `DefaultBigGlyph` a stable marker and assert it (plus the 215 guard
   hue) is what renders.

5. **Reconcile the off-union diagnostic and prop docs with `Glyph`** (addresses:
   the silent-fallback theme + the prop-divergence minors). Add a guarded
   `import.meta.env.DEV` `console.warn` on the off-union fallback branch (still
   rendering `DEFAULT_BIG`), and add JSDoc notes contrasting `hue` with `Glyph`'s
   `colorVar` and BigGlyph's free scalability with `Glyph`'s fixed size union.

6. **Optional consistency/structure tidies** (addresses: remaining minors and
   suggestions). Re-anchor the `bigPalette` JSDoc to the prototype rather than the
   to-be-deleted `PaperFold`; consider `BigGlyph.constants.ts` over a constants
   file under `icons/`; lowercase `#ffffff`; automate the AC2 "no stray literal"
   check via a deep-walk; weigh a neutral shared hue-map location to avoid the
   upward dependency; and capture the 0083 baseline-migration obligation on the
   `blocks` edge.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound and well-grounded in established
precedent: it mirrors the proven 0037 Glyph component/showcase/visual-regression
structure, reuses the sanctioned PaperFold runtime-hsl colour model, and
correctly isolates each traced shape into one file per type for reviewability.
Phase boundaries are clean and independently integratable (Phase 1 prerequisite;
Phases 2/3 parallel), and the divergence from Glyph's contract (palette-arg
dispatch vs zero-arg ComponentType, runtime-hsl vs CSS-var) is explicitly
identified and justified. The main architectural concern is a dependency-direction
inversion: a shared component under src/components reaches up into a route module
(src/routes/library/empty-descriptions) for its hue source, coupling a reusable
primitive to a feature-area module.

**Strengths**:
- Clean phase decomposition with explicit dependency graph, each phase
  independently integratable and mergeable.
- The key architectural divergence from the 0037 Glyph contract is explicitly
  called out and justified against the prototype's contract rather than left
  implicit.
- Exhaustiveness is enforced structurally at compile time via
  `Record<DocTypeKey, ...>`, so adding/removing a doc type fails the typecheck.
- One-file-per-shape isolation directly serves the tracing-fidelity review and
  keeps each illustration independently replaceable.
- The orphaned-PaperFold removal is correctly scoped: deletion and its sole
  consumer's swap are atomic within Phase 3, and the single textual reference in
  migration.test.ts is accounted for.
- Colour-constant boundaries are deliberately drawn and centralised, keeping
  colour identity single-sourced from the doc-type hue.

**Findings**:
- 🔵 minor (high) — _Phase 1, Section 3_ — **Shared component depends upward on a
  route-area module for its hue source.** Placing `BigGlyph` under
  `src/components/` but importing `TYPE_COPY` from `src/routes/library/` inverts
  the usual dependency direction; `PaperFold` avoids this by taking `hue` as a
  prop. Suggestion: keep a numeric `hue` prop and resolve at the EmptyState call
  site, or move the per-doc-type hue map to a neutral shared location (e.g.
  alongside `DOC_TYPE_KEYS` in `src/api/types.ts`).
- 🔵 minor (high) — _Phase 1, Section 3 (snippet vs prose)_ — **Fallback contract
  not realised in the code as written.** The snippet computes
  `hue ?? TYPE_COPY[docType].hue` while the prose prescribes
  `TYPE_COPY[docType]?.hue ?? 215`; as written the hue lookup throws before the
  `?? DefaultBigGlyph` fallback is reached. Suggestion: make the snippet the
  source of truth with `hue ?? TYPE_COPY[docType]?.hue ?? 215`.
- 🔵 suggestion (medium) — _What We're NOT Doing / Phase 2_ — **Throwaway showcase
  route is an acknowledged duplication that 0083 must reconcile.** The VR locator
  contract + 26 baselines become a hard dependency on a throwaway surface.
  Suggestion: ensure the 0083 `blocks` edge captures the obligation to migrate
  (not just delete) the spec + baselines.

### Code Quality

**Summary**: A well-structured, proportionate plan for a fundamentally simple
feature: a decorative per-doc-type SVG component built by verbatim-porting a
traceable prototype. It mirrors a proven, recent precedent (0037 Glyph) for
component layout, dispatch exhaustiveness, showcase route, and visual-regression
structure, keeping maintainability high and cognitive load low. The main concern
is a contradiction between the BigGlyph code sample and the prose around
off-union fallback hue resolution, plus smaller consistency/maintainability nits.

**Strengths**:
- Strong adherence to an existing, recent precedent (0037 Glyph): co-located
  component, one-file-per-type illustrations, `Record<DocTypeKey, …>`
  exhaustiveness, dev-only showcase route, per-cell clipped visual-regression.
- Clean single-responsibility decomposition (bigPalette / per-type files /
  prReviewsDiffTints / BigGlyph dispatch + svg wrapper).
- Sanctioned colour-literal exceptions extracted into named/commented constants
  and pinned by a unit test, removing the magic-number smell.
- Complexity proportional to requirements; correctly resists over-engineering.
- Decorative-by-default a11y contract and 96px sizing rationale documented
  inline.

**Findings**:
- 🔴 major (high) — _Phase 1, Section 3_ — **Code sample throws on off-union;
  contradicts the prose and the fallback test.** `hue ?? TYPE_COPY[docType].hue`
  throws for off-union keys with no override, yet the prose mandates a guard and
  the plan specifies an "off-union renders without throwing" test. Suggestion:
  make the sample authoritative with `hue ?? TYPE_COPY[docType]?.hue ?? 215` and
  drop the contradictory prose.
- 🔵 minor (high) — _Phase 1, Section 1_ — **`bigPalette` JSDoc forward-references
  PaperFold, which is deleted in the same change set.** Anchor the formulas to
  the prototype (`big-glyphs.jsx:16-26`) so the comment doesn't dangle.
- 🔵 minor (medium) — _Phase 1, Sections 2/3_ — **Illustration render functions
  look like components but are invoked as `draw(palette)`.** PascalCase
  `(p) => ReactElement` functions may mislead a contributor into `<XBigGlyph />`.
  Suggestion: a one-line comment on the `BIG_GLYPHS` record.
- 🔵 minor (medium) — _Phase 1, Section 3_ — **Silent DEFAULT_BIG fallback departs
  from Glyph's null+`console.warn` for unknown types.** Rendering the fallback is
  correct per AC1, but the divergence is silent. Suggestion: optional DEV-only
  warn on the off-union branch, or note the divergence in the dispatch comment.

### Test Coverage

**Summary**: A well-structured test strategy that faithfully mirrors the proven
0037 Glyph patterns: genuinely test-first unit coverage for the mechanically
verifiable surface and a per-cell-clipped 26-combination visual-regression suite
for the inherently visual shapes. The TDD boundary is drawn honestly. The main
gaps are that per-type distinctness and tracing fidelity rest entirely on a
manual sign-off with no automated cross-baseline differencing, the EmptyState
integration test only asserts a generic decorative svg, and the off-union
fallback test asserts "no throw" without confirming DefaultBigGlyph was selected.

**Strengths**:
- TDD boundary drawn honestly: mechanically-verifiable behaviour test-first; the
  SVG shapes' red-green loop is the visual-regression baseline.
- The `bigPalette` hue-equality test is specific and mutation-resistant.
- Per-cell clipped screenshots prevent a single-illustration regression hiding
  under a viewport-wide diff budget.
- Cross-platform baseline commitment correctly accounts for darwin/linux drift
  and the GITHUB_TOKEN re-trigger gotcha.
- PR_REVIEW_DIFF_TINTS exact-equality and dispatch exhaustiveness (=== 13) give
  concrete regression protection.

**Findings**:
- 🟡 major (high) — _Phase 2, Section 4; AC5_ — **Per-type distinctness has no
  automated guard.** The 26 baselines guard each cell against its own future
  regression but nothing asserts the 13 illustrations differ from each other; a
  dispatch copy-paste error or duplicated trace would pass. Suggestion: assert
  the 13 light-theme baseline buffers are pairwise non-identical (hash).
- 🟡 major (high) — _Phase 3, Section 3_ — **EmptyState integration test doesn't
  verify the correct per-type hero rendered.** Asserting only an aria-hidden
  80×80 svg passes for any docType/size. Suggestion: render two docTypes and
  assert distinct hue tones (`hsl(12 ` vs `hsl(355 `) and width/height 96.
- 🔵 minor (high) — _Phase 1, Section 4_ — **Off-union fallback test asserts "no
  throw" but not that DefaultBigGlyph was selected.** Passes whether the fallback
  branch fires or not. Suggestion: give DefaultBigGlyph a stable marker and
  assert it (plus the 215 guard hue).
- 🔵 minor (medium) — _Phase 2, Section 3_ — **Dark-theme polling predicate must
  assert the resolved colour matches the dark token, not "non-empty".** The
  mirrored template (glyph-showcase.spec.ts:23-35) has a tautological predicate.
  Suggestion: compare resolved backgroundColor via parseRgb against
  hexToRgb(DARK_COLOR_TOKENS['ac-bg-card']).
- 🔵 minor (medium) — _Phase 1, Section 4 / Testing Strategy_ — **The "no colour
  literals outside the sanctioned set" (AC2) invariant is left to manual review.**
  The 0037 Glyph suite automated its equivalent with a source-walking test.
  Suggestion: mirror the deep-walk fill/stroke allow-list check.
- 🔵 minor (low) — _Phase 2 vs Phase 3_ — **VR covers the throwaway showcase but
  not the actual EmptyState hero surface.** The production gradient panel has no
  visual baseline. Suggestion: consider one representative EmptyState baseline
  (single docType, both themes).

### Correctness

**Summary**: The plan is logically sound in its core structure — the 13-key
reconciliation, the seven-tone palette port, and the phase dependency graph all
check out against the live code. However, there is a load-bearing inconsistency
between the component code block (which dereferences TYPE_COPY[docType].hue
unguarded) and the surrounding prose (which mandates a defensive guard), and the
specified unit test would fail against the code as written. The dark-theme polling
logic also carries a subtle correctness trap: getComputedStyle returns rgb()
strings, not the hex token value the plan proposes to compare against.

**Strengths**:
- The 13-key DocTypeKey reconciliation is verified correct against
  `src/api/types.ts:4-19`, with the two renames accurately identified.
- The seven-tone bigPalette port is faithful to the prototype and a correct
  superset of PaperFold's four-tone subset with identical formulas.
- The `Record<DocTypeKey, …>` dispatch correctly enforces compile-time
  exhaustiveness; the `?? DefaultBigGlyph` arm is correctly reasoned as
  off-union-only.
- The phase dependency graph is correct; the PaperFold deletion is atomic with
  its sole consumer's update.

**Findings**:
- 🔴 major (high) — _Phase 1, Section 3_ — **Unguarded `TYPE_COPY[docType].hue`
  throws on off-union, defeating DEFAULT_BIG (AC1) and the "no throw" test.** The
  code block (line 306) and the prose (lines 322–327) are flatly contradictory.
  Suggestion: use `hue ?? TYPE_COPY[docType]?.hue ?? 215`, matching the
  prototype's `window.TYPE_META[type] || { hue: hue || 215 }`.
- 🟡 major (high) — _Phase 2, Section 3_ — **Dark-theme poll compares an rgb()
  result to a hex token (and the mirrored template predicate is a no-op).**
  `getComputedStyle().backgroundColor` is `rgb(19, 21, 36)`, never `#131524`, so
  the poll hangs to timeout; the template predicate confirms nothing. Suggestion:
  convert the token to rgb via `parseRgb` and poll for strict equality.
- 🔵 minor (medium) — _Phase 1, Sections 3/4_ — **Hue override must use `??`, not
  `||`, or `hue={0}` (valid red) is discarded.** The prototype uses `hue || 215`;
  copying that idiom drops the boundary hue 0. Suggestion: specify `??` and add a
  `hue={0}` case to the override test.
- 🔵 minor (medium) — _Phase 3, Section 1_ — **EmptyState and BigGlyph re-resolve
  the hue independently.** Identical today via the typed Record, but a future
  panel-hue override without a matching hero override would desync. Suggestion:
  pass the resolved hue to BigGlyph, or document the deliberate delegation.

### Standards

**Summary**: The plan adheres closely to established frontend conventions: it
mirrors the 0037 Glyph component's co-located one-file-per-type layout, the
non-crumbed showcase route registration, the per-cell visual-regression
structure, the runtime-hsl PaperFold precedent, and the decorative aria-hidden
hero contract. Naming of the per-type illustration files and the component is
consistent. The main frictions are the location/naming of the shared-constants
modules and a silent fallback that departs from Glyph's documented warning
behaviour without calling out the divergence.

**Strengths**:
- One-file-per-type illustration layout faithfully mirrors the Glyph convention.
- The two prototype renames are correctly reconciled, and the dispatch
  exhaustiveness check matches the proven Glyph pattern (=== 13).
- Accessibility convention correctly preserved and tested (aria-hidden, no role).
- Showcase route follows the established non-crumbed createRoute pattern.
- Stable data-testid locator naming and per-cell clipped strategy match existing
  conventions; snapshot path/naming correctly anticipated.
- Module names avoid underscore prefixes; camelCase helper / PascalCase component
  naming consistent.

**Findings**:
- 🔵 minor (high) — _Phase 1, Section 1_ — **Shared-constants modules diverge from
  the Glyph.constants.ts convention; a constants file sits under icons/.** Glyph
  co-locates shared constants in `Glyph.constants.ts` at the component root.
  Suggestion: consolidate into `BigGlyph.constants.ts`; keep constant-only files
  out of icons/.
- 🔵 minor (high) — _Phase 1, Section 3_ — **Silent DEFAULT_BIG fallback diverges
  from Glyph's null+console.warn, undocumented.** Suggestion: call out the
  deliberate departure in the dispatch comment, optionally retaining a dev warn.
- 🔵 suggestion (medium) — _Phase 1, Section 1_ — **`white: '#FFFFFF'` (prototype
  casing) vs the codebase's lowercase `#ffffff`.** Suggestion: prefer lowercase,
  or note the verbatim-port intent.

### Usability

**Summary**: The BigGlyph API is small, well-typed, and ergonomic for its primary
call site — `<BigGlyph docType={docType} size={96} />` is clean and the internal
hue resolution removes a per-call-site burden. The main friction is divergence
from the sibling Glyph component that a developer would reasonably use as a mental
model: a different override mechanism (`hue` number vs `colorVar` string), a
silent off-union fallback where Glyph warns, and an inconsistent decision about
whether `size` is required.

**Strengths**:
- The primary call site is minimal and obvious; internal hue resolution removes a
  step the PaperFold caller previously had to do.
- Sensible default `size={96}` (the EmptyState hero-column width).
- The `hue?: number` override has clear JSDoc (range, default, 0083 consumer) —
  good progressive disclosure.
- Showcase route mirrors the established `/glyph-showcase` pattern.
- BigGlyphProps is a tight three-prop surface with no redundant members.

**Findings**:
- 🔵 minor (high) — _Phase 1, Section 3_ — **Override prop `hue?: number` diverges
  from Glyph's `colorVar?: string` in name and type.** Justified by the
  runtime-hsl architecture; suggestion: a JSDoc note contrasting the two.
- 🔵 minor (high) — _Phase 1, Section 3_ — **Off-union docType fails silently where
  Glyph emits a dev warning.** Off-union keys produce a plausible-but-wrong
  illustration with no signal. Suggestion: mirror Glyph's guarded DEV warn before
  rendering the fallback.
- 🔵 suggestion (medium) — _Phase 1, Section 3_ — **`size` optionality/typing
  differs from Glyph's required `16|24|32` union.** The looser typing is the right
  call for a scalable hero; suggestion: a brief JSDoc note documenting the
  intentional divergence.
- 🔵 suggestion (medium) — _Phase 1, Section 3 (snippet vs prose)_ — **Plan shows
  two contradictory hue-resolution expressions.** Suggestion: update the code
  block to the defensive form and name 215 as the documented blue fallback.
- 🔵 suggestion (low) — _Phase 2 / Phase 1 dispatch_ — **The showcase exercises
  only the 13 union keys, never DEFAULT_BIG.** Suggestion: optionally add one cast
  off-union cell labelled "fallback / DEFAULT_BIG".

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-10

**Verdict:** APPROVE

The plan was revised to address every finding from the initial review, then
re-reviewed fresh across all six lenses. All four prior majors are resolved. The
re-review surfaced one **new** major — a defect in the distinctness guard added by
recommended change #3 — which was then corrected in the same pass, along with the
small consequences of the Pass-1 edits (hex-casing brittleness, an overloaded
magic number, an unverified refactor claim). The plan now carries no open critical
or major findings; the residual items are minor enhancements that do not block
implementation.

### Previously Identified Issues

- 🔴 **Code Quality / Correctness**: Off-union snippet throws, contradicting prose
  & AC1 — **Resolved.** Snippet now `hue ?? DOC_TYPE_HUE[docType] ?? DEFAULT_BIG_HUE`
  with `??` semantics; the contradictory prose was removed. Correctness and Code
  Quality both confirmed snippet/prose/test now agree.
- 🟡 **Correctness / Test Coverage**: Dark-theme poll compared `rgb()` against a hex
  token (and mirrored a no-op predicate) — **Resolved.** Now compares resolved
  `rgb(19, 21, 36)` via `parseRgb` and explicitly rejects the tautological template
  predicate.
- 🟡 **Test Coverage**: Per-type distinctness had no automated guard — **Resolved,
  then re-fixed.** A guard was added in Pass 1 but the re-review found it
  defective (see new issues); it was replaced with a deterministic dispatch-
  collision check.
- 🟡 **Test Coverage**: EmptyState integration test didn't verify the correct hero
  — **Resolved.** Now renders two doc types, asserts distinct per-type hue tones
  (derived from `DOC_TYPE_HUE`) and the 96px contract.
- 🔵 **Architecture / Correctness**: `src/components/` reached up into the route
  module for the hue; hero/panel could drift — **Resolved.** `DOC_TYPE_HUE`
  extracted to `src/styles/tokens.ts`; both surfaces now resolve from one source.
- 🔵 **Code Quality / Standards / Usability**: Silent off-union fallback —
  **Resolved.** DEV `console.warn` added (and now asserted in the unit test).
- 🔵 Remaining Pass-1 minors (bigPalette JSDoc re-anchor; render-function note;
  `BigGlyph.constants.ts` relocation; `#ffffff` lowercasing; `DefaultBigGlyph`
  marker; source-walk literal guard; `hue={0}` boundary; JSDoc prop contrasts;
  0083 migration note) — **all Resolved.**

### New Issues Introduced (and addressed in this pass)

- 🔴 **Correctness** (also flagged by Architecture & Test Coverage): The Pass-1
  distinctness guard hashed rendered PNG bytes and asserted pairwise non-identity
  — but since all thirteen hues are distinct, two keys mapped to the same
  illustration function still render different bytes, so the hash never collides
  and the dispatch copy-paste it targets slips through. **Addressed**: replaced
  with a deterministic unit assertion `new Set(Object.values(BIG_GLYPHS)).size
  === 13` (referentially distinct functions); genuine visual near-duplicate
  distinctness is explicitly scoped to the design sign-off.
- 🔵 **Code Quality / Correctness**: The `#FFFFFF`→`#ffffff` lowercasing made the
  new source-walk literal guard reject a verbatim-ported `#FFFFFF`. **Addressed**:
  the guard now normalises case and matches by exact set membership against
  `Object.values(bigPalette(hue))` with per-type-scoped sanctioned constants; the
  `bigPalette` JSDoc records the lowercasing as the sole, intentional
  normalisation.
- 🔵 **Architecture / Usability**: The off-union fallback `215` was a magic number
  that collided with `templates`' real hue. **Addressed**: named `DEFAULT_BIG_HUE
  = 215` with a comment that it is independent of any doc-type hue.
- 🔵 **Correctness**: The `DOC_TYPE_HUE` extraction's value-preservation was
  unverified (no test pins the hues). **Addressed**: added a `TYPE_COPY[k].hue ===
  DOC_TYPE_HUE[k]` parity assertion across all keys.
- 🔵 **Standards / Test / Usability** minors (tokens.ts placement rationale; warn
  assertion; tests deriving hues from `DOC_TYPE_HUE`; `BigGlyphDraw` type alias;
  decorative-only JSDoc note) — **Addressed.**

### Residual (non-blocking) items

- A shared test helper (`renderBigGlyph` / `bigPaletteTones`) to de-duplicate the
  walk/tone setup across suites (Test, suggestion) — left to implementation taste.
- Making the 0083 baseline-migration obligation concrete in 0083's own body
  (Architecture, minor) — an action on a downstream work item, noted in this plan
  but actioned there.

### Assessment

The plan is now in good shape and ready for implementation. The revision cycle
was productive: it resolved all four original majors and, valuably, the re-review
caught that one of the fixes was itself unsound (the byte-hash distinctness guard)
before it reached code — which was corrected to a deterministic dispatch-collision
check. No open critical or major findings remain.

---
*Re-review generated by /accelerator:review-plan*
