---
date: "2026-05-21T23:30:00+01:00"
type: plan
skill: create-plan
work-item: "meta/work/0076-code-block-syntax-highlight-palette.md"
status: accepted
review: "meta/reviews/plans/2026-05-21-0076-code-block-syntax-highlight-palette-review-1.md"
---

# Code-Block Syntax-Highlight Palette Implementation Plan

## Overview

Ship the prototype's self-contained `--code-*` and `--tk-*` token set
through the visualiser's token system, drive both render pathways
(`MarkdownRenderer` + `TemplateHighlight`) from a single shared
`.hljs-*` → `--tk-*` CSS layer, and retire the
`highlight.js/styles/github.css` global theme plus the local
`.previewBody :global(.hljs-*)` rules. Test-driven throughout, organised
into four phases sequenced so each lands a green, mergeable unit of
value, with phases 3 and 4 fully independent of each other once phase 2
is in.

## Current State Analysis

The visualiser already has a mature token system at `src/styles/`,
shipped by work item 0033. The `--ac-*` family + `--sp-*`, `--radius-*`,
`--size-*`, `--shadow-*` families all flow through the
`global.css` ↔ `tokens.ts` ↔ `global.test.ts` parity contract. Code
blocks have NO place in that contract today:

- `src/components/MarkdownRenderer/MarkdownRenderer.module.css:13-21`
  defines `.markdown pre` with hard-coded `#1e1e1e` background and
  `#d4d4d4` text. Both literals are currently classified
  **irreducible** by ADR-0026 §3 and reflected in
  `src/styles/migration.test.ts:59-60`.
- `src/main.tsx:7` imports `highlight.js/styles/github.css` to colour
  the `.hljs-*` spans `rehype-highlight` emits.
- `src/routes/library/LibraryTemplatesView.module.css:153-215` declares
  a parallel, locally-scoped hljs theme that maps to generic accent/fg
  tokens rather than a named code-syntax palette. The module's own
  comment block (lines 153-156) signals the intent to share, but the
  rules never made the leap to a reusable layer.

The two pathways do not cross-pollinate today:
`MarkdownRenderer` consumes `highlight.js/styles/github.css` (whatever
that ships) and `TemplateHighlight` consumes the locally-scoped
`.previewBody` rules. There is no single source of truth for code
colours in the application.

The prototype at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
declares its palette **once**, on `.ac-codeblock`, with no light/dark
overrides — the same values render against both surfaces in the
prototype's fullpage screenshots. Five `--code-*` surface tokens and
twenty-seven `--tk-*` syntax tokens.

Key constraints discovered:

- **`global.test.ts:readCssVar` requires flat blocks.** Nested rules
  inside `:root`/`[data-theme="dark"]` silently truncate the regex; a
  truncation guard at lines 197-204 asserts a known-last token is still
  readable. Plan must add a new truncation-guard line for whatever
  trailing `--tk-*` lands last.
- **`global.test.ts:127-165` MIRROR-A/B parity check** is set-based. A
  family declared only under `:root` does NOT appear in either dark
  mirror — that is correct behaviour for a theme-invariant family and
  no special handling is needed.
- **Vitest/jsdom does not substitute `var()` reliably for cascaded
  colour resolution** (documented at `Glyph.test.tsx:167-170`). The
  repo has a working Playwright precedent for resolved-colour
  assertions at `tests/visual-regression/glyph-resolved-fill.spec.ts`
  and `chip-resolved-colours.spec.ts`. AC2/AC3 must use this pattern.
- **`migration.test.ts:46-67` EXCEPTIONS entries** are a forward-
  pressure mechanism: leaving `#1e1e1e`/`#d4d4d4` in EXCEPTIONS after
  the migration would let future contributors re-add those literals.
  ADR-0026 §3 lists them as irreducible; both must be revised in lock-
  step with the `<pre>` chrome migration.
- **`LibraryTemplatesView.test.tsx:198-199`** locks the presence of
  `.previewBody :global(.hljs-meta)` and
  `.previewBody :global(.hljs-template-variable)` rules in the CSS
  module text. Phase 4 must invert those assertions (rules must NOT
  remain) and add new assertions on the shared layer.
- **`rehype-highlight` is passed bare** (no `subset`/`languages`
  option) in `MarkdownRenderer.tsx:45`. Default common-language bundle
  + auto-detect is in force; explicit `language-*` fence labels are
  the reliable way to target a grammar.
- **`hljs-template-variable` is emitted only by
  `template-highlight.tsx:63-68`** — not by stock highlight.js. Mapping
  it to `--tk-var` in the shared layer is templates-preview-only by
  emission, but harmless if any future markdown content surfaces the
  class.

## Desired End State

After all four phases land:

- `src/styles/global.css` declares the full 5+27 `--code-*` / `--tk-*`
  token set under `:root` only (theme-invariant). The
  `[data-theme="dark"]` and `@media (prefers-color-scheme: dark)`
  blocks do not duplicate these tokens; the family is intentionally
  outside the dark-override surface.
- `src/styles/tokens.ts` exposes `CODE_SURFACE_TOKENS` and
  `CODE_SYNTAX_TOKENS` (+ key-type exports), mirroring every `global.css`
  declaration value-for-value (lowercase).
- `src/styles/fixtures/prototype-tokens.json` captures the
  prototype's `--code-*`/`--tk-*` constants verbatim (uppercase, as in
  the source). `prototype-tokens.fixture.test.ts` reads the
  `prototype-standalone.html` source and asserts byte-equivalent (case-
  and whitespace-normalised) parity with the fixture — prototype drift
  surfaces as a test failure. The `fixtures/` subdirectory is new
  (no prior precedent in the visualiser) and documented in ADR-0026
  §5 as the home for committed snapshots of external authoritative
  sources.
- `src/styles/global.test.ts` asserts three-way parity between
  `global.css`, `tokens.ts`, and the fixture for the new families.
- A new shared CSS layer at `src/styles/code-syntax.global.css` maps
  every required `.hljs-*` class to a `var(--tk-*)` reference, with
  `.language-diff` descendant selectors for the four diff rows.
- `src/main.tsx` imports `./styles/code-syntax.global.css` (after
  `./styles/global.css`) and no longer imports
  `highlight.js/styles/github.css`.
- `MarkdownRenderer.module.css` `.markdown pre` consumes
  `var(--code-bg)`, `var(--code-fg)`, `var(--code-stroke)`, and
  `var(--code-fg-faint)` rather than `#1e1e1e`/`#d4d4d4`. Border-radius
  stays at `6px` (irreducible — between `--radius-sm` and
  `--radius-md`; existing EXCEPTION at `migration.test.ts:67` is
  retained).
- `LibraryTemplatesView.module.css` no longer contains any
  `.previewBody :global(.hljs-*)` rule (lines 153-215 removed). The
  `.tpl-line` rules (lines 148-151) and all `.previewPane`/
  `.previewHeader`/`.previewBody` container chrome (lines 104-146) are
  retained. The `.previewBody` font-size, line-height, padding stay as
  they are.
- `ADR-0026` §3 no longer lists `#1e1e1e`/`#d4d4d4` as irreducible.
  A new top-level **§5 "Code-block surface and syntax palette"**
  documents the `--code-*`/`--tk-*` family, codifies "theme-invariant
  token families" as a named convention with eligibility criteria,
  and explains the rationale for declaring the palette only in `:root`.
  §5 references story 0076 and `code-syntax.global.css`.
- `migration.test.ts:EXCEPTIONS` no longer contains the two color
  entries for `MarkdownRenderer.module.css`. The corresponding
  `expect(violations(...)).toEqual([])` continues to pass because
  the literals are now expressed as `var(--code-*)`.
- The Playwright spec at
  `tests/visual-regression/code-block-resolved-colours.spec.ts` asserts
  `getComputedStyle(span).color` matches the canonical form of each
  required `--tk-*` token in light AND dark themes (same values),
  for both the markdown renderer (via a new `/code-syntax-showcase`
  dev route) and the templates preview (via `/library/templates/<fixture>`).
- `MarkdownRenderer.test.tsx` extends with six AC4 behaviours: GFM
  task list checkbox, GFM table, wiki-link routing, python
  `language-` fence emits `.hljs-keyword`, typescript `language-`
  fence emits `.hljs-keyword`, fenced+wiki-link adjacency.
- `LibraryTemplatesView.test.tsx` asserts (via CSS-module text
  inspection) that no `.previewBody :global(.hljs-*)` rule remains
  and that, for each previously-mapped class, the shared
  `code-syntax.global.css` declares the right `var(--tk-*)`.

### Key Discoveries

- The prototype value source extracted verbatim from
  `prototype-standalone.html` (after the brand `:root` block and
  before the `body` rule):
  `--code-bg #0E1320`, `--code-bg-head #161B2C`,
  `--code-stroke rgba(255,255,255,0.07)`, `--code-fg #D7DCEC`,
  `--code-fg-faint #6F7796`; and 27 `--tk-*` tokens (full enumeration
  in story 0076's Context section).
- The story's `src/design/...` path references are non-existent. The
  de-facto design-token home is `src/styles/`. This plan uses
  `src/styles/code-syntax.global.css` (mirrors the existing
  `wiki-links.global.css` convention) and
  `src/styles/fixtures/prototype-tokens.json` (introduces the
  `fixtures/` subdirectory — see ADR-0026 §5).
- The existing global `.hljs` neutralisation at
  `LibraryTemplatesView.module.css:171-174`
  (`background: transparent; color: inherit`) becomes redundant once
  github.css is removed — the shared layer ships an equivalent base
  rule at the global scope.
- AC4's "auto-detected python / typescript" requirement is satisfied
  more deterministically by emitting explicit `language-python` /
  `language-typescript` fences in the test fixture. `rehype-highlight`
  honours the language hint via `language-*` class on the `<code>`
  element.
- The `hljs-code` class referenced in AC5 (templates preview) is NOT
  in the story's mapping table. This plan adds it to the
  `--tk-com` mapping group alongside `hljs-comment` and `hljs-quote`,
  treating it the same way as the existing `.previewBody` mapping
  did (the templates-preview test fixture must continue to render
  it).

## What We're NOT Doing

- **No new `--radius-code` token.** `MarkdownRenderer.module.css`
  `border-radius: 6px` stays as an EXCEPTION entry (irreducible —
  between `--radius-sm` 4px and `--radius-md` 8px). Out of scope.
- **No light-mode contrast tuning.** The palette is intentionally
  theme-independent per the prototype. Accessibility re-tuning is a
  future story.
- **No `template-highlight.tsx` refactor.** The post-processor that
  wraps `{{template variables}}` in `hljs-template-variable` is
  orthogonal to the palette work and continues to use that class
  unchanged (story Technical Notes §5).
- **No new `--tk-*` mappings beyond the Requirements table** (plus the
  added `.hljs-code` row). The six unmapped tokens (`--tk-macro`,
  `--tk-key`, `--tk-flag`, `--tk-heredoc`, `--tk-lifet`,
  `--tk-atrule`) ship as defined-but-unused.
- **No DevDesignSystem reference page changes.** Story 0083 owns the
  showcase of the `--tk-*` primitives; the lightweight
  `/code-syntax-showcase` route added here is a dev/test-only fixture
  surface for Playwright assertions, NOT a documented design-system
  page.
- **No backward-compat aliasing.** `highlight.js/styles/github.css` is
  removed in phase 3 (after the shared layer covers every class the
  visualiser emits); no transitional re-export.
- **No `<pre>` typography or width changes.** Story 0075 owns the
  size-scale work. If 0075 has not landed when phase 3 begins, this
  plan touches only the `background`/`color`/border properties of
  `.markdown pre`, leaving size and padding alone.

## Implementation Approach

Test-driven, four phases, sequenced for incremental green:

- **Phase 1** lays the foundation (tokens, helpers, fixture, drift
  test). Adds no consumer; landing this phase changes nothing
  app-visible. No phase depends on phase 1 outputs.
- **Phase 2** adds the shared CSS layer and wires the markdown
  renderer to it via `main.tsx`. With github.css still imported, the
  shared layer's selectors win on load order. Playwright proves the
  live cascade. This is where the application starts visibly using
  the new palette for markdown code blocks.
- **Phase 3** is the cleanup pass for markdown code blocks: migrate
  the `<pre>` chrome literals to the new tokens, amend ADR-0026,
  remove the EXCEPTIONS entries, remove the github.css import. This
  phase only depends on phase 2 having shipped (so github.css is
  removable without regressing other classes).
- **Phase 4** migrates the templates preview to consume the shared
  layer. Independent of phase 3 — only depends on phase 2. Touches
  `LibraryTemplatesView.module.css` and `LibraryTemplatesView.test.tsx`
  exclusively; no overlap with phase 3's MarkdownRenderer changes.

Each phase follows the same internal rhythm: tests first (extending
existing suites where possible — `global.test.ts`,
`MarkdownRenderer.test.tsx`, `LibraryTemplatesView.test.tsx`), watch
them fail, then make the implementation change, watch them pass.

---

## Phase 1: Token foundation

### Overview

Land the `--code-*` and `--tk-*` token declarations in `global.css`,
mirror them in `tokens.ts`, add the prototype fixture, extend
`global.test.ts` for parity, add the drift-detection test, and add the
`hexToRgbString` helper to `contrast.ts`. After this phase: tokens
exist, parity is enforced, helper is available. No CSS consumer
references the tokens yet.

### Changes Required

#### 1. Prototype fixture (new file)

**File**: `skills/visualisation/visualise/frontend/src/styles/fixtures/prototype-tokens.json`
**Changes**: New file in a new `fixtures/` subdirectory under
`src/styles/` (chosen over Jest's `__fixtures__/` dunder convention,
which has no precedent in the visualiser, and over flat co-location,
which would mix fixture data with source modules). Flat JSON map of
every `--code-*` and `--tk-*` declaration from
`prototype-standalone.html`'s `.ac-codeblock` block, keys including
the leading `--`, values preserving the prototype's case (uppercase
hex). The `fixtures/` directory is the new home for committed
snapshots of external authoritative sources; document this scope in
ADR-0026 §5 alongside the code-block convention.

```json
{
  "--code-bg":       "#0E1320",
  "--code-bg-head":  "#161B2C",
  "--code-stroke":   "rgba(255,255,255,0.07)",
  "--code-fg":       "#D7DCEC",
  "--code-fg-faint": "#6F7796",
  "--tk-com":        "#6F7796",
  "--tk-str":        "#6BE58B",
  "--tk-num":        "#F9DE6F",
  "--tk-kw":         "#C1C5FF",
  "--tk-lit":        "#F9A66B",
  "--tk-typ":        "#73E4E2",
  "--tk-fn":         "#FFC1A8",
  "--tk-attr":       "#C18CF0",
  "--tk-deco":       "#C18CF0",
  "--tk-macro":      "#DF9CE6",
  "--tk-var":        "#72CBF5",
  "--tk-key":        "#C1C5FF",
  "--tk-flag":       "#F9DE6F",
  "--tk-heredoc":    "#C18CF0",
  "--tk-pun":        "#8990B0",
  "--tk-lifet":      "#F9A66B",
  "--tk-header":     "#C18CF0",
  "--tk-anchor":     "#DF9CE6",
  "--tk-tag":        "#DF5758",
  "--tk-doctype":    "#C18CF0",
  "--tk-bn":         "#72CBF5",
  "--tk-prop":       "#72CBF5",
  "--tk-sel":        "#FFC1A8",
  "--tk-atrule":     "#C18CF0",
  "--tk-dhdr":       "#C18CF0",
  "--tk-dhunk":      "#72CBF5",
  "--tk-dadd":       "#6BE58B",
  "--tk-ddel":       "#E56B7E"
}
```

#### 2. Drift-detection test (new file, written first)

**File**: `skills/visualisation/visualise/frontend/src/styles/prototype-tokens.fixture.test.ts`
**Changes**: New Vitest spec. Reads
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
via:

```ts
const PROTOTYPE_PATH = resolve(
  process.cwd(),
  '..', '..', '..', '..',
  'meta', 'research', 'design-inventories',
  '2026-05-21-015231-claude-design-prototype',
  'prototype-standalone.html',
)
const source = readFileSync(PROTOTYPE_PATH, 'utf-8')
```

(four `..` walks from `skills/visualisation/visualise/frontend/`
to the repo root; mirrors the `fonts.test.ts:1-29` precedent).
Extracts the `.ac-codeblock { ... }` block via brace-balanced
scanning, normalises declarations into a `Map<name,value>` (strip
ALL whitespace from values via `.replace(/\s+/g, '')` so
`rgba(255,255,255,0.07)` and `rgba(255, 255, 255, 0.07)` compare
equal), and asserts byte-equal (case-normalised) parity with the
JSON fixture loaded by direct import.

Test cases:
- `it('every prototype token is captured in the fixture')` — keys parity.
- `it('every fixture value matches the prototype source')` — values parity (case-insensitive on the value side).
- `it('fixture introduces no token absent from the prototype')` — symmetric check.

This test fails before step 1 lands and passes after; it then serves
as the drift detector going forward.

#### 3. `hexToRgbString` helper

**File**: `skills/visualisation/visualise/frontend/src/styles/contrast.ts`
**Changes**: Add an exported helper alongside `parseHex` /
`parseRgba`. Test first.

**Test file**: `skills/visualisation/visualise/frontend/src/styles/contrast.test.ts`
(extend the existing file if present; create otherwise). Cases:

```ts
import { hexToRgbString, formatRgba } from './contrast'

describe('hexToRgbString', () => {
  it('lowercase 6-digit hex → rgb(R, G, B) with Chromium-style spacing', () => {
    expect(hexToRgbString('#0e1320')).toBe('rgb(14, 19, 32)')
  })
  it('uppercase 6-digit hex normalises identically', () => {
    expect(hexToRgbString('#0E1320')).toBe('rgb(14, 19, 32)')
  })
  it('white and black edges', () => {
    expect(hexToRgbString('#ffffff')).toBe('rgb(255, 255, 255)')
    expect(hexToRgbString('#000000')).toBe('rgb(0, 0, 0)')
  })
  it('throws with "not 6 hex digits" diagnostic on 3-digit hex', () => {
    expect(() => hexToRgbString('#abc')).toThrow(/not 6 hex digits/)
  })
  it('throws with "missing #" diagnostic on bare hex', () => {
    expect(() => hexToRgbString('0e1320')).toThrow(/missing leading #/)
  })
  it('throws with "non-hex characters" diagnostic on invalid characters', () => {
    expect(() => hexToRgbString('#ZZZZZZ')).toThrow(/non-hex characters/)
  })
  it('throws on 8-digit hex with alpha', () => {
    expect(() => hexToRgbString('#0e1320ff')).toThrow(/not 6 hex digits/)
  })
})

describe('formatRgba', () => {
  it('formats with Chromium canonical spacing', () => {
    expect(formatRgba(255, 255, 255, 0.07)).toBe('rgba(255, 255, 255, 0.07)')
  })
  it('preserves integer alpha', () => {
    expect(formatRgba(0, 0, 0, 1)).toBe('rgba(0, 0, 0, 1)')
  })
})
```

**Implementation**:

```ts
// hexToRgbString deliberately rejects 3-digit shorthand — the
// prototype palette uses 6-digit hex exclusively, and accepting
// shorthand silently in this helper would mask typos. parseHex
// (the older sibling) accepts shorthand for backwards-compatibility
// but is intentionally not reused here.
export function hexToRgbString(hex: string): string {
  if (!hex.startsWith('#')) {
    throw new Error(`hexToRgbString: missing leading # — got "${hex}"`)
  }
  const body = hex.slice(1)
  if (body.length !== 6) {
    throw new Error(`hexToRgbString: not 6 hex digits — got "${hex}" (${body.length} chars after #)`)
  }
  if (!/^[0-9a-f]{6}$/i.test(body)) {
    throw new Error(`hexToRgbString: non-hex characters in body — got "${hex}"`)
  }
  const { r, g, b } = parseHex(hex)
  return `rgb(${r}, ${g}, ${b})`
}

// formatRgba returns the Chromium canonical rgba() form so
// resolved-colour assertions on rgba-valued tokens (e.g.
// --code-stroke) can use a single canonicalisation point instead
// of inlining the format string per Playwright spec.
export function formatRgba(r: number, g: number, b: number, a: number): string {
  return `rgba(${r}, ${g}, ${b}, ${a})`
}
```

(`parseHex` already exists at lines 1-15.)

#### 4. Token declarations in `global.css`

**File**: `skills/visualisation/visualise/frontend/src/styles/global.css`
**Changes**: Insert two new comment-headed declaration blocks inside
`:root` (after the existing Layout block at lines 175-178, before
`color-scheme: light dark`). NO entries added to either dark mirror —
the family is theme-invariant per the prototype.

```css
  /* Code-block surface tokens — theme-independent palette adopted
     from the design prototype's .ac-codeblock block (see
     meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/
     prototype-standalone.html). Same values resolve in both light and
     dark themes; declared only under :root by design (see ADR-0026 §5). */
  --code-bg:        #0e1320;
  --code-bg-head:   #161b2c;
  --code-stroke:    rgba(255, 255, 255, 0.07);
  --code-fg:        #d7dcec;
  --code-fg-faint:  #6f7796;

  /* Syntax-highlight tokens (--tk-* — "tk" = "syntax token", per the
     prototype's naming; kept verbatim so the drift fixture matches the
     prototype byte-for-byte). Each token names a semantic colour role
     the code-syntax.global.css layer maps onto one or more hljs class
     names. Theme-invariant. The six tokens marked "reserved" below have
     no current hljs selector consumer; they ship with the prototype
     palette and are reserved for future grammar support — do not
     remove without coordinating with prototype-tokens.fixture.test. */
  --tk-com:      #6f7796;  /* comment */
  --tk-str:      #6be58b;  /* string literal */
  --tk-num:      #f9de6f;  /* number literal */
  --tk-kw:       #c1c5ff;  /* keyword */
  --tk-lit:      #f9a66b;  /* boolean/null literal */
  --tk-typ:      #73e4e2;  /* type / class name */
  --tk-fn:       #ffc1a8;  /* function name */
  --tk-attr:     #c18cf0;  /* attribute / yaml key */
  --tk-deco:     #c18cf0;  /* @decorator / hljs-meta */
  --tk-macro:    #df9ce6;  /* reserved — C/Rust macros */
  --tk-var:      #72cbf5;  /* variable / template-variable */
  --tk-key:      #c1c5ff;  /* reserved — key-value key (some grammars) */
  --tk-flag:     #f9de6f;  /* reserved — CLI/option flags */
  --tk-heredoc:  #c18cf0;  /* reserved — heredoc bodies */
  --tk-pun:      #8990b0;  /* punctuation */
  --tk-lifet:    #f9a66b;  /* reserved — Rust lifetimes */
  --tk-header:   #c18cf0;  /* markdown section heading */
  --tk-anchor:   #df9ce6;  /* link / symbol */
  --tk-tag:      #df5758;  /* HTML/XML tag */
  --tk-doctype:  #c18cf0;  /* HTML doctype */
  --tk-bn:       #72cbf5;  /* built-in */
  --tk-prop:     #72cbf5;  /* object property */
  --tk-sel:      #ffc1a8;  /* CSS selector */
  --tk-atrule:   #c18cf0;  /* reserved — CSS @-rule */
  --tk-dhdr:     #c18cf0;  /* diff file header */
  --tk-dhunk:    #72cbf5;  /* diff hunk header */
  --tk-dadd:     #6be58b;  /* diff addition */
  --tk-ddel:     #e56b7e;  /* diff deletion */
```

(Values are lowercased per the existing `:root` casing convention; the
fixture preserves uppercase for byte-equivalent drift detection
against the prototype source.)

#### 5. `tokens.ts` mirror

**File**: `skills/visualisation/visualise/frontend/src/styles/tokens.ts`
**Changes**: Add two new `as const` objects and matching type
exports. Pattern matches the existing families.

```ts
// Code-block surface tokens — theme-independent palette adopted from
// the design prototype's .ac-codeblock block (meta/research/
// design-inventories/2026-05-21-015231-claude-design-prototype/
// prototype-standalone.html). Same values resolve in both light and
// dark themes (declared only in `:root`). See ADR-0026 §5 and story
// meta/work/0076-code-block-syntax-highlight-palette.md.
export const CODE_SURFACE_TOKENS = {
  'code-bg':        '#0e1320',
  'code-bg-head':   '#161b2c',
  'code-stroke':    'rgba(255, 255, 255, 0.07)',
  'code-fg':        '#d7dcec',
  'code-fg-faint':  '#6f7796',
} as const

// Syntax-highlight tokens. The `tk-` prefix means "syntax token" and
// is preserved from the prototype source so the drift fixture matches
// byte-for-byte. Theme-invariant. Six tokens marked "reserved" below
// ship with the prototype palette but have no current hljs selector
// in code-syntax.global.css — they are kept to preserve the
// prototype-parity contract; do not delete without coordinating with
// prototype-tokens.fixture.test.ts.
export const CODE_SYNTAX_TOKENS = {
  'tk-com':      '#6f7796', // comment
  'tk-str':      '#6be58b', // string literal
  'tk-num':      '#f9de6f', // number literal
  'tk-kw':       '#c1c5ff', // keyword
  'tk-lit':      '#f9a66b', // boolean/null literal
  'tk-typ':      '#73e4e2', // type / class name
  'tk-fn':       '#ffc1a8', // function name
  'tk-attr':     '#c18cf0', // attribute / yaml key
  'tk-deco':     '#c18cf0', // @decorator / hljs-meta
  'tk-macro':    '#df9ce6', // reserved — C/Rust macros
  'tk-var':      '#72cbf5', // variable / template-variable
  'tk-key':      '#c1c5ff', // reserved — key-value key
  'tk-flag':     '#f9de6f', // reserved — CLI/option flags
  'tk-heredoc':  '#c18cf0', // reserved — heredoc bodies
  'tk-pun':      '#8990b0', // punctuation
  'tk-lifet':    '#f9a66b', // reserved — Rust lifetimes
  'tk-header':   '#c18cf0', // markdown section heading
  'tk-anchor':   '#df9ce6', // link / symbol
  'tk-tag':      '#df5758', // HTML/XML tag
  'tk-doctype':  '#c18cf0', // HTML doctype
  'tk-bn':       '#72cbf5', // built-in
  'tk-prop':     '#72cbf5', // object property
  'tk-sel':      '#ffc1a8', // CSS selector
  'tk-atrule':   '#c18cf0', // reserved — CSS @-rule
  'tk-dhdr':     '#c18cf0', // diff file header
  'tk-dhunk':    '#72cbf5', // diff hunk header
  'tk-dadd':     '#6be58b', // diff addition
  'tk-ddel':     '#e56b7e', // diff deletion
} as const

export type CodeSurfaceToken = keyof typeof CODE_SURFACE_TOKENS
export type CodeSyntaxToken = keyof typeof CODE_SYNTAX_TOKENS
```

Naming note: the export is `CODE_SYNTAX_TOKENS` (paired with
`CODE_SURFACE_TOKENS` under a shared `CODE_*` prefix) rather than the
earlier draft's `SYNTAX_TOKENS` — the two families are siblings and
read more clearly together in imports.

#### 6. Parity tests in `global.test.ts`

**File**: `skills/visualisation/visualise/frontend/src/styles/global.test.ts`
**Changes**: Three additions, written first so they fail before steps
4-5 land.

(a) Extend the `describe.each` table at lines 85-97 to include the
new families:

```ts
describe.each([
  ['typography', TYPOGRAPHY_TOKENS],
  ['spacing', SPACING_TOKENS],
  ['radius', RADIUS_TOKENS],
  ['light shadow', LIGHT_SHADOW_TOKENS],
  ['layout', LAYOUT_TOKENS],
  ['code surface', CODE_SURFACE_TOKENS],
  ['syntax', CODE_SYNTAX_TOKENS],
])('tokens.ts ↔ global.css :root parity (%s)', (_label, tokens) => { ... })
```

(With the matching `import` additions at the top of the file.)

(b) Add a three-way parity describe block that asserts
`prototype-tokens.json` ↔ `tokens.ts` ↔ `global.css`. Imports the
JSON fixture (via Vite's JSON support: `import prototypeTokens from
'./fixtures/prototype-tokens.json'`), strips the leading `--`
from each key, canonicalises whitespace (strip all `\s+`) and case,
and asserts the value matches
`CODE_SURFACE_TOKENS[name] ?? CODE_SYNTAX_TOKENS[name]`. One `it()` per
fixture entry so failures surface per-token.

```ts
// Normalise BOTH sides identically — strip all whitespace so
// `rgba(255,255,255,0.07)` (prototype source) and
// `rgba(255, 255, 255, 0.07)` (tokens.ts) compare equal. Lowercase
// on both sides to absorb prototype's uppercase hex.
const canonical = (v: string) => v.toLowerCase().replace(/\s+/g, '')

describe('prototype fixture ↔ tokens.ts parity (theme-invariant families)', () => {
  for (const [rawName, rawValue] of Object.entries(prototypeTokens)) {
    const name = rawName.replace(/^--/, '')
    const expectedValue = canonical(rawValue)
    it(`--${name} matches the combined token map`, () => {
      const actual =
        (CODE_SURFACE_TOKENS as Record<string, string>)[name] ??
        (CODE_SYNTAX_TOKENS as Record<string, string>)[name]
      expect(actual).toBeDefined()
      expect(canonical(actual!)).toBe(expectedValue)
    })
  }
})
```

(c) Extend the truncation-guard describe at lines 197-204 with one
additional `it()` asserting a known-last `--tk-*` token reads
non-null:

```ts
describe('readCssVar truncation guard', () => {
  it(':root block extends past --ac-topbar-h', () => { ... })  // existing
  it(':root block extends past --tk-ddel', () => {
    expect(readCssVar('tk-ddel', 'root')).not.toBeNull()
  })
  it('[data-theme="dark"] block extends past --ac-shadow-lift', () => { ... })  // existing
})
```

#### 7. Extend `migration.test.ts` declared-token Set

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**: The `var(--NAME) references resolve to declared tokens`
test at lines 342-372 builds a `declared` Set from the existing
token families. Phase 2 will introduce `var(--tk-*)` references in
`code-syntax.global.css` (caught by the `*.global.css` glob) and
Phase 3 will introduce `var(--code-*)` references in
`MarkdownRenderer.module.css`. Both fail this test unless the new
families are added to `declared` here in Phase 1.

Spread `CODE_SURFACE_TOKENS` and `CODE_SYNTAX_TOKENS` keys into the
`declared` Set construction:

```ts
const declared = new Set<string>([
  ...Object.keys(LIGHT_COLOR_TOKENS),
  ...Object.keys(DARK_COLOR_TOKENS),
  ...Object.keys(TYPOGRAPHY_TOKENS),
  ...Object.keys(SPACING_TOKENS),
  ...Object.keys(RADIUS_TOKENS),
  ...Object.keys(LIGHT_SHADOW_TOKENS),
  ...Object.keys(DARK_SHADOW_TOKENS),
  ...Object.keys(LAYOUT_TOKENS),
  ...Object.keys(CODE_SURFACE_TOKENS),   // NEW
  ...Object.keys(CODE_SYNTAX_TOKENS),    // NEW
])
```

(Exact spread shape mirrors the existing pattern at the cited
lines — adjust to match the actual variable name there.)

### Success Criteria

#### Automated Verification:

- [x] All `:root` parity tests pass (32 new `it()` cases — 5 surface +
  27 syntax): `npm --prefix skills/visualisation/visualise/frontend run test -- global.test.ts`
- [x] Three-way fixture parity test passes for all 32 tokens
  (including `--code-stroke`, which the canonicalised normaliser
  treats as equal across spaced/unspaced rgba forms).
- [x] Drift-detection test passes: `npm --prefix skills/visualisation/visualise/frontend run test -- prototype-tokens.fixture.test.ts`
- [x] `hexToRgbString` AND `formatRgba` tests pass:
  `npm --prefix skills/visualisation/visualise/frontend run test -- contrast.test.ts`
- [x] Truncation guard for `--tk-ddel` passes.
- [x] `migration.test.ts` declared-token Set includes
  `CODE_SURFACE_TOKENS` and `CODE_SYNTAX_TOKENS` keys; full
  `migration.test.ts` still passes:
  `npm --prefix skills/visualisation/visualise/frontend run test -- migration.test.ts`
- [x] Type-check passes: `npm --prefix skills/visualisation/visualise/frontend run typecheck`
- [~] Lint: no `lint` script defined in `frontend/package.json` (only `typecheck`); skipped.

#### Manual Verification:

- [ ] `global.css` shows the new tokens after the Layout block and
  before `color-scheme`; visual inspection confirms flat declarations
  (no nested rules) so `readCssVar` continues to work.
- [ ] Booting the dev server shows no visual regression — tokens are
  declared but no consumer references them yet.

---

## Phase 2: Shared hljs layer + MarkdownRenderer wiring

### Overview

Create `src/styles/code-syntax.global.css` with the required
`.hljs-*` → `var(--tk-*)` mappings plus a base `.hljs` reset, wire it
into `main.tsx`, add a dev-only `/code-syntax-showcase` route that
renders fenced code in each in-scope language, and add the Playwright
spec that proves the live cascade in both themes. github.css is NOT
yet removed in this phase — it stays imported alongside the new layer,
which wins on load order. Phase 3 removes it after confirming
coverage.

### Changes Required

#### 0. Shared CSS-rule assertion helper (written first)

**File**: `skills/visualisation/visualise/frontend/src/styles/testing/cssRules.ts`
**Changes**: New file. Extracts the CSS-rule assertion pattern out of
the per-test regex so the same logic is reused by `code-syntax.test.ts`
(Phase 2) and `LibraryTemplatesView.test.tsx` (Phase 4). The tighter
implementation parses the selector list properly so substring
matches (`.hljs-attr` does not match inside `.hljs-attribute`) and
compound-selector confusion (`.hljs-meta` does not match
`.hljs-meta.doctype`) are prevented.

```ts
// Shared CSS-rule structural assertion utilities for design-system
// Vitest specs. Used by `src/styles/code-syntax.test.ts` (Phase 2)
// and `src/routes/library/LibraryTemplatesView.test.tsx` (Phase 4)
// to verify the shared code-syntax layer maps each hljs class to
// the right `--tk-*` token without regex-in-test fragility.
//
// EXACT-MATCH INVARIANT: `assertSelectorColorIs` rejects compound
// suffixes (`.hljs-meta` does not match a `.hljs-meta.doctype`
// rule) and substring siblings (`.hljs-attr` does not match a
// `.hljs-attribute` rule). This is the contract both consumer
// test files rely on — DO NOT weaken to a regex shortcut.
//
// SCOPE: flat global CSS layers (single-class selectors, optional
// compound and descendant selectors, NO nested at-rules). The
// parser throws if the input contains `@media`/`@supports`/etc.
// at top level — the helper is not equipped to recurse into
// nested rule blocks and would silently miss assertions if it
// tried.
//
// Adding a third consumer? Review both existing call sites first
// and consider whether your needs match the flat-layer scope. See
// ADR-0026 §5 for the design-token testing context.

interface CssRule { selectors: string[]; body: string; offset: number }

// Parses a flat CSS source into `{ selectors, body, offset }`
// records. Throws if the input contains nested at-rules — the
// parser does not recurse, so silently parsing them would lose
// assertions. Comments and string literals containing `{` or `}`
// are not handled; the current consumers (`code-syntax.global.css`
// and the templates-preview module after migration) are flat and
// contain neither, but a future consumer producing such content
// must extend the parser before using this helper.
export function parseFlatCssRules(css: string): CssRule[] {
  if (/^\s*@(?:media|supports|container|layer)\b/m.test(css)) {
    throw new Error(
      'parseFlatCssRules: input contains a top-level @-rule (e.g. @media). ' +
      'This helper does not recurse into at-rules; assertions against nested ' +
      'rules would be silently missed. Extend the parser before using.',
    )
  }
  const rules: CssRule[] = []
  let i = 0
  while (i < css.length) {
    const openBrace = css.indexOf('{', i)
    if (openBrace < 0) break
    const closeBrace = css.indexOf('}', openBrace + 1)
    if (closeBrace < 0) break
    const selectorBlock = css.slice(i, openBrace).replace(/\/\*[\s\S]*?\*\//g, '').trim()
    const body = css.slice(openBrace + 1, closeBrace)
    if (selectorBlock && !selectorBlock.startsWith('@')) {
      const selectors = selectorBlock.split(',').map((s) => s.trim())
      rules.push({ selectors, body, offset: openBrace })
    }
    i = closeBrace + 1
  }
  return rules
}

// Asserts that some rule in `css` whose comma-separated selector list
// contains EXACTLY `selector` (no compound suffix, no substring of a
// sibling) declares `color: var(--<token>)`. The `color:` match is
// anchored at a property-name boundary so `border-color:`,
// `background-color:`, `outline-color:`, etc. do NOT satisfy it.
// Throws with all matched rule bodies on failure for actionable
// diagnostics.
export function assertSelectorColorIs(css: string, selector: string, token: string): void {
  const rules = parseFlatCssRules(css)
  const matchingRules = rules.filter((r) => r.selectors.includes(selector))
  if (matchingRules.length === 0) {
    throw new Error(`assertSelectorColorIs: no rule declares selector "${selector}" exactly. ` +
      `Compound selectors like "${selector}.foo" do NOT satisfy this check.`)
  }
  const tokenRef = `var(--${token})`
  const escapedRef = tokenRef.replace(/[()]/g, '\\$&')
  // Anchor on a property-name boundary: start-of-body, `;`, or `{`
  // so the substring `color:` inside `border-color:` does not match.
  const colourRegex = new RegExp(`(?:^|[;{\\s])color\\s*:\\s*${escapedRef}`)
  const ok = matchingRules.some((r) => colourRegex.test(r.body))
  if (!ok) {
    throw new Error(`assertSelectorColorIs: selector "${selector}" found but no matching rule declares color: ${tokenRef}. ` +
      `Inspected bodies: ${JSON.stringify(matchingRules.map((r) => r.body))}`)
  }
}

// Returns the source-offset of the FIRST rule whose selector list
// includes `selector`. Used by source-order assertions in
// `code-syntax.test.ts`.
export function selectorOffset(css: string, selector: string): number | null {
  const rules = parseFlatCssRules(css)
  const match = rules.find((r) => r.selectors.includes(selector))
  return match?.offset ?? null
}
```

Co-located unit tests at `src/styles/testing/cssRules.test.ts` cover:
exact-match success; compound selector rejection (`.hljs-meta` does
NOT match `.hljs-meta.doctype` rule); substring rejection
(`.hljs-attr` does NOT match `.hljs-attribute` rule); multi-selector
grouping success (`.hljs-comment, .hljs-quote { color: var(--tk-com) }`
matches both `.hljs-comment` and `.hljs-quote`); property-boundary
rejection (`.x { border-color: var(--tk-com) }` does NOT satisfy
`assertSelectorColorIs(css, '.x', 'tk-com')`); `@media`-guard throws
with a diagnostic message; explicit failure diagnostic includes the
matched bodies; `selectorOffset` returns ascending values for
source-order arguments.

#### 1. Static structural Vitest test (written first)

**File**: `skills/visualisation/visualise/frontend/src/styles/code-syntax.test.ts`
**Changes**: New file. Imports
`./code-syntax.global.css?raw` and the shared helper from §0. Asserts
the layer declares every required mapping via `assertSelectorColorIs`,
and asserts source-order for diff-context overrides via
`selectorOffset`. One `it()` per mapping row.

```ts
import { describe, it, expect } from 'vitest'
import css from './code-syntax.global.css?raw'
import { assertSelectorColorIs, selectorOffset } from './testing/cssRules'

const REQUIRED_MAPPINGS: ReadonlyArray<[selector: string, token: string]> = [
  ['.hljs-comment',                       'tk-com'],
  ['.hljs-quote',                         'tk-com'],
  ['.hljs-code',                          'tk-com'],
  ['.hljs-bullet',                        'tk-com'],
  ['.hljs-string',                        'tk-str'],
  ['.hljs-number',                        'tk-num'],
  ['.hljs-keyword',                       'tk-kw'],
  ['.hljs-literal',                       'tk-lit'],
  ['.hljs-type',                          'tk-typ'],
  ['.hljs-class',                         'tk-typ'],
  ['.hljs-function',                      'tk-fn'],
  ['.hljs-title.function_',               'tk-fn'],
  ['.hljs-attr',                          'tk-attr'],
  ['.hljs-attribute',                     'tk-attr'],
  ['.hljs-meta',                          'tk-deco'],
  ['.hljs-built_in',                      'tk-bn'],
  ['.hljs-variable',                      'tk-var'],
  ['.hljs-template-variable',             'tk-var'],
  ['.hljs-property',                      'tk-prop'],
  ['.hljs-selector-class',                'tk-sel'],
  ['.hljs-selector-id',                   'tk-sel'],
  ['.hljs-selector-tag',                  'tk-sel'],
  ['.hljs-selector-pseudo',               'tk-sel'],
  ['.hljs-tag',                           'tk-tag'],
  ['.hljs-name',                          'tk-tag'],
  // `.hljs-meta.doctype` was previously listed here but standard
  // highlight.js does not emit the compound class — doctype lines
  // are wrapped in `.hljs-meta` only. The doctype is therefore
  // coloured via the general `.hljs-meta` → `--tk-deco` rule. If a
  // future hljs version starts emitting the compound class, add
  // the row back and a Playwright assertion for it.
  ['.hljs-section',                       'tk-header'],
  ['.hljs-link',                          'tk-anchor'],
  ['.hljs-symbol',                        'tk-anchor'],
  ['.hljs-punctuation',                   'tk-pun'],
  ['.hljs-addition',                      'tk-dadd'],
  ['.hljs-diff-added',                    'tk-dadd'],
  ['.hljs-deletion',                      'tk-ddel'],
  ['.hljs-diff-deleted',                  'tk-ddel'],
  ['.language-diff .hljs-meta',           'tk-dhdr'],
  ['.language-diff .hljs-comment',        'tk-dhunk'],
]

describe('code-syntax.global.css', () => {
  it('declares a .hljs base rule resetting background to transparent', () => {
    expect(css).toMatch(/\.hljs\s*\{[^}]*background:\s*transparent/)
    expect(css).toMatch(/\.hljs\s*\{[^}]*color:\s*inherit/)
  })

  for (const [selector, token] of REQUIRED_MAPPINGS) {
    it(`${selector} → var(--${token})`, () => {
      // Uses the shared exact-match helper — compound selectors
      // (`.hljs-meta.doctype`) and substring siblings
      // (`.hljs-attribute`) are rejected, so the assertion cannot
      // pass spuriously when a sibling rule has the right colour.
      assertSelectorColorIs(css, selector, token)
    })
  }

  it('.hljs-section declares font-weight: 600 (preserved from templates preview)', () => {
    expect(css).toMatch(/\.hljs-section\s*\{[^}]*font-weight:\s*600/)
  })

  it('.hljs-emphasis declares font-style: italic (no colour override — inherits surrounding fg)', () => {
    expect(css).toMatch(/\.hljs-emphasis\s*\{[^}]*font-style:\s*italic/)
    // Explicit: .hljs-emphasis must NOT declare a color — the
    // prototype intent is italic-only and the templates-preview
    // colour-muting is deliberately dropped (see ADR-0026 §5).
    const block = /\.hljs-emphasis\s*\{([^}]*)\}/.exec(css)?.[1] ?? ''
    expect(block).not.toMatch(/color\s*:/)
  })

  it('.hljs-strong declares font-weight: 600 (no colour override)', () => {
    expect(css).toMatch(/\.hljs-strong\s*\{[^}]*font-weight:\s*600/)
    const block = /\.hljs-strong\s*\{([^}]*)\}/.exec(css)?.[1] ?? ''
    expect(block).not.toMatch(/color\s*:/)
  })

  it('.language-diff .hljs-meta rule appears AFTER the general .hljs-meta rule in source', () => {
    // Source-order insurance: even though specificity already
    // favours the descendant selector, a future refactor that flips
    // specificity (e.g. moving to [data-language="diff"]) would
    // depend on source order. This guard fails loud if the
    // ordering invariant is broken. Both offsets use the same
    // measurement point (the rule's open-brace position via
    // selectorOffset) for symmetric comparison.
    const general = selectorOffset(css, '.hljs-meta')
    const override = selectorOffset(css, '.language-diff .hljs-meta')
    expect(general).not.toBeNull()
    expect(override).not.toBeNull()
    expect(override!).toBeGreaterThan(general!)
  })

  it('.language-diff .hljs-comment rule appears AFTER the general .hljs-comment rule in source', () => {
    const general = selectorOffset(css, '.hljs-comment')
    const override = selectorOffset(css, '.language-diff .hljs-comment')
    expect(general).not.toBeNull()
    expect(override).not.toBeNull()
    expect(override!).toBeGreaterThan(general!)
  })
})
```

This is the structural ground truth for the mapping; it runs in
Vitest and is fast. The Playwright spec below is the live-cascade
verification.

#### 2. Shared hljs layer

**File**: `skills/visualisation/visualise/frontend/src/styles/code-syntax.global.css`
**Changes**: New file. Defines:

- Base `.hljs` reset (transparent background, inherit color) so the
  cascade lets `<pre>`'s `--code-bg`/`--code-fg` win in phase 3.
- Multi-selector rules grouping classes that share a token (e.g.
  `.hljs-comment, .hljs-quote, .hljs-code, .hljs-bullet`).
- `.language-diff` descendant rules for `--tk-dhdr` and `--tk-dhunk`
  declared AFTER the general `.hljs-meta` / `.hljs-comment` rules so
  the cascade resolves diff context correctly.
- `font-style: italic` for `.hljs-emphasis`, `font-weight: 600` for
  `.hljs-strong` (no color override — inherit).

Outline:

```css
/* Code-syntax layer — single source of truth for hljs class
   colouring across the visualiser.
 
   WHY GLOBAL (not a CSS module): rehype-highlight emits literal
   `hljs-*` strings at AST-build time. CSS-module hashing would
   break the selector match because the rendered DOM never carries
   the hashed class. This file is one of a narrow set of deliberate
   globals (see also wiki-links.global.css) and should NOT be moved
   into a per-component .module.css.
 
   WHY ONE LAYER, NOT TWO: both consumers (MarkdownRenderer and
   LibraryTemplatesView templates preview) emit the same `hljs-*`
   class names via rehype-highlight / template-highlight. Diverging
   their colours would produce inconsistent reader experience for
   the same content type.
 
   SPECIFICITY CONTRACT: rules below are flat single-class
   specificity (0,1,0) plus two descendant overrides for
   `.language-diff`. Any future consumer adding a more-specific
   rule (e.g. `.previewBody :global(.hljs-*)`, specificity 0,2,0)
   will silently win over this layer and break the
   single-source-of-truth contract — don't.
 
   Imported from src/main.tsx after global.css. See ADR-0026 §5 and
   story meta/work/0076-code-block-syntax-highlight-palette.md. */

.hljs {
  background: transparent;
  color: inherit;
}

.hljs-comment,
.hljs-quote,
.hljs-code,
.hljs-bullet {
  color: var(--tk-com);
}
.hljs-string         { color: var(--tk-str); }
.hljs-number         { color: var(--tk-num); }
.hljs-keyword        { color: var(--tk-kw); }
.hljs-literal        { color: var(--tk-lit); }
.hljs-type,
.hljs-class          { color: var(--tk-typ); }
.hljs-function,
.hljs-title.function_ { color: var(--tk-fn); }
.hljs-attr,
.hljs-attribute      { color: var(--tk-attr); }
.hljs-meta           { color: var(--tk-deco); }
.hljs-built_in       { color: var(--tk-bn); }
.hljs-variable,
.hljs-template-variable { color: var(--tk-var); }
.hljs-property       { color: var(--tk-prop); }
.hljs-selector-class,
.hljs-selector-id,
.hljs-selector-tag,
.hljs-selector-pseudo { color: var(--tk-sel); }
.hljs-tag,
.hljs-name           { color: var(--tk-tag); }
.hljs-section        { color: var(--tk-header); font-weight: 600; }
.hljs-link,
.hljs-symbol         { color: var(--tk-anchor); }
.hljs-punctuation    { color: var(--tk-pun); }
.hljs-addition,
.hljs-diff-added     { color: var(--tk-dadd); }
.hljs-deletion,
.hljs-diff-deleted   { color: var(--tk-ddel); }

/* Diff-context overrides — higher specificity (.language-diff
   .hljs-meta = 0,2,0) overrides the bare .hljs-meta rule (0,1,0).
   Source order is also `after` for defence in depth; the
   code-syntax.test.ts source-order assertions guard the ordering
   in case a future refactor flattens specificity. */
.language-diff .hljs-meta    { color: var(--tk-dhdr); }
.language-diff .hljs-comment { color: var(--tk-dhunk); }

/* Inline emphasis: italic-only, colour inherited from surrounding
   prose. The templates-preview module previously coloured these to
   var(--ac-fg-muted) / var(--ac-fg-strong); that pairing is
   deliberately dropped in 0076 — see ADR-0026 §5. */
.hljs-emphasis { font-style: italic; }
.hljs-strong   { font-weight: 600; }
```

#### 3. Wire into `main.tsx`

**File**: `skills/visualisation/visualise/frontend/src/main.tsx`
**Changes**: Add an import line after the `global.css` import (so the
new layer's selectors are at the same specificity as github.css but
declared later → wins the cascade tie):

```ts
import './styles/global.css'
import './styles/code-syntax.global.css'
import './styles/wiki-links.global.css'
```

(The `highlight.js/styles/github.css` import at line 7 is RETAINED in
phase 2. Phase 3 removes it.)

#### 4. Dev-only showcase route + component (new files)

**File**: `skills/visualisation/visualise/frontend/src/routes/code-syntax-showcase/CodeSyntaxShowcase.tsx`
**Changes**: New file. Mirrors `GlyphShowcase.tsx` shape. The
FIXTURES array is the single source of truth for the Playwright
spec — each entry is annotated with the hljs spans the spec
relies on so a future contributor cleaning up fixtures sees the
contract inline.

```tsx
// Dev-only Playwright fixture surface for resolved-colour
// assertions in tests/visual-regression/code-block-resolved-colours.spec.ts.
// NOT part of the design-system index — story 0083 owns the
// documented showcase. Do not link from DevDesignSystem.
// See meta/work/0076-code-block-syntax-highlight-palette.md for
// provenance.
import { MarkdownRenderer } from '../../components/MarkdownRenderer/MarkdownRenderer'

// Each fixture's `spans` field names the hljs classes the
// Playwright spec asserts on. Do not remove a class' triggering
// text without updating the spec.
export interface CodeSyntaxFixture {
  lang: string
  code: string
  spans: ReadonlyArray<string>
}

export const FIXTURES: ReadonlyArray<CodeSyntaxFixture> = [
  {
    lang: 'python',
    code: 'def foo(x: int) -> int:\n    print("hi")\n    return x + 42  # comment\n',
    spans: ['hljs-keyword', 'hljs-string', 'hljs-number', 'hljs-comment',
            'hljs-built_in', 'hljs-function', 'hljs-title.function_'],
  },
  {
    lang: 'typescript',
    code: 'const greet = (name: string): string => `Hi ${name}`;\nconst obj = { prop: 1 };\nobj.prop = 2;\n',
    spans: ['hljs-keyword', 'hljs-type', 'hljs-variable', 'hljs-template-variable',
            'hljs-property', 'hljs-punctuation'],
  },
  {
    lang: 'yaml',
    code: 'title: "Example"\ncount: 7\nactive: true\n',
    spans: ['hljs-attr', 'hljs-string', 'hljs-number', 'hljs-literal'],
  },
  {
    lang: 'json',
    code: '{\n  "key": "value",\n  "n": 42\n}\n',
    spans: ['hljs-attr', 'hljs-string', 'hljs-number'],
  },
  {
    lang: 'css',
    code: '.cls { color: red; }\n#id { background: blue; }\na:hover { opacity: 0.5; }\n',
    spans: ['hljs-selector-class', 'hljs-selector-id', 'hljs-selector-pseudo'],
  },
  {
    lang: 'html',
    code: '<!DOCTYPE html>\n<div class="x" data-foo="y">hi</div>\n',
    spans: ['hljs-tag', 'hljs-name', 'hljs-attr', 'hljs-meta'],
  },
  {
    lang: 'diff',
    code: 'diff --git a/x b/x\n@@ -1,1 +1,1 @@\n-old\n+new\n',
    spans: ['hljs-meta', 'hljs-comment', 'hljs-addition', 'hljs-deletion'],
  },
  {
    lang: 'markdown',
    code: '# Heading\n\n- item\n[link](http://x)\n',
    spans: ['hljs-section', 'hljs-bullet', 'hljs-link', 'hljs-symbol'],
  },
]

export function CodeSyntaxShowcase() {
  return (
    <main data-testid="code-syntax-showcase">
      {FIXTURES.map(({ lang, code }) => (
        <section key={lang} data-testid={`code-syntax-cell-${lang}`}>
          <h2>{lang}</h2>
          <MarkdownRenderer content={'```' + lang + '\n' + code + '```\n'} />
        </section>
      ))}
    </main>
  )
}
```

**File**: `skills/visualisation/visualise/frontend/src/router.ts`
**Changes**: Add an import and register the route using the same
`createRoute({ getParentRoute: () => rootRoute, ... })` pattern as
the existing showcase routes, AND add the new route variable to the
`rootRoute.addChildren([...])` call. Without the `addChildren`
update the route is not reachable and `/code-syntax-showcase`
returns 404 — silently breaking the Playwright spec.

```ts
import { CodeSyntaxShowcase } from './routes/code-syntax-showcase/CodeSyntaxShowcase'

const codeSyntaxShowcaseRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/code-syntax-showcase',
  component: CodeSyntaxShowcase,
})

// In the rootRoute.addChildren([...]) call (around line 157),
// add `codeSyntaxShowcaseRoute` alongside the other showcase
// routes (`glyphShowcaseRoute`, `chipShowcaseRoute`).
```

(Follow the exact shape of `glyphShowcaseRoute` / `chipShowcaseRoute`
at the cited lines — both the `createRoute(...)` declaration AND the
`addChildren([...])` entry are required.)

#### 5. Playwright spec (written first; fails until step 2 lands)

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/code-block-resolved-colours.spec.ts`
**Changes**: New Playwright spec. Mirrors `glyph-resolved-fill.spec.ts`
shape. Source-of-truth tokens come from `CODE_SYNTAX_TOKENS`
(imported from `src/styles/tokens`).

**Pre-implementation emission probe (do this BEFORE writing the
spec)**: rehype-highlight class emissions are grammar-version
dependent and not all rows below are guaranteed without verifying
against the live renderer. Run a quick Vitest probe that renders
each FIXTURES entry through `MarkdownRenderer` and records the set
of `hljs-*` classes that actually appear in the DOM. Pin each row
in the assertion table to a fixture line that is empirically
confirmed to emit the target class. If a row's class is not
emitted, drop the row or adjust the fixture text (e.g. add a
`print(x)` line for `.hljs-built_in`, a `${name}` template-literal
interpolation for `.hljs-template-variable`, etc.).

(a) **AC2 — required-mapping table (excluding diff)**: navigates to
`/code-syntax-showcase`, locates a span of the right class inside the
right language's cell, asserts the resolved colour matches the
token.

| Selector tested            | Source cell      | Token to expect |
|----------------------------|------------------|-----------------|
| `.hljs-keyword`            | python OR typescript | `tk-kw`     |
| `.hljs-string`             | python (`"..."`) | `tk-str`        |
| `.hljs-number`             | python `42`      | `tk-num`        |
| `.hljs-literal`            | yaml `true`      | `tk-lit`        |
| `.hljs-type` / `.hljs-class` | typescript `string` | `tk-typ` |
| `.hljs-function` AND `.hljs-title.function_` | python `def foo` / ts arrow | `tk-fn` |
| `.hljs-attr` / `.hljs-attribute` | yaml key       | `tk-attr`       |
| `.hljs-meta` (general)     | html `<!DOCTYPE html>` line in the html cell | `tk-deco` |
| `.hljs-built_in`           | python `print(x)` line | `tk-bn` |
| `.hljs-variable` / `.hljs-template-variable` | a `${...}` interpolation in TS template literal | `tk-var` |
| `.hljs-property`           | TS `obj.prop` access | `tk-prop` |
| `.hljs-selector-*`         | css cell         | `tk-sel`        |
| `.hljs-tag` / `.hljs-name` | html cell        | `tk-tag`        |
| `.hljs-section`            | markdown `# Heading` | `tk-header` |
| `.hljs-link` / `.hljs-symbol` | markdown link  | `tk-anchor`     |
| `.hljs-punctuation`        | TS cell (rehype-highlight emits it for the const declaration's `,`/`;`) | `tk-pun` |
| `.hljs-comment` (incl. `.hljs-quote`, `.hljs-code`, `.hljs-bullet`) | python comment + markdown bullet | `tk-com` |

(`.hljs-meta.doctype` is removed — standard highlight.js does not
emit the compound class; the general `.hljs-meta` rule covers
doctype lines via the same `tk-deco` token.)

(b) **AC3 — diff rows + general-rule pair-check**: navigates to the
diff cell, asserts the four spans compute to `tk-dhdr`, `tk-dhunk`,
`tk-dadd`, `tk-ddel`. Then a *paired* assertion: a `.hljs-meta`
span in the *non*-diff html cell still resolves to `tk-deco` (not
`tk-dhdr`). This catches a class of bugs where the diff override
accidentally fires globally.

Per-test shape (parameterised over themes to enforce
theme-invariance without copy-paste — divergence between the two
theme bodies would silently defeat the point of the contract):

```ts
import { test, expect } from '@playwright/test'
import { CODE_SYNTAX_TOKENS } from '../../src/styles/tokens'
import { hexToRgbString, formatRgba } from '../../src/styles/contrast'

const THEMES = [
  { name: 'light', setup: async () => {} },
  {
    name: 'dark',
    setup: async (page: import('@playwright/test').Page) => {
      await page.evaluate(() => {
        document.documentElement.dataset.theme = 'dark'
      })
      await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark')
    },
  },
] as const

for (const theme of THEMES) {
  test.describe(`AC2 — resolved colours (${theme.name} theme)`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto('/code-syntax-showcase')
      await theme.setup(page)
    })

    test('.hljs-keyword in python cell resolves to --tk-kw', async ({ page }) => {
      const locator = page.locator('[data-testid="code-syntax-cell-python"] .hljs-keyword').first()
      // Precondition: fail loud if the class isn't emitted at all,
      // rather than silently passing "colour is undefined".
      await expect(locator).toBeVisible()
      const colour = await locator.evaluate((el) => getComputedStyle(el).color)
      expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS['tk-kw']))
    })
    // … one test per row in the table above, same shape.
  })

  test.describe(`AC3 — diff overrides (${theme.name} theme)`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto('/code-syntax-showcase')
      await theme.setup(page)
    })

    test('.hljs-meta inside .language-diff resolves to --tk-dhdr', async ({ page }) => {
      const locator = page.locator('[data-testid="code-syntax-cell-diff"] .hljs-meta').first()
      await expect(locator).toBeVisible()
      const colour = await locator.evaluate((el) => getComputedStyle(el).color)
      expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS['tk-dhdr']))
    })

    test('.hljs-meta OUTSIDE .language-diff still resolves to --tk-deco', async ({ page }) => {
      // Paired with the above: proves the override is scoped to
      // .language-diff and hasn't accidentally become global.
      const locator = page.locator('[data-testid="code-syntax-cell-html"] .hljs-meta').first()
      await expect(locator).toBeVisible()
      const colour = await locator.evaluate((el) => getComputedStyle(el).color)
      expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS['tk-deco']))
    })
    // … three more diff-row tests, same shape.
  })

  test.describe(`<pre> chrome (${theme.name} theme)`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto('/code-syntax-showcase')
      await theme.setup(page)
    })

    test('<pre> border-color resolves to --code-stroke (rgba canonical)', async ({ page }) => {
      // Anchors the only rgba-valued surface token via the
      // formatRgba canonical formatter — would otherwise drift
      // between Chromium's `rgba(R, G, B, A)` spacing and inline
      // format strings per spec.
      const locator = page.locator('[data-testid="code-syntax-cell-python"] pre').first()
      await expect(locator).toBeVisible()
      const border = await locator.evaluate((el) => getComputedStyle(el).borderTopColor)
      // --code-stroke = rgba(255, 255, 255, 0.07)
      expect(border).toBe(formatRgba(255, 255, 255, 0.07))
    })
  })
}
```

(The `<pre>` chrome assertion only runs after Phase 3 ships — the
border declaration lives in `MarkdownRenderer.module.css`. Until
then this test is skipped or excluded from the Phase 2 CI run. The
formatter is introduced in Phase 1 so the assertion is ready to
land alongside the chrome migration in Phase 3.)

Theme-parameterisation via the `THEMES` loop ensures the two bodies
stay literally identical; the only thing that differs is the
`beforeEach` setup. Token-bump-time drift between light/dark assertions
becomes impossible by construction — which is the right shape for a
theme-invariance contract.

#### 6. Sanity import in CodeSyntaxShowcase test (Vitest)

**File**: `skills/visualisation/visualise/frontend/src/routes/code-syntax-showcase/CodeSyntaxShowcase.test.tsx`
**Changes**: New file. Per-language smoke tests via `it.each(FIXTURES)`
so a missing language cell fails fast with a precise diagnostic
(before a Playwright timeout). Does NOT assert resolved colours
(jsdom limitation) — Playwright owns that.

```ts
import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { CodeSyntaxShowcase, FIXTURES } from './CodeSyntaxShowcase'

describe('CodeSyntaxShowcase', () => {
  it('renders the top-level showcase container', () => {
    const { container } = render(<CodeSyntaxShowcase />)
    expect(container.querySelector('[data-testid="code-syntax-showcase"]')).not.toBeNull()
  })

  it.each(FIXTURES.map(({ lang }) => ({ lang })))(
    'renders a cell with a fenced code block for $lang',
    ({ lang }) => {
      const { container } = render(<CodeSyntaxShowcase />)
      const cell = container.querySelector(`[data-testid="code-syntax-cell-${lang}"]`)
      expect(cell).not.toBeNull()
      const pre = cell!.querySelector('pre code')
      expect(pre).not.toBeNull()
      expect(pre!.className).toMatch(new RegExp(`\\blanguage-${lang}\\b`))
      expect(pre!.className).toMatch(/\bhljs\b/)
    },
  )
})
```

(Note: this showcase intentionally departs from the four-file
`*.module.css` pattern of `GlyphShowcase`/`ChipShowcase`. The route
exists only as a Playwright fixture surface — visual ergonomics for
human review are not a goal; story 0083 owns the documented
showcase. Browser-default layout is acceptable. The file-header
comment in `CodeSyntaxShowcase.tsx` records this rationale.)

### Success Criteria

#### Automated Verification:

- [x] `code-syntax.test.ts` passes for all required-mapping rows
  including diff: `npm --prefix skills/visualisation/visualise/frontend run test -- code-syntax.test.ts`
- [x] `CodeSyntaxShowcase.test.tsx` smoke test passes.
- [x] `MarkdownRenderer.test.tsx` (unchanged in this phase) still
  passes — github.css coexists harmlessly.
- [x] `LibraryTemplatesView.test.tsx` (unchanged in this phase) still
  passes — `.previewBody :global(.hljs-*)` rules still present,
  shared layer's selectors lose to the templates-preview module's
  higher specificity (`.previewBody :global(.hljs-attr)` beats
  `.hljs-attr` on specificity).
- [x] Playwright resolved-colour spec passes in both light and dark
  (50/50): `npm exec playwright test -- code-block-resolved-colours --project visual-regression`
- [x] `migration.test.ts` still passes — `code-syntax.global.css`
  contains `var(--tk-*)` references that all resolve to declared
  tokens (per phase 1's tokens.ts additions).
- [x] Type-check passes. (No `lint` script in `frontend/package.json`.)

#### Manual Verification:

- [ ] Open the dev server and navigate to `/code-syntax-showcase`;
  each language section renders with the prototype palette
  (purple-ish keywords, green strings, yellow numbers).
- [ ] Toggle theme via DevTools (`document.documentElement.dataset.theme = 'dark'`);
  the code-block palette is visually unchanged (theme-invariant).
- [ ] Open `/library/work-items/<any-work-item-with-a-fenced-block>`
  (e.g. this plan); confirm code blocks render with the new colours.
- [ ] Inspect `<pre>` element in DevTools: still has the pre-existing
  `#1e1e1e` background (phase 3 migrates it). Confirms phase scope.

---

## Phase 3: MarkdownRenderer chrome migration + cleanup (depends on Phase 2)

### Overview

Replace the `#1e1e1e`/`#d4d4d4` literals in
`MarkdownRenderer.module.css` with `var(--code-bg)`/`var(--code-fg)`,
add a subtle `1px solid var(--code-stroke)` border for parity with the
prototype's chrome (optional but matches prototype intent), amend
`ADR-0026` §3 to remove the irreducible classification for those two
literals, remove the matching EXCEPTIONS entries from
`migration.test.ts`, and remove the now-redundant
`highlight.js/styles/github.css` import from `main.tsx`. Extend
`MarkdownRenderer.test.tsx` with the six AC4 behaviours.

### Changes Required

#### 1. AC4 behaviour tests (written first)

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.test.tsx`
**Changes**: Six new `it()` cases wrapped in a `describe('Story 0076
AC4 — markdown pipeline behaviours', ...)` so the AC trace is
discoverable in test output. Each asserts a structural property of
the rendered DOM, not a computed colour (Playwright owns that).

```ts
describe('Story 0076 AC4 — markdown pipeline behaviours', () => {

it('renders a GFM task list with interactive checkboxes', () => {
  const { container } = render(<MarkdownRenderer content={'- [x] done\n- [ ] todo\n'} />)
  const checkboxes = container.querySelectorAll('input[type="checkbox"]')
  expect(checkboxes.length).toBe(2)
  expect((checkboxes[0] as HTMLInputElement).checked).toBe(true)
  expect((checkboxes[1] as HTMLInputElement).checked).toBe(false)
})

it('renders a GFM table with thead/tbody/tr/td structure', () => {
  const { container } = render(
    <MarkdownRenderer content={'| H1 | H2 |\n|----|----|\n| a  | b  |\n'} />
  )
  expect(container.querySelector('table thead tr th')?.textContent).toBe('H1')
  expect(container.querySelector('table tbody tr td')?.textContent).toBe('a')
})

it('routes a [[NNNN]] wiki-link in body prose through the resolver', () => {
  const resolver: Resolver = (id) => ({
    kind: 'resolved',
    href: `/library/work-items/${id}`,
    title: `Work item ${id}`,
  })
  const { container } = render(
    <MarkdownRenderer content={'See [[0042]] for context.'} resolveWikiLink={resolver} />
  )
  const anchor = container.querySelector('a')
  expect(anchor?.getAttribute('href')).toBe('/library/work-items/0042')
})

it('emits .hljs-keyword spans for an explicit language-python fenced code block', () => {
  const { container } = render(
    <MarkdownRenderer content={'```python\ndef foo():\n    return 1\n```'} />
  )
  expect(container.querySelectorAll('.hljs-keyword').length).toBeGreaterThanOrEqual(1)
})

it('emits .hljs-keyword spans for an explicit language-typescript fenced code block', () => {
  const { container } = render(
    <MarkdownRenderer content={'```typescript\nconst x: number = 1\n```'} />
  )
  expect(container.querySelectorAll('.hljs-keyword').length).toBeGreaterThanOrEqual(1)
})

it('renders a fenced code block and a [[NNNN]] wiki-link in the same document without regression', () => {
  const resolver: Resolver = (id) => ({
    kind: 'resolved',
    href: `/library/work-items/${id}`,
    title: `Work item ${id}`,
  })
  const { container } = render(
    <MarkdownRenderer
      content={'See [[0042]].\n\n```python\nx = 1\n```\n'}
      resolveWikiLink={resolver}
    />,
  )
  expect(container.querySelector('a[href="/library/work-items/0042"]')).not.toBeNull()
  expect(container.querySelector('pre code')).not.toBeNull()
})

it('does NOT resolve [[NNNN]] inside an inline code span (verbatim pass-through)', () => {
  const resolver: Resolver = () => ({ kind: 'resolved', href: '/x', title: 'x' })
  const { container } = render(
    <MarkdownRenderer content={'inline `[[0042]]` should not resolve'} resolveWikiLink={resolver} />,
  )
  expect(container.querySelector('a[href="/x"]')).toBeNull()
  expect(container.textContent).toContain('[[0042]]')
})

it('renders an unknown-language fence with the base .hljs class (no thrown error)', () => {
  const { container } = render(<MarkdownRenderer content={'```klingon\nbatlh Daqawlu\'taH\n```'} />)
  const code = container.querySelector('pre code')
  expect(code).not.toBeNull()
  expect(code!.className).toMatch(/\bhljs\b/)
})

}) // end describe('Story 0076 AC4 — markdown pipeline behaviours')
```

#### 2. `<pre>` chrome migration

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css`
**Changes**: Update `.markdown pre` (lines 13-17) to consume the new
tokens. Border-radius stays at `6px` (irreducible, retained
EXCEPTION). Add a 1px hairline border using `--code-stroke` for
prototype parity (the prototype puts a stroke around the code chrome).

```css
.markdown pre {
  background: var(--code-bg);
  color: var(--code-fg);
  border: 1px solid var(--code-stroke);
  border-radius: 6px;
  padding: var(--sp-4);
  overflow-x: auto;
  font-size: var(--size-xs);
}
```

(The `1px` border width is already permitted: there is a
`{ literal: '1px', count: 2, kind: 'irreducible' }` EXCEPTION at
`migration.test.ts:61` for this module that covers two existing
hairlines. The new border raises the count from 2 to 3 — update the
EXCEPTION's `count` to 3 in the same commit. Alternative: omit the
border to keep the change scope narrow. **Plan default: include the
border** for prototype fidelity; bump the count.)

#### 3. ADR-0026 amendment

**File**: `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
**Changes**:

- Remove the row `| Editor palette colours | #1e1e1e, #d4d4d4 | Code-block dark colours, no surface token |` from the §3 table at line 135 (now reducible — covered by §5 below).
- Remove the last row of the §Appendix mapping table at line 202: `| #1e1e1e, #d4d4d4 | irreducible | Editor palette — no token  |`.
- Add a new top-level **§5 "Code-block surface and syntax palette"**
  alongside §4 ("Two-blue collapse"). Top-level §5 (rather than a
  §3a subsection) is correct because the new content describes a
  family that is now *reducible* — putting it as a sub-rule of §3
  "Irreducible literal categories" would be semantically misleading.

```markdown
## 5. Code-block surface and syntax palette

### Context

The visualiser renders source code in two pathways: `MarkdownRenderer`
(rehype-highlight emits `hljs-*` spans) and `LibraryTemplatesView`
templates preview (template-highlight emits `hljs-template-variable`
plus the same hljs class names). Prior to story 0076 the two
pathways had different colour sources: `MarkdownRenderer` consumed
`highlight.js/styles/github.css` (whatever that ships), and the
templates preview declared a parallel locally-scoped hljs theme
mapped to generic `--ac-*` tokens. There was no single source of
truth.

### Decision

The visualiser ships a theme-independent code-block palette via two
new token families, declared once under `:root` only:

- `--code-*` (5 tokens): code-block surface chrome
  (background, fg, head, stroke, fg-faint)
- `--tk-*` (27 tokens): syntax-highlight roles (the `tk-` prefix
  preserves the prototype source's naming so the
  `prototype-tokens.fixture.test.ts` drift detector matches
  byte-for-byte — `tk` = "syntax token")

A single shared CSS layer at `src/styles/code-syntax.global.css`
maps each required `.hljs-*` class to one `var(--tk-*)` reference.
`MarkdownRenderer.module.css` consumes the surface tokens for
`<pre>` chrome.

The palette is **theme-invariant**: declared only under `:root`,
absent from both `[data-theme="dark"]` and `@media (prefers-color-scheme: dark)`
blocks. The `global.test.ts` MIRROR-A/B parity machinery correctly
treats `:root`-only families as expected.

### Why theme-invariant

The prototype renders this palette identically against both light
and dark page chrome (visible in the two fullpage screenshots at
`meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/screenshots/doc-detail-plan-meta-visualisation-fullpage.png`
and `…-fullpage-dark.png`). The deep navy `--code-bg` (#0e1320)
provides sufficient contrast against both surfaces; tuning the
palette per theme would diverge from the prototype's visual ground
truth.

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

Future contributors adding a `:root`-only family should add a
declaration to this list and document the source.

### Operational guidance

- **Adding a new hljs class to the shared layer**: prefer reusing an
  existing `--tk-*` token by semantic match before introducing a
  new one. Six tokens (`--tk-macro`, `--tk-key`, `--tk-flag`,
  `--tk-heredoc`, `--tk-lifet`, `--tk-atrule`) ship without active
  consumers and are reserved for future grammar support — consult
  the inline comments in `tokens.ts` and `global.css` before adding.
- **Adding a new `--tk-*` token**: must (a) appear in
  `prototype-standalone.html` (drift fixture will fail otherwise)
  or be deliberately introduced as an in-repo extension with the
  fixture updated in lock-step; (b) be declared in `global.css`
  under `:root` only; (c) be added to `CODE_SYNTAX_TOKENS` in
  `tokens.ts`.
- **Adding a more-specific rule that overrides the shared layer**:
  don't. The whole point of the shared layer is single-source-of-
  truth. If a consumer truly needs a per-context override, propose
  an ADR amendment.

### `src/styles/fixtures/` directory

This story also introduces `src/styles/fixtures/` as the home for
committed snapshots of external authoritative sources (e.g.
`prototype-tokens.json`). The directory scope: committed JSON
captures of an external file, kept adjacent to the consuming test.
Not for general test data or in-repo fixtures that have no external
source.

### `src/styles/testing/` directory

This story also introduces `src/styles/testing/` as the home for
test-only helpers shared across multiple Vitest specs in
`src/styles/` and its descendants (e.g. `cssRules.ts` with
`assertSelectorColorIs`/`parseFlatCssRules`/`selectorOffset`).
Helpers in this directory must (a) be import-only from `*.test.ts`
files (never from production code), (b) be co-located with their
unit tests, and (c) carry a file-header comment naming their known
consumers so a future test author can find them. Single-consumer
helpers stay alongside their test; promote to `testing/` only when
a second consumer arrives.

### Consequences

- Code-block colours that were classified irreducible in §3
  (`#1e1e1e`, `#d4d4d4`) are now reducible to `var(--code-bg)` and
  `var(--code-fg)` respectively. The §3 table rows are removed.
- The `highlight.js/styles/github.css` import is removed from
  `main.tsx` — the shared layer covers every class the visualiser
  emits. The `main.import-hygiene.test.ts` spec guards against
  reintroduction.

### References

- Story: `meta/work/0076-code-block-syntax-highlight-palette.md`
- Shared layer: `src/styles/code-syntax.global.css`
- Drift fixture: `src/styles/fixtures/prototype-tokens.json` +
  `src/styles/prototype-tokens.fixture.test.ts`
- Foundation: story 0033 (design token system)
```

#### 4. `migration.test.ts` EXCEPTIONS + AC5_FLOOR update

**File**: `skills/visualisation/visualise/frontend/src/styles/migration.test.ts`
**Changes**:

- Remove the two color EXCEPTIONS at lines 59-60.
- Bump the `1px` EXCEPTION for `MarkdownRenderer.module.css` (line 61)
  from `count: 2` to `count: 3` to cover the new `<pre>` border.
  Expand the `reason` field to enumerate the three sites so a future
  diff can attribute the count back to a specific declaration: e.g.
  `reason: 'hairline borders/rules: <pre>, table cell, blockquote — below --sp-1 floor'`
  (adjust to match actual existing rule sites).
- All other entries for this file (lines 62-68) are retained — the
  irreducible spacings/sizes around `<pre>` chrome are unchanged.

The `EXCEPTIONS hygiene` test at lines 418-449 will fail loudly if
either step is mis-applied (declared vs observed count mismatch).

**AC5_FLOOR ratchet bump** (same commit):

Phase 3 introduces three new `var(--*)` references in
`MarkdownRenderer.module.css` (`var(--code-bg)`, `var(--code-fg)`,
`var(--code-stroke)`). Per the documented two-sided ratchet protocol
at `migration.test.ts:381-405`, bump `AC5_FLOOR` from its
pre-migration value (currently 423) to the observed count after
migration. Run the test once to observe the new count and update the
constant in the same commit. The second `it()` (`AC5_FLOOR is not
above observed`) catches mistakes in either direction.

#### 5. Remove github.css import + lock its absence

**File**: `skills/visualisation/visualise/frontend/src/main.tsx`
**Changes**: Delete the line `import 'highlight.js/styles/github.css'`
at line 7. The shared layer covers every class the visualiser emits
for the in-scope languages.

**File**: `skills/visualisation/visualise/frontend/src/main.import-hygiene.test.ts`
**Changes**: New file. Vitest spec that reads `main.tsx` as raw text
and asserts the github.css import is absent. Without this guard, a
future contributor could reintroduce the import to "fix a missing
class" and silently flip the cascade tie-break depending on load
order.

```ts
import { describe, it, expect } from 'vitest'
import mainSource from './main.tsx?raw'

describe('main.tsx import hygiene', () => {
  it('does not import highlight.js/styles/github.css (replaced by code-syntax.global.css in story 0076)', () => {
    expect(mainSource).not.toMatch(/highlight\.js\/styles\/github\.css/)
  })

  it('imports code-syntax.global.css after global.css (load-order contract)', () => {
    const globalIdx = mainSource.indexOf("import './styles/global.css'")
    const syntaxIdx = mainSource.indexOf("import './styles/code-syntax.global.css'")
    expect(globalIdx).toBeGreaterThanOrEqual(0)
    expect(syntaxIdx).toBeGreaterThan(globalIdx)
  })
})
```

### Success Criteria

#### Automated Verification:

- [ ] AC4 describe block (eight cases — six original AC4 + inline-
  code-passthrough + unknown-language) passes:
  `npm --prefix skills/visualisation/visualise/frontend run test -- MarkdownRenderer.test.tsx`
- [ ] `migration.test.ts` AC3 (hex literals) test for
  `components/MarkdownRenderer/MarkdownRenderer.module.css` passes
  with `#1e1e1e`/`#d4d4d4` removed from EXCEPTIONS (their absence
  from the module body now satisfies the rule).
- [ ] `migration.test.ts` `EXCEPTIONS hygiene` test passes — declared
  count for `1px` is 3, observed count is 3; reason text enumerates
  the three sites.
- [ ] `migration.test.ts` `var(--NAME) references resolve to declared
  tokens` test passes for the new `var(--code-bg)`,
  `var(--code-fg)`, `var(--code-stroke)` references (declared Set
  extended in Phase 1).
- [ ] `migration.test.ts` `AC5_FLOOR` ratchet passes — floor bumped
  to match observed count after migration.
- [ ] `main.import-hygiene.test.ts` passes — github.css absent;
  load-order assertion holds.
- [ ] Playwright spec from phase 2 still passes (cascade-resolved
  colour is unchanged; only chrome background/foreground tokens
  shifted).
- [ ] Playwright `<pre>` chrome describe block passes — `<pre>`
  border-color resolves to `formatRgba(255, 255, 255, 0.07)` (i.e.
  `--code-stroke`). This block was deferred from Phase 2 because the
  border declaration lands in Phase 3.
- [ ] `LibraryTemplatesView.test.tsx` still passes (templates preview
  is untouched in this phase — phase 4 owns it).
- [ ] Type-check + lint pass.

#### Manual Verification:

- [ ] Open a document detail view with fenced code blocks (e.g.
  `/library/plans/<this-plan>`). Code-block surface is now the deep
  navy `--code-bg` (#0e1320), text is `--code-fg` (#d7dcec), token
  colours are the prototype palette.
- [ ] Devtools: inspect the rendered `<pre>` element — `background`
  resolves to `rgb(14, 19, 32)`; no `#1e1e1e` literal anywhere.
- [ ] Inspect Network: `github.css` is no longer requested.

---

## Phase 4: Templates preview migration (depends on Phase 2; independent of Phase 3)

### Overview

Remove the locally-scoped `.previewBody :global(.hljs-*)` rules from
`LibraryTemplatesView.module.css` and let the shared layer take over.
Update `LibraryTemplatesView.test.tsx` to reflect the new contract:
the existing assertions at lines 198-199 (which expect the local
rules to be present) are inverted; new assertions assert the local
rules are absent and the shared layer declares the right mapping for
each previously-local class.

### Changes Required

#### 1. Test inversion (written first)

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.test.tsx`
**Changes**:

(a) Update the test at lines 174-200
(`emits highlight.js token classes for frontmatter YAML, markdown body, and template variables`)
to:

- Keep all the `container.querySelector('.hljs-meta')` /
  `.hljs-section` / `.hljs-template-variable` rendering assertions
  unchanged (they cover that the highlighter still emits the right
  classes — orthogonal to the CSS migration).
- Replace the two CSS-text assertions at lines 198-199 with a
  broader negative assertion that catches ALL forms of local
  hljs-colour-rule re-introduction (not just the `.previewBody`
  prefix). A future contributor wrapping a rule in `@media`, moving
  it under `.previewPane`, or nesting in any other selector should
  also fail the contract:

  ```ts
  // After 0076: hljs class colours come exclusively from the
  // shared `code-syntax.global.css` layer; the templates preview
  // module must not declare any `:global(.hljs-` colour rule.
  // Broader than the prior `.previewBody :global(.hljs-` check —
  // this catches selector renames, @media wraps, and nested
  // re-introductions.
  expect(templatesCss).not.toMatch(/:global\(\.hljs-/)
  ```

(b) Add a new describe block asserting each previously-local hljs
class is mapped by the shared layer to its `--tk-*` token. Reuses
the shared `assertSelectorColorIs` helper introduced in Phase 2 §0
(so the regex is not duplicated and the exact-match contract is the
same across test files).

```ts
import codeSyntaxCss from '../../styles/code-syntax.global.css?raw'
import { assertSelectorColorIs } from '../../styles/testing/cssRules'

describe('LibraryTemplatesView: hljs class colouring delegated to shared layer', () => {
  // Subset of REQUIRED_MAPPINGS in src/styles/code-syntax.test.ts
  // covering classes previously declared locally in
  // LibraryTemplatesView.module.css prior to 0076. If the shared
  // layer mapping changes, update BOTH this array and
  // REQUIRED_MAPPINGS in lock-step.
  const PREVIOUSLY_LOCAL_MAPPINGS: ReadonlyArray<[selector: string, token: string]> = [
    ['.hljs-attr',              'tk-attr'],
    ['.hljs-meta',              'tk-deco'],
    ['.hljs-string',            'tk-str'],
    ['.hljs-number',            'tk-num'],
    ['.hljs-literal',           'tk-lit'],
    ['.hljs-section',           'tk-header'],
    ['.hljs-bullet',            'tk-com'],
    ['.hljs-code',              'tk-com'],
    ['.hljs-quote',             'tk-com'],
    ['.hljs-link',              'tk-anchor'],
    ['.hljs-symbol',            'tk-anchor'],
    ['.hljs-template-variable', 'tk-var'],
  ]

  for (const [selector, token] of PREVIOUSLY_LOCAL_MAPPINGS) {
    it(`shared layer maps ${selector} → var(--${token})`, () => {
      assertSelectorColorIs(codeSyntaxCss, selector, token)
    })
  }

  it('shared layer preserves font-weight: 600 on .hljs-section (was local)', () => {
    expect(codeSyntaxCss).toMatch(/\.hljs-section\s*\{[^}]*font-weight:\s*600/)
  })

  it('shared layer carries emphasis/strong styling for markdown emphasis', () => {
    expect(codeSyntaxCss).toMatch(/\.hljs-emphasis\s*\{[^}]*font-style:\s*italic/)
    expect(codeSyntaxCss).toMatch(/\.hljs-strong\s*\{[^}]*font-weight:\s*600/)
  })

  it('templates preview module no longer declares any local hljs colour rules', () => {
    // Broader than the prior `.previewBody :global(.hljs-` check —
    // catches @media wraps, selector renames, and any nested
    // re-introduction.
    expect(templatesCss).not.toMatch(/:global\(\.hljs-/)
  })
})
```

(`.hljs-emphasis`/`.hljs-strong` were previously in the
templates-preview module at lines 197-204 alongside the colour
mappings; the shared layer preserves the italic/bold but
deliberately drops the colour-muting — see ADR-0026 §5
"Consequences" for the rationale.)

#### 2. Remove local hljs rules

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css`
**Changes**: Delete the entire comment block + rule block at lines
153-215 (the `Token theme` comment through the
`.previewBody :global(.hljs-template-variable)` rule). Keep
lines 148-151 (`.previewBody :global(.tpl-line)`) — those are
structural (min-height + white-space), not colour.

The file shrinks by ~63 lines. No other content moves.

(The base `.hljs` neutralisation at lines 171-174 is also removed —
the shared layer's `.hljs { background: transparent; color: inherit; }`
covers it globally.)

#### 3. Live-cascade verification via Playwright (optional but recommended)

**File**: `skills/visualisation/visualise/frontend/tests/visual-regression/code-block-resolved-colours.spec.ts`
**Changes**: Add a third describe block that navigates to a real
templates preview view and asserts a `getComputedStyle` for one
previously-local class (e.g. `.hljs-attr` → `--tk-attr`). This proves
the shared layer's specificity is sufficient to win in the templates
preview context (now that the local `.previewBody` rules are gone,
nothing competes).

```ts
test.describe('templates preview', () => {
  test('.hljs-attr in templates preview resolves to --tk-attr', async ({ page }) => {
    await page.goto('/library/templates/adr')  // or whichever fixture template is loaded by the dev server
    const colour = await page
      .locator('[data-testid="template-preview-pane"] .hljs-attr')
      .first()
      .evaluate((el) => getComputedStyle(el).color)
    expect(colour).toBe(hexToRgbString(CODE_SYNTAX_TOKENS['tk-attr']))
  })
})
```

(If a stable templates fixture is not available in the dev server's
in-memory data, this block may need to be guarded behind a fixture
seed or omitted; the Vitest CSS-structural assertion is the floor.)

### Success Criteria

#### Automated Verification:

- [ ] Inverted CSS-text assertion in
  `LibraryTemplatesView.test.tsx` (line ~199) passes — local rules
  are absent: `npm --prefix skills/visualisation/visualise/frontend run test -- LibraryTemplatesView.test.tsx`
- [ ] All 12 `PREVIOUSLY_LOCAL_MAPPINGS` assertions pass.
- [ ] `.hljs-emphasis` / `.hljs-strong` shared-layer assertions pass.
- [ ] Rest of `LibraryTemplatesView.test.tsx` (TIER cards, content
  hash, SSE end-to-end, `.tpl-line` empty-line preservation, etc.)
  continues to pass — no regression in templates preview functionality.
- [ ] `migration.test.ts` continues to pass — removing rules can only
  reduce literal counts in this file, and the removed rules
  consumed `var(--ac-*)` tokens, not literals. All EXCEPTIONS for
  `routes/library/LibraryTemplatesView.module.css` (lines 138-146)
  are retained — those cover non-hljs literals.
- [ ] Type-check + lint pass.
- [ ] Playwright templates-preview block passes (if included).

#### Manual Verification:

- [ ] Navigate to `/library/templates/<any-template>` in the dev
  server. The preview pane renders YAML frontmatter keys in
  `--tk-attr` (#c18cf0), strings in `--tk-str` (#6be58b), the `---`
  frontmatter delimiter and template variables in their respective
  tokens.
- [ ] The preview pane's `.previewBody` background (`--ac-bg-sunken`)
  is unchanged — only the syntax colours moved.
- [ ] Toggle theme; the syntax colours stay the same (theme-
  invariant); the preview pane container chrome (border, background)
  follows the existing theme tokens.

---

## Testing Strategy

### Unit Tests (Vitest)

- **`global.test.ts`**: extend the existing parity matrix and add
  the fixture three-way check. Driven by `describe.each` so per-token
  failures surface individually.
- **`prototype-tokens.fixture.test.ts`** (new): drift detector
  reading the prototype HTML.
- **`contrast.test.ts`** (new or extended): `hexToRgbString`
  contract — edge cases for case, length validation, error paths.
- **`code-syntax.test.ts`** (new): structural assertion that the
  shared CSS layer declares every required mapping; one `it()` per
  row of the mapping table; explicit `.hljs-emphasis` /
  `.hljs-strong` font-style/weight checks.
- **`MarkdownRenderer.test.tsx`**: extended with six AC4 behaviours
  (GFM task list, GFM table, wiki-link routing, python keyword,
  typescript keyword, fence+wiki-link adjacency).
- **`CodeSyntaxShowcase.test.tsx`** (new): smoke test for the dev-
  only showcase route.
- **`LibraryTemplatesView.test.tsx`**: invert existing CSS-text
  assertions; add the shared-layer mapping assertions.
- **`migration.test.ts`**: phase 3 removes two EXCEPTIONS entries
  and updates one count; the `EXCEPTIONS hygiene` test enforces
  declared-vs-observed parity automatically.

### Integration Tests (Playwright)

- **`code-block-resolved-colours.spec.ts`** (new): live-cascade
  verification for the mapping table on `/code-syntax-showcase`,
  including diff rows; both themes; templates-preview verification
  block (phase 4).
- Existing visual-regression specs (`tokens.spec.ts`,
  `glyph-resolved-fill.spec.ts`, etc.) continue to pass —
  no overlap with the new tokens.

### Manual Testing Steps

1. After phase 1: open the dev server; navigate anywhere; nothing
   should look different. Confirms tokens are inert until consumed.
2. After phase 2: navigate to `/code-syntax-showcase`; each language
   section renders with the prototype palette. Check DevTools that
   github.css is still loaded (phase 3 removes it).
3. After phase 3: navigate to a doc with fenced code (e.g.
   `/library/plans/<this-plan>`); the `<pre>` chrome is deep navy
   (`#0e1320`), text is `#d7dcec`, with a hairline `--code-stroke`
   border. github.css no longer loads (Network tab).
4. After phase 4: navigate to `/library/templates/<template-name>`;
   the YAML/markdown preview renders with the prototype palette
   (purple keys, green strings, etc.); container chrome is
   unchanged.

## Performance Considerations

- The new `code-syntax.global.css` adds ~40 selectors at flat
  `.hljs-*` single-class specificity. Cascade resolution cost is
  trivial; the file is smaller than the github.css it replaces.
- The shared layer fully replaces `highlight.js/styles/github.css`
  (removed in phase 3), so the application's total CSS surface
  shrinks net.
- No runtime cost: tokens resolve at paint time via standard CSS
  custom-property substitution.

## Migration Notes

- All existing user-visible code blocks (markdown renderer +
  templates preview) shift palette in a single deploy. There is no
  feature flag — the prototype palette is the target end state and
  parallel-running both palettes would add no value (the prototype
  is theme-invariant so accessibility risk is bounded).
- Visual regression screenshots that capture code-block surfaces
  WILL diff after this story. The existing visual-regression
  snapshot suite at `tests/visual-regression/__screenshots__/`
  must be reviewed for diffs and re-baselined as part of the phase
  3/4 PR. Re-baseline procedure: run the visual-regression suite
  with the project's standard snapshot-update flag (check the
  `package.json` `scripts` section under
  `skills/visualisation/visualise/frontend/` for the exact npm
  script; typical Playwright pattern is
  `npm run test:visual -- --update-snapshots` or equivalent). If
  no procedure is documented in the package README, the phase 3/4
  PR must add one — visual rebaselining without a documented
  procedure is itself a documentation gap that needs filling.

## References

- Original work item: `meta/work/0076-code-block-syntax-highlight-palette.md`
- Codebase research: `meta/research/codebase/2026-05-21-0076-code-block-syntax-highlight-palette.md`
- Review pass-2 verdict: `meta/reviews/work/0076-code-block-syntax-highlight-palette-review-1.md`
- Prototype source: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html` (`.ac-codeblock` block)
- Visual ground truth (light/dark, theme-invariant):
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/screenshots/doc-detail-plan-meta-visualisation-fullpage.png`
  and `…-fullpage-dark.png`
- ADR being amended: `meta/decisions/ADR-0026-css-design-token-application-conventions.md` (§3 table; Appendix table)
- Foundation: `meta/work/0033-design-token-system.md` (closes the `--code-*`/`--tk-*` gap)
- Test pattern precedents:
  - `skills/visualisation/visualise/frontend/tests/visual-regression/glyph-resolved-fill.spec.ts` (resolved-colour Playwright)
  - `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.test.tsx:58-62` (inline `style={{ color: 'var(--ac-*)' }}` precedent)
  - `skills/visualisation/visualise/frontend/src/styles/global.test.ts:127-165` (two-source parity precedent)
  - `skills/visualisation/visualise/frontend/src/styles/fonts.test.ts:1-29` (`readFileSync` fixture pattern)
- Related stories: 0042 (templates view consumes shared palette), 0083 (DevDesignSystem showcase), 0088 (markdown body width), 0089 (templates preview whitespace fix), 0075 (typography size scale)
