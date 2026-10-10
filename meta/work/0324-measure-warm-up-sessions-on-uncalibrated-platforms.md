---
type: "work-item"
id: "0324"
title: "Measure warm-up sessions on uncalibrated platforms"
date: "2026-10-10T18:25:09+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0219"
blocks: ["work-item:0325"]
relates_to: ["work-item:0217"]
tags: ["cli", "launcher", "performance", "measurement", "ci"]
last_updated: "2026-10-10T18:25:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0324: Measure warm-up sessions on uncalibrated platforms

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Let the harness measure a platform that has no GitHub-hosted entry instead of
refusing it, judging each such warm-up session valid by strict in-session
checks rather than by calibrated floors. Warm-up sessions are what a platform's
GitHub-hosted entry is later calibrated from, so without them no runner can
ever be calibrated.

## Context

Child of 0219 — Track warm-dispatch latency per release in CI. GitHub-hosted
entry, runner-identity tuple, instrument floors and in-session stability band
are as 0219's Vocabulary defines them.

Only linux-arm64 is calibrated, from a Colima VM (0217), and the darwin-arm64
entry is demoted to context because its calibration recorded no `bash` or
`shasum` provenance. Neither resembles a GitHub-hosted runner, and neither x64
platform has an entry. A platform with no entry fails today before measuring.

## Requirements

- `Calibration` gains an optional runner-identity tuple; an entry is a
  GitHub-hosted entry only when that tuple is present.
- On a platform with a GitHub-hosted entry, validity is the harness's existing
  rule, including the instrument-floor gate.
- On a platform without one, the harness measures instead of refusing. The
  floor gate is skipped; both instrument floors are measured before and after
  sampling and recorded.
- Such a warm-up session is valid only when strict preconditions pass, both
  releases fetch and verify, the in-session stability band holds, the
  wall-clock budget is not exhausted, teardown restore and verify pass, and for
  each instrument floor `|post − pre| / pre ≤ 0.20`.
- The farm's tool list and power probes are derived without an entry, so the
  fast farm carries `sha256sum` and the tools the launcher and the baseline
  probe need on any of the four platforms.
- The record states whether the session ran under warm-up validity.
- The task's message for a platform without a GitHub-hosted entry describes
  what now happens.

## Acceptance Criteria

- [ ] Given a platform without a GitHub-hosted entry, when a session runs,
      then the harness measures rather than refusing, records both instrument
      floors before and after sampling, and skips the floor gate.
- [ ] Given such a session where either floor has `|post − pre| / pre > 0.20`,
      or that fails any other warm-up validity check, then it is recorded
      invalidated with that reason; given both floors within 0.20 and every
      other check passing, then it is recorded valid under warm-up validity.
- [ ] Given linux-arm64 or darwin-arm64 with only its existing entry, when a
      session runs, then it runs under warm-up validity.
- [ ] Given a fixture entry whose calibration carries a runner-identity tuple,
      when a session runs on that platform, then the harness applies the floor
      gate.
- [ ] Given a session on each of linux-x64, linux-arm64, darwin-arm64 and
      darwin-x64 with no entry, then both farms build and the backend
      assertion passes wherever the backend's tool exists.

## Open Questions

None.

## Dependencies

- Blocked by: 0322, 0323.
- Blocks: 0325.

## Assumptions

- The 0.20 floor-stability tolerance passes ordinary jitter on a quiet runner
  and fails a neighbour starting heavy work mid-session; it is revisable at
  tuning (0329).

## Technical Notes

- Without an entry `run_session` falls back to
  `tools = ("bash", "jj", "true")` (`tasks/measure.py:1741`), so `build_rig`
  builds a fast farm with no `sha256sum` and `assert_backends` raises
  (`tasks/measure.py:1402`) before preconditions or provenance are recorded.
  Replacing the raise at `tasks/measure.py:1750` is not enough on its own.
- `gate_floors` (`tasks/measure.py:1694-1723`) gates on the entry's floors;
  post-sampling floors are recorded but never compared with pre-sampling ones
  (`tasks/measure.py:1801`).
- `assess_drift` (`tasks/measure.py:2637`) derives its band from the session's
  own permutation null, so it needs no entry.
- `unconfirmed_calibration_fields` (`tasks/shared/measurement.py:850-880`)
  already treats any `None` calibration field as unconfirmable.
- The misleading "recorded as uncalibrated context" message is at
  `tasks/measure.py:1150`.

## Drafting Notes

- The runner-identity tuple lands here rather than with the tuning fields
  because it is what decides which validity rule a session runs under.

## References

- Parent: `meta/work/0219-own-the-recurring-absolute-budget-check.md`
- `meta/work/0217-measure-warm-dispatch-on-linux.md` — the VM-scoped
  linux-arm64 entry
