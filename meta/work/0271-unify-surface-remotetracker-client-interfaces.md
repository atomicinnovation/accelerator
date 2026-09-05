---
type: "work-item"
id: "0271"
title: "Unify Surface, RemoteTracker And Client Interfaces"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["work", "crates", "refactor"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-801"
---

# 0271: Unify Surface, RemoteTracker And Client Interfaces

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Rationalise the `Surface`, `RemoteTracker`, and client-specific interfaces
into a single unified interface for remote tracker interaction.

## Context

Captured in the further-ideas backlog. Three overlapping abstractions
describe tracker interaction; consolidating reduces indirection and
divergence.

## Requirements

- Design one unified interface subsuming `Surface`, `RemoteTracker`, and the
  client-specific interfaces.
- Migrate providers (Jira, Linear) onto it.

## Acceptance Criteria

- [ ] Tracker interaction is expressed through a single unified interface.
- [ ] Provider implementations conform to it.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.
- Sizeable; may become an epic.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
