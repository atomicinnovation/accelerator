---
type: "work-item"
id: "0239"
title: "Visualiser Permission Block From Dynamic Args"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "bug"
priority: "medium"
tags: ["visualiser", "permissions"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0239: Visualiser Permission Block From Dynamic Args

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Running the visualiser triggers a permission block because the invoked
command includes dynamic arguments that do not match a static allow rule.

## Context

Captured in the further-ideas backlog. The command used to launch the
visualiser interpolates arguments at runtime, so the harness permission
matcher prompts or blocks instead of allowing it.

## Requirements

- Launching the visualiser should not hit a permission block due to dynamic
  arguments.

## Acceptance Criteria

- [ ] Starting the visualiser proceeds without a permission prompt or block
  caused by dynamic command arguments.

## Open Questions

- Fix by stabilising the command shape, or by adjusting the allow rule to
  tolerate the dynamic portion?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
