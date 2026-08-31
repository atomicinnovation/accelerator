---
type: "work-item"
id: "0265"
title: "Collapse Discoverability-Hook And Format-Hook Switches"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0276"
tags: ["cli", "hooks"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-850"
---

# 0265: Collapse Discoverability-Hook And Format-Hook Switches

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Collapse the `--discoverability-hook` and `--format=hook` switches into a
single switch, removing the redundant way of requesting hook output.

## Context

Captured in the further-ideas backlog. Two flags currently express the same
hook-output intent.

## Requirements

- Provide one switch for hook-formatted output and retire the redundant one.

## Acceptance Criteria

- [ ] A single switch controls hook output.
- [ ] The removed switch is gone (or aliased with a deprecation, if
  required).

## Open Questions

- Which switch name survives, and is a deprecation alias needed?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
