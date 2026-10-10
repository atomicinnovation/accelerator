---
type: "work-item"
id: "0329"
title: "Tune the GitHub-hosted platform entries"
date: "2026-10-10T18:25:09+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0219"
tags: ["cli", "launcher", "performance", "measurement", "ci"]
last_updated: "2026-10-10T18:25:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0329: Tune the GitHub-hosted platform entries

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As each platform's ready-to-tune issue opens, commit its GitHub-hosted entry —
calibration provenance with the runner-identity tuple, floor gates, ceilings,
each backend's `τ` and detector parameters — in one human-authored change per
platform. Until a platform is tuned, only its invalidation signal can fire.

## Context

Child of 0219 — Track warm-dispatch latency per release in CI.

Thresholds are gating constants, so they are committed by a person rather
than by the lane, whose writes are confined to `meta/measurements/`. The 20th
valid session and the tuning change are separated by human time; that gap is
the awaiting-tuning state.

## Requirements

- For each of linux-x64, linux-arm64, darwin-arm64 and darwin-x64, once its
  ready-to-tune issue opens, the epic owner commits one change that adds its
  GitHub-hosted entry to `PLATFORM_TABLE` and the `### Criterion constants`
  block, from 0328's derivation over its 20 warm-up sessions.
- The change records any regression exclusions, at most 2, each naming the
  session and the merge identified as its cause.
- The change re-runs 0327's drift fixture criteria at each non-report-only
  backend's committed `τ`, its remaining sessions' standard deviation of
  `log Δ_merge` and its committed detector parameters.
- The change revisits the 0.20 floor-stability tolerance against the warm-up
  floors and records whether it stands.
- The change closes the platform's ready-to-tune issue.
- Once the first sessions' wall-clock durations are recorded, the measuring
  jobs' provisional `timeout-minutes` of 60 is revised from them.
- The 0217 linux-arm64 entry and the darwin-arm64 entry are replaced by their
  GitHub-hosted entries.

## Acceptance Criteria

- [ ] Given a committed tuning change, then each floor gate, exclusion,
      ceiling and `τ` recomputes from the committed records and named
      exclusions by 0328's derivation, any backend whose `τ` exceeds 10% is
      report-only, and the lockstep test passes.
- [ ] Given a committed tuning change, then the drift fixture criteria pass for
      each non-report-only backend at its committed values, or the backend is
      drift report-only.
- [ ] Given a committed tuning change, when the next session runs on that
      platform, then the platform is calibrated, the harness applies the floor
      gate, and the entry's provenance carries the runner-identity tuple.
- [ ] Given all four platforms, then each has a committed GitHub-hosted entry,
      or this item records why it has none (for example, a runner label that
      stopped being offered).
- [ ] Given the recorded wall-clock durations of the first 10 sessions per
      platform, then the measuring jobs' `timeout-minutes` is set from them.

## Open Questions

None.

## Dependencies

- Blocked by: 0328.
- Gated on merge cadence: each platform waits for 20 valid lane sessions — 20
  merges to `main` at best.
- External system: GitHub-hosted runners; Intel macOS is expected to lose
  support around Fall 2027.

## Assumptions

- Each platform's cohort stays stable enough over 20 merges for its warm-up
  sessions to describe the runner the lane keeps landing on.

## Technical Notes

- Tuning changes are independent per platform and can land as each platform
  completes warm-up.

## Drafting Notes

- Kept as its own child so the code stories can close without waiting on
  merge cadence.

## References

- Parent: `meta/work/0219-own-the-recurring-absolute-budget-check.md`
- GitHub-hosted runner reference —
  https://docs.github.com/en/actions/reference/runners/github-hosted-runners
