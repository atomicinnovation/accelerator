---
type: "work-item"
id: "0317"
title: "Guard Migration Registry Order and ID Uniqueness"
date: "2026-10-09T09:34:53+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
parent: "work-item:0312"
tags: ["migration-engine", "registry"]
last_updated: "2026-10-09T09:34:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0317: Guard Migration Registry Order and ID Uniqueness

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Migrations run in the order of the `registry()` literal, and nothing checks
that order or the uniqueness of IDs. A misplaced or duplicated entry would
silently reorder migrations or confuse the ledgers and the lag advisory.

## Context

`registry()` returns a `vec!` literal (`cli/migrate/src/registry.rs:71-84`).
`ledger::pending` keeps that order. The discoverability lag rule takes a
lexicographic `max()` of registry IDs (`discoverability.rs:39-41`), which
assumes literal order equals lexicographic order.

## Requirements

- Fail the test suite when registry IDs are not strictly ascending.
- Fail the test suite when two registry entries share an ID, or share a
  numeric prefix.

## Acceptance Criteria

- [ ] Given a registry with two entries out of lexicographic order, when the
      test suite runs, then a test fails naming both IDs.
- [ ] Given a registry with two entries sharing a numeric prefix, when the
      test suite runs, then a test fails naming the prefix.
- [ ] Given the current registry, when the test suite runs, then both
      guards pass.

## Open Questions

None.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- A test over the real `registry()` is enough. A compile-time check is not
  needed, since every change runs CI.

## Technical Notes

None.

## Drafting Notes

- Shared numeric prefixes are guarded as well as exact duplicates, because
  the `NNNN` prefix is the convention that orders the ledgers. Two
  `0011-…` entries with different slugs would pass an exact-match check.

## References

- `cli/migrate/src/registry.rs`
