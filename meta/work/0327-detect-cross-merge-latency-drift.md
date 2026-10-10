---
type: "work-item"
id: "0327"
title: "Detect cross-merge latency drift"
date: "2026-10-10T18:25:09+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0219"
blocks: ["work-item:0328"]
tags: ["cli", "launcher", "performance", "measurement", "ci"]
last_updated: "2026-10-10T18:25:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0327: Detect cross-merge latency drift

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add a change-point detector over each platform-backend's `log Δ_merge` series
that raises a drift issue when a sustained upward shift appears, naming the
window of sessions it attributes the shift to. It catches a slowdown spread
across several merges, each too small for the per-merge regression signal.

## Context

Child of 0219 — Track warm-dispatch latency per release in CI. Drift, `τ`,
report-only and drift report-only are as 0219's Vocabulary defines them;
"drift" means only the cross-merge signal, not the in-session stability band.

## Requirements

- On a calibrated platform, for a backend neither report-only nor drift
  report-only, a change-point detector finds a sustained upward shift in the
  **mean** of the `log Δ_merge` series and reports the window of sessions it
  attributes the shift to.
- The detector is deterministic under a fixed seed.
- Its parameters are per platform-backend and supplied to it, so tuning (0328)
  can choose them; before tuning, fixture tests use reference values.
- Synthetic fixture series are drawn i.i.d. Normal(0, σ) from seeds recorded in
  the fixtures, at reference values σ = 0.02 and `τ` = 0.03.
- Drift issues go through 0326's open-or-comment step, keyed on signal,
  platform and backend.
- The job summary adds each platform-backend's detector state and drift
  report-only status.

## Acceptance Criteria

- [ ] Given 100 seeded stationary series of 50 sessions, then at most 1 opens
      a drift issue.
- [ ] Given 100 seeded series of 30 sessions with a sustained shift of
      `log(1 + τ / 2)` in the mean from session 20, then at least 90 open a
      drift issue within 10 sessions of the shift, each naming a window
      containing session 20.
- [ ] Given a platform in warm-up or awaiting tuning, when its series carries a
      sustained 20% shift, then no drift issue opens.
- [ ] Given a report-only backend, when the detector would report a shift, then
      no drift issue opens.
- [ ] Given a drift report-only backend, when the detector would report a
      shift, then no drift issue opens, while a `Δ_merge` lower bound above
      `1 + τ` still opens a regression issue.
- [ ] Given the same series and seed, when the detector runs twice, then both
      runs report the same window.
- [ ] Given any lane run, then the job summary contains each platform-backend's
      detector state and drift report-only status.

## Open Questions

- Which change-point detector? Candidates are E-Divisive means
  (`mongodb/signal-processing-algorithms`, Apache Otava) and an in-house
  windowed two-sample test. Decide on dependency footprint and supply-chain
  review, behaviour on series shorter than 30, and determinism under a fixed
  seed. **Default if unresolved**: the in-house windowed test, which adds no
  dependency.

## Dependencies

- Blocked by: 0326.
- Blocks: 0328.

## Assumptions

None.

## Technical Notes

- Fixture series and their seeds live with the tests so the drift criteria
  re-run unchanged at each backend's committed values in the tuning change
  (0329).

## Drafting Notes

- Drift is detected on the mean of `log Δ_merge`, not its cumulative sum,
  because a sum compounds per-session error like a random walk.
- The reference values make the detector testable before any platform is
  tuned.

## References

- Parent: `meta/work/0219-own-the-recurring-absolute-budget-check.md`
- Daly et al., "The Use of Change Point Detection to Identify Software
  Performance Regressions", ICPE 2020 — https://arxiv.org/pdf/2003.00584
