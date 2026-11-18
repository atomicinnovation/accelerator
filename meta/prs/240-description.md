---
type: "pr-description"
id: "240"
title: "Cancel superseded PR runs and gate merges on one CI check"
date: "2026-11-18T09:57:37+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/240"
pr_number: 240
tags: ["ci"]
revision: "0f97367623d0b3645c0d0eb0cbbd0591022a5753"
repository: "accelerator"
last_updated: "2026-11-18T09:57:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Cancel superseded PR runs and gate merges on one CI check

## Summary

PR CI is slow mostly because jobs queue, not because the work itself is slow.
The org is on GitHub's Free plan, which allows 20 jobs to run at once, and only
5 of them on macOS. Every PR run takes 4 of those macOS slots. This PR stops CI
from running work that nobody needs. It also adds a single `CI passed` gate job,
so the ruleset can require one check instead of 15, and the "up to date before
merging" rule can then be turned off.

## Changes

- **Editing a PR's title or description no longer re-runs CI.** The
  `pull_request` trigger drops `edited`. Nothing in the workflow, `tasks/` or
  `skills/` reads the PR title or body.
- **A new push to a PR cancels the run it supersedes.** A workflow-level
  concurrency group is keyed on the PR number, with `cancel-in-progress` for
  `pull_request` events only.
- **Each run on `main` gets a concurrency group of its own.** Outside PRs, the
  group is keyed on `run_id`. A `main` run can wait on release approval for any
  length of time. If `main` runs shared a group, that wait would hold back
  every later push, and GitHub keeps only one pending run per group, so queued
  pushes would be evicted and lose their prereleases. The job-level
  `accelerator-release` and `github-pages` groups are unchanged.
- **The measurement harness runs only on `main`.** It is non-blocking
  (`continue-on-error`) and does about a minute of work per run, but on PRs its
  macOS leg waited a median of 7 minutes for a macOS slot.
- **New `CI passed` gate job.** It needs the 12 jobs that block a PR and passes
  only if every one of them succeeded: a failed, cancelled or skipped job fails
  it. It runs with `if: always()`, because a required check that is skipped
  counts as passing. The job holds `permissions: {}` and checks out nothing.
- **Guard tests in `tests/unit/tasks/test_workflows.py`.** They cover the
  trigger, the two concurrency behaviours, the harness running only on `main`
  and the gate's name. One test fails if a new blocking job is added without
  being wired into the gate. The gate's pass/fail logic is tested by running
  the job's own shell script. The nightly-isolation guard now accepts the gate,
  alongside the release jobs, as a job allowed to depend on
  `check-architecture`.
- The E2E job's comment no longer says the ruleset requires its exact check
  name, since that stops being true once the ruleset requires only the gate.

## Context

Figures from two weeks of `main.yml` runs (5–17 Nov: 303 PR runs, about 8,300
jobs):

- macOS jobs on PRs waited a median of 5 minutes for a runner, and 42 minutes
  at p90. macOS jobs ran at the 5-job limit for 41% of the time any were
  running.
- From a PR opening to all required checks passing took a median of 34 minutes,
  and 88 minutes at p90.
- 86 PR runs (28%) re-tested a commit that had already been tested, almost all
  because of `edited`.
- 87 runs kept going after a newer push had made them obsolete.
- Together these wasted about 100 macOS hours (39% of macOS time on PRs) and 71
  Linux hours (30% of Linux time on PRs).

The "up to date before merging" rule adds to this. Every merge, and every
`[skip ci]` version bump from the release bot, makes every open PR out of date,
and updating one re-runs the whole suite.

Two further steps were considered and left out:

- **Running macOS on `main` only.** macOS caught 13 failures across 10 branches
  that Linux didn't, so it stays on PRs.
- **Running macOS only after Linux passes.** That would make every PR wait
  longer for a full result.

## Testing

- [x] Wrote the 11 new tests in `tests/unit/tasks/test_workflows.py` first and
  saw them fail, then made the workflow change and saw all 54 tests in the file
  pass.
- [x] Checked the gate's logic by running its step script with each kind of
  result: all `success` passes; any `failure`, `cancelled` or `skipped` fails.
- [x] `mise run build-system:check` passes, which includes actionlint over the
  new `concurrency` expressions and the gate job.
- [x] The full default task, `mise run`, exits 0. The first full run failed one
  timing test, `accelerator-research::arxiv_pacing
  a_killed_calls_request_still_spaces_the_next`, which measured 2.996 s against
  a 3 s minimum under the load of the whole parallel suite. It passed 3 times
  out of 3 on its own, and the next full run passed. This PR doesn't touch it.
- [ ] Confirm on GitHub that a second push to a PR cancels the first push's run.
- [ ] Confirm on GitHub that two pushes to `main` in quick succession both run
  to completion.

## Notes for Reviewers

**The ruleset changes after this PR merges, not before.** This PR's own run
still reports all 15 currently required checks, so it can merge under today's
ruleset. After it merges, ruleset `18227251` should be changed so that:

- `CI passed` is the only required status check;
- the "up to date before merging" rule
  (`strict_required_status_checks_policy`) is `false`.

**Each open PR then needs one new run** before it can merge, since its existing
runs have no `CI passed` check. A push or "Update branch" will do: `pull_request`
runs use the workflow from the PR merged with the current `main`. Re-running an
old run won't work, because a re-run keeps its original workflow file.

**With `always()`, a run cancelled by a newer push may still show a failed `CI
passed` on the old commit.** That's harmless, because the new commit gets its
own gate. The alternative, `!cancelled()`, would skip the gate on a run someone
cancels by hand, and a skipped required check counts as passing, so that commit
could merge untested.

**`prerelease` still lists its prerequisites itself rather than needing the
gate.** Switching it over is a separate change, because existing guard tests
check its `needs` list directly.
