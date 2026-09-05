---
type: "work-item"
id: "0255"
title: "Merge Conflicting Chunks In Work-Item Sync"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
tags: ["work", "sync"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-840"
---

# 0255: Merge Conflicting Chunks In Work-Item Sync

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Allow the work-item sync skill to merge conflicting chunks between local and
remote state, rather than forcing a whole-item choose-one resolution.

## Context

Captured in the further-ideas backlog. `/sync-work-items` currently resolves
divergence coarsely; chunk-level merging would preserve non-conflicting
edits from both sides.

## Requirements

- The sync skill can merge non-conflicting chunks automatically and surface
  only the truly conflicting ones.

## Acceptance Criteria

- [ ] Given local and remote both changed different chunks of one work item,
  sync merges both without a full-item conflict.
- [ ] Genuinely conflicting chunks are surfaced for resolution.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.
- Relates to the existing conflict-flow dossier design.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
