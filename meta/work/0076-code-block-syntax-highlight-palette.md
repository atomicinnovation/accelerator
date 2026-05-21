---
work_item_id: "0076"
title: "Code-Block Syntax-Highlight Tokens and Renderer Adoption"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, tokens, markdown, code]
---

# 0076: Code-Block Syntax-Highlight Tokens and Renderer Adoption

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Introduce the prototype's self-contained `--code-*` and `--tk-*` token
set so fenced code blocks and the templates preview share a
theme-independent palette across light and dark modes, and adopt that
palette in the current `react-markdown` + `rehype-highlight` pipeline
without losing GFM, wiki-link routing, or hljs language support.

## Context

The prototype declares a self-contained dark code-block palette at
`src/app.css:766-800` — `--code-bg #0E1320`, `--code-fg #D7DCEC`, plus
token-class colours `--tk-com`, `--tk-str`, `--tk-num`, `--tk-kw`,
`--tk-lit`, `--tk-typ`, `--tk-fn`, `--tk-attr`, `--tk-var`, `--tk-key`,
`--tk-tag`, `--tk-prop`, `--tk-sel`, and diff tokens `--tk-dhdr`,
`--tk-dhunk`, `--tk-dadd`, `--tk-ddel`.

The current app uses `react-markdown` + `remark-gfm` + `remarkWikiLinks`
+ `rehype-highlight` and relies on hljs default class names with no
named token layer for syntax colours, leaving no surface for theming or
per-language overrides. The prototype's renderer is hand-rolled and less
capable on parser depth and link routing; keep the current renderer for
parser correctness and adopt only the prototype's syntax-highlight
palette so fenced code blocks render with the prototype's visual
treatment.

## Requirements

- Add `--code-bg`, `--code-fg`, the full `--tk-*` token set, and the
  diff tokens to `global.css` and `tokens.ts`.
- Map hljs class names (e.g. `hljs-comment`, `hljs-string`, …) onto the
  new `--tk-*` tokens via stylesheet rules, so the existing
  `rehype-highlight` pipeline produces the prototype's palette without
  swapping highlighters.
- Apply the same palette to the templates preview pane (0042's preview).

## Acceptance Criteria

- [ ] `global.css` defines `--code-*` and `--tk-*` tokens; `tokens.ts`
  mirrors them; `global.test.ts` asserts parity.
- [ ] A stylesheet rule maps each hljs class to the corresponding
  `--tk-*` token; fenced code blocks render in the prototype palette
  in both light and dark themes.
- [ ] Diff hunks rendered through hljs's diff language consume the diff
  tokens (`--tk-dhdr`, `--tk-dhunk`, `--tk-dadd`, `--tk-ddel`).
- [ ] GFM (task lists, tables), wiki-link routing, and hljs language
  auto-detection continue to work after the change.
- [ ] Templates preview pane consumes the same palette.

## Open Questions

- Should the code-block palette stay theme-independent (prototype model)
  or expose theme-specific overrides for accessibility tuning?

## Dependencies

- Blocked by: 0033 (token infrastructure).
- Related: 0042 (templates preview consumes the palette).
- Blocks: 0088 (markdown body width harmonisation may want to land
  together for a unified markdown surface).

## Assumptions

- `rehype-highlight` continues to emit standard hljs class names that
  CSS rules can target.

## Technical Notes

- 0033's AC1 enumerated in-scope tokens; `--code-*` / `--tk-*` were not
  in that set, so this story closes that gap.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0033, 0042, 0088
