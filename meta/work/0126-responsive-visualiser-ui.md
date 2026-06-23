---
type: work-item
id: "0126"
title: "Responsive Visualiser UI for Larger Screens"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: epic
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-147
---

# 0126: Responsive Visualiser UI for Larger Screens

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Make the visualiser UI responsive and better suited to larger screens, so the
layout adapts gracefully across viewport sizes rather than targeting a single
fixed width.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The visualiser frontend currently
targets a narrower layout; this epic groups the work needed to make it scale up
to wide and large displays.

## Requirements

- Audit the current layout for fixed widths and breakpoints that prevent the UI
  scaling to larger screens.
- Introduce responsive behaviour so content reflows / makes use of available
  width on large displays.

## Acceptance Criteria

- [ ] The visualiser layout adapts across small, medium, and large viewports
      without horizontal overflow or wasted whitespace.

## Open Questions

- Which screen sizes / breakpoints are the primary targets?
- Are there specific views (lists, detail pages, pipeline) that need bespoke
  large-screen layouts versus a global responsive grid?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- "Larger screens" refers to wide desktop / external monitors rather than a
  specific device class.

## Technical Notes

- Likely touches the frontend layout primitives and design-token spacing scale.

## Drafting Notes

- Treated as an epic per the user's instruction; child stories (per-view
  responsive work) to be decomposed during refinement.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
