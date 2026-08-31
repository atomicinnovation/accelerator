---
type: "work-item"
id: "0254"
title: "Add CLI Logging"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["cli", "observability"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0254: Add CLI Logging

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add structured logging to the CLI so its behaviour can be traced and
diagnosed.

## Context

Captured in the further-ideas backlog. The CLI has limited diagnostic
output today, making failures hard to investigate.

## Requirements

- Introduce a logging mechanism across the CLI with a controllable level.

## Acceptance Criteria

- [ ] CLI operations emit logs at an appropriate level.
- [ ] Log verbosity is controllable (e.g. via a flag or env var).

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
