---
date: "2026-05-26T17:35:00+00:00"
type: plan-review
producer: review-plan
target: "plan:2026-05-26-0084-detail-page-chip-strip-cap"
review_number: 1
verdict: APPROVE
lenses: [correctness, code-quality, test-coverage, architecture, usability, compatibility, standards]
review_pass: 2
status: complete
id: "2026-05-26-0084-detail-page-chip-strip-cap-review-1"
title: "2026-05-26-0084-detail-page-chip-strip-cap-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-26T17:35:00+00:00"
last_updated_by: Toby Clemson
---

## Plan Review: Detail-Page Chip Strip Cap (Status, Date, Author)

**Verdict:** REVISE

The plan is tightly scoped, TDD-disciplined, and confined to a single
component with clear non-goals — its structure, phase layering, and
deletion of contradicting tests are all strong. However, one critical
issue makes Phase 1 unbuildable as written: the plan explicitly asserts
that `LibraryDocView.dispatch.test.tsx` is unaffected, but that file
asserts `verdict-badge` and `result-badge` render through
`FrontmatterChips` — exactly the dispatches Phase 1 removes — so the
full frontend suite will fail at Phase 1's gating CI step. Several
major issues compound this: the Phase 2 absent-state test calls an
undefined `cssClass()` helper, the `min-height: 1lh` choice has both a
height-mismatch and a real browser-support floor (Safari 16.4+) that the
plan dismisses too quickly, the case-fold dedup has a subtle
null/undefined-collision bug, and the Phase 3 TDD-failure expectation is
self-contradictory.

### Cross-Cutting Themes

- **Undefined `cssClass` helper in Phase 2 tests** (flagged by:
  correctness, test-coverage, compatibility, code-quality) — the
  literal test code as written will fail to compile. The plan's
  parenthetical fallback (`container.querySelector('div')`) is
  fragile and lacks a stable test-id anchor. Pick one approach
  up-front: a `data-testid="frontmatter-chips"` on the wrapper div
  is the simplest fix aligned with existing project conventions.
- **`min-height: 1lh` is doubly problematic** (flagged by:
  correctness, compatibility, usability) — (a) it resolves against
  the chip-strip element's font cascade, not the chip's own
  font-size/padding, so the height likely won't visually match a
  one-chip strip; (b) `1lh` requires Safari 16.4 / Chrome 110 /
  Firefox 120 — the plan's "no browserslist override therefore
  safe" reasoning is backwards (absence ≠ evergreen-only target);
  (c) the manual verification step only checks `height > 0`, not
  visual parity. Either calibrate to chip box metrics or commit to
  a stated minimum browser target.
- **Case-fold dedup tie-break is brittle and untested** (flagged by:
  correctness, code-quality, test-coverage) — `pickCanonical`'s
  `if (!folded.has(lk)) folded.set(lk, v)` means a YAML doc with
  `{ Status: null, status: 'draft' }` silently renders no status
  chip because the first iteration wins the Map slot with `null`.
  Combined with no test pinning the collision behaviour, this is a
  latent bug with no regression armour.
- **13-kind parameterised matrix is largely tautological** (flagged
  by: test-coverage, code-quality, architecture) — `FrontmatterChips`
  has no per-kind branching, so iterating `DOC_TYPE_KEYS` × one
  extra key exercises the same code path 13 times. The plan's own
  Phase 3 prose acknowledges this. It still has value as regression
  armour, but framing should be honest.
- **Empty container is invisible whitespace with no accessibility
  affordance** (flagged by: usability, architecture, standards) —
  zero-chip pages render an empty unlabeled `div`. Future maintainers
  may "fix" it by removing it; screen-reader users get an
  undifferentiated landmark; the layout invariant lives on
  `FrontmatterChips` rather than on the slot's layout primitive.

### Tradeoff Analysis

- **Information density vs visual rest**: removing `verdict`/`result`
  badges from chips reduces chrome but demotes the most scannable
  state on review/validation pages from a coloured header chip to
  a plain table row. The plan justifies this ("two coloured tones
  compete"), but doesn't propose any compensating tone treatment in
  `FrontmatterTable`. Recommendation: confirm the trade-off is
  acceptable during Phase 1 manual verification by attempting to
  scan a plan-review's verdict at-a-glance; if it's noticeably
  worse, capture a follow-up for tone-coloured table rows.
- **Consistency vs flexibility (no caller-opt-in prop)**: hardcoding
  the whitelist is the right call for a single-consumer component,
  but the rejection rationale is mechanical. Naming the architectural
  principle ("variation belongs at the component-boundary level —
  introduce a new component, don't parameterise this one") pre-empts
  re-litigation when a future consumer appears.

### Findings

#### Critical

- 🔴 **Test Coverage**: LibraryDocView.dispatch.test.tsx will break — plan's claim it is unaffected is wrong
  **Location**: Testing Strategy → Integration Tests; Phase 1 Success Criteria
  The plan states the integration test's assertions "don't depend on chip count or content", but the file asserts `[data-testid="verdict-badge"]` and `[data-testid="result-badge"]` render with specific `data-variant` values for plan-reviews, work-item-reviews, and validations — exactly the dispatches Phase 1 removes. Phase 1's "full frontend test suite still passes" gate cannot be met; the implementer will discover the breakage only at CI time.

#### Major

- 🟡 **Correctness**: First-match-wins dedup can discard a valid value when a case-variant duplicate stores null/undefined first
  **Location**: Phase 1, Implementation Changes: `pickCanonical`
  `if (!folded.has(lk)) folded.set(lk, v)` followed by a null/undefined filter means `{ Status: null, status: 'draft' }` (or any case-variant pair with null-first) silently renders no chip. Fix by skipping null/empty during dedup so a later non-null variant wins, or preferring the exact-canonical key.

- 🟡 **Correctness / Test Coverage / Compatibility**: Phase 2 test code references an undefined `cssClass` helper
  **Location**: Phase 2, Test-First Changes: replacement `'absent state'` test
  The literal test snippet uses `` container.querySelector(`.${cssClass('chips')}`) `` but no `cssClass` helper exists in the codebase. The plan's parenthetical fallback is fragile (selecting `'div'` collides with `'malformed'`'s banner div, and `firstChild` couples to JSDOM tree position). Commit to a stable approach in the plan body, ideally a `data-testid="frontmatter-chips"` on the wrapper div.

- 🟡 **Compatibility**: `1lh` browser-support floor (Safari 16.4 / Chrome 110 / Firefox 120) not concretely justified
  **Location**: Phase 2: Empty-Container Height Preservation; Key Discoveries
  The plan reasons "no browserslist override therefore safe", but absence of a target ≠ evergreen-only. Vite 6's default `modules` target spans browsers from 2018. On Safari 16.3, Firefox ESR 115, and older Chromium builds, `min-height: 1lh` is dropped and the slot collapses — silently regressing exactly what AC 5 fixes. Either declare a minimum target in the plan, codify a browserslist, or use a calc/em fallback.

- 🟡 **Correctness / Test Coverage**: Phase 3's TDD-failure expectation is internally contradictory
  **Location**: Phase 3, Test-First Changes: 'Run the suite' paragraph
  The paragraph says the 13-kind parameterised assertions "should fail" then immediately admits they "should pass if Phase 1 is in place". The contradiction is resolved one paragraph later but mid-paragraph hedging confuses TDD ordering. Reframe the parameterised test as regression armour added in Phase 3 (not a failing-first test), and tie the Phase 3 red step only to the whitespace-trim case.

- 🟡 **Test Coverage**: Case-fold tie-breaker has no regression test
  **Location**: Phase 1, Implementation Changes: `pickCanonical`
  Combined with the null-collision bug above, the tie-break behaviour is invisible to all tests. Add one assertion: `{ Status: 'first', status: 'second' }` produces `aria-label === 'status: first'` (or whatever the corrected dedup rule yields).

- 🟡 **Test Coverage**: Non-string canonical values lack assertion coverage
  **Location**: Phase 1, Test-First Changes
  Tests fixture-supply strings only, but `IndexEntry.frontmatter` is `Record<string, unknown>` and YAML parsers commonly emit Date objects for ISO dates. `String(new Date('2026-04-05'))` produces a `Tue Apr 05 2026 ...` string, which would render ugly chip text without any test failing. Add a smoke test for `date: new Date(...)` and `author: ['Alice', 'Bob']`, or document the type assumption in a code comment.

- 🟡 **Usability**: Empty chip strip is indistinguishable from a layout bug
  **Location**: Phase 2 — `min-height: 1lh` rationale
  When all three canonical keys are absent, the page renders invisible whitespace between H1 and divider. To an end user this looks identical to a broken layout. Either emit a low-visibility placeholder (dimmed em-dash chip / "no metadata" text), or at minimum add an `aria-hidden="true"` + a CSS comment justifying the empty slot as deliberate vertical-rhythm preservation.

- 🟡 **Usability**: 2-chip vs 3-chip asymmetry across half the corpus is not visually verified
  **Location**: Overview / What We're NOT Doing — schema-alignment deferral
  Six of twelve doc kinds will render only 2 chips because their templates lack `author`. The follow-up schema-alignment story has no ID. None of the manual verification steps compare a 2-chip strip side-by-side with a 3-chip strip to confirm the asymmetry is acceptable. Add a comparison step (work-item with all 3 chips vs its plan with 2 chips), and capture the follow-up story ID before this lands.

#### Minor

- 🔵 **Correctness**: `1lh` resolves to the subtitle's line-height, not the chip's rendered height
  **Location**: Phase 2, Implementation Changes
  The chip uses smaller font-size + padding + border; `1lh` of the subtitle cascade is a different value. The "same vertical height as a one-chip strip" claim may be visually close but not exact. Either accept the approximation (likely <2px) and document it, or calibrate to chip box metrics.

- 🔵 **Code Quality**: BADGE_FOR_KEY and BadgeProps are over-abstracted for a single entry
  **Location**: Phase 1, Implementation Changes
  After narrowing, the map has one entry. Inline the status dispatch (`if (key === 'status') return <StatusBadge .../>`) and delete `BADGE_FOR_KEY`, `badgeFor`, `BadgeProps`. The current shape signals an extensibility surface the whitelist actively forecloses.

- 🔵 **Code Quality**: Case-fold Map iterates the whole frontmatter when only three keys are needed
  **Location**: Phase 1, Implementation Changes
  A direct `frontmatter[key]` lookup with a case-insensitive fallback would make the tie-break explicit at the call site instead of relying on Map insertion order. The `// First match wins.` comment is a code-smell signal — the comment carries weight the code should bear directly.

- 🔵 **Code Quality**: `Array<readonly [string, unknown]>` return type is heavier than needed
  **Location**: Phase 1, Implementation Changes — `pickCanonical` return type
  Either drop `readonly` (the array is local and consumed once) or switch to an object-shape element type (`Array<{ key: string; value: unknown }>`) for a self-documenting call site.

- 🔵 **Test Coverage**: Empty-container test does not pin min-height presence in DOM
  **Location**: Phase 2 — Test-First Changes
  The container-present test and the CSS-source `min-height: 1lh` regex are disjoint — a refactor renaming `styles.chips` would render an empty `<div>` with no min-height while both tests pass. Import the CSS module hash and assert `className === styles.chips` on the container.

- 🔵 **Architecture**: Canonical key list duplicates schema authority inside a presentation component
  **Location**: Phase 1, Implementation Changes — `CANONICAL_KEYS`
  `{status, date, author}` are ADR-0033 base-schema fields, but they're hardcoded in `FrontmatterChips.tsx` with no comment pointing at the authority. Add a one-line comment citing ADR-0033 §Base schema so the upcoming schema-alignment follow-up can find this site.

- 🔵 **Architecture**: Subtitle slot height invariant lives on chip strip, not the slot's layout primitive
  **Location**: Phase 2, Implementation Changes
  `FrontmatterChips` quietly depends on `.subtitle`'s `line-height: 1.5` for `1lh` to resolve correctly. The cross-component contract is implicit. Add a comment near the `min-height` declaration referencing the inheritance source; flag as known debt for moving the slot reservation onto `Page.module.css`'s `.subtitle`.

- 🔵 **Architecture / Code Quality**: Reducing dispatch to a single entry weakens the abstraction's justification
  **Location**: Phase 1 — `BADGE_FOR_KEY` narrowed to status only
  Either inline (preferred — matches the closed-set design), or keep the map and add a comment naming it as a deliberate extension point.

- 🔵 **Usability**: Coloured verdict/result badges demoted to plain table rows is a real visual loss
  **Location**: Phase 1 — removal of `verdict`/`result` from dispatch
  Review/validation readers lose at-a-glance verdict scannability. Confirm during Phase 1 manual verification that a reviewer can still scan a plan-review's verdict quickly; if not, capture a follow-up to teach `FrontmatterTable` tone-coloured rows.

- 🔵 **Usability**: Silent omission of whitespace-only `author` gives no developer feedback
  **Location**: Phase 3 — whitespace-trim
  A frontmatter author who accidentally writes `author: "   "` sees the chip disappear with no signal. Consider a dev-mode `console.warn`, or document the silent behaviour in a JSDoc on `pickCanonical`.

- 🔵 **Usability**: Manual verification doesn't exercise the visual "same height" claim
  **Location**: Phase 2 — Manual Verification
  Current step only asks for `height > 0`. Replace with side-by-side H1-to-divider distance comparison between a zero-chip and one-chip page.

- 🔵 **Compatibility**: `NON_CANONICAL_PER_KIND` is non-exhaustive over `DOC_TYPE_KEYS` at the type level
  **Location**: Phase 3
  Type as `Record<DocTypeKey, [string, unknown]>` so adding a doc kind forces a fixture entry (catching the omission at typecheck instead of runtime destructuring error).

- 🔵 **Standards**: Empty rendered container has no accessible role or aria-hidden treatment
  **Location**: Phase 2
  The codebase pattern is labelled wrapper elements (`aria-label="Document metadata"` on `FrontmatterTable`). Either add `aria-hidden="true"` when empty, or a code comment explaining the deliberate omission.

- 🔵 **Standards**: Parameterised test fixture conflates DocTypeKey plurals with ADR-0033 artifact-type extras
  **Location**: Phase 3 — `NON_CANONICAL_PER_KIND`
  Some picks don't match ADR-0033's per-type extras (e.g. `adr` for decisions; `last_updated_by` is a base field, not type-specific). Either source extras from the ADR or add a comment that picks are deliberately illustrative.

#### Suggestions

- 🔵 **Code Quality**: Keep `pickCanonical` co-located rather than extracting it (~15 lines, one consumer).
- 🔵 **Code Quality**: 13-kind parameterised matrix is partially tautological after Phase 1 — reframe as regression armour, or drop the kind axis.
- 🔵 **Code Quality**: `// First match wins.` comment is doing work code structure could bear; consider restructuring or renaming the variable.
- 🔵 **Correctness**: Frontmatter values that are Date objects/numbers/non-string types are not explicitly handled — verify serde or add a guard.
- 🔵 **Architecture**: Caller-opt-in rejection is correct but the rationale should name the architectural force (boundary-level variation, not prop-level).
- 🔵 **Architecture**: Per-kind verification asserts uniform behaviour but the component has no per-kind branch — document framing.
- 🔵 **Usability**: No prop escape hatch — add a JSDoc on `CANONICAL_KEYS` ("build a new component, don't parameterise this one") so the constraint is discoverable.
- 🔵 **Standards**: Canonical-key lowercasing changes aria-label output for case-variant frontmatter (`Date: ...` → `date: ...`). Call this out in the plan's Notes so reviewers don't read it as accidental.
- 🔵 **Standards**: `DOC_TYPE_LABELS` available but unused in parameterised test description.
- 🔵 **Standards**: `// First match wins.` is consistent with codebase style but slightly redundant — optional trim.
- 🔵 **Standards**: Migration Notes are correct but could note explicitly that `verdict`/`priority`/`tags` migrate to the `FrontmatterTable` surface.

### Strengths

- ✅ Scope is tightly bounded: one component, one CSS module, one test file — no caller churn.
- ✅ TDD-first phasing with explicit "should fail" callouts after each new test block, and clear ownership of AC subsets per phase.
- ✅ Each phase is independently reviewable and shippable; the layered diffs swap clearly identified slabs (loop body, early returns, predicate).
- ✅ Canonical whitelist `{status, date, author}` aligns exactly with ADR-0033's base-schema field names — no schema drift.
- ✅ Rejection of caller-opt-in props (`keys`/`include`/`exclude`/`renderChip`) is deliberate and well-justified for a single-consumer component.
- ✅ Explicit list of retained vs deleted tests in the Testing Strategy section makes the final test composition auditable.
- ✅ Plan correctly identifies that the existing source-order and AC-integration tests directly contradict the new contract and must be deleted.
- ✅ Component boundaries are well-respected: explicit non-changes to Page, LibraryDocView, FrontmatterTable, StatusBadge, Chip.
- ✅ Keeping `VerdictBadge`/`ResultBadge` components but removing only their dispatch entries preserves the open/closed boundary.
- ✅ aria-label format `status: draft` (lowercase, colon-space) is consistent with established convention in sibling components.
- ✅ `?raw` CSS source-assertion pattern is an established project pattern, faithfully reused.
- ✅ Parameterised `it.each` tests for whitelist exclusion and subset ordering provide broad cheap regression armour.
- ✅ Date/author precedence over `last_updated*` mirrors is locked in with dedicated tests, addressing two ACs that are easy to miss.
- ✅ Migration story is explicit and correct — no data migration, no shim, `FrontmatterTable` continues to surface non-chip keys.

### Recommended Changes

Prioritised by impact:

1. **Plan an explicit fate for `LibraryDocView.dispatch.test.tsx`**
   (addresses: critical "LibraryDocView.dispatch.test.tsx will break")
   Add to Phase 1: either delete the three failing tests (and verify
   the verdict→variant / result→variant tone contracts are covered at
   the `VerdictBadge`/`ResultBadge` unit-test layer; add unit tests if
   not), or rewrite them to no longer pass through `FrontmatterChips`.
   This is gating: Phase 1 cannot meet its "full suite passes" success
   criterion until this is resolved.

2. **Fix the case-fold dedup null-collision bug + add a regression test**
   (addresses: "First-match-wins dedup can discard valid values",
   "Case-fold tie-breaker has no regression test")
   Change `pickCanonical` to skip null/undefined/empty values during
   the Map dedup pass (don't insert them at all), so a later non-null
   variant wins. Add a one-line test: `{ Status: null, status: 'draft' }`
   produces a status chip.

3. **Commit to a concrete Phase 2 test approach for the empty container**
   (addresses: "Phase 2 test code references an undefined cssClass
   helper", "Empty-container test does not pin min-height presence")
   Add `data-testid="frontmatter-chips"` to the wrapper `<div>` in
   `FrontmatterChips.tsx` and select via that. Drop the `cssClass`
   prose. Also import the CSS module hash and assert
   `className === styles.chips` so the two halves of the AC-5 contract
   are wired together.

4. **Address `1lh` properly — both the height-match claim and the
   browser-support floor**
   (addresses: "`1lh` resolves to subtitle's line-height", "`1lh`
   support floor not concretely justified")
   Either (a) declare a minimum browser target in the plan (e.g.
   "internal tool, evergreen Safari 16.4+/Chrome 110+/Firefox 120+
   only") and accept the risk; or (b) use a calc/em fallback derived
   from chip box metrics. Update Phase 2 manual verification to
   compare H1-to-divider distance side-by-side between zero-chip
   and one-chip pages, not just `height > 0`.

5. **Reframe the Phase 3 TDD-failure narrative**
   (addresses: "Phase 3 TDD-failure expectation contradictory")
   Rewrite the "Run the suite" paragraph as: "If Phase 3 runs after
   Phase 1, only the whitespace-trim test fails (red). If Phase 3
   runs standalone before Phase 1, the parameterised tests also fail
   (red). Either ordering is valid TDD; the parameterised tests
   double as regression armour against future relaxation of the
   whitelist." Drop the mid-paragraph "— actually" correction.

6. **Add a smoke test for non-string canonical values**
   (addresses: "Non-string canonical values lack assertion coverage")
   At least `date: new Date('2026-04-05')` (YAML date) and
   `author: ['Alice', 'Bob']` (co-authored docs). Or document the
   string-only assumption in a `pickCanonical` comment.

7. **Decide the empty-container UX explicitly**
   (addresses: "Empty chip strip indistinguishable from layout bug",
   "Empty rendered container has no accessible role")
   Either emit a low-visibility placeholder, or add a CSS comment
   justifying the deliberately-blank slot plus `aria-hidden="true"`
   when empty.

8. **Verify the 2-chip vs 3-chip visual asymmetry**
   (addresses: "2-chip vs 3-chip asymmetry not visually verified")
   Add a side-by-side manual verification step in Phase 1 (open a
   work-item and its plan in two tabs) and capture the schema-alignment
   follow-up story ID before this work lands.

9. **Simplify the dispatch — inline status, delete the map**
   (addresses: "BADGE_FOR_KEY and BadgeProps over-abstracted")
   With one entry remaining, `if (key === 'status') return <StatusBadge
   value={value} />` is honest to the closed-set design. Delete
   `BADGE_FOR_KEY`, `badgeFor`, `BadgeProps`.

10. **Tighten types on `NON_CANONICAL_PER_KIND`**
    (addresses: "non-exhaustive over DOC_TYPE_KEYS at the type level")
    Type as `Record<DocTypeKey, [string, unknown]>` so future doc-kind
    additions force a fixture entry at typecheck time.

11. **Add ADR-0033 cross-reference next to `CANONICAL_KEYS`**
    (addresses: "Canonical key list duplicates schema authority")
    One-line comment is sufficient for this story.

---
*Review generated by /review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan's core logic — case-folded canonical-key extraction, fixed ordering, whitelist enforcement, and whitespace-trim filtering — is generally sound and the chip aria-label format (`status: draft` etc.) correctly matches the test expectations via `FrontmatterChip`'s `${name}: ${text}` template and `StatusBadge`'s hardcoded `name="status"`. However, three correctness issues warrant attention: a subtle interaction between Map-based dedup and null/undefined filtering when duplicate case-variant keys collide, a Phase 2 test fixture that references a non-existent `cssClass` helper, and a claim that `min-height: 1lh` exactly matches the rendered chip height when the chip uses a different font-size cascade. The phase-ordering 'fail/pass' commentary in Phase 3 is internally contradictory in prose but resolved later.

**Strengths**:
- `pickCanonical` correctly enforces canonical order by iterating `CANONICAL_KEYS` rather than input frontmatter.
- Whitespace-trim predicate short-circuits for non-string values.
- aria-label assertions verifiable against `FrontmatterChip`'s template and `StatusBadge`'s hardcoded name.
- `pickCanonical` uses canonical lowercase key when constructing tuples — chip names guaranteed lowercase regardless of input casing.
- `Page.tsx`'s `subtitle !== undefined` gate correctly identified.

**Findings**:
- 🟡 **Major / high**: First-match-wins dedup can discard a valid value when a case-variant duplicate stores null/undefined first.
- 🟡 **Major / high**: Phase 2 test code references an undefined `cssClass` helper.
- 🔵 **Minor / medium**: `1lh` resolves to subtitle's line-height, not chip's rendered height.
- 🔵 **Suggestion / high**: Phase 3 "should fail" commentary is internally contradictory.
- 🔵 **Suggestion / medium**: Date objects / non-string values not explicitly handled.

### Code Quality

**Summary**: The plan is well-scoped, TDD-disciplined, and proportional. `pickCanonical` is readable and pure; layered phase structure keeps diffs reviewable. Main concerns are over-abstraction (BADGE_FOR_KEY map with one entry; custom BadgeProps interface) and the case-fold Map performing more work than the contract requires.

**Strengths**:
- Tight bounded scope; one component, one CSS module, one test file.
- `pickCanonical` is a small pure function with clear name and single responsibility.
- `CANONICAL_KEYS` uses `as const` for type-level guarantee.
- YAGNI invoked explicitly to reject caller-opt-in props.
- `min-height: 1lh` avoids magic numbers (though see correctness/compat lenses for caveats).
- Phases layered to swap clearly identified slabs of code.

**Findings**:
- 🔵 **Minor / high**: `BADGE_FOR_KEY` and `BadgeProps` over-abstracted for a single entry.
- 🔵 **Minor / high**: Case-fold Map iterates whole frontmatter when only three keys needed.
- 🔵 **Minor / medium**: `Array<readonly [string, unknown]>` return type heavier than needed.
- 🔵 **Suggestion / medium**: Keep `pickCanonical` co-located (don't extract).
- 🔵 **Suggestion / medium**: Test sketch hand-waves a CSS-module class resolver.
- 🔵 **Suggestion / medium**: Parameterised matrix is partially tautological after Phase 1.
- 🔵 **Suggestion / high**: `// First match wins.` comment is doing work the code structure should bear.

### Test Coverage

**Summary**: Thorough TDD-first test layering with rigorous coverage for canonical ordering, whitelist exclusion, subset ordering, last_updated precedence, whitespace trimming, and CSS source assertions. However, the plan's claim that `LibraryDocView.dispatch.test.tsx` is unaffected is factually wrong — it asserts verdict/result badges this work removes. Additional concerns: `cssClass` helper undefined, case-fold tie-breaker not regression-locked, non-string canonical values not covered.

**Strengths**:
- Strict TDD ordering with explicit "should fail" callouts.
- Parameterised `it.each` tests for whitelist exclusion (11 keys) and 13-kind corpus.
- Subset-ordering covers the source-order-reversed permutation.
- `date`/`author` precedence over `last_updated*` directly tested.
- CSS source regex provides lightweight contract on styling.
- Explicit retained-vs-deleted test inventory.
- Correctly identifies contradicting tests for deletion.

**Findings**:
- 🔴 **Critical / high**: `LibraryDocView.dispatch.test.tsx` will break — plan's claim is wrong.
- 🟡 **Major / high**: Phase 3 TDD-failure expectation is incorrect and silently weakens the test.
- 🟡 **Major / high**: `cssClass` helper undefined; documented fallback fragile.
- 🟡 **Major / high**: Case-fold tie-breaker ("first match wins") has no regression test.
- 🟡 **Major / medium**: Non-string canonical values lack assertion coverage.
- 🔵 **Minor / high**: CSS regex requires opening brace on same line as selector (low risk, but consider tightening).
- 🔵 **Minor / medium**: Templates entry adds little signal; fixture realism could be tightened.
- 🔵 **Minor / medium**: Narrowed dispatch test loses signal on neutral variant.
- 🔵 **Minor / medium**: Empty container test does not pin min-height presence in DOM.

### Architecture

**Summary**: Tightly scoped, well-bounded change to a single component with clear contract and explicit non-goals. Strengths real: sole-consumer simplicity, ADR-0033 alignment, deliberate non-change to Page/LibraryDocView/FrontmatterTable. But embeds the canonical key list as a magic constant inside a presentation component rather than co-locating with the schema authority, and pushes a layout invariant onto the chip strip rather than the layout primitive owning the slot.

**Strengths**:
- Component boundaries well-respected.
- Rejecting caller-opt-in props matches single-consumer reality.
- Whitelist consistent with ADR-0033 base schema.
- Keeping VerdictBadge/ResultBadge preserves open/closed boundary.
- TDD-first phasing with self-contained AC subsets per phase.
- Plan acknowledges chip/table duplication tradeoff explicitly.

**Findings**:
- 🔵 **Minor / high**: Canonical key list duplicates schema authority inside presentation component.
- 🔵 **Minor / high**: Subtitle slot's height invariant lives on chip strip, not the layout primitive.
- 🔵 **Minor / medium**: Caller-opt-in rejection rationale should name the architectural force.
- 🔵 **Minor / medium**: Reducing dispatch to a single entry weakens the abstraction's justification.
- 🔵 **Suggestion / medium**: Per-kind verification asserts uniform behaviour with no per-kind branch.

### Usability

**Summary**: Well-structured for the implementing developer (TDD-first, file-scoped, clear). UX-wise, several user-facing trade-offs are implicit: invisible empty container, removal of coloured verdict/result badges, 2-chip vs 3-chip asymmetry affecting half the doc kinds. Manual verification steps don't exercise the resulting visual asymmetry.

**Strengths**:
- Each phase has explicit Manual Verification steps tied to real doc kinds.
- Hardcoded whitelist is a deliberate, well-justified DX choice.
- Canonical left-to-right order makes strip positionally scannable.
- Verdict/result remain in `FrontmatterTable` — demoted but not lost.
- Whitespace-trim closes a footgun.

**Findings**:
- 🟡 **Major / high**: Empty chip strip indistinguishable from layout bug.
- 🟡 **Major / high**: 2-chip vs 3-chip asymmetry not visually verified.
- 🔵 **Minor / high**: Coloured verdict/result badges demoted to plain table rows is a real visual loss.
- 🔵 **Minor / medium**: Silent omission of whitespace-only `author` gives no developer feedback.
- 🔵 **Minor / medium**: Manual verification doesn't exercise the "same height as one-chip strip" claim.
- 🔵 **Minor / medium**: `1lh` resolves against chip-strip's own cascade, not subtitle's inherited 1.5 — visual claim may not hold.
- 🔵 **Suggestion / medium**: No prop escape hatch — add JSDoc making constraint discoverable.

### Compatibility

**Summary**: Mostly sound for an internal dev tool: React 19, TS 5, Vitest 3, jsdom 26 all up-to-date; new test patterns mirror existing project conventions. Material concerns: `1lh` browser-support floor (Safari 16.4 / Chrome 110 / Firefox 120) not concretely justified, and `cssClass()` helper doesn't exist.

**Strengths**:
- Tests assert CSS via regex against raw text — JSDOM `1lh` support irrelevant.
- `it.each(DOC_TYPE_KEYS)` works with readonly arrays.
- TS 5/React 19 versions support all the new syntax.
- Existing CSS-module-class hash assertions already use the raw-CSS pattern.

**Findings**:
- 🟡 **Major / high**: `1lh` support floor not concretely justified.
- 🔵 **Minor / high**: `cssClass('chips')` helper does not exist.
- 🔵 **Minor / medium**: `NON_CANONICAL_PER_KIND` lookup non-exhaustive over `DOC_TYPE_KEYS` at the type level.

### Standards

**Summary**: Adheres well to component conventions: co-located files, lowercase-key aria-label format, `?raw` CSS source assertion pattern. Canonical whitelist aligns with ADR-0033. Concerns: small accessibility risk in always-rendered empty container, two test-fixture inaccuracies (DocTypeKey plurals vs ADR-0033 per-type extras).

**Strengths**:
- Canonical whitelist matches ADR-0033 base schema exactly.
- aria-label format consistent with sibling components.
- File co-location preserved.
- `?raw` import is an established project pattern.
- Code comments consistent with existing style.
- `data-testid` wrapper assertions follow the Chip primitive's data-* drop constraint.
- Manual verification walks WCAG-relevant visual layout.

**Findings**:
- 🔵 **Minor / high**: Parameterised test fixture conflates DocTypeKey (plural) with ADR-0033 artifact-type extras.
- 🔵 **Minor / high**: Empty rendered container has no accessible role or aria-hidden treatment.
- 🔵 **Suggestion / medium**: Canonical-key lowercasing changes aria-label output for case-variant frontmatter.
- 🔵 **Suggestion / medium**: `DOC_TYPE_LABELS` available but unused in parameterised test.
- 🔵 **Suggestion / low**: Inline `// First match wins.` comment slightly redundant.
- 🔵 **Suggestion / high**: Migration Notes correct but could explicitly note `verdict`/`priority`/`tags` migrate to `FrontmatterTable`.

## Re-Review (Pass 2) — 2026-05-26T17:35:00+00:00

**Verdict:** APPROVE

### Previously Identified Issues

**Critical (1)**
- ✅ **Test Coverage**: `LibraryDocView.dispatch.test.tsx` will break — **Resolved**. Phase 1 now explicitly deletes the file, with documented verification that the verdict→variant and result→variant tone contracts remain covered by `VerdictBadge.test.tsx` and `ResultBadge.test.tsx` (confirmed independently during re-review).

**Major (8)**
- ✅ **Correctness**: First-match-wins dedup discards valid values on null-collision — **Resolved**. `pickCanonical` now skips null/undefined/whitespace-only values *during* the Map fold pass, so a skipped variant never claims the slot. Locked in by the new `case-fold dedup precedence` describe.
- ✅ **Correctness / Test Coverage / Compatibility / Code Quality**: `cssClass()` helper undefined — **Resolved**. All Phase 2 tests use the `data-testid="frontmatter-chips"` anchor and `toHaveClass(styles.chips)` via the standard CSS-module default import.
- ✅ **Compatibility**: `1lh` browser-support floor not concretely justified — **Resolved (accepted with mitigation)**. Plan now declares evergreen-only target explicitly with a calc-fallback escape hatch documented.
- ✅ **Correctness / Test Coverage**: Phase 3 TDD-failure expectation contradictory — **Resolved**. Phase 3 is reframed as regression armour (test-only, no production code); both orderings explicitly valid.
- ✅ **Test Coverage**: Case-fold tie-breaker has no regression test — **Resolved**. New `case-fold dedup precedence` describe with two assertions (null-collision + first-non-skipped-wins).
- ✅ **Test Coverage**: Non-string canonical values lack assertion coverage — **Partially resolved**. Smoke test added for Date and array values, but the inline comment misrepresents the formatting rule (see new finding below). Test passes due to loose `/^date: /` regex.
- ✅ **Usability**: Empty chip strip indistinguishable from layout bug — **Resolved (accepted mitigation)**. `aria-hidden="true"` added on empty state with code/CSS comments explaining the deliberate spacer intent.
- ✅ **Usability**: 2-chip vs 3-chip asymmetry not visually verified — **Resolved**. Phase 1 manual verification step adds side-by-side comparison; a related verdict-scannability step added too.

### New Issues Introduced

All new issues are minor or suggestion-level and do not block implementation.

- 🔵 **Correctness / Test Coverage / Compatibility (cross-cutting minor)**: Non-string smoke test comment misrepresents `FrontmatterChip`'s Date formatting. The comment says `String() for Date` but `FrontmatterChip.formatChipValue` routes non-array objects through `JSON.stringify(value)`, so the rendered label is `date: "2026-04-05T00:00:00.000Z"` (literal quotes), not `date: 2026-04-05T00:00:00.000Z`. The assertion `expect(labels[0]).toMatch(/^date: /)` passes regardless, so the test is loose enough that a future refactor of `formatChipValue` would not be caught.
  **Recommendation**: Tighten the assertion to the exact rendered value and correct the comment to say `JSON.stringify` for non-array objects.

- 🔵 **Test Coverage (minor)**: First-non-skipped-wins test only exercises uppercase-first ordering (`{ Status: 'first', status: 'second' }`). A symmetric assertion for lowercase-first would pin the contract in both directions cheaply.

- 🔵 **Code Quality (suggestion)**: `aria-hidden={isEmpty ? true : undefined}` reads awkwardly. A future maintainer might "simplify" to `aria-hidden={isEmpty}` which would render `aria-hidden="false"` on populated strips. Consider a one-line code comment noting the `undefined → React omits attr` semantics.

- 🔵 **Code Quality (suggestion)**: Map fold still iterates the whole frontmatter — justified by the dedup correctness requirement, but the rationale could be added to the existing comment so a future optimiser doesn't rewrite as a three-key targeted lookup.

- 🔵 **Code Quality (suggestion)**: `k.trim().toLowerCase()` silently trims key whitespace. Either intentional (add a test) or incidental (drop the `.trim()`); document the choice.

- 🔵 **Code Quality (minor)**: Phase 1 vs Phase 2 JSX snippets duplicate the return block with small deltas. Consider showing Phase 2 as a unified diff or annotating the delta inline.

- 🔵 **Architecture (suggestion)**: ADR-0033 cross-reference is a code comment, not a structural link. Consider adding a mirror reference in ADR-0033 itself naming `FrontmatterChips.CANONICAL_KEYS` as a downstream projection. Optional follow-up.

- 🔵 **Architecture (suggestion)**: `aria-hidden` boundary inside FrontmatterChips is appropriate but adds "subtitle slot reserver" to the component's responsibility. Consider naming this in a JSDoc.

- 🔵 **Usability (suggestion)**: Empty spacer has no DOM-level breadcrumb beyond `aria-hidden="true"` — a developer inspecting devtools sees an empty div without contextual hint. Consider an additional `data-empty="true"` or `data-state="empty-spacer"` attribute. Optional.

- 🔵 **Usability (suggestion)**: 2-chip vs 3-chip asymmetry manual step lacks a concrete tolerance. Either add a threshold (e.g. ">25% width difference") or explicitly note the gate is intentionally subjective.

### Assessment

**The plan is ready for implementation.** The critical issue and all eight major findings from the initial review have been resolved or explicitly accepted with mitigations the user chose. The phase structure is now coherent (whitespace-trim folded into Phase 1's dedup pass for correctness; Phase 3 honestly framed as regression armour, not a TDD red), production code is simpler (status dispatch inlined, `BADGE_FOR_KEY`/`badgeFor`/`BadgeProps` deleted), tests are properly anchored (`data-testid="frontmatter-chips"` + `toHaveClass(styles.chips)`), and explicit verification steps cover the user-facing tradeoffs (verdict scannability, 2-chip vs 3-chip asymmetry, H1-to-divider parity).

The one item worth fixing during implementation rather than deferring is the smoke-test Date comment — it's a code-comment defect that will be copy-pasted verbatim and could mislead. Tightening the assertion to the actual rendered value (`'date: "2026-04-05T00:00:00.000Z"'`) is a one-line change.

All other new findings are pure suggestions and can be addressed during code review or deferred to follow-up.
