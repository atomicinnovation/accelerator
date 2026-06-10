---
type: plan
id: "2026-06-09-0082-big-glyph-hero-illustrations"
title: "BigGlyph Hero Illustration Set Implementation Plan"
date: "2026-06-09T21:09:35+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0082"
parent: "work-item:0082"
derived_from: ["codebase-research:2026-06-09-0082-big-glyph-hero-illustrations"]
tags: [design, frontend, components, illustrations]
revision: "d6c16730a6ca97d048cba981d3bc8fd921c38bca"
repository: "build-system"
last_updated: "2026-06-10T19:53:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# BigGlyph Hero Illustration Set Implementation Plan

## Overview

Ship a `BigGlyph` React component: one bespoke per-doc-type SVG hero
illustration (80×80 viewBox, scalable) for each of the thirteen `DocTypeKey`
values, drawn through a `bigPalette(hue)` helper that derives exactly seven
tones (six hue-derived + a fixed `white`) from the doc type's single numeric
hue, with a `DEFAULT_BIG` fallback. The component replaces the generic
`PaperFold` hero in the per-type empty state (`EmptyState.tsx`), rendered at
96px and `aria-hidden`. Fidelity and per-type distinctness are confirmed by a
26-combination (13 types × light/dark) Playwright visual-regression suite with
design-approved baselines.

## Current State Analysis

The app has no large-scale hero illustration system. The per-type empty state
(`src/routes/library/EmptyState.tsx:32`) renders a generic `PaperFold` hero at
72px — hue-tinted but not type-specific. 0037 delivered the small `Glyph`
(16/24/32px icon) but no hero. The prototype
(`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/big-glyphs.jsx`)
is the authoritative source and is fully traceable: it contains `bigPalette`
(seven tones), `DEFAULT_BIG`, thirteen named per-type illustrations, and the
`pr-reviews` diff-tint constants.

Key facts confirmed against the live code:

- **Hue source already exists, numeric, per type.** `EmptyState` reads
  `TYPE_COPY[docType].hue` (a 0–360 number, table in
  `src/routes/library/empty-descriptions.ts:16-83`) and sets
  `--ac-empty-page-hue`. `Record<DocTypeKey, TypeCopy>` typing guarantees all
  thirteen hues are present at compile time. (Phase 1 lifts this numeric hue map
  into `src/styles/tokens.ts` as `DOC_TYPE_HUE` so the shared `BigGlyph` component
  can resolve it without depending on the library route module; `TYPE_COPY`
  continues to expose `hue` by sourcing it from there.)
- **`PaperFold` is the sanctioned runtime-`hsl()` precedent**
  (`src/routes/library/PaperFold.tsx:12-15`): it derives `hsl(${hue} ...)`
  tones inline from a numeric hue. Its four tones (`stroke`/`fill`/`fold`/
  `line`) are a strict subset of `bigPalette`'s seven, with identical formulas.
- **`PaperFold` is consumed only by `EmptyState.tsx`** (line 5 import, line 32
  usage). The sole other textual reference is a reason string in
  `src/styles/migration.test.ts:223`. So Phase 3's swap orphans the component.
- **Key set reconciles cleanly.** `src/api/types.ts:4-19` — `DOC_TYPE_KEYS` /
  `DocTypeKey` have exactly thirteen keys matching the work item's list. The
  prototype's `BIG_GLYPHS` maps on with **two renames**: `work`→`work-items`,
  `work-reviews`→`work-item-reviews`. The other eleven are identical.
- **`Glyph` (0037) is the structure to mirror**
  (`src/components/Glyph/`): co-located component, one-file-per-type icons
  (`icons/<DocType>Icon.tsx`), `Record<DocTypeKey, …>` exhaustiveness, a
  dev-only un-crumbed showcase route (`/glyph-showcase`, registered in
  `src/router.ts:143-147`), a showcase unit test, and a per-cell × theme
  visual-regression spec (`tests/visual-regression/glyph-showcase.spec.ts`).
- **One architectural divergence from `Glyph`.** The prototype's per-type
  functions take the palette as an argument (`(p) => <g>…`), so the BigGlyph
  dispatch record is `Record<DocTypeKey, (p: BigPalette) => ReactElement>`,
  **not** the zero-arg `ComponentType` that `Glyph` uses. BigGlyph is a
  *runtime-hsl* component (compute `hsl(${hue} …)` tones from a bare numeric
  hue), not a *currentColor/CSS-var* component — do **not** reach for
  `DOC_TYPE_COLOR_VAR`.

### Key Discoveries

- `bigPalette(hue)` in the prototype returns exactly seven tones
  (`.../big-glyphs.jsx:16-26`); a stale header comment says "five-tone" — ignore
  it, the code returns seven.
- The `pr-reviews` diff tints are the only non-palette constant colours besides
  `white` (`.../big-glyphs.jsx:300-306`): added bg `hsl(140 60% 85%)`, added
  marker `hsl(140 50% 40%)`, removed bg `hsl(0 65% 88%)`, removed marker
  `hsl(0 55% 45%)`. They use fixed hues (140/0) that ignore the type hue.
- The `notes` illustration has one further literal — the drop-shadow
  `fill="rgba(0,0,0,0.08)"` (`.../big-glyphs.jsx:355`). Per the research's
  resolved decision #1 this is **kept and documented** as a sanctioned
  non-palette structural constant.
- `EmptyState.module.css:9` is a `grid-template-columns: 96px 1fr` grid — the
  hero column is fixed at 96px, which is exactly why `size={96}` is specified.
- `DARK_COLOR_TOKENS['ac-bg-card']` = `#131524` (vs `#ffffff` light) — usable to
  poll the dark-theme cascade commit in the visual-regression spec.

## Desired End State

- A `BigGlyph` component lives at `src/components/BigGlyph/BigGlyph.tsx`,
  rendering a hero illustration for each of the thirteen `DocTypeKey` values and
  falling back to `DEFAULT_BIG` for any off-union key.
- `bigPalette(hue)` returns exactly seven tones (six hue-derived + fixed
  `white`); no per-doc-type tone is hard-coded. The only constant colours are
  `white`, the two `pr-reviews` diff tints, and the `notes` shadow literal.
- BigGlyph is the hero of the per-type empty state, rendered at 96px,
  `aria-hidden="true"`, with the empty state's accessible title/lede/footer
  unchanged.
- A `/big-glyph-showcase` dev route hosts a Playwright visual-regression spec
  covering all 26 hero × theme combinations, passing against design-approved
  baselines committed for darwin and linux.

**Verification of end state**: `npm run typecheck` clean;
`mise run test:unit:frontend` green (palette/component/showcase unit tests);
`mise run test:e2e:visualiser` green against the 26 signed-off baselines; the
empty state renders the type-specific hero in the live app on both themes.

## What We're NOT Doing

- **No DevDesignSystem / dev design-system page showcase** — that surface
  belongs to 0083. The `/big-glyph-showcase` route built here is a throwaway
  dev-only fixture purely to host the visual-regression spec; 0083's
  consolidated DevDesignSystem will supersede it later. Note: the spec's
  `data-testid` locator contract and the 26 committed baselines become a
  dependency of this throwaway route — when 0083 supersedes it, the spec and
  baselines must be **migrated** to the new surface, not just deleted, or the
  visual-regression coverage is lost. The 0083 `blocks` edge should carry this
  migration obligation.
- **No library-landing empty-card change** — the prototype's landing card uses
  the small 34px `TypeGlyph`, not BigGlyph (removed from scope per the work
  item's drafting notes).
- **No change to the small `Glyph` (0037)** or to `DOC_TYPE_COLOR_VAR` — BigGlyph
  is a separate runtime-hsl component.
- **No per-tone contrast tuning** — the illustration is decorative
  (`aria-hidden`), so WCAG per-tone contrast tuning is not required. Knockout-
  text legibility ("WRK", "PR-0133", "{{ title }}") is verified at baseline
  design sign-off, not pre-emptively tuned (research resolved decision #4).
- **No new colour tokens or ADRs** — runtime `hsl()` construction is
  unconstrained by ADR-0026/0035; ADR-0035 explicitly names BigGlyph as a
  sanctioned brand-layer consumer.

## Implementation Approach

Three phases. **Phase 1 is the prerequisite**; Phases 2 and 3 each depend only
on Phase 1 and are independent of each other, so after Phase 1 lands they may be
merged in either order. Each phase is independently integratable: Phase 1 adds a
component used nowhere yet (compiles, unit-tested); Phase 2 adds a dev route +
tests; Phase 3 is the call-site swap.

TDD applies to everything mechanically verifiable — `bigPalette` hue-equality,
dispatch exhaustiveness, fallback, decorative-by-default a11y, the diff-tint
constants, and the showcase cell contract are all written test-first. The
**traced SVG shapes themselves** are inherently visual; their "red-green" loop
is the visual-regression baseline (Phase 2): author the spec, capture darwin
baselines, then confirm fidelity/distinctness by signing the baselines off
against side-by-side prototype renders at the same 80×80 viewBox.

---

## Phase 1: `BigGlyph` component, palette, and traced illustrations

### Overview

Build the `BigGlyph` component, the `bigPalette(hue)` helper, the thirteen
traced per-type illustrations (one file each) plus `DEFAULT_BIG`, and the unit
tests that pin the palette/dispatch/a11y contract. Also lift the per-doc-type
numeric hue map into `src/styles/tokens.ts` as `DOC_TYPE_HUE` (a behaviour-
preserving refactor of `empty-descriptions.ts`) so the component resolves its hue
from shared design data rather than depending on the library route module.
Nothing consumes `BigGlyph` yet, so this phase is mergeable on its own.

### Changes Required

#### 1. Palette helper + shared types and constants

**File**: `src/styles/tokens.ts` (edit) — extract the per-doc-type numeric hue map
**Changes**: Add `DOC_TYPE_HUE: Record<DocTypeKey, number>` (the 0–360 hues
currently inlined in `empty-descriptions.ts`'s `TYPE_COPY` table) to the shared
design-tokens module, alongside the existing `ac-doc-*` colour tokens it already
houses. This gives the numeric hue a **neutral home** that both the library route
and the `BigGlyph` component can depend on, instead of `BigGlyph` (a
`src/components/` primitive) reaching *up* into a `src/routes/library/` module.
`src/styles/tokens.ts` is already a route-agnostic design-data module that
components and visual-regression specs import (and this plan already reads
`DARK_COLOR_TOKENS['ac-bg-card']` from it), so the dependency direction is clean.
Note the placement is deliberate: the per-doc-type *token-key* maps
(`DOC_TYPE_TOKEN_KEY`, `DOC_TYPE_COLOR_VAR`) live in `Glyph.constants.ts`, but the
numeric *hue* is the upstream colour-identity source that those tokens and the
gradient panel both derive from — it belongs with the design tokens, not in a
single component's constants. Add a one-line comment in `tokens.ts` saying so.
This is the first `DocTypeKey`-keyed map in `tokens.ts`, requiring a `DocTypeKey`
import from `api/types` (no cycle — `api/types` does not import `tokens`).

```ts
/** Per-doc-type identity hue (0–360), the single source of colour identity shared
 *  across the sidebar glyph, the empty-state gradient panel, and the BigGlyph
 *  hero. Mirrors the prototype's TYPE_META.hue. */
export const DOC_TYPE_HUE: Record<DocTypeKey, number> = {
  'work-items': 12,
  'work-item-reviews': 340,
  // …all 13…
}
```

**File**: `src/routes/library/empty-descriptions.ts` (edit)
**Changes**: Source each `TypeCopy.hue` from `DOC_TYPE_HUE[docType]` rather than
re-declaring the number inline, so the hue is single-sourced. The `EmptyState`
gradient panel keeps reading `copy.hue`; behaviour is unchanged. Because no
existing test pins the per-type hue values, the refactor's value-preservation is
otherwise unverified — so add a parity assertion (in `BigGlyph.test.tsx` or a
co-located `empty-descriptions.test.ts`) that `TYPE_COPY[k].hue ===
DOC_TYPE_HUE[k]` for every `DOC_TYPE_KEYS` entry, turning "values are identical"
into an enforced invariant that catches a transcription slip during the move.

**File**: `src/components/BigGlyph/bigPalette.ts` (new)
**Changes**: Port `bigPalette` from the prototype (seven tones), with a typed
return. Export the `BigPalette` type and a named render-function alias
`export type BigGlyphDraw = (p: BigPalette) => ReactElement` for the illustration
signatures and the dispatch record — one place to evolve the dispatch contract,
and a self-documenting name at each of the thirteen illustration call sites
instead of re-stating the bare signature.

```ts
export interface BigPalette {
  stroke: string
  fill: string
  fold: string
  line: string
  accent: string
  deep: string
  white: string
}

/** Derive the seven-tone BigGlyph palette from a single HSL hue (0–360).
 *  Six hue-derived tones + a fixed `white`. Value-preserving port of the
 *  prototype's `bigPalette` (big-glyphs.jsx:16-26) — the only normalisation is
 *  `white`, lowercased to `#ffffff` (the prototype writes `#FFFFFF`) to match the
 *  codebase's lowercase-hex convention. No per-doc-type tone is hard-coded —
 *  every illustration colours itself from this one hue. */
export function bigPalette(hue: number): BigPalette {
  return {
    stroke: `hsl(${hue} 50% 50%)`,
    fill: `hsl(${hue} 78% 96%)`,
    fold: `hsl(${hue} 50% 86%)`,
    line: `hsl(${hue} 30% 78%)`,
    accent: `hsl(${hue} 65% 56%)`,
    deep: `hsl(${hue} 55% 38%)`,
    white: '#ffffff',
  }
}
```

**File**: `src/components/BigGlyph/BigGlyph.constants.ts` (new)
**Changes**: The two `pr-reviews` diff-tint pairs as named, commented constants —
the only constant colours besides `white` and the `notes` shadow. Values copied
exactly from `.../big-glyphs.jsx:300-306`. Exported so a unit test can assert
they equal the prototype constants (AC2). Co-located at the component root
(mirroring the sibling `Glyph.constants.ts` convention) rather than under
`icons/`, which holds only per-type illustration components.

```ts
/** Diff-semantic tints for the `pr-reviews` illustration ONLY. These use fixed
 *  hues (140 green / 0 red) that deliberately ignore the doc-type hue, so they
 *  are NOT members of the seven-tone `bigPalette`. They are sanctioned
 *  non-palette structural constants and must equal the prototype's
 *  big-glyphs.jsx diff tints exactly. */
export const PR_REVIEW_DIFF_TINTS = {
  addedBg: 'hsl(140 60% 85%)',
  addedMarker: 'hsl(140 50% 40%)',
  removedBg: 'hsl(0 65% 88%)',
  removedMarker: 'hsl(0 55% 45%)',
} as const
```

#### 2. Thirteen traced illustrations + fallback (one file per type)

**Files** (new, under `src/components/BigGlyph/icons/`): one per doc type, each
exporting a named function `(p: BigPalette) => ReactElement` whose body is the
inner `<g>` traced from the corresponding prototype shape at the 80×80 viewBox.
Re-key the two renamed prototype entries to the live `DocTypeKey` names.

| File | Source prototype key | Live key |
|---|---|---|
| `WorkItemsBigGlyph.tsx` | `work` | `work-items` |
| `WorkItemReviewsBigGlyph.tsx` | `work-reviews` | `work-item-reviews` |
| `DesignInventoriesBigGlyph.tsx` | `design-inventories` | `design-inventories` |
| `DesignGapsBigGlyph.tsx` | `design-gaps` | `design-gaps` |
| `ResearchBigGlyph.tsx` | `research` | `research` |
| `PlansBigGlyph.tsx` | `plans` | `plans` |
| `PlanReviewsBigGlyph.tsx` | `plan-reviews` | `plan-reviews` |
| `ValidationsBigGlyph.tsx` | `validations` | `validations` |
| `PrDescriptionsBigGlyph.tsx` | `pr-descriptions` | `pr-descriptions` |
| `PrReviewsBigGlyph.tsx` | `pr-reviews` | `pr-reviews` |
| `DecisionsBigGlyph.tsx` | `decisions` | `decisions` |
| `NotesBigGlyph.tsx` | `notes` | `notes` |
| `TemplatesBigGlyph.tsx` | `templates` | `templates` |

Each file is a faithful JSX→TSX port: convert prototype attribute values to the
`p.*` palette references already used in the source (no per-type colour
literals). `PrReviewsBigGlyph.tsx` imports `PR_REVIEW_DIFF_TINTS` instead of the
inline `hsl(140 …)`/`hsl(0 …)` strings. `NotesBigGlyph.tsx` keeps the
`rgba(0,0,0,0.08)` shadow literal **with a comment** naming it as the sanctioned
non-palette exception (research resolved decision #1) so the AC2 code review does
not flag it. Knockout text keeps `fontFamily="ui-monospace, monospace"`.

Example (shape of each file — `ResearchBigGlyph.tsx`):

```tsx
import type { ReactElement } from 'react'
import type { BigPalette } from '../bigPalette'

/** RESEARCH — two stacked sheets with a magnifier. Traced from
 *  big-glyphs.jsx `research` at the 80×80 viewBox. */
export function ResearchBigGlyph(p: BigPalette): ReactElement {
  return (
    <g>
      {/* …inner SVG ported verbatim, all colours via p.* … */}
    </g>
  )
}
```

**File**: `src/components/BigGlyph/icons/DefaultBigGlyph.tsx` (new)
**Changes**: Port `DEFAULT_BIG` (`.../big-glyphs.jsx:408-415`) — a rotated paper
sheet, same `(p: BigPalette) => ReactElement` signature. Add
`data-testid="default-big-glyph"` to its root `<g>` so the off-union fallback
unit test can assert the `DEFAULT_BIG` shape (not just any `<svg>`) was selected.

#### 3. The component + dispatch

**File**: `src/components/BigGlyph/BigGlyph.tsx` (new)
**Changes**: Owns the outer `<svg viewBox="0 0 80 80">`, resolves hue internally
from the shared `DOC_TYPE_HUE` map (with optional numeric `hue` override per
research resolved decision #2), and dispatches via an exhaustive
`Record<DocTypeKey, …>`.

```tsx
import type { ReactElement } from 'react'
import { type DocTypeKey, DOC_TYPE_KEYS } from '../../api/types'
import { DOC_TYPE_HUE } from '../../styles/tokens'
import { bigPalette, type BigPalette } from './bigPalette'
import { DefaultBigGlyph } from './icons/DefaultBigGlyph'
// …one import per <DocType>BigGlyph…

/** Exhaustiveness enforced by `Record<DocTypeKey, …>` across all 13 keys.
 *  Each entry takes the palette and returns inner SVG (the prototype contract);
 *  this is the key divergence from `Glyph`'s zero-arg `ComponentType` map. */
const BIG_GLYPHS: Record<DocTypeKey, (p: BigPalette) => ReactElement> = {
  'decisions': DecisionsBigGlyph,
  'work-items': WorkItemsBigGlyph,
  // …all 13…
}

/** Neutral blue hue for the off-union fallback path (mirrors the prototype's
 *  `|| 215` default). Named so it is not mistaken for `templates`' canonical
 *  hue, which happens to be 215 too — they are independent facts. */
const DEFAULT_BIG_HUE = 215

export interface BigGlyphProps {
  docType: DocTypeKey
  /** Rendered px (square). Defaults to 96 — the EmptyState hero column width.
   *  Freely scalable, unlike `Glyph`'s fixed `16 | 24 | 32` size union — this is
   *  an illustrative hero, not a fixed-grid icon. */
  size?: number
  /** Numeric HSL hue (0–360) override. Defaults to DOC_TYPE_HUE[docType].
   *  Exposed for the 0083 showcase / off-canon rendering. Unlike `Glyph.colorVar`
   *  (a CSS-var string), BigGlyph overrides via a raw numeric hue because it
   *  constructs `hsl()` tones at render time (the runtime-hsl model). */
  hue?: number
}

/** Decorative per-doc-type hero illustration. Runtime-hsl coloured from a single
 *  numeric hue; theme-agnostic (the surrounding panel handles light/dark).
 *  Decorative-only by design — always `aria-hidden`, with no `ariaLabel` escape
 *  hatch (unlike the small `Glyph`): the empty-state copy carries the meaning, so
 *  a labelled/announced hero is intentionally out of scope. */
export function BigGlyph({ docType, size = 96, hue }: BigGlyphProps): ReactElement {
  // `??` (not `||`) so an explicit `hue={0}` (valid red) is honoured rather than
  // discarded. `DOC_TYPE_HUE[docType]` is undefined for off-union keys (cast /
  // JS callers), so fall back to DEFAULT_BIG_HUE — this keeps the off-union path
  // null-safe so the `?? DefaultBigGlyph` fallback can render.
  const resolvedHue = hue ?? DOC_TYPE_HUE[docType] ?? DEFAULT_BIG_HUE
  const draw = BIG_GLYPHS[docType] ?? DefaultBigGlyph
  if (import.meta.env.DEV && !BIG_GLYPHS[docType]) {
    console.warn(
      `[BigGlyph] Unknown docType "${docType}"; falling back to DEFAULT_BIG. ` +
        `Expected one of: ${DOC_TYPE_KEYS.join(', ')}.`,
    )
  }
  return (
    <svg
      viewBox="0 0 80 80"
      width={size}
      height={size}
      aria-hidden="true"
      style={{ display: 'block' }}
    >
      {draw(bigPalette(resolvedHue))}
    </svg>
  )
}
```

The typed `Record` makes `?? DefaultBigGlyph` reachable only for off-union keys
passed via casting/JS callers — which is exactly the `DEFAULT_BIG` fallback AC1
requires. The hue read above is null-safe (`hue ?? DOC_TYPE_HUE[docType] ?? DEFAULT_BIG_HUE`)
precisely because `DOC_TYPE_HUE[docType]` is `undefined` for an off-union
`docType`; the `DEFAULT_BIG_HUE` (215) blue default mirrors the prototype's
`window.TYPE_META[type] || { hue: hue || 215 }`. Use `??` rather than the
prototype's `||` so an explicit `hue={0}` (a legitimate red) is not silently
discarded. Because the hero and the empty-state gradient panel now both resolve
from the single `DOC_TYPE_HUE` source, the two cannot drift.

**Two deliberate divergences from the small `Glyph`, documented here so review
doesn't flag them:**

- **Off-union behaviour.** `Glyph` returns `null` and emits a DEV `console.warn`
  for an unknown `docType`; `BigGlyph` instead renders `DEFAULT_BIG` (the
  AC1-mandated graceful fallback — `Glyph` has no fallback art). The DEV
  `console.warn` above keeps the off-union *signal* `Glyph` provides while still
  rendering the fallback.
- **Dispatch entries are render functions, not components.** Each `BIG_GLYPHS`
  value is a `(p: BigPalette) => ReactElement` invoked as `draw(palette)`, not a
  zero-arg `ComponentType` rendered as `<Icon />` — the palette must be threaded
  in. A `BIG_GLYPHS` record comment should state this so a contributor adding a
  type doesn't reach for `<XBigGlyph />`.

#### 4. Unit tests (written first)

**File**: `src/components/BigGlyph/BigGlyph.test.tsx` (new)
**Changes**:

- `bigPalette` returns exactly seven keys; `white === '#ffffff'`; for sample
  hues including the boundaries **`0`, `12`, `280`, and `215`** every one of the
  six hue-derived tones contains the input hue (parse the `hsl(<hue> …)` prefix
  and assert the numeric hue component equals the input — `0` confirms the parse
  yields exactly `0`, not empty/`NaN`) — directly satisfies AC2's hue-equality
  check.
- The `hue` override honours the boundary value: `<BigGlyph docType="plans"
  hue={0} />` renders a child carrying `hsl(0 ` (red), proving `??` resolution
  (not `||`, which would discard `0` and fall through to the default).
- **Source-walk literal guard (AC2, automated).** Mirroring the 0037 `Glyph`
  suite's deep-walk: render each of the thirteen `BigGlyph`s (plus the off-union
  fallback) and assert every descendant `fill`/`stroke` is a member of an
  allowed-value set, built per type as
  `new Set(Object.values(bigPalette(DOC_TYPE_HUE[docType])))` plus the
  **type-scoped** sanctioned constants — `#ffffff` for all types, the four
  `PR_REVIEW_DIFF_TINTS` **only for `pr-reviews`**, and the `rgba(0,0,0,0.08)`
  shadow **only for `notes`**. Match by **exact set membership after
  normalising case/whitespace** (both the rendered attribute and the expected
  set lowercased), not by a loose `hsl(<hue>` prefix — so a wrong-lightness tone
  or a misplaced sanctioned constant (e.g. a diff tint on a non-`pr-reviews`
  type) is rejected, while a verbatim-ported `#FFFFFF` is not falsely flagged.
  This turns the "no stray per-type literal" check from a manual review step into
  an automated invariant.
- `Object.keys(BIG_GLYPHS).length === 13` and every `DOC_TYPE_KEYS` entry is a
  key (exhaustiveness; mirrors the 0037 `=== 12`→here `=== 13` assertion).
- `BigGlyph` renders a single `<svg>` with `viewBox="0 0 80 80"`,
  `width`/`height` equal to `size`, and `aria-hidden="true"` with **no** `role`
  (decorative-by-default — AC4).
- Default `size` is 96 when omitted.
- Off-union `docType` (cast) renders without throwing and renders the
  **`DEFAULT_BIG` shape specifically** — not merely "an `<svg>`". `DefaultBigGlyph`
  carries a stable marker (`data-testid="default-big-glyph"` on its root `<g>`);
  the test asserts that marker is present and that the applied hue is
  `DEFAULT_BIG_HUE` (215). It also asserts (via `vi.spyOn(console, 'warn')`) that
  the DEV warning fires once naming the bad key, and that an in-union render does
  **not** warn — mirroring the 0037 `Glyph` warn test. (Asserting only "an `<svg>`
  exists" would pass whether the fallback fired or not, since every in-union type
  also emits an `<svg>`.)
- An explicit `hue` prop overrides `DOC_TYPE_HUE` (render with `hue={280}`, assert
  a child carries a tone string containing `hsl(280`).
- `PR_REVIEW_DIFF_TINTS` equals the four exact prototype constants (AC2 — diff
  tints equal the prototype's).

### Success Criteria

#### Automated Verification

- [x] Type checking passes: `npm run typecheck`
- [x] Unit tests pass: `mise run test:unit:frontend`
- [x] `bigPalette` hue-equality test passes for ≥2 distinct hues
- [x] Dispatch exhaustiveness test (`=== 13`) passes
- [x] Dispatch-collision guard passes (`new Set(Object.values(BIG_GLYPHS)).size
  === 13` — thirteen referentially distinct illustration functions)
- [x] `PR_REVIEW_DIFF_TINTS` equality test passes
- [x] Source-walk literal guard passes (no per-type colour literal outside the
  sanctioned set)
- [x] Off-union fallback test passes: `DEFAULT_BIG` marker present, `215` guard
  hue applied, no throw
- [x] `hue` override honours the boundary `hue={0}` (renders `hsl(0 `)
- [x] a11y test confirms `aria-hidden="true"` and no `role`

#### Manual Verification

- [ ] Code review confirms no per-doc-type colour literals exist outside
  `white`, the two `pr-reviews` diff tints, and the documented `notes` shadow
- [ ] Each ported illustration uses only `p.*` tones (plus the sanctioned
  constants) and matches its prototype source structurally

---

## Phase 2: Showcase route + visual-regression suite

### Overview

Add a throwaway dev-only `/big-glyph-showcase` route (one cell per doc type at
96px, on a themed surface) and the Playwright spec covering all 26 hero × theme
combinations, then capture and sign off the design-approved baselines. This is
the phase that operationalises AC1's tracing-fidelity/distinctness and AC5.
Depends only on Phase 1.

### Changes Required

#### 1. Showcase route

**File**: `src/routes/big-glyph-showcase/BigGlyphShowcase.tsx` (new)
**Changes**: Mirror `GlyphShowcase` but with **no size axis** — one cell per
`DocTypeKey` with a stable `data-testid="big-glyph-cell-<docType>"`, each
rendering `<BigGlyph docType={docType} size={96} />`. Cells sit on a
`var(--ac-bg-card)` background so the light/dark captures genuinely differ and
validate the hero's legibility on each themed surface.

**File**: `src/routes/big-glyph-showcase/BigGlyphShowcase.module.css` (new)
**Changes**: A simple responsive grid; each `.cell` uses
`background: var(--ac-bg-card)` and fixed padding so the captured backdrop is
theme-driven and stable.

**File**: `src/router.ts`
**Changes**: Register an un-crumbed route mirroring `glyphShowcaseRoute`
(`src/router.ts:143-147`) — import `BigGlyphShowcase`, add
`bigGlyphShowcaseRoute` at `/big-glyph-showcase`, and append it to the
`rootRoute.addChildren([...])` array (alongside the other showcase routes at
lines 191-194).

#### 2. Showcase unit test (written first)

**File**: `src/routes/big-glyph-showcase/BigGlyphShowcase.test.tsx` (new)
**Changes**: Mirror `GlyphShowcase.test.tsx`:

- Renders exactly 13 `<svg>` elements.
- A cell with stable `data-testid="big-glyph-cell-<docType>"` exists for every
  `DOC_TYPE_KEYS` entry, each containing an `<svg>`.

#### 3. Visual-regression spec

**File**: `tests/visual-regression/big-glyph-showcase.spec.ts` (new)
**Changes**: Mirror `glyph-showcase.spec.ts`:

- Outer loop over `['light','dark']`, each a `test.describe` with
  `mode: 'parallel'` (the run is serial anyway — `playwright.config.ts` forces
  `workers: 1`).
- `beforeEach`: `setViewportSize`, `goto('/big-glyph-showcase')`; for dark, set
  `document.documentElement.dataset.theme = 'dark'` then `page.waitForFunction`
  **polling a resolved value** to confirm the cascade committed — poll a cell's
  resolved `getComputedStyle(...).backgroundColor` until it equals the dark
  `--ac-bg-card`. **Critical: `getComputedStyle().backgroundColor` returns an
  `rgb(...)` string (here `rgb(19, 21, 36)`), never the hex token `#131524`** —
  so the predicate must compare against the resolved rgb, not the raw hex, or it
  never becomes true and `waitForFunction` hangs to timeout. Convert the token
  with the existing `parseRgb` helper
  (`tests/visual-regression/lib/expected-colours.ts`):
  `parseRgb(DARK_COLOR_TOKENS['ac-bg-card'])` → compare to
  `parseRgb(getComputedStyle(cell).backgroundColor)`. **Do NOT inherit the
  `glyph-showcase.spec.ts:23-35` predicate** (`colour !== '' && hex.length > 0`)
  — it is tautological (confirms only that *a* colour resolved, never that the
  *dark* cascade committed) and copying it reintroduces exactly the flaky dark
  captures this poll exists to prevent. The predicate must return the
  equality result.
- Inner loop over `DOC_TYPE_KEYS`: per-cell `toHaveScreenshot('<docType>-<theme>.png',
  { maxDiffPixelRatio: 0.05, animations: 'disabled' })` scoped to the
  `[data-testid="big-glyph-cell-<docType>"]` locator (per-cell clipped, so a
  per-illustration regression can't hide under a viewport-wide diff budget).
- Total: 13 types × 2 themes = **26 baselines per platform**.

#### 4. Automated dispatch-collision guard

The 26 per-cell baselines guard each cell against its *own* future regression,
but they do **not** assert the thirteen illustrations differ *from each other* —
a dispatch copy-paste error (two `DOC_TYPE_KEYS` pointing at the same
illustration function) would pass every per-cell baseline. **Do not** try to
catch this by hashing rendered PNG bytes and asserting pairwise non-identity:
because all thirteen hues are distinct, two keys mapped to the *same* function
still render at *different* hues → different bytes → no hash collision, so a
byte-hash guard cannot detect the very copy-paste it targets. Catch it
deterministically at the dispatch level instead, in `BigGlyph.test.tsx`
(Phase 1):

- Assert the thirteen `BIG_GLYPHS` values are **referentially distinct
  functions** — `new Set(Object.values(BIG_GLYPHS)).size === 13`. This fails
  immediately and platform-independently if two keys share an illustration
  function, which is the concrete copy-paste error, and runs in the unit suite
  every CI pass.

Genuine *visual* distinctness (two distinct functions that nonetheless trace
near-identical shapes) is not mechanically assertable and remains the job of the
design sign-off below — the success criteria scope the automated guard to
exact-duplicate dispatch collisions accordingly.

#### 5. Baseline capture + sign-off

- Capture darwin baselines via Playwright `--update-snapshots` (run the
  `visual-regression` project). On-disk names resolve to
  `<docType>-<theme>-visual-regression-darwin.png` under
  `tests/visual-regression/__screenshots__/big-glyph-showcase.spec.ts-snapshots/`.
- Generate linux baselines via the **"Update visual regression baselines"** CI
  workflow (linux baselines drift behind darwin — committing both platforms is
  required; a `GITHUB_TOKEN` push won't re-trigger Main CI).
- **Design sign-off**: review the 26 darwin captures side-by-side against the
  prototype shapes rendered at the same 80×80 viewBox, confirming tracing
  fidelity, per-type distinctness, and 96px knockout-text legibility on both
  themes. AC5 is met only when the signed-off baselines exist and the spec
  passes against them.

### Success Criteria

#### Automated Verification

- [x] Type checking passes: `npm run typecheck`
- [x] Showcase unit test passes: `mise run test:unit:frontend`
- [x] Visual-regression spec passes against committed baselines:
  `mise run test:e2e:visualiser` (26/26 darwin, expected 26 / unexpected 0)
- [~] 26 darwin + 26 linux baseline PNGs committed under
  `__screenshots__/big-glyph-showcase.spec.ts-snapshots/` — **darwin committed;
  linux baselines must be generated via the "Update visual regression
  baselines" CI workflow after the branch is pushed (cannot be captured
  locally on darwin)**

#### Manual Verification

- [ ] `/big-glyph-showcase` renders all thirteen heroes at 96px in the running
  app, distinct and on-brand, on both light and dark themes
- [ ] Each baseline matches the prototype shape side-by-side at the 80×80 viewBox
  (tracing fidelity signed off)
- [ ] Monospace knockout text ("WRK", "PR-0133", "{{ title }}") is legible at
  96px in both themes

---

## Phase 3: EmptyState integration swap

### Overview

Replace the `PaperFold` hero in the per-type empty state with `BigGlyph` at
96px, preserving the empty state's accessible text and decorative-hero
behaviour, and remove the now-orphaned `PaperFold`. Delivers AC3 + AC4. Depends
only on Phase 1; independent of Phase 2.

### Changes Required

#### 1. The swap

**File**: `src/routes/library/EmptyState.tsx`
**Changes**:

- Line 5: replace `import { PaperFold } from './PaperFold'` with
  `import { BigGlyph } from '../../components/BigGlyph/BigGlyph'`.
- Line 32: replace `<PaperFold size={72} hue={hue} />` with
  `<BigGlyph docType={docType} size={96} />`.
- `hue` stays in scope and continues to drive `--ac-empty-page-hue` (line 26);
  it is simply no longer passed to the hero (BigGlyph resolves it internally).
  No CSS change — the `.card` hero column is already 96px
  (`EmptyState.module.css:9`).

#### 2. Remove the orphaned PaperFold

**File**: `src/routes/library/PaperFold.tsx`
**Changes**: Delete — `EmptyState` was its only consumer (confirmed by grep).

**File**: `src/styles/migration.test.ts`
**Changes**: Update the reason string at line 223 (`'…PaperFold hero column
track from design…'`) to reference the BigGlyph hero, since the 96px literal now
hosts BigGlyph. The literal/count assertion itself is unchanged.

#### 3. Test the swapped hero

**File**: `src/routes/library/EmptyState.test.tsx`
**Changes**: Add a test asserting the hero is present, decorative, **and correctly
wired** — asserting only that an `<svg viewBox="0 0 80 80" aria-hidden="true">`
exists is too weak: it passes for any `docType` and any `size`, so a regression
that hardcoded the wrong `docType` or dropped `size={96}` back to 72 would still
pass. Instead:

- Render `EmptyState` for **two different doc types** (e.g. `work-items` and
  `decisions`) and assert each hero carries a hue-bearing tone matching *its own*
  type — deriving the expected hue from `DOC_TYPE_HUE[docType]` rather than
  re-typing the literal (so a future hue retune can't leave a stale assertion),
  and asserting up front that the two chosen types have distinct hues so the
  "tones differ between the two" check is genuinely discriminating. This pins the
  `docType → hero` wiring, not just "a hero exists".
- Assert the hero `<svg>` is `aria-hidden="true"` with **no** `role`, and that its
  `width`/`height` is **96** (pins AC3's 96px contract).
- Assert the accessible title/lede/footer copy is unchanged (existing assertions
  already cover the copy). No existing test references `PaperFold`, so none break.

### Success Criteria

#### Automated Verification

- [ ] Type checking passes: `npm run typecheck`
- [ ] Unit tests pass (incl. updated `EmptyState.test.tsx` and
  `migration.test.ts`): `mise run test:unit:frontend`
- [ ] No remaining references to `PaperFold` in `src`/`tests`:
  `grep -rn "PaperFold" src tests` returns nothing
- [ ] EmptyState-adjacent visual-regression specs still pass:
  `mise run test:e2e:visualiser` (`typography-resolved-sizes.spec.ts`,
  `radius-resolved-radii.spec.ts`)

#### Manual Verification

- [ ] The per-type empty state shows the type-specific BigGlyph hero (not the
  generic PaperFold) at 96px in the running app, on both themes
- [ ] Screen-reader / accessibility tree shows the hero as hidden and the
  title/lede/footer as the only announced content

---

## Testing Strategy

### Unit Tests

- `bigPalette`: seven tones, `white` fixed (`#ffffff`), hue-equality at ≥2
  distinct hues.
- `PR_REVIEW_DIFF_TINTS`: exact equality with the prototype's four constants.
- Source-walk literal guard: every descendant `fill`/`stroke` across all thirteen
  illustrations (plus the fallback) is a `bigPalette` tone or a documented
  sanctioned constant — no stray per-type literal (automates AC2).
- `BigGlyph`: 80×80 viewBox, default size 96, `aria-hidden` + no `role`,
  exhaustive 13-key dispatch, off-union fallback selects `DEFAULT_BIG`
  specifically (marker assertion, no throw, 215 guard hue), `hue` override
  including the boundary `hue={0}`.
- `BigGlyphShowcase`: 13 svgs, one stably-`data-testid`'d cell per type.
- `EmptyState`: correct per-type hero wired (distinct hue tones across two doc
  types), decorative (`aria-hidden`, no `role`), 96px; title/lede/footer copy
  unchanged.

### Integration / Visual-Regression Tests

- `big-glyph-showcase.spec.ts`: 26 per-cell clipped screenshots (13 × light/dark)
  against design-approved baselines; dark-theme commit confirmed by polling the
  resolved `--ac-bg-card` colour as an `rgb(...)` value (not the hex token).
- Dispatch-collision guard (unit, in `BigGlyph.test.tsx`): the thirteen
  `BIG_GLYPHS` values are referentially distinct functions, so a copy-paste that
  points two keys at the same illustration fails CI deterministically. (Genuine
  visual near-duplicate distinctness remains the design sign-off's job.)

### Manual Testing Steps

1. `mise run dev`, open `/big-glyph-showcase`; toggle
   `document.documentElement.dataset.theme` between `light`/`dark`; confirm all
   thirteen heroes are distinct, on-brand, and legible at 96px.
2. Visit a doc type with no documents (e.g. `/library/validations` on an empty
   corpus); confirm the type-specific hero renders at 96px in the empty-state
   card on both themes.
3. Inspect the accessibility tree; confirm the hero is `aria-hidden` and only the
   title/lede/footer are announced.
4. Sign off the 26 darwin baselines against side-by-side prototype renders.

## Performance Considerations

Negligible — thirteen small static SVGs, no runtime image loading. `hsl()` tone
strings are computed once per render from a single hue. No new network or bundle
weight beyond the component code (consistent with `PaperFold`/`Glyph`).

## Migration Notes

No data migration. `PaperFold` is removed in Phase 3; its sole consumer
(`EmptyState`) is updated in the same phase, so the deletion is atomic with the
swap and recoverable via VCS if needed. Baselines for both platforms (darwin +
linux) must be committed in the Phase 2 PR.

## References

- Original work item: `meta/work/0082-big-glyph-hero-illustrations.md`
- Codebase research: `meta/research/codebase/2026-06-09-0082-big-glyph-hero-illustrations.md`
- Work-item review (APPROVE): `meta/reviews/work/0082-big-glyph-hero-illustrations-review-1.md`
- Prototype source: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full/src/big-glyphs.jsx`
- Mirror structure (0037): `meta/plans/2026-05-12-0037-glyph-component.md`
- Component to mirror: `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.tsx`
- Runtime-hsl precedent: `skills/visualisation/visualise/frontend/src/routes/library/PaperFold.tsx:12-15`
- Integration point: `skills/visualisation/visualise/frontend/src/routes/library/EmptyState.tsx:32`
- Shared design-tokens module (new `DOC_TYPE_HUE` home; existing `DARK_COLOR_TOKENS`): `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
- Hue table being lifted into `DOC_TYPE_HUE`: `skills/visualisation/visualise/frontend/src/routes/library/empty-descriptions.ts:16-83`
- VR spec template: `skills/visualisation/visualise/frontend/tests/visual-regression/glyph-showcase.spec.ts`
- Route registration pattern: `skills/visualisation/visualise/frontend/src/router.ts:143-147`
