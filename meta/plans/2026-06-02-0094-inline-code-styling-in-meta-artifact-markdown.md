---
type: plan
id: "2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown"
title: "Inline Code Styling In Meta Artifact Markdown Implementation Plan"
date: "2026-06-02T14:56:53+00:00"
author: "Toby Clemson"
producer: create-plan
status: accepted
work_item_id: "0094"
parent: ""
reviewer: "Toby Clemson"
tags: [visualiser, markdown, css, design-tokens, inline-code, bug]
revision: "c7f49b8e3ad2c6b37f8645d1168bb6be02939295"
repository: "ticket-management"
last_updated: "2026-06-02T17:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Inline Code Styling In Meta Artifact Markdown Implementation Plan

## Overview

Bring inline `` `code` `` spans in the visualiser's meta-artifact markdown
renderer into line with the frozen design prototype's `.ac-md-code` pill. The
headline defect is that inline code inherits the prose body font (Inter)
instead of the monospace face (Fira Code); the prototype also gives inline
code a soft border, a smaller font size, and tighter pill padding/radius. This
is a CSS-only fix to a single rule (plus one new table-cell rule), driven
test-first, in three independent phases. It consumes existing size tokens by
their current names — no token is renamed here; the scale's inconsistent
naming is handled by a separate remap initiative (see What We're NOT Doing).

## Current State Analysis

The live inline-code rule declares only four properties and sets neither
`font-family`, `color`, nor `border`:

`skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:57-60`
```css
.markdown code:not(pre code) {
  background: var(--ac-bg-sunken); border-radius: var(--radius-sm);
  padding: 0.1rem var(--sp-1); font-size: var(--size-xs);
}
```

- `font-family` is unset → inherits `--ac-font-body` (Inter). **The headline
  defect.**
- `border` is unset → no pill border.
- `font-size: var(--size-xs)` = 14px → too large vs the prototype's 11.5px.
- `padding: 0.1rem var(--sp-1)` and `border-radius: var(--radius-sm)` (4px) →
  minor divergences from the prototype's `1px 5px` / `3px`.
- `background: var(--ac-bg-sunken)` and inherited `color` already match the
  prototype — **no change needed there**.

The renderer applies **no class** to inline code — react-markdown emits a bare
`<code>` (`MarkdownRenderer.tsx:29-44` overrides only `pre`). So the fix must
be expressed through the existing `.markdown …` descendant selectors, not by
porting the prototype's `.ac-md-code` class.

There is **no `pre code` rule** anywhere in the module; fenced blocks receive
only `.markdown pre` (lines 19-27) styling. The `:not(pre code)` negation is
therefore a genuine structural guard, not merely conventional.

### Key Discoveries:

- **Font sizes must be tokens, not literals.** ADR-0036 (typography font-size
  consumption rule) categorically bans literal `font-size` in current-app CSS,
  enforced by `src/styles/migration.test.ts:565-612`. The exact tokens already
  exist: `--size-xxs-sm` = **11.5px** (`global.css:183`, mirrored
  `tokens.ts:164`) and `--size-eyebrow` = **11px** (`global.css:184`,
  `tokens.ts:165`). So the prototype's `11.5px`/`11px` are written as token
  references — identical computed pixels, so every acceptance criterion holds.
- **The 11px token is consumed by its current name; no rename here.** That 11px
  token is `--size-eyebrow`, classed in the convention comment
  (`global.css:160-167`) as a *semantic single-purpose* token. The table-cell
  rule (Phase 2) adds a second consumer, which would ordinarily argue for a
  more general name — but the whole sub-14px scale mixes three naming schemes
  (t-shirt tiers, semantic names, `-sm`/`-lg` tween suffixes) at every 0.5px
  step, so a one-token rename here would just add a fourth ad-hoc name. That
  systemic naming inconsistency is being addressed holistically by a **separate
  scale-remap initiative** (a dedicated work item + ADR superseding ADR-0036,
  adopting a pure-numeric scheme such as `--size-110` = 11px). 0094 therefore
  consumes `--size-eyebrow` as-is and the remap sweeps it up with every other
  consumer later. The transient "semantic token with two consumers" wart is
  accepted and resolved by the remap. `--size-eyebrow` is declared in
  `TYPOGRAPHY_TOKENS`, so the `var()`-resolves-to-declared-token test stays
  green.
- **The chrome literals are "irreducible" and must be ledgered.** ADR-0026 §3
  classifies `1px` border widths, sub-4px padding, and off-scale radii as
  irreducible literals that must be registered in the per-file `EXCEPTIONS`
  array in `migration.test.ts:46-69`. The harness keys per `(file, literal)`
  and a reverse-hygiene test (`migration.test.ts:431-452`) fails if declared
  count ≠ observed count, so the ledger must be updated exactly.
- **Table-cell override has a specificity trap.** A naive `.markdown td code`
  rule is specificity (0,1,2); the base rule `.markdown code:not(pre code)` is
  (0,1,3) — **higher** — so the td override would lose and 11px would never
  apply. The td rule must out-specify the base: `.markdown td code:not(pre
  code)` is (0,1,4). Writing the Playwright 11px assertion first surfaces this
  immediately.
- **Computed-style ACs belong in Playwright, not vitest.** jsdom resolves no
  cascade and no `var()` (documented at `Glyph.test.tsx:169-172`). Every
  `getComputedStyle` assertion in the suite lives under
  `tests/visual-regression/*.spec.ts` in real Chromium. The model to copy is
  `typography-resolved-sizes.spec.ts` (font-size) and
  `root-resolved-tokens.spec.ts:71-78` (resolving a token to a concrete
  rgb/px via a throwaway element). Fast structural guards use the
  CSS-as-text pattern at `migration.test.ts:455-471` (`css.toContain(...)`).
- **The e2e fixture has no inline code.** The route the existing MarkdownRenderer
  cases render against,
  `/library/plans/first-plan` ←
  `server/tests/fixtures/meta/plans/2026-01-01-first-plan.md`, currently
  contains only prose, two wiki-links, and a mermaid block (confirmed; the
  `typography-resolved-sizes.spec.ts` header comment also notes the absence).
  Each phase appends the fixture content it needs. Augmenting this fixture is
  safe: no screenshot baseline references it, and `fixture-coverage.spec.ts`
  asserts no counts.
- **Dark mode is free.** The fix only *consumes* already-themed colour tokens
  (`--ac-bg-sunken`, `--ac-stroke-soft`), so the MIRROR-A/MIRROR-B parity test
  in `global.test.ts` is not engaged and no dark-block edit is needed.

## Desired End State

Inline `` `code` `` spans in the meta-artifact markdown view render as a
monospace (Fira Code) pill: `--ac-bg-sunken` background, `1px solid
var(--ac-stroke-soft)` border, `3px` radius, `1px 5px` padding, `11.5px`
(`--size-xxs-sm`) in prose, and `11px` (`--size-eyebrow`) inside table body
cells. Table header (`th`) inline code stays at the 11.5px base. Fenced
`<pre><code>` blocks are visually unchanged. Both light and dark themes resolve
correctly. The full vitest and Playwright suites pass, including the ADR-0036
font-size ban and the ADR-0026 `EXCEPTIONS` hygiene check.

## What We're NOT Doing

- Not changing `background` or `color` of inline code (already correct).
- Not touching fenced-block rules (`.markdown pre`, `.codeblock*`) or the
  `pre` component override; the `:not(pre code)` scoping is retained verbatim.
- Not adding a `code` component override to `MarkdownRenderer.tsx` — the fix is
  pure CSS via descendant selectors.
- Not adding a new 11px token — the existing `--size-eyebrow` is reused as-is; a
  duplicate would violate ADR-0036's defined-but-not-consumed spirit.
- **Not renaming any token.** 0094 consumes `--size-eyebrow` (11px) and
  `--size-xxs-sm` (11.5px) by their current names. The sub-14px scale's mixed
  naming (tiers / semantic names / `-sm`-`-lg` tweens) is a real but systemic
  problem out of scope for this bug: it is captured as a **separate remap
  initiative** — a dedicated work item + ADR (superseding ADR-0036) adopting a
  pure-numeric scheme (e.g. `--size-110` = 11px) — which will rename
  `--size-eyebrow` along with ~100 other consumers in one coherent pass. Folding
  that ~100-site migration into this medium-priority bug would explode its scope
  and mix concerns.
- Not applying the 11px override to `th` header cells (decision: match the
  prototype's `tbody td`-only scoping).
- Not introducing new tokens for `3px` / `1px` / `5px` — per ADR-0026 these
  are irreducible literals admitted via the `EXCEPTIONS` ledger.

## Implementation Approach

Test-first throughout. Each phase writes its failing test(s) first (a fast
vitest CSS-as-text guard and/or a real-cascade Playwright spec), then makes
them pass with the minimal CSS change plus any required `EXCEPTIONS` ledger
edit and fixture content. The three phases are independent across phases — they
touch distinct selectors and distinct fixture content, can be implemented and
reviewed in any order, and only Phase 1 touches the `EXCEPTIONS` ledger. Two
invariants keep ordering safe: (a) every fixture change is an **append, never a
prepend**, so the typography spec's `[class*="markdown"] p`/`h1` `.first()`
selectors keep resolving to the original first paragraph/heading regardless of
phase order; and (b) the new spec's locators are scoped so they never collide
(`p > code` for prose, `td code`/`th code` for the table, `pre code` for the
fence).

Verification uses the repo's `mise` tasks (preferred over raw `npm`):

- `mise run test:unit:frontend` — Vitest (frontend unit + CSS-as-text guards).
- `mise run test:e2e:visualiser` — Playwright e2e; auto-runs `build:frontend`
  + `build:server:dev` and wires `ACCELERATOR_VISUALISER_BIN`, so the real
  cascade is exercised against the built SPA + server.

There is no `mise`/npm `lint` task; type-checking is
`npm --prefix skills/visualisation/visualise/frontend run typecheck`
(`tsc --noEmit`). For fast local iteration on a single spec, the underlying
npm scripts accept filters
(`npm --prefix …/frontend run test -- <pattern>`; the Playwright task is best
run via `mise` so the server binary and env are present).

---

## Phase 1: Inline-code base pill styling

### Overview

Amend the base inline-code rule so spans render as the prototype's monospace
pill: mono font, soft border, 11.5px token size, `1px 5px` padding, `3px`
radius. Update the `EXCEPTIONS` ledger for the new/removed literals. Covers
acceptance criteria 1, 2, 3, and 5 (dark mode). Independent.

### Changes Required:

#### 1. Test — fast CSS-as-text guard (write first, red)

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**: Add a sibling describe to the existing MarkdownRenderer text guard
(`migration.test.ts:455-471`), asserting the inline-code rule consumes the
intended tokens/literals.

```ts
describe('MarkdownRenderer inline-code rule (0094)', () => {
  const path = 'components/MarkdownRenderer/MarkdownRenderer.module.css'
  const css = cssBySrcRelative.get(path)
  const itIfPresent = css ? it : it.skip
  itIfPresent('inline code uses the monospace face', () => {
    expect(css!).toContain('font-family: var(--ac-font-mono)')
  })
  itIfPresent('inline code uses the 11.5px token, not --size-xs', () => {
    expect(css!).toContain('var(--size-xxs-sm)')
  })
  itIfPresent('inline code has the soft pill border', () => {
    expect(css!).toContain('1px solid var(--ac-stroke-soft)')
  })
  itIfPresent('inline code retains the :not(pre code) scoping', () => {
    expect(css!).toContain('code:not(pre code)')
  })
  itIfPresent('inline code no longer sizes off --size-xs', () => {
    // indexOf finds the base rule `.markdown code:not(pre code)` first (the
    // td override shares the substring); scope the negative check to its body
    // so it does not trip on `.markdown pre`'s legitimate var(--size-xs).
    const i = css!.indexOf('code:not(pre code)')
    const body = i >= 0 ? extractBlockBody(css!, i) : null
    expect(body).not.toBeNull()
    expect(body!).not.toContain('var(--size-xs)')
  })
})
```

#### 2. Test — real-cascade computed styles, both themes (write first, red)

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/inline-code-resolved-styles.spec.ts` (new)
**Changes**: New Playwright spec modelled on `chip-resolved-colours.spec.ts`
(`setTheme` + throwaway-element token resolution) and
`typography-resolved-sizes.spec.ts`. Render `/library/plans/first-plan`, locate
the prose inline `<code>` via a table-proof `p > code` selector, and assert
**fully-written** computed values in both themes — including the colour
assertions that AC2 and AC5 require, which must be executable `expect()` calls,
not comments. Reuse the shared `setTheme` helper rather than inlining the
dataset toggle. Resolve the theme-varying colour tokens through the cascade via
a throwaway element (`chip-resolved-colours.spec.ts:56-63`) and compare with
`toBe` — these are direct token consumptions, not `color-mix` blends, so exact
equality holds. Note `--ac-stroke-soft` is an `rgba(…)` token, so it must be
resolved through the cascade (`hexToRgb` would not apply). Also assert the
prose ≠ mono contrast that AC1 calls for, guarding the implicit precondition
that the default (non-`[data-font="mono"]`) mode is active. A separate case
cross-checks that the colours genuinely diverge between themes (so a token that
accidentally became theme-invariant can't pass both branches trivially). The
`[class*="markdown"]` root selector matches the deterministic source-name
prefix of the CSS-modules hashed class (not the generated hash suffix); the one
precedent for this is `typography-resolved-sizes.spec.ts`.

**Shared helper**: add `resolveToken(page, token)` to
`tests/visual-regression/lib/expected-colours.ts` (beside `setTheme`) rather
than defining it inline — the throwaway-element resolution pattern already
appears in `chip-resolved-colours.spec.ts:56-63` and
`root-resolved-tokens.spec.ts:71-78`, so this avoids a third divergent copy:

```ts
// lib/expected-colours.ts
export async function resolveToken(page: Page, token: string): Promise<string> {
  return page.evaluate((t) => {
    const tmp = document.createElement('div')
    tmp.style.color = `var(${t})`
    document.body.appendChild(tmp)
    const resolved = getComputedStyle(tmp).color
    tmp.remove()
    return resolved
  }, token)
}
```

```ts
import { test, expect } from '@playwright/test'
import { setTheme, resolveToken } from './lib/expected-colours'

test.use({ viewport: { width: 1280, height: 720 } })

// `p > code` (not `:not(pre) > code`) so the prose locator can never resolve
// to a table-cell `<code>` regardless of fixture append order.
const PROSE_CODE = '[class*="markdown"] p > code'

for (const theme of ['light', 'dark'] as const) {
  test(`inline code is a monospace pill (${theme})`, async ({ page }) => {
    await page.goto('/library/plans/first-plan')
    if (theme === 'dark') await setTheme(page, 'dark')

    const el = page.locator(PROSE_CODE).first()
    const s = await el.evaluate((n) => {
      const c = getComputedStyle(n)
      return {
        fontFamily: c.fontFamily, fontSize: c.fontSize,
        backgroundColor: c.backgroundColor,
        borderTopWidth: c.borderTopWidth, borderTopStyle: c.borderTopStyle,
        borderTopColor: c.borderTopColor,
        borderTopLeftRadius: c.borderTopLeftRadius,
        paddingTop: c.paddingTop, paddingLeft: c.paddingLeft,
      }
    })

    // Theme-invariant chrome (AC2 dimensions + AC3 size + AC1 face).
    expect(s.fontFamily).toContain('Fira Code')
    expect(s.fontSize).toBe('11.5px')
    expect(s.borderTopWidth).toBe('1px')
    expect(s.borderTopStyle).toBe('solid')
    expect(s.borderTopLeftRadius).toBe('3px')
    expect(s.paddingTop).toBe('1px')
    expect(s.paddingLeft).toBe('5px')

    // Theme-varying colours (AC2 colour + AC5): resolve the tokens through the
    // cascade and compare exactly. Light bg #f4f6fa / dark #070b12; light
    // stroke rgba(32,34,49,0.06) / dark rgba(255,255,255,0.04).
    expect(s.backgroundColor).toBe(await resolveToken(page, '--ac-bg-sunken'))
    expect(s.borderTopColor).toBe(await resolveToken(page, '--ac-stroke-soft'))

    // AC1 contrast: the prose body must NOT be the mono face. Guard the
    // precondition that the default font mode is active — [data-font="mono"]
    // repoints --ac-font-body to mono and would collapse the contrast.
    const fontMode = await page.evaluate(
      () => document.documentElement.dataset.font ?? 'default',
    )
    expect(fontMode).not.toBe('mono')
    const proseFont = await page
      .locator('[class*="markdown"] p')
      .first()
      .evaluate((n) => getComputedStyle(n).fontFamily)
    expect(proseFont).not.toContain('Fira Code')
  })
}

// AC5 divergence: the pill colours must actually change between themes, not
// merely resolve to *a* token value — otherwise a theme-invariant token would
// pass both per-theme branches above trivially.
test('inline-code pill colours diverge between light and dark', async ({ page }) => {
  await page.goto('/library/plans/first-plan')
  const lightBg = await resolveToken(page, '--ac-bg-sunken')
  const lightBorder = await resolveToken(page, '--ac-stroke-soft')
  await setTheme(page, 'dark')
  expect(await resolveToken(page, '--ac-bg-sunken')).not.toBe(lightBg)
  expect(await resolveToken(page, '--ac-stroke-soft')).not.toBe(lightBorder)
})
```

#### 3. Fixture — add prose inline code

**File**: `skills/visualisation/visualise/server/tests/fixtures/meta/plans/2026-01-01-first-plan.md`
**Changes**: Append a paragraph containing an inline code span. Append (do not
prepend): the new spec's `p > code` prose locator is already order-proof, but
the existing `.first()` selectors in `typography-resolved-sizes.spec.ts`
(`[class*="markdown"] p`, `[class*="markdown"] h1`) must keep resolving to the
original first paragraph/heading, which appending guarantees.

```markdown

Inline configuration like `--ac-font-mono` should render as a monospace pill.
```

#### 4. Implementation — amend the base rule

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css`
**Changes**: Replace the rule at lines 57-60.

```css
.markdown code:not(pre code) {
  background: var(--ac-bg-sunken);
  border: 1px solid var(--ac-stroke-soft);
  border-radius: 3px;
  padding: 1px 5px;
  font-family: var(--ac-font-mono);
  font-size: var(--size-xxs-sm);
}
```

#### 5. Implementation — update the `EXCEPTIONS` ledger

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**: In the MarkdownRenderer block (lines 64-69):

- **Bump** `1px` count `5` → `7` (the rule adds a `1px` border and a `1px`
  vertical padding). Restate the `reason` with the full enumeration in
  declaration order, e.g.: `'hairline borders + sub-px padding: <pre> stroke,
  h1 underline, table cell, codeblock wrapper, codeblockHead bottom, inline-code
  pill border, inline-code pill vertical padding — below --sp-1 floor'`.
- **Remove** the `0.1rem` entry (line 67) — the inline rule was its only
  occurrence in the file; with `padding: 1px 5px` it drops to zero, and the
  hygiene test requires declared == observed.
- **Add** `{ … literal: '3px', count: 1, kind: 'irreducible', reason: 'inline-code pill radius — below --radius-sm (4px)' }`.
- **Add** `{ … literal: '5px', count: 1, kind: 'irreducible', reason: 'inline-code pill horizontal padding — off-scale, between --sp-1 (4px) and --sp-2 (8px)' }`.
- **Note** the hygiene check counts *exact tokens*, not substrings
  (`migration.test.ts:445` filters with `h === literal` over regex-tokenised
  hits), so the new `1px`/`3px`/`5px` counts cannot be perturbed by the
  multi-digit literals `11px`/`11.5px` — which in this file are written as
  `var()` token references anyway, never as literals.

(`--ac-font-mono`, `--size-xxs-sm`, `--ac-stroke-soft`, `--ac-bg-sunken` are
token references — no ledger entry needed and all are declared in
`TYPOGRAPHY_TOKENS`/`tokens.ts`, so the `var()`-resolves-to-declared-token test
at `migration.test.ts:344-376` stays green.)

### Success Criteria:

#### Automated Verification:

- [x] Vitest passes (incl. ADR-0036 font-size ban + `EXCEPTIONS` hygiene): `mise run test:unit:frontend`
- [x] Playwright e2e passes incl. the new inline-code spec (light + dark) and the unchanged typography spec: `mise run test:e2e:visualiser`
- [x] Type-checking clean: `npm --prefix skills/visualisation/visualise/frontend run typecheck`

#### Manual Verification:

- [ ] Open any meta artifact with inline code in the visualiser; the span is a
  Fira Code pill with a soft border, visibly smaller than surrounding Inter prose.
- [ ] Toggle dark mode (`document.documentElement.dataset.theme = 'dark'`): the
  pill background darkens (`#070b12`) and the border remains a faint hairline.

---

## Phase 2: Table-cell font-size override

### Overview

Add a table-body-cell rule so inline code inside a `td` renders at 11px,
matching the prototype's `tbody td`-only scoping (header `th` code stays at the
11.5px base). The rule consumes the existing `--size-eyebrow` (11px) token by
its current name — no rename (see What We're NOT Doing; naming is deferred to
the scale-remap initiative). Covers acceptance criterion 4. Independent and
purely additive.

### Changes Required:

#### 1. Test — CSS-as-text guard (write first, red)

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**: Extend the 0094 describe:

```ts
itIfPresent('table-body inline code uses the 11px token, out-specifying the base rule', () => {
  expect(css!).toContain('td code:not(pre code)')
  expect(css!).toContain('var(--size-eyebrow)')
})
```

#### 2. Test — computed font-size in td vs th (write first, red)

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/inline-code-resolved-styles.spec.ts`
**Changes**: Add cases asserting the td override applies and th does not.

```ts
const sizeOf = (page, sel: string) =>
  page.locator(sel).first().evaluate((n) => getComputedStyle(n).fontSize)

test('table-body inline code is 11px, header + prose stay at the 11.5px base', async ({ page }) => {
  await page.goto('/library/plans/first-plan')
  // `th code` requires the GFM delimiter row (|---|) so the first table row
  // renders as <thead><th>; without it the header collapses to <td>.
  const [td, th, prose] = await Promise.all([
    sizeOf(page, '[class*="markdown"] td code'),
    sizeOf(page, '[class*="markdown"] th code'),
    sizeOf(page, '[class*="markdown"] p > code'),
  ])
  expect(td).toBe('11px')   // fails first if the (0,1,4) specificity is wrong
  // Assert the *intent* (header code == prose code, strictly above body code),
  // not just the bare 11.5px literal, so a future base-size change can't
  // silently void the td-only scoping.
  expect(th).toBe('11.5px')
  expect(th).toBe(prose)
})
```

#### 3. Fixture — add a table with inline code in a body cell and a header cell

**File**: `skills/visualisation/visualise/server/tests/fixtures/meta/plans/2026-01-01-first-plan.md`
**Changes**: Append a small GFM table with inline code in both a header cell
(to give the th assertion a node) and a body cell:

```markdown

| `tok` | Usage |
|---|---|
| `--ac-font-mono` | applied via `code` spans |
```

#### 4. Implementation — add the td rule

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css`
**Changes**: Add after the base inline rule (and after the table rules at
61-63). The `:not(pre code)` raises specificity to (0,1,4) so it beats the
base rule's (0,1,3); placement after it is belt-and-braces. Add a comment
tying it back to the base rule, since the two rules that jointly define the
inline-code pill are separated by the table-layout rules:

```css
/* table-body size override for the inline-code pill defined above —
   :not(pre code) lifts specificity to (0,1,4) to beat the base (0,1,3). */
.markdown td code:not(pre code) { font-size: var(--size-eyebrow); }
```

### Success Criteria:

#### Automated Verification:

- [x] Vitest passes — incl. the td CSS-as-text guard and the
  `var()`-resolves-to-declared-token test (`migration.test.ts:344-376`): `mise run test:unit:frontend`
- [x] Playwright e2e passes incl. td=11px / th=11.5px cases and `fixture-coverage`: `mise run test:e2e:visualiser`
- [x] Type-checking clean: `npm --prefix skills/visualisation/visualise/frontend run typecheck`

#### Manual Verification:

- [ ] In a meta artifact with a table, inline code inside body cells is
  visibly smaller (11px) than inline code in prose (11.5px); header-cell code
  matches prose size.

---

## Phase 3: Fenced-block isolation guard

### Overview

Lock in acceptance criterion 6: fenced `<pre><code>` blocks remain untouched by
the inline-pill styling. Pure non-regression — no production change. Independent
guard phase.

### Changes Required:

#### 1. Test — fenced code is unaffected (write first)

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/inline-code-resolved-styles.spec.ts`
**Changes**: Add a case asserting a fenced block's inner `<code>` is NOT
pill-styled — it keeps the `.markdown pre` size (`--size-xs` = 14px) and has no
inline border. The 14px assertion verifies *inheritance* from
`.markdown pre { font-size: var(--size-xs) }` — there is no direct `pre code`
font-size rule — so first confirm the syntax-highlight layer (rehype-highlight /
`.hljs`) sets no `font-size` on `pre code`/`.hljs`; if it ever did, this
assertion would silently stop reflecting the `.markdown pre` inheritance.

```ts
test('fenced code is not pill-styled', async ({ page }) => {
  await page.goto('/library/plans/first-plan')
  const s = await page.locator('[class*="markdown"] pre code').first()
    .evaluate((n) => {
      const c = getComputedStyle(n)
      return {
        fontSize: c.fontSize,
        borderTopWidth: c.borderTopWidth,
        backgroundColor: c.backgroundColor,
      }
    })
  expect(s.fontSize).toBe('14px')             // inherits .markdown pre var(--size-xs), not 11.5px
  expect(s.borderTopWidth).toBe('0px')        // no inline pill border leaked in
  // Anchor 'unchanged' on a second, independent property: the inner <code> has
  // no pill background of its own (the .markdown pre wrapper owns the surface),
  // so the sunken pill background must NOT have leaked onto it.
  expect(s.backgroundColor).toBe('rgba(0, 0, 0, 0)')
})
```

#### 2. Fixture — ensure a fenced code block with a language is present

**File**: `skills/visualisation/visualise/server/tests/fixtures/meta/plans/2026-01-01-first-plan.md`
**Changes**: The fixture currently has only a `mermaid` block (rendered as a
diagram, not a `<pre><code>` palette block). Append a labelled fenced block so
`pre code` exists to assert against:

````markdown

```js
const x = 1
```
````

#### 3. Test — scoping retained (CSS-as-text, already added in Phase 1)

The `code:not(pre code)` assertion from Phase 1's text guard already enforces
that the negation is retained. No further production change.

### Success Criteria:

#### Automated Verification:

- [ ] Playwright e2e passes incl. the fenced-code case and the unchanged
  fenced-block palette spec (`code-block-resolved-colours`): `mise run test:e2e:visualiser`
- [ ] Vitest stays green: `mise run test:unit:frontend`

#### Manual Verification:

- [ ] In a meta artifact containing both inline code and a fenced block, the
  fenced block is unchanged from before the change (no pill border, larger
  monospace size, dark code-surface background).

---

## Testing Strategy

### Unit (vitest, CSS-as-text — `mise run test:unit:frontend`):

- New 0094 describe in `migration.test.ts` asserting the inline rule consumes
  `--ac-font-mono`, `--size-xxs-sm`, the `1px solid var(--ac-stroke-soft)`
  border, retains `code:not(pre code)`, and that the td rule consumes
  `--size-eyebrow` via `td code:not(pre code)`.
- The ADR-0036 categorical font-size ban (`migration.test.ts:565-612`), the
  ADR-0026 `EXCEPTIONS` hygiene check (`:431-452`), and the
  `var()`-resolves-to-declared-token test (`:344-376`) act as guardrails — the
  ledger edits (Phase 1) must keep all three green, and the last one confirms
  the td rule's `var(--size-eyebrow)` resolves to a declared token.
- **AC5 var() ratchet** (`migration.test.ts:378-410`): the change is net **+1**
  `var()` reference (Phase 1's base-rule swap is var-count-neutral; Phase 2's
  `td` rule adds `var(--size-eyebrow)`). Both ratchet tests stay green at the
  current `AC5_FLOOR = 426`, but the documented bump protocol (`:382`) requires
  bumping `AC5_FLOOR` upward to the new observed count (expected `427`) in the
  same commit that adds the reference — do this in Phase 2.

### Integration / e2e (Playwright, real cascade — `mise run test:e2e:visualiser`):

- `inline-code-resolved-styles.spec.ts` (new): prose pill (font-family,
  font-size, background, border, radius, padding) in light + dark; td=11px;
  th=11.5px; fenced `pre code` unaffected.
- Token colours resolved to concrete rgb via the throwaway-element pattern from
  `root-resolved-tokens.spec.ts:71-78`.
- For fast iteration on one spec locally:
  `npm --prefix …/frontend run test:e2e -- inline-code-resolved-styles`
  (requires the built server binary + `ACCELERATOR_VISUALISER_BIN`, which
  `mise run test:e2e:visualiser` provisions).

### Key edge cases:

- Specificity: `.markdown td code:not(pre code)` (0,1,4) must beat the base
  (0,1,3) — the td=11px assertion fails first if this regresses.
- `[data-font="mono"]` mode repoints `--ac-font-body` to `--ac-font-mono`, so
  the font-family distinction between prose and inline code collapses; the
  border, sunken background, and smaller size still distinguish inline code.
  The Phase 1 spec asserts the default font mode is active before the AC1
  prose-contrast check, so it cannot fail spuriously under `[data-font="mono"]`.
- Fixture `.first()` stability: prose paragraph with inline code is appended,
  not prepended, so existing typography cases keep targeting the original
  first `p`/`h1`.

## Performance Considerations

None. CSS-only change to a descendant selector plus one new low-specificity
rule; no JS, no token renames, no new tokens, no additional stylesheet imports.

## Migration Notes

No data or schema migration. Two test/code-coupled surfaces:

- **`EXCEPTIONS` ledger** in `migration.test.ts` (Phase 1): `1px` 5→7, remove
  `0.1rem`, add `3px` ×1 and `5px` ×1 for `MarkdownRenderer.module.css`. The
  reverse-hygiene test enforces these counts exactly.
- **AC5 floor** (Phase 2): bump `AC5_FLOOR` 426 → 427 (net +1 var reference from
  the td rule's `var(--size-eyebrow)`) per the ratchet bump protocol.
- **No token rename, no ADR change.** 0094 consumes existing tokens by their
  current names; ADR-0036 and ADR-0026 are untouched (the new `3px`/`5px`
  irreducible literals are recorded in the per-file `EXCEPTIONS` ledger, which
  is the mechanism ADR-0026 prescribes). The sub-14px scale's mixed naming is
  out of scope — see the separate **scale-remap initiative** below.

### Follow-on: scale-remap initiative (work item 0099)

Not part of 0094. Captured as **work item 0099**
(`meta/work/0099-remap-typography-size-scale-to-pure-numeric-tokens.md`); a
dedicated ADR (superseding ADR-0036) + plan will follow. It
will remap the typography scale to a **pure-numeric** scheme (e.g.
`--size-110` = 11px, `--size-115` = 11.5px), resolving the sub-14px band's
mixed naming (t-shirt tiers / semantic names / `-sm`-`-lg` tweens) in one
coherent pass. That migration renames `--size-eyebrow` (the token 0094
consumes) along with ~100 other consumer references frontend-wide; 0094 is
deliberately decoupled from it and needs no rework when it lands (the remap
simply updates 0094's `var(--size-eyebrow)` reference with the rest).

## References

- Work item: `meta/work/0094-inline-code-styling-in-meta-artifact-markdown.md`
- Follow-on (scale remap): `meta/work/0099-remap-typography-size-scale-to-pure-numeric-tokens.md`
- Research: `meta/research/codebase/2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown.md`
- Live rule: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:57-60`
- Renderer (no inline-code class): `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:29-44`
- Tokens: `skills/visualisation/visualise/frontend/src/styles/global.css:160-187` (convention comment + scale, incl. `--size-xxs-sm` = 11.5px, `--size-eyebrow` = 11px), `tokens.ts:146-175`
- Token ban + ledger + var-resolves test: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:46-69,344-376,431-452,455-471,557-612`
- Test models: `…/tests/visual-regression/typography-resolved-sizes.spec.ts`, `…/root-resolved-tokens.spec.ts:71-78`, `…/code-block-resolved-colours.spec.ts`
- Fixture: `skills/visualisation/visualise/server/tests/fixtures/meta/plans/2026-01-01-first-plan.md`
- mise tasks: `mise.toml` (`test:unit:frontend` :80-83, `test:e2e:visualiser` :142-144)
- Prototype: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html` (`.ac-md-code`)
- ADRs: `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md`, `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
</content>
