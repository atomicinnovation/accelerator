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
revision: "47500a3965f38ef2772d31debde2f84418aaef92"
repository: "accelerator"
last_updated: "2026-11-26T16:25:36+00:00"
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
one instead of building a second. On macOS the nextest phase falls from
131.2 s to 121.5 s (median of 3, beyond noise); one ubuntu CI sample shows the
same tests gaining by a similar ratio.

## Changes

- **`stalled-migration` shared family.** A third shared fixture family, built
  in the build step from the `accelerator-migrate` binary's path, makes the 4
  distinct stalls (single parent or merge, pure or colocated jj).
  `guarded_resume`'s 26 scenario stalls become copies of them. Migrate's test
  knowledge lives behind `stalled_migration`: the corpus, how a test runs
  migrate, and the one ordered ledger in `stalling_corpus::MIGRATIONS`. The
  manifest names stalls relative to its build, and `MigrateBinary` reads the
  binary the build step hands over.
- **Repository-state view.** `RepositoryState::of` projects a git or jj
  repository's state independent of IDs and times, reading jj with
  `--ignore-working-copy` so it never snapshots. It records whether git's
  `HEAD` is attached and which commit jj's `@` is, orders siblings without
  letting IDs decide, and refuses content that is not UTF-8. Its module doc
  lists what it leaves out. Each reused state gets a test showing a copy equals
  a scratch build.
- **`rekey_run` tail tests.** Each test builds one committed template per VCS
  and copies it per halt point; the kill test runs as one test per VCS. The
  halt loops are pinned to 13 and 14 mutations, and the edit-refusal test now
  halts at all 14 instead of 13. Rekey passes write through a test-only
  `UnsyncedStore`, which drops the `F_FULLFSYNC` calls that serialised parallel
  tests; nine contract cases hold it to `FileCorpusStore`'s behaviour.
- **`store::create_without_sync`.** The `store` crate gains `atomic_create`
  without its directory fsync, beside `replace_without_sync`, and
  `corpus_adapters::store::to_store_error` is public, so `UnsyncedStore`
  composes production write logic instead of copying it.
- **Copy helpers in `vcs-test-support`.** `copy_private_tree` copies a test's
  own template. On macOS, `copy_part` clones a directory whole with
  `fclonefileat` (a new macOS-only `rustix` dependency) and finishes the clone
  as an entry-by-entry copy would. `each_of` fails a loop unless it completes
  its expected count, and `JjColocation` names the colocation.
- **nextest priority.** The kill tests and `matrix_private_build` start first,
  so they no longer finish last.
- **`measure:test-durations`.** A new task prints per-test durations, binary
  totals, family build times and the last finishers from a `measure:build`
  record or from alone-run logs. `--charge` and `--served-by` combine figures
  within each run, and a figure no run can supply is refused.
- **`measure:build` fix.** A timed run whose task printed only `ERROR task
  failed` was announced as `no task failed`, though the record stored it as
  failed. It now names the task.
- **Docs.** `tasks/README.md` gains a "Registering a shared fixture family"
  checklist and covers the copy guard's allow-list. The expired `zod` advisory
  ignore is removed: the `astro-rehype-relative-markdown-links` override
  already resolves `zod` 4.6.7, past the advisory's range.

## Context

- Work item: `meta/work/PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite.md`
- Plan, with every phase's figures:
  `meta/plans/2026-11-23-PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite.md`
- Research: `meta/research/codebase/2026-11-23-PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite.md`
- Review: `meta/reviews/prs/270-review-1.md`
- Records: `meta/measurements/build_2026-11-26T012117Z_…` (before, at
  `403318ab`) and `build_2026-11-26T013622Z_…` (after).

## Testing

- [x] `mise run check` exits 0 at the branch tip.
- [x] The full `test:unit:cli` passed twice after the review fixes, before the
      last two commits (a non-macOS clone stub and the nextest priority).
- [ ] `mise run` not re-run since the review fixes; see Notes.
- [x] `nextest list`: `guarded_resume` 13 → 15, `rekey_run` 29 → 42,
      `matrix_private_build` 1 → 1. The kill test became `…_in_git` and
      `…_in_jj`, and the two template-copy checks became
      `…_matches_its_template`; no test was removed.
- [x] Linux clippy passes for `store`, `corpus-adapters` and
      `vcs-test-support`; `accelerator-migrate` and `work-adapters` need CI,
      since `ring` will not build for Linux on a Mac.
- [x] A held run's stalled-migration tree names no absolute path.

Figures (seconds, macOS, median of 3; checks charged to the test whose state
they verify). They were taken before the review round, so the kill test is
still one test:

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

One ubuntu CI sample, this PR against a sibling branch without it:

| Figure | Before | After |
|---|---|---|
| `guarded_resume` test-seconds + family build | 24.8 | 9.0 |
| `guarded_resume` slowest test | 4.26 | 1.26 |
| `rekey_run` kill test | 2.83 | 1.41 |

## Notes for Reviewers

- **Measurement conditions.** Both records ran on AC with the IDE closed, so
  neither records IDE or `fsnotifier` CPU, against the README's advice; the
  author accepted this. The before record is a rebaseline at `403318ab`,
  because the rebase onto `main` brought in new `work-adapters` tests.
- **The ubuntu sample** is one run each from different branches, so it shows
  the gain holds where copies go entry by entry, not its size.
- **The nextest priority** is kept on mechanism, not on a measured phase time:
  the host was too loaded to time it. The tail tests moved from the last
  finishers to within the first 15%.
- **`github::octocrab_client` flakes under load.** Its client takes 12 s to
  build for 10 tests even alone, and it failed in 4 of 6 full runs while the
  host was busy. This PR does not touch it.
- **Unrelated commit.** "Sync work items with Linear" changes only `meta/` and
  `.accelerator/state/integrations/linear/last-sync.json`.
