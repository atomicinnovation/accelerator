---
type: work-item
id: "0146"
title: "Work Item Synchronisation Enhancements"
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
external_id: PP-167
---

# 0146: Work Item Synchronisation Enhancements

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Enhance work item synchronisation with remote trackers (Jira/Linear) by adding
richer field mapping, relationship handling, and scoping controls.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Work item sync exists but maps a
limited set of fields and lacks relationship and scoping controls.

## Requirements

Candidate child work items captured for decomposition during refinement:

- Add **status** mapping to work item synchronisation.
- Add **kind** mapping to work item synchronisation.
- Add **priority** mapping to work item synchronisation.
- Establish **parent-child relationships** between work items on
  synchronisation.
- **Restrict** work item sync based on configured labels or projects.

## Acceptance Criteria

- [ ] Status, kind, and priority are mapped bidirectionally during sync.
- [ ] Parent-child relationships are established/maintained on sync.
- [ ] Sync can be restricted to configured labels or projects.

## Open Questions

- How are mappings configured per tracker (Jira vs. Linear)?
- What is the conflict-resolution policy when local and remote values diverge?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- Builds on the existing `/sync-work-items` skill and the `external_id`
  remote-key convention.

## Technical Notes

- Relates to the work-management integration and sync-work-items skill.

## Drafting Notes

- Created as a new epic per the user's instruction to contain source candidates
  21–25 (status / kind / priority mapping, parent-child relationships, and
  label/project sync restriction), captured here as child candidates rather than
  as separate work items.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
