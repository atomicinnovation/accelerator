---
type: "work-item"
id: "0272"
title: "Move insecure-local-ok Marker Under .accelerator"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["config", "cleanup"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0272: Move insecure-local-ok Marker Under .accelerator

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move the `insecure-local-ok` marker file under the `.accelerator` directory,
consolidating Accelerator-owned files in one place.

## Context

Captured in the further-ideas backlog. The marker currently sits outside
`.accelerator`, unlike other plugin-owned files.

## Requirements

- Relocate the `insecure-local-ok` marker under `.accelerator`.
- Update the code that reads/writes it, with migration for existing repos.

## Acceptance Criteria

- [ ] The marker is read from and written to `.accelerator`.
- [ ] Existing repos are migrated so the marker keeps working.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
