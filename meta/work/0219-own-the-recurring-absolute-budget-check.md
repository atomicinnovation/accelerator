---
type: "work-item"
id: "0219"
title: "Own the recurring absolute-budget check"
date: "2026-08-17T20:36:50+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0136"
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
relates_to: ["work-item:0189"]
tags: ["cli", "launcher", "performance", "measurement"]
last_updated: "2026-08-17T20:36:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-748"
---

# 0219: Own the recurring absolute-budget check

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Give the absolute warm-dispatch budget an owner that re-runs it. C1-C4 were
designated **primary** on the ground that an absolute ceiling is re-runnable where
a ratio against a deleted baseline is not — but nothing currently re-runs them, so
the primary gate is as one-shot as the ratio it displaced.

## Context

Work item 0189 measured the budget once and closed. Four things keep it from
recurring: `measure:*` is deliberately outside `check` and the bare `default`
task; `test:integration:measure` is a smoke check that emits **no gating figure**;
the ceilings are calibrated for a quiet darwin-arm64 host; and the
instrument-floor gate is one no shared runner reliably clears.

⚠️ **Without an owner the first regression is discovered the way this one was** —
by a spike months later — which is the orphaned-obligation pattern 0189's plan
exists to end. Three attempts were needed to get one valid session, so a lane
that reports a verdict on every push is not the shape; a trend is.

## Requirements

- A scheduled, **non-blocking** `measure:warm-dispatch` lane on a self-hosted or
  best-effort runner, recording a trend and alerting on a ceiling breach.
- The host-quietness caveat stated in whatever it reports: the instrument-floor
  gate and the permutation-derived drift band both exist because a loaded host
  produces figures that look precise and are not.
- An uncalibrated platform key must report **context, not a verdict** — the
  behaviour the harness already implements. A runner whose `bash` or `shasum`
  differs from the calibrated entry's provenance demotes automatically, which is
  correct and must not be worked around.
- Invalidated sessions must be visible in the trend rather than dropped, or the
  trend silently reports only the sessions that happened to be steady.

## Acceptance Criteria

- [ ] A scheduled non-blocking lane runs `measure:warm-dispatch` and records its
      record, including invalidated attempts.
- [ ] A ceiling breach on C1-C4 raises an alert; a drift invalidation does not,
      but is visible in the trend.
- [ ] The lane reports uncalibrated context rather than a verdict where the
      platform key has no calibrated entry.
- [ ] **If this item is declined, the re-runnability argument is struck from
      0189's Latency Criterion in the same change**, leaving C1-C4 justified on
      user-perceptibility and the `B`/`G` work asymmetry alone — the arguments
      that hold without it.

## Open Questions

- Is a self-hosted runner available and quiet enough to clear the instrument
  floors? **Default if unresolved**: a best-effort lane that records figures as
  uncalibrated context, which is still a trend and still catches a gross
  regression, but cannot enforce a ceiling.

## Dependencies

- **Relates to** 0189, which designated C1-C4 primary and committed the harness,
  and the linux measurement item, whose calibrated entry this lane would consume.
- **Parent**: epic 0136.

## Assumptions

- A ceiling breach is worth alerting on even from an uncalibrated host, because
  the ceilings carry 26% to 36% of headroom and a breach at that margin is
  unlikely to be pure host noise.

## References

- `tasks/README.md#the-measure-namespace` — why the namespace is out of `check`
  and `default`, and who owns what today
- `tests/unit/tasks/test_mise.py` — the transitive-closure guard that keeps it out
- `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md` — the criterion
  and the re-runnability argument this item underwrites
