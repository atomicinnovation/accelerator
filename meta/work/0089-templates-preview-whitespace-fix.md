---
work_item_id: "0089"
title: "Templates Preview Body White-Space Fix"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
kind: bug
status: draft
priority: low
parent: ""
tags: [design, frontend, templates, bug]
---

# 0089: Templates Preview Body White-Space Fix

**Kind**: Bug
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

The templates preview pane renders with `white-space: normal`, which
collapses internal whitespace despite the Fira Code monospace face — an
obvious styling oversight in a code-preview surface. Set
`white-space: pre` (or `pre-wrap`) on the preview body so template
content renders verbatim.

## Context

The current app's template preview pane uses Fira Code 12 with
`white-space: normal`, which collapses internal whitespace despite the
monospace face. The prototype's templates list reuses
`HighlightedCode` and preserves whitespace via per-line wrappers.
0042 redesigns the templates view but does not explicitly address the
`white-space` setting on the preview body; this bug captures the fix.

## Requirements

- Set `white-space: pre` (or `pre-wrap`) on the templates preview pane
  so newlines, indentation, and internal whitespace render verbatim.
- Confirm the fix lands compatible with 0042's two-column detail layout
  and the prototype's per-line wrappers (if 0042 adopts that pattern).

## Acceptance Criteria

- [ ] The templates preview pane renders multi-line template content
  with all newlines and indentation preserved.
- [ ] Visual rendering matches the prototype's code-style preview
  (mono face + preserved whitespace).
- [ ] No regression on existing single-line templates.

## Open Questions

- `pre` vs `pre-wrap` — does the team want long lines to overflow
  horizontally (pre) or wrap (pre-wrap)?

## Dependencies

- Related: 0042 (Templates View Redesign — landing order matters; if
  0042 lands first it may incorporate this fix, in which case this
  becomes a no-op verification).
- Blocks: none.

## Assumptions

- The fix is a CSS one-liner; no markup or component changes required.

## Technical Notes

- This is small enough to fold into 0042's PR if both land together.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0042
