---
work_item_id: "0099"
title: "Mismatched Identity Example"
date: "2026-01-20T10:00:00+00:00"
author: Toby Clemson
type: task
status: draft
priority: low
parent: ""
tags: []
---

# 0099: Mismatched Identity Example

**Type**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

This fixture is used by eval id 31 to exercise the at-write identity-swap
check. The file has work_item_id 0099, but the eval scenario caches 0042
during Step 0. The at-write check detects the mismatch and aborts.

## Context

Fixture file for testing the identity-swap abort path in the
create-work-item enrich-existing Step 5 flow.

## Requirements

Not applicable — this is a test fixture.

## Acceptance Criteria

- [ ] Not applicable.

## Open Questions

- None.

## Dependencies

- Blocked by: nothing
- Blocks: nothing

## Assumptions

- None.

## Technical Notes

This file intentionally has a different work_item_id (0099) than the
value that would be cached in Step 0 (0042) during the eval scenario.

## Drafting Notes

- Fixture only.

## References

- None.
