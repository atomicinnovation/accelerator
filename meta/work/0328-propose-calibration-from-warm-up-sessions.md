---
type: "work-item"
id: "0328"
title: "Propose calibration from warm-up sessions"
date: "2026-10-10T18:25:09+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0219"
blocks: ["work-item:0329"]
relates_to: ["work-item:0217"]
tags: ["cli", "launcher", "performance", "measurement", "ci"]
last_updated: "2026-10-10T18:25:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0328: Propose calibration from warm-up sessions

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

When a platform completes warm-up, derive its proposed GitHub-hosted entry —
floor gates, ceilings, each backend's `τ` and detector parameters — from its
20 warm-up sessions by fixed rules, and open a ready-to-tune issue carrying the
proposal. The human tuning change then commits figures anyone can recompute
from the committed records.

## Context

Child of 0219 — Track warm-dispatch latency per release in CI. Warm-up,
awaiting tuning, `τ`, report-only and drift report-only are as 0219's
Vocabulary defines them.

`PlatformEntry` carries ceilings, floors and calibration provenance but no
regression tolerance or detector parameters, and every gating constant is held
in lockstep with `tasks/README.md`'s `### Criterion constants` block.

## Requirements

- `PlatformEntry` gains, per backend, `τ`, the detector parameters, and
  report-only and drift report-only status; `criterion_constants()` and the
  README block carry them under the lockstep test.
- The derivation runs in this order over a platform's 20 warm-up sessions:
  1. Each floor gate (`bash`, `true`) is 1.6× the median across sessions of
     the larger of that session's pre- and post-session floor — the midpoint
     of 0217's 40–80% band.
  2. Sessions whose larger floor breaches either gate are excluded. The gates
     are not re-derived.
  3. At most 2 further sessions may be excluded as carrying a real regression,
     each naming the session and the merge identified as its cause; the
     derivation takes these as input.
  4. From the remaining sessions, each ceiling is the smallest 10 ms multiple
     leaving at least 18% headroom over its statistic (0217's rule), and each
     backend's `τ` is the smallest whole percentage at or above the 95th
     percentile of `(Δ_merge lower bound − 1)`, floored at 2%; a backend whose
     `τ` would exceed the 10% cap is report-only.
  5. Each non-report-only backend's detector parameters are chosen so 0327's
     drift criteria hold on fixture series at that backend's `τ` and its
     remaining sessions' standard deviation of `log Δ_merge`; where none do,
     the backend is drift report-only.
- The derivation is a runnable task, so a tuning change's figures can be
  recomputed from the committed records and its named exclusions.
- When a platform completes 20 valid sessions, the alerting job opens a
  ready-to-tune issue, keyed per platform, carrying each backend's warm-up
  `Δ_merge` series and proposed `τ`, and the proposed calibration figures.

## Acceptance Criteria

- [ ] Given a platform completes 20 valid sessions, when the lane runs, then
      the platform is awaiting tuning and a ready-to-tune issue opens carrying
      each backend's warm-up `Δ_merge` series and proposed `τ`, and the
      proposed calibration figures.
- [ ] Given 20 fixture warm-up sessions, when the derivation runs, then each
      floor gate equals 1.6× the median of the sessions' larger floors and the
      floor-excluded sessions are exactly those whose larger floor breaches a
      gate.
- [ ] Given more than 2 regression exclusions, or one that names no causal
      merge, when the derivation runs, then it refuses.
- [ ] Given fixture sessions whose `τ` would exceed 10%, then that backend is
      proposed report-only; given a quieter series whose 95th percentile is
      below 2%, then `τ` is 2%.
- [ ] Given a backend for which no detector parameters meet both drift
      criteria, then it is proposed drift report-only.
- [ ] Given a `PlatformEntry` with the new fields, then the lockstep test fails
      when the README block omits or misstates any of them.

## Open Questions

None.

## Dependencies

- Blocked by: 0326, 0327.
- Blocks: 0329.

## Assumptions

None.

## Technical Notes

- `PlatformEntry` and `Calibration` are at
  `tasks/shared/measurement.py:781-816`; `PLATFORM_TABLE` at
  `tasks/measure.py:152-266`; `criterion_constants()` at
  `tasks/measure.py:281-324`.
- The lockstep test is `TestCriterionConstantsLockstep`
  (`tests/unit/tasks/test_measure.py:1264-1301`), parsing the block at
  `tasks/README.md:362-409`.

## Drafting Notes

- The 10% cap rests on paired detection reaching 10% or better and on C1-C4
  carrying 18–29% headroom; the 2% floor guards against a warm-up so quiet that
  ordinary variation would alert.
- Excluding sessions that breach the derived floor gates means a session that
  would fail after calibration cannot shape the post-calibration thresholds.
  Each session is judged on its larger floor so one that loaded up mid-run is
  judged on its worse reading. The cap of 2 regression exclusions stops
  exclusions from being used to tune `τ` down.

## References

- Parent: `meta/work/0219-own-the-recurring-absolute-budget-check.md`
- `meta/work/0217-measure-warm-dispatch-on-linux.md` — the headroom and floor
  derivation rules
