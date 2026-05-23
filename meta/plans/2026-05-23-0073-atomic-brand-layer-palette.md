---
date: "2026-05-23T16:00:00+01:00"
type: plan
skill: create-plan
work-item: "0073"
status: accepted
---

# 0073 Atomic Brand-Layer Palette — Implementation Plan

## Overview

Introduce the prototype's `--atomic-*` brand-layer palette into the
visualiser's CSS and TypeScript token surfaces, then rewire every
`--ac-*` semantic declaration whose resolved hex exactly matches an
`--atomic-*` value to reference the brand layer via `var()`. Ship the
work as four independent phases driven test-first: (1) brand-layer
foundation, (2) semantic-layer rewrite, (3) visual-regression evidence,
(4) ADR-0026 amendment. Each phase ends with a green test suite and a
self-contained PR-ready slice.

## Current State Analysis

The visualiser frontend lives under
`skills/visualisation/visualise/frontend/`. The `--ac-*` semantic
layer was delivered by story 0033, which **explicitly excluded** the
`--atomic-*` brand palette
(`meta/work/0033-design-token-system.md:113-117`). Today's situation:

- `src/styles/global.css:69-228` declares `:root` with 53 `--ac-*` and
  related tokens, every colour stored as a hex/rgba literal.
- `src/styles/global.css:236-290` mirrors dark overrides in
  `[data-theme="dark"]`; `:296-349` re-declares the same set inside
  `@media (prefers-color-scheme: dark)`. The two dark blocks are kept
  byte-equivalent by a parity test
  (`src/styles/global.test.ts:132-170`).
- `src/styles/tokens.ts` exports nine bare-key constants
  (`LIGHT_COLOR_TOKENS`, `DARK_COLOR_TOKENS`,
  `LIGHT_SHADOW_TOKENS`, `DARK_SHADOW_TOKENS`, `TYPOGRAPHY_TOKENS`,
  `SPACING_TOKENS`, `RADIUS_TOKENS`, `LAYOUT_TOKENS`,
  `CODE_SURFACE_TOKENS`, `CODE_SYNTAX_TOKENS`, `MONO_FONT_TOKENS`),
  each mirrored against `:root` by parity tests in
  `global.test.ts:64-102`.
- `src/styles/prototype-tokens.fixture.test.ts` already drift-detects
  `--code-*` / `--tk-*` against the prototype's `.ac-codeblock` block.
  The fixture lives at `src/styles/fixtures/prototype-tokens.json` (35
  entries today).
- `tests/visual-regression/tokens.spec.ts` runs Playwright
  `toHaveScreenshot()` across six routes × two themes plus three
  special cases; 28 baseline PNGs sit under
  `tests/visual-regression/__screenshots__/`.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
  §5 lists `:root`-only token families (currently `--code-*` and
  `--tk-*`); it does **not** yet model a brand layer.

The prototype's brand palette is declared inline on a single minified
line at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html:183`
and enumerates **37 distinct `--atomic-*` names** (verified by grep).
462 `var(--ac-...)` references across 46 files consume the semantic
layer; the rewrite is transparent to all of them.

## Desired End State

After all four phases land:

1. **Brand layer present in three places, kept in sync by tests.**
   `src/styles/global.css` `:root` declares all 37 `--atomic-*` tokens;
   `src/styles/tokens.ts` exports `BRAND_COLOR_TOKENS` with 37
   bare-key entries (aliases store their resolved hex);
   `src/styles/fixtures/prototype-tokens.json` lists all 37 entries
   in the prototype's raw form (mixed `rgb(...)` / `#XXXXXX` /
   `var(--atomic-X)` / `rgba(...)`).

2. **Semantic layer rewired where exact-hex matches exist.** Nine
   light `--ac-*` declarations in `:root` and 16 dark declarations in
   each of the two dark blocks reference `var(--atomic-X)` instead of
   hex literals. Resolved hex values are unchanged, so no consumer
   sees any visual difference.

3. **Parity comparator resolves `var(--atomic-X)` through the brand
   layer.** `global.test.ts` reads the literal CSS value, follows the
   `var(--atomic-X)` indirection through `BRAND_COLOR_TOKENS`, and
   compares the resolved hex against `LIGHT_COLOR_TOKENS` /
   `DARK_COLOR_TOKENS`. TS-side `LIGHT_COLOR_TOKENS` continues to
   store resolved hex (no `var()` strings), preserving the "TS knows
   the resolved hex of every semantic token" invariant.

4. **Fixture drift detector extended to `--atomic-*`.**
   `prototype-tokens.fixture.test.ts` parses both the `.ac-codeblock`
   block and the `:root` block from the prototype HTML; the
   `canonical()` normaliser handles `rgb(...) ↔ #XXXXXX` and
   `var(--atomic-X)` round-trips so byte-true prototype values compare
   equal to TS-side hex.

5. **Visual-regression baselines pass without `--update-snapshots`**
   for every captured route × theme. If any baseline is regenerated,
   the PR description includes ΔE2000 evidence from a small in-repo
   script using `culori`'s `differenceCiede2000`.

6. **ADR-0026 §5 documents the brand layer.** The brand palette is
   added to the §5 `:root`-only families list with rationale, and a
   short "brand → semantic" indirection rule is added describing
   when an `--ac-*` token references `var(--atomic-X)`.

### Verification end-to-end

```bash
cd skills/visualisation/visualise/frontend
npm test -- src/styles/global.test.ts \
            src/styles/prototype-tokens.fixture.test.ts
npm test            # full vitest suite
npm run test:e2e    # Playwright incl. tokens.spec.ts
```

All three must pass on the implementation branch with the same
baseline PNGs as `main` at branch-cut.

### Key Discoveries

- **37 `--atomic-*` declarations** (verified): named brand colours
  (23), light neutrals (7), aliases (4), overlays (3). Story body's
  illustrative list under-counts; AC1 is anchored to the fixture, so
  the count is determined by enumeration not estimation.
- **9 light + 16 dark rewrite candidates** under AC2's exact
  normalised-hex rule (`research/codebase/2026-05-23-...md`
  section "Hex-to-brand rewrite candidates in `global.css`").
- **Existing fixture comparator (`canonical()` at
  `prototype-tokens.fixture.test.ts:50-52`) only lowercases and
  strips whitespace.** It cannot equate `rgb(14, 15, 25)` with
  `#0e0f19`; the prototype writes 23 of 30 concrete `--atomic-*`
  values in `rgb()` form. The normaliser must be extended.
- **`declarationsOf()` at `prototype-tokens.fixture.test.ts:56-65`
  hard-codes `(?:code|tk)-`**; needs broadening to also accept
  `atomic-`.
- **Existing CSS↔TS parity loop at `global.test.ts:64-70` reads
  literal CSS values.** Rewriting `--ac-bg: #fbfcfe` → `--ac-bg:
  var(--atomic-bone)` would break the test unless the comparator
  learns to resolve `var(--atomic-X)` through `BRAND_COLOR_TOKENS`.
- **`--ac-violet` (`#7b5cd9`) does NOT match
  `--atomic-medium-purple` (`#965dd9`)** — close-but-not-equal; stays
  as literal under AC2's exact-match rule.
- **`--ac-doc-bg-*` dark `#1d2030` vs `--atomic-night-4` `#1d2131`** —
  near-miss (green +1 and blue +1); stays as literal under AC2; PR
  description must call out the near-miss so a future maintainer
  doesn't silently consolidate.
- **`culori` is not yet a dependency**; `npm` install needed before
  Phase 3's ΔE evidence step.
- **ADR-0026 §5 already provides the eligibility framework** for
  `:root`-only families: external source, no light/dark a11y
  differential, ships with drift test. The brand layer satisfies
  all three; adding an entry is the natural extension.

## What We're NOT Doing

- Touching consumer CSS or components — the rewrite is purely in
  declarations.
- Modifying `--ac-*` tokens that don't have an exact normalised-hex
  match to an `--atomic-*` value (near-misses stay as literals; PR
  description enumerates them with rationale).
- Per-theme brand-layer overrides — the brand palette is
  theme-invariant by Assumption.
- Theme-dependent overlay/shadow token decisions — overlay tokens
  (`--atomic-overlay-ink`, `--atomic-stroke-light`,
  `--atomic-shadow-soft`) are seeded as brand-layer literals from
  the prototype; per-theme handling is deferred to story 0077 per
  the story's Non-Goal.
- Colour harmonisation — only exact-hex matches are rewritten.
- Migrating `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` entries to
  `var(--atomic-X)` strings — TS-side keeps resolved hex; the
  comparator handles the indirection.
- Introducing a `color-mix()` derivation for `--atomic-overlay-ink`
  even though its RGB triplet matches `--atomic-night-3`. The
  prototype declares it as a literal `rgba(...)`; the fixture
  preserves that byte-for-byte.

## Implementation Approach

Four phases bundled into two PRs. Each phase starts with a failing
test before any production-source change.

```
PR-A — Brand layer foundation + semantic rewrite + ADR amendment
  Phase 1 (TDD) — Brand layer foundation
    └─ adds 37 --atomic-* declarations + BRAND_COLOR_TOKENS export
       + fixture entries + extended drift detector
       (no --ac-* changes; existing tests untouched)
  Phase 2 (TDD) — Semantic-layer rewrite
    └─ extends parity comparator to resolve var(--atomic-X);
       rewrites 9 light + 16 dark + 16 mirror declarations to
       var(--atomic-X); adds AC2-invariant guard test
  Phase 4 — ADR-0026 amendment
    └─ adds brand layer to §5 list + brand→semantic indirection rule
       as §6 with scope clause, alias-target tie-breaker, references

PR-B — Visual-regression evidence (gated on PR-A)
  Phase 3 — runs full Playwright sweep; ships pngjs dev-dep +
            culori dev-dep + scripts/visual-diff-ciede2000.ts
            (with unit tests); attaches evidence to PR
```

**Phase ordering and PR coupling.** Phase 1 + Phase 2 + Phase 4
ship together in a single PR (call it "PR-A"). Phase 3 ships as a
separate PR (call it "PR-B") gated on PR-A. Rationale:

- Phase 1 alone ships ~37 unconsumed CSS declarations and a 37-entry
  TS map; if Phase 2 stalls, downstream consumers may reach for
  `var(--atomic-*)` directly and bypass the semantic layer. Bundling
  Phase 1 + Phase 2 closes this window.
- Phase 4 introduces the canonical ADR-0026 §6 "brand → semantic
  indirection" rule. Merging Phases 1-2 to `main` without §6 leaves
  the rule undocumented exactly during the window in which it begins
  being applied; future contributors reading `main` could not infer
  the rule from the ADR alone. Bundling Phase 4 with Phase 2 keeps
  the governance record synchronised with the code.
- Phase 3 is the visual-regression evidence step. Re-baselining before
  the rewrite has landed is meaningless; Phase 3 is gated on PR-A
  and ships independently.

### Design decisions recorded inline

The research surfaced six judgement calls; each is settled here so
implementation is fully specified:

| # | Question | Decision | Rationale |
|---|---|---|---|
| 1 | Parity-test failure mode for rewritten `--ac-*` | Comparator resolves `var(--atomic-X)` through `BRAND_COLOR_TOKENS`; `LIGHT_COLOR_TOKENS` keeps resolved hex | Preserves "TS-side tokens know their resolved hex" invariant; one resolution path beats two |
| 2 | Fixture value form | Store prototype's raw form (`rgb(...)`, `#XXXXXX`, `var(--atomic-X)`, `rgba(...)` as written) | Fixture is a byte-accurate snapshot of the prototype; normalisation lives in the comparator |
| 3 | TS naming | New `BRAND_COLOR_TOKENS` export in existing `tokens.ts` | Matches `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` precedent; co-located with consumers |
| 4 | ADR-0026 scope | Amend in Phase 4, shipped in the same PR as Phase 2 | ADR is the right home for the §5 list; amending alongside the introducing story is the established pattern (cf. 0076). Bundling with Phase 2 keeps governance synchronised with the introduction of the rule on `main`. |
| 5 | ΔE tooling | Add `culori` as devDep; ship `scripts/visual-diff-ciede2000.ts` | AC5 names CIEDE2000 explicitly; in-repo script makes the evidence reproducible |
| 6 | `--atomic-overlay-ink` encoding | Literal `rgba(...)` matching prototype byte-for-byte | Drift test stays simple; `color-mix(...)` derivation would diverge from source-of-truth |

---

## Phase 1: Brand layer foundation

### Overview

Introduce the brand layer in three coordinated artefacts (fixture,
TS, CSS) and extend the drift detector to cover it. **No `--ac-*`
declaration changes in this phase** — existing tests stay green
throughout.

### Changes Required

#### 1.1 Extend the fixture drift detector (TDD: failing tests first)

**File**: `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts`

Three changes, written test-first so the new behaviour fails before
the production fixture is updated:

- Broaden `declarationsOf()` regex so the prefix accepts
  `atomic-` alongside `code-` and `tk-`. Pattern becomes
  `/--((?:code|tk|atomic)-[\w-]+):\s*([^;]+);/g`.
- **Promote the brace-balanced block scanner to a shared helper**
  at `src/styles/testing/cssBlocks.ts` exporting
  `extractBlockBody(css, openIndex)`. Today the scanner exists as
  a local function inside `global.test.ts:132-170` and (in
  derivative form) inside `prototype-tokens.fixture.test.ts`'s
  `extractAcCodeblockBlock`. Both call-sites rewire to the shared
  helper. This closes the brace-balanced-extractor duplication
  pattern at the same time as adding the new extractor below.
- Add a sibling extractor `extractRootBlockBody(html)` that uses
  the shared `extractBlockBody` primitive. The
  prototype has TWO top-level (non-`@media`) `:root` blocks: the
  first declares `--atomic-*` (the brand palette, at line 183) and
  the second declares `--ac-*` light defaults. Disambiguation is
  by **content**: the extractor returns the first `:root { ... }`
  block whose body contains `--atomic-night:`. This makes selection
  resilient to future reordering of the prototype's `:root` blocks
  and produces a precise failure mode ("brand block not found") if
  the prototype is restructured.
- Replace the existing `canonical()` body with an import from the
  shared canonicaliser (see §1.6 — extracted into
  `src/styles/testing/canonicaliseBrand.ts`). The fixture test
  needs only the rgb→hex + whitespace/case path, but it imports the
  same helper as `global.test.ts` so the codebase has a single CSS-
  colour normaliser. Unknown `var(--atomic-X)` refs in the helper
  throw (see §1.6); fixture entries that are aliases (e.g.
  `--atomic-violet: var(--atomic-medium-purple)`) are stored
  verbatim on both sides and compare equal after whitespace/case
  normalisation without ever entering the var-resolution branch.

  ```ts
  import { canonicaliseBrand } from './testing/canonicaliseBrand'
  // ...uses canonicaliseBrand wherever the previous canonical() was used
  ```

  `rgba(...)` is left in canonical form so the prototype's
  overlay tokens compare on whitespace/case only (they remain
  literal on both sides).

Add direct unit tests for `extractRootBlockBody` covering the
selection rule:

```ts
describe('extractRootBlockBody', () => {
  it('returns the first :root block containing --atomic-night', () => { /* ... */ })
  it('skips :root blocks without --atomic-night (e.g. --ac-* defaults)', () => { /* ... */ })
  it('returns undefined when no qualifying :root exists', () => { /* ... */ })
  it('handles balanced-brace pathological input (e.g. nested rules)', () => { /* ... */ })
})
```

The existing two `describe` blocks (set membership + value parity)
continue to operate against a unified prototype-source map that now
includes both `.ac-codeblock` and `:root` declarations. Test labels
remain stable.

```ts
const protoMap = new Map<string, string>([
  ...declarationsOf(extractAcCodeblockBlock(source)),
  ...declarationsOf(extractRootBlockBody(source)),
])
```

#### 1.2 Populate `prototype-tokens.json` with `--atomic-*` entries

**File**: `skills/visualisation/visualise/frontend/src/styles/fixtures/prototype-tokens.json`

Append all 37 `--atomic-*` entries to the existing JSON object,
storing each value **byte-for-byte** as written in
`prototype-standalone.html:183`. Example skeleton (full set
populated by reading the prototype directly):

```json
{
  "--code-bg":            "#0E1320",
  "...":                  "...",
  "--tk-ddel":            "#E56B7E",

  "--atomic-night":       "rgb(14, 15, 25)",
  "--atomic-night-2":     "rgb(10, 17, 27)",
  "--atomic-night-3":     "rgb(23, 25, 37)",
  "--atomic-night-4":     "rgb(29, 33, 49)",
  "--atomic-ink":         "rgb(32, 34, 49)",
  "--atomic-ink-2":       "rgb(44, 46, 65)",
  "--atomic-red":         "rgb(203, 70, 71)",
  "--atomic-red-2":       "rgb(223, 87, 88)",
  "--atomic-red-3":       "rgb(226, 78, 83)",
  "--atomic-indigo":      "rgb(89, 95, 200)",
  "--atomic-indigo-2":    "rgb(50, 48, 98)",
  "--atomic-indigo-tint": "rgb(193, 197, 255)",
  "--atomic-medium-purple": "#965DD9",
  "--atomic-cream-can":   "#F5C25F",
  "--atomic-steel-blue":  "#4295A5",
  "--atomic-pastel-green": "#6BE58B",
  "--atomic-river-bed":   "#4A545F",
  "--atomic-aquamarine":  "#73E4E2",
  "--atomic-tradewind":   "#52B0AA",
  "--atomic-geyser":      "#D3DBE0",
  "--atomic-malibu":      "#72CBF5",
  "--atomic-link-water":  "#DDECF4",
  "--atomic-marigold":    "#F9DE6F",

  "--atomic-white":       "rgb(255, 255, 255)",
  "--atomic-bone":        "rgb(251, 252, 254)",
  "--atomic-mist":        "rgb(217, 217, 217)",
  "--atomic-ash":         "rgb(211, 219, 224)",
  "--atomic-smoke":       "rgb(199, 201, 216)",
  "--atomic-slate":       "rgb(95, 99, 120)",
  "--atomic-slate-2":     "rgb(74, 84, 95)",

  "--atomic-violet":      "var(--atomic-medium-purple)",
  "--atomic-teal":        "var(--atomic-tradewind)",
  "--atomic-sky":         "var(--atomic-malibu)",
  "--atomic-sky-2":       "var(--atomic-malibu)",

  "--atomic-overlay-ink":  "rgba(23, 25, 37, 0.56)",
  "--atomic-stroke-light": "rgba(255, 255, 255, 0.35)",
  "--atomic-shadow-soft":  "rgba(0, 0, 0, 0.08)"
}
```

The exact set of values is determined by reading
`prototype-standalone.html:183` at implementation time — the table
above is the verified snapshot at plan-cut. Any deviation between
plan and prototype is reconciled by the drift test.

#### 1.3 Declare `--atomic-*` in `global.css` `:root`

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`

Insert a new commented section inside the existing `:root` block,
placed **after** the `LAYOUT` section and **before** the
`Code-block surface tokens` section, so the brand layer reads as a
distinct unit:

```css
:root {
  /* … existing --ac-*, typography, spacing, radius, shadow, layout … */

  /* Atomic brand palette — canonical source for named brand colours,
     neutrals, aliases, and overlays. Mirrored to BRAND_COLOR_TOKENS
     in tokens.ts and snapshotted in
     src/styles/fixtures/prototype-tokens.json (drift-tested in
     prototype-tokens.fixture.test.ts). The palette is theme-invariant
     (ADR-0026 §5); semantic --ac-* tokens whose resolved hex matches
     a brand value reference it via var() (ADR-0026 §6 — brand →
     semantic indirection rule). Declared verbatim from
     meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/
     prototype-standalone.html:183. */
  --atomic-night:       rgb(14, 15, 25);
  --atomic-night-2:     rgb(10, 17, 27);
  /* … all 37 entries verbatim from the prototype … */
  --atomic-violet:      var(--atomic-medium-purple);
  --atomic-teal:        var(--atomic-tradewind);
  --atomic-sky:         var(--atomic-malibu);
  --atomic-sky-2:       var(--atomic-malibu);
  --atomic-overlay-ink:  rgba(23, 25, 37, 0.56);
  --atomic-stroke-light: rgba(255, 255, 255, 0.35);
  --atomic-shadow-soft:  rgba(0, 0, 0, 0.08);

  /* … existing code-block surface tokens … */
}
```

**Critical**: declarations must remain inside the **same flat
`:root` block** so `readCssVar()` (which uses a non-greedy match,
`global.test.ts:32-44`) reads them correctly. No nested rules.
**Critical**: aliases (`--atomic-violet`, `--atomic-teal`,
`--atomic-sky`, `--atomic-sky-2`) MUST be declared **after** their
target so the cascade resolves correctly within `:root`.

#### 1.4 Add `BRAND_COLOR_TOKENS` export in `tokens.ts`

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`

Insert the new export **immediately before `CODE_SURFACE_TOKENS`**
so the file's theme-invariant families (`BRAND` → `CODE_SURFACE` →
`CODE_SYNTAX`) cluster contiguously after the theme-variant
families and the typography/spacing/radius/shadow/layout groups.

The export name keeps `BRAND_COLOR_TOKENS` (not `ATOMIC_BRAND_TOKENS`)
because the family is named by what it contains (brand colours), not
by its CSS prefix. The `--atomic-` prefix is the CSS-side namespace;
the TS-side constant uses `BRAND` as the conceptual handle, consistent
with the comment block (which calls it "the Atomic brand palette"
rather than "the atomic tokens"). A short inline comment records this
choice so future contributors do not re-litigate it.

Each entry stores the **resolved** hex (or rgba) — aliases are
collapsed to their target's hex, NOT to `var(--atomic-medium-purple)`
strings. This keeps `BRAND_COLOR_TOKENS` a pure value map and lets
the parity comparator look up "what hex does `--atomic-X` resolve
to" with a single `BRAND_COLOR_TOKENS[name]` read.

```ts
// Atomic brand palette — see global.css :root header for the
// canonical rationale. Theme-invariant (ADR-0026 §5). Aliases store
// their RESOLVED hex (not a 'var(...)' string) so this map is a
// pure name→value lookup for the parity comparator; the
// alias-target equality is asserted in global.test.ts.
export const BRAND_COLOR_TOKENS = {
  'atomic-night':         '#0e0f19',
  'atomic-night-2':       '#0a111b',
  'atomic-night-3':       '#171925',
  'atomic-night-4':       '#1d2131',
  'atomic-ink':           '#202231',
  'atomic-ink-2':         '#2c2e41',
  'atomic-red':           '#cb4647',
  'atomic-red-2':         '#df5758',
  'atomic-red-3':         '#e24e53',
  'atomic-indigo':        '#595fc8',
  'atomic-indigo-2':      '#323062',
  'atomic-indigo-tint':   '#c1c5ff',
  'atomic-medium-purple': '#965dd9',
  'atomic-cream-can':     '#f5c25f',
  'atomic-steel-blue':    '#4295a5',
  'atomic-pastel-green':  '#6be58b',
  'atomic-river-bed':     '#4a545f',
  'atomic-aquamarine':    '#73e4e2',
  'atomic-tradewind':     '#52b0aa',
  'atomic-geyser':        '#d3dbe0',
  'atomic-malibu':        '#72cbf5',
  'atomic-link-water':    '#ddecf4',
  'atomic-marigold':      '#f9de6f',
  'atomic-white':         '#ffffff',
  'atomic-bone':          '#fbfcfe',
  'atomic-mist':          '#d9d9d9',
  'atomic-ash':           '#d3dbe0',
  'atomic-smoke':         '#c7c9d8',
  'atomic-slate':         '#5f6378',
  'atomic-slate-2':       '#4a545f',
  'atomic-violet':        '#965dd9', // resolved alias of atomic-medium-purple
  'atomic-teal':          '#52b0aa', // resolved alias of atomic-tradewind
  'atomic-sky':           '#72cbf5', // resolved alias of atomic-malibu
  'atomic-sky-2':         '#72cbf5', // resolved alias of atomic-malibu
  'atomic-overlay-ink':   'rgba(23, 25, 37, 0.56)',
  'atomic-stroke-light':  'rgba(255, 255, 255, 0.35)',
  'atomic-shadow-soft':   'rgba(0, 0, 0, 0.08)',
} as const

export type BrandColorToken = keyof typeof BRAND_COLOR_TOKENS

// Documented alias pairs — keep in sync with the comments above. The
// alias-target equality test in global.test.ts iterates this list.
export const BRAND_ALIAS_PAIRS: ReadonlyArray<readonly [BrandColorToken, BrandColorToken]> = [
  ['atomic-violet', 'atomic-medium-purple'],
  ['atomic-teal',   'atomic-tradewind'],
  ['atomic-sky',    'atomic-malibu'],
  ['atomic-sky-2',  'atomic-malibu'],
] as const
```

#### 1.5 Extend `global.test.ts` parity coverage

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`

Four extensions:

**(a) CSS↔TS parity for the brand layer.** Add `['brand',
BRAND_COLOR_TOKENS]` to the `describe.each([...])` table at
`global.test.ts:88-102`. No new infrastructure — the parameterised
describe already covers any `:root`-only token family. Test labels
follow the existing `--<name> matches` pattern.

```ts
describe.each([
  ['typography', TYPOGRAPHY_TOKENS],
  ['spacing', SPACING_TOKENS],
  ['radius', RADIUS_TOKENS],
  ['light shadow', LIGHT_SHADOW_TOKENS],
  ['layout', LAYOUT_TOKENS],
  ['code surface', CODE_SURFACE_TOKENS],
  ['syntax', CODE_SYNTAX_TOKENS],
  ['brand', BRAND_COLOR_TOKENS],   // NEW
])('tokens.ts ↔ global.css :root parity (%s)', ...)
```

For this to pass, `expectMatches` is upgraded to canonicalise both
sides through the shared `canonicaliseBrand` helper (see §1.6 and
Phase 2 §2.1). The helper folds the existing `canonicaliseTokenValue`
(which only did lowercase + whitespace-strip) as a strict superset —
`canonicaliseTokenValue` is deleted, and the existing parity loops
continue to pass against unchanged CSS because no `--ac-*`
declaration uses `var()` yet (Phase 1 adds no rewrites).

**(b) Fixture↔BRAND_COLOR_TOKENS parity.** Extend the existing
fixture↔tokens.ts parity loop at `global.test.ts:221-235` so it
asserts each `--atomic-*` fixture entry against
`BRAND_COLOR_TOKENS`. Today that loop only covers
`CODE_SURFACE_TOKENS` / `CODE_SYNTAX_TOKENS`; without this
extension the TS-side brand map could silently drift from the
fixture (and thus the prototype) for any token whose CSS form is
`rgb(...)`. Use `canonicaliseBrand` on both sides so alias entries
in the fixture (`var(--atomic-medium-purple)`) compare equal to
their resolved hex in `BRAND_COLOR_TOKENS`.

**(c) Brand-layer theme invariance.** Add a single regression test
asserting **theme invariance** mirroring the existing
`--ac-violet` guard at `global.test.ts:190-200`:

```ts
describe('--atomic-* theme invariance', () => {
  it('no --atomic-* declaration appears in [data-theme="dark"] block', () => {
    const darkMatch = /\[data-theme="dark"\]\s*\{/.exec(globalCss)
    const darkBody = darkMatch
      ? extractBlockBody(globalCss, darkMatch.index)
      : undefined
    expect(darkBody).toBeDefined()
    expect(darkBody!).not.toMatch(/--atomic-[\w-]+\s*:/)
  })
  it('no --atomic-* declaration appears in @media (prefers-color-scheme: dark)', () => {
    const mediaMatch = /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{/.exec(globalCss)
    const mediaBody = mediaMatch
      ? extractBlockBody(globalCss, mediaMatch.index)
      : undefined
    expect(mediaBody).toBeDefined()
    expect(mediaBody!).not.toMatch(/--atomic-[\w-]+\s*:/)
  })
})
```

This guards against a future maintainer adding a per-theme brand
override; the brand palette is documented theme-invariant.

**(d) Alias-target equality.** Add a small test driven by
`BRAND_ALIAS_PAIRS` (exported from `tokens.ts`, §1.4) asserting
each documented alias resolves to the same value as its target. If
the prototype retargets `--atomic-violet` from `medium-purple` to
something else, this test fails loudly with the alias name rather
than as an indirect hex mismatch elsewhere.

```ts
import { BRAND_ALIAS_PAIRS, BRAND_COLOR_TOKENS } from './tokens'

describe('BRAND_COLOR_TOKENS alias-target equality', () => {
  it.each(BRAND_ALIAS_PAIRS)(
    '%s resolves to the same hex as its target %s',
    (alias, target) => {
      expect(BRAND_COLOR_TOKENS[alias]).toBe(BRAND_COLOR_TOKENS[target])
    },
  )
})
```

#### 1.6 Extract brand-layer canonicaliser to a shared helper

**File**: `skills/visualisation/visualise/frontend/src/styles/testing/canonicaliseBrand.ts` (NEW, ~40 lines)

Placement under `src/styles/testing/` matches the convention
ADR-0026 §5 records for shared test-only helpers: imported only
from `*.test.ts`, co-located with its unit test, file-header
comment naming consumers. The companion unit test lives at
`src/styles/testing/canonicaliseBrand.test.ts` (Phase 2 §2.1).

The helper folds `canonicaliseTokenValue` (which only did
lowercase + whitespace-strip) as a strict superset; the existing
helper is deleted and its single call-site at
`global.test.ts:221-235` switches its import. Two key contract
choices encoded below: (i) unknown `--atomic-*` refs **throw**
because `BRAND_COLOR_TOKENS` is a closed enum at type level — a
mismatch is a bug, not an expected fallback; (ii) recursion
includes a `seen`-set cycle guard so a future schema change that
stores `var()` strings in `BRAND_COLOR_TOKENS` cannot stack-
overflow silently.

```ts
// Test-only helper. Consumers: global.test.ts,
// prototype-tokens.fixture.test.ts.
//
// Do not import from production code.

import { BRAND_COLOR_TOKENS } from '../tokens'

function rgbToHex(r: string, g: string, b: string): string {
  const hex = (n: string) => Number(n).toString(16).padStart(2, '0')
  return `#${hex(r)}${hex(g)}${hex(b)}`
}

/**
 * Normalise a CSS colour value for parity comparison. Handles:
 *  - whitespace and case (lowercase + strip whitespace)
 *  - rgb(r, g, b) → #rrggbb (six-digit lowercase hex)
 *  - var(--atomic-X) → look through BRAND_COLOR_TOKENS, recur
 *    (alias chains followed; cycle guard prevents infinite recursion)
 *  - rgba(...) and #XXXXXX pass through unchanged after stripping
 *  - var() refs whose name is not in BRAND_COLOR_TOKENS pass through
 *    unchanged (consumer's semantic-layer refs, not brand-layer)
 *
 * Throws if a var(--atomic-X) ref names an --atomic-* token that
 * does not exist in BRAND_COLOR_TOKENS — this is a bug, not a soft
 * mismatch, and a hard failure produces an actionable error message
 * rather than an opaque string-mismatch test failure downstream.
 *
 * Domain assumption: the prototype uses comma-separated integer
 * rgb() with 0-255 channels. Whitespace-separated rgb(), percentage
 * channels, and 4-channel rgba() shapes are out of scope; values
 * outside that domain fall through to the lowercased/stripped form.
 */
export function canonicaliseBrand(v: string): string {
  return resolve(v, new Set())
}

function resolve(v: string, seen: Set<string>): string {
  const s = v.toLowerCase().replace(/\s+/g, '')
  const rgb = /^rgb\((\d{1,3}),(\d{1,3}),(\d{1,3})\)$/.exec(s)
  if (rgb) return rgbToHex(rgb[1], rgb[2], rgb[3])

  const ref = /^var\(--(atomic-[\w-]+)\)$/.exec(s)
  if (ref) {
    const name = ref[1]
    if (seen.has(name)) {
      throw new Error(`canonicaliseBrand: cycle detected at --${name}`)
    }
    const target = (BRAND_COLOR_TOKENS as Record<string, string | undefined>)[name]
    if (target === undefined) {
      throw new Error(
        `canonicaliseBrand: unknown brand token --${name}; ` +
        `check spelling or add to BRAND_COLOR_TOKENS`,
      )
    }
    return resolve(target, new Set(seen).add(name))
  }

  // Non-brand var() refs (e.g. var(--ac-bg)) pass through unchanged
  // so callers can detect them via string mismatch rather than
  // exception. Only --atomic-* refs are resolved here.
  return s
}

export { rgbToHex }
```

### Success Criteria

#### Automated Verification

- [x] `npm test -- src/styles/global.test.ts` passes (brand-layer
  CSS↔TS parity, fixture↔BRAND_COLOR_TOKENS parity, theme-invariance
  guards, alias-target equality)
- [x] `npm test -- src/styles/prototype-tokens.fixture.test.ts` passes
  (all 37 `--atomic-*` entries drift-check green; 0 missing, 0 extra;
  `extractRootBlockBody` unit tests green)
- [x] `npm test -- src/styles/testing/canonicaliseBrand.test.ts`
  passes (Phase 2 §2.1 introduces this file; placeholder during
  Phase 1 if the helper-only edit lands first)
- [x] `npm run build` passes (TS compile + Vite build)
- [ ] `npm run lint` passes (if a lint script exists; otherwise N/A)
- [x] `npm test` (full suite) passes with zero regressions
- [x] Existing tests at `global.test.ts:64-102` and `:132-170` still
  pass byte-identically (sanity: `--ac-*` not yet touched)

#### Manual Verification

- [ ] `git diff` shows zero changes to existing `--ac-*` declarations
  in `global.css` and zero changes to `LIGHT_COLOR_TOKENS` /
  `DARK_COLOR_TOKENS` in `tokens.ts`
- [ ] Running the visualiser dev server (`npm run dev`) shows no
  visual change — brand layer is declared but unconsumed
- [ ] Spot-check 5 random brand tokens by inspecting computed
  styles in DevTools: each resolves to the expected hex via the
  `:root` declaration

### Notes

- **Hex collisions are preserved**: `--atomic-ash` and `--atomic-geyser`
  both resolve to `#d3dbe0`; `--atomic-slate-2` and `--atomic-river-bed`
  both resolve to `#4a545f`. The brand layer is name-keyed (not
  hex-keyed) so semantic identity is retained.
- **Drift detector is now the source-of-truth for AC1**: any future
  prototype edit either propagates through the fixture or fails the
  test loudly. AC1 is satisfied by green test runs, no manual count
  required.

---

## Phase 2: Semantic-layer rewrite

### Overview

Rewire every `--ac-*` declaration whose normalised six-digit hex
matches a `--atomic-*` value to reference `var(--atomic-X)` instead.
Extend the CSS↔TS parity comparator to follow the indirection so
existing parity tests continue to pass without changes to
`LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS`.

The "exact match" reference snapshot is the state of `global.css`
on `main` at the implementation branch's cut point. (Set
`MAIN_CSS_SHA = <commit-sha>` in the PR description so reviewers
can re-derive the candidate set if needed.)

### Changes Required

#### 2.1 Wire `canonicaliseBrand` into existing parity loops (TDD: failing test first)

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`

Before touching `global.css`, **flip `expectMatches` to use
`canonicaliseBrand` on both sides** so the existing parity loops
become aware of `var(--atomic-X)` references. After this change,
the loops still pass against unchanged CSS because no `--ac-*`
declaration uses `var()` yet — but they'll keep passing through
the rewrite.

```ts
import { canonicaliseBrand } from './testing/canonicaliseBrand'

function expectMatches(actual: string | null, expected: string): void {
  expect(actual).not.toBeNull()
  expect(canonicaliseBrand(actual!)).toBe(canonicaliseBrand(expected))
}
```

Whitespace-strip note: switching both sides through
`canonicaliseBrand` slightly relaxes existing rgba/shadow value
comparisons (which today tolerate trim-only). To preserve the
TS-side resolved-hex invariant explicitly, add a format guard test
alongside the parity loop:

```ts
describe('LIGHT_COLOR_TOKENS / DARK_COLOR_TOKENS contain no indirection', () => {
  it.each(Object.entries({ ...LIGHT_COLOR_TOKENS, ...DARK_COLOR_TOKENS }))(
    '%s contains no var(--atomic-*) or bare rgb(...) indirection',
    (_, value) => {
      // TS-side stores resolved hex (or rgba/shadow). Bare rgb()
      // would compare equal to a CSS hex through the comparator and
      // silently relax parity; var() would obviously break the
      // 'TS knows the resolved hex' invariant. See ADR-0026 §6.
      expect(value).not.toMatch(/var\(--atomic-/)
      expect(value).not.toMatch(/^\s*rgb\(/i)
    },
  )
})
```

Write the **dedicated failing test first** to drive the comparator
change. Place at the top of the new
`src/styles/testing/canonicaliseBrand.test.ts` (NEW):

```ts
import { describe, it, expect } from 'vitest'
import { canonicaliseBrand } from './canonicaliseBrand'

describe('canonicaliseBrand', () => {
  it('normalises rgb(...) to lowercase six-digit hex', () => {
    expect(canonicaliseBrand('rgb(14, 15, 25)')).toBe('#0e0f19')
  })
  it('lowercases plain hex without changing channels', () => {
    expect(canonicaliseBrand('#0E0F19')).toBe('#0e0f19')
  })
  it('resolves var(--atomic-X) through BRAND_COLOR_TOKENS', () => {
    expect(canonicaliseBrand('var(--atomic-bone)')).toBe('#fbfcfe')
  })
  it('resolves alias to hex via BRAND_COLOR_TOKENS (single hop today; recursion-safe by design)', () => {
    expect(canonicaliseBrand('var(--atomic-violet)')).toBe('#965dd9')
  })
  it('leaves rgba(...) in canonical form', () => {
    expect(canonicaliseBrand('rgba(0, 0, 0, 0.08)')).toBe('rgba(0,0,0,0.08)')
  })
  it('throws on unknown --atomic-* refs with an actionable message', () => {
    expect(() => canonicaliseBrand('var(--atomic-nonexistent)')).toThrow(
      /unknown brand token --atomic-nonexistent/,
    )
  })
  it('passes through non-brand var() refs unchanged (e.g. var(--ac-bg))', () => {
    expect(canonicaliseBrand('var(--ac-bg)')).toBe('var(--ac-bg)')
  })
  it('detects cycles when BRAND_COLOR_TOKENS contains var() strings (defensive)', async () => {
    // Stubs the brand map with a self-referential entry so the cycle
    // guard actually fires. Today BRAND_COLOR_TOKENS stores resolved
    // hex (no cycles possible) but the guard exists to defend against
    // a future refactor; this test pins that defence.
    vi.doMock('../tokens', () => ({
      BRAND_COLOR_TOKENS: {
        'atomic-a': 'var(--atomic-b)',
        'atomic-b': 'var(--atomic-a)',
      },
    }))
    const { canonicaliseBrand: cycling } = await import('./canonicaliseBrand')
    expect(() => cycling('var(--atomic-a)')).toThrow(/cycle detected/)
    vi.doUnmock('../tokens')
  })
})
```

Also add a **AC2-invariant guard test** that verifies the
brand-layer rewrite rule is enforced going forward. The test
depends on a small helper `extractAllAcDeclarations(css)` that
returns one entry per `--ac-*` declaration across the three
relevant blocks; specified inline below.

**Helper**: `extractAllAcDeclarations(css)` lives in
`src/styles/testing/extractAcDeclarations.ts` (co-located with
its companion unit test). It reuses the existing `extractBlockBody`
brace-balanced scanner (extracted in §1.1 as a side-effect of
`extractRootBlockBody`) to iterate the three blocks and emit
`{ name, value, block }` entries, where `block` is one of
`'root' | 'data-dark' | 'media-dark'`.

```ts
// src/styles/testing/extractAcDeclarations.ts
// Test-only helper. Consumers: global.test.ts.

import { extractBlockBody } from './cssBlocks'

export type AcBlockTag = 'root' | 'data-dark' | 'media-dark'

export interface AcDeclaration {
  name: string
  value: string
  block: AcBlockTag
}

const BLOCK_OPENERS: ReadonlyArray<[AcBlockTag, RegExp]> = [
  ['root',       /(^|\n):root\s*\{/],
  ['data-dark',  /\[data-theme="dark"\]\s*\{/],
  ['media-dark', /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{/],
]

export function extractAllAcDeclarations(css: string): AcDeclaration[] {
  const result: AcDeclaration[] = []
  for (const [tag, opener] of BLOCK_OPENERS) {
    const match = opener.exec(css)
    if (!match) continue
    const body = extractBlockBody(css, match.index)
    const declRe = /--(ac-[\w-]+):\s*([^;]+);/g
    for (const m of body.matchAll(declRe)) {
      result.push({ name: m[1], value: m[2].trim(), block: tag })
    }
  }
  return result
}
```

`extractBlockBody(css, openIndex)` is the existing brace-balanced
scanner already used by the `[data-theme="dark"] ↔ @media` parity
test at `global.test.ts:132-170`; promote it from a `global.test.ts`
local to `src/styles/testing/cssBlocks.ts` so this helper and the
existing tests share one implementation. Ship a small unit test
for `extractAllAcDeclarations` covering: (a) returns one entry per
declaration across all three blocks tagged correctly, (b) handles
declarations missing semicolon-terminated boundaries gracefully,
(c) skips non-`--ac-*` declarations.

**AC2-invariant guard test**:

```ts
import { extractAllAcDeclarations } from './testing/extractAcDeclarations'
import { canonicaliseBrand } from './testing/canonicaliseBrand'

describe('AC2 invariant: --ac-* hex literals must reference brand when possible', () => {
  // Tokens intentionally left as hex literals despite a brand-value
  // match — typically near-misses or theme-specific overrides.
  // Adding to this list requires a code-review reason recorded in
  // the PR. Populated from Phase 2 §2.2/§2.3 residue tables.
  const ALLOW_LIST_LITERALS: ReadonlyArray<string> = [
    // Format: `${block}:${name}` — at plan-cut the residue tables
    // contain no exact-match offenders, so the allow-list is empty.
  ]

  it('every --ac-* hex literal that matches a BRAND_COLOR_TOKENS entry is in the allow-list', () => {
    const decls = extractAllAcDeclarations(globalCss)
    const offenders = decls.filter((d) => {
      if (d.value.startsWith('var(')) return false
      // rgba() is out of scope per ADR-0026 §6 (six-digit hex only).
      if (d.value.toLowerCase().startsWith('rgba(')) return false
      const hex = canonicaliseBrand(d.value)
      const brandMatches = Object.values(BRAND_COLOR_TOKENS).includes(hex)
      return brandMatches && !ALLOW_LIST_LITERALS.includes(`${d.block}:${d.name}`)
    })
    expect(offenders).toEqual([])
  })
})
```

This is the automated guardrail that AC2 requires for ongoing
correctness; the allow-list is the auditable surface.

**Rewrite-count guard** (companion test, same describe block) —
asserts the expected number of `var(--atomic-` references per
block. A partial merge dropping one rewrite would still pass the
symmetric MIRROR-A↔MIRROR-B parity test if both dark blocks lost
the same rewrite together; this guard catches that mode:

```ts
it.each([
  ['root',       9],
  ['data-dark',  16],
  ['media-dark', 16],
])('block %s contains exactly %d var(--atomic-X) refs', (block, expected) => {
  const refs = extractAllAcDeclarations(globalCss).filter(
    (d) => d.block === block && d.value.startsWith('var(--atomic-'),
  )
  expect(refs).toHaveLength(expected)
})
```

#### 2.2 Rewrite the light `:root` block

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css:69-228`

Nine declarations rewritten (six distinct brand tokens):

```diff
-  --ac-bg:             #fbfcfe;
-  --ac-bg-raised:      #ffffff;
-  --ac-bg-sunken:      #f4f6fa;
-  --ac-bg-chrome:      #ffffff;
-  --ac-bg-sidebar:     #f7f8fb;
-  --ac-bg-card:        #ffffff;
-  ...
-  --ac-fg-strong:      #0a111b;
-  --ac-fg-muted:       #5f6378;
-  ...
-  --ac-accent:         #595fc8;
-  --ac-accent-2:       #cb4647;
-  ...
-  --ac-err:            #cb4647;
+  --ac-bg:             var(--atomic-bone);
+  --ac-bg-raised:      var(--atomic-white);
+  --ac-bg-sunken:      #f4f6fa;
+  --ac-bg-chrome:      var(--atomic-white);
+  --ac-bg-sidebar:     #f7f8fb;
+  --ac-bg-card:        var(--atomic-white);
+  ...
+  --ac-fg-strong:      var(--atomic-night-2);
+  --ac-fg-muted:       var(--atomic-slate);
+  ...
+  --ac-accent:         var(--atomic-indigo);
+  --ac-accent-2:       var(--atomic-red);
+  ...
+  --ac-err:            var(--atomic-red);
```

**Tokens that stay as literals (light)** — to be enumerated
verbatim in the PR description per AC2:

| Token | Light literal | Reason |
|---|---|---|
| `--ac-bg-sunken` | `#f4f6fa` | no exact brand match |
| `--ac-bg-sidebar` | `#f7f8fb` | no exact brand match |
| `--ac-fg` | `#14161f` | no exact brand match |
| `--ac-fg-faint` | `#8b90a3` | no exact brand match |
| `--ac-ok` | `#2e8b57` | no exact brand match |
| `--ac-warn` | `#d98f2e` | no exact brand match |
| `--ac-violet` | `#7b5cd9` | near-miss to `--atomic-medium-purple` `#965dd9` |
| all 12 `--ac-doc-*` light | various | eyedroppered values |
| all 12 `--ac-doc-bg-*` light | various | pastel set, no match |
| `--ac-bg-hover` / `--ac-bg-active` / `--ac-stroke*` / `--ac-accent-*-tint` etc. | various `rgba(...)` | not in scope under AC2 (hex-only) |

#### 2.3 Rewrite the dark `[data-theme="dark"]` block

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css:236-290`

Sixteen declarations rewritten (three distinct brand tokens):

```diff
-  --ac-bg:             #0a111b;
-  --ac-bg-raised:      #0e0f19;
-  --ac-bg-chrome:      #0e0f19;
-  --ac-fg-strong:      #ffffff;
-  --ac-doc-decisions:           #ffffff;
-  ... (12 --ac-doc-* lines, all #ffffff) ...
+  --ac-bg:             var(--atomic-night-2);
+  --ac-bg-raised:      var(--atomic-night);
+  --ac-bg-chrome:      var(--atomic-night);
+  --ac-fg-strong:      var(--atomic-white);
+  --ac-doc-decisions:           var(--atomic-white);
+  ... (12 --ac-doc-* lines, all var(--atomic-white)) ...
```

**Tokens that stay as literals (dark)** — enumerated in PR
description:

| Token | Dark literal | Reason |
|---|---|---|
| `--ac-bg-sunken` | `#070b12` | no exact brand match |
| `--ac-bg-sidebar` | `#0b121c` | no exact brand match |
| `--ac-bg-card` | `#131524` | no exact brand match |
| `--ac-fg` | `#e7e9f2` | no exact brand match |
| `--ac-fg-muted` | `#a0a5b8` | no exact brand match |
| `--ac-fg-faint` | `#6c7088` | no exact brand match |
| `--ac-accent` | `#8a90e8` | no exact brand match |
| `--ac-accent-2` | `#e86a6b` | near-miss to `--atomic-red-2`/`-3` |
| `--ac-ok` / `--ac-warn` / `--ac-err` | various | no exact match |
| all 12 `--ac-doc-bg-*` dark | `#1d2030` | near-miss to `--atomic-night-4` `#1d2131` (green +1, blue +1); explicitly called out so a future maintainer doesn't silently consolidate |
| all `--ac-bg-hover` / `--ac-bg-active` / `--ac-stroke*` / `--ac-accent-*-tint` | `rgba(...)` | not in scope under AC2 (hex-only) |

#### 2.4 Mirror the rewrites in `@media (prefers-color-scheme: dark)`

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css:296-349`

Apply the **same 16 substitutions** byte-equivalently to the
inner `:root:not([data-theme="light"]) { ... }` block. The parity
test at `global.test.ts:132-170` already asserts the two dark
blocks declare identical name→value maps; if MIRROR-A and
MIRROR-B disagree after the rewrite, that test fails loudly.

#### 2.5 Verify `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` are unchanged

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`

**Do NOT modify** these constants' values. The whole point of the
comparator extension in §2.1 is to keep the TS side as a pure
resolved-hex map; rewriting TS entries to `var(--atomic-X)`
strings would lose that invariant. CI will catch any accidental
edit because the parity loop continues to assert the resolved
hex, and the format-guard test (§2.1) explicitly rejects any
`var(--atomic-` substring in these maps.

Add a short comment block above both maps documenting the dual-
layer model — this is the most likely place a future contributor
will encounter the rule:

```ts
// Resolved-hex semantic palette. Values are stored as resolved hex
// (or rgba) even where the corresponding global.css declaration
// uses var(--atomic-X) brand-layer indirection — see ADR-0026 §6.
// The CSS↔TS parity comparator resolves var() refs via
// BRAND_COLOR_TOKENS, preserving 'TS knows the resolved hex of
// every semantic token' as a load-bearing invariant.
export const LIGHT_COLOR_TOKENS = { /* ... */ } as const
export const DARK_COLOR_TOKENS  = { /* ... */ } as const
```

### Success Criteria

#### Automated Verification

- [x] `npm test -- src/styles/testing/canonicaliseBrand.test.ts`
  passes (7 unit tests covering rgb→hex, hex lowercasing,
  brand var() resolution, alias resolution, rgba pass-through,
  unknown-ref throws, non-brand var() pass-through)
- [x] `npm test -- src/styles/global.test.ts` passes
  (existing 53+ light + 41+ dark parity assertions, brand-layer
  parity from Phase 1, AC2-invariant guard test, format-guard
  test on LIGHT_COLOR_TOKENS / DARK_COLOR_TOKENS)
- [x] `npm test -- src/styles/prototype-tokens.fixture.test.ts`
  passes
- [x] `npm test` (full suite) passes with zero regressions
- [x] `[data-theme="dark"] ↔ @media` parity test
  (`global.test.ts:132-170`) passes — confirms MIRROR-A and
  MIRROR-B were rewritten symmetrically
- [x] `npm run build` passes (no TS errors)

#### Manual Verification

- [ ] Running the visualiser dev server (`npm run dev`) shows
  **zero visual difference** in light or dark mode (resolved hex
  is preserved by construction)
- [ ] Spot-check 3 rewritten declarations in DevTools: e.g.
  `--ac-bg` light shows computed `var(--atomic-bone)` indirection
  resolving to `rgb(251, 252, 254)`
- [ ] `git diff` enumerates exactly 9 changes in `:root`, 16 in
  `[data-theme="dark"]`, and 16 in the `@media` mirror (total: 41
  declaration changes; TS changes limited to new test files, the
  dual-layer comment block above `LIGHT_COLOR_TOKENS`, and
  `BRAND_ALIAS_PAIRS` export)
- [ ] PR-A description includes every artefact listed in the
  "PR Descriptions" section, especially the literal-residue
  tables and the corrected `--ac-doc-bg-*` `#1d2030` vs
  `--atomic-night-4` `#1d2131` (green +1, blue +1) near-miss
  callout

---

## Phase 3: Visual-regression evidence

### Overview

Run the existing Playwright `tokens.spec.ts` suite against the
Phase-2 branch. By construction the rewrite preserves resolved hex,
so all 28 baselines should pass without `--update-snapshots`. If
any baseline drifts (e.g. due to a rounding edge case in the
browser's `var()` resolution at very low subpixel deltas), gather
per-pixel ΔE2000 evidence using `culori` and attach to the PR.

### Changes Required

#### 3.1 Add `culori` and `pngjs` as dev dependencies

**File**: `skills/visualisation/visualise/frontend/package.json`

```diff
   "devDependencies": {
+    "culori": "^4.0.1",
+    "pngjs": "^7.0.0",
     ...
   }
```

Pin the version range to the majors in use at plan-cut.
`culori` is tree-shakeable and small; no runtime import.
`pngjs` is the lightest in-process PNG decoder available; chosen
over Sharp (large native binary) and over Playwright's bundled
decoder (only available inside Playwright's runner). Committing
to one decoder up front makes the script's behaviour reproducible
across contributors.

#### 3.2 Add a ΔE2000 diff script (with unit tests)

**File**: `skills/visualisation/visualise/frontend/scripts/visual-diff-ciede2000.ts` (NEW)

Shipped as TypeScript invoked via `npx tsx`, matching the existing
`scripts/scan-css-literals.ts` precedent. The script:

- Reads two PNG paths (baseline + actual) plus an optional output
  path for a heatmap PNG.
- Decodes both PNGs via `pngjs`.
- For each pixel where the RGBA differs by more than 1 channel
  step, computes `differenceCiede2000` via `culori`:

  ```ts
  import { differenceCiede2000, parse } from 'culori'
  const dE = differenceCiede2000()
  const value = dE(parse(`rgb(${r1},${g1},${b1})`), parse(`rgb(${r2},${g2},${b2})`))
  ```

- Reports: pixel count, max ΔE, mean ΔE, 95th-percentile ΔE,
  region bounding boxes of changed pixels.
- Exit code 0 iff max ΔE2000 < 5 across all changed pixels (AC5
  threshold).

The script is structured as small pure functions (decode → diff →
aggregate → format) so the colour-math layer can be unit-tested
without touching the filesystem. Ship file-header doc following
the `scan-css-literals.ts` convention:

```ts
/**
 * Visual diff using CIEDE2000 colour-difference metric.
 *
 * Usage: npx tsx scripts/visual-diff-ciede2000.ts <baseline> <actual>
 *
 * Exit codes:
 *   0 — max ΔE2000 < 5 across all changed pixels (AC5 threshold from
 *       meta/work/0073-atomic-brand-layer-palette.md)
 *   1 — at least one pixel exceeds the threshold
 *   2 — invalid input (missing file, decode error, etc.)
 *
 * See AC5 in 0073 for the threshold rationale; the script is intended
 * to be reusable across future visual-regression PRs.
 */
```

Usage:

```bash
npx tsx scripts/visual-diff-ciede2000.ts \
  tests/visual-regression/__screenshots__/tokens.spec.ts-snapshots/library-light-darwin.png \
  test-results/tokens-library-light-actual.png
```

**Unit tests**: ship `scripts/visual-diff-ciede2000.test.ts` (NEW)
exercising the pure functions against synthetic in-memory PNGs:

- Identical buffers → max ΔE = 0, exit code 0.
- One-channel-off pair → known ΔE value (golden snapshot).
- Three-channel divergent pair pushing ΔE > 5 → exit code 1.
- Mismatched dimensions → exit code 2 with the expected message.

Decoding can be exercised against tiny generated PNG buffers
rather than real screenshots so the test is fast and deterministic.

#### 3.3 Run the Playwright suite

```bash
cd skills/visualisation/visualise/frontend
npm run test:e2e -- tokens.spec.ts
```

Expected outcome: all 28 baselines pass without
`--update-snapshots`. If any baseline drifts:

1. Capture the actual PNG (Playwright writes it to
   `test-results/` on failure).
2. Run `scripts/visual-diff-ciede2000.ts` against the
   corresponding baseline.
3. If max ΔE2000 < 5 across all changed pixels, regenerate that
   baseline (`npm run test:e2e -- tokens.spec.ts
   --update-snapshots <route>-<theme>`) and attach the ΔE2000
   report to the PR.
4. If max ΔE2000 ≥ 5, treat as a real regression — investigate
   before regenerating.

#### 3.4 Run the broader visual-regression set as a sanity check

```bash
npm run test:e2e
```

Covers `glyph-showcase.spec.ts`, `chip-showcase.spec.ts`,
`glyph-resolved-fill.spec.ts`, `chip-resolved-colours.spec.ts`,
`code-block-resolved-colours.spec.ts` in addition to
`tokens.spec.ts`. By construction nothing in the rewrite changes
resolved colour, so these should all pass without regeneration.

### Success Criteria

#### Automated Verification

- [ ] `npm run test:e2e -- tokens.spec.ts` passes without
  `--update-snapshots`
- [ ] `npm run test:e2e` (full Playwright suite) passes without
  `--update-snapshots`
- [ ] `npx tsx scripts/visual-diff-ciede2000.ts` exits 0 for any
  regenerated baseline (max ΔE2000 < 5)
- [ ] `npm test -- scripts/visual-diff-ciede2000.test.ts` passes
  (script unit tests against synthetic PNG fixtures)

#### Manual Verification

- [ ] If any baseline was regenerated: PR description includes the
  per-baseline ΔE2000 report (max / mean / 95th percentile / pixel
  count) and an inline side-by-side image so a reviewer can sanity
  check
- [ ] If no baseline was regenerated: PR description states
  "0 baselines regenerated; Playwright sweep clean against `main`
  baselines"

---

## Phase 4: ADR-0026 amendment

### Overview

Document the brand layer in the canonical decision record so future
contributors find a clear precedent for the brand → semantic
indirection rule. This phase has no code, no tests, and no runtime
impact — it's a documentation update — but it carries the same
review weight because ADR-0026 governs all future token work.

### Changes Required

#### 4.1 Extend §5 (`Theme-invariant token families`)

**File**: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`

Add a paragraph to §5 listing the brand palette as a `:root`-only
family alongside `--code-*` / `--tk-*`. Edit the existing
`§5 — Theme-invariant token families — eligibility criteria`
section (lines 192-206) so the worked example list grows by one:

```markdown
### Theme-invariant token families — eligibility criteria

A future token family is eligible to be `:root`-only (skipping both
dark mirrors) if all of:

1. The values are adopted from an external authoritative source
   (prototype, brand palette) where the source itself does not vary
   by theme;
2. No accessibility differential between light and dark surfaces is
   intended;
3. The family ships with a drift-detection test against its
   authoritative source so the asymmetry cannot regress silently.

Token families currently in this list:

- `--code-*` / `--tk-*` (story 0076) — code-block surface and
  syntax-highlight palettes.
- `--atomic-*` (story 0073) — brand-layer named colours, neutrals,
  aliases, and overlays. Source:
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html:183`.
  Drift test: `src/styles/prototype-tokens.fixture.test.ts`.
  Brand → semantic indirection rule: see §6.

Future contributors adding a `:root`-only family should add a
declaration to this list and document the source.
```

#### 4.2 Add §6 (`Brand-layer indirection`)

**File**: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`

Insert a new top-level section after §5 / §5's appendix and before
the existing "Consequences" section, documenting the
brand → semantic rule introduced by 0073:

```markdown
## 6. Brand-layer indirection

### Context

Story 0033 declared the `--ac-*` semantic layer with hex literals.
Story 0073 introduces the upstream `--atomic-*` brand layer; the
two layers form a deliberate "brand → semantic → consumer" chain
where consumers reference only `var(--ac-*)`, the semantic layer
references `var(--atomic-X)` where exact-hex matches exist, and the
brand layer declares the literal hex (or rgba) once.

This section concerns only brand → semantic indirection between
the `--atomic-*` layer and the `--ac-*` layer. Other `:root`-only
families listed in §5 (e.g. `--code-*` / `--tk-*`) do not
currently have indirection consumers; §5 governs whether a family
is `:root`-only, §6 governs whether a `:root`-only family becomes
a referent for downstream semantic tokens.

### Decision

Where an `--ac-*` semantic token's resolved value (normalised to
lowercase six-digit hex) **exactly matches** an `--atomic-*` brand
value, the semantic declaration is rewritten as
`var(--atomic-X)`. Near-misses stay as hex literals; the PR
introducing such a token must enumerate the residual literals.

**Scope**: the rule applies to six-digit hex values only.
Alpha-bearing `rgba(...)` declarations on the semantic side are
out of scope (overlays and tints stay as semantic-side literals
even when their RGB triplet matches a brand colour, because the
brand layer's overlay tokens are declared as `rgba(...)`
themselves and consolidating them would couple two unrelated
opacity decisions). `color-mix(...)` outputs are also out of
scope; if a future need arises, this section is the place to
revisit.

**Tie-breaker for hex collisions**: when both a brand alias and
its target match a semantic value, prefer the target so that the
indirection chain stays one hop deep (e.g. a future `--ac-brand-purple`
matching `#965dd9` would reference `--atomic-medium-purple`, not
`--atomic-violet` — note the existing `--ac-violet` is `#7b5cd9`,
a near-miss, and stays as a hex literal). When two non-alias brand
tokens share the
same resolved hex (the `--atomic-ash` / `--atomic-geyser`
collision at `#d3dbe0`, or `--atomic-slate-2` / `--atomic-river-bed`
at `#4a545f`), the PR introducing the rewrite picks the token
whose semantic meaning best matches the consuming `--ac-*` and
records the choice in the PR description.

Rationale:

- Single source of truth for brand colours; downstream
  illustrative work (BigGlyph, type-tinted iconography) consumes
  the brand layer directly.
- Brand-layer changes propagate through the semantic layer
  automatically.
- Resolved values are preserved by construction, so the rewrite is
  visually transparent.

### Operational guidance

- Adding a new `--ac-*` token: check if its colour matches an
  existing `--atomic-*` value (normalised lowercase six-digit
  hex). If yes, declare as `var(--atomic-X)`. If no, declare as a
  hex literal AND consider whether a new `--atomic-*` token is
  warranted; if so, add it in the same PR with prototype source
  evidence and update the drift fixture.
- Adding a new `--atomic-*` token: must be sourced from
  `prototype-standalone.html` (or a successor prototype) and
  drift-tested.
- TS-side `BRAND_COLOR_TOKENS` stores resolved hex even for
  aliases; the CSS↔TS parity comparator resolves `var(--atomic-X)`
  references via this map, preserving "TS knows the resolved hex
  of every semantic token" as an invariant. The AC2-invariant
  test in `global.test.ts` enforces this rule going forward.
- Per-theme overrides remain at `--ac-*`; the brand layer is
  theme-invariant.

### References

- Introducing story: `meta/work/0073-atomic-brand-layer-palette.md`
- Implementation plan:
  `meta/plans/2026-05-23-0073-atomic-brand-layer-palette.md`
- Shared canonicaliser:
  `skills/visualisation/visualise/frontend/src/styles/testing/canonicaliseBrand.ts`
- Drift fixture:
  `skills/visualisation/visualise/frontend/src/styles/fixtures/prototype-tokens.json`
- Drift test:
  `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts`
- CSS↔TS parity:
  `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
- Foundation story:
  `meta/work/0033-design-token-system.md`
```

#### 4.3 Update Appendix colour reference (single-sentence note)

**File**: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`

The appendix table at lines 301-323 maps pre-0033 Tailwind hex
values to `--ac-*` tokens. After 0073 those mappings still hold
at the consumer surface, but a reader using the appendix may want
to know that `--ac-*` may now itself resolve through `--atomic-*`.
Add this single sentence at the top of the appendix:

> Some `--ac-*` entries below may resolve through the brand layer
> (`var(--atomic-X)`) after story 0073 — the appendix lists the
> consumer-visible mapping, which is unchanged.

### Success Criteria

#### Automated Verification

- [ ] `npm test` (full suite) passes — no test depends on ADR text,
  so this is just a sanity check
- [ ] ADR markdown lints clean (if `markdownlint` is wired; else
  N/A)

#### Manual Verification

- [ ] ADR-0026 §5 lists `--atomic-*` alongside `--code-*` /
  `--tk-*` with source and drift-test references
- [ ] ADR-0026 §6 documents the brand → semantic indirection rule
- [ ] A reviewer reading ADR-0026 fresh can infer:
  - which families are `:root`-only and why
  - how to add a new brand-layer token
  - the rule for rewriting `--ac-*` to `var(--atomic-X)`

---

## Testing Strategy

### Unit Tests

- `src/styles/testing/canonicaliseBrand.test.ts` (NEW, Phase 2):
  7 unit tests covering `rgb()` normalisation, hex lowercasing,
  brand `var()` resolution, alias resolution (single-hop today,
  recursion-safe by design), `rgba()` pass-through, throw on
  unknown brand refs, pass-through on non-brand `var()` refs.
- `src/styles/global.test.ts` (extended, Phase 1 + 2):
  - Brand-layer CSS↔TS parity (37 new assertions, parameterised).
  - Brand-layer fixture↔TS parity (extends existing
    `:221-235` loop).
  - Theme-invariance guards (2 new, both dark blocks).
  - Alias-target equality (parameterised over
    `BRAND_ALIAS_PAIRS`).
  - AC2-invariant guard (every `--ac-*` hex literal matching a
    brand value must be in the allow-list).
  - Format-guard test (LIGHT/DARK_COLOR_TOKENS values contain no
    `var(--atomic-*)` strings).
- `src/styles/prototype-tokens.fixture.test.ts` (extended,
  Phase 1): drift-detector covers 37 new `--atomic-*` entries,
  imports shared `canonicaliseBrand` from
  `./testing/canonicaliseBrand`; new `extractRootBlockBody` unit
  tests cover content-based selection rule, miss cases, and
  pathological brace input.
- `scripts/visual-diff-ciede2000.test.ts` (NEW, Phase 3): 4 unit
  tests against synthetic PNGs (identical → 0; one-channel-off
  golden; threshold-failing → exit 1; mismatched dimensions →
  exit 2).

### Integration / E2E Tests

- `tests/visual-regression/tokens.spec.ts` (Phase 3): 28 baselines
  × routes × themes pass without `--update-snapshots`.
- All other Playwright specs continue to pass against unchanged
  resolved colours.

### Manual Testing Steps

1. **Phase 1**: `npm run dev`; navigate to `/library`. DevTools →
   Computed → confirm `--atomic-bone` exists on `:root` and
   resolves to `rgb(251, 252, 254)`. Confirm `--ac-bg` still
   resolves to the same hex via its hex literal.
2. **Phase 2**: `npm run dev`; toggle light/dark; confirm zero
   visual difference. DevTools → Computed → confirm `--ac-bg`
   now reads `var(--atomic-bone)` resolving identically to step 1.
3. **Phase 2**: Set OS to dark mode (`prefers-color-scheme: dark`)
   without `[data-theme="dark"]` attribute; confirm dark theme
   activates and renders identically to explicit `data-theme="dark"`.
4. **Phase 3**: Run `npm run test:e2e` and inspect any failures
   in `playwright-report/`.

## Performance Considerations

No runtime performance impact. CSS variable indirection through one
extra `var(--atomic-X)` hop costs nothing the browser doesn't already
pay for the existing `var(--ac-*)` consumer references. Bundle size
grows by ~37 declarations in `global.css` (~1.5 kB raw, less after
gzip).

## PR Descriptions

Two PRs ship under this plan. Each has required artefacts the
reviewer should mechanically check off.

### PR-A: Brand layer foundation + semantic rewrite + ADR amendment
(Phases 1 + 2 + 4)

- **MAIN_CSS_SHA**: record the `main` commit SHA at branch-cut
  so reviewers can re-derive the exact-match candidate set if needed.
- **Light-block literal residues**: full table of `--ac-*` tokens
  in `:root` that stayed as hex literals despite Phase 2's
  consideration, with one-phrase reasons (verbatim from Phase 2
  §2.2).
- **Dark-block literal residues**: full table of `--ac-*` tokens
  in `[data-theme="dark"]` and the `@media` mirror that stayed as
  hex literals, with one-phrase reasons (verbatim from Phase 2
  §2.3). **Must include the explicit `--ac-doc-bg-*` `#1d2030` vs
  `--atomic-night-4` `#1d2131` near-miss callout (green +1, blue
  +1)** so a future maintainer doesn't silently consolidate.
- **Near-miss enumeration**: every `--ac-*` whose hex is within ΔE
  ~5 of a brand value but doesn't satisfy AC2's exact-match rule
  (currently `--ac-violet` vs `--atomic-medium-purple` and the
  `--ac-doc-bg-*` block above).
- **ADR-0026 §6 cross-reference**: link to the new §6 from the PR
  description so reviewers can read the rule alongside the rewrite.
- **Guardrail tests included and passing**: AC2-invariant guard,
  rewrite-count guard, format-guard on `LIGHT_COLOR_TOKENS` /
  `DARK_COLOR_TOKENS`, alias-target equality, theme-invariance
  guards, brand-layer fixture↔TS parity. Allow-list entries (if
  any) must match the literal-residue tables above.

### PR-B: Visual-regression evidence (Phase 3)

- **Baselines regenerated**: explicit statement — either
  "0 baselines regenerated; Playwright sweep clean against `main`
  baselines" or a list of regenerated baselines with ΔE2000
  evidence per baseline (max / mean / p95 / pixel count) plus a
  side-by-side image.
- **Threshold rationale**: link to AC5 confirming max ΔE2000 < 5
  per regenerated baseline.
- **Script self-tests**: link to `scripts/visual-diff-ciede2000.test.ts`
  green so reviewers can verify the colour-math layer is exercised.

## Migration Notes

Not applicable — this is a pure declaration refactor with byte-true
resolved-value preservation. No data migration, no consumer changes,
no backwards compatibility shims.

## References

- Original work item: `meta/work/0073-atomic-brand-layer-palette.md`
- Research: `meta/research/codebase/2026-05-23-0073-atomic-brand-layer-palette.md`
- Pass-1 review: `meta/reviews/work/0073-atomic-brand-layer-palette-review-1.md`
- Source-of-truth artefact:
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html:183`
- ADR being amended:
  `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
- Foundation story:
  `meta/work/0033-design-token-system.md`
- Closest precedent (`--code-*` / `--tk-*` rollout):
  `meta/work/0076-code-block-syntax-highlight-palette.md`
- Downstream consumer:
  `meta/work/0082-bigglyph-hero-illustrations.md`
- Related parallel story:
  `meta/work/0077-shadow-and-dark-accent-token-audit.md`
- Implementation surface (files):
  - `skills/visualisation/visualise/frontend/src/styles/global.css`
  - `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
    (adds `BRAND_COLOR_TOKENS`, `BrandColorToken`,
    `BRAND_ALIAS_PAIRS`, dual-layer comment above
    `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS`)
  - `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
  - `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts`
  - `skills/visualisation/visualise/frontend/src/styles/fixtures/prototype-tokens.json`
  - `skills/visualisation/visualise/frontend/src/styles/testing/canonicaliseBrand.ts` (NEW)
  - `skills/visualisation/visualise/frontend/src/styles/testing/canonicaliseBrand.test.ts` (NEW)
  - `skills/visualisation/visualise/frontend/scripts/visual-diff-ciede2000.ts` (NEW)
  - `skills/visualisation/visualise/frontend/scripts/visual-diff-ciede2000.test.ts` (NEW)
  - `skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts`
