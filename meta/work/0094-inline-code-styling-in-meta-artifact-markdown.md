---
type: work-item
id: "0094"
title: "Inline Code Styling In Meta Artifact Markdown"
date: "2026-06-02T12:11:27+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: bug
priority: medium
parent: ""
external_id: ""
tags: [visualiser, markdown, rendering, bug]
last_updated: "2026-06-02T12:11:27+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0094: Inline Code Styling In Meta Artifact Markdown

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The visualiser's markdown renderer for meta artifacts renders inline code
spans (`` `like this` ``), but not with the correct styling. The current
treatment does not match the latest design prototype, leaving inline code
visually inconsistent with the intended design system.

## Context

Inline code is emitted by the renderer, so this is a styling gap rather
than a parsing failure. The latest prototype defines the intended inline
code treatment; the live visualiser diverges from it.

## Requirements

- Inline code spans render with styling matching the latest prototype
  (monospace face plus the prototype's inline code treatment), distinct
  from surrounding prose and from fenced code blocks.
- The styling consumes theme tokens so it renders correctly in both
  light and dark mode.

## Acceptance Criteria

- [ ] Inline code in an artifact body renders with styling matching the
  prototype's inline `<code>` treatment.
- [ ] Inline code remains visually distinct from, and does not regress,
  fenced code block rendering.
- [ ] Renders correctly in both light and dark mode.

## Open Questions

- Which specific properties diverge from the prototype (background, padding,
  border-radius, colour, font), and are the corresponding design tokens
  already available?

## Dependencies

- Related: 0076 (code-block syntax highlight palette), 0088 (markdown body
  width harmonisation).

## Drafting Notes

- Captured as a stub without interactive enrichment. Acceptance criteria
  and priority may need refinement before promoting from `draft` to
  `ready`.
- Reframed from "not rendered" to "rendered with incorrect styling" per
  author clarification: inline code is emitted, but the styling does not
  match the latest prototype.

## References

- Source: `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-standalone.html`
- Related: 0076, 0088
