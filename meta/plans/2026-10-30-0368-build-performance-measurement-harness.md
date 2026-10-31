---
type: "plan"
id: "2026-10-30-0368-build-performance-measurement-harness"
title: "Build-Performance Measurement Harness and Host Guidance Implementation Plan"
date: "2026-10-30T17:19:28+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0368"
parent: "work-item:0368"
derived_from: ["codebase-research:2026-10-30-0368-build-performance-measurement-harness", "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"]
relates_to: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
tags: ["build-system", "measurement", "performance", "mise", "jj", "macos", "tasks"]
revision: "d6b35fbf840dee28bbe3cb3a6be9d12d6b433438"
repository: "accelerator"
last_updated: "2026-10-31T18:04:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Build-Performance Measurement Harness and Host Guidance Implementation Plan

## Overview

The plan adds `measure:build`, an operator-run task that times a full
`mise run default`. Per task it reports duration, queueing delay, cargo unit
counts, lock wait and nextest phase. It also reports CPU-busy buckets and the
CPU time of the host processes that react to the build. It repeats the build
over a priming run and N timed runs and writes a JSON record.

With the harness in place, the plan records the warm no-op baseline that
0362–0367 wait on, and adds the isolation mode and the incremental and cold
cache states that 0365 and 0366 measure with. `measure:compare-builds`
then applies the comparability and citation rules to two records. Last,
the plan documents the macOS host settings in `CONTRIBUTING.md` and cites
their measured effect.

## Current State Analysis

- **`measure` namespace.** The namespace holds four operator-run tasks
  (`tasks/measure.py`) over a pure analysis core
  (`tasks/shared/measurement.py`). Tests swap commands through `Protocol`
  ports passed as parameters (`tasks/measure.py:393-430`).
- **Closure guard.** The guard keeps every task whose `run` string contains
  `invoke measure.` out of the `check` and `default` closures
  (`tests/unit/tasks/test_mise.py:244-299`). A `measure:build` registered
  as `run = "invoke measure.build"` is covered with no change to the guard.
- **Missing pieces.** Nothing in `tasks/` streams a child's output, samples
  `iostat` or `ps`, parses a mise `--output prefix` log or
  `mise tasks --json`, records the measured commit, or refuses a non-macOS
  host.
- **mise's actual behaviour.** mise 2026.10.8 prints `[t] $ <run>` when a
  task spawns, `[t] Finished in <n>ms|s` when it succeeds, and
  `[t] sources up-to-date, skipping` when it skips it. A failed task's
  `exited with status <n>` comes only at the very end of the run. Under
  `--continue-on-error`, mise runs a failed task's dependants anyway.
- **Merges defeat jj's `files()`.** jj's `files()` revset treats a clean
  merge as changing nothing, so a commit search built on it names the
  commit before a PR merge. That was verified in a scratch repository.
  The search therefore compares each commit with its first parent.
- **Run directory location.** `.accelerator-measure/` is already
  gitignored. Warm-dispatch only ever unlinks its manifest file and makes
  a non-recursive `rmdir` attempt there (`tasks/measure.py:606-612`,
  `:628-631`, `:1171-1174`), so a sibling `build/` subdirectory is safe.
- **Lint and type constraints.**
  - Ruff runs `select = ["ALL"]`. C901, PLR0912, PLR0915, FBT003, DTZ, ANN
    and N818 stay enforced; S603/S607, T201 and FBT001/FBT002 are ignored.
  - Pyrefly runs the `strict` preset over `tasks/**`.
  - A new package under `tasks/` needs an `__init__.py`.

## Desired End State

- **The task.** `mise run measure:build -- --runs 3 --keep --host-settings
  <label>` runs on a macOS host. It writes
  `.accelerator-measure/build/<run-id>/record.json` beside the stamped build
  and sampler logs, and with `--keep` it also writes
  `meta/measurements/<run-id>.json` unless the record holds a fetch
  failure. It refuses any other OS before writing anything.
- **The baseline.** A warm no-op `--runs 3` baseline record taken on the
  reference host is committed, and 0362, 0363, 0364 and 0367 no longer list
  0368 in `blocked_by`.
- **The modes.**
  - `--isolate <task>` runs named tasks alone with `--skip-deps`.
  - `--cache-state incremental` touches `cli/kernel/src/lib.rs` before each timed run.
  - `--cache-state cold` skips priming and deletes `cli/target` and
    `cli/visualiser/frontend/node_modules` before each timed run.
  - 0365 and 0366 have dropped their remaining edges.
- **Comparison.** `mise run measure:compare-builds -- <previous> <current>`
  prints:
  - the comparability check, then both records' run ids;
  - the wall-clock and per-process changes in seconds and as a ratio, with
    the noise verdict, naming rather than comparing a process the two
    records tracked by different patterns;
  - the task-set difference and the cargo unit totals with their total;
  - with `--task`, a named task's duration and nextest-phase changes.
- **Sibling work items.** 0361 names the baseline record and its closing
  `measure:compare-builds` command, and 0365 and 0366 name the
  `measure:compare-builds` commands whose lines carry their before-and-after
  changes.
- **Host guidance.** `CONTRIBUTING.md` gives the host-settings steps and
  their tradeoffs, and cites the three condition records.
- **Verification.** `mise run` exits 0 after every code phase.

### Key Discoveries

- **The closure guard.** It is keyed on the `run` string, not the task name
  (`tests/unit/tasks/test_mise.py:249-258`).
  `TestMeasureNamespaceDocs` pins five strings in the README's measure
  section (`tests/unit/tasks/test_measure.py:1304-1319`).
- **Roll-ups are visible in the graph.** `mise tasks --json` gives each
  task a `run` list, and a roll-up's is `[]` (verified: `test` has
  `run: []`, `deny:check` has `["invoke deny.check"]`). A runnable task
  that printed nothing can therefore be told apart from a roll-up.
- **The mise log contract.** It was probed live and is recorded in
  `meta/research/codebase/2026-10-30-0368-build-performance-measurement-harness.md`
  ("The mise output contract").
  That contract holds for a direct `mise run`. Under an enclosing
  `mise run` (as `measure:build` itself is), mise exports `MISE_TASK_*` to
  its children, and `MISE_TASK_TIMINGS=0` silences every per-task
  `Finished in` line; the build runner therefore drops the inherited
  `MISE_TASK_*` and sets `MISE_TASK_TIMINGS=1`. Found by the first golden
  capture, which read every printing task as failed.
- **`iostat`'s first report** averages since boot (from the man page). It is
  discarded.
- **`ps` output.** `TIME` is `minutes:seconds.hundredths` with no hour
  field (`115:00.06`). Each IDE's `fsnotifier` lives inside its bundle, so
  the IDE is matched by its main executable (`Contents/MacOS/idea`,
  `Contents/MacOS/rustrover`).
- **`pmset`.** `pmset -g ps` reports `Now drawing from 'AC Power'`, and
  `pmset -g` reports `powermode 0` on macOS 26.3. The existing
  `power_state` helper keys readings by `argv[0]`, so two `pmset` probes
  would collide (`tasks/shared/measurement.py:698-717`). The plan parses its
  own.
- **`Summary` has no maximum** (`tasks/shared/measurement.py:56-62`). The
  plan adds a `Spread` type rather than widening the warm-dispatch record
  shape.
- **The unit test lane** runs on ubuntu and macOS in CI. Tests that call
  real `jj` or `git` gate on `tests/support/tools.py` `require`, which
  skips locally but fails in CI.
- **Precedents for structure.**
  - Ports and wiring: `tasks/shared/vendor/pin_guard/` (`guard.py`
    `GuardPorts`, `wiring.py` `real_ports`) and `tasks/shared/dev/`
    (`DevDeps`, `PopenHandle`, `start_new_session=True`).
  - Process reaping: `tasks/shared/processes.py` `PsutilProcessOps` with
    `FakeProcs`/`FakeClock` in `tests/unit/tasks/shared/doubles.py`.

## What We're NOT Doing

- Linux support, Spotlight exclusions, IDEs other than JetBrains, and any
  gate on `check` or CI.
- Committing raw logs. `--keep` writes only the record, which already holds
  every per-run figure.
- Adding power state or the mise version to Requirement 7's comparability
  key. Both are recorded as metadata only.
- Per-task CPU attribution by walking `ppid` chains (the research
  appendix's extra). Only the tracked host processes are attributed.
- Recording link time or binary size (0366's Requirement 4). The record
  carries per-run unit totals, which 0366's acceptance criterion needs, and
  nothing more.
- Changing the behaviour of `measure:warm-dispatch`, `Summary`, or the
  existing record formats. Some of its helpers move into `tasks/shared/`,
  and only `repository_kind`'s exception type changes (see Implementation
  Approach).
- Re-deriving a record from a run directory. The logs are kept, but not
  the graph, provenance and settings the analysis needs (Phase 4, §2).
- A `--preflight-only` mode. The preflight already comes before any build,
  and stopping there costs seconds (Phase 7, §2).
- A strict exit status for `measure:compare-builds`. It reports rather
  than gates, and each reader decides what it accepts (Phase 9, §2).

## Implementation Approach

The harness keeps the measure namespace's shape: a pure domain over
recorded observations, plus a thin imperative shell whose collaborators are
injected. The `@task` in `tasks/measure.py` validates its flags, builds the
real ports, and maps a refusal to `Exit`. Everything else lives in a new
package, `tasks/shared/build_measurement/`:

| Module | Phase | Role |
|---|---|---|
| `errors.py` | 1 | `MeasurementRefusedError` and subclasses: every "cannot measure" exit |
| `record_model.py` | 1 | `RecordModel`, the frozen, strict, closed Pydantic base of every recorded type; `NoReading` |
| `task_graph.py` | 1 | `TaskGraph` parsed from `mise tasks --json`; closure, dependencies, runnability |
| `timeline.py` | 1 | Pure: stamped mise log → per-task `TaskFigures`, outcomes, after-failure flags |
| `fetch_failures.py` | 1 | Pure: fetch-failure patterns per external input |
| `cpu_load.py` | 2 | Pure: `iostat` readings, CPU buckets, idle load |
| `tracked_processes.py` | 2 | Pure: the tracked-process catalogue, the `ps` command and line format, the sampler's markers, snapshots and per-process CPU |
| `across_runs.py` | 2 | Pure: `Spread`, per-task and per-process presence across runs |
| `locations.py` | 3 | The run-directory root, `SESSION_LOCK` and `BUILD_PGID`, each defined once |
| `provenance.py` | 3 | Pure: measured commit and working-copy match over a `History` port |
| `history.py` | 3 | `JjHistory`, `GitHistory` adapters |
| `host.py` | 3 | `HostDescription` and its readings; the platform check |
| `cache_states.py` | 4, 7 | Pure: `BuildCacheState` and what each state primes, touches and deletes |
| `cold_trash.py` | 7 | Pure: `TRASH_PREFIX` and the two trash facts, `UnremovedTrash` and `FoundTrash` |
| `measured_build.py` | 4 | Pure: the record's schema — `MeasuredBuild` and its parts, `MeasurementConditions`, `Mode`, `HostSettings` |
| `settings.py` | 4 | Pure: `BuildSettings` and its smart constructor, `BuildScope` and its invocations, `SessionRun` and its log stems |
| `analysis.py` | 4 | Pure: the captured inputs (`BuildResult`, `CapturedInvocation`, `CapturedRun`, `CapturedSession`) and `analyse_session` over them |
| `record.py` | 4 | Run id, record text, reading and writing records |
| `reports.py` | 4, 7 | Pure: `Preflight`, `SessionOutcome`, the preflight report and the printed summary; from Phase 7, `SessionNotice` |
| `session.py` | 4, 7 | Orchestration over `BuildPorts` and the session's ports (from Phase 7, `Workspace`) and `OrphanedBuild`: priming, timed runs, capture; returns a `SessionOutcome` |
| `build_group.py` | 4 | `BuildGroupFile`: writes, reads and judges `BUILD_PGID`, the only code that knows its format |
| `build_runner.py` | 4 | `SubprocessBuildRunner`: the streaming, stamping build |
| `samplers.py` | 4 | `StreamSampler` (`iostat`) and `PsProcessSampler` (`ps`) |
| `log_files.py` | 4 | `open_log`: the line-buffered, `\n`-only log writer both the build runner and the samplers use, so neither adapter imports the other |
| `session_lock.py` | 4 | `WorkspaceSessionLock`: an adapter over `tasks/shared/locking.py` `workspace_lock` |
| `wiring.py` | 4 | `real_ports(root)`: the one composition root |
| `workspace.py` | 7 | `FilesystemWorkspace`, the only code that deletes: incremental touch and cold deletion with containment checks |
| `comparison.py` | 9 | Pure: changes between two records, the noise rule, the task-set difference, named tasks and unit totals |

Imports run one way: `session` → `analysis`, `record`, `reports` →
`measured_build`, `settings`; and `comparison` → `measured_build`. Every
type a pure module consumes is defined in that module or below it, so the
session imports its inputs' types rather than the reverse. No pure module
imports `session`, and only `wiring` imports the adapters
(`build_runner`, `samplers`, `session_lock`, `workspace`, `history`,
`build_group`). `tests/unit/tasks/shared/build_measurement/test_imports.py`
enforces the rule with an import-boundary test in the style of
`tests/unit/tasks/shared/test_skill_parsing.py`, so a misplaced type fails
CI rather than recreating a cycle.

As built, tests mirror the modules: the settings tests below live in
`test_settings.py`, the summary and preflight tests listed under
`test_record.py` in `test_reports.py`, and the analysis has its own
`test_analysis.py`. `FakeHistory`, `FakeSession` and the scripted builds
are shared from `doubles.py`. Every log line is written by
`timeline.stamped_text`, the inverse of `read_stamped`, and the process
sampler's markers are `StampedLine`s, so a sampler holds in memory exactly
what it writes.

Some helpers the package needs live where the package cannot import them:
`tasks/measure.py` will import the package, and `_reap` is private to the
dev server. Phases 3 and 4 first move them down a layer rather than copy
them. Only `repository_kind`'s exception type changes; every other move
keeps its behaviour:

| Helper | From | To | Phase |
|---|---|---|---|
| `DiagnosticRunner`, `ambient_diagnostic_runner`, `raw_diagnostic_runner` | `tasks/measure.py:409`, `:541-563` | `tasks/shared/diagnostics.py` | 3 |
| `RepositoryKind`, `repository_kind` | `tasks/measure.py:3186-3209` | `tasks/shared/repository.py` | 3 |
| `".accelerator-measure"` and `"meta/measurements"` literals | `tasks/measure.py:343`, `:1830` | `tasks/shared/paths.py` `MEASURE_SCRATCH_DIRNAME`, `MEASUREMENTS_RELATIVE` | 3 |
| `_UNWIND_SIGNALS`, `unwind_signals()` and the handler swap | `tasks/measure.py:566-575`, `:634-647` | `tasks/shared/signals.py` | 4 |
| `_reap`'s grace loop | `tasks/shared/dev/host_server.py:138-159` | `tasks/shared/processes.py` | 4 |

Tests mirror the package under `tests/unit/tasks/shared/build_measurement/`,
as `tests/unit/tasks/shared/dev/` does. Shared doubles go in
`tests/unit/tasks/shared/build_measurement/doubles.py`; real jj and git
repository builders go in `.../repositories.py`. Fixtures go in
`tests/unit/tasks/fixtures/build-measurement/`.

Decisions taken while planning:
- **Record name.** `build_<UTC YYYY-MM-DDThhmmssZ>_<hostname>_<cache
  state>_<label>`, with `_isolated` appended in isolation mode, shared by
  the run directory and the kept record: for example
  `build_2026-10-30T162000Z_engywook_warm-no-op_devtools+ide-exclusion`.
  The fields are joined by `_`, which none of them can contain (the
  hostname is sanitised to `[A-Za-z0-9-]`, and states and labels use only
  `-` and `+`), so each boundary is unambiguous. The label keeps its `+`,
  which is safe in file names and in jj and git, so it reads as
  `--host-settings` takes it. The date reads like the rest of `meta/`, and
  the suffix shows what a record measured without opening it.
- **IDE matching.** The IDE is matched by its main executable path. Every
  `fsnotifier` process counts.
- **Measured commit.** The search walks the first-parent chain from
  `first_parent(@)` (jj) or `HEAD` (git), and takes the nearest commit that differs from its
  first parent outside `meta/measurements/`. 0368's Requirement 5 was
  amended to say so.
- **Working-copy match.** It is checked before the priming run and after the
  last timed run. The record says `matched` only when both checks pass,
  because `format:fix` can rewrite the tree mid-run.
- **`ps` filtering.** The `ps` sampler keeps only lines that match a
  tracked process, plus one marker line per tick. An unfiltered tick is
  about 700 lines, roughly 30 MB per timed run.
- **Nextest phase.** A task's nextest phase is the sum of its `Summary`
  lines. Today only `test:unit:cli` prints one, once.
- **The record is `MeasuredBuild`.** Its schema lives in the pure
  `measured_build.py`, apart from the orchestration that produces it and
  the comparison that reads it. Every type the record is made of is a
  Pydantic `BaseModel` deriving from one `RecordModel` base, which is
  frozen, strict and closed (`extra="forbid"`). The models' field names are
  the record's keys. `model_dump_json` writes the record and
  `model_validate_json` reads it back strictly, so the shape is defined
  once and `schema_version: Literal[1]` refuses any other version without a
  hand-written check. Transient analysis types (`StampedLine`, `Timeline`,
  `ProcessSnapshot`) and the ports stay frozen dataclasses: the line
  between the two is "is it persisted?". Because the field names are the
  keys, every field holding a time or quantity carries its unit
  (`duration_seconds`).

The record is `MeasuredBuild` written by
`measured.model_dump_json(indent=2)`. Fields are in declaration order,
`None` is written as `null`, enums as their values and tuples as arrays.
`schema_version` is declared first, with no default, so it is written first
and a document without it is refused. Every time or quantity key carries
its unit (`_seconds`, `_percent`), and every per-run time is seconds from
that run's epoch. A map keyed by name is named for what it holds:
`runs[].process_cpu_seconds` maps each process to a figure, while
`summary.processes` maps each to its `ProcessAcrossRuns`, as
`summary.tasks` does for tasks. The schema freezes at the Phase 5
baseline, so the names are settled here:

```jsonc
{
  "schema_version": 1,
  "run_id": "build_2026-10-30T162000Z_engywook_warm-no-op_devtools",
  "run_directory": ".accelerator-measure/build/build_2026-10-30T162000Z_engywook_warm-no-op_devtools",
  "started_at": "2026-10-30T16:20:00+00:00",
  "conditions": {
    "hostname": "engywook", "model": "Mac16,5", "os_version": "26.3", "mise_jobs": 8,
    "cache_state": "warm-no-op", "mode": "full", "isolated_tasks": [], "host_settings": "devtools"
  },
  "commit": {"id": "…", "matched_at_start": true, "matched_at_end": true, "matched": true},
  "host": {
    "hostname": "engywook", "model": "Mac16,5", "chip": "Apple M4 Max",
    "os_version": "26.3", "os_build": "25D125",
    "cores": {"total": 16, "performance": 12, "efficiency": 4},
    "power": {"source": "AC Power", "mode": "0"}
  },
  "mise": {"version": "2026.10.8 macos-arm64 (2026-10-11)", "jobs": 8},
  "idle_load_percent": 26.8,
  "task_set": ["build:frontend", "…"],
  "tracked_processes": {"syspolicyd": ["…"], "XprotectService": ["…"], "ide": ["…"], "fsnotifier": ["…"]},
  "priming": {"wall_clock_seconds": 318.2, "discarded": true},
  "runs": [
    {
      "index": 1, "wall_clock_seconds": 309.4, "exit_status": 0, "contains_failure": false,
      "tasks": {"test:unit:cli": {"outcome": "succeeded", "exit_status": 0,
                "start_seconds": 10.2, "duration_seconds": 297.1, "end_seconds": 307.3,
                "units": {"compiling": 44, "checking": 0, "documenting": 0},
                "lock_wait_seconds": 0.0, "nextest_phase_seconds": 149.2,
                "queueing_delay_seconds": 0.4, "after_failure": false}},
      "unit_totals": {"compiling": 61, "checking": 12, "documenting": 0},
      "cpu_buckets": [{"start_seconds": 0.0, "busy_percent": 91.2, "running": ["…"]}],
      "process_cpu_seconds": {"syspolicyd": 51.3, "XprotectService": null, "ide": 301.0, "fsnotifier": 12.4},
      "fetch_failures": []
    }
  ],
  "summary": {
    "wall_clock_seconds": {"minimum": 305.1, "median": 309.4, "maximum": 322.0},
    "tasks": {"test:unit:cli": {
              "presence": {"standing": "measured"},
              "duration_seconds": {"minimum": 291.8, "median": 297.1, "maximum": 304.6},
              "nextest_phase_seconds": {"minimum": 146.0, "median": 149.2, "maximum": 151.3}}},
    "processes": {"XprotectService": {
      "presence": {"standing": "absent", "absence": "not-seen"},
      "cpu_seconds": null}}
  },
  "fetch_failures": [],
  "failed_tasks": []
}
```

---

## Phase 1: Task timeline analysis

### Overview

This phase is a pure analysis of one timed run. It takes the stamped
`--output prefix` log and the task graph and produces every per-task figure
in Requirement 1: duration, queueing delay, unit counts, lock wait, nextest
phase, outcome, exit status and the after-failure flag. It also finds the
fetch failures. Nothing calls it until Phase 4; it merges on its own with
its tests.

### Changes Required:

#### 1. Tests first

**File**: `tests/unit/tasks/shared/build_measurement/test_task_graph.py`

Covering `TaskGraph`:
- `test_depends_are_read_from_mise_tasks_json`
- `test_the_closure_of_a_root_excludes_the_root`
- `test_the_closure_follows_depends_transitively`
- `test_a_task_missing_from_the_graph_has_no_dependencies`
- `test_unreadable_json_is_refused_naming_mise_tasks` (raises
  `TaskGraphError`)
- `test_a_non_list_depends_is_refused`
- `test_a_task_with_a_run_is_runnable`
- `test_a_roll_up_with_an_empty_run_is_not_runnable`
- `test_a_non_list_run_is_refused`

**File**: `tests/unit/tasks/shared/build_measurement/test_timeline.py`

Each stage gets tests of its own before the composed tests below:
- `test_transcripts_group_lines_by_prefix_and_drop_unprefixed_ones`.
- `test_a_transcript_keeps_the_verdict_status_and_skip_line`: the
  end-of-run status lines set `exit_status` and are not counted.
- `test_own_figures_follow_the_outcome`: parametrised over each named
  constructor, asserting which fields are `None`.
- `test_placing_in_the_graph_adds_queueing_and_after_failure`: over
  hand-built `OwnFigures`, with no log at all.
- `test_read_stamped_splits_only_at_newlines`: a stamped line holding `\r`
  and ` ` comes back as one `StampedLine`.
- `test_tasks_mise_reported_failing_come_from_the_status_lines`: over
  `failed-task.log`.

Hand-seeded fixture logs under
`tests/unit/tasks/fixtures/build-measurement/`. Each is written in the
stamped format `<seconds-since-run-start> <raw line>`, and each known
answer is worked out in a comment beside the assertion, following
`test_measure.py:198-204`.

| Test | Fixture | Asserts |
|---|---|---|
| `test_duration_is_mises_finished_in_figure` | `finished-in.log` | `Finished in 1.01s` → 1.01; `5.4ms` → 0.0054 |
| `test_duration_without_finished_in_spans_first_to_last_line` | `failed-task.log` | first `$` line to last output line |
| `test_a_failed_tasks_status_comes_from_the_end_of_run_line` | `failed-task.log` | `exit_status == 3`; the trailing `exited with status` and `ERROR task failed` lines do not extend the duration |
| `test_a_task_with_no_verdict_counts_as_failed` | inline | lines but no `Finished in` and no status → `FAILED`, `exit_status is None` |
| `test_a_skipped_task_has_no_duration_and_ends_at_its_skip_line` | `skipped-task.log` | `SKIPPED`, `duration is None`, dependant's queueing measured from the skip line |
| `test_a_roll_up_that_printed_nothing_is_silent` | inline | `run: []` in the graph, absent from the log → `SILENT` |
| `test_a_runnable_task_that_printed_nothing_was_not_run` | inline | a `run` in the graph, absent from the log → `NOT_RUN`, and its dependants have `after_failure` |
| `test_a_failed_dependency_with_no_counted_line_passes_readiness_through` | inline | only the end-of-run status pair → its dependant is ready at the failed task's own `ready_at` |
| `test_dependants_of_a_failure_are_flagged_from_the_graph` | `failed-task.log` | direct and transitive dependants have `after_failure`; unrelated tasks do not |
| `test_a_task_with_no_dependencies_is_ready_at_the_run_start` | `finished-in.log` | queueing delay equals its start |
| `test_a_task_with_no_dependencies_is_ready_at_the_given_origin` | inline | `ready_from=40.0`, first line at 40.3 → 0.3 |
| `test_each_task_records_its_end` | `failed-task.log` | succeeded: start + duration; skipped: its skip line; failed: its last counted line |
| `test_disjoint_timelines_merge` | inline | two single-task timelines → one holding both |
| `test_a_silent_dependency_ends_when_its_own_dependencies_end` | `silent-roll-up.log` | queueing measured from the latest end beneath the roll-up |
| `test_queueing_delay_is_never_negative` | inline | dependant's `$` line before its dependency's computed end → 0 |
| `test_lock_wait_runs_to_the_next_line` | `lock-wait.log` | two waits summed |
| `test_consecutive_lock_waits_each_run_to_the_next_line` | inline | package-cache then build-directory blocking lines at 1.0 and 1.4, next line at 3.0 → 0.4 + 1.6 = 2.0 |
| `test_a_lock_wait_still_open_at_the_last_line_runs_to_the_tasks_end` | `lock-wait.log` | closed at start + duration |
| `test_an_open_wait_in_a_failed_task_runs_to_its_last_counted_line` | inline | no `Finished in`; closed at the last counted line, not at the end-of-run status line |
| `test_each_unit_kind_is_counted` | `cargo-units.log` | `Compiling`, `Checking`, `Documenting` |
| `test_units_are_counted_inside_carriage_return_progress_and_colour` | `cargo-units.log` | ANSI-coloured, `\r`-joined segments |
| `test_the_nextest_phase_comes_from_the_summary_line` | `nextest.log` | `     Summary [ 149.123s] …` → 149.123 |
| `test_several_summary_lines_sum` | inline | 2 lines → sum |
| `test_unprefixed_lines_belong_to_no_task` | `finished-in.log` | run-level `Finished in` and `ERROR 1 task(s) failed:` ignored |
| `test_unit_totals_sum_every_task` | `cargo-units.log` | run-level totals |

**File**: `tests/unit/tasks/shared/build_measurement/test_fetch_failures.py`

- `test_a_curl_error_is_a_launcher_release_failure`: input
  `[test:integration:hooks] curl: (6) Could not resolve host: github.com`.
- `test_a_failed_advisory_fetch_is_an_advisory_database_failure`: the line
  captured in step 3.
- `test_each_npm_network_code_is_an_npm_registry_failure`: parametrised over
  `ETIMEDOUT`, `ECONNRESET`, `ENOTFOUND` and `EAI_AGAIN`, after either
  `npm ERR! code` or npm 10's `npm error code`.
- `test_a_network_code_outside_an_npm_error_is_not_a_failure`: a pytest ID
  holding `[ETIMEDOUT]`. Added in Phase 4, after the first golden capture
  read the fetch-failure tests' own output as registry failures.
- `test_a_non_network_failure_leaves_the_list_empty`: `deny:check` failing on
  an advisory finding, and `npm ERR! code ERESOLVE`.
- `test_each_failure_names_its_task_and_line`.

**File**: `tests/unit/tasks/shared/build_measurement/test_record_model.py`

- `test_a_recorded_value_cannot_be_changed`: assigning a field raises.
- `test_a_recorded_value_refuses_coercion`: `CargoUnits(compiling="3")`
  raises `ValidationError`.
- `test_a_recorded_value_refuses_an_unknown_field`.

#### 2. Implementation

**File**: `pyproject.toml`

Add Pydantic to the `build` group with an exact pin to the latest stable
release at implementation time. Its comment takes the full form of the
neighbouring pins, wrapped at 80 columns: what it does and where it is
used, then why the pin is exact. For example: "Parses and validates
build-measurement records (tasks/shared/build_measurement/). Exact pin:
committed records are read back through it, so a relock must not silently
move the validator under them." Run `uv lock`; `deps:install:python` runs
`uv sync --frozen`.

This is the first Pydantic under `tasks/`, so probe pyrefly's strict
preset with real models before building on it. Write `CargoUnits` and
`NoReading` (Phase 3's type, brought forward) as the first models, with a
`str | NoReading` field and a `Literal[1]` field, and construct each by
keyword inside `tasks/`. Run `mise run build-system:check` and record the
result in this plan. If pyrefly cannot check the inherited config, the
synthesised keyword `__init__` or the unions, stop and revisit the
Pydantic choice before Phase 2.

Probe result (Pydantic 2.14.2, pyrefly 1.0.0): pyrefly checks all four. A
well-typed `Probe(schema_version=1, model=NoReading(...), units=...)`
checks clean; an unknown keyword, `compiling="3"`, `schema_version=2` and
assigning a field of a model inheriting the frozen config are each
reported (`unexpected-keyword`, `bad-argument-type`, `read-only`).

**File**: `tasks/shared/build_measurement/__init__.py` (empty)

**File**: `tasks/shared/build_measurement/record_model.py`

```python
class RecordModel(BaseModel):
    model_config = ConfigDict(frozen=True, strict=True, extra="forbid")


class NoReading(RecordModel):
    argv: tuple[str, ...]

    @override
    def __str__(self) -> str:
        return f"no reading from {shlex.join(self.argv)}"
```

`NoReading` stands in a recorded field for a value that could not be read.
It lives here because several record types use it, and because Phase 1's
pyrefly probe needs it.

Every recorded type derives from it and is built with keyword arguments.
`strict=True` also applies when the analysis constructs a value, so a
string where a number belongs fails at the point of the bug rather than in
the written record.

**File**: `tasks/shared/build_measurement/errors.py`

```python
class MeasurementRefusedError(RuntimeError):
    """The harness cannot measure; no record is written."""


class TaskGraphError(MeasurementRefusedError):
    pass
```

**File**: `tasks/shared/build_measurement/task_graph.py`

```python
@dataclass(frozen=True, slots=True)
class TaskGraph:
    depends: Mapping[str, tuple[str, ...]]
    runnable: frozenset[str]

    @classmethod
    def parse(cls, mise_tasks_json: str) -> Self: ...

    def dependencies_of(self, task: str) -> tuple[str, ...]:
        return self.depends.get(task, ())

    def is_runnable(self, task: str) -> bool:
        return task in self.runnable

    def closure(self, root: str) -> frozenset[str]: ...
```

`parse` follows the `browsers.py` strict-parse precedent: it checks
`isinstance` per entry and raises `TaskGraphError("mise tasks --json …")`.

**File**: `tasks/shared/build_measurement/timeline.py`

The public surface:

```python
@dataclass(frozen=True, slots=True)
class StampedLine:
    at: float
    text: str


@unique
class TaskOutcome(StrEnum):
    SUCCEEDED = "succeeded"
    FAILED = "failed"
    SKIPPED = "skipped"
    NOT_RUN = "not-run"
    SILENT = "silent"


class CargoUnits(RecordModel):
    compiling: int = 0
    checking: int = 0
    documenting: int = 0
```

`TaskFigures` is defined with the analysis stages below, as `OwnFigures`
plus `queueing_delay_seconds` and `after_failure`. Its `end_seconds` is a
stored field, set by the analysis, so every record
gives each task's interval in every mode. That is what Requirement 3 asks
of isolation mode, and what 0369 needs from a full run. A `computed_field`
would be written too, but then refused on reading back under
`extra="forbid"`.

```python
@dataclass(frozen=True, slots=True)
class Timeline:
    tasks: Mapping[str, TaskFigures]

    def failed(self) -> tuple[str, ...]: ...
    def unit_totals(self) -> CargoUnits: ...


def read_stamped(text: str) -> tuple[StampedLine, ...]: ...
```

What failed is derived in one place: `failed_tasks(figures:
Mapping[str, TaskFigures]) -> tuple[str, ...]` names every task whose
outcome is `FAILED` or `NOT_RUN`. `Timeline.failed()` calls it, and so do
the analysis and the `MeasuredBuild` validator, so the record and its
check can never disagree. `tasks_mise_reported_failing(lines) ->
tuple[str, ...]` is a different, cheaper question for the session's
per-run progress line: it reads only mise's end-of-run `exited with
status` lines, so it never names a `NOT_RUN` task, and its name says so.

`read_stamped` splits with `text.split("\n")`, dropping only a trailing
empty element. `str.splitlines()` would split at `\r` and at Unicode line
separators too, undoing what the build runner preserves.

```python
def analyse_timeline(
    lines: Iterable[StampedLine],
    graph: TaskGraph,
    tasks: Iterable[str],
    *,
    ready_from: float = 0.0,
) -> Timeline: ...
```

`Timeline.merged(timelines)` combines timelines whose task keys are
disjoint, which is how a run of several invocations is analysed as one.

`tasks` is the set to report on: the `default` closure in a full run. Each
line is normalised once: ANSI CSI sequences are stripped, the `[task]`
prefix is split off, and the text is split on `\r` into non-empty segments.
The rules:

| Marker (on the last segment unless noted) | Effect |
|---|---|
| `^Finished in (?P<amount>[\d.]+)(?P<unit>ms\|s)$` | Reported duration; exit status 0; not counted as a line |
| `sources up-to-date, skipping` | `SKIPPED`; start and end at this line |
| `^exited with status (?P<status>\d+)$` | Exit status; not counted as a line |
| `ERROR task failed` | Not counted as a line |
| contains `Blocking waiting for file lock` | Closes any open wait, then opens one; the next counted line closes it |
| any segment: `(?:^\|\s)(Compiling\|Checking\|Documenting) \S+ v` | Unit count |
| `^\s*Summary \[\s*(?P<seconds>[\d.]+)s\]` | Added to the nextest phase |

- **Start and end.** A task's start is its first counted line. Its end is
  start plus duration, the skip line for a skipped task, or the last
  counted line for a failed one. The `Finished in` line is not counted, so
  a lock wait still open when a task succeeds closes at start plus
  duration rather than at that line.
- **No lines.** A task that printed nothing is `SILENT` when the graph
  says it is a roll-up, and `NOT_RUN` when it is runnable. `NOT_RUN`
  counts as a failure.
- **Readiness.** `ready_at(task)` is the latest `end_seconds` among its
  dependencies. A dependency whose `end_seconds` is `None` contributes its
  own `ready_at`, whatever its outcome, and a task with no dependencies is
  ready at `ready_from`.
- **Queueing delay.** `max(0.0, start - ready_at(task))`.
- **After failure.** A task has `after_failure` when any task in its
  transitive dependencies failed or was not run.

Keep each rule in its own small function. C901 and PLR0912 are enforced,
and a single collector loop over all markers would exceed them. The rules
feed two named stages, which `analyse_timeline` composes:

```python
@dataclass(frozen=True, slots=True)
class TaskTranscript:
    counted: tuple[StampedLine, ...]
    finished_in: float | None
    exit_status: int | None
    skipped_at: float | None


class OwnFigures(RecordModel):
    outcome: TaskOutcome
    exit_status: int | None
    start_seconds: float | None
    duration_seconds: float | None
    end_seconds: float | None
    units: CargoUnits
    lock_wait_seconds: float
    nextest_phase_seconds: float | None

    @staticmethod
    def skipped(at: float) -> "OwnFigures": ...
    @staticmethod
    def silent() -> "OwnFigures": ...
    @staticmethod
    def not_run() -> "OwnFigures": ...


class TaskFigures(OwnFigures):
    queueing_delay_seconds: float | None
    after_failure: bool

    @classmethod
    def placed(
        cls,
        own: OwnFigures,
        *,
        queueing_delay_seconds: float | None,
        after_failure: bool,
    ) -> Self: ...


def transcripts(
    lines: Iterable[StampedLine],
) -> Mapping[str, TaskTranscript]: ...
def own_figures(transcript: TaskTranscript) -> OwnFigures: ...
def place_in_graph(
    own: Mapping[str, OwnFigures], graph: TaskGraph, *, ready_from: float
) -> Timeline: ...
```

`TaskFigures` extends `OwnFigures` with the two figures that need the
graph, so a new per-task figure is declared once. It replaces the
`TaskFigures` sketched above, and its record keys are unchanged. The named
constructors return `OwnFigures`, not `Self`, so `TaskFigures` does not
inherit constructors that could only raise for want of its two extra
fields; only `TaskFigures.placed` builds one. Its record keys come in
declaration order, inherited fields first: `outcome`, `exit_status`,
`start_seconds`, `duration_seconds`, `end_seconds`, `units`,
`lock_wait_seconds`, `nextest_phase_seconds`, `queueing_delay_seconds`,
`after_failure`.

- **Stage 1, one task at a time.** `transcripts` normalises and groups
  the lines by their `[task]` prefix. `own_figures` derives everything a
  printing task's own lines decide. A task with no transcript gets
  `OwnFigures.silent()` or `OwnFigures.not_run()`, chosen by
  `analyse_timeline` from `graph.is_runnable`. The named constructors keep
  the rules that link an outcome to its `None` fields in one place: a
  skipped task has no duration, and a silent or unrun task has no start or
  end.
- **Stage 2, across the graph.** `place_in_graph` adds what needs other
  tasks: queueing delay through `ready_at`, and `after_failure`. It builds
  each `TaskFigures` with `TaskFigures.placed`.

**File**: `tasks/shared/build_measurement/fetch_failures.py`

```python
@unique
class ExternalInput(StrEnum):
    LAUNCHER_RELEASE = "launcher-release"
    ADVISORY_DATABASE = "advisory-database"
    NPM_REGISTRY = "npm-registry"


class FetchFailure(RecordModel):
    run: int
    task: str
    external_input: ExternalInput
    line: str


FETCH_FAILURE_PATTERNS: Mapping[ExternalInput, re.Pattern[str]] = (
    MappingProxyType(
        {
            ExternalInput.LAUNCHER_RELEASE: re.compile(r"\bcurl: \(\d+\)"),
            ExternalInput.ADVISORY_DATABASE: re.compile(...),
            ExternalInput.NPM_REGISTRY: re.compile(
                r"\bnpm (?:ERR!|error) code "
                r"(?:ETIMEDOUT|ECONNRESET|ENOTFOUND|EAI_AGAIN)\b"
            ),
        }
    )
)


def fetch_failures(
    lines: Iterable[StampedLine], *, run: int
) -> tuple[FetchFailure, ...]: ...
```

`run` is a parameter because `FetchFailure.run` is required and the lines
do not carry it. Lines are attributed to tasks by `attributed_lines`, which
`timeline.py` exposes so both modules strip colour and split the `[task]`
prefix the same way.

The launcher pattern matches what `bin/accelerator:152` prints, since it
runs `curl --fail --silent --show-error`.

#### 3. Capture the advisory-database failure text

With the network disabled, run `mise run --output prefix --skip-deps
deny:check`, the argv isolation mode uses, so each line carries the
`[deny:check]` prefix that `fetch_failures` attributes by. Copy the
`deny:check` lines it prints into
`tests/unit/tasks/fixtures/build-measurement/advisory-fetch-failure.log` as
a hand-seeded fixture, prefixed with a stamp. Then write the
`ADVISORY_DATABASE` pattern to match that text and not cargo-deny's
finding output. `deny:check` passes "advisories ok" in 1.43 s today, so the
finding case is hand-seeded from cargo-deny's documented
`error[vulnerability]` form. The module docstring of
`test_fetch_failures.py` records how the fixture was captured (the
command, the network disabled, the stamp prefix) and the cargo-deny
version, so it can be re-captured rather than hand-edited when the
wording changes.

Capture result (cargo-deny 0.19.8): with outbound network denied by
`sandbox-exec -p '(version 1)(allow default)(deny network-outbound (remote
ip))'` and the advisory database cached, the failed fetch is silent and
`deny:check` passes, so that case leaves nothing to detect. With a scratch
`CARGO_HOME` linking the real registry but holding no advisory database,
cargo-deny prints `[ERROR] failed to get 'FETCH_HEAD' metadata: …` and the
task fails. That text is the fixture, and the pattern matches it.

### Success Criteria:

#### Automated Verification:

- [x] The new tests pass: `uv run pytest tests/unit/tasks/shared/build_measurement -v`
- [x] The tasks unit lane passes: `mise run test:unit:tasks`
- [x] Lint and types pass: `mise run build-system:check`
- [x] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [x] The advisory fixture is the text cargo-deny actually printed offline,
      not a guess.

---

## Phase 2: Host activity analysis and run statistics

### Overview

This phase covers Requirement 1's CPU and process figures and Requirement
2's statistics:
- CPU busy from `iostat`, in 30-second buckets with the tasks running in
  each, and the idle-load mean;
- each tracked host process's CPU increase from `ps` snapshots;
- the median, min and max across runs, with flags for tasks and processes
  that have none.

### Changes Required:

#### 1. Tests first

**File**: `tests/unit/tasks/shared/build_measurement/test_cpu_load.py`

- `test_iostat_busy_is_user_plus_system`: `us` + `sy` from the sixth and
  fifth fields from the end.
- `test_the_first_iostat_report_is_discarded_as_a_since_boot_average`.
- `test_iostat_header_lines_are_skipped_wherever_they_repeat`.
- `test_buckets_average_the_readings_inside_each_30_seconds`.
- `test_a_reading_on_a_boundary_belongs_to_the_later_bucket`: buckets are
  half-open, so a reading at 30.0 is in `[30, 60)`.
- `test_the_final_partial_bucket_is_kept`: `until=75.0` gives three buckets.
- `test_a_bucket_with_no_reading_has_no_busy_figure`.
- `test_a_bucket_lists_the_tasks_running_at_its_midpoint`: a skipped task is
  never running.
- `test_a_silent_task_is_never_running`.
- `test_a_failed_task_runs_until_its_last_counted_line`.
- `test_idle_load_is_the_mean_of_the_readings`.
- `test_idle_load_with_no_reading_is_unknown`.

**File**: `tests/unit/tasks/shared/build_measurement/test_tracked_processes.py`

- `test_ps_time_parses_minutes_beyond_sixty`: `115:00.06` → 6900.06.
- `test_ps_time_parses_hours_and_days`: `1-02:03:04.50`.
- `test_a_pid_in_the_first_snapshot_counts_from_that_snapshot`.
- `test_a_pid_first_seen_mid_run_counts_from_zero`.
- `test_a_gap_marker_is_counted_not_read_as_a_snapshot`.
- `test_a_failed_first_tick_makes_the_cpu_unreadable`: `NoReading`, not a
  figure counted from zero.
- `test_a_gap_mid_run_makes_the_cpu_unreadable`.
- `test_the_markers_are_read_as_the_sampler_writes_them`: lines built with
  `sample_marker` and `gap_marker` parse back.
- `test_a_pid_that_exits_mid_run_keeps_its_last_seen_time`: seen in
  snapshots 1–3 with a growing TIME, gone from 4 and 5.
- `test_a_matching_pid_with_no_increase_is_zero_not_absent`.
- `test_matching_pids_are_summed`.
- `test_a_process_never_seen_is_absent_not_zero`.
- `test_the_ide_is_matched_by_its_main_executable_not_its_bundle`: the
  bundle's `fsnotifier` is not IDE time.
- `test_every_fsnotifier_counts`: IntelliJ's and PyCharm's.
- `test_ide_patterns_replace_only_the_ide_entry`: `with_ide_patterns`
  swaps the `ide` patterns and leaves `syspolicyd`, `XprotectService` and
  `fsnotifier` as they were.
- `test_a_ps_line_parses_into_pid_parent_time_and_command`.
- `test_an_unreadable_ps_line_parses_to_none`.
- `test_running_tracked_names_each_tracked_process_present`: matched
  against the given set, so an `--ide-pattern` override is honoured.
- `test_the_filter_matches_the_command_behind_padded_columns`: the raw line
  `  412     1  51:03.22 /usr/libexec/syspolicyd` is kept, so the anchored
  pattern is applied to the command field, not the whole line.
- `test_the_filter_drops_untracked_lines`.
- `test_the_filter_keeps_exactly_the_tracked_lines_of_a_real_tick`: over
  `ps-tick.golden`, with the known count from the `grep -cE` written beside
  it.

**File**: `tests/unit/tasks/shared/build_measurement/test_across_runs.py`

- `test_the_spread_of_three_values`: minimum, median and maximum.
- `test_a_task_clean_in_every_run_is_measured_with_spreads`.
- `test_a_task_failing_in_one_run_keeps_per_run_values_but_no_spread`.
- `test_a_task_after_a_failure_in_any_run_is_flagged_with_no_spread`.
- `test_a_task_skipped_in_every_run_is_absent_as_skipped`.
- `test_a_task_skipped_in_some_runs_is_flagged`.
- `test_a_task_silent_in_every_run_is_absent_as_silent`.
- `test_a_task_silent_in_some_runs_is_flagged`.
- `test_a_task_not_run_in_any_run_is_flagged_with_no_spread`.
- `test_failing_in_one_run_and_skipped_in_another_carries_both_flags`.
- `test_a_nextest_spread_needs_a_phase_in_every_run`.
- `test_a_run_with_a_failure_is_marked_and_still_counts_for_the_wall_clock`.
- `test_a_process_present_in_every_run_has_a_spread`: 51.3, 44.0, 78.2 →
  44.0, 51.3, 78.2.
- `test_a_run_using_no_cpu_keeps_the_process_present`: 0.0 in one run is a
  value, not an absence.
- `test_a_process_absent_from_every_run_is_absent_as_not_seen`.
- `test_a_process_absent_from_one_run_of_three_is_flagged_with_no_spread`:
  the caveat is `absent-in-some-runs`.
- `test_a_process_unreadable_in_a_run_is_flagged_with_no_spread`.
- `test_a_process_unreadable_in_every_run_is_flagged_not_absent`: it went
  unmeasured, which is not the same as never running.
- `test_unreadable_in_one_run_and_unseen_in_another_carries_both_caveats`.
- `test_tasks_and_processes_share_one_presence_shape`: a task skipped in
  some runs and a process absent from some runs are both `Flagged`.
- `test_a_flagged_presence_needs_a_caveat`: `Flagged(caveats=())` raises
  `ValidationError`.
- `test_a_spread_is_present_exactly_when_measured`: parametrised over each
  variant with and without a spread; the mismatches raise
  `ValidationError`.
- `test_each_presence_variant_round_trips_through_json`: the
  discriminator picks the variant back on read.

#### 2. Implementation

**File**: `tasks/shared/build_measurement/cpu_load.py`

```python
@dataclass(frozen=True, slots=True)
class CpuReading:
    at: float
    busy_percent: float


class CpuBucket(RecordModel):
    start_seconds: float
    busy_percent: float | None
    running: tuple[str, ...]


def cpu_readings(lines: Iterable[StampedLine]) -> tuple[CpuReading, ...]: ...
def idle_load(readings: Sequence[CpuReading]) -> float | None: ...
def cpu_buckets(
    readings: Sequence[CpuReading],
    timeline: Timeline,
    *,
    until: float,
    width: float = 30.0,
) -> tuple[CpuBucket, ...]: ...
```

**File**: `tasks/shared/build_measurement/tracked_processes.py`

`iostat` and `ps` change for different reasons, so each has a module: the
CPU-load figures above, and here the process catalogue, the `ps` format
and the per-process CPU.

```python
@dataclass(frozen=True, slots=True)
class TrackedProcess:
    name: str
    patterns: tuple[str, ...]

    def matches(self, command: str) -> bool: ...


DEFAULT_TRACKED_PROCESSES = (
    TrackedProcess("syspolicyd", (r"^/usr/libexec/syspolicyd(?:\s|$)",)),
    TrackedProcess("XprotectService", (r"/MacOS/XprotectService(?:\s|$)",)),
    TrackedProcess(
        "ide",
        (
            r"/Contents/MacOS/idea(?:\s|$)",
            r"/Contents/MacOS/rustrover(?:\s|$)",
        ),
    ),
    TrackedProcess("fsnotifier", (r"/fsnotifier(?:\s|$)",)),
)


@dataclass(frozen=True, slots=True)
class ObservedProcess:
    pid: int
    parent_pid: int
    cpu_seconds: float
    command: str


@dataclass(frozen=True, slots=True)
class ProcessSnapshot:
    at: float
    processes: tuple[ObservedProcess, ...]


@dataclass(frozen=True, slots=True)
class ProcessSamples:
    snapshots: tuple[ProcessSnapshot, ...]
    gaps: int

    @property
    def complete(self) -> bool:
        return self.gaps == 0


PS_ARGV = ("ps", "-ww", "-Ao", "pid=,ppid=,time=,command=")


def sample_marker(at: float) -> str: ...
def gap_marker(at: float) -> str: ...
def parse_ps_line(raw: str) -> ObservedProcess | None: ...
def process_snapshots(lines: Iterable[StampedLine]) -> ProcessSamples: ...
def tracked_line_filter(
    tracked: Sequence[TrackedProcess],
) -> Callable[[str], bool]: ...
def running_tracked(
    ps_output: str, tracked: Sequence[TrackedProcess]
) -> tuple[str, ...]: ...
def with_ide_patterns(
    tracked: Sequence[TrackedProcess], patterns: Sequence[str]
) -> tuple[TrackedProcess, ...]: ...
def cpu_used(
    sampled: ProcessSamples, tracked: TrackedProcess
) -> float | NoReading | None: ...
```

`ProcessSamples` is what `process_snapshots` returns: the snapshots and
the gap count. A gap on the first tick already counts as a gap, so no
separate flag is kept.

- **One `ps` line format.** `PS_ARGV` is the only spelling of the `ps`
  command, and `parse_ps_line` the only reading of its output: it splits a
  raw line with `maxsplit=3` and returns `None` for one it cannot read.
  `tracked_line_filter`, `process_snapshots` and `running_tracked` (the
  preflight's "running now" list) all use it. `-ww` stops `ps` sizing the
  command column to a terminal it may inherit; a truncated
  `…/XprotectService.xpc/Contents/MacOS/XprotectService` would never match
  its end-anchored pattern.
- **One marker vocabulary.** `sample_marker` and `gap_marker` are the only
  spellings of the sampler's markers; `PsProcessSampler` writes them and
  `process_snapshots` reads them.
- **`tracked_line_filter`.** It matches the patterns against the command,
  the fourth field. Matching the whole line would never satisfy an
  anchored pattern, because `ps` right-aligns the PID, PPID and TIME
  columns in front of it.

- **Stamped `ps` format.** For each tick whose `ps` exited 0, the sampler
  writes a `<at> sample` marker followed by one `<at> <pid> <ppid> <time>
  <command>` line for each tracked match. A tick that timed out, failed to
  decode or exited non-zero writes a `<at> gap` marker instead, and the
  sampler thread goes on. `process_snapshots` groups the lines under their
  `sample` marker and counts the gaps. A successful tick with no match is
  still a snapshot, which is how "present in the build's first sample"
  stays decidable.
- **`cpu_used`.** It takes each matching PID's last cumulative time, minus
  its time in the first snapshot, or minus 0 if it was not in that
  snapshot. "Last" is the last snapshot holding that PID, so a PID that
  exits mid-run keeps its time. It sums across PIDs, and returns `None` when
  no snapshot holds a match. When the samples are not `complete`, it
  returns `NoReading(PS_ARGV)` instead of a figure: a missing first tick
  would count every long-lived process from zero, and a gap would
  undercount, both without a sign in the record.
- **Buckets.** Each bucket is the half-open `[start, start + width)`, and
  the last bucket ends at `until` even when it is shorter. A task is running
  from its start to its end; a failed task ends at its last counted line,
  and a skipped or silent task is never running.
- **Marker stamps.** `sample_marker` and `gap_marker` write the stamp as
  `f"{at}"`, so `read_stamped` reads back the exact float. Phase 4's
  sampler stamps its kept `ps` lines the same way.
- **`iostat` lines.** A report line is one whose fields all parse as
  numbers, at least six of them. Reading `us` and `sy` from the end keeps
  the parse independent of how many disk columns come first. Probed on
  the reference host: `iostat -w 1` prints the `disk0 / cpu / load
  average` header and the column header once, then one report per
  interval.

**File**: `tasks/shared/build_measurement/across_runs.py`

```python
class Spread(RecordModel):
    minimum: float
    median: float
    maximum: float


def spread(values: Sequence[float]) -> Spread:
    return Spread(
        minimum=min(values), median=median(values), maximum=max(values)
    )


@unique
class Caveat(StrEnum):
    FAILED = "failed"
    NOT_RUN = "not-run"
    AFTER_FAILURE = "after-failure"
    SKIPPED_IN_SOME_RUNS = "skipped-in-some-runs"
    SILENT_IN_SOME_RUNS = "silent-in-some-runs"
    ABSENT_IN_SOME_RUNS = "absent-in-some-runs"
    UNREADABLE = "unreadable"


@unique
class Absence(StrEnum):
    SKIPPED = "skipped"
    SILENT = "silent"
    NOT_SEEN = "not-seen"


class Measured(RecordModel):
    standing: Literal["measured"]


class Flagged(RecordModel):
    standing: Literal["flagged"]
    caveats: tuple[Caveat, ...] = Field(min_length=1)


class Absent(RecordModel):
    standing: Literal["absent"]
    absence: Absence


type Presence = Annotated[
    Measured | Flagged | Absent, Field(discriminator="standing")
]


class TaskAcrossRuns(RecordModel):
    presence: Presence
    duration_seconds: Spread | None
    nextest_phase_seconds: Spread | None


class ProcessAcrossRuns(RecordModel):
    presence: Presence
    cpu_seconds: Spread | None


def task_across_runs(per_run: Sequence[TaskFigures]) -> TaskAcrossRuns: ...
def process_across_runs(
    per_run: Sequence[float | NoReading | None],
) -> ProcessAcrossRuns: ...
```

`Presence` is a discriminated union on `standing`, so each variant
carries only what it means, every reader branches once on the variant,
and a record holding an impossible state is refused on read:
- **`Measured`.** A value in every run, with no caveat, so it has a
  spread.
- **`Flagged`.** Per-run values only, with at least one `Caveat` saying
  why. A process missing from some runs is flagged with
  `absent-in-some-runs`, exactly as a task skipped in some runs is flagged
  with `skipped-in-some-runs`. A process whose CPU could not be read in a
  run, because of a `ps` gap, is flagged with `unreadable`,
  even when that holds for every run.
- **`Absent`.** Nothing in any run, with the `Absence` saying why: a task
  skipped or silent in every run, or a process never seen.

A `model_validator(mode="after")` on `TaskAcrossRuns` and
`ProcessAcrossRuns` requires a spread exactly when the presence is
`Measured`: `duration_seconds` and `cpu_seconds` are present then and
`None` otherwise. `nextest_phase_seconds` may still be `None` for a
measured task that does not run nextest. Each `*_across_runs` function
derives its `Presence` through one named function. Caveats are listed in
`Caveat`'s declaration order, so a record's caveats read the same whichever
run raised them. Absence wins over caveats: a task skipped in every run is
`Absent` even when a run also flagged it `after-failure`, because it
measured nothing. Each variant's `standing` defaults to its literal, so
`Measured()` and `Flagged(caveats=…)` are written without it. The caveats are named
`Caveat`, not `Flag`, because `enum.Flag` means bit flags to a reader of
enum code.

`median` is imported from `tasks.shared.measurement`.

#### 3. Capture a real `ps` tick

Before writing the real-tick filter test, run this on the reference host
with the IDE open on the project:

```bash
golden=tests/unit/tasks/fixtures/build-measurement/ps-tick.golden
ps -ww -Ao pid=,ppid=,time=,command= > "$golden"
tracked='(/usr/libexec/syspolicyd'
tracked+='|.*/MacOS/XprotectService'
tracked+='|.*/Contents/MacOS/(idea|rustrover)'
tracked+='|.*/fsnotifier)(\s|$)'
grep -cE "^\s*\S+\s+\S+\s+\S+\s+${tracked}" "$golden"
```

Write the count beside the test's constant. A full tick lists every
process on the host with its arguments, so only whole lines are kept:
every tracked line, plus untracked near-misses (the IDEs' bundle helpers,
a non-tracked IDE's main executable, the other Xprotect services) and a
few system daemons. No kept line is edited. The module docstring of
`test_tracked_processes.py` records these commands, split as above to fit
80 columns, with the trimming rule, the IDE-open precondition and the
macOS version, so it can be re-captured.

Capture result (macOS 26.3, `engywook`, IntelliJ IDEA open on the project
and PyCharm open too): the raw tick was 1,198 lines and 467 KB, with 5
tracked lines: `syspolicyd`, `XprotectService`, `idea`, and IntelliJ's
and PyCharm's `fsnotifier`. It also held unrelated apps' full arguments,
among them two Google API keys an Electron app embeds in its crash
annotations, which is why the golden is trimmed. The committed golden is
13 lines, and `_TRACKED_LINES_IN_GOLDEN` is 5.

### Success Criteria:

#### Automated Verification:

- [x] The new tests pass: `uv run pytest tests/unit/tasks/shared/build_measurement -v`
- [x] Lint and types pass: `mise run build-system:check`
- [x] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [x] `ps-tick.golden` is a real tick from the reference host with the IDE
      open, trimmed to whole lines, and holds `syspolicyd`, `idea` and at
      least one `fsnotifier`.

---

## Phase 3: Provenance

### Overview

This phase covers what a record says about where and when it was taken:
- the measured commit and whether the working copy matched it, in jj and
  git, verified against real repositories;
- the host description, including power and mise;
- the macOS check.

### Changes Required:

#### 0. Move the diagnostic runner and repository kind down a layer

This is a refactor under the existing tests, with no behaviour change:
- **`tasks/shared/diagnostics.py`** takes `DiagnosticRunner`,
  `ambient_diagnostic_runner` and `raw_diagnostic_runner` from
  `tasks/measure.py`, which imports them back.
- **`tasks/shared/repository.py`** takes `RepositoryKind` and
  `repository_kind`. It raises a new `NotARepositoryError(RuntimeError)` in
  place of `PreconditionFailureError`, which stays in `tasks/measure.py`.
  The one warm-dispatch caller, `snapshot_large_repository`, does not catch
  either, so only the traceback's type changes there.
- **`TestRepositoryKindDetection`** moves to
  `tests/unit/tasks/shared/test_repository.py`, asserting
  `NotARepositoryError`. Every other `test_measure.py` test stays green
  with only its imports changed.

#### 1. Tests first

**File**: `tests/unit/tasks/shared/build_measurement/repositories.py`

These are test builders for real repositories in `tmp_path`, with a hermetic
environment:
- **git:** `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=os.devnull`, the
  identity variables, and `GIT_CEILING_DIRECTORIES`.
- **jj:** `JJ_CONFIG` pointing at an empty file, plus `JJ_USER` and
  `JJ_EMAIL`.

Each builder offers `commit(files: Mapping[str, str], message: str) -> str`
and `merge(left: str, right: str) -> str`. The jj builder runs
`jj git init --no-colocate`, `jj new <a> <b>` and `jj commit`. The git
builder runs `git init`, `git commit` and `git merge --no-ff`. Both call
`require("jj")` or `require("git")` from `tests/support/tools.py`.

Each builder also offers `checkout`, which the branch and merge tests need:
`jj new <revisions…>`, or `git checkout --detach <revision>`. The
hermetic environment is applied with `monkeypatch` (`isolate`), not passed
to the builders, so the adapters under test inherit it too and a
contributor's own jj or git config cannot change their output.

**File**: `tests/unit/tasks/shared/build_measurement/test_provenance.py`

The pure tests run over a `FakeHistory`:
- `test_the_nearest_commit_changing_code_is_measured`.
- `test_a_commit_changing_only_measurements_is_skipped`.
- `test_an_empty_commit_is_skipped`.
- `test_a_history_with_no_qualifying_commit_is_refused`: a
  `NoMeasurableCommitError` whose message says no commit on the
  first-parent chain behind the working copy changes anything outside
  `meta/measurements/`.
- `test_changes_only_under_measurements_still_match`.
- `test_a_change_outside_measurements_does_not_match`.
- `test_a_kept_record_never_counts_as_a_change`: a path under
  `MEASUREMENTS_RELATIVE` is dropped by `outside_measurements`, which ties
  the commit search to wherever `--keep` writes.

**File**: `tests/unit/tasks/shared/build_measurement/test_history.py`

These are real-repository tests, parametrised over `JjHistory` and
`GitHistory`. They cover the kept-record and mismatch acceptance criteria
and the merge discovery:
- `test_an_uncommitted_kept_record_names_the_same_commit_and_matches`.
- `test_a_committed_kept_record_names_the_same_commit_and_matches`.
- `test_an_edit_outside_measurements_does_not_match`.
- `test_an_untracked_file_outside_measurements_does_not_match`: jj
  snapshots untracked files, so git must count them too.
- `test_a_clean_merge_is_measured_by_its_difference_from_its_first_parent`:
  the side branch adds `b.txt`, and the working copy at the merge matches.
- `test_a_merged_working_copy_follows_its_first_parent`: jj only. `@` is
  `jj new <a> <b>`, and the measured commit is on `<a>`'s line even when
  `<b>`'s line has the newer commit. `@-` would be both parents, so the
  chain starts at `first_parent(@)`.
- `test_a_gitignored_run_directory_does_not_break_the_match`: with
  `/.accelerator-measure/` in `.gitignore` and a file beneath it, the
  working copy still matches.
- `test_a_failing_command_names_its_argv_and_stderr`: a `HistoryError`
  from an unknown revision carries both.
- `test_the_adapters_only_issue_read_only_commands`: every argv's
  subcommand is in `{"log", "diff", "rev-list", "diff-tree", "ls-files"}`.

**File**: `tests/unit/tasks/shared/build_measurement/test_host.py`

These use the argv-keyed fake diagnostic pattern from
`test_measure.py:1061-1074`:
- `test_the_host_is_described_from_its_probes`, giving `engywook`,
  `Mac16,5`, `Apple M4 Max`, `26.3`, `25D125`, 16/12/4, `AC Power`, `0`,
  the mise version and `8`.
- `test_a_probe_that_fails_names_what_it_tried`, giving
  `NoReading(("sysctl", "-n", "hw.model"))`, rendered as
  `no reading from sysctl -n hw.model`.
- `test_each_unreadable_reading_names_what_it_tried`, each a `NoReading`,
  parametrised over:
  partial output from the three-OID core probe; `pmset -g` with no
  `powermode` line; `pmset -g ps` with no quoted source; a non-integer
  from `mise settings get jobs`; and empty output from each string
  reading (`scutil`, `hw.model`, the chip, both `sw_vers` readings and
  `mise --version`).
- `test_the_hostname_comes_from_scutil_local_host_name`.
- `test_power_source_comes_from_pmset_ps`.
- `test_power_mode_comes_from_pmset_g`.
- `test_the_job_limit_comes_from_mise_settings`.
- `test_a_non_macos_platform_is_refused_naming_macos`.
- `test_macos_is_accepted`.

#### 2. Implementation

**File**: `tasks/shared/build_measurement/errors.py`

Add `NoMeasurableCommitError`, `HistoryError` and `UnsupportedPlatformError`,
each a subclass of `MeasurementRefusedError`. Each message is a sentence
naming the cause; `HistoryError` also names the failed argv and its stderr.

**File**: `tasks/shared/build_measurement/locations.py`

```python
RUN_DIRECTORY_ROOT = Path(MEASURE_SCRATCH_DIRNAME) / "build"
SESSION_LOCK = RUN_DIRECTORY_ROOT / ".lock"
BUILD_PGID = RUN_DIRECTORY_ROOT / "build.pgid"
```

`tasks/shared/paths.py` gains `MEASURE_SCRATCH_DIRNAME =
".accelerator-measure"` and `MEASUREMENTS_RELATIVE = Path("meta") /
"measurements"`. `tasks/measure.py` defines `MANIFEST_DIRNAME` from the
first, and writes warm-dispatch records to `plugin_root /
MEASUREMENTS_RELATIVE` (`tasks/measure.py:1830`), so both harnesses name
the directory once, and the package uses `MEASUREMENTS_RELATIVE` under
that one name. These two are relative constants because both harnesses
root them, at `plugin_root` or at a test's `tmp_path`; the package's other
relative paths derive from `paths.py`'s absolute constants with
`.relative_to(REPO_ROOT)`.

**File**: `tasks/shared/build_measurement/provenance.py`

```python
class History(Protocol):
    def first_parent_chain(self) -> Iterator[str]: ...
    def changed_paths(self, commit: str) -> tuple[str, ...]: ...
    def working_copy_changes(self, commit: str) -> tuple[str, ...]: ...


def outside_measurements(paths: Iterable[str]) -> tuple[str, ...]:
    return tuple(
        p
        for p in paths
        if not PurePosixPath(p).is_relative_to(MEASUREMENTS_RELATIVE)
    )


def measured_commit(history: History) -> str:
    for commit in history.first_parent_chain():
        if outside_measurements(history.changed_paths(commit)):
            return commit
    raise NoMeasurableCommitError(
        "no commit on the first-parent chain behind the working copy "
        f"changes anything outside {MEASUREMENTS_RELATIVE}/"
    )


def working_copy_matches(history: History, commit: str) -> bool:
    return not outside_measurements(history.working_copy_changes(commit))
```

**File**: `tasks/shared/build_measurement/history.py`

Both adapters run with `cwd=root`, using a checked runner that raises
`HistoryError(MeasurementRefusedError)` on a non-zero exit.

The runner is a `HistoryCommandRunner` field defaulting to `checked_run`, so
`test_the_adapters_only_issue_read_only_commands` records every argv by
wrapping it. `HistoryError` keeps `argv` and `stderr` as attributes and
names both in its message.

| Operation | `JjHistory` | `GitHistory` |
|---|---|---|
| First-parent chain | `jj --ignore-working-copy log --no-graph -r 'first_ancestors(first_parent(@)) ~ root()' -T 'commit_id ++ "\n"'` | `git rev-list --first-parent HEAD` |
| Changed by a commit | `jj --ignore-working-copy diff --from 'first_parent(<c>)' --to <c> --name-only` | `git diff-tree -r --root --no-commit-id --name-only --diff-merges=first-parent <c>` |
| Working copy against a commit | `jj diff --from <c> --to @ --name-only`, which snapshots on purpose | `git diff --name-only <c>` plus `git ls-files --others --exclude-standard` |

`wiring.real_ports` chooses the adapter from `repository_kind`
(`tasks/shared/repository.py`, after step 0). A `.jj` directory always means
jj. That matters in a secondary jj workspace, where `git` resolves to the
parent repository.

**File**: `tasks/shared/build_measurement/host.py`

`NoReading` comes from `record_model.py` (Phase 1).

```python
class CoreCount(RecordModel):
    total: int | NoReading
    performance: int | NoReading
    efficiency: int | NoReading


class PowerState(RecordModel):
    source: str | NoReading
    mode: str | NoReading


class HostDescription(RecordModel):
    hostname: str | NoReading
    model: str | NoReading
    chip: str | NoReading
    os_version: str | NoReading
    os_build: str | NoReading
    cores: CoreCount
    power: PowerState


class MiseDescription(RecordModel):
    version: str | NoReading
    jobs: int | NoReading


def observed_build_host(diagnostics: DiagnosticRunner) -> HostDescription: ...
def observed_mise(diagnostics: DiagnosticRunner) -> MiseDescription: ...
def require_macos(system: str) -> None: ...
```

- **`DiagnosticRunner`.** The readings take the existing port from
  `tasks/shared/diagnostics.py` (step 0): `(argv) -> str`, raising
  `FileNotFoundError` or `PermissionError`, and otherwise returning the
  stripped output even when the command failed. A raised error, empty
  output or unparseable output therefore all yield `NoReading(argv)`;
  empty output matters most, since a failing `scutil` or `sysctl` returns
  `""` rather than raising. This replaces a bare `unknown` or an
  in-band string, following the 0217 lesson. It is written as
  `{"argv": [...]}` and rendered as "no reading from <argv>", so a
  consumer tells a reading from a failure by type, not by comparing
  strings. The readers are named `observed_*`, like `observed_host`.
- **The probes:**
  - `scutil --get LocalHostName` for the hostname. It is the name set in
    System Settings › Sharing and does not follow the network, unlike
    `socket.gethostname()`, which on macOS returns a DHCP-derived kernel
    hostname unless `HostName` was set. The hostname is part of the
    comparability key, and the baseline and 0361's closing record are
    months apart;
  - `sysctl -n hw.model`;
  - `sysctl -n machdep.cpu.brand_string`;
  - `sw_vers -productVersion` and `sw_vers -buildVersion`;
  - `sysctl -n hw.ncpu hw.perflevel0.logicalcpu hw.perflevel1.logicalcpu`;
  - `pmset -g ps`, whose first line is parsed between the quotes;
  - `pmset -g`, whose `powermode` line is parsed;
  - `mise --version` and `mise settings get jobs`.
- **`require_macos`.** It raises
  `UnsupportedPlatformError("measure:build supports macOS only; this host
  is Linux")`.
- **Probed on the reference host** (macOS 26.3, `engywook`): every probe
  printed what the tests assume. The three-OID core probe prints `16`,
  `12` and `4` on separate lines. `mise --version` writes its
  update-available warnings to stderr, which the ambient runner drops.

### Success Criteria:

#### Automated Verification:

- [x] The new tests pass, including the real-repository ones:
      `uv run pytest tests/unit/tasks/shared/build_measurement tests/unit/tasks/shared/test_repository.py -v`
- [x] The moved helpers leave warm-dispatch green: `uv run pytest tests/unit/tasks/test_measure.py -v`
- [ ] The tasks unit lane passes on both CI OSes: `mise run test:unit:tasks`
- [x] Lint and types pass: `mise run build-system:check`
- [x] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [x] In this jj workspace, `JjHistory(REPO_ROOT)` names the same commit as
      `jj log -r @-` when `@-` changes code, and reports a match on a clean
      working copy.

---

## Phase 4: `measure:build`, warm full runs

### Overview

This phase wires the pure core into a runnable task: priming, N timed runs,
the samplers, the record, `--keep`, `--host-settings`, the summary and the
exit codes. The task is registered with mise, documented, and checked
against a captured real log. After this phase the harness can record the
baseline.

### Changes Required:

#### 0. Move signal unwinding and tree reaping down a layer

This is a refactor under the existing tests, with no behaviour change:
- **`tasks/shared/signals.py`** takes `_UNWIND_SIGNALS` as the public
  `UNWIND_SIGNALS` and adds `unwinding_signals()`, a context manager that
  installs the handler swap of `tasks/measure.py:634-647` and restores the
  previous handlers on exit. `MeasurementSession` drops
  `_install_handlers`, `_restore_handlers` and `_unwind`, and enters
  `unwinding_signals()` through its own `ExitStack` in `__enter__`, so the
  swap exists once. The `unwind_signals()` accessor
  (`tasks/measure.py:569`) is deleted, since the constant is now public;
  its test moves to `test_signals.py` and asserts on `UNWIND_SIGNALS`.
  That keeps three letters from separating an accessor and a context
  manager.
- **`tasks/shared/processes.py`** gains `escalate(targets, procs, clock,
  grace, *, relist)`: terminate the targets, poll until the grace ends,
  adding whatever `relist()` returns on each tick, then kill what
  survives. `reap_tree(pid, procs, clock, grace)` is `escalate` over one
  snapshot of `pid` and its children with a `relist` that returns
  nothing, and `host_server._reap` now calls it.
- **Tests.** `tests/unit/tasks/shared/test_signals.py` takes the
  unwinding test modelled on `test_measure.py:2066-2094`.
  `tests/unit/tasks/shared/test_processes.py` gains direct `escalate` and
  `reap_tree` tests over `FakeProcs` and `FakeClock`: terminate first,
  then kill what outlives the grace period. The warm-dispatch and
  host-server tests stay green unchanged.

Then, test first, the additions the build runner needs and the dev server
does not:
- **`deferred_signals(numbers)`** in `tasks/shared/signals.py`: a context
  manager that, for its duration, swaps each given signal's handler for
  one that only records the signal. On exit it restores the previous
  handlers and, if any signal arrived, raises `KeyboardInterrupt` once.
  `signal.pthread_sigmask` would not do: it masks only the calling thread,
  so while the sampler threads are alive the kernel delivers the signal to
  one of them, and CPython still runs the Python handler in the main
  thread. A Python-level handler swap holds whichever thread the signal
  lands on, and changes nothing for child processes.
- **`unwind_then_defer(numbers)`**, its sibling: the first signal swaps
  every given handler to the recording one and then raises
  `KeyboardInterrupt`, so from that moment on later signals are recorded
  and dropped rather than raised. The build runner holds it from `Popen`
  to the end of its reap, so no window remains, between the first signal
  and entering the reap, in which a second one can abort the `finally`.
- **Tests** for both, each run inside `unwinding_signals()` as in
  production, so a regression that fails to swap a handler fails one test
  instead of killing pytest:
  - `test_a_signal_during_the_block_is_raised_after_it`: a live helper
    thread is running, and `os.kill(os.getpid(), SIGTERM)` inside the block
    raises nothing until the block exits;
  - `test_two_signals_during_the_block_raise_once`;
  - `test_the_previous_handlers_are_restored`;
  - `test_a_quiet_block_raises_nothing`;
  - `test_after_the_first_unwind_later_signals_are_held`: SIGINT then
    SIGTERM back to back inside `unwind_then_defer` raise exactly one
    `KeyboardInterrupt`, and the code after it in the block's `finally`
    runs to its end.
- **`reap_session(leader, procs, clock, grace)`** in
  `tasks/shared/processes.py`, with `ProcessOps` gaining
  `signal_group(pgid, number)`. In `PsutilProcessOps` that is
  `os.killpg`, suppressing `ProcessLookupError` and `PermissionError` as
  `terminate` and `kill` already do, so an emptied group never raises
  inside a `finally`. `reap_session`:
  1. snapshots the leader's descendants, with their create times, before
     signalling anything, since `children(leader)` returns nothing once
     the leader dies;
  2. sends SIGTERM to the leader's group, plus `escalate` over the
     snapshot with a `relist` of the leader's current descendants, which
     reaches tasks invoke ran under `pty=True` (their `setsid` takes them
     out of the group);
  3. always ends with SIGKILL to the group, even when every listed
     process has exited, for members that were reparented after the
     leader died.

  It signals a group only when `procs.group_of(leader)` equals `leader`
  (the leader leads its group) and differs from `procs.own_group()`;
  otherwise, including when `group_of` returns `None` because the leader
  has gone, it falls back to `reap_tree`. A caller whose child shares its
  group therefore never signals itself, and the check cannot raise inside
  a `finally`.
- **`ProcessOps`** gains four group queries:
  - `group_of(pid) -> int | None`: `os.getpgid`, `None` on
    `ProcessLookupError`;
  - `own_group() -> int`: `os.getpgrp()`;
  - `group_alive(pgid) -> bool`: `os.killpg(pgid, 0)`, `False` on
    `ProcessLookupError`. A `PermissionError` also reads as `False` for
    this harness's purposes: a group with no member this user may signal
    cannot be this user's build;
  - `group_members(pgid) -> tuple[tuple[int, str], ...]`: each live
    process whose `os.getpgid` is `pgid`, with its command line, read
    through `psutil.process_iter`; a process that vanishes or denies
    access mid-scan is skipped.

  Real-adapter tests in `test_processes.py`, each with a deadline:
  - `test_signalling_a_vanished_group_raises_nothing`: `signal_group` on
    the group of a child already waited for;
  - `test_a_vanished_group_is_not_alive`;
  - `test_a_group_this_user_cannot_signal_is_not_alive`: `group_alive(1)`,
    skipped when run as root, whom no permission stops;
  - `test_a_live_childs_group_is_alive`;
  - `test_a_live_childs_group_lists_its_members`: the child's PID and
    command.
- **`FakeProcs`** gains process groups: each `FakeProc` has a `pgid`,
  `group_of` and `group_members` answer from it, `own_group` returns a
  group the test sets,
  `signal_group` terminates or kills every live member, `group_alive`
  reports whether any member lives, and `children()` returns `[]` once
  the queried PID is dead, as `psutil` does. No test reaches a real
  `os.getpgid` with a fake PID. Tests:
  - `test_reap_session_signals_the_group_then_kills_what_outlives_the_grace`;
  - `test_reap_session_terminates_children_that_appear_during_the_grace`;
  - `test_an_orphaned_group_member_is_killed_by_the_closing_group_signal`:
    the leader dies on SIGTERM, and a member that ignores SIGTERM is
    reparented and killed only by the group SIGKILL;
  - `test_a_descendant_outside_the_group_is_reaped_from_the_snapshot`;
  - `test_a_vanished_group_is_not_an_error`;
  - `test_a_leader_in_the_callers_group_gets_no_group_signal`: the leader
    does not lead a group of its own;
  - `test_a_leader_leading_the_callers_group_gets_no_group_signal`: the
    leader's PID equals the caller's group, so only the `own_group` check
    stops the signal;
  - `test_a_leader_already_gone_is_reaped_without_a_group_signal`:
    `group_of` returns `None`.

#### 1. Tests first

**File**: `tests/unit/tasks/shared/build_measurement/doubles.py`

- **`ScriptedBuildRunner`.** It holds one `BuildScript` per call. Each
  script gives the lines, exit status and wall-clock for one build. It
  stamps each line at `clock.now() - epoch` plus the line's offset, then
  advances `FakeClock` by the wall-clock, so the sequencing and the
  offsets in a session test follow from what the session does rather than
  from fixed stamps. On every call the stub records its argv and a
  `WorkspaceObservation`: sentinel files present, the
  `cli/kernel/src/lib.rs` mtime, and which `node_modules` exist. Like mise,
  a script still prints lines for a failed task's dependants. Helpers:
  - `succeeded(task, at, seconds, *output)`;
  - `failed(task, at, status, *output)`;
  - `skipped(task, at)`.
- **`ScriptedSampler`.** It returns canned stamped lines per start and
  stop, and records the epochs it was started with.
- **`FakeHistory`, `fake_diagnostics(table)`, `FakeClock`.** `FakeClock` is
  reused from `tests/unit/tasks/shared/doubles.py`.
- **`untouchable_ports()`.** Ports that fail the test if any of them is
  called, for refusal tests, following the `nothing_read` precedent in
  `test_vendor_pin_guard_task.py:217-223`.
- **`measured_build(**overrides)`**, with `run_figures(...)` and
  `task_across_runs(...)` helpers. It builds a valid `MeasuredBuild`,
  deriving `conditions`, the top-level `fetch_failures`, `failed_tasks`,
  `commit.matched`, `contains_failure` and `unit_totals` from their
  sources, so every fixture passes the validators and an additive schema
  field means editing one builder. Every test that needs a record builds
  it through this.
- **`FakeBuildGroup`.** Answers `orphaned()` as the test sets it and
  records `remember` and `forget` calls.

**File**: `tests/unit/tasks/shared/build_measurement/test_session.py`

| Test | Asserts |
|---|---|
| `test_settings_refuse_runs_below_one` | `InvalidSettingsError` |
| `test_settings_refuse_an_unknown_label_listing_the_known_ones` | `InvalidSettingsError` naming all three labels |
| `test_settings_track_the_default_processes` | `tracked == DEFAULT_TRACKED_PROCESSES` without `ide_patterns` |
| `test_settings_refuse_an_ide_pattern_that_is_not_a_regex` | `--ide-pattern '['` → `InvalidSettingsError` naming the pattern and `re`'s message |
| `test_a_non_macos_host_is_refused_before_anything_runs` | `UnsupportedPlatformError` naming macOS; no port touched |
| `test_idle_load_is_sampled_for_30_seconds_before_priming` | the sampler starts, `FakeClock` sleeps 30, then the sampler stops, all before the first build |
| `test_a_warm_session_primes_then_runs_n_timed_builds` | 1 + N build calls; priming wall-clock is kept and discarded |
| `test_the_default_build_command` | `("mise", "run", "--output", "prefix", "--continue-on-error", "default")`, one invocation per timed run |
| `test_each_invocation_is_ready_from_its_own_start` | the `ready_from` handed to the analysis is the clock reading just before that invocation |
| `test_samplers_bracket_each_timed_run_only` | started before and stopped after each timed build; not around priming |
| `test_a_failure_in_one_run_is_reported_and_the_session_completes` | the injected failing task is `failed`, its dependants are `after_failure`, and the run is `contains_failure` |
| `test_a_non_zero_mise_exit_marks_the_run_even_with_no_failed_task` | an early abort, every closure task silent or not run, and mise exits 2: the run is `contains_failure` |
| `test_a_task_not_run_marks_the_run_even_when_mise_exits_zero` | the `NOT_RUN` task is in `Timeline.failed()` and `failed_tasks`, and the run is `contains_failure` |
| `test_a_task_failing_only_in_run_two_keeps_three_values_no_spread` | per-run values kept; wall-clock spread covers all three runs |
| `test_each_log_is_written_as_its_run_happens` | after the session, the run directory holds `idle-iostat.log`, `priming.log`, `run-<n>.log`, `run-<n>-iostat.log` and `run-<n>-ps.log`, each holding the scripted lines |
| `test_an_interrupt_in_run_two_keeps_the_logs_and_writes_no_record` | `ScriptedBuildRunner` raises `KeyboardInterrupt` in run 2; `priming.log` and `run-1.log` exist, `record.json` does not, the exception propagates, and `progress` received the run directory's absolute path after the interrupt |
| `test_both_samplers_stop_when_the_build_raises` | every `ScriptedSampler` started in run 2 has been stopped by the time the exception leaves `measure_build` |
| `test_the_idle_sampler_stops_when_idle_sampling_is_interrupted` | `FakeClock.sleep` raises `KeyboardInterrupt`; the CPU sampler is stopped |
| `test_a_failing_end_check_records_an_unknown_match` | `working_copy_changes` raises `HistoryError` on its second call; `matched_at_end is None`, `matched is False`, and a record is still written |
| `test_the_match_is_checked_before_priming_and_after_the_last_run` | two `working_copy_changes` calls |
| `test_matched_needs_both_checks_to_pass` | parametrised over (match, mismatch) and (mismatch, match): `matched` is `false` in both |
| `test_the_task_set_is_the_default_closure` | from the injected graph |
| `test_the_run_takes_the_kernel_hostname_when_scutil_has_none` | parametrised over `scutil` raising and returning `""`: the run id carries `ports.kernel_hostname()`, never "no reading from" or an empty host |
| `test_the_session_opens_in_one_order` | a recording double sees the first port calls in the stated order: platform, the session lock, the orphan check, the run directory. Phases 6 and 7 extend this one test with their checks rather than each claiming "right after the platform check" |
| `test_a_held_session_lock_refuses_before_anything_else_runs` | the fake lock raises `SessionInProgressError`; no run directory, sampler or build is touched |
| `test_an_orphaned_build_refuses_while_the_lock_is_held` | `FakeBuildGroup` reports an orphan; `OrphanedBuildError` is raised inside the lock, and no run directory is created |
| `test_the_session_lock_is_held_from_start_to_finish` | the fake records entry before the run directory and exit after the end-of-run match |
| `test_idle_load_is_reported_once_sampled` | `progress` gives the idle-load mean as soon as idle sampling ends, before priming |
| `test_the_process_sampler_is_given_the_settings_tracked_set` | every `ScriptedSampler` start for processes receives `settings.tracked` |
| `test_progress_reports_each_stage_in_order` | a recording `progress` sees the run directory, the preflight block, idle sampling, priming start and end, and for each timed run its start, each invocation's start with its absolute log path, and its end (with wall-clock and the failed tasks' names), in that order |
| `test_the_preflight_comes_before_idle_sampling` | the preflight block is reported before the CPU sampler first starts |
| `test_a_fetch_failure_is_reported_at_the_end_of_its_run` | with `--keep`, run 1 of 3's end line names the external input, says the record will not be kept, and says "press Ctrl-C to stop, then rerun", before run 2 starts |
| `test_the_last_runs_fetch_failure_gives_no_ctrl_c_hint` | on the final timed run, and with `--runs 1`, the end line names the failure without the hint, since stopping then would only lose the record |
| `test_the_preflight_names_the_tracked_processes_running_now` | a scripted `ps` output through `fake_diagnostics` and a custom `--ide-pattern`: the preflight reports the IDE as running, matched against `settings.tracked`, and names the command lines the IDE patterns matched |

**File**: `tests/unit/tasks/shared/build_measurement/test_record.py`

- `test_the_run_id_names_the_time_host_cache_state_and_label`:
  `build_2026-10-30T162000Z_engywook_warm-no-op_devtools`.
  Non-`[A-Za-z0-9-]` characters in the hostname, `_` included, become `-`,
  and `devtools+ide-exclusion` keeps its `+`.
- `test_an_isolated_run_id_says_so`: `…_warm-no-op_devtools_isolated`.
- `test_each_field_of_a_run_id_is_recovered_by_splitting_on_underscores`:
  parametrised over a hyphenated hostname and the `devtools+ide-exclusion`
  label.
- `test_the_record_is_written_to_the_run_directory`: `record.json`, whole,
  with no temporary file left beside it.
- `test_the_record_names_its_run_directory_relative_to_the_root`.
- `test_without_keep_nothing_is_written_under_measurements`.
- `test_keep_also_writes_the_record_under_measurements`.
- `test_keep_never_overwrites_an_existing_record`: an existing kept file
  with the same name is left as it was, and the write is refused.
- `test_no_temporary_file_is_ever_left_under_measurements`: the kept
  copy's temporary lives in the run directory.
- `test_the_top_level_keys_come_in_the_documented_order`:
  `schema_version` first, then the order of the record example.
- `test_a_record_without_a_schema_version_is_refused`.
- `test_a_record_whose_copies_disagree_is_refused`: parametrised over each
  of the four mirrored `conditions` fields (derived from the set the
  validator mirrors), and over both directions of each list: a top-level
  fetch failure missing from the runs and a run's missing from the top
  level; a task in `failed_tasks` that did not fail and a failed task not
  listed.
- `test_a_stored_derived_field_that_disagrees_is_refused`: parametrised
  over `commit.matched` against its two checks, `contains_failure` against
  the run's outcomes and exit status, and `unit_totals` against the
  tasks' units.
- `test_started_at_is_written_with_an_offset_and_whole_seconds`:
  `2026-10-30T16:20:00+00:00`, with no `Z` and no fraction.
- `test_conditions_holding_the_same_no_reading_are_not_comparable`:
  `comparable_with` is false although `==` is true.
- `test_every_committed_build_record_is_readable`: parametrised over every
  `meta/measurements/build_*.json`, asserting `parse_record` accepts it.
  It has nothing to run until Phase 5 commits the baseline. From then on,
  a model edit in any later phase that would make a committed record
  unreadable fails CI in the commit that makes it.
- `test_one_run_serialises_as_the_record_example_shows`: a known answer
  for one `RunFigures`, covering task figures, buckets and process CPU.
- `test_keep_withholds_a_record_with_fetch_failures`: nothing is written
  under `meta/measurements/`, `record.json` is still written, and
  `WrittenRecord.kept is None`.
- `test_the_run_directory_is_gitignored`: the run root's first part is
  `.accelerator-measure` and `.gitignore` holds `/.accelerator-measure/`.
- `test_the_record_carries_every_required_metadata_field`: every metadata
  field the record must state, per the work item's "Given any record"
  acceptance criterion.
- `test_the_summary_block_carries_spreads_and_standings`: a known answer
  for the serialised `summary` of one `MeasuredBuild`, keyed exactly as the
  record example shows.
- `test_a_record_reads_back_equal_to_what_was_written`:
  `parse_record(record_text(m)) == m`, parametrised over a session with a
  failed task, a skipped task, an absent process and a `None` idle load,
  over a host with a `NoReading` in both a `str` and an `int` field, and
  over a session with a `ps` gap, so a `NoReading` sits in
  `runs[].process_cpu_seconds`. Phases 6 and 7 add an isolated record and
  a cold record with no priming.
- `test_every_committed_build_record_is_readable` takes its glob from
  `MEASUREMENTS_RELATIVE` and the run-id prefix constant, rooted at
  `REPO_ROOT`, so a renamed prefix or directory cannot quietly empty it.
- `test_the_baseline_is_among_the_committed_records`: asserts a named
  path is in that glob's matches. It is skipped, with the reason "no
  baseline committed yet", until Phase 5 names the baseline's path.
- `test_an_unknown_schema_version_is_refused_naming_the_file`.
- `test_a_malformed_field_is_refused_naming_its_location`: a string where
  `wall_clock_seconds` should be a number.
- `test_a_warm_dispatch_record_is_refused`.
- `test_the_summary_names_each_failed_task`.
- `test_the_summary_warns_of_output_contract_drift`: mise exited 0, yet a
  runnable task has no verdict; the warning names the task and the
  recorded mise version.
- `test_the_summary_says_a_record_with_fetch_failures_was_not_kept`:
  `not kept: 1 fetch failure (advisory-database in run 2); retake`.
- `test_the_summary_names_fetch_failures_without_keep_too`: a `fetch
  failures:` line whenever the list is not empty, as Requirement 1 asks.
- `test_the_summary_gives_the_idle_load`.
- `test_the_summary_gives_the_wall_clock_spread_and_record_paths`.
- `test_the_summary_says_whether_the_working_copy_matched`: matched,
  mismatched at start or at end, and unknown at end.
- `test_the_summary_gives_each_tracked_processes_cpu`.
- `test_the_summary_hints_a_retake_when_a_required_process_was_absent`:
  `syspolicyd`, `ide` or `fsnotifier`; not `XprotectService`.
- `test_the_preflight_report_shows_the_conditions`.
- `test_the_preflight_warns_of_each_condition_a_kept_record_fails`:
  parametrised over `unspecified`, a mismatch, battery, and each required
  process absent; no warning without `--keep`.

**File**: `tests/unit/tasks/shared/build_measurement/test_build_runner.py`

These are portable real-process tests, using `sys.executable -u` scripts
as the child, so output is unbuffered. Every poll has a deadline that
fails the test, and every child's sleep is capped, so a regression fails
rather than hanging a CI leg:
- `test_the_build_stamps_lines_in_arrival_order`: a script prints, sleeps
  0.5 s and prints again, and the stamps differ by more than 0.3, leaving
  margin for a loaded runner.
- `test_the_build_merges_stderr_and_reports_the_exit_status`.
- `test_the_log_holds_every_stamped_line_the_run_returns`.
- `test_an_interrupted_build_leaves_no_child_or_grandchild`: with the real
  `PsutilProcessOps`, the child spawns a sleeping grandchild and prints
  both PIDs. A helper thread polls the runner's log file until both PIDs
  appear, which also proves lines reach disk before the run ends, then
  sends SIGTERM to the main thread with `signal.pthread_kill` under
  `unwinding_signals()`. The `KeyboardInterrupt` propagates, and
  `PsutilProcessOps().is_alive` is false for both PIDs within a short
  deadline; `psutil.pid_exists` would count a zombie as alive.
- `test_a_carriage_return_stays_inside_its_line`: the child writes
  `[t] Building\r   Compiling foo v0.1.0\n`, and one stamped line comes back
  holding both segments.
- `test_an_invalid_utf8_byte_is_replaced_not_fatal`.
- `test_the_builds_group_is_remembered_while_it_runs_and_forgotten_after`:
  the child prints a line once started; once that line is in the log,
  `build.pgid` names the child's PID, and it is gone once the reap
  finishes, including after an interrupt.
- `test_a_signal_during_the_reap_cannot_skip_the_kill`: the child
  ignores SIGTERM, and the injected `ProcessOps` wraps `PsutilProcessOps`,
  sending `os.kill(os.getpid(), SIGTERM)` on its first `signal_group`
  call. It is parametrised over how the reap is entered: an interrupt, so
  the injected signal is the second, and an error raised while reading
  the output (a log path that cannot be written), so it is the first and
  meets `deferred_signals`. Either way the child's group is dead
  afterwards, `build.pgid` is gone, and exactly one `KeyboardInterrupt`
  propagates after the reap.
- `test_a_group_that_survives_the_reap_is_named_with_its_kill_command`:
  the wrapped `ProcessOps` reports the group alive after the reap, and
  the propagated message names the PGID and `kill -KILL -<pgid>`; after a
  quiet reap the message does not mention it.

**File**: `tests/unit/tasks/shared/build_measurement/test_samplers.py`

- `test_the_stream_sampler_stamps_each_line_until_stopped`.
- `test_the_process_sampler_writes_a_marker_per_tick_and_only_tracked_lines`:
  the child stands in for `ps` and prints a tracked and an untracked line.
- `test_a_failing_tick_writes_a_gap_and_the_sampler_goes_on`: the stand-in
  exits non-zero on one tick, then succeeds; the log holds a `gap` marker
  followed by further `sample` markers.
- `test_a_tick_that_times_out_writes_a_gap`: with `tick_timeout=0.2`, a
  stand-in that sleeps past it gives a `gap` marker, and the thread lives.
- `test_each_sampler_writes_its_lines_to_its_log`.
- `test_stopping_twice_returns_the_same_lines`.
- `test_a_stopped_stream_sampler_leaves_no_process`: the `iostat`
  stand-in's PID no longer exists after `stop()`.
- `test_sampler_threads_are_daemons`.

**File**: `tests/unit/tasks/shared/build_measurement/test_session_lock.py`

`WorkspaceSessionLock` adapts `workspace_lock`, whose own tests cover the
locking, so these cover the refusal:
- `test_a_held_lock_is_refused_saying_another_session_is_running`: two
  `WorkspaceSessionLock(tmp_path)` instances, the second entered while the
  first is held. The message says to wait for the other session and not to
  delete the lock file.
- `test_a_released_lock_can_be_taken_again`.

**File**: `tests/unit/tasks/shared/build_measurement/test_build_group.py`

Over `FakeProcs` and a `tmp_path` root:
- `test_a_remembered_group_reads_back`: `remember(pgid, create_time)`
  writes with `atomic_write_text`, and `read()` returns both.
- `test_the_living_leader_marks_an_orphan`: the process at the PGID has
  the recorded create time.
- `test_a_live_group_with_no_leader_is_an_orphan`: the leader has gone,
  members remain.
- `test_a_reused_pid_makes_the_file_stale`: a process at that PID with
  a different create time; the file is removed and nothing is refused.
- `test_a_group_this_user_cannot_signal_is_not_the_orphan`.
- `test_an_implausible_group_is_stale`: parametrised over `0`, `1` and the
  caller's own group.
- `test_an_unreadable_file_is_stale`: empty, not two integers, or
  truncated; removed.
- `test_the_orphans_refusal_lists_its_members_and_both_kill_commands`:
  each live member's PID and command, `kill -TERM -<pgid>`, then `kill
  -KILL -<pgid>` for members that survive TERM, then "if none is, delete
  <path>".

**File**: `tests/unit/tasks/shared/build_measurement/test_imports.py`

- `test_no_pure_module_imports_the_session_or_an_adapter`: parametrised
  over the pure modules in the module table, reading each one's imports
  with `ast`.
- `test_only_wiring_imports_the_adapters`.

**File**: `tests/unit/tasks/shared/build_measurement/test_captured_run.py`

These tests run against the goldens captured in step 5. Each known answer
is derived independently at capture time and recorded as a module constant
beside the command that produced it. The module docstring records the
capture steps and the mise version the goldens came from, as the
`GOLDEN_ASSET_BYTES` precedent does:
- `test_the_silent_tasks_are_exactly_the_roll_ups`: the oracle is
  `jq` over `warm-no-op-tasks.golden.json` for closure tasks whose `run`
  is `[]`.
- `test_the_skipped_tasks_are_exactly_the_up_to_date_ones`: the oracle is
  `zgrep 'sources up-to-date, skipping'`.
- `test_test_unit_cli_duration_is_its_finished_in_figure`.
- `test_test_unit_cli_nextest_phase_is_its_summary_figure`.
- `test_test_unit_cli_compiling_count_matches_the_log`: the oracle counts
  occurrences with `grep -o`, not lines, since `\r`-joined segments share a
  line.
- `test_one_dependants_queueing_delay`: `test:unit:cli`'s, worked out by
  hand from its `$` line and its latest dependency's end.
- `test_the_iostat_golden_gives_the_known_readings_and_first_bucket`: the
  reading count after the since-boot report, and the first bucket's mean of
  `us + sy`, both by `awk`.
- `test_one_tracked_process_cpu_increase`: `syspolicyd`'s TIME at the last
  tick minus the first, by `awk`.
- `test_every_other_closure_task_succeeded`: outside the roll-ups and the
  up-to-date set, every closure task's outcome is `succeeded`, so a
  misclassified task in a clean warm run fails here rather than in the
  baseline.

**File**: `tests/unit/tasks/test_mise.py`

- Extend `test_the_measure_module_is_reached_by_at_least_one_task` so its
  expected set includes `measure:build`.

**File**: `tests/unit/tasks/test_measure.py`

- Add `test_build_is_documented_without_a_quiet_host` to
  `TestMeasureNamespaceDocs`. The measure section must name
  `measure:build`, `.accelerator-measure/build/` and `--keep`, say a quiet
  host is not required, say per-process CPU is a lower bound, and name
  `presence` and `conditions`.
- Add `test_taking_a_comparable_record_is_documented`: the section has a
  `Taking a baseline or condition record` list naming each
  `--host-settings` label, keeping the IDE open, making no commit during a
  run, retaking on a fetch failure, and running nothing heavy in another
  workspace.
- Add `TestBuildTask`, for the flags. An autouse fixture replaces
  `real_ports` with a factory that fails the test if called, following the
  `nothing_read` precedent:
  - `test_runs_below_one_is_refused`;
  - `test_an_unknown_host_settings_label_is_refused_listing_the_known_ones`;
  - `test_runs_default_to_three`: `measure_build_with` is monkeypatched to
    capture, and call, the settings factory it receives, following
    `test_the_issue_author_and_cap_reach_the_policy`;
  - `test_an_ide_pattern_replaces_only_the_ide_in_the_settings`;
  - `test_each_flag_has_help`;
  - `test_repeated_flags_are_parsed_as_lists`: `measure.build
    --ide-pattern a --ide-pattern b` goes through invoke's own parser, so
    a missing `iterable` entry fails here.
- Add `TestMeasureBuildWith`, for the refusal-to-exit mapping, over fake
  ports rooted at `tmp_path`:
  - `test_a_failed_task_still_exits_zero_writes_the_record_and_names_it`:
    the scripted mise exits 3, and `capsys` shows the failed task;
  - `test_a_non_macos_host_exits_one_naming_macos_and_creates_nothing`:
    the `system` port returns `Linux`, and `tmp_path/.accelerator-measure`
    does not exist;
  - `test_a_directory_that_is_no_repository_exits_one`: the ports factory
    is `real_ports`, given a `tmp_path` root that is a plain directory, so
    the real `history_of` conversion is exercised; building the real ports spawns
    nothing. The exit names the directory.

**File**: `tests/unit/tasks/shared/build_measurement/test_wiring.py`

- `test_a_directory_with_jj_gets_jj_history`: including alongside `.git`,
  since in a secondary jj workspace `git` resolves to the parent.
- `test_a_git_directory_gets_git_history`.
- `test_a_plain_directory_is_refused_naming_it`: a
  `MeasurementRefusedError`, not the bare `NotARepositoryError`.

#### 2. Session and record

`HostSettings` and `Mode` are part of the record, so they live in
`measured_build.py` with the rest of its schema (below).

**File**: `tasks/shared/build_measurement/settings.py`

```python
@dataclass(frozen=True, slots=True)
class Priming:
    log_stem: ClassVar[str] = "priming"


@dataclass(frozen=True, slots=True)
class TimedRun:
    number: int

    @property
    def log_stem(self) -> str:
        return f"run-{self.number}"


type SessionRun = Priming | TimedRun


def log_name(run: SessionRun, suffix: str = "") -> str:
    return f"{run.log_stem}{suffix}.log"


@dataclass(frozen=True, slots=True)
class Invocation:
    argv: tuple[str, ...]
    log_suffix: str
    graph: TaskGraph
    reported: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class FullBuild:
    mode: ClassVar[Mode] = Mode.FULL
    isolated_tasks: ClassVar[tuple[str, ...]] = ()

    def invocations(self, graph: TaskGraph) -> tuple[Invocation, ...]: ...


type BuildScope = FullBuild


@dataclass(frozen=True, slots=True)
class BuildSettings:
    root: Path
    runs: int
    keep: bool
    host_settings: HostSettings
    tracked: tuple[TrackedProcess, ...]
    scope: BuildScope

    @classmethod
    def of(
        cls,
        *,
        root: Path,
        runs: int,
        keep: bool,
        host_settings: str,
        ide_patterns: Sequence[str],
    ) -> Self: ...
```

**File**: `tasks/shared/build_measurement/session.py`

```python
@dataclass(frozen=True, slots=True)
class BuildPorts:
    system: Callable[[], str]
    build: BuildRunner
    cpu_sampler: Sampler
    process_sampler: ProcessSampler
    history: History
    diagnostics: DiagnosticRunner
    kernel_hostname: Callable[[], str]
    mise_tasks: Callable[[], str]
    clock: Clock
    utc_now: Callable[[], dt.datetime]
    progress: Callable[[str], None]
    session_lock: Callable[[], AbstractContextManager[None]]
    build_group: BuildGroup


def measure_build(
    settings: BuildSettings, ports: BuildPorts
) -> SessionOutcome: ...
```

`SessionOutcome`, declared in `reports.py` beside the summary that
consumes it, will separate what is recorded from what the operator must
be told. Phase 7 adds its `notices`, the leftover cold trash and the
dev-launcher warning, which are not part of the frozen record; returning
the wrapper from the start means that adds a field rather than changing
`measure_build`'s and `summary_report`'s signatures. The session's ports (`BuildRunner`, `Sampler`,
`ProcessSampler`, `BuildGroup` and, from Phase 7, `Workspace`) are all
declared in `session.py`, beside `BuildPorts`.

`BuildSettings.of` is the only way to build settings. It refuses with
`InvalidSettingsError(MeasurementRefusedError)` when `runs` is below 1
("--runs must be at least 1") or the label is unknown ("unknown
--host-settings 'x'; known: none, devtools, devtools+ide-exclusion"). It
compiles each `--ide-pattern`, refusing one that is not a regex with the
pattern and `re`'s message, and builds `tracked` with `with_ide_patterns`.
Phases 6 and 7 add their flags and combination rules here, so an illegal
plan cannot reach `measure_build`.

`BuildScope` says what one build runs, for priming and for each timed run
alike: `invocations(graph)` takes no run, since nothing it yields depends
on which run it is. `FullBuild.invocations` yields a single `Invocation`:
argv `("mise", "run", "--output", "prefix", "--continue-on-error",
"default")`, an empty log suffix, the whole graph, and the `default`
closure as the tasks to report. `SessionRun` owns the naming:
`log_name(run, suffix)` is the one place a log file is named, so priming
writes `priming.log`, timed run 2 writes `run-2.log`, and its samplers
write `run-2-iostat.log` and `run-2-ps.log`. Phase 6 adds `IsolatedTasks`
to the union, and the session loops over invocations the same way for
both, so a mode adds a scope rather than a branch. Every scope has `mode`
and `isolated_tasks`, from which the record's conditions derive without
inspecting the scope's type.

`measure_build` runs these steps in order, each a named helper. Capture
comes first and analysis last, so whatever ends a session early leaves
every log already captured on disk. The session opens in one stated
order, which `test_the_session_opens_in_one_order` asserts and Phases 6
and 7 extend: the platform check; Phase 6's `--isolate` name check;
Phase 7's `workspace.verify`; the session lock; the orphan check; the run
directory. Both checks before the lock only read, and the cold path
repeats its containment check before every deletion, so running them
outside the lock is safe.
1. `require_macos(ports.system())`.
   Then the session enters `ports.session_lock()` and holds it until it
   returns. The real lock, `WorkspaceSessionLock`, is a thin adapter over
   `tasks/shared/locking.py` `workspace_lock(SESSION_LOCK)`, the same
   one-per-workspace lock the dev lifecycle takes, with its `mkdir`
   fallback. When `workspace_lock` yields `False`, it refuses with
   `SessionInProgressError(MeasurementRefusedError)`: "another
   measure:build session is running in this workspace; wait for it to
   finish (its lock is released when it exits; do not delete <path>)".
   The kernel releases a `flock` when its process dies, so an interrupted
   session leaves nothing to clean up.

   Holding the lock, the session asks `ports.build_group.orphaned()`
   about any `BUILD_PGID` left by a session that died without unwinding.
   `BuildGroupFile` (in `build_group.py`) is the only code that knows
   that file: it writes the leader's PGID and create time with
   `atomic_write_text`, reads them back, and judges them. The file names
   an orphan, with its members from `procs.group_members`, only when the process at that PID still has the recorded
   create time, or when no process holds the PID but the group still has
   members. It is stale, and removed, when the PID now belongs to another
   process, when the group is gone or not this user's to signal, when the
   PGID is 1 or less or the caller's own, or when the file cannot be read.
   For an orphan, the session refuses with
   `OrphanedBuildError(MeasurementRefusedError)`, which lists each live
   member's PID and command, then says: "if these are an earlier
   measure:build's mise, cargo or nextest, stop them with `kill -TERM
   -<pgid>` (then `kill -KILL -<pgid>` for any that survive) and retry; if
   none is, delete <path>".
2. The run directory: `<root>/.accelerator-measure/build/<run-id>/`,
   created at once. The session reads `ports.utc_now()` once and drops
   its microseconds; both the run id and `started_at` come from that one
   value, so the record's start reads like the `meta/` frontmatter dates
   and the round trip stays exact. The host description is read here, once:
   the run id takes its `hostname`, from `scutil --get LocalHostName`, or
   `ports.kernel_hostname()` when that is a `NoReading`, so the run id and
   `conditions.hostname` cannot come from different reads.
3. Provenance: the commit and the match at start, the host description
   read in step 2, mise, and the task set.
4. Preflight: one `ports.diagnostics(PS_ARGV)` reading, read with
   `running_tracked(output, settings.tracked)`, gives the tracked
   processes running now, and `preflight_report` (pure, in `reports.py`)
   goes out through `ports.progress`. It shows the commit and whether the
   working copy matches (naming up to five differing paths), the label,
   the power source, the mise job limit, the tracked processes running,
   and the command lines the IDE patterns matched. It warns, without refusing, when `--keep` meets an
   `unspecified` label, a working copy that does not match, battery power,
   or an absent `syspolicyd`, IDE or `fsnotifier`. The operator can Ctrl-C
   before the 20 minutes are spent.
5. Idle load: the CPU sampler runs for `IDLE_SAMPLE_SECONDS = 30.0`,
   writing `idle-iostat.log`. As soon as it stops, the pure `idle_load`
   over those readings goes out through `progress`, so an operator on a
   busy host sees it before the builds start; the summary repeats it.
6. The priming build: every invocation of `settings.scope`, writing
   `log_name(Priming(), suffix)`.
7. The timed runs. Each starts both samplers against one epoch, then runs
   every invocation of `settings.scope` in turn, reading
   `ready_from = clock.now() - epoch` just before each. It stops the
   samplers, writing `run-<n>-iostat.log`, `run-<n>-ps.log` and each
   invocation's log. Each yields a `CapturedRun`; nothing is analysed yet.
   `BuildResult`, `CapturedInvocation`, `CapturedRun` and
   `CapturedSession` are defined in `analysis.py`, which consumes them, so
   the session imports its output types from the pure module rather than
   the reverse.
   A run's wall-clock is `clock.now() - epoch` after its last invocation,
   and its exit status is the first non-zero invocation status, or 0.
8. The match at end. A `HistoryError` here is recorded as
   `matched_at_end = None`, so `matched` is `false`, rather than refused,
   and the summary warns.
9. `analyse_session(captured) -> MeasuredBuild`, a pure function in
   `analysis.py` over the captured runs and the provenance. Each invocation is analysed with its
   own graph, reported tasks and `ready_from`, and a run's invocations
   merge with `Timeline.merged` before the buckets are computed. A run is
   `contains_failure` when any task failed or was not run, or when mise's
   own exit status is non-zero, so an early abort that leaves every task
   silent still marks the run.

Through `ports.progress` the session also reports, in order: the run
directory; idle sampling; priming's start and its wall-clock at the end;
and each `timed run i/N`'s start, each invocation's start with its log
path so the operator can `tail -f` it, and the run's end, with the
wall-clock and the failed tasks' names. Every path the session prints
(progress, interrupt and summary) is absolute, so it resolves from any
directory; the record keeps paths relative to the root.

The run's end line comes from a cheap pure scan of that run's lines, not
the full analysis, which still waits for step 9: `tasks_mise_reported_failing`
reads the end-of-run status lines, and `fetch_failures` the
fetch-failure patterns. A fetch failure is named at once, with a hint
when `--keep` is set and timed runs remain, for example `timed run 1/3:
312.4 s, 1 failed (deny:check); fetch failure (advisory-database): with
--keep this record will not be kept; press Ctrl-C to stop, then rerun`. A
doomed baseline attempt then costs one run, not three. On the last timed
run the hint is left out, since stopping then would only lose the record
without saving time.

An interrupt or an error before step 9 finishes keeps the run directory,
prints its path, and writes no record. The run directory holds the logs,
not every input the analysis needs: the task graph, the provenance, the
settings, and each invocation's `ready_from` and exit status stay in
memory. Re-deriving a record from a run directory would need those
written too, and is left for later; the README says the logs alone
cannot be turned back into a record.

The ports:

```python
class BuildRunner(Protocol):
    def run(
        self, argv: Sequence[str], *, epoch: float, log: Path
    ) -> BuildResult: ...


class Sampler(Protocol):
    def start(self, *, epoch: float, log: Path) -> RunningSampler: ...


class ProcessSampler(Protocol):
    def start(
        self,
        *,
        epoch: float,
        log: Path,
        tracked: Sequence[TrackedProcess],
    ) -> RunningSampler: ...


class RunningSampler(Protocol):
    def stop(self) -> tuple[StampedLine, ...]: ...


class BuildGroup(Protocol):
    def remember(self, pgid: int, create_time: float) -> None: ...
    def forget(self) -> None: ...
    def orphaned(self) -> OrphanedBuild | None: ...


@dataclass(frozen=True, slots=True)
class OrphanedBuild:
    pgid: int
    members: tuple[tuple[int, str], ...]
    path: Path
```

`OrphanedBuild` is a pure value beside its port, so `build_group.py`
imports it from the session's module rather than the session importing
an adapter.

The session hands `settings.tracked` to the process sampler at each start,
so the set the sampler keeps lines for and the set the analysis attributes
CPU to are one value. With `--ide-pattern`, `BuildSettings.of` builds that
value with `with_ide_patterns`, and nothing in the wiring changes.

`BuildResult` holds the stamped lines, the exit status and the wall-clock.
Every adapter appends each stamped line to its `log` as it arrives, opened
line-buffered, and also returns the lines for analysis. Every stamp is
seconds since the run's `epoch`, a reading of `ports.clock.now()`, so the
build log and both samplers share one timeline.

**File**: `tasks/shared/build_measurement/record.py`

`RUN_DIRECTORY_ROOT` comes from `locations.py`, and `MEASUREMENTS_RELATIVE` from `tasks/shared/paths.py`.

```python
def run_id(
    started_at: dt.datetime,
    hostname: str,
    *,
    cache_state: BuildCacheState,
    host_settings: HostSettings,
) -> str: ...
def record_text(measured: MeasuredBuild) -> str: ...
def parse_record(text: str, *, source: Path) -> MeasuredBuild: ...
def write_measured_build(
    measured: MeasuredBuild, *, root: Path, keep: bool
) -> WrittenRecord: ...
```

`run_id` takes no `mode` until Phase 6 adds the `_isolated` suffix and
its test. `run_id` takes values from `measured_build.py`, not `BuildSettings`, so
`record.py` never imports `settings.py` or `session.py`.

**File**: `tasks/shared/build_measurement/reports.py`

```python
def preflight_report(preflight: Preflight) -> str: ...
def summary_report(
    outcome: SessionOutcome, *, record_path: Path, kept_path: Path | None
) -> str: ...
```

`Preflight` and `SessionOutcome` are defined here, beside the reports
that consume them, and `session.py` imports them:

```python
@dataclass(frozen=True, slots=True)
class SessionOutcome:
    measured: MeasuredBuild
    keep: bool
```

`keep` lets the summary tell a record withheld for a fetch failure from
one never asked to be kept.
 `summary_report` takes plain paths rather than
`record.py`'s `WrittenRecord`, so the pure reports never import the
module that writes files. Each report is composed from one small renderer
per section (header, commit, idle load, wall-clock, processes, failures,
fetch failures, paths), so no single function carries every
branch past ruff's C901 and PLR0912 limits.

- **`record_text`.** `measured.model_dump_json(indent=2)`.
- **`parse_record`.** `MeasuredBuild.model_validate_json(text)`, strict
  through `RecordModel`'s config. It turns a `ValidationError` into
  `UnreadableRecordError(MeasurementRefusedError)` naming `source` and the
  first failing field's location, so a `warm-dispatch-N.json`, a truncated
  file or a `schema_version: 2` record is refused with a sentence rather
  than a traceback.

**File**: `tasks/shared/build_measurement/measured_build.py`

```python
@unique
class HostSettings(StrEnum):
    UNSPECIFIED = "unspecified"
    NONE = "none"
    DEVTOOLS = "devtools"
    DEVTOOLS_AND_IDE_EXCLUSION = "devtools+ide-exclusion"


class MeasuredBuild(RecordModel):
    schema_version: Literal[1]
    run_id: str
    run_directory: Path
    started_at: dt.datetime
    conditions: MeasurementConditions
    commit: CommitMatch
    host: HostDescription
    mise: MiseDescription
    idle_load_percent: float | None
    task_set: tuple[str, ...]
    tracked_processes: Mapping[str, tuple[str, ...]]
    priming: PrimingRun | None
    runs: tuple[RunFigures, ...]
    summary: SessionSummary
    fetch_failures: tuple[FetchFailure, ...]
    failed_tasks: tuple[str, ...]
```

`schema_version` comes first and has no default: it is written first, and
a document without it is refused rather than read as version 1.
`analyse_session` passes `schema_version=1`.

- **Every stored derived field is checked on read.** The rule: each
  derived field is validated by the model that holds it, with the same
  function the analysis used to derive it. `CommitMatch` checks `matched`
  against its two checks. `RunFigures` checks `contains_failure` against
  its tasks' outcomes and exit status (through `failed_tasks`), and
  `unit_totals` against its tasks' units. `MeasuredBuild` checks only the
  cross-part copies: the four host fields in `conditions` against `host`
  and `mise`, the top-level `fetch_failures` against the runs' lists, and
  `failed_tasks` against the runs' outcomes. The copies exist for `jq`
  checks and the summary, and the validators keep them from ever giving
  two answers.
- **`started_at` keeps the `meta/` form.** The session truncates it to
  whole seconds at capture, and a `field_serializer` writes it
  with `isoformat()`, `2026-10-30T16:20:00+00:00`, rather than Pydantic's
  default `…Z`, matching the frontmatter dates in `meta/`.
- **`fetch_failures` name their run.** `FetchFailure.run` says which run
  each came from, so the top-level list loses nothing.

`RunFigures`, `SessionSummary`, `CommitMatch` and `PrimingRun` are
`RecordModel`s with the fields the record example shows.

```python
class MeasurementConditions(RecordModel):
    hostname: str | NoReading
    model: str | NoReading
    os_version: str | NoReading
    mise_jobs: int | NoReading
    cache_state: BuildCacheState
    mode: Mode
    isolated_tasks: tuple[str, ...]
    host_settings: HostSettings

    def differences(
        self, other: Self
    ) -> tuple[ConditionDifference, ...]: ...

    def comparable_with(self, other: Self) -> bool:
        return not self.differences(other)
```

`MeasurementConditions` is Requirement 7's comparability key as one value,
plus the mode and isolated tasks, so an isolated record never compares as
equivalent to a full one. `analyse_session` builds it from the host and
mise descriptions and the settings, so the four host fields it repeats
cannot disagree with them. `differences` names each field that differs,
with both values. A `NoReading` on either side is reported as differing,
never as equal, since two failed readings say nothing about the machine.
Comparability is asked only through `comparable_with`. Pydantic's
generated `==` compares field by field, so two conditions holding the same
`NoReading` are equal under `==` while not comparable; `==` stays as it is
because the round-trip tests rely on it.
`Mode` (in `measured_build.py`) and `BuildCacheState` (in a new pure
`cache_states.py`, with its `primes` property) land with this phase
holding one member each, `full` and `warm-no-op`, so the schema does not
change when Phases 6 and 7 add members.

`write_measured_build` writes only the record, since the logs are already
on disk:
- **`record.json`** is written with `tasks/shared/files.py`
  `atomic_write_text`, which writes a `.tmp` sibling, moves it into place
  with `Path.replace` and removes the temporary on failure.
- **The kept copy** is written to `kept.json.tmp` in the run directory,
  which is gitignored and on the same filesystem, then published with
  `kept_path.hardlink_to(temporary)`. A hard link makes the whole file
  appear in one step and raises `FileExistsError` when the name is taken,
  so it is both atomic and exclusive, and no temporary ever sits under
  `meta/measurements/` for jj to snapshot. The temporary is then removed.
  A taken name raises `KeptRecordExistsError(MeasurementRefusedError)`,
  naming the kept path and the run directory's `record.json`.

With `--keep`, a record holding any fetch failure is withheld from
`meta/measurements/`: Requirement 7 says such a record is retaken, not
kept, and under jj a kept file joins the working-copy commit at once.
`WrittenRecord.kept` is then `None`, and the summary says why.

The summary follows the measure convention: a pure function returns the
string and the task prints it. For example:

```text
measure:build build_2026-10-30T162000Z_engywook_warm-no-op_devtools (warm-no-op, devtools, 3 runs)
commit: d55cad6d85, working copy matched at start and end
idle load: 26.8%
wall-clock: median 309.4 s (min 305.1, max 322.0)
processes: syspolicyd 51.3 s, XprotectService absent, ide 301.0 s, fsnotifier 12.4 s
failed: deny:check (run 2, see <run directory>/run-2.log) — downstream flagged: 0 tasks
record: <root>/.accelerator-measure/build/build_2026-10-30T162000Z_engywook_warm-no-op_devtools/record.json
kept: <root>/meta/measurements/build_2026-10-30T162000Z_engywook_warm-no-op_devtools.json
```

Each path is printed absolute; `<root>` stands for the checkout here.

With a fetch failure, the last line becomes:

```text
not kept: 1 fetch failure (advisory-database in run 2); retake
```

#### 3. Adapters and wiring

**File**: `tasks/shared/build_measurement/build_runner.py`

- **`SubprocessBuildRunner(root, clock, procs: ProcessOps, build_group:
  BuildGroup)`.** It runs
  `subprocess.Popen(argv, cwd=root, stdout=PIPE, stderr=STDOUT,
  start_new_session=True)` with a binary pipe. It wraps `stdout` in
  `io.TextIOWrapper(encoding="utf-8", errors="replace", newline="\n")`,
  iterates it and stamps each line with `clock.now() - epoch`.
  - `newline="\n"` ends a line only at `\n`. Text mode on `Popen` would
    split at every lone `\r` too, so a cargo `Compiling` segment after a
    progress bar's `\r` would arrive without its `[task]` prefix.
    `build:cli:fixture-size` runs cargo with `pty=True` inside `default`,
    so this happens in real runs.
  - `errors="replace"` keeps one stray non-UTF-8 byte from ending the
    session.
  - Every log and golden is written and read with `newline="\n"` for the
    same reason.
  - The child runs in its own session, so a terminal Ctrl-C reaches only
    the harness. The harness then terminates the whole tree in a `finally`
    when the child is still alive.
  - The whole run, from `Popen` to the end of the reap, sits inside
    `unwind_then_defer(UNWIND_SIGNALS)`, and the `try` opens on the line
    after `Popen`. The first unwind signal raises and switches every
    handler to recording, so a second one, whichever thread receives it,
    cannot abort the `finally`. The `finally` runs `reap_session` (step 0)
    on the child's PID, inside `deferred_signals(UNWIND_SIGNALS)` when no
    signal has arrived yet, so the closing SIGKILL is always sent.
  - Inside the `try`, it remembers the child's PGID (its PID, since it leads
    its own session) with `BuildGroup.remember`, passing the create time
    `procs.create_time` reads straight after `Popen`. A child that has
    already exited has no create time and no group to orphan, so nothing
    is remembered. It calls `forget` inside the deferred region straight
    after the reap, so a re-raise cannot leave the file behind. If the
    harness dies without unwinding (SIGKILL, an OOM kill, a crash), the
    file is left naming the orphaned group for the next session to
    find.
  - After the reap, it asks `ProcessOps.group_alive` once more. If the
    group survived, the interrupt or error message names it and gives
    `kill -KILL -<pgid>`, so a build left running is never silent. It
    raises `BuildLeftRunningError(MeasurementRefusedError)`.
  - It reaps only a child that is still running: once `poll()` has
    collected the leader its PID may be reused, and `reap_tree` would
    signal whatever now holds it.
  - Its environment drops the inherited `MISE_TASK_*` and sets
    `MISE_TASK_TIMINGS=1`, so the measured build prints what a direct
    `mise run default` prints (see Key Discoveries).

**File**: `tasks/shared/build_measurement/samplers.py`

- **`StreamSampler(argv, clock)`.** It drives `iostat -w 2`: a `Popen` plus
  a daemon reader thread that appends stamped lines under a lock.
  `RunningSampler.stop()` terminates the process, joins the thread with a
  5-second timeout and returns the lines.
- **Stopping.** Every sampler thread is a daemon, every join has a
  timeout, and `stop()` is idempotent: a second call returns the same
  lines. The session registers each running sampler's `stop` on a
  `contextlib.ExitStack` the moment it starts, so an exception or an
  interrupt stops it, and calls `stop()` itself on the success path to
  collect the lines.
- **`PsProcessSampler(argv, clock, interval, tick_timeout)`.** It drives
  the injected `argv`, which the wiring sets to `PS_ARGV` and the tests to
  a `sys.executable` stand-in. `start` builds
  `tracked_line_filter(tracked)`, then a thread runs `subprocess.run(argv,
  capture_output=True, check=False, timeout=tick_timeout)` once each
  `interval` seconds and decodes the output with `errors="replace"`. Each
  tick it appends `sample_marker(at)` and the lines the filter keeps, or
  `gap_marker(at)` when the tick timed out or `ps` exited non-zero. It is
  stopped through a `threading.Event`. The wiring passes
  `tick_timeout=5.0`; the tests pass a short one.

**File**: `tasks/shared/build_measurement/wiring.py`

```python
def real_ports(root: Path) -> BuildPorts:
    clock = Clock()
    procs = PsutilProcessOps()
    build_group = BuildGroupFile(root, procs)
    return BuildPorts(
        system=platform.system,
        build=SubprocessBuildRunner(root, clock, procs, build_group),
        cpu_sampler=StreamSampler(("iostat", "-w", "2"), clock),
        process_sampler=PsProcessSampler(
            PS_ARGV, clock, interval=1.0, tick_timeout=5.0
        ),
        history=history_of(root),
        diagnostics=ambient_diagnostic_runner,
        kernel_hostname=socket.gethostname,
        mise_tasks=mise_tasks_json,
        clock=clock,
        utc_now=lambda: dt.datetime.now(tz=dt.UTC),
        progress=lambda line: print(line, flush=True),
        session_lock=WorkspaceSessionLock(root).held,
        build_group=build_group,
    )
```

`history_of` picks `JjHistory` when `repository_kind(root).is_jj`, and
`GitHistory` otherwise. It turns `NotARepositoryError` into a
`MeasurementRefusedError` subclass that names the directory.

#### 4. The task, registration and docs

**File**: `tasks/measure.py`

```python
@task(
    name="build",
    iterable=["ide_pattern"],
    help={
        "runs": "Timed runs (default 3).",
        "keep": (
            "Also write the record under meta/measurements/ for "
            "committing, unless it holds a fetch failure."
        ),
        "host_settings": (
            "Label for the host settings already in force; nothing is "
            "changed. none, devtools or devtools+ide-exclusion (default "
            "unspecified): Developer Tools in System Settings > Privacy "
            "& Security, and the IDE's excluded folders."
        ),
        "ide_pattern": (
            "Regex searched in each process's full command line, "
            "replacing the IntelliJ and RustRover defaults; anchor it on "
            "the main executable, e.g. '/Contents/MacOS/clion(?:\\s|$)'. "
            "Repeat for several."
        ),
    },
)
def build(
    context: Context,
    runs: int = 3,
    keep: bool = False,
    host_settings: str = str(HostSettings.UNSPECIFIED),
    ide_pattern: list[str] | None = None,
) -> None:
    """Time a full `default` build: a priming run, then N timed runs.

    Writes `.accelerator-measure/build/<run-id>/`; `--keep` also writes the
    record under `meta/measurements/` for the operator to commit, unless
    it holds a fetch failure. macOS only. Exits 0 whenever it writes a
    record, even if a measured task failed.
    """
    del context
    measure_build_with(
        lambda: BuildSettings.of(
            root=REPO_ROOT,
            runs=runs,
            keep=keep,
            host_settings=host_settings,
            ide_patterns=ide_pattern or (),
        ),
        real_ports,
    )
```

- **`measure_build_with(make_settings, make_ports)`.** Both arguments are
  factories, named as such. Inside one `try`, it builds the settings, then
  the ports with `make_ports(settings.root)`, so the session has one root
  for its logs, its build and its deletions, runs the session under `unwinding_signals()`, writes, and prints the summary. It
  maps `MeasurementRefusedError` to `Exit(str(error), code=1)`. Building
  the settings first means a flag refusal happens before any port exists.
  Building the ports inside the mapped region is what lets a
  non-repository exit with a message rather than a traceback.

**File**: `mise.toml`

Add after `test:integration:measure`, under a comment of its own, because
the existing exclusion comment ("a quiet host, a pinned PATH, network
egress … Owned by its own non-blocking CI job") does not describe this
task:

```toml
# Deliberately NOT in the aggregate `check` or the bare `default` task: an
# operator-run, macOS-only build session of 20 minutes or more that no CI
# job owns. See tasks/README.md#the-measure-namespace.
[tasks."measure:build"]
description = "Time a full default build: a priming run then N timed runs, per-task figures and host contention (macOS, ~20 min at --runs 3; writes .accelerator-measure/build/, --keep also writes meta/measurements/ unless a fetch failed)"
depends = ["deps:install:python"]
run = "invoke measure.build"
```

**File**: `tasks/README.md`

- **Section opening.** The first sentence says `measure:*` drives the
  warm-dispatch harness. Generalise it to name the build-timing harness
  too. Scope the next sentence's reasons for exclusion (the real
  bootstrap, network egress, a published signed release, instrument
  floors) to the warm-dispatch tasks, and give `measure:build` its own
  reason: an operator-run, macOS-only build session of 20 minutes or
  more that no CI job owns. Add `measure:build` to "Who runs what" as
  operator-run. Rename the "What a run requires" label to "What a
  warm-dispatch run requires", so none of it reads as describing
  `measure:build`.
- **Bold-lead paragraphs**, in the section's existing style, after
  "Launcher size and summary latency". Phases 6, 7 and 9 add their own
  rather than lengthening these:
  - **Build timing.** What the task runs, and that it needs no quiet host
    because idle load is recorded and printed instead. The run directory,
    and `--keep`, which writes the kept record unless it holds a fetch
    failure. `--ide-pattern`: a regex searched in each process's full
    command line that replaces the IDE defaults, best anchored on the main
    executable so the IDE's helpers are not counted as IDE time. That
    per-process CPU is a lower bound, since one-second sampling misses
    short-lived processes and the post-build tail is not counted.
  - **Run directories.** The `build_*` run directories under
    `.accelerator-measure/build/` are never pruned, hold the only raw logs
    behind a kept record, and are safe to delete by hand when no
    `measure:build` session is running. Leave `.lock` and `build.pgid`
    beside them: `build.pgid` is the only trace of a build orphaned by a
    killed session. The logs alone cannot be turned back into a record,
    since the graph, provenance and settings are not kept there.
  - **Reading a build record.** The record name,
    `build_<UTC time>_<hostname>_<cache state>_<label>`, with fields joined
    by `_` so each boundary is unambiguous, chosen over a counter because
    a counter collides across jj workspaces. The commit is the nearest
    first-parent commit changing anything outside `meta/measurements/`.
    Every per-run time is seconds from that run's start, and
    `process_cpu_seconds` is CPU-seconds. `conditions` is the
    comparability key, and the comparability rule follows. `presence`
    says whether a figure has a spread and, if not, why: its `standing` is
    `measured`, `flagged` with `caveats`, or `absent` with an `absence`,
    and only that variant's fields appear. A process `flagged` as
    `unreadable` had a failed `ps` tick in some run, so its CPU there
    could not be measured. A task's `outcome` is `succeeded`, `failed`,
    `skipped`, `not-run` (it has a `run` but printed nothing) or `silent`
    (a roll-up). A value that could not be read is `{"argv": [...]}`, the
    command that failed. Elsewhere `null` means no value was observed.
  - **The record's schema.** An additive field carries a default and
    keeps version 1. A breaking change bumps the version, and
    `parse_record` upgrades an older record by a function over its raw
    JSON, applied before validation, so committed records stay readable.
    A record is read by code at least as new as the code that wrote it.
- **Taking a baseline or condition record.** A list after the paragraphs.
  It is the one statement of these rules; Phases 5 and 10 point at it:
  - what each `--host-settings` label means: the Developer Tools list in
    System Settings › Privacy & Security, and the IDE's excluded folders
    (Phase 8 adds a pointer to the `CONTRIBUTING.md` steps);
  - keep the IDE open on the project;
  - make no commit in any workspace while a run is going, because the
    launcher's `build.rs` watches the parent `.git/HEAD`;
  - record `none` first, since undoing the exemption can need a reboot;
  - run nothing heavy in any other workspace of the repository, because
    the session lock covers only this one;
  - if a session is refused because an earlier session's build is still
    running, check the members the message lists; stop them with the kill
    commands it gives if they are that build, or delete `build.pgid` if
    none of them is;
  - retake on any fetch failure, or when `syspolicyd`, the IDE or
    `fsnotifier` is `absent`, or is flagged `absent-in-some-runs` or
    `unreadable`, in the record's `summary.processes`;
  - to re-measure the baseline after an OS or settings change: from the
    baseline commit (`jj new <commit>`, or `git checkout <commit>`), run
    without `--keep`; go back to the current checkout; then copy the run
    directory's `record.json` to `meta/measurements/<run id>.json` and
    commit it there;
  - the baseline commit's launcher release must stay published until 0361
    closes.

Keep the five strings that `TestMeasureNamespaceDocs` pins.

**File**: `.gitignore`

Extend the comment above `/.accelerator-measure/` to say it also holds
`measure:build` run directories. The pattern itself is unchanged.

**File**: `meta/work/0368-build-performance-measurement-and-host-guidance.md`

Amend the `--keep` contract:
- **Requirement 5.** `--keep` also writes the record under
  `meta/measurements/` unless the record holds a fetch failure. This
  applies to every `--keep` run, not only baseline and condition records:
  under jj a kept file joins the working-copy commit at once, and a
  fetch-failed record is never one to commit.
- **The "Given a macOS host and a warm workspace … with `--keep`"
  criterion** gains "and no fetch failure".
- **A new criterion:** "Given `--keep` and a fetch failure, when it runs,
  then nothing is written under `meta/measurements/`, the run directory
  still holds `record.json`, and the summary says the record was not
  kept."

#### 5. Capture the goldens

After the tests above pass against the stubs, run on the reference host:

```bash
mise run measure:build -- --runs 1
```

From the run directory, store these in
`tests/unit/tasks/fixtures/build-measurement/`:
- `run-1.log` as `warm-no-op-build.golden.gz`, compressed with `gzip -9`;
- a 60-second slice of `run-1-iostat.log` as `warm-no-op-iostat.golden`;
- the first 60 ticks of `run-1-ps.log` as `warm-no-op-ps.golden`;
- `mise tasks --json`, taken at the same commit, as
  `warm-no-op-tasks.golden.json`, so the closure the tests compute does not
  drift as the epic reshapes `default`.

Derive each known answer with a command written into the test module beside
its constant, for example:

```bash
zgrep -E '^\S+ \[test:unit:cli\] Finished in' warm-no-op-build.golden.gz
zgrep -oE '\[test:unit:cli\] .*' warm-no-op-build.golden.gz \
  | grep -oE '(^|\s)Compiling \S+ v' | wc -l
```

The goldens are never hand-edited. If the format changes, re-capture them
by the steps in the test module's docstring.

### Success Criteria:

#### Automated Verification:

- [x] The new tests pass: `uv run pytest tests/unit/tasks/shared/build_measurement tests/unit/tasks/shared/test_signals.py tests/unit/tasks/shared/test_processes.py -v`
- [x] The measure and mise guards pass: `uv run pytest tests/unit/tasks/test_measure.py tests/unit/tasks/test_mise.py -v`
- [x] The extracted reaper leaves the dev server green: `mise run test:unit:tasks`
- [x] Neither `check` nor `default` reaches `measure:build`: `test_no_measure_reaching_task_is_in_the_ci_mirror`
- [x] Lint and types pass: `mise run build-system:check`
- [x] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [x] On the reference host, `mise run measure:build -- --runs 1 --keep`
      writes the record to both places with the following:
  - [x] per-task figures;
  - [x] buckets;
  - [x] wall-clock;
  - [x] the four tracked processes' CPU-seconds;
  - [x] every metadata field.
- [x] The run directory holds every log. `jj status` shows no change under
      `.accelerator-measure/`, and shows only the kept record under
      `meta/measurements/`.
- [x] The preflight block appears before idle sampling, progress lines
      appear while the run is going, and `tail -f` on the printed log path
      follows the build.
- [x] Ctrl-C during a timed run leaves no `mise`, `cargo`, `nextest` or
      `iostat` process behind (`pgrep -fl 'mise run|iostat'`), writes no
      record, and leaves the logs captured so far in the run directory,
      whose path it prints.
- [x] Pressing Ctrl-C twice in quick succession leaves the same clean
      state.

---

## Phase 5: Warm no-op baseline and sibling updates

### Overview

This phase is operator work. It records Requirement 7's baseline before the
first of 0362–0367 merges, then discharges the gate in the sibling work
items.

### Changes Required:

#### 1. Record the baseline

On the reference host (`engywook`, Apple M4 Max 12P + 4E, macOS 26.3, on AC
power), following the README's "Taking a baseline or condition record":
1. Start from `main` after Phase 4 has merged, with none of 0362–0367
   merged: `jj new main`.
2. Choose the label for the settings in force, as the README list says.
3. Run `mise run measure:build -- --runs 3 --keep --host-settings <label>`.
4. Retake under any of the README list's retake conditions. Beyond those,
   the baseline needs `commit.matched` true, a label other than
   `unspecified`, and `conditions.mise_jobs` of `8`.
5. Commit only `meta/measurements/<run-id>.json`, with a message naming the
   baseline.
6. In the same commit, name the baseline's path in
   `test_the_baseline_is_among_the_committed_records` and remove its
   skip, so the committed-record check can never quietly match nothing.

#### 2. Update the work items

Edit frontmatter with `/accelerator:update-work-item`; edit body text by
hand.

| Item | Frontmatter | Body |
|---|---|---|
| 0368 | Remove 0362, 0363, 0364 and 0367 from `blocks`; keep 0365 and 0366 | Dependencies: cite the baseline record path |
| 0362, 0363, 0364, 0367 | Remove `work-item:0368` from `blocked_by` | Dependencies: replace the "remove the edge once the baseline is recorded" text with the baseline path |
| 0365 | Keep the edge | Dependencies: baseline recorded at `<path>`; the edge now waits for isolation mode only |
| 0366 | Keep the edge | Dependencies: baseline recorded at `<path>`; the edge now waits for the cold state only |
| 0361 | — | Technical Notes: name the baseline record its 200 s criterion compares against, and say the closing comparison runs `mise run measure:compare-builds -- <baseline> <final>`, whose first line must report no differing condition |

### Success Criteria:

#### Automated Verification:

- [ ] Every edited work item's frontmatter validates: `accelerator corpus frontmatter validate --file meta/work/<item>.md`
- [ ] `test_every_committed_build_record_is_readable` now runs over the baseline, and `test_the_baseline_is_among_the_committed_records` passes: `uv run pytest tests/unit/tasks/shared/build_measurement/test_record.py -v`
- [ ] Lint and types pass: `mise run build-system:check`
- [ ] The kept record parses and carries the baseline conditions: `jq -e '.fetch_failures == [] and .commit.matched and .conditions.host_settings != "unspecified" and .conditions.mise_jobs == 8 and (.runs | length) == 3 and ([.summary.processes | to_entries[] | select(.key != "XprotectService") | .value.presence.standing] | all(. == "measured"))' meta/measurements/<run-id>.json`

#### Manual Verification:

- [ ] The baseline commit contains none of 0362–0367's changes.
- [ ] The record's host fields name the reference host.

---

## Phase 6: Isolation mode

### Overview

This phase covers Requirement 3. `--isolate <task>` runs each named task
alone with `--skip-deps`, one after another, under the same priming and
repeat rules. The record gives each task's figures, plus its start and end
within each run.

### Changes Required:

#### 1. Tests first

**File**: `tests/unit/tasks/shared/build_measurement/test_session.py`

- `test_isolation_runs_each_named_task_alone_with_skip_deps`: argv
  `("mise", "run", "--output", "prefix", "--skip-deps", task)`, once per
  task per run, in the given order.
- `test_isolated_intervals_do_not_overlap`.
- `test_isolation_primes_once_over_all_named_tasks`.
- `test_an_isolated_task_is_ready_at_its_own_invocations_start`: queueing
  is measured from its invocation's start, not the run's epoch.
- `test_isolation_reports_duration_nextest_phase_and_status_with_spreads`.
- `test_an_isolated_runs_wall_clock_spans_every_invocation`.
- `test_an_isolated_runs_exit_status_is_its_first_failure`.
- `test_a_failed_isolated_task_takes_its_invocations_exit_status`: the log
  ends `[t] ERROR task failed` with no `exited with status` line, and the
  invocation exits 3 → the task's `exit_status == 3`.
- `test_the_buckets_list_each_isolated_task_while_it_runs`.
- `test_the_record_marks_its_mode_and_isolated_tasks`.
- `test_settings_refuse_a_task_isolated_twice`: `InvalidSettingsError`
  naming the repeated task.
- `test_an_unknown_isolated_task_is_refused_before_the_lock`:
  `UnknownTaskError` names the unknown task and its closest match, and
  only the `system` and `mise_tasks` ports are touched.
- Extend `test_the_session_opens_in_one_order` with the name check,
  between the platform check and the session lock.
- `test_isolated_priming_writes_one_log_per_task`:
  `priming-<task>.log`, from the same `invocations` call as a timed run.
- `test_progress_names_each_isolated_invocations_log`: the summary's
  failed-task pointer names the failing invocation's own log.

**File**: `tests/unit/tasks/shared/build_measurement/test_record.py`

- Extend `test_a_record_reads_back_equal_to_what_was_written` with an
  isolated record.

**File**: `tests/unit/tasks/test_measure.py`

- Extend `test_build_is_documented_without_a_quiet_host` to require
  `--isolate` in the measure section.
- Extend `test_repeated_flags_are_parsed_as_lists` to `--isolate a
  --isolate b`.

#### 2. Implementation

- **`settings.py`.** `BuildScope` gains `IsolatedTasks(tasks)`, with
  `mode = Mode.ISOLATED` and `isolated_tasks = tasks`, and `Mode` gains
  `ISOLATED`. Its `invocations(graph)` yields one `Invocation` per
  named task, in the given order:
  - argv `("mise", "run", "--output", "prefix", "--skip-deps", task)`;
  - log suffix `-<task with ':' → '_'>`, so priming writes
    `priming-test_unit_cli.log` and timed run 2 writes
    `run-2-test_unit_cli.log`;
  - `TaskGraph.isolated(task)`, a graph in which the task has no
    dependencies, so with `ready_from` its readiness is its invocation's
    start;
  - `(task,)` as the tasks to report.

  The session's loop is unchanged from Phase 4; only the scope differs.
- **A failed isolated task's exit status.** A single-task `mise run
  --skip-deps <task>` prints `ERROR task failed` but no `exited with
  status` line (probed with mise 2026.10.8 during Phase 1), so the
  timeline leaves a failed isolated task's `exit_status` as `None`. The
  analysis fills it from that invocation's exit status, the only place
  mise reports it.
- **`analysis.py`.** `conditions.mode` and `conditions.isolated_tasks`
  come from the scope's members, and the schema is unchanged.
- **Name validation.** `BuildSettings.of` gains `isolate`, and builds
  `IsolatedTasks` from it when it is not empty. Straight after
  `require_macos`, `measure_build` checks each name against the
  `TaskGraph` it loads through `ports.mise_tasks`, and refuses with
  `UnknownTaskError(MeasurementRefusedError)`. The message names each
  unknown task and its closest match from `difflib.get_close_matches`.
  `BuildSettings.of` also refuses a task named twice with
  `InvalidSettingsError`, since two invocations of one task would share a
  log and break `Timeline.merged`'s disjoint keys.
- **`tasks/measure.py`.** Add `iterable=["ide_pattern", "isolate"]`, an
  `isolate` parameter and its `help` entry ("Run this task alone with
  --skip-deps; repeat for several."), and pass it to `BuildSettings.of`.

#### 3. Docs

- **`tasks/README.md`.** Add an **Isolation mode** bold-lead paragraph:
  `--isolate <task>` runs each named task alone with `--skip-deps`, one
  after another, and reports each one's start and end; it does not run
  `default`. Each isolated task has its own log, with `:` in the task
  name written as `_`, so `test:unit:cli` logs to
  `run-2-test_unit_cli.log`. Add the `_isolated` suffix to the record
  name in "Reading a build record".
- **Docstring, `mise.toml` description and comment.** Reword them so they
  are true for isolation mode as well as a full run; the comment's "build
  session of 20 minutes or more" already holds.

#### 4. Work items, after merging

| Item | Frontmatter | Body |
|---|---|---|
| 0365 | Remove `work-item:0368` from `blocked_by` | Dependencies: replace "the edge now waits for isolation mode only" with the merged isolation mode and the baseline path |
| 0368 | Remove 0365 from `blocks` | Dependencies: say 0365's wait on Requirement 3 is discharged |

### Success Criteria:

#### Automated Verification:

- [ ] The tests pass: `uv run pytest tests/unit/tasks/shared/build_measurement tests/unit/tasks/test_measure.py -v`
- [ ] Lint and types pass: `mise run build-system:check`
- [ ] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [ ] `mise run measure:build -- --isolate test:unit:cli --runs 1` reports a
      nextest phase close to the one in the research document's isolated
      phase table (149 s), adjusted for whatever has landed since.

---

## Phase 7: Incremental and cold cache states

### Overview

This phase covers Requirement 4:
- **`--cache-state incremental`** touches `cli/kernel/src/lib.rs` before each timed run
  and leaves its contents unchanged.
- **`--cache-state cold`** skips the priming run and deletes `cli/target` and
  `cli/visualiser/frontend/node_modules` before each timed run.

`--cache-state cold` together with `--isolate` is refused before anything is deleted.

### Changes Required:

#### 1. Tests first

**File**: `tests/unit/tasks/shared/build_measurement/test_cache_states.py`

- `test_each_state_names_what_it_touches_and_deletes`: parametrised over
  the three states. Warm touches and deletes nothing and primes;
  incremental touches `cli/kernel/src/lib.rs` and primes; cold deletes
  `cli/target` and `cli/visualiser/frontend/node_modules` and never
  primes.

**File**: `tests/unit/tasks/shared/build_measurement/test_workspace.py`

These run on a real `tmp_path` workspace, observed by
`ScriptedBuildRunner`:
- `test_incremental_touches_the_kernel_before_each_timed_run`: after each
  observation the stub resets the file's mtime to `(0, 0)`, so each timed
  run sees an mtime later than that and the priming run does not. The
  bytes are unchanged. Comparing against a fixed old mtime, rather than the
  previous call's, keeps the test independent of the filesystem clock's
  granularity on the Linux CI leg.
- `test_incremental_primes_first`.
- `test_incremental_refuses_a_missing_touch_target`: parametrised over a
  missing `cli/kernel/src/lib.rs` and a symlink in its place. `verify`
  raises `MissingTouchTargetError`, and no file is created.
- `test_cold_deletes_the_target_and_frontend_node_modules_before_each_run`.
- `test_cold_never_primes`.
- `test_cold_leaves_docs_site_node_modules_alone`.
- `test_without_cold_a_sentinel_in_cli_target_survives`.
- `test_a_symlinked_target_is_refused_before_anything_is_deleted`: the
  symlink is on `cli/visualiser/frontend/node_modules`, the second target,
  and a sentinel in `cli/target` survives, so the check cannot be
  interleaved with the deletions.
- `test_a_target_reached_through_a_symlinked_ancestor_is_refused`:
  `cli/visualiser` links out of the root, so only the containment check
  refuses it.
- `test_a_missing_directory_is_not_an_error`.
- `test_each_deletion_rechecks_containment`: the scripted build replaces
  `cli/target` with a symlink during run 1, and run 2's preparation
  refuses before deleting anything.
- `test_a_cold_target_is_moved_aside_before_it_is_removed`: with a
  `remove_tree` that removes nothing and returns a `PermissionError`,
  `cli/target` is absent from the workspace and whole under the trash
  directory. `prepare_timed_run` returns an `UnremovedTrash` with that
  path, the tree's size in bytes and the error's message.
- `test_the_trash_is_removed_when_removal_succeeds`: and
  `prepare_timed_run` returns `None`.
- `test_delete_from_disk_returns_the_first_failure`: over a tree holding a
  read-only directory, it returns the `PermissionError` rather than
  raising or discarding it. It is skipped when run as root, whom no
  permission stops.
- `test_a_trash_directory_outside_the_run_root_is_refused`: the trash
  must sit under `root / RUN_DIRECTORY_ROOT`, and is created with
  `exist_ok=False`.
- `test_leftover_trash_is_found_under_every_run_directory`:
  `leftover_trash()` gives a `FoundTrash` for each `cold-trash-*` under
  `root / RUN_DIRECTORY_ROOT`, from this session and earlier ones, with
  its size; an entry that vanishes during the size walk is skipped, not
  raised.
- `test_the_dev_launcher_marker_is_read_from_the_real_file`:
  `has_dev_launcher_marker()` follows `tmp_path/.accelerator-dev-launcher`.
- `test_verifying_a_warm_state_checks_nothing`.
- `test_a_refused_deletion_names_the_path_and_why`: "refusing to delete
  cli/target: it is a symlink to <target>; nothing was deleted", and for
  a target outside the root, "it resolves outside <root>".
- `test_the_record_states_the_cache_state`.

These build ports with a real `FilesystemWorkspace(tmp_path)` and a
`ScriptedBuildRunner` that observes the workspace, so the acceptance
criteria about sentinels and deletions are seen on real directories that
can never be the checkout's.

**File**: `tests/unit/tasks/shared/build_measurement/test_session.py`

These use a `RecordingWorkspace` double:
- `test_settings_refuse_an_unknown_cache_state_listing_the_known_ones`.
- `test_settings_refuse_isolate_with_cold`: `BuildSettings.of` raises
  `IncompatibleSettingsError` naming both flags.
- Extend `test_the_session_opens_in_one_order` with `workspace.verify`,
  after the `--isolate` name check and before the session lock, whatever
  the cache state.
- `test_preparation_precedes_each_timed_run_but_not_priming`.
- `test_preparation_finishes_before_the_runs_epoch_and_samplers`: the
  recording double sees `prepare_timed_run` before that run's clock
  reading and its first sampler start, so deletion time never lands in a
  run's wall-clock or buckets.
- `test_each_timed_run_has_its_own_trash_directory`: run `n` is prepared
  with `<run directory>/cold-trash-<n>`.
- `test_unremoved_trash_stops_the_session_before_the_next_timed_run`: the
  double returns an `UnremovedTrash` from preparing run 1 of 3. Run 1
  still runs; then `TrashNotRemovedError` names its cause, run 2 is
  neither prepared nor started, no record is written, and `progress` has
  already named the trash's absolute path and size.
- `test_unremoved_trash_from_the_last_run_becomes_a_notice`: the record
  is written, and the outcome's notices hold the `UnremovedTrash`.
- `test_an_interrupt_after_a_deletion_reports_leftover_trash`: run 2's
  build raises `KeyboardInterrupt`, and `progress` names each
  `leftover_trash()` entry by absolute path and size.
- `test_progress_says_priming_was_skipped_and_what_is_being_deleted`.
- `test_a_cold_preflight_lists_what_will_be_deleted`: both targets, as
  absolute paths, before idle sampling starts.
- `test_every_preflight_lists_trash_left_by_earlier_sessions`:
  parametrised over the three states.
- `test_a_cold_preflight_warns_when_the_dev_launcher_marker_exists`: and
  stays silent for a warm or incremental session, or without the marker.
- `test_the_dev_launcher_warning_is_repeated_however_the_session_ends`:
  parametrised over success, where it is a notice, an interrupt, a
  refusal after the first preparation, and a preparation that raises
  part-way (`UnsafeDeletionError` on run 1, with `leftover_trash()`
  returning an entry), each with the rebuild command and, where there is
  one, the trash entry. A session that ends before its first preparation
  does not repeat it.
- `test_a_failing_exit_report_never_hides_why_the_session_ended`:
  `leftover_trash()` raises during an interrupt. The `KeyboardInterrupt`
  still propagates, `progress` says the trash could not be listed, and
  the dev-launcher warning is still given.
- `test_a_quiet_session_has_no_notices`.

**File**: `tests/unit/tasks/shared/build_measurement/test_reports.py`

- `test_the_summary_renders_each_notice`: parametrised over
  `UnremovedTrash`, `FoundTrash` and `DevLauncherNeedsRebuild`, each with
  its exact line.
- `test_the_preflight_names_what_the_state_touches_and_deletes`:
  parametrised over the three states, rendered from `Preflight` data
  alone.

**File**: `tests/unit/tasks/test_measure.py`

- `test_isolate_with_cold_exits_one_and_nothing_is_deleted`: `REPO_ROOT`
  is monkeypatched to a `tmp_path` holding real `cli/target` and
  `cli/visualiser/frontend/node_modules`, and the untouchable `real_ports`
  fixture stays in place. Both directories survive, and `real_ports` is
  never called.
- Extend `test_build_is_documented_without_a_quiet_host` to require
  `--cache-state incremental` and `--cache-state cold` in the measure section.

**File**: `tests/unit/tasks/shared/build_measurement/test_record.py`

- Extend `test_a_record_reads_back_equal_to_what_was_written` with a cold
  record whose `priming` is `None`.

#### 2. Implementation

**File**: `tasks/shared/build_measurement/cache_states.py`

`BuildCacheState` gains its other two members, and each state states what
it primes, touches and deletes as data:

```python
@unique
class BuildCacheState(StrEnum):
    WARM_NO_OP = "warm-no-op"
    INCREMENTAL = "incremental"
    COLD = "cold"

    @property
    def primes(self) -> bool:
        return self is not BuildCacheState.COLD

    @property
    def touches(self) -> tuple[Path, ...]:
        if self is BuildCacheState.INCREMENTAL:
            return (INCREMENTAL_TOUCH,)
        return ()

    @property
    def deletions(self) -> tuple[Path, ...]:
        if self is BuildCacheState.COLD:
            return COLD_DELETIONS
        return ()


INCREMENTAL_TOUCH = (CLI_DIR / "kernel" / "src" / "lib.rs").relative_to(
    REPO_ROOT
)
COLD_DELETIONS = (
    CLI_TARGET_DIR.relative_to(REPO_ROOT),
    (FRONTEND / "node_modules").relative_to(REPO_ROOT),
)
```

The paths derive from `tasks/shared/paths.py` and stay relative, so the
tests can root them at `tmp_path`. The enum is `BuildCacheState` because
`tasks/measure.py` already has `CacheMode.COLD`, meaning a cold launcher
cache, and will import both. `cache_states.py` stays pure, so a module
that reads or compares a record never imports the code that deletes.
Every report renders a state's effects from `touches` and `deletions`, so
none of them branches on `COLD`.

**File**: `tasks/shared/build_measurement/cold_trash.py`

```python
TRASH_PREFIX = "cold-trash-"


@dataclass(frozen=True, slots=True)
class UnremovedTrash:
    path: Path
    size_bytes: int
    cause: str


@dataclass(frozen=True, slots=True)
class FoundTrash:
    path: Path
    size_bytes: int
```

The two facts come from different places: preparation knows why its own
removal failed, while a scan of the run directories finds trash without
knowing how it got there. Keeping them apart, rather than one type with
an optional cause, keeps the refusal from handling a cause that cannot be
missing. They sit in their own module so `cache_states.py` stays the
persisted state and its declared effects.

**File**: `tasks/shared/build_measurement/settings.py`

`trash_name(run: TimedRun) -> str` gives `cold-trash-<n>` from
`TRASH_PREFIX`, beside `log_name`, so every per-run path in the run
directory is named in one module.

**File**: `tasks/shared/build_measurement/session.py`

The `Workspace` port joins the session's other ports:

```python
class Workspace(Protocol):
    def has_dev_launcher_marker(self) -> bool: ...
    def verify(self, state: BuildCacheState) -> None: ...
    def prepare_timed_run(
        self, state: BuildCacheState, *, trash: Path
    ) -> UnremovedTrash | None: ...
    def leftover_trash(self) -> tuple[FoundTrash, ...]: ...
```

**File**: `tasks/shared/build_measurement/reports.py`

```python
@dataclass(frozen=True, slots=True)
class DevLauncherNeedsRebuild:
    pass


type SessionNotice = UnremovedTrash | FoundTrash | DevLauncherNeedsRebuild


def notices_report(notices: Sequence[SessionNotice]) -> str: ...
```

`SessionNotice` holds what the operator must be told that the record does
not hold, so a later notice adds a variant rather than a record field.
`notices_report` is the one place their words are written: the summary,
the preflight and the session's exit report all render through it, so
the rebuild command and the trash wording cannot drift between a
successful session and an interrupted one. `SessionOutcome` gains
`notices: tuple[SessionNotice, ...]`. `Preflight` gains the state's
touches and deletions as absolute paths, the leftover trash found, and
whether the dev-launcher marker exists.

**File**: `tasks/shared/build_measurement/workspace.py`

```python
def delete_from_disk(path: Path) -> BaseException | None:
    failures: list[BaseException] = []

    def collect(_call: object, _path: str, error: BaseException) -> None:
        failures.append(error)

    shutil.rmtree(path, onexc=collect)
    return failures[0] if failures else None


@dataclass(frozen=True, slots=True)
class FilesystemWorkspace:
    root: Path
    remove_tree: Callable[[Path], BaseException | None] = delete_from_disk

    def has_dev_launcher_marker(self) -> bool: ...
    def verify(self, state: BuildCacheState) -> None: ...
    def prepare_timed_run(
        self, state: BuildCacheState, *, trash: Path
    ) -> UnremovedTrash | None: ...
    def leftover_trash(self) -> tuple[FoundTrash, ...]: ...
```

`remove_tree` defaults to a named function, as `Clock.sleep` does, rather
than a `partial`, which ruff's RUF009 flags.

- **The port.** `BuildPorts` gains `workspace: Workspace`. Only `wiring.py`
  imports `workspace.py`, and `real_ports` builds
  `FilesystemWorkspace(root)`; it is the only code that can delete
  anything, and no session test builds one over the checkout.
- **Verification.** `verify(state)` checks each of the state's effects.
  For each of `state.deletions`, it confirms that `(root / path).resolve()`
  is inside `root.resolve()`, and refuses with
  `UnsafeDeletionError(MeasurementRefusedError)` when the path is a
  symlink or resolves outside the root; the message names the path and
  why, and says nothing was deleted. For each of `state.touches`, it
  refuses with `MissingTouchTargetError(MeasurementRefusedError)` unless
  the path is a regular file and not a symlink: "cli/kernel/src/lib.rs is
  missing or not a regular file; --cache-state incremental touches it
  before each timed run". A warm state has no effects to check.
- **Preparation.** Before each timed run, `prepare_timed_run` touches each
  of `state.touches` with `os.utime`, which, unlike `Path.touch()`, never
  creates a missing file. It repeats the containment and symlink check for
  each of `state.deletions`, since the builds recreate both directories
  between runs. It creates `trash` with `exist_ok=False`, after checking
  it is under `root / RUN_DIRECTORY_ROOT`, moves each target into it with
  `Path.rename` on the same filesystem, and removes it with
  `remove_tree`. A rename is atomic, so the live workspace only ever holds
  a target that is whole or absent; an interrupted or failed removal
  leaves debris only in the gitignored run directory.
- **Leftover trash.** When the trash is still there after `remove_tree`,
  `prepare_timed_run` returns an `UnremovedTrash` with its absolute path,
  its size summed over `lstat`, and the first failure's message, or
  "the directory remained after removal" when `remove_tree` reported
  none. `leftover_trash()` gives a `FoundTrash` for every `cold-trash-*`
  under `root / RUN_DIRECTORY_ROOT`, with its size; the size walk skips
  an entry that vanishes or cannot be read rather than raising.
- **Session.** `measure_build` calls `ports.workspace.verify(state)` in
  the opening order Phase 4 states: after the `--isolate` name check and
  before the session lock. Before each timed run it calls
  `ports.workspace.prepare_timed_run(state, trash=run_directory /
  trash_name(run))`, and that call finishes before the run's epoch is
  read and its samplers start, so deletion time never lands in a run's
  figures. It skips priming when `not state.primes`. Progress reports
  `priming skipped (cold)` and, before each run that deletes, the
  directories being deleted.
- **Unremoved trash.** An `UnremovedTrash` from preparing run n is held,
  and run n goes ahead, since its targets are already out of the
  workspace. When timed runs remain, the session then reports the trash
  through `progress` and, instead of preparing run n+1, refuses with
  `TrashNotRemovedError(MeasurementRefusedError)`: "stopped before timed
  run 2: the cold trash from timed run 1 could not be removed (<cause>),
  and each further cold run would leave another copy of cli/target;
  delete the trash listed above once the cause is fixed". When n is the
  last run, the trash becomes a notice and the record is written as
  usual. The run directory and its logs are kept either way.
- **Exit report.** Just before the first preparation of a state that
  deletes, the session pushes an exit callback onto its `ExitStack`. When
  the session ends by an exception, whether an interrupt, a refusal or an
  error, the callback builds a `FoundTrash` notice for each
  `leftover_trash()` entry and a `DevLauncherNeedsRebuild` when the
  marker exists, and reports them through `notices_report` and
  `progress`. On success the same facts reach the summary as notices
  instead. The callback is pushed before the first deletion rather than
  after it, so a failure part-way through a preparation still warns.
  Each fact is gathered on its own and the callback never raises: if
  listing the trash fails, it says so and still gives the dev-launcher
  warning, so the exception that ended the session is the one that
  propagates.
- **Preflight.** For a state that deletes, the preflight block lists the
  absolute paths it will delete before every timed run, so the operator
  sees them before idle sampling starts and can Ctrl-C. Every preflight,
  whatever the state, lists trash that earlier sessions left, with its
  size and "safe to delete", since a warm session's operator would
  otherwise never learn of a multi-gigabyte leftover. No separate `--preflight-only` mode is added: the
  check already comes before any build, and stopping there costs seconds.
- **Dev-launcher warning.** For a state that deletes, the preflight block
  warns when `<root>/.accelerator-dev-launcher` exists. Hooks that use
  this checkout's dev launcher pass `--fail-safe`, so while `cli/target`
  is deleted, `bin/accelerator` refuses the missing binary and exits 0.
  The `vcs guard` hook then fails open until the launcher is rebuilt with
  the `cargo build` in `CONTRIBUTING.md`'s "Local development". The exit
  report and the `DevLauncherNeedsRebuild` notice repeat the warning with
  that command, since the preflight scrolls away about 25 minutes before
  a cold session ends.
- **Settings.** `BuildSettings.of` gains `cache_state: str`, parsed into
  `BuildCacheState`. It refuses an unknown value with
  `InvalidSettingsError` listing the three, and `cold` with `isolate`
  with `IncompatibleSettingsError` ("--isolate does not support
  --cache-state cold"). Both refusals happen while the settings are built,
  before any port exists.
- **Task flag.** `tasks/measure.py` gains one `cache_state` flag, default
  `warm-no-op`, mirroring the record field and `--host-settings`. Its
  `help` lists the three states and says `cold` deletes `cli/target` and
  `cli/visualiser/frontend/node_modules` before every timed run, so
  choosing it is the explicit opt-in Requirement 4 asks for.

#### 3. Docs

- **`tasks/README.md`.** Add a **Cache states** bold-lead paragraph,
  after Isolation mode, covering:
  - `--cache-state incremental`, which touches `cli/kernel/src/lib.rs`
    before each timed run and refuses to start when that file is missing;
  - `--cache-state cold`, which skips priming, deletes `cli/target` and
    `cli/visualiser/frontend/node_modules` before each timed run, takes
    about 25 minutes at `--runs 3`, fetches npm packages every run, and is
    refused with `--isolate`;
  - the run directory's `cold-trash-<n>/`, which every deleted directory
    passes through. When one cannot be removed, the harness names it with
    its size and the cause; if timed runs remain it stops before the next
    one, and otherwise the summary reports it. Every preflight lists trash
    earlier sessions left, by path and size; all of it is safe to
    delete;
  - the dev launcher: with its marker present, hooks using it fail open
    while `cli/target` is gone, until it is rebuilt. The warning comes in
    the preflight and again however the session ends.
- **Docstring, `mise.toml` description and `--runs` help.** Reword all
  three so they are true for the cold state, which skips priming and
  takes about 25 minutes at `--runs 3`.

#### 4. Work items, after merging

| Item | Frontmatter | Body |
|---|---|---|
| 0366 | Remove `work-item:0368` from `blocked_by` | Dependencies: replace "the edge now waits for the cold state only" with the merged cold state and the baseline path |
| 0368 | Remove 0366 from `blocks` | Dependencies: say 0366's wait on Requirement 4 is discharged |

### Success Criteria:

#### Automated Verification:

- [ ] The tests pass: `uv run pytest tests/unit/tasks/shared/build_measurement tests/unit/tasks/test_measure.py -v`
- [ ] Lint and types pass: `mise run build-system:check`
- [ ] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [ ] `mise run measure:build -- --cache-state cold --runs 1` deletes and rebuilds both
      directories. `deps:install:node` succeeds in the run. The record says
      `cold` and has no priming entry.
- [ ] `mise run measure:build -- --cache-state incremental --runs 1` leaves
      `jj diff cli/kernel/src/lib.rs` empty.

---

## Phase 8: Host guidance

### Overview

This phase covers Requirement 8: the host-settings steps and their
tradeoffs in `CONTRIBUTING.md`. It depends only on Phase 4, whose README
list it links to the new steps.

### Changes Required:

#### 1. Tests first

**File**: `tests/unit/tasks/test_contributing.py`

`TestHostSettingsGuidance` slices `CONTRIBUTING.md` at
`## Faster local builds on macOS`, as `TestMeasureNamespaceDocs` does. It
asserts the section:
- names Developer Tools and the restart;
- names tmux and zellij;
- names `cli/target` and `node_modules` as excluded folders;
- states that the exemption stops XProtect scanning what the app launches;
- states that an IDE exclusion stops indexing but not file watching.

**File**: `tests/unit/tasks/test_measure.py`

- Extend `test_build_is_documented_without_a_quiet_host` to require a link
  to `CONTRIBUTING.md#faster-local-builds-on-macos` in the measure
  section.

#### 2. Implementation

**File**: `CONTRIBUTING.md`

Add `## Faster local builds on macOS` after "Local development". It
contains:

1. **Developer Tools exemption.** In System Settings › Privacy & Security ›
   Developer Tools, add the terminal app, the JetBrains IDE, and tmux or
   zellij if used. Restart each. The exemption follows the responsible
   app, so it covers everything the listed app launches. It should help
   because XProtect scans each newly linked or copied binary on its first
   launch, serially, at about 100–300 ms each, and a coverage run relinks
   369 test binaries.
2. **Security tradeoff.** XProtect no longer scans anything an exempted app
   launches. That includes binaries the build downloads or links.
3. **IDE exclusions.** Mark `cli/target`, `cli/visualiser/frontend/node_modules`
   and `docs-site/node_modules` as Excluded. RustRover excludes `target`
   automatically only for an attached Cargo project. They should help
   because the IDE otherwise indexes what the build writes.
4. **What an exclusion does not do.** An exclusion stops indexing, not file
   watching. `fsnotifier` watches all of `/` and the IDE filters its
   events (IJPL-2175).

Phase 10 adds the measured effect.

**File**: `tasks/README.md`

In "Taking a baseline or condition record", link the `--host-settings`
label bullet to `CONTRIBUTING.md#faster-local-builds-on-macos`, the
pointer Phase 4 leaves for this phase.

### Success Criteria:

#### Automated Verification:

- [ ] The docs tests pass: `uv run pytest tests/unit/tasks/test_contributing.py tests/unit/tasks/test_measure.py -v`
- [ ] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [ ] Following the steps on the reference host shows the terminal and IDE
      under Developer Tools after a restart.

---

## Phase 9: Record comparison

### Overview

This phase adds `measure:compare-builds <previous> <current>`, which applies
Requirement 9's citation rules mechanically:
- each change in seconds and as a ratio;
- the noise verdict, given when the current median lies within the
  previous [min, max];
- a change to or from absent, named in words;
- comparability under Requirement 7: every condition that differs, the
  host-settings label included, named on the report's first line;
- the task-set difference;
- with `--task <name>`, that task's duration and nextest-phase changes,
  under the same rules;
- the cargo unit totals and their total, as counts with their difference
  and ratio.

0361's closing comparison reuses it, and so do 0365 (`--task
test:unit:cli` for its nextest phase) and 0366 (the unit totals and the
cold wall-clock). It can merge any time after Phase 4.

### Changes Required:

#### 1. Tests first

**File**: `tests/unit/tasks/shared/build_measurement/test_comparison.py`

Every fixture is a `MeasuredBuild` built in code with the shared
`measured_build(**overrides)` builder, not a literal dictionary.

The changes:
- `test_a_change_is_given_in_seconds_and_as_a_ratio`.
- `test_a_median_inside_the_previous_range_is_within_noise`.
- `test_a_median_on_either_end_of_the_previous_range_is_within_noise`:
  parametrised at the previous minimum and maximum, since the range is
  inclusive.
- `test_a_median_outside_the_previous_range_is_not_noise`.
- `test_a_previous_record_of_one_run_judges_no_noise`: its range has no
  width, so the verdict is `unjudged`, not `beyond`.
- `test_a_process_present_in_both_records_has_its_change`.
- `test_the_ratio_is_current_over_previous`: 51.3 → 12.0 gives 0.234.
- `test_a_change_from_an_idle_median_has_seconds_but_no_ratio`.
- `test_a_change_from_an_idle_median_still_has_a_noise_verdict`:
  parametrised with the current median inside and outside the previous
  [min, max].
- `test_a_change_to_an_idle_median_keeps_its_ratio`: 51.3 → 0.0 gives 0.0.
- `test_a_written_record_compared_with_itself_shows_no_change`: built with
  `measure_build` over fakes, passed through `record_text` and
  `parse_record`, and compared with itself. It reports zero change, within
  noise, `conditions match`, and no task-set difference.

Presence and tracking:
- `test_a_process_absent_now_is_named_in_words_without_ratio_or_verdict`:
  "XprotectService did not run in any 'devtools' run".
- `test_a_process_absent_before_is_named_in_words`.
- `test_a_process_absent_from_both_records_is_named_as_unchanged`: "did not
  run in any 'none' or 'devtools' run", with no seconds, ratio or verdict.
- `test_a_flagged_side_is_named_in_words_with_its_caveats`: parametrised over
  `absent-in-some-runs` and `unreadable` on either side.
- `test_a_process_tracked_by_different_patterns_is_not_compared`: the
  `ide` entry of `tracked_processes` differs, so its change is
  `TrackedDifferently` holding both pattern lists, with no seconds, ratio
  or verdict. A process tracked on one side only is the same case, with an
  empty list on the other.

Conditions:
- `test_each_differing_condition_is_named_with_both_values`: parametrised
  over `MeasurementConditions.model_fields`, so a field added later is
  covered without editing the test.
- `test_a_differing_label_is_reported_like_any_condition`: `conditions
  differ: host_settings (none → devtools)`.
- `test_identical_conditions_read_as_matching`: `conditions match`.
- `test_a_condition_with_no_reading_never_matches`: the same `NoReading` on
  both sides, and a `NoReading` on one side, are both reported as
  differing.

Tasks and units:
- `test_the_task_set_difference_lists_added_and_removed_tasks`.
- `test_a_named_task_gets_its_duration_and_nextest_changes`: a task
  measured in both records gives a `TaskComparison` of `MeasuredChange`s
  with the noise verdict.
- `test_a_named_task_is_found_in_the_summary`: two isolated records whose
  `summary.tasks` hold the task are compared, whatever their `task_set`
  holds.
- `test_a_named_task_flagged_on_either_side_is_named_in_words`.
- `test_a_nextest_phase_on_one_side_only_is_named_in_words`: both sides
  measured, one with a nextest phase, gives `FigureOnOneSide`.
- `test_a_flagged_task_has_no_nextest_line`: when either side's presence
  is not `measured`, `nextest_phase` is `None` and the duration's
  `ChangeInPresence` carries the change.
- `test_a_named_task_missing_from_a_record_is_said_so`: parametrised over
  each side; `TaskMissingFromOne` says which record lacks it.
- `test_a_task_in_neither_record_suggests_the_closest_name`:
  `test:unit:cl` suggests `test:unit:cli`, and a name close to nothing
  suggests nothing.
- `test_no_task_is_compared_unless_named`.
- `test_unit_totals_are_compared_as_counts_without_a_noise_verdict`:
  run 1's `compiling`, `checking` and `documenting`, then their `total`,
  each with its difference and ratio; a zero previous count is named in
  words rather than given a ratio.

The report:
- `test_the_report_renders_every_change_in_words`: parametrised over every
  variant the worked example does not show (`ChangeFromIdle`,
  `FigureOnOneSide`, `TrackedDifferently`, `TaskMissingFromOne`,
  `TaskInNeither` with and without a closest name, a flagged side, an
  `unjudged` verdict, and a task-set difference), each with its exact
  line. Wording lives only in `comparison_report`, so the comparison's
  values carry facts, not sentences.
- `test_the_report_names_both_records_by_run_id`: `previous:` and
  `current:` lines straight after the verdict, each with its run count.
- `test_a_condition_with_no_reading_renders_its_command`: `os_version
  (unreadable (sw_vers -productVersion) → 26.4)`, and `os_version
  (unreadable in both records)` when both sides are.
- `test_a_tuple_condition_renders_comma_separated`: `isolated_tasks (empty
  → test:unit:cli, lint:cli)`.
- `test_the_report_matches_the_worked_example`: the example's two records
  render exactly the text below.

**File**: `tests/unit/tasks/test_mise.py`

- Add `measure:compare-builds` to the expected reaching set.

**File**: `tests/unit/tasks/test_measure.py`

- `TestCompareBuildsTask`:
  - `test_differing_conditions_still_exit_zero`: the report is printed and
    its first line names the differences;
  - `test_an_unreadable_record_exits_one_naming_the_file`;
  - `test_repeated_task_flags_reach_the_comparison`: `--task a --task b`
    parsed by invoke itself.

#### 2. Implementation

**File**: `tasks/shared/build_measurement/comparison.py`

```python
@unique
class Noise(StrEnum):
    WITHIN = "within"
    BEYOND = "beyond"
    UNJUDGED = "unjudged"


@unique
class RecordSide(StrEnum):
    PREVIOUS = "previous"
    CURRENT = "current"


@dataclass(frozen=True, slots=True)
class MeasuredChange:
    name: str
    seconds: float
    ratio: float
    noise: Noise


@dataclass(frozen=True, slots=True)
class ChangeFromIdle:
    name: str
    seconds: float
    current_median: float
    noise: Noise


@dataclass(frozen=True, slots=True)
class ChangeInPresence:
    name: str
    previous: Presence
    current: Presence


@dataclass(frozen=True, slots=True)
class FigureOnOneSide:
    name: str
    previous_median: float | None
    current_median: float | None


@dataclass(frozen=True, slots=True)
class TrackedDifferently:
    name: str
    previous_patterns: tuple[str, ...]
    current_patterns: tuple[str, ...]


type FigureChange = (
    MeasuredChange | ChangeFromIdle | ChangeInPresence | FigureOnOneSide
)
type ProcessChange = FigureChange | TrackedDifferently


@dataclass(frozen=True, slots=True)
class TaskComparison:
    name: str
    duration: FigureChange
    nextest_phase: FigureChange | None


@dataclass(frozen=True, slots=True)
class TaskMissingFromOne:
    name: str
    missing_from: RecordSide


@dataclass(frozen=True, slots=True)
class TaskInNeither:
    name: str
    closest: str | None


type NamedTask = TaskComparison | TaskMissingFromOne | TaskInNeither


@dataclass(frozen=True, slots=True)
class UnitCountChange:
    name: str
    previous: int
    current: int


@dataclass(frozen=True, slots=True)
class RecordComparison:
    previous_run_id: str
    current_run_id: str
    previous_runs: int
    current_runs: int
    previous_label: HostSettings
    current_label: HostSettings
    differing_conditions: tuple[ConditionDifference, ...]
    wall_clock: MeasuredChange
    processes: tuple[ProcessChange, ...]
    tasks_added: tuple[str, ...]
    tasks_removed: tuple[str, ...]
    named_tasks: tuple[NamedTask, ...]
    unit_totals: tuple[UnitCountChange, ...]


def compare_records(
    previous: MeasuredBuild,
    current: MeasuredBuild,
    *,
    tasks: Sequence[str] = (),
) -> RecordComparison: ...
def comparison_report(comparison: RecordComparison) -> str: ...
```

Every change type names its subject in `name`, so the report renders a
subject the same way whatever the case.

- **Named tasks.** Each name in `tasks` is looked up in both records'
  `summary.tasks`, not `task_set`, since an isolated record's tasks are
  the ones it ran rather than the `default` closure. A task in both gives
  a `TaskComparison`, whose figures follow the cases below. The nextest
  phase is settled from the task's presence first: when either side is
  not `measured`, `nextest_phase` is `None` and the duration's
  `ChangeInPresence` carries the change; otherwise it is `None` when
  neither side has a phase, `FigureOnOneSide` when one has, and a
  measured change when both have. A task missing from one record gives a
  `TaskMissingFromOne` naming that record. A task in neither gives a
  `TaskInNeither`, whose `closest` holds the nearest task name in either
  record by `difflib.get_close_matches`, or `None`. Without `tasks`, no task is
  compared, which keeps a ~79-task report from burying the verdict.
- **Tracked processes.** Each process is compared only when both records
  tracked it by the same patterns in `tracked_processes`. Otherwise its
  change is `TrackedDifferently`: two figures from different command-line
  matches do not measure one thing, so the report gives the two pattern
  lists in words and no figures.
- **Unit totals.** `UnitCountChange` compares run 1's `unit_totals` kind
  by kind, then their `total`, which 0366 cites. Unit counts do not vary
  between runs, so the report gives the two counts, their difference and
  their ratio, with no noise verdict; a zero previous count is named in
  words rather than given a ratio.

The task reads each path with `parse_record`, so an unreadable or
foreign record is refused with `UnreadableRecordError` naming the file
before anything is compared. Once both parse, it exits 0: it is a report,
not a gate. The report's first line opens with the verdict: `conditions
match`, or `conditions differ:` followed by each differing condition with
both values, for example `conditions differ: os_version (26.3 → 26.4)`.
Leading with the verdict keeps a glance from reading "comparable" when the
records do not compare. Each reader decides what it accepts: Phase 10
accepts exactly the label differing, and 0361's closing comparison accepts
nothing differing. A strict exit-status mode was considered and left out,
keeping the earlier decision that the tool reports rather than gates.

Each figure change is one of four cases, each holding facts;
`comparison_report` renders every sentence, using the two labels:
- **`MeasuredChange`.** Both medians are present and the previous one is
  above zero. The ratio is current ÷ previous. The wall-clock is always
  this case, since every run has one.
- **`ChangeFromIdle`.** The previous median is 0.0, so there is no ratio.
  The change is given in seconds, with its noise verdict, and the report
  names it in words: "XprotectService used no measurable CPU in any
  'none' run".
- **`ChangeInPresence`.** Either side is not `measured`: absent, or
  flagged (missing or unreadable in some runs). The report gives only
  words, with no seconds, ratio or noise verdict, built from the two
  `Presence` values. When both sides are absent it says the process did
  not run under either label, and names nothing as a change.
- **`FigureOnOneSide`.** Both sides are measured, but only one has the
  figure in every run, as when a task gains or loses its nextest phase.
  The report gives the one median and says the other record has no
  nextest phase in every run.

The noise verdict is `within` when the current median lies in the
previous [min, max] and `beyond` otherwise. When the previous record has
a single timed run its range has no width, so the verdict is `unjudged`
and the report says "no spread to judge noise (1 run)" rather than
calling every change beyond noise.

`comparison_report` is composed from one renderer per section (verdict,
records, wall-clock, processes, task set, named tasks, unit totals), so
no single function carries every branch past ruff's C901 and PLR0912
limits. Condition values render readably: a `NoReading` as `unreadable
(<command>)`, a pair of them as `unreadable in both records`, and a tuple
comma-separated, or `empty`. A host-settings label inside a sentence is
quoted, so `'none'` the label never reads as the word. For example,
comparing the `none` and `devtools` records with `--task test:unit:cli`:

```text
conditions differ: host_settings (none → devtools)
previous: build_2026-11-02T091500Z_engywook_warm-no-op_none (3 runs)
current: build_2026-11-02T101200Z_engywook_warm-no-op_devtools (3 runs)
wall-clock: 309.4 s → 262.0 s, -47.4 s (×0.847), beyond noise
syspolicyd: 51.3 s → 12.0 s, -39.3 s (×0.234), beyond noise
XprotectService: did not run in any 'none' or 'devtools' run
ide: 301.0 s → 296.4 s, -4.6 s (×0.985), within noise
fsnotifier: 12.4 s → 12.1 s, -0.3 s (×0.976), within noise
task set: unchanged
test:unit:cli duration: 297.1 s → 251.0 s, -46.1 s (×0.845), beyond noise
test:unit:cli nextest phase: 149.2 s → 118.5 s, -30.7 s (×0.794), beyond noise
units compiling: 61 → 61, +0 (×1.000)
units checking: 12 → 12, +0 (×1.000)
units documenting: none in either record
units total: 73 → 73, +0 (×1.000)
```

**Files**: `tasks/measure.py`, `mise.toml`, `tasks/README.md`

- **`tasks/measure.py`.** Add `@task(name="compare-builds",
  iterable=["task"])`, taking two record paths and a repeatable `--task`,
  with `help` entries and a docstring stating what the comparability
  check covers and what `--task` adds. It prints the report.
- **`mise.toml`.** Register it as `run = "invoke measure.compare-builds"`
  under a one-line comment of its own ("operator-run; reads two kept
  records, no build"), since the `measure:build` comment describes a
  20-minute build. Its `description` names the arguments' order
  (previous, then current).
- **`tasks/README.md`.** Three changes:
  - a **Comparing build records** bold-lead paragraph, after Cache
    states: the task applies the comparability rule, label included,
    names every differing condition on its first line, then both records'
    run ids, and exits 0 whenever both records parse; a baseline-to-final
    comparison must show none differing. A process tracked by different
    patterns is named but not compared. `--task <name>` adds a task's
    duration and nextest-phase changes, and the unit totals, with their
    total, are always compared;
  - `measure:compare-builds` in "Who runs what" as operator-run;
  - a bullet in "Taking a baseline or condition record": before pushing
    or describing the change that holds a kept record (under jj it joins
    the working-copy commit at once), compare it with the previous kept
    record of the same condition, and look into any change beyond noise.
    A condition's first record has nothing to compare with, so the step
    is skipped.
- **Doc test.** Extend `test_build_is_documented_without_a_quiet_host` to
  require `measure:compare-builds` and `--task` in the measure section.

#### 3. Work items, after merging

| Item | Frontmatter | Body |
|---|---|---|
| 0365 | — | Technical Notes: compare the before and after isolated records with `mise run measure:compare-builds -- <before> <after> --task test:unit:cli`, whose nextest-phase line carries the change and the noise verdict |
| 0366 | — | Technical Notes: compare the before and after cold records with `mise run measure:compare-builds -- <before> <after>`, whose unit-totals lines carry the counts and whose wall-clock line carries the median change |

### Success Criteria:

#### Automated Verification:

- [ ] The tests pass: `uv run pytest tests/unit/tasks/shared/build_measurement tests/unit/tasks/test_mise.py tests/unit/tasks/test_measure.py -v`
- [ ] Lint and types pass: `mise run build-system:check`
- [ ] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [ ] `mise run measure:compare-builds -- <baseline> <baseline> --task
      test:unit:cli` on the committed baseline reports zero change, within
      noise, `conditions match`, no task-set difference, equal unit
      totals, and zero change for `test:unit:cli`.

---

## Phase 10: Host-settings measurements and citations

### Overview

This phase covers Requirement 9. It records the `none`, `devtools` and
`devtools+ide-exclusion` conditions at one commit and cites their changes in
`CONTRIBUTING.md`. It starts only once 0362, 0363 and 0367 have each landed
or been abandoned.

### Changes Required:

#### 1. Record the conditions

On the reference host, at one commit on `main` taken after 0362, 0363 and
0367 have landed or been abandoned, with the IDE open on the project for
every condition:
1. Remove the terminal, the IDE and tmux or zellij from Developer Tools.
   Clear the IDE exclusions. Reboot if the exemption persists. Then run
   `mise run measure:build -- --runs 3 --keep --host-settings none`.
2. Add the Developer Tools entries and restart the apps. Run with
   `--host-settings devtools`.
3. Add the IDE exclusions. Run with
   `--host-settings devtools+ide-exclusion`.

Retake a condition under any of the retake conditions in the README's
"Taking a baseline or condition record", which is their one statement.
Beyond those, each record needs `commit.matched` true.

Check comparability with
`mise run measure:compare-builds -- <none> <devtools>` and
`… <devtools> <devtools+ide-exclusion>`. Each must name exactly
`host_settings` as differing, and nothing else. Commit the three records.

#### 2. Cite them

**File**: `CONTRIBUTING.md`

Under `## Faster local builds on macOS`, add `### Measured effect`, built
from the two `measure:compare-builds` reports. It contains:
- the host (`engywook`, Apple M4 Max, macOS version) and the commit;
- each step's change in median wall-clock and in each tracked process's
  median CPU-seconds, in seconds and as a ratio. The IDE exclusion's effect
  is stated as on top of the exemption;
- "not distinguishable from noise on the reference host" wherever the
  report says so;
- every change to or from absent, named in words, with no ratio;
- every change from an idle median, in seconds and in words, with no
  ratio;
- the three record paths;
- a sentence saying the ratios are what carry over to other Macs, and the
  absolute seconds do not;
- a pointer to `mise run measure:build -- --host-settings <label>` for
  measuring another Mac.

**File**: `tests/unit/tasks/test_contributing.py`

- Add `test_the_measured_effect_cites_three_condition_records`. Each cited
  path exists under `meta/measurements/` and carries its label.
- Add `test_every_cited_change_matches_the_comparison`. It runs
  `compare_records` over the cited records and asserts that each rendered
  change appears in the section: seconds, ratio and noise wording.
- Add `test_the_measured_effect_points_at_measure_build`.

### Success Criteria:

#### Automated Verification:

- [ ] The docs test passes: `uv run pytest tests/unit/tasks/test_contributing.py -v`
- [ ] Each condition record meets the bar: `jq -e '.fetch_failures == [] and .commit.matched and (.runs | length) == 3 and ([.summary.processes | to_entries[] | select(.key != "XprotectService") | .value.presence.standing] | all(. == "measured"))' meta/measurements/<run-id>.json`
- [ ] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [ ] The three records share one commit, and each comparison report names
      exactly `host_settings` as differing.

---

## Testing Strategy

### Unit Tests:

- **Pure core (Phases 1, 2, 3 and 9).**
  - Hand-seeded fixture logs, with each known answer worked out beside its
    assertion. They cover every acceptance-criterion bullet under "Given
    fixture logs with known answers".
  - One captured golden per sampler, one for the build log, one for
    `mise tasks --json` and one unfiltered `ps` tick, with answers derived
    independently at capture time.
- **Orchestration (Phases 4, 6 and 7).** These run against
  `ScriptedBuildRunner`, `ScriptedSampler`, `FakeHistory`, `FakeClock`,
  `FakeBuildGroup`, `RecordingWorkspace` and `fake_diagnostics`. A refusal
  test proves no port is touched, and the cache-state tests observe a real
  `tmp_path` workspace.
- **Shared builders.** `measured_build(**overrides)` and its part builders
  give every record, report and comparison test a valid `MeasuredBuild`
  that differs from the default only where the test says.
- **Module boundaries.** `test_imports.py` fails CI when a pure module
  imports `session`, or anything but `wiring` imports an adapter.
- **Adapters.**
  - Portable real-process tests, with `sys.executable` scripts standing in
    for the child and for `ps`, synchronised on the log rather than a
    timer.
  - Real `jj` and `git` repositories for `JjHistory` and `GitHistory`,
    gated by `require`, so CI fails rather than skips.
  - The session lock's refusal over two `WorkspaceSessionLock(tmp_path)`
    instances, and `history_of` over real `.jj` and `.git` directories.
  - `BuildGroupFile` in `test_build_group.py`, over real, stale and
    malformed `build.pgid` files and `FakeProcs`.
- **Records.** A round trip through `record_text` and `parse_record` for
  every record shape the phases add, known answers for the serialised
  summary and one run, and a test that parses every committed
  `build_*.json`, so the frozen schema cannot drift unnoticed.
- **Task layer.**
  - Flag validation, with a real `Context()`, `measure_build_with`
    monkeypatched to capture its settings factory, `real_ports` replaced
    by a factory that fails the test if called, and repeated flags parsed
    by invoke itself.
  - The refusal-to-`Exit` mapping, tested by calling `measure_build_with`
    directly over fake ports rooted at `tmp_path`.
- **Documentation.** `TestMeasureNamespaceDocs` pins the README's measure
  section, `TestHostSettingsGuidance` the `CONTRIBUTING.md` steps, and
  Phase 10's tests check each cited record and figure against
  `compare_records`.

### Integration Tests:

- None in an automated lane. The harness is operator-run and macOS-only,
  and a real run takes about 20 minutes. Requirement 10 also keeps it out
  of every closure.

### Manual Testing Steps:

1. `mise run measure:build -- --runs 1 --keep` on the reference host.
   Inspect both copies of the record and the run directory.
2. Ctrl-C mid-run. Confirm no stray `mise`, `cargo`, `nextest` or `iostat`
   processes remain and no record is written.
3. `mise run measure:build -- --isolate test:unit:cli --runs 1`.
4. `mise run measure:build -- --cache-state incremental --runs 1`, then
   `--cache-state cold --runs 1`.
5. `mise run measure:compare-builds -- <a> <a>`.

## Performance Considerations

- ⏱️ **Run length.** A warm `--runs 3` session takes about 20 minutes: 30 s
  of idle sampling, a priming build, and three builds of roughly 300 s. A
  cold `--runs 3` takes about 25 minutes and fetches npm packages each run.
- **Sampler cost.** The `ps` sampler runs `ps -ww -A` once a second. It keeps
  only tracked lines, so a run's `ps` log is kilobytes rather than about
  30 MB. `iostat -w 2` costs nothing measurable.
- **Measurement bias.** Per-process CPU is a lower bound: one-second
  sampling misses short-lived processes, and the post-build tail is not
  counted. The README's build-timing paragraph says so once, rather than
  the record repeating it beside every figure.

## Migration Notes

None. The plan changes no existing record format. The `.gitignore` pattern
is unchanged, and only its comment grows. The record carries
`schema_version: 1`, typed `Literal[1]`, so `parse_record` refuses any
other shape and `measure:compare-builds` never compares one it does not
understand.

The record's models are the live analysis types, so after the Phase 5
baseline every change to them follows one rule, which the README's
"The record's schema" paragraph states too:
- **Additive.** A new field carries a default and the version stays 1, so
  every committed record still validates.
- **Breaking.** A rename, a removal or a changed type bumps the version.
  `parse_record` then reads each older version through an explicit upgrade
  function over its raw JSON, applied before validation, so the baseline
  stays readable until 0361 closes. Records are read by code at least as
  new as the code that wrote them.

`test_every_committed_build_record_is_readable` enforces the rule: a
breaking edit without an upgrade fails CI in the commit that makes it.

## References

- Work item: `meta/work/0368-build-performance-measurement-and-host-guidance.md`
- Implementation research: `meta/research/codebase/2026-10-30-0368-build-performance-measurement-harness.md`
- Source research and its harness appendix: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Precedent plan: `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md`
- Ports and wiring: `tasks/shared/vendor/pin_guard/guard.py`, `tasks/shared/vendor/pin_guard/wiring.py:13-22`
- Child processes: `tasks/shared/dev/circus.py:187-214`, `tasks/shared/dev/host_server.py:138-159`, `tasks/shared/processes.py:34-87`
- Signal unwinding: `tasks/measure.py:566-575`, `:634-647`
- Helpers moved down a layer: `tasks/measure.py:409`, `:541-563`, `:3186-3209`
- Plan review: `meta/reviews/plans/2026-10-30-0368-build-performance-measurement-harness-review-1.md`
- Closure guard: `tests/unit/tasks/test_mise.py:244-299`
