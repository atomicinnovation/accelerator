---
type: "work-item"
id: "0264"
title: "Remove Bash-Migration Negative-Assertion Tests"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0136"
tags: ["testing", "cleanup"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0136 (Migrate Shell Scripts into a Rust CLI): belongs to the shell-to-Rust migration, its shipped cli/ crates, or the launcher runtime-cache cluster."
schema_version: 1
external_id: "PP-849"
---

# 0264: Remove Bash-Migration Negative-Assertion Tests

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Remove the negative-assertion tests left over from the Bash-to-Rust
migration, which assert the absence of behaviour that is no longer relevant.

## Context

Captured in the further-ideas backlog. These tests were scaffolding during
the migration and now add noise without value.

## Requirements

- Identify and remove negative-assertion tests that only existed to guard
  the Bash-to-Rust transition.

## Acceptance Criteria

- [ ] Migration-era negative-assertion tests are removed.
- [ ] The remaining suite still passes.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
