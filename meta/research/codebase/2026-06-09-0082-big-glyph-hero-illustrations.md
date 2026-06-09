---
type: codebase-research
id: "2026-06-09-0082-big-glyph-hero-illustrations"
title: "Research: Implementing the BigGlyph hero illustration set (0082)"
date: "2026-06-09T19:29:44+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0082"
parent: "work-item:0082"
relates_to: ["codebase-research:2026-05-12-0037-glyph-component", "codebase-research:2026-05-24-0074-per-doc-type-hues-on-detail-page"]
topic: "Implementing the BigGlyph per-doc-type hero illustration component"
tags: [research, codebase, bigglyph, glyph, empty-state, palette, visual-regression, illustrations]
revision: "666aa11b997815da7fff3730ad89161dd3c37e4f"
repository: "build-system"
last_updated: "2026-06-09T19:29:44+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Resolved the four open questions into settled decisions"
schema_version: 1
---

# Research: Implementing the BigGlyph hero illustration set (0082)

**Date**: 2026-06-09T19:29:44+00:00
**Author**: Toby Clemson
**Git Commit**: 666aa11b997815da7fff3730ad89161dd3c37e4f
**Branch**: HEAD (jj working copy)
**Repository**: build-system

## Research Question

What does work item 0082 ("BigGlyph Hero Illustration Set") require, and what
does the live codebase already provide to support it? Specifically: where is
the prototype source, how do the analogous live components (`Glyph`,
`PaperFold`, `EmptyState`) work, how is the doc-type/hue/palette plumbing
wired, how is the integration point (`EmptyState.tsx`) structured, and what is
the proven pattern for the showcase route + 26-baseline visual-regression
suite the acceptance criteria demand?

## Summary

0082 asks for a `BigGlyph` React component: one bespoke per-doc-type SVG hero
illustration (80×80 viewBox, scalable) for each of the **thirteen**
`DocTypeKey` values, drawn through a `bigPalette(hue)` helper that derives
exactly **seven tones** (six hue-derived + a fixed `white`) from the doc
type's single numeric hue, with a `DEFAULT_BIG` fallback. It replaces the
generic `PaperFold` hero in `EmptyState.tsx`, rendered at 96px and
`aria-hidden`. AC requires a 26-combination (13 types × light/dark) Playwright
visual-regression suite with design-approved baselines.

The codebase is unusually well-prepared for this work — almost everything has
a direct, recent precedent:

1. **The prototype is the authoritative source and is fully traceable.**
   `big-glyphs.jsx` contains `bigPalette(hue)` (exactly 7 tones, verified),
   `DEFAULT_BIG`, 13 named per-type illustrations on an 80×80 viewBox, and the
   four `pr-reviews` diff-tint constants. Nothing needs drafting from scratch.

2. **The key set reconciles cleanly.** Live `DOC_TYPE_KEYS` has exactly 13
   keys matching the work item's list key-for-key. The prototype's 13 named
   `BIG_GLYPHS` entries map onto them with only **two renames**:
   `work`→`work-items` and `work-reviews`→`work-item-reviews`. The other 11
   are identical.

3. **The hue source already exists, numeric, per type.** `EmptyState` reads
   `TYPE_COPY[docType].hue` (a 0–360 number, table in `empty-descriptions.ts`)
   and sets `--ac-empty-page-hue`. The swap is a one-line element change.

4. **`PaperFold` is the sanctioned runtime-`hsl()` precedent.** It already
   derives `hsl(${hue} ...)` tones inline from a numeric hue. No ADR prohibits
   this; ADR-0035 explicitly names BigGlyph as a sanctioned brand-layer
   consumer.

5. **The small `Glyph` (0037) is the proven component + showcase + visual-
   regression structure to mirror** — co-located component, one-file-per-type
   icons, exhaustiveness assertion, a dev-only non-crumbed showcase route, and
   per-cell clipped screenshots × themes.

App root for all frontend paths below:
`skills/visualisation/visualise/frontend/`.

## Detailed Findings

### Area 1 — The prototype source (`big-glyphs.jsx`, the authoritative spec)

Path:
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/big-glyphs.jsx`
(433 lines).

**`bigPalette(hue)` — exactly seven tones (lines 16-26):**

```jsx
function bigPalette(hue) {
  return {
    stroke: `hsl(${hue} 50% 50%)`,
    fill:   `hsl(${hue} 78% 96%)`,
    fold:   `hsl(${hue} 50% 86%)`,
    line:   `hsl(${hue} 30% 78%)`,
    accent: `hsl(${hue} 65% 56%)`,
    deep:   `hsl(${hue} 55% 38%)`,
    white:  "#FFFFFF",
  };
}
```

Six hue-derived tones + fixed `white`. Uses modern space-separated HSL syntax.
(A stale header comment at line 12 says "five-tone palette" — ignore it; the
code returns seven.) Note `fill` (`hsl(${hue} 78% 96%)`) is **identical** to
PaperFold's `fill` and to the EmptyState light-gradient stop — this is what
makes the hero blend into the panel.

**`DEFAULT_BIG` fallback (lines 408-415):** a render function `(p) => <g>...`
of the same shape as every `BIG_GLYPHS` entry — a rotated paper sheet (1 rect +
3 lines). It carries no hue of its own; it renders in whatever palette is
passed. Unknown types resolve to hue `215` (blue) in the prototype.

**The 13 named illustrations (`BIG_GLYPHS`, lines 31-405):** one `(p) => <g>`
per key. All render into the wrapper's single `viewBox="0 0 80 80"` (line 422).
Keys: `work`, `work-reviews`, `design-inventories`, `design-gaps`, `research`,
`plans`, `plan-reviews`, `validations`, `pr-descriptions`, `pr-reviews`,
`decisions`, `notes`, `templates`. (The "14" figure seen in an early count is
13 named entries **plus** `DEFAULT_BIG` as a 14th render function.)

**`pr-reviews` diff tints — the only non-palette constant colours besides
`white` (lines 300-306):**

- Added background (green): `hsl(140 60% 85%)`
- Added marker `+` (green): `hsl(140 50% 40%)`
- Removed background (red): `hsl(0 65% 88%)`
- Removed marker `−` (red): `hsl(0 55% 45%)`

These use fixed hues (140/0) that ignore the type hue — deliberately diff-
semantic. They are NOT members of the seven-tone palette. **One further literal
exists**: the `notes` drop shadow `fill="rgba(0,0,0,0.08)"` (line 355) — worth
flagging against the work item's claim that `white` + the two diff tints are
the *only* constants. The implementer should decide whether to keep the shadow
literal (it's a translucent black, arguably not a "colour" in the palette
sense) or note it as a sanctioned exception.

**Component contract (lines 417-430):** `BigGlyph({ type, size = 88, hue })`,
`<svg viewBox="0 0 80 80" width={size} height={size} aria-hidden="true">`. Hue
resolves from `window.TYPE_META[type].hue` (a prototype global **absent in the
app** — the live equivalent is `TYPE_COPY[docType].hue`), with an explicit
`hue` prop override.

**Tone-to-element conventions (so the family reads as one set):** `fill` =
primary paper/card body; `stroke` = body outline + bold title lines; `line` =
soft secondary content lines/grids (often dashed); `fold` = folded/secondary
surfaces; `accent` = the single highlight element; `deep` = darkest emphasis
(checkmarks, header bars, seal strokes); `white` = knockout text/interiors on
accent/deep fills. Knockout text uses `fontFamily="ui-monospace, monospace"`.

### Area 2 — Doc-type key reconciliation (the load-bearing correctness check)

`src/api/types.ts:4-19` — `DOC_TYPE_KEYS` and the `DocTypeKey` union both
contain **exactly 13 keys**, matching the work item's list key-for-key, order-
for-order, spelling-for-spelling:

```
decisions, work-items, plans, research, plan-reviews, pr-reviews,
work-item-reviews, validations, notes, pr-descriptions, design-gaps,
design-inventories, templates
```

Prototype `BIG_GLYPHS` key → live `DocTypeKey`:

| Prototype key | Live DocTypeKey | Status |
|---|---|---|
| `work` | `work-items` | **Rename** |
| `work-reviews` | `work-item-reviews` | **Rename** |
| `design-inventories` | `design-inventories` | match |
| `design-gaps` | `design-gaps` | match |
| `research` | `research` | match |
| `plans` | `plans` | match |
| `plan-reviews` | `plan-reviews` | match |
| `validations` | `validations` | match |
| `pr-descriptions` | `pr-descriptions` | match |
| `pr-reviews` | `pr-reviews` | match |
| `decisions` | `decisions` | match |
| `notes` | `notes` | match |
| `templates` | `templates` | match |

Implication: when porting shapes, the two renamed keys must be re-keyed to the
live names. `templates` is included (13-key `DocTypeKey`), unlike the small
Glyph's historical 12-key framing that the work item's drafting notes flag as a
prior error.

### Area 3 — Hue source and palette plumbing

`src/routes/library/empty-descriptions.ts` — `TYPE_COPY: Record<DocTypeKey,
TypeCopy>` where `TypeCopy = { purpose; path; hue: number }`. Comment (lines
7-8): hue "Matches the prototype's `TYPE_META.hue`." Complete per-type hues:

| docType | hue | | docType | hue |
|---|---|---|---|---|
| work-items | 12 | | pr-descriptions | 200 |
| work-item-reviews | 340 | | pr-reviews | 280 |
| design-inventories | 185 | | decisions | 355 |
| design-gaps | 95 | | notes | 50 |
| research | 28 | | templates | 215 |
| plans | 220 | | validations | 160 |
| plan-reviews | 260 | | | |

`Record<DocTypeKey, TypeCopy>` typing guarantees all 13 are present at compile
time. This is the numeric hue `bigPalette` needs — sourced exactly as
`EmptyState` already does (no resolved colour token).

`src/routes/library/PaperFold.tsx` — the runtime-`hsl()` precedent to mirror:
props `{ size = 72, hue }`, `viewBox="0 0 80 80"`, `aria-hidden="true"` (line
21), and tone derivation at lines 12-15:

```ts
const stroke = `hsl(${hue} 50% 50%)`
const fill   = `hsl(${hue} 78% 96%)`
const fold   = `hsl(${hue} 50% 86%)`
const line   = `hsl(${hue} 30% 78%)`
```

Note these four are a strict subset of `bigPalette`'s seven, with identical
formulas — so `bigPalette` is effectively PaperFold's palette extended with
`accent`, `deep`, and `white`. PaperFold also has one `#ffffff` literal (the
"+" badge), same pattern as `bigPalette.white`.

### Area 4 — The integration point (`EmptyState.tsx`)

`src/routes/library/EmptyState.tsx`:

- Props (lines 8-17): `docType: DocTypeKey` (required, passed in by caller —
  not derived), `dirPath?: string` (path override only).
- Resolution (lines 19-24): `copy = TYPE_COPY[docType]`, `hue = copy.hue`.
- CSS var (lines 25-30): `--ac-empty-page-hue` set inline as a string on the
  `.card` root; card is `role="status"`.
- **The exact swap (lines 31-33):**

  ```tsx
  <div className={styles.hero}>
    <PaperFold size={72} hue={hue} />   {/* line 32 — replace */}
  </div>
  ```

  Replace line 32 with `<BigGlyph docType={docType} size={96} />` (both
  `docType` and `hue` are in scope). Swap the import at line 5.
- Accessible text (lines 34-44): `.eyebrow` (path), `.title` h2
  (`data-testid="empty-state-title"`, "No {plural} yet."), `.lede` (purpose),
  `.foot` (indexer copy). PaperFold is `aria-hidden`, so all accessible text is
  this body copy — BigGlyph must stay `aria-hidden="true"` to preserve
  behaviour. **Unchanged by 0082.**

`EmptyState.module.css`: `.card` is a `grid-template-columns: 96px 1fr` grid
(line 9) — the hero column is **fixed at 96px**, which is exactly why
`size={96}` is specified (PaperFold's 72px left ~24px slack). `--ac-empty-page-
hue` defaults to `215` (line 7) and drives the radial-gradient panel
(light/dark stops, lines 18/30/41) and eyebrow text colour. `.hero` (lines
48-53) centres + top-aligns the glyph. Responsive collapse at 820px still fits
96px.

### Area 5 — Proven component + showcase + visual-regression structure

**Small Glyph component** (`src/components/Glyph/`, the 0037 structure to
mirror):

- `Glyph.tsx` — component + `ICON_COMPONENTS: Record<DocTypeKey,
  ComponentType>` dispatch (exhaustiveness enforced by the `Record` type). Owns
  the outer `<svg viewBox="0 0 24 24">`; icon files emit only inner `<g>`.
  Props: `docType`, `size: 16|24|32`, `ariaLabel?`, `framed?`, `colorVar?`.
  Decorative-by-default: omitting `ariaLabel` yields `aria-hidden: true` (lines
  85-88).
- `icons/<DocType>Icon.tsx` — **one file per doc type** (13 files), each a
  named component returning inner SVG primitives on a 24×24 grid. This isolates
  each shape for review against the prototype. BigGlyph should mirror this:
  `src/components/BigGlyph/BigGlyph.tsx` + `icons/<DocType>BigGlyph.tsx` (or
  equivalent), 80×80 grid.
- `Glyph.constants.ts` — `DOC_TYPE_COLOR_VAR: Record<DocTypeKey, string>` maps
  each type to `var(--ac-doc-<key>)`. **BigGlyph does NOT use this** — Glyph
  resolves colour via CSS var + `fill="currentColor"`, whereas BigGlyph
  computes `hsl(...)` tones at render from the numeric hue (the PaperFold
  approach). This is the key architectural divergence: BigGlyph is a *runtime-
  hsl* component, not a *currentColor/CSS-var* component.
- The 0037 plan asserted `Object.keys(ICON_COMPONENTS).length === 12`; BigGlyph
  should assert `=== 13`.
- No `.module.css` was created for Glyph ("empty-for-convention files are a
  smell"). BigGlyph likely needs none for the component itself.

**Framed vs bare:** `Glyph` (framed) and `IconFrame` apply a tinted square bg
with ~14% padding. But the work item specifies BigGlyph replaces the *bare*
PaperFold hero, and PaperFold is unframed. So BigGlyph is **bare** (plain
`<svg>`), matching PaperFold — not framed. (Note: this differs from the memory
note about per-doc-type glyphs being framed on every surface, which applies to
the small icon `Glyph`, not the large illustrative hero.)

**Showcase route** (`src/routes/glyph-showcase/GlyphShowcase.tsx`): CSS-grid,
one cell per type with stable `data-testid` (`glyph-cell-<docType>-<size>`).
Registered in `src/router.ts` as a top-level **non-crumbed** route via plain
`createRoute(...)` (not `withCrumb`). Recommendation from the analysis: create
a **new** `/big-glyph-showcase` route rather than extend `GlyphShowcase` (the
existing showcase has a fixed 3-size layout and a locator contract shared by
two specs; a hero has no size axis). The DevDesignSystem showcase surface
itself is **out of scope** (belongs to 0083).

**Visual-regression** (`tests/visual-regression/glyph-showcase.spec.ts`):

- Outer loop over `['light','dark']`, each a `test.describe(...)` with
  `mode: 'parallel'`.
- `beforeEach`: `setViewportSize`, `goto('/<route>')`, then for dark switch via
  `document.documentElement.dataset.theme = 'dark'` + `page.waitForFunction`
  polling a resolved `getComputedStyle(...).color` to confirm the cascade
  committed (stronger than the shared `setTheme` helper, which only confirms
  the attribute is set — prefer the polling version to avoid flaky dark
  captures).
- Inner loop: per-cell `toHaveScreenshot` scoped to the `data-testid` locator,
  `{ maxDiffPixelRatio: 0.05, animations: 'disabled' }`. **Per-cell clipped
  screenshots** so a per-illustration regression can't hide under a viewport-
  wide diff budget.
- Spec passes only `<docType>-<theme>.png`; Playwright's default
  `snapshotPathTemplate` appends `-visual-regression-<platform>`, so on-disk
  baselines are `<docType>-<theme>-visual-regression-{darwin,linux}.png` under
  `__screenshots__/big-glyph-showcase.spec.ts-snapshots/`.
- For BigGlyph: 13 types × 2 themes = **26 baselines per platform** (×2
  platforms committed in the PR). Darwin via `--update-snapshots`; linux via
  the "Update visual regression baselines" CI workflow (linux baselines drift
  behind darwin — see project memory). `playwright.config.ts:13` forces
  `workers: 1`, so the run is serial against the shared dev server regardless
  of `mode: 'parallel'`.
- Reusable helper: `tests/visual-regression/lib/expected-colours.ts` provides
  `setTheme`, `resolveToken`, `parseRgb`, and `EXPECTED_COLOR: Record<DocTypeKey,
  Record<Theme, string>>` — usable if an optional resolved-colour spec is
  wanted, though BigGlyph's hsl-derived tones won't map to `--ac-doc-<key>`
  tokens the way Glyph's do.

## Code References

- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/big-glyphs.jsx:16-26` — `bigPalette(hue)`, seven tones
- `.../big-glyphs.jsx:300-306` — the four `pr-reviews` diff-tint constants
- `.../big-glyphs.jsx:355` — `notes` `rgba(0,0,0,0.08)` shadow literal (the one undocumented constant)
- `.../big-glyphs.jsx:408-415` — `DEFAULT_BIG` fallback
- `.../big-glyphs.jsx:417-430` — `BigGlyph` component contract (80×80 viewBox, aria-hidden)
- `src/api/types.ts:4-19` — `DocTypeKey` / `DOC_TYPE_KEYS` (the 13-key authority)
- `src/routes/library/empty-descriptions.ts:16-83` — `TYPE_COPY` per-type hue table
- `src/routes/library/PaperFold.tsx:12-15` — runtime `hsl(${hue} ...)` tone derivation precedent
- `src/routes/library/EmptyState.tsx:24,30,32` — hue resolution, `--ac-empty-page-hue`, the PaperFold swap point
- `src/routes/library/EmptyState.module.css:9` — fixed 96px hero column (why size=96)
- `src/components/Glyph/Glyph.tsx:23-37,75,84-88` — dispatch record, decorative-by-default semantics
- `src/routes/glyph-showcase/GlyphShowcase.tsx` — showcase grid + `data-testid` locator pattern
- `src/router.ts:143-147` — non-crumbed showcase route registration pattern
- `tests/visual-regression/glyph-showcase.spec.ts` — theme × cell screenshot spec template
- `tests/visual-regression/lib/expected-colours.ts` — `setTheme`, `EXPECTED_COLOR`, colour helpers

## Architecture Insights

- **Two distinct colour models coexist deliberately.** The small `Glyph` uses
  the *theme-token* model (`color: var(--ac-doc-<key>)` + `fill="currentColor"`,
  so themes swap the resolved colour). `PaperFold`/`BigGlyph` use the
  *runtime-hsl* model (compute `hsl(${hue} ...)` from a bare numeric hue, theme-
  agnostic — the surrounding CSS panel handles light/dark, not the SVG). 0082
  is firmly in the second camp; do not reach for `DOC_TYPE_COLOR_VAR`.
- **The hue is the single source of colour identity** shared across the
  gradient panel, eyebrow text, and the hero — sourced once as
  `TYPE_COPY[docType].hue`. Keeping BigGlyph on that same numeric hue is what
  preserves cross-surface colour coherence (and matches the small Glyph's hue
  map, per 0074).
- **One-file-per-shape isolation** (from 0037) exists specifically so each
  traced shape can be diffed against the prototype in review — directly serves
  0082's "tracing fidelity confirmed by baseline sign-off" criterion.
- **Per-cell clipped visual-regression** is the established guard against per-
  illustration regressions hiding in aggregate diffs — 0082's 26-combination
  AC is the same pattern at a smaller cell count.
- **No ADR blocks runtime `hsl()`.** ADR-0026/0035 govern CSS-side token
  declarations; runtime HSL string construction in component code is
  unconstrained, and ADR-0035 explicitly names BigGlyph as a sanctioned brand-
  layer consumer.

## Historical Context

- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — the normative design-gap spec; the "Implement BigGlyph..." recommendation
  (lines 408-411), seven-tone palette, and empty-state usage. Note it mentions
  "the landing card hero" as a BigGlyph surface, but 0082 **removed the landing
  card from scope** (the prototype's landing card uses the small 34px
  `TypeGlyph`, not BigGlyph).
- `meta/reviews/work/0082-big-glyph-hero-illustrations-review-1.md` — APPROVE
  (Pass 2). Load-bearing constraints folded into the work item: thirteen keys
  (incl. `templates`), exactly-seven-tone palette, `pr-reviews` diff-tint
  exception, `bigPalette` hue-equality unit test, baseline-driven distinctness/
  fidelity (not subjective), AC5 design-approved-baseline gate, single-element
  `EmptyState` swap at 96px + `aria-hidden`, 0083 as downstream `blocks` edge.
- `meta/plans/2026-05-12-0037-glyph-component.md` — the proven component +
  showcase + visual-regression structure this work should mirror.
- `meta/research/codebase/2026-05-24-0074-per-doc-type-hues-on-detail-page.md`
  and its plan — the per-doc-type hue precedent and token surfacing.
- ADR-0026 (CSS token application conventions) and ADR-0035 (brand-layer
  indirection) — confirm runtime hsl is out of their scope and BigGlyph is a
  sanctioned consumer.
- **No existing plan for 0082** — planning is greenfield.

## Related Research

- `meta/research/codebase/2026-05-12-0037-glyph-component.md` — small Glyph
- `meta/research/codebase/2026-05-23-0073-atomic-brand-layer-palette.md` — brand palette tokens
- `meta/research/codebase/2026-05-24-0074-per-doc-type-hues-on-detail-page.md` — per-doc-type hues
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md` — the empty-state card owner

## Resolved Decisions

These four were open during research and resolved by Toby on 2026-06-09; the
plan should inherit them as settled:

1. **The `notes` `rgba(0,0,0,0.08)` shadow literal — KEEP AND DOCUMENT.** It is
   ported as-is and recorded as a sanctioned non-palette structural constant
   (alongside the two `pr-reviews` diff tints). The AC2 code-review check for
   "no per-type colour literals" must be read as: no per-type *palette* literals
   outside `white`, the two `pr-reviews` diff tints, **and** the `notes` shadow.
   A code comment at the literal should name it as the sanctioned exception so
   review doesn't flag it.
2. **Component-internal hue resolution — FOLLOW RECOMMENDATION.** `BigGlyph`
   resolves its hue internally from `TYPE_COPY[docType].hue`, so the
   `EmptyState` call site is a clean `<BigGlyph docType={docType} size={96} />`.
   Also expose an optional `hue` override prop (numeric 0–360) for the future
   0083 showcase / off-canon rendering, matching the prototype's `type` + `hue`
   contract.
3. **Throwaway `/big-glyph-showcase` route now — YES.** Build a minimal dev-
   only, non-crumbed showcase route mirroring `glyph-showcase` purely to host
   the 26-combination visual-regression spec. It is not blocked on 0083; 0083's
   consolidated DevDesignSystem will supersede it later.
4. **96px knockout-text legibility — CHECK AT SIGN-OFF.** Acknowledged; the
   monospace knockout text ("WRK", "PR-0133", "{{ title }}") is verified for
   legibility during baseline design-approval sign-off (AC5), not pre-emptively
   tuned.
