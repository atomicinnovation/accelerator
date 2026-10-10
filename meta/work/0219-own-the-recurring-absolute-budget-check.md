---
type: "work-item"
id: "0219"
title: "Track warm-dispatch latency per release in CI"
date: "2026-08-17T20:36:50+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "draft"
kind: "epic"
priority: "medium"
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
relates_to: ["work-item:0136", "work-item:0189", "work-item:0299", "work-item:0225"]
tags: ["cli", "launcher", "performance", "measurement", "ci"]
last_updated: "2026-10-10T18:25:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-748"
---

# 0219: Track warm-dispatch latency per release in CI

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Give warm-dispatch latency an owner that re-measures each prerelease the
`prerelease` job creates — one per merge to `main` — against its predecessor,
interleaved on the same GitHub Actions runner so runner differences cancel.
Regressions are judged per merge, as drift across merges, and against
per-platform ceilings calibrated on GitHub-hosted runners, never on one
unpaired run; alerts arrive as GitHub issues, and 0189's Latency Criterion is
amended to make the paired comparison its primary signal.

## Context

`G`, `B` and the cells C1-C6 are as 0189's Latency Criterion defines them; the
terms this epic adds are defined under Requirements → Vocabulary.

Work item 0189 measured the budget once and closed. C1-C4 were designated
**primary** on the ground that an absolute ceiling is re-runnable where a ratio
against a deleted baseline is not — but nothing re-runs them. `measure:*` is
deliberately outside `check` and the bare `default` task, and the
`check-measure-harness` job runs `test:integration:measure`, a smoke check that
emits **no gating figure**.

⚠️ **Without an owner a regression is discovered by a spike months later**,
rather than on the merge that caused it — the orphaned-obligation pattern
0189's plan exists to end.

The calibration landscape is narrower than 0189 assumed. Only linux-arm64 is
calibrated, from a Colima VM on a darwin-arm64 host (0217). The darwin-arm64
entry is permanently demoted to context because its calibrating session
recorded no `bash` or `shasum` provenance, and neither x64 platform has an entry.
None of these hosts resembles a GitHub-hosted runner: `macos-15` is a 3-vCPU
M1, `ubuntu-24.04-arm` a 4-vCPU Azure Cobalt 100. A platform with no entry
produces no figures at all today: the harness refuses it before measuring.

An absolute ceiling judged on one run from a shared runner does not survive
contact with GitHub-hosted hardware. The same `ubuntu-24.04` label lands on
AMD EPYC or Intel Xeon at random, and unpaired benchmarks on hosted runners
show a coefficient of variation near 2.7%, needing roughly a 7% gate for a 1%
false-positive rate. Running old and new interleaved on one machine is the
established answer: paired, randomised-order designs detect slowdowns of 10%
or less with high confidence, and interleaved designs reach low single digits.

Two properties of the pipeline shape the lane. The harness measures the
**published** release for the tree's own version, not the merged tree, and the
`prerelease` job bumps, signs, tags and publishes on every push to `main` — so
a job triggered on the merge commit would measure the previous merge's binary.
And a signed published release is always re-fetchable, which is what makes a
release-against-predecessor ratio re-runnable where 0189's `G`/`B` ratio was
not.

## Requirements

**Vocabulary.**

- **`V`** — the version a lane run measures: the tag created by the
  `prerelease` run the lane `needs`.
- **Baseline version** — the most recent prerelease before `V` with a valid
  record on that platform, or `V`'s immediate predecessor when the platform has
  no valid record yet. When it is older than the immediate predecessor,
  `Δ_merge` spans every merge between the two.
- **`G`** — `bin/accelerator vcs guard --format=hook --fail-safe` dispatched
  through the launcher with a warm cache (0189). C1-C4 are its per-session
  statistics: median (C1 fast, C3 fallback) and p90 (C2 fast, C4 fallback).
- **Backend** — the digest backend the launcher resolves: **fast**
  (`sha256sum`) or **fallback** (Perl `shasum -a 256`). The harness prepares
  one **farm** — an isolated install of a release — per backend.
- **Instrument floors** — the per-session timings of a bare `bash` and a bare
  `true` spawn, which show whether the host is quiet enough to measure.
- **Platform** — one row of the table below.
- **GitHub-hosted entry** — a `PLATFORM_TABLE` entry whose calibration
  provenance records a runner-identity tuple. The linux-arm64 entry from 0217
  and the provenance-less darwin-arm64 entry are not GitHub-hosted entries.
- **Runner-identity tuple** — `ImageOS`, `ImageVersion`, `RUNNER_ARCH`, CPU
  model, vCPU count and kernel release.
- **Cohort** — the sessions on one platform sharing CPU model and vCPU count.
  Image and kernel fields are recorded for attribution but do not split a
  cohort. The **current cohort** is the cohort of the session being evaluated.
- **Session** — one harness run measuring `V` and its baseline version on one
  platform.
- **Platform state** — a platform is in **warm-up** until its 20th valid
  session, then **awaiting tuning** until its GitHub-hosted entry is
  committed, then **calibrated**. "Bootstrap" means only the resampling
  estimator.
- **In-session stability band** — the harness's permutation-derived check that
  a session's own samples did not drift while it ran. "Drift" without
  qualification means only the cross-merge signal.
- **`τ`** — a platform-backend's per-merge regression tolerance, a fraction
  (0.03 is 3%).
- **Report-only** — a platform-backend whose `τ` would exceed the 10% cap; it
  records and summarises but never raises a regression or drift issue.
  **Drift report-only** — a platform-backend for which no detector parameters
  meet both drift bounds; it raises regression issues but not drift issues.
- **Issue key** — signal, platform and backend; for invalidation and
  ready-to-tune, which are per platform, the backend slot is `all`.

| Platform | Runner label | `PLATFORM_TABLE` key |
| --- | --- | --- |
| linux-x64 | `ubuntu-24.04` | `("Linux", "x86_64")` |
| linux-arm64 | `ubuntu-24.04-arm` | `("Linux", "aarch64")` |
| darwin-arm64 | `macos-15` | `("Darwin", "arm64")` |
| darwin-x64 | `macos-15-intel` | `("Darwin", "x86_64")` |

**Granularity.** Sessions, validity, warm-up, cohorts, calibration entries and
the invalidation and ready-to-tune signals are **per platform**. `Δ_merge`,
`τ`, report-only status and the regression, drift and ceiling signals are
**per platform-backend**; the ceiling cells split by backend as C1-C4 do.

**Goals.**

- **Measure** every prerelease the `prerelease` job publishes against its
  baseline version, interleaved on one GitHub-hosted runner per platform, on
  both backends.
- **Record** every session, valid or invalidated, as a committed record that
  the signals read and anyone can re-derive from its samples.
- **Calibrate** each platform from its own warm-up sessions on GitHub-hosted
  hardware, replacing the VM-scoped and provenance-less entries.
- **Alert** on per-merge regressions, cross-merge drift, cohort ceiling
  breaches and repeated invalidation as GitHub issues, never on one unpaired
  run.
- **Amend** 0189's Latency Criterion so the paired `Δ_merge` is the primary
  regression signal and C1-C4 the cohort-trend bound.

### Child work items

Prior measurement work, moved from 0136:

- 0205 — Close the warm-dispatch latency measurement method *(spike, done)*
- 0217 — Measure warm dispatch on linux *(done)*

The per-prerelease lane, along its dependency spine:

- 0322 — Record every measurement session's outcome
- 0323 — Pair a prerelease against its predecessor in the harness
- 0324 — Measure warm-up sessions on uncalibrated platforms
- 0325 — Measure each prerelease in a CI lane
- 0326 — Raise regression, ceiling and invalidation alerts
- 0327 — Detect cross-merge latency drift
- 0328 — Propose calibration from warm-up sessions
- 0329 — Tune the GitHub-hosted platform entries

## Acceptance Criteria

- [ ] Given the children listed in Requirements and any added later, when this
      epic closes, then every child has a terminal status.
- [ ] Given a merge to `main`, when its prerelease publishes, then every
      platform the lane measured has a committed record for that prerelease
      without human action.
- [ ] Given this epic closes, then each of the four platforms has a committed
      GitHub-hosted entry, or 0329 records why it has none.
- [ ] Given this epic closes, then 0189's Latency Criterion names the paired
      `Δ_merge` as the primary regression signal.

## Open Questions

None at epic level. The change-point detector choice is open on 0327.

## Dependencies

- Blocked by, for 0325's commit job and 0326's alerting job only: a dedicated
  measurement GitHub App, being created by Toby Clemson, with
  `contents: write`, a branch-protection bypass on `main`, and `issues: write`.
- Gated on merge cadence: 0329 waits for 20 valid lane sessions per platform.
- External systems: the release host, which every session fetches two signed
  releases from, and the four GitHub-hosted runner labels.
- Coupled to the release pipeline through the `accelerator-release`
  concurrency group, and to the local `mise run prerelease` flow, whose
  prereleases enter the version sequence unmeasured.
- Relates to 0136, whose launcher work this epic measures; 0189, whose Latency
  Criterion it amends; 0299, which takes one-off before/after figures and does
  not wait for this epic; and 0225, which can reuse 0326's open-or-comment
  step.

## Assumptions

- A ratio of two interleaved variants cancels runner speed to first order. It
  does not cancel a change in the work mix — a merge that adds hashing shows a
  smaller delta on a runner with SHA extensions — which is second-order and
  attributable through the runner-identity tuple.

## Technical Notes

The codebase findings that shape each child live in that child's Technical
Notes. The harness is `tasks/measure.py` with its statistics in
`tasks/shared/measurement.py`; the lane extends `.github/workflows/main.yml`.

## Drafting Notes

- Retitled from "Own the recurring absolute-budget check": the absolute check
  is now the secondary signal.
- Promoted from task to epic on 2026-10-10 and decomposed into 0322–0329 along
  the seams its acceptance criteria were grouped by. The detailed requirements
  and criteria moved verbatim into the children; the Vocabulary stays here as
  the epic's shared glossary.
- Made standalone with `relates_to` 0136, as 0312 was: 0136 closes the port
  itself, and this lane is ongoing operational work that would hold 0136 open
  for at least 20 merges. The measurement items 0205 and 0217 moved here from
  0136; 0189 stayed, since its subject is the launcher guarantee.
- Recording every session (0322) was split out after codebase analysis found
  that most invalidations write no record today.
- The baseline-version rule keeps the drift series gap-free at the cost of
  occasionally attributing a delta to several merges.
- Both backends gate: the fallback is the path every machine without coreutils
  `sha256sum` takes, and the harness builds its farm regardless.

## References

- `tasks/README.md#the-measure-namespace` — why the namespace is out of `check`
  and `default`, and who owns what today
- `tests/unit/tasks/test_mise.py` — the transitive-closure guard that keeps it
  out
- `tests/unit/tasks/test_measure.py` — the criterion-constants lockstep test
- `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md` — the
  Latency Criterion this item amends, and the definitions of `G`, `B` and C1-C6
- `meta/work/0217-measure-warm-dispatch-on-linux.md` — the VM-scoped linux
  entry and its derivation rules
- `meta/work/0299-bring-the-cli-workspace-s-crate-dependencies-into-line-with.md`
  — the one-off release-against-release comparison
- `meta/work/0225-advisory-feed-monitoring-for-the-vendored-runtime-pins.md` —
  CI-opened GitHub issues
- `.github/workflows/main.yml` — the `prerelease`, `release` and
  `check-measure-harness` jobs and the `accelerator-release` group
- `meta/reviews/work/0219-own-the-recurring-absolute-budget-check-review-1.md`
  — the review these revisions answer
- CodSpeed, "Benchmarks in CI without noise" —
  https://codspeed.io/blog/benchmarks-in-ci-without-noise
- Laaber et al., "Software microbenchmarking in the cloud. How bad is it
  really?", EMSE 2019 — https://research.chalmers.se/publication/511491
- Daly et al., "The Use of Change Point Detection to Identify Software
  Performance Regressions", ICPE 2020 — https://arxiv.org/pdf/2003.00584
- Mozilla Perfherder alert FAQ —
  https://wiki.mozilla.org/Performance_sheriffing/Alert_FAQ
- GitHub-hosted runner reference —
  https://docs.github.com/en/actions/reference/runners/github-hosted-runners
