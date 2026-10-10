---
type: "work-item"
id: "0326"
title: "Raise regression, ceiling and invalidation alerts"
date: "2026-10-10T18:25:09+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0219"
blocks: ["work-item:0327", "work-item:0328"]
relates_to: ["work-item:0225"]
tags: ["cli", "launcher", "performance", "measurement", "ci"]
last_updated: "2026-10-10T18:25:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0326: Raise regression, ceiling and invalidation alerts

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add an alerting job to the measurement lane that evaluates the per-merge
regression, ceiling and invalidation signals over the committed record series
and raises each as a GitHub issue, opening one per issue key and commenting on
it when it fires again. A regression then surfaces on the merge that caused
it, not in a spike months later.

## Context

Child of 0219 — Track warm-dispatch latency per release in CI. Platform state,
cohort, `τ`, report-only, issue key and the per-platform versus
per-platform-backend granularity are as 0219's Vocabulary and Granularity
define them.

No CI job opens GitHub issues today; no workflow grants `issues: write`. 0225
needs the same open-or-comment behaviour for its advisory feed.

## Requirements

- Signal evaluation is a set of pure functions over a platform's committed
  record series, testable against fixture series without CI.
- **Platform state** is derived from the series and `PLATFORM_TABLE`:
  **warm-up** until the 20th valid session, then **awaiting tuning** until its
  GitHub-hosted entry is committed, then **calibrated**. In warm-up and
  awaiting tuning only the invalidation signal fires.
- **Per-merge regression** — on a calibrated platform, for a backend not
  report-only, the `Δ_merge` interval's lower bound exceeds `1 + τ`.
- **Ceiling** — on a calibrated platform, with at least 10 valid sessions in
  the current cohort, the median over that cohort's last 10 valid sessions of
  a C1-C4 statistic exceeds that cell's ceiling in the GitHub-hosted entry.
  Report-only status does not suppress it.
- **Invalidation** — 3 consecutive invalidated sessions on a platform, live in
  every state. A run in which the platform is unmeasured produces no session,
  so it neither extends nor breaks the streak.
- The alerting job runs after the commit job, reads the committed series from
  `main`, and holds the measurement App's token.
- An open-or-comment step keyed on the issue key opens a GitHub issue or
  comments on the open one. It is reusable by 0225; generalising it beyond
  this lane is 0225's concern.
- The job summary extends the lane's with each platform-backend's `Δ_merge` and
  interval, current-cohort C1-C4 medians, each platform's state with warm-up
  progress, and each backend's report-only status.

## Acceptance Criteria

Verified against fixture record series committed as tests, not by waiting for
live regressions.

- [ ] Given a platform in warm-up or awaiting tuning, when a session records a
      `Δ_merge` lower bound of 1.15 or C1-C4 medians above 0217's linux-arm64
      ceilings, then no regression or ceiling issue opens and the job summary
      shows the platform's state.
- [ ] Given a backend on a calibrated platform, when the `Δ_merge` lower bound
      exceeds `1 + τ`, then a regression issue names `V`, the baseline
      version, the backend, the interval and the runner identity; and when it
      is at or below `1 + τ`, no regression issue opens.
- [ ] Given a report-only backend, when any `Δ_merge` arrives, then no
      regression issue opens and the summary says report-only.
- [ ] Given a calibrated platform with 10 valid sessions in the current cohort,
      when the median of a C1-C4 statistic over them exceeds that cell's
      ceiling, then a ceiling issue names the cell, the cohort and the rolling
      value, whether or not the backend is report-only.
- [ ] Given 9 valid sessions in one cohort followed by 1 in another, then no
      ceiling issue opens.
- [ ] Given 3 consecutive invalidated sessions on a platform, in any state,
      then an invalidation issue opens; given 2 followed by a valid one, then
      none opens; given 2, an unmeasured run, then a third, then an issue
      opens.
- [ ] Given an open issue for an issue key, when that signal fires again, then
      the alerting job comments on it and opens no new issue.
- [ ] Given any lane run, then its job summary contains each platform-backend's
      `Δ_merge` and interval, current-cohort C1-C4 medians, each platform's
      state with warm-up progress, each backend's report-only status, and any
      unmeasured platform.
- [ ] Given the workflow definition, then the measurement App's token is minted
      in the alerting job and the measuring jobs still never hold it.

## Open Questions

None.

## Dependencies

- Blocked by: 0325.
- Blocked by: the measurement GitHub App's `issues: write` permission.
- Blocks: 0327, 0328.
- Relates to 0225, which can reuse the open-or-comment step.

## Assumptions

- Within a cohort, image and kernel updates move `G` less than the CPU model
  does, so they need not split a cohort.

## Technical Notes

- No existing code opens issues: `gh issue`, `issues: write` and `/issues`
  appear nowhere under `tasks/`, `.github/` or `tests/`. 0225 cites an existing
  scheduled guard that opens issues, but none exists in this tree.
- `classify_cell` (`tasks/measure.py:946`) and `cells_for`
  (`tasks/measure.py:892-943`) already map a statistic and an entry's ceilings
  to a verdict.

## Drafting Notes

- A cohort is keyed on hardware alone because weekly image updates would
  otherwise reset it before it reached 10 sessions.
- The regression tolerance is `τ` rather than `t` because 0189 uses `t` for a
  cell's ceiling.

## References

- Parent: `meta/work/0219-own-the-recurring-absolute-budget-check.md`
- `meta/work/0225-advisory-feed-monitoring-for-the-vendored-runtime-pins.md`
- Mozilla Perfherder alert FAQ —
  https://wiki.mozilla.org/Performance_sheriffing/Alert_FAQ
