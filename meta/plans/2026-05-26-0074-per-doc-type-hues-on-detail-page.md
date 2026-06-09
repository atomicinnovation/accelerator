---
date: "2026-05-26T00:00:00+01:00"
type: plan
producer: create-plan
work-item: "0074"
status: done
id: "2026-05-26-0074-per-doc-type-hues-on-detail-page"
title: "0074 — Per-Doc-Type Hues on Detail Page"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-26T00:00:00+01:00"
last_updated_by: Toby Clemson
revision: "86048379aecc"
repository: "ticket-management"
relates_to: ["work-item:0074", "codebase-research:2026-05-24-0074-per-doc-type-hues-on-detail-page"]
---

# 0074 — Per-Doc-Type Hues on Detail Page

## Overview

Surface the existing `--ac-doc-<key>` per-doc-type colour tokens (delivered by
0037) on the detail-page eyebrow and on the right-hand `RelatedArtifacts` row
icons. The detail page is currently colour-agnostic and renders no eyebrow at
all; this plan introduces an eyebrow on `LibraryDocView`, adds per-row icons
to `RelatedArtifacts`, and folds the previously-virtual `templates` key into
the `Glyph` component (via a data-driven `DOC_TYPE_COLOR_VAR` lookup that
resolves templates to `--ac-fg-muted`) so a single consumer surface
(`<Glyph>`) drives all 13 doc-type icons. The `GlyphDocTypeKey` type and
`isGlyphDocTypeKey` guard are removed; `Glyph.docType` accepts `DocTypeKey`
directly.

Six independent phases. Phases 1–3 are prerequisites with no inter-phase
dependencies and can be merged in any order. Phases 4 and 5 each deliver
one acceptance criterion and depend only on 1–3 (not on each other). Phase 6
is a standalone non-regression spec. Each phase follows a write-failing-test
→ implement → verify cycle.

## Current State Analysis

The work item's pre-implementation research surfaced five mismatches between
the work-item text and the code. The plan resolves each before implementation
begins:

- **No `TypeGlyph` or `StageTile` component.** The unified component is
  `Glyph` (`frontend/src/components/Glyph/Glyph.tsx`). It is consumed by
  `HubCard` (overview hub, `routes/library/LibraryOverviewHub.tsx:58-88`),
  `EyebrowLabel` (listing route, `routes/library/LibraryTypeView.tsx:270-279`),
  `LibraryTemplatesIndex.tsx:116` (per-row), `ActivityFeed.tsx:122-123`
  (overridden monochrome), and the dev-only `/glyph-showcase`. The
  `Sidebar` does not consume `--ac-doc-*` at all.
- **No `--ac-text-muted` token.** The semantic muted-text token is
  `--ac-fg-muted` (`#5f6378` light; `global.css:81-84`). All acceptance
  criteria are read as `--ac-fg-muted`.
- **Specs live under `frontend/tests/visual-regression/`**, not
  `frontend/e2e/`. The Playwright `visual-regression` project runs ahead
  of `chromium` and is the home for computed-style assertions
  (`playwright.config.ts:21-37`).
- **The detail page does not pass an `eyebrow` prop today.**
  `LibraryDocView.tsx:152-156` renders `<Page title={title}
  subtitle={subtitle}>` — there is no eyebrow on this surface. Phase 4
  introduces one.
- **`RelatedArtifacts` rows have no icons today.** Each `<li>` renders an
  anchor plus a single text "declared"/"inferred" badge
  (`RelatedArtifacts.tsx:88-94`). `IndexEntry.type: DocTypeKey` is already
  in scope on every row (used by the href), so no upstream plumbing is
  needed. Phase 5 inserts a `<Glyph>` per row.

The remaining context is in good shape:

- `--ac-doc-<key>` tokens declared at `frontend/src/styles/global.css:100-126`
  (light fg + bg), mirrored to white / `#1d2030` in dark theme
  (`global.css:328-353` and `:388-411`).
- TS-side resolved hex in `frontend/src/styles/tokens.ts:35-46, 50-61,
  94-105, 109-120`, parity-tested against CSS.
- `DocTypeKey` 13-key union and runtime tables in
  `frontend/src/api/types.ts:4-49`. `VIRTUAL_DOC_TYPE_KEYS = ['templates']`
  at line 30. 12-key `GlyphDocTypeKey` at
  `frontend/src/components/Glyph/Glyph.constants.ts:19` — **removed in
  Phase 1**; `Glyph.docType` becomes `DocTypeKey` directly and a
  `DOC_TYPE_COLOR_VAR: Record<DocTypeKey, string>` lookup table replaces
  the inline `var(--ac-doc-${docType})` resolution.
- `--ac-fg-muted` is `#5f6378` in light theme and `#a0a5b8` in dark
  (`tokens.ts:77`, `global.css:376`). Unlike the 12 `--ac-doc-<key>`
  foreground tokens — which DO collapse to `var(--atomic-white)` in dark
  — `--ac-fg-muted` does NOT collapse to white. The templates Glyph is
  intentionally distinct from the other 12 in dark theme.
- Rust server already supports `work-item-reviews` / `design-gaps` /
  `design-inventories` via `DocTypeKey::all()` (`server/src/docs.rs:6-39`).
  No server changes required — only frontend fixtures + `start-server.mjs`
  doc_paths entries are missing.
- Visual-regression spec helpers `hexToRgb()`, `parseRgb()`,
  `expectChannelsBetween()` exist at
  `tests/visual-regression/chip-resolved-colours.spec.ts:4-42`; the
  parametrised light + dark looping pattern is in the same file at lines
  46-103. **Phase 4 extracts these helpers plus an `EXPECTED_COLOR` map
  into `tests/visual-regression/lib/expected-colours.ts`** so Phases 4/5/6
  share one source of truth (and the two existing specs migrate to it).
- The canonical theme-switch mechanism in visual-regression specs is
  `document.documentElement.dataset.theme = 'dark'` followed by
  `page.waitForFunction(...)` to await CSS application (see
  `chip-resolved-colours.spec.ts:67-72` and
  `glyph-resolved-fill.spec.ts:25-29`). There is no `?theme=` query string.

### Key Discoveries

- The detail-page route narrows `$type` to `DocTypeKey` at
  `LibraryDocView.tsx:37-42`; the narrowed `type` variable is already in
  scope at the render call site (`:152-156`).
- The listing-route `EyebrowLabel` (`LibraryTypeView.tsx:270-279`) is the
  canonical eyebrow shape: `<Glyph framed> + uppercase label`.
- Today's templates eyebrow uses an inline `LayersIcon` (Feather "layers"
  SVG, `LibraryTemplatesIndex.tsx:23-41`) that inherits `currentColor`
  from `.eyebrow` (`Page.module.css:30-40` → `--ac-fg-faint`, `#8b90a3`).
  Per user direction, this icon is promoted to a `templates.tsx` entry
  under `Glyph/icons/` and consumed via `<Glyph>` everywhere, with
  templates colour-resolved to `--ac-fg-muted` (`#5f6378`).
- Dark theme collapses all 12 `--ac-doc-<key>` foreground tokens to
  `var(--atomic-white)` (`rgb(255, 255, 255)`). All visual-regression
  assertions parametrise expected RGB by theme.
- The chip-resolved-colours spec already proves the looping + parametrised
  expected-table pattern is viable.

## Desired End State

A user navigating to any detail page (`/library/<type>/<slug>`) sees an
eyebrow above the page heading composed of a per-doc-type tinted glyph
(in `--ac-doc-<key>` for the 12 non-virtual keys, `--ac-fg-muted` for the
virtual `templates` key) followed by the uppercase doc-type label. The
right-hand aside's `Related artifacts` block shows the same tinted glyph
on every row, identifying the related document's doc type at a glance.

Verification:
- Visual-regression suite passes for the new specs (Phases 4, 5, 6) across
  light + dark themes.
- The existing chip-resolved-colours, glyph-resolved-fill, and CSS↔TS
  parity tests continue to pass (chip and glyph specs are mechanically
  refactored in Phase 4 to consume the shared helper module).
- Manual inspection of `/library/<each-type>/<slug>` confirms the eyebrow
  tint matches the colour the same doc-type carries in the overview hub
  card and listing-route eyebrow.

### Dark-theme behaviour

The 12 `--ac-doc-<key>` foreground tokens collapse to `var(--atomic-white)`
in dark theme (see `global.css:328-353`); the templates Glyph uses
`--ac-fg-muted`, which is `#a0a5b8` in dark and does NOT collapse to white.
This means:

- In dark mode, the 12 non-virtual doc-type icons all render as white,
  which effectively erases the per-doc-type colour cue. The cue is
  preserved only in light mode.
- The templates icon renders as light-grey `#a0a5b8` in dark, visibly
  distinct from the white-on-dark of the other 12 keys.

This is a known limitation of the existing token design and is **not
in scope for this plan** — re-introducing per-doc-type hues in dark mode
is owned by 0073 (brand-layer palette). Recording the tradeoff here so
future readers know it has been considered.

## What We're NOT Doing

- **Hero illustration / `BigGlyph` tint** — owned by 0082 (blocked by
  this plan).
- **Aside structural redesign** (section vocabulary, declared/inferred
  re-grouping, cluster block, row background/border treatments) — owned
  by 0079 (also blocked by this plan).
- **New tokens.** `--ac-doc-*` namespace is unchanged. Templates colour
  resolution is component-local inside `Glyph`, via a data-driven
  `DOC_TYPE_COLOR_VAR: Record<DocTypeKey, string>` lookup that resolves
  `templates → var(--ac-fg-muted)`. No new `--ac-doc-templates` token.
- **Dark-theme hue restoration.** All 12 `--ac-doc-<key>` foregrounds
  continue to collapse to white in dark mode. Re-introducing per-doc-type
  hues in dark is owned by 0073.
- **Sidebar consumption.** Sidebar is colour-agnostic today and stays
  that way. AC #3 is reframed to cover `HubCard` and the listing-route
  `EyebrowLabel` (the two real consumers).
- **Row container background / border / border-width on
  `RelatedArtifacts`.** Icon-only tint, per work-item review-1.
- **Eyebrow label-text colour change.** Label text continues to inherit
  `.eyebrow`'s `color: var(--ac-fg-faint)` (`Page.module.css:30-40`).
- **Server (Rust) changes.** `DocTypeKey::all()` already includes all
  three missing keys (verified in `server/src/docs.rs:23-39`).

## Implementation Approach

The plan funnels every per-doc-type tint through a single
component — `Glyph` — and reuses one extracted `EyebrowLabel` helper at
both eyebrow surfaces. The `GlyphDocTypeKey`/`isGlyphDocTypeKey` pair is
collapsed: `Glyph.docType` accepts `DocTypeKey` directly and resolves
colour via a `DOC_TYPE_COLOR_VAR: Record<DocTypeKey, string>` lookup
table (data-driven, exhaustively typed). Pre-existing surfaces that
previously relied on the guard to exclude templates (`HubCard`,
`ActivityFeed`) call the new `isPhysicalDocTypeKey(docType)` predicate
(introduced in `api/types.ts` alongside `VIRTUAL_DOC_TYPE_KEYS`) to
preserve current behaviour without scattering string-literal
exclusions across consumers. This keeps the 0037 Glyph consumer contract
central and means the 0073 brand-layer rewrite continues to work
end-to-end.

Phase ordering inside a strict TDD cycle:

1. Add a failing visual-regression / Vitest spec for the phase's
   behaviour. Confirm it fails for the expected reason (missing fixture,
   missing prop, missing icon).
2. Make the minimum code change required to satisfy the spec.
3. Run the phase's success checks (typecheck, lint, vitest, the new
   visual-regression spec, plus the broader suite for non-regression).

---

## Phase 1: Collapse `GlyphDocTypeKey` and route templates through `Glyph`

### Overview

Make `Glyph` render all 13 `DocTypeKey` values directly by:

1. Adding a data-driven `DOC_TYPE_COLOR_VAR: Record<DocTypeKey, string>`
   lookup in `Glyph.constants.ts` (`templates → var(--ac-fg-muted)`, the
   other 12 → `var(--ac-doc-<key>)`).
2. Deleting `GlyphDocTypeKey`, `GLYPH_DOC_TYPE_KEYS`, and
   `isGlyphDocTypeKey`. `Glyph.docType` accepts `DocTypeKey` directly.
3. Adding the inline `LayersIcon` as `Glyph/icons/TemplatesIcon.tsx`
   under the Glyph consumer contract (24×24 viewBox, `fill="currentColor"`).
4. Updating every `isGlyphDocTypeKey` callsite. **Special attention**:
   `HubCard` (`LibraryOverviewHub.tsx`) and `ActivityFeed` currently
   rely on the guard's exclusion of `templates`. After the collapse the
   guard no longer exists; both gain a call to the new
   `isPhysicalDocTypeKey(docType)` predicate (introduced in
   `api/types.ts`, see §4 below) to preserve current behaviour. The
   user-facing decision is that HubCard's templates tile continues to
   render **without** a Glyph, matching today's behaviour.

This phase does not touch `LibraryTemplatesIndex` — its eyebrow rewrite
is owned by Phase 2 (which extracts `EyebrowLabel`).

### Changes Required

#### 1. New icon file

**File**: `frontend/src/components/Glyph/icons/TemplatesIcon.tsx` (new)
**Changes**: Re-house the layers SVG path under the Glyph icon contract,
matching the shape of every sibling icon (explicit `ReactElement` return
type, string-quoted SVG attributes).

```tsx
import type { ReactElement } from 'react'

export function TemplatesIcon(): ReactElement {
  return (
    <g
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="m12 3 9 5-9 5-9-5z" />
      <path d="m3 13 9 5 9-5" />
      <path d="m3 18 9 5 9-5" />
    </g>
  )
}
```

(`viewBox` is owned by `Glyph.tsx`; the icon component only renders
child paths, matching every existing icon under `Glyph/icons/`.)

#### 2. Replace `GlyphDocTypeKey`/`isGlyphDocTypeKey` with `DOC_TYPE_TOKEN_KEY` + `DOC_TYPE_COLOR_VAR`

##### 2a. Prerequisite — add a unified `ColorTokenKey` alias

**File**: `frontend/src/styles/tokens.ts`
**Changes**: `tokens.ts:314-315` exports `ColorTokenLight = keyof typeof
LIGHT_COLOR_TOKENS` and `ColorTokenDark = keyof typeof DARK_COLOR_TOKENS`
but no unified alias. Add:

```ts
// Intersection of light/dark token key sets — names tokens defined
// in BOTH themes. The CSS↔TS parity test in global.test.ts:185
// codifies a deliberate exclusion (e.g. `ac-violet` is light-only),
// so this intersection drops any one-side-only tokens by design. Use
// this alias when a token key must resolve in both themes.
export type ColorTokenKey = ColorTokenLight & ColorTokenDark
```

This makes the cross-theme guarantee visible at the type level — any
token added to only one theme is automatically excluded from this
alias and cannot be used as a `DOC_TYPE_TOKEN_KEY` value.

##### 2b. Constants module

**File**: `frontend/src/components/Glyph/Glyph.constants.ts`
**Changes**:
- Delete the `GlyphDocTypeKey` type alias, the `GLYPH_DOC_TYPE_KEYS`
  array, and the `isGlyphDocTypeKey` guard.
- Delete the file-level invariant comment about virtual-key exclusion
  (the invariant no longer holds — Glyph now renders all 13 keys).
- Add a `DOC_TYPE_TOKEN_KEY: Record<DocTypeKey, ColorTokenKey>` table
  that names the CSS-token key per doc type. Then derive
  `DOC_TYPE_COLOR_VAR` from it:

```ts
import { type ColorTokenKey } from '../../styles/tokens'

// Per-doc-type colour token key. Templates is a virtual doc-type with
// no dedicated colour token — it borrows --ac-fg-muted (a neutral
// text token) for visual consistency with `--ac-fg-faint` neighbours.
export const DOC_TYPE_TOKEN_KEY: Record<DocTypeKey, ColorTokenKey> = {
  decisions: 'ac-doc-decisions',
  'work-items': 'ac-doc-work-items',
  plans: 'ac-doc-plans',
  research: 'ac-doc-research',
  'plan-reviews': 'ac-doc-plan-reviews',
  'pr-reviews': 'ac-doc-pr-reviews',
  'work-item-reviews': 'ac-doc-work-item-reviews',
  validations: 'ac-doc-validations',
  notes: 'ac-doc-notes',
  'pr-descriptions': 'ac-doc-pr-descriptions',
  'design-gaps': 'ac-doc-design-gaps',
  'design-inventories': 'ac-doc-design-inventories',
  templates: 'ac-fg-muted',
}

// Direct literal (no Object.fromEntries cast) so the Record<…,…>
// constraint is enforced at definition rather than via post-hoc `as`.
export const DOC_TYPE_COLOR_VAR: Record<DocTypeKey, string> = {
  decisions: `var(--${DOC_TYPE_TOKEN_KEY.decisions})`,
  'work-items': `var(--${DOC_TYPE_TOKEN_KEY['work-items']})`,
  plans: `var(--${DOC_TYPE_TOKEN_KEY.plans})`,
  research: `var(--${DOC_TYPE_TOKEN_KEY.research})`,
  'plan-reviews': `var(--${DOC_TYPE_TOKEN_KEY['plan-reviews']})`,
  'pr-reviews': `var(--${DOC_TYPE_TOKEN_KEY['pr-reviews']})`,
  'work-item-reviews': `var(--${DOC_TYPE_TOKEN_KEY['work-item-reviews']})`,
  validations: `var(--${DOC_TYPE_TOKEN_KEY.validations})`,
  notes: `var(--${DOC_TYPE_TOKEN_KEY.notes})`,
  'pr-descriptions': `var(--${DOC_TYPE_TOKEN_KEY['pr-descriptions']})`,
  'design-gaps': `var(--${DOC_TYPE_TOKEN_KEY['design-gaps']})`,
  'design-inventories': `var(--${DOC_TYPE_TOKEN_KEY['design-inventories']})`,
  templates: `var(--${DOC_TYPE_TOKEN_KEY.templates})`,
}
```

The `Record<DocTypeKey, ColorTokenKey>` constraint plus the existing
CSS↔TS parity test gives compile-time exhaustiveness for both the doc
key set AND the token key set. Tests (and any future consumer needing
to resolve a doc type to its hex value) consume the typed
`DOC_TYPE_TOKEN_KEY` map directly — no string-format coupling.

#### 3. `Glyph` consumes the lookup table; `ICON_COMPONENTS` gains templates

**File**: `frontend/src/components/Glyph/Glyph.tsx`
**Changes**:
- Widen the `docType` prop type from `GlyphDocTypeKey` to `DocTypeKey`.
- Update the `import { GLYPH_DOC_TYPE_KEYS, type GlyphDocTypeKey }`
  line at `Glyph.tsx:4` — drop the deleted symbols; import `DOC_TYPE_KEYS`
  from `'../../api/types'` if the dev-only warn still needs the list.
- Delete the re-export block at `Glyph.tsx:22-26` entirely
  (`export { GLYPH_DOC_TYPE_KEYS, isGlyphDocTypeKey, type GlyphDocTypeKey }`).
  These symbols no longer exist; the module's remaining public
  constants surface is `DOC_TYPE_TOKEN_KEY` / `DOC_TYPE_COLOR_VAR`,
  imported directly from `Glyph.constants` by consumers.
- Import `TemplatesIcon`; add a `'templates': TemplatesIcon` entry to
  `ICON_COMPONENTS`. The compile-time exhaustiveness check on
  `Record<DocTypeKey, …>` now requires the entry.
- Replace the inline-style expression `style={{ color:
  \`var(--ac-doc-${docType})\` }}` with
  `style={{ color: DOC_TYPE_COLOR_VAR[docType] }}` at **both** call
  sites — the framed branch (`Glyph.tsx:110`) and the unframed branch
  (`Glyph.tsx:129`). Both must be updated; the framed branch is the
  one EyebrowLabel uses (Phase 2), so missing it would cause the
  detail-page templates eyebrow to silently render with the wrong
  colour.
- Update the JSDoc consumer-contract comment at `Glyph.tsx:73-74` to
  reference `DocTypeKey` / `DOC_TYPE_KEYS` instead of the deleted
  `GlyphDocTypeKey` / `GLYPH_DOC_TYPE_KEYS`. For the dev-only
  `console.warn` at `Glyph.tsx:81`, derive the enumeration at
  warn-emit time from the runtime list — e.g.
  ``console.warn(`Unknown docType: ${docType}. Expected one of: ${DOC_TYPE_KEYS.join(', ')}.`)``
  — so the message stays accurate by construction if future virtual
  keys are added.

**No CSS rule for templates frame.** The default `.frame` background
(`Glyph.module.css:11`) is `--ac-bg-sunken`, which is acceptable for
templates. Adding a `.frame[data-doc-type="templates"]` rule that
sets the same value would be a functional no-op. Instead, add a
single comment in `Glyph.module.css` adjacent to the per-doc-type
background rules:

```css
/* templates intentionally has no [data-doc-type="templates"] rule —
   the default .frame background (--ac-bg-sunken) is correct for the
   neutral muted-text glyph. See meta/work/0074. */
```

#### 4. Introduce `isPhysicalDocTypeKey` predicate and update every consumer

Knowledge of "which keys are virtual" should live in one place. Add a
type alias for the virtual-key set and a small data-driven predicate
in `frontend/src/api/types.ts` alongside the existing
`VIRTUAL_DOC_TYPE_KEYS`. First **drop the explicit
`: readonly DocTypeKey[]` annotation** from the existing constant so
`as const` narrows it properly:

```ts
// Before:
export const VIRTUAL_DOC_TYPE_KEYS: readonly DocTypeKey[] = ['templates'] as const

// After (the explicit annotation widens the inferred type and defeats
// the `as const` narrowing; with the annotation removed, the constant's
// type becomes `readonly ['templates']`, which is what we want):
export const VIRTUAL_DOC_TYPE_KEYS = ['templates'] as const satisfies
  readonly DocTypeKey[]
```

Then add the type alias and predicate:

```ts
export type VirtualDocTypeKey = typeof VIRTUAL_DOC_TYPE_KEYS[number]
// Resolves to the literal-union 'templates' (today), broadening
// automatically if VIRTUAL_DOC_TYPE_KEYS grows in future.

export function isPhysicalDocTypeKey(
  key: DocTypeKey,
): key is Exclude<DocTypeKey, VirtualDocTypeKey> {
  return !(VIRTUAL_DOC_TYPE_KEYS as readonly DocTypeKey[]).includes(key)
}
```

Notes:
- The `satisfies` clause keeps the runtime widening-guard (every
  virtual key is still a valid `DocTypeKey`) without losing the
  literal-union narrowing the type alias depends on.
- `isPhysicalDocTypeKey` returns a type predicate so callers wanting
  narrowing (`if (isPhysicalDocTypeKey(k)) { /* k narrowed */ }`)
  get it. Current call sites (HubCard, ActivityFeed) use the
  predicate as a boolean in an `&&` gate — no behavioural change.
- The `as readonly DocTypeKey[]` widening inside the body matches the
  existing `isDocTypeKey` pattern at `api/types.ts:22-24`.

This replaces the deleted `isGlyphDocTypeKey` semantically for callers
whose intent is "this doc type has an on-disk fixture and a per-doc
colour palette". Adding a future virtual key to `VIRTUAL_DOC_TYPE_KEYS`
automatically propagates to every consumer — restoring the data-driven
property the original 0037 `GlyphDocTypeKey` was designed to provide.

Update every existing consumer of the deleted symbols:

- **`LibraryOverviewHub.tsx:72-74`** (`HubCard`): replace
  `isGlyphDocTypeKey(docType.id) && <Glyph ...>` with
  `isPhysicalDocTypeKey(docType.id) && <Glyph ...>`. Preserves current
  behaviour: the templates HubCard tile renders without a Glyph. Add a
  code comment: `// templates excluded — see meta/work/0074`.
- **`ActivityFeed.tsx`** (the existing `isGlyphDocTypeKey` gate at
  `:122-123`): replace with `isPhysicalDocTypeKey`. ActivityFeed today
  renders monochrome glyphs via the `.row [data-doc-type] { color:
  ... !important; }` CSS override; the templates exclusion remains
  visually consistent.
- **`LibraryTypeView.tsx`**: drop the now-unused `isGlyphDocTypeKey`
  import; the private `EyebrowLabel` function (which used the guard)
  is replaced wholesale in Phase 2.
- **`routes/library/template-tier.ts`**: the file imports
  `GlyphDocTypeKey` and uses it as the value type of `STEM_TO_GLYPH`
  and the return type of `glyphKeyForTemplate`. Replace
  `GlyphDocTypeKey` with `DocTypeKey` throughout. Add a comment at the
  top of `STEM_TO_GLYPH`: `// Stems map to physical doc-type keys
  only — templates is the umbrella, not a target.`
- **`routes/glyph-showcase/GlyphShowcase.tsx`**: imports
  `GLYPH_DOC_TYPE_KEYS` and iterates it (`:2, :26`). Replace with
  `DOC_TYPE_KEYS` from `api/types.ts`. The showcase grid will gain a
  templates cell automatically (correct: Phase 1 §5 / Phase 1 §6 both
  assume the cell exists).
- **`routes/glyph-showcase/GlyphShowcase.test.tsx`**: imports
  `GLYPH_DOC_TYPE_KEYS` (`:4, :14, :26`). Repoint to `DOC_TYPE_KEYS`;
  update length assertion 12 → 13 if present; ensure the templates
  case is exercised consistently with the new contract.
- **`tests/visual-regression/glyph-showcase.spec.ts`**: imports
  `GLYPH_DOC_TYPE_KEYS` from `Glyph.constants` (`:4, :41`). Repoint to
  `DOC_TYPE_KEYS` from `api/types.ts`. If existing pixel snapshots
  exist under `tests/visual-regression/__screenshots__/`, regenerate
  them after the templates cell appears (call out the regeneration
  step in Phase 1 success criteria).
- **`Glyph.tsx`** JSDoc and `console.warn`: already covered in Phase 1
  §3.
- **`LibraryTemplatesIndex.tsx:23-41` (`LayersIcon`)** and its eyebrow
  usage are unchanged in this phase. They are removed in Phase 2 as
  part of the `EyebrowLabel` consolidation.

#### 5. Update `Glyph.test.tsx` — invert the 12-key contract assertions

**File**: `frontend/src/components/Glyph/Glyph.test.tsx` (extend, do NOT
place under `__tests__/` — every existing component test in this codebase
is co-located).

The existing suite encodes the 12-key exclusion contract that Phase 1
removes. Each of the following assertions must be inverted, deleted, or
replaced:

- Line 3 (top-level imports): **update** —
  `import { Glyph, GLYPH_DOC_TYPE_KEYS, isGlyphDocTypeKey, type
  GlyphDocTypeKey } from './Glyph'` becomes `import { Glyph } from
  './Glyph'`. Re-import `DOC_TYPE_KEYS` from `'../../api/types'` if
  the rewritten `describe.each` loop needs it directly.
- Lines 11-16 (`_typeContractGuards`): **delete the entire block.**
  The `describe.each(DOC_TYPE_KEYS)` loop (updated below) exercises
  every docType at the type level via the JSX, making
  `_typeContractGuards` redundant. Don't leave a defanged version —
  the underscore-prefixed name signals "intentionally unused" and a
  vestigial helper invites future confusion.
- Lines 19-21 (`'GLYPH_DOC_TYPE_KEYS has 12 entries'`): **delete**.
  `GLYPH_DOC_TYPE_KEYS` is removed.
- Lines 23-25 (`'GLYPH_DOC_TYPE_KEYS excludes the virtual templates key'`):
  **delete**.
- Lines 33-35 (`'isGlyphDocTypeKey rejects the virtual templates key'`):
  **delete**. `isGlyphDocTypeKey` is removed.
- Line 38 (`expected = DOC_TYPE_KEYS.filter(k => k !== 'templates')`):
  **update** — drop the filter; the expected list is all 13 keys.
- The `describe.each(GLYPH_DOC_TYPE_KEYS)` loop at line 176 should
  become `describe.each(DOC_TYPE_KEYS)` — it now exercises all 13 keys
  including templates, which validates the `every-descendant-fill =
  currentColor / none / var(--ac-*)` contract for the new
  `TemplatesIcon`.
- Inside that loop body, line 185 currently asserts
  `expect(svg!.style.color).toBe(\`var(--ac-doc-${docType})\`)`. This
  hardcoded `--ac-doc-` prefix is correct for 12 keys but wrong for
  templates (which resolves to `var(--ac-fg-muted)`). **Update line
  185** to `expect(svg!.style.color).toBe(DOC_TYPE_COLOR_VAR[docType])`,
  importing `DOC_TYPE_COLOR_VAR` from `./Glyph.constants` at the top of
  the file. This keeps the assertion data-driven and exhaustive
  across all 13 keys.

**Add new positive assertions**:
- `<Glyph docType="templates" size={16} />` (unframed) sets inline
  style `color: var(--ac-fg-muted)` and emits `data-doc-type="templates"`
  on the rendered SVG.
- `<Glyph docType="templates" size={16} framed />` (framed branch —
  the EyebrowLabel path) sets inline style `color: var(--ac-fg-muted)`
  on the inner SVG, and emits `data-doc-type="templates"` on both the
  wrapper span and the inner SVG. This explicitly guards the framed
  branch update at `Glyph.tsx:110`.
- `DOC_TYPE_COLOR_VAR['templates'] === 'var(--ac-fg-muted)'`.
- `DOC_TYPE_COLOR_VAR['decisions'] === 'var(--ac-doc-decisions)'`
  (sampled — full exhaustiveness comes from the TS Record type plus the
  existing CSS↔TS parity test).

#### 6. Extend `glyph-resolved-fill.spec.ts` minimally (full extraction in Phase 4)

**File**:
`frontend/tests/visual-regression/glyph-resolved-fill.spec.ts` (extend)

Phase 1 makes only the minimum-needed change: parametrise the existing
spec over `DOC_TYPE_KEYS × ['light', 'dark']`, looking up the expected
hex via `DOC_TYPE_TOKEN_KEY[docType]` against `LIGHT_COLOR_TOKENS` /
`DARK_COLOR_TOKENS`. Keep the existing inline `hexToRgb`/`parseRgb`
helpers and theme-switch boilerplate as-is — Phase 4 owns the helper
extraction and migrates this spec to the shared module in the same
phase. This keeps Phase 1 confined to Glyph-internal changes and
matches the "phases 1–3 can be merged in any order" claim (Phase 4 is
the only phase that touches `lib/expected-colours.ts`).

The `/glyph-showcase` route's grid is built from the `DOC_TYPE_KEYS`
runtime list (after Phase 1 §4 repoints showcase imports), so the
templates cell appears automatically — no route changes needed.

### Success Criteria

#### Automated Verification

- [x] Typecheck passes: `cd skills/visualisation/visualise/frontend && npm run typecheck`
- [~] Lint passes: no `lint` script exists in this project; `tsc -b` (build) acts as the static check and passes.
- [x] Vitest passes including the rewritten Glyph test: `cd skills/visualisation/visualise/frontend && npm run test` (only pre-existing unrelated `scripts/visual-diff-ciede2000.test.ts` pngjs-resolve failure remains)
- [x] `glyph-resolved-fill.spec.ts` passes for all 13 keys × 2 themes
- [x] `glyph-showcase.spec.ts` passes; new templates cell pixel baselines regenerated (darwin via host, linux via Docker) and committed.
- [x] CSS ↔ TS token parity test still passes (no new tokens added): `global.test` green within the vitest run

#### Manual Verification

- [ ] `/glyph-showcase` shows a templates cell at sizes 16/24/32 with a
  muted-grey fill in light theme (`#5f6378`) and light-grey fill in dark
  theme (`#a0a5b8`).
- [ ] `/library` overview hub: the templates tile renders **without** a
  Glyph (preserved behaviour from before this work).
- [ ] Activity Feed rows for templates documents render without a glyph
  (preserved).

---

## Phase 2: Extract shared `EyebrowLabel` and route templates through it

### Overview

Move the private `EyebrowLabel` helper out of `LibraryTypeView.tsx` into
`components/EyebrowLabel/`. Repoint both `LibraryTypeView` and
`LibraryTemplatesIndex` to consume the shared component, and remove the
inline `LayersIcon` from `LibraryTemplatesIndex` in the same diff (it has
no other consumers — Phase 1's `TemplatesIcon` replaces it via Glyph).
The component returns a wrapping `<span data-testid="eyebrow-label">`
so Playwright (and CSS) can target it without depending on internal SVG
structure.

This is the component Phase 4 will use on the detail page.

### Changes Required

#### 1. New shared component

**File**: `frontend/src/components/EyebrowLabel/EyebrowLabel.tsx` (new)

```tsx
import type { ReactElement } from 'react'
import { Glyph } from '../Glyph/Glyph'
import { DOC_TYPE_LABELS, type DocTypeKey } from '../../api/types'

interface Props {
  type: DocTypeKey
}

export function EyebrowLabel({ type }: Props): ReactElement {
  return (
    <span data-testid="eyebrow-label">
      <Glyph docType={type} size={16} framed />
      {DOC_TYPE_LABELS[type].toUpperCase()}
    </span>
  )
}
```

Notes:
- After Phase 1, `Glyph.docType` accepts `DocTypeKey` directly — no
  guard is required. The component renders a Glyph for all 13 keys
  including templates (which resolves to `--ac-fg-muted` via the
  `DOC_TYPE_COLOR_VAR` lookup).
- The wrapping `<span>` is inline (so existing layout is unchanged) and
  carries `data-testid="eyebrow-label"` for selector stability. This
  matches the test-only hook convention used elsewhere by this plan
  (`[data-testid="hub-grid"]`, `[data-testid="related-group-inferred"]`).
- Size and framed are fixed at the EyebrowLabel surface — the eyebrow
  is the only consumer for now. If a future surface needs variation,
  add props at that point (YAGNI).

**File**: `frontend/src/components/EyebrowLabel/EyebrowLabel.test.tsx`
(new — co-located, NOT under `__tests__/`)
**Changes**: Vitest renders `EyebrowLabel` for one non-virtual key
(e.g. `decisions`) and for `templates`; asserts the wrapping span has
`data-testid="eyebrow-label"`, the Glyph is rendered with the correct
`docType`, and the label text matches `DOC_TYPE_LABELS[type].toUpperCase()`.

#### 2. Repoint `LibraryTypeView`

**File**: `frontend/src/routes/library/LibraryTypeView.tsx`
**Changes**:
- Remove the private `EyebrowLabel` function (`:270-279`).
- Drop the now-unused `isGlyphDocTypeKey` import (deleted in Phase 1).
- `import { EyebrowLabel } from '../../components/EyebrowLabel/EyebrowLabel'`.
- The four existing usages at `:159, 170, 189, 201` need no change
  (same name, same prop shape).

#### 3. Repoint `LibraryTemplatesIndex` and remove `LayersIcon`

**File**: `frontend/src/routes/library/LibraryTemplatesIndex.tsx`
**Changes**:
- Replace the inline `<><LayersIcon … />TEMPLATES</>` eyebrow body
  (`:159-164`) with `<EyebrowLabel type="templates" />`.
- Delete the local `LayersIcon` declaration and export at `:23-41`.
  Verify with `grep -r 'LayersIcon' frontend/src` that no other consumer
  exists (the visualiser frontend should not have any). If anything
  unexpectedly imports it, halt and re-scope.
- Leave the per-row `Glyph` call at `:116` unchanged. The
  `glyphKeyForTemplate` fallback `<span className={styles.rowGlyphFallback}>`
  at `:118` is also left alone — replacing it with a templates Glyph
  would misleadingly imply the unknown row is a templates document.

### Success Criteria

#### Automated Verification

- [x] Typecheck passes
- [~] Lint: no `lint` script; typecheck/build green
- [x] Vitest passes including new EyebrowLabel test
- [x] Existing route tests still pass (no `LibraryTypeView` /
  `LibraryTemplatesIndex` regressions; the `TEMPLATES` eyebrow text
  assertion still passes via EyebrowLabel)
- [x] Full Playwright suite passes (tokens `library-templates` full-page
  screenshot stays within tolerance; no baseline regen needed)

#### Manual Verification

- [ ] Listing route (`/library/decisions`, `/library/work-items`, …)
  renders the same eyebrow as pre-change.
- [ ] Templates listing route (`/library/templates`) eyebrow shows the
  templates Glyph (muted-grey `#5f6378` in light, `#a0a5b8` in dark)
  followed by `TEMPLATES`.

---

## Phase 3: Add e2e fixtures + `start-server.mjs` doc_paths

### Overview

Three doc types (`work-item-reviews`, `design-gaps`, `design-inventories`)
have no e2e fixture directories today, so they cannot appear in the AC #1
/ AC #2 / AC #3 loops. This phase adds minimal fixture markdowns and the
matching `doc_paths` entries in the Playwright launcher. No Rust changes
(verified — `server/src/docs.rs:23-39` already wires all three).

### Changes Required

#### 1. New fixture directories + files

**Directory**:
`skills/visualisation/visualise/server/tests/fixtures/meta/reviews/work/`
**File**: `2026-05-26-example-review-1.md` (new)
**Body**: Minimal frontmatter (`type: work-item-review`, `status: draft`,
`work_item: 0001`) plus a one-line "Example work-item review for e2e
fixtures." body.
**Slug note**: `WorkItemReviews` slug derivation strips the
`YYYY-MM-DD-` prefix AND the `-review-N` suffix (`slug.rs:30-33`), so
this filename derives slug `example`. A numeric-prefixed name like
`0001-example.md` would yield slug `None` (no date prefix) and fail to
resolve — match the `reviews/prs/` convention.

**Directory**:
`skills/visualisation/visualise/server/tests/fixtures/meta/research/design-gaps/`
**File**: `2026-05-26-example-gap.md` (new)
**Body**: Minimal frontmatter + one-line body.

**Directory**:
`skills/visualisation/visualise/server/tests/fixtures/meta/research/design-inventories/2026-05-26-example/`
**File**: `inventory.md` (new) — nested manifest, per
`docs.rs:99-104` (`nested_manifest_filename()` special-cases
`DesignInventories`).
**Body**: Minimal frontmatter + one-line body.

Existing fixture directories follow the conventions in
`server/tests/fixtures/meta/decisions/`, `…/work/`, etc. — copy a
sample frontmatter shape and adjust `type:`.

#### 2. `start-server.mjs` doc_paths

**File**: `skills/visualisation/visualise/frontend/e2e/start-server.mjs`
**Changes**: Extend the doc_paths object (`:60-92`) with three entries:

```js
review_work: 'tests/fixtures/meta/reviews/work',
research_design_gaps: 'tests/fixtures/meta/research/design-gaps',
research_design_inventories: 'tests/fixtures/meta/research/design-inventories',
```

Config keys match `DocTypeKey::config_path_key()` mappings at
`server/src/docs.rs:41-60` (verified: `review_work`,
`research_design_gaps`, `research_design_inventories`).

#### 3. Smoke spec (TDD — write first)

**File**:
`frontend/tests/visual-regression/lib/detail-route-slugs.ts` (new)

Define `DETAIL_ROUTE_SLUGS: Record<DocTypeKey, string>` — the slug
component of each doc type's detail route. For the 12 physical doc
types this is a fixture file slug; for `templates` (a virtual doc
type) this is a template name consumed by the template-summaries
endpoint at `/library/templates/<name>`. The asymmetry is noted in
the JSDoc:

```ts
import { type DocTypeKey } from '../../../src/api/types'

/**
 * Canonical slug per doc type for the detail-route URL
 * `/library/<docType>/<slug>`.
 *
 * For physical doc types this is the fixture file's slug (filename
 * minus `.md`). For the virtual `templates` key this is a template
 * NAME — `templates` has no on-disk fixture; its detail route is
 * served by the template-summaries endpoint.
 *
 * Slug values mirror the existing fixtures under
 * `server/tests/fixtures/meta/` as of Phase 3 (which adds the three
 * missing key directories). Verify each route renders by running
 * fixture-coverage.spec.ts.
 *
 * Single source of truth consumed by Phases 3, 4, 5.
 */
export const DETAIL_ROUTE_SLUGS: Record<DocTypeKey, string> = {
  decisions: 'ADR-0001-example-decision',
  'work-items': '0001-first-work-item',
  plans: '2026-01-01-first-plan',
  research: '2026-01-01-first-research',
  'plan-reviews': '2026-01-01-first-plan-review-1',
  'pr-reviews': '2026-01-15-add-config-layer-review-1',
  'work-item-reviews': 'example', // Phase 3 fixture 2026-05-26-example-review-1.md → slug 'example'
  validations: '2026-01-01-first-plan-validation',
  notes: '2026-01-01-first-note',
  'pr-descriptions': '42-add-config-layer',
  'design-gaps': 'example-gap', // Phase 3 fixture 2026-05-26-example-gap.md → slug 'example-gap'
  'design-inventories': 'example', // Phase 3 nested manifest dir 2026-05-26-example/ → slug 'example'
  templates: 'work-item', // template NAME, not a fixture file slug
}
```

Slug values mirror the existing fixtures under
`skills/visualisation/visualise/server/tests/fixtures/meta/` (verified
by directory listing at plan time). Phase 3's three new fixtures use
the slugs annotated above.

**File**:
`frontend/tests/visual-regression/fixture-coverage.spec.ts` (new)
**Changes**: Two looped tests — one for listing routes, one for detail
routes — using direct navigation (deterministic):

```ts
import { test, expect } from '@playwright/test'
import { DOC_TYPE_KEYS } from '../../src/api/types'
import { DETAIL_ROUTE_SLUGS } from './lib/detail-route-slugs'

for (const docType of DOC_TYPE_KEYS) {
  test(`listing route renders for ${docType}`, async ({ page }) => {
    await page.goto(`/library/${docType}`)
    // Listing routes render a Page with eyebrow + heading; <article>
    // is only on detail pages. Use a heading-level-1 probe to assert
    // the page rendered something meaningful (not just the shell).
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
  })

  test(`detail route renders for ${docType}`, async ({ page }) => {
    const slug = DETAIL_ROUTE_SLUGS[docType]
    await page.goto(`/library/${docType}/${slug}`)
    await expect(page.locator('article')).toBeVisible()
  })
}
```

Templates is included via the `DOC_TYPE_KEYS` loop. Its detail route
`/library/templates/<name>` is served by the template-summaries
endpoint (templates is in `VIRTUAL_DOC_TYPE_KEYS`). The spec treats
both code paths uniformly via `DETAIL_ROUTE_SLUGS`.

The spec fails initially for `work-item-reviews`, `design-gaps`,
`design-inventories` because no fixtures exist; it passes after the
fixture and start-server changes.

### Success Criteria

#### Automated Verification

- [x] Frontend build passes; server binary rebuilds via start-server with the new doc_paths
- [x] `fixture-coverage.spec.ts` passes for all 13 doc types (templates detail asserts the tiers layout, not `<article>`, since it is served by LibraryTemplatesView)
- [x] No regressions in existing e2e specs (full `npx playwright test` green). Notes: (1) start-server's `research` doc_paths key was corrected to `research_codebase` to match `config_path_key` so research fixtures load; (2) the EmptyState `.title` case in `typography-resolved-sizes.spec.ts` was removed — populating every doc type leaves no empty listing to reach EmptyState; its font-size compliance stays enforced by `migration.test.ts`.

#### Manual Verification

- [ ] Navigate to `/library/work-item-reviews`, `/library/design-gaps`,
  `/library/design-inventories` in the dev server — listing route
  renders without errors.
- [ ] Click into each new fixture document — detail page renders without
  errors.

---

## Phase 4: Detail-page eyebrow tint (AC #1)

### Overview

Add an eyebrow to `LibraryDocView` using the shared `EyebrowLabel`
component from Phase 2, gated to render only when a document is actually
loaded (not on the loading or "Document not found" branches). Extract
shared spec helpers (`hexToRgb`, `parseRgb`, the `EXPECTED_COLOR` map,
the theme-switch helper) into a single library module so Phases 5 and 6
import unconditionally. Migrate the two existing specs
(`chip-resolved-colours`, `glyph-resolved-fill`) to consume the same
helpers — a mechanical refactor under existing green tests.

Land a visual-regression spec that loops 13 doc-type keys × {light, dark}
themes and asserts the eyebrow icon resolves to the expected RGB.
Depends on Phases 1–3 (Glyph 13-key support, shared EyebrowLabel,
fixtures + DETAIL_ROUTE_SLUGS present for all 13).

### Changes Required

#### 1. Shared spec helpers (write first, mechanical refactor)

**File**: `frontend/tests/visual-regression/lib/expected-colours.ts` (new)

The helper module must match the **existing** `parseRgb` / `hexToRgb`
shapes so migration of `chip-resolved-colours.spec.ts` and
`glyph-resolved-fill.spec.ts` is truly mechanical (find/replace import
paths only — no callsite edits). The existing `parseRgb` returns
`[number, number, number]` (tuple); the existing `hexToRgb` returns
a CSS string `'rgb(r, g, b)'`. The new module preserves both shapes.

`EXPECTED_COLOR` is derived from the typed `DOC_TYPE_TOKEN_KEY` lookup
in `Glyph.constants.ts` (introduced in Phase 1 §2) — no string parsing
of `var(--...)` expressions, no coupling to the production lookup's
string format.

```ts
import {
  LIGHT_COLOR_TOKENS,
  DARK_COLOR_TOKENS,
} from '../../../src/styles/tokens'
import { DOC_TYPE_KEYS, type DocTypeKey } from '../../../src/api/types'
import { DOC_TYPE_TOKEN_KEY } from '../../../src/components/Glyph/Glyph.constants'

export type Theme = 'light' | 'dark'

// Match the existing inline helpers in chip/glyph specs verbatim.
// Copy bodies from chip-resolved-colours.spec.ts:4-42 — do not reshape
// signatures; the chip and glyph specs depend on exact return types
// (tuple for parseRgb, 'rgb(r, g, b)' string for hexToRgb).
export function hexToRgb(hex: string): string { /* copy from chip-resolved-colours.spec.ts */ }
export function parseRgb(str: string): [number, number, number] {
  /* copy from chip-resolved-colours.spec.ts */
}
export function expectChannelsBetween(/* copy signature from chip-resolved-colours.spec.ts */) {
  /* copy body from chip-resolved-colours.spec.ts */
}

const TOKEN_TABLE: Record<Theme, Record<string, string>> = {
  light: LIGHT_COLOR_TOKENS,
  dark: DARK_COLOR_TOKENS,
}

export const EXPECTED_COLOR: Record<DocTypeKey, Record<Theme, string>> =
  Object.fromEntries(
    DOC_TYPE_KEYS.map((key) => {
      const tokenKey = DOC_TYPE_TOKEN_KEY[key]
      return [
        key,
        {
          light: TOKEN_TABLE.light[tokenKey],
          dark: TOKEN_TABLE.dark[tokenKey],
        },
      ]
    }),
  ) as Record<DocTypeKey, Record<Theme, string>>

export async function setTheme(
  page: import('@playwright/test').Page,
  theme: Theme,
): Promise<void> {
  // Matches the convention used by chip-resolved-colours.spec.ts:67-72
  // and glyph-resolved-fill.spec.ts:25-29.
  await page.evaluate((t) => {
    document.documentElement.dataset.theme = t
  }, theme)
  await page.waitForFunction(
    (t) => document.documentElement.dataset.theme === t,
    theme,
  )
}
```

Migrate `chip-resolved-colours.spec.ts` and `glyph-resolved-fill.spec.ts`
to import `hexToRgb`/`parseRgb`/`expectChannelsBetween`/`setTheme` from
this module (delete the inline copies). Because the helper shapes match
exactly, no callsite edits are needed. Both specs must still pass after
the refactor — run them as part of Phase 4 success criteria.

#### 2. Detail-page eyebrow spec (write first)

**File**:
`frontend/tests/visual-regression/detail-eyebrow-resolved-colours.spec.ts` (new)

```ts
import { test, expect } from '@playwright/test'
import { DOC_TYPE_KEYS } from '../../src/api/types'
import { EXPECTED_COLOR, hexToRgb, setTheme } from './lib/expected-colours'
import { DETAIL_ROUTE_SLUGS } from './lib/detail-route-slugs'
import { LIGHT_COLOR_TOKENS, DARK_COLOR_TOKENS } from '../../src/styles/tokens'

const THEMES = ['light', 'dark'] as const

for (const theme of THEMES) {
  test.describe(`eyebrow icon colour — ${theme}`, () => {
    for (const docType of DOC_TYPE_KEYS) {
      test(`${docType}`, async ({ page }) => {
        await page.goto(`/library/${docType}/${DETAIL_ROUTE_SLUGS[docType]}`)
        await setTheme(page, theme)

        // Target the SVG directly — data-doc-type is on the svg for both
        // framed and unframed Glyphs. Scope to the eyebrow region.
        const icon = page.locator(
          `[data-slot="eyebrow"] svg[data-doc-type="${docType}"]`,
        )
        await expect(icon).toBeVisible()
        const color = await icon.evaluate(
          (el) => getComputedStyle(el).color,
        )
        // String-compare two 'rgb(r, g, b)' values — the convention
        // used by chip-resolved-colours.spec.ts and glyph-resolved-fill.spec.ts.
        expect(color).toBe(hexToRgb(EXPECTED_COLOR[docType][theme]))
      })
    }
  })

  // Single theme-level assertion: the eyebrow LABEL TEXT colour resolves
  // to --ac-fg-faint (theme-invariant across all 13 doc types).
  //
  // We read `color` from the wrapping eyebrow-label span. The text node
  // inherits `color` from this span (which inherits from .eyebrow's
  // --ac-fg-faint rule); the Glyph's inline `color` is set on the
  // INNER <svg>, so it does NOT propagate upward to the span.
  test(`eyebrow label text colour is --ac-fg-faint — ${theme}`, async ({
    page,
  }) => {
    await page.goto(`/library/decisions/${DETAIL_ROUTE_SLUGS.decisions}`)
    await setTheme(page, theme)
    const eyebrowText = page.locator(
      '[data-slot="eyebrow"] [data-testid="eyebrow-label"]',
    )
    const color = await eyebrowText.evaluate(
      (el) => getComputedStyle(el).color,
    )
    const tokens = theme === 'light' ? LIGHT_COLOR_TOKENS : DARK_COLOR_TOKENS
    expect(color).toBe(hexToRgb(tokens['ac-fg-faint']))
  })
}
```

Selector notes:
- `svg[data-doc-type="${docType}"]` resolves directly to the SVG
  element where the inline `color` is set (avoids matching framed-Glyph
  wrapper spans, which have a different inherited `color`).
- The label-text colour assertion runs **once per theme**, not per doc
  type, because `--ac-fg-faint` is invariant across keys — guarding
  against an accidental tint regression without 13× duplication.
- The `[data-testid="eyebrow-label"]` selector comes from Phase 2's
  wrapping span (test-only hook, matches the `data-testid` pattern
  used by `[data-testid="hub-grid"]` and `[data-testid="related-group-inferred"]`
  elsewhere in this plan).

The spec fails because `LibraryDocView` does not render an eyebrow yet.

#### 3. `LibraryDocView` change

**File**: `frontend/src/routes/library/LibraryDocView.tsx`
**Changes**: Import `EyebrowLabel`; pass it to `<Page>` **only when a
document is actually loaded**. The render reaches `<Page>` not just on
the happy path but also on the loading ("Loading…") and "Document not
found" branches — rendering a tinted doc-type eyebrow above a missing
document would be misleading.

```tsx
import { EyebrowLabel } from '../../components/EyebrowLabel/EyebrowLabel'

// …
const hasResolvedDocument = Boolean(entry && content.data)

return (
  <Page
    eyebrow={hasResolvedDocument ? <EyebrowLabel type={type} /> : undefined}
    title={title}
    subtitle={subtitle}
  >
    {body}
  </Page>
)
```

`type` is the narrowed `DocTypeKey` already in scope at `:41-42`. The
gate ensures the eyebrow only appears once the document has resolved.

### Success Criteria

#### Automated Verification

- [x] `detail-eyebrow-resolved-colours.spec.ts` passes for all 26
  combinations (13 keys × 2 themes) plus 2 label-text assertions
- [x] Migrated `chip-resolved-colours.spec.ts` and
  `glyph-resolved-fill.spec.ts` still pass (helper extraction is a
  no-op refactor)
- [x] Typecheck passes (no `lint` script in this project)
- [x] Full Playwright suite green (cross-refs, wiki-links, mermaid,
  navigation included)

#### Manual Verification

- [ ] Open one detail page per doc type in light + dark themes —
  eyebrow icon colour matches the colour the same doc type carries on
  the listing-route eyebrow (and, for the 12 non-virtual keys, the
  overview hub card).
- [ ] `templates` detail page (`/library/templates/<name>`) — eyebrow
  icon resolves to `#5f6378` in light, `#a0a5b8` in dark.
- [ ] Eyebrow label text appears identical to pre-change (it inherits
  `--ac-fg-faint` from `.eyebrow`).
- [ ] Navigate to a non-existent slug (e.g.
  `/library/decisions/no-such-doc`) — the "Document not found" page
  renders **without** an eyebrow.

---

## Phase 5: RelatedArtifacts per-row icon (AC #2)

### Overview

Insert a `<Glyph docType={entry.type} size={16}>` (unframed,
`aria-hidden`) into every row of `RelatedArtifacts`. Scope the new
spec's locators to the aside region via a `data-testid` attached to
`LibraryDocView`'s wrapping `<section>` (RelatedArtifacts itself
returns a Fragment with no root element — the section wrapper lives in
the parent route).

**AC #2 scope decision (pinned by pass-4 review)**: The work-item AC #2
originally required a `templates`-typed row in the aside. Verification
against `server/src/indexer.rs:272-274` and `clusters.rs:37-39`
revealed that **the backend never emits a `templates` row in any
RelatedArtifacts response** — templates are excluded from both
indexing and inferred clustering by design (they have no on-disk
fixture). AC #2 is therefore **descoped to the 12 non-virtual doc
types**; templates Glyph rendering is verified indirectly by the
Phase 4 eyebrow spec (templates detail page) and the Phase 6 listing
eyebrow spec (`/library/templates`). A follow-up edit to the work
item's AC #2 wording is listed in "Out-of-band followups" below.

**Fixture strategy (inferred-cluster based)**: The backend's
inferred-cluster algorithm groups documents by their post-prefix slug
(`clusters.rs:34-67` matches on exact post-strip slug equality). The
12 non-virtual doc types use distinct prefix conventions
(`NNNN-` for work-items, `ADR-NNNN-` for decisions, `YYYY-MM-DD-` for
plans/research/notes/etc.) — but their **post-prefix slugs** can be
made identical. We add 12 sibling fixtures all named with the same
post-prefix slug `ac2-coverage`. Navigating to any one detail page
surfaces the other 11 as inferred-cluster siblings, giving a single
12-row aside with one row of every non-virtual type — and no backend
changes.

Pre-commit to `align-items: center` on the row container so the icon
aligns visually with the anchor and badge. Depends on Phases 1, 3, and
the helper module from Phase 4.

### Changes Required

#### 1. Spec scoping — no new section testid needed

The Phase 5 spec scopes its locators to the per-group testids added in
§4 (`related-group-inferred`, etc.), which sit on the `.group`
wrappers *inside* `RelatedArtifacts`. These already isolate the row
icons from the detail-page eyebrow Glyph (which is outside any
`related-group-*` element), so **no section-level `data-testid` on
LibraryDocView is required**. (Earlier passes proposed a
`data-testid="related-artifacts"` on the LibraryDocView `<section>`,
but the finer group-level testids supersede it — adding a redundant,
unasserted hook would just widen the test surface.)

#### 2. Spec — write first

**File**:
`frontend/tests/visual-regression/aside-row-resolved-colours.spec.ts` (new)

```ts
import { test, expect } from '@playwright/test'
import { DOC_TYPE_KEYS, isPhysicalDocTypeKey } from '../../src/api/types'
import { EXPECTED_COLOR, hexToRgb, setTheme } from './lib/expected-colours'

// 12 non-virtual doc types. AC #2 templates row is descoped — see
// Phase 5 Overview for the rationale (backend doesn't emit templates
// rows in any RelatedArtifacts response). Reuse the production
// predicate rather than an inline `!VIRTUAL_DOC_TYPE_KEYS.includes(k)`
// filter — after Phase 1 narrows VIRTUAL_DOC_TYPE_KEYS to
// `readonly ['templates']`, the inline `.includes(k)` would reject a
// `DocTypeKey`-typed argument at typecheck.
const PHYSICAL_DOC_TYPE_KEYS = DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey)

// Navigate to any one of the 12 sibling 'ac2-coverage' fixtures —
// the other 11 surface in the inferred-cluster section (see §3).
// Owned by AC #2.
const ANCHOR_URL = '/library/work-items/0099-ac2-coverage'

const THEMES = ['light', 'dark'] as const

// Up-front coverage guard. If a sibling fixture renames or breaks its
// shared slug, this fails with a clear "expected 12 row icons, got N"
// signal naming the anchor fixture family in the error message.
test('inferred-cluster surfaces one row per non-virtual doc type', async ({
  page,
}) => {
  await page.goto(ANCHOR_URL)
  const rowIcons = page.locator(
    '[data-testid="related-group-inferred"] svg[data-doc-type]',
  )
  await expect(rowIcons).toHaveCount(PHYSICAL_DOC_TYPE_KEYS.length - 1, {
    // -1 because the anchor doc itself is not listed as its own
    // cluster sibling (cluster excludes the source entry). The
    // remaining 11 cover the other non-virtual types.
    timeout: 5000,
  })
})

for (const theme of THEMES) {
  test.describe(`related-artifacts row icon colour — ${theme}`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto(ANCHOR_URL)
      await setTheme(page, theme)
    })

    // Loop over the 11 expected sibling types (all non-virtual except
    // 'work-items', which is the anchor doc's own type and therefore
    // not in its own inferred cluster).
    for (const target of PHYSICAL_DOC_TYPE_KEYS.filter((k) => k !== 'work-items')) {
      test(`row icon for ${target}`, async ({ page }) => {
        // Scope to the inferred-cluster region; target the SVG directly.
        const icon = page.locator(
          `[data-testid="related-group-inferred"] svg[data-doc-type="${target}"]`,
        )
        await expect(icon).toBeVisible()
        const color = await icon.evaluate(
          (el) => getComputedStyle(el).color,
        )
        expect(color).toBe(hexToRgb(EXPECTED_COLOR[target][theme]))

        // Aria-hidden contract: row icons are decorative; the adjacent
        // anchor text carries the row's accessible name.
        expect(await icon.getAttribute('aria-hidden')).toBe('true')
        expect(await icon.getAttribute('role')).toBeNull()
      })
    }

    // Cover the work-items case by navigating to a sibling anchor of
    // a different doc type, so the work-items ac2-coverage fixture
    // appears as one of its cluster siblings.
    test('row icon for work-items (via sibling anchor)', async ({ page }) => {
      await page.goto('/library/decisions/ADR-0099-ac2-coverage')
      await setTheme(page, theme)
      const icon = page.locator(
        '[data-testid="related-group-inferred"] svg[data-doc-type="work-items"]',
      )
      await expect(icon).toBeVisible()
      const color = await icon.evaluate(
        (el) => getComputedStyle(el).color,
      )
      expect(color).toBe(hexToRgb(EXPECTED_COLOR['work-items'][theme]))
    })
  })
}

// Row-container layout invariance: AC #2 requires that adding the icon
// does not change the row container's background/border styling.
test.describe('related-artifacts container invariance', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(ANCHOR_URL)
    // Pin theme even though the assertions in this describe are
    // theme-invariant by construction.
    await setTheme(page, 'light')
  })

  test('row container background and border-width are unchanged', async ({
    page,
  }) => {
    // Scope to the inferred group (the only group rendered by this
    // anchor — declared-outbound/inbound are not populated by the
    // ac2-coverage fixture family by design).
    const row = page.locator(
      '[data-testid="related-group-inferred"] li',
    ).first()
    const bg = await row.evaluate((el) => getComputedStyle(el).backgroundColor)
    // .groupItem sets no background — assert the transparent default.
    expect(bg).toBe('rgba(0, 0, 0, 0)')
    // .groupItem sets no border — assert the 0px default.
    const borderWidth = await row.evaluate(
      (el) => getComputedStyle(el).borderTopWidth,
    )
    expect(borderWidth).toBe('0px')
  })

  test('inferred group border is 2px dashed', async ({ page }) => {
    const group = page.locator('[data-testid="related-group-inferred"]')
    expect(await group.evaluate((el) => getComputedStyle(el).borderLeftWidth))
      .toBe('2px')
    expect(await group.evaluate((el) => getComputedStyle(el).borderLeftStyle))
      .toBe('dashed')
  })

  test('row uses align-items: center for icon/text/badge alignment', async ({
    page,
  }) => {
    const row = page.locator(
      '[data-testid="related-group-inferred"] li',
    ).first()
    expect(await row.evaluate((el) => getComputedStyle(el).alignItems))
      .toBe('center')
  })
})
```

Selector notes:
- The icon locator scopes to
  `[data-testid="related-group-inferred"]` — the only RelatedGroup
  the `ac2-coverage` fixture family surfaces. The detail-page
  eyebrow Glyph (also carrying `data-doc-type`) is outside this scope.
- `svg[data-doc-type=…]` targets the SVG directly — for unframed
  Glyphs, `data-doc-type` is set on the svg element itself
  (`Glyph.tsx:130`).
- Group testids use `related-group-declared-outbound` /
  `related-group-declared-inbound` / `related-group-inferred` —
  three distinct values, since both Targets (declared-outbound) and
  Referenced by (declared-inbound) blocks pass `kind="declared"` and
  would otherwise be indistinguishable. Phase 5 §4 wires all three;
  Phase 5's spec only asserts on the inferred testid because that's
  the only group the `ac2-coverage` fixtures surface. The declared
  testids are exercised indirectly by their wiring being live in
  other detail pages (e.g. plan-reviews' aside, which uses the
  existing `target:` mechanism).
- `border-color` is NOT asserted — when `border-width: 0` browsers
  resolve `border-color` to `currentColor`, which is theme-dependent.
- `align-items: center` is explicitly asserted as the deliberate
  layout change (not invariance) — see §5.

Spec fails because rows have no `<svg>` today.

#### 3. Twelve sibling `ac2-coverage` fixtures (inferred-cluster strategy)

**Directory**: `skills/visualisation/visualise/server/tests/fixtures/meta/`

Add 12 sibling fixtures — one per non-virtual `DocTypeKey` — all
sharing the same **post-prefix slug** `ac2-coverage`. The backend's
inferred-cluster algorithm groups by post-prefix slug (`clusters.rs:
34-67`), so all 12 cluster together. Navigating to any one of them
shows the other 11 in its inferred-cluster section.

Per-type filename conventions (paths relative to
`server/tests/fixtures/meta/`, mirroring existing
`config_path_key` mappings at `server/src/docs.rs:41-60`):

```
work/0099-ac2-coverage.md                                    (work-items)
decisions/ADR-0099-ac2-coverage.md                           (decisions)
plans/2026-05-26-ac2-coverage.md                             (plans)
research/2026-05-26-ac2-coverage.md                          (research)
reviews/plans/2026-05-26-ac2-coverage-review-1.md            (plan-reviews)
reviews/prs/2026-05-26-ac2-coverage-review-1.md              (pr-reviews)
reviews/work/2026-05-26-ac2-coverage-review-1.md             (work-item-reviews)
validations/2026-05-26-ac2-coverage.md                       (validations)
notes/2026-05-26-ac2-coverage.md                             (notes)
prs/2026-05-26-ac2-coverage.md                               (pr-descriptions)
research/design-gaps/2026-05-26-ac2-coverage.md              (design-gaps)
research/design-inventories/2026-05-26-ac2-coverage/inventory.md  (design-inventories)
```

The post-prefix slug derivation per type, verified against
`slug.rs:14-86` — `derive()`:

- `work/0099-ac2-coverage.md` → `strip_prefix_work_item_id` (strip
  leading `NNNN-`) → `ac2-coverage` ✓
- `decisions/ADR-0099-ac2-coverage.md` → `strip_prefix_numbered("ADR-")`
  → `ac2-coverage` ✓
- `plans` / `research` / `validations` / `notes` / `pr-descriptions` /
  `design-gaps` → `strip_prefix_date` (strip leading `YYYY-MM-DD-`) →
  `ac2-coverage` ✓ — **NOTE**: `pr-descriptions` also uses
  `strip_prefix_date`, so its fixture MUST use a date prefix
  (`prs/2026-05-26-ac2-coverage.md`), NOT a numeric prefix — a
  `prs/0099-…` filename would yield slug `None` and silently drop out
  of the cluster.
- `reviews/plans/…`, `reviews/prs/…`, `reviews/work/…` →
  `strip_prefix_date` then `strip_suffix_review_n` → `ac2-coverage` ✓
- `research/design-inventories/2026-05-26-ac2-coverage/inventory.md` →
  slug derives from the parent directory name `2026-05-26-ac2-coverage`
  via `strip_prefix_date` → `ac2-coverage` ✓

Each fixture's frontmatter carries minimal type-appropriate fields
plus a top-of-file ownership comment:

```yaml
# Owned by meta/work/0074 Phase 5 / AC #2.
# Slug stem 'ac2-coverage' MUST match across all 12 sibling fixtures
# under server/tests/fixtures/meta/. Removing or renaming any sibling
# breaks tests/visual-regression/aside-row-resolved-colours.spec.ts.
# Carry NO cross-reference keys (work_item_id / work-item / ticket /
# parent / related / target) — see note below.
```

**Critical fixture constraint — no cross-references.** The backend
moves any sibling that ALSO appears in a *declared* relation out of the
inferred cluster (`api/related.rs:91-102` dedups declared-overlapping
entries from `inferredCluster`). Declared membership is driven by the
ref keys `work_item_id` / `work-item` / `ticket` / `parent` /
`related` (`frontmatter.rs:305`) and plan-review `target:`
(`indexer.rs:613`). If any `ac2-coverage` fixture carries one of these
pointing at a sibling (e.g. the work-items fixture's id `0099` matched
by another sibling's `related:`), that sibling drops out of the
inferred cluster and the `toHaveCount(11)` assertion plus the
per-target `toBeVisible()` fail for reasons unrelated to the icon
feature. **Every `ac2-coverage` fixture must be ref-free** — minimal
`type:` / `status:` / title frontmatter only, no ref keys.

This decouples the spec from natural evolution of demo fixtures (a
contributor editing `decisions/ADR-0001-example-decision.md` cannot
break AC #2) and avoids dependence on any specific backend mechanism
beyond the well-tested inferred-cluster behaviour.

**Note**: This strategy populates only the inferred-cluster group in
the anchor's aside. The declared-outbound and declared-inbound groups
are not exercised by AC #2 because the existing backend's mechanisms
(`target:` for plan-reviews; work-item refs for work-items) cannot
surface a row of arbitrary doc type from a single anchor. The
declared-group testids are wired in §4 (so future fixtures and
real-world detail pages benefit from them) but are not asserted by
this phase's spec.

#### 4. `RelatedArtifacts.tsx` change

**File**:
`frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx`
**Changes**:

1. Insert a `<Glyph>` before the anchor in each row:

```tsx
import { Glyph } from '../Glyph/Glyph'

// …
<li key={entry.path} className={styles.groupItem}>
  <Glyph docType={entry.type} size={16} />
  <a href={`/library/${entry.type}/${fileSlugFromRelPath(entry.relPath)}`}>
    {entry.title || entry.relPath}
  </a>
  <span className={`${styles.badge} ${badgeClass}`}>{kind}</span>
</li>
```

2. Add a `data-testid` to each `RelatedGroup`'s wrapping `<div
className={styles.group}>` so the spec can locate the three groups
distinctly. `RelatedArtifacts` invokes `RelatedGroup` three times
today — twice with `kind="declared"` (the "Targets" outbound block
and the "Referenced by" inbound block) and once with `kind="inferred"`
(rendered with the heading label "Same lifecycle"; see
`RelatedArtifacts.tsx:53`). Today `RelatedGroup`'s prop signature is
`{ label, entries, kind }` — **add an optional `testId?: string`
prop** and render it on the wrapper:

```tsx
// RelatedGroup signature — add testId:
function RelatedGroup({
  label,
  entries,
  kind,
  testId,
}: {
  label: string
  entries: IndexEntry[]
  kind: 'declared' | 'inferred'
  testId?: string
}) {
  return (
    <div className={styles.group} data-testid={testId}>
      {/* … */}
    </div>
  )
}

// RelatedArtifacts.tsx call sites — pass distinguishing testIds:
<RelatedGroup kind="declared" testId="related-group-declared-outbound" … />
<RelatedGroup kind="declared" testId="related-group-declared-inbound" … />
<RelatedGroup kind="inferred" testId="related-group-inferred" … />
```

Three distinct testid values prevent the ambiguity of two
`declared`-kind groups carrying the same attribute. Stable hooks in
the `data-testid` namespace match the convention used by
`[data-testid="hub-grid"]`. (Note: the "inferred" group's visible
heading is "Same lifecycle", not "Inferred cluster" — the spec scopes
by testid, not heading text, so this is just a manual-verification
naming note.)

Notes:
- No `isGlyphDocTypeKey` / `isPhysicalDocTypeKey` guard — `Glyph.docType`
  accepts `DocTypeKey` directly after Phase 1; every doc type
  including templates renders.
- `ariaLabel` is **omitted**, which causes `Glyph` to render the icon
  with `aria-hidden="true"` (`Glyph.tsx:86-89`). The row's accessible
  name comes from the adjacent `<a>` text; the icon is decorative.
  Do NOT pass `ariaLabel=""` — that would emit `role="img"` with an
  empty accessible name (an axe-core anti-pattern).

#### 5. CSS — align row, no other changes

**File**:
`frontend/src/components/RelatedArtifacts/RelatedArtifacts.module.css`
**Changes**:
- Change `.groupItem` `align-items: baseline` → `align-items: center`
  (`:30`). The 16×16 SVG has no text baseline, so `baseline` would
  leave the icon riding high against the anchor's text baseline.
  `center` aligns icon, anchor, and badge on a shared vertical axis.
  Note: this is a deliberate layout change for the icon-bearing row,
  not invariance; the spec asserts the new value (see §2). Add a
  comment in the CSS: `/* Aligns Glyph (16×16 SVG, no text baseline)
  with anchor text — see meta/work/0074. */`.
- Keep the existing `gap: 0.4rem` (`:31`).
- Do NOT add `background-color`, `border-color`, or `border-width` to
  `.groupItem` — the spec asserts these remain absent.

#### 6. Update `RelatedArtifacts.test.tsx` (Vitest — write before §4)

**File**:
`frontend/src/components/RelatedArtifacts/RelatedArtifacts.test.tsx`
**Changes** (TDD: extend the suite first; §4 JSX change makes it pass):

- Each rendered row now contains a `<svg>` with `aria-hidden="true"`,
  no `role` attribute, and `data-doc-type` matching the row's
  `entry.type`. Assert all three for at least one declared row and
  one inferred row.
- The three `<RelatedGroup>` wrappers carry the testids
  `related-group-declared-outbound`, `related-group-declared-inbound`,
  and `related-group-inferred`. Render a synthetic component-level
  fixture exercising all three group kinds (Vitest renders the
  component directly with stub data — no server required) and assert
  each testid appears exactly once.
- Existing assertions that count rows or match link text are updated
  to ignore icon elements (e.g. match `entry.title` rather than full
  element text content).

**Implementation order within Phase 5**:

The TDD sequence is §6 → §1 → §2 → §4 → §5. §6 (Vitest) and §2
(Playwright spec) are both written first; §4 (JSX edit) and §5 (CSS
edit) make them pass. §1 (testid on LibraryDocView) and §3 (sibling
fixtures) are infrastructure that §2 depends on.

### Success Criteria

#### Automated Verification

- [x] `aside-row-resolved-colours.spec.ts` passes for all 12
  non-virtual target types × 2 themes (incl. work-items via a sibling
  anchor) plus aria-hidden assertions.
- [x] Up-front `toHaveCount(11)` coverage assertion passes.
- [x] Container-invariance assertions pass (inferred group border,
  row background, row border-width, row align-items).
- [x] Typecheck passes (no `lint` script); full Playwright suite + vitest green.

#### Manual Verification

- [ ] Open any one of the 12 `ac2-coverage` fixtures (e.g.
  `/library/work-items/0099-ac2-coverage`) — the aside's
  inferred-cluster section shows the other 11 rows, each carrying the
  appropriately tinted glyph for its doc type.
- [ ] Visual alignment: icon, anchor text, and inferred badge sit on
  a shared vertical centreline; gap of 0.4rem looks balanced.
- [ ] Toggle theme — all 12 non-virtual icons collapse to white in
  dark (templates row is intentionally absent — see Overview).

### Out-of-band followups

- [x] Update `meta/work/0074-per-doc-type-hues-on-detail-page.md`
  AC #2 wording to scope to "12 non-virtual doc types" and add a
  Drafting Note explaining the templates descope (backend never
  emits a templates row in any RelatedArtifacts response;
  `indexer.rs:272-274`, `clusters.rs:37-39`). This is a documentation
  edit; the implementation in this plan delivers what the revised AC
  actually requires.

---

## Phase 6: Non-regression spec for HubCard + listing EyebrowLabel (AC #3)

### Overview

AC #3 in the work item asserts that "sidebar and library hub consumption
is unchanged". Sidebar has no `--ac-doc-*` consumption today and stays
that way; the reframed AC #3 covers the two real Glyph consumer surfaces:
`HubCard` (overview hub) and the listing-route `EyebrowLabel`. Phase 1
gated `templates` out of `HubCard` (via `isPhysicalDocTypeKey(docType.id)`
check, preserving current "no glyph" behaviour for the templates hub
tile), so the spec asserts:

- For the 12 non-virtual doc types: hub-card glyph and listing-route
  eyebrow glyph resolve to the expected `--ac-doc-<key>` RGB.
- For `templates`: the hub card has NO glyph; the listing-route eyebrow
  glyph resolves to `--ac-fg-muted`.

Two `describe` blocks split the contract by surface so future failures
distinguish "hub regression" from "eyebrow regression".

Independent of Phases 4 and 5 — it tests existing-behaviour-preservation,
not new behaviour.

### Changes Required

#### 1. Add `data-testid` to LibraryOverviewHub grid

**File**: `frontend/src/routes/library/LibraryOverviewHub.tsx`
**Changes**: Add `data-testid="hub-grid"` to the wrapper element of the
HubCard grid (the existing container at the level above the per-card
loop). Lets the spec scope `data-doc-type` locators to the hub region
unambiguously.

#### 2. Spec — write first

**File**:
`frontend/tests/visual-regression/non-regression-glyph-consumers.spec.ts` (new)

```ts
import { test, expect } from '@playwright/test'
import { DOC_TYPE_KEYS, isPhysicalDocTypeKey } from '../../src/api/types'
import { EXPECTED_COLOR, hexToRgb, setTheme } from './lib/expected-colours'

const THEMES = ['light', 'dark'] as const
// Reuse the production predicate (see Phase 5 note) — avoids the
// narrowed-`includes` typecheck failure.
const NON_VIRTUAL_KEYS = DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey)

for (const theme of THEMES) {
  test.describe(`hub-card glyph colour — ${theme}`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto('/library')
      await setTheme(page, theme)
    })

    for (const docType of NON_VIRTUAL_KEYS) {
      test(`${docType}`, async ({ page }) => {
        // Target the SVG inside the framed HubCard Glyph. Framed
        // Glyphs put data-doc-type on both wrapper span and svg;
        // svg[data-doc-type] disambiguates.
        const glyph = page.locator(
          `[data-testid="hub-grid"] svg[data-doc-type="${docType}"]`,
        )
        await expect(glyph).toBeVisible()
        const color = await glyph.evaluate(
          (el) => getComputedStyle(el).color,
        )
        // String-compare matches the convention in Phases 4 and 5
        // and the existing chip/glyph specs.
        expect(color).toBe(hexToRgb(EXPECTED_COLOR[docType][theme]))
      })
    }

    test('templates hub card has NO glyph', async ({ page }) => {
      // Templates is intentionally gated out of HubCard rendering by
      // Phase 1. This guards against accidental regression of that gate.
      const glyph = page.locator(
        '[data-testid="hub-grid"] svg[data-doc-type="templates"]',
      )
      await expect(glyph).toHaveCount(0)
    })
  })

  test.describe(`listing-route eyebrow glyph colour — ${theme}`, () => {
    for (const docType of DOC_TYPE_KEYS) {
      test(`${docType}`, async ({ page }) => {
        await page.goto(`/library/${docType}`)
        await setTheme(page, theme)
        const glyph = page.locator(
          `[data-slot="eyebrow"] svg[data-doc-type="${docType}"]`,
        )
        await expect(glyph).toBeVisible()
        const color = await glyph.evaluate(
          (el) => getComputedStyle(el).color,
        )
        expect(color).toBe(hexToRgb(EXPECTED_COLOR[docType][theme]))
      })
    }
  })
}
```

Selector notes:
- All locators use `svg[data-doc-type="…"]` to target the SVG directly
  (avoiding framed-Glyph wrapper-span ambiguity).
- Hub-card locators scope via `[data-testid="hub-grid"]`; eyebrow
  locators scope via `[data-slot="eyebrow"]`. Both are stable
  attributes.
- The two `describe` blocks isolate failures: a hub-card colour
  regression and an eyebrow colour regression produce distinct,
  attributable failures.

### Success Criteria

#### Automated Verification

- [x] `non-regression-glyph-consumers.spec.ts` passes for all
  hub-card cases (12 non-virtual × 2 themes + 2 templates-has-no-glyph
  checks) and all eyebrow cases (13 × 2 themes).
- [x] All other visual-regression specs (chip, glyph, code-block,
  fixture-coverage, detail-eyebrow, aside-row) pass — full suite green.

#### Manual Verification

- [ ] `/library` overview hub: 12 doc-type cards show framed tinted
  glyphs (light: per-doc-type hex; dark: white). Templates card shows
  NO glyph (preserved behaviour).
- [ ] `/library/<each-type>` listing route — eyebrow tint matches the
  hub card for the 12 non-virtual keys.
- [ ] `/library/templates` listing route eyebrow shows the muted
  templates Glyph (`#5f6378` light, `#a0a5b8` dark).

---

## Testing Strategy

### Unit Tests (Vitest)

- `Glyph.test.tsx` — invert the 12-key exclusion assertions (`@ts-expect-error`,
  length-12, exclusion-from-DOC_TYPE_KEYS, `isGlyphDocTypeKey(templates) ===
  false`); add positive cases for `templates → var(--ac-fg-muted)` and the
  `DOC_TYPE_COLOR_VAR` lookup; expand `describe.each` to all 13 keys
  (Phase 1).
- `EyebrowLabel.test.tsx` — new component, two cases (non-virtual key
  + templates), co-located (NOT under `__tests__/`) (Phase 2).
- `RelatedArtifacts.test.tsx` — update to assert each row contains a
  `<svg aria-hidden="true">` with matching `data-doc-type` and no
  `role` attribute (Phase 5).

### Integration / Visual-Regression Tests (Playwright)

- `glyph-resolved-fill.spec.ts` — parametrised over all 13 keys × 2
  themes; migrated to consume `lib/expected-colours.ts` (Phase 1 + Phase 4).
- `chip-resolved-colours.spec.ts` — migrated to consume
  `lib/expected-colours.ts` (Phase 4, mechanical refactor).
- `lib/expected-colours.ts` — new shared helper module with `hexToRgb`,
  `parseRgb`, `setTheme`, `EXPECTED_COLOR` (Phase 4).
- `lib/detail-route-slugs.ts` — new shared `DETAIL_ROUTE_SLUGS` map (Phase 3).
- `fixture-coverage.spec.ts` — new, smoke test for all 13 listing +
  detail routes (Phase 3).
- `detail-eyebrow-resolved-colours.spec.ts` — new, AC #1 (Phase 4).
- `aside-row-resolved-colours.spec.ts` — new, AC #2 (Phase 5).
- `non-regression-glyph-consumers.spec.ts` — new, AC #3 (Phase 6).
- Existing e2e specs (`cross-refs`, `wiki-links`, `mermaid`,
  `navigation`) — non-regression coverage.

### Manual Testing Steps

1. Start the dev frontend with a real meta directory:
   `cd skills/visualisation/visualise/frontend && npm run dev`
2. Open `/library` — verify every doc-type card shows a tinted glyph.
3. Open `/library/<each-type>` — verify eyebrow tint.
4. Open `/library/<each-type>/<slug>` — verify detail-page eyebrow
   tint matches the listing route.
5. On a document with rich related artifacts (e.g. an anchor work
   item), verify every aside row shows a tinted glyph.
6. Toggle theme — verify all glyphs collapse to white in dark.
7. Navigate to `/library/templates` and `/library/templates/<name>` —
   verify the layers glyph renders in muted grey in light, white in
   dark.

## Performance Considerations

The additions are pure rendering — one extra `<svg>` per detail-page
eyebrow and one per RelatedArtifacts row. Aside rows typically number
in the tens; impact is negligible. No new network requests, no new
state, no new computation. The existing `Glyph` already enforces a
fixed three-size policy (16/24/32) — no layout thrash.

## Migration Notes

None. No data shape changes; no persisted state changes; no API
contract changes. The frontend renders a new icon on existing routes.

## References

- Original work item: `meta/work/0074-per-doc-type-hues-on-detail-page.md`
- Pre-implementation research:
  `meta/research/codebase/2026-05-24-0074-per-doc-type-hues-on-detail-page.md`
- Plan review 1:
  `meta/reviews/plans/2026-05-26-0074-per-doc-type-hues-on-detail-page-review-1.md`
- Sibling work items: 0037 (Glyph component, done), 0041 (Library page
  wrapper, done), 0073 (brand-layer palette, ready), 0079 (aside
  redesign — blocked by this), 0082 (BigGlyph — blocked by this)
- Token declarations: `frontend/src/styles/global.css:100-126,
  328-353, 388-411`
- TS resolved hex: `frontend/src/styles/tokens.ts:35-46, 50-61,
  94-105, 109-120`
- `DocTypeKey`: `frontend/src/api/types.ts:4-49`
- `Glyph`: `frontend/src/components/Glyph/Glyph.tsx`,
  `Glyph.constants.ts`, `Glyph.module.css`
- `Page`: `frontend/src/components/Page/Page.tsx`,
  `Page.module.css`
- `RelatedArtifacts`:
  `frontend/src/components/RelatedArtifacts/RelatedArtifacts.tsx`,
  `RelatedArtifacts.module.css`
- `LibraryDocView`: `frontend/src/routes/library/LibraryDocView.tsx`
- `LibraryTypeView`: `frontend/src/routes/library/LibraryTypeView.tsx`
- `LibraryTemplatesIndex`:
  `frontend/src/routes/library/LibraryTemplatesIndex.tsx`
- `LibraryOverviewHub`:
  `frontend/src/routes/library/LibraryOverviewHub.tsx`
- E2e infra: `frontend/playwright.config.ts`,
  `frontend/e2e/start-server.mjs`
- Rust server doc-type registry:
  `skills/visualisation/visualise/server/src/docs.rs`
- Model specs:
  `frontend/tests/visual-regression/chip-resolved-colours.spec.ts`,
  `frontend/tests/visual-regression/glyph-resolved-fill.spec.ts`
