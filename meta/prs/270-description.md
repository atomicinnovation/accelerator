---
type: "pr-description"
id: "270"
title: "[PP-943] Build the slowest CLI unit tests' repository states once and copy them"
date: "2026-11-26T08:05:19+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-943"
parent: "work-item:PP-943"
relates_to: ["plan:2026-11-23-PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/270"
pr_number: 270
tags: ["performance", "testing", "rust", "migrate", "vcs", "nextest"]
revision: "7b781c65e29d8fc03102710f64ca2e17f9b9b2dc"
repository: "accelerator"
last_updated: "2026-11-26T08:17:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-943] Build the slowest CLI unit tests' repository states once and copy them

## Summary

`test:unit:cli`'s slowest tests spent their time spawning `git` and `jj` to
rebuild the same starting state on every pass. This PR builds each state once
and copies it: `guarded_resume` copies a stalled migration the build step makes
once per run, each `rekey_run` tail test copies a committed template per VCS,
and `matrix_private_build` compares its private matrix with the run's shared
one instead of building a second. The nextest phase falls from 131.2 s to
121.5 s (median of 3, beyond noise).

## Changes

- **`stalled-migration` shared family.** A third shared fixture family, built
  in the build step from the `accelerator-migrate` binary's path, makes the 4
  distinct stalls (single parent or merge, pure or colocated jj).
  `guarded_resume`'s 26 scenario stalls become copies of them. Family builds
  now take inputs and print their own `built in` time.
- **Repository-state view.** `RepositoryState::of` projects a git or jj
  repository's state independent of IDs and times, reading jj with
  `--ignore-working-copy` so it never snapshots. Each reused state gets a test
  showing a copy equals one built from scratch.
- **`rekey_run` tail tests.** Each test builds one committed template per VCS
  and copies it per halt point. The edit-refusal test now halts at all 14
  mutations instead of 13. Rekey passes write through a test-only
  `UnsyncedStore`, which drops the `F_FULLFSYNC` calls that serialised parallel
  tests; eight contract tests hold it to `FileCorpusStore`'s behaviour.
- **Copy helpers in `vcs-test-support`.** `copy_private_tree` copies a test's
  own template. On macOS, `copy_part` clones a directory whole with
  `fclonefileat` (new `rustix` dependency) and restores what the clone drops.
  `each_of` fails a loop unless it completes its expected count, and
  `JjColocation` names the colocation.
- **`measure:test-durations`.** A new task prints per-test durations, binary
  totals, family build times and the last finishers from a `measure:build`
  record or from alone-run logs. `--charge` and `--served-by` combine figures
  within each run.
- **`measure:build` fix.** A timed run whose task printed only `ERROR task
  failed` was announced as `no task failed`, though the record stored it as
  failed. It now names the task.
- **Docs audit.** The expired `zod` advisory ignore is removed: the
  `astro-rehype-relative-markdown-links` override already resolves `zod` 4.6.7,
  past the advisory's range.

## Context

- Work item: `meta/work/PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite.md`
- Plan, with every phase's figures:
  `meta/plans/2026-11-23-PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite.md`
- Research: `meta/research/codebase/2026-11-23-PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite.md`
- Records: `meta/measurements/build_2026-11-26T012117Z_…` (before, at
  `403318ab`) and `build_2026-11-26T013622Z_…` (after).

## Testing

- [x] `mise run` exits 0 at the branch tip.
- [x] `mise run test:unit:cli` passes at each landing commit (`rekey_run`,
      `matrix_private_build`, `guarded_resume`).
- [x] `nextest list` before and after matches the plan's test-mapping tables:
      `guarded_resume` 13 → 15, `rekey_run` 29 → 40, `matrix_private_build`
      1 → 1, none renamed or removed.
- [x] `cargo nextest run` directly on each changed binary passes through the
      private-build fallbacks.
- [x] A held run's stalled-migration tree names no absolute path.

Figures (seconds, median of 3; checks charged to the test whose state they
verify):

| Figure | Before | After | Threshold |
|---|---|---|---|
| `guarded_resume` total + family build | 212.1 | 147.3 | ≤ 106.05 (missed) |
| `guarded_resume` slowest test | 30.6 | 16.9 | < 20 (met) |
| `rekey_run` kill test | 24.4 | 21.3 | ≤ 12.2 (missed) |
| `rekey_run` edit-refusal test | 18.2 | 11.5 | ≤ 9.1 (missed) |
| `matrix_private_build` alone | 6.6 | 3.2 | ≤ 3.3 (met) |
| `matrix_private_build` under the suite | 33.8 | 34.9 | ≤ 33.8 (missed) |
| Nextest phase | 131.2 | 121.5 | ≤ 131.2 (met) |

Every alone figure is under half its before median. The four under-suite
misses were reviewed under the work item's half-ratio rule and accepted as the
outcome.

## Notes for Reviewers

- **Measurement conditions.** Both records ran on AC with the IDE closed, so
  neither records IDE or `fsnotifier` CPU, against the README's advice; the
  author accepted this. The before record is a rebaseline at `403318ab`,
  because the rebase onto `main` brought in new `work-adapters` tests.
- **`matrix_private_build` under the suite** is the one figure that grew
  (+1.1 s) and is still the second-last finisher. It is the best candidate for
  follow-up work if the phase's end matters.
- **Unrelated commit.** "Sync work items with Linear" changes only `meta/` and
  `.accelerator/state/integrations/linear/last-sync.json`.
