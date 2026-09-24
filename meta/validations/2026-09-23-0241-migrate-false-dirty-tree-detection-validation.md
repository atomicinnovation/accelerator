---
type: "plan-validation"
id: "2026-09-23-0241-migrate-false-dirty-tree-detection-validation"
title: "Validation Report: False Dirty-Tree Detection on jj Repositories Implementation Plan"
date: "2026-09-24T09:12:57+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-23-0241-migrate-false-dirty-tree-detection"
tags: ["migration", "vcs", "jj", "preflight"]
last_updated: "2026-09-24T10:05:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: False Dirty-Tree Detection on jj Repositories Implementation Plan

Every production behaviour the plan specifies is implemented, and `mise run`
exits 0 with no formatter churn. The first pass returned `partial`. Four gaps
were closed before this re-validation:

- one test still computed jj dirty paths in the test process;
- two excludes consumer tests could not detect a `core.excludesFile`
  regression;
- several test-table rows were dropped;
- a `SKILL.md` sentence still sent users to `FORCE`.

The remaining deviations are structural or improve on the plan.

### Implementation Status

✓ Phase 1: Name the Unowned Changes - Fully implemented
✓ Phase 2: Derive the Run Base from `@`'s Parents - Fully implemented
✓ Phase 3: Honour the Git Excludes in the jj Dirty Paths - Fully implemented
✓ Phase 4: Honour `snapshot.max-new-file-size` in the jj Dirty Paths - Fully implemented

### Automated Verification Results

✓ `mise run` exits 0 end to end. It includes `check`, `public-api:check`,
  `lint:vcs-settings:check`, cargo-deny and `docs:check`. The formatters
  changed no file.
✓ CLI suite: 3,339 tests run, 3,339 passed, 1 skipped. This includes the
  `bash-parity` suites `guarded_resume`, `dirty_tree_preflight`,
  `preflight_vcs_parity`, `base_commits`, `dirty_paths`,
  `dirty_paths_parity`, `dirty_paths_excludes`, `dirty_paths_size`,
  `report_excludes` and `sync_working_copy_status`.
✓ Other suites: frontend unit (2,610) and visualiser e2e (355), plus the
  integration and visualiser unit suites.
✓ `rg -n 'ForeignDirt|Ownership::Foreign|(?i)foreign[ _-]?dirt' cli skills`
  returns nothing.
✓ Excludes consumer tests catch a regression. With `core.excludesFile`
  resolution disabled in `git_excludes::base_ignores`, both global-excludes
  consumer tests fail and both `info/exclude` tests pass. The source was
  then restored.
✓ `meta/work/0263-rename-foreigndirt.md` is `done`.

### Code Review Findings

#### Matches Plan:

- **Unowned changes**: `UnownedChanges { paths, stale_run }`
  (`cli/migrate/src/preflight.rs:29-39`) is byte-sorted, and its stale-run
  rule matches the plan.
- **Refusal rendering**: `render::unowned_changes_refusal`
  (`cli/migrate-cli/src/render.rs:20-43`) escapes only control characters.
- **Decisions file**: `-decisions.txt` is in `SESSION_SUFFIXES`, and the
  stall builds its path through `manifest::decisions_file`.
- **One read**: `WorkingCopyState` gives base commits and dirty paths from
  one read.
  - On git, `HEAD` and the status walk share one `gix::Repository`.
  - On jj, the parent ids come from the same `load_at_head` view as the
    diff.
- **Ports**:
  - `RunBase`, the `WorkingCopy` port and `WorkingCopyObservation` are in
    place.
  - `MigrationContext::revision` is gone.
  - The pre-flight takes the lock before `observe`, and a test pins that
    order.
- **Logging**: `kernel::logging::init_if_requested` is used by `vcs-cli`,
  `accelerator-migrate` and both fixture binaries.
- **Git excludes**: `git_excludes::base_ignores` is wired into the snapshot,
  and the `excludes_file_path` table is complete.
- **Size limit**: `JjConfigEnvironment`, `JjConfigSources`,
  `per_id_config_file`, `MaxNewFileSize` and `UnresolvableLimit` match the
  plan. The layer order is System, User, Repo, Workspace, as in jj-lib
  0.43.0.
- **No in-process jj reads in tests**: no test computes jj dirty paths,
  base commits or status in the test process. `dirty_paths.rs` now runs
  every case, git included, through `vcs-adapters-fixture`.
- **Dependencies**: `whoami` is added, `wit-bindgen` is in the deny
  build-script snapshot, and the third-party notices are updated.
- **`CHANGELOG.md`**: the three `### Fixed` entries match the plan verbatim.

#### Deviations from Plan:

- **Stall wording changed.** Commit `wtvxyntk` changed "base revision" to
  "run base" and doctests the `SKILL.md` stall example against the binary.
  The plan had deferred that rewording. The change matches the plan's
  Terminology section.
- **`SKILL.md` recovery rewritten beyond the plan.**
  `skills/config/migrate/SKILL.md` step 4 now gives the stale-run recovery:
  commit the run's own output, then re-run without `FORCE`. The plan
  scoped this file to a wording swap. The old sentence contradicted the new
  refusal, so it had to change. 0286 still owns the broader guidance.
- **Renamed and reshaped types**:
  - `SnapshotDiff` is `WorkingCopySnapshot`.
  - `JjConfigSources::resolved` is split into `stacked()` plus a resolve
    inside `MaxNewFileSize::resolve`.
  - `chain_or_warn` is `chained_or_skipped`, and
    `global_git_config_excludes_file` is `global_excludes_file`.
- **No `backing_git_repo` warn branch.** It would be unreachable: in jj-lib
  0.43.0, `get_git_repo`'s only error is `UnexpectedGitBackendError`.
- **Test locations**:
  - `RunBase` tests are in `cli/migrate/tests/run_base.rs`.
  - `stalled_0007` is `Stalled::on`, local to `guarded_resume.rs`.
  - `common::decisions_file_for` re-spells the decisions-file format as an
    independent oracle.
- **Remaining test-shape differences**:
  - `base_commits.rs` uses `jj new root()` for the "another parent" row, and
    runs corrupt `op_heads` only `--no-colocate`.
  - `dirty_paths_size.rs` case 4 tests value forms at the user layer.
  - The work-sync size cases run only `--no-colocate`.
  - The merge resume in `guarded_resume.rs` runs without `--decisions-file`,
    so the manifest survives for the stale step that follows.

#### Gaps closed in this pass:

- **In-process test.** `cli/vcs-adapters/tests/dirty_paths.rs` now runs
  every case through `support::dirty_paths`. Previously
  `jj_excludes_an_ignored_file` and three git cases ran in-process and could
  read the developer's environment.
- **Excludes consumer tests.** Each was split into a global
  `core.excludesFile` test and a backing `info/exclude` test:
  - `a_work_item_the_global_excludes_file_hides_is_clean` and
    `a_work_item_the_backing_info_exclude_hides_is_clean` in
    `sync_working_copy_status.rs`;
  - `a_file_the_global_excludes_file_hides_is_not_listed` and
    `a_file_the_backing_info_exclude_hides_is_not_listed` in
    `report_excludes.rs`.

  `bash-parity-baseline.txt` records the new count.
- **Renderer size tests.** The 1 KiB case now includes `meta/half`. A new
  `a_new_file_over_the_default_limit_is_not_listed` covers the unset limit.
- **Work-sync `"abc"` case.** It now also edits a tracked item and asserts
  both files `dirty`.
- **Fresh-run variants.** `moving_onto_new_parents_starts_a_fresh_jj_run`
  covers `jj new`, `jj commit -m z` and `jj edit` to a change on the
  sibling. It asserts that the recorded run base moved and equals the
  oracle.
- **Merge test.** It edits an owned file before the resume.

#### Potential Issues:

- **Fail-open erases a stalled run's resume state.** This is plan-sanctioned.
  An `observe` error without force takes the clean branch, truncates the
  manifest and records an empty run base. Only a `warn` shows it.
- **Malformed config id diverges from jj.** jj fails with
  `BadConfigIdError`, but `per_id_config_file` drops the layer silently.
  That can under-report changes. The plan specified `None`; a `warn` would
  make it visible.
- **Mode-000 test fails as root.** `dirty_paths_excludes.rs:252` relies on
  the file being unreadable, which root ignores. This matters only if CI
  ever runs as root.

### Manual Testing Required:

1. Guarded resume on jj:
  - [ ] In a real jj-colocated repo, stall 0007, write the decisions file,
        run `jj status`, then re-run with `--decisions-file`. 0007 completes
        without `FORCE`.
  - [ ] Stall on the pre-change binary, upgrade, and follow the
        `CHANGELOG.md` recovery. The refusal is stale, and after committing
        the run's output the re-run resumes without `FORCE`.

2. Refusal output:
  - [ ] In a scratch git repo with an unowned `meta/x.md`,
        `accelerator migrate` prints the refusal, `Unowned changes (1):`
        and `  meta/x.md`.
  - [ ] The `CHANGELOG.md` entries read correctly at 80 columns.

3. `jj status` parity:
  - [ ] With a global `core.excludesFile` ignoring `*.scratch`,
        `accelerator vcs status` and `jj status` agree on a new
        `meta/x.scratch`.
  - [ ] With `jj config set --repo snapshot.max-new-file-size 1KiB` and a
        new 2 KiB `meta/` file:
        - the file is absent from `accelerator vcs status`;
        - `accelerator migrate` does not refuse over it;
        - `jj status` lists it as `?`.
  - [ ] Outside any repo, run `jj config path --user` and list its `repos/`
        directory. Run `accelerator vcs status` in a scratch repo with no
        config id, then list `repos/` again. No new id directory appears.

### Recommendations:

- **Warn on a malformed config id.** Log a `warn` when
  `per_id_config_file` rejects a malformed id.
- **Track the fail-open erasure.** Consider a follow-up work item for the
  case where a failed working-copy read erases a stalled run's manifest.
- **Mention the `SKILL.md` change in the PR.** The PR description should
  cover the rewritten recovery guidance, since it goes beyond the plan's
  scope for that file.
