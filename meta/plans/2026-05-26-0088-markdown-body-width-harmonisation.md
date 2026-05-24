---
date: "2026-05-26T16:30:00+01:00"
type: plan
skill: create-plan
work-item: "0088"
status: accepted
---

# 0088 Markdown Body Width Harmonisation — Implementation Plan

## Overview

Replace the hard-coded `max-width: 720px` on `.markdown` in
`MarkdownRenderer.module.css` with a new layout token
`--ac-content-max-width-prose: 72ch` (declared in both
`global.css` and `LAYOUT_TOKENS` in `tokens.ts`), wrapped in
`min(var(--ac-content-max-width-prose), 100%)` so the cap composes
with the parent grid track at any viewport. Pin the markdown body
size explicitly with `font-size: var(--size-body)` on the same
element so the `ch`-based measure resolves against a known
context. Retire the `EXCEPTIONS` entry that previously admitted
the `720px` literal as irreducible, and introduce two new
Playwright screenshot specs (`/library/plans/first-plan`,
`/code-syntax-showcase`) so the markdown-body surface gains the
PNG regression coverage the story's AC implies. Delivered as two
internally-independent phases driven test-first: Phase 1
introduces and verifies the token; Phase 2 consumes it, removes
the exception, and lands the screenshot baselines.

## Current State Analysis

- **Hard-coded literal**:
  `MarkdownRenderer.module.css:2` declares `max-width: 720px` on
  `.markdown`. `.markdown` has no `font-size` of its own — body
  text inherits from the document root, leaving the *effective*
  reading measure coupled to whatever the inherited size is.
- **Token surface (two-mirror discipline)**: `global.css:195-198`
  carries the Layout block in the single `:root` rule:
  ```css
  /* Layout */
  --ac-topbar-h: 48px;
  --ac-content-max-width:        1200px;
  --ac-content-max-width-narrow: 600px;
  ```
  Mirrored at `tokens.ts:189-193` in `LAYOUT_TOKENS`. The
  `global.test.ts:99-114` parametric `describe.each` already
  drives a parity check between the two for every key in
  `LAYOUT_TOKENS`; adding a key to one side without the other
  fails the suite.
- **Body-size token already exists**: `--size-body: 20px` is
  declared at `global.css:151` inside the size scale.
  `MarkdownRenderer.module.css` already consumes `--size-*`
  tokens for every other `font-size` declaration in the file
  (h1/h2/h3, pre, codeblockLang, inline code at lines 9–53),
  so adding `font-size: var(--size-body)` on `.markdown`
  continues the established pattern.
- **`EXCEPTIONS` entry in place**:
  `migration.test.ts:70` lists
  ```ts
  { file: 'components/MarkdownRenderer/MarkdownRenderer.module.css',
    literal: '720px', count: 1, kind: 'irreducible',
    reason: 'prose max-width — no token equivalent' }
  ```
  This entry is forced into existence by the `EXCEPTIONS hygiene`
  block at `migration.test.ts:401-432`, which asserts every
  observed unit-literal in `cssBySrcRelative` has a matching
  exception entry with the right count. Removing the literal
  without removing the entry — or the reverse — fails the suite.
- **Unit-regex coverage**: `PX_REM_EM_RE` at
  `migration.test.ts:31` matches only `px|rem|em`. The new `72ch`
  token value is invisible to the literal-detector, so no new
  exception entry is needed for the token itself.
- **Var-reference resolver**: `migration.test.ts:323-355` builds
  its declared-token set from `Object.keys(LAYOUT_TOKENS)` at
  line 332. Adding `'ac-content-max-width-prose'` to
  `LAYOUT_TOKENS` is sufficient for `var(--ac-content-max-width-prose)`
  references in CSS to resolve under that check.
- **Consumers of `MarkdownRenderer`** (uniformly affected — no
  per-surface carve-out):
  1. `LibraryDocView.tsx:144-146` — renders inside a
     `1fr | 260px` article grid (`LibraryDocView.module.css:1-9`)
     under the 1200px `Page` wrapper. At a 1440px viewport the
     body column has ~828px of horizontal budget, so the new
     `72ch ≈ 720px` cap applies; at 800px the budget shrinks
     below 720px so the `100%` branch of `min()` applies.
  2. `CodeSyntaxShowcase.tsx:79-90` — dev-only Playwright
     fixture with no width wrapping; the `.markdown` rule's
     `max-width` is the only width cap active here.
  3. `FrontmatterTable.tsx` is **not** a `MarkdownRenderer`
     consumer — it imports only the `Resolver` type from
     `MarkdownRenderer/wiki-link-plugin` and does not render
     `<MarkdownRenderer>` or apply the `.markdown` class. The
     CSS edit has no effect on this component. (The research
     note originally listed it as a third consumer; that was a
     misclassification.)
- **Playwright visual-regression**: under
  `tests/visual-regression/`, only `tokens.spec.ts`,
  `glyph-showcase.spec.ts`, and `chip-showcase.spec.ts` carry
  `toHaveScreenshot` baselines today. None of them render the
  markdown body. `typography-resolved-sizes.spec.ts` exercises
  `/library/plans/first-plan` for an H1 resolved-size assertion
  but produces no screenshot. `code-block-resolved-colours.spec.ts`
  exercises `/code-syntax-showcase` for colour assertions only.
  → The story's "refreshed baselines for `library-doc-view` and
  `code-syntax-showcase`" AC is satisfied not by *refreshing*
  PNGs but by *introducing* the screenshot specs and shipping
  the initial baselines.
- **Playwright suite invocation**: no dedicated npm script;
  `package.json:17` exposes `"test:e2e": "playwright test"`. The
  visual-regression project is defined at
  `playwright.config.ts:21-29` with `snapshotDir: './tests/visual-regression/__screenshots__'`.

### Key Discoveries

- `src/styles/global.test.ts:99-114` — the parametric `LAYOUT_TOKENS`
  parity check is the natural TDD hook for Phase 1: a one-line
  edit to `tokens.ts` fails the suite until `global.css` is
  mirrored.
- `src/styles/migration.test.ts:401-432` — the `EXCEPTIONS hygiene`
  block enforces the literal-vs-entry equality in both
  directions, so the Phase 2 CSS edit and the
  `migration.test.ts:70` deletion must land together.
- `src/components/MarkdownRenderer/MarkdownRenderer.module.css:1-5`
  — the `.markdown` rule that owns both the literal width and
  the implicit (inherited) font-size; the `1ch` cap and the
  `--size-body` pin must live on this same element so the
  measure resolves against the renderer's own font context.
- `tests/visual-regression/tokens.spec.ts:1-87` — the canonical
  precedent for `toHaveScreenshot` specs in this repo (viewport
  1440×900, theme loop via `data-theme="dark"`, 0.05
  `maxDiffPixelRatio`, `animations: 'disabled'`); the new specs
  follow the same shape.
- `src/styles/migration.test.ts:31` `PX_REM_EM_RE` — `ch` is not
  in its alternation, so the new `72ch` token value never trips
  the unit guards.

## Desired End State

- A new layout token `--ac-content-max-width-prose: 72ch` is
  declared in `global.css` inside the same `:root` Layout block
  as the existing `--ac-content-max-width{,-narrow}` family, and
  mirrored in `LAYOUT_TOKENS` in `tokens.ts` with the value
  literal `'72ch'` (the `ch` unit is baked into the value).
- `MarkdownRenderer.module.css` `.markdown` rule reads:
  ```css
  .markdown {
    max-width: min(var(--ac-content-max-width-prose), 100%);
    font-size: var(--size-body);
    line-height: 1.6;
    color: var(--ac-fg-strong);
  }
  ```
  with **no** `720px` literal anywhere in the file, and **no**
  literal-px `font-size` anywhere in the file.
- `migration.test.ts` no longer contains the
  `MarkdownRenderer.module.css … 720px` exception entry.
- Two new Playwright screenshot specs ship baseline PNGs that
  render the markdown body at the new 20px body-size:
  - `tests/visual-regression/library-doc-view.spec.ts`
    (renders `/library/plans/first-plan` at 1440×900, light +
    dark).
  - `tests/visual-regression/code-syntax-showcase.spec.ts`
    (renders `/code-syntax-showcase` at 1440×900, light + dark).
- All vitest suites pass: `npm run test`.
- All Playwright suites pass: `npm run test:e2e`.
- Type checking passes: `npm run typecheck`.
- The renderer's behaviour at each documented breakpoint matches
  the story's ACs: the cap applies at 1440px, the `100%` branch
  applies at 800px, and the prose column never bleeds into the
  260px aside grid track.

### Verifications:

- `npm run test` from
  `skills/visualisation/visualise/frontend/` passes.
- `npm run test:e2e` from
  `skills/visualisation/visualise/frontend/` passes.
- `npm run typecheck` from
  `skills/visualisation/visualise/frontend/` passes.
- Manual: `/library/plans/first-plan` at 1440px viewport shows
  the markdown body capped at ~720px (visibly inside the
  `1fr | 260px` grid track, not reaching the aside).
- Manual: `/library/plans/first-plan` at 800px viewport shows
  the markdown body filling its grid-track width (the `100%`
  branch is active).
- Manual: paragraph text at 20px is visibly larger than the
  pre-change inherited-root rendering.

## What We're NOT Doing

- **Not touching `FrontmatterTable`.** It imports only the
  `Resolver` type from `MarkdownRenderer/wiki-link-plugin`; it
  does not render `<MarkdownRenderer>` or apply the `.markdown`
  class, so the CSS edit has no effect on it. No verification
  is in scope because there is no behavioural overlap.
- **Not touching `LibraryTemplatesView` / `TemplateHighlight`.**
  Templates preview uses `TemplateHighlight` (hljs monospace),
  not `MarkdownRenderer`, and a `ch`-based reading measure is
  not the right policy for a code-preview surface. A future work
  item can decide its width policy independently.
- **Not introducing a new size token.** `--size-body: 20px`
  exists already; this plan consumes it, it does not augment the
  scale.
- **Not touching `Page.module.css`**, the 1200px page wrapper,
  or the `LibraryDocView` grid. The story's AC about the prose
  column not bleeding into the aside is satisfied by the
  composition of the cap with the existing grid track; no grid
  change is required.
- **Not re-litigating the choice of `72ch`** vs the more
  conservative 65ch reading-comfort default. The story records
  the rationale for 72ch (visual continuity with the 720px
  literal); a future story can revisit on reading-comfort
  grounds.
- **Not changing `--size-body` itself** (20px is taken as the
  canonical body size for prose, established by 0033).

## Implementation Approach

The work is decomposed into two internally-independent phases.
Phase 1 introduces the token and is shippable on its own (the
token is harmless if never consumed). Phase 2 consumes the
token, retires the exception, and ships the new visual-regression
coverage. Each phase is driven test-first: each begins by
writing the test (or by making a change that fails an existing
test), then making the minimum edit that turns the suite green.

Two design notes that anchor the approach:

1. **`1ch` lives on `.markdown`, not on a wrapper.** CSS `ch` is
   the advance width of the `0` glyph in the *element's own*
   computed font and font-size. Putting the cap on `.markdown`
   means the new explicit `font-size: var(--size-body)` on the
   same element fixes the measure's reference frame — bundling
   the two is not a convenience, it is what makes the cap
   meaningful. This is why both edits live in Phase 2.
2. **The token value bakes in the `ch` unit.** `LAYOUT_TOKENS`
   holds `'72ch'` (string), not the bare number 72. Consumers
   cannot accidentally re-unit it. This follows the existing
   convention in `LAYOUT_TOKENS` (`'48px'`, `'1200px'`, etc).

## Phase 1: Introduce `--ac-content-max-width-prose` token

### Overview

Declare a new theme-invariant layout token
`--ac-content-max-width-prose: 72ch` in both `global.css`
(`:root` Layout block) and `LAYOUT_TOKENS` in `tokens.ts`,
mirroring the existing `--ac-content-max-width{,-narrow}` family.
No consumers yet — the token is unused at the end of this phase
and Phase 2 wires it in. The existing parity test in
`global.test.ts` drives the parity edits test-first.

### Changes Required:

#### 1. Add the token to `LAYOUT_TOKENS` (Red — fails parity)

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`

**Change**: Add a new entry between the existing `-narrow` line
and the closing brace so the family stays contiguous:

```ts
export const LAYOUT_TOKENS = {
  'ac-topbar-h': '48px',
  'ac-content-max-width': '1200px',
  'ac-content-max-width-narrow': '600px',
  'ac-content-max-width-prose': '72ch',
} as const
```

After this edit, run `npm run test -- global.test.ts`. The
parametric `describe.each` at `global.test.ts:99-114` iterates
`Object.entries(LAYOUT_TOKENS)` and asserts each name resolves
in `global.css :root`. The new key fails — this is the red.

#### 2. Add the token to `global.css` (Green)

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`

**Change**: Insert the new declaration immediately after the
`-narrow` line so the family stays contiguous in source order:

```css
/* Layout */
--ac-topbar-h: 48px;
--ac-content-max-width:        1200px;
--ac-content-max-width-narrow: 600px;
--ac-content-max-width-prose:  72ch;
```

(Align the colon-following whitespace with the existing entries
so the file's column alignment is preserved.)

After this edit, `npm run test -- global.test.ts` passes. The
key now exists on both sides of the parity check.

### Success Criteria:

#### Automated Verification:

- [ ] Vitest passes for the styles module:
      `npm run test -- src/styles/`
      (covers the `LAYOUT_TOKENS ↔ global.css` parity case for
      the new key plus the `var(--NAME) references resolve`
      ratchet, which gains the new key automatically via
      `Object.keys(LAYOUT_TOKENS)` at `migration.test.ts:332`).
- [ ] Full vitest suite passes: `npm run test`.
- [ ] Type checking passes: `npm run typecheck` (the
      `LayoutToken` alias at `tokens.ts:321` is derived from
      `keyof typeof LAYOUT_TOKENS`, so the new key gains type
      coverage with no code change).
- [ ] Playwright suite passes unchanged: `npm run test:e2e`.

#### Manual Verification:

- [ ] DevTools on any page shows `--ac-content-max-width-prose`
      defined on `:root` with computed value `72ch`.
- [ ] Grepping for the new token name yields exactly two
      declarations (one in `global.css`, one in `tokens.ts`) and
      zero consumers (Phase 2 adds the consumer).

---

## Phase 2: Apply the token in `MarkdownRenderer`, retire the exception, add baselines

### Overview

Replace the hard-coded `max-width: 720px` in
`MarkdownRenderer.module.css` with
`min(var(--ac-content-max-width-prose), 100%)`, add an explicit
`font-size: var(--size-body)` on the same rule, delete the
matching `EXCEPTIONS` entry, and introduce two new Playwright
screenshot specs that capture the markdown body surface in
light + dark themes at 1440×900. TDD ordering: introduce a tiny
new vitest case that fails until the CSS is rewritten, then let
the existing `EXCEPTIONS hygiene` block force the exception
deletion, then introduce the Playwright specs (the first run
creates the baselines, the second verifies them).

### Changes Required:

#### 1. Add a vitest case that pins the `.markdown` rule's token consumption (Red)

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

**Change**: Add a new `describe` block placed **immediately
after** the existing `describe('EXCEPTIONS hygiene', …)` block
that ends at line 432. This position keeps the new block
adjacent to its thematic neighbours while ensuring the
file-scoped `cssBySrcRelative` map (built at lines 391–395) is
already in scope. The block tests architectural facts only —
that the renderer file mentions both token references — not the
specific `min(…, 100%)` syntactic shape. The existing
`EXCEPTIONS hygiene` block and the `var(--NAME) references
resolve` block already enforce the negative half of the
invariant (no `720px` literal, all var-refs declared); pixel-
level shape is locked by the new Playwright PNG baselines in
§6–§8.

```ts
describe('MarkdownRenderer .markdown rule consumes prose-width and body-size tokens', () => {
  const path = 'components/MarkdownRenderer/MarkdownRenderer.module.css'
  const css = cssBySrcRelative.get(path)
  const itIfPresent = css ? it : it.skip

  it('the file is discoverable', () => {
    expect(css, `expected ${path} to be globbed by cssModules`).toBeDefined()
  })

  itIfPresent('references var(--ac-content-max-width-prose) for the prose cap', () => {
    expect(css!).toContain('var(--ac-content-max-width-prose)')
  })

  itIfPresent('references var(--size-body) for the body font-size', () => {
    expect(css!).toContain('var(--size-body)')
  })
})
```

Run `npm run test -- migration.test.ts`. The two
`itIfPresent` assertions fail (the current `.markdown` rule
has neither var-ref). This is the red. The `itIfPresent`
guard short-circuits the assertions on a missing file so the
file-rename failure mode produces one coherent diagnostic
instead of one real plus two cryptic ones.

#### 2. Rewrite the `.markdown` rule (Green for the new case)

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css`

**Change**: Edit the rule at lines 1–5 to:

```css
/* font-size + max-width co-located: 1ch resolves against this
   element's own computed font, so the prose cap is meaningful
   only when both declarations live on the same rule. */
.markdown {
  max-width: min(var(--ac-content-max-width-prose), 100%);
  font-size: var(--size-body);
  line-height: 1.6;
  color: var(--ac-fg-strong);
}
```

The comment captures the single non-obvious design invariant
from Implementation Approach §1 so that a future CSS refactor
which splits `font-size` and `max-width` across different
selectors hits an in-file warning rather than silently breaking
the measure's reference frame.

Run `npm run test -- migration.test.ts`. The new
`MarkdownRenderer .markdown rule …` describe now passes — but
the existing `EXCEPTIONS hygiene` block at lines 401–432 now
fails: the `720px` literal is gone from the file (observed
count: 0) but the declared count is still 1. This is the next
red, fired by the existing machinery.

#### 3. Delete the `720px` `EXCEPTIONS` entry (Green for hygiene)

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

**Change**: Delete line 70 verbatim:

```ts
// DELETE this line:
{ file: 'components/MarkdownRenderer/MarkdownRenderer.module.css', literal: '720px', count: 1, kind: 'irreducible', reason: 'prose max-width — no token equivalent' },
```

The surrounding `MarkdownRenderer` exception group at lines
64–69 stays intact (those cover 1px borders, the 0.4rem cell
padding, 0.1rem code padding, 4px blockquote, 6px code-block
radii — all still present in the file and unaffected by this
story).

Run `npm run test -- migration.test.ts`. The `EXCEPTIONS hygiene`
block passes (declared = 0 occurrences of `720px` in the file,
observed = 0). The full vitest suite is now green.

#### 4. Add a body-`p` resolved-size case to typography-resolved-sizes.spec.ts

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/typography-resolved-sizes.spec.ts`

**Change**: Add one entry to the `CASES` array (currently at
lines 42–129) so the new explicit `font-size: var(--size-body)`
on `.markdown` becomes a fast, deterministic CI guard
alongside the slower PNG comparison. The spec already
navigates to `/library/plans/first-plan` for the
`MarkdownRenderer H1` case, so this is the file's idiomatic
sibling extension:

```ts
{
  name: 'MarkdownRenderer body p',
  route: '/library/plans/first-plan',
  selector: '[class*="markdown"] p',
  expected: '20px',
},
```

Run `npm run test:e2e -- typography-resolved-sizes.spec.ts`.
Pre-change, the case fails (paragraphs inherit the document-
root size, which resolves to 16px). Post-change (the §2 CSS
rewrite is already landed by this point), it passes. The
case follows the file's existing convention of one
representative selector per outlier and protects the body-size
invariant from a silent regression that a PNG re-baseline
would otherwise absorb.

#### 5. Extract shared visual-regression helpers

**File** (new):
`skills/visualisation/visualise/frontend/tests/visual-regression/helpers.ts`

**Change**: Promote two helpers currently inlined in
`tokens.spec.ts` to a shared module so the new specs in §6 and
§7 (and any future visual-regression spec) consume one
canonical implementation. Without this, every new spec faces
the same copy-paste-and-tweak decision the plan would
otherwise introduce twice.

```ts
import type { Page } from '@playwright/test'

// Relative timestamps ("57s ago", "2m ago") change between baseline
// capture and test runs. Mask any <span> whose text ends with " ago"
// so pixel differences in card headers / article chrome don't cause
// spurious failures.
export const relativeTimeMask = (page: Page) =>
  page.locator('span').filter({ hasText: / ago$/ })

// Apply a theme by mutating `data-theme` on <html>, then wait for a
// rAF so the browser commits the style recalculation before the next
// observable read or screenshot. Light is the default theme and
// requires no mutation.
export const applyTheme = async (page: Page, theme: 'light' | 'dark') => {
  if (theme === 'light') return
  await page.evaluate(() => new Promise<void>(resolve => {
    document.documentElement.dataset.theme = 'dark'
    requestAnimationFrame(() => resolve())
  }))
}
```

**File** (existing):
`skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts`

**Change**: Replace the inlined `relativeTimeMask` declaration
(lines 1–7) and the two inline `data-theme` + rAF blocks (the
one in the main loop at lines 25–32 and the duplicate in
`lifecycle-cluster-after-click (dark)` at lines 60–63) with
imports from the new `helpers.ts`. Substitute the loop's
conditional with `await applyTheme(page, theme)`. No
behaviour change; the baselines stay byte-identical.

#### 6. Add Playwright screenshot spec for the library detail route

**File** (new):
`skills/visualisation/visualise/frontend/tests/visual-regression/library-doc-view.spec.ts`

**Change**: Add a new spec modelled on the existing
`tokens.spec.ts` pattern: a per-file `ROUTES` table iterated by
`(id, path)` and theme, 1440×900 viewport, shared helpers from
§5, `toHaveScreenshot` with `maxDiffPixelRatio: 0.05` and
`animations: 'disabled'`. Two readiness gates are critical:
(a) `LibraryDocView` issues two independent React Query reads
via `useDocPageData` — `useDocContent` and `useRelated` — and
renders `<p>Loading…</p>` placeholders for *both* (the body at
`LibraryDocView.tsx:74` and the aside at `:113`). The spec must
wait for all `Loading…` placeholders to detach before
capturing, otherwise a half-loaded aside or body lands in the
baseline. (b) The long-form prose exercises web fonts (Sora
display, Inter body) whose late arrival would diff against the
baseline, so `document.fonts.ready` must settle before the
screenshot. The `.then(() => undefined)` is required because
`document.fonts.ready` resolves to the non-serialisable
`FontFaceSet` — Playwright's `page.evaluate` structured-clones
the resolved value back, so returning `undefined` avoids the
serialisation error.

```ts
import { test, expect } from '@playwright/test'
import { applyTheme, relativeTimeMask } from './helpers'

const ROUTES = [
  ['library-doc-view', '/library/plans/first-plan'],
] as const

const VIEWPORT = { width: 1440, height: 900 }

for (const [id, path] of ROUTES) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${id} (${theme})`, async ({ page }) => {
      await page.setViewportSize(VIEWPORT)
      await page.goto(path)
      await applyTheme(page, theme)
      // Both body and aside render <p>Loading…</p> placeholders;
      // wait for every Loading text node to detach before capture.
      await expect(page.getByText('Loading…')).toHaveCount(0)
      await page.evaluate(() => document.fonts.ready.then(() => undefined))
      await expect(page).toHaveScreenshot(`${id}-${theme}.png`, {
        maxDiffPixelRatio: 0.05,
        animations: 'disabled',
        mask: [relativeTimeMask(page)],
      })
    })
  }
}
```

#### 7. Add Playwright screenshot spec for the code-syntax-showcase route

**File** (new):
`skills/visualisation/visualise/frontend/tests/visual-regression/code-syntax-showcase.spec.ts`

**Change**: Add a new spec mirroring the library-doc-view spec
but pointed at `/code-syntax-showcase`. The route has no
relative-time chrome, so the mask is dropped. The readiness
gate is anchored on a highlighted-code token (matching the
existing `code-block-resolved-colours.spec.ts` precedent of
waiting on `.hljs-*` tokens to be present) plus the same
`document.fonts.ready` settle:

```ts
import { test, expect } from '@playwright/test'
import { applyTheme } from './helpers'

const ROUTES = [
  ['code-syntax-showcase', '/code-syntax-showcase'],
] as const

const VIEWPORT = { width: 1440, height: 900 }

for (const [id, path] of ROUTES) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${id} (${theme})`, async ({ page }) => {
      await page.setViewportSize(VIEWPORT)
      await page.goto(path)
      await applyTheme(page, theme)
      await page.locator('.hljs-keyword').first().waitFor()
      await page.evaluate(() => document.fonts.ready.then(() => undefined))
      await expect(page).toHaveScreenshot(`${id}-${theme}.png`, {
        maxDiffPixelRatio: 0.05,
        animations: 'disabled',
      })
    })
  }
}
```

#### 8. Capture the initial baselines

**Command**:

```sh
cd skills/visualisation/visualise/frontend
npm run test:e2e -- --project=visual-regression --update-snapshots \
  tests/visual-regression/library-doc-view.spec.ts \
  tests/visual-regression/code-syntax-showcase.spec.ts
```

Playwright appends a `-<project>-<platform>` suffix to each
snapshot filename. Matching the existing
`tokens.spec.ts-snapshots/` policy (both `-darwin.png` and
`-linux.png` committed), this populates 8 PNGs total:

- `__screenshots__/library-doc-view.spec.ts-snapshots/library-doc-view-light-visual-regression-darwin.png`
- `__screenshots__/library-doc-view.spec.ts-snapshots/library-doc-view-light-visual-regression-linux.png`
- `__screenshots__/library-doc-view.spec.ts-snapshots/library-doc-view-dark-visual-regression-darwin.png`
- `__screenshots__/library-doc-view.spec.ts-snapshots/library-doc-view-dark-visual-regression-linux.png`
- `__screenshots__/code-syntax-showcase.spec.ts-snapshots/code-syntax-showcase-light-visual-regression-darwin.png`
- `__screenshots__/code-syntax-showcase.spec.ts-snapshots/code-syntax-showcase-light-visual-regression-linux.png`
- `__screenshots__/code-syntax-showcase.spec.ts-snapshots/code-syntax-showcase-dark-visual-regression-darwin.png`
- `__screenshots__/code-syntax-showcase.spec.ts-snapshots/code-syntax-showcase-dark-visual-regression-linux.png`

The darwin baselines come from a local capture; the linux
baselines should be captured either inside the CI container
(canonical) or by running the same command on a Linux host —
font rasterisation differs enough between macOS and Linux that
relying on the 0.05 diff tolerance to absorb the gap will
produce post-merge CI bounces. The simplest workflow is: land
the spec with darwin baselines, let the first CI run fail with
"missing linux snapshot", pull the generated PNGs back into
the PR.

Then re-run without `--update-snapshots` to verify the specs
pass against the captured baselines:

```sh
npm run test:e2e -- --project=visual-regression \
  tests/visual-regression/library-doc-view.spec.ts \
  tests/visual-regression/code-syntax-showcase.spec.ts
```

Commit the 8 PNGs alongside the spec files. The PNGs
canonicalise the 20px body size as the new baseline.

### Success Criteria:

#### Automated Verification:

- [ ] The `MarkdownRenderer.module.css … 720px` exception is no
      longer present in `migration.test.ts` (enforced by the
      `EXCEPTIONS hygiene` block; the absence of the literal in
      the file combined with the absence of the entry leaves
      both sides at count 0).
- [ ] The new `describe('MarkdownRenderer .markdown rule …')`
      block in `migration.test.ts` passes its two presence
      assertions (file mentions `var(--ac-content-max-width-prose)`
      and `var(--size-body)`).
- [ ] The existing `EXCEPTIONS hygiene` block passes (no stale
      entry, no uncovered literal — this is the authoritative
      ratchet for the no-`720px`-literal invariant).
- [ ] The existing `var(--NAME) references resolve` block
      passes (`var(--ac-content-max-width-prose)` and
      `var(--size-body)` references both resolve via
      `Object.keys(LAYOUT_TOKENS)` and the typography token
      set).
- [ ] New `typography-resolved-sizes.spec.ts MarkdownRenderer body p`
      case passes against the live `/library/plans/first-plan`
      route, returning `'20px'` — the deterministic CI guard
      that the new `font-size: var(--size-body)` declaration
      is in effect.
- [ ] Existing `typography-resolved-sizes.spec.ts MarkdownRenderer H1`
      case still passes — the H1 keeps its explicit
      `var(--size-h3)` (28px) override and is unaffected by the
      new parent `font-size`.
- [ ] Existing `code-block-resolved-colours.spec.ts` cases still
      pass — the spec asserts colours only; body font-size does
      not feature.
- [ ] Existing `tokens.spec.ts` cases still pass after the §5
      helpers refactor (no behaviour change; baselines
      byte-identical).
- [ ] New `library-doc-view.spec.ts` light/dark cases pass
      against committed PNGs (both darwin and linux).
- [ ] New `code-syntax-showcase.spec.ts` light/dark cases pass
      against committed PNGs (both darwin and linux).
- [ ] Existing `tokens.spec.ts` baselines (`library-light.png`,
      `library-dark.png`, etc.) are unchanged — the library hub
      and plans list don't render the markdown body and are not
      visually affected.
- [ ] Full vitest suite passes: `npm run test`.
- [ ] Full Playwright suite passes: `npm run test:e2e`.
- [ ] Type checking passes: `npm run typecheck`.

#### Manual Verification:

- [ ] Open `/library/plans/first-plan` in the dev server
      (`npm run dev`) at a 1440×900 viewport. Inspect
      `[class*="markdown"]` in DevTools and confirm:
      - computed `max-width` resolves to ~720px (i.e. the
        `72ch` branch of `min()` is active, not the `100%`
        branch);
      - computed `font-size` is `20px`;
      - the prose column does not extend into the right-hand
        260px aside grid track.
- [ ] At the same route, resize the window to ~800px width.
      Inspect `.markdown` again and confirm the computed
      `max-width` now equals the parent grid track's computed
      width (the `100%` branch is active) and the prose column
      does not overflow the page wrapper.
- [ ] Visit `/code-syntax-showcase` and confirm the markdown
      body and the code blocks render correctly under the new
      cap and body size in both light and dark themes (toggle
      via `document.documentElement.dataset.theme = 'dark'` in
      DevTools).
- [ ] Open `/library/plans/first-plan` once in light mode and
      once in dark mode and visually confirm the captured
      baseline PNGs are an accurate canonical reference (no
      off-by-one rendering artefacts, no font-loading flash,
      no half-loaded code blocks).

---

## Testing Strategy

### Unit (vitest)

- Phase 1 leans on the existing
  `LAYOUT_TOKENS ↔ global.css :root parity` `describe.each` —
  no new test is added; the parity loop drives the parity edit
  test-first.
- Phase 2 introduces one new `describe` block in
  `migration.test.ts` with two assertions pinning the token
  consumption inside the `.markdown` rule. The block uses the
  same `cssBySrcRelative` map that the existing hygiene block
  uses, so it integrates with the file's idioms.
- The existing `EXCEPTIONS hygiene` block at
  `migration.test.ts:401-432` serves as a second test driver
  for Phase 2 — it forces the EXCEPTIONS entry deletion as soon
  as the literal is gone.
- One new case is added to
  `typography-resolved-sizes.spec.ts` —
  `MarkdownRenderer body p` at `/library/plans/first-plan`,
  selector `[class*="markdown"] p`, expected `20px`. This
  gives the body-size invariant a deterministic CI guard
  alongside the slower PNG baselines.

### Integration / visual-regression (Playwright)

- Two new screenshot specs land in Phase 2:
  - `library-doc-view.spec.ts` — light + dark, 1440×900,
    `/library/plans/first-plan`.
  - `code-syntax-showcase.spec.ts` — light + dark, 1440×900,
    `/code-syntax-showcase`.
- Both specs follow the established `tokens.spec.ts` idiom
  (theme-loop, `data-theme` mutation, 0.05 `maxDiffPixelRatio`,
  `animations: 'disabled'`).
- The `library-doc-view` spec carries the `relativeTimeMask`
  helper to suppress " ago" pixel diffs; the
  `code-syntax-showcase` spec omits it (no relative-time
  chrome on that route).
- Baselines are captured with
  `--update-snapshots` once, then re-verified without the flag,
  before the changes are committed.

### Manual Testing Steps

1. `npm run dev` and open `/library/plans/first-plan`.
2. At 1440×900, confirm in DevTools that
   `[class*="markdown"]` has `max-width ≈ 720px`,
   `font-size = 20px`, and the body column ends well before
   the aside.
3. Resize the window to ~800×900 and confirm the body column
   tracks the grid's `1fr` track width (i.e. `100%` branch is
   active).
4. Toggle dark mode via DevTools
   (`document.documentElement.dataset.theme = 'dark'`) and
   confirm the body styling remains correct.
5. Open `/code-syntax-showcase` and confirm prose + code blocks
   render correctly under the new cap and body size in light and
   dark themes.

## Performance Considerations

- The change is a CSS-only edit on a static rule. There is no
  runtime cost difference between `720px` and
  `min(var(--ac-content-max-width-prose), 100%)` — modern
  browsers resolve the `min()` and `var()` once per rule
  application.
- The new `font-size: var(--size-body)` declaration adds a
  single property to the cascading default for `.markdown`
  descendants. No layout-thrash risk.

## Migration Notes

- The body-size change from inherited-root to `20px` is a
  visible change. The story records it as accepted — the new
  PNG baselines captured in Phase 2 canonicalise the new
  rendering. No data migration, no flag, no rollout work.
- The `LAYOUT_TOKENS` addition is purely additive; no consumer
  rename or alias is required.
- Phase 1 alone is shippable (an unused token is harmless under
  this codebase's two-mirror discipline — `global.test.ts`
  parity and `migration.test.ts` ratchet both stay green).
  Phase 2 builds on Phase 1; they will normally land in the
  same PR, but the phases can be reviewed and committed
  independently.

## References

- Work item: `meta/work/0088-markdown-body-width-harmonisation.md`
- Research: `meta/research/codebase/2026-05-26-0088-markdown-body-width-harmonisation.md`
- Source files:
  - `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:1-5`
  - `skills/visualisation/visualise/frontend/src/styles/global.css:195-198`
  - `skills/visualisation/visualise/frontend/src/styles/global.css:151` (`--size-body: 20px`)
  - `skills/visualisation/visualise/frontend/src/styles/tokens.ts:189-193`
  - `skills/visualisation/visualise/frontend/src/styles/tokens.ts:321` (`LayoutToken` alias)
  - `skills/visualisation/visualise/frontend/src/styles/global.test.ts:99-114` (parity loop)
  - `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:31` (`PX_REM_EM_RE`)
  - `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:70` (entry to delete)
  - `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:323-355` (var-resolve ratchet)
  - `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:401-432` (`EXCEPTIONS hygiene`)
  - `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:77` (renderer outer element)
  - `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:144-146`
  - `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css:1-9`
  - `skills/visualisation/visualise/frontend/src/routes/code-syntax-showcase/CodeSyntaxShowcase.tsx:79-90`
  - `skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts:1-87` (screenshot-spec precedent)
  - `skills/visualisation/visualise/frontend/playwright.config.ts:21-29` (visual-regression project)
- Related prior work:
  - `meta/work/0033-design-token-system.md` (status: done) — established the size scale and layout-token family.
  - `meta/plans/2026-05-23-0075-typography-size-scale-consumption.md` (status: accepted) — established the `font-size` consumption rule the new declaration adheres to.
  - `meta/work/0076-code-block-syntax-highlight-palette.md` (status: done) — recent neighbour edit on the same `MarkdownRenderer.module.css` file; no rebase risk against the current commit.
