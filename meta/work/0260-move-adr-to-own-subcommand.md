---
type: "work-item"
id: "0260"
title: "Move ADR Into Its Own Subcommand"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["cli", "adr", "corpus"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-790"
---

# 0260: Move ADR Into Its Own Subcommand

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move `adr` out of the `corpus` subcommand into its own top-level subcommand,
mirroring how `work` is structured.

## Context

Captured in the further-ideas backlog. ADR operations currently nest under
`corpus`; promoting them to a peer of `work` gives a cleaner command tree.

## Requirements

- ADR operations are reachable via a top-level `adr` subcommand.

## Acceptance Criteria

- [ ] `accelerator adr ...` exposes the ADR operations previously under
  `corpus`.
- [ ] Skills and docs referencing the old path are updated.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
