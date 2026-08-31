---
type: "work-item"
id: "0249"
title: "Richer Document Model Crate"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["corpus", "crates", "refactor"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0249: Richer Document Model Crate

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build a richer document model and consolidate all document handling onto a
single crate, so document parsing, frontmatter, and rendering share one
domain model.

## Context

Captured in the further-ideas backlog. Document handling is spread across
call sites; a consolidated model would reduce duplication and drift.

## Requirements

- Define a richer document domain model.
- Route all document handling through the consolidating crate.

## Acceptance Criteria

- [ ] Document handling is served by one crate with a shared model.
- [ ] Duplicated ad-hoc document parsing is removed.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.
- Large scope; may be an epic once refined.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
