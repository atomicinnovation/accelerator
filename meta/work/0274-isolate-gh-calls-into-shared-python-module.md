---
type: "work-item"
id: "0274"
title: "Isolate gh Calls Into A Shared Python Module"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["build-system", "github", "refactor"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-859"
---

# 0274: Isolate gh Calls Into A Shared Python Module

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Isolate all calls to the `gh` CLI into a shared Python module, so GitHub
interaction has one wrapper rather than scattered invocations.

## Context

Captured in the further-ideas backlog. `gh` is invoked from multiple places;
a single module would centralise argument handling and error handling.

## Requirements

- Provide a shared Python module wrapping `gh`.
- Route all `gh` invocations through it.

## Acceptance Criteria

- [ ] No code invokes `gh` directly; all use the shared module.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.
- "Python module" implies the build-system/`tasks/` layer as the home.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
