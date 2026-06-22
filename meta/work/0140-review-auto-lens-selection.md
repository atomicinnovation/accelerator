---
type: work-item
id: "0140"
title: "Auto-Lens Selection for Review Skills"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0140: Auto-Lens Selection for Review Skills

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add auto-lens selection to all review skills, supporting modes such as `all`,
`auto`, `manual`, `confirm`, and `list` for choosing which review lenses run.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Review skills currently apply lenses
without a unified selection mechanism.

## Requirements

- Add lens-selection modes to review skills: `all`, `auto`, `manual`, `confirm`,
  `list`.
- `auto` should select appropriate lenses based on what is being reviewed.

## Acceptance Criteria

- [ ] Review skills accept a lens-selection mode and behave accordingly:
      `all` (every lens), `auto` (inferred set), `manual` (user-specified),
      `confirm` (propose then confirm), `list` (enumerate available lenses).

## Open Questions

- How does `auto` infer the appropriate lens set?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.
- Related: effort control for review skills; simplicity/elegance review lenses.

## Assumptions

- Applies uniformly across the review skill family.

## Technical Notes

- Builds on the existing lens catalogue and reviewer-agent pattern.

## Drafting Notes

- Lens-selection modes captured verbatim from the source line.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
