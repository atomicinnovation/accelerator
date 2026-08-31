---
type: "work-item"
id: "0247"
title: "Retire server.pid In Design Commands"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0276"
tags: ["design", "cleanup"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-832"
---

# 0247: Retire server.pid In Design Commands

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Remove the `server.pid` file mechanism from the design commands in favour
of the current process-management approach.

## Context

Captured in the further-ideas backlog. The design commands still track a
running server via a `server.pid` file.

## Requirements

- Remove reliance on `server.pid` in the design commands.

## Acceptance Criteria

- [ ] The design commands manage the server lifecycle without a
  `server.pid` file.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.
- Likely relates to daemon process-management work.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
