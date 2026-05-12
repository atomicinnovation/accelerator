---
date: 2026-05-12T10:14:12Z
type: plan
skill: create-plan
work-item: "0037"
status: approved
---

# Glyph Component Implementation Plan

## Overview

Land a `Glyph` React component in the visualiser frontend that renders a per-doc-type icon at 16/24/32 px with theme-aware fill colour, plus the supporting `--ac-doc-<key>` token sub-namespace, a `/glyph-showcase` developer route, a Vitest smoke test covering all 36 (docType × size) combinations, a Playwright visual-regression spec (with per-cell clips for per-icon coverage) and a resolved-hex `getComputedStyle` test, and an expanded frontend README that documents prerequisites, project layout, developer routes, and visual-regression troubleshooting. Scope is bounded by work item 0037: consumer integration is delivered by downstream work items (0036/0040/0041/0042/0043/0053/0054/0055), not here.

The plan follows TDD where the tooling supports it cleanly: token parity tests are driven by extending `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` (the parity tests in `global.test.ts` iterate those exports, so adding entries automatically generates failing tests that the CSS edits then satisfy); Glyph's type contract, runtime contract, and accessibility behaviour are each tested before implementation; the per-doc-type icon components are written with red-then-green DOM assertions on the dispatched SVG content. Visual fidelity (the actual SVG path geometry derived from the canonical screenshots) is the only step where the test acts as a regression baseline rather than a fail-first driver — Playwright per-cell visual-regression and a real-engine `getComputedStyle` assertion backstop it.

### Consumer Contract

Downstream work items (0036/0040/0041/0042/0043/0053/0054/0055) MUST adhere to the following invariants when consuming Glyph:

1. **Do not override `fill`** on Glyph or any ancestor that targets it via CSS. Glyph drives colour through `color: var(--ac-doc-<key>)` on the `<svg>` and `fill="currentColor"` on children; an override on `color` would tint, an override on `fill` would break the theme contract.
2. **Provide an adjacent text label OR pass `ariaLabel`** for any Glyph used as a standalone visual without nearby text. The default render is `aria-hidden="true"` and assumes a sibling text label is present in the consumer's layout.
3. **Do not wrap Glyph in another `<svg>`**. Glyph owns the `<svg>` boundary.
4. **Sizes are restricted to 16/24/32**. If a consumer needs an off-grid size (e.g. 20 px for avatar contexts), open a PR to widen the union with a documented specimen rather than casting.
5. **Narrow `DocTypeKey` to `GlyphDocTypeKey` via the provided `isGlyphDocTypeKey()` guard or `GLYPH_DOC_TYPE_KEYS` array** — both exported from `Glyph.tsx`. Do not reinvent the filter, do not use `as` casts.

## Current State Analysis

The infrastructure 0037 builds on is already in place:

- **Type contract** — `DocTypeKey` is a closed 13-key union at `skills/visualisation/visualise/frontend/src/api/types.ts:4-8`; `templates` is the only `virtual: true` key (`types.ts:26-36`). `GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>` is the 12-key subset Glyph accepts.
- **Token layer** — `--ac-*` tokens are declared in three CSS blocks (light `:root`, explicit `[data-theme="dark"]` = MIRROR-A, `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])` = MIRROR-B) in `src/styles/global.css:66-205`, mirrored in `src/styles/tokens.ts:1-47`. There is no per-doc-type `--ac-doc-<key>` sub-namespace today.
- **Parity enforcement** — `src/styles/global.test.ts:59-95` iterates each frozen export and asserts `--<name>` in `global.css` matches the TS value; lines 125-163 byte-compare MIRROR-A vs MIRROR-B. `src/styles/migration.test.ts` enforces "every `var(--NAME)` reference resolves to a declared token". Adding to `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` automatically generates new test cases.
- **Theming** — `useTheme` writes `data-theme` to `document.documentElement` (`src/api/use-theme.ts:30-32`); Glyph reads `color: var(--ac-doc-<key>)` from the cascade and propagates it via `fill="currentColor"` on children, without holding any React state.
- **Component conventions** — Co-located `<Name>/<Name>.tsx` + `.module.css` + `.test.tsx`, named exports, no barrels, inline JSX SVGs. `Brand.tsx:15-31` is the canonical example of writing `fill="var(--ac-accent-2)"` and `stroke="var(--ac-accent)"` directly as SVG attribute literals; `Brand.test.tsx:11-19` shows the matching attribute-assertion pattern. ARIA prop naming convention is `ariaLabel` per `TopbarIconButton.tsx:6`.
- **Routing** — TanStack Router, imperative `createRoute(...)` style; tree assembled at `src/router.ts:139-151`. `kanbanRoute` (line 132) is the closest precedent for a top-level non-crumbed route.
- **Visual regression** — `tests/visual-regression/tokens.spec.ts:9-39` is the project template: a `ROUTES` array, a theme loop, `maxDiffPixelRatio: 0.05`, theme swap via `document.documentElement.dataset.theme = 'dark'` + `requestAnimationFrame` yield. Per-platform baselines (`-darwin.png` + `-linux.png`) live under `tests/visual-regression/__screenshots__/<spec>.ts-snapshots/`. The current spec captures one screenshot per route × theme; this plan extends with per-cell clipped screenshots so individual icon regressions can't hide under a viewport-wide diff budget.
- **Canonical screenshots** — `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-{light,dark}.png` exist and are the source of truth for icon shape and per-doc-type fill colour.

The pieces **missing** that this plan creates:

- `skills/visualisation/visualise/frontend/README.md` does not exist (created in Phase 5).
- `package.json` has no `typecheck` script — only `build` runs `tsc -b`. This means `@ts-expect-error` directives in tests are not run by `npm test` today. Phase 2 adds an explicit `"typecheck": "tsc --noEmit"` script.

### Key Discoveries:

- Parity tests in `global.test.ts:67-73` are driven by iterating `DARK_COLOR_TOKENS`, so growing that export automatically expands test coverage. Same for the light side. Token additions therefore need no test edits — only CSS edits to satisfy the new auto-generated cases.
- The MIRROR-A↔MIRROR-B byte-equivalence test (`global.test.ts:125-163`) catches the "added in one block, forgot the other" failure mode at unit-test time, not at visual-regression time.
- `Brand.tsx:15-31` proves SVG presentation attributes accept `var(--ac-*)` literals in this codebase. Glyph deliberately uses a different pattern — `color: var(...)` on the `<svg>` + `fill="currentColor"` on children — so that any icon that mistakenly hard-codes a `fill` on a child fails loudly visually rather than silently breaking the theme contract.
- The codebase uses `ariaLabel` (camelCase mirror of `aria-label`) as the prop name for ARIA labels on icon-bearing components (`TopbarIconButton.tsx:6`, used by `ThemeToggle` and `FontModeToggle`). Glyph follows this convention. The work item AC text uses `accessibleLabel`; this plan deliberately departs and the work item AC is updated in lock-step.
- `AC #4` and `AC #8` in the work item assert `getComputedStyle(glyphSvg).fill` resolves to a hex. JSDOM's CSS engine does not reliably substitute `var()` inside SVG presentation attributes. The Vitest contract therefore lands as `getAttribute('fill') === 'currentColor'` plus an `expect(svg.style.color)` check on the inline style attribute; the resolved-hex check is verified by a dedicated Playwright `page.evaluate(getComputedStyle)` test in Phase 4 (the showcase-route visual-regression spec is separate). This is recorded as the canonical interpretation of those ACs in this plan and the work item Resolved Decisions.
- The user-chosen icon packaging is **per-doc-type component files** under `src/components/Glyph/icons/<DocType>Icon.tsx`. Glyph imports and dispatches. This keeps each icon's path geometry isolated and easy to diff against the canonical screenshot during review.

## Desired End State

After this plan is complete, the following is true:

- `Glyph` is importable from `src/components/Glyph/Glyph.tsx`, exposes a typed `GlyphProps` with `docType: GlyphDocTypeKey`, `size: 16 | 24 | 32`, optional `ariaLabel?: string`, and renders an `<svg>` whose CSS `color` is `var(--ac-doc-<key>)` and whose child paths inherit via `fill="currentColor"`.
- The module also exports `GLYPH_DOC_TYPE_KEYS: readonly GlyphDocTypeKey[]` (filtered from `DOC_TYPE_KEYS`) and `isGlyphDocTypeKey(k: DocTypeKey): k is GlyphDocTypeKey` for downstream consumers to narrow safely.
- In development builds, Glyph emits a `console.warn` and returns `null` if `docType` falls outside the dispatch record (e.g. from an `as` cast or an untyped data source). In production builds, it silently returns `null`.
- All 12 non-virtual doc types render distinct, recognisable icons at all 3 sizes (verified by Vitest smoke test + per-cell Playwright clips).
- `tokens.ts` and `global.css` (all three theme blocks) declare 12 new `--ac-doc-<key>` tokens with light + dark hex values eyedroppered from the canonical screenshots, and verified to meet WCAG 1.4.11 (3:1 contrast against `--ac-bg`). All parity tests pass.
- `/glyph-showcase` route renders a 12×3 grid with both kebab-case keys and friendly labels; viewable under `data-theme="light"` and `data-theme="dark"`.
- `tests/visual-regression/glyph-showcase.spec.ts` exists, captures one screenshot per (cell × theme) at tight bounds (36 cells × 2 themes), and passes against committed darwin + linux baselines at `tests/visual-regression/__screenshots__/glyph-showcase.spec.ts-snapshots/`. A separate `glyph-resolved-fill.spec.ts` asserts `getComputedStyle().color` resolves to the canonical light/dark hex for a representative Glyph per theme.
- `skills/visualisation/visualise/frontend/README.md` exists with Overview / Prerequisites / Project Layout / Development / Testing / Building / Developer Routes / Visual-Regression Baselines sections; `/glyph-showcase` is listed under Developer Routes.
- `package.json` includes a `"typecheck": "tsc --noEmit"` script; both Phase 2's `@ts-expect-error` directives and CI's gate reference it.

Verification: `npm test`, `npm run typecheck`, and `npm run test:e2e` all pass; the showcase renders correctly when manually toggling theme; downstream work items can safely import `{ Glyph, type GlyphDocTypeKey, GLYPH_DOC_TYPE_KEYS, isGlyphDocTypeKey }` without surfacing the underlying icon-component files.

## What We're NOT Doing

- **Consumer integration.** Glyph is not threaded through Sidebar nav, page eyebrows, kanban cards, lifecycle steps, activity rows, search results, or template-type indicators — those are 0036/0040/0041/0042/0043/0053/0054/0055.
- **`templates` doc type.** `GlyphDocTypeKey` deliberately excludes it. No fallback rendering for virtual keys.
- **A new `--ac-doc-*` tint family or `color-mix()` derivations.** ADR-0026 reserves `color-mix()` for tint/hover/border surfaces; Glyph uses solid foreground fills. Tints are out of scope.
- **Off-grid sizes** (e.g. 20 px, 28 px). The strict `16 | 24 | 32` union is enforced by the work item AC. Resolution path for a new size: open a PR to widen the union with a new specimen captured in the design inventory — not a cast at the consumer.
- **Forced-colors fallback.** Windows High Contrast users will see uniform colour across glyphs, but shape-distinct icons satisfy WCAG 1.4.1 (use of colour) so the doc-type signal is preserved via geometry. WCAG 1.4.11 (non-text contrast) is satisfied in standard mode but is not separately re-verified for forced-colors. Tracked as a follow-up consideration but not delivered here.
- **Storybook / Histoire / Ladle.** No story-based artefact — the `/glyph-showcase` route is the showcase deliverable.
- **Changes to `DocType` shape or `LIFECYCLE_PIPELINE_STEPS`.** The per-doc-type colour map lives co-located with Glyph; `DocType` keeps its current `{ key, label, virtual }` shape.
- **A `GlyphProvider` or hook-based theme detection inside Glyph.** Theme switching is handled by the CSS cascade reading `data-theme`; Glyph holds no state.
- **`getComputedStyle(svg).fill === '<resolved-hex>'` inside Vitest.** This assertion moves to a Playwright spec (`glyph-resolved-fill.spec.ts`) that calls `page.evaluate(() => getComputedStyle(svg).color)` against a real Chromium engine. See Phase 4 for rationale.

## Implementation Approach

Five phases, each independently shippable as a commit. TDD order within each phase. Phase 2 is the largest (contract + all 12 icons with real geometry); the prior plan's Phase 3 has been folded in so no commit ever advertises a working Glyph backed by placeholder rectangles.

1. **Phase 1**: Per-doc-type colour tokens — viewBox grid measurement, eyedropper, contrast check, tokens.ts + global.css.
2. **Phase 2**: Glyph type contract, dispatch shell, accessibility behaviour, runtime guard, AND 12 icon components with real screenshot-derived geometry (single shippable commit).
3. **Phase 3**: Vitest smoke test covering 12 × 3 = 36 combinations + ariaLabel a11y semantics.
4. **Phase 4**: `/glyph-showcase` route + Playwright per-cell visual-regression spec + Playwright resolved-fill spec.
5. **Phase 5**: Frontend README.

Phases 3 and 4 are the two halves of "test the implementation"; they stay separate because Vitest covers the type/runtime contract and Playwright covers the visual + computed-style contract — three distinct quality lenses on the same artefact.

---

## Phase 1: Per-doc-type colour tokens

### Overview

Measure the canonical screenshot's glyph grid (to confirm the 24×24 viewBox assumption), sample hex values from the screenshots for all 12 doc types in both light and dark themes, verify WCAG 1.4.11 contrast against `--ac-bg`, then extend `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` and `global.css` (all three theme blocks) with the new `--ac-doc-<key>` declarations. Parity tests pick up the new entries automatically.

### Changes Required:

#### 0. Add static virtual-key and label exports to `api/types.ts`

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts`
**Changes**: `DocType` records currently come from the server at runtime (`useQuery` in `RootLayout.tsx`); there is no module-load-time `DOC_TYPES` array. Glyph's exhaustiveness assertion and the showcase label lookup both need a static source. Add two new exports next to `DOC_TYPE_KEYS`:

```ts
/** Doc-type keys that are virtual (not backed by on-disk documents). Static
 *  mirror of the server's `virtual: true` flag on `DocType`. If a future
 *  virtual key is added here, Glyph's `GLYPH_DOC_TYPE_KEYS` automatically
 *  filters it out. Keep this list in lock-step with the server's
 *  `DocType.virtual` flag (see `RootLayout.tsx` useQuery). */
export const VIRTUAL_DOC_TYPE_KEYS: readonly DocTypeKey[] = ['templates'] as const

/** Static, human-friendly labels for each `DocTypeKey`. Mirrors the
 *  server-emitted `DocType.label` field; used in dev-only routes (e.g.
 *  `/glyph-showcase`) and tests where a runtime `useQuery` is undesirable. */
export const DOC_TYPE_LABELS: Readonly<Record<DocTypeKey, string>> = {
  'decisions': 'Decision',
  'work-items': 'Work item',
  'plans': 'Plan',
  'research': 'Research',
  'plan-reviews': 'Plan review',
  'pr-reviews': 'PR review',
  'work-item-reviews': 'Work item review',
  'validations': 'Validation',
  'notes': 'Notes',
  'prs': 'PR',
  'design-gaps': 'Design gap',
  'design-inventories': 'Design inventory',
  'templates': 'Template',
}
```

Add a Vitest assertion that the keys of `DOC_TYPE_LABELS` exactly equal `DOC_TYPE_KEYS` (catches drift if a key is added to the union but not the label record).

#### 1. Measure the canonical glyph grid

Before sampling hex values, measure one well-defined glyph (e.g. the `decisions` glyph) in `library-view-updated-light.png` to determine the pixel cell its bounding box occupies. Confirm whether the source grid is 24×24, 16×16, 20×20, or another size. If it is not 24×24, either:

- Adjust the planned `viewBox` to match (e.g. `0 0 16 16`), recorded as a comment on the literal in `Glyph.tsx`; OR
- Confirm with the designer that 24×24 is the intended target viewBox and that Phase 2 paths should be hand-fitted to that box from the source grid.

Record the measured grid in the work item Resolved Decisions and as a comment in `Glyph.tsx` near the `viewBox` literal.

#### 2. Eyedropper hex values

For each of the 12 doc types, identify the centre-pixel coordinate of the corresponding glyph in `library-view-updated-light.png` and `…-dark.png`. **Choose an `(x, y)` that demonstrably lies inside the glyph's opaque solid fill region** (verify by checking alpha is fully opaque before sampling; avoid stroke pixels and anti-aliased edges). For glyphs whose centre falls on a thin stroke or transparent inner region, pick an alternative point inside the solid body. Sample with:

```sh
magick identify -format "%[hex:p{x,y}]" library-view-updated-light.png
magick identify -format "%[hex:p{x,y}]" library-view-updated-dark.png
```

Capture the 12 light hex + 12 dark hex values, the (x, y) coordinates used, and the alpha check outcome. Record in:

- Commit message body (work item AC line 81)
- A new `(x, y) sample` column added to the Colour Token Table in `meta/work/0037-glyph-component.md:47-60`

#### 3. WCAG 1.4.11 contrast check

For each sampled hex, compute the contrast ratio against the corresponding `--ac-bg` token in the same theme. The codebase already has `contrastRatio` in `src/styles/contrast.ts` (sibling to `global.test.ts`); reuse it directly. Add 24 assertions iterating the 12 new keys × 2 themes asserting contrast ≥ 3.0:

```ts
import { contrastRatio } from './contrast'
import { GLYPH_DOC_TYPE_KEYS } from '../components/Glyph/Glyph'

const BG_LIGHT = LIGHT_COLOR_TOKENS['ac-bg']
const BG_DARK = DARK_COLOR_TOKENS['ac-bg']
for (const key of GLYPH_DOC_TYPE_KEYS) {
  it(`light: ${key} contrast >= 3:1 vs --ac-bg`, () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS[`ac-doc-${key}`], BG_LIGHT)).toBeGreaterThanOrEqual(3)
  })
  it(`dark: ${key} contrast >= 3:1 vs --ac-bg`, () => {
    expect(contrastRatio(DARK_COLOR_TOKENS[`ac-doc-${key}`], BG_DARK)).toBeGreaterThanOrEqual(3)
  })
}
```

The existing `contrastRatio` handles 3-digit shorthand hex via `parseHex`, so no additional defensive code is needed.

For any failure, remap with designer sign-off (preferring shifts within the same hue family) and re-sample, OR document the exception explicitly in the work item with a rationale referencing SC 1.4.11.

#### 4. Token TS exports

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
**Changes**: Append 12 entries to `LIGHT_COLOR_TOKENS` (lines 1-25) and 12 to `DARK_COLOR_TOKENS` (lines 27-47). Key naming follows the existing kebab-case convention without the `--` prefix:

```ts
export const LIGHT_COLOR_TOKENS = {
  // …existing entries…
  'ac-doc-decisions':           '#…',
  'ac-doc-research':            '#…',
  'ac-doc-plans':               '#…',
  'ac-doc-plan-reviews':        '#…',
  'ac-doc-validations':         '#…',
  'ac-doc-prs':                 '#…',
  'ac-doc-pr-reviews':          '#…',
  'ac-doc-notes':               '#…',
  'ac-doc-work-items':          '#…',
  'ac-doc-work-item-reviews':   '#…',
  'ac-doc-design-gaps':         '#…',
  'ac-doc-design-inventories':  '#…',
} as const
```

Mirror the same 12 keys in `DARK_COLOR_TOKENS` with the dark hex values. The flat-record convention is preserved.

#### 5. CSS theme blocks

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
**Changes**: Add the 12 `--ac-doc-<key>: <hex>;` declarations to each of the three theme blocks: light `:root` (around lines 66-143), MIRROR-A `[data-theme="dark"]` (lines 145-173), MIRROR-B `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])` (lines 175-205). MIRROR-A and MIRROR-B must be byte-equivalent for the parity test in `global.test.ts:125-163`.

### Success Criteria:

#### Automated Verification:

- [ ] Token parity passes: `cd skills/visualisation/visualise/frontend && npx vitest run src/styles/global.test.ts`
- [ ] WCAG 1.4.11 contrast tests pass: same command picks up the new contrast block automatically
- [ ] Full unit-test suite still green: `npm test`
- [ ] Type check: `npx tsc --noEmit` (the `typecheck` script lands in Phase 2 but `tsc --noEmit` is invokable now)

#### Manual Verification:

- [ ] Visual sanity-check: open a browser dev-tools inspector on any existing page, query `getComputedStyle(document.documentElement).getPropertyValue('--ac-doc-decisions')`, confirm it returns the light hex.
- [ ] Toggle `document.documentElement.dataset.theme = 'dark'`, re-query, confirm the value switches to the dark hex.
- [ ] Confirm the Colour Token Table in the work item file has been updated with sampled hex values, (x, y) coordinates, and the measured viewBox grid (no `TBD` cells remain).

---

## Phase 2: Glyph contract, dispatch shell, accessibility, runtime guard, and real icon geometry

### Overview

Add `"typecheck": "tsc --noEmit"` to `package.json`. Define `GlyphDocTypeKey`, `GLYPH_DOC_TYPE_KEYS`, and `isGlyphDocTypeKey`. Write the Glyph component with its full public API (props, accessibility behaviour, runtime guard, `color`-driven theming) and create 12 per-doc-type icon component files **with real screenshot-derived SVG geometry, not placeholders**. Tests for type rejection, runtime DOM shape, accessibility branches, and runtime-guard behaviour are written first and drive the implementation.

The prior plan's Phase 3 has been folded into this phase so the commit landing Glyph also lands the real icons — no in-between commit advertises a working Glyph backed by indistinguishable placeholder rectangles.

### Changes Required:

#### 1. Add typecheck script

**File**: `skills/visualisation/visualise/frontend/package.json`
**Changes**: Add `"typecheck": "tsc --noEmit"` under `scripts`. Phase 2's Success Criteria and CI's gate both invoke this script so `@ts-expect-error` directives actually fire.

#### 2. Test the type contract (red)

**File**: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.test.tsx` (new)
**Changes**: Write a Vitest test file asserting:

```tsx
// @ts-expect-error — 'templates' is excluded from GlyphDocTypeKey
;<Glyph docType="templates" size={24} />
// @ts-expect-error — size 20 is not 16 | 24 | 32
;<Glyph docType="decisions" size={20} />
```

Plus runtime assertions:
- Root element is `<svg>` (no `<img>` fallback)
- `width` and `height` attributes match the requested `size`
- `viewBox` is present and matches the Phase 1 measured grid
- The `<svg>`'s inline `style.color` resolves to (or contains) `var(--ac-doc-<key>)` for the requested `docType`
- Every descendant element under the `<svg>` — walked via `Array.from(svg.querySelectorAll('*'))`, not `svg.children` — has `fill` either absent or exactly `"currentColor"`. No descendant carries an explicit hex `fill` (catches icons that nest paths inside `<g>` groups, where a direct-children walk would miss the inner paths). Same assertion applied to `stroke` if any icon uses stroked shapes.
- Default render carries `aria-hidden="true"` and neither `role` nor `aria-label`
- Render with `ariaLabel="Decision"` carries `role="img"` and `aria-label="Decision"` and does not carry `aria-hidden`
- Render with `ariaLabel=""` (explicit empty string) carries `role="img"` and `aria-label=""` and does not carry `aria-hidden` (branch on `!== undefined`, not truthiness)
- Render with `docType` cast to an unknown string in dev returns `null` and `console.warn` is called once with a message naming the unknown key

These tests fail at this step because Glyph.tsx does not exist yet.

#### 3. Define GlyphDocTypeKey + helpers (green)

**File**: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx` (new)
**Changes**: Co-locate the type alias and runtime helpers with the component:

```ts
import { type DocTypeKey, DOC_TYPE_KEYS, VIRTUAL_DOC_TYPE_KEYS } from '../../api/types'

/**
 * The 12 non-virtual `DocTypeKey` values Glyph renders.
 *
 * INVARIANT: Glyph is for real document types only. Virtual keys (currently
 * `templates`) are excluded by construction. The exclusion is data-driven:
 * the runtime `GLYPH_DOC_TYPE_KEYS` below filters by `VIRTUAL_DOC_TYPE_KEYS`,
 * so adding a future virtual key in `api/types.ts` automatically removes it
 * from Glyph's set. The type alias must be updated in lock-step (extend
 * `Exclude<DocTypeKey, ...>` to cover the new virtual key) — caught at unit-
 * test time by the exhaustiveness assertion below.
 */
export type GlyphDocTypeKey = Exclude<DocTypeKey, 'templates'>

/** Runtime mirror of `GlyphDocTypeKey`. Derived from `VIRTUAL_DOC_TYPE_KEYS`
 *  at module load — assumes the virtual-keys list is statically resolvable. */
export const GLYPH_DOC_TYPE_KEYS: readonly GlyphDocTypeKey[] = DOC_TYPE_KEYS.filter(
  (k): k is GlyphDocTypeKey => !VIRTUAL_DOC_TYPE_KEYS.includes(k),
)

/** Narrow `DocTypeKey` to `GlyphDocTypeKey`. Use in data-driven consumers. */
export function isGlyphDocTypeKey(k: DocTypeKey): k is GlyphDocTypeKey {
  return GLYPH_DOC_TYPE_KEYS.includes(k as GlyphDocTypeKey)
}
```

Add a unit assertion that `GLYPH_DOC_TYPE_KEYS.length === 12` AND `Object.keys(ICON_COMPONENTS).sort()` equals `[...GLYPH_DOC_TYPE_KEYS].sort()` — so a new virtual key added to `DocTypeKey` (filtered out of `GLYPH_DOC_TYPE_KEYS` automatically) or a missing icon component is caught at test time.

#### 4. Per-doc-type icon components with real geometry (green)

**Directory**: `skills/visualisation/visualise/frontend/src/components/Glyph/icons/` (new)
**Files**: One per doc type (12 files): `DecisionsIcon.tsx`, `ResearchIcon.tsx`, `PlansIcon.tsx`, `PlanReviewsIcon.tsx`, `ValidationsIcon.tsx`, `PrsIcon.tsx`, `PrReviewsIcon.tsx`, `NotesIcon.tsx`, `WorkItemsIcon.tsx`, `WorkItemReviewsIcon.tsx`, `DesignGapsIcon.tsx`, `DesignInventoriesIcon.tsx`.

Each file exports a named function component returning the **real, screenshot-derived** SVG content (paths, groups, etc.) — not a placeholder. Glyph owns the outer `<svg>` wrapper. Children MUST use `fill="currentColor"` (or omit `fill` entirely) so the cascade-driven `color` propagates correctly. Any hex literal in a child fill will fail the Phase 2 test that asserts no child carries an explicit hex.

```tsx
// DecisionsIcon.tsx
export function DecisionsIcon(): React.ReactElement {
  return (
    <>
      <path d="M..." fill="currentColor" />
      {/* …additional primitives… */}
    </>
  )
}
```

For each of the 12 icons:

1. Open `library-view-updated-light.png` and locate the corresponding glyph (centre coords recorded in Phase 1).
2. By visual inspection, reproduce the icon as a set of SVG path / rect / circle / polygon primitives inside the measured viewBox (24×24 by default, adjusted by Phase 1 measurement if different).
3. Use `fill="currentColor"` on each primitive.
4. Visually diff against `library-view-updated-light.png` at multiple zoom levels.
5. Confirm the icon is recognisable at the smallest (16 px) size — i.e. doesn't depend on detail invisible at that scale.

**Manual visual review of all 12 icons must be performed by someone other than the implementer** (record the reviewer in the work item) before Phase 4 baselines are committed.

#### 5. Glyph dispatch + accessibility + runtime guard (green)

**File**: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx`
**Changes**: Implement Glyph dispatching to the right icon component, owning the `<svg>` wrapper, color, size, and a11y attributes:

```tsx
import { DecisionsIcon } from './icons/DecisionsIcon'
// …11 more imports…

// Ordering matters for review: keys mirror the Colour Token Table in the work item.
const ICON_COMPONENTS: Record<GlyphDocTypeKey, React.ComponentType> = {
  decisions: DecisionsIcon,
  // …
}

export interface GlyphProps {
  docType: GlyphDocTypeKey
  size: 16 | 24 | 32
  /** Accessible label. If provided (including empty string), Glyph renders with role="img" + aria-label. If omitted (undefined), Glyph is decorative (aria-hidden). */
  ariaLabel?: string
}

/**
 * Render a per-doc-type icon at 16/24/32 px with theme-aware fill.
 *
 * **Consumer Contract** (downstream WIs 0036/0040/0041/0042/0043/0053/0054/0055):
 * 1. Do not override `fill` on Glyph or any ancestor that targets it via CSS.
 *    Glyph drives colour through `color: var(--ac-doc-<key>)` on the `<svg>` and
 *    `fill="currentColor"` on children; an override on `color` would tint, an
 *    override on `fill` would break the theme contract.
 * 2. Provide an adjacent text label OR pass `ariaLabel` for any Glyph used as a
 *    standalone visual without nearby text. The default render is `aria-hidden`
 *    and assumes a sibling text label is present.
 * 3. Do not wrap Glyph in another `<svg>`. Glyph owns the `<svg>` boundary.
 * 4. Sizes are restricted to 16/24/32. For off-grid sizes (e.g. 20 px), open a
 *    PR to widen the union with a documented specimen — do not cast.
 * 5. Narrow `DocTypeKey` to `GlyphDocTypeKey` via `isGlyphDocTypeKey()` or
 *    `GLYPH_DOC_TYPE_KEYS`. Do not reinvent the filter, do not use `as` casts.
 */
export function Glyph({ docType, size, ariaLabel }: GlyphProps): React.ReactElement | null {
  const Icon = ICON_COMPONENTS[docType]
  if (!Icon) {
    // Dev gate uses Vite's `import.meta.env.DEV` to match the codebase
    // convention (see `Breadcrumbs.tsx`). Production renders null silently —
    // adjacent text label remains intact; downstream consumers expecting a
    // visible icon should validate `docType` via `isGlyphDocTypeKey` first.
    if (import.meta.env.DEV) {
      console.warn(`[Glyph] Unknown docType: ${docType}. Expected one of: ${GLYPH_DOC_TYPE_KEYS.join(', ')}.`)
    }
    return null
  }
  const a11y = ariaLabel !== undefined
    ? { role: 'img' as const, 'aria-label': ariaLabel }
    : { 'aria-hidden': true as const }
  return (
    // viewBox literal matches the Phase 1 measured glyph grid (default 24×24).
    // The `color` / `currentColor` split is the AC #3/#4 contract surface: Glyph drives
    // theme via the CSS cascade (`color: var(--ac-doc-<key>)`) and children inherit via
    // `fill="currentColor"`. Any child overriding fill fails loudly visually rather than
    // silently breaking the theme contract.
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      style={{ color: `var(--ac-doc-${docType})` }}
      data-doc-type={docType}
      {...a11y}
    >
      <Icon />
    </svg>
  )
}
```

Notes:
- The dispatch record's value type is `React.ComponentType` (not `() => React.ReactElement`) — exhaustiveness is preserved via the `Record<GlyphDocTypeKey, ...>` key constraint, and the wider value type permits future icon components to take props (e.g. `title`) without an API break.
- The `data-doc-type` attribute is a CSS/test hook following the `PipelineDots.tsx:17-18` precedent.
- `Glyph.module.css` is **not created**. Empty-for-convention files are a smell; add one when Glyph genuinely needs a stylesheet.

#### 6. In-source pointer to the showcase

Add a single-line comment near the top of `Glyph.tsx`:

```ts
// Preview all 12 doc types × 3 sizes in both themes at /glyph-showcase (see frontend README).
```

This surfaces the showcase from the import surface, not just from the README.

### Success Criteria:

#### Automated Verification:

- [ ] Type contract: `npm run typecheck` confirms `@ts-expect-error` directives fire correctly
- [ ] Glyph unit tests pass: `npx vitest run src/components/Glyph/Glyph.test.tsx`
- [ ] Dispatch exhaustiveness: `GLYPH_DOC_TYPE_KEYS.length === Object.keys(ICON_COMPONENTS).length === 12`
- [ ] Every child element under each rendered Glyph SVG has `fill="currentColor"` or no `fill` attribute (no hex literals)
- [ ] Full unit-test suite green: `npm test`

#### Manual Verification:

- [ ] Spot-check `Glyph.tsx` import resolves cleanly from a sibling component (e.g. `import { Glyph, isGlyphDocTypeKey } from '../Glyph/Glyph'` in a scratch consumer)
- [ ] Each of the 12 icons visually matches its counterpart in `library-view-updated-light.png`
- [ ] Manual visual review of all 12 icons performed by someone other than the implementer (reviewer recorded in the work item)
- [ ] Each icon is recognisable at the smallest (16 px) size

---

## Phase 3: Vitest smoke test for all 36 combinations

### Overview

Add the explicit 12 × 3 = 36-combination smoke test required by AC line 74, plus richer a11y semantics tests using Testing Library's accessible-name resolution. The Phase 2 tests cover the contract for individual doc types; this phase makes the matrix coverage explicit and exercises real a11y semantics rather than literal attribute strings.

### Changes Required:

**File**: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.test.tsx`
**Changes**: Append a parametrised describe block iterating all 36 combinations and replace attribute-string a11y assertions with `getByRole`/`queryByRole`:

```tsx
import { render } from '@testing-library/react'
import { GLYPH_DOC_TYPE_KEYS } from './Glyph'

const SIZES = [16, 24, 32] as const

describe.each(GLYPH_DOC_TYPE_KEYS)('Glyph: %s', (docType) => {
  describe.each(SIZES)('size %s', (size) => {
    it('renders an SVG with correct dimensions and color var', () => {
      const { container } = render(<Glyph docType={docType} size={size} />)
      const svg = container.querySelector('svg')
      expect(svg).not.toBeNull()
      expect(svg!.getAttribute('width')).toBe(String(size))
      expect(svg!.getAttribute('height')).toBe(String(size))
      expect(svg!.style.color).toBe(`var(--ac-doc-${docType})`)
      expect(svg!.getAttribute('viewBox')).toBeTruthy()
    })
  })
})

// A11y: use Testing Library's accessible-name resolution rather than attribute literals
it('default render is not exposed as an image to ATs', () => {
  const { queryByRole } = render(<Glyph docType="decisions" size={24} />)
  expect(queryByRole('img')).toBeNull()
})

it('with ariaLabel, render exposes role=img with the given name', () => {
  const { getByRole } = render(<Glyph docType="decisions" size={24} ariaLabel="Decision" />)
  expect(getByRole('img', { name: 'Decision' })).toBeTruthy()
})
```

**On AC #4's `getComputedStyle(svg).fill` requirement**: this plan substitutes (a) an attribute/style-literal assertion in Vitest (because JSDOM does not reliably substitute `var()` in inline styles inside SVG), AND (b) a dedicated Playwright `page.evaluate(getComputedStyle)` spec in Phase 4 that asserts the resolved hex against `LIGHT_COLOR_TOKENS`/`DARK_COLOR_TOKENS` for one Glyph per theme. The Playwright resolved-hex spec — not the screenshot spec — is what verifies AC #4 end-to-end. This decision is recorded inline as a comment in `Glyph.test.tsx`.

The "no React state change has occurred" sub-clause of AC #4 is enforced both structurally (Glyph holds no `useState`/`useEffect`/`useContext`) and by a guard test:

```tsx
import GlyphSource from './Glyph.tsx?raw'
it('Glyph source contains no React state/effect hooks', () => {
  expect(GlyphSource).not.toMatch(/\buse(State|Effect|Reducer|Context|LayoutEffect)\b/)
})
```

This is intentionally ugly but durable: a future refactor introducing a hook must consciously remove or update this test.

### Success Criteria:

#### Automated Verification:

- [ ] All 36 cases pass: `npx vitest run src/components/Glyph/Glyph.test.tsx`
- [ ] Test count includes 36 parametrised cases plus the a11y, runtime-guard, and source-grep tests from Phase 2
- [ ] `npm run typecheck` still passes
- [ ] Full unit-test suite green: `npm test`

#### Manual Verification:

- [ ] Reviewing the test output, confirm each `(docType, size)` case names both the doc type and the size in its title

---

## Phase 4: `/glyph-showcase` route + Playwright per-cell visual-regression + resolved-fill spec

### Overview

Add the developer showcase route (rendering both kebab keys and friendly labels), the Playwright per-cell visual-regression spec (one screenshot per cell × theme, so individual icon regressions cannot hide under a viewport-wide diff budget), and a separate Playwright spec that verifies `getComputedStyle().color` resolves to the canonical hex for a representative Glyph per theme.

### Changes Required:

#### 1. GlyphShowcase route component

**File**: `skills/visualisation/visualise/frontend/src/routes/glyph-showcase/GlyphShowcase.tsx` (new)
**Changes**: A page rendering a CSS-grid (not `<table>`) layout: rows = doc types, columns = sizes. Each row shows the kebab-case key, the friendly label (from `DOC_TYPES[*].label`), and the three Glyphs. Background uses `var(--ac-bg)`; cell padding uses `var(--sp-3)` etc. — no new tokens introduced. Each cell has a stable `data-testid` (e.g. `glyph-cell-decisions-16`) so the per-cell Playwright spec can locate it.

```tsx
import { Glyph, GLYPH_DOC_TYPE_KEYS } from '../../components/Glyph/Glyph'
import { DOC_TYPE_LABELS } from '../../api/types'
import styles from './GlyphShowcase.module.css'

const SIZES = [16, 24, 32] as const

export function GlyphShowcase(): React.ReactElement {
  // The `data-testid` format `glyph-cell-${docType}-${size}` is the contract
  // surface for `tests/visual-regression/glyph-showcase.spec.ts` — any change
  // requires updating the spec's per-cell loop accordingly.
  return (
    <main className={styles.root}>
      <h1>Glyph Showcase</h1>
      <p className={styles.note}>Toggle <code>document.documentElement.dataset.theme</code> between <code>light</code> and <code>dark</code> in dev tools to compare.</p>
      <div className={styles.grid}>
        <div className={styles.headerRow}>
          <span>doc type</span>
          {SIZES.map(s => <span key={s}>{s}px</span>)}
        </div>
        {GLYPH_DOC_TYPE_KEYS.map(docType => {
          const label = DOC_TYPE_LABELS[docType]
          return (
            <div key={docType} className={styles.row}>
              <span className={styles.label}>
                <code>{docType}</code> <span className={styles.friendly}>{label}</span>
              </span>
              {SIZES.map(size => (
                // Decorative: Glyph is aria-hidden; the adjacent text label above
                // provides the accessible name (per Consumer Contract invariant #2).
                <span key={size} className={styles.cell} data-testid={`glyph-cell-${docType}-${size}`}>
                  <Glyph docType={docType} size={size} />
                </span>
              ))}
            </div>
          )
        })}
      </div>
    </main>
  )
}
```

**File**: `skills/visualisation/visualise/frontend/src/routes/glyph-showcase/GlyphShowcase.module.css` (new)
**Changes**: Minimal CSS-grid layout — `display: grid; grid-template-columns: auto repeat(3, auto); gap: var(--sp-3)`. No animation. Pads cells with `var(--sp-3)`; aligns labels to baseline.

#### 2. Component test for the showcase page

**File**: `skills/visualisation/visualise/frontend/src/routes/glyph-showcase/GlyphShowcase.test.tsx` (new)
**Changes**: Render the page and assert it contains exactly 36 `<svg>` elements (12 doc types × 3 sizes) and that each cell carries the expected `data-testid`.

#### 3. Router wiring

**File**: `skills/visualisation/visualise/frontend/src/router.ts`
**Changes**: Import GlyphShowcase, declare a top-level non-crumbed route after `kanbanRoute`, append to the tree:

```ts
import { GlyphShowcase } from './routes/glyph-showcase/GlyphShowcase'

// Top-level literal path; order in addChildren is not significant for non-ambiguous literals.
const glyphShowcaseRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/glyph-showcase',
  component: GlyphShowcase,
})

export const routeTree = rootRoute.addChildren([
  // …existing children…
  kanbanRoute,
  glyphShowcaseRoute,
])
```

Plain `createRoute` (not `withCrumb`) — a developer route doesn't appear in the breadcrumb trail.

#### 4. Playwright per-cell visual-regression spec

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/glyph-showcase.spec.ts` (new)
**Changes**: Capture one screenshot **per cell × theme** (36 cells × 2 themes = 72 baselines per platform). Each clip is bounded by the cell's `data-testid` locator, so per-icon regressions cannot hide under a viewport-wide diff. Threshold remains `maxDiffPixelRatio: 0.05` per cell (a cell is ~576-1152 pixels; 5% of that is 29-58 pixels of drift — sufficient to absorb anti-aliasing without masking shape errors). Use `page.waitForFunction` to confirm the cascade has settled rather than relying on a single `requestAnimationFrame`.

**CI cost note**: This spec runs 72 test cases per platform vs the prior plan's 2 — roughly a 36× increase in test-case count, but NOT 36× wall-clock, because Playwright workers run in parallel by default (config-determined; typical default 4 workers). Configure the describe block with `test.describe.configure({ mode: 'parallel' })` to ensure intra-describe parallelism, and use a shared `test.beforeEach` for theme setup rather than per-test navigation when possible. Estimated wall-clock impact: 2-4 minutes per platform per CI run (vs prior ~30 s). Capture the actual baseline timing during Phase 4 and record it in the commit message; if it materially exceeds estimate, consider reducing parallel workers' viewport or sharing a single page across cells of the same theme:

```ts
import { test, expect } from '@playwright/test'
import { GLYPH_DOC_TYPE_KEYS } from '../../src/components/Glyph/Glyph'
import { DARK_COLOR_TOKENS, LIGHT_COLOR_TOKENS } from '../../src/styles/tokens'

const SIZES = [16, 24, 32] as const
const VIEWPORT = { width: 1024, height: 768 }

for (const theme of ['light', 'dark'] as const) {
  test.describe(`glyph-showcase (${theme})`, () => {
    test.describe.configure({ mode: 'parallel' })
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize(VIEWPORT)
      await page.goto('/glyph-showcase')
      if (theme === 'dark') {
        const expected = DARK_COLOR_TOKENS['ac-doc-decisions']
        await page.evaluate(() => { document.documentElement.dataset.theme = 'dark' })
        await page.waitForFunction(
          (hex) => {
            const el = document.querySelector('[data-testid="glyph-cell-decisions-24"] svg') as SVGElement | null
            if (!el) return false
            const c = getComputedStyle(el).color
            return c.toLowerCase().includes(hex.toLowerCase()) || c.startsWith('rgb')
          },
          expected,
        )
      }
    })

    for (const docType of GLYPH_DOC_TYPE_KEYS) {
      for (const size of SIZES) {
        test(`${docType} @ ${size}px`, async ({ page }) => {
          const cell = page.locator(`[data-testid="glyph-cell-${docType}-${size}"]`)
          await expect(cell).toHaveScreenshot(`${docType}-${size}-${theme}.png`, {
            maxDiffPixelRatio: 0.05,
            animations: 'disabled',
          })
        })
      }
    }
  })
}
```

#### 5. Playwright resolved-fill spec

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/glyph-resolved-fill.spec.ts` (new)
**Changes**: Verifies AC #4's resolved-hex contract using a real Chromium engine. Picks one representative Glyph per theme and asserts `getComputedStyle().color` resolves to the expected hex/rgb from `LIGHT_COLOR_TOKENS`/`DARK_COLOR_TOKENS`:

```ts
import { test, expect } from '@playwright/test'
import { LIGHT_COLOR_TOKENS, DARK_COLOR_TOKENS } from '../../src/styles/tokens'

function hexToRgb(hex: string): string {
  const v = hex.replace('#', '')
  const r = parseInt(v.slice(0, 2), 16)
  const g = parseInt(v.slice(2, 4), 16)
  const b = parseInt(v.slice(4, 6), 16)
  return `rgb(${r}, ${g}, ${b})`
}

test('Glyph resolves --ac-doc-decisions to the light hex', async ({ page }) => {
  await page.goto('/glyph-showcase')
  const color = await page.locator('[data-testid="glyph-cell-decisions-24"] svg').evaluate(el => getComputedStyle(el).color)
  expect(color).toBe(hexToRgb(LIGHT_COLOR_TOKENS['ac-doc-decisions']))
})

test('Glyph resolves --ac-doc-decisions to the dark hex after theme swap', async ({ page }) => {
  await page.goto('/glyph-showcase')
  await page.evaluate(() => { document.documentElement.dataset.theme = 'dark' })
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark')
  const color = await page.locator('[data-testid="glyph-cell-decisions-24"] svg').evaluate(el => getComputedStyle(el).color)
  expect(color).toBe(hexToRgb(DARK_COLOR_TOKENS['ac-doc-decisions']))
})
```

The threshold and per-cell-clip choice are recorded in the work item Resolved Decisions section; the plan references those decisions rather than asserting the value in isolation.

#### 6. Generate baselines

Run `npx playwright test --update-snapshots tests/visual-regression/glyph-showcase.spec.ts` on macOS to produce darwin baselines. For Linux baselines, run locally via the Playwright Docker image (matching the version pinned in `package.json`):

```sh
docker run --rm -v "$(pwd):/work" -w /work --ipc=host \
  mcr.microsoft.com/playwright:v<version>-jammy \
  npx playwright test --update-snapshots tests/visual-regression/glyph-showcase.spec.ts
```

(Replace `<version>` with the `@playwright/test` version from `package.json`.) Both `-darwin.png` and `-linux.png` baselines are committed in the same PR so review can compare them side-by-side. If Docker is genuinely unavailable, CI may produce the linux baselines on first run — but capture this fallback in the PR description, since the implicit-acceptance path locks in whatever Linux rendered.

### Success Criteria:

#### Automated Verification:

- [ ] Showcase component test passes: `npx vitest run src/routes/glyph-showcase`
- [ ] Route is reachable: `cd skills/visualisation/visualise/frontend && npm run dev`, then `curl -I http://localhost:5173/glyph-showcase` returns 200
- [ ] Playwright per-cell spec passes against fresh baselines: `npm run test:e2e -- glyph-showcase`
- [ ] Playwright resolved-fill spec passes for both themes: `npm run test:e2e -- glyph-resolved-fill`
- [ ] `npm run typecheck` still passes
- [ ] Full unit-test suite green: `npm test`

#### Manual Verification:

- [ ] Open `http://localhost:5173/glyph-showcase` in browser, confirm 12-row × 3-column grid renders with kebab keys + friendly labels
- [ ] Toggle `document.documentElement.dataset.theme` between `light` and `dark` via dev tools console; confirm icons re-fill smoothly without a page reload
- [ ] Visually compare each row of the showcase against the corresponding glyph in `library-view-updated-{light,dark}.png` — shapes and colours match
- [ ] Confirm both `-darwin.png` and `-linux.png` baselines are committed under `tests/visual-regression/__screenshots__/glyph-showcase.spec.ts-snapshots/` (72 per platform)

---

## Phase 5: Frontend README

### Overview

Create the frontend README from scratch. The README serves the audience of contributors landing in `skills/visualisation/visualise/frontend/`: it should be enough to run dev, test, and build without consulting other docs, and explicit about the project's developer-only routes and visual-regression baseline conventions.

### Changes Required:

**File**: `skills/visualisation/visualise/frontend/README.md` (new)
**Changes**: A README with:

```markdown
# Visualiser Frontend

The visualiser frontend is the React app served by the [`visualisation/visualise`](../) accelerator skill. It renders a meta-directory inspector (research, plans, decisions, work items, etc.) backed by a Rust server. This README documents how to develop, test, and build the frontend in isolation.

## Prerequisites

- Node.js (version per the engines field in `package.json`)
- npm (matches the lockfile)
- For Linux Playwright baselines on macOS: Docker (optional; CI can produce them otherwise)

## Project Layout

- `src/components/` — Reusable React components (`Brand`, `Glyph`, `PipelineDots`, etc.). Co-located `<Name>/<Name>.tsx + .module.css + .test.tsx`, named exports, no barrels.
- `src/routes/` — Top-level routes wired through `src/router.ts`. TanStack Router imperative `createRoute` style.
- `src/styles/` — Design-system tokens (`tokens.ts` ↔ `global.css`), parity tests, and migration invariants.
- `src/api/` — Domain types (`DocTypeKey`, `DOC_TYPES`, ...) and hooks for backend communication.
- `tests/visual-regression/` — Playwright specs and committed PNG baselines (darwin + linux per spec).

## Development

```sh
npm install
npm run dev
```

## Testing

```sh
npm test                # Vitest unit tests
npm run typecheck       # tsc --noEmit (enforces @ts-expect-error directives)
npm run test:e2e        # Playwright E2E + visual regression
```

## Building

```sh
npm run build
```

## Developer Routes

The following routes exist solely to preview internal components and are not part of the user-facing navigation:

- `/glyph-showcase` — renders all 12 doc-type Glyphs at all 3 supported sizes (16/24/32 px), viewable under both `data-theme="light"` and `data-theme="dark"` via dev-tools attribute toggling. See [`meta/work/0037-glyph-component.md`](../../../../meta/work/0037-glyph-component.md).

## Visual-Regression Baselines

Each visual-regression spec commits two PNG baselines per case: `<name>-darwin.png` and `<name>-linux.png`. Capture both before merging.

- **macOS**: `npx playwright test --update-snapshots <spec>` produces darwin baselines.
- **Linux (from macOS)**: Use the Playwright Docker image matching `package.json`:

  ```sh
  docker run --rm -v "$(pwd):/work" -w /work --ipc=host \
    mcr.microsoft.com/playwright:v<version>-jammy \
    npx playwright test --update-snapshots <spec>
  ```

  (Replace `<version>` with the pinned `@playwright/test` version.)

If a Playwright test fails locally with a baseline mismatch on the opposite platform, do not regenerate that platform's baseline locally — let CI regenerate it under a known environment.

## Troubleshooting

- **`tsc -b` fails after editing a token**: ensure the new entry appears in BOTH `tokens.ts` and all three theme blocks in `global.css`; run `npx vitest run src/styles/global.test.ts` to surface parity violations.
- **Theme swap not reflected in `getComputedStyle`**: the cascade may not have settled. Prefer `page.waitForFunction` checking a known computed value over a fixed `requestAnimationFrame` wait.
```

(Paths back to the work item use the relative path from the frontend directory.)

### Update the work item Resolved Decisions

**File**: `meta/work/0037-glyph-component.md`
**Changes**: Append a "Consumer Contract" entry to the Resolved Decisions section mirroring the five invariants from the Glyph TSDoc block. This is the durable anchor 8 downstream WIs (0036/0040/0041/0042/0043/0053/0054/0055) consult when planning their integration — they read the work item, not this plan. Use this content:

```markdown
### Consumer Contract

Downstream consumers of Glyph MUST adhere to these invariants. The same list lives as a TSDoc block on `Glyph` in `src/components/Glyph/Glyph.tsx` so it travels with the import.

1. Do not override `fill` on Glyph or any ancestor that targets it via CSS.
2. Provide an adjacent text label OR pass `ariaLabel` for any Glyph used as a standalone visual without nearby text.
3. Do not wrap Glyph in another `<svg>`.
4. Sizes are restricted to 16/24/32. Off-grid sizes require a PR widening the union with a documented specimen.
5. Narrow `DocTypeKey` to `GlyphDocTypeKey` via `isGlyphDocTypeKey()` or `GLYPH_DOC_TYPE_KEYS` — never via `as` casts.
```

### Success Criteria:

#### Automated Verification:

- [ ] README exists: `test -f skills/visualisation/visualise/frontend/README.md`
- [ ] README mentions `/glyph-showcase` under a Developer Routes heading: `grep -E "Developer [Rr]outes" skills/visualisation/visualise/frontend/README.md && grep "/glyph-showcase" skills/visualisation/visualise/frontend/README.md`
- [ ] Work item Resolved Decisions contains the Consumer Contract: `grep "Consumer Contract" meta/work/0037-glyph-component.md`

#### Manual Verification:

- [ ] README renders cleanly in a Markdown viewer (no broken section headings)
- [ ] The link to the work item resolves (`../../../../meta/work/0037-glyph-component.md` exists from the README's location)
- [ ] A new contributor running `npm install && npm run dev` from the README's instructions succeeds without consulting other docs
- [ ] The Visual-Regression Baselines section provides enough detail to regenerate both darwin and linux baselines

---

## Testing Strategy

### Unit Tests (Vitest):

- **Type rejection** (Phase 2): `@ts-expect-error` directives on invalid `docType` and invalid `size`. Run by the new `npm run typecheck` script — not by `npm test` alone.
- **DOM shape** (Phase 2): root element is `<svg>`, has `viewBox`, `width`/`height` match `size`, inline `style.color` is `var(--ac-doc-<key>)`, every child uses `currentColor` (no hex literals).
- **Accessibility branches** (Phase 2): default render is not exposed as an image (`queryByRole('img')` is null); with `ariaLabel`, `getByRole('img', { name })` resolves. Empty-string `ariaLabel` is a labelled image, not decorative.
- **Runtime guard** (Phase 2): unknown `docType` in dev calls `console.warn` and returns null; production returns null silently.
- **Exhaustiveness assertion** (Phase 2): `GLYPH_DOC_TYPE_KEYS.length === 12 === Object.keys(ICON_COMPONENTS).length`; a future virtual key added to `DocTypeKey` is filtered automatically by the `virtual`-discriminant filter, surfacing the gap via this assertion.
- **No state hooks source guard** (Phase 3): `Glyph.tsx?raw` does not match `/use(State|Effect|Reducer|Context|LayoutEffect)/`. Durable enforcement of AC #4's no-render invariant.
- **36-combination matrix** (Phase 3): parametrised over `GLYPH_DOC_TYPE_KEYS × SIZES`. Asserts dimensions and color var for every combination.
- **Showcase page** (Phase 4): renders 36 `<svg>` elements with stable `data-testid`s.
- **Token parity** (Phase 1): automatically extended by adding entries to `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS`.
- **WCAG 1.4.11 contrast** (Phase 1): 24 contrast tests (12 light, 12 dark) assert ≥ 3:1 against `--ac-bg`.

### Visual + Computed-Style Regression (Playwright):

- **`glyph-showcase` per-cell × per-theme** (Phase 4): 36 cells × 2 themes = 72 screenshots against committed baselines (darwin + linux). Catches per-icon path, colour, and rendering regressions without a viewport-wide budget that could absorb them.
- **`glyph-resolved-fill`** (Phase 4): `page.evaluate(getComputedStyle)` on one representative Glyph per theme; asserts the resolved color RGB matches the canonical hex. **This is the test that verifies AC #4 end-to-end**, not the screenshot spec.
- `maxDiffPixelRatio: 0.05` per cell — a cell is small enough that 5% of its pixels equals 29-58 pixels of drift, sufficient for anti-aliasing without masking errors.

### Manual Testing:

- **Visual fidelity** (Phase 2 manual review by non-implementer): per-icon visual diff against `library-view-updated-{light,dark}.png`.
- **Smallest-size legibility** (Phase 2): each icon recognisable at 16 px.
- **Theme swap** (Phase 4): toggle `data-theme` in dev tools, confirm icons re-fill without a page reload.
- **No regressions** (every phase): existing pages (Library, Lifecycle, Kanban) render unchanged.

## Performance Considerations

### Render cost
Glyph holds no React state and has no `useEffect`. Theme swap is entirely CSS-cascade-driven; no React render is triggered. AC #4's "no React state change has occurred" is structurally and test-guarded.

### Bundle size
Statically importing all 12 icon components in the dispatch record defeats tree-shaking: any module that imports `Glyph` pulls all 12 icon path geometries. **Estimated overhead** (to be measured during Phase 2 implementation and recorded in the commit message body): ~12 icons × ~300-600 bytes of path data minified + gzipped ≈ 4-8 KB per chunk that imports Glyph. Since 7 of 8 downstream consumers render multiple doc types per page, eager bundling is the right tradeoff; if a future consumer renders only one doc type in a hot bundle, the `Record<GlyphDocTypeKey, React.ComponentType>` value type already abstracts the icon as a component reference, so a `React.lazy`-per-icon variant is reachable without an API break.

**Action**: After Phase 2 lands, capture the actual bundle-size delta (`vite build` before/after or `rollup-plugin-visualizer` output). If measured overhead is materially larger than estimated (e.g. > 15 KB gzipped), open a follow-up to evaluate dynamic-import variants.

### DOM weight in high-cardinality consumers
Glyph inlines its full SVG path string into the DOM per instance. **Threshold guidance for downstream consumers**:

- **≤ 50 instances per page** (sidebar nav, page eyebrows, kanban cards in typical view): inline-per-instance is fine; per-Glyph DOM cost is negligible.
- **50-200 instances per page** (long activity feeds, large kanban views, search-result pages): inline-per-instance is acceptable but should be measured. Watch for cumulative layout-shift and parse time on slower devices.
- **> 200 instances per page**: prefer a sprite-sheet pattern — mount a single hidden `<svg><defs><symbol id="glyph-decisions">…</symbol>…</defs></svg>` at app root and have a thin Glyph variant render `<svg><use href="#glyph-${docType}" /></svg>`. Path geometry is interned once per document. This variant is **out of scope for 0037** but documented here so downstream WIs (notably 0040 kanban, 0054 search results, 0055 activity feed) have a known escape valve.

### Per-render allocations
Glyph constructs a fresh `a11y` object literal on every render. Negligible at unit scale; in a 1000-row list this is 1000 small-object allocations per render pass. Not optimised here — premature. If profiling in a downstream WI shows pressure, lift the decorative-branch literal to module scope (the `aria-hidden`-only branch is invariant).

### Visual-regression baseline footprint
72 cells × 2 platforms = 144 baseline PNGs for the glyph-showcase spec. Each cell is small (16-32 px square + padding); estimated 1-3 KB per baseline gzip-compressed = ~150-450 KB total repo footprint. Spot-check after Phase 4 captures them; if materially larger, run `oxipng`/`pngquant` losslessly before commit.

## Migration Notes

No data migration. No existing consumer of `--ac-doc-*` tokens (this plan introduces the sub-namespace). No existing route at `/glyph-showcase`. Adding the new token declarations to all three theme blocks is the only edit to a file shared with other features; the additions are pure extensions (no existing tokens renamed or removed) and parity tests catch any divergence between the three blocks.

## References

- Work item: `meta/work/0037-glyph-component.md` (Resolved Decisions section captures: `maxDiffPixelRatio: 0.05` per cell, `ariaLabel` prop name, per-cell clip strategy, `currentColor` fill pattern, viewBox measured grid)
- Research: `meta/research/2026-05-12-0037-glyph-component.md`
- Canonical screenshots:
  - `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-light.png`
  - `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/library-view-updated-dark.png`
- Token-layer foundation: `meta/plans/2026-05-06-0033-design-token-system.md`
- Sibling component plans: `meta/research/2026-05-07-0035-topbar-component.md`, `meta/research/2026-05-08-0034-theme-and-font-mode-toggles.md`
- Sibling ARIA prop convention: `skills/visualisation/visualise/frontend/src/components/TopbarIconButton/TopbarIconButton.tsx`
- Related: 0033, 0036, 0038, 0040, 0041, 0042, 0043, 0053, 0054, 0055
- ADR-0026 (CSS design-token application conventions): `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
- Visual contract: token parity in `skills/visualisation/visualise/frontend/src/styles/global.test.ts:59-95`; MIRROR-A↔MIRROR-B parity in lines 125-163
- Routing precedent: `skills/visualisation/visualise/frontend/src/router.ts:132-151`
- Component pattern precedent: `skills/visualisation/visualise/frontend/src/components/Brand/Brand.tsx:15-31` + `Brand.test.tsx:11-19`
- Visual-regression spec precedent: `skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts:9-39`
