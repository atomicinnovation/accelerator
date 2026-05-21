---
date: 2026-05-21T22:51:56+01:00
researcher: Toby Clemson
git_commit: 504eb39ccc6e8eb44b3134e76ccabcbed4067f68
branch: vulqmkrnzrykvxxlplmplysrzvspwoon
repository: accelerator
topic: "Code-block syntax-highlight tokens and renderer adoption (story 0076)"
tags: [research, codebase, design-tokens, syntax-highlighting, hljs, markdown, react-markdown, rehype-highlight, code-blocks, visualiser]
status: complete
last_updated: 2026-05-21
last_updated_by: Toby Clemson
---

# Research: Code-block syntax-highlight tokens and renderer adoption (story 0076)

**Date**: 2026-05-21 22:51:56 BST
**Researcher**: Toby Clemson
**Git Commit**: 504eb39ccc6e8eb44b3134e76ccabcbed4067f68
**Branch**: vulqmkrnzrykvxxlplmplysrzvspwoon
**Repository**: accelerator

## Research Question

Gather the codebase context required to plan implementation of
`meta/work/0076-code-block-syntax-highlight-palette.md` — a story that
adds a self-contained `--code-*` / `--tk-*` palette to the visualiser,
maps `hljs-*` classes onto it via a shared CSS layer, and adopts that
layer in both the `react-markdown` pipeline and the templates preview.

## Summary

The story is buildable on the existing foundation, but four concrete
implementation realities diverge from the story body and must inform
the plan:

1. **The token-system directory is `src/styles/`, not `src/design/`.**
   Every story reference to `src/design/...` (the shared CSS layer, the
   `__fixtures__/prototype-tokens.json`, and the
   `prototype-tokens.fixture.test.ts`) lands on a non-existent path.
   The de-facto design-token layer is `src/styles/` (see
   [skills/visualisation/visualise/frontend/src/styles/](#token-infrastructure)).
2. **AC2/AC3 `getComputedStyle(span).color === rgb(r, g, b)` is not
   feasible under Vitest/jsdom** — the harness does not substitute
   `var()`. The repo's existing convention for resolved-value
   assertions is a Playwright spec under
   `tests/visual-regression/`. Two AC items must either move to
   Playwright or assert literal `var(--tk-*)` strings on inline styles
   (the `Glyph.test.tsx:58-62` pattern).
3. **ADR-0026 currently lists the code-block surface colours
   (`#1e1e1e`, `#d4d4d4`) as irreducible** in
   `src/styles/migration.test.ts:59-67`. Story 0076 directly invalidates
   that classification — landing the `--code-*` family requires
   amending the ADR and removing those entries from the `EXCEPTIONS`
   array, or `migration.test.ts` will keep treating the new tokens'
   replacement values as ordinary literals.
4. **Two distinct render pathways must converge on the same layer.**
   `MarkdownRenderer` (`src/components/MarkdownRenderer/`) currently
   colours its `.hljs-*` via the global `highlight.js/styles/github.css`
   import at `src/main.tsx:7`. The templates preview colours its
   `.hljs-*` via locally-scoped `.previewBody :global(.hljs-*)` rules
   in `LibraryTemplatesView.module.css:153-215`. Neither layer reaches
   the other today; the new shared layer must replace both.

All other story assumptions check out: 0033 is shipped (token system
foundation exists), `rehype-highlight` runs with default options
(plain hljs class emission), `template-highlight.tsx` emits exactly the
`hljs-template-variable` class the mapping table assigns to `--tk-var`,
and the prototype palette is indeed theme-independent (declared once
on `.ac-codeblock`, no light/dark overrides).

## Detailed Findings

### Token infrastructure

The design-token system lives entirely under
`skills/visualisation/visualise/frontend/src/styles/`. There is no
`src/design/` directory anywhere in the frontend.

- `src/styles/global.css` — canonical CSS variable declarations. Three
  parallel blocks:
  - `:root` (lines 69-181) — light + theme-invariant tokens, grouped by
    family with comment headers (colour, doc-type, typography, spacing,
    radius, shadow, layout, `color-scheme: light dark`).
  - `[data-theme="dark"]` (lines 183-242) — MIRROR-A, canonical dark
    colour/shadow overrides. Flat block, no nested rules.
  - `@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { ... } }`
    (lines 244-301) — MIRROR-B, byte-equivalent duplicate of MIRROR-A.
  - `[data-font="mono"]` (lines 303-311) — MIRROR-C, mono-mode font
    remap.
- `src/styles/tokens.ts` — TS mirror. One `as const` object per family
  (`LIGHT_COLOR_TOKENS`, `DARK_COLOR_TOKENS`, `TYPOGRAPHY_TOKENS`,
  `SPACING_TOKENS`, `RADIUS_TOKENS`, `LIGHT_SHADOW_TOKENS`,
  `DARK_SHADOW_TOKENS`, `LAYOUT_TOKENS`, `MONO_FONT_TOKENS`), then
  `keyof typeof X_TOKENS` type exports. Keys are the CSS name **without**
  the leading `--`, kebab-case, quoted, e.g. `'ac-bg'`, `'sp-1'`. Values
  are the verbatim CSS-value string, lowercase.
- `src/styles/global.test.ts` — parity harness. Reads `global.css?raw`,
  parses each block with a non-greedy regex
  (`readCssVar(name, scope)` at lines 29-41) that **requires flat
  blocks** — any nested selector inside `:root` truncates the regex
  silently. Truncation-guard tests at lines 197-204 assert a known-last
  token is still readable. Set-equality check between MIRROR-A and
  MIRROR-B at lines 127-165 catches divergence between the two dark
  blocks but only over the keys that appear in those blocks (a
  theme-invariant family declared only under `:root` is not cross-
  checked against the dark blocks — which is the correct behaviour for
  0076's theme-independent `--code-*`/`--tk-*` family).
- `src/styles/contrast.ts` — colour helpers. Public `parseHex(hex) →
  {r,g,b}` (lines 1-15), `parseRgba(value) → {r,g,b,a}` (lines 17-26),
  `contrastRatio(fg, bg)` (lines 53-59). **There is no
  `hexToRgbString`.** Two Playwright specs each define a local inline
  `hexToRgb()` returning `"rgb(R, G, B)"`:
  `tests/visual-regression/chip-resolved-colours.spec.ts:4-10` and
  `tests/visual-regression/glyph-resolved-fill.spec.ts:4-10`. The
  story's planned helper is genuinely net-new.
- `src/styles/migration.test.ts` — literal-value migration guard, with
  an `EXCEPTIONS` array of irreducible literals. Lines 59-67 currently
  list the editor palette colours `#1e1e1e`/`#d4d4d4` from
  `MarkdownRenderer.module.css` as
  `kind: 'irreducible' /* Code-block dark colours, no surface token */`.
  Removing this exception is part of 0076's landing surface.
- `src/styles/wiki-links.global.css` — separate global stylesheet
  pattern. Confirms the repo convention: `*.global.css` next to
  `global.css` for layered global rules. The new shared hljs layer
  should follow this convention rather than living under a `src/design/`
  directory that does not exist.

`global.css` is imported once at `src/main.tsx:8`. `wiki-links.global.css`
is imported at `src/main.tsx:9`. `highlight.js/styles/github.css` is
imported at `src/main.tsx:7` — this is the single global hljs theme
applied to MarkdownRenderer output today.

README contract: `frontend/README.md:81` documents that tokens must
appear in **both** `tokens.ts` and all three theme blocks in
`global.css`, gated by `src/styles/global.test.ts`.

### Markdown renderer pipeline

`src/components/MarkdownRenderer/MarkdownRenderer.tsx` is the single
production renderer. Plugin wiring (lines 34-45):

- `remarkPlugins`: `[remarkGfm]` by default; `[remarkGfm,
  [remarkWikiLinks, effectivePattern, resolveWikiLink]]` when a
  resolver is supplied.
- `rehypePlugins`: `[rehypeHighlight]` — passed **bare, no options
  object**. No `subset`/`ignoreMissing`/`languages` override; full
  `lowlight` common bundle and auto-detect defaults are in force.
- Output wrapper: `<div className={styles.markdown}>` (line 42).

Wiki-link plugin
(`src/components/MarkdownRenderer/wiki-link-plugin.ts:48-65`) visits
mdast `text` nodes only — code nodes are skipped by structure, so the
new CSS layer cannot regress wiki-link rendering. Resolved links
become standard mdast `link` nodes pointing at
`/library/{type}/{fileSlug}` (constructed in
`src/api/wiki-links.ts:110-129`). Unresolved/pending get `<span>`
wrappers with the classes `unresolved-wiki-link` / `wiki-link-pending`.

Production call sites of `MarkdownRenderer`:

- `src/routes/library/LibraryDocView.tsx:120` — the only one. Inside
  `<div className={styles.body}>`, with resolver + pattern from
  `useWikiLinkResolver()`.

`LibraryTemplatesView` (the templates preview at
`src/routes/library/LibraryTemplatesView.tsx:169`) does **not** use
`MarkdownRenderer`. It uses `<TemplateHighlight content={…} />`
which runs `highlight.js` imperatively (markdown + yaml languages only,
registered at `template-highlight.tsx:2-12`), wraps `{{var}}` in
`<span class="hljs-template-variable">` (lines 63-68 — class name
confirmed), and emits a hand-injected `<span class="hljs-meta">---</span>`
for the frontmatter fence (line 110). Its root element carries the
`hljs` class (`<pre className="tpl-highlight hljs">`, lines 37-43) so
the global github.css theme would apply if not neutralised by
`.previewBody :global(.hljs) { background: transparent; color: inherit; }`
at `LibraryTemplatesView.module.css:171-174`.

### Current code-block / hljs styling surfaces

Three independent surfaces govern code-block appearance today:

1. **`highlight.js/styles/github.css`** — imported globally at
   `src/main.tsx:7`. Provides all `.hljs-*` colours in the
   `MarkdownRenderer` output. The new shared layer must replace this
   (or be loaded after it, since `.hljs-*` selectors have identical
   specificity and load-order decides).
2. **`MarkdownRenderer.module.css:13-21`** — the `<pre>` / `<code>`
   chrome:

   ```css
   .markdown pre {
     background: #1e1e1e; color: #d4d4d4;
     border-radius: 6px; padding: var(--sp-4);
     overflow-x: auto; font-size: var(--size-xs);
   }
   .markdown code:not(pre code) {
     background: var(--ac-bg-sunken); border-radius: var(--radius-sm);
     padding: 0.1rem var(--sp-1); font-size: 0.88em;
   }
   ```

   The `#1e1e1e`/`#d4d4d4` literals are the targets for replacement by
   `var(--code-bg)` / `var(--code-fg)`. This is exactly the `<pre>`
   touchup the story flags as dependent on 0075.
3. **`LibraryTemplatesView.module.css:153-215`** — local hljs theme
   scoped under `.previewBody`. Current mappings (with non-colour
   properties noted):

   | Lines | Selector (under `.previewBody :global(...)`) | Token / extra |
   |-------|----------------------------------------------|--------------|
   | 171-174 | `.hljs` | `background: transparent; color: inherit` |
   | 175-177 | `.hljs-attr` | `var(--ac-accent)` |
   | 178-180 | `.hljs-meta` | `var(--ac-fg-faint)` |
   | 181-183 | `.hljs-string` | `var(--ac-ok)` |
   | 184-186 | `.hljs-number` | `var(--ac-warn)` |
   | 187-189 | `.hljs-literal` | `var(--ac-accent-2)` |
   | 190-193 | `.hljs-section` | `var(--ac-fg-strong)`; `font-weight: 600` |
   | 194-196 | `.hljs-bullet` | `var(--ac-fg-faint)` |
   | 197-200 | `.hljs-emphasis` | `var(--ac-fg-muted)`; `font-style: italic` |
   | 201-204 | `.hljs-strong` | `var(--ac-fg-strong)`; `font-weight: 600` |
   | 205-208 | `.hljs-code, .hljs-quote` | `var(--ac-accent-2)` |
   | 209-212 | `.hljs-link, .hljs-symbol` | `var(--ac-accent)` |
   | 213-215 | `.hljs-template-variable` | `var(--ac-accent-2)` |

   Container chrome (lines 104-146): `.previewPane` (border, radius,
   sunken bg, overflow), `.previewHeader` (card-tinted band), and
   `.previewBody` (padding, mono font, size-xxs, fg). These container
   tokens stay; only the hljs colour layer migrates.

The file's intent comment at lines 153-156 ("we map them to
project-token colours via :global selectors so the same theme is
reusable wherever hljs is rendered") signals the planned migration; the
actual `.previewBody` scoping is what makes the rules locally bound and
prevents reuse today.

### Existing tests that lock the current shape

These break or need updating once 0076 lands:

- `src/routes/library/LibraryTemplatesView.test.tsx:198-199` — regex-
  asserts that the `:global(.hljs-meta)` and
  `:global(.hljs-template-variable)` rules exist with `color:` in the
  CSS module. Once the rules move out, these assertions must update.
- `src/styles/migration.test.ts:59-67` — the irreducible-literals
  exception entry for the editor palette. Must be removed when the
  `<pre>` chrome adopts `var(--code-bg)` / `var(--code-fg)`.
- `src/styles/global.test.ts:127-165` — the MIRROR-A vs MIRROR-B set-
  equality check. New `--code-*` / `--tk-*` tokens declared only under
  `:root` do not appear in either dark block, so this test does not
  fire. Good (theme-invariant is the intent), but worth confirming.

### Test patterns and constraints (Vitest jsdom)

- Test runner config: `vite.config.ts:63-70` — `environment: 'jsdom'`,
  `css: true` (so `?raw` imports work), setup file is
  `src/test/setup.ts`. The setup file registers
  `@testing-library/jest-dom` and stubs `EventSource`, `ResizeObserver`,
  `matchMedia`, `scrollTo`, `scrollIntoView`. **It does not inject
  `global.css` into the document** and does not establish CSS variable
  resolution.
- jsdom limitation, explicitly documented at
  `src/components/Glyph/Glyph.test.tsx:167-170`:
  > "AC #4 (`getComputedStyle(svg).fill resolves to a hex`) is verified
  > end-to-end by Playwright … JSDOM does not reliably substitute
  > `var()` in SVG presentation attributes — see the Resolved Decision
  > in `meta/work/0037-glyph-component.md`."
- Existing Playwright "resolved style" precedents:
  `tests/visual-regression/glyph-resolved-fill.spec.ts:18-21` and
  `tests/visual-regression/chip-resolved-colours.spec.ts:67-92`. Both
  use `page.locator(...).evaluate((el) => getComputedStyle(el).color)`
  and compare to the channel-converted token value. Theme is toggled by
  `document.documentElement.dataset.theme = '…'`.
- Inline-style precedent (the Vitest-feasible alternative):
  `src/components/Glyph/Glyph.test.tsx:58-62` — asserts
  `svg.style.color === 'var(--ac-doc-research)'`. Useful if the new
  shared layer is restructured to expose `--tk-*` references on
  `style` attributes rather than relying on cascade resolution.
- Two-source parity precedent for the prototype-fixture comparison:
  `src/styles/global.test.ts:127-165`. The
  `normalise(s) → Map<string,string>` helper plus key-set + value
  equality is the template for asserting `prototype-tokens.json`
  matches `prototype-standalone.html`.
- File-from-disk pattern: `src/styles/fonts.test.ts:1-29` uses
  `readFileSync(resolve(process.cwd(), 'public/fonts/…'))`. For the
  prototype HTML, `process.cwd()`-rooted reads are the right call (it
  lives outside the frontend `src/` tree). For the committed
  `prototype-tokens.json`, place it inside `src/styles/` (no
  `__fixtures__/` convention currently exists — the closest precedent
  is sibling-named test fixtures).
- Tests rendering `react-markdown` and asserting on output:
  `src/components/MarkdownRenderer/MarkdownRenderer.test.tsx` —
  `render(<MarkdownRenderer content={...} />)` plus
  `container.querySelector` for class-name probes (e.g. lines 66-76 for
  the wiki-link unresolved span).

### Prototype palette (extracted verbatim from `prototype-standalone.html`)

All tokens declared **once**, under a single selector `.ac-codeblock`.
No `:root` declaration, no `[data-theme="dark"]` override anywhere in
the file. The comment immediately above the block reads "reads as code
regardless of the surrounding theme" — confirming the story's theme-
independent claim.

`--code-*` (5):

```
--code-bg:        #0E1320
--code-bg-head:   #161B2C
--code-stroke:    rgba(255,255,255,0.07)
--code-fg:        #D7DCEC
--code-fg-faint:  #6F7796
```

`--tk-*` (27 — incl. diff tokens):

```
--tk-com: #6F7796     --tk-str: #6BE58B     --tk-num: #F9DE6F
--tk-kw:  #C1C5FF     --tk-lit: #F9A66B     --tk-typ: #73E4E2
--tk-fn:  #FFC1A8     --tk-attr: #C18CF0    --tk-deco: #C18CF0
--tk-macro: #DF9CE6   --tk-var: #72CBF5     --tk-key: #C1C5FF
--tk-flag: #F9DE6F    --tk-heredoc: #C18CF0 --tk-pun: #8990B0
--tk-lifet: #F9A66B   --tk-header: #C18CF0  --tk-anchor: #DF9CE6
--tk-tag: #DF5758     --tk-doctype: #C18CF0 --tk-bn: #72CBF5
--tk-prop: #72CBF5    --tk-sel: #FFC1A8     --tk-atrule: #C18CF0
--tk-dhdr: #C18CF0    --tk-dhunk: #72CBF5   --tk-dadd: #6BE58B
--tk-ddel: #E56B7E
```

Format notes: all values are 6-digit hex with uppercase letters in
the source **except** `--code-stroke` which is `rgba(...)`. `tokens.ts`
convention is lowercase (`'#0e1320'`); the fixture should preserve the
original case as written in the prototype for byte-equal drift
detection, with the case normalisation applied at compare time.

Heavily-shared hex values (matters for the fixture and the assertion
shape — assertions that compare token name → hex are robust to this;
assertions that compare hex → name are not):
`#C18CF0` is reused across 7 tokens (`--tk-attr`, `--tk-deco`,
`--tk-heredoc`, `--tk-header`, `--tk-doctype`, `--tk-atrule`,
`--tk-dhdr`). `#72CBF5` across 4 (`--tk-var`, `--tk-bn`, `--tk-prop`,
`--tk-dhunk`). And `#6F7796` straddles families (`--code-fg-faint` and
`--tk-com`).

### Story 0033 alignment (foundation)

Status: **done**. Path:
`meta/work/0033-design-token-system.md`. 0033 shipped:

- `--ac-*` colour layer (light in `:root`, dark in `[data-theme="dark"]`).
- `--size-*`, `--lh-*`, `--tracking-caps` typography.
- `--sp-1..11` spacing.
- `--radius-{sm,md,lg,pill}` radius.
- `--shadow-*` + `--ac-shadow-*` shadow.

Excluded explicitly: brand `--atomic-*` and legacy aliases (`--fg-*`,
`--bg-*`, `--accent`, `--stroke`). 0033 did NOT add a "code/syntax
tokens out of scope" caveat — story 0076's Technical Notes correctly
flags this as the gap 0076 closes.

The 0033 parity test pattern (per-token `it()` inside `Object.entries`
loops, lowercase-normalised compare, truncation guard) is the harness
0076 must extend.

### ADR-0026 — the irreducibility wrinkle

`meta/decisions/ADR-0026-css-design-token-application-conventions.md`
codifies three things: `color-mix()` for theme-aware tints, a ±2px
tolerance for spacing/typography substitution, and a fixed list of
irreducible literal categories. Critically:

> "Editor palette colours `#1e1e1e`, `#d4d4d4` are explicitly listed as
> irreducible (`Code-block dark colours, no surface token`) and tagged
> `kind: 'irreducible'` in EXCEPTIONS."

Story 0076 directly contradicts this classification — landing a
`--code-*` family means `#1e1e1e` IS reducible to `var(--code-bg)`,
and `#d4d4d4` to `var(--code-fg)`. The implementation must (a) amend
ADR-0026 to remove the irreducible classification for those two
colours, AND (b) update the EXCEPTIONS array in
`src/styles/migration.test.ts` to remove the corresponding entry.
Otherwise `migration.test.ts` continues to permit the literals
indefinitely, defeating the migration's enforcement purpose for any
later regression.

The ADR is otherwise silent on hljs / syntax highlighting and does not
prescribe naming for new token families.

### Story 0076 review history

`meta/reviews/work/0076-code-block-syntax-highlight-palette-review-1.md`
performed two passes. Pass 1 verdict was REVISE with three majors
(dependency ordering, AC4 unboundedness, AC2 hljs class set
unpinned) and ten minors. Pass 2 verdict is COMMENT — "The work item is
ready for implementation" — all eleven items resolved in the current
story body (mapping table now enumerates the required hljs classes,
AC4 enumerates six concrete behaviours, AC1 introduces the committed
`prototype-tokens.json` fixture and the drift-detection sibling test,
AC2/AC3 specify the canonical-form comparison via `hexToRgbString`,
dependency relationships promoted where appropriate).

Open planning-phase observations from Pass 2 (these are NOT story
blockers but should be settled during plan-of-record):

- `hljs-meta` precedence: the general row (`hljs-meta → --tk-deco`)
  vs the diff row (`hljs-meta inside language-diff → --tk-dhdr`).
  Cascade order requires the diff selector to come after the general
  one, with `:where()` or `:not()` scoping options to consider.
- AC4's "auto-detected python / typescript" — clarify whether
  un-classed fences or `language-python` / `language-typescript`
  fences (the latter is implementer-trivial; the former depends on
  hljs auto-detect outcomes).
- `hljs-code` referenced in AC5 but absent from the Requirements
  mapping table. Resolve: add a row, or drop the AC5 mention.
- Slash-separated entries in AC5 (`hljs-code/hljs-quote`,
  `hljs-link/hljs-symbol`) — clarify whether one assertion per slash-
  separated class or one per group.
- Canonical label for the shared layer (`code-syntax.css` or
  similar) — settle naming with the directory correction (use
  `src/styles/`, not `src/design/`).
- AC2 "one span per row of the required-mappings table" is ambiguous
  for chained-class rows like `hljs-function, hljs-title.function_`
  and `hljs-meta.doctype` — clarify whether to exercise both
  variants.
- `prototype-tokens.json` schema unspecified — recommend a flat
  `{ "--tk-com": "#6F7796", ... }` map, mirroring how `tokens.ts`
  represents values.
- `hexToRgbString` contract — define behaviour for 3-digit hex (the
  prototype uses only 6-digit; 3-digit can be rejected), uppercase
  input (the prototype uses uppercase), Chromium's modern
  `color(srgb r g b)` serialisation, and the rgba branch (only
  `--code-stroke` uses rgba).

### Related work items (story dependencies)

- 0033 (Design Token System) — done; the foundation.
- 0042 (Templates View Redesign) — Blocks-by relationship: must wait
  for the shared layer.
- 0075 (Typography Size-Scale Consumption) — Blocked-by; or 0076
  takes the `<pre>` touchup if 0075 has not landed.
- 0083 (DevDesignSystem Reference Page) — showcases primitives
  introduced here.
- 0088 (Markdown Body Width Harmonisation) — consumes the unified
  markdown code-block surface.
- 0089 (Templates Preview Whitespace Fix) — touches the same hljs
  region; rebases onto post-migration CSS.

## Code References

Token infrastructure:
- `skills/visualisation/visualise/frontend/src/styles/global.css` — three-block mirror structure (`:root`, `[data-theme="dark"]`, `@media (prefers-color-scheme: dark)`), plus `[data-font="mono"]` and `@keyframes ac-pulse`.
- `skills/visualisation/visualise/frontend/src/styles/tokens.ts` — per-family `as const` objects with `keyof typeof` type exports.
- `skills/visualisation/visualise/frontend/src/styles/global.test.ts:29-41` — `readCssVar` regex helper; lines 113-125 brace-balanced `extractBlockBody`; lines 127-165 MIRROR-A/B set-equality; lines 197-204 truncation guard.
- `skills/visualisation/visualise/frontend/src/styles/contrast.ts:1-26` — `parseHex` / `parseRgba` (no `hexToRgbString`).
- `skills/visualisation/visualise/frontend/src/styles/migration.test.ts:59-67` — irreducible exception for `#1e1e1e`/`#d4d4d4`.
- `skills/visualisation/visualise/frontend/src/main.tsx:7-9` — global stylesheet import order (`highlight.js/styles/github.css`, `global.css`, `wiki-links.global.css`).
- `skills/visualisation/visualise/frontend/README.md:81` — "tokens appear in both `tokens.ts` and global.css" contract.

Markdown renderer pipeline:
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:34-45` — plugin wiring, `rehypeHighlight` bare.
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:13-21` — hard-coded `<pre>` chrome with `#1e1e1e`/`#d4d4d4`.
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/wiki-link-plugin.ts:48-65` — visits `text` nodes only (code-safe).
- `skills/visualisation/visualise/frontend/src/api/wiki-links.ts:110-129` — `/library/{type}/{fileSlug}` href shape.
- `skills/visualisation/visualise/frontend/src/api/use-wiki-link-resolver.ts:43-80` — combined resolver hook.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx:120` — sole `MarkdownRenderer` call site.

Templates preview:
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx:169` — uses `TemplateHighlight`, not `MarkdownRenderer`.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css:104-146` — container chrome (`.previewPane`, `.previewHeader`, `.previewBody`).
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css:153-215` — hljs colour mappings to migrate.
- `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.test.tsx:198-199` — locks the `hljs-meta` / `hljs-template-variable` rule presence; must update.
- `skills/visualisation/visualise/frontend/src/routes/library/template-highlight.tsx:2-12` — `markdown` + `yaml` only.
- `skills/visualisation/visualise/frontend/src/routes/library/template-highlight.tsx:63-68` — `hljs-template-variable` span emission.
- `skills/visualisation/visualise/frontend/src/routes/library/template-highlight.tsx:110` — hand-injected `hljs-meta` for frontmatter fence.

Test setup:
- `skills/visualisation/visualise/frontend/vite.config.ts:63-70` — Vitest config (`environment: 'jsdom'`, `css: true`).
- `skills/visualisation/visualise/frontend/src/test/setup.ts` — registers jest-dom, stubs `EventSource`/`ResizeObserver`/`matchMedia`/`scrollTo`/`scrollIntoView`; does NOT inject global CSS.
- `skills/visualisation/visualise/frontend/src/components/Glyph/Glyph.test.tsx:167-170` — jsdom limitation comment.
- `skills/visualisation/visualise/frontend/tests/visual-regression/glyph-resolved-fill.spec.ts:18-21` — canonical `getComputedStyle` against token assertion (Playwright).
- `skills/visualisation/visualise/frontend/tests/visual-regression/chip-resolved-colours.spec.ts:4-10, 67-92` — `hexToRgb` local helper + theme-toggle via `data-theme` dataset.
- `skills/visualisation/visualise/frontend/src/styles/fonts.test.ts:1-29` — `readFileSync(resolve(process.cwd(), ...))` fixture-reading pattern.

Prototype source:
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html` — palette declared under `.ac-codeblock` only (no theme overrides). Embedded on a single minified line.

## Architecture Insights

- **Two-mirror dark blocks.** Dark colour overrides exist in both
  `[data-theme="dark"]` (canonical) and the `@media (prefers-color-
  scheme: dark)` block (hand-mirrored). New theme-invariant families
  declared only under `:root` are not double-declared — and need not
  be — but the MIRROR-A/B parity test does cross-check them. The
  truncation-guard test depends on **flat block structure**; any
  nested rule introduced inside `:root` silently breaks the regex.
- **Token consumption convention is `var(--name)` in CSS modules.**
  Inline `style={{ color: 'var(--tk-kw)' }}` is also valid and is the
  only practical way to make colour resolution observable from
  jsdom-bound tests. The Glyph component uses this pattern
  intentionally (see `Glyph.test.tsx:58-62`).
- **Global CSS layers live as `*.global.css` next to `global.css`.**
  The natural home for the new shared hljs layer is therefore
  `src/styles/code-syntax.global.css` (NOT `src/design/code-syntax.css`
  — the directory does not exist, and an ad-hoc convention split is
  unnecessary).
- **Specificity and load order rule the cascade for `.hljs-*`.**
  `highlight.js/styles/github.css` carries flat `.hljs-*` selectors
  with single-class specificity, identical to whatever the new layer
  emits. The new layer must be imported **after** github.css in
  `main.tsx`, OR github.css must be removed entirely (the safer choice
  — once the new layer covers every hljs class the visualiser
  encounters, github.css is redundant).
- **`.previewBody` scoping was an early reusability hedge that didn't
  ship as intended.** The intent comment at
  `LibraryTemplatesView.module.css:153-156` describes the shared layer
  this story builds; the actual selectors never made the leap. The
  migration is genuinely lifting the same code, just removing the
  scope and re-pointing token names.
- **`migration.test.ts` is a forward-pressure mechanism.** Removing
  the `#1e1e1e`/`#d4d4d4` exception when the new tokens land turns the
  guard from "permits the literal" to "prevents regression to the
  literal". This is the right way to convert ADR-0026 from
  documentation into enforcement.

## Historical Context

- `meta/work/0033-design-token-system.md` — shipped foundation
  (status: done); established the `:root`/`[data-theme="dark"]` split
  and the `tokens.ts` mirror.
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md`
  — currently lists `#1e1e1e`/`#d4d4d4` as irreducible; needs
  amendment as part of 0076.
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
  — the design-gap analysis 0076 cites as its source.
- `meta/research/codebase/2026-05-06-0033-design-token-system.md` —
  prior research that established the token-system conventions.
- `meta/research/codebase/2026-05-18-0042-templates-view-redesign.md`
  — captured the templates preview design before this story's
  migration of the hljs layer.
- `meta/reviews/work/0076-code-block-syntax-highlight-palette-review-1.md`
  — two-pass review; final verdict ready for implementation, with
  Pass-2 observations carried forward for the plan-of-record.
- `meta/work/0037-glyph-component.md` — the resolved decision behind
  the Vitest/jsdom `getComputedStyle` limitation that bears directly on
  AC2/AC3 of 0076.

## Related Research

- `meta/research/codebase/2026-05-06-0033-design-token-system.md`
- `meta/research/codebase/2026-05-08-0034-theme-and-font-mode-toggles.md`
- `meta/research/codebase/2026-05-18-0042-templates-view-redesign.md`
- `meta/research/codebase/2026-05-15-0041-library-page-wrapper-and-overview-hub.md`
- `meta/research/codebase/2026-05-21-0078-detail-page-frontmatter-table.md`

## Open Questions

1. **AC2/AC3 testing strategy.** Vitest/jsdom cannot resolve
   `var(--tk-*)` cascade lookups, so `getComputedStyle(span).color
   === rgb(r, g, b)` is not viable inside the Vitest suite. Decide
   between: (a) restructure to a Playwright spec under
   `tests/visual-regression/`, mirroring `glyph-resolved-fill.spec.ts`,
   tolerating the extra-runtime cost; (b) restructure assertions to
   check that the CSS module text declares the right `var(--tk-*)` for
   each `.hljs-*` selector (regex on `?raw` CSS), which keeps the
   suite in Vitest at the cost of not testing the live cascade; or
   (c) inject `global.css` and the new layer into jsdom in
   `src/test/setup.ts` and verify whether modern jsdom resolves
   `var()` for `color` (it has improved since the Glyph decision —
   but the Glyph decision should be re-validated before relying on
   this).
2. **Directory naming.** Story body uses `src/design/...`
   throughout; the codebase uses `src/styles/`. Adopt
   `src/styles/code-syntax.global.css`,
   `src/styles/prototype-tokens.fixture.test.ts`, and
   `src/styles/__fixtures__/prototype-tokens.json` (or place the
   fixture sibling to `tokens.ts` without a `__fixtures__/` directory,
   since none currently exists in the frontend).
3. **ADR-0026 amendment scope.** Decide whether to (a) amend ADR-0026
   in place to remove `#1e1e1e`/`#d4d4d4` from the irreducible list
   and add a paragraph documenting the new `--code-*` family, or
   (b) supersede ADR-0026 with a successor that explicitly notes the
   change. Either way, `migration.test.ts` EXCEPTIONS must be updated
   in the same commit as the `MarkdownRenderer.module.css` token swap.
4. **github.css removal vs co-existence.** Once the new shared layer
   covers every hljs class the visualiser renders for the in-scope
   languages (yaml, markdown, python, typescript, json, css, html,
   diff), the `import 'highlight.js/styles/github.css'` at
   `src/main.tsx:7` can be removed entirely. Confirm the new layer's
   class coverage is exhaustive for those languages before deleting
   the import — otherwise unmapped classes fall back to the cascade
   default (inherit from `<pre>`/`<code>` — `--code-fg`), which is the
   intended fallback per the prototype but worth verifying.
5. **`hljs-template-variable` scope.** The class is currently emitted
   only by `template-highlight.tsx` for the templates preview; it is
   not produced by stock `highlight.js` for the markdown renderer.
   When the shared layer maps it to `--tk-var`, it remains effectively
   templates-preview-only. Confirm this is the intended scope and
   that no future markdown content will inadvertently match the class.
