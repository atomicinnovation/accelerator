---
type: "codebase-research"
id: "2026-11-23-PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite"
title: "Where the time goes in PP-943's target tests, and what can shorten them"
date: "2026-11-23T02:05:50+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "PP-943"
parent: "work-item:PP-943"
relates_to: ["codebase-research:2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run", "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"]
topic: "Where the time goes in PP-943's target tests, and what can shorten them"
tags: ["research", "codebase", "performance", "testing", "nextest", "migrate-cli", "work-adapters", "vcs-test-support", "tasks"]
revision: "7ad0cdfb87d014fd389ca1db6663013fbead44e4"
repository: "accelerator"
last_updated: "2026-11-23T11:39:41+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added the author's decisions on the review timing, a shared stalled-migration family and the missing halt point"
schema_version: 1
---

# Where the time goes in PP-943's target tests, and what can shorten them

**Date**: 2026-11-23 02:05 UTC
**Author**: Toby Clemson
**Git Commit**: `7ad0cdfb87`
**Branch**: detached working copy (`main` is at `4a0579d7cc`)
**Repository**: accelerator

## Research Question

For PP-943: how do `accelerator-migrate::guarded_resume`,
`vcs-test-support::matrix_private_build` and the two `work-adapters::rekey_run`
tail tests spend their time? How much of it is their own work and how much
comes from contention? What in the codebase can be reused or extended to
shorten them, within the work item's constraints?

## Summary

All three binaries spend their time spawning `git` and `jj` processes one after
another. Most of those processes rebuild a starting state that is the same in
every pass, apart from IDs and the temp path.

- **`guarded_resume`** builds a stalled migration from scratch 26 times, with
  7 `jj` calls and 1 migrate run each time. That setup is 208 of the binary's
  ~317 spawns (66%), yet it produces only 4 distinct states: SingleParent or
  MergeWithSibling, under each colocation.
- **`rekey_run`** builds a fresh committed repository for each of the
  13 halt points: 26 passes and 91 spawns in the kill test, 13 passes and 78
  spawns in the edit-refusal test. The repository before the halt is the same
  in every pass.
- **`matrix_private_build`** runs two full fixture-matrix builds one after the
  other: the private build, then an in-process `shared_build`. It compares only
  the manifests, not repository contents.

Contention takes up more than half of most targets' time under the suite. I
ran each binary alone three times at `7ad0cdfb87`. That is not the baseline
record's commit, and the host had not been quiesced, so these are indicative
figures only. On these figures, four of the five checks named in the work
item's Assumptions trigger an author review before planning:

| Target | Alone median (s) | Under suite, reference record median (s) | Alone ÷ suite |
|---|---|---|---|
| `guarded_resume` total test-seconds | 118.0 | 228.6 | 0.52 |
| `guarded_resume` slowest test (`moving_onto_new_parents…`) | 14.4 | 34.7 | 0.42 ⚠️ |
| `matrix_private_build` test | 6.3 | 32.0 | 0.20 ⚠️ |
| `rekey_run` `a_run_killed_at_every_write_resumes…` | 9.4 | 22.4 | 0.42 ⚠️ |
| `rekey_run` `a_resume_refuses_an_edit…` | 5.6 | 16.0 | 0.35 ⚠️ |

⚠️ marks a ratio under 0.5, which the Assumptions say triggers an author
review. The `guarded_resume` slowest test is already under the 20 s ceiling
when run alone. The ceiling is breached only through contention.

The phase-end payoff from the tail tests is capped. Once the three tail tests
are left out, the latest finisher in each run ends 9.3, 7.2 and 9.2 s before
the phase does. So no change to the tail tests can take more than ~7–9 s
(5–7%) off the nextest phase.

## Detailed Findings

### Timeline of the reference record

I parsed the `PASS [ Ns ]` lines in `run-{1,2,3}-test_unit_cli.log` under
`.accelerator-measure/build/build_2026-11-20T130237Z_engywook_warm-no-op_devtools_isolated/`.
Each line carries a wall-clock stamp, so start = stamp − duration. Times below
are seconds after nextest's `Starting` line.

```text
                       run 1          run 2          run 3
guarded_resume         start 6–7      start 7–8      start 9–10   (all 13 tests at once)
  slowest ends         37.1           42.5           45.6
matrix_private_build   94.1 → 126.1   113.2 → 143.0  108.2 → 144.0
rekey kill test        103.4 → 125.8  122.0 → 142.8  120.1 → 143.8
rekey edit test        103.1 → 119.1  121.9 → 137.7  119.9 → 138.7
last other finisher    116.8          135.8          134.8
tests running at end   2              2              2–3
```

- **Saturation:** 16 tests run at once (nextest's default test-threads) almost
  until the end. `guarded_resume` takes 13 of the 16 slots during the first
  ~40 s.
- **Late start:** the tail tests start 94–122 s in. They end the phase because
  they start late and run long, not only because they are long.
- **Coupled end:** in all three runs, `matrix_private_build` finished
  0.2–0.3 s after the `rekey_run` kill test. I could not tell whether this is
  coincidence (CPU freed when the rekey test finishes) or a real dependency.
- **Ordering:** nextest 0.9.138 (pinned in `mise.toml:32`) appears to start
  binaries in list order, so `vcs-test-support` and `work-adapters` come last.
  `cli/.config/nextest.toml` sets no test-threads, priorities, test groups or
  overrides.

### `guarded_resume` (`cli/migrate-cli/tests/guarded_resume.rs`)

**Stalled-state builder.** `Stalled::on(colocation, shape)` (`:144-180`) runs
these steps:

1. `jj --version`, from `assert_jj_matches("0.43.0")`.
2. `tempdir` and `Hermetic::rooted_at`.
3. `jj git init <colocation>`.
4. Seed the corpus and the ledger 0001–0005 (`common/mod.rs:167-196`).
5. `jj commit -m corpus`.
6. `jj log` to read `base`.
7. `jj describe -m sibling`.
8. Write `meta/sibling.md`.
9. `jj log` to read `sibling`.
10. `jj new <base>` for SingleParent, or `jj new <base> <sibling>` for
    MergeWithSibling.
11. Run the migrate binary, asserting a stall (exit 1).

The stall comes about naturally. Stdin is null, so migration 0007 hits
`DecisionError::NoInputAvailable` (`cli/migrate/src/engine.rs:252-260`). The
binary reads jj through jj-lib in-process and never records an operation, so
the migrated files stay unsnapshotted on disk until the next `jj` call.

**Builds per test.** Ten tests loop over `COLOCATIONS` (`:125`). Some loop
again inside that:

| Test | Builds | Spawns (jj + migrate) | Alone (s) |
|---|---|---|---|
| `moving_onto_new_parents_starts_a_fresh_jj_run` (`:386`) | 6 (2 colocations × 3 moves) | 58 + 12 | 14.4 |
| `rewriting_or_rebasing_the_parent_makes_a_jj_stall_stale` (`:319`) | 4 (2 colocations × 2 rewrites) | 32 + 8 | 12.6 |
| `a_clean_jj_run_records_the_parents_of_the_working_copy` (`:484`) | 2 | 30 + 6 | 12.2 |
| `a_stall_recorded_before_the_upgrade_is_refused…` (`:543`) | 2 | 20 + 8 | 11.3 |
| `a_stall_on_a_merge_resumes…` (`:458`) | 2 (MergeWithSibling) | 20 + 6 | 10.6 |
| six other loop tests (`:247`, `:269`, `:291`, `:409`, `:431`) | 2 each | 14–18 + 4 | 7.9–10.7 |
| `an_unreadable_jj_working_copy…`, `force_over_an_unreadable…` (`:79`, `:108`) | own builder, `:61-76` | 3 + 1 | ~2.5 |
| `a_git_stall_resumes…` (`:17`) | git, inline | 5 git + 2 | ~1.9 |

In total that is 26 `Stalled::on` builds: 24 SingleParent and 2
MergeWithSibling. The builds account for 182 of the ~248 `jj` spawns and 26 of
the 64 migrate runs. Alone, the slowest test runs ~70 spawns in 14.4 s, about
0.2 s per spawn. Under the suite it runs them in ~35 s, about 0.5 s per spawn.

**What a copy must carry** if the state is built once and copied:

- **On disk:** the seeded corpus; `.accelerator/state/migrations-applied`
  (0001–0006); `migrations-run-paths.txt` (the manifest); `migrations-run.id`
  (the run base, a sorted list of parent commit ids joined with `+`); the
  rewritten `OWNED_NOTE`; and `.jj/`, plus `.git/` for `--colocate`.
- **In memory:** the `base` and `sibling` change IDs, which tests 7–12 use.
  A copy would need a sidecar file or a `jj log` re-query to recover them.
- **Unverified:** whether jj 0.43 writes absolute paths into
  `.jj/repo/store/git_target` or a colocated `.git/config`.
  `copy_part`'s `named_by` scan would catch an absolute path.

**What does not block a copy.** No assertion compares a literal change id,
commit hash, op id or timestamp. Every one is captured and compared live.
Every stderr assertion uses repo-relative paths.

**Shared helpers.** These helpers in `common/mod.rs` are used only by
`guarded_resume`: `seed_ambiguous_reference`,
`seed_output_then_ambiguous_reference`, `decisions_file_for`, `OWNED_NOTE`,
`MIGRATION_0007` and `mark_migrations_before_000{6,7}_applied`. Other binaries
also use `run`, `tempdir`, `write`, `Outcome`, `TestError` and
`applied_ledger`: `dirty_tree_preflight`, `full_registry_over_git`,
`migration_0011`, `preflight_vcs_parity` and `run_lock`.

### `rekey_run` tail tests (`cli/work-adapters/tests/rekey_run.rs`)

**Halt points.** `mutations_of_a_whole_run()` (`:91-96`) counts the mutations
of an apply over a scripted, VCS-free repository. By reading the code, it is
**13**:

- 3 setup mutations: two `.gitignore` files and the first plan save;
- per entry, a plan save followed by that entry's file writes, across the two
  entries;
- the final plan removal.

A halt is in-process: `Fault::HaltAt` calls `resume_unwind(Halted(n))` before
mutation n (`tests/support/mod.rs:243-278`), and `halted_at` catches it
(`:496-500`). Nothing is spawned except `git` and `jj`.

**Per pass.** `committed(vcs)` (`:860-875`) runs these processes:

- **Git (4):** `git --version`, `git init`, `git add -A`, `git commit`.
- **jj (3):** `jj --version`, `jj git init --no-colocate`, `jj commit`.

After that come the halting apply and the resuming apply. Each opens the
repository once through `InProcessProbe` (gix or jj-lib).

**Per test.**

- **Kill test** (`:927-946`): Git and jj × 13 halts gives 26 passes and
  91 spawns. It asserts `Rekeyed`.
- **Edit-refusal test** (`:948-978`): Git × 13 halts, 78 spawns. It commits an
  extra `synced-only.md` file, then skips any pass whose halt left nothing to
  check. That is roughly halts 0–2 and 9–12, about half the passes. The test
  asserts `checked > 0`.

⚠️ The edit-refusal test's fixture has a 5th file, so its own run has about 14
mutations, but the loop still counts from the 4-file fixture. Its last halt
point is never exercised. This is a pre-existing gap, and it matters for the
Test-mapping criterion: halt points must be derived from
`mutations_of_a_whole_run()`.

**Reuse.** The repository before each halt is the same in every pass, apart
from the temp path and the commit or change IDs, and no assertion reads those.
The passes share no global state: there are no `set_current_dir` or `set_var`
calls, and lock directories live in each pass's own repository. `Mutations` and
`Ports` use `Rc` and `RefCell`, so each thread must build its own. `Repo`
wraps a private `TempDir` (`support/mod.rs:136-138`), so adopting a copied
directory needs a new constructor.

**Unknown split.** In-process work is part of each pass: the two apply runs,
plus jj-lib/gix loads. Alone, each pass takes ~0.36 s. I did not profile how
much of that is spawns and how much is in-process work.

### `matrix_private_build` (`cli/vcs-test-support/tests/matrix_private_build.rs`)

1. `Workbench::with(Handover { log: <tmp>/private.jsonl, ..nothing() })` and
   `workbench.matrix()` give build 1. This is the private build, and it logs
   one `private-build` event.
2. `shared_build(<tmp>)` (`:30-39`) runs `SharedBuildRequest::build`, which
   gives build 2. This is `Matrix::build_for_sharing`, run on one thread and
   logged to a separate `shared.jsonl`.
3. The test compares `shapes` (`:17-28`): (key, shape, start relative to base).
   It compares only manifest metadata.
4. It asserts that `private.jsonl` has exactly one line, of kind
   `private-build` with fixtures `matrix`.

By static count, one matrix build is ~67 `git` and ~20 `jj` spawns, all in
series through `Hermetic::run`. The submodule block (`fixtures.rs:329-403`) is
the largest git cluster. The test runs in 6.3 s alone, about two 3.3 s builds,
and in 30–36 s under the suite.

Reuse options within the work item's constraints:

- **Concurrent builds:** run the private and shared builds on two threads.
  Same work, roughly half the wall time when cores are free.
- **Adopt the run's tree:** compare the private build against the matrix the
  run already shares (`ACCELERATOR_SHARED_MATRIX`) instead of rebuilding it.
  - The test still compares a private build with a shared one, at the cost of
    one build.
  - It would make the binary a `Matrix` consumer in
    `tasks/shared/shared_fixtures/consumers.py`.
  - It needs a fallback for runs without a handover.
- **Threads inside a build:** scoped threads within `Matrix::build_with`.
  PP-957 held this in reserve. It would also speed up the shared build step.

### Fixture infrastructure available to reuse (`vcs-test-support`, `tasks/`)

- **Copy helper:** `copies::copy_part(shared_tree, entries, destination)`
  (`src/copies.rs:43-98`). It keeps mtimes and atimes to the nanosecond, modes
  and symlinks, and its `named_by` result reports any file that still names the
  source. `tests/copy_fidelity.rs` proves that copied jj and git states report
  the same status and record no op.
  - ⚠️ Guard `tests/unit/tasks/test_fixture_builds.py` refuses `copy_part(`
    outside `copies.rs` and `copy_fidelity.rs`. A test that copies within
    itself would need a guard change, or a call through a `Workbench` method.
- **Shared families are closed.** `Family { Matrix, StatusLog }` is mirrored in
  several places, and a new shared state (for example a stalled migration)
  needs a variant in each:
  - Rust: `src/family.rs:16-39` and the dispatch match at
    `src/shared_build.rs:149-154`.
  - Python: `tasks/shared/shared_fixtures/families.py:5-47` and
    `consumers.py:16-26`.
  - Contract files: `contract/{variables.json,flags.json}`.
- **Single builder.** The build step runs one builder executable,
  `shared-fixtures`, found by name in `tasks/test/cli.py:63` and `:289-304`.
- **Sealed trees.** Trees are sealed read-only and digested. Tests that write
  must copy through `Workbench::writable`.
- ⚠️ **Crate boundary.** A stalled-migration builder needs the
  `accelerator-migrate` binary. `vcs-test-support` sits below `migrate-cli`
  and cannot name it, so the builder has three options:
  - live in `migrate-cli` as a second builder bin;
  - take the binary path from the task's `UnitListing`;
  - produce the stall another way.
- **No reusable state comparison.** The existing comparisons are private to
  their tests: `shapes` in `matrix_private_build.rs`, and the status and
  op-log checks in `copy_fidelity.rs`. The **Reused state** criterion
  (commit-graph shape, descriptions, trees, bookmarks, working-copy parents,
  colocation, stall state, excluding IDs and timestamps) needs a new
  projection.
- **Output constraint.** Any new fixture output must not match
  `timeline._NEXTEST_SUMMARY` or `_FINISHED_IN`. The guards are
  `test_session.py:221-245` and `test_log.py:330-337`.

### Measurement tooling (`tasks/shared/build_measurement/`)

- **Phase figure:** `measure:build` stamps each line and sums every
  `Summary [` match into the nextest phase (`timeline.py:26`, `:321-328`).
- **No per-test parsing:** nothing in `tasks/` parses `PASS [` lines.
  Per-binary test-seconds, final attempts of retried tests and completion
  positions all need post-processing of the kept logs, as done here.
- **Noise verdict:** `compare-builds` reports `within` noise when the current
  median lies inside the previous record's [min, max] (`comparison.py:152-157`).
  The reference record's phase ran 126–144 s, so "within noise" covers an
  18 s range.

## Code References

- `cli/migrate-cli/tests/guarded_resume.rs:125`: `COLOCATIONS`
- `cli/migrate-cli/tests/guarded_resume.rs:144-180`: `Stalled::on`, the stalled-state builder
- `cli/migrate-cli/tests/guarded_resume.rs:367-406`: `Move` and the slowest test
- `cli/migrate-cli/tests/common/mod.rs:16,66-91`: migrate binary path and `run`
- `cli/migrate/src/preflight.rs:73-142`: ownership and staleness decisions the tests assert
- `cli/work-adapters/tests/rekey_run.rs:91-96`: `mutations_of_a_whole_run`
- `cli/work-adapters/tests/rekey_run.rs:860-901`: `committed`, `commit_everything`, `halt_at`
- `cli/work-adapters/tests/rekey_run.rs:927-978`: the two tail tests
- `cli/work-adapters/tests/support/mod.rs:243-278,496-500`: `Fault::HaltAt`, `halted_at`
- `cli/vcs-test-support/tests/matrix_private_build.rs:17-64`: the test and `shapes`
- `cli/vcs-test-support/src/fixtures.rs:93-113`: `Matrix::build_with`
- `cli/vcs-test-support/src/copies.rs:43-98`: `copy_part`
- `cli/vcs-test-support/src/workbench.rs:150-212`: `shared`, `writable`, `private_states`
- `cli/vcs-test-support/src/family.rs:16-39`, `src/shared_build.rs:149-154`: the closed family set
- `cli/vcs-test-support/src/hermetic.rs:253-272,330-379`: `Hermetic::run` and the per-call `--version` checks
- `tasks/test/cli.py:63,289-333,371-393`: builder location, build step, nextest invocation
- `tasks/shared/shared_fixtures/{families,consumers}.py`: Python side of the family set
- `tasks/shared/build_measurement/timeline.py:26,321-328`: nextest-phase parsing
- `cli/.config/nextest.toml`: default filters only

## Architecture Insights

- **Spawns run in series.** Every target is bound by sequential
  `Hermetic::run` spawns. The October research found the phase about 50%
  CPU-busy and limited by spawn latency, and these alone ratios agree: under
  load each spawn costs 2–5× more.
- **Cutting spawns helps twice.** Fewer spawns shorten a target's own time and
  reduce the contention it puts on everything else.
- **Two ways to build once.** Within one test process, build once per
  colocation or VCS and copy per pass. Within a run, extend PP-957's shared
  families. Nextest's process-per-test model rules out an in-memory
  `OnceLock`. The work item forbids caches that survive across runs and
  dependency tasks.
- **Splitting does not reduce load.** Splitting per colocation, VCS or halt
  point shortens the slowest test but not the test-seconds. Under 16-way
  saturation, the extra processes may add contention.
- **Version checks are repeated.** `assert_jj_matches` and
  `assert_git_is_recent_enough` spawn `--version` on every builder call:
  26 times in `guarded_resume` and 39 times across the two `rekey_run` tests.

## Historical Context

- `meta/plans/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
  - Built the shared-family mechanism, `copy_part` and the fixture log.
  - Rejected nextest setup scripts and dependency tasks, because
    `Summary [` summing and `--isolate --skip-deps` rule them out.
  - Held scoped threads inside builds in reserve.
- `meta/prs/247-description.md`, `meta/prs/249-description.md`
  - The matrix is never copied, because it holds absolute paths. Status-log
    builders were made path-free so their trees could be copied.
  - Copy fidelity relies on keeping mtimes, including `tree_state`'s.
- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
  - One matrix build takes 3.3 s alone and 35–47 s under load.
  - The phase is spawn-latency-bound at ~50% CPU.
  - Host noise from `syspolicyd` and the IDE is unquantified.
- `meta/reviews/work/PP-943-shorten-the-slowest-tests-in-the-cli-unit-suite-review-1.md`
  - Pass 5 required that shared state be built inside the measured task.
  - Minors still open: equality-check cost unaccounted, "same assertions"
    checked only by review, aggregation order.

## Related Research

- `meta/research/codebase/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run.md`
- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- `meta/research/codebase/2026-10-30-PP-960-build-performance-measurement-harness.md`

## Open Questions

- ❓ **Author review.** Four of the five alone ratios fall under 0.5. The
  baseline record's alone runs need to confirm this before planning. If they
  do, the Assumptions require an author review of the `guarded_resume` 20 s
  ceiling and all three tail-test halving targets.
- **Whole-run sharing for `guarded_resume`.** Is a shared stalled-migration
  family worth its cost? It needs a new `Family` variant across Rust, Python
  and the contract files, plus a way to run the migrate binary from the build
  step. Within-test reuse is the cheaper alternative, but it only cuts the
  26 builds to ~20, because the variants of tests 7 and 8 would share a build.
  That would not halve the test-seconds.
- **Spawn versus in-process time in `rekey_run`.** How much of each pass is
  `git`/`jj` spawns and how much is in-process apply and repository loading?
  This decides whether copying the pre-halt repository is enough to halve the
  tests.
- **Absolute paths in copied jj repos.** Does a jj 0.43 repository, colocated
  or not, record absolute paths that would make a copied stalled state point
  back at its source?
- **Coupled end of the tail tests.** Why does `matrix_private_build` finish
  0.2–0.3 s after the `rekey_run` kill test in every run?
- **The unexercised halt point.** Should PP-943 fix the edit-refusal test's
  last halt point, or record it as out of scope? Its coverage baseline reads
  "every halt point in `0..mutations_of_a_whole_run()`", and that range
  excludes the last one.

Not checked: nextest's scheduling order and per-test `priority` support in
0.9.138, the full list of paths migration 0006 writes, and whether the alone
figures hold at `main` (`4a0579d7cc`) on a quiesced host. The alone-run logs
are in this session's scratchpad and are not kept.

## Follow-up Research 2026-11-23T11:39:41+00:00

### Decisions on the open questions

Toby Clemson answered three of the open questions on 2026-11-23.

| Question | Decision | Consequence for planning |
|---|---|---|
| Hold the author review now, or after the baseline? | Wait | Take the baseline record and its alone runs at `main` first. Re-apply the half-ratio rule to those figures. Treat this document's alone figures as indicative only. |
| Add a shared stalled-migration family for `guarded_resume`? | Yes | Plan a third shared family (see below). |
| Fix the edit-refusal test's unexercised halt point? | Fix | Derive that test's halt points from its own five-file corpus, so every mutation of its run is halted at. |

### What the shared stalled-migration family touches

The family set is closed, so adding the family has to change each of these
places together:

- **Rust family set:** a new `Family` variant with its traits, including the
  handover variable and builder flag (`cli/vcs-test-support/src/family.rs:16-39`).
  Also an arm in the `build_for_sharing` dispatch
  (`src/shared_build.rs:149-154`), and a `SharedFamily` and `CopyableFamily`
  implementation.
- **Python mirror:** the family table in
  `tasks/shared/shared_fixtures/families.py:5-47`. Also
  `accelerator-migrate::guarded_resume` added to `CONSUMERS`
  (`consumers.py:16-26`), which drives family selection for filtered runs and
  the silent-consumer check.
- **Contract files:** `cli/vcs-test-support/contract/variables.json` and
  `flags.json`, which `test_log.py:265-277` checks against Python.
- **Builder:** something must run the `accelerator-migrate` binary during the
  build step. There are two routes:
  - **Path through the existing builder:** `tasks/test/cli.py` already parses
    the cargo test-build output (`UnitListing`, `:289-304`). It can find the
    migrate binary there and hand its path to `shared-fixtures`.
  - **Second builder bin in `migrate-cli`:** this means generalising `_BUILDER`
    and `_locate_builder` to run more than one builder.

  The first route keeps a single builder and keeps `vcs-test-support` free of
  any dependency on migrate code, but the stall seeding has to move into
  `vcs-test-support`. That seeding is the corpus, the ledger and the
  `OWNED_NOTE` constants, now in `migrate-cli/tests/common/mod.rs`.
- **Copy, not adopt:** every `guarded_resume` test changes its repository. So
  the family must be copyable, with tests reaching it through
  `Workbench::writable`, and the builder must record no absolute paths.
  - Check `.jj/repo/store/git_target` and, for `--colocate`, `.git/config`
    under jj 0.43.
  - The `named_by` scan will fail a copy that names its source.
- **Parts:** four states, SingleParent and MergeWithSibling under each
  colocation, plus sidecar files with each state's `base` and `sibling`
  change IDs. Tests 7–12 need those IDs.
- **Fixture log:** a shared-build event and one copy event per test pass. The
  summary output must still not match `Summary [` or `Finished in`.
- **Reused-state test:** compares a copied state with one built from scratch,
  under each colocation, over the projection the **Reused state** criterion
  lists. No such comparison exists to reuse.
- **Measurement:** the family's build time comes from its own log line, which
  the work item calls "its own entry in the run log". That time is added to
  `guarded_resume`'s total test-seconds. The build step currently prints one
  combined `built in X s` line, so it needs a line per family.

The unreadable-working-copy and Git-stall tests (tests 1–3) do not use
`Stalled::on`, so they stay outside the family.

### Fixing the unexercised halt point

The edit-refusal test (`rekey_run.rs:948-978`) commits a fifth file,
`meta/notes/synced-only.md`. That file adds a mutation, but the loop still uses
`0..mutations_of_a_whole_run()`, which counts the four-file fixture. A fix
needs a count taken from the same corpus the test runs. One option is a
`mutations_of_a_whole_run` variant that takes the corpus setup. The fix also
needs the iteration-count assertion that the **Test mapping** criterion asks
for.

This adds one pass to a tail test, so the test's before figure and its
after figure cover different halt sets. The plan should record the fix as a
coverage change. The work item's coverage baseline and **Test mapping**
criterion still say `0..mutations_of_a_whole_run()`, so they need updating to
match.
