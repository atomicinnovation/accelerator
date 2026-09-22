---
type: "work-item"
id: "0291"
title: "Parent-Child Relationship Synchronisation for Work Item Sync"
date: "2026-09-20T19:15:54+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0229", "work-item:0290"]
tags: ["sync", "tracker", "jira", "linear", "hierarchy", "parent-child", "relationships"]
last_updated: "2026-09-20T19:15:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-877"
---

# 0291: Parent-Child Relationship Synchronisation for Work Item Sync

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Establish and maintain work-item parent-child relationships across sync between
local items and remote trackers (Jira, Linear). Push and pull the single parent
edge, ordering the push so every parent exists remotely before its children, with
last-writer-wins on divergence. Closes epic 0146's parent-child acceptance
criterion.

## Context

Local items carry a single parent (`parent: "work-item:NNNN"`). The sync engine
currently crosses only `body` and `external_id`; hierarchy edges never reach the
port. Jira models hierarchy natively (Epic > Story/Task/Bug > Subtask via a
level-constrained parent field); Linear models it via `parentId` with arbitrary
nesting. Both expose a single parent pointer, matching the local model.

## Requirements

- Extend the `RemoteTracker` port to carry the parent reference (the remote
  parent's key/UUID) on read and write.
- Push: set the remote parent from the local edge, mapping local id → `external_id`.
- Pull: set the local `parent` from the remote parent, mapping `external_id` → local
  id.
- Order the push topologically so every parent is created before its children are
  linked; a child whose parent lacks an `external_id` is linked once the parent is
  created in the same run.
- Detect cycles in the local hierarchy and report them rather than looping.
- Divergence (parent differs, reparented, or unlinked on one side): last-writer-wins
  by timestamp (local `last_updated` vs remote `updated`), consistent with field
  mapping; the resolution is reported.
- Unresolvable or invalid edge (parent outside the synced set, or a Jira
  hierarchy-level violation): leave the edge unchanged, sync the rest of the item,
  warn.
- The parent edge joins the digest and baseline so divergence is detected.

## Acceptance Criteria

- [ ] Given a local item with a parent, both new to the remote, when sync pushes,
      then the parent is created first and the child is linked in the same run.
- [ ] Given a remote issue whose parent maps to a local item, when sync pulls, then
      the local `parent` frontmatter is set to that item.
- [ ] Given the parent differs between sides since the baseline, when sync runs,
      then the later-timestamp change wins (reparent or unlink) and the resolution
      is reported.
- [ ] Given an edge references an item outside the synced set, when sync runs, then
      the edge is left unchanged and a warning names the item and unresolved parent.
- [ ] Given a local edge violates the remote tracker's hierarchy rules, when sync
      pushes, then the edge is skipped and a warning is surfaced.
- [ ] Given a cycle in the local hierarchy, when sync orders the push, then it
      reports the cycle rather than looping.
- [ ] Given the port, when an issue is read or pushed, then its parent reference
      crosses it.

## Open Questions

- Jira mapping: does `parent` map to the Jira parent field, the epic link, or a
  subtask relationship, and how are Jira's level constraints reconciled with
  Accelerator's free-form parent (any kind may parent any kind)?
- Scope boundary: if a parent lives outside the configured pull scope (0229), is the
  edge deferred, skipped, or does it pull the parent in?
- Are other typed links (blocks/blocked_by/relates_to) in scope later, or is this
  strictly the parent edge?

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- Only the single parent edge is in scope; other typed links are out of scope here.
- Both trackers' single-parent model is sufficient; no multi-parent support needed.
- Item-level `last_updated` governs edge divergence (same limitation as field
  mapping).

## Technical Notes

- `RemoteIssue`/port: add a parent reference (remote key/UUID) on read and write.
- Jira: parent field / issue hierarchy, level-constrained; parent linkage needs the
  parent field or epic link.
- Linear: `IssueUpdateInput.parentId`, catalogue-resolved, arbitrary nesting.
- `fetch`/`apply`/`digest`/`baseline`: the parent edge joins projection,
  application, and change detection.
- Topological ordering + cycle detection in the push path (`run.rs`/`create.rs`).

## Drafting Notes

- Scoped to the single parent edge (matches local `parent:`); other typed links
  deferred.
- Treated ordering as topological parents-first per the user's decision; cycles are
  reported, not silently broken.
- last-writer-wins on divergence per the user's decision, consistent with field
  mapping.
- Priority set to medium to match epic 0146 and siblings.
- The exact Jira parent mechanism (parent field vs epic link) is flagged as an open
  question for implementation.

## References

- Source: `meta/work/0146-work-item-sync-enhancements.md`
- Related: 0146 (parent), 0229, 0290
