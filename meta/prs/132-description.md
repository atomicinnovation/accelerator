---
type: "pr-description"
id: "132"
title: "[0241] Resume stalled migrate runs on jj and match jj status on dirty paths"
date: "2026-09-24T10:33:00+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0241"
parent: "work-item:0241"
relates_to: ["work-item:0263", "work-item:0286"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/132"
pr_number: 132
tags: ["migration", "vcs", "jj", "preflight"]
revision: "438bdf8a1fe5d6207c4538bb646436609d51f06d"
repository: "accelerator"
last_updated: "2026-09-24T10:33:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0241] Resume stalled migrate runs on jj and match jj status on dirty paths

## Summary

On jj, `accelerator migrate` recorded `@`'s commit id as the base guarding a
stalled run. That id changes on every snapshot, so after any `jj status` the
guarded resume never engaged. The documented recovery then hit a dirty-tree
refusal that listed nothing (#97).

This PR:

- bases the run on `@`'s parents;
- makes the refusal list the paths that blocked it;
- brings the jj dirty-path computation to parity with `jj status` on the git
  excludes and `snapshot.max-new-file-size`.

## Changes

### Migrate pre-flight (`cli/migrate`, `cli/migrate-adapters`, `cli/migrate-cli`)

- **Run base.** On jj, the **run base** is now `@`'s parent commit ids,
  sorted and joined by `+`. On git it is still `HEAD`, so a git stall
  recorded before the upgrade still matches.
  - It is read after the run lock is taken.
  - It comes from the same repository load as the dirty paths it guards,
    through the new `WorkingCopy` port and `WorkingCopyObservation`.
  - `MigrationContext::revision` is removed. Corpus stamps still use
    `@` / `HEAD`.
- **Refusal lists the unowned changes.** `PreflightError::ForeignDirt`
  becomes `PreflightError::UnownedChanges(UnownedChanges { paths, stale_run })`.
  The refusal now prints `Unowned changes (<n>):` and each path, byte-sorted,
  with control characters escaped.
  - When a recorded run base no longer matches, it says so and gives the
    recovery: commit the run's own output, then re-run without `FORCE`.
  - This closes 0263.
- **Decisions file is owned.** The decisions file the stall names
  (`migrations-<id>-decisions.txt`) is now an owned session artefact.
  Writing it no longer blocks the documented resume, on git or jj.
- **Warnings are visible.** `kernel::logging::init_if_requested()` gates the
  subscriber on `ACCELERATOR_LOG`. `accelerator-migrate` now calls it, so the
  working-copy adapter's `warn` is visible.

### jj dirty paths (`cli/vcs-adapters`)

The migrate pre-flight, `work sync` and `accelerator vcs status` all read
this computation.

- **Git excludes.** `git_excludes::base_ignores` feeds the snapshot the same
  base ignores jj-cli builds:
  - `core.excludesFile`, with `~`, relative and XDG resolution;
  - the XDG `git/ignore` default;
  - the backing repo's `info/exclude`.

  An unreadable source logs a `warn` and is skipped.
- **Size limit.** `jj_config` resolves `snapshot.max-new-file-size` as jj
  0.43 does for `jj status`, read-only and without `UserSettings`:
  - system, user (`JJ_CONFIG`, `conf.d`), repo and workspace layers, found
    through the per-id config layout and the legacy files;
  - conditional `[[--scope]]` / `--when` scopes, matched on repositories,
    commands, hostnames and environments.

  An invalid value logs a `warn` and falls back to no limit.
- **One read.** `InProcessProbe::working_copy_state` returns the base commits
  and dirty paths together. There is deliberately no standalone base-commits
  read.
- **Dependency.** `whoami` is added, for `--when.hostnames`. It is cleared
  through cargo-deny, and the third-party notices are updated.

### Tests and harness

- **Harness.** `vcs-test-support` gains:
  - a `jj status` parity oracle (`jj_status::changed_paths`,
    `run_base_oracle`);
  - `Hermetic` overrides for the git global config, `XDG_CONFIG_HOME` and
    `JJ_CONFIG`.
- **Out-of-process.** Every jj dirty-path assertion now runs out of process,
  under `Hermetic::apply`, through `vcs-adapters-fixture` or the new
  `work-adapters-fixture`. The status goldens and parity tests now spawn
  under `Hermetic` too.
- **New suites**: `guarded_resume`, `preflight_vcs_parity`, `base_commits`,
  `dirty_paths_parity`, `dirty_paths_excludes`, `dirty_paths_size` and
  `report_excludes`.

### Docs

- **`CHANGELOG.md`** gains three `### Fixed` entries.
- **`skills/config/migrate/SKILL.md`**:
  - says "run base";
  - doctests its stall example against the binary;
  - gives the stale-run recovery: commit, then resume without `FORCE`.

## Context

- **Work item**: `meta/work/0241-migrate-false-dirty-tree-detection.md`,
  which also closes 0263 (`meta/work/0263-rename-foreigndirt.md`).
- **Research**:
  `meta/research/codebase/2026-09-23-0241-migrate-false-dirty-tree-detection.md`
- **Plan**: `meta/plans/2026-09-23-0241-migrate-false-dirty-tree-detection.md`,
  with review `meta/reviews/plans/2026-09-23-0241-migrate-false-dirty-tree-detection-review-1.md`.
- **Validation (`pass`)**:
  `meta/validations/2026-09-23-0241-migrate-false-dirty-tree-detection-validation.md`
- **Issue**: #97

## Testing

- [x] `mise run` exits 0 end to end: 3,339 CLI tests passed, 1 skipped. The
      formatters changed no file.
- [x] **Guarded resume on jj.** It is exercised for `--no-colocate` and
      `--colocate`:
      - it survives `jj status`, owned edits, `jj describe @` and merges;
      - it is reported stale after a rewrite or rebase of a parent;
      - a move onto new parents starts a fresh run (`jj new`, `jj commit`,
        `jj edit`);
      - a stall recorded before the upgrade is refused until its output is
        committed.
- [x] **`jj status` parity.** The dirty paths equal `jj status`'s change
      lines for every excludes case, size limit and conditional scope. Each
      of migrate, `work sync` and the renderer is checked with a control
      file that must still be reported.
- [x] **Mutation check.** With `core.excludesFile` resolution disabled, the
      global-excludes consumer tests fail.
- [ ] **Manual: #97 reproduction.** In a real colocated jj repo, stall 0007,
      write the decisions file, run `jj status`, then re-run with
      `--decisions-file`. 0007 completes without `FORCE`.
- [ ] **Manual: pre-upgrade stall.** Stall on the pre-change binary, upgrade,
      then follow the `CHANGELOG.md` recovery.
- [ ] **Manual: excludes.** With a global `core.excludesFile` ignoring
      `*.scratch`, `accelerator vcs status` and `jj status` agree on a new
      `meta/x.scratch`.
- [ ] **Manual: size limit.** With `jj config set --repo
      snapshot.max-new-file-size 1KiB`, a new 2 KiB `meta/` file:
      - is absent from `accelerator vcs status`;
      - does not make `accelerator migrate` refuse;
      - shows as `?` in `jj status`.
- [ ] **Manual: no config written.** `accelerator vcs status` in a repo
      without a jj config id creates no `repos/<id>` directory.

## Notes for Reviewers

- **Behaviour changes users will notice:**
  - A jj run that stalled before this upgrade is refused as stale. The
    refusal and `CHANGELOG.md` give the recovery: commit, then re-run
    without `FORCE`.
  - On jj, untracked `meta/` files that the git excludes ignore, or that
    exceed `snapshot.max-new-file-size` (1 MiB when unset), no longer count
    as changes, as in `jj status`. The migrate pre-flight therefore no
    longer protects them.
- **Where to look:**
  - `cli/vcs-adapters/src/library/jj_config.rs` mirrors jj-cli 0.43's config
    layout and scope resolution. The jj-lib pin comment in `cli/Cargo.toml`
    now lists it for re-verification on a bump.
  - `cli/migrate/src/preflight.rs` holds the ownership and stale-run logic.
- **Known divergences, accepted or deferred:**
  - **Fail-open (unchanged policy).** A failed working-copy read, without
    force, is still treated as a clean tree. It truncates a stalled run's
    manifest, visible only as a `warn`.
  - **Malformed config id.** jj fails on a malformed config id. Here the
    layer is dropped silently, which can under-report changes. A `warn` is
    a candidate follow-up.
  - **Relative `core.excludesFile`.** It is resolved as jj-cli does but not
    verified against the binary. The parity tests cover it.
- **Related work.** 0286 owns the broader rewrite of the migrate skill's
  dirty-path guidance. This PR changes only the stale-run recovery sentence,
  because the old one contradicted the new refusal.
- **Other files in the diff.** It also carries the 0241 work-item, research,
  plan, review and validation documents. It includes a one-line Linear
  sync-state bump and a cross-reference to 0241 in 0286.
