---
type: work-item
id: "0143"
title: "Caching Strategy for token_cmd Skills"
date: "2026-06-22T23:41:03+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: spike
priority: medium
source: "note:2026-06-22-ideas-backlog"
tags: []
last_updated: "2026-06-22T23:41:03+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0143: Caching Strategy for token_cmd Skills

**Kind**: Spike
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Determine a caching strategy for skills that use `token_cmd`-type configuration,
to avoid redundant recomputation of injected context.

## Context

Extracted from the ideas backlog note
(`meta/notes/2026-06-22-ideas-backlog.md`). Skills inject live context via shell
commands at invocation time; repeatedly running expensive commands is wasteful.

## Requirements

- Research question: what caching strategy best fits `token_cmd`-style context
  injection?
- Consider cache keys, invalidation, and scope (per-session, per-repo).

## Acceptance Criteria

- [ ] A spike write-up recommends a caching strategy (or decides against one)
      with rationale and follow-on work items.

## Open Questions

- What is the freshness requirement for injected context vs. the cost of
  recomputation?
- Where would a cache live and how is it invalidated?

## Dependencies

- Blocked by: None identified yet.
- Blocks: None identified yet.

## Assumptions

- `token_cmd` refers to a configuration type used to inject command output as
  context.

## Technical Notes

- Relates to the `!` preprocessor mechanism that runs shell at skill invocation
  time.

## Drafting Notes

- Kept as a spike (the source says "Determine a caching strategy ...") with an
  explicit research question; time-box to be set during refinement.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-22-ideas-backlog.md`
