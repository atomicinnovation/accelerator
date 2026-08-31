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
parent: "work-item:0276"
tags: ["work", "crates", "refactor"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-856"
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
