---
date: "2026-05-06T18:00:00+01:00"
type: plan
skill: create-plan
work-item: "meta/work/0033-design-token-system.md"
status: draft
---

# 0033 Design Token System Implementation Plan

## Overview

Introduce the prototype's layered token system — `--ac-*` colour
(light + dark, including per-theme shadow overrides), Sora/Inter/Fira
Code typography (self-hosted woff2 under `public/fonts/`), eleven-step
spacing, four-step radius, and five elevation tokens — as a single
foundational pass across `skills/visualisation/visualise/frontend/src/styles/`,
then migrate every literal in component CSS modules and global
stylesheets onto named tokens. The dark colour *values* ship in 0033;
the toggle UI and `data-theme` persistence ship separately in 0034.

The plan is organised around test-driven development: each migration
step writes a failing token-coverage / literal-absence test before
editing the consuming CSS, so AC3 / AC4 / AC5 are enforced in CI at
the per-file granularity the work item demands. The full migration
ships as a **single PR** containing all five phases; intermediate
commits inside the PR remain green by virtue of the Phase 2 harness
landing with `EXCEPTIONS` pre-populated for every current literal,
which phases 3–4 progressively *remove* as files migrate. The trunk
never sees a red state.

Visual parity (Phase 5) is enforced automatically via Playwright's
`toHaveScreenshot()` on the seven AC6 routes in both themes, with
the captured baselines stored under
`skills/visualisation/visualise/frontend/tests/visual-regression/`.

## Current State Analysis

The frontend at `skills/visualisation/visualise/frontend/` is a Vite
6 + Vitest 3 + React 19 SPA with CSS Modules wired by Vite's default
`*.module.css` convention (no Tailwind / PostCSS / vanilla-extract).
The current token surface is minimal:

- `src/styles/global.css:1-10` — eight `--color-*` custom properties
  in a single `:root` block.
- `src/styles/tokens.ts:1-12` — the same eight values mirrored as
  `COLOR_TOKENS` (frozen `as const` literal).
- `src/styles/contrast.test.ts:6-18` — the parity invariant: a
  `readCssVar` regex extractor loops `Object.entries(COLOR_TOKENS)`
  and asserts each TS entry matches the corresponding `:root`
  declaration. (The work item's Technical Notes say this loop lives
  in `global.test.ts` — that is wrong; `global.test.ts:4-18` only
  asserts focus-ring rules.)
- `src/styles/contrast.test.ts:20-35` — three WCAG AA contrast
  assertions for the existing palette.
- `src/styles/wiki-links.global.css:11,18` — the only hex literals
  in a global stylesheet (`#9ca3af`, `#6b7280`).
- `index.html` — a minimal `<head>` (`<meta charset>`, `<meta
  viewport>`, `<title>`); no font `<link>` tags.
- 16 CSS module files under `src/` (the work item's "17" is off by
  one). Component modules consume essentially no tokens today (6
  `var(--*)` references across two files, all using the defensive
  `var(--token, #fallback)` two-arg form).
- Hex / px / rem literals are scattered across 18 CSS files; verified
  baselines (run from `skills/visualisation/visualise/frontend/`):
  hex regex matches in 16 modules + `wiki-links.global.css` (~150
  total), px/rem matches in 16 modules + `wiki-links.global.css`
  (~210 total). Worst offenders: `LifecycleClusterView.module.css`
  (25 hex / 43 px-rem), `LifecycleIndex.module.css` (24 / 37),
  `KanbanBoard.module.css` (15 / 23).

The two single-line `box-shadow` rules in `LifecycleClusterView`
(lines 50, 55) are coloured geometric rings, not elevation shadows;
the only true elevation rule is `LifecycleIndex.module.css:79`. Three
`#fef2f2`/`#fecaca`/`#fee2e2` error-tint literals recur across modules
with no clean `--ac-*` mapping. Two blue shades (`#2563eb`, `#1d4ed8`)
need to collapse onto the prototype's indigo `--ac-accent: #595FC8`
(visible hue shift; bounded by AC6 ΔE < 5).

## Desired End State

A specification of the desired end state and how to verify it:

1. **`src/styles/tokens.ts`** exports seven frozen objects:
   `LIGHT_COLOR_TOKENS`, `DARK_COLOR_TOKENS`, `TYPOGRAPHY_TOKENS`,
   `SPACING_TOKENS`, `RADIUS_TOKENS`, `LIGHT_SHADOW_TOKENS`,
   `DARK_SHADOW_TOKENS`. The legacy `COLOR_TOKENS` export is removed.

2. **`src/styles/global.css`** declares all light tokens (colour,
   typography, spacing, radius, light shadow) under `:root` and
   dark colour + dark shadow overrides under `[data-theme="dark"]`,
   with a `@media (prefers-color-scheme: dark)` block applying the
   same dark overrides for users who have not explicitly opted into
   light. `@font-face` blocks reference the woff2 files under
   `public/fonts/`; no third-party font origins are loaded.

3. **`src/styles/global.test.ts`** asserts CSS↔TS parity for every
   in-scope token category (light colour, dark colour, typography,
   spacing, radius, light shadow, dark shadow), via a
   `readCssVar(name, scope)` helper that scopes the regex to the
   right block. `src/styles/contrast.test.ts` asserts WCAG AA
   contrasts on the `--ac-*` palette in both themes — including
   stroke and tinted-background contrasts via a new
   `composeOverSurface(rgba, surface)` helper exported from
   `contrast.ts`.

4. **`src/styles/migration.test.ts`** (new) imports every module
   CSS file as raw, asserts: (a) zero hex literals outside the
   typed `EXCEPTIONS` constant, (b) zero px/rem literals outside
   `EXCEPTIONS` (with `0px`/`0rem` resets auto-excluded), (c) zero
   `var(--token, #fallback)` two-arg consumption sites, (d) every
   `var(--NAME)` reference resolves to a key in the union of the
   declared token exports, (e) ≥ 300 `var(--*)` references in
   total across all modules combined.

5. **All 16 component CSS modules and `wiki-links.global.css`**
   reference `--ac-*` / `--sp-*` / `--radius-*` / `--ac-font-*` /
   `--size-*` / `--ac-shadow-*` tokens for every value the migration
   covers. Existing `var(--color-*, #fallback)` two-arg sites are
   converted to the no-fallback `var(--ac-*)` form. The eight
   `--color-*` declarations in `global.css` are removed.

6. **AC1 parity tests pass, and AC3 / AC4 / AC5 grep checks pass**
   when run from `skills/visualisation/visualise/frontend/`. The
   0033 PR description lists every irreducible-literal exception
   (mechanically generated from the `EXCEPTIONS` constant) and
   every typographic / spacing / colour drift consciously accepted
   under AC6.

7. **AC6 visual parity**: Playwright `toHaveScreenshot()` baselines
   for the seven inventory routes × two themes are captured before
   the migration begins and re-asserted after each phase, with
   `maxDiffPixelRatio: 0.05` per the AC6 5%-of-viewport bound.
   Documented drifts bounded by ΔE < 5 / ±2px / ±1px tolerances
   are encoded as per-route `maxDiffPixels` overrides where
   applicable; route-level pixel-diff regions exceeding 5% of
   viewport area are listed in the PR description with a one-line
   justification.

### Key Discoveries

- Existing parity-test idiom: `src/styles/contrast.test.ts:6-18` —
  a regex extractor that handles any token value as long as it
  terminates with `;`. Extends mechanically to typography stacks,
  multi-segment shadows, and unit-bearing scalars.
- Vitest reuses the Vite config (`vite.config.ts`); `?raw` imports
  work in tests with no extra wiring (Vite 6 + Vitest 3 per
  `package.json:14,44-45`).
- Existing `var(--token, #fallback)` consumer sites
  (`SidebarFooter.module.css:6,7,19`,
  `LibraryDocView.module.css:24,25,26`) all consume `--color-*`
  tokens that 0033 retires. They migrate to `--ac-*` in the same
  pass.
- AC3 / AC4 grep globs (`-g '!src/styles/global.css'`) are
  cwd-relative; running from the workspace root leaks `global.css`
  and `tokens.ts` into the match set. The plan mandates running
  from the frontend root. (`migration.test.ts` is cwd-independent
  via Vite's `import.meta.glob`, so the unit-test channel is
  preferred for CI.)
- The `--font-mode` swap (`[data-font="mono"]` repointing
  `--ac-font-display` and `--ac-font-body` to Fira Code) is
  scope of 0034, **not** 0033 — only the base typography tokens
  are authored here.

## What We're NOT Doing

- The user-facing theme toggle UI, `data-theme` persistence, and
  the `[data-font="mono"]` swap are deferred to 0034.
- The brand palette (`--atomic-*`) and legacy semantic aliases
  (`--fg-*` / `--bg-*` / `--accent` / `--stroke`) are explicitly
  out of scope per the work item AC1 exclusions. The prototype
  exposes them; 0033 ships only the active `--ac-*` semantic layer.
- No new tokens beyond the inventory's named set. Where no clean
  mapping exists (error-tint backgrounds, layout literals), the
  PR description lists the exception rather than growing the
  token set.
- Component and route redesigns (0034–0042) are deferred. 0033
  is a CSS-only token-substitution pass; no TSX changes (other
  than `index.html`) are made.
- Google Fonts CDN. The fonts are self-hosted under `public/fonts/`
  (per the plan-review decision); the work item's
  Dependencies/CSP/GDPR concerns are resolved by removing the
  third-party origin coupling rather than mitigating it.
- Test fixtures (`*.test.ts`, `*.test.tsx`) retain their inline
  hex literals as test inputs; both AC3 and AC4 globs exclude them.

## Implementation Approach

The migration ships as a **single PR** containing all five phases as
ordered commits. Every commit within the PR keeps the unit-test suite
green; the trunk only sees the merged final state. Phases:

1. **Token foundation** — write parity tests first (failing against
   absent tokens), then author the new tokens in TS and CSS,
   self-host the three font families under `public/fonts/`, update
   the WCAG AA contrast tests for the `--ac-*` palette in both
   themes, capture Playwright visual baselines for the seven AC6
   routes × two themes.
2. **Migration enforcement test harness** — write
   `migration.test.ts` that runs the AC3 / AC4 / AC5 grep
   equivalents at vitest time on raw-imported CSS. The harness
   lands **green** by pre-populating its typed `EXCEPTIONS`
   constant with every current hex/px/rem literal. Phases 3–4
   progressively *delete* exceptions as files migrate. The final
   state of `EXCEPTIONS` is the truly-irreducible set (1–3px
   hairlines, layout literals) and is the canonical source for
   the PR's "Irreducible-literal exceptions" section.
3. **Globals + retire legacy** — migrate `wiki-links.global.css`,
   migrate the six existing `var(--color-*, #fallback)` consumer
   sites in `SidebarFooter.module.css` and `LibraryDocView.module.css`
   onto `var(--ac-*)`, then delete the eight `--color-*`
   declarations, the `COLOR_TOKENS` export, and the legacy parity
   describe — all in a single commit so parity is never broken
   asymmetrically.
4. **Component module migration** — migrate the 16 CSS modules
   in two waves: worst offenders first (`LifecycleClusterView`,
   `LifecycleIndex`, `KanbanBoard`) to absorb the bulk of the
   judgement-call decisions early, locking the resulting mappings
   into a `meta/work/0033-token-mapping-conventions.md`
   artefact, then apply those conventions to the remaining 13
   modules. Each module migrates to green against
   `migration.test.ts` by removing its corresponding `EXCEPTIONS`
   entries and replacing the literals with `var(...)` references.
5. **Visual parity verification** — re-run Playwright's
   `toHaveScreenshot()` for the seven AC6 routes × two themes
   against the baselines captured in Phase 1; any diff exceeding
   the per-route `maxDiffPixelRatio: 0.05` threshold either
   indicates a regression to fix or is documented as a conscious
   AC6 drift in the PR description with a one-line justification.

The TDD invariant across phases 1–4 is: **never edit a CSS file
without first having a failing assertion that justifies the edit**.
Phase 5's verification is automated via Playwright; baselines
captured in Phase 1 are committed alongside the migrated code.

---

## Phase 1: Token Foundation

### Overview

Define every in-scope token in `tokens.ts` and `global.css`, with
parity tests asserting CSS↔TS sync for each category. Self-host
the three font families under `public/fonts/` and wire `@font-face`
declarations + critical-path preload links. Update WCAG AA contrast
tests to assert against the `--ac-*` palette in both themes,
including stroke-on-surface and warn-on-warn-tinted-bg via a new
`composeOverSurface` helper in `contrast.ts`. Add a small
`fonts.test.ts` covering AC2 wiring. Capture Playwright visual
baselines for the seven AC6 routes × two themes against the
unmigrated codebase, committed under
`tests/visual-regression/__screenshots__/`. The legacy `--color-*`
block (CSS, TS export, parity describe) is left in place — the
deprecated `COLOR_TOKENS` parity describe continues to run during
phases 1–2 so the legacy block remains under the parity invariant
until Phase 3 deletes both halves in a single commit.

### Changes Required

#### 1. Move and extend the parity test (TDD: write tests first)

**File**: `src/styles/global.test.ts`
**Changes**: replace the file with a parity loop over the new token
categories, **retain** the existing `COLOR_TOKENS` parity describe
(annotated `@deprecated`) until Phase 3 deletes both halves of the
legacy block in the same commit, plus the existing focus-ring
assertions retained verbatim. Introduce `readCssVar(name, scope)`
that captures the relevant block (`:root` or `[data-theme="dark"]`)
before running the existing regex.

`readCssVar` lower-cases both sides of the comparison so hex casing
in `tokens.ts` is decoupled from hex casing in `global.css`. The
regex assumes the captured block is *flat* (no nested rules); a
comment above the helper documents this invariant so future
contributors don't introduce `@media` / nested selectors inside
`:root` or `[data-theme="dark"]` and silently truncate the match.
Tests for new token categories fail initially (tokens not yet
authored); the legacy `COLOR_TOKENS` describe continues to pass.

```ts
import { describe, it, expect } from 'vitest'
import globalCss from './global.css?raw'
import {
  LIGHT_COLOR_TOKENS,
  DARK_COLOR_TOKENS,
  TYPOGRAPHY_TOKENS,
  SPACING_TOKENS,
  RADIUS_TOKENS,
  LIGHT_SHADOW_TOKENS,
  DARK_SHADOW_TOKENS,
  COLOR_TOKENS,
} from './tokens'

type Scope = 'root' | 'dark'

/**
 * Reads a CSS custom property's declared value from the relevant top-level
 * block in `global.css`. Comparison is case-insensitive on the value side
 * so hex casing differences (e.g. `#FBFCFE` vs `#fbfcfe`) do not break
 * parity.
 *
 * INVARIANT: the captured block must be flat — no nested selectors, no
 * `@media` wrappers, no CSS nesting inside `:root` or `[data-theme="dark"]`.
 * The non-greedy regex would silently truncate at the first inner `}`.
 */
function readCssVar(name: string, scope: Scope = 'root'): string | null {
  const blockRe =
    scope === 'root'
      ? /:root\s*\{([\s\S]*?)\}/
      : /\[data-theme="dark"\]\s*\{([\s\S]*?)\}/
  const block = blockRe.exec(globalCss)?.[1] ?? ''
  // Defensive: token names today are kebab-case alphanumeric, but escape
  // metacharacters so a future contributor passing an unusual name doesn't
  // silently get a corrupted match.
  const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const re = new RegExp(`--${escapedName}:\\s*([^;]+);`)
  return re.exec(block)?.[1].trim().toLowerCase() ?? null
}

function expectMatches(actual: string | null, expected: string): void {
  expect(actual).toBe(expected.toLowerCase())
}

describe('global focus rings', () => {
  it('declares :focus-visible with an outline', () => {
    expect(globalCss).toMatch(/:focus-visible\s*\{[^}]*outline:[^;]+;/)
  })
  it('declares an outline-offset for breathing room', () => {
    expect(globalCss).toMatch(/:focus-visible\s*\{[^}]*outline-offset:[^;]+;/)
  })
  it('overrides the focus-ring colour under forced-colors mode', () => {
    expect(globalCss).toMatch(
      /@media\s*\(forced-colors:\s*active\)\s*\{[^}]*:focus-visible[^}]*outline-color:\s*Highlight/i,
    )
  })
})

describe('tokens.ts ↔ global.css :root parity (light colour)', () => {
  for (const [name, value] of Object.entries(LIGHT_COLOR_TOKENS)) {
    it(`--${name} matches LIGHT_COLOR_TOKENS.${name}`, () => {
      expectMatches(readCssVar(name, 'root'), value)
    })
  }
})

describe('tokens.ts ↔ global.css [data-theme="dark"] parity (dark colour)', () => {
  for (const [name, value] of Object.entries(DARK_COLOR_TOKENS)) {
    it(`--${name} matches DARK_COLOR_TOKENS.${name}`, () => {
      expectMatches(readCssVar(name, 'dark'), value)
    })
  }
})

describe('tokens.ts ↔ global.css [data-theme="dark"] parity (dark shadow)', () => {
  for (const [name, value] of Object.entries(DARK_SHADOW_TOKENS)) {
    it(`--${name} matches DARK_SHADOW_TOKENS.${name}`, () => {
      expectMatches(readCssVar(name, 'dark'), value)
    })
  }
})

describe.each([
  ['typography', TYPOGRAPHY_TOKENS],
  ['spacing', SPACING_TOKENS],
  ['radius', RADIUS_TOKENS],
  ['light shadow', LIGHT_SHADOW_TOKENS],
])('tokens.ts ↔ global.css :root parity (%s)', (_label, tokens) => {
  for (const [name, value] of Object.entries(tokens)) {
    it(`--${name} matches`, () => {
      expectMatches(readCssVar(name, 'root'), value)
    })
  }
})

// Retained until Phase 3 deletes both `COLOR_TOKENS` (in tokens.ts) and the
// eight `--color-*` declarations (in global.css) in the same commit. While
// retained the parity invariant continues to cover the legacy block, so any
// drift (typo / accidental edit) between the two halves is caught at unit-
// test time during phases 1–2.
describe('tokens.ts ↔ global.css :root parity (legacy --color-* — retired in Phase 3)', () => {
  for (const [name, value] of Object.entries(COLOR_TOKENS)) {
    it(`--${name} matches COLOR_TOKENS.${name}`, () => {
      expectMatches(readCssVar(name, 'root'), value)
    })
  }
})

/**
 * The `[data-theme="dark"]` block and the `@media (prefers-color-scheme: dark)`
 * mirror block are hand-maintained duplicates of the same dark token values.
 * `readCssVar` cannot read the mirror block (its flat-block invariant
 * forbids `@media` wrappers), so we use a separate two-step extraction
 * here: first capture the `@media` body, then capture the inner
 * `:root:not([data-theme="light"])` block, and compare its declarations
 * against the explicit `[data-theme="dark"]` block.
 */
/** Extract a `{ ... }` block body starting at the first `{` after `index`,
 *  using brace-balanced scanning so nested rules don't truncate. Returns
 *  the body (without the enclosing braces) or `undefined` if no balanced
 *  block exists at that position. Resilient to formatter changes (no
 *  column-0 anchor required). */
function extractBlockBody(source: string, index: number): string | undefined {
  const open = source.indexOf('{', index)
  if (open === -1) return undefined
  let depth = 1
  for (let i = open + 1; i < source.length; i++) {
    if (source[i] === '{') depth++
    else if (source[i] === '}') {
      depth--
      if (depth === 0) return source.slice(open + 1, i)
    }
  }
  return undefined
}

describe('global.css [data-theme="dark"] ↔ @media (prefers-color-scheme: dark) parity', () => {
  it('the two dark blocks declare the same tokens with the same values', () => {
    const explicitMatch = /\[data-theme="dark"\]\s*\{/.exec(globalCss)
    const explicit = explicitMatch
      ? extractBlockBody(globalCss, explicitMatch.index)
      : undefined

    const mediaMatch = /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{/.exec(globalCss)
    const mediaBody = mediaMatch
      ? extractBlockBody(globalCss, mediaMatch.index)
      : undefined
    const innerMatch = mediaBody
      ? /:root:not\(\[data-theme="light"\]\)\s*\{/.exec(mediaBody)
      : null
    const mirror = innerMatch
      ? extractBlockBody(mediaBody!, innerMatch.index)
      : undefined

    expect(explicit, 'failed to extract [data-theme="dark"] body').toBeDefined()
    expect(mirror, 'failed to extract prefers-color-scheme inner block').toBeDefined()

    const normalise = (s: string): Map<string, string> => {
      const map = new Map<string, string>()
      for (const m of s.matchAll(/--([\w-]+):\s*([^;]+);/g)) {
        map.set(m[1], m[2].trim().toLowerCase())
      }
      return map
    }
    const a = normalise(explicit!)
    const b = normalise(mirror!)

    // Sets of declared property names match (catches "added in one, forgot the other")
    expect([...a.keys()].sort()).toEqual([...b.keys()].sort())
    // Each name has the same value in both blocks
    for (const [name, value] of a) {
      expect(b.get(name)).toBe(value)
    }
  })
})

/**
 * Sanity guard: catch the silent-truncation failure mode of the flat-block
 * regex in `readCssVar`. If a future contributor introduces a nested rule
 * inside `:root` or `[data-theme="dark"]`, the non-greedy match terminates
 * at the inner `}` and tokens declared after it return `null`. By asserting
 * one known-last token from each block, the truncation produces a hard
 * failure rather than passing on a partial extraction.
 */
describe('readCssVar truncation guard', () => {
  it(':root block extends past --shadow-crisp', () => {
    expect(readCssVar('shadow-crisp', 'root')).not.toBeNull()
  })
  it('[data-theme="dark"] block extends past --ac-shadow-lift', () => {
    expect(readCssVar('ac-shadow-lift', 'dark')).not.toBeNull()
  })
})
```

#### 2. Author `tokens.ts` against the inventory

**File**: `src/styles/tokens.ts`
**Changes**: replace the single `COLOR_TOKENS` export with seven
named exports (light/dark colour, typography, spacing, radius,
light/dark shadow). Token names and values come from the inventory
at
`meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`
(sections "Active semantic layer — `--ac-*` (light)", "Active
semantic layer — `--ac-*` (dark)", "Typography", "Spacing", "Radius",
"Shadow"). Keep the `COLOR_TOKENS` export in place — annotated
`@deprecated` — until Phase 3 deletes it.

Hex values are normalised to lowercase to match the existing
codebase convention; `readCssVar` lower-cases both sides of the
comparison so this is decoupled from the inventory's casing
(which uses uppercase). `--ac-shadow-soft` and `--ac-shadow-lift`
are theme-variant per the inventory's dark override table (lines
154–155) and live in the per-theme exports; the three
`--shadow-card*` / `--shadow-crisp` tokens are theme-invariant
and live in `LIGHT_SHADOW_TOKENS` only.

```ts
export const LIGHT_COLOR_TOKENS = {
  'ac-bg':            '#fbfcfe',
  'ac-bg-raised':     '#ffffff',
  // ... full inventory list (light --ac-* colour layer, including
  //     semantic colours --ac-ok / --ac-warn / --ac-err / --ac-violet
  //     which are theme-invariant and therefore appear *only* here)
} as const

export const DARK_COLOR_TOKENS = {
  'ac-bg':            '#0a111b',
  // ... full inventory list (dark colour overrides only — tokens
  //     redefined under [data-theme="dark"]).
  // NOTE: --ac-ok / --ac-warn / --ac-err / --ac-violet are deliberately
  //       omitted — they are theme-invariant per the inventory and
  //       fall through to the LIGHT_COLOR_TOKENS values under dark.
  //       Future contrast assertions for these tokens against
  //       --ac-bg in dark must read the value from LIGHT_COLOR_TOKENS
  //       and the surface from DARK_COLOR_TOKENS.
} as const

export const TYPOGRAPHY_TOKENS = {
  'ac-font-display': '"Sora", system-ui, sans-serif',
  'ac-font-body':    '"Inter", system-ui, sans-serif',
  'ac-font-mono':    '"Fira Code", ui-monospace, monospace',
  'size-hero':       '68px',
  'size-h1':         '48px',
  // ... full size scale
  'lh-tight':        '1.05',
  // ... line-heights + tracking-caps
} as const

export const SPACING_TOKENS = {
  'sp-1':  '4px',
  'sp-2':  '8px',
  // ... through sp-11: 124px
} as const

export const RADIUS_TOKENS = {
  'radius-sm':   '4px',
  'radius-md':   '8px',
  'radius-lg':   '12px',
  'radius-pill': '999px',
} as const

// Theme-invariant shadows (--shadow-card / --shadow-card-lg /
// --shadow-crisp) plus light-theme values for theme-variant shadows
// (--ac-shadow-soft / --ac-shadow-lift).
export const LIGHT_SHADOW_TOKENS = {
  'shadow-card':    '6px 12px 85px 0px rgba(0, 0, 0, 0.08)',
  'shadow-card-lg': '12px 24px 120px 0px rgba(0, 0, 0, 0.12)',
  'shadow-crisp':   '0 1px 2px rgba(10, 17, 27, 0.06), 0 4px 12px rgba(10, 17, 27, 0.04)',
  'ac-shadow-soft': '0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06)',
  'ac-shadow-lift': '0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10)',
} as const

// Dark-theme overrides — only the theme-variant shadows redefine.
export const DARK_SHADOW_TOKENS = {
  'ac-shadow-soft': '0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4)',
  'ac-shadow-lift': '0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55)',
} as const

export type ColorTokenLight = keyof typeof LIGHT_COLOR_TOKENS
export type ColorTokenDark = keyof typeof DARK_COLOR_TOKENS
export type TypographyToken = keyof typeof TYPOGRAPHY_TOKENS
export type SpacingToken = keyof typeof SPACING_TOKENS
export type RadiusToken = keyof typeof RADIUS_TOKENS
export type LightShadowToken = keyof typeof LIGHT_SHADOW_TOKENS
export type DarkShadowToken = keyof typeof DARK_SHADOW_TOKENS

/**
 * @deprecated Migrating to LIGHT_COLOR_TOKENS / DARK_COLOR_TOKENS in 0033
 * Phase 3. Do not add new consumers. The eight legacy `--color-*`
 * declarations in `global.css` and this export are deleted together in
 * a single commit so parity remains continuous across the migration.
 */
export const COLOR_TOKENS = {
  'color-text':              '#0f172a',
  'color-muted-text':        '#4b5563',
  'color-muted-decorative':  '#9ca3af',
  'color-divider':           '#e5e7eb',
  'color-focus-ring':        '#2563eb',
  'color-warning-bg':        '#fff8e6',
  'color-warning-border':    '#d97706',
  'color-warning-text':      '#7c2d12',
} as const
/** @deprecated See COLOR_TOKENS. */
export type ColorToken = keyof typeof COLOR_TOKENS
```

#### 3. Author `global.css` `:root` and `[data-theme="dark"]` blocks

**File**: `src/styles/global.css`
**Changes**: extend the `:root` block to include every light token
in column-aligned form (light colour, typography, spacing, radius,
light shadow); add a `[data-theme="dark"]` block with the dark
colour overrides plus the per-theme dark shadow overrides
(`--ac-shadow-soft`, `--ac-shadow-lift`); add a
`@media (prefers-color-scheme: dark)` block that applies the same
dark overrides for users who have not explicitly opted into light
via `[data-theme="light"]`. Retain the eight `--color-*`
declarations and the `:focus-visible` rules verbatim — they're
removed in Phase 3 once consumers migrate. The `:root` ordering
keeps `:focus-visible` rules near the bottom of the file unchanged
so the focus-ring tests still match. Hex values are lowercase to
match the existing codebase convention.

The block structure is intentionally **flat** — no nested rules or
nested at-rules inside `:root` or `[data-theme="dark"]`. This
preserves the parity-helper invariant documented above
`readCssVar` in `global.test.ts`.

```css
:root {
  /* Light --ac-* colour layer */
  --ac-bg:            #fbfcfe;
  --ac-bg-raised:     #ffffff;
  /* ... full set including --ac-ok / --ac-warn / --ac-err / --ac-violet */

  /* Typography */
  --ac-font-display:  "Sora", system-ui, sans-serif;
  /* ... full set */

  /* Spacing */
  --sp-1: 4px;
  /* ... through sp-11 */

  /* Radius */
  --radius-sm:   4px;
  /* ... */

  /* Shadow (light + theme-invariant) */
  --shadow-card:    6px 12px 85px 0px rgba(0, 0, 0, 0.08);
  --shadow-card-lg: 12px 24px 120px 0px rgba(0, 0, 0, 0.12);
  --shadow-crisp:   0 1px 2px rgba(10, 17, 27, 0.06), 0 4px 12px rgba(10, 17, 27, 0.04);
  --ac-shadow-soft: 0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06);
  --ac-shadow-lift: 0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10);

  /* Legacy --color-* (deprecated, removed in Phase 3) */
  --color-text:              #0f172a;
  /* ... existing eight */
}

/* MIRROR-A: explicit [data-theme="dark"] block — the canonical source of
   truth read by readCssVar('<name>', 'dark'). Any edit to the dark token
   set MUST also be made to MIRROR-B below; identity is asserted at unit-
   test time by the parity describe in global.test.ts (search for
   "[data-theme=\"dark\"] ↔ @media (prefers-color-scheme: dark) parity"). */
[data-theme="dark"] {
  /* Dark --ac-* colour overrides */
  --ac-bg:            #0a111b;
  /* ... full dark colour override set */

  /* Dark --ac-shadow-* overrides (per inventory lines 154–155) */
  --ac-shadow-soft: 0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4);
  --ac-shadow-lift: 0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55);
}

/* MIRROR-B: honour OS dark-mode preference until 0034 ships the toggle UI;
   an explicit [data-theme="light"] opts back into light. Declarations here
   are a hand-mirrored duplicate of MIRROR-A above (deliberate, so the
   parity helper can remain flat-block-only). */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    /* MUST stay byte-equivalent (modulo whitespace) to MIRROR-A. */
    --ac-bg:            #0a111b;
    /* ... duplicate of the [data-theme="dark"] block */
    --ac-shadow-soft: 0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4);
    --ac-shadow-lift: 0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55);
  }
}

:focus-visible { /* unchanged */ }
@media (forced-colors: active) { /* unchanged */ }
```

Note: the `prefers-color-scheme` block intentionally duplicates the
dark values rather than nesting selectors inside `[data-theme="dark"]`
or wrapping the dark block in a media query. The duplication keeps
the explicit `[data-theme="dark"]` block flat (preserving the
`readCssVar` invariant) and is the single source of truth that the
parity test asserts; the media-query mirror is editor-maintained
alongside it. A small Phase 1 success criterion verifies the two
blocks have identical declarations.

#### 4. Self-host the three font families under `public/fonts/`

**Files**:
- `skills/visualisation/visualise/frontend/public/fonts/` (new directory)
- `skills/visualisation/visualise/frontend/src/styles/global.css` (add `@font-face` block)
- `skills/visualisation/visualise/frontend/index.html` (add `<link rel="preload">` for the two above-the-fold weights)

**Rationale**: keeping fonts same-origin removes the third-party
runtime coupling, eliminates the CSP/SRI/GDPR question entirely
(no `fonts.googleapis.com` / `fonts.gstatic.com` calls), works
offline, and makes integrity hashes pin to the Vite build.

**Changes**:

1. Add the woff2 files. Acquire each weight's woff2 from the family's
   official source (Google Fonts download — the family files are
   licensed under SIL OFL 1.1, redistribution permitted) and place
   under `public/fonts/`:

   ```
   public/fonts/Sora-SemiBold.woff2     (weight 600)
   public/fonts/Sora-Bold.woff2         (weight 700)
   public/fonts/Inter-Regular.woff2     (weight 400)
   public/fonts/Inter-Medium.woff2      (weight 500)
   public/fonts/Inter-SemiBold.woff2    (weight 600)
   public/fonts/Inter-Bold.woff2        (weight 700)
   public/fonts/FiraCode-Regular.woff2  (weight 400)
   public/fonts/FiraCode-Medium.woff2   (weight 500)
   ```

   Alternatively, install the `@fontsource/sora`,
   `@fontsource/inter`, and `@fontsource/fira-code` packages and
   import their `400.css` / `500.css` / `600.css` / `700.css`
   subsets from `src/main.tsx`. The choice is the implementer's;
   the plan's automated AC2 test (§6 below) only asserts that
   `@font-face` declarations exist and resolve to served files.
   The bundled-woff2 approach above is recommended as the more
   transparent option.

2. Add the `@font-face` block to the **top** of `src/styles/global.css`,
   above `:root`:

   ```css
   @font-face {
     font-family: "Sora";
     src: url("/fonts/Sora-SemiBold.woff2") format("woff2");
     font-weight: 600;
     font-style: normal;
     font-display: swap;
   }
   @font-face {
     font-family: "Sora";
     src: url("/fonts/Sora-Bold.woff2") format("woff2");
     font-weight: 700;
     font-style: normal;
     font-display: swap;
   }
   /* ... Inter 400/500/600/700, Fira Code 400/500 — same shape */
   ```

   `font-display: swap` keeps the FOUT brief; the woff2 files are
   served from the same origin so there is no DNS+TLS handshake
   on top of the file fetch.

3. Add critical-path preload hints to `index.html`'s `<head>` for
   the two above-the-fold weights (Inter 400 body, Sora 700 hero):

   ```html
   <link rel="preload" as="font" type="font/woff2"
         href="/fonts/Inter-Regular.woff2" crossorigin="anonymous" />
   <link rel="preload" as="font" type="font/woff2"
         href="/fonts/Sora-Bold.woff2" crossorigin="anonymous" />
   ```

   The `crossorigin="anonymous"` attribute is required even for
   same-origin font preloads (per the HTML spec; fonts are always
   fetched in CORS mode).

4. Verify Vite serves files from `public/` at the expected URLs
   (Vite default: `/fonts/Inter-Regular.woff2` resolves to
   `public/fonts/Inter-Regular.woff2`). No `vite.config.ts` change
   required.

The 0033 PR description should note that the woff2 files are
bundled per SIL OFL 1.1 and include a brief `LICENSE-fonts.md`
under `public/fonts/` listing each family's licence.

#### 5. Update WCAG AA contrast tests + extend `contrast.ts` for rgba composition

**Files**:
- `src/styles/contrast.ts` (extend with `parseRgba` + `composeOverSurface`)
- `src/styles/contrast.test.ts` (refresh assertions for the `--ac-*` palette)

**Pre-resolution of the muted-foreground question**: the inventory
values pass WCAG 2.2 AA comfortably:

| Pair                                                  | Ratio  | Threshold | Status |
| ----------------------------------------------------- | -----: | --------: | :----- |
| light `--ac-fg` (#14161f) on `--ac-bg` (#fbfcfe)      | 17.57  | 4.5       | pass   |
| light `--ac-fg-muted` (#5f6378) on `--ac-bg`          |  5.78  | 4.5       | pass   |
| light `--ac-accent` (#595fc8) on `--ac-bg`            |  5.25  | 3.0 (UI)  | pass   |
| dark `--ac-fg` (#e7e9f2) on `--ac-bg` (#0a111b)       | 15.63  | 4.5       | pass   |
| dark `--ac-fg-muted` (#a0a5b8) on `--ac-bg`           |  7.73  | 4.5       | pass   |
| dark `--ac-accent` (#8a90e8) on `--ac-bg`             |  6.53  | 3.0 (UI)  | pass   |
| dark `--ac-err` (#cb4647) on `--ac-bg`                |  4.06  | 3.0 (UI)  | pass   |
| dark `--ac-violet` (#7b5cd9) on `--ac-bg`             |  3.95  | 3.0 (UI)  | pass   |
| dark `--ac-ok` (#2e8b57) on `--ac-bg`                  |  4.47  | 3.0 (UI)  | pass   |

The previous "if it does not reach 4.5:1, raise in the PR" hedge
is removed — all six assertions pass against the inventory values.

**Extend `contrast.ts`**: today it exports only `parseHex` and
`contrastRatio`. Add `parseRgba` (accepts `rgba(R,G,B,A)` /
`rgb(R,G,B)`) and `composeOverSurface(rgbaOrHex, surfaceHex)` that
returns the resulting hex when the foreground is composited over
the surface (standard `α·F + (1-α)·B` per channel). All new
assertions go through these helpers so a `--ac-stroke-strong:
rgba(32,34,49,0.18)` (the inventory's value) is contrast-tested
against the composed surface (`--ac-bg-card` for raised cards,
`--ac-bg` for the document body).

```ts
// contrast.ts additions
export function parseRgba(value: string): { r: number; g: number; b: number; a: number } { /* ... */ }
export function composeOverSurface(fg: string, surfaceHex: string): string { /* ... */ }
// `contrastRatio` keeps its strict opaque-hex signature; rgba inputs go
// through `contrastRatioComposed` which makes the composition surface
// a required argument. The split avoids the flag-argument smell where
// a string-format check silently selects the calculation mode.
export function contrastRatio(fg: string, bg: string): number { /* hex/hex only */ }
export function contrastRatioComposed(
  fgRgbaOrHex: string,
  bgHex: string,
  surfaceHex: string,
): number {
  return contrastRatio(composeOverSurface(fgRgbaOrHex, surfaceHex), bgHex)
}
```

**Test file**:

```ts
import { describe, it, expect } from 'vitest'
import {
  contrastRatio,
  contrastRatioComposed,
  composeOverSurface,
  parseHex,
  parseRgba,
} from './contrast'
import { LIGHT_COLOR_TOKENS, DARK_COLOR_TOKENS } from './tokens'

const lightBg = LIGHT_COLOR_TOKENS['ac-bg']
const darkBg = DARK_COLOR_TOKENS['ac-bg']

// Helper unit tests — the contrast suite below depends on these being
// implemented correctly; without these pinned cases a buggy alpha-blend
// formula could silently mask wrong intermediate colours.
describe('parseHex', () => {
  it('parses 6-digit hex (canonical token form)', () => {
    expect(parseHex('#fbfcfe')).toEqual({ r: 251, g: 252, b: 254 })
  })
  it('parses 3-digit hex by expansion', () => {
    expect(parseHex('#fff')).toEqual({ r: 255, g: 255, b: 255 })
  })
  it('throws or normalises 8-digit hex (alpha) — pin behaviour explicitly', () => {
    // Token convention is 6-digit lowercase (per Phase 1 §2). 8-digit hex
    // is not a token-side input but could be produced by a composition
    // helper. Pin the policy here so a bug doesn't ship silently.
    // Implementer chooses one of: (a) throw with a clear message, or
    // (b) parse the alpha channel and return { r, g, b, a }. Whichever
    // is chosen, this test documents the contract.
    expect(() => parseHex('#fbfcfeff')).not.toThrow()
  })
})

describe('parseRgba', () => {
  it('parses opaque rgb()', () => {
    expect(parseRgba('rgb(255, 0, 0)')).toEqual({ r: 255, g: 0, b: 0, a: 1 })
  })
  it('parses rgba() with fractional alpha', () => {
    expect(parseRgba('rgba(0, 0, 0, 0.5)')).toEqual({ r: 0, g: 0, b: 0, a: 0.5 })
  })
  it('tolerates whitespace and integer alpha', () => {
    expect(parseRgba('rgba( 16, 32, 64, 1 )')).toEqual({ r: 16, g: 32, b: 64, a: 1 })
  })
})

describe('composeOverSurface', () => {
  it('returns the surface when foreground alpha is 0', () => {
    expect(composeOverSurface('rgba(255, 255, 255, 0)', '#000000').toLowerCase())
      .toBe('#000000')
  })
  it('returns the foreground when alpha is 1', () => {
    expect(composeOverSurface('rgba(128, 128, 128, 1)', '#000000').toLowerCase())
      .toBe('#808080')
  })
  it('blends 50% black over white to mid-grey', () => {
    expect(composeOverSurface('rgba(0, 0, 0, 0.5)', '#ffffff').toLowerCase())
      .toBe('#808080')
  })
  it('accepts hex foreground (treated as alpha=1)', () => {
    expect(composeOverSurface('#cc0000', '#ffffff').toLowerCase())
      .toBe('#cc0000')
  })
})

describe('contrastRatio (regression — opaque-hex path matches legacy)', () => {
  it('matches the legacy hex-only ratio for fg/bg', () => {
    // Black on white: 21:1 exact
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 1)
  })
})

describe('contrastRatioComposed', () => {
  it('composes 50% black over white and contrasts against white', () => {
    // 50% black over white = #808080 → contrast vs white ≈ 3.95
    expect(contrastRatioComposed('rgba(0, 0, 0, 0.5)', '#ffffff', '#ffffff'))
      .toBeCloseTo(3.95, 1)
  })
  it('opaque hex foreground passes through composeOverSurface unchanged', () => {
    expect(contrastRatioComposed('#000000', '#ffffff', '#ffffff'))
      .toBeCloseTo(21, 1)
  })
})

describe('design-token contrast (WCAG 2.2 AA, light)', () => {
  it('--ac-fg on --ac-bg ≥ 4.5:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-fg'], lightBg)).toBeGreaterThanOrEqual(4.5)
  })
  it('--ac-fg-muted on --ac-bg ≥ 4.5:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-fg-muted'], lightBg)).toBeGreaterThanOrEqual(4.5)
  })
  it('--ac-accent on --ac-bg ≥ 3:1 (UI component, WCAG 1.4.11)', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-accent'], lightBg)).toBeGreaterThanOrEqual(3)
  })
  it('composed --ac-stroke-strong on --ac-bg ≥ 3:1 (WCAG 1.4.11, stateful UI)', () => {
    expect(
      contrastRatioComposed(LIGHT_COLOR_TOKENS['ac-stroke-strong'], lightBg, lightBg),
    ).toBeGreaterThanOrEqual(3)
  })
  // Composed warning callout: warn-text on warn-tinted surface
  it('--ac-warn text on warn-tinted bg ≥ 4.5:1', () => {
    const warnBg = composeOverSurface('rgba(217,143,46,0.12)', lightBg) // matches color-mix(--ac-warn 12%, --ac-bg)
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-warn'], warnBg)).toBeGreaterThanOrEqual(4.5)
  })
})

describe('design-token contrast (WCAG 2.2 AA, dark)', () => {
  it('--ac-fg on --ac-bg ≥ 4.5:1', () => {
    expect(contrastRatio(DARK_COLOR_TOKENS['ac-fg'], darkBg)).toBeGreaterThanOrEqual(4.5)
  })
  it('--ac-fg-muted on --ac-bg ≥ 4.5:1', () => {
    expect(contrastRatio(DARK_COLOR_TOKENS['ac-fg-muted'], darkBg)).toBeGreaterThanOrEqual(4.5)
  })
  it('--ac-accent on --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(DARK_COLOR_TOKENS['ac-accent'], darkBg)).toBeGreaterThanOrEqual(3)
  })
  // Theme-invariant semantic colours read from the LIGHT export by design
  // (per the inventory; --ac-warn / --ac-err / --ac-ok / --ac-violet do
  // not redefine under [data-theme="dark"]). All four are asserted at the
  // 3:1 UI-component threshold rather than 4.5:1 because they are typically
  // used as borders/backgrounds; if any of them is used as inline text
  // against --ac-bg, that consumer site is responsible for tightening to
  // 4.5:1 via composition or a darker derived shade (see the warn-text
  // fallback recipe in Phase 3 §3).
  it('--ac-warn (theme-invariant) on dark --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-warn'], darkBg)).toBeGreaterThanOrEqual(3)
  })
  it('--ac-err (theme-invariant) on dark --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-err'], darkBg)).toBeGreaterThanOrEqual(3)
  })
  it('--ac-ok (theme-invariant) on dark --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-ok'], darkBg)).toBeGreaterThanOrEqual(3)
  })
  it('--ac-violet (theme-invariant) on dark --ac-bg ≥ 3:1', () => {
    expect(contrastRatio(LIGHT_COLOR_TOKENS['ac-violet'], darkBg)).toBeGreaterThanOrEqual(3)
  })
})
```

If the warn-text-on-warn-tinted-bg assertion fails for the chosen
percentage (12% in `color-mix`), that is a real signal: either
darken the text via `color-mix(in srgb, var(--ac-err) X%, var(--ac-warn))`
for the text, or strengthen the tint percentage. The plan's
mapping convention (Phase 4 §1) is locked against this assertion's
outcome, not picked arbitrarily.

#### 6. Automated AC2 wiring test

**File**: `src/styles/fonts.test.ts` (new)
**Purpose**: AC2 demands the three families load and that each is
referenced via a typography token. With self-hosting, "load" means
the woff2 files exist and `@font-face` declarations point at them.
This test asserts the wiring at unit-test time so a future edit
that drops a weight or breaks a path is caught in CI.

```ts
import { describe, it, expect } from 'vitest'
import { readdirSync, readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { fileURLToPath } from 'node:url'
import globalCss from './global.css?raw'
import indexHtml from '../../index.html?raw'
import { TYPOGRAPHY_TOKENS } from './tokens'

// `frontend/package.json` declares `"type": "module"`, so `__dirname` is
// not defined. Vite's `import.meta.glob` deliberately excludes `public/`
// from the module graph (those files are served as static assets, not
// processed by the bundler), so a glob over `/public/fonts/*.woff2` would
// silently return an empty record and pass-vacuously. Resolve the directory
// via `import.meta.url` and read with node:fs instead — this works in
// Vitest under ESM because `import.meta.url` is well-defined.
const fontsDir = fileURLToPath(new URL('../../public/fonts/', import.meta.url))
const fontFiles = readdirSync(fontsDir)

const EXPECTED_FONT_FILES = [
  'Sora-SemiBold.woff2',
  'Sora-Bold.woff2',
  'Inter-Regular.woff2',
  'Inter-Medium.woff2',
  'Inter-SemiBold.woff2',
  'Inter-Bold.woff2',
  'FiraCode-Regular.woff2',
  'FiraCode-Medium.woff2',
]

describe('AC2: self-hosted font wiring', () => {
  for (const file of EXPECTED_FONT_FILES) {
    it(`public/fonts/${file} exists`, () => {
      expect(fontFiles).toContain(file)
    })
  }

  for (const family of ['Sora', 'Inter', 'Fira Code']) {
    it(`global.css declares @font-face for "${family}"`, () => {
      expect(globalCss).toMatch(
        new RegExp(`@font-face\\s*\\{[^}]*font-family:\\s*"${family}"`, 'i'),
      )
    })
  }

  // AC2's second clause: each family is referenced *via a typography token*.
  for (const [family, tokenKey] of [
    ['Sora', 'ac-font-display'],
    ['Inter', 'ac-font-body'],
    ['Fira Code', 'ac-font-mono'],
  ] as const) {
    it(`TYPOGRAPHY_TOKENS["${tokenKey}"] references "${family}"`, () => {
      expect(TYPOGRAPHY_TOKENS[tokenKey]).toMatch(new RegExp(`"${family}"`))
    })
  }

  it('global.css uses font-display: swap on every @font-face', () => {
    const blocks = globalCss.match(/@font-face\s*\{[^}]+\}/g) ?? []
    expect(blocks.length).toBeGreaterThanOrEqual(8)
    for (const block of blocks) {
      expect(block).toMatch(/font-display:\s*swap/)
    }
  })

  it('index.html preloads at least one critical-path font', () => {
    expect(indexHtml).toMatch(
      /<link[^>]*rel="preload"[^>]*as="font"[^>]*type="font\/woff2"[^>]*\/fonts\/[^"]+\.woff2/,
    )
  })

  it('no third-party font origins are referenced', () => {
    expect(indexHtml).not.toMatch(/fonts\.googleapis\.com|fonts\.gstatic\.com/)
    expect(globalCss).not.toMatch(/fonts\.googleapis\.com|fonts\.gstatic\.com/)
  })

  // Supply-chain integrity: compute SHA-256 of each woff2 file and compare
  // against `public/fonts/SHA256SUMS`. The test runs unconditionally —
  // missing SUMS is a hard failure, not a silent skip.
  it('woff2 binary checksums match public/fonts/SHA256SUMS', () => {
    const sumsPath = fileURLToPath(
      new URL('../../public/fonts/SHA256SUMS', import.meta.url),
    )
    const sums = readFileSync(sumsPath, 'utf8')
    // SHA256SUMS format: `<hex>  <filename>\n` per line (sha256sum default).
    const expected = new Map<string, string>()
    for (const line of sums.split('\n').filter(Boolean)) {
      const [hex, file] = line.trim().split(/\s+/)
      expected.set(file, hex)
    }
    for (const file of EXPECTED_FONT_FILES) {
      const bytes = readFileSync(`${fontsDir}${file}`)
      const actual = createHash('sha256').update(bytes).digest('hex')
      expect(actual, `checksum mismatch for ${file}`).toBe(expected.get(file))
    }
  })
})
```

The test imports `readFileSync` and `createHash` (from `node:fs` and
`node:crypto`); add them to the imports at the top of `fonts.test.ts`.

**Note on font integrity**: Either (a) prefer `@fontsource/sora` /
`@fontsource/inter` / `@fontsource/fira-code` so the npm lockfile's
integrity hashes pin each weight (in which case the SHA256SUMS test
becomes redundant and can be deleted), or (b) commit
`public/fonts/SHA256SUMS` (one-line `cd public/fonts && sha256sum
*.woff2 > SHA256SUMS` during Phase 1) and let the test enforce it.
The plan ships option (b) by default; the team can switch to (a)
in a follow-up by deleting both the SUMS file and the integrity
test.

### Success Criteria

#### Automated Verification

- [ ] Frontend tests pass: `mise run test:unit:frontend`
- [ ] Frontend builds (typechecks): `mise run build:frontend`
- [ ] All parity tests in `global.test.ts` pass for the seven
      token categories (light colour, dark colour, typography,
      spacing, radius, light shadow, dark shadow) plus the
      retained legacy `COLOR_TOKENS` describe.
- [ ] All eight WCAG AA / 1.4.11 assertions in `contrast.test.ts`
      pass (six base + stroke composition + warn-on-warn-tint).
- [ ] All AC2 wiring assertions in `fonts.test.ts` pass.
- [ ] Playwright baseline capture for the seven AC6 routes × two
      themes is committed under
      `tests/visual-regression/__screenshots__/`.
- [ ] The `[data-theme="dark"]` declarations and the
      `@media (prefers-color-scheme: dark)` declarations are
      identical (a small assertion in `global.test.ts` that
      extracts both blocks and asserts equality).

#### Manual Verification

- [ ] DevTools Network panel shows 200 responses for the woff2
      files at `/fonts/...` (same-origin) when the dev server
      (`mise run build:server:dev` + the visualiser binary, or
      `npm run dev` from the frontend root) loads the app, and
      **no** outbound requests to `fonts.googleapis.com` or
      `fonts.gstatic.com`.
- [ ] In DevTools, `getComputedStyle(document.documentElement)`
      reports the new `--ac-*` / `--sp-*` / `--radius-*` / etc
      tokens.
- [ ] Toggling OS dark-mode preference (or
      `Emulate CSS prefers-color-scheme: dark` in DevTools)
      switches the palette without manually setting `data-theme`.

---

## Phase 2: Migration Enforcement Test Harness

### Overview

Stand up a vitest file that enforces AC3, AC4, AC5, and the
no-fallback `var(--ac-*)` convention at unit-test time, scanning
every CSS module file in `src/`. The test consults a typed
constant `EXCEPTIONS` listing every literal that the test permits.

The harness lands **green** by pre-populating `EXCEPTIONS` with
every current hex/px/rem literal in the codebase (a programmatic
inventory captured by a one-off scan run during Phase 2
implementation). Phases 3–4 then progressively *delete*
`EXCEPTIONS` entries as files migrate: removing a literal from
the CSS while leaving its `EXCEPTIONS` entry leaves the test green
(but the entry becomes stale — the per-occurrence count check
catches that), and removing the `EXCEPTIONS` entry while the
literal is still in CSS turns the test red — exactly the TDD
gate the migration needs.

The final state of `EXCEPTIONS` (after all phases land) is the
truly-irreducible set: 1–3px hairlines that fall below `--radius-sm`,
layout literals that have no scale equivalent, and any other
literal the implementer and reviewer agree to keep. The PR's
"Irreducible-literal exceptions" section is **mechanically
generated** from `EXCEPTIONS` (a small script in `scripts/` runs
at PR-prep time) so the test and the PR cannot drift.

### Changes Required

#### 1. Add `migration.test.ts`

**File**: `src/styles/migration.test.ts` (new)
**Changes**: import all CSS files (component modules + globals) via
Vite's `import.meta.glob` using the modern `query: '?raw',
import: 'default'` form (the older `{ as: 'raw' }` form is
deprecated in Vite 5+). All test inputs are eager-loaded; no
top-level `await`.

```ts
import { describe, it, expect } from 'vitest'
import {
  LIGHT_COLOR_TOKENS,
  DARK_COLOR_TOKENS,
  TYPOGRAPHY_TOKENS,
  SPACING_TOKENS,
  RADIUS_TOKENS,
  LIGHT_SHADOW_TOKENS,
  DARK_SHADOW_TOKENS,
} from './tokens'

const cssModules = import.meta.glob('../**/*.module.css', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>

const cssGlobals = import.meta.glob('../**/*.global.css', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>

// All `0` resets are auto-permitted (admitted by AC4's escape-hatch);
// the regex excludes them at the source so they never need EXCEPTIONS
// entries — keeps the list focused on genuine exceptions.
const HEX_RE = /#[0-9a-fA-F]{3,8}\b/g
const PX_REM_EM_RE = /\b(?!0(?:px|rem|em)\b)\d+(?:\.\d+)?(?:px|rem|em)\b/g
const VAR_REF_RE = /var\(\s*--([\w-]+)\s*[,)]/g
// Scoped to --ac-* / --sp-* / --radius-* / --size-* / --shadow-* / --lh-*
// tokens — i.e. the new layered set. Legacy `--color-*` fallback sites
// remain present at Phase 2 commit and are deleted by Phase 3; this regex
// deliberately does not flag them so the harness lands green.
const VAR_FALLBACK_RE = /var\(\s*--(?:ac-|sp-|radius-|size-|shadow-|lh-|tracking-)[\w-]+\s*,/g
const VAR_COUNT_RE = /var\(\s*--/g

// Per-occurrence exception model. `count` is how many times this
// `(file, literal)` pair is allowed to match; when the implementer
// migrates one occurrence, they decrement the count or remove the
// entry. Adding a new exception requires `count: 1` and a reason.
// `file` is the path **relative to `src/`** to disambiguate any future
// modules that share a basename across directories.
type Exception = { file: string; literal: string; count: number; reason: string }

// Phase 2 lands this constant pre-populated by a one-off scan
// (`scripts/scan-css-literals.ts`, run once during implementation
// and committed alongside the harness). Each entry is annotated
// `to-migrate` until phases 3–4 either delete the entry or
// re-annotate it as `irreducible` (1–3px hairline, layout literal,
// etc). The PR-prep script lists only `irreducible` entries in
// the PR description.
const EXCEPTIONS: ReadonlyArray<Exception & { kind: 'to-migrate' | 'irreducible' }> = [
  // Example irreducible: hairlines and pill radii below the scale.
  // Paths are relative to `src/`.
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '1.5px', count: 2,
    kind: 'irreducible',
    reason: 'coloured ring widths — below --radius-sm/--sp-1 floor' },
  { file: 'routes/lifecycle/LifecycleClusterView.module.css', literal: '800px', count: 1,
    kind: 'irreducible',
    reason: 'max-width — no spacing-scale equivalent' },
  // ... pre-populated to-migrate entries elided. See
  // `scripts/scan-css-literals.ts` output committed at Phase 2 time.
]

// Build O(1) lookup maps once at module load — at the Phase 2 baseline
// `EXCEPTIONS` is large (~360 entries); per-literal linear scans across
// the whole array would be super-linear in CI.
const exceptionsByFile = new Map<string, Map<string, number>>()
for (const e of EXCEPTIONS) {
  let inner = exceptionsByFile.get(e.file)
  if (!inner) {
    inner = new Map()
    exceptionsByFile.set(e.file, inner)
  }
  inner.set(e.literal, (inner.get(e.literal) ?? 0) + e.count)
}

// Map vite-glob keys (e.g. '../routes/lifecycle/Foo.module.css') to the
// src-relative form used by EXCEPTIONS.file ('routes/lifecycle/Foo.module.css').
// The function asserts the input shape so a future refactor that changes
// the glob root (and therefore the key shape) fails loudly rather than
// silently producing wrong keys that never match EXCEPTIONS entries.
function srcRelative(globKey: string): string {
  if (!globKey.startsWith('../') || globKey.startsWith('../../')) {
    throw new Error(
      `srcRelative: unexpected glob key shape "${globKey}". ` +
        `Expected exactly one "../" prefix (test sits at src/styles/, globs "../**/*.module.css"). ` +
        `If the test or glob has been moved, update srcRelative accordingly.`,
    )
  }
  return globKey.slice(3)
}

function permittedCount(file: string, literal: string): number {
  return exceptionsByFile.get(srcRelative(file))?.get(literal) ?? 0
}

function violations(matches: string[], file: string): string[] {
  const counts = new Map<string, number>()
  for (const m of matches) counts.set(m, (counts.get(m) ?? 0) + 1)
  const result: string[] = []
  for (const [literal, observed] of counts) {
    const allowed = permittedCount(file, literal)
    if (observed > allowed) {
      // Surface every excess occurrence in the failure message.
      for (let i = 0; i < observed - allowed; i++) result.push(literal)
    }
  }
  return result
}

const allCss = { ...cssModules, ...cssGlobals }

describe('AC3: no hex literals outside EXCEPTIONS', () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} hex literals all accounted for`, () => {
      const matches = [...css.matchAll(HEX_RE)].map((m) => m[0])
      expect(violations(matches, path)).toEqual([])
    })
  }
})

describe('AC4: no px/rem/em literals outside EXCEPTIONS (0-resets auto-excluded)', () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} px/rem/em literals all accounted for`, () => {
      const matches = [...css.matchAll(PX_REM_EM_RE)].map((m) => m[0])
      expect(violations(matches, path)).toEqual([])
    })
  }
})

describe('var(--token, fallback) two-arg form is retired', () => {
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} contains no var(--*, fallback) sites`, () => {
      const fallbacks = [...css.matchAll(VAR_FALLBACK_RE)].map((m) => m[0])
      expect(fallbacks).toEqual([])
    })
  }
})

describe('color-mix() convention (Phase 4 special conventions)', () => {
  // Locked-in percentage ladder: 8 (default tinted bg), 18 (hover state),
  // 30 (stroke/border tint). Composition surface always var(--ac-bg).
  const COLOR_MIX_RE = /color-mix\(\s*in\s+srgb\s*,\s*var\(--ac-(err|warn|ok|violet)\)\s+(\d+)%\s*,\s*var\(--ac-bg\)\s*\)/g
  const COLOR_MIX_ANY_RE = /color-mix\(/g
  const ALLOWED_PERCENTAGES = new Set([8, 18, 30])

  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} color-mix sites use the locked-in convention`, () => {
      const totalSites = (css.match(COLOR_MIX_ANY_RE) ?? []).length
      const conventionalSites = [...css.matchAll(COLOR_MIX_RE)]
      // Every color-mix site must match the locked-in shape.
      expect(conventionalSites.length).toBe(totalSites)
      // Every percentage must be one of the three sanctioned values.
      for (const m of conventionalSites) {
        expect(ALLOWED_PERCENTAGES.has(parseInt(m[2], 10))).toBe(true)
      }
    })
  }
})

describe('var(--NAME) references resolve to declared tokens', () => {
  const declared = new Set([
    ...Object.keys(LIGHT_COLOR_TOKENS),
    ...Object.keys(DARK_COLOR_TOKENS),
    ...Object.keys(TYPOGRAPHY_TOKENS),
    ...Object.keys(SPACING_TOKENS),
    ...Object.keys(RADIUS_TOKENS),
    ...Object.keys(LIGHT_SHADOW_TOKENS),
    ...Object.keys(DARK_SHADOW_TOKENS),
  ])
  for (const [path, css] of Object.entries(allCss)) {
    it(`${path} references only declared tokens`, () => {
      const refs = [...css.matchAll(VAR_REF_RE)].map((m) => m[1])
      const unknown = refs.filter((name) => !declared.has(name))
      expect(unknown).toEqual([])
    })
  }
})

// AC5 enforcement is a true two-sided ratchet:
//
// - `AC5_FLOOR` is the *committed minimum*. It MUST equal the value
//   observed at the last committed state (or below it by no more than
//   AC5_REGRESSION_SLACK). The implementer bumps AC5_FLOOR upward in
//   the same commit that adds new var(--*) references; any commit that
//   removes references without lowering AC5_FLOOR fails immediately.
// - `AC5_TARGET = 300` is the work-item contract. The harness fails if
//   AC5_FLOOR is ever set above the observed count, and the final
//   commit must observe at least AC5_TARGET. The pre-merge guard
//   (Wave 4b end) trips a hard failure if the migration ends below
//   the target.
//
// Bumping protocol: at each Wave 4a / 4b commit, run the harness, read
// the printed observed count from a green run, set AC5_FLOOR equal to it.
const AC5_FLOOR = 6 // start; raise per commit as var refs grow
const AC5_TARGET = 300 // contract from work item AC5
const AC5_REGRESSION_SLACK = 0 // tighten to allow zero per-commit regression

describe('AC5: aggregate var(--*) coverage (two-sided ratchet)', () => {
  const observed = Object.values(cssModules).reduce(
    (acc, css) => acc + (css.match(VAR_COUNT_RE)?.length ?? 0),
    0,
  )

  it(`observed count (${observed}) is at least AC5_FLOOR (${AC5_FLOOR})`, () => {
    expect(observed).toBeGreaterThanOrEqual(AC5_FLOOR - AC5_REGRESSION_SLACK)
  })

  it(`AC5_FLOOR (${AC5_FLOOR}) is not above observed (${observed}) — bump protocol followed`, () => {
    // Catches the failure mode where AC5_FLOOR is bumped *ahead* of the
    // commit that adds the supporting references; if true it indicates
    // the implementer raised the floor without landing the migration.
    expect(AC5_FLOOR).toBeLessThanOrEqual(observed)
  })

  // The final-state assertion is the work-item AC5 contract. To preserve
  // "trunk stays green" within the single-PR commit history, this `it()`
  // auto-skips while AC5_FLOOR < AC5_TARGET, and auto-runs once the
  // implementer sets AC5_FLOOR === AC5_TARGET. The implementer's wave-4b
  // commit MUST land both the migrations AND the floor bump that flips
  // this from skipped to passing.
  const finalStateActive = AC5_FLOOR >= AC5_TARGET
  ;(finalStateActive ? it : it.skip)(
    `(final-state gate) observed reaches AC5_TARGET (${AC5_TARGET})`,
    () => {
      expect(observed).toBeGreaterThanOrEqual(AC5_TARGET)
    },
  )
})

// Build the inverse map (vite-glob path → src-relative path → CSS body)
// once for hygiene checks.
const cssBySrcRelative = new Map<string, string>()
for (const [globKey, css] of Object.entries(allCss)) {
  cssBySrcRelative.set(srcRelative(globKey), css)
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

describe('EXCEPTIONS hygiene', () => {
  it('every EXCEPTIONS entry resolves to exactly one CSS file', () => {
    const unresolved: Exception[] = []
    for (const e of EXCEPTIONS) {
      if (!cssBySrcRelative.has(e.file)) unresolved.push(e)
    }
    expect(unresolved).toEqual([])
  })

  it('declared count equals observed count (no stale entries, no over-count)', () => {
    const mismatches: Array<{
      file: string
      literal: string
      declared: number
      observed: number
    }> = []
    for (const [file, literalMap] of exceptionsByFile) {
      const css = cssBySrcRelative.get(file)
      if (!css) continue // resolved by the previous test
      // Count occurrences using the *same* regex family the migration tests
      // use (HEX_RE / PX_REM_EM_RE), filtered to the exact literal string.
      // A naive substring scan would count `1px` inside `11px`/`21px`, etc.
      const hexHits = [...css.matchAll(HEX_RE)].map((m) => m[0])
      const unitHits = [...css.matchAll(PX_REM_EM_RE)].map((m) => m[0])
      const allHits = [...hexHits, ...unitHits]
      for (const [literal, declared] of literalMap) {
        const observed = allHits.filter((h) => h === literal).length
        if (observed !== declared) {
          mismatches.push({ file, literal, declared, observed })
        }
      }
    }
    // Equality assertion: catches both stale entries (declared > observed)
    // and silent gate-relaxation (declared exceeds observed because a
    // literal was migrated but the count was not decremented).
    expect(mismatches).toEqual([])
  })
})
```

The harness lands **green** because `EXCEPTIONS` is pre-populated
(via the one-off `scripts/scan-css-literals.ts` scan) with every
current literal at its observed count. The trunk never sees a red
state. As phases 3–4 land, each migrated literal removes its
`EXCEPTIONS` entry (or decrements its count) and the corresponding
CSS is updated in the same commit. The `EXCEPTIONS hygiene` test
catches the case where an entry is forgotten after the literal is
dropped.

#### 2. Add the literal-scanning script

**File**: `skills/visualisation/visualise/frontend/scripts/scan-css-literals.ts`
(new)
**Purpose**: produces the initial `EXCEPTIONS` pre-population at
Phase 2 implementation time, and (later) generates the PR
description's "Irreducible-literal exceptions" section from the
final `EXCEPTIONS` constant. Runs under `tsx` or `ts-node` from
the frontend root.

```ts
// Walks src/ for *.module.css and *.global.css; emits a JSON array
// of { file, literal, count, kind: 'to-migrate', reason: '<TODO>' }
// suitable for pasting into EXCEPTIONS as the Phase 2 starting
// state. Re-run as a sanity check during phases 3–4.
```

The script's output is committed alongside the harness in Phase 2.
The implementer hand-edits `kind` and `reason` to mark the genuinely
irreducible entries before Phase 4 wraps; phases 3–4 delete the
`to-migrate` entries as files migrate.

### Success Criteria

#### Automated Verification

- [ ] `migration.test.ts` exists and is **green** at Phase 2
      commit time (`mise run test:unit:frontend`), reflecting the
      pre-populated `EXCEPTIONS` baseline.
- [ ] AC5 aggregate threshold ratchets monotonically via the
      `AC5_BASELINE` constant: 6 at Phase 2 commit, ~150 at end
      of Wave 4a, 300 at end of Wave 4b. Each ratchet bump lands
      in the same commit that adds the migration making it
      attainable, so the gate is enforced per-commit rather than
      deferred to the final phase.
- [ ] Type-check passes: `mise run build:frontend`.
- [ ] No two-arg `var(--ac-*, fallback)` (or `--sp-*` / `--radius-*` /
      etc) sites are introduced. The fallback regex is scoped to
      the new layered set, so the six legacy `var(--color-*,
      #fallback)` sites do not break Phase 2 — they are removed
      cleanly by Phase 3.

#### Manual Verification

- [ ] `EXCEPTIONS` baseline at Phase 2 reflects the current
      literal inventory; running the harness with no migration
      done is green.
- [ ] `scripts/scan-css-literals.ts` re-run after each migration
      wave produces a strictly smaller list than the previous
      run.

---

## Phase 3: Migrate Globals + Retire Legacy `--color-*`

### Overview

Cleanest, smallest migration. Removes the parallel
`--color-*` token system entirely:

1. Migrate the two hex literals in `wiki-links.global.css`.
2. Migrate the six `var(--color-*, #fallback)` consumer sites in
   `SidebarFooter.module.css` (lines 6, 7, 19) and
   `LibraryDocView.module.css` (lines 24, 25, 26) onto bare
   `var(--ac-*)` references (no fallbacks).
3. Delete the eight `--color-*` declarations from `global.css`,
   delete `COLOR_TOKENS` and `ColorToken` exports from `tokens.ts`,
   and delete any references to them from `contrast.test.ts`
   (already removed in Phase 1).

### Changes Required

#### 1. `wiki-links.global.css`

**File**: `src/styles/wiki-links.global.css`
**Changes**:

- Line 11: `color: #9ca3af;` → `color: var(--ac-fg-faint);`
- Line 18: `color: #6b7280;` → `color: var(--ac-fg-muted);`

#### 2. `SidebarFooter.module.css`

**File**: `src/components/SidebarFooter/SidebarFooter.module.css`
**Changes**:

- Line 6: `var(--color-muted-text, #4b5563)` → `var(--ac-fg-muted)`
- Line 7: `var(--color-divider, #e5e7eb)` → `var(--ac-stroke-soft)`
- Line 19: `var(--color-warning-text, #7c2d12)` →
  `var(--ac-warn)` *(was `--ac-err` in the draft; corrected to
  the warn family so bg/border/text agree semantically — see
  Phase 1 §5 contrast tests for the warn-on-tinted-bg
  assertion that this mapping must satisfy)*

(The exact `--ac-*` mapping above is best-effort — the
implementer reviews each per the AC6 visual-parity rule and
documents any drift in the PR.)

#### 3. `LibraryDocView.module.css`

**File**: `src/routes/library/LibraryDocView.module.css`
**Changes**:

- Line 24: `var(--color-warning-bg, #fff8e6)` → composed via
  `color-mix(in srgb, var(--ac-warn) 12%, var(--ac-bg))` (per
  the Phase 4 `color-mix` convention). The composed surface
  swaps with theme because `--ac-bg` redefines under
  `[data-theme="dark"]`.
- Line 25: `var(--color-warning-border, #d97706)` → `var(--ac-warn)`
- Line 26: `var(--color-warning-text, #7c2d12)` →
  `var(--ac-warn)` *(was `--ac-err` in the draft; same
  correction as SidebarFooter line 19. If contrast against the
  composed warn-bg fails the Phase 1 §5 assertion, fall back to
  a darker derived shade — `color-mix(in srgb, var(--ac-err)
  40%, var(--ac-warn))` — and document the drift in the PR; do
  not silently use `--ac-err` raw.)*

#### 4. Retire `--color-*` — atomic CSS + TS + test deletion

All three deletions land in **a single commit** to keep the
parity invariant continuous (deleting one half before the other
would briefly fail the legacy `COLOR_TOKENS` parity describe
retained in Phase 1):

**File**: `src/styles/global.css`
**Changes**: delete the eight `--color-*` declarations from `:root`.
Retain `:focus-visible` and the `forced-colors` block — replace
`var(--color-focus-ring)` with `var(--ac-accent)`.

**File**: `src/styles/tokens.ts`
**Changes**: delete `COLOR_TOKENS` and `ColorToken` exports
(including their `@deprecated` annotations).

**File**: `src/styles/global.test.ts`
**Changes**: delete the `tokens.ts ↔ global.css :root parity
(legacy --color-*)` describe block and remove the
`COLOR_TOKENS` import. The remaining new-token parity blocks
continue to enforce CSS↔TS sync.

#### 5. Update `migration.test.ts` `EXCEPTIONS`

**File**: `src/styles/migration.test.ts`
**Changes**: delete every `EXCEPTIONS` entry that referenced a
literal in the four files migrated above (`wiki-links.global.css`,
`SidebarFooter.module.css`, `LibraryDocView.module.css`, and the
`global.css` legacy block). The `EXCEPTIONS hygiene` test will
fail loudly if any stale entry is forgotten.

### Success Criteria

#### Automated Verification

- [ ] `mise run test:unit:frontend` passes (parity, contrast,
      and the three migrated files in `migration.test.ts`).
- [ ] `mise run build:frontend` passes (no orphan references to
      `COLOR_TOKENS` / `ColorToken`).
- [ ] From `skills/visualisation/visualise/frontend/`:
      `rg '#[0-9a-fA-F]{3,8}\b' --type css src/styles/wiki-links.global.css`
      returns zero matches.
- [ ] `rg '\-\-color-' src/` returns zero matches outside the
      Phase 1 typed test deletion.

#### Manual Verification

- [ ] Sidebar footer renders with the new muted / divider /
      warning palette in both light and dark themes (manually
      toggle `data-theme` via DevTools).
- [ ] Library doc view warning callouts render with the new
      warning palette in both themes.
- [ ] Wiki-link pending and unresolved styles appear correctly
      against new background tokens.

---

## Phase 4: Component Module Migration

### Overview

Migrate the 16 CSS modules in two waves. Wave 4a handles the three
worst offenders so the implementer settles judgement-call decisions
(error tints, two-blue collapse, font-size bucketing,
`box-shadow` mappings) up front. Wave 4b applies the locked-in
mappings to the remaining 13 modules.

### Wave 4a: Worst Offenders (TDD per file)

For each file: (1) confirm `migration.test.ts` reports the file's
literals as `to-migrate` exceptions in `EXCEPTIONS`; (2) migrate
hex literals onto colour tokens; (3) migrate px/rem literals onto
spacing/radius/typography tokens; (4) for any literal that has no
clean mapping, re-annotate its `EXCEPTIONS` entry from
`to-migrate` to `irreducible` with a one-line `reason`;
(5) delete the migrated entries from `EXCEPTIONS`; (6) re-run
tests; (7) green.

#### Wave 4a → 4b decision artefact

**File**: `meta/work/0033-token-mapping-conventions.md` (new)
**Purpose**: capture every judgement-call `from → to` decision made
during Wave 4a so Wave 4b applies them mechanically and the PR
review has a single audit trail rather than commit-by-commit
prose. Format:

```markdown
# 0033 Token Mapping Conventions

Locked-in during Wave 4a from observation of three worst-offender
modules; applied unchanged to Wave 4b.

## Colour
- `#1d4ed8`, `#2563eb` → `var(--ac-accent)` (indigo collapse;
  AC6 drift documented in PR)
- `#fef2f2` → `color-mix(in srgb, var(--ac-err) 8%, var(--ac-bg))`
- `#fecaca` → `color-mix(in srgb, var(--ac-err) 30%, var(--ac-bg))`
- `#fee2e2` → `color-mix(in srgb, var(--ac-err) 18%, var(--ac-bg))`
  *(hover state; one step above the base 8%)*
- ... full list

## Spacing
- `0.25rem` → `var(--sp-1)` (4px)
- `0.85rem` → `var(--size-xs)` *(reclassified as font-size)*
- ... full list

## Typography
- `font-family: monospace` → `var(--ac-font-mono)`
- `font-size: 0.85rem` → `var(--size-xs)` (12px; off-scale 0.85rem
  → bucketed to nearest scale step; ±0.6px drift accepted)
- ... full list
```

The artefact is *not* a long-lived ADR — it's a working document
for the duration of the migration. It can be deleted after 0033
merges (the conventions then live in the migrated CSS itself).

#### 1. `LifecycleClusterView.module.css` (25 hex / 43 px-rem)

**File**: `src/routes/lifecycle/LifecycleClusterView.module.css`
**Mapping decisions** (full per-line map populated during
implementation; locked-in conventions below):

- Colour:
  - `#111827` → `var(--ac-fg-strong)`
  - `#374151` → `var(--ac-fg)`
  - `#6b7280` → `var(--ac-fg-muted)`
  - `#9ca3af` → `var(--ac-fg-faint)`
  - `#ffffff` → `var(--ac-bg-card)`
  - `#e5e7eb` → `var(--ac-stroke-soft)`
  - `#d1d5db` → `var(--ac-stroke)`
  - `#1d4ed8` and `#2563eb` → both collapse to `var(--ac-accent)`
    (visible hue shift indigo vs blue; documented as PR drift
    note per AC6).
  - `#dbeafe` → `var(--ac-accent-tint)`
  - `#f3f4f6` → `var(--ac-bg-sunken)`
  - `#fef2f2` / `#fecaca` → `color-mix(in srgb, var(--ac-err) 8%, var(--ac-bg))`
    / `color-mix(in srgb, var(--ac-err) 30%, var(--ac-bg))`
    (lossy approximations; documented in PR).
  - `#991b1b` → `var(--ac-err)`
- Spacing:
  - `0.25rem` → `var(--sp-1)`, `0.5rem` → `var(--sp-2)`,
    `0.75rem` → `var(--sp-3)`, `1rem` → `var(--sp-4)`,
    `1.5rem` → `var(--sp-5)`, `2rem` → `var(--sp-6)`.
  - Off-scale: `0.4rem`, `0.55rem`, `0.7rem`, `0.85rem`, `1.4rem`
    — first reclassify as font-size (most are) and migrate to
    typography scale; remaining true spacing rounds to nearest
    `--sp-N` accepting ±2px drift, documented in PR.
- Radius: `4px`/`0.25rem` → `var(--radius-sm)`, `8px` →
  `var(--radius-md)`, `9999px` → `var(--radius-pill)`.
  `2px`/`3px` hairlines → `EXCEPTIONS`.
- Typography:
  - `font-family: monospace` (line 25) → `var(--ac-font-mono)`.
  - `.title` (line 24, `1.4rem`) → `var(--ac-font-display)` +
    `var(--size-lg)` (22px — close to 22px; documented if
    mapped to `--size-h4` 26px instead).
  - Body `font-size:` literals (`0.85rem`, `0.75rem`, `0.95rem`)
    → `var(--size-xs)`, `var(--size-xxs)`, `var(--size-sm)`.
- Box-shadow:
  - Line 50 (`0 0 0 1.5px #1d4ed8`) — coloured ring; replace
    `#1d4ed8` with `var(--ac-accent)`; keep `1.5px` as
    `EXCEPTIONS` hairline.
  - Line 55 (`0 0 0 1.5px #d1d5db`) — same shape; replace
    `#d1d5db` with `var(--ac-stroke)`; same `1.5px` exception.

#### 2. `LifecycleIndex.module.css` (24 hex / 37 px-rem)

**File**: `src/routes/lifecycle/LifecycleIndex.module.css`
**Mapping decisions**: same colour / spacing / radius / typography
conventions as above. The single elevation `box-shadow:` (line 79,
`0 1px 4px rgba(29, 78, 216, 0.12)`) → `var(--ac-shadow-soft)`,
losing the accent tint — documented in PR per AC6.

#### 3. `KanbanBoard.module.css` (15 hex / 23 px-rem)

**File**: `src/routes/kanban/KanbanBoard.module.css`
**Mapping decisions**: same conventions. The four error-tint
literals (`#fef2f2` line 31, `#fecaca` line 32 / 39 / 51 / 52,
`#fee2e2` line 45) all migrate to `color-mix(...)` against
`var(--ac-err)`, with hover state using a higher mix percentage.

### Wave 4b: Remaining 13 Modules

Each migrated against the locked-in mapping conventions from 4a.
Files in approximate decreasing literal-count order:

- `src/routes/library/LibraryTypeView.module.css` (14 / 22)
- `src/components/MarkdownRenderer/MarkdownRenderer.module.css` (9 / 23)
- `src/routes/library/LibraryDocView.module.css` (8 / 22 — the
  six `var(--color-*)` sites already gone in Phase 3)
- `src/routes/library/LibraryTemplatesView.module.css` (10 / 20)
- `src/components/RelatedArtifacts/RelatedArtifacts.module.css` (9 / 18)
- `src/routes/kanban/KanbanColumn.module.css` (7 / 14)
- `src/components/Sidebar/Sidebar.module.css` (9 / 12)
- `src/components/FrontmatterChips/FrontmatterChips.module.css` (5 / 13)
- `src/routes/kanban/WorkItemCard.module.css` (9 / 10)
- `src/routes/library/LibraryTemplatesIndex.module.css` (4 / 9)
- `src/components/PipelineDots/PipelineDots.module.css` (5 / 6)
- `src/components/SidebarFooter/SidebarFooter.module.css` (3 hex
  remaining outside Phase 3 sites / 6)
- `src/components/RootLayout/RootLayout.module.css` (0 / 2 —
  `system-ui, sans-serif` → `var(--ac-font-body)`)

For each file: confirm `migration.test.ts` failures, migrate,
re-run tests, green. The PR's "Irreducible-literal exceptions"
section is the union of all `EXCEPTIONS` entries.

### Special Conventions (codified in CSS / tests; reflected in PR)

- **`var(--token, #fallback)` retired**: the no-fallback
  `var(--ac-*)` form is the project convention from now on.
  Enforced by `migration.test.ts`'s `var(--ac-*, fallback) two-arg
  form is retired` describe block (Phase 2 §1).
- **Token-name validity**: every `var(--NAME)` reference must
  resolve to a key in the union of declared token exports.
  Enforced by `migration.test.ts`'s `var(--NAME) references
  resolve to declared tokens` describe.
- **Two-blue collapse**: `#2563eb` and `#1d4ed8` both → `--ac-accent`.
  Visible hue shift indigo vs blue; documented as an AC6 drift
  in the PR description.
- **Error-tint / warn-tint composition via `color-mix()`**:
  `color-mix(in srgb, var(--ac-err) Y%, var(--ac-bg))` (and
  analogously for `--ac-warn`) rather than introducing
  `--ac-err-tint` / `--ac-warn-tint` tokens.
  - **Colour space**: `in srgb` is chosen for: (a) deterministic
    parity with the legacy hex tints during migration, (b)
    universal browser support (color-mix overall is
    Baseline 2023; `oklch` is supported but produces visibly
    different mixes that drift further from the prototype's
    sRGB-authored palette).
  - **Percentage ladder** (locked in Wave 4a, applied in Wave 4b):
    `8%` for default tinted background, `18%` for hover state,
    `30%` for stroke/border tint. Higher percentages reserved
    for future use.
  - **Composition surface**: always `var(--ac-bg)` (theme-swapping
    via the dark override) — *not* `--ac-bg-card` or
    `--ac-bg-raised`, to keep recipes compositionally simple
    and reduce the surface of contrast assertions.
  - **Browser-support assumption**: visualiser ships only to
    local dev today; the maintained browser baseline (modern
    Chromium / Firefox / Safari) supports `color-mix()` natively.
    If the visualiser is ever delivered to a more conservative
    baseline, every error/warn-tinted surface fails open to the
    flat token (`var(--ac-err)` / `var(--ac-warn)`) — a noisy
    but safe failure mode. Re-evaluate the convention before any
    such delivery.
- **Sora / Inter / Fira Code assignment**: Sora on every `.title`,
  `<h1>`, `<h2>`, `.cardTitle`, `.columnHeading`; Inter as body
  default via `RootLayout`; Fira Code on the existing five
  monospace sites.

### Success Criteria

#### Automated Verification

- [ ] `mise run test:unit:frontend` passes — every per-file hex
      test in `migration.test.ts` is green; every per-file
      px/rem test is green; the aggregate `var(--*)` count is
      ≥ 300.
- [ ] `mise run build:frontend` passes (no broken CSS Modules
      typings, no TS errors).
- [ ] AC3 grep from `skills/visualisation/visualise/frontend/`:
      `rg '#[0-9a-fA-F]{3,8}\b' --type css --type ts src/ -g '!**/*.test.ts' -g '!**/*.test.tsx' -g '!src/styles/global.css' -g '!src/styles/tokens.ts'`
      returns zero matches.
- [ ] AC4 grep from `skills/visualisation/visualise/frontend/`:
      `rg '\b\d+(\.\d+)?(px|rem)\b' src/ -g '*.css' -g '!src/styles/global.css' -g '!src/styles/tokens.ts' -g '!**/*.test.ts' -g '!**/*.test.tsx'`
      returns only literals enumerated in the PR's
      "Irreducible-literal exceptions" list (i.e. matching the
      `EXCEPTIONS` typed constant).
- [ ] AC5 grep from `skills/visualisation/visualise/frontend/`:
      `rg 'var\(--' src/ -g '*.module.css' --count-matches | awk -F: '{s+=$2} END {print s}'`
      reports ≥ 300.

#### Manual Verification

- [ ] Each migrated module renders without visible regression
      under spot-check during migration; the Playwright
      visual-regression suite from Phase 1 is the authoritative
      gate (run **once at the end of Wave 4a, once at the end of
      Wave 4b, and once at PR-prep time** — not per per-module
      commit, to keep dev-loop cost bounded).
- [ ] Theme swap (toggling `[data-theme]` via DevTools, plus
      `prefers-color-scheme: dark` via DevTools "Emulate CSS")
      shows every migrated colour token reacting correctly under
      both selectors.

---

## Phase 5: Visual Parity Verification (Playwright)

### Overview

Final phase: re-run the Playwright `toHaveScreenshot()` baselines
captured at Phase 1 against the migrated state. Any pair that
diverges beyond `maxDiffPixelRatio: 0.05` (i.e. AC6's 5% of
viewport bound) either indicates a regression to fix or is
documented as a conscious drift, with the baseline updated and
a one-line PR justification.

### Changes Required

#### 1. Add the Playwright visual-regression suite

**File**:
`skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts`
(new — `@playwright/test` is already a devDep)

**Routes** (from work item AC6):

| ID                            | Path                                         |
| ----------------------------- | -------------------------------------------- |
| `kanban`                      | `/kanban`                                    |
| `library`                     | `/library`                                   |
| `library-type`                | `/library/plans`                             |
| `library-decisions`           | `/library/decisions`                         |
| `library-templates`           | `/library/templates`                         |
| `lifecycle-cluster`           | `/lifecycle/<cluster>` (same cluster as baseline) |
| `lifecycle-cluster-after-click` | as above, after the navigation variant     |

```ts
import { test, expect } from '@playwright/test'

const ROUTES = [
  ['kanban', '/kanban'],
  ['library', '/library'],
  ['library-type', '/library/plans'],
  ['library-decisions', '/library/decisions'],
  ['library-templates', '/library/templates'],
  ['lifecycle-cluster', '/lifecycle/<cluster>'],
] as const

const VIEWPORT = { width: 1440, height: 900 } // matches baseline inventory

for (const [id, path] of ROUTES) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${id} (${theme})`, async ({ page }) => {
      await page.setViewportSize(VIEWPORT)
      await page.goto(path)
      if (theme === 'dark') {
        await page.evaluate(() => {
          document.documentElement.dataset.theme = 'dark'
        })
      }
      await expect(page).toHaveScreenshot(`${id}-${theme}.png`, {
        maxDiffPixelRatio: 0.05,
        animations: 'disabled',
      })
    })
  }
}

// Cluster-after-click variant — capture the post-navigation state.
// The implementer identifies a stable interactive element on the cluster
// route (e.g. the first child card link) via either an existing class
// from `LifecycleClusterView.module.css` or a role-based locator. If
// neither resolves cleanly, add `data-testid="cluster-card"` to the
// relevant element in `LifecycleClusterView.tsx` — this is the **only**
// permitted TSX edit beyond `index.html` in 0033, and is explicitly
// flagged as such in the PR description.
test('lifecycle-cluster-after-click (light)', async ({ page }) => {
  await page.setViewportSize(VIEWPORT)
  await page.goto('/lifecycle/<cluster>')
  // Implementer to confirm the locator that resolves on the unmigrated
  // codebase before Phase 1 baselines are captured. Examples:
  //   - role-based: `page.getByRole('link').first()`
  //   - class-based: `page.locator('a.<existing-card-class>').first()`
  //   - testid (only if the above don't work): `page.locator('[data-testid="cluster-card"]')`
  await page.getByRole('link').first().click()
  await page.waitForLoadState('networkidle')
  await expect(page).toHaveScreenshot('lifecycle-cluster-after-click-light.png', {
    maxDiffPixelRatio: 0.05,
    animations: 'disabled',
  })
})
test('lifecycle-cluster-after-click (dark)', async ({ page }) => { /* same shape */ })

// Sanity test for the prefers-color-scheme path: the equality assertion
// in global.test.ts asserts the two dark blocks are byte-equivalent, but
// we exercise the OS-preference route at least once visually so a
// selector-engine bug in the @media block can't escape both checks.
test('library (prefers-color-scheme: dark, no data-theme attribute)', async ({ page }) => {
  await page.setViewportSize(VIEWPORT)
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/library')
  await expect(page).toHaveScreenshot('library-dark.png', {
    maxDiffPixelRatio: 0.05,
    animations: 'disabled',
  })
})
```

**Baselines**: captured during Phase 1 in **two passes** so the
typography swap and the literal-substitution work can be audited
separately:

- **Pass A** ("genuinely-before"): captured *before* any global.css
  edit, against the wholly-unmigrated codebase. Committed under
  `tests/visual-regression/__screenshots__/before-fonts/` and
  retained as a reference artefact only — not asserted by any test.
- **Pass B** ("post-fonts, pre-substitution"): captured after the
  `@font-face` block lands and the typography tokens are wired,
  but before any module-CSS literal substitution. Committed under
  `tests/visual-regression/__screenshots__/`. Phase 5 asserts
  against this set.

A reviewer comparing Pass A vs Pass B can see the typography
swap's pixel impact in isolation; Pass B vs the post-Phase-4
state isolates the literal-substitution impact. Phase 4 commits
may update specific Pass-B baselines if the diff is justified as
an AC6 drift, with the diff image and one-line justification
linked from the PR.

**Per-route overrides**: where AC6's ΔE/pixel tolerances permit
a known drift (e.g. the two-blue → indigo collapse), the
corresponding test uses a tighter `maxDiffPixels` count rather
than the default ratio:

```ts
await expect(page).toHaveScreenshot('kanban-light.png', {
  maxDiffPixels: 5000, // ~0.4% of 1440×900; tightens the gate
})
```

#### 2. Wire the suite into the test runner

**File**: `skills/visualisation/visualise/frontend/playwright.config.ts`
**Changes**: add a `visual-regression` project (or extend an
existing one) that points at `tests/visual-regression/` and uses
`webServer` to start the visualiser binary against the local
`meta/` directory. If the existing config already wires this,
extend the `testMatch` glob.

#### 3. PR description sections

**Location**: PR description body (drafted with the work item).

Sections:

- **Irreducible-literal exceptions**: mechanically generated from
  the final `EXCEPTIONS` constant via `scripts/scan-css-literals.ts`
  (filtered to `kind: 'irreducible'`).
- **Conscious drifts (AC6)**: cross-referenced from
  `meta/work/0033-token-mapping-conventions.md` plus any
  per-route Playwright `maxDiffPixels` overrides:
  - Two-blue collapse to `--ac-accent` (indigo).
  - Error/warn-tint composition via `color-mix()`.
  - `LifecycleIndex.module.css:79` shadow loses accent tint.
  - Typographic shifts where 14px → 16px or similar bucket
    rounding occurred.
- **Visual-regression diff regions**: any baseline updated by
  Phase 4 commits is listed with a one-line justification, and
  the diff image (auto-generated by Playwright into
  `test-results/`) is linked from the PR.
- **Self-hosted font verification**: `fonts.test.ts` covers AC2
  in CI; the PR description simply notes that no third-party
  font origins are referenced (also asserted by the
  `no third-party font origins` test).

### Success Criteria

#### Automated Verification

- [x] All Phase 4 success criteria still pass.
- [x] `mise run test` (full suite) passes — including the new
      Playwright `tests/visual-regression/tokens.spec.ts`.
- [x] All 14 visual-regression tests (7 routes × 2 themes) pass
      against their committed baselines, or any updated baseline
      is justified in the PR description.
- [x] No `fonts.googleapis.com` / `fonts.gstatic.com` requests
      appear in Playwright's network log (asserted alongside the
      visual-regression suite, or via the `fonts.test.ts`
      static check).

#### Manual Verification

- [ ] Reviewer skims the diff images Playwright produced for any
      baseline-updated route and agrees the drift is intentional.
- [ ] Theme toggle via DevTools and OS-level
      `prefers-color-scheme: dark` both render the dark palette
      correctly across all routes — confirms the dark token
      *values* and the `prefers-color-scheme` mirror are wired
      even though the toggle UI is 0034.

---

## Testing Strategy

### Unit tests

- **Token parity** (`global.test.ts`) — every CSS↔TS pair for
  each of the seven in-scope token categories (light colour,
  dark colour, typography, spacing, radius, light shadow, dark
  shadow), plus the retained legacy `COLOR_TOKENS` describe
  until Phase 3 deletes it. Includes a small assertion that the
  `[data-theme="dark"]` declarations and the
  `@media (prefers-color-scheme: dark)` declarations are
  identical.
- **WCAG contrast** (`contrast.test.ts`) — eight assertions
  spanning light + dark and including stroke-on-surface and
  warn-on-warn-tinted-bg via the new `composeOverSurface` helper.
- **Migration enforcement** (`migration.test.ts`) — per-file
  hex absence, per-file px/rem/em absence (with the typed
  per-occurrence `EXCEPTIONS` list), no-fallback `var()` form,
  declared-token-name validity, aggregate `var(--*)` count,
  EXCEPTIONS staleness check.
- **Self-hosted font wiring** (`fonts.test.ts`) — woff2 file
  existence, `@font-face` declarations per family,
  `font-display: swap` on every block, `<link rel="preload">`
  for at least one critical-path font, no third-party origin
  references in `index.html` or `global.css`.
- **Focus rings** (`global.test.ts`) — three regex assertions
  retained verbatim.

### Visual-regression tests

- **Playwright `toHaveScreenshot()`** (`tests/visual-regression/tokens.spec.ts`)
  — 14 tests (7 routes × 2 themes) plus the cluster-after-click
  variant in both themes. Baselines are captured in Phase 1
  (against the unmigrated codebase) and committed under
  `tests/visual-regression/__screenshots__/`. Phase 4 commits
  may update specific baselines if the diff is justified as an
  AC6 drift; the diff images Playwright produces are linked
  from the PR description.

### Integration tests

None new. Existing E2E (`mise run test:e2e:visualiser`) continues
to run as part of the broader suite. Layout-rounding regressions
are caught primarily by the visual-regression suite above; the
plan does not rely on the E2E suite for that purpose.

### Manual testing steps

1. `mise run build:server:dev` and launch against the local
   `meta/` directory.
2. For each of the seven AC6 routes:
   1. Open the route in light theme.
   2. Toggle `data-theme="dark"` via DevTools.
   3. Toggle the OS-level `prefers-color-scheme: dark` (or
      DevTools' "Emulate CSS prefers-color-scheme") and confirm
      the dark palette applies without explicit `data-theme`.
3. In DevTools Network panel, filter by `font` and confirm 200s
   for `/fonts/...` (same-origin) — and **no** outbound requests
   to `fonts.googleapis.com` or `fonts.gstatic.com`.
4. In DevTools Elements, run
   `getComputedStyle(document.documentElement).getPropertyValue('--ac-bg')`
   and confirm the active value matches the active theme.

## Performance Considerations

Self-hosting eight woff2 files adds ~120–200 KB of fonts to the
`public/` directory (latin subset; estimated from typical Inter
~30 KB/weight, Sora ~40 KB/weight, Fira Code ~50 KB/weight). All
files are served same-origin by the existing Vite dev server / the
Rust visualiser binary in production; there is no DNS+TLS handshake
to a third-party origin and no third-party runtime coupling. The
two above-the-fold weights (Inter 400, Sora 700) are preloaded via
`<link rel="preload">` to keep the FOUT brief; the remaining six
weights load on demand with `font-display: swap`.

CSS file size grows by ~4 KB (token blocks plus dark override block
plus `@font-face` declarations); negligible for a SPA bundle.

The migration test harness eager-globs ~16 small `.module.css` files
plus globals; per-file regex passes are sub-millisecond, so vitest
startup cost is bounded. If the module count grows past ~100 in
future stories, revisit the per-file `it()` granularity in favour
of a single aggregated assertion.

## Migration Notes

The migration is intentionally visually neutral *where mappings
are clean*. Phase 5's Playwright `toHaveScreenshot()` diffs are the
audit trail. Rollback is a single revert of the merged commit; no
data-migration concerns.

The `var(--color-*, #fallback)` two-arg consumption pattern is
retired by Phase 3 and not re-introduced. Phase 1's `tokens.ts`
keeps `COLOR_TOKENS` exported (annotated `@deprecated`) so the
legacy parity describe in `global.test.ts` continues to assert
CSS↔TS parity for the legacy block until Phase 3 deletes both
halves in a single commit; this temporary retention is the only
cross-phase coupling.

`color-mix()` is a new pattern in this codebase; the convention
(Phase 4 "Special Conventions") pins the colour space (`in srgb`),
the percentage ladder (8% / 18% / 30%), the composition surface
(`var(--ac-bg)`), and the browser-support assumption.

## References

- Work item: `meta/work/0033-design-token-system.md`
- Research: `meta/research/2026-05-06-0033-design-token-system.md`
- Token source: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/inventory.md`
  (sections "Active semantic layer — `--ac-*` (light)", "Active
  semantic layer — `--ac-*` (dark, overrides under `[data-theme="dark"]`)" —
  inventory lines 154–155 specify per-theme `--ac-shadow-*` overrides;
  "Typography", "Spacing", "Radius", "Shadow")
- Baseline screenshots (informational; Playwright captures its own baselines
  in Phase 1): `meta/design-inventories/2026-05-06-135214-current-app/screenshots/`
- Target screenshots: `meta/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/main-light.png`,
  `main-dark.png`
- Existing parity-test idiom: `skills/visualisation/visualise/frontend/src/styles/contrast.test.ts:6-18`
- Vitest + Vite setup: `skills/visualisation/visualise/frontend/vite.config.ts`,
  `skills/visualisation/visualise/frontend/package.json:13-15`
- Playwright setup: `skills/visualisation/visualise/frontend/playwright.config.ts`
- Plan-review artefact: `meta/reviews/plans/2026-05-06-0033-design-token-system-review-1.md`
- Successor work item (toggle UI / persistence):
  `meta/work/0034-theme-and-font-mode-toggles.md`
