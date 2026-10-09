---
type: "work-item"
id: "0316"
title: "Stall Lists Already-Decided Transformations as Pending"
date: "2026-10-09T09:34:53+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "low"
parent: "work-item:0312"
relates_to: ["work-item:0116", "work-item:0117"]
tags: ["migration-engine", "interactive-contract", "stall"]
last_updated: "2026-10-09T09:34:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0316: Stall Lists Already-Decided Transformations as Pending

**Kind**: Bug
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

When an interactive run stalls for lack of decision input, its `pending
decision:` list includes every transformation from the stall point onward,
including ones already decided in the session log and ones routed
mechanically. An agent following the stall's guidance is told to decide
items that need no decision.

## Context

`prompt_and_decide` builds `pending_keys` from all of `remaining`
(`cli/migrate/src/engine.rs:252-261`). The `Reporter` contract says the list
names "every undecided transformation from the stalled one onward"
(`cli/migrate/src/ports.rs:550-553`). `--list` uses
`engine::pending_transformations`, which already filters correctly.

## Requirements

Expected: the stall lists exactly the transformations a live run would
prompt for, in emission order, matching `--list`.
Actual: the list also includes decided and mechanical transformations.

## Acceptance Criteria

- [ ] Given a session log recording a decision for a transformation after
      the stall point, when the run stalls, then that transformation is not
      listed as pending.
- [ ] Given a transformation after the stall point whose predicate routes it
      mechanically, when the run stalls, then it is not listed as pending.
- [ ] Given any stall, when its pending list is compared with `--list`
      output for the same state, then the keys and their order match.

## Open Questions

None.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

None.

## Technical Notes

- `pending_transformations` (`engine.rs:120-159`) already holds the
  decided/drift/route filter, so the stall can reuse it instead of
  duplicating it.

## Drafting Notes

- Identified by reading the code, not by running a stall. The first
  acceptance criterion's test is the reproduction.

## References

- Related: 0116, 0117
