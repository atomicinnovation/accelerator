---
type: "work-item"
id: "0322"
title: "Record every measurement session's outcome"
date: "2026-10-10T18:25:09+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0219"
blocks: ["work-item:0323", "work-item:0324"]
tags: ["cli", "launcher", "performance", "measurement", "ci"]
last_updated: "2026-10-10T18:25:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0322: Record every measurement session's outcome

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Make every warm-dispatch session that starts write a record, so an
invalidated session leaves its reason on disk rather than vanishing. The
per-prerelease lane commits invalidated sessions and counts them towards the
invalidation streak, which is impossible while most invalidations write
nothing.

## Context

Child of 0219 — Track warm-dispatch latency per release in CI.

`run_session` writes its record only after the session's `with` block
completes. Every `PreconditionFailureError` raised inside it — a floor-gate
failure, an invalid warm-up dispatch, a failed term harness — propagates and
leaves no file, as do the `Exit`s raised by the outlier trip and by wall-clock
budget exhaustion. Only a teardown failure and a drifting session produce an
invalidated record today.

0219 requires that invalidated sessions are committed with their reason and
that an unreachable release host invalidates a session rather than refusing it.

## Requirements

- A session that fails after it has started writes its record with validity
  invalid and a named invalidation reason, then runs teardown restore and
  verify as a successful session does.
- Invalidation reasons distinguish at least: a named precondition failure, the
  floor gate, an invalid warm-up dispatch, the outlier trip, wall-clock budget
  exhaustion, an unreachable release host, the in-session stability band, and
  a teardown failure.
- An unreachable release host is identified as such, distinct from a cold cache
  for any other cause.
- Refusals that happen before a session starts — a stale manifest, a release
  key digest mismatch, a non-empty diff over the guarded paths — remain
  refusals and write no record.
- The samples gathered before an invalidation are written alongside the record
  when any exist.

## Acceptance Criteria

- [ ] Given a session that fails at each point named in Requirements, when it
      ends, then exactly one record exists with validity invalid and that
      reason, and teardown verify has run.
- [ ] Given `ACCELERATOR_RELEASE_BASE_URL` pointing at a refusing host, when a
      session runs against a cold cache, then its record is invalidated with
      the reason release host unreachable.
- [ ] Given a stale manifest, a release-key digest mismatch or a guarded-path
      diff, when a session is requested, then no record is written and the
      existing refusal is reported.
- [ ] Given a session invalidated after sampling began, then its samples file
      holds every sample taken before the invalidation.

## Open Questions

None.

## Dependencies

- Blocked by: none.
- Blocks: 0323, 0324.

## Assumptions

- Teardown remains safe to run from every failure point that follows the
  manifest being written, since `measure:teardown` already replays it from the
  manifest alone.

## Technical Notes

- The record is written after the `with` block (`tasks/measure.py:1829-1842`);
  `MeasurementSession.__exit__` returns `False` (`tasks/measure.py:632`), so
  every raise inside the block propagates past it.
- Raises inside the session: the missing-entry refusal
  (`tasks/measure.py:1750`), the floor gate (`tasks/measure.py:1759`), the
  warm-up dispatch (`tasks/measure.py:1784`); `Exit`s from the outlier trip
  (`tasks/measure.py:2463`) and budget exhaustion (`tasks/measure.py:2481`)
  in `sample_blocks`.
- The two paths that already record an invalidation: teardown failure
  (`tasks/measure.py:1824-1828`) and drift (`tasks/measure.py:2321-2324`).
- `REFUSING_RELEASE_URL` (`tasks/measure.py:3141`) already points the launcher
  at a host that refuses, for the summary-latency task.

## Drafting Notes

- Pre-session refusals stay refusals: each means the harness must not touch
  the host at all, and a CI run that hits one is a lane failure rather than a
  measurement.

## References

- Parent: `meta/work/0219-own-the-recurring-absolute-budget-check.md`
