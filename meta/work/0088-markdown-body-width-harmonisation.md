---
work_item_id: "0088"
title: "Markdown Body Width Harmonisation"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: low
parent: ""
tags: [design, frontend, markdown, tokens]
---

# 0088: Markdown Body Width Harmonisation

**Type**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Settle on one canonical max-width cap (px or `ch`) and one canonical
body-text size for the markdown body, then apply it across every
markdown-rendering surface.

## Context

The current app caps the markdown body at a hard-coded
`max-width: 720px` in `MarkdownRenderer.module.css:2` (off the
`--ac-content-max-width-narrow` 600px scale). The prototype caps at
`max-width: 72ch` on `.ac-md` with a 14.5px / line-height 1.65 body. The
two caps produce different visual reading widths in practice.

## Requirements

- Decide between:
  1. **Pixel cap** — `max-width: 720px` (current) or another px value
     aligned to the design-token scale.
  2. **Character cap** — `max-width: 72ch` (prototype).
- Decide one canonical body-text size (current: inherits `--size-sm`;
  prototype: `14.5px` hard-coded).
- Apply both decisions uniformly to every markdown-rendering surface
  (detail-page body, templates preview, any other consumer).

## Acceptance Criteria

- [ ] Decision recorded on max-width cap and body-text size with
  rationale.
- [ ] Every markdown-rendering surface consumes the chosen values via
  CSS variables (no per-component literals).
- [ ] Visual-regression baselines captured for the body width and text
  size on at least one detail-page route and the templates preview.

## Open Questions

- Does the team prefer the px-based "reliable visual width" or the
  ch-based "reading-length-aware" model?
- Should the body size align with `--size-sm` or a dedicated
  `--size-body` token?

## Dependencies

- Blocked by: 0033 (size and width tokens), 0075 (size-scale
  consumption rule).
- Related: 0076 (code-block palette ships on the same markdown surface).
- Blocks: none.

## Assumptions

- The same width and size apply to every markdown surface; no
  per-surface carve-outs (e.g. templates preview vs detail page).

## Technical Notes

- `MarkdownRenderer.module.css:2` is the current hard-coded cap.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0033, 0075, 0076
