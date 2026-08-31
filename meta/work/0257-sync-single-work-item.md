---
type: work-item
id: "0257"
title: "Sync A Single Work Item"
date: "2026-08-31T12:11:13+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
tags: [work, sync]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0257: Sync A Single Work Item

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Allow `/sync-work-items` to operate on a single named work item, rather than
always reconciling the whole set.

## Context

Captured in the further-ideas backlog. A full sync is heavyweight when the
user only wants to push or pull one item.

## Requirements

- The sync skill accepts a single work item as its target.

## Acceptance Criteria

- [ ] Given a single work item target, sync reconciles only that item.
- [ ] The whole-set behaviour remains available when no target is given.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
