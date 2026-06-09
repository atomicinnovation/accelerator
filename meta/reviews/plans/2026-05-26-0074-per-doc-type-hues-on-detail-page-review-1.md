---
date: "2026-05-26T18:30:00Z"
type: plan-review
producer: review-plan
target: "plan:2026-05-26-0074-per-doc-type-hues-on-detail-page"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability]
review_pass: 6
status: complete
id: "2026-05-26-0074-per-doc-type-hues-on-detail-page-review-1"
title: "2026-05-26-0074-per-doc-type-hues-on-detail-page-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-26T18:30:00Z"
last_updated_by: Toby Clemson
---

## Plan Review: 0074 — Per-Doc-Type Hues on Detail Page

**Verdict:** REVISE

The plan is well-structured (six independent phases, strict TDD cycle,
thorough pre-implementation reconciliation with code) and rests on a sound
central decision — funnel all per-doc-type tinting through the existing
`<Glyph>` component. However, six lenses converged on a tight cluster of
correctness, architecture, and standards issues that should be resolved
before implementation: the Phase 1 widening of `GlyphDocTypeKey` inverts a
documented invariant, silently changes HubCard rendering, leaves the
existing `Glyph.test.tsx` suite red, and reduces `isGlyphDocTypeKey` to
dead code; the new visual-regression specs have several wrong selectors
(framed-wrapper-vs-svg, unframed `[data-doc-type]` resolving directly to
the svg), brittle hardcoded baselines, and a factual error about dark-theme
behaviour (`--ac-fg-muted` does not collapse to white).

### Cross-Cutting Themes

These issues were flagged by multiple lenses and deserve the most attention:

- **`isGlyphDocTypeKey` becomes dead code after Phase 1**
  (flagged by: architecture, code-quality, correctness, standards, usability)
  — Plan widens `GlyphDocTypeKey = DocTypeKey` then keeps the guard "for
  diff minimisation" / "type narrowing", but it no longer narrows anything.
  Misleading name, propagated across 4-5 callsites.
- **Phase 1 silently changes HubCard rendering on the overview hub**
  (flagged by: architecture, code-quality, usability)
  — `HubCard` gates Glyph rendering on `isGlyphDocTypeKey`; after Phase 1
  it starts rendering a templates Glyph. Plan acknowledges this in a Phase
  6 parenthetical but the work item's AC #3 says "hub consumption is
  unchanged".
- **Existing `Glyph.test.tsx` invariants encode the 12-key contract**
  (flagged by: test-coverage, correctness, standards)
  — `@ts-expect-error` on `<Glyph docType="templates">`, length-12 assertion,
  `isGlyphDocTypeKey('templates') === false`, exclusion-from-DOC_TYPE_KEYS
  filter. All will fail at typecheck/vitest after Phase 1; plan only says
  "extend".
- **Phase 5 row container invariance baseline is wrong**
  (flagged by: architecture, code-quality, test-coverage, correctness, standards)
  — Hardcoded "browser defaults" (`rgba(0,0,0,0)`, `0px`, "inherited
  foreground") are guesses. The real declared/inferred border styling lives
  on `.group` (the wrapping div) not on `.groupItem`. Border-color when
  `border-width: 0` resolves to `currentColor`, theme-dependent.
- **Phase 5/6 locators target the wrong DOM element**
  (flagged by: test-coverage, correctness, standards)
  — Unframed Glyph puts `data-doc-type` on the `<svg>` itself, so
  `row.locator('[data-doc-type]').locator('svg')` fails. Framed Glyph puts
  it on both wrapper `<span>` AND `<svg>`; `.first()` picks the span,
  whose `color` is inherited (wrong value).
- **Phase 5 aria-hidden vs ariaLabel="" snippet is internally contradictory**
  (flagged by: code-quality, test-coverage, standards, usability)
  — Code snippet writes `ariaLabel=""`; prose immediately below says omit
  for `aria-hidden`. A literal copy-paste ships the accessibility
  anti-pattern.
- **Shared `EXPECTED_COLOR` / helper extraction left half-specified**
  (flagged by: code-quality, test-coverage, usability)
  — Phase 4 invites duplication; Phase 5 mandates extraction; Phase 6 just
  references it. Three callers need the same lookup — extract once, up front.
- **Framed templates Glyph silently degrades to default frame background**
  (flagged by: architecture, code-quality, correctness)
  — In dark theme `--ac-bg-sunken` is `#070b12` against a `#a0a5b8`
  templates icon, visibly inconsistent with neighbouring framed glyphs.
- **Phase 1 scope creep into Phase 2 (LibraryTemplatesIndex edited twice)**
  (flagged by: code-quality, usability)
  — Phase 1 rewrites the templates eyebrow inline; Phase 2 rewrites it
  again to use `EyebrowLabel`. Two diff hunks where one would suffice.

### Tradeoff Analysis

- **Type-system rigour vs diff minimisation** — Architecture, Code Quality
  and Usability all want either a clean separation (`MutedGlyph` /
  `VirtualGlyph` wrapper, or a `Record<VirtualDocTypeKey, string>` table)
  OR a full collapse (delete `isGlyphDocTypeKey`/`GlyphDocTypeKey`, use
  `DocTypeKey` directly). The plan's current middle path — widen the type
  but keep the guard — satisfies no one. Recommendation: pick one. The
  "data-driven table" option preserves the original 0037 invariant
  (virtual keys excluded by construction) most faithfully.
- **End-user dark-theme cue vs scope discipline** — Usability flags that
  the per-doc-type cue collapses to white in dark mode (defeating the
  story's purpose for ~half the audience). Plan correctly keeps token
  changes out of scope (owned by 0073), but should at least name the
  tradeoff in "Desired End State" so it's a known limitation rather than
  ambient.

### Findings

#### Major

- 🟡 **Correctness**: Dark-theme `--ac-fg-muted` does NOT collapse to white
  **Location**: Current State Analysis — Key Discoveries; Phase 6 Manual Verification
  Plan asserts "every doc-type card shows … white in dark theme" — but
  `--ac-fg-muted` resolves to `#a0a5b8` in dark, not white. The EXPECTED_COLOR
  maps work if implementers look up `DARK_COLOR_TOKENS['ac-fg-muted']`
  correctly, but the prose misleads.

- 🟡 **Architecture**: Plan inverts a documented INVARIANT on Glyph
  **Location**: Phase 1 — Promote `templates` to a 13th Glyph key
  `Glyph.constants.ts:8-17` carries a load-bearing comment: "Glyph is for
  real document types only. Virtual keys (currently `templates`) are
  excluded by construction. ... so adding a future virtual key in
  api/types.ts automatically removes it from Glyph's set." Phase 1 silently
  abandons this and special-cases `templates` inline with a string-literal
  ternary. Either keep the invariant and add a `MutedGlyph` wrapper / data
  table, or collapse the type fully — don't leave it in between with a
  stale comment.

- 🟡 **Architecture / Code Quality / Usability**: Phase 1 silently changes
  HubCard rendering on the overview hub
  **Location**: Phase 6 §2 — Templates HubCard appearance
  `HubCard` (`LibraryOverviewHub.tsx:72`) and `ActivityFeed` both gate
  Glyph rendering on `isGlyphDocTypeKey`. Post-Phase-1 the templates hub
  tile begins rendering a Glyph. Plan calls this an "optional intentional
  change" in a Phase 6 note — but the work-item AC #3 says hub consumption
  is "unchanged". Promote to an explicit decision (or gate it out).

- 🟡 **Code Quality**: Phase 1 scope creep into Phase 2's eyebrow rewrite
  **Location**: Phase 1 §4 — Repoint listing-route fallback
  Phase 1 §4 rewrites `LibraryTemplatesIndex` eyebrow inline; Phase 2 §3
  rewrites the same eyebrow again to use `EyebrowLabel`. `LayersIcon`
  removal is also ambiguously owned. Make Phase 1 strictly Glyph-internal;
  defer all eyebrow rewrites to Phase 2.

- 🟡 **Test Coverage / Correctness / Standards**: Existing `Glyph.test.tsx`
  invariants encode the 12-key contract — Phase 1 leaves the suite red
  **Location**: Phase 1 §5 — Tests
  `Glyph.test.tsx:11-16` has a `@ts-expect-error` on `<Glyph docType="templates">`,
  `:20` asserts `GLYPH_DOC_TYPE_KEYS.length === 12`, `:24` asserts the
  array excludes templates, `:34` asserts `isGlyphDocTypeKey('templates')
  === false`. All will fail post-Phase-1. Plan only says "extend";
  enumerate the inversions/deletions required.

- 🟡 **Test Coverage / Architecture / Code Quality / Correctness / Standards**:
  Row container invariance baseline is wrong on multiple axes
  **Location**: Phase 5 §1 — aside-row-resolved-colours spec
  Hardcoded "browser default" RGB values for `.groupItem` are not real
  browser defaults (border-color falls back to `currentColor`,
  theme-dependent). The declared/inferred border that AC #2 cares about
  actually lives on `.group` (the wrapping div), not `.groupItem`. As
  written the spec will fail spuriously or pass for the wrong reasons.

- 🟡 **Test Coverage / Standards / Correctness**: Phase 5 row locator
  targets the wrong DOM element
  **Location**: Phase 5 §1 — Spec
  `row.locator('[data-doc-type="${target}"]').locator('svg')` will not
  resolve — unframed Glyph puts `data-doc-type` on the svg itself, not on
  an outer wrapper. The locator can also accidentally match the detail
  page's own eyebrow Glyph (post-Phase-4). Scope to the aside region
  explicitly and target svg directly.

- 🟡 **Correctness / Standards**: Phase 6 hub-card locator matches the
  framed wrapper `<span>`, not the SVG
  **Location**: Phase 6 §1 — Spec
  Framed Glyph emits `data-doc-type` on both wrapper span and inner svg;
  `.first()` picks the span. `getComputedStyle(span).color` reads inherited
  text colour, not the per-doc-type inline `color` on the svg. Use
  `svg[data-doc-type="..."]` and scope to a hub-grid testid.

- 🟡 **Test Coverage**: Phase 4 eyebrow LABEL TEXT colour assertion is
  vaguely specified
  **Location**: Phase 4 §1 — Spec
  "Text node's parent's computed colour minus inner svg" has no DOM
  realisation. The correct assertion is
  `getComputedStyle([data-slot="eyebrow"]).color === --ac-fg-faint` per
  theme — once per theme, not per doc type. Pin this concretely.

- 🟡 **Test Coverage**: Phase 5 anchor-fixture wiring is hand-waved
  **Location**: Phase 5 §1, §2 — Spec & Fixture updates
  Plan offers two alternative strategies ("one rich anchor" vs "13 anchors")
  and concludes "the spec drives this naturally". Define an explicit
  `ANCHOR_FIXTURES: Record<DocTypeKey, {anchorUrl, expectedRowCount}>` map
  up-front.

- 🟡 **Standards**: Test files placed under `__tests__/` subdirectory
  **Location**: Phase 1 §5, Phase 2 §1
  Every existing test in `frontend/src/` is co-located (`Foo/Foo.test.tsx`).
  No `__tests__/` directory exists. Plan would introduce an inconsistent
  layout.

- 🟡 **Standards**: Theme-switch convention mis-cited as `?theme=` query
  string
  **Location**: Phase 4 §1 — Spec
  Real chip/glyph specs use `document.documentElement.dataset.theme = 'dark'`
  — no query string. Update Phases 4/5/6 to match.

- 🟡 **Code Quality / Test Coverage / Standards / Usability**: Phase 5
  aria-hidden vs ariaLabel="" snippet is internally contradictory
  **Location**: Phase 5 §3 — RelatedArtifacts.tsx change
  Code snippet shows `ariaLabel=""` (yields `role="img"` with empty name —
  axe-core anti-pattern); prose immediately below says omit. Pick one in
  the snippet.

- 🟡 **Usability**: EyebrowLabel returns a Fragment with hard-coded
  size/framed
  **Location**: Phase 2 §1 — New shared component
  No wrapping element, no `data-component`, no `size`/`framed` props.
  Every future variation re-introduces prop drilling. Add minimal optional
  props OR wrap in `<span data-component="eyebrow-label">`.

- 🟡 **Usability**: "Test visually and adjust gap if needed" leaves the
  layout contract open
  **Location**: Phase 5 §4 — CSS
  `.groupItem` is `align-items: baseline; gap: 0.4rem`. A 16×16 svg's
  baseline does not align with text. Pre-commit to `align-items: center`
  and add a Playwright assertion.

- 🟡 **Usability / Architecture / Code Quality / Correctness / Standards**:
  `isGlyphDocTypeKey` becomes dead-code-by-design (cross-cutting)
  **Location**: Phase 1 §2, Phase 2 §1, Phase 5 §3
  Guard always returns true after widening; kept "for diff minimisation".
  Drop the guard and rename callsites to `isDocTypeKey`, OR keep the type
  narrow (preferred) and route templates through a separate path.

#### Minor

- 🔵 **Architecture / Code Quality / Correctness**: Framed templates Glyph
  silently uses default `--ac-bg-sunken` background
  **Location**: Phase 1 §3 — Glyph colour branch
  Visibly inconsistent with neighbouring framed glyphs in dark theme. Add
  an explicit CSS rule (even if redundant) so design intent is stated.

- 🔵 **Architecture**: Fixture-driven row coverage couples spec to fixture
  frontmatter
  **Location**: Phase 5 §2 — Fixture updates
  Editing a fixture for unrelated reasons breaks the aside-row spec
  silently. Introduce a dedicated `0099-related-coverage-anchor.md`
  fixture set marked "owned by AC #2".

- 🔵 **Code Quality / Test Coverage / Usability**: Shared
  `EXPECTED_COLOR`/helper extraction inconsistent across Phases 4-6
  **Location**: Phase 4/5/6 — Spec sections
  Commit to extracting `tests/visual-regression/lib/expected-colours.ts`
  in Phase 4 as a discrete sub-step.

- 🔵 **Test Coverage**: glyph-resolved-fill.spec.ts only extended to
  templates — 11 other keys remain uncovered at e2e layer
  **Location**: Phase 1 §5
  Parametrise over all 13 keys × 2 themes; `/glyph-showcase` already
  renders the full grid.

- 🔵 **Test Coverage**: Phase 3 smoke spec leaves navigation strategy
  ambiguous
  **Location**: Phase 3 §3
  Pick direct navigation, define `FIXTURE_SLUGS: Record<DocTypeKey, string>`
  once.

- 🔵 **Test Coverage / Code Quality**: No test enforces the chosen
  accessibility branch for row icons
  **Location**: Phase 5
  Add `RelatedArtifacts.test.tsx` assertion that the row svg has
  `aria-hidden="true"` and no `role`.

- 🔵 **Test Coverage**: Phase 6 spec mixes "non-regression" assertions
  with new-contract assertions
  **Location**: Phase 6 §1
  Split into two describes (or rename) so future failures distinguish
  regression-to-fix from contract-update-needed.

- 🔵 **Correctness / Usability**: Eyebrow renders during loading and
  "Document not found" states
  **Location**: Phase 4 §2 — LibraryDocView change
  `<Page title="Document not found">` will display a tinted doc-type
  eyebrow above a missing-document body. Gate on `entry && content.data`
  or document as intentional.

- 🔵 **Correctness**: Hardcoded border-color baseline is unstable across
  themes
  **Location**: Phase 5 §1
  When `border-width: 0px`, `border-color` resolves to `currentColor` —
  theme-dependent. Drop the assertion or parametrise per theme.

- 🔵 **Standards**: TemplatesIcon component shape diverges slightly from
  sibling icons
  **Location**: Phase 1 §1
  Existing icons use `: ReactElement` return type and string-quoted SVG
  attributes (`strokeWidth="2"`). Match the shape exactly.

- 🔵 **Usability**: Dark-theme cue collapses — purpose vs implementation gap
  **Location**: Desired End State
  All 12 `--ac-doc-*` foregrounds collapse to white in dark mode, defeating
  the doc-type cue for ~half the audience. Name the tradeoff explicitly
  (out of scope for this plan, owned by 0073) so it isn't ambient.

- 🔵 **Usability**: Eyebrow selector coupling vs Phase 6 selector
  inconsistent
  **Location**: Phase 4 §1
  Phase 4 uses `[data-slot="eyebrow"] svg`; Phase 6 uses `[data-doc-type]`.
  Standardise on `data-doc-type` for icon assertions across all specs.

#### Suggestions

- 🔵 **Architecture / Code Quality**: After Phase 1, audit the four
  `isGlyphDocTypeKey` callsites (HubCard, ActivityFeed, EyebrowLabel,
  RelatedArtifacts) and decide jointly whether they retain the guard or
  drop it. Don't ship dead guards.

- 🔵 **Code Quality**: Phase 3 smoke spec iterates `GLYPH_DOC_TYPE_KEYS`
  which expands to 13 keys after Phase 1 — but templates has no on-disk
  fixture. Branch on `VIRTUAL_DOC_TYPE_KEYS` for templates.

### Strengths

- ✅ Strong adherence to single-source-of-truth: routing all 13 doc-type
  icons through the existing `<Glyph>` component preserves the 0037
  consumer contract and avoids parallel rendering paths.
- ✅ Strict write-failing-test-first cycle per phase, with explicit
  expected-failure reasons.
- ✅ Pre-implementation reconciliation with codebase reality is unusually
  thorough — five mismatches between work item and code resolved before
  any code is written.
- ✅ Phase ordering is clean: Phases 1–3 are independent prerequisites,
  Phases 4 and 5 each map to one AC, Phase 6 is a standalone non-regression
  spec. Each phase can be merged independently.
- ✅ Reuses the proven `chip-resolved-colours.spec.ts` /
  `glyph-resolved-fill.spec.ts` computed-style pattern — the correct level
  for these assertions.
- ✅ Out-of-scope list is explicit (BigGlyph owned by 0082, aside redesign
  by 0079, no new tokens, no sidebar consumption) — prevents scope creep.
- ✅ Compile-time exhaustiveness via `Record<GlyphDocTypeKey, ComponentType>`
  is the right mechanism for catching missing icon registrations.
- ✅ Plan correctly identifies that the work item's `--ac-text-muted` is a
  rename of the real `--ac-fg-muted` token.
- ✅ EyebrowLabel extraction (Phase 2) is clean, well-placed, and respects
  existing patterns.

### Recommended Changes

Prioritised by impact:

1. **Resolve the `GlyphDocTypeKey` / `isGlyphDocTypeKey` dilemma upfront**
   (addresses: cross-cutting theme #1, Architecture major, Code Quality
   major, Usability major)
   Pick one path:
   - (a) Keep `GlyphDocTypeKey` narrow (12 keys). Templates renders through
     a separate `MutedGlyph` wrapper OR Glyph internally consults a
     `Record<VirtualDocTypeKey, string>` colour table. `isGlyphDocTypeKey`
     remains meaningful.
   - (b) Collapse `GlyphDocTypeKey` to `DocTypeKey`, delete
     `isGlyphDocTypeKey`, update all 4-5 callsites to use `isDocTypeKey`
     (already exists at `api/types.ts:22`).
   Update the file-level comment in `Glyph.constants.ts` in lock-step.

2. **Promote HubCard / ActivityFeed templates-rendering change to an
   explicit decision**
   (addresses: cross-cutting theme #2)
   Either gate it out (`docType.id !== 'templates'` in HubCard) or list it
   in the plan's "Knock-on Changes" with explicit acceptance. Reword work
   item AC #3 if the contract is changing.

3. **Enumerate the `Glyph.test.tsx` inversions required by Phase 1**
   (addresses: cross-cutting theme #3, three major findings)
   Add a Phase 1 sub-section listing every assertion to update: length 12
   → 13, `not.toContain('templates')` → `toContain('templates')`,
   `isGlyphDocTypeKey('templates') === false` → `=== true`, drop the
   `@ts-expect-error`, update the filter at line 38.

4. **Fix Phase 5 spec selectors and baseline strategy**
   (addresses: cross-cutting themes #4, #5; Test Coverage majors)
   - Scope row locator to aside region: `[data-testid="related-artifacts"]
     svg[data-doc-type="..."]` (add the testid to RelatedArtifacts root).
   - Drop hardcoded "browser default" baselines; either capture pre-change
     values empirically per theme or assert only what's semantically
     meaningful (e.g. `border-width === '0px'` and skip border-color).
   - Confirm whether AC #2 invariance should target `.group` (where the
     declared/inferred border lives) — not `.groupItem`.

5. **Fix Phase 4 eyebrow text-colour assertion and theme-switch citation**
   (addresses: Test Coverage major, Standards minor)
   - Pin label-text assertion concretely: read
     `getComputedStyle([data-slot="eyebrow"]).color`, assert against
     `--ac-fg-faint` per theme, once per theme.
   - Replace `?theme=` references with `documentElement.dataset.theme`
     across Phases 4/5/6.

6. **Fix Phase 6 hub-card locator**
   (addresses: Correctness major, Standards minor)
   Use `svg[data-doc-type="..."]` scoped to a hub-grid testid. Apply the
   same pattern to all three new specs for consistency.

7. **Fix dark-theme prose error**
   (addresses: Correctness major)
   Rewrite the Key Discoveries note and Phase 6 manual-verification step
   to state that `--ac-fg-muted` resolves to `#a0a5b8` in dark, NOT white.
   Other 12 doc-type foregrounds collapse to white; templates is
   intentionally different.

8. **Clean up the Phase 5 ariaLabel snippet**
   (addresses: cross-cutting theme #6)
   Delete `ariaLabel=""` from the code snippet. Keep only the prose
   rationale. Add the corresponding Vitest assertion that the row svg has
   `aria-hidden="true"`.

9. **Extract `tests/visual-regression/lib/expected-colours.ts` as a Phase
   4 sub-step**
   (addresses: cross-cutting theme #7)
   Phases 5 and 6 import unconditionally. Optionally migrate the helpers
   from `chip-resolved-colours.spec.ts` in the same move.

10. **Move LibraryTemplatesIndex eyebrow rewrite from Phase 1 to Phase 2**
    (addresses: cross-cutting theme #9)
    Phase 1 stays Glyph-internal (add icon, add colour branch, add tests).
    Phase 2 owns all eyebrow consolidation and `LayersIcon` removal in
    one diff.

11. **Add explicit decisions for residual minor issues:**
    - Frame background for templates: add explicit CSS rule OR comment
      stating intent.
    - Eyebrow rendering in loading/error states: gate on `entry &&
      content.data` OR document as intentional.
    - Row alignment: pre-commit to `align-items: center` and assert.
    - Dark-theme cue collapse: add a "Dark-theme behaviour" subsection to
      "Desired End State" naming the tradeoff.
    - TemplatesIcon shape: match sibling icons exactly (return type,
      attribute style).
    - Phase 3 smoke spec: pick direct navigation, define `FIXTURE_SLUGS`
      once, branch on `VIRTUAL_DOC_TYPE_KEYS` for templates.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan funnels all per-doc-type tint through a single Glyph
consumer surface and reuses one shared EyebrowLabel helper at both eyebrow
sites, which is a sound centralisation move that preserves the 0037
consumer contract. However, Phase 1 inverts a documented INVARIANT on
Glyph (virtual keys excluded by construction) by widening GlyphDocTypeKey
to include templates and special-casing the colour branch inline, which
weakens the type-narrowing guarantee and silently changes HubCard
rendering on the overview hub. Phase 5's container-invariance assertion
and fixture-driven row coverage introduce architectural brittleness that
should be tightened before the plan moves to implementation.

**Strengths**: single consumer surface; shared EyebrowLabel; strict TDD;
honest pre-implementation reconciliation; CSS-free constants vs CSS-module
Glyph boundary preserved; explicit out-of-scope list.

**Findings**: 2 major (invariant inversion, HubCard implicit change), 3
minor (container baseline brittle, fixture coupling, frame background
degradation), 1 suggestion (guard dead-code).

### Code Quality

**Summary**: Well-structured plan with proper SRP and DRY through Glyph
centralisation and EyebrowLabel extraction. Concerns: templates
primitive-obsession (string-literal ternary), `GlyphDocTypeKey` invariant
erosion, Phase 1 leaks into Phase 2's territory, Phase 6 hides a
behavioural change.

**Strengths**: SoT through Glyph; pure-refactor isolation in Phase 2; TDD
applied uniformly; existing parametrised loop patterns reused;
compile-time exhaustiveness leveraged.

**Findings**: 3 major (templates special-case, Phase 1 scope creep into
Phase 2, hidden HubCard change), 4 minor (frame bg glossed over, baseline
brittleness, dead type guard, aria contradiction, helper extraction
inconsistent), 1 suggestion (smoke spec key iteration).

### Test Coverage

**Summary**: Disciplined TDD cycle and proven computed-style pattern,
right shape for this work. Several test designs have correctness or
maintainability problems: Phase 5 row-container baseline misidentifies
where the relevant CSS lives, Phase 4 label-text assertion mechanism is
inconsistent, Phase 3 fixture-coverage is under-specified, and existing
Glyph.test.tsx exhaustiveness/type-guard assertions need explicit
updating.

**Strengths**: write-failing-test cycle; pattern reuse; 13×2 matrix; Phase
6 non-regression spec; Phase 3 self-verifying; Vitest test updates planned.

**Findings**: 4 major (container baseline wrong, Glyph.test.tsx will fail,
label-text vague, anchor fixtures hand-waved), 5 minor (helper extraction,
glyph-fill 11 keys uncovered, smoke nav ambiguous, no aria assertion,
non-regression vs new-contract mixed).

### Correctness

**Summary**: Logically coherent but carries several latent issues: dark-theme
`--ac-fg-muted` assumption is wrong (won't collapse to white), Vitest
12-key contracts unaddressed, Phase 6 locator targets wrapper span not
SVG, eyebrow renders in loading/error states.

**Strengths**: TDD makes regressions visible per phase; compile-time
exhaustiveness right mechanism; correct identification of `--ac-fg-muted`
vs work item's `--ac-text-muted`; Phase 3 smoke prerequisite sound;
LibraryDocView early-return paths verified.

**Findings**: 3 major (dark-theme misstatement, Glyph.test.tsx contracts,
Phase 6 wrong-element locator), 6 minor (eyebrow in loading/error,
border-color hardcoded, vague label assertion phrasing, framed bg
degradation in dark, viewBox verification, dead `isGlyphDocTypeKey` guard).

### Standards

**Summary**: Generally follows project conventions (TDD, file co-location,
visual-regression patterns, Glyph consumer contract). Diverges in several
concrete ways: tests under `__tests__/` subdirectory, `isGlyphDocTypeKey`
semantically misleading, existing `_typeContractGuards` unaddressed,
theme-switch convention mis-cited, Phase 5 row icon selector incorrect.

**Strengths**: Glyph icon contract correctly followed; `data-slot="eyebrow"`
verified; `--ac-fg-muted` token correctly resolved; decorative aria
guidance correct; spec placement correct.

**Findings**: 3 major (`__tests__/` placement, contract guards
unaddressed, Phase 5 locator wrong), 6 minor (misleading guard name,
theme-switch citation, ariaLabel snippet contradiction, TemplatesIcon
shape, hub locator ambiguous, container baseline hardcoded).

### Usability

**Summary**: Well-organised, clear TDD cycle, clean phase split,
centralised tinting story. EyebrowLabel API is unergonomic (Fragment, fixed
size/framed); row icon a11y choice oscillates; `isGlyphDocTypeKey` becomes
misleading dead code; Phase 6 EXPECTED_COLOR map half-specified. For
end-users, dark-theme white-collapse means the per-doc-type cue effectively
disappears in dark mode — plan should own this tradeoff.

**Strengths**: clean phase ordering; thorough pre-implementation research;
explicit TDD with expected-failure reasons; single Glyph consumer;
EyebrowLabel extracted once before consumed twice.

**Findings**: 5 major (Fragment API, ariaLabel snippet, layout gap
open-ended, dead guard, HubCard implicit change), 4 minor (helper
extraction, dark-theme cue gap, selector inconsistency, Phase 1 file
churn).

## Re-Review (Pass 2) — 2026-05-26

**Verdict:** REVISE

Pass-1 revisions resolved the overwhelming majority of findings (~26 of
the original 32), particularly the central architectural dilemma
(`GlyphDocTypeKey` collapse), the dark-theme prose error, the helper
extraction inconsistency, the framed/unframed locator confusion, the
`__tests__/` placement, the theme-switch citation, the ariaLabel
contradiction, the EyebrowLabel Fragment, the open-ended row alignment,
and the implicit HubCard change. However, the collapse decision
introduced a critical correctness gap (Phase 1 §4 missed four real
consumers of the deleted symbols) and several major spec-mechanics
issues (Phase 5 testid attachment, fixture-coverage `article` locator,
ANCHOR_FIXTURES map incompleteness, helper-shape migration). One
cross-cutting architectural concern — stringly-typed `!== 'templates'`
exclusions scattered across consumers — was flagged by both Architecture
and Usability as a regression in maintainability that needs a small
predicate to fix.

### Previously Identified Issues

**Resolved (pass 1 → pass 2):**
- 🟡 **Correctness**: Dark-theme `--ac-fg-muted` does NOT collapse to
  white — **Resolved** (prose corrected; new "Dark-theme behaviour"
  subsection)
- 🟡 **Architecture**: Plan inverts documented INVARIANT on Glyph —
  **Resolved** (type fully collapsed; `DOC_TYPE_COLOR_VAR` table)
- 🟡 **Architecture/Code Quality/Usability**: Phase 1 silently changes
  HubCard rendering — **Resolved** (explicit `docType !== 'templates'`
  gate with Phase 6 `toHaveCount(0)` assertion)
- 🟡 **Code Quality**: Phase 1 scope creep into Phase 2 — **Resolved**
  (LibraryTemplatesIndex eyebrow rewrite moved entirely to Phase 2)
- 🟡 **Test Coverage/Correctness/Standards**: Existing `Glyph.test.tsx`
  invariants — **Resolved** (Phase 1 §5 enumerates each line to
  invert/delete)
- 🟡 **Test Coverage/Architecture/Code Quality/Correctness/Standards**:
  Row container baseline wrong — **Resolved** (border-color hardcoding
  dropped; assertions moved to `.group` where the CSS lives)
- 🟡 **Test Coverage/Standards/Correctness**: Phase 5 row locator wrong
  element — **Resolved** (uses `svg[data-doc-type=…]` scoped via testid)
- 🟡 **Correctness/Standards**: Phase 6 hub locator matches wrapper —
  **Resolved** (`svg[data-doc-type=…]` scoped via `hub-grid` testid)
- 🟡 **Test Coverage**: Phase 4 label-text colour assertion vague —
  **Resolved** (concrete selector; once per theme)
- 🟡 **Standards**: Tests under `__tests__/` — **Resolved** (co-located,
  convention noted explicitly)
- 🟡 **Standards**: Theme-switch `?theme=` mis-cited — **Resolved**
  (`documentElement.dataset.theme`, plus `setTheme` helper)
- 🟡 **Code Quality/Test Coverage/Standards/Usability**: Phase 5
  ariaLabel snippet contradiction — **Resolved** (snippet shows omit;
  Vitest assertion added)
- 🟡 **Usability**: EyebrowLabel returns Fragment — **Resolved**
  (wrapping `<span data-component=…>`)
- 🟡 **Usability**: "Test visually and adjust gap" open-ended —
  **Resolved** (pre-commit to `align-items: center`)
- 🟡 **Usability/Architecture/Code Quality/Correctness/Standards**:
  `isGlyphDocTypeKey` dead code — **Resolved** (deleted entirely)
- 🔵 Several minors: framed templates frame bg (partially), helper
  extraction inconsistency, dark-theme cue not named, eyebrow in
  loading state, glyph-fill 11 keys uncovered, Phase 3 nav ambiguous,
  no aria assertion, smoke spec key iteration — all **Resolved**.

**Partially resolved:**
- 🔵 **Architecture**: Framed templates Glyph silently uses default
  background — **Partially resolved**. Plan adds an explicit
  `.frame[data-doc-type="templates"] { background: var(--ac-bg-sunken); }`
  rule, but the new rule is functionally identical to the default
  (no-op), so it documents intent without providing structural
  protection. See new finding below.
- 🔵 **Architecture**: Fixture-driven row coverage couples spec —
  **Partially resolved**. Dedicated `0099-related-coverage-anchor`
  fixture set introduced (good), but the `ANCHOR_FIXTURES` map in the
  spec snippet still shows only 2 of 13 entries with a `…` placeholder
  (see new finding below).

### New Issues Introduced

**Critical:**
- 🔴 **Correctness**: Phase 1 §4 misses four real consumers of the
  deleted `GlyphDocTypeKey` / `GLYPH_DOC_TYPE_KEYS` symbols —
  `template-tier.ts` (uses `GlyphDocTypeKey` as `STEM_TO_GLYPH` value
  type and `glyphKeyForTemplate` return type), `GlyphShowcase.tsx`,
  `GlyphShowcase.test.tsx`, and `glyph-showcase.spec.ts` all import
  `GLYPH_DOC_TYPE_KEYS`. The Glyph JSDoc contract (`Glyph.tsx:73-74`)
  and dev-only console.warn (`Glyph.tsx:81`) also reference these
  symbols. Phase 1 typecheck/build will fail.

**Major:**
- 🟡 **Architecture/Usability**: Stringly-typed `docType !== 'templates'`
  exclusion at every consumer (HubCard, ActivityFeed). Knowledge of
  which keys are virtual is scattered across modules as string
  literals; a future second virtual key requires touching every
  consumer. Suggestion: introduce a small named predicate
  `isPhysicalDocTypeKey(key)` (or similar) co-located with
  `VIRTUAL_DOC_TYPE_KEYS` in `api/types.ts`.
- 🟡 **Test Coverage/Correctness**: Phase 5 §1 says "add
  `data-testid="related-artifacts"` to the component's root element"
  but `RelatedArtifacts.tsx` returns a React Fragment with no root.
  The wrapping `<section>` lives in `LibraryDocView.tsx`. Phase 5 spec
  locators won't resolve as written. Suggestion: attach the testid to
  `LibraryDocView.tsx`'s `<section>` (smallest diff) or wrap
  `RelatedArtifacts`' return in an element.
- 🟡 **Test Coverage/Correctness**: Phase 3 `fixture-coverage.spec.ts`
  uses `expect(page.locator('article')).toBeVisible()` for both
  listing AND detail routes. `<article>` exists only in
  `LibraryDocView.tsx` (detail). The listing-route assertion will
  fail for every doc type, masking the actual "missing fixtures"
  signal. Suggestion: use `'main'` or a content-specific testid for
  listing routes; keep `<article>` only on detail routes.
- 🟡 **Test Coverage/Correctness/Usability**: Phase 5 `ANCHOR_FIXTURES`
  map shown with only `decisions` + `work-items` entries plus a `…`
  placeholder; templates entry's value is described in prose
  ("option a") but never written. `Record<DocTypeKey, string>`
  requires all 13 keys — TS will reject the partial literal.
  Suggestion: enumerate all 13 URLs explicitly OR derive
  mechanically from a single anchor URL pattern OR loop over
  `DOC_TYPE_KEYS.filter(k => k !== 'templates')` for the dedicated
  anchors and add one explicit `'templates row appears in <anchor>'`
  case.
- 🟡 **Test Coverage**: Phase 4 declares the chip/glyph spec migration
  to the new helper module a "mechanical refactor", but the new
  `parseRgb`/`hexToRgb` return `Rgb` objects while the existing
  versions return tuples / CSS strings. Every callsite in both specs
  needs editing. Suggestion: match the existing tuple-returning
  shapes in the helper (zero-edit migration) OR enumerate the
  assertions to change.

**Minor:**
- 🔵 **Architecture/Code Quality**: Phase 4's `tokenKeyFromCssVar`
  string-parses `DOC_TYPE_COLOR_VAR` values to recover token keys —
  coupling test infrastructure to the production lookup's exact
  string format. Suggestion: export a parallel
  `DOC_TYPE_TOKEN_KEY: Record<DocTypeKey, string>` from
  `Glyph.constants.ts` so the test consumes a typed lookup.
- 🔵 **Architecture/Code Quality/Correctness**: The new
  `.frame[data-doc-type="templates"] { background: var(--ac-bg-sunken); }`
  rule duplicates the default `.frame` background — functionally a
  no-op that won't catch future changes. Suggestion: either drop and
  document intent via a single CSS comment, or co-locate the rationale
  in `Glyph.constants.ts` next to the `DOC_TYPE_COLOR_VAR['templates']`
  entry.
- 🔵 **Code Quality/Correctness/Usability**: Phase 5 introduces
  `data-relation-kind` attribute on `.group` wrapper in §5 (CSS
  section), but the Phase 5 spec consumes it in §2. Cross-section
  ownership confusion. Suggestion: move the JSX attribute addition to
  §4 (`RelatedArtifacts.tsx` change) alongside the other JSX edits.
- 🔵 **Correctness**: Phase 5 `data-relation-kind="declared"` is
  emitted by both `Targets` and `Referenced by` blocks (both pass
  `kind="declared"`). Two `<div>`s carry the same attribute value;
  the `.first()` in the spec masks this. Suggestion: narrow to
  `declared-outbound` / `declared-inbound` / `inferred`, or
  acknowledge intentionally non-unique.
- 🔵 **Correctness**: Phase 5 container-invariance test navigates to
  one anchor and asserts both declared AND inferred groups exist —
  but the anchor fixture's frontmatter needs to surface both row
  kinds. Not specified. Suggestion: require each `0099-anchor`
  fixture to produce at least one declared and one inferred row, or
  split the invariance test across two anchors.
- 🔵 **Correctness/Code Quality**: Phase 1 extends
  `glyph-resolved-fill.spec.ts` to 13×2 BEFORE the Phase 4 helper
  extraction lands. Either Phase 1 ships interim duplication or
  reaches into Phase 4 scope. Suggestion: pick one explicitly —
  recommend (a) Phase 1 keeps minimal inline helpers, Phase 4
  absorbs them during the documented migration.
- 🔵 **Standards/Usability**: New `data-component="eyebrow-label"`
  attribute diverges from established `data-slot=…` / `data-testid=…`
  conventions used elsewhere (HubCard, RelatedArtifacts, Page).
  Suggestion: use `data-testid="eyebrow-label"` (test-only hook
  pattern) or `data-slot="label"` (Page/Chip pattern).
- 🔵 **Standards**: New `data-relation-kind` attribute introduces a
  third stable-hook namespace alongside `data-slot` / `data-testid`,
  duplicating the existing `.groupDeclared` / `.groupInferred` CSS
  modifier classes. Suggestion: use `data-testid="related-group-…"`
  to stay within established hook namespaces.
- 🔵 **Standards**: Phase 1 §4's prescribed code comment cites "plan
  0074 Phase 1" — the codebase convention is to cite
  `meta/work/NNNN` (work items are durable; plans are draft).
  Suggestion: rephrase as `see meta/work/0074`.
- 🔵 **Correctness**: Phase 1 §3 says "update both call sites" for
  the inline-style edit in `Glyph.tsx` but doesn't show diffs for
  both. The framed-branch update is the higher-risk one (EyebrowLabel
  uses framed). Suggestion: enumerate both line numbers explicitly,
  add a Vitest assertion for `<Glyph docType="templates" framed />`.
- 🔵 **Code Quality**: `FIXTURE_SLUGS` (Phase 3) and `ANCHOR_FIXTURES`
  (Phase 5) are two parallel `Record<DocTypeKey, string>` maps in
  `tests/visual-regression/lib/` that will inevitably drift.
  Suggestion: co-locate with a doc comment explaining the distinction,
  or derive one from the other.
- 🔵 **Test Coverage/Usability**: `FIXTURE_SLUGS.templates: 'work-item'`
  is a template name, not a fixture file slug — semantic overload in
  a `Record<DocTypeKey, string>` typed as canonical slugs. Suggestion:
  rename to `DETAIL_ROUTE_SLUGS` with a JSDoc note, or split into two
  maps.
- 🔵 **Usability**: Phase 4 label-text colour assertion reads `color`
  from the wrapping `<span data-component="eyebrow-label">` and
  relies on the reader inferring CSS inheritance to understand why
  this measures the label text (not the inner SVG). Suggestion: wrap
  the label text in its own slot/span or add a one-line comment to
  the spec.

### Assessment

The plan is much improved — pass-1 substantive changes were applied
faithfully and the cross-cutting architectural decision (type collapse +
explicit HubCard gate) was the right call. However, **the plan is not
yet ready to implement**:

- The Phase 1 callsite enumeration is incomplete in a way that will
  fail the Phase 1 success criteria immediately (critical).
- Phase 5 cannot start until the `data-testid` attachment question and
  the `ANCHOR_FIXTURES` enumeration are resolved.
- Phase 3 needs a corrected smoke-spec locator before its TDD cycle
  can produce a useful failure signal.
- The stringly-typed exclusion pattern wants a small named predicate
  to avoid re-introducing the maintenance problem the collapse was
  meant to solve.

Most fixes are small and localised. One more revision pass should land
the plan; the changes do not require re-litigating any pass-1
decisions.

## Re-Review (Pass 3) — 2026-05-26

**Verdict:** REVISE

Pass-2 revisions resolved nearly all pass-2 findings convincingly. The
plan's structure, decisions, and selector strategy are now sound across
all six lenses. The remaining issues cluster tightly around three small
concrete defects (one of which is critical) that the pass-2 fix
sequence introduced: a non-existent type name (`ColorTokenKey`) in the
new Phase 1 §2 snippet, a tuple-vs-string assertion-shape bug in
Phase 6 that diverges from the Phase 4/5 pattern, and a missed
re-export block in `Glyph.tsx` that re-exports the symbols Phase 1 §2
deletes. All three are confined to specific snippets; no architectural
re-decision is needed. After these three fixes plus a handful of small
enumeration/clarity tweaks, the plan is ready to implement.

### Previously Identified Issues (pass 2 → pass 3)

**Resolved:**
- 🔴 **Correctness**: Phase 1 §4 missed callsites (template-tier.ts,
  GlyphShowcase x3) — **Resolved** (Phase 1 §4 enumerates all consumers
  plus JSDoc and console.warn).
- 🟡 **Architecture/Usability**: Stringly-typed `!== 'templates'`
  exclusion — **Resolved** (`isPhysicalDocTypeKey` predicate
  co-located with `VIRTUAL_DOC_TYPE_KEYS`).
- 🟡 **Test Coverage/Correctness**: `RelatedArtifacts` Fragment-no-root
  — **Resolved** (testid moved to `LibraryDocView`'s existing
  `<section>`).
- 🟡 **Test Coverage/Correctness**: Phase 3 `article` locator —
  **Resolved** (`getByRole('heading', { level: 1 })` for listings).
- 🟡 **Test Coverage/Correctness/Usability**: `ANCHOR_FIXTURES`
  partial Record + templates undefined — **Resolved** (single
  `ANCHOR_URL` constant; one dedicated 0099 anchor fixture).
- 🟡 **Test Coverage**: Helper "mechanical refactor" not mechanical —
  **Resolved** (helper shapes pinned to existing tuple/string
  conventions verbatim).
- 🔵 **Architecture/Code Quality**: `tokenKeyFromCssVar` regex coupling
  — **Resolved** (`DOC_TYPE_TOKEN_KEY` typed lookup; no regex).
- 🔵 **Architecture/Code Quality/Correctness**: Redundant
  `.frame[templates]` CSS rule — **Resolved** (replaced with comment).
- 🔵 **Code Quality/Correctness/Usability**: `data-relation-kind`
  cross-section ownership — **Resolved** (replaced with three distinct
  `data-testid` values in §4 alongside the JSX edit).
- 🔵 **Correctness**: Non-unique `data-relation-kind="declared"` —
  **Resolved** (three distinct testids).
- 🔵 **Correctness**: Anchor fixture missing both kinds — **Resolved**
  (§3 explicitly requires declared-outbound + declared-inbound +
  inferred).
- 🔵 **Test Coverage/Correctness**: Phase 1 vs Phase 4 helper
  ordering — **Resolved** (Phase 1 keeps inline; Phase 4 absorbs).
- 🔵 **Standards/Usability**: `data-component` divergent — **Resolved**
  (`data-testid="eyebrow-label"`).
- 🔵 **Standards**: Third namespace `data-relation-kind` — **Resolved**.
- 🔵 **Standards**: "plan 0074 Phase 1" citation — **Resolved**
  (`see meta/work/0074`).
- 🔵 **Correctness**: Phase 1 §3 both call sites — **Resolved**
  (line 110 and 129 enumerated explicitly).
- 🔵 **Code Quality**: `FIXTURE_SLUGS`/`ANCHOR_FIXTURES` parallel maps
  — **Resolved** (ANCHOR_FIXTURES collapsed to single URL; only one
  per-DocTypeKey map remains).
- 🔵 **Test Coverage/Usability**: `FIXTURE_SLUGS.templates` semantic
  overload — **Resolved** (renamed to `DETAIL_ROUTE_SLUGS` with
  explicit JSDoc).
- 🔵 **Usability**: Label-text inheritance comment — **Resolved**.

### New Issues Introduced (pass 2 → pass 3)

**Critical:**
- 🔴 **Test Coverage / Correctness**: Phase 6 §2 spec compares tuple
  (`parseRgb` return) against string (`hexToRgb` return) via
  `.toEqual`. Phases 4 and 5 correctly use `expect(color).toBe(hexToRgb(...))`.
  Every Phase 6 hub-card and eyebrow assertion (~52 cases) will fail.
  Suggestion: replace both occurrences with the string-compare pattern;
  drop the unused `parseRgb` import.

**Major (cluster — 5 lenses flagged the same root cause):**
- 🟡 **Architecture / Code Quality / Correctness / Standards / Usability**:
  Phase 1 §2's `DOC_TYPE_TOKEN_KEY: Record<DocTypeKey, ColorTokenKey>`
  imports `ColorTokenKey` from `tokens.ts`, but that file exports only
  `ColorTokenLight` and `ColorTokenDark` — there is no `ColorTokenKey`
  symbol. Phase 1 typecheck will fail at the named import. The
  load-bearing "compile-time exhaustiveness across both doc and token
  key sets" property the plan promises depends on a non-existent type.
  Suggestion: introduce `export type ColorTokenKey = ColorTokenLight &
  ColorTokenDark` in `tokens.ts` as a small Phase 1 prerequisite (the
  intersection asserts key-set parity at the type level — useful
  invariant in its own right), OR change the snippet to use the
  concrete `ColorTokenLight` and rely on the existing CSS↔TS parity
  test for dark coverage.

**Major (related):**
- 🟡 **Architecture / Correctness**: Phase 1 §2/§3 deletes
  `GlyphDocTypeKey` / `GLYPH_DOC_TYPE_KEYS` / `isGlyphDocTypeKey` but
  doesn't enumerate the removal of the re-export block at
  `Glyph.tsx:22-26` (and the matching import at line 4) which still
  reference these symbols. Typecheck fails until both are removed.
  Suggestion: add an explicit bullet to Phase 1 §3 covering the
  re-export block and import line.
- 🟡 **Architecture**: `isPhysicalDocTypeKey` body casts to
  `VirtualDocTypeKey`, but `api/types.ts` exports only the runtime
  `VIRTUAL_DOC_TYPE_KEYS` constant — no `VirtualDocTypeKey` type alias.
  Suggestion: add `export type VirtualDocTypeKey = typeof
  VIRTUAL_DOC_TYPE_KEYS[number]` as part of the same Phase 1 §4
  sub-step that introduces the predicate, OR rewrite the predicate
  body without the cast using the established `isDocTypeKey` pattern
  (`(VIRTUAL_DOC_TYPE_KEYS as readonly DocTypeKey[]).includes(key)`).

**Minor:**
- 🔵 **Architecture / Test Coverage / Correctness**: Single ANCHOR_URL
  concentrates cross-fixture coordination (declared-outbound,
  declared-inbound, inferred-cluster co-membership) into one place
  without a deterministic specification of which surrounding fixtures
  must declare 0099 as a target or share its slug stem. Suggestion:
  pin the choreography — name the specific co-member fixtures, or
  make the cluster a sibling set of `0099-related-coverage-*` files.
- 🔵 **Test Coverage**: Phase 5 spec asserts a row exists for all 13
  doc types including the virtual templates case via a
  `templates:<name>` declared link, but whether the backend actually
  emits a templates row from a virtual cross-ref isn't called out as
  a Phase 3 prerequisite. Suggestion: verify the server contract
  ahead of Phase 5, or scope templates row coverage via inferred-cluster
  membership instead.
- 🔵 **Test Coverage / Correctness**: Phase 4 spec snippet ends each
  test with `void parseRgb // re-exported for symmetry with chip/glyph
  specs` despite the test only using `hexToRgb`. Dead code in a TDD
  reference snippet that implementers will copy. Suggestion: drop the
  `void parseRgb` line and the `parseRgb` import from the Phase 4
  snippet.
- 🔵 **Test Coverage**: Phase 5's `RelatedArtifacts.test.tsx` update is
  listed only in Success Criteria, not in Changes Required. Promote
  to an explicit Phase 5 §6 (or add to §4 alongside the JSX edit).
- 🔵 **Test Coverage**: No Vitest assertion exercises `<Glyph
  docType="templates" framed />`. The framed branch is the
  higher-risk EyebrowLabel path and warrants a unit-level guard.
  Suggestion: add to Phase 1 §5 positive assertions.
- 🔵 **Test Coverage**: Container-invariance describe lacks a
  `setTheme` call in `beforeEach` — fine today (assertions are
  theme-invariant) but fragile to future additions. Suggestion: pin
  `setTheme(page, 'light')` or comment that the describe is
  theme-invariant by construction.
- 🔵 **Test Coverage / Correctness**: `.groupItem` border-width / style
  invariance is not asserted in Phase 5. Suggestion: add one short
  `borderTopWidth === '0px'` assertion to `.groupItem`, or document
  explicitly that border-on-`.groupItem` is not asserted at the spec
  layer.
- 🔵 **Correctness**: Phase 1 §5 enumerates assertion-line updates but
  doesn't explicitly call out the Glyph.test.tsx import line
  (`import { Glyph, GLYPH_DOC_TYPE_KEYS, isGlyphDocTypeKey, type
  GlyphDocTypeKey } from './Glyph'`) which becomes a TS error after
  §2's deletions. Suggestion: add an import-line update bullet to §5.
- 🔵 **Code Quality / Code Quality**: AC #2 fixture coverage gap has
  no compile-time or test-time guard — a missing target in the
  anchor fixture surfaces only as a `toBeVisible` timeout. Suggestion:
  add an up-front assertion in the spec counting
  `svg[data-doc-type]` elements equals `DOC_TYPE_KEYS.length` with
  an error message naming the anchor fixture.
- 🔵 **Code Quality**: The CSS-comment-only documentation for
  templates frame background provides no automated guard. Suggestion
  (optional): add a small Vitest assertion that `<Glyph
  docType="templates" framed />` resolves frame background to
  `--ac-bg-sunken`.
- 🔵 **Usability**: `DETAIL_ROUTE_SLUGS` snippet shows only 3 entries
  with a `// …` placeholder for the other 10. `Record<DocTypeKey, string>`
  forces the implementer to invent each slug by reading fixture
  directories. Suggestion: enumerate all 13 slug values explicitly in
  the snippet (or list canonical existing fixture filenames per doc
  type above it).
- 🔵 **Usability**: Phase 4 helper-module snippet shows function
  signatures with `/* … existing impl */` placeholders. The zero-edit
  migration promise depends on absolute fidelity; an implementer
  inlining "cleaned-up" rewrites could silently break the chip/glyph
  spec migrations. Suggestion: inline the function bodies (they total
  <40 lines) or instruct "copy verbatim from
  `chip-resolved-colours.spec.ts:4-42`; do not reshape signatures".

### Assessment

The plan has narrowed sharply across three review passes — pass 1
surfaced 32 findings, pass 2 reduced to ~26 of those resolved with 25
new findings introduced by the collapse, and pass 3 closed all the
remaining major architectural and standards concerns from pass 2. Only
two real new defects emerged from pass 2's fix sequence: the
non-existent `ColorTokenKey` import (high-confidence, 5-lens consensus)
and the Phase 6 assertion-shape bug (high-confidence, critical). Both
are tiny line-level fixes. The undocumented `Glyph.tsx` re-export
block, the missing `VirtualDocTypeKey` type alias, and the Phase 1 §5
import-line oversight are similarly local.

The plan is **one small round away from ready-to-implement**. None of
the remaining issues require re-decisions or re-litigation; all are
"fix the snippet, name the missing type alias, enumerate the import
line, switch one assertion shape" mechanical edits.

## Re-Review (Pass 4) — 2026-05-26

**Verdict:** REVISE

Pass-3 revisions cleanly closed every pass-3 finding the lenses had
seen at the time, including the critical `ColorTokenKey` and Phase 6
assertion-shape bugs, plus all enumerated missing alias / import /
re-export concerns. The **Standards lens returned zero findings** —
the plan is now standards-conformant.

However, this pass's correctness agent verified the Phase 5 fixture
choreography against the actual backend code for the first time and
surfaced **two critical defects that have been present since Phase 5
was first written**:

1. The anchor `targets:` frontmatter list is not a contract the
   backend supports — `server/src/indexer.rs:611-643` only recognises
   `target:` (singular, plan-reviews only) and work-item refs via
   `work_item_id` / `parent` / `related`. The plan's "one row per
   doc type from a single anchor's targets list" design cannot work.
2. The three-fixture choreography assumed slug-stem-based inferred
   clustering (`0099-…-anchor` + `0099-…-mirror`). But
   `server/src/slug.rs:48-55` strips the numeric prefix, and the
   clustering algorithm in `clusters.rs` / `related.rs` matches on
   the exact post-prefix slug — so two work-items with `0099` prefix
   but different post-prefix slugs do not cluster.

These are deeper than pass-3's "is the field named right" question;
they're "is the underlying mechanism wired the way the plan assumes".
The plan's AC #2 coverage strategy needs to be redesigned around the
actual backend mechanisms. The Phase 3 backend-contract verification
step that pass 3 introduced was the right instinct — but pass 3 didn't
verify the contract itself, only added a step asking the implementer
to. Pass 4 did the verification, and the contract isn't there.

### Previously Identified Issues (pass 3 → pass 4)

**Resolved:**
- 🟡 **All 5 lenses**: `ColorTokenKey` non-existent type — **Resolved**
  (Phase 1 §2a introduces the alias as an intersection of light/dark
  token-key sets).
- 🟡 **Architecture/Correctness**: `Glyph.tsx` re-export block + import
  line not enumerated — **Resolved** (Phase 1 §3 lists both).
- 🟡 **Architecture**: `VirtualDocTypeKey` type alias missing —
  **Partially resolved** (alias declared, but degenerates to
  `DocTypeKey` due to runtime annotation — see new finding below).
- 🔴 **Test Coverage / Correctness**: Phase 6 tuple-vs-string
  assertion bug — **Resolved** (both occurrences switched to
  `expect(color).toBe(hexToRgb(...))`).
- 🔵 **Correctness**: Phase 1 §5 import-line update — **Resolved**.
- 🔵 **Test Coverage**: Framed-templates Vitest assertion —
  **Resolved**.
- 🔵 **Usability**: All 13 `DETAIL_ROUTE_SLUGS` inlined — **Resolved**
  (values verified against actual fixture directory).
- 🔵 **Usability**: Phase 4 helper-bodies elided — **Partially
  resolved** (now cites "copy verbatim from
  chip-resolved-colours.spec.ts:4-42" — strong directive, but bodies
  still not inlined; minor remains).
- 🔵 **Test Coverage**: `void parseRgb` dead line — **Resolved**.
- 🔵 **Test Coverage**: `setTheme` in invariance describe — **Resolved**.
- 🔵 **Test Coverage**: `.groupItem` border-width invariance —
  **Resolved**.
- 🔵 **Test Coverage**: Up-front anchor coverage count — **Resolved**.
- 🔵 **Test Coverage**: RelatedArtifacts.test.tsx promoted to §6 —
  **Resolved**.
- 🔵 **Code Quality**: AC #2 fixture coverage gap (no compile-time
  guard) — **Resolved** (the `toHaveCount(DOC_TYPE_KEYS.length)`
  assertion with naming error message provides this).
- 🔵 **Standards lens**: **0 findings** — all standards concerns
  resolved.

### New Issues Introduced / Newly Discovered

**Critical (both fixture-mechanism mismatches — surfaced by code
verification this pass):**

- 🔴 **Correctness**: Phase 5 §3 anchor `targets:` frontmatter
  contract does not exist in the backend.
  `server/src/indexer.rs:611-643` supports only (a) plan-review
  `target:` (singular) and (b) work-item refs via `work_item_id` /
  `parent` / `related`. There is no generic `targets:` plural list,
  no typed `<docType>:<slug>` link syntax, and no path from
  work-item frontmatter to a declared-outbound row of arbitrary doc
  type. The AC #2 coverage strategy as designed cannot surface
  declared-outbound rows of plans, research, decisions, notes,
  validations, etc. from a work-item anchor. The Phase 3 backend-
  verification step was added in pass 3 — pass 4 verified, and the
  contract isn't there.
- 🔴 **Correctness**: Phase 5 §3 mirror fixture's "shares the
  anchor's slug stem `0099`" claim is incompatible with the actual
  clustering algorithm. `slug.rs:48-55` strips the numeric prefix;
  `clusters.rs:34-67` and `related.rs:56-71` then match on **exact
  post-prefix slug**. Two work-items with the same `0099` prefix but
  different post-prefix slugs (e.g. `…-anchor` vs `…-mirror`) do not
  cluster. The mirror fixture must share the anchor's POST-prefix
  slug — typically meaning it must be a different doc type with the
  same slug (e.g. `plans/2026-05-26-related-coverage-anchor.md`).

**Major:**

- 🟡 **Test Coverage**: Phase 1 §5 enumerates assertion updates but
  misses the inline `style.color` assertion at `Glyph.test.tsx:185`
  which hardcodes ``var(--ac-doc-${docType})``. After widening the
  loop to `DOC_TYPE_KEYS`, every templates × {16, 24, 32} case will
  fail this assertion. Suggestion: add a bullet to §5 updating line
  185 to `expect(svg!.style.color).toBe(DOC_TYPE_COLOR_VAR[docType])`.
- 🟡 **Correctness**: `VirtualDocTypeKey = typeof
  VIRTUAL_DOC_TYPE_KEYS[number]` degenerates to `DocTypeKey` because
  `api/types.ts:30` explicitly annotates the constant as
  `readonly DocTypeKey[]`. The alias provides no narrowing.
  Suggestion: either drop the explicit annotation (so `as const`
  narrows to `readonly ['templates']`) OR define
  `VirtualDocTypeKey = 'templates'` as a direct literal-union alias.

**Minor:**

- 🔵 **Correctness**: Phase 1 §2a's stated parity invariant for
  `ColorTokenKey` (`'asserts light/dark have identical key sets'`) is
  factually inverted — `global.test.ts:185` asserts `ac-violet` is
  light-only. The compile result still works (all DOC_TYPE_TOKEN_KEY
  values exist in both maps), but the docstring rationale is wrong.
- 🔵 **Architecture / Correctness**: Phase 5 §1 says "the existing
  wrapping `<section>` (around line 104)" but LibraryDocView has two
  sibling sections under `.aside` (Related artifacts at :104, File at
  :122). Suggestion: tighten phrasing to "the first `<section>`
  inside `.aside` — the one whose `<h3>` reads 'Related artifacts'".
- 🔵 **Code Quality / Test Coverage / Usability**: Phase 4 helper-
  module snippet still uses `/* copy from chip-resolved-colours.spec.ts */`
  placeholders. The strong directive mitigates risk but inlining the
  three function bodies (<40 lines total) would eliminate the last
  copy-by-instruction hop.
- 🔵 **Code Quality**: Phase 1 §5 `_typeContractGuards` retention is
  "optional" rather than decided. Pick one: delete entirely or rename
  and document.
- 🔵 **Code Quality**: `isPhysicalDocTypeKey` returns `boolean` rather
  than a TS type predicate. Current call sites don't need narrowing,
  but the signature locks in a less expressive contract. Suggestion:
  `key is Exclude<DocTypeKey, VirtualDocTypeKey>`.
- 🔵 **Code Quality**: `DOC_TYPE_COLOR_VAR` derivation via
  `Object.fromEntries(...) as Record<DocTypeKey, string>` requires two
  `as` casts. Marginal risk of silent widening on refactor.
- 🔵 **Test Coverage**: Container-invariance `.first()` row locator
  is technically uniform across all `.groupItem` instances but
  silently couples to DOM order. Suggestion: scope to a specific
  group or add a comment.
- 🔵 **Test Coverage**: Templates row coverage in the per-target loop
  is unconditional — if the backend gap (critical #1) means templates
  rows can't be surfaced, the colour-equality test will fail rather
  than produce a clearly-labelled "backend doesn't support" boundary.
- 🔵 **Architecture**: Inferred-cluster fixture choreography (now
  invalidated by critical #2) was implicitly coupled to the slug-stem
  pairing assumption. Even with critical #2 fixed, the plan should
  cite the clustering mechanism by file/line so future server-side
  refactors surface the dependency.
- 🔵 **Architecture / Usability**: Phase 5 prerequisite backend-
  contract verification lives under Phase 3 Success Criteria. Phase
  5's Overview should explicitly name it as an entry gate.
- 🔵 **Architecture**: `data-testid="related-artifacts"` on the
  LibraryDocView section includes Legend / loading / error states,
  not just rows. Future SVGs in that scope could pollute the spec.
  Suggestion: name more honestly or move the testid to a tighter
  scope.
- 🔵 **Usability**: Phase 5 §3 names three coordinating fixtures but
  doesn't exemplify the inbound/mirror frontmatter shapes. Bound up
  with critical #1 anyway — both need a redesign.
- 🔵 **Usability**: Phase 5 §6 (Vitest update — write before §4) is
  buried at §6 in document order. Implementer may invert TDD ordering.
- 🔵 **Suggestion (Usability)**: Phase 1 §3 console.warn enumeration
  could use `DOC_TYPE_KEYS.join(', ')` rather than a hand-edited
  string.

### Assessment

The plan has improved continuously across four passes. The Standards
lens is now clean. Architecture, Code Quality, and Usability findings
are all minors. The two new criticals reflect deeper code verification
than prior passes performed — the AC #2 fixture choreography section
has been wrong since Phase 5 was first authored, but lenses didn't
catch it until pass 4 actually read the backend code.

**The Phase 5 fixture strategy needs a real redesign**, not just a
prose edit:

- AC #2 requires asserting a colour-tinted row for every doc type. The
  existing backend can produce:
  - Declared-outbound: 0 or 1 row, depending on anchor doc type
    (`target:` on plan-reviews; no other declared-outbound mechanism
    surfaces non-work-item targets).
  - Declared-inbound: any number, via work-item-ref frontmatter in
    *other* fixtures pointing at the anchor's work-item id.
  - Inferred: any number sharing the anchor's post-prefix slug.
- A realistic strategy may be: pick anchor as a plan-review (gets one
  declared-outbound), have 12 inbound work-items declaring the anchor
  in their refs (declared-inbound for all work-items), use sibling
  fixtures with matching post-prefix slugs for inferred (multiple doc
  types possible). Templates rows may not be reachable via declared
  edges — would require server changes OR descoping templates row
  coverage from AC #2.

The other remaining findings (`VirtualDocTypeKey` annotation,
`Glyph.test.tsx:185` line, parity-invariant docstring, line-:104
disambiguation, etc.) are all one-line fixes.

This is the first pass where the verdict is REVISE because of
**design-level concerns** rather than implementation-detail concerns.
The plan is otherwise high quality; the AC #2 strategy is the
remaining substantive item.

## Re-Review (Pass 5) — 2026-05-26

**Verdict:** REVISE

The pass-4 AC #2 redesign (inferred-cluster sibling fixtures) is
directionally correct and resolves the two pass-4 criticals (the
non-existent `targets:` contract and the slug-stem clustering
misconception). The new strategy uses only the well-tested
inferred-cluster mechanism. However, this pass's correctness and
standards agents verified the new fixture filenames against
`server/src/slug.rs` and found **two critical slug-derivation bugs**
in the redesign itself, plus a typecheck-breaking interaction with
the Phase 1 `VIRTUAL_DOC_TYPE_KEYS` narrowing. All three are
line-level fixes, now applied. The Standards lens (which errored on
first attempt and was re-run) returned one new minor.

### Previously Identified Issues (pass 4 → pass 5)

**Resolved:**
- 🔴🔴 **Correctness**: Both pass-4 criticals (anchor `targets:`
  contract doesn't exist; slug-stem clustering misconception) —
  **Resolved** by the inferred-cluster redesign that uses exact
  post-prefix slug equality (the real mechanism) and 12 sibling
  fixtures.
- 🟡 **Correctness**: `VirtualDocTypeKey` degenerate alias —
  **Resolved** (`as const satisfies readonly DocTypeKey[]`).
- 🟡 **Test Coverage**: `Glyph.test.tsx:185` hardcoded prefix —
  **Resolved** (now `DOC_TYPE_COLOR_VAR[docType]`).
- 🔵 **Code Quality**: `isPhysicalDocTypeKey` boolean vs predicate —
  **Resolved** (now a type predicate).
- 🔵 **Code Quality**: `DOC_TYPE_COLOR_VAR` double-cast — **Resolved**
  (direct typed literal).
- 🔵 **Code Quality**: `_typeContractGuards` undecided — **Resolved**
  (deleted).
- 🔵 **Correctness**: `ColorTokenKey` docstring inverted — **Resolved**.
- 🔵 **Usability**: console.warn manual enumeration — **Resolved**
  (`DOC_TYPE_KEYS.join(', ')`).
- 🔵 **Usability**: Phase 5 §6 TDD ordering — **Resolved** (explicit
  "Implementation order" bullet).
- 🔵 **Architecture**: line-:104 section disambiguation — **Resolved**
  (and the section testid was subsequently dropped as redundant).
- Standards lens (pass 4): remained clean except the one new minor below.

### New Issues Discovered (all now fixed in the plan)

**Critical (verified against `slug.rs` source):**
- 🔴 **Correctness**: pr-descriptions ac2 fixture was named
  `prs/0099-ac2-coverage.md`, but `PrDescriptions` uses
  `strip_prefix_date` (requires `YYYY-MM-DD-`), so a numeric prefix
  yields slug `None` and the fixture drops out of the cluster —
  failing `toHaveCount(11)`. **Fixed**: renamed to
  `prs/2026-05-26-ac2-coverage.md`, with an inline slug-derivation
  note added to §3.
- 🔴 **Correctness**: `DETAIL_ROUTE_SLUGS['design-inventories']` was
  `2026-05-26-example`, matching neither the entry slug (`example`,
  parent-dir with date stripped) nor the file slug (`inventory`). The
  design-inventories detail route would render "Document not found",
  failing Phase 3 + Phase 4 specs. **Fixed**: set to `example`. The
  Phase 3 work-item-reviews fixture (`0001-example.md`) had the same
  class of bug (no date prefix → slug `None`); **fixed**: renamed to
  `2026-05-26-example-review-1.md`, slug `example`.

**Major:**
- 🟡 **Correctness / Test Coverage**: The Phase 5/6 spec helpers used
  inline `DOC_TYPE_KEYS.filter((k) => !VIRTUAL_DOC_TYPE_KEYS.includes(k))`.
  After Phase 1 narrows `VIRTUAL_DOC_TYPE_KEYS` to `readonly
  ['templates']`, `.includes(k)` rejects a `DocTypeKey` argument at
  typecheck. **Fixed**: both specs now use
  `DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey)` — reusing the
  production predicate (also resolves the duplicate-filter-constant
  usability finding).
- 🟡 **Test Coverage**: The inferred-cluster row count is silently
  sensitive to sibling fixture cross-refs — any `ac2-coverage` fixture
  carrying a ref key (`work_item_id`/`related`/`target`/etc.) matching
  a sibling would be deduped out of the inferred cluster
  (`related.rs:91-102`), breaking the count. **Fixed**: §3 now carries
  a "Critical fixture constraint — no cross-references" note requiring
  all 12 fixtures to be ref-free, with the ownership comment updated.

**Minor:**
- 🔵 **Standards**: Phase 3 work-item-reviews fixture filename didn't
  follow the review naming convention — **Fixed** (same rename as the
  critical above).
- 🔵 **Correctness**: inferred group's visible heading is "Same
  lifecycle", not "Inferred cluster" — **Fixed** (clarifying note
  added; spec scopes by testid regardless).
- 🔵 **Correctness / Architecture**: `RelatedGroup`'s prop signature
  `{ label, entries, kind }` had no `testId` — **Fixed** (§4 now shows
  the explicit `testId?: string` prop addition).
- 🔵 **Architecture / Usability**: section-level `related-artifacts`
  testid was added in §1 but no spec consumed it (the finer
  `related-group-*` testids supersede it) — **Fixed** (§1 rewritten to
  state no section testid is needed; the two prose references
  repointed to `related-group-inferred`).

**Not fixed (accepted minor tradeoffs):**
- 🔵 **Usability / Test Coverage**: Phase 4 helper bodies still shown
  as "copy from chip-resolved-colours.spec.ts:4-42" placeholders
  (strong directive; <40 lines). Accepted — inlining is optional.
- 🔵 **Architecture**: declared-group testids are wired but only
  asserted at the Vitest layer (§6 synthetic fixture), not e2e.
  Accepted — the §6 assertion covers the labelling contract.
- 🔵 **Code Quality**: `DOC_TYPE_COLOR_VAR` is a hand-written parallel
  literal to `DOC_TYPE_TOKEN_KEY` (chosen to avoid the `Object.fromEntries`
  cast). Accepted tradeoff — the `Record<DocTypeKey,…>` type catches
  dropped keys.
- 🔵 **Usability**: work-items special-case in the Phase 5 loop is
  intrinsic to "anchor's own type isn't in its own cluster". Accepted;
  the separate test is clearly commented.

### Assessment

Pass 5 is the verification pass that caught the implementation-level
errors in pass 4's design-level redesign — exactly the right division
of labour. The two criticals were genuine slug-derivation bugs in the
new fixture filenames (the redesign was sound; the per-type filename
arithmetic had two mistakes), and the typecheck-breaking spec-filter
interaction was a real consequence of the Phase 1 narrowing. All have
been fixed in the plan with source-cited slug-derivation notes.

The remaining open items are all accepted minor tradeoffs. After this
pass's fixes, the plan's correctness, standards, architecture, code
quality, and usability concerns are resolved or consciously accepted.
A pass-6 spot-check of the applied fixes is reasonable but not
strictly required — the changes are mechanical and source-verified.
The plan is now **ready to implement**.

## Re-Review (Pass 6) — 2026-05-26

**Verdict:** APPROVE

Final verification pass. All six lenses (architecture, code-quality,
test-coverage, correctness, standards, usability) returned **zero
findings**. Each agent independently traced the pass-5 fixes through
the actual source and confirmed them correct.

### Verified Pass-5 Fixes

- ✅ **pr-descriptions fixture** `prs/2026-05-26-ac2-coverage.md` →
  `strip_prefix_date` → slug `ac2-coverage` (correctness + test-coverage
  traced through `slug.rs:27,57`).
- ✅ **work-item-reviews fixture** `reviews/work/2026-05-26-example-review-1.md`
  → date strip + `-review-N` strip → slug `example`; matches
  `DETAIL_ROUTE_SLUGS['work-item-reviews']` (`slug.rs:30-33`).
- ✅ **design-inventories** `DETAIL_ROUTE_SLUGS = 'example'` → nested
  manifest parent-dir `2026-05-26-example` → `strip_prefix_date` →
  `example`, matches `e.slug` (`indexer.rs:998-1016`,
  `LibraryDocView.tsx:50-51`).
- ✅ **Spec filters** `DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey)`
  typecheck against the narrowed `VIRTUAL_DOC_TYPE_KEYS` (predicate
  accepts `DocTypeKey`).
- ✅ **Ref-free fixture constraint** is load-bearing and correctly
  defends the `toHaveCount(11)` count against the declared-overlap
  dedup (`related.rs:91-102`).
- ✅ **RelatedGroup `testId?: string` prop** matches the existing
  3-prop signature (`RelatedArtifacts.tsx:73-77, 85`).
- ✅ **Section-testid drop** sound — group-level testids on the
  existing `.group` div fully scope the spec.
- ✅ **`toHaveCount(11)` arithmetic**: `PHYSICAL_DOC_TYPE_KEYS.length
  - 1 = 11`; cluster self-excludes the anchor (`related.rs:56-67`);
  all 12 non-virtual types covered exactly once (11 via work-items
  anchor + work-items via decisions anchor).
- ✅ **`ColorTokenKey = ColorTokenLight & ColorTokenDark`** is
  non-empty and admits `ac-fg-muted` (present in both theme tables) —
  Phase 1 typechecks.

### Strengths (synthesised across lenses)

- Single-consumer funnel through `Glyph` + data-driven
  `DOC_TYPE_COLOR_VAR` keeps the 0073 brand-layer rewrite to one
  retarget surface.
- Virtual-key knowledge centralised in `api/types.ts`
  (`isPhysicalDocTypeKey` + `VIRTUAL_DOC_TYPE_KEYS`).
- Compile-time exhaustiveness via `Record<DocTypeKey, …>` at three
  production definition sites — stronger than the deleted
  `_typeContractGuards` artifact.
- Vitest/Playwright split is sound: type + render contracts at Vitest,
  computed-style colour resolution at Playwright.
- Accessibility handled correctly (decorative `aria-hidden` glyphs);
  dark-theme cue-collapse and AC #2 templates descope both explicitly
  recorded with ownership boundaries.

### Assessment

The plan converged cleanly over six passes: 32 → 25 → 23 → 18 → 12
findings → **0**. The verdict is APPROVE. Two open documentation
followups remain (non-blocking): update the work-item 0074 AC #2
wording to "12 non-virtual doc types" per the Phase 5 descope, and the
accepted minor tradeoffs noted in pass 5 (helper-body placeholders,
hand-written `DOC_TYPE_COLOR_VAR` literal, work-items loop
special-case). The plan is ready for `/implement-plan`.
