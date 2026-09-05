---
type: "work-item"
id: "0270"
title: "Move Filesystem Into A Shared Crate"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["crates", "testing", "refactor"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-800"
---

# 0270: Move Filesystem Into A Shared Crate

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move the `Filesystem` abstraction into a shared crate and use it everywhere
filesystem access is required, along with a fake implementation for tests.

## Context

Captured in the further-ideas backlog. Filesystem access is done directly in
places; a shared abstraction with a fake enables deterministic testing.

## Requirements

- A shared crate exposes the `Filesystem` abstraction and a fake.
- All filesystem access goes through it.

## Acceptance Criteria

- [ ] Filesystem access across the workspace uses the shared abstraction.
- [ ] A fake implementation is available for tests.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
