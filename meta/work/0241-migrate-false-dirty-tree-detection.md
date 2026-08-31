---
type: "work-item"
id: "0241"
title: "Migrate False Dirty-Tree Detection"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "bug"
priority: "medium"
tags: ["migration", "vcs"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-771"
---

# 0241: Migrate False Dirty-Tree Detection

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`accelerator migrate` refuses to run, reporting a dirty working tree, when
the tree is actually clean.

## Context

Captured in the further-ideas backlog. Migration is guarded against a dirty
tree; the dirtiness check produces a false positive under some conditions
(possibly VCS-specific).

## Requirements

- The dirty-tree guard must report clean when the working tree is clean.

## Acceptance Criteria

- [ ] Given a clean working tree, `accelerator migrate` proceeds without a
  dirty-tree refusal.

## Open Questions

- Which VCS and state reproduces the false positive?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
