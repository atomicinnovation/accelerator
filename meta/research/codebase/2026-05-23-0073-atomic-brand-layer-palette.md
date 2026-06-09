---
date: "2026-05-23T15:00:31+01:00"
researcher: Toby Clemson
revision: "0303ec8773f59a3c7ecaf875a3e6eb4c09ad92f1"
repository: accelerator
topic: "0073 Atomic Brand-Layer Palette — implementation surface"
tags: [research, codebase, design-tokens, brand-palette, atomic, visualiser, frontend, css, tokens-ts]
status: complete
last_updated: "2026-05-23T00:00:00+00:00"
last_updated_by: Toby Clemson
type: codebase-research
id: "2026-05-23-0073-atomic-brand-layer-palette"
title: "Research: 0073 Atomic Brand-Layer Palette — implementation surface"
author: Toby Clemson
schema_version: 1
relates_to: ["adr:ADR-0026", "work-item:0033", "work-item:0076", "design-gap:2026-05-21-current-app-vs-claude-design-prototype"]
---

# Research: 0073 Atomic Brand-Layer Palette — implementation surface

**Date**: 2026-05-23T15:00:31+01:00
**Researcher**: Toby Clemson
**Git Commit**: 0303ec8773f59a3c7ecaf875a3e6eb4c09ad92f1
**Branch**: HEAD (detached)
**Repository**: accelerator

## Research Question

What is the concrete codebase surface for implementing 0073 — adding the
`--atomic-*` brand-layer palette to the visualiser and rewriting `--ac-*`
semantic tokens to reference it via `var()`? Specifically: what is the
exact set of brand tokens to seed from the prototype, which `--ac-*`
declarations can cleanly be rewritten, what existing test/fixture
infrastructure extends naturally, what governance (ADRs, related work
items) constrains the work, and what hidden risks might bite.

## Summary

- The visualiser frontend lives at
  `skills/visualisation/visualise/frontend/` (not at the repo root the
  story's bare `src/styles/...` paths suggest). All work in scope touches
  that subtree.
- The prototype declares **37** `--atomic-*` tokens, not "~30" as the
  design-gaps doc estimated and not the 11 the story bullets list. A
  faithful mirror must seed all 37 in the new `BRAND_COLOR_TOKENS`
  constant and the new `--atomic-*` section of `prototype-tokens.json`.
- The brand palette is theme-invariant in the prototype — every
  `--atomic-*` declaration lives once in `:root`, with no
  `[data-theme="dark"]` or `@media` overrides. The story's theme-
  invariance assumption holds exactly.
- The rewrite candidate set is **9 declarations in `:root`** (collapsing
  to 6 distinct brand tokens) and **16 declarations across the dark
  mirrors** (collapsing to 3 distinct brand tokens, plus 12 identical
  `--ac-doc-*` glyph entries that all collapse to `var(--atomic-white)`).
- Three risks that the story does not call out: (a) prototype declares
  many `--atomic-*` values in `rgb(...)` form rather than hex, so the
  fixture's existing `canonical()` normaliser cannot compare them — a
  richer normaliser is needed; (b) ADR-0026 does not model a brand layer
  at all, so 0073 is implicitly an ADR-amendment scope and §5's three-
  part eligibility for `:root`-only families needs to be met
  explicitly; (c) `culori` (or any CIEDE2000 implementation) is not yet
  a dependency, and Playwright's `toHaveScreenshot()` does not support
  per-pixel ΔE thresholds natively — AC5 needs a tooling step.

## Detailed Findings

### Visualiser frontend layout

All affected source lives under
`skills/visualisation/visualise/frontend/`. The directory paths the
story names (`src/styles/global.css`, `tokens.ts`, etc.) are anchored
relative to that workspace, not to the repo root.

Core files relevant to 0073:
- `skills/visualisation/visualise/frontend/src/styles/global.css` (375 lines)
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts` (251 lines)
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts` (313 lines)
- `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts` (97 lines)
- `skills/visualisation/visualise/frontend/src/styles/fixtures/prototype-tokens.json` (35 lines)
- `skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts` (88 lines)
- `skills/visualisation/visualise/frontend/playwright.config.ts`
- `skills/visualisation/visualise/frontend/vite.config.ts` (Vitest config lives here)
- `skills/visualisation/visualise/frontend/package.json` (scripts + deps)

Consumer surface: `var(--ac-` appears **462 times across 46 files** in
`src/`. The semantic layer's existing indirection means the rewrite is
transparent to consumers; no component CSS needs to change.

### Prototype `--atomic-*` enumeration (37 declarations)

All declarations sit in a single `:root` block at line 183 of
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
(the file is essentially minified — most CSS is on that one line).

**Named brand colours (23) — `:root`, line 183:**

| Token | Value | Normalised |
|---|---|---|
| `--atomic-night` | `rgb(14, 15, 25)` | `#0e0f19` |
| `--atomic-night-2` | `rgb(10, 17, 27)` | `#0a111b` |
| `--atomic-night-3` | `rgb(23, 25, 37)` | `#171925` |
| `--atomic-night-4` | `rgb(29, 33, 49)` | `#1d2131` |
| `--atomic-ink` | `rgb(32, 34, 49)` | `#202231` |
| `--atomic-ink-2` | `rgb(44, 46, 65)` | `#2c2e41` |
| `--atomic-red` | `rgb(203, 70, 71)` | `#cb4647` |
| `--atomic-red-2` | `rgb(223, 87, 88)` | `#df5758` |
| `--atomic-red-3` | `rgb(226, 78, 83)` | `#e24e53` |
| `--atomic-indigo` | `rgb(89, 95, 200)` | `#595fc8` |
| `--atomic-indigo-2` | `rgb(50, 48, 98)` | `#323062` |
| `--atomic-indigo-tint` | `rgb(193, 197, 255)` | `#c1c5ff` |
| `--atomic-medium-purple` | `#965DD9` | `#965dd9` |
| `--atomic-cream-can` | `#F5C25F` | `#f5c25f` |
| `--atomic-steel-blue` | `#4295A5` | `#4295a5` |
| `--atomic-pastel-green` | `#6BE58B` | `#6be58b` |
| `--atomic-river-bed` | `#4A545F` | `#4a545f` |
| `--atomic-aquamarine` | `#73E4E2` | `#73e4e2` |
| `--atomic-tradewind` | `#52B0AA` | `#52b0aa` |
| `--atomic-geyser` | `#D3DBE0` | `#d3dbe0` |
| `--atomic-malibu` | `#72CBF5` | `#72cbf5` |
| `--atomic-link-water` | `#DDECF4` | `#ddecf4` |
| `--atomic-marigold` | `#F9DE6F` | `#f9de6f` |

**Light neutrals (7) — `:root`, line 183:**

| Token | Value | Normalised |
|---|---|---|
| `--atomic-white` | `rgb(255, 255, 255)` | `#ffffff` |
| `--atomic-bone` | `rgb(251, 252, 254)` | `#fbfcfe` |
| `--atomic-mist` | `rgb(217, 217, 217)` | `#d9d9d9` |
| `--atomic-ash` | `rgb(211, 219, 224)` | `#d3dbe0` |
| `--atomic-smoke` | `rgb(199, 201, 216)` | `#c7c9d8` |
| `--atomic-slate` | `rgb(95, 99, 120)` | `#5f6378` |
| `--atomic-slate-2` | `rgb(74, 84, 95)` | `#4a545f` |

**Aliases (4) — each `var(--atomic-X)`:**

| Token | Resolves to | Resolved hex |
|---|---|---|
| `--atomic-violet` | `var(--atomic-medium-purple)` | `#965dd9` |
| `--atomic-teal` | `var(--atomic-tradewind)` | `#52b0aa` |
| `--atomic-sky` | `var(--atomic-malibu)` | `#72cbf5` |
| `--atomic-sky-2` | `var(--atomic-malibu)` | `#72cbf5` |

**Overlays (3) — `rgba(...)`, theme-invariant:**

| Token | Value |
|---|---|
| `--atomic-overlay-ink` | `rgba(23, 25, 37, 0.56)` (`#171925` @ 0.56 — matches `--atomic-night-3`) |
| `--atomic-stroke-light` | `rgba(255, 255, 255, 0.35)` |
| `--atomic-shadow-soft` | `rgba(0, 0, 0, 0.08)` |

Hex collisions in the brand palette (two tokens with same hex but
distinct semantics):
- `--atomic-ash` and `--atomic-geyser` both resolve to `#d3dbe0`.
- `--atomic-slate-2` and `--atomic-river-bed` both resolve to `#4a545f`.
- `--atomic-sky` and `--atomic-sky-2` both alias `--atomic-malibu`.

A hex-keyed dedup table would lose information; the brand layer must be
keyed by token name. The fixture and the TS constant should both
preserve all 37 entries even when values coincide.

### Hex-to-brand rewrite candidates in `global.css`

Rewrites are evaluated per-declaration-site (theme-invariant brand layer
feeding a theme-variant semantic layer). All matches below are exact
normalised six-digit hex equalities.

**`:root` (light, `global.css:69-228`) — 9 declarations, 6 unique brand tokens:**

| Declaration | Current value | Rewrite |
|---|---|---|
| `--ac-bg` | `#fbfcfe` | `var(--atomic-bone)` |
| `--ac-bg-raised` | `#ffffff` | `var(--atomic-white)` |
| `--ac-bg-chrome` | `#ffffff` | `var(--atomic-white)` |
| `--ac-bg-card` | `#ffffff` | `var(--atomic-white)` |
| `--ac-fg-strong` | `#0a111b` | `var(--atomic-night-2)` |
| `--ac-fg-muted` | `#5f6378` | `var(--atomic-slate)` |
| `--ac-accent` | `#595fc8` | `var(--atomic-indigo)` |
| `--ac-accent-2` | `#cb4647` | `var(--atomic-red)` |
| `--ac-err` | `#cb4647` | `var(--atomic-red)` |

**`[data-theme="dark"]` (`global.css:236-290`) — 16 declarations, 3 unique brand tokens:**

| Declaration | Current value | Rewrite |
|---|---|---|
| `--ac-bg` | `#0a111b` | `var(--atomic-night-2)` |
| `--ac-bg-raised` | `#0e0f19` | `var(--atomic-night)` |
| `--ac-bg-chrome` | `#0e0f19` | `var(--atomic-night)` |
| `--ac-fg-strong` | `#ffffff` | `var(--atomic-white)` |
| all 12 `--ac-doc-*` | `#ffffff` | `var(--atomic-white)` |

(`--ac-doc-bg-*` all sit at `#1d2030`; this is **close but not equal** to
`--atomic-night-4` `#1d2131` and so does NOT match under the AC2 exact-
hex rule — those declarations stay as literals.)

**`@media (prefers-color-scheme: dark)` mirror (`global.css:296-349`):**
must be rewritten byte-equivalently with the same 16 substitutions
since `global.test.ts:132-170` already asserts the two dark blocks are
declaration-set equal.

**Tokens that remain as hex literals and need PR documentation:**
- `--ac-bg-sunken` (`#f4f6fa` light, `#070b12` dark)
- `--ac-bg-sidebar` (`#f7f8fb` light, `#0b121c` dark)
- `--ac-bg-card` dark (`#131524`)
- `--ac-fg` (`#14161f` light, `#e7e9f2` dark)
- `--ac-fg-faint` (`#8b90a3` light, `#6c7088` dark)
- `--ac-fg-muted` dark (`#a0a5b8`)
- `--ac-accent` dark (`#8a90e8`)
- `--ac-accent-2` dark (`#e86a6b`) — close to but ≠ `--atomic-red-2` or `-3`
- `--ac-ok` (`#2e8b57` light, `#79d9a6` dark)
- `--ac-warn` (`#d98f2e` light, `#e4b76e` dark)
- `--ac-err` dark (`#e86a6b`)
- `--ac-violet` (`#7b5cd9`) — close to but ≠ `--atomic-medium-purple` `#965dd9`
- All 12 `--ac-doc-*` light tokens (eyedroppered values, no match)
- All 12 `--ac-doc-bg-*` light tokens (pastels, no match)
- All 12 `--ac-doc-bg-*` dark tokens (`#1d2030`, ≠ `#1d2131`)

This is the set the story's AC2 expects to be enumerated in the PR
description.

### Existing test infrastructure (extension points)

**`global.test.ts:88-102`** — parameterised describe.each block:

```ts
describe.each([
  ['typography', TYPOGRAPHY_TOKENS],
  ['spacing', SPACING_TOKENS],
  ['radius', RADIUS_TOKENS],
  ['light shadow', LIGHT_SHADOW_TOKENS],
  ['layout', LAYOUT_TOKENS],
  ['code surface', CODE_SURFACE_TOKENS],
  ['syntax', CODE_SYNTAX_TOKENS],
])('tokens.ts ↔ global.css :root parity (%s)', ...)
```

Adding `['brand', BRAND_COLOR_TOKENS]` extends parity coverage with no
new infrastructure. `readCssVar(name, 'root')` at `global.test.ts:32-44`
already handles arbitrary kebab-case names via a defensive metacharacter
escape; it will read `--atomic-X` correctly out of the box, provided
`--atomic-*` declarations live in the same flat `:root` block (story
requires this).

**`global.test.ts:64-70`** — light `--ac-*` parity loop. After
rewrites that change a declaration from `#fbfcfe` to `var(--atomic-bone)`,
`readCssVar('ac-bg', 'root')` returns the literal `var(--atomic-bone)`
string, not the resolved hex. **The existing parity test will break for
every rewritten token** because `LIGHT_COLOR_TOKENS['ac-bg']` is
`'#fbfcfe'`, not `'var(--atomic-bone)'`. Two options:
1. Update `LIGHT_COLOR_TOKENS` entries for the rewritten tokens to
   `'var(--atomic-bone)'` etc. — keeps the parity contract literal,
   but loses the resolved-hex assertion on the TS side.
2. Add a `var()`-resolution step in `readCssVar` (or a new comparator)
   that, when the read value matches `^var\(--atomic-([\w-]+)\)$`,
   resolves through `BRAND_COLOR_TOKENS`.

Option 2 preserves the property "TS knows the resolved hex of every
semantic token" and seems more honest. Story does not specify; this is
a real implementation choice that ought to be settled before coding.

**`global.test.ts:132-170`** — the `[data-theme="dark"]` ↔ `@media
(prefers-color-scheme: dark)` parity check. It compares value strings.
If both dark blocks are rewritten identically, this test continues to
pass with literal `var(...)` strings on both sides. No change needed.

**`prototype-tokens.fixture.test.ts:23-47`** — `extractAcCodeblockBlock()`
parses only the `.ac-codeblock { ... }` block (for `--code-*` / `--tk-*`).
The brand palette lives in `:root`. The test must be extended with a
second extractor that pulls declarations from the prototype's `:root`
block, or generalised to a per-selector extractor parameterised by
`.ac-codeblock` and `:root`. The `declarationsOf()` helper at
`prototype-tokens.fixture.test.ts:56-65` currently hard-codes a
`(?:code|tk)-` prefix in its regex — needs broadening to also accept
`atomic-`.

**`prototype-tokens.fixture.test.ts:50-52`** — the existing `canonical()`
normaliser:

```ts
const canonical = (v: string): string => v.toLowerCase().replace(/\s+/g, '')
```

**Critical gap:** this canonicalises `rgba(255,255,255,0.07)` vs
`rgba(255, 255, 255, 0.07)` correctly, but it will NOT make
`rgb(14, 15, 25)` (prototype) compare equal to `#0e0f19` (TS-side bare-
key hex). 23 of the 30 concrete `--atomic-*` declarations in the
prototype are written in `rgb()` form. Without enriching the normaliser
to convert `rgb(r, g, b)` → `#rrggbb` (and accepting both as equal),
the fixture test will fail for nearly every named brand token. AC1
needs to land alongside a normaliser upgrade.

**`prototype-tokens.json:1-35`** — currently a flat object keyed by
`--name`. Append all 37 `--atomic-*` entries. Values can be stored
either as written in the prototype (e.g. `"rgb(14, 15, 25)"`) or pre-
normalised to hex. Storing the prototype's raw form keeps the drift
detector literal; storing the normalised form simplifies TS mirroring.
Story does not specify; recommend the prototype's raw form so the
fixture remains a byte-accurate snapshot, with normalisation pushed
into the comparator.

### Visual regression infrastructure

**`tests/visual-regression/tokens.spec.ts:1-88`** — 6 routes × {light, dark}
+ a `lifecycle-cluster-after-click` pair + a `prefers-color-scheme: dark`
sanity case on `/library`. Uses `toHaveScreenshot()` with
`maxDiffPixelRatio: 0.05` and `animations: 'disabled'`. The
`relativeTimeMask` masks "57s ago"-style chip text.

Baselines under
`tests/visual-regression/__screenshots__/tokens.spec.ts-snapshots/` — 28
PNGs (route × theme × {darwin, linux}).

**Other visual-regression spec files** in the same directory:
- `glyph-showcase.spec.ts` — icon families × sizes × themes (large baseline set)
- `chip-showcase.spec.ts`
- `glyph-resolved-fill.spec.ts`
- `chip-resolved-colours.spec.ts`
- `code-block-resolved-colours.spec.ts`

The story's AC5 only names `tokens.spec.ts`, but if any of the resolved-
colour or showcase specs assert against `--ac-*` colour tokens whose
resolved value changes via the rewrite (it shouldn't — the rewrites are
exact-hex-preserving by construction), they would also need attention.

**ΔE2000 tooling gap.** `culori` is **not** in `package.json` deps.
`differenceCiede2000` would need to be added. Independently, Playwright's
`toHaveScreenshot()` does not natively support a per-pixel ΔE2000
threshold — it supports `maxDiffPixels` / `maxDiffPixelRatio` /
`threshold` (perceptual RGB delta) only. AC5's "max per-pixel ΔE2000 < 5"
implies an out-of-band step where the implementer fetches the actual
and expected baseline PNGs, walks the changed regions, and computes
CIEDE2000 with `culori`. That tooling does not yet exist; the AC asks
for evidence rather than a CI gate.

### ADR-0026 — token application conventions

ADR-0026 (`meta/decisions/ADR-0026-css-design-token-application-conventions.md`)
governs how tokens are authored and consumed. Key constraints relevant
to 0073:

- ADR-0026 does **not** model a brand layer. Its framing is literal →
  semantic only. 0073 adds a brand layer upstream of the semantic
  layer; this is an **extension of the ADR's model**, not a fit into
  an existing slot. The ADR explicitly pushes deviations through
  amendment rather than ad-hoc exception (lines 90-92, 222-225). The
  story's dependency note ("Governed by: ADR-0026 — any deviation
  from the `var(--atomic-X)` rewrite rule requires consulting the
  ADR") may be the wrong direction of governance: it's not deviations
  from the rewrite rule that need amendment, it's the introduction of
  the rewrite rule itself.

- **§5 — `:root`-only families.** Three-part eligibility (lines 192-203):
  1. Adopted from external authoritative source that itself does not
     vary by theme.
  2. No intended accessibility differential between light/dark.
  3. Ships with a drift-detection test against authoritative source.
  
  The `--atomic-*` brand palette meets all three (prototype is the
  source; theme-invariance verified; AC4 ships the drift test). 0073
  should explicitly state §5 compliance and add the brand layer to
  the §5 "list of `:root`-only families" the ADR maintains.

- **Single source of truth** (line 224): no per-context overrides at
  the shared layer. Rewriting `--ac-*` to `var(--atomic-X)` honours
  this because the brand layer is the only place a colour is
  literally declared.

### Related work items

**0033 (Design Token System, status: done)** — `meta/work/0033-design-token-system.md:113-117`
explicitly excludes the `--atomic-*` palette from scope: *"`Brand
palette — --atomic-*` (raw brand colours not consumed by components)"*.
0073 closes that gap directly. The infrastructure 0073 builds on
(`global.css` `:root` + `[data-theme="dark"]`, `tokens.ts` bare-key
constants, `global.test.ts` parity loops) was all delivered by 0033.
0033's AC3 (hex-literal grep) already excludes `global.css`/`tokens.ts`,
so adding brand colours there will not trip prior acceptance criteria.

**0077 (Shadow and Dark-Accent Token Audit, status: draft)** — audit
surface is `--ac-shadow-*` and dark `--ac-accent` / `--ac-accent-2`
only. **No token-name collision with 0073.** Risk vector: 0077 may
revise dark `--ac-accent` to a value that 0073 could later have aliased
to an `--atomic-*` brand colour. Today's dark `--ac-accent` is
`#8a90e8`, which does not match any current `--atomic-*` token, so
there's no rewrite candidate today. If 0077 lands first and chooses
`#595fc8` (the brand indigo) for dark, 0073's dark rewrite set grows
by one. If 0073 lands first, 0077 can simply check against the now-
canonical brand layer.

**0082 (BigGlyph Hero Illustrations, status: draft)** — declares 0073
as a blocker but does NOT import individual `--atomic-*` colours by
name. It consumes per-doc-type hues via 0074's tokens and generates a
seven-tone palette at runtime via `bigPalette(hue)`. 0073 just needs
to ship a complete, iterable `BRAND_COLOR_TOKENS` export; no specific
named imports are required by 0082.

### Design-gap analysis

`meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md:44-54`
is the gap analysis the story closes. It establishes architectural
intent (brand → semantic layering) and downstream consumer list
(AtomicMark, BigGlyph, type-tinted iconography, HexChain/StageTile)
but leaves precise rewrite semantics, ΔE thresholds, and theme
handling to 0073. It also cites the prototype source incorrectly as
`assets/tokens.css:34-105`; the prototype is actually a single self-
contained HTML file with `--atomic-*` declarations inline at line 183
of `prototype-standalone.html`. The story's drafting notes already
flag this correction.

### Test scripts

`skills/visualisation/visualise/frontend/package.json` scripts:
- `test: vitest run`
- `test:watch: vitest`
- `test:coverage: vitest run --coverage`
- `test:e2e: playwright test`
- `test:e2e:ui: playwright test --ui`

Story's AC4 says `npm test -- src/styles/global.test.ts src/styles/prototype-tokens.fixture.test.ts`
— that resolves to `vitest run` with file filters. AC5's visual-
regression run is `npm run test:e2e`.

## Code References

- `skills/visualisation/visualise/frontend/src/styles/global.css:69-228` — `:root` semantic-layer light tokens (rewrite source #1)
- `skills/visualisation/visualise/frontend/src/styles/global.css:236-290` — `[data-theme="dark"]` (rewrite source #2)
- `skills/visualisation/visualise/frontend/src/styles/global.css:296-349` — `@media (prefers-color-scheme: dark)` mirror block (rewrite source #3, must stay byte-equal to #2)
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:1-56` — `LIGHT_COLOR_TOKENS` (pattern for `BRAND_COLOR_TOKENS`)
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts:188-194` — `CODE_SURFACE_TOKENS` (also a useful pattern for theme-invariant brand layer)
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts:32-44` — `readCssVar()` regex pattern (extends without change)
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts:88-102` — `describe.each` block (extension point for brand layer parity)
- `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts:23-47` — `extractAcCodeblockBlock` (needs sibling `extractRootBlock`)
- `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts:50-52` — `canonical()` normaliser (needs rgb→hex extension)
- `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts:56-65` — `declarationsOf()` regex `(?:code|tk)-` prefix (needs broadening)
- `skills/visualisation/visualise/frontend/src/styles/fixtures/prototype-tokens.json:1-35` — fixture (append 37 `--atomic-*` entries)
- `skills/visualisation/visualise/frontend/tests/visual-regression/tokens.spec.ts:1-88` — visual regression spec
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html:183` — single line containing all 37 `--atomic-*` declarations
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md:161-206` — §5 theme-invariant tokens (the eligibility test 0073 must pass)
- `meta/work/0033-design-token-system.md:113-117` — verbatim brand-palette exclusion
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md:44-54` — gap analysis verbatim

## Architecture Insights

- The semantic layer's existing `--ac-*` indirection means consumers
  never see the rewrite; 462 `var(--ac-` references across 46 files
  remain literally unchanged. The brand layer is a pure refactor of
  declarations, not consumers.
- The `:root` + `[data-theme="dark"]` + `@media (prefers-color-scheme: dark)`
  triplet is asserted byte-equivalent between the dark blocks but
  intentionally separate from `:root` so the `readCssVar` helper can
  remain flat-block-only. Rewriting hex → `var(--atomic-X)` in any of
  the three blocks must be mirrored in the others where the token is
  declared in both.
- `prototype-tokens.json` is positioned as a frozen-at-story-open snapshot
  of the prototype's literal declarations. Keeping the fixture in the
  prototype's raw form (mixed `rgb(...)` / `#XXXXXX`) and pushing
  normalisation into the comparator preserves that property.
- The TS-side `LIGHT_COLOR_TOKENS` pattern stores resolved hex with
  bare keys (`'ac-bg': '#fbfcfe'`). For the brand layer this becomes
  `BRAND_COLOR_TOKENS = { 'atomic-night': '#0e0f19', ... }` — including
  the aliases (each storing the resolved hex of its target, not
  `'var(--atomic-medium-purple)'`) keeps the TS side a single source
  of resolved values.
- Hex-collision tokens (`--atomic-ash` / `--atomic-geyser`,
  `--atomic-slate-2` / `--atomic-river-bed`,
  `--atomic-sky` / `--atomic-sky-2`) all carry distinct semantic
  identity in the prototype. Treating the brand layer as a name-keyed
  map (not a hex-keyed map) preserves this.

## Hidden Risks

1. **Existing CSS↔TS parity tests will fail for every rewritten `--ac-*`
   declaration** unless either (a) `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS`
   entries are rewritten to `'var(--atomic-X)'` literal strings, or
   (b) `readCssVar`/its comparator learns to resolve `var(--atomic-X)`
   through the new `BRAND_COLOR_TOKENS`. The story does not specify.
   Recommendation: option (b) — keep `LIGHT_COLOR_TOKENS` as resolved
   hex; let the parity comparator resolve var()-references through
   the brand layer. This preserves the invariant "TS-side tokens know
   their resolved hex" which is what the parity test was originally
   designed to verify.

2. **The fixture normaliser cannot compare `rgb(...)` to `#XXXXXX`.**
   Most named `--atomic-*` declarations in the prototype use `rgb()`
   form, but the obvious TS-side representation is six-digit hex.
   Without enriching `canonical()` to normalise `rgb(r, g, b)` →
   `#rrggbb`, AC1 (CSS↔TS parity over brand layer) will fail for
   ~23 of the 30 concrete tokens. This is an unstated AC1 sub-task.

3. **ADR-0026 does not contemplate a brand layer.** 0073 implicitly
   extends the ADR's mental model. The cleanest implementation path
   adds the brand palette to §5's `:root`-only families list and
   notes the new "brand → semantic" indirection rule. Whether this
   needs a formal ADR amendment or can ship as a single-PR ADR edit
   is a judgement call. Story currently treats ADR-0026 as a
   constraint to consult rather than a document to amend.

4. **`culori` is not a dependency**, and Playwright `toHaveScreenshot()`
   has no native ΔE2000 mode. AC5 ("max per-pixel ΔE2000 < 5") is an
   evidence-attached-to-PR requirement, not a CI gate. The implementer
   needs to add `culori` and write an out-of-band script that diffs
   baseline + actual PNGs region-by-region. Without this tooling, AC5
   is unenforceable. Worth raising before sizing.

5. **`--ac-doc-bg-*` dark `#1d2030` vs `--atomic-night-4` `#1d2131`** —
   these differ by one bit in the green channel. AC2's "exact normalised
   hex" rule means they stay as literals, but a future maintainer
   might (wrongly) "fix" this near-miss. PR description should call
   out the near-miss so it's not silently consolidated later.

6. **The story bullet ("named tokens such as `--atomic-night`,
   `--atomic-ink`, `--atomic-indigo`, ...") undercounts dramatically.**
   Story authors should not be surprised when the fixture grows by
   37 entries and `BRAND_COLOR_TOKENS` exports 37 keys. AC1's
   "every `--atomic-*` declaration parsed from prototype-standalone.html"
   correctly leaves the count free, but a reviewer expecting "around
   30" might balk at 37. Worth communicating up-front.

## Historical Context

- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` —
  the ADR 0073 references; §5 governs the theme-invariant family
  pattern the brand palette must qualify for.
- `meta/work/0033-design-token-system.md` (status: done) — landed
  the semantic layer 0073 builds on; explicitly excluded the brand
  layer (lines 113-117). 0073 closes that gap.
- `meta/work/0076-code-block-syntax-highlight-palette.md` (cited
  via `tokens.ts:186` and ADR-0026 §5) — established the
  `:root`-only + fixture-drift pattern that 0073 reuses for the
  brand palette. The `--code-*` / `--tk-*` rollout is the closest
  precedent for what 0073 is doing.
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md` —
  the gap analysis. Identifies brand-layer adoption as a foundation
  for AtomicMark, BigGlyph, type-tinted iconography, HexChain, and
  StageTile.

## Related Research

No prior research document in `meta/research/codebase/` specifically
covers the brand palette. The closest adjacent topics are token-system
work landed via 0033 (not separately researched in codebase/) and the
design-inventory and design-gap documents under `meta/research/`.

## Open Questions

1. **Parity-test failure mode resolution (Hidden Risk #1):** keep
   `LIGHT_COLOR_TOKENS` storing resolved hex (and resolve `var()`-refs
   in the comparator), or rewrite TS entries to literal `'var(--atomic-X)'`
   strings? The story does not specify.

2. **Fixture value form (raw `rgb(...)` vs normalised hex):** store
   `--atomic-night` as `"rgb(14, 15, 25)"` (byte-true snapshot of
   prototype) or `"#0e0f19"` (normalised, easier to TS-mirror)? Choice
   is coupled to the `canonical()` normaliser upgrade.

3. **ADR-0026 amendment scope:** is the brand-layer introduction an
   ADR-amendment-in-this-PR scope, or does it ship as code-only with
   ADR-0026 left untouched (relying on §5's "future contributors
   adding a `:root`-only family should add a declaration to this
   list" clause as informal authorisation)?

4. **AC5 ΔE2000 evidence tooling:** add `culori` and a diff script as
   part of 0073, or treat the ΔE2000 evidence as an ad-hoc one-off
   for this PR? Affects how subsequent visual-regression PRs handle
   similar evidence.

5. **Naming convention for the new TS constant:** `BRAND_COLOR_TOKENS`
   (matches `LIGHT_COLOR_TOKENS` / `DARK_COLOR_TOKENS` precedent) or
   `ATOMIC_COLOR_TOKENS` (mirrors the `--atomic-` prefix and the
   `CODE_SYNTAX_TOKENS` prefix pattern)? Story's Open Question #2
   asks "new export in `tokens.ts` or sibling file `brand-tokens.ts`?";
   the codebase precedent (six tokens groups all coexist in
   `tokens.ts` today) argues for a new export in the same file.

6. **`--atomic-overlay-ink` derivation:** the prototype declares
   `rgba(23, 25, 37, 0.56)` as a literal; the RGB triplet matches
   `--atomic-night-3`. Should the brand layer encode this as
   `color-mix(in srgb, var(--atomic-night-3) 56%, transparent)` to
   reflect the derivation, or keep it as a literal `rgba()`? Literal
   matches the prototype byte-for-byte (preferred for drift testing);
   derivation reads more honestly. Either way, the story's Non-Goal
   "overlay tokens may be deferred to 0077 if theme-dependent" still
   applies (they're not theme-dependent in the prototype source —
   verified directly).
