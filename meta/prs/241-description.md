---
type: "pr-description"
id: "241"
title: "[0364] Record the closing CI comparison and close 0364"
date: "2026-11-18T10:05:05+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:0364"
relates_to: ["plan:2026-11-15-0364-parallelise-the-serial-test-suites"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/241"
pr_number: 241
tags: ["build-system", "testing", "performance", "ci"]
revision: "9bd4c48ec95c5fb7b29a08811a23d4be07fe7a87"
repository: "accelerator"
last_updated: "2026-11-18T10:50:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Record the closing CI comparison and close 0364

## Summary

This PR records Phase 7 of the 0364 plan, the closing CI comparison,
and closes the work item. Both suites' worker changes pass the CI rule
on ubuntu and macOS: with the change, each median is at most 5% above
the median without it. The two failures the comparison exposed were
fixed and merged in #238 and #239. Both suites are now landed and 0364
is done. Only `meta/` changes.

## Changes

- **`meta/prs/222-description.md` and `meta/prs/232-description.md`**:
  - Each gains a "Paired CI comparison" section: the run IDs, the
    medians, a per-attempt table with worker and CPU counts, the job
    wall-clock, the failures with their fixes, and the after window so
    far.
  - Stale boxes and "what's next" notes are brought up to date, and each
    file's frontmatter gains its `merge_commit`.
  - Both bodies are already posted to the merged PRs, #222 and #232.
- **The 0364 plan**:
  - Status `done`, and the Progress table brought up to date.
  - Phase 3's before-window box stays unchecked, because the window was
    never met, with a note that the paired comparison settled the rule.
    Its after window grows to seven runs.
  - Phase 6's anti-masking box now records a CI failure on #232's own
    branch that the earlier record missed.
  - Phase 7's three boxes are ticked, with the results.
- **Work item 0364**:
  - Status `done`, and Landing Status shows both suites landed.
  - All 15 acceptance criteria are ticked, each with a one-line pointer
    to its evidence.
- **`meta/validations/2026-11-15-0364-parallelise-the-serial-test-suites-validation.md`**
  (new): the plan's validation report, with result `pass`.

## Context

- Work item: `meta/work/0364-parallelise-the-serial-test-suites.md`
- Plan: `meta/plans/2026-11-15-0364-parallelise-the-serial-test-suites.md`
  (Phase 7)
- **Why a paired comparison.** Both CI before windows were contaminated,
  so Phase 7 compared the change in pairs instead.
  - Two draft PRs came from `main` at `a646f527`, #232's merge. #235
    kept both suites' workers, and #236 reverted #222's `-n auto` and
    #232's worker allocation.
  - The dev suite runs only in the integration job and Playwright only
    in the E2E job, on separate runners. So one pair of PRs covers both
    suites.
  - Each arm's `Main` run was re-run 10 times: run 44189947764 for #235
    and run 44189949799 for #236. Both PRs are now closed.

| Suite | OS | With workers | Serial | Change |
|---|---|---|---|---|
| Dev | ubuntu | 92.25 s | 174.00 s | −47.0% |
| Dev | macOS | 48.28 s | 223.78 s | −78.4% |
| Playwright | ubuntu | 168.69 s | 231.44 s | −27.1% |
| Playwright | macOS | 274.53 s (9 runs) | 313.88 s | −12.5% |

- **Failures.** The arm with workers failed 4 of its 11 runs, and the
  serial arm none. The dev suite itself passed 21/21 in all 22 runs.
  - **Playwright's scroll-hash spec** failed twice on macOS, and once on
    #232's own branch. #238 fixes it.
  - **The entrypoint lane's footprint test** failed twice on ubuntu,
    under the dev suite's xdist load. #239 fixes it.

## Testing

- [x] `accelerator corpus frontmatter validate` passes for the plan, the
      work item and both PR descriptions.
- [x] `mise run lint:corpus-identity:check` passes.
- [x] Every figure in the new records was checked against the collected
      run data. That includes the like-sized-runner medians and the
      after-window medians.
- [x] No code changes. The validation still ran the full `mise run` on
      `main` at `52f5d20f` with these records on top, and it exits 0:
  - The first run failed only `test:unit:cli`'s coverage floor, on stale
    profile data.
  - `cargo llvm-cov clean --workspace`, as the task advises, fixed it.
- [x] The JSON-reporter overlap check, rerun on `main`, prints `true` then
      `true`.

## Notes for Reviewers

- **A correction to the windows recorded earlier.** The "image" versions
  in Phases 3 and 6 (20261103.629 and 20261028.624) are the Hosted
  Compute Agent's version, not the runner image's. The runner images
  also change inside the windows, so the contamination verdicts stand.
- **macOS runner size varies.** Runners came with 3 or 5 CPUs, and that
  swings the timings more than the arm does. The records compare
  like-sized runners too, and the result holds: about −79% for the dev
  suite and −13.6% for Playwright.
- **The acceptance criteria were ticked by validation.** `validate-plan`
  checked each one against the shipped code, the timing records and the
  PR records before ticking it. The report lists the evidence.
- **The parent repository's `mise.toml` has jj conflict markers** from a
  rebase outside this work. The final `mise run` ignored it with
  `MISE_IGNORED_CONFIG_PATHS`, because this workspace's own `mise.toml`
  is the one that applies.
- **The after windows are incomplete.** They hold 7 `main` runs for the
  dev suite and 1 for Playwright. They are information only, because the
  paired comparison decided the rule.

https://claude.ai/code/session_01HEhrwWSAMeTH1BGPYLkEZP
