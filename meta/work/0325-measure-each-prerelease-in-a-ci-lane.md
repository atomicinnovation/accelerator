---
type: "work-item"
id: "0325"
title: "Measure each prerelease in a CI lane"
date: "2026-10-10T18:25:09+00:00"
author: "Toby Clemson"
producer: "refine-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0219"
blocks: ["work-item:0326"]
relates_to: ["work-item:0189"]
tags: ["cli", "launcher", "performance", "measurement", "ci"]
last_updated: "2026-10-10T18:25:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0325: Measure each prerelease in a CI lane

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add a non-blocking CI lane that, after every `prerelease` run, measures the
prerelease it published against its baseline version on each of the four
GitHub-hosted platforms, and commits one record per platform to
`meta/measurements/`. This gives warm-dispatch latency a recurring owner and
the series every signal reads.

## Context

Child of 0219 — Track warm-dispatch latency per release in CI. `V`, platform
and session are as 0219's Vocabulary defines them, and the platform table
there names each runner label.

The harness measures the **published** release, and the `prerelease` job
bumps, signs, tags and publishes on every push to `main`, so a job triggered
on the merge commit would measure the previous merge's binary. The lane must
therefore follow `prerelease` and be told `V`. `prerelease` declares no
outputs today.

## Requirements

- `prerelease` exposes the tag it created as a job output.
- A matrix of measuring jobs, one per platform, with `needs: prerelease`,
  checks out `V` and runs a paired session with the git engine. It runs only
  after `prerelease`: no `schedule`, no `workflow_dispatch`. Neither stable
  releases nor the post-stable prerelease the `release` job publishes are
  measured.
- Measuring jobs are non-blocking, start with `timeout-minutes: 60`, and
  upload samples as workflow artefacts with 90-day retention.
- A platform whose measuring job does not succeed — unknown label, queue
  expiry or failure — is reported unmeasured with the job's conclusion; the
  other platforms proceed.
- A commit job runs on `always()` after the measuring jobs and makes one
  commit per prerelease to `main`, touching only `meta/measurements/` and
  carrying `[skip ci]`. Invalidated sessions are committed with their reason.
- The commit job joins the `accelerator-release` concurrency group with that
  group's identical settings and does nothing inside it but commit and push.
- A dedicated measurement GitHub App pushes the commit. Its token is minted
  only in the commit job and, later, the alerting job; the measuring jobs,
  which run fetched release binaries, never hold it.
- The lane writes a job summary listing each platform as measured, with its
  validity, or unmeasured, with the job's conclusion.
- The change that introduces the lane amends 0189's Latency Criterion: the
  primary regression signal becomes the paired `Δ_merge` against a published,
  signed baseline version, and C1-C4 become the user-perceptibility bound,
  judged on the cohort trend against the GitHub-hosted entries' ceilings.

## Acceptance Criteria

- [ ] Given `prerelease` publishes `V`, when the lane runs, then each record
      names `V` equal to the tag that `prerelease` run created.
- [ ] Given a platform's measuring job does not succeed, when the lane
      finishes, then the other platforms' records are committed and the job
      summary reports that platform unmeasured with the job's conclusion.
- [ ] Given the workflow definition, then the measuring jobs' only trigger is
      `needs: prerelease`, no `schedule` or `workflow_dispatch` reaches them,
      and the commit job's steps inside the `accelerator-release` group are
      limited to commit and push.
- [ ] Given all measuring jobs finish, when the commit job pushes, then exactly
      one commit lands on `main`, touching only `meta/measurements/` and
      carrying `[skip ci]`, and no `Main` workflow run starts from it.
- [ ] Given any session, valid or invalidated, then its record is committed
      and its samples are uploaded as an artefact with 90-day retention.
- [ ] Given the workflow definition, then the measurement App's token is
      minted only in the commit job (and the alerting job, once it exists),
      and the commit job declares the `accelerator-release` group with the same
      settings as `prerelease` and `release`.
- [ ] Given the change that introduces the lane, then 0189's Latency Criterion
      names the paired `Δ_merge` as the primary regression signal and C1-C4 as
      the cohort-trend bound against the GitHub-hosted entries' ceilings.

## Open Questions

None.

## Dependencies

- Blocked by: 0323, 0324.
- Blocked by, for the commit job only: a dedicated measurement GitHub App,
  being created by Toby Clemson, with `contents: write` and a branch-protection
  bypass on `main` (plus `issues: write` for 0326); its client ID and key stored
  as a repository variable and secret.
- Blocks: 0326.
- External system: GitHub-hosted runners for the four labels. A job for an
  unavailable label can queue for up to 24 hours, and Intel macOS is expected
  to lose support around Fall 2027.
- Coupled to the release pipeline: the commit job's time inside
  `accelerator-release` delays queued releases.

## Assumptions

- Prereleases cut locally with `mise run prerelease` enter the version
  sequence unmeasured, and the baseline-version rule treats them like any
  other predecessor without a record.

## Technical Notes

- `prerelease` (`.github/workflows/main.yml:552-675`) runs only on `push`,
  declares `concurrency: {group: accelerator-release, cancel-in-progress: false,
  queue: max}` (`.github/workflows/main.yml:582-585`) and has no `outputs:`.
  `release` repeats the group with a keep-in-sync comment
  (`.github/workflows/main.yml:724-727`).
- `check-measure-harness` (`.github/workflows/main.yml:380-422`) stays the
  harness's smoke owner; it uses unpinned `ubuntu-latest`/`macos-latest`
  labels, whereas the lane uses the four pinned labels.
- `measure:*` is outside `check` and `default` by design; the transitive-closure
  guard in `tests/unit/tasks/test_mise.py:253-303` must keep passing.
- Pushes made with an App installation token trigger workflows, which is why
  `[skip ci]` is load-bearing. `ACCELERATOR_RELEASER` is deliberately not
  reused, so the release credential never reaches a lane that runs fetched
  binaries.
- A queued job for an unavailable label can hold the commit job behind it,
  hence `always()`.
- The literal Ubuntu `ImageOS` values were not verified from source; the first
  lane run should print them.

## Drafting Notes

- The criterion amendment lands with the lane because the paired signal is not
  re-runnable until the lane exists to re-run it.

## References

- Parent: `meta/work/0219-own-the-recurring-absolute-budget-check.md`
- `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md` — the
  Latency Criterion this story amends
- `tasks/README.md#the-measure-namespace`
- `meta/research/issues/2026-06-14-release-concurrency-group-blocks-prereleases.md`
