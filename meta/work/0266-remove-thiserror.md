---
type: "work-item"
id: "0266"
title: "Remove thiserror From The Codebase"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["cli", "dependencies", "refactor"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-796"
---

# 0266: Remove thiserror From The Codebase

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Remove the `thiserror` dependency from the codebase, replacing its derived
error types with hand-written error definitions.

## Context

Captured in the further-ideas backlog. The intent is to drop the
`thiserror` crate.

## Requirements

- Replace all `thiserror`-derived error types.
- Remove `thiserror` from the dependency graph.

## Acceptance Criteria

- [ ] No crate depends on `thiserror`.
- [ ] Error types are defined without it and the build passes.

## Open Questions

- What is the driver for removal — dependency reduction, control over error
  shapes, or a policy?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
