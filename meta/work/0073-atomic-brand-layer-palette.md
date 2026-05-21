---
work_item_id: "0073"
title: "Atomic Brand-Layer Palette"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, tokens, foundation]
---

# 0073: Atomic Brand-Layer Palette

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Introduce the `--atomic-*` brand-layer palette declared in the prototype's
`assets/tokens.css:34-105` — roughly thirty named tokens — and rearchitect
the stylesheet so the brand palette feeds the existing `--ac-*` semantic
layer rather than hex literals being baked directly into the semantic layer.

## Context

The current `src/styles/global.css` declares only the semantic `--ac-*`
layer with no brand-named source-of-truth, so type-tinted iconography and
illustrative artwork have nowhere to pull from. 0033 (Design Token System)
explicitly excluded the `--atomic-*` brand palette from scope (see 0033 AC1:
"'Brand palette — `--atomic-*`' is excluded; raw brand colours not consumed
by components"). The new gap analysis identifies this exclusion as the
missing brand layer that downstream BigGlyph and illustrative work depends
on.

The prototype's brand palette includes named tokens such as
`--atomic-night`, `--atomic-ink`, `--atomic-indigo`, `--atomic-marigold`,
`--atomic-aquamarine`, `--atomic-cream-can`, plus the aliases
(`--atomic-violet`, `--atomic-teal`) and overlays
(`--atomic-overlay-ink`, `--atomic-stroke-light`, `--atomic-shadow-soft`).

## Requirements

- Introduce the full `--atomic-*` brand palette (~30 tokens including
  named colours, aliases, and overlays) in `src/styles/global.css` and
  its `tokens.ts` mirror.
- Rearchitect the stylesheet so the `--ac-*` semantic layer references
  `--atomic-*` brand tokens via `var()`, rather than baking hex literals
  into the semantic layer.
- Preserve resolved `--ac-*` values to keep visual output identical
  against the current app where mappings are clean.

## Acceptance Criteria

- [ ] All `--atomic-*` brand tokens enumerated in the prototype's
  `assets/tokens.css:34-105` are defined in `global.css` and mirrored in
  `tokens.ts`.
- [ ] The `--ac-*` semantic layer consumes `--atomic-*` tokens via
  `var()` for every colour whose brand mapping is clean; remaining
  literal usage is documented in the PR description.
- [ ] `global.test.ts` asserts CSS↔TS parity over the new brand layer.
- [ ] Visual diff against current app stays bounded by ΔE < 5 on any
  colour for routes captured in the design inventory.

## Open Questions

- Are the prototype's `--atomic-violet` / `--atomic-teal` aliases the
  exact same hue as any existing `--ac-*` semantic value, or do they
  introduce new brand variants?

## Dependencies

- Blocked by: 0033 (semantic token layer must exist before brand layer
  feeds it).
- Blocks: 0082 (BigGlyph illustrations depend on brand palette tokens).

## Assumptions

- The brand palette is theme-invariant; per-theme overrides remain at the
  `--ac-*` semantic layer.

## Technical Notes

- 0033's AC1 explicitly excluded this token set; this story closes that
  gap.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0033, 0082
