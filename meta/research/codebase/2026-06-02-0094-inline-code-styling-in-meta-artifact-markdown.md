---
type: codebase-research
id: "2026-06-02-0094-inline-code-styling-in-meta-artifact-markdown"
title: "Research: Inline code styling in meta artifact markdown (0094)"
date: "2026-06-02T14:30:26+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0094"
topic: "Inline code styling in the visualiser's meta-artifact markdown renderer"
tags: [research, codebase, visualiser, markdown, css, design-tokens, inline-code]
revision: "fd91646a8309247cdd84fd5654582f81fe5d8a52"
repository: "ticket-management"
last_updated: "2026-06-02T14:30:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Inline code styling in meta artifact markdown (0094)

**Date**: 2026-06-02T14:30:26+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: fd91646a8309247cdd84fd5654582f81fe5d8a52
**Branch**: anonymous working-copy change `xlqvmuqpzroz` (no bookmark)
**Repository**: ticket-management

## Research Question

Verify and deepen the implementation context for work item 0094 — inline
`` `code` `` spans in the visualiser's meta-artifact markdown renderer render in
the prose body font (Inter) instead of the monospace face, and otherwise
diverge from the frozen design prototype's `.ac-md-code` pill. Confirm the
live code locations, the exact divergence, the token availability, and surface
any constraints or nuances the work item did not capture.

## Summary

The work item's diagnosis is **substantially correct and the fix is CSS-only**,
but the research surfaces several precise corrections and one genuine
architectural tension worth resolving before implementation:

1. **The live inline-code rule declares only four properties**, not the larger
   set the Context prose implies. `.markdown code:not(pre code)` (lines 57-60)
   sets `background`, `border-radius`, `padding`, and `font-size` — and
   **nothing else**. It does *not* set `font-family`, `color`, or `border`.
   The work item's framing ("already match on background and text colour") is
   accurate only because both the live rule and the prototype *inherit* colour;
   neither declares it. The headline defect (inherited Inter font) is confirmed.

2. **There is no `.ac-md-code` class in the live app, and there cannot be one
   without a component change.** The prototype styles inline code via a class
   (`.ac-md-code`) on `<code>`; the live renderer applies **no class** to inline
   code (react-markdown emits a bare `<code>` with no component override). The
   live fix must therefore be made on the **descendant selector**
   `.markdown code:not(pre code)`, not by porting the prototype's class. This is
   the single most important architectural fact for implementation.

3. **The table-cell override has no live equivalent and needs a new selector.**
   The prototype scopes table-cell code via `.ac-md-table tbody td code.ac-md-code`.
   The live app has zero table-plus-code rules, so the 11px requirement needs a
   brand-new selector such as `.markdown td code, .markdown th code`. Note the
   prototype scopes **only `tbody td`** — header cells (`th`) are *not* covered
   by the prototype's 11px rule. The work item's AC says "inside a table cell"
   without distinguishing — a decision point (see Open Questions).

4. **All tokens the work item relies on exist and match exactly**, in both
   themes. No `global.css` edit is required, so the MIRROR-A/MIRROR-B dark-theme
   parity test is not engaged by this work.

5. **Genuine tension: the prescribed literal values (`11.5px`, `11px`, `3px`,
   `1px 5px`, `1px solid`) sit against ADR-0026 (token application conventions)
   and ADR-0036 (typography font-size consumption rule).** The requirement text
   says "all values consume theme tokens", yet the prototype values it mandates
   do not map onto existing tokens (`3px` ≠ `--radius-sm`=4px; `11.5px` ≠
   `--size-xs`=14px). This is an internal inconsistency that should be reconciled
   against the ADRs before implementation (see Architecture Insights).

## Detailed Findings

### Live inline-code CSS rule (the thing being fixed)

File: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:57-60`

```css
.markdown code:not(pre code) {
  background: var(--ac-bg-sunken); border-radius: var(--radius-sm);
  padding: 0.1rem var(--sp-1); font-size: var(--size-xs);
}
```

- Selector and line range (57-60) match the work item exactly.
- Declared properties: `background` (`--ac-bg-sunken`), `border-radius`
  (`--radius-sm` = 4px), `padding` (`0.1rem var(--sp-1)` — the vertical value
  `0.1rem` is a **hard-coded literal**, only the horizontal uses `--sp-1`=4px),
  `font-size` (`--size-xs` = 14px).
- **Not declared**: `font-family` (→ inherits Inter, the defect), `color`
  (→ inherits, matches prototype intent), `border` (→ none, a divergence).
- The `:not(pre code)` is a pure negation guard. There is **no positive
  `pre code` rule** anywhere in the file, so fenced-block `<code>` receives only
  `.markdown pre` styling. Correcting the inline rule cannot regress fenced
  blocks — the scoping is genuinely safe.

Fenced-block rules that must remain untouched:
`.markdown pre` (lines 19-27, `background: var(--code-bg)`, `font-size:
var(--size-xs)`), `.codeblock pre` (lines 40-44), and the language-label chrome
classes `.codeblock` / `.codeblockHead` / `.codeblockLang` (lines 33-56). None
selects an inline `<code>`.

Table rules present (no code scoping): `.markdown table` (61),
`.markdown th, .markdown td` (62), `.markdown th` (63).

### Renderer component — confirms CSS-only, no inline-code class

File: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx`

- Library: `react-markdown` (line 2) with `remark-gfm` (line 3, always applied,
  lines 69-75/79) and `rehype-highlight` (line 80).
- `MARKDOWN_COMPONENTS` (lines 29-44) overrides **only `pre`**. There is no
  `code`, `table`, `td`, or `th` key. The work item's "lines 29-44" is the
  entire map, not just the `pre` override — minor citation nuance.
- Inline `code` falls through to react-markdown's default → a **bare `<code>`
  with no className**. Confirmed: the fix is CSS-only, and it must be expressed
  through the `.markdown ...` descendant selector (no class hook exists).
- Root container is `<div className={styles.markdown}>` (line 77); `.markdown`
  is defined at `MarkdownRenderer.module.css:4-10`. All descendant styling is
  scoped under `.markdown`.
- The `pre` override does **not** highlight — `rehypeHighlight` annotates the
  inner `<code class="language-…">`; the override only wraps fenced blocks with
  a language band when a language is detected (`fenceLanguageOf`, lines 14-22).
- Tables: GFM tables ARE emitted (default elements). A table-cell code fix has
  no JS in its way — a `.markdown td code, .markdown th code` rule is the
  natural mechanism.

### Design tokens — all present, both themes, exact matches

File: `skills/visualisation/visualise/frontend/src/styles/global.css`

| Token | Line(s) | Light | Dark | Theme-varying? |
|---|---|---|---|---|
| `--ac-font-mono` | 155 | `"Fira Code", ui-monospace, monospace` | same | No |
| `--ac-bg-sunken` | 82 / 333 / 406 | `#f4f6fa` | `#070b12` | **Yes** |
| `--ac-stroke-soft` | 93 / 344 / 417 | `rgba(32, 34, 49, 0.06)` | `rgba(255, 255, 255, 0.04)` | **Yes** |
| `--radius-sm` | 209 | `4px` | same | No |
| `--size-xs` | 179 | `14px` | same | No |
| `--sp-1` | 196 | `4px` | same | No |
| `--ac-font-body` | 154 | `"Inter", system-ui, sans-serif` | same | No* |

All values match the work item's assertions exactly (modulo hex case and
whitespace). The two theme-varying tokens this fix consumes
(`--ac-bg-sunken`, `--ac-stroke-soft`) are already defined in both dark blocks,
so **no `global.css` edit is needed** and the MIRROR-A (`[data-theme="dark"]`,
line 330) / MIRROR-B (`@media prefers-color-scheme: dark`, lines 401-402) byte-
parity test enforced by `global.test.ts` is not engaged.

*Theming mechanism: `[data-theme="dark"]` forces dark (canonical source of
truth); otherwise OS `prefers-color-scheme: dark` applies unless
`data-theme="light"` is set. The toggle UI is pending (work item 0034).

\* Edge case: `[data-font="mono"]` (line 471) repoints `--ac-font-body` to
`--ac-font-mono`. In mono-font mode the prose body is *already* Fira Code, so
inline code styled to mono would lose its font-family distinction (both
monospace) — though the border / sunken background / smaller size still
distinguish it. Not a blocker; worth noting.

### Frozen prototype — the authoritative target

File: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
(entire stylesheet minified onto line 183)

Base rule:
```css
.ac-md-code { font-family: var(--ac-font-mono); font-size: 11.5px; background: var(--ac-bg-sunken); padding: 1px 5px; border-radius: 3px; border: 1px solid var(--ac-stroke-soft); }
```

Table-cell override:
```css
.ac-md-table tbody td code.ac-md-code { font-size: 11px; }
```

- No `color` declaration (inherits) — matches the live rule's behaviour.
- The string `ac-md-code` appears **only inside the `<style>` block**, never as
  a literal `class="ac-md-code"` in markup. The prototype ships the *styling
  contract*, not pre-rendered markup. The `code.ac-md-code` selector is the only
  evidence that `.ac-md-code` is intended for `<code>` elements.
- Prototype token values for `--ac-font-mono`, `--ac-bg-sunken`,
  `--ac-stroke-soft` match the live `global.css` values exactly. The prototype
  is internally consistent with the live token system for everything this fix
  touches.
- **The table override scopes `tbody td` only** — `th` cells are excluded from
  the 11px rule in the prototype.

## Code References

- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:57-60` — the live inline-code rule to be amended (the target of this work).
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:19-27` — `.markdown pre` fenced-block rule (must stay unchanged).
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css:61-63` — table / cell rules (where a new `td code, th code` selector would sit).
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:29-44` — `MARKDOWN_COMPONENTS`; only `pre` overridden, no inline-code component.
- `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx:77-81` — root `.markdown` container + plugin wiring (remark-gfm, rehype-highlight).
- `skills/visualisation/visualise/frontend/src/styles/global.css:82,93,155,179,196,209` — light-theme token definitions consumed by the fix.
- `skills/visualisation/visualise/frontend/src/styles/global.css:333,344 / 406,417` — dark-theme `--ac-bg-sunken` / `--ac-stroke-soft` (MIRROR-A / MIRROR-B).
- `skills/visualisation/visualise/frontend/src/styles/global.css:471` — `[data-font="mono"]` repoint of `--ac-font-body` (edge case).
- `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html:183` — `.ac-md-code` base + `tbody td` override (authoritative target values).

## Architecture Insights

- **Class-based prototype vs descendant-selector live app.** The prototype's
  design contract is expressed as classes (`.ac-md-code`, `.ac-md-table`, …)
  applied to runtime-rendered markdown. The live renderer does not apply these
  classes; it relies on `.markdown <element>` descendant selectors. Porting
  prototype styling means **translating class rules into descendant rules**, not
  copying selectors. This is the consistent pattern across the renderer.

- **Token-literal tension with ADR-0026 and ADR-0036.** The work item's
  Requirements state "All values consume theme tokens", but the values it then
  mandates from the prototype are **hard-coded literals that do not map to
  existing tokens**: `border-radius: 3px` (vs `--radius-sm`=4px),
  `font-size: 11.5px` / `11px` (vs `--size-xs`=14px), `padding: 1px 5px`
  (vs the current `0.1rem var(--sp-1)`), `border: 1px solid` width. Only the
  *colour* inputs (`--ac-bg-sunken`, `--ac-stroke-soft`) and `--ac-font-mono`
  are tokenised.
  - **ADR-0036 (typography font-size consumption rule)** governs how font sizes
    are consumed. A literal `11.5px`/`11px` may violate it; this should be
    checked, because 0075's plan included a rem-vs-px stance review and a size
    scale exists. If ADR-0036 forbids off-scale literals, the work item's
    explicit author decision ("font-size target set to `11.5px`") collides with
    it and needs an exception or a new token.
  - **ADR-0026 (CSS design-token application conventions)** / **ADR-0035
    (brand-layer indirection)** govern literals like `3px`, `1px 5px`,
    `1px solid`. The fix should either introduce tokens or document why
    prototype-literal fidelity overrides the convention here.
  - The contradiction inside the work item ("consume theme tokens" + prescribe
    untokenised literals) should be resolved before/within planning.

- **Dark-mode safety is free.** Because the fix only *consumes* already-themed
  tokens for its colour inputs and uses literals for everything else, dark mode
  works automatically and no MIRROR parity edit is needed.

- **Fenced-block isolation is structurally guaranteed**, not merely
  conventional: there is no `pre code` rule to collide with, and the `pre`
  component override is independent.

## Historical Context

- `meta/work/0076-code-block-syntax-highlight-palette.md` + its plan/research
  (`meta/plans/2026-05-21-0076-…`, `meta/research/codebase/2026-05-21-0076-…`) —
  the precedent for tokenising code surfaces and adopting tokens in this exact
  renderer; the closest pattern to follow for any new tokens 0094 might need.
- `meta/work/0088-markdown-body-width-harmonisation.md` + plan/research — prior
  surgical change to the same MarkdownRenderer CSS module.
- `meta/work/0089-templates-preview-whitespace-fix.md` — adjacent Fira Code /
  monospace surface work.
- `meta/work/0095-markdown-checkboxes-always-dark-mode-styled.md` — sibling
  theme-token bug in the same renderer (the "always dark" failure mode is a
  cautionary analogue: ensure the inline-code fix resolves correctly in *both*
  themes, not just dark).
- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` — token
  application conventions (directly relevant to the literal-vs-token tension).
- `meta/decisions/ADR-0036-typography-font-size-consumption-rule.md` — font-size
  consumption rule (directly relevant to the `11.5px`/`11px` decision).
- `meta/decisions/ADR-0035-brand-layer-indirection-supplement-to-adr-0026.md` —
  brand-layer indirection supplement.
- `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
  and its plan — the rem-vs-px / size-scale stance most likely to constrain
  the font-size literal.
- `meta/research/codebase/2026-05-06-0033-design-token-system.md`,
  `2026-04-17-meta-visualiser-implementation-context.md` — foundational token
  system and renderer architecture context.

## Related Research

- `meta/research/codebase/2026-05-21-0076-code-block-syntax-highlight-palette.md`
- `meta/research/codebase/2026-05-26-0088-markdown-body-width-harmonisation.md`
- `meta/research/codebase/2026-05-23-0075-typography-size-scale-consumption.md`
- `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`

## Open Questions

1. **Table header cells (`th`).** The prototype's 11px override scopes only
   `tbody td`. The work item AC says "inside a table cell → 11px" without
   distinguishing `td` from `th`. Should the live fix match the prototype
   exactly (`td` only) or cover both `td` and `th`? Recommend matching the
   prototype (`td` only) unless the author wants header-cell parity.
2. **Literal values vs ADR-0026 / ADR-0036.** Should `11.5px` / `11px` / `3px` /
   `1px 5px` be introduced as tokens (per the conventions), or accepted as
   prototype-fidelity literals with a documented exception? The work item's
   "consume theme tokens" requirement and its prescribed literals are in
   tension. This is the main thing to resolve in planning.
3. **`[data-font="mono"]` interaction.** In mono-font mode the prose body is
   already Fira Code, collapsing the font-family distinction for inline code.
   Acceptable (border + background + size still distinguish), but confirm it is
   not considered a regression.
4. **Vertical padding.** The current rule's hard-coded `0.1rem` vertical padding
   is being replaced by the prototype's `1px 5px`. Confirm `1px` vertical is
   intended (it is slightly tighter than `0.1rem` ≈ 1.6px).
