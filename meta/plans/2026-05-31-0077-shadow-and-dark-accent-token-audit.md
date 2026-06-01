---
type: plan
id: "2026-05-31-0077-shadow-and-dark-accent-token-audit"
title: "Shadow and Dark-Accent Token Audit Implementation Plan"
date: "2026-05-31T22:42:34+00:00"
author: "Toby Clemson"
producer: create-plan
status: approved
work_item_id: "0077"
parent: ""
reviewer: "Toby Clemson"
tags: [design, frontend, tokens, audit, visualiser]
revision: "a1ba3a3789d27bd626f9f7f8c6f92b5517d81c0a"
repository: "build-system"
last_updated: "2026-06-01T01:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Shadow and Dark-Accent Token Audit Implementation Plan

## Overview

Confirm that the visualiser frontend's `--ac-shadow-soft`,
`--ac-shadow-lift`, dark `--ac-accent`, and dark `--ac-accent-2`
declarations match the claude-design-prototype, lock the parity in CI
with a single new Playwright spec, and compile the audit evidence into
the PR description. The research established that all four tokens
already match exactly; no value migration is needed.

## Current State Analysis

The four tokens in `src/styles/global.css` are byte-equivalent (modulo
whitespace and hex-casing) to the prototype's `src/app.css`:

- `--ac-shadow-soft` light (`global.css:201`) and dark
  (`global.css:364`, mirrored at `global.css:422` under
  `@media (prefers-color-scheme: dark)`).
- `--ac-shadow-lift` light (`global.css:202`) and dark
  (`global.css:365`, mirrored at `global.css:423`).
- `--ac-accent` dark (`global.css:329`, mirrored at `global.css:391`).
- `--ac-accent-2` dark (`global.css:330`, mirrored at `global.css:392`).

The same values are mirrored into the JS-consumable token table at
`src/styles/tokens.ts:82–83` (`DARK_COLOR_TOKENS.ac-accent` and
`ac-accent-2`) and `tokens.ts:177–189` (`LIGHT_SHADOW_TOKENS` and
`DARK_SHADOW_TOKENS`). The CSS↔TS parity test
`src/styles/global.test.ts:202–203` already asserts dark-block
coverage extends past `--ac-shadow-lift`, and `tokens.spec.ts`
visual-regression baselines (kanban, library, lifecycle-cluster,
etc.) already exercise the highest-traffic consumer surfaces in both
themes — those baselines passing IS evidence of rendering parity.

What is missing is an explicit assertion that
`getComputedStyle(document.documentElement)` resolves these four
tokens to the prototype values under `data-theme="dark"`, which is
what AC#3 of the work item asks for. The existing
`tests/visual-regression/lib/expected-colours.ts` exports a
`setTheme(page, theme)` helper that flips
`document.documentElement.dataset.theme` and awaits the attribute
landing — the canonical primitive for this assertion.

Consumer enumeration via
`rg 'var\(--ac-shadow-soft\)|var\(--ac-shadow-lift\)|var\(--ac-accent\)\b|var\(--ac-accent-2\)' src/`
yields 26 unique consumer files / ~56 sites (one file appears in both
the `--ac-accent` and `--ac-accent-2` consumer sets, so the per-token
sum of 27 collapses to 26 unique files), which exceeds AC#4's
six-surface threshold and triggers its follow-up clause.

## Desired End State

A new Playwright spec at
`skills/visualisation/visualise/frontend/tests/visual-regression/root-resolved-tokens.spec.ts`
asserts in CI that all four token values, computed off
`document.documentElement`, equal the values declared in `tokens.ts`
(`LIGHT_SHADOW_TOKENS`, `DARK_SHADOW_TOKENS`, `LIGHT_COLOR_TOKENS`,
`DARK_COLOR_TOKENS`) under all three cascade paths the visualiser
supports: `[data-theme="light"]`, `[data-theme="dark"]`, and the
`@media (prefers-color-scheme: dark)` mirror with `data-theme`
unset. The MIRROR-B test waits for React's `useTheme` mount effect
to land its attribute write, then removes the attribute and asserts
absence as an invariant guard, so the test cannot silently
degenerate into a MIRROR-A-equivalence check if a future regression
re-introduces the race. Accent equality is asserted via `hexToRgb`
so hex/rgb() serialisation differences don't surface as false
failures; shadow equality strips internal whitespace and lowercases
both sides for the same reason. The PR description carries:

- The verbatim current and prototype declarations for all four
  tokens, side-by-side, for both themes (AC#1).
- An explicit "no divergence — values match exactly" statement
  (AC#2's vacuous-truth case; no justification required).
- The dark-theme `--ac-accent` and `--ac-accent-2` values quoted
  from `tokens.ts` (`DARK_COLOR_TOKENS`) normalised to `rgb()`,
  confirmed to equal `rgb(138, 144, 232)` and `rgb(232, 106, 107)`
  (AC#3). The new spec asserts `hexToRgb(getComputedStyle(...))
  === hexToRgb(DARK_COLOR_TOKENS[...])` in CI, so quoting
  `tokens.ts` IS quoting the computed value modulo serialisation.
- The consumer enumeration (26 unique files / 56 sites) plus an explicit
  application of AC#4's six-surface follow-up clause using its
  "spirit reading": because no value drifts, no pixel diff can exceed
  0.1%, the existing `tokens.spec.ts` baselines passing is the
  evidence, and no follow-up baseline-refresh work item is raised.

Verification:

- `cd skills/visualisation/visualise/frontend && npx playwright test tests/visual-regression/root-resolved-tokens.spec.ts` exits 0.
- `cd skills/visualisation/visualise/frontend && npm run test` exits 0 (unit + Playwright suites both green, including the unchanged `tokens.spec.ts` baselines).
- `cd skills/visualisation/visualise/frontend && npm run typecheck` exits 0.
- `cd skills/visualisation/visualise/frontend && npm run lint` exits 0.

### Key Discoveries:

- All four tokens already match the prototype — see [research summary](../research/codebase/2026-05-31-0077-shadow-and-dark-accent-token-audit.md#current-declarations-vs-prototype-declarations) for the side-by-side. The audit closes as confirmation, not migration.
- `setTheme(page, 'dark')` at `tests/visual-regression/lib/expected-colours.ts:73` is the canonical primitive for theme-swap-then-read assertions; it awaits the attribute landing via `waitForFunction` so an immediate `getComputedStyle` read in the same `page.evaluate` block is sound (per the `*-resolved-colours.spec.ts` pattern).
- `frontend/README.md:82`'s warning about `waitForFunction` over `getComputedStyle` applies to theme-swap equality checks where the read happens before the attribute lands; `setTheme` resolves only after the attribute has landed, so the warning does not apply here.
- The four tokens already live as canonical values in `tokens.ts` — the new spec should consume `LIGHT_SHADOW_TOKENS`, `DARK_SHADOW_TOKENS`, `LIGHT_COLOR_TOKENS`, and `DARK_COLOR_TOKENS` rather than re-declaring string literals, to avoid creating a fourth source of truth.
- AC#4's six-surface follow-up clause exists to bound work, not to manufacture work when nothing changed — the "spirit reading" recommended in the [research's Open Questions §1](../research/codebase/2026-05-31-0077-shadow-and-dark-accent-token-audit.md#open-questions) is the right call.

## What We're NOT Doing

- **No token-value changes.** The four tokens already match the prototype byte-for-byte (modulo hex casing); no edit to `global.css` or `tokens.ts` is required or permitted by this plan.
- **No new visual-regression baselines.** AC#4's follow-up clause is invoked under the spirit reading; existing `tokens.spec.ts` baselines remain the evidence of rendering parity. No `__screenshots__/` files are added, refreshed, or deleted.
- **No follow-up baseline-refresh work item.** Per the spirit reading of AC#4 (research recommendation, accepted), no value drift means no pixel diff is possible; raising a no-op follow-up work item would manufacture process without producing evidence.
- **No promotion of dark accents to the brand layer.** `#8a90e8` and `#e86a6b` have no `--atomic-*` brand-token equivalent (per [`2026-05-23-0073-atomic-brand-layer-palette.md`](../research/codebase/2026-05-23-0073-atomic-brand-layer-palette.md)); promoting them is out of scope.
- **No edits to `global.test.ts` or `prototype-tokens.fixture.test.ts`.** Existing CSS↔TS parity coverage is sufficient.
- **No hex-casing normalisation.** The prototype uses uppercase (`#8A90E8`), the current app uses lowercase (`#8a90e8`) per ADR-0035; this is treated as parity (resolved-`rgb()` equivalence), not drift.

## Implementation Approach

A two-phase plan:

1. **Phase 1** adds a single Playwright spec that consumes the
   `tokens.ts` tables to assert computed-style equivalence under both
   themes. This closes AC#3 with permanent CI coverage and emits the
   computed values in test output so they can be harvested verbatim
   for the PR description.
2. **Phase 2** compiles the PR description content using the work
   item's four acceptance criteria as the structural skeleton. No
   code changes in Phase 2.

The phases are sequential because Phase 2 depends on Phase 1's spec
output for the AC#3 computed-value record.

## Phase 1: Add Computed-Style Token-Value Spec

### Overview

Add a single new Playwright spec that asserts, for both themes, that
`getComputedStyle(document.documentElement)` resolves `--ac-shadow-soft`,
`--ac-shadow-lift`, `--ac-accent`, and `--ac-accent-2` to the values
declared in `tokens.ts`. Reuse the existing `setTheme` helper from
`tests/visual-regression/lib/expected-colours.ts`.

### Changes Required:

#### 1. New Playwright spec consuming tokens.ts as the source of truth

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/root-resolved-tokens.spec.ts`
**Changes**: Create the file. Define `SHADOW_KEYS` / `COLOR_KEYS`
tuples so the four token names live in one place. For each of the
two `[data-theme="…"]` paths, navigate to `/library` (a fast-loading
route already used by `tokens.spec.ts`; the route content is
incidental — only `:root`-level cascade matters), call
`setTheme(page, theme)`, then read all four computed values in a
single `page.evaluate` and assert. Add a third test that clears
`data-theme` and uses `page.emulateMedia({ colorScheme: 'dark' })`
to exercise the `@media (prefers-color-scheme: dark)` MIRROR-B
cascade. Use `hexToRgb` for accents (closes AC#3's "normalised to
`rgb()`" wording in CI) and whitespace-strip + lowercase for the
shadow strings (mirrors the `canonicaliseBrand` pattern at
`src/styles/testing/canonicaliseBrand.ts:38`).

```ts
import { test, expect, type Page } from '@playwright/test'
import {
  LIGHT_COLOR_TOKENS,
  DARK_COLOR_TOKENS,
  LIGHT_SHADOW_TOKENS,
  DARK_SHADOW_TOKENS,
} from '../../src/styles/tokens'
import { setTheme, hexToRgb, type Theme } from './lib/expected-colours'

const SHADOW_KEYS = ['ac-shadow-soft', 'ac-shadow-lift'] as const
const COLOR_KEYS = ['ac-accent', 'ac-accent-2'] as const
type ShadowKey = (typeof SHADOW_KEYS)[number]
type ColorKey = (typeof COLOR_KEYS)[number]
type TokenSnapshot = {
  shadows: Record<ShadowKey, string>
  colors: Record<ColorKey, string>
}

const EXPECTED_SHADOWS: Record<Theme, Record<ShadowKey, string>> = {
  light: {
    'ac-shadow-soft': LIGHT_SHADOW_TOKENS['ac-shadow-soft'],
    'ac-shadow-lift': LIGHT_SHADOW_TOKENS['ac-shadow-lift'],
  },
  dark: {
    'ac-shadow-soft': DARK_SHADOW_TOKENS['ac-shadow-soft'],
    'ac-shadow-lift': DARK_SHADOW_TOKENS['ac-shadow-lift'],
  },
}

const EXPECTED_COLORS: Record<Theme, Record<ColorKey, string>> = {
  light: {
    'ac-accent':   LIGHT_COLOR_TOKENS['ac-accent'],
    'ac-accent-2': LIGHT_COLOR_TOKENS['ac-accent-2'],
  },
  dark: {
    'ac-accent':   DARK_COLOR_TOKENS['ac-accent'],
    'ac-accent-2': DARK_COLOR_TOKENS['ac-accent-2'],
  },
}

// Collapse internal whitespace to a single space rather than removing
// it entirely. This tolerates Chromium re-spacing inside `rgba(...)`
// argument lists across versions while preserving the mandatory
// separator between shadow components, so e.g. `0 1px 2px` cannot
// collapse to `01px2px` and accidentally compare equal to a broken
// declaration.
const normaliseShadow = (s: string) =>
  s.toLowerCase().replace(/\s+/g, ' ').trim()

async function readRootTokens(page: Page): Promise<TokenSnapshot> {
  return page.evaluate(
    ({ shadowKeys, colorKeys }) => {
      const s = getComputedStyle(document.documentElement)
      const read = (k: string) => s.getPropertyValue('--' + k).trim()
      return {
        shadows: Object.fromEntries(shadowKeys.map((k) => [k, read(k)])),
        colors:  Object.fromEntries(colorKeys.map((k) => [k, read(k)])),
      }
    },
    { shadowKeys: [...SHADOW_KEYS], colorKeys: [...COLOR_KEYS] },
  ) as Promise<TokenSnapshot>
}

function assertParity(theme: Theme, actual: TokenSnapshot) {
  for (const k of SHADOW_KEYS) {
    expect(normaliseShadow(actual.shadows[k])).toEqual(
      normaliseShadow(EXPECTED_SHADOWS[theme][k]),
    )
  }
  // Both sides are hex literals today; hexToRgb normalises to
  // `rgb(r, g, b)` so the assertion satisfies AC#3's "normalised to
  // `rgb()` notation" wording literally. Constraint: this assertion
  // assumes both `tokens.ts` and the computed CSS value are 6-char
  // hex. If a future edit declares any of the four colours as
  // `rgb(...)` or `color(srgb …)`, route the actual side through
  // `parseRgb` (also exported from `lib/expected-colours.ts`) and
  // compare tuples instead.
  for (const k of COLOR_KEYS) {
    expect(hexToRgb(actual.colors[k])).toEqual(
      hexToRgb(EXPECTED_COLORS[theme][k]),
    )
  }
}

test.describe('root resolved tokens', () => {
  test.beforeEach(async ({ page }) => {
    // Prevent cross-test localStorage residue from pushing
    // use-theme.ts:readInitial off prefers-color-scheme. Each test
    // navigates fresh, so removing the key on init is sufficient.
    await page.addInitScript(() => {
      try {
        localStorage.removeItem('ac-theme')
      } catch {
        /* private-mode SecurityError */
      }
    })
  })

  for (const theme of ['light', 'dark'] as const) {
    test(`values resolve under [data-theme="${theme}"]`, async ({ page }) => {
      await page.goto('/library')
      await setTheme(page, theme)
      assertParity(theme, await readRootTokens(page))
    })
  }

  test('values resolve under prefers-color-scheme: dark (no data-theme)', async ({ page }) => {
    await page.emulateMedia({ colorScheme: 'dark' })
    await page.goto('/library')
    // useTheme's mount useEffect (use-theme.ts:30-32) unconditionally
    // writes data-theme on mount. Under emulated dark + empty
    // localStorage, readInitial resolves to 'dark', so the effect sets
    // data-theme="dark". Wait for that write to land BEFORE removing
    // the attribute — the effect's deps are `[theme]`, so once it has
    // fired it does not re-fire without a state change, and our manual
    // removal sticks.
    await page.waitForFunction(
      () => document.documentElement.getAttribute('data-theme') === 'dark',
    )
    await page.evaluate(() => {
      document.documentElement.removeAttribute('data-theme')
    })
    // Invariant guard: confirm MIRROR-B (`:root:not([data-theme="light"])`
    // under `@media (prefers-color-scheme: dark)`) is the cascade source,
    // not MIRROR-A. If React's effect re-applied the attribute, this
    // assertion fails loudly rather than silently degenerating into a
    // MIRROR-A-equivalence check.
    expect(
      await page.evaluate(() =>
        document.documentElement.hasAttribute('data-theme'),
      ),
    ).toBe(false)
    assertParity('dark', await readRootTokens(page))
  })
})
```

##### Plan rationale (do not transcribe into source comments)

- The `Theme` import from `expected-colours.ts` is the canonical
  `'light' | 'dark'` discriminator used across the resolved-colour
  specs; do not redeclare locally.
- `setTheme` is used rather than the lighter `applyTheme` from
  `helpers.ts` because its `waitForFunction` post-condition is
  load-bearing here: it guarantees the `data-theme` attribute has
  landed before the immediate `page.evaluate` read, sidestepping
  the `getComputedStyle` lag warned about in `frontend/README.md:82`.
- `/library` is chosen over `/kanban` because it has fewer dynamic
  elements (no SSE-driven board refresh); the spec only reads
  `:root`-level computed style so the route content is incidental.
- The spec consumes `tokens.ts` values rather than re-declaring
  prototype string literals. `prototype-tokens.fixture.test.ts`
  validates `tokens.ts` against the prototype, and `global.test.ts`
  validates CSS↔TS string parity; this new spec closes the residual
  gap by asserting the **post-cascade** values resolved at `:root`
  match `tokens.ts` under all three cascade paths
  (`[data-theme="light"]`, `[data-theme="dark"]`, and `@media
  (prefers-color-scheme: dark)` with `data-theme` unset).
- Accents are compared via `hexToRgb` so the assertion satisfies
  AC#3's "normalised to `rgb()` notation" phrasing literally and in
  CI. Constraint: both sides must be 6-char hex literals; `hexToRgb`
  does not handle `rgb(...)` or `color(srgb …)` inputs and would
  produce `rgb(NaN, NaN, NaN)` on both sides if either drifted to
  those forms. If a future edit changes a declaration away from hex,
  swap the actual side for `parseRgb` (also exported from
  `lib/expected-colours.ts`) and compare tuples — see the inline
  comment on `assertParity`.
- Shadow comparison normalises whitespace + lowercases both sides
  because `getPropertyValue` may re-serialise internal whitespace
  inside `rgba(...)` argument lists across Chromium versions. The
  helper collapses runs of whitespace to a single space rather than
  stripping entirely, so canonical separators between shadow
  components are preserved — a dropped separator (e.g.
  `0 1px 2px` → `01px2px`) is still detected as drift.
- Per-spec `beforeEach` clears the `ac-theme` localStorage key via
  `addInitScript` so cross-test residue cannot push
  `use-theme.ts:readInitial` off `prefersDark()`. This is defensive
  against future test ordering changes; today's tests would not
  pollute localStorage, but a future theme-toggle interaction test
  could.
- All three tests are wrapped in `test.describe('root resolved
  tokens', …)` to match the grouping convention used by
  `chip-resolved-colours.spec.ts` and the rest of the resolved-*
  family. Reporter output shows e.g.
  `root resolved tokens › values resolve under [data-theme="dark"]`.
- No new baseline screenshot is added — this spec asserts resolved
  values only, not visual output.
- The MIRROR-B test (third case) is hardened against React's
  `useTheme` mount effect. `RootLayout` calls `useTheme()` whose
  `useEffect` at `src/api/use-theme.ts:30–32` unconditionally writes
  `data-theme` on mount based on `readInitial`. Under emulated dark +
  empty `ac-theme` localStorage, `readInitial` falls through to
  `prefersDark()` and returns `'dark'`, so the effect writes
  `data-theme="dark"`. To genuinely exercise MIRROR-B
  (`:root:not([data-theme="light"])` under `@media
  (prefers-color-scheme: dark)`), the test (a) clears
  `ac-theme` via `addInitScript` so prior-test residue cannot push
  `readInitial` to `'light'`, (b) waits for the useEffect's
  attribute write to land via `waitForFunction`, (c) then removes
  the attribute, and (d) asserts the absence as an invariant guard
  so a future regression that re-introduces the race fails loudly
  rather than silently exercising MIRROR-A.

### Success Criteria:

#### Automated Verification:

- [x] Spec file exists at `skills/visualisation/visualise/frontend/tests/visual-regression/root-resolved-tokens.spec.ts`.
- [x] New spec passes for all three cascade paths: `cd skills/visualisation/visualise/frontend && npx playwright test tests/visual-regression/root-resolved-tokens.spec.ts` reports three passing tests (`[data-theme="light"]`, `[data-theme="dark"]`, `prefers-color-scheme: dark`). The MIRROR-B test's `hasAttribute('data-theme')` invariant guard asserts absence before reading tokens.
- [x] Full Playwright suite still passes (no regressions in existing baselines): `cd skills/visualisation/visualise/frontend && npx playwright test`.
- [x] Unit tests pass: `cd skills/visualisation/visualise/frontend && npm test`.
- [x] Type checking passes: `cd skills/visualisation/visualise/frontend && npm run typecheck`.
- [ ] Linting passes: `cd skills/visualisation/visualise/frontend && npm run lint`. **N/A — no `lint` script defined in `frontend/package.json`.**

#### Manual Verification:

- [x] On the first green run, confirm the spec exercises the three cascade paths — the Playwright reporter should list `root resolved tokens › values resolve under [data-theme="light"]`, `… [data-theme="dark"]`, and `… prefers-color-scheme: dark (no data-theme)`. No source mutation or `console.log` insertion is required: the spec's `hexToRgb` and `normaliseShadow` assertions are themselves the recorded-equality evidence for AC#3, and the PR description quotes `tokens.ts` literals (which the spec proves equal to the computed values).

---

## Phase 2: Compile PR Description Evidence

### Overview

Author the PR description content covering all four acceptance
criteria. This is documentation-only — no code changes. The structure
below maps one-to-one onto the work item's acceptance criteria.

### Changes Required:

#### 1. PR description: shadow declaration comparison (AC#1)

**File**: PR description (composed at PR-creation time).
**Changes**: Render the four shadow declarations as a single
4-row Markdown table with columns **Token / Theme / Current /
Prototype**. Each Current / Prototype cell is a fenced inline code
span quoting the declaration verbatim with hex casing preserved per
source (lowercase current, uppercase prototype). Sources:

| Source side | Light shadows | Dark shadows |
| --- | --- | --- |
| Current | `global.css:201–202` | `global.css:364–365` |
| Prototype | `/Users/tobyclemson/Downloads/Accelerator/src/app.css:36–37` | `/Users/tobyclemson/Downloads/Accelerator/src/app.css:68–69` |

Layout template (paste into the PR description verbatim — cells
already contain the full multi-layer declarations, with hex casing
matching each source):

```markdown
| Token              | Theme | Current                                                                          | Prototype                                                                        |
| ------------------ | ----- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `--ac-shadow-soft` | light | `0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06)`                  | `0 1px 2px rgba(10,17,27,0.04), 0 8px 28px rgba(10,17,27,0.06)`                  |
| `--ac-shadow-lift` | light | `0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10)`                 | `0 2px 4px rgba(10,17,27,0.06), 0 20px 60px rgba(10,17,27,0.10)`                 |
| `--ac-shadow-soft` | dark  | `0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4)`                          | `0 1px 2px rgba(0,0,0,0.3), 0 8px 28px rgba(0,0,0,0.4)`                          |
| `--ac-shadow-lift` | dark  | `0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55)`                        | `0 2px 4px rgba(0,0,0,0.4), 0 20px 60px rgba(0,0,0,0.55)`                        |
```

Before opening the PR, spot-check each Current cell against the
file:line reference (`global.css:201–202` / `:364–365`) and each
Prototype cell against `app.css:36–37` / `:68–69`, in case an
intervening edit has shifted any value.

#### 2. PR description: divergence justification (AC#2)

**Changes**: Write a short paragraph (2–3 sentences) that (a) states
no divergence exists across all four shadow declarations, (b) names
the comparison method — verbatim source quotation per AC#1's table
plus the new `root-resolved-tokens.spec.ts` asserting computed
equality under all three cascade paths, and (c) cross-references
AC#1's table and AC#3's computed-equality record. This satisfies
AC#2's "shadow values either match the prototype" branch and gives
a reviewer auditing only AC#2 a self-contained pointer chain into
the supporting evidence. Suggested template:

> Shadow values match the prototype byte-for-byte (modulo whitespace)
> across both themes — see the side-by-side table under AC#1 above
> for the verbatim declarations. The new
> `root-resolved-tokens.spec.ts` (see AC#3 below) asserts computed
> equality at `:root` under all three cascade paths in CI, so the
> parity holds at both declaration and resolution time. AC#2's
> divergence-justification clause is not invoked.

#### 3. PR description: dark-accent computed-value record (AC#3)

**Changes**: Quote the dark-theme values for `--ac-accent` and
`--ac-accent-2` from `tokens.ts` (`DARK_COLOR_TOKENS['ac-accent'] =
'#8a90e8'`, `DARK_COLOR_TOKENS['ac-accent-2'] = '#e86a6b'`),
normalised via `hexToRgb` to `rgb(138, 144, 232)` and `rgb(232,
106, 107)` respectively, and confirm equality with the prototype's
`rgb(138, 144, 232)` / `rgb(232, 106, 107)`. The new
`root-resolved-tokens.spec.ts` asserts in CI that
`hexToRgb(getComputedStyle(document.documentElement).getPropertyValue('--ac-accent').trim())`
equals `hexToRgb(DARK_COLOR_TOKENS['ac-accent'])` (and the same for
`--ac-accent-2`) under all three cascade paths, so quoting
`tokens.ts` is equivalent to quoting the computed value modulo
serialisation — no manual capture step required. Because the
computed values equal the prototype values, no migration is
performed and AC#3's conditional ("If, when normalised… they do
not equal…") never fires.

For `--ac-accent-2` specifically, note that AC#3's no-consumer
fallback does *not* apply because the AC#4 enumeration in §4 below
includes three `--ac-accent-2` consumer sites (`Brand.tsx:15`,
`Brand.tsx:25`, `FrontmatterTable.module.css:39`). The spec's
`:root`-level assertion plus the existing visual-regression
baselines that paint these consumers satisfy AC#3 for both accents.

#### 4. PR description: consumer enumeration + AC#4 spirit-reading justification (AC#4)

**Changes**: Record the enumeration as a table (token → file count →
site count). Reproducible command:

```bash
cd skills/visualisation/visualise/frontend && \
rg --no-heading -n 'var\(--ac-shadow-soft\)|var\(--ac-shadow-lift\)|var\(--ac-accent\)\b|var\(--ac-accent-2\)' src/
```

Expected tally (per research §Consumer Enumeration, re-verified during
Phase 2 from the live `rg` output as canonical):
`--ac-shadow-soft` → 1 file / 1 site;
`--ac-shadow-lift` → 2 files / 2 sites;
`--ac-accent` → 21 files / 50 sites;
`--ac-accent-2` → 3 files / 3 sites;
per-token sum 27 files / 56 sites, **26 unique files / 56 sites**
after collapsing the one file (`Brand.tsx`) that consumes both
`--ac-accent` and `--ac-accent-2`. Quote whatever the live `rg`
output produces at PR-open time as canonical; the figures above are
illustrative and may drift by ±1–2 with any unrelated edit. If the
live tally differs from 26/56, update every numeric reference
elsewhere in the PR description (the Overview's "26 unique files"
phrasing, the AC#2 paragraph if it cites a count, and the AC#4
spirit-reading block-quote below which names "26 unique files") so
the PR description is internally consistent.

Then explicitly invoke AC#4's six-surface follow-up clause using the
spirit reading:

> The enumerated consumer set (26 unique files) exceeds AC#4's
> six-surface threshold, triggering its follow-up clause. The strict
> reading would raise a follow-up baseline-refresh work item that
> enumerates the surfaces and captures before/after Playwright
> snapshots; we reject that reading here because Phase 1's
> `root-resolved-tokens.spec.ts` confirms no token value changes
> under any of the three cascade paths, so no pixel diff can exceed
> 0.1% on any surface and the follow-up would perform no migration
> and produce no diff. Manufacturing process without producing
> evidence is net-negative. AC#4's before/after baseline contract
> is degenerate when no value migrates. The existing
> `tokens.spec.ts` visual-regression baselines (kanban, library,
> lifecycle-cluster, lifecycle-cluster-after-click in both themes)
> passing IS the evidence of rendering parity on the highest-traffic
> consumers, and no follow-up baseline-refresh work item is raised.
> This applies the "spirit reading" recommended in the [companion
> research's Open Questions §1](../research/codebase/2026-05-31-0077-shadow-and-dark-accent-token-audit.md#open-questions).

#### 5. PR description: scope note on the new spec

**Changes**: One sentence stating that the only code change in this
PR is the addition of
`tests/visual-regression/root-resolved-tokens.spec.ts` — permanent
CI assertion of AC#3's computed-value equality across all three
cascade paths. Refer back to the plan's "What We're NOT Doing"
section for the negative-scope enumeration rather than restating it.

### Success Criteria:

#### Automated Verification:

- [x] `rg --no-heading -n 'var\(--ac-shadow-soft\)|var\(--ac-shadow-lift\)|var\(--ac-accent\)\b|var\(--ac-accent-2\)' src/` (run from `skills/visualisation/visualise/frontend/`) produces output that matches the file-and-site counts quoted in the PR description; any drift between the quoted enumeration and the actual command output is corrected before PR open.
  - **NOTE on the regex.** Rust's `\b` requires `\w`-to-`\W` (or vice versa) transition; `\)\b` matches zero sites because every `--ac-accent)` is followed by `;`, space, `,`, etc. (non-word). The `\b` is therefore inert — the plan's intent to "exclude `--ac-accent-2` matches" is already satisfied by `var\(--ac-accent\)` alone because the literal `)` in the regex requires a `)` immediately after `--ac-accent`, which cannot match `--ac-accent-2)`. When composing the PR description, run `var\(--ac-accent\)` (no `\b`) to get the true count.
  - **Live enumeration captured 2026-06-01** (from `skills/visualisation/visualise/frontend`):
    - `var(--ac-shadow-soft)` — 1 file / 1 site (`src/routes/lifecycle/LifecycleIndex.module.css:79`).
    - `var(--ac-shadow-lift)` — 3 files / 3 sites (`Toaster.module.css:28`, `Popover.module.css:15`, `Toaster.test.tsx:160` — the last is a test-string literal, not a runtime consumer).
    - `var(--ac-accent)` — 20 files / 52 sites.
    - `var(--ac-accent-2)` — 3 files / 4 sites (`FrontmatterTable.module.css:39`, `Brand.tsx:15`, `Brand.tsx:25`, `Brand.test.tsx:19` — the last is a test-string literal).
    - Per-token sum: **27 files / 60 sites**. Unique files after collapsing the 3-file overlap between `--ac-accent` and `--ac-accent-2` (`Brand.tsx`, `Brand.test.tsx`, `FrontmatterTable.module.css`): **21 unique files / 60 sites**.
  - When composing the PR description, update the consumer-enumeration figures to **21 unique files / 60 sites** (and ensure the AC#4 spirit-reading block-quote names "21 unique files" rather than "26").

#### Manual Verification:

- [ ] PR description contains all four shadow declarations (light and dark, current and prototype) quoted verbatim with file:line references.
- [ ] PR description explicitly states "no divergence — values match exactly" for shadows (AC#2).
- [ ] PR description records the dark `--ac-accent` and `--ac-accent-2` computed values harvested from Phase 1's spec output, normalised to `rgb()`, with the expected-equality confirmation (AC#3).
- [ ] PR description includes the consumer enumeration table plus the AC#4 spirit-reading paragraph naming the no-follow-up outcome (AC#4).
- [ ] PR description notes that the only code change is `root-resolved-tokens.spec.ts`.
- [ ] All four acceptance-criteria checkboxes in `meta/work/0077-shadow-and-dark-accent-token-audit.md` are tickable from the PR description content.

---

## Testing Strategy

### Unit Tests:

- No new unit tests. Existing `src/styles/global.test.ts` parity coverage at lines 202–203 (CSS↔TS parity extending past `--ac-shadow-lift`) and `src/styles/prototype-tokens.fixture.test.ts` already cover the static-string parity.

### Integration Tests:

- The new Playwright spec `root-resolved-tokens.spec.ts` is the integration-level assertion: it exercises the full CSS cascade (including the dark-theme attribute swap and the `@media (prefers-color-scheme: dark)` MIRROR-B path via `page.emulateMedia({ colorScheme: 'dark' })` with `data-theme` removed) and reads what the browser actually computes, not what is declared in source.

### Manual Testing Steps:

1. Run `cd skills/visualisation/visualise/frontend && npx playwright test tests/visual-regression/root-resolved-tokens.spec.ts` and confirm three passing tests under the `root resolved tokens` describe: `values resolve under [data-theme="light"]`, `values resolve under [data-theme="dark"]`, and `values resolve under prefers-color-scheme: dark (no data-theme)`.
2. Run the full `cd skills/visualisation/visualise/frontend && npm test` to confirm no regressions in any existing suite (including the unchanged `tokens.spec.ts` baselines).

## Performance Considerations

None. The new spec adds three tests that each navigate to `/library`
once, swap themes via attribute mutation or `emulateMedia`, and read
four computed properties — negligible compared to the existing
visual-regression suite's screenshot capture cost.

## Migration Notes

None. No token values change. No data migration. No baseline
migration. Existing baselines (light and dark) remain untouched.

## References

- Original work item: [`meta/work/0077-shadow-and-dark-accent-token-audit.md`](../work/0077-shadow-and-dark-accent-token-audit.md)
- Companion research: [`meta/research/codebase/2026-05-31-0077-shadow-and-dark-accent-token-audit.md`](../research/codebase/2026-05-31-0077-shadow-and-dark-accent-token-audit.md)
- Work-item review: [`meta/reviews/work/0077-shadow-and-dark-accent-token-audit-review-1.md`](../reviews/work/0077-shadow-and-dark-accent-token-audit-review-1.md)
- Related work items:
  - [`meta/work/0033-design-token-system.md`](../work/0033-design-token-system.md) — introduced the per-theme shadow split and dark accent remap.
  - [`meta/work/0034-theme-and-font-mode-toggles.md`](../work/0034-theme-and-font-mode-toggles.md) — established the `setTheme`/`applyTheme` Playwright helpers this plan reuses.
- Source-of-truth declarations: `skills/visualisation/visualise/frontend/src/styles/global.css:201–202, 329–330, 364–365, 391–392, 422–423` and `skills/visualisation/visualise/frontend/src/styles/tokens.ts:82–83, 177–189`.
- Reusable test primitives: `skills/visualisation/visualise/frontend/tests/visual-regression/lib/expected-colours.ts:73` (`setTheme(page, theme)`), `:11` (`hexToRgb(hex)`).
- Prototype declarations (for verbatim PR-description quotation): `/Users/tobyclemson/Downloads/Accelerator/src/app.css:36–37, 63–64, 68–69`.
