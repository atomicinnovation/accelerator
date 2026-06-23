---
type: work-item
id: "0147"
title: "Reuse /update-work-item Status Logic in Kanban Board"
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
external_id: PP-168
---

# 0147: Reuse /update-work-item Status Logic in Kanban Board

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Use the same logic as `/update-work-item N status <status>` when changing a work
item's status from the visualiser's kanban board, so status changes behave
consistently regardless of entry point.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). The kanban board supports
drag-and-drop status changes; this should route through the same status-update
logic the skill uses.

## Requirements

- Make kanban-board status changes apply the same logic/side-effects as
  `/update-work-item N status <status>`.

## Acceptance Criteria

- [ ] Given a work item moved between columns on the kanban board, when the
      status changes, then the same logic as `/update-work-item N status` is
      applied (same validation, frontmatter updates, and side-effects).

## Open Questions

- Where does the shared logic live so both the skill and the board can call it
  (shared script vs. server endpoint)?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- The kanban board already performs status changes via drag-and-drop.

## Technical Notes

- Relates to the kanban drag-and-drop feature and the update-work-item skill.

## Drafting Notes

- None beyond faithful restatement of the source line.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
