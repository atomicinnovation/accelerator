---
type: "pr-description"
id: "249"
title: "[0365] Build the status-log states without recording where they were built"
date: "2026-11-19T18:58:43+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-957"
parent: "work-item:PP-957"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/249"
pr_number: 249
tags: ["build-system", "testing", "rust", "vcs"]
revision: "d85b373b3a6359b6b27e0fa8d06838904c1d23ec"
repository: "accelerator"
last_updated: "2026-11-19T18:58:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0365] Build the status-log states without recording where they were built

## Summary

A test can only work in a copy of the status-log states if no state
records the directory it was built in. Otherwise the copy still points
back at the original. This PR changes the status-log builders so that
none does, and adds a test that checks every state for its build
directory. Every golden is byte-identical. Nothing shares or copies the
states yet; that is Phase 5.

This is Phase 4 of 0365's plan, the first phase of its status-log
increment. It is stacked on #247 (the matrix increment) and is a draft
mainly so that CI's `ubuntu-latest` leg runs the new test and the
status-log goldens.

## Changes

- **A path scan.** `path_scan::files_naming(tree, needle)` lists every
  regular file under a tree whose bytes name a directory, and every
  symlink whose target does, in path order. It matches the needle both
  as given and as its canonical path, so `/var` and `/private/var` on
  macOS cannot hide each other. Phase 5's copy helper will use the same
  scan on every copy. `walkdir` becomes a workspace dependency; it was
  already in the lockfile.
- **Path-free status-log builders.** Each place a state recorded its
  build directory now records a relative path, or nothing:
  - `git-ahead` and `git-behind` are cloned with a command-scoped
    `-c core.logAllRefUpdates=false`, so the clone writes no
    `clone: from <path>` reflog entry. Their `remote.origin.url` is then
    set to `../origin.git`. The repository's own config still enables
    reflogs, so later commits are logged as before.
  - The bare `origin.git` points back at `../seed`.
  - `adversarial-git`'s `core.hooksPath` is `hostile-hooks`, which git
    resolves against the working tree, so it is the same directory.
  - The secondary jj workspace is added as `../jj-secondary`, so jj's
    operation log, which records each command line, names no absolute
    path.
- **The matrix's build split, for the status-log states.**
  `States::build_or_adopt` built, recorded and adopted in one body. The
  build now goes through three functions:
  - `build_with(base, env)` builds the full set with the hermetic
    environment it is given;
  - `build_in(base)` refuses a base inside a checkout, roots a
    hermetic environment there and builds;
  - `build_for_sharing(base)` builds and writes the manifest.

  `build_or_adopt` adopts or calls `build_for_sharing`, and stays until
  Phase 5 removes it.
- **`StatusLogManifest`.** The manifest is read and written through its
  own type in `status_log_manifest.rs`, beside `MatrixManifest`. The
  `status-log-states.tsv` format is unchanged.
- **A guard test.** `status_log_states::no_state_names_the_directory_it_was_built_in`
  builds the full set and scans every state, plus `seed`, `origin.git`
  and `jj-secondary-main`, for the build directory. The manifest and
  the `hermetic-*` entries are not scanned.
- **The plan and work item.** Phase 4's automated checks are ticked, and
  the status-log build's cost is recorded (see Notes for Reviewers).

## Context

- Work item: `meta/work/PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
- Plan: `meta/plans/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`,
  Phase 4
- Builds on #247, which builds the matrix once per run and hands it to
  the tests.

## Testing

- [x] Written test first: before the builder changes,
      `no_state_names_the_directory_it_was_built_in` failed, naming
      `git-ahead` and `git-behind` (config and three reflogs each),
      `origin.git/config`, `adversarial-git/.git/config` and one jj
      operation in `jj-secondary-main`. These are the files the plan
      predicted.
- [x] The status-log suites pass:
      `cargo nextest run --manifest-path cli/Cargo.toml -p vcs-test-support -p accelerator-vcs -p vcs-adapters`
      (310 tests).
- [x] Regenerating the goldens changes nothing:
      `REGENERATE_GOLDENS=1 cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-vcs -E 'test(status_and_log_match_every_captured_state)'`
      rewrote all 38 files, and `jj diff --stat cli/vcs-test-support/fixtures`
      is empty.
- [x] `mise run cli:check` passes.
- [x] `mise run` passes end to end (8,083 tests in `test:unit:cli`).
- [x] The path scan's own tests cover:
  - a tree that doesn't name the needle;
  - a file naming the needle as given;
  - a file naming its canonical spelling;
  - a symlink whose target names it;
  - several matches, returned in path order.
- [ ] On `ubuntu-latest` (CI's `test-unit` leg), the status-log goldens
      and the new scan test pass. This is why the PR is open as a draft.

## Notes for Reviewers

- **No behaviour change for consumers.** The only files that change are
  git config, reflog and jj operation-log files, which the status and
  log renderers don't read. The byte-identical goldens confirm it. After
  the build, nothing fetches from or pushes to the relative remotes.
- **One change from the plan.** The plan said to widen a shared
  `Manifest` to hold both families' rows. Since the plan was written,
  that type became `MatrixManifest`, matrix-only, on purpose. So the
  status-log states got their own `StatusLogManifest` beside it, rather
  than undoing that rename. The plan has been updated to say so.
- **Visibility.** `States::build_in` is `pub` for now, because the guard
  test calls it from `tests/`. `build_with` and `build_for_sharing` are
  `pub(crate)`, like the matrix's. Phase 5 moves the guard test onto
  the workbench and makes `build_in` `pub(crate)` too.
- **The status-log build's cost.** Timed on engywook, which was not
  idle (load average 27–29 on 16 cores):

  | Build | Median wall time |
  |---|---|
  | Status-log set alone | 2.4 s |
  | Matrix alone | 3.55 s |
  | Both together | about 4 s |

  Built together, the step takes as long as the matrix does, inside the
  build step's 5 s budget. So neither family needs to build its parts
  on separate threads. A rerun on an idle host is still owed and is
  noted in the work item.
- **Stacked base.** The base is `0365-build-vcs-fixtures-once` (#247).
  Retarget this PR to `main` once #247 merges. Phases 5 and 6 (sharing
  and copying the status-log states, then the closing measurement) can
  grow this PR into the full status-log increment, or follow it.
