---
work_item_id: "0076"
title: "Code-Block Syntax-Highlight Tokens and Renderer Adoption"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: done
priority: medium
parent: ""
tags: [design, frontend, tokens, markdown, code]
---

# 0076: Code-Block Syntax-Highlight Tokens and Renderer Adoption

**Type**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a reader of markdown documents in the visualiser, I want fenced code
blocks to render with a consistent, branded syntax-highlight palette so
that code is legible and visually coherent in both light and dark
themes. Introduce the prototype's self-contained `--code-*` and `--tk-*`
token set, expose them through `tokens.ts`, and adopt them in the
current `react-markdown` + `rehype-highlight` pipeline and the templates
preview pane without losing GFM, wiki-link routing, or hljs language
support.

## Context

The prototype defines a self-contained, theme-independent code-block
palette at `prototype-standalone.html` — code surface tokens
(`--code-bg #0E1320`, `--code-bg-head #161B2C`, `--code-stroke
rgba(255,255,255,0.07)`, `--code-fg #D7DCEC`, `--code-fg-faint
#6F7796`), token-class colours
(`--tk-com`, `--tk-str`, `--tk-num`, `--tk-kw`, `--tk-lit`, `--tk-typ`,
`--tk-fn`, `--tk-attr`, `--tk-deco`, `--tk-macro`, `--tk-var`,
`--tk-key`, `--tk-flag`, `--tk-heredoc`, `--tk-pun`, `--tk-lifet`,
`--tk-header`, `--tk-anchor`, `--tk-tag`, `--tk-doctype`, `--tk-bn`,
`--tk-prop`, `--tk-sel`, `--tk-atrule`), and diff tokens (`--tk-dhdr`,
`--tk-dhunk`, `--tk-dadd`, `--tk-ddel`). The same values render against
both light and dark page chrome in the prototype's
`doc-detail-plan-meta-visualisation-fullpage{,-dark}.png` screenshots —
the palette is intentionally theme-independent.

The current app uses `react-markdown` + `remark-gfm` + `remarkWikiLinks`
+ `rehype-highlight` and relies on hljs default class names with no
named token layer for syntax colours, leaving no surface for theming or
per-language overrides. The templates preview at
`skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css:153-213`
already maps hljs class names — but to generic accent/foreground tokens
(`--accent`, `--fg-strong`, `--fg-muted`), not a named code-syntax
palette. Its module comment ("reusable wherever hljs is rendered (this
view and future markdown views)") signals the intent to share, but the
mapping needs to be re-pointed at the new `--tk-*` tokens and lifted out
of the templates-preview module into a shared layer.

The prototype's renderer is hand-rolled and less capable on parser depth
and link routing; keep the current `react-markdown` renderer for parser
correctness and adopt only the prototype's syntax-highlight palette so
fenced code blocks and the templates preview render with the
prototype's visual treatment.

## Requirements

- Add the full prototype code surface and `--tk-*` token set to
  `global.css` and mirror them in `tokens.ts`. Values must match the
  prototype constants in `prototype-standalone.html` exactly.
- Apply the same values in both light and dark themes
  (theme-independent palette).
- Provide a single shared CSS layer that maps hljs class names onto the
  new `--tk-*` tokens, so the existing `rehype-highlight` pipeline
  produces the prototype's palette without swapping highlighters. The
  required mappings are enumerated in the table below; any `--tk-*`
  token not present in the table may ship without an active mapping
  (still defined in `global.css` and mirrored in `tokens.ts` for future
  consumers, but no selector in the shared layer).
- Migrate the templates preview at
  `LibraryTemplatesView.module.css` to consume the shared layer
  rather than its current local accent/fg-strong/fg-muted mapping —
  same hljs spans, same `--tk-*` tokens, single source of truth.
- Apply the shared layer in the markdown renderer so fenced code blocks
  in document detail views consume the same palette.

### Required hljs class → `--tk-*` mappings

| hljs class                         | `--tk-*` token | Notes / source                              |
|------------------------------------|----------------|---------------------------------------------|
| `hljs-comment`, `hljs-quote`       | `--tk-com`     | Standard hljs comment class                 |
| `hljs-string`                      | `--tk-str`     | Current templates-preview mapping; markdown |
| `hljs-number`                      | `--tk-num`     | Current templates-preview mapping; markdown |
| `hljs-keyword`                     | `--tk-kw`      | Standard hljs keyword                       |
| `hljs-literal`                     | `--tk-lit`     | Current templates-preview mapping (yaml)    |
| `hljs-type`, `hljs-class`          | `--tk-typ`     | Standard hljs type                          |
| `hljs-function`, `hljs-title.function_` | `--tk-fn`  | Standard hljs function                      |
| `hljs-attr`, `hljs-attribute`      | `--tk-attr`    | Current templates-preview mapping (yaml)    |
| `hljs-meta`                        | `--tk-deco`    | Decorator/metadata role                     |
| `hljs-built_in`                    | `--tk-bn`      | Built-in identifiers                        |
| `hljs-variable`, `hljs-template-variable` | `--tk-var` | Includes preview's custom class            |
| `hljs-property`                    | `--tk-prop`    | Property access                             |
| `hljs-selector-class`, `hljs-selector-id`, `hljs-selector-tag`, `hljs-selector-pseudo` | `--tk-sel` | CSS selectors |
| `hljs-tag`, `hljs-name`            | `--tk-tag`     | HTML/XML tag                                |
| `hljs-meta.doctype`                | `--tk-doctype` | Doctype declarations                        |
| `hljs-section`                     | `--tk-header`  | Markdown headings                           |
| `hljs-link`, `hljs-symbol`         | `--tk-anchor`  | Current templates-preview mapping; links    |
| `hljs-bullet`                      | `--tk-com`     | List markers — faint, comment-like          |
| `hljs-punctuation`                 | `--tk-pun`     | Standard hljs punctuation                   |
| `hljs-emphasis`                    | (inherit; `font-style: italic`) | Carry markdown emphasis styling |
| `hljs-strong`                      | (inherit; `font-weight: 600`)   | Carry markdown strong styling   |
| `hljs-addition`, `hljs-diff-added` | `--tk-dadd`    | Diff additions                              |
| `hljs-deletion`, `hljs-diff-deleted` | `--tk-ddel`  | Diff deletions                              |
| `hljs-meta` inside `language-diff` (file header lines) | `--tk-dhdr` | Diff file headers     |
| `hljs-comment` inside `language-diff` (hunk headers `@@`) | `--tk-dhunk` | Diff hunk headers |

Tokens defined but not actively mapped in the shared layer (no current
hljs class targets them across the languages the visualiser renders):
`--tk-macro`, `--tk-key`, `--tk-flag`, `--tk-heredoc`, `--tk-lifet`,
`--tk-atrule`. These ship as defined tokens for future consumers and
do not require a selector in the shared mapping layer; if a future
language brings an emitted hljs class that semantically matches one
of these, the mapping is added then.

## Acceptance Criteria

- [ ] `global.css` defines the full `--code-*` and `--tk-*` token set
  enumerated above. `tokens.ts` mirrors every name. `global.test.ts`
  asserts both name parity and value parity between `global.css`,
  `tokens.ts`, and a committed fixture
  `skills/visualisation/visualise/frontend/src/design/__fixtures__/prototype-tokens.json`
  which captures the `--code-*` / `--tk-*` constants extracted from
  `prototype-standalone.html`. The fixture is committed alongside the
  test; a sibling check (`prototype-tokens.fixture.test.ts`) reads
  `prototype-standalone.html` directly and asserts the fixture matches
  the prototype source, so prototype drift surfaces as a test failure.
- [ ] A shared stylesheet rule maps each hljs class listed in the
  Requirements mapping table to the corresponding `--tk-*` token. Given
  a fenced code block rendered through `react-markdown` +
  `rehype-highlight`, when the test inspects one span per row of the
  required-mappings table (excluding the diff rows, which AC3 covers),
  then `getComputedStyle(span).color` resolves to the canonical form of
  the matching prototype `--tk-*` value, in both light and dark themes.
  Canonical form: hex tokens convert to `rgb(r, g, b)` via a shared
  `hexToRgbString` helper; rgba tokens compare via `rgba(r, g, b, a)`
  with alpha to two decimal places; assertions use exact string match.
- [ ] Given a fenced code block with `language-diff`, when the test
  inspects spans for the four diff rows of the mapping table (file
  header, hunk header, addition, deletion), then their computed `color`
  values resolve to `--tk-dhdr`, `--tk-dhunk`, `--tk-dadd`, `--tk-ddel`
  respectively, using the same canonical-form comparison as AC2.
- [ ] The following behaviours each have at least one passing test
  after this story lands (extend existing suites or add new ones as
  needed):
  - A GFM task list (`- [x]` / `- [ ]`) renders interactive checkbox markup.
  - A GFM table renders `<table>` / `<thead>` / `<tbody>` structure.
  - A `[[0042]]` wiki-link in body prose routes through the
    visualiser's wiki-link resolver and produces a navigable anchor.
  - An auto-detected `python` fenced code block produces at least one
    `.hljs-keyword` span coloured by `--tk-kw`.
  - An auto-detected `typescript` fenced code block produces at least
    one `.hljs-keyword` span coloured by `--tk-kw`.
  - A fenced code block adjacent to a `[[0042]]` wiki-link in the same
    document renders both correctly (no regression at the boundary).
- [ ] The local `.previewBody :global(.hljs-*)` rules in
  `LibraryTemplatesView.module.css` (currently at lines 153-213) are
  removed; the templates preview consumes the shared layer instead. A
  test at `LibraryTemplatesView.test.tsx` (or a sibling file) renders
  the preview with a representative yaml-front-matter + markdown fixture
  and asserts, for each hljs class previously mapped locally in the
  preview (`hljs-attr`, `hljs-meta`, `hljs-string`, `hljs-number`,
  `hljs-literal`, `hljs-section`, `hljs-bullet`, `hljs-emphasis`,
  `hljs-strong`, `hljs-code`/`hljs-quote`, `hljs-link`/`hljs-symbol`,
  `hljs-template-variable`), that `getComputedStyle(span).color`
  resolves to the canonical form of the same `--tk-*` value the
  Requirements mapping table assigns to that class. A `grep` check or
  CSS-module inspection in the same test confirms no `.previewBody
  :global(.hljs-*)` rules remain in the module.

## Open Questions

(None outstanding — palette is theme-independent per prototype; see
Drafting Notes.)

## Dependencies

- Blocked by: 0033 (token infrastructure); 0075 (typography size-scale
  consumption — lands first so this story rebases onto the rationalised
  `<pre>` radius and code-block CSS selectors; if 0075 has not landed
  when implementation begins, this story takes ownership of the `<pre>`
  selector touch-ups and the sequence reverses).
- Blocks: 0042 (templates preview consumes the shared palette this
  story provides — 0042 must wait for the shared layer to exist);
  0089 (templates preview whitespace fix touches `LibraryTemplatesView.module.css`
  in the same hljs-rule region this story rewrites — 0089 rebases onto
  the post-migration CSS); 0083 (DevDesignSystem reference page
  showcases the `--tk-*` / `--code-*` primitives shipped here); 0088
  (markdown body width harmonisation consumes the unified markdown
  code-block surface).
- Test fixture dependency: AC1 requires
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
  to be readable from the test runner. The committed
  `prototype-tokens.json` fixture (see AC1) decouples the unit-test
  fast path from this external file; the sibling drift-detection test
  is the only consumer that needs the prototype path stable.

## Assumptions

- `rehype-highlight` continues to emit standard hljs class names that
  CSS rules can target.
- The hljs class names listed in the Requirements mapping table are the
  required set. `--tk-*` tokens not in that table (`--tk-macro`,
  `--tk-key`, `--tk-flag`, `--tk-heredoc`, `--tk-lifet`, `--tk-atrule`)
  ship as defined tokens in `global.css` and `tokens.ts` without an
  active selector in the shared mapping layer; future stories add
  selectors when a consumer brings an emitted hljs class for them.

## Technical Notes

- Authoritative palette source:
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`.
  The prototype uses its own `.tk-*` element classes (not hljs class
  names), so the hljs → `--tk-*` mapping table in Requirements is
  derived in this story rather than transcribed from the prototype.
- Visual ground truth (light vs dark, same code palette):
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/screenshots/doc-detail-plan-meta-visualisation-fullpage.png`
  and `…-fullpage-dark.png`.
- Existing templates-preview hljs mapping to migrate:
  `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css:153-213`.
- New artefacts introduced by this story:
  - `skills/visualisation/visualise/frontend/src/design/code-syntax.css`
    (or equivalent named module) — the shared hljs class-to-`--tk-*`
    layer, imported by the markdown renderer and the templates preview.
  - `skills/visualisation/visualise/frontend/src/design/__fixtures__/prototype-tokens.json`
    — committed JSON snapshot of the prototype's `--code-*` / `--tk-*`
    constants, consumed by `global.test.ts`.
  - `skills/visualisation/visualise/frontend/src/design/prototype-tokens.fixture.test.ts`
    — drift-detection test that reads `prototype-standalone.html` and
    asserts the fixture matches.
  - Shared `hexToRgbString` helper used by the computed-style
    assertions in AC2/AC3/AC5.
- Out of scope: the per-template variable wrapper in
  `skills/visualisation/visualise/frontend/src/routes/library/template-highlight.tsx`
  (which post-processes hljs output to wrap `{{template variables}}`)
  is structurally orthogonal to the palette work; it continues to use
  its existing `hljs-template-variable` class, which the new mapping
  colours via `--tk-var`.
- 0033's AC1 enumerated in-scope tokens; `--code-*` / `--tk-*` were not
  in that set, so this story closes that gap.

## Drafting Notes

- Palette is theme-independent — same `--code-*` and `--tk-*` values
  resolve in both light and dark modes, matching the prototype's
  approach as evidenced by the fullpage screenshots. Accessibility
  tuning (e.g. raising contrast in light mode) can be revisited later
  via standard custom-property overrides without re-architecting the
  token layer; not in scope here.
- Token set expanded beyond the original extraction. The first draft
  enumerated only a subset; the prototype actually ships additional
  code-surface tokens (`--code-bg-head`, `--code-stroke`,
  `--code-fg-faint`) and additional `--tk-*` tokens (`--tk-deco`,
  `--tk-macro`, `--tk-flag`, `--tk-heredoc`, `--tk-pun`, `--tk-lifet`,
  `--tk-header`, `--tk-anchor`, `--tk-doctype`, `--tk-bn`, `--tk-atrule`).
  All ship together; partial enumeration would leave the token layer
  inconsistent with the prototype.
- hljs → `--tk-*` mapping table is derived, not transcribed. The
  prototype uses its own `.tk-*` element classes, so there is no
  authoritative hljs mapping in the prototype source. The Requirements
  table combines today's templates-preview coverage with the standard
  hljs classes a markdown renderer with `rehype-highlight` will emit
  for the languages the visualiser shows (yaml, markdown, python,
  typescript, json, css, html, diff). Mapping choices align hljs's
  semantic class names to the prototype's semantic `--tk-*` tokens
  (`hljs-comment → --tk-com`, etc).
- Shared mapping layer rather than per-view duplication. The templates
  preview already has a hljs mapping but it targets generic accent/fg
  tokens, not the named code-syntax palette. The work re-points that
  mapping at the new `--tk-*` tokens and lifts it out of the
  templates-preview CSS module so the markdown renderer and the
  templates preview share one source of truth.
- Diff tokens kept in this story rather than split out — the four
  `--tk-d*` tokens are part of the same prototype palette block and
  flow through the same hljs class-mapping mechanism; splitting them
  would add coordination overhead without a clean seam.
- Renderer choice carried over from prior research: keep the existing
  `react-markdown` + `remark-gfm` + `remarkWikiLinks` + `rehype-highlight`
  pipeline. The prototype's hand-rolled renderer loses GFM and
  wiki-link routing; only the palette is borrowed.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Prototype tokens: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
- Visual reference (light/dark): `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/screenshots/doc-detail-plan-meta-visualisation-fullpage.png`, `…-fullpage-dark.png`
- Related: 0033, 0042, 0075, 0083, 0088, 0089
