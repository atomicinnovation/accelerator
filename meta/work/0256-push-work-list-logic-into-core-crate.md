---
type: work-item
id: "0256"
title: "Push Work-List Logic Into Core Crate"
date: "2026-08-31T12:11:13+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: task
priority: medium
tags: [work, crates, refactor]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0256: Push Work-List Logic Into Core Crate

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Move the work-list logic (filtering, sorting, hierarchy) down into a core
crate so it is reusable and tested independently of the command layer.

## Context

Captured in the further-ideas backlog. Work-list logic sits in the command
surface rather than a shared crate.

## Requirements

- Extract work-list logic into a core crate with its own tests.

## Acceptance Criteria

- [ ] Work-list filtering and hierarchy logic lives in a core crate.
- [ ] The list command delegates to that crate.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
