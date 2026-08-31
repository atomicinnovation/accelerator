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
parent: "work-item:0276"
tags: ["cli", "observability"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-839"
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
