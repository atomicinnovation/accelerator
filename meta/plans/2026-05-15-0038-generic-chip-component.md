---
date: "2026-05-15T00:00:00+01:00"
type: plan
producer: create-plan
work_item_id: "0038"
status: done
id: "2026-05-15-0038-generic-chip-component"
title: "0038 — Generic Chip Component Implementation Plan"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-15T00:00:00+01:00"
last_updated_by: Toby Clemson
revision: "4a4febd1f1ac"
repository: "ticket-management"
relates_to: ["work-item:0038", "codebase-research:2026-05-14-0038-generic-chip-component", "plan:2026-05-12-0037-glyph-component", "work-item:0033", "adr:ADR-0026"]
---

# 0038 — Generic Chip Component Implementation Plan

## Overview

Introduce a generic `<Chip>` component to the visualiser frontend and use it to replace every open-coded status-pill element across the lifecycle, library, templates, and kanban surfaces. Also restyle the existing `FrontmatterChips` component to render each frontmatter value as a `Chip` (status colour-coded, other values neutral) — matching the prototype's per-document page-subtitle treatment.

The work is delivered test-first across seven phases: a token extension (dark feedback colours plus new `--size-chip` / `--size-chip-md` typography tokens), the Chip primitive itself, a shared status→variant mapping utility (located at `src/api/status-variant.ts`, outside the Chip folder), the FrontmatterChips restyle, surface migrations (including a new shared `PageSubtitle` component used by the kanban route), a developer showcase route with Playwright visual regression, and cleanup. The plan ships full status colour-coding now (per user direction), not deferring it to 0040/0041/0042.

## Current State Analysis

### Tokens (0033 — shipped)

The `--ac-*` token layer is the single source of truth at `skills/visualisation/visualise/frontend/src/styles/global.css:69-251` with TypeScript parity at `src/styles/tokens.ts`. Theme switching uses `[data-theme="dark"]` overrides plus a byte-identical `@media (prefers-color-scheme: dark)` mirror. Parity is enforced by `src/styles/global.test.ts`.

Semantic feedback tokens are present but **light-only** at `global.css:90-93`:

```css
--ac-ok:     #2e8b57;
--ac-warn:   #d98f2e;
--ac-err:    #cb4647;
--ac-violet: #7b5cd9;
```

The dark `[data-theme="dark"]` block (`global.css:169-207`) does not redefine these — they cascade unchanged into dark mode. Acceptance criterion 3 ("Chip variant colours swap correctly between light and dark theme") requires fixing this.

### No existing Chip primitive

A repo-wide search for `.ac-chip` or `<Chip` returns zero results. The closest analogues are:

- `src/components/OriginPill/` — token-driven pill with pulsing dot and `prefers-reduced-motion` safeguard. Defines `ac-pulse` keyframes.
- `src/components/SseIndicator/` — canonical `data-state`-driven variant template, with the Vitest `?raw` CSS-source assertion pattern.
- `src/components/FrontmatterChips/` — renders `<dl>` of key/value pairs as rounded **rectangles** (`--radius-sm`), not pills. No colour variants.

### Open-coded status pills today (the migration surface)

| Surface | File:line | Element | Coloured? | Migration target variant |
|---|---|---|---|---|
| Lifecycle | `LifecycleClusterView.tsx:125` (`.statusBadge`, `LifecycleClusterView.module.css:95`) | pill, `--ac-stroke-soft` / `--ac-fg` | neutral only | colour-coded by status |
| Library by type | `LibraryTypeView.tsx:123` (`.badge`, `LibraryTypeView.module.css:26`) | pill, `--ac-stroke-soft` / `--ac-fg` | neutral only | colour-coded by status (with neutral fallback for dates) |
| Templates view (active) | `LibraryTemplatesView.tsx:60` (`.activeBadge`, `LibraryTemplatesView.module.css:8`) | pill, `--ac-accent-tint` / `--ac-accent` | indigo | `indigo` |
| Templates view (absent) | (currently no chip — only `.panel.absent` opacity) | — | — | introduce `<Chip variant="neutral">absent</Chip>` |
| Templates index | `LibraryTemplatesIndex.tsx:38` (`.active` text, `.module.css:5`) | plain text, not a pill | — | promote to `<Chip variant="neutral">` |
| Kanban page subtitle | not present today | — | — | introduce `<Chip variant="indigo">live</Chip>` (matches prototype) |

The kanban **work-item card** (`WorkItemCard.tsx`) deliberately renders no status chip — the column is the status. Out of scope.

### Prototype CSS — extracted

Captured from the live prototype (`Accelerator Visualiser _standalone_.html`, served locally and crawled). Verbatim base rule:

```css
.ac-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 999px;
  font-family: var(--ac-font-mono);
  font-size: 10.5px;
  font-weight: 500;
  letter-spacing: 0.02em;
  border: 1px solid var(--ac-stroke);
  background: var(--ac-bg-raised);
  color: var(--ac-fg-muted);
  white-space: nowrap;
}
.ac-chip--md { padding: 3px 10px; font-size: 11.5px; }
```

Critical correction to the work item: the prototype's size modifier is `--md` (larger), not `--sm`. The base `.ac-chip` IS the small chip. The Chip component will expose `size: 'sm' | 'md'` with default `'sm'`.

Six colour variants exist (the work item enumerated four). Verbatim:

```css
.ac-chip--neutral { /* inherits base */ }
.ac-chip--indigo  { color: var(--ac-accent);    background: var(--ac-accent-faint);  border-color: var(--ac-accent-tint); }
.ac-chip--green   { color: rgb(46,139,87);      background: rgba(46,139,87,0.08);    border-color: rgba(46,139,87,0.22); }
.ac-chip--amber   { color: rgb(180,118,31);     background: rgba(217,143,46,0.1);    border-color: rgba(217,143,46,0.25); }
.ac-chip--red     { color: var(--ac-accent-2);  background: rgba(203,70,71,0.08);    border-color: rgba(203,70,71,0.22); }
.ac-chip--violet  { color: var(--ac-violet);    background: rgba(123,92,217,0.08);   border-color: rgba(123,92,217,0.22); }

[data-theme="dark"] .ac-chip--green { color: rgb(121,217,166); background: rgba(121,217,166,0.1); border-color: rgba(121,217,166,0.22); }
[data-theme="dark"] .ac-chip--amber { color: rgb(228,183,110); background: rgba(228,183,110,0.1); border-color: rgba(228,183,110,0.22); }
```

Resolved hex values per theme (probed from `getComputedStyle`):

| Variant | Light fg | Light bg | Light border | Dark fg | Dark bg | Dark border |
|---|---|---|---|---|---|---|
| neutral | `#5F6378` (--ac-fg-muted) | `#FFFFFF` (--ac-bg-raised) | `--ac-stroke` | `#A0A5B8` | `#0E0F19` | `--ac-stroke` |
| indigo | `#595FC8` (--ac-accent) | `--ac-accent-faint` | `--ac-accent-tint` | `#8A90E8` | `--ac-accent-faint` (dark) | `--ac-accent-tint` (dark) |
| green | `#2E8B57` (--ac-ok) | rgba(46,139,87,0.08) | rgba(46,139,87,0.22) | `#79D9A6` | rgba(121,217,166,0.1) | rgba(121,217,166,0.22) |
| amber | `#B4761F` | rgba(217,143,46,0.1) | rgba(217,143,46,0.25) | `#E4B76E` | rgba(228,183,110,0.1) | rgba(228,183,110,0.22) |
| red | `#CB4647` (--ac-accent-2) | rgba(203,70,71,0.08) | rgba(203,70,71,0.22) | `#E86A6B` (--ac-accent-2 dark) | rgba(203,70,71,0.08) | rgba(203,70,71,0.22) |
| violet | `#7B5CD9` (--ac-violet) | rgba(123,92,217,0.08) | rgba(123,92,217,0.22) | `#7B5CD9` (unchanged) | rgba(123,92,217,0.08) | rgba(123,92,217,0.22) |

Sample markup verified in the prototype:

```html
<span class="ac-chip ac-chip--indigo ac-chip--sm">In progress</span>
<span class="ac-chip ac-chip--green ac-chip--sm">Accepted</span>
<span class="ac-chip ac-chip--amber ac-chip--sm">Approve w/ changes</span>
<span class="ac-chip ac-chip--neutral ac-chip--sm">2026-04-05</span>
<span class="ac-chip ac-chip--neutral ac-chip--sm">Toby Clemson</span>
```

The prototype's React-like template is also visible: `<span className={\`ac-chip ac-chip--${tone} ac-chip--${size}\`}>{children}</span>`.

The prototype's `.ac-pulse` element is documented but **does not nest inside chips** — it lives in the topbar status / sidebar nav. No chip in the prototype contains a leading pulse, SVG, or other glyph. Per user decision, the Chip API will still expose an optional `leading` slot for future use.

### Status→variant mapping evidence

Observed pairings across kanban, lifecycle, decisions, plan reviews, templates:

| Status string | Variant | Source surface |
|---|---|---|
| `In progress` | indigo | page subtitle, lifecycle entries |
| `live` | indigo | kanban page subtitle |
| `Proposed` | indigo | decisions list |
| `active` | indigo | templates panel |
| `Done` | green | lifecycle |
| `Accepted` | green | decisions, decision detail page |
| `Todo` | neutral | lifecycle |
| `absent` | neutral | templates panel |
| date strings (e.g. `2026-04-05`) | neutral | document detail page |
| author names | neutral | document detail page |
| `Approve w/ changes` | amber | plan reviews |

`red` and `violet` variants exist in the prototype CSS but were not observed in any rendered view. They are kept as exposed variants for completeness (blocked/rejected/deprecated → red; reserved → violet).

### Frontmatter chip styling — confirmed

On the ADR detail page (`#/library/decisions/ADR-0006`), the page subtitle (`ac-pagehead__sub`) contains three flat chips with **no key labels**:

```html
<span class="ac-chip ac-chip--green   ac-chip--sm">Accepted</span>
<span class="ac-chip ac-chip--neutral ac-chip--sm">2026-04-05</span>
<span class="ac-chip ac-chip--neutral ac-chip--sm">Toby Clemson</span>
```

This confirms the design owner's direction: in the prototype, frontmatter chips ARE chips. They are flat, key-less, with the status field colour-coded and other values neutral. The current `<dl><dt>key</dt><dd>value</dd></dl>` structure does not match the prototype.

### Theming and test wiring

- Boot: `src/api/boot-theme.ts:27` reads `localStorage` and sets `data-theme` before React mounts.
- Runtime: `src/api/use-theme.ts` (owning hook `useTheme` in `RootLayout`; consumer hook `useThemeContext` elsewhere).
- Tests: `vitest run` via `npm test`. No `make` wrapper exists.
- Available scripts (`package.json:9-19`): `dev`, `build`, `typecheck`, `test`, `test:watch`, `test:coverage`, `test:e2e`, `test:e2e:ui`.

### Key constraints

- **No hardcoded hex** in CSS modules per ADR-0026 and `migration.test.ts:28-49`. Variant colours must resolve from tokens. The plan uses `color-mix()` against semantic tokens for the rgba backgrounds (precedent: `FrontmatterChips.module.css:9`).
- **Variant selection via `data-*` attributes**, not className concatenation (canonical: `SseIndicator`).
- **CSS-source `?raw` regex assertions** are mandatory for variant→token binding (canonical: `SseIndicator.test.tsx`).
- **Off-scale rem values** (`0.05rem`, `0.1rem`, `0.4rem`) currently used by existing pill rules must NOT be reintroduced — Chip uses `--sp-1` / `--sp-2` instead, or the prototype's pixel values converted to rem (`2px = 0.125rem`, `4px = 0.25rem`, `8px = 0.5rem`, `10px = 0.625rem`).

## Desired End State

After all phases land:

1. A `Chip` component exists at `src/components/Chip/` with six colour variants (`neutral` / `indigo` / `green` / `amber` / `red` / `violet`), two sizes (`sm` default, `md`), an optional `leading` slot, an optional `aria-label` prop, full keyboard- and screen-reader-safe rendering, and theme-correct light + dark visuals.
2. A `statusToChipVariant(status)` utility lives at `src/api/status-variant.ts` (outside the Chip folder, since it is a domain function consumed by routes) and maps known status strings to variants. Unknown / non-string values return `neutral`.
3. `FrontmatterChips` renders each frontmatter value as a flat `Chip` (no key labels), with status colour-coded and other fields neutral. Each chip carries an `aria-label` of the form `"${key}: ${value}"` so screen readers retain the key context the visual restyle removes. The malformed-frontmatter banner text is preserved verbatim.
4. The four open-coded pills (`.statusBadge`, `.badge`, `.activeBadge`, plus the `LibraryTemplatesIndex.active` text element) are replaced by `Chip`, and the dead CSS rules are removed in the same phase that performs the migration.
5. The templates view introduces a `<Chip variant="neutral">absent</Chip>` element for absent tiers (matching the prototype) alongside the existing `<Chip variant="indigo">active</Chip>`.
6. A new shared `PageSubtitle` component is extracted at `src/components/PageSubtitle/` and used on the kanban route to render a `<Chip variant="indigo">live</Chip>` indicator (matching the prototype's page-subtitle treatment). The component is structured so other routes can adopt it later without further refactoring.
7. `--ac-ok`, `--ac-warn`, `--ac-err` have dark-theme overrides; new `--size-chip` (10.5px) and `--size-chip-md` (11.5px) typography tokens exist (identical in light and dark); parity tests pass; `[data-theme="dark"]` and `@media (prefers-color-scheme: dark)` remain byte-identical mirrors.
8. A `/chip-showcase` developer route renders all 24 variant × size × theme cells (6 variants × 2 sizes × 2 themes); Playwright captures one screenshot per cell for visual regression.
9. `grep -r 'border-radius:\s*var(--radius-pill)' src/` returns only the Chip module, the column count, the sidebar unseen-dot, and the OriginPill (i.e. no remaining open-coded status pills).

## What We're NOT Doing

- **No icon nesting inside Chip in this work item.** The Chip API exposes a `leading` prop, but no consumer in scope renders an icon. The prototype confirms no chip nests `.ac-pulse` or an SVG. OriginPill keeps its own pulse implementation; that lift-into-Chip exercise is deferred.
- **No `--ac-violet` dark override.** The prototype keeps `--ac-violet` identical in both themes. Leave it alone.
- **No semantic `--ac-status-*` namespace.** Per user direction, we extend the existing `--ac-ok` / `--ac-warn` / `--ac-err` tokens with dark values; we do not introduce a parallel status namespace.
- **No changes to OriginPill, SseIndicator, the unseen-doc-type sidebar dot, or the kanban column count badge.** Those are existing pill-radius elements with distinct semantics; they remain as-is.
- **No `WorkItemCard` chip migration.** Kanban cards do not render a per-card status chip today (status is implied by column placement) and that stays out of scope for 0038.
- **No new spacing tokens.** Chip uses `--ac-font-mono`, existing `--sp-*` tokens, `--radius-pill`, and two new chip-specific typography tokens (`--size-chip`, `--size-chip-md`) introduced in Phase 1.
- **No widespread `PageSubtitle` migration.** Phase 5e extracts a shared `PageSubtitle` component and applies it on the kanban route only. Migrating other route headers to use it is tracked separately as a follow-up.
- **No `status` convenience prop on Chip.** Chip is intentionally a thin presentational primitive: `variant` is required, and the recommended call shape is `<Chip variant={statusToChipVariant(status)}>`. If consumer boilerplate grows untenable, introduce a separate `<StatusChip status={…}>` wrapper rather than expanding Chip's API.
- **No dev-mode warning for unknown statuses.** `statusToChipVariant` returns `'neutral'` silently for any unrecognised input — typos and new statuses are indistinguishable from legitimate non-status values (dates, author names). Adding a development `console.warn` was considered and deferred; the cost of reasoning about which warnings would fire across the test suite outweighed the diagnostic benefit at this stage.
- **No Storybook setup.** The developer showcase route under `/chip-showcase` follows the Glyph plan's precedent (Phase 4 of `meta/plans/2026-05-12-0037-glyph-component.md`).

## Implementation Approach

Each phase is test-first: failing tests are committed before the implementation. Phases are ordered to keep each commit independently shippable and reviewable:

- **Phase 1** (tokens) is upstream of everything else and is risk-isolated to `tokens.ts` plus its CSS mirror.
- **Phase 2** (Chip primitive) is self-contained — no other file in the repo imports it yet.
- **Phase 3** (status→variant mapping) is a pure function in `src/api/`, fully isolated from Chip internals.
- **Phase 4** (FrontmatterChips restyle) is the first consumer; it exercises Phases 1–3 end-to-end on every detail page.
- **Phase 5** (surface migrations) replaces the four open-coded pills, extracts a shared `PageSubtitle` component, and introduces the kanban-subtitle and templates-absent chips. Each sub-phase deletes its corresponding CSS rule in the same commit so no dead CSS is left in flight.
- **Phase 6** (showcase route + visual regression) follows the Glyph plan's structure and pins the rendered output for future regression.
- **Phase 7** (cleanup) finalises the migration ledger and adds a repo-wide pill-radius guard that prevents future regressions.

The plan does not gate phases behind one another in the literal "merge separately" sense — they can land in a single PR or be split, but each phase is committable in isolation and leaves the build green.

---

## Phase 1: Extend the token layer with dark-theme feedback colours and new chip typography tokens

### Overview

Add three sets of token changes:

1. **Dark-theme values for `--ac-ok` / `--ac-warn` / `--ac-err`** so the Chip's green / amber / red variants can theme correctly. The dark values are taken verbatim from the prototype's `[data-theme="dark"] .ac-chip--*` rules. `--ac-violet` is intentionally left light-only (the prototype keeps it identical in both themes), and the plan adds an explicit test asserting this invariant.
2. **New chip typography tokens `--size-chip` (10.5px) and `--size-chip-md` (11.5px)** sourced directly from the prototype's `.ac-chip` / `.ac-chip--md` font sizes. These are theme-invariant; they appear in both the light `:root` block and (identically) in the dark `[data-theme="dark"]` mirror so the parity tests cover them.
3. **All hex literals use lowercase** to match the existing convention in `tokens.ts` / `global.css`.

### Changes Required

#### 1. Add the new tokens to `tokens.ts`

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`

Verify `tokens.ts`'s structure at implementation start: the file currently exports `LIGHT_COLOR_TOKENS` (holding `ac-violet` and the light feedback values) and `DARK_COLOR_TOKENS`. Add the three new dark feedback entries to `DARK_COLOR_TOKENS` (alphabetised within the colour group):

```ts
export const DARK_COLOR_TOKENS = {
  // ...existing entries...
  'ac-ok':    '#79d9a6',
  'ac-warn':  '#e4b76e',
  'ac-err':   '#e86a6b',
} as const
```

Add the chip typography tokens to the existing `TYPOGRAPHY_TOKENS` map (or whichever map already holds the `--size-*` entries). They are theme-invariant, so they go alongside the existing size tokens — not in a dark override map:

```ts
export const TYPOGRAPHY_TOKENS = {
  // ...existing entries (size-hero, size-h1..h4, size-lg, size-body, size-md, size-sm, size-xs, size-xxs, ...)...
  'size-chip':    '10.5px',
  'size-chip-md': '11.5px',
} as const
```

Order: place the two chip tokens at the end of the size group (after `size-xxs`), since `size-chip` (10.5px) is smaller than `size-xxs` (12px) and the scale otherwise descends.

#### 2. Mirror the values in `global.css`

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`

In the `:root` block (around `global.css:69-167`), add the two new typography tokens alongside the existing `--size-*` declarations:

```css
--size-chip:    10.5px;
--size-chip-md: 11.5px;
```

In the `[data-theme="dark"]` block (around `global.css:169-207`), add the three feedback overrides:

```css
--ac-ok:    #79d9a6;
--ac-warn:  #e4b76e;
--ac-err:   #e86a6b;
```

Then add the same three declarations to the `@media (prefers-color-scheme: dark) :root:not([data-theme="light"])` block (around `global.css:213-251`). The `global.test.ts` MIRROR-A / MIRROR-B assertion (`global.test.ts:127`) requires byte-identical content; copy-paste exactly.

Do **not** add `--size-chip` / `--size-chip-md` to either dark block — they are theme-invariant and live in `:root` only. Do **not** add `--ac-violet` to either dark block.

#### 3. Lock the `--ac-violet` theme-invariance invariant

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`

Add a focused assertion:

```ts
it('--ac-violet has no dark-theme override (intentionally theme-invariant per prototype)', () => {
  expect(DARK_COLOR_TOKENS).not.toHaveProperty('ac-violet')
  // Sanity: the dark CSS block must not declare --ac-violet either
  expect(darkBlockText).not.toMatch(/--ac-violet\s*:/)
})
```

(Reuse the existing helper for reading the dark block text; mirror the pattern used by the existing parity tests.)

#### 4. Audit existing consumers for dark-mode regression

**Files**: `OriginPill.module.css`, `FrontmatterChips.module.css` (banner), `SseIndicator.module.css`

These transitively consume `--ac-ok` / `--ac-warn` / `--ac-err`. The dark values shift, so document the expected visual change for each — slightly brighter / more saturated tones in dark mode rather than the muted light values they previously inherited. No code changes required; capture the expected delta in the manual verification checklist.

### Success Criteria

#### Automated Verification

- [ ] `cd skills/visualisation/visualise/frontend && npm test src/styles/global.test.ts` passes (including the new `--ac-violet` theme-invariance assertion)
- [ ] `getComputedStyle(document.documentElement).getPropertyValue('--ac-ok')` returns `#79d9a6` when `data-theme="dark"`
- [ ] `getComputedStyle(document.documentElement).getPropertyValue('--size-chip')` returns `10.5px` in both themes
- [ ] `npm run typecheck` is clean

#### Manual Verification

- [ ] Open the visualiser in dark mode and confirm no functional regression in any existing surface (OriginPill, SseIndicator, FrontmatterChips banner). They will look slightly brighter — verify against a sibling light-mode tab side-by-side.
- [ ] Confirm `--ac-violet` rendering is unchanged in dark mode.

---

## Phase 2: Chip primitive (TDD)

### Overview

Implement the `Chip` component at `src/components/Chip/` following the canonical pattern: folder of `Chip.tsx` + `Chip.module.css` + `Chip.test.tsx`, named exports only, no `index.ts`. Variants via `data-variant`, sizes via `data-size`. The component is theme-passive — it binds to CSS custom properties; the theme cascade reaches it via `[data-theme="dark"]` automatically.

Three notable design choices:

1. **`color-mix()` composes against `var(--ac-bg)` using the locked `{8, 18, 30}` ladder** (per ADR-0026 and `migration.test.ts:233-249`). The prototype's verbatim `rgba(... 0.08 / 0.10 / 0.22 / 0.25)` over the page surface is approximated by `color-mix(in srgb, var(--ac-<sem>) {8|30}%, var(--ac-bg))`. Visual fidelity to the prototype is intentionally traded for token-discipline consistency; a side-by-side comparison in the manual verification step pins the acceptable visual delta.
2. **Typography uses the new `--size-chip` / `--size-chip-md` tokens** introduced in Phase 1. No font-size literals appear in the Chip CSS.
3. **`aria-label` is exposed as an optional prop** so consumers (especially `FrontmatterChips`) can attach key context for screen readers that the visual restyle removes.

### Consumer Contract

Downstream consumers (FrontmatterChips, lifecycle, library, templates, kanban PageSubtitle, and future 0040/0041/0042 consumers) must honour the following invariants:

1. Status colour-coding must route through `statusToChipVariant` — consumers must not hand-pick variants from raw status strings.
2. `children` must be inline-renderable: short text, a `<Glyph>`, or a single inline span. No block-level content or multi-line text.
3. Consumers must not pass a `className` (Chip does not expose one and that is deliberate — styling is owned by the primitive).
4. The `leading` slot is for compact decorations only (≤ 12px). Use `aria-hidden="true"` on purely decorative leading content.
5. When the chip conveys meaning beyond visible text (status colour-coding, kanban "live" indicator), pass `aria-label` so screen readers receive the full context. For chips that render a frontmatter value without its key (the FrontmatterChips pattern), the canonical aria-label shape is `` `${key}: ${visibleText}` ``. New consumers of this pattern should adopt the same shape so the screen-reader convention stays uniform across surfaces.

### Changes Required

#### 1. Write the test file first

**File**: `skills/visualisation/visualise/frontend/src/components/Chip/Chip.test.tsx`

The test file covers behaviour (render assertions), structure (`data-*` attributes), and CSS source contracts (variant→token bindings). Skeleton:

```tsx
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { Chip } from './Chip'
import chipCss from './Chip.module.css?raw'

const Q = `['"]` // quote-style-agnostic character class for attribute selectors

describe('Chip', () => {
  describe('rendering', () => {
    it('renders its children', () => {
      render(<Chip variant="neutral">Done</Chip>)
      expect(screen.getByText('Done')).toBeInTheDocument()
    })

    it('renders the leading slot before the children', () => {
      render(
        <Chip variant="indigo" leading={<span data-testid="lead" />}>live</Chip>,
      )
      const lead = screen.getByTestId('lead')
      const text = screen.getByText('live')
      expect(lead.compareDocumentPosition(text) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    })

    it.each([
      ['undefined', undefined],
      ['null', null],
      ['false', false],
    ] as const)('does not render a leading wrapper when leading=%s', (_label, value) => {
      const { container } = render(<Chip variant="neutral" leading={value as never}>x</Chip>)
      expect(container.querySelector('[data-slot="leading"]')).toBeNull()
    })

    it('forwards aria-label to the chip element', () => {
      const { container } = render(<Chip variant="green" aria-label="status: accepted">accepted</Chip>)
      expect(container.querySelector('[aria-label="status: accepted"]')).not.toBeNull()
    })
  })

  describe('variants', () => {
    it.each([
      ['neutral'], ['indigo'], ['green'], ['amber'], ['red'], ['violet'],
    ] as const)('renders variant=%s with the matching data-variant attribute', (variant) => {
      const { container } = render(<Chip variant={variant}>x</Chip>)
      expect(container.querySelector(`[data-variant="${variant}"]`)).not.toBeNull()
    })
  })

  describe('sizes', () => {
    it('defaults to size sm', () => {
      const { container } = render(<Chip variant="neutral">x</Chip>)
      expect(container.querySelector('[data-size="sm"]')).not.toBeNull()
    })

    it.each([['sm'], ['md']] as const)('renders size=%s with the matching data-size attribute', (size) => {
      const { container } = render(<Chip variant="neutral" size={size}>x</Chip>)
      expect(container.querySelector(`[data-size="${size}"]`)).not.toBeNull()
    })
  })

  describe('CSS source assertions', () => {
    // Quote-style-agnostic regex helpers — Prettier/stylelint may flip the quote style
    // and we don't want every assertion to break in that case.

    it('binds base font-family to --ac-font-mono', () => {
      expect(chipCss).toMatch(/\.chip\s*\{[^}]*font-family:\s*var\(--ac-font-mono\)/)
    })
    it('binds base border-radius to --radius-pill', () => {
      expect(chipCss).toMatch(/\.chip\s*\{[^}]*border-radius:\s*var\(--radius-pill\)/)
    })
    it('binds base font-size to --size-chip', () => {
      expect(chipCss).toMatch(/\.chip\s*\{[^}]*font-size:\s*var\(--size-chip\)/)
    })

    it(`[data-variant=…neutral…] binds color to --ac-fg-muted`, () => {
      expect(chipCss).toMatch(new RegExp(`\\[data-variant=${Q}neutral${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-fg-muted\\)`))
    })
    it('[data-variant=indigo] binds color to --ac-accent', () => {
      expect(chipCss).toMatch(new RegExp(`\\[data-variant=${Q}indigo${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-accent\\)`))
    })
    it('[data-variant=green] binds color to --ac-ok', () => {
      expect(chipCss).toMatch(new RegExp(`\\[data-variant=${Q}green${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-ok\\)`))
    })
    it('[data-variant=amber] binds color to --ac-warn', () => {
      expect(chipCss).toMatch(new RegExp(`\\[data-variant=${Q}amber${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-warn\\)`))
    })
    it('[data-variant=red] binds color to --ac-err', () => {
      expect(chipCss).toMatch(new RegExp(`\\[data-variant=${Q}red${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-err\\)`))
    })
    it('[data-variant=violet] binds color to --ac-violet', () => {
      expect(chipCss).toMatch(new RegExp(`\\[data-variant=${Q}violet${Q}\\][^{]*\\{[^}]*color:\\s*var\\(--ac-violet\\)`))
    })

    // Each colour-coded variant must compose its background against var(--ac-bg)
    // at 8% and its border at 30%, matching the locked ladder Chip emits.
    it.each([
      ['green',  '--ac-ok'],
      ['amber',  '--ac-warn'],
      ['red',    '--ac-err'],
      ['violet', '--ac-violet'],
    ] as const)('[data-variant=%s] background composes at 8%% against --ac-bg', (variant, token) => {
      expect(chipCss).toMatch(new RegExp(
        `\\[data-variant=${Q}${variant}${Q}\\][^{]*\\{[^}]*background:\\s*color-mix\\(\\s*in\\s+srgb\\s*,\\s*var\\(\\${token}\\)\\s+8%\\s*,\\s*var\\(--ac-bg\\)\\s*\\)`,
      ))
    })

    it.each([
      ['green',  '--ac-ok'],
      ['amber',  '--ac-warn'],
      ['red',    '--ac-err'],
      ['violet', '--ac-violet'],
    ] as const)('[data-variant=%s] border-color composes at 30%% against --ac-bg', (variant, token) => {
      expect(chipCss).toMatch(new RegExp(
        `\\[data-variant=${Q}${variant}${Q}\\][^{]*\\{[^}]*border-color:\\s*color-mix\\(\\s*in\\s+srgb\\s*,\\s*var\\(\\${token}\\)\\s+30%\\s*,\\s*var\\(--ac-bg\\)\\s*\\)`,
      ))
    })

    it('[data-size=md] overrides padding and font-size', () => {
      expect(chipCss).toMatch(new RegExp(`\\[data-size=${Q}md${Q}\\][^{]*\\{[^}]*padding:`))
      expect(chipCss).toMatch(new RegExp(`\\[data-size=${Q}md${Q}\\][^{]*\\{[^}]*font-size:\\s*var\\(--size-chip-md\\)`))
    })

    // Per-Chip hex check intentionally omitted — migration.test.ts owns the
    // repo-wide hex prohibition and is the single source of truth.
  })
})
```

Note: the test file imports the raw CSS via `?raw`, so CSS-Modules class-name hashing does not affect the assertions — the regex matches the source text, not the rendered DOM.

#### 2. Implement the component

**File**: `skills/visualisation/visualise/frontend/src/components/Chip/Chip.tsx`

```tsx
import type { ReactNode } from 'react'
import styles from './Chip.module.css'

/**
 * Chip colour variants with prescriptive semantic intent. Status-bearing
 * consumers must route through `statusToChipVariant` rather than picking
 * a variant literal directly.
 *
 * - `neutral` — default; non-status metadata (dates, names, generic labels)
 * - `indigo`  — in-flight / currently active states
 * - `green`   — terminal success states
 * - `amber`   — needs human attention (review, revision)
 * - `red`     — blocked / withdrawn / terminal failure
 * - `violet`  — reserved for future use; no current consumer
 */
export type ChipVariant = 'neutral' | 'indigo' | 'green' | 'amber' | 'red' | 'violet'

export type ChipSize = 'sm' | 'md'

export interface ChipProps {
  variant: ChipVariant
  /** Default `'sm'` matches the prototype's base chip. Use `'md'` for emphasis. */
  size?: ChipSize
  /**
   * Optional compact decoration rendered before the children. Should be ≤ 12px
   * (e.g. a pulse dot, a small Glyph). Use `aria-hidden="true"` on purely
   * decorative content.
   */
  leading?: ReactNode
  /**
   * Optional accessible label. Consumers must pass this when the chip conveys
   * meaning beyond its visible text (e.g. status colour-coding, the kanban
   * "live" indicator). FrontmatterChips uses `${key}: ${value}`.
   */
  'aria-label'?: string
  children: ReactNode
}

export function Chip({ variant, size = 'sm', leading, children, ...rest }: ChipProps) {
  const hasLeading = leading !== undefined && leading !== null && leading !== false
  return (
    <span
      className={styles.chip}
      data-variant={variant}
      data-size={size}
      aria-label={rest['aria-label']}
    >
      {hasLeading && (
        <span className={styles.leading} data-slot="leading">{leading}</span>
      )}
      <span className={styles.label}>{children}</span>
    </span>
  )
}
```

Notes:
- The `data-slot="leading"` attribute lets tests detect the slot wrapper without coupling to a CSS-module class name (which would be hashed at build time).
- The `hasLeading` guard collapses `null` / `false` / `undefined` to "no slot rendered" — important because React's common conditional pattern `condition && <Icon />` produces `false`, which the previous `!== undefined` guard would have rendered as an empty wrapper.

#### 3. Implement the CSS module

**File**: `skills/visualisation/visualise/frontend/src/components/Chip/Chip.module.css`

Use token-bound values throughout. Backgrounds and borders for green / amber / red / violet use `color-mix()` against `var(--ac-bg)` using the locked `{8, 18, 30}%` ladder (per ADR-0026 and `migration.test.ts:233-249`). For indigo, use the existing `--ac-accent-faint` / `--ac-accent-tint` tokens directly (already dark-aware). Typography uses the new `--size-chip` and `--size-chip-md` tokens introduced in Phase 1.

Padding: the prototype's `2px 8px` and `3px 10px` cannot be expressed cleanly on the `--sp-*` scale (2px and 3px sit below the `--sp-1` 4px floor). Following the Sidebar / ActivityFeed precedent, these are added to the `EXCEPTIONS` ledger with a reason noting they are sub-floor and prototype-derived. The 10px horizontal padding on `md` substitutes to `var(--sp-3)` (12px, within the ±2px tolerance band).

```css
.chip {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-1); /* 4px */
  padding: 0.125rem var(--sp-2); /* 2px / 8px — 2px vertical is below --sp-1 floor, see ledger */
  border-radius: var(--radius-pill);
  border: 1px solid var(--ac-stroke);
  font-family: var(--ac-font-mono);
  font-size: var(--size-chip);
  font-weight: 500;
  letter-spacing: 0.02em; /* prototype-derived — see ledger */
  white-space: nowrap;
  background: var(--ac-bg-raised);
  color: var(--ac-fg-muted);
}

.chip[data-size='md'] {
  padding: 0.1875rem var(--sp-3); /* 3px vertical (sub-floor, see ledger) / 12px horizontal */
  font-size: var(--size-chip-md);
}

.label { display: inline; }
.leading { display: inline-flex; align-items: center; }

.chip[data-variant='neutral'] {
  /* inherits base */
  color: var(--ac-fg-muted);
}
.chip[data-variant='indigo'] {
  color: var(--ac-accent);
  background: var(--ac-accent-faint);
  border-color: var(--ac-accent-tint);
}
.chip[data-variant='green'] {
  color: var(--ac-ok);
  background: color-mix(in srgb, var(--ac-ok) 8%, var(--ac-bg));
  border-color: color-mix(in srgb, var(--ac-ok) 30%, var(--ac-bg));
}
.chip[data-variant='amber'] {
  color: var(--ac-warn);
  background: color-mix(in srgb, var(--ac-warn) 8%, var(--ac-bg));
  border-color: color-mix(in srgb, var(--ac-warn) 30%, var(--ac-bg));
}
.chip[data-variant='red'] {
  color: var(--ac-err);
  background: color-mix(in srgb, var(--ac-err) 8%, var(--ac-bg));
  border-color: color-mix(in srgb, var(--ac-err) 30%, var(--ac-bg));
}
.chip[data-variant='violet'] {
  color: var(--ac-violet);
  background: color-mix(in srgb, var(--ac-violet) 8%, var(--ac-bg));
  border-color: color-mix(in srgb, var(--ac-violet) 30%, var(--ac-bg));
}
```

Note on the visual delta from the prototype: the prototype mixes against `transparent` (so the chip is semi-transparent over whatever surface is behind it); this plan mixes against `var(--ac-bg)` (the chip background is opaque, matching the page background tinted by the semantic colour). The visual difference is small on white pages (the prototype's default surface) and is the deliberate tradeoff for token-discipline consistency. The visual-regression snapshots in Phase 6 lock the resulting appearance.

#### 4. Update the migration ledger

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

Add three entries to `EXCEPTIONS`, matching the shape used elsewhere in the file (`file` is relative to `src/`, with no `src/` prefix; `count` and `kind` are both required — the array's type is `ReadonlyArray<Exception & { kind: 'to-migrate' | 'irreducible' }>`):

```ts
{
  file: 'components/Chip/Chip.module.css',
  literal: '0.125rem',
  count: 1,
  kind: 'irreducible',
  reason: 'chip base vertical padding — 2px, below --sp-1 (4px) floor; prototype-derived',
},
{
  file: 'components/Chip/Chip.module.css',
  literal: '0.1875rem',
  count: 1,
  kind: 'irreducible',
  reason: 'chip md vertical padding — 3px, below --sp-1 (4px) floor; prototype-derived',
},
{
  file: 'components/Chip/Chip.module.css',
  literal: '0.02em',
  count: 1,
  kind: 'irreducible',
  reason: 'chip letter-spacing — prototype-derived typography refinement',
},
```

Notes:
- The 10px horizontal padding on `md` substitutes to `var(--sp-3)` and needs no exception.
- The 4px gap is `var(--sp-1)` (no literal, no exception).
- If any of the literals appear more than once in the Chip module, increment the matching `count` to reflect the actual occurrence count.

### Success Criteria

#### Automated Verification

- [ ] All Chip.test.tsx tests pass: `cd skills/visualisation/visualise/frontend && npm test src/components/Chip/`
- [ ] `npm run typecheck` clean
- [ ] `npm run build` succeeds
- [ ] `npm test src/styles/migration.test.ts` passes (no unexpected hex / off-scale rem)

#### Manual Verification

- [ ] Render a temporary `<Chip>` instance in `App.tsx` or a scratch route, view it in both light and dark themes, confirm visual parity with the prototype (use the downloaded `Accelerator Visualiser _standalone_.html` for side-by-side comparison).

---

## Phase 3: status→variant mapping utility (TDD)

### Overview

A pure function `statusToChipVariant(value: unknown): ChipVariant` that maps observed status strings to Chip variants. Consumers across lifecycle, library, templates, and frontmatter all need the same mapping; centralising it prevents drift.

**Location**: `src/api/status-variant.ts`, not inside `src/components/Chip/`. The mapping is a domain function (document status vocabulary → chip variant) consumed by routes that have no other reason to depend on Chip internals. The component folder convention reserves `src/components/Foo/` for `Foo.tsx` + `Foo.module.css` + `Foo.test.tsx`; putting a non-component file inside it breaks that convention. Co-locating with the existing `src/api/` helpers (`boot-theme.ts`, `use-doc-events.ts`) preserves the established "presentation in components/, domain helpers in api/" split.

### Changes Required

#### 1. Write the test first

**File**: `skills/visualisation/visualise/frontend/src/api/status-variant.test.ts`

```ts
import { describe, expect, it } from 'vitest'
import { statusToChipVariant, isStatusKey, __SETS_FOR_TEST } from './status-variant'

describe('statusToChipVariant', () => {
  describe('green (terminal success)', () => {
    it.each([
      'done', 'complete', 'accepted', 'approved', 'implemented', 'final', 'shipped',
    ])('maps %s → green', (s) => {
      expect(statusToChipVariant(s)).toBe('green')
    })
  })

  describe('indigo (in-flight / active)', () => {
    it.each(['in-progress', 'in_progress', 'reviewed', 'ready', 'active', 'proposed', 'live'])(
      'maps %s → indigo',
      (s) => expect(statusToChipVariant(s)).toBe('indigo'),
    )
  })

  describe('amber (needs attention)', () => {
    it.each(['approve-with-changes', 'approve w/ changes', 'Approve w/ changes', 'review', 'revised'])(
      'maps %s → amber',
      (s) => expect(statusToChipVariant(s)).toBe('amber'),
    )
  })

  describe('red (blocked / terminal failure)', () => {
    it.each(['blocked', 'rejected', 'deprecated', 'superseded', 'abandoned'])(
      'maps %s → red',
      (s) => expect(statusToChipVariant(s)).toBe('red'),
    )
  })

  describe('neutral (default)', () => {
    it.each(['draft', 'todo', 'absent'])('maps %s → neutral', (s) => {
      expect(statusToChipVariant(s)).toBe('neutral')
    })

    it('returns neutral for unknown strings', () => {
      expect(statusToChipVariant('whatever')).toBe('neutral')
    })

    it('returns neutral for ISO date strings (fallback used by LibraryTypeView)', () => {
      expect(statusToChipVariant('2026-04-05')).toBe('neutral')
    })

    it('returns neutral for undefined / null / empty / non-string', () => {
      expect(statusToChipVariant(undefined)).toBe('neutral')
      expect(statusToChipVariant(null)).toBe('neutral')
      expect(statusToChipVariant('')).toBe('neutral')
      expect(statusToChipVariant(42)).toBe('neutral')
      expect(statusToChipVariant(true)).toBe('neutral')
      expect(statusToChipVariant(['accepted'])).toBe('neutral')
      expect(statusToChipVariant({ status: 'accepted' })).toBe('neutral')
    })
  })

  describe('case and separator insensitivity', () => {
    it('maps "Accepted" (capitalised) → green', () => {
      expect(statusToChipVariant('Accepted')).toBe('green')
    })
    it('maps "  In Progress  " → indigo', () => {
      expect(statusToChipVariant('  In Progress  ')).toBe('indigo')
    })
    it('treats hyphen, space, underscore, and slash equivalently', () => {
      expect(statusToChipVariant('in progress')).toBe('indigo')
      expect(statusToChipVariant('in_progress')).toBe('indigo')
      expect(statusToChipVariant('in-progress')).toBe('indigo')
      expect(statusToChipVariant('approve w/ changes')).toBe('amber')
    })
  })

  describe('internal invariants', () => {
    it('all Set keys are separator-free lowercase (the normalised form)', () => {
      expect(__SETS_FOR_TEST).toBeDefined()
      expect(__SETS_FOR_TEST.length).toBeGreaterThan(0)
      for (const s of __SETS_FOR_TEST) {
        expect(s.size).toBeGreaterThan(0)
        for (const k of s) {
          expect(k).toMatch(/^[a-z]+$/)
        }
      }
    })
  })
})

describe('isStatusKey', () => {
  it.each(['status', 'Status', 'STATUS', '  status  '])('returns true for %s', (k) => {
    expect(isStatusKey(k)).toBe(true)
  })
  it.each(['state', 'lifecycle-status', 'StatusX', ''])('returns false for %s', (k) => {
    expect(isStatusKey(k)).toBe(false)
  })
})
```

The `__SETS_FOR_TEST` symbol is imported statically alongside `statusToChipVariant` and `isStatusKey` at the top of the test file. Static import means a missing or renamed export becomes a TypeScript compilation error rather than a silent test no-op. The `expect(__SETS_FOR_TEST.length).toBeGreaterThan(0)` and `expect(s.size).toBeGreaterThan(0)` guards also catch the degenerate case where the export exists but is empty.

#### 2. Implement the function

**File**: `skills/visualisation/visualise/frontend/src/api/status-variant.ts`

```ts
import type { ChipVariant } from '../components/Chip/Chip'

// Semantic buckets for the status taxonomy.
//
//   green  — terminal success states
//   indigo — in-flight / currently active states
//   amber  — needs human attention (review, revision, blocked-on-input)
//   red    — terminal failure / blocked / withdrawn states
//   (anything else falls through to neutral)
//
// Set keys MUST be in normalised form: lowercased and with all separators
// stripped (spaces, hyphens, underscores, slashes). The normalise() function
// below produces this form. When adding a new status, normalise it first.
const GREEN = new Set(['done', 'complete', 'accepted', 'approved', 'implemented', 'final', 'shipped'])
const INDIGO = new Set(['inprogress', 'reviewed', 'ready', 'active', 'proposed', 'live'])
// 'approvewchanges' is the normalised form of the literal observed string
// 'Approve w/ changes' (slash + 'w' shorthand collapses to 'wchanges').
const AMBER = new Set(['approvewithchanges', 'approvewchanges', 'review', 'revised'])
const RED = new Set(['blocked', 'rejected', 'deprecated', 'superseded', 'abandoned'])

/** Internal: exported only for tests asserting Set-key shape. */
export const __SETS_FOR_TEST = [GREEN, INDIGO, AMBER, RED]

function normalise(value: unknown): string {
  if (typeof value !== 'string') return ''
  return value.trim().toLowerCase().replace(/[\s_\-/]+/g, '')
}

/**
 * Maps a document status string to a Chip variant.
 *
 * Inputs are normalised: lowercased, trimmed, and stripped of spaces,
 * hyphens, underscores, and slashes. Non-string inputs (and unknown
 * strings, including ISO dates and author names) return `'neutral'`.
 */
export function statusToChipVariant(value: unknown): ChipVariant {
  const key = normalise(value)
  if (GREEN.has(key)) return 'green'
  if (INDIGO.has(key)) return 'indigo'
  if (AMBER.has(key)) return 'amber'
  if (RED.has(key)) return 'red'
  return 'neutral'
}

/**
 * Predicate: does a frontmatter key name the document status field?
 *
 * Case- and whitespace-tolerant: `'status'`, `'Status'`, `' status '` all
 * match. Co-located with `statusToChipVariant` so the "what counts as a
 * status field?" and "what statuses map to which variant?" decisions live
 * in one module.
 */
export function isStatusKey(key: string): boolean {
  return key.trim().toLowerCase() === 'status'
}
```

The normaliser strips `/` in addition to spaces / hyphens / underscores, so the production string `"Approve w/ changes"` (observed in plan reviews) correctly maps to amber.

### Success Criteria

#### Automated Verification

- [ ] All `src/api/status-variant.test.ts` cases pass
- [ ] `npm run typecheck` clean

#### Manual Verification

- [ ] None — this is a pure function with full test coverage.

---

## Phase 4: Restyle `FrontmatterChips` around `Chip`

### Overview

Match the prototype's per-document page-subtitle treatment: render each frontmatter value as a flat `Chip` with no key label. The `status` field (and synonym fields — see below) uses the colour-coded variant; every other field uses `neutral`. The malformed-frontmatter banner is unchanged.

### Field selection — which keys render as chips?

The current `FrontmatterChips` renders every non-null key. Per the prototype evidence (only `status` / `date` / `author` shown on the ADR detail page), we need to decide whether to keep rendering every key (with colour-coding only on `status`) or to allow-list a curated set.

**Decision**: keep rendering every non-null key (preserves the current discoverability behaviour), but only the `status` field (case-insensitive: `status`, `Status`, `STATUS`) maps via `statusToChipVariant` — every other key renders with `variant="neutral"`. This is the minimum behavioural change and keeps the migration risk-isolated. If product wants curation later, that is a follow-up.

### Accessibility

The visual restyle drops the `<dl>/<dt>/<dd>` semantic markup that previously gave screen readers the key context (a screen reader would read `"status, accepted"`). To compensate, every chip carries an `aria-label` of the form `"${key}: ${value}"` so the key context survives in the accessibility tree even when it disappears from the visual surface. This is asserted by tests.

The malformed-frontmatter banner text is preserved verbatim (existing copy is `"Frontmatter unparseable — showing raw content."`) — this is a visual restyle of the parsed state only.

### Changes Required

#### 1. Update the tests first

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx`

Add (or replace existing assertions with):

```tsx
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { FrontmatterChips } from './FrontmatterChips'
import css from './FrontmatterChips.module.css?raw'

describe('FrontmatterChips', () => {
  describe('parsed state', () => {
    it('renders a Chip for each non-null frontmatter value', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'accepted', date: '2026-04-05', author: 'Toby Clemson' }}
        />,
      )
      const chips = container.querySelectorAll('[data-variant]')
      expect(chips.length).toBe(3)
    })

    it('renders the status field with the colour-coded variant', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'accepted' }} />,
      )
      expect(container.querySelector('[data-variant="green"]')).not.toBeNull()
    })

    it('colour-codes status case-insensitively (Status, STATUS)', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ Status: 'accepted' }} />,
      )
      expect(container.querySelector('[data-variant="green"]')).not.toBeNull()
    })

    it('renders non-status fields with variant="neutral"', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ date: '2026-04-05' }} />,
      )
      expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
    })

    it('does not render keys (e.g. the literal text "status:") in visible content', () => {
      render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'accepted' }} />,
      )
      // Visible text contains only the value, not the key
      expect(screen.getByText('accepted')).toBeInTheDocument()
      // The key is recoverable via aria-label, not as visible text
      expect(screen.queryByText(/^status:/i)).toBeNull()
    })

    it('attaches an aria-label of "${key}: ${value}" to each chip', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'accepted', date: '2026-04-05' }}
        />,
      )
      expect(container.querySelector('[aria-label="status: accepted"]')).not.toBeNull()
      expect(container.querySelector('[aria-label="date: 2026-04-05"]')).not.toBeNull()
    })

    it('renders the value text', () => {
      render(<FrontmatterChips state="parsed" frontmatter={{ status: 'draft' }} />)
      expect(screen.getByText('draft')).toBeInTheDocument()
    })

    it('skips null and undefined values', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ status: 'draft', author: null, date: undefined } as Record<string, unknown>}
        />,
      )
      const chips = container.querySelectorAll('[data-variant]')
      expect(chips.length).toBe(1)
    })

    it('skips empty-string values', () => {
      const { container } = render(
        <FrontmatterChips state="parsed" frontmatter={{ status: 'draft', author: '' }} />,
      )
      const chips = container.querySelectorAll('[data-variant]')
      expect(chips.length).toBe(1)
    })

    it('renders boolean and numeric values as strings', () => {
      render(
        <FrontmatterChips state="parsed" frontmatter={{ archived: false, version: 0 }} />,
      )
      expect(screen.getByText('false')).toBeInTheDocument()
      expect(screen.getByText('0')).toBeInTheDocument()
    })

    it('joins array values with ", " and reflects the joined text in the aria-label', () => {
      const { container } = render(
        <FrontmatterChips
          state="parsed"
          frontmatter={{ tags: ['design', 'frontend'] }}
        />,
      )
      expect(screen.getByText('design, frontend')).toBeInTheDocument()
      expect(container.querySelector('[aria-label="tags: design, frontend"]')).not.toBeNull()
    })
  })

  describe('absent state', () => {
    it('renders nothing', () => {
      const { container } = render(<FrontmatterChips state="absent" />)
      expect(container.firstChild).toBeNull()
    })
  })

  describe('malformed state', () => {
    it('renders the warning banner role="alert"', () => {
      render(<FrontmatterChips state="malformed" />)
      expect(screen.getByRole('alert')).toBeInTheDocument()
    })

    it('preserves the existing banner text verbatim', () => {
      render(<FrontmatterChips state="malformed" />)
      expect(screen.getByText(/Frontmatter unparseable/i)).toBeInTheDocument()
    })
  })

  describe('CSS source assertions', () => {
    it('no longer defines a .chip class (replaced by <Chip>)', () => {
      expect(css).not.toMatch(/\.chip\s*\{/)
    })
    it('still defines the .banner class for the malformed state', () => {
      expect(css).toMatch(/\.banner\s*\{/)
    })
  })
})
```

#### 2. Rewrite the component

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`

```tsx
import { Chip } from '../Chip/Chip'
import { statusToChipVariant, isStatusKey } from '../../api/status-variant'
import styles from './FrontmatterChips.module.css'

type FrontmatterChipsProps =
  | { state: 'absent' }
  | { state: 'malformed' }
  | { state: 'parsed'; frontmatter: Record<string, unknown> }

function formatChipValue(value: unknown): string {
  if (Array.isArray(value)) return value.join(', ')
  if (typeof value === 'object' && value !== null) return JSON.stringify(value)
  return String(value)
}

export function FrontmatterChips(props: FrontmatterChipsProps) {
  if (props.state === 'absent') return null
  if (props.state === 'malformed') {
    return (
      <div role="alert" className={styles.banner}>
        Frontmatter unparseable — showing raw content.
      </div>
    )
  }

  const entries = Object.entries(props.frontmatter).filter(([, v]) => {
    if (v === null || v === undefined) return false
    if (typeof v === 'string' && v === '') return false
    return true
  })

  return (
    <div className={styles.chips}>
      {entries.map(([key, value]) => {
        const text = formatChipValue(value)
        const variant = isStatusKey(key) ? statusToChipVariant(value) : 'neutral'
        return (
          <Chip key={key} variant={variant} aria-label={`${key}: ${text}`}>
            {text}
          </Chip>
        )
      })}
    </div>
  )
}
```

Key behavioural notes:
- The status-key check is case-insensitive and whitespace-tolerant, matching the input flexibility of `statusToChipVariant`.
- Empty-string values are skipped to avoid rendering empty bordered chips.
- Banner copy is preserved verbatim — no behavioural change for the malformed state.
- Every chip carries `aria-label="${key}: ${value}"` so the dropped `<dl>` semantics survive in the accessibility tree.

#### 3. Trim the CSS module

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.module.css`

Delete the `.chip`, `.key`, `.value` rules. Keep `.chips` (the layout container) and `.banner` (the malformed state). Result:

```css
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  margin: 0 0 var(--sp-4);
}

.banner {
  background: color-mix(in srgb, var(--ac-warn) 8%, var(--ac-bg));
  border: 1px solid var(--ac-warn);
  border-radius: var(--radius-sm);
  padding: var(--sp-2) var(--sp-3);
  font-size: var(--size-xs);
  margin-bottom: var(--sp-4);
}
```

### Success Criteria

#### Automated Verification

- [ ] `npm test src/components/FrontmatterChips/` passes
- [ ] `npm run typecheck` clean

#### Manual Verification

- [ ] Open any document detail page (a decision, a plan, a research doc); confirm frontmatter values render as flat chips with the status field colour-coded and other fields neutral. No key labels appear. Visual matches the prototype's `ac-pagehead__sub` section.
- [ ] Open a document with malformed frontmatter; confirm the banner still renders as before.
- [ ] Open a document with absent frontmatter; confirm nothing renders (no empty container).

---

## Phase 5: Migrate open-coded pills + introduce new chip sites

### Overview

Replace each of the four open-coded pills with `Chip`, route their status strings through `statusToChipVariant`, and introduce two new chip sites that the prototype shows but the current app lacks: the kanban page-subtitle "live" indicator and the templates panel "absent" chip.

Each Phase 5 sub-phase performs both the TSX migration AND the corresponding CSS rule deletion in the same commit — keeping each sub-commit self-contained with no dead CSS in flight. Phase 7 then handles only the migration-ledger updates and the repo-wide pill-radius guard.

**Test pattern for verifying migrations**: assertions about removed legacy classes must use raw-CSS source checks (`expect(moduleCss).not.toMatch(/\.statusBadge\b/)`), not DOM `querySelector('.statusBadge')` — CSS-Modules hash class names at build time so a literal selector never matches in the rendered DOM, before or after migration. The DOM-side assertions are positive (`[data-variant="green"]` is present).

### 5a. Lifecycle cluster — `.statusBadge`

#### Tests first

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleClusterView.test.tsx`

Extend the existing fixture helper to accept a `status` override so a new test can render a cluster entry with `frontmatter.status = 'done'` (the current fixture only carries `date`). Then add:

```tsx
import lifecycleCss from './LifecycleClusterView.module.css?raw'

it('renders the status as a Chip with the variant from statusToChipVariant', () => {
  const { container } = render(<LifecycleClusterView entries={[entryWithStatus('done')]} />)
  expect(container.querySelector('[data-variant="green"]')).not.toBeNull()
})

it('renders neutral chip for unknown status', () => {
  const { container } = render(<LifecycleClusterView entries={[entryWithStatus('mystery')]} />)
  expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
})

it('CSS module no longer defines the legacy .statusBadge rule', () => {
  expect(lifecycleCss).not.toMatch(/\.statusBadge\b/)
})
```

#### Implementation

**File**: `LifecycleClusterView.tsx:125`

Replace:

```tsx
{typeof status === 'string' && (
  <span className={styles.statusBadge}>{status}</span>
)}
```

With:

```tsx
{typeof status === 'string' && (
  <Chip variant={statusToChipVariant(status)}>{status}</Chip>
)}
```

Import: `import { Chip } from '../../components/Chip/Chip'` and `import { statusToChipVariant } from '../../api/status-variant'`.

In the same commit, delete the `.statusBadge` rule (and any unused colour variables tied to it) from `LifecycleClusterView.module.css:95-100`.

### 5b. Library by type — `.badge`

#### Tests first

**File**: `LibraryTypeView.test.tsx`

Extend the existing fixture helper to allow `frontmatter.status` overrides, then add:

```tsx
import typeViewCss from './LibraryTypeView.module.css?raw'

it('renders status cells as a coloured Chip when status is present', () => {
  const { container } = render(<LibraryTypeView entries={[entryWith({ status: 'accepted' })]} />)
  expect(container.querySelector('[data-variant="green"]')).not.toBeNull()
})

it('falls back to a neutral Chip when statusCellValue returns a date string', () => {
  const { container } = render(<LibraryTypeView entries={[entryWith({ date: '2026-04-05' })]} />)
  expect(container.querySelector('[data-variant="neutral"]')).not.toBeNull()
})

it('renders a plain em-dash (no chip) when statusCellValue is empty', () => {
  const { container } = render(<LibraryTypeView entries={[entryWith({})]} />)
  expect(screen.getByText('—')).toBeInTheDocument()
  // No chip at all for empty cells — matches the prototype's treatment
  expect(container.querySelectorAll('[data-variant]').length).toBe(0)
})

// Two cases pin both directions of the variant/label divergence:
// (a) status present → variant from status, label from statusCellValue (which prefers status)
// (b) status absent, only date → variant resolves to neutral via the mapper's date fallback,
//     while label is the date string surfaced by statusCellValue.
it('case (a): with status present, variant derives from frontmatter.status', () => {
  const { container } = render(
    <LibraryTypeView entries={[entryWith({ status: 'accepted', date: '2026-04-05' })]} />,
  )
  const chip = container.querySelector('[data-variant="green"]')
  expect(chip).not.toBeNull()
  expect(chip!.textContent).toBe('accepted')
})

it('case (b): with only a date, variant is neutral and label is the date', () => {
  // A swapped implementation that derived variant from statusCellValue's date
  // would still resolve to neutral (dates → neutral via the mapper), but a
  // correct implementation reads raw frontmatter.status (undefined → neutral).
  // The label assertion pins that the date reaches the chip — distinguishing
  // this case from one where the row collapses to an em-dash.
  const { container } = render(
    <LibraryTypeView entries={[entryWith({ date: '2026-04-05' })]} />,
  )
  const chip = container.querySelector('[data-variant="neutral"]')
  expect(chip).not.toBeNull()
  expect(chip!.textContent).toBe('2026-04-05')
})

it('CSS module no longer defines the legacy .badge rule', () => {
  expect(typeViewCss).not.toMatch(/\.badge\b/)
})
```

#### Implementation

**File**: `LibraryTypeView.tsx:123`

Extract a small helper so variant and label always pair correctly:

```tsx
function chipForEntry(entry: LibraryEntry): { label: string; variant: ChipVariant } | null {
  const label = statusCellValue(entry)
  if (!label) return null
  return { label, variant: statusToChipVariant(entry.frontmatter?.status) }
}
```

Then in the row template, hoist the helper call once so variant and label come from a single evaluation:

```tsx
const chip = chipForEntry(entry)
return (
  <td>
    {chip ? <Chip variant={chip.variant}>{chip.label}</Chip> : '—'}
  </td>
)
```

Notes:
- Variant selection uses raw `entry.frontmatter?.status`, not `statusCellValue(entry)`'s date fallback. This keeps non-status fallback values (dates) routing to `neutral` correctly.
- Empty cells render a plain em-dash with no chip, matching the prototype.
- Add a test pinning the variant/label divergence — an entry with `frontmatter.status = 'accepted'` whose `statusCellValue` would also return a non-status surrogate must still render `[data-variant="green"]` with the status as the label, not the surrogate.

In the same commit, delete the `.badge` rule from `LibraryTypeView.module.css:26-30`.

### 5c. Library templates view — `.activeBadge` + new absent chip

#### Tests first

**File**: `LibraryTemplatesView.test.tsx`

```tsx
import templatesCss from './LibraryTemplatesView.module.css?raw'

it('renders an indigo "active" Chip on the active tier panel', () => {
  const activePanel = within(getActivePanel())
  expect(activePanel.getByText('active').closest('[data-variant="indigo"]')).not.toBeNull()
})

it('renders a neutral "absent" Chip on absent tier panels', () => {
  const absentPanel = within(getAbsentPanel())
  expect(absentPanel.getByText('absent').closest('[data-variant="neutral"]')).not.toBeNull()
})

it('renders no chip on present, non-active tier panels', () => {
  const presentInactivePanel = within(getPresentInactivePanel())
  expect(presentInactivePanel.queryByText(/active|absent/i)).toBeNull()
})

it('CSS module no longer defines the legacy .activeBadge rule', () => {
  expect(templatesCss).not.toMatch(/\.activeBadge\b/)
})
```

#### Implementation

**File**: `LibraryTemplatesView.tsx:55-70`

Replace `TierPanel`'s chip section:

```tsx
<header className={styles.panelHeader}>
  <span className={styles.tierLabel}>{TIER_LABELS[tier.source] ?? tier.source}</span>
  {isActive && <Chip variant="indigo">active</Chip>}
  {!tier.present && <Chip variant="neutral">absent</Chip>}
  <code className={styles.path}>{tier.path}</code>
</header>
```

In the same commit, delete the `.activeBadge` rule from `LibraryTemplatesView.module.css:8-10`. Leave `.panel.absent` (the opacity dimming) untouched — it still serves a panel-level signal that complements the chip.

### 5d. Library templates index — promote `.active` text to a Chip

#### Tests first

**File**: `LibraryTemplatesIndex.test.tsx`

```tsx
import indexCss from './LibraryTemplatesIndex.module.css?raw'

it('renders the active-tier label as a neutral Chip per row', () => {
  const chip = screen.getByText('Plugin default').closest('[data-variant="neutral"]')
  expect(chip).not.toBeNull()
})

it('CSS module no longer defines the legacy .active text rule', () => {
  expect(indexCss).not.toMatch(/\.active\b/)
})
```

#### Implementation

**File**: `LibraryTemplatesIndex.tsx:38`

Replace:

```tsx
<span className={styles.active}>{TIER_LABELS[t.activeTier]}</span>
```

With:

```tsx
<Chip variant="neutral">{TIER_LABELS[t.activeTier]}</Chip>
```

In the same commit, delete the `.active` text rule from `LibraryTemplatesIndex.module.css:5`.

### 5e. Extract a shared `PageSubtitle` component and use it on the kanban route

The kanban route does not currently render a page subtitle (`KanbanBoard.tsx` only renders `<h1 className={styles.title}>Kanban</h1>` across three render branches: loading, error, and loaded). Per the prototype, the kanban page should carry a "live" chip in its subtitle. Rather than inline a chip into one of the three render branches, this phase introduces a small shared `PageSubtitle` component that other routes can adopt later.

**Scope note**: only the kanban route adopts `PageSubtitle` in this plan. Migrating other route headers is tracked separately.

#### New component

**File**: `skills/visualisation/visualise/frontend/src/components/PageSubtitle/PageSubtitle.tsx`

```tsx
import type { ReactNode } from 'react'
import styles from './PageSubtitle.module.css'

export interface PageSubtitleProps {
  /** The page title text. */
  title: string
  /** Optional inline content rendered alongside the title (typically a Chip). */
  children?: ReactNode
}

export function PageSubtitle({ title, children }: PageSubtitleProps) {
  const hasChildren = children !== undefined && children !== null && children !== false
  return (
    <header className={styles.pagehead}>
      <h1 className={styles.title}>{title}</h1>
      {hasChildren && (
        <div className={styles.subtitle} data-slot="subtitle">{children}</div>
      )}
    </header>
  )
}
```

The `hasChildren` guard collapses `null` / `false` / `undefined` to "no slot rendered" — same shape as Chip's `hasLeading`, so conditional patterns like `{loaded && <Chip…/>}` don't emit empty wrappers.

**File**: `skills/visualisation/visualise/frontend/src/components/PageSubtitle/PageSubtitle.module.css`

Token-bound styles modelled on the prototype's `ac-pagehead`. Concrete shape:

```css
.pagehead {
  display: flex;
  align-items: baseline;
  gap: var(--sp-3);
  margin-bottom: var(--sp-4);
}

.title {
  margin: 0;
  font-family: var(--ac-font-display);
  font-size: var(--size-h2);
  font-weight: 600;
  color: var(--ac-fg);
}

.subtitle {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: var(--size-sm);
  color: var(--ac-fg-muted);
}
```

Verify `--ac-font-display`, `--size-h2`, `--size-sm`, `--sp-2`, `--sp-3`, `--sp-4`, `--ac-fg`, `--ac-fg-muted` exist in `global.css` at implementation start. The CSS-source assertion in the test file pins `.title` → `--size-*`; extend it to cover the other token bindings if visual review requires tighter coverage.

**File**: `skills/visualisation/visualise/frontend/src/components/PageSubtitle/PageSubtitle.test.tsx`

```tsx
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { PageSubtitle } from './PageSubtitle'
import { Chip } from '../Chip/Chip'
import pageSubtitleCss from './PageSubtitle.module.css?raw'

describe('PageSubtitle', () => {
  it('renders the title', () => {
    render(<PageSubtitle title="Kanban" />)
    expect(screen.getByRole('heading', { name: 'Kanban', level: 1 })).toBeInTheDocument()
  })

  it('renders children alongside the title when provided', () => {
    render(
      <PageSubtitle title="Kanban">
        <Chip variant="indigo">live</Chip>
      </PageSubtitle>,
    )
    expect(screen.getByText('live').closest('[data-variant="indigo"]')).not.toBeNull()
  })

  it.each([
    ['undefined', undefined],
    ['null', null],
    ['false', false],
  ] as const)('does not render the subtitle slot when children=%s', (_label, value) => {
    const { container } = render(<PageSubtitle title="Kanban">{value as never}</PageSubtitle>)
    expect(container.querySelector('[data-slot="subtitle"]')).toBeNull()
  })

  it('binds title typography to a --size-* token (CSS source assertion)', () => {
    expect(pageSubtitleCss).toMatch(/\.title\s*\{[^}]*font-size:\s*var\(--size-/)
  })
})
```

#### Kanban adoption

**File**: `KanbanBoard.tsx`

The three render branches each currently render `<h1 className={styles.title}>Kanban</h1>`. Replace the heading region in the **loaded** branch only (the loading and error states do not display a "live" indicator) with:

```tsx
<PageSubtitle title="Kanban">
  <Chip variant="indigo">live</Chip>
</PageSubtitle>
```

The loading and error branches keep a plain `<PageSubtitle title="Kanban" />` (no chip) so the visual hierarchy is consistent across states.

#### Test for kanban adoption

Extend `KanbanBoard.test.tsx` (or create if absent):

```tsx
it('renders a "live" Chip with indigo variant in the page subtitle when loaded', () => {
  render(<KanbanBoard data={loadedFixture} />)
  expect(screen.getByText('live').closest('[data-variant="indigo"]')).not.toBeNull()
})

it('does not render the "live" chip in the loading branch', () => {
  render(<KanbanBoard state="loading" />)
  expect(screen.queryByText('live')).toBeNull()
})
```

#### Work-item update

This is an introduction, not a replacement, so update `meta/work/0038-generic-chip-component.md` to acknowledge the kanban "live" chip (and the shared `PageSubtitle` extraction) as in-scope additions alongside AC1–AC4. The change is captured in the Migration Notes section of this plan.

### Success Criteria

#### Automated Verification

- [ ] `npm test src/routes/lifecycle src/routes/library src/routes/kanban src/components/PageSubtitle` passes
- [ ] `npm run typecheck` clean
- [ ] `grep -rn '\.statusBadge\|\.activeBadge\|\.badge\b' skills/visualisation/visualise/frontend/src/routes` returns no matches in either `.tsx` or `.module.css` files (Phase 5 deletes both)
- [ ] `npm test src/styles/migration.test.ts` passes

#### Manual Verification

- [ ] Lifecycle cluster page: status badges are now coloured per status (e.g. "done" green, "in-progress" indigo, "todo" neutral). No visual regression in the cluster row layout.
- [ ] Library type view: rows with status values show coloured chips; rows that fall back to dates show neutral chips; empty rows show a plain em-dash (no chip).
- [ ] Templates view: each tier panel header shows the new chip(s) — `active` (indigo) on the active tier, `absent` (neutral) on missing tiers, no chip on present-but-non-active tiers.
- [ ] Templates index: each row's active-tier label renders as a neutral chip.
- [ ] Kanban view: page subtitle renders via the new `PageSubtitle` component and shows a "live" indigo chip in the loaded state. Loading and error states render a plain heading (no chip).
- [ ] All chips swap correctly between light and dark themes (toggle via topbar).

---

## Phase 6: Chip showcase route + Playwright visual regression

### Overview

Mirror the Glyph plan's Phase 4 structure (`meta/plans/2026-05-12-0037-glyph-component.md:516-720`). Build a `/chip-showcase` developer-only route that renders all 12 variant × size cells (6 variants × 2 sizes) and is captured by Playwright in two theme passes (light and dark) — yielding **24 baseline screenshots in total** (12 cells × 2 themes).

Theme strategy: follow the Glyph plan's pattern of toggling `documentElement.dataset.theme` between passes rather than nesting `[data-theme="dark"]` overrides on the same page. The dark token cascade is declared at `:root` and at `[data-theme="dark"]` (also at `:root` level), so nested theme overrides on child elements are not the established pattern in this codebase. Keep the showcase page theme-passive and let the topbar toggle drive the theme.

### Changes Required

#### 1. The showcase route

**File**: `skills/visualisation/visualise/frontend/src/routes/chip-showcase.tsx` (or matching TanStack Router file convention)

A 6 × 2 grid: rows are variants (`neutral`, `indigo`, `green`, `amber`, `red`, `violet`), columns are sizes (`sm`, `md`). Each cell wraps a `<Chip>` with a `data-testid` of `chip-cell-<variant>-<size>` so Playwright can target each cell precisely without depending on the current theme.

#### 2. Playwright visual-regression spec

**File**: `skills/visualisation/visualise/frontend/tests/e2e/chip-showcase.spec.ts`

For each theme (light, dark — toggled via the topbar control), iterate the 12 cells and take an element-level screenshot. Snapshots live under `tests/e2e/__screenshots__/chip-showcase/` with filenames of the form `<variant>-<size>-<theme>-<platform>.png` (e.g. `green-md-dark-darwin.png`).

**Cross-platform baselines**: borrow the Glyph plan's recipe for committing both `-darwin.png` and `-linux.png` baselines (Docker-based generation for Linux on macOS-only developer machines). See `meta/plans/2026-05-12-0037-glyph-component.md` Phase 4 for the concrete commands. Both platform baselines must be committed in the same PR as the spec.

#### 3. Resolved-colour spec

**File**: `skills/visualisation/visualise/frontend/tests/e2e/chip-resolved-colours.spec.ts`

For each variant × theme (12 pairs), navigate to the showcase, read `getComputedStyle(chipEl).color`, `.backgroundColor`, and `.borderColor`, and assert against canonical strings. `getComputedStyle` returns normalised `rgb(...)` / `rgba(...)` strings (never hex), so the expected values are precomputed and stored in a helper table.

**Helpers and expected values** (in the spec file):

```ts
// Parse a "rgb(r, g, b)" string into a [r, g, b] tuple. getComputedStyle
// always normalises to this shape for opaque colours; fully-transparent or
// alpha values are not expected for any chip cell in scope.
function parseRgb(rgb: string): [number, number, number] {
  const m = rgb.match(/rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)/)
  if (!m) throw new Error(`Cannot parse colour: ${rgb}`)
  return [Number(m[1]), Number(m[2]), Number(m[3])]
}

function rgbFromHex(hex: string): string {
  const h = hex.replace('#', '')
  const r = parseInt(h.slice(0, 2), 16)
  const g = parseInt(h.slice(2, 4), 16)
  const b = parseInt(h.slice(4, 6), 16)
  return `rgb(${r}, ${g}, ${b})`
}

/**
 * Assert that each channel of `actual` sits between the corresponding channels
 * of `low` and `high` (inclusive, channel-by-channel, regardless of which is
 * larger per channel). This pins a `color-mix(in srgb, A <pct>%, B)` result
 * without requiring exact rgba arithmetic (browser rounding can vary by 1).
 */
function expectChannelsBetween(actual: string, a: string, b: string) {
  const [ar, ag, ab] = parseRgb(actual)
  const [xr, xg, xb] = parseRgb(a)
  const [yr, yg, yb] = parseRgb(b)
  expect(ar).toBeGreaterThanOrEqual(Math.min(xr, yr))
  expect(ar).toBeLessThanOrEqual(Math.max(xr, yr))
  expect(ag).toBeGreaterThanOrEqual(Math.min(xg, yg))
  expect(ag).toBeLessThanOrEqual(Math.max(xg, yg))
  expect(ab).toBeGreaterThanOrEqual(Math.min(xb, yb))
  expect(ab).toBeLessThanOrEqual(Math.max(xb, yb))
  // And NOT equal to either endpoint — proves the mix actually happened
  expect(actual).not.toBe(a)
  expect(actual).not.toBe(b)
}

// Foreground expectations: only the four colour-coded variants have
// chip-owned hex values that are stable contracts of this plan. The
// `neutral` foreground reads `--ac-fg-muted` at runtime (it is not owned
// by this plan and may shift), so the assertion resolves it from the
// document root and compares dynamically.
const EXPECTED_FG_LIGHT = {
  indigo: rgbFromHex('#595fc8'),
  green:  rgbFromHex('#2e8b57'),
  amber:  rgbFromHex('#b4761f'),
  red:    rgbFromHex('#cb4647'),
  violet: rgbFromHex('#7b5cd9'),
} as const
const EXPECTED_FG_DARK = {
  indigo: rgbFromHex('#8a90e8'),
  green:  rgbFromHex('#79d9a6'),
  amber:  rgbFromHex('#e4b76e'),
  red:    rgbFromHex('#e86a6b'),
  violet: rgbFromHex('#7b5cd9'),
} as const

// Example assertion for one variant × theme cell:
test('green light: color matches token, background mixed between --ac-ok and --ac-bg', async ({ page }) => {
  await page.goto('/chip-showcase')
  const cell = page.getByTestId('chip-cell-green-sm')
  const fg  = await cell.evaluate(el => getComputedStyle(el).color)
  const bg  = await cell.evaluate(el => getComputedStyle(el).backgroundColor)
  const bd  = await cell.evaluate(el => getComputedStyle(el).borderColor)
  const acBg = await page.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue('--ac-bg').trim()
  )
  expect(fg).toBe(EXPECTED_FG_LIGHT.green)
  expectChannelsBetween(bg, EXPECTED_FG_LIGHT.green, rgbFromHex(acBg))
  expectChannelsBetween(bd, EXPECTED_FG_LIGHT.green, rgbFromHex(acBg))
})
```

Iterate the same assertion shape across all 12 variant × theme cells via `test.each`. For `neutral`, resolve `--ac-fg-muted` from the document root and compare against the resolved value rather than a hardcoded hex (since that token is outside the plan's diff scope and may shift). The `var(--ac-bg)` resolution always uses the live root value — that is the only way the `expectChannelsBetween` lower bound stays correct as the bg token evolves.

The plan deliberately does not pin exact rgba values for backgrounds/borders — `color-mix` precision varies by browser by 1–2 channel units and would produce flake. The visual-regression snapshots in spec #2 cover the pixel-exact case.

### Success Criteria

#### Automated Verification

- [ ] `npm run test:e2e tests/e2e/chip-showcase.spec.ts` passes (initial run generates baseline snapshots; subsequent runs diff against them)
- [ ] `npm run test:e2e tests/e2e/chip-resolved-colours.spec.ts` passes — all 12 variant × theme pairs resolve their foreground to the documented hex
- [ ] Both `-darwin.png` and `-linux.png` baseline files are committed for every cell (12 cells × 2 themes × 2 platforms = 48 baseline files)
- [ ] The showcase route renders cleanly: no console errors on mount

#### Manual Verification

- [ ] Visit `/chip-showcase` in light and dark; visually confirm each cell matches the prototype side-by-side (allowing for the documented `var(--ac-bg)`-vs-`transparent` background-compositing delta).
- [ ] Toggle dark theme via the topbar and confirm transitions are immediate (no flash, no layout shift).

---

## Phase 7: Cleanup

### Overview

Phase 5 sub-phases have already deleted the orphan CSS rules per surface (each Phase 5 commit covers both the TSX change and its matching CSS deletion). Phase 7 only handles the migration-ledger updates and adds a repo-wide guard that prevents new open-coded pill-radius elements from creeping in.

### Changes Required

#### 1. Migration ledger cleanup

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

Remove any existing `EXCEPTIONS` entries that referenced the now-deleted pill rules (e.g. off-scale rem exceptions tied to `.statusBadge` / `.badge` / `.activeBadge` / `.active`). The three new chip-related `EXCEPTIONS` entries (`0.125rem`, `0.1875rem`, `0.02em` — see Phase 2 §4) are already in place from Phase 2.

#### 2. Repo-wide pill-radius guard

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`

Add a focused test using the existing Vite glob pattern (`import.meta.glob`) so the harness stays consistent with the rest of the file — do **not** introduce `glob.sync` / `fs.readFileSync`, which mix bundle-time and filesystem-time CSS access:

```ts
const cssModules = import.meta.glob('../**/*.module.css', { eager: true, query: '?raw', import: 'default' }) as Record<string, string>

// Files that legitimately use --radius-pill — every other usage in src/routes
// is an open-coded status pill regression and must be migrated to <Chip>.
const PILL_RADIUS_ALLOW_LIST = new Set([
  '../components/Chip/Chip.module.css',
  '../components/OriginPill/OriginPill.module.css',
  '../components/Sidebar/Sidebar.module.css',           // unseen-doc dot
  '../routes/kanban/KanbanColumn.module.css',           // column count badge
])

it('no module outside the allow-list defines a pill-radius element', () => {
  const offenders: string[] = []
  for (const [path, text] of Object.entries(cssModules)) {
    if (PILL_RADIUS_ALLOW_LIST.has(path)) continue
    if (/border-radius:\s*var\(--radius-pill\)/.test(text)) {
      offenders.push(path)
    }
  }
  expect(offenders).toEqual([])
})
```

The allow-list is maintained explicitly so future open-coded pills fail this test. To add a new allowed site, both the new file path and a brief reason should be added in the same change.

### Success Criteria

#### Automated Verification

- [ ] `npm test src/styles/migration.test.ts` passes (including the new pill-radius guard)
- [ ] `grep -rn 'statusBadge\|activeBadge' skills/visualisation/visualise/frontend/src` returns no matches
- [ ] `grep -rn '\.badge' skills/visualisation/visualise/frontend/src/routes/library` returns no matches in `.tsx` or `.module.css` files
- [ ] Full test suite passes: `npm test && npm run typecheck && npm run build`

#### Manual Verification

- [ ] All four migrated surfaces still render correctly in both themes.
- [ ] No visual regression in unrelated pill-radius elements (OriginPill, SseIndicator, kanban column count, sidebar unseen-dot).

---

## Testing Strategy

### Unit tests (Vitest)

- **Chip.test.tsx**: variant / size rendering, leading slot (with `null` / `false` / `undefined` edge cases), `aria-label` forwarding, `data-*` attributes, CSS source assertions for each variant→token binding (quote-style-agnostic regex), `color-mix()` convention compliance.
- **src/api/status-variant.test.ts**: every documented status string → expected variant, case/whitespace/separator/slash insensitivity, fallback to neutral, non-string inputs (arrays, objects, booleans, numbers) → neutral, Set-key shape invariant.
- **FrontmatterChips.test.tsx**: per-value Chip rendering, status colour-coding (case-insensitive), `aria-label="${key}: ${value}"` per chip, neutral fallback for non-status fields, array joining preserved, empty-string values skipped, boolean/numeric values stringified, malformed banner text preserved verbatim, absent state returns null.
- **PageSubtitle.test.tsx**: title rendering, optional children slot, `data-slot="subtitle"` only when children are provided.
- Per-surface test updates for lifecycle, library, templates, kanban that assert `[data-variant]` is present with the right value and that the corresponding CSS module no longer carries the legacy class (raw-CSS `?raw` source check, not DOM `querySelector`).

### CSS source assertions

Every Chip variant uses the `?raw` regex pattern (canonical: `SseIndicator.test.tsx`) to lock variant→token bindings. Without these, CSS can drift silently. The repo-wide hex prohibition lives in `migration.test.ts` (single source of truth — no duplicated per-component hex guard).

### Visual regression (Playwright)

The `/chip-showcase` route renders 12 cells (6 variants × 2 sizes) and is captured by Playwright in two theme passes (light and dark) — yielding 24 baseline screenshots per platform, 48 across darwin and linux. A second Playwright spec reads each cell's resolved `color` and asserts the canonical foreground hex per theme, then sanity-checks the `backgroundColor` / `borderColor` are tinted (between `--ac-bg` and the full semantic colour) — catches token drift that pixel diffs might tolerate without requiring exact rgba arithmetic.

### Manual smoke tests per phase

Each phase's success criteria include manual eyeball verification of the affected surface in both themes. Surface-by-surface coverage is explicit in Phase 5.

## Performance Considerations

The Chip primitive is a thin `<span>` wrapper with no effects, no context, no state. Each occurrence is O(1) DOM nodes (one span, or two when `leading` is set). The prototype renders dozens of chips on a single lifecycle page with no perceptible cost; migrating to React `<Chip>` carries no measurable overhead.

`statusToChipVariant` is a pure function with two normalised string operations and a constant-time `Set.has` lookup — negligible compared to React render cost.

`color-mix()` is supported in all evergreen browsers (Chrome 111+, Safari 16.2+, Firefox 113+) and is already used elsewhere in the codebase (`FrontmatterChips.module.css:9`). No fallback needed.

## Migration Notes

These notes capture deviations from work item 0038 that this plan introduces. The work item should be updated to reflect them.

- **Variant list expanded from four to six.** The work item enumerates `green`, `indigo`, `amber`, `neutral` plus `sm`. The prototype has six colour variants (`neutral`, `indigo`, `green`, `amber`, `red`, `violet`) and two sizes (`sm` default, `md`). The plan ships the full six.
- **Size naming corrected.** The work item lists `sm` (default). The prototype's size modifier is `--md` (larger), with the base `.ac-chip` being the small variant. The plan adopts `ChipSize = 'sm' | 'md'` with `'sm'` as default. This is an inversion of the work item's implicit "sm is the override" framing.
- **FrontmatterChips restyle is structural, not just stylistic.** The work item describes consuming the generic Chip variants "where applicable". This plan goes further: the current `<dl>/<dt>/<dd>` key/value structure is replaced by a flat list of key-less chips matching the prototype's `ac-pagehead__sub` treatment. The key context is preserved in `aria-label` for screen-reader users.
- **New kanban "live" chip introduced.** Work item ACs cover replacements only; the plan adds a new chip site on the kanban page subtitle (per the prototype). A small shared `PageSubtitle` component is extracted to host it.
- **`statusToChipVariant` lives at `src/api/status-variant.ts`.** Originally suggested in the work item as a Chip-internal helper, the plan places it in `src/api/` because it is a domain function consumed by routes, not by Chip itself.
- **AC4 sharpened.** The work item's AC4 ("`grep` for open-coded inline chip styles returns no results") is operationalised in Phase 7 as a concrete allow-list pinned in `migration.test.ts` (Chip + OriginPill + Sidebar unseen-dot + KanbanColumn count).
- **New typography tokens.** Phase 1 introduces `--size-chip` (10.5px) and `--size-chip-md` (11.5px) to capture the prototype's chip-specific font sizes without admitting raw pixel literals to the Chip CSS module.
- **`color-mix()` blends against `var(--ac-bg)` not `transparent`.** Per ADR-0026 and the locked `migration.test.ts` convention, Chip variant tints use the `{8, 30}%` ladder against `var(--ac-bg)` rather than the prototype's verbatim `rgba(... 0.08/0.10/0.22/0.25, transparent)`. Visual fidelity to the prototype is intentionally traded for token-discipline consistency; the delta is small on white surfaces.

## References

- Work item: `meta/work/0038-generic-chip-component.md`
- Research: `meta/research/codebase/2026-05-14-0038-generic-chip-component.md`
- Structural template: `meta/plans/2026-05-12-0037-glyph-component.md`
- Blocker (shipped): `meta/work/0033-design-token-system.md`
- ADR-0026 (token application conventions): `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
- Canonical pattern reference: `skills/visualisation/visualise/frontend/src/components/SseIndicator/SseIndicator.tsx` (variant via `data-*`) and `SseIndicator.test.tsx` (CSS source assertion pattern)
- Canonical pill reference: `skills/visualisation/visualise/frontend/src/components/OriginPill/OriginPill.module.css` (pill geometry + `prefers-reduced-motion`)
- Prototype source (downloaded standalone HTML): `~/Downloads/Accelerator Visualiser _standalone_.html`
- Live prototype (auth required): `https://claude.ai/design/p/64bfef0a-f5fb-4b90-81e4-229d1ebc705c?file=Accelerator%20Visualiser.html&present=1`
