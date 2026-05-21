---
work_item_id: "0086"
title: "Kanban Drag-and-Drop with Toast Confirmations"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: story
status: draft
priority: medium
parent: ""
tags: [design, frontend, kanban]
---

# 0086: Kanban Drag-and-Drop with Toast Confirmations

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add HTML5 drag-and-drop to the kanban board so cards can be moved
between columns, with status write-back to disk and Toaster
confirmations on each successful move. Column-set configuration stays
dynamic per ADR-0024; this story owns only the drag-and-drop +
write-back + toast loop.

## Context

The current app's `KanbanBoard` renders cards in columns but does not
support drag-and-drop status mutation. The prototype's `KanbanBoard`
exposes HTML5 drag-and-drop, status write-back, and toast confirmations.
The column-set question (Draft/Ready vs Todo) is owned by 0044's spike,
which confirmed ADR-0024 mandates configurable columns — this story
does not change column semantics, only the drag-and-drop loop.

## Requirements

- Implement HTML5 drag-and-drop on kanban cards using
  `dragstart`/`dragover`/`drop` events (no third-party library).
- On drop, write the new status value back to the work-item file on
  disk via a server endpoint (PATCH on the work-item or a dedicated
  status-mutation route).
- On successful write-back, emit a `Toaster` (0039) confirming the
  move with the card title and new column.
- On failure, emit a `Toaster` error variant and revert the card to its
  original column without losing optimistic UI state.

## Acceptance Criteria

- [ ] A card can be dragged from any column to any other column on the
  kanban board.
- [ ] On drop, a server request fires that updates the work-item's
  `status` frontmatter field on disk.
- [ ] On successful response, a Toaster appears with the card title and
  the new column name.
- [ ] On error response, a Toaster appears with an error message and
  the card returns to its original column.
- [ ] The drag-and-drop loop respects ADR-0024's configurable column
  set — drops onto any configured column work regardless of column
  count or labels.

## Open Questions

- What server endpoint owns work-item status mutation, and does it
  already exist?
- Should the drag interaction support keyboard accessibility (e.g.
  arrow-key reordering with screen-reader announcements) in this story
  or as a follow-up?

## Dependencies

- Blocked by: 0039 (Toaster), 0044 (spike resolves column-set
  scope).
- Blocks: none.

## Assumptions

- A status-mutation endpoint exists or can be added trivially; the
  server already serves work-item documents.
- ADR-0024's configurable column set is the canonical column model.

## Technical Notes

- HTML5 drag-and-drop has well-known accessibility limitations;
  keyboard accessibility may need a follow-up.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0039, 0044
