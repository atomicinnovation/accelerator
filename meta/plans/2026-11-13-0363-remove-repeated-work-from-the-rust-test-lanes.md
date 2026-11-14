---
type: "plan"
id: "2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes"
title: "Remove Repeated Work from the Rust Test Lanes Implementation Plan"
date: "2026-11-13T17:25:00+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0363"
parent: "work-item:0363"
derived_from: ["codebase-research:2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes"]
relates_to: ["codebase-research:2026-10-28-full-build-duration-contention-and-slowness"]
tags: ["build-system", "performance", "testing", "rust", "coverage"]
revision: "ad58463b332b7d22d8e54fb3f1c60a51f308d6d7"
repository: "accelerator"
last_updated: "2026-11-14T09:36:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Remove Repeated Work from the Rust Test Lanes Implementation Plan

## Overview

Two Rust test lanes repeat work on every run. This plan removes it in three
phases, each of which can be merged on its own:

1. The visualiser's `log::tests` rotation tests reach the 5 MiB cap in a few
   large writes instead of millions of 2-byte writes.
2. `test:unit:cli` replaces cargo-llvm-cov's full clean with a profraw-only
   clean, runs nextest with `--no-clean --no-report`, then reports the totals
   as a single line.
3. `test:unit:cli` fails when any total falls below a coverage floor. This
   goes beyond 0363's scope, at the user's request, and reverses the
   report-only stance of the 0162 plan.

## Current State Analysis

**Coverage lane.** `tasks/test/cli.py:43-50` runs one command,
`cargo llvm-cov nextest {_MANIFEST} --summary-only`, streams it, and raises
`Exit("nextest: cli tests failed")` on a non-zero exit. cargo-llvm-cov does a
full `cargo clean --package` of all 44 members before each run (61 s plus a
30 s instrumented rebuild). `--summary-only` is ignored in text mode, so a
523-row per-file table is printed. With `ACCELERATOR_COVERAGE=off`
(`tasks/shared/rust.py:41-52`), the task runs `cargo nextest run {_MANIFEST}`
instead.

**Log-rotation tests.** `cli/visualiser/server/src/log.rs:121-198` holds three
tests that loop `write_all(b"x\n")` until they pass `MAX_BYTES + 64 KiB`.
`retains_at_most_three_rotated_segments` alone makes about 13.3 M writes
(22.3 s, 20.8 s of it in the OS kernel). The tests assert against `MAX_BYTES`
and `MAX_FILES` by name and check `rotated <= MAX_FILES`, so they pass under
`MAX_FILES = 2` and `MAX_FILES = 4`. No test checks the size of a rotated
segment.

### Key Discoveries:

- file-rotate 0.7.6 splits a single buffer exactly at the cap and loops over
  several caps in one write (`file-rotate-0.7.6/src/lib.rs:637-654`).
  Rotation fires only when a write would go *past* the cap, so a write that
  lands exactly on 5 MiB does not rotate.
  `AppendCount::new(3)` keeps `server.log.1` to `.3`
  (`suffix.rs:138-140`, `lib.rs:590-607`).
- `cargo llvm-cov report --json --summary-only` emits
  `{data: [{files: [...], totals: {...}}], ...}`, not just the totals. The
  percentages are at `data[0].totals.{lines,functions,regions}.percent`.
  The work item's Technical Notes say "the export's `totals` object only",
  which is wrong. Phase 2 corrects the note and pins the real shape with a
  trimmed real export as the parser fixture.
- `cargo llvm-cov report` in 0.8.7 has no `--workspace`/`--exclude`. The
  report stays scoped as today only because no member depends on
  `accelerator-visualiser`. `tasks/lint/crate_dependencies.py` does not fully
  enforce this. The visualiser is a `composition-root`, but two gaps remain.
  `broken_rules` (`:369-401`) returns `[]` for any test-support source, and
  `edges` (`:319-329`) drops dev-dependencies entirely. So a test-support
  crate may depend on the visualiser, and any member may take it as a
  dev-dependency. Phase 2 adds a guard test, built on
  `tasks/shared/cargo_metadata.workspace_packages()`, that fails if any
  member depends on the visualiser by any kind of dependency.
- `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS` only skips the permission tests
  when the process ignores permissions, that is, when it runs as root
  (`cli/store/src/test_support.rs:27-43`). CI never sets it. A run that
  exercises the skip must run as root.
- The capture-then-parse pattern to follow: `tasks/docs.py:53-58`
  (`hide="out"`) and the pure parser `tasks/shared/npm_audit.py:38-55`, with
  its own tests in `tests/unit/tasks/test_npm_audit.py`.
- `tests/unit/tasks/test_rust.py:104-139` (`TestTestUnitCli`) checks only the
  last `context.run` call, and its `ctx` fixture returns `stdout=""`. It must
  be rewritten around the call sequence.
- `tests/unit/tasks/test_nextest_filter.py:37-47` fails if the substring
  `--profile` or `--ignore-default-filter` appears anywhere in
  `tasks/test/cli.py`. `--profraw-only` is safe, but no wording may contain
  `--profile`.
- `tests/unit/tasks/test_cli_permission_skip.py` runs with coverage off and a
  single-result `MockContext`, so it is unaffected.
- `_MANIFEST` is also imported by `tasks/test/integration.py:8`, so the new
  `clean` and `report` arguments must not be folded into it.
- `tasks/README.md:778`, `mise.toml:333-336` and the `coverage_enabled`
  docstring (`tasks/shared/rust.py:44`) describe the coverage switch and
  lane, and all three need their wording updated.
  `tests/unit/tasks/fixtures/build-measurement/warm-no-op-tasks.golden.json`
  is a recorded measurement holding the old description as historical data.
  It is not compared with `mise.toml` and is left alone.

## Desired End State

- `log::tests` takes under 2 CPU-s (user + sys) for the `dev-frontend` lib
  build. It asserts the literal production values: rotated segments of exactly
  5,242,880 bytes and exactly 3 rotated segments kept. Halving or doubling
  `MAX_BYTES`, or setting `MAX_FILES` to 2 or 4, fails at least one test.
- With coverage on, `test:unit:cli` runs exactly these commands in order:
  `cargo llvm-cov clean --profraw-only`,
  `cargo llvm-cov nextest … --no-clean --no-report`, then
  `cargo llvm-cov report … --json --summary-only` (captured). It prints one
  line, `coverage: lines NN.NN%, functions NN.NN%, regions NN.NN%`, and no
  per-file rows or JSON.
- A failing clean, nextest run or report, or an export with no usable totals,
  fails the task. A total below its floor fails the task with a message that
  names each shortfall, where the floor is set, and the
  `cargo llvm-cov clean --workspace` remedy.
- The nextest and report commands share one manifest-path and one
  feature-selection fragment, and a guard test fails if any workspace member
  depends on `accelerator-visualiser`.
- With `ACCELERATOR_COVERAGE=off`, the task runs only `cargo nextest run`.
- The docstring names `cargo llvm-cov clean --workspace` as the remedy for
  suspected stale instrumented binaries.

## What We're NOT Doing

- Not dropping instrumentation locally. `default` mirrors CI.
- Not adding `--ignore-filename-regex` for the visualiser to `report`. The
  Phase 2 guard test keeps visualiser objects out of `llvm-cov-target`.
- Not scheduling a periodic or fingerprint-triggered full clean. A Phase 2
  manual check confirms that a dependency bump leaves the totals unchanged.
  The remedy for stale binaries is the manual one named in the docstring and
  the floor-failure message. Revisit this only if drift is observed.
- Not changing `test:unit:visualiser` to nextest, or changing its two
  `cargo test --lib` runs.
- Not touching `make_writer_errors_when_parent_path_is_a_file` or
  `emits_json_lines_with_message_and_field`. In
  `pre_existing_oversized_log_rotates_on_first_write`, only the
  segment-counting call changes, to use the shared helper.
- Not changing production logging, including file-rotate's habit of
  splitting a JSON line that straddles the cap.
- Not changing the rust-cache step in `.github/workflows/main.yml`.
- Not ratcheting the coverage floor beyond its starting values.
- Not exempting root runs that set `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS`
  from the floor. The starting floors are chosen so those runs meet them too.

## Implementation Approach

Phase 1 changes only Rust test code, and Phases 2 and 3 change only the
Python task tree, so Phase 1 is independent of the other two. Phase 3 builds
on Phase 2's parsed totals. Each phase follows red-green-refactor:

- Phase 1's red step proves the current tests miss the mutations, then that
  the new ones catch them. Production code doesn't change there.
- Phases 2 and 3 write failing pytest cases first. Domain logic (parsing
  totals, judging them against a floor) lives in a pure module,
  `tasks/shared/coverage.py`. The task body only runs the commands and turns
  failures into `Exit`.

Take the "before" timings for each phase on engywook before applying that
phase.

## Phase 1: Rotation tests reach the cap in a few large writes

### Overview

Rewrite the three slow rotation tests to drive file-rotate with buffers of
5 MiB + 64 KiB, and assert the literal production values.

### Changes Required:

#### 1. Record the before figures and the mutation gap (red)

On engywook, with the host otherwise idle, on the unchanged code:

```sh
cargo nextest run -p accelerator-visualiser --lib \
  --manifest-path cli/Cargo.toml \
  --no-default-features --features dev-frontend --no-run
/usr/bin/time -l cargo nextest run -p accelerator-visualiser --lib \
  --manifest-path cli/Cargo.toml \
  --no-default-features --features dev-frontend log::tests
cargo test -p accelerator-visualiser --lib \
  --manifest-path cli/Cargo.toml \
  --no-default-features --features dev-frontend --no-run
/usr/bin/time -l cargo test -p accelerator-visualiser --lib \
  --manifest-path cli/Cargo.toml \
  --no-default-features --features dev-frontend log::tests
```

The second pair times the tests under `cargo test`, the runner
`test:unit:visualiser` uses, with all tests as threads in one process.
For each command, record user + sys as the median of three runs. Then apply each of the four
mutations in turn (`MAX_BYTES / 2`, `MAX_BYTES * 2`, `MAX_FILES = 2`,
`MAX_FILES = 4`) and record which ones the current tests miss. The
`MAX_FILES` mutations are expected to pass. That is the red state the
rewrite must turn green.

#### 2. Rewrite the three tests

**File**: `cli/visualiser/server/src/log.rs`
**Changes**: In `mod tests`, add literal constants and helpers, and rewrite
`rotates_when_active_segment_crosses_5_megabytes`,
`retains_at_most_three_rotated_segments` and
`active_and_rotated_segments_are_owner_only`. Keep their names. The new tests
must not refer to `MAX_BYTES` or `MAX_FILES`. Replace `count_segments` with
the shared helpers: `pre_existing_oversized_log_rotates_on_first_write`
asserts `!rotated_segments(dir.path()).is_empty()`. That test can't call
`write_past_the_cap`, because it seeds the file before the writer opens it.

```rust
    const ROTATION_CAP: u64 = 5 * 1024 * 1024;
    const RETAINED_SEGMENTS: usize = 3;
    const OVERSHOOT: u64 = 64 * 1024;

    fn write_past_the_cap(
        writer: &mut FileRotate<AppendCount>,
        rotations: usize,
    ) {
        let past_the_cap = vec![b'x'; (ROTATION_CAP + OVERSHOOT) as usize];
        for _ in 0..rotations {
            writer.write_all(&past_the_cap).unwrap();
        }
        writer.flush().unwrap();
    }

    fn log_segments(dir: &Path) -> Vec<std::fs::DirEntry> {
        let mut segments: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with("server.log")
            })
            .collect();
        segments.sort_by_key(std::fs::DirEntry::file_name);
        segments
    }

    fn rotated_segments(dir: &Path) -> Vec<std::fs::DirEntry> {
        log_segments(dir)
            .into_iter()
            .filter(|e| e.file_name() != "server.log")
            .collect()
    }

    fn rotated_segment_sizes(dir: &Path) -> Vec<u64> {
        rotated_segments(dir)
            .iter()
            .map(|e| e.metadata().unwrap().len())
            .collect()
    }

    #[test]
    fn rotates_when_active_segment_crosses_5_megabytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let mut writer = make_writer(&path).unwrap();

        write_past_the_cap(&mut writer, 1);
        drop(writer);

        assert_eq!(rotated_segment_sizes(dir.path()), vec![ROTATION_CAP]);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), OVERSHOOT);
    }

    #[test]
    fn retains_at_most_three_rotated_segments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let mut writer = make_writer(&path).unwrap();

        write_past_the_cap(&mut writer, 5);
        drop(writer);

        assert_eq!(
            rotated_segment_sizes(dir.path()),
            vec![ROTATION_CAP; RETAINED_SEGMENTS]
        );
    }

    #[cfg(unix)]
    #[test]
    fn active_and_rotated_segments_are_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let mut writer = make_writer(&path).unwrap();
        let rotations = 2;

        write_past_the_cap(&mut writer, rotations);
        drop(writer);

        let segments = log_segments(dir.path());
        assert_eq!(segments.len(), rotations + 1);
        for segment in segments {
            let mode = segment.metadata().unwrap().permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o600,
                "{:?} must be owner-only",
                segment.file_name()
            );
        }
    }
```

What each assertion covers:

| Property in the acceptance criteria | Assertion |
|---|---|
| Each full rotated segment is exactly 5 MiB | `rotated_segment_sizes` equals `ROTATION_CAP` per segment (tests 1 and 2) |
| The active segment is smaller than 5 MiB after the cap is crossed | the active segment is exactly `OVERSHOOT` long (test 1), which also pins that the split happens at the cap |
| More than three rotations keep exactly three segments | five rotations leave `vec![ROTATION_CAP; 3]` (test 2) |
| A single write that crosses the cap triggers rotation | one `write_all` gives one rotated segment (test 1) |
| The active segment and every rotated segment are owner-only | `rotations + 1` files counted, each `0o600` (test 3) |

How each mutation is caught:

| Mutation | Failing assertion |
|---|---|
| `MAX_BYTES / 2` | segment sizes are 2.5 MiB (tests 1 and 2) |
| `MAX_BYTES * 2` | test 1: no rotation, so the size list is empty and the active segment is 5 MiB + 64 KiB; test 2: `[10 MiB, 10 MiB]`; test 3: 2 files, not 3 |
| `MAX_FILES = 2` | 2 segments kept, not 3 (test 2) |
| `MAX_FILES = 4` | 4 segments kept, not 3 (test 2) |

The buffer of 5 MiB + 64 KiB never lands exactly on a cap multiple, so
file-rotate's "rotate only past the cap" rule never leaves a pending
rotation.

### Success Criteria:

#### Automated Verification:

- [x] The visualiser lib suite passes in both feature sets:
      `mise run test:unit:visualiser`
- [x] The Rust lints pass: `mise run server:check` and `mise run cli:check`

#### Manual Verification:

- [x] Each of the four mutations, applied alone and then reverted, fails at
      least one of the three rotation tests in the way the mutation table
      predicts:
      `cargo nextest run -p accelerator-visualiser --lib --manifest-path cli/Cargo.toml --no-default-features --features dev-frontend log::tests`
- [x] On engywook, idle, with the lib binary prebuilt (`--no-run`), the timed
      `cargo test … log::tests` command above shows user + sys under 2 CPU-s
      (median of three). The PR description records both runners' before and
      after figures.
      - Recorded 2026-11-14, user + sys median of three, with IntelliJ busy
        on one core and a 5-minute load average of 11 to 18: `cargo nextest
        run` 35.86 → 0.39 CPU-s, `cargo test` 30.59 → 0.14 CPU-s.
- [x] `log.rs` has no new comments.

---

## Phase 2: Coverage lane keeps profiles clean and prints a totals line

### Overview

Replace the single `cargo llvm-cov nextest` command with clean, run and
report steps. Parse the report's totals into a domain value and print it as
one line. Failures in the coverage steps fail loudly.

### Changes Required:

#### 1. Record the before figures

On engywook, warm (after one completed `test:unit:cli` run with no source
changes since), with no other task running:
`time mise run --skip-deps test:unit:cli`. Record the median of three runs
and the number of tests nextest reports.

#### 2. Coverage totals parser (tests first)

**File**: `tests/unit/tasks/fixtures/coverage/llvm-cov-summary.json` (new)
**Changes**: Capture a real export once, after a lane run, with
`cargo llvm-cov report --manifest-path cli/Cargo.toml --all-features --json --summary-only`.
Trim `data[0].files` to two entries and leave everything else, including
`totals`, as captured. This pins the real 0.8.7 shape, not a hand-built
guess at it.

**File**: `tests/unit/tasks/test_coverage.py` (new)
**Changes**: Pure tests of the parser and formatter.

- `parse_totals` on the real fixture gives the fixture's
  `data[0].totals.{lines,functions,regions}.percent`, each rounded to two
  decimals.
- `parse_totals` rounds to two decimals: an export whose lines percent is
  `77.98742138364779` gives `77.99`, and one of `76.996` gives `77.0`.
- `summary_line()` on totals of lines `77.99`, functions `81.2` and regions
  `74.05` gives exactly
  `coverage: lines 77.99%, functions 81.20%, regions 74.05%`.
- `summary_line()` on the real fixture's totals contains no file name from
  the fixture and no `{`.
- Non-JSON text raises `CoverageReportError` saying the report is not JSON.
- A missing `data`, an empty `data` list, a missing `totals`, a missing
  metric and a non-numeric percent each raise `CoverageReportError` saying
  the report carries no totals. Build these from small dicts.
- `CoverageTotals` built directly with a metric missing raises `ValueError`
  naming that metric.

**File**: `tasks/shared/coverage.py` (new)

The totals are held already rounded to the two decimals the totals line
shows, so Phase 3's floor judges the same figure a developer reads.

```python
"""Coverage totals from a cargo-llvm-cov summary export."""

import dataclasses
import json
from collections.abc import Mapping
from enum import StrEnum


class CoverageReportError(Exception): ...


class Metric(StrEnum):
    LINES = "lines"
    FUNCTIONS = "functions"
    REGIONS = "regions"


def _require_every_metric(percents: Mapping[Metric, float]) -> None:
    missing = [metric for metric in Metric if metric not in percents]
    if missing:
        raise ValueError(f"no percentage for {', '.join(missing)}")


@dataclasses.dataclass(frozen=True)
class CoverageTotals:
    percents: Mapping[Metric, float]

    def __post_init__(self) -> None:
        _require_every_metric(self.percents)

    def summary_line(self) -> str:
        return "coverage: " + ", ".join(
            f"{metric} {self.percents[metric]:.2f}%" for metric in Metric
        )


def parse_totals(export: str) -> CoverageTotals:
    try:
        document = json.loads(export)
    except json.JSONDecodeError as error:
        raise CoverageReportError(
            f"the coverage report is not JSON ({export[:80]!r})"
        ) from error
    try:
        totals = document["data"][0]["totals"]
        return CoverageTotals(
            {
                metric: round(float(totals[metric]["percent"]), 2)
                for metric in Metric
            }
        )
    except (LookupError, TypeError, ValueError) as error:
        raise CoverageReportError(
            f"the coverage report carries no totals ({error!r})"
        ) from error
```

#### 3. Task sequence (tests first)

**File**: `tests/unit/tasks/test_rust.py`
**Changes**: Rewrite `TestTestUnitCli` around the whole call list. Give it
its own fixture whose `run` returns `MagicMock(exited=0, stdout=EXPORT)`,
with `EXPORT` the text of the real fixture export. Every call returns it, and
only the report step reads it. Leave the shared `ctx` fixture as it is for
the other classes.

- `test_clears_profile_data_then_tests_then_reports`: with coverage on, the
  commands are exactly three, in order. The first starts with
  `cargo llvm-cov clean` and contains `--profraw-only`. The second starts
  with `cargo llvm-cov nextest` and contains `--no-clean`, `--no-report`,
  `--exclude accelerator-visualiser` and `--all-features`. The third starts
  with `cargo llvm-cov report` and contains `--all-features`, `--json` and
  `--summary-only`. All three contain
  `--manifest-path {CLI_WORKSPACE_CARGO_TOML}`.
- `test_only_the_report_output_is_captured`: only the report call passes
  `hide="out"`.
- `test_prints_the_totals_line` (capsys): stdout contains the line for
  `EXPORT`'s totals and no `"files"`/`{`.
- `test_plain_nextest_when_coverage_off`: one call, starting with
  `cargo nextest run`, with no `llvm-cov`.
- `test_failing_tests_stop_before_the_report`: if the nextest step exits 1,
  `Exit` is raised and only two calls were made.
- `test_failing_profile_clean_stops_before_the_tests`: if clean exits 1,
  `Exit` is raised after one call.
- `test_failing_report_fails_the_task`: if the report exits 1, `Exit` is
  raised.
- `test_unreadable_report_fails_the_task`: if the report's stdout is
  `"not json"`, `Exit` is raised.
- `test_docstring_names_the_stale_binary_remedy`:
  `"cargo llvm-cov clean --workspace"` appears in `test_cli.run.body.__doc__`.

Use `side_effect` lists for the per-step failure cases. Delete
`test_instrumented_by_default`, because the sequence test replaces it.
Repoint `test_raises_when_inner_tests_fail` at the coverage-off path and
rename it `test_failing_plain_nextest_fails_the_task`: with
`ACCELERATOR_COVERAGE=off` and the one call exiting 1, `Exit` is raised. Keep
`test_carries_no_coverage_threshold` in this phase. It asserts that no
command contains `--fail-under` across all calls, and Phase 3 keeps it
because the floor is applied in Python.

**File**: `tests/unit/tasks/test_rust.py`
**Changes**: Add `TestVisualiserHasNoDependents` beside the surface-pin
guard. It uses a test-local `_dependents_of(crate, packages)` over
`cargo metadata` packages. It returns the names of packages with any entry
in `dependencies` whose `name` is `crate`. cargo has already resolved every
kind (normal, dev, build), every target-specific table, renames and
`workspace = true` inheritance, so the test parses no manifests itself.
This follows `tests/unit/tasks/test_crate_dependencies.py`, which runs
`workspace_packages()` against the real tree and uses synthetic package
dicts for the branch cases.

- `test_no_workspace_member_depends_on_the_visualiser`:
  `_dependents_of(test_cli._VISUALISER, workspace_packages())` is empty. The
  failure message says that `cargo llvm-cov report` cannot exclude the
  visualiser, so a dependent would pull it into the totals.
- `test_the_guard_finds_a_dependent` (parametrised over `kind` `None`,
  `"dev"` and `"build"`): a synthetic package listing the visualiser as a
  dependency of that kind is reported.
- `test_the_guard_ignores_other_dependencies`: a synthetic package that
  depends only on another member is not reported.

**File**: `tasks/test/cli.py`

The manifest path and feature selection become named fragments shared by
`_MANIFEST` and the report command. `_MANIFEST`'s value is unchanged, and
`tasks/test/integration.py` keeps importing it. The existing comment that
explains why the visualiser is excluded (its own lanes, its own features and
environment) stays, unchanged, above these fragments. The guard test's
failure message carries the reasoning about the report's scope, so the
comment does not grow.

```python
_VISUALISER = "accelerator-visualiser"
_WORKSPACE = f"--manifest-path {CLI_WORKSPACE_CARGO_TOML}"
_FEATURES = "--all-features"
_MANIFEST = f"{_WORKSPACE} --workspace --exclude {_VISUALISER} {_FEATURES}"


@task
def run(context: Context) -> None:
    """Run cli/ workspace tests (instrumented with coverage unless disabled).

    The instrumented run skips cargo-llvm-cov's full clean, which rebuilds
    every crate, and clears only the previous run's profile data: profiles
    left over from an earlier run would be merged into this run's totals.
    The totals are reported, not enforced. If instrumented binaries are
    suspected stale, run `cargo llvm-cov clean --workspace` and rerun.
    ACCELERATOR_COVERAGE=off drops to plain nextest for a faster loop.

    `--all-features` turns on ... (unchanged)

    The permission tests refuse to run as root; ... (unchanged)
    """
    _announce_skipped_permission_tests()
    if coverage_enabled():
        _run_instrumented(context)
    else:
        _run_tests(context, f"cargo nextest run {_MANIFEST}")


def _run_instrumented(context: Context) -> None:
    _clear_previous_profile_data(context)
    _run_tests(
        context, f"cargo llvm-cov nextest {_MANIFEST} --no-clean --no-report"
    )
    print(_report_totals(context).summary_line())


def _clear_previous_profile_data(context: Context) -> None:
    result = context.run(
        f"cargo llvm-cov clean --profraw-only {_WORKSPACE}",
        warn=True,
        pty=False,
    )
    if result.exited != 0:
        raise Exit("cargo llvm-cov: clearing profile data failed", code=1)


def _run_tests(context: Context, command: str) -> None:
    result = context.run(command, warn=True, pty=False)
    if result.exited != 0:
        raise Exit("nextest: cli tests failed", code=1)


def _report_totals(context: Context) -> CoverageTotals:
    result = context.run(
        f"cargo llvm-cov report {_WORKSPACE} {_FEATURES} "
        "--json --summary-only",
        warn=True,
        hide="out",
        pty=False,
    )
    if result.exited != 0:
        raise Exit("cargo llvm-cov: coverage report failed", code=1)
    try:
        return parse_totals(result.stdout)
    except CoverageReportError as error:
        raise Exit(f"cargo llvm-cov: {error}", code=1) from error
```

`_announce_skipped_permission_tests` is the current permission-skip banner,
extracted as is. `report` takes `_FEATURES` so it resolves the same build the
tests ran. The sequence test pins this, and the totals-equality check under
Manual Verification confirms it. The docstring must not contain the
substring `--profile`.

#### 4. Descriptions

**File**: `mise.toml` (`test:unit:cli` description)

```toml
description = "Run cli/ workspace tests via nextest with coverage (clears old profile data, prints a totals line; ACCELERATOR_COVERAGE=off drops to plain nextest)"
```

**File**: `tasks/README.md:778`: describe `on` as "profraw-only clean, then
`cargo llvm-cov nextest --no-clean`, then a totals line", keeping the
off-path wording.

**File**: `tasks/shared/rust.py:44` (`coverage_enabled` docstring): replace
"`cargo llvm-cov nextest` (coverage reported)" with "instrumented under
cargo-llvm-cov (see `test:unit:cli`)", so it no longer restates the lane's
steps.

**File**: `meta/work/0363-remove-repeated-work-from-the-rust-test-lanes.md`:
correct the Technical Note on the export shape. The totals sit at
`data[0].totals`, beside per-file summaries.

### Success Criteria:

#### Automated Verification:

- [ ] The parser and task tests pass:
      `uv run pytest tests/unit/tasks/test_coverage.py tests/unit/tasks/test_rust.py tests/unit/tasks/test_cli_permission_skip.py tests/unit/tasks/test_nextest_filter.py -v`
- [ ] The build-system checks pass: `mise run build-system:check`
- [ ] The full local run passes: `mise run`

#### Manual Verification:

On engywook:

- [ ] Warm and alone, `time mise run --skip-deps test:unit:cli` takes under
      180 s (median of three) and reports the same number of tests run, all
      passing, as the before run.
- [ ] After a full lane run, the clean, nextest and report commands are each
      timed on their own. The figures are recorded in the PR description,
      with the profraw-only clean's cost measured against a full run's
      leftover profiles.
- [ ] Its totals line equals the line from the same lane run straight after
      `cargo llvm-cov clean --workspace`.
- [ ] Stale binaries: after a full lane run, bump one dependency of a
      workspace member in `cli/Cargo.lock` (`cargo update -p <crate>
      --precise <adjacent version>`) so that member's test binaries get new
      hashes, then rerun the lane. Its totals line equals the line from a run
      straight after `cargo llvm-cov clean --workspace` on the same lockfile.
      Its report step takes within a second of the earlier timing.
      `du -sh cli/target/llvm-cov-target` is recorded before and after.
      Restore the lockfile afterwards. If the totals differ, stop and revisit
      the "no fingerprint-triggered clean" decision before Phase 3.
- [ ] After a full lane run, these restricted commands are run by hand. The
      nextest command takes `-p kernel` in place of `--workspace --exclude`,
      because `--workspace` would override `-p`:

      ```sh
      cargo llvm-cov clean --profraw-only --manifest-path cli/Cargo.toml
      cargo llvm-cov nextest --manifest-path cli/Cargo.toml -p kernel \
        --all-features --no-clean --no-report
      cargo llvm-cov report --manifest-path cli/Cargo.toml -p kernel \
        --all-features --json --summary-only
      ```

      The `kernel` totals equal those of the same commands run straight
      after `cargo llvm-cov clean --workspace`. Control: after a fresh full
      run, running the restricted nextest and report with bare `--no-clean`
      and no profraw clean gives different `kernel` totals.
- [ ] After two consecutive lane runs, `cli/target/llvm-cov-target` holds at
      least one `*.profraw` file, and every one has a modification time
      later than the second run's start.
- [ ] `ACCELERATOR_COVERAGE=off mise run --skip-deps test:unit:cli` runs no
      `cargo llvm-cov` command and passes the same number of tests.
- [ ] A full `mise run` prints one `coverage:` line, with no per-file rows
      and no JSON.
- [ ] In CI, the `test-unit` job on both `ubuntu-latest` and `macos-latest`
      runs the three `cargo llvm-cov` steps and prints the totals line. The
      report's stderr has no mismatched-data or missing-object warnings.
      Record both legs' lines for Phase 3.

---

## Phase 3: Coverage floor

### Overview

Fail `test:unit:cli` when any of the line, function or region totals falls
below its floor. The starting floors are met, with at least a percentage
point of headroom, by both CI legs, a local run and a root run that skips the
permission tests. They can be raised over time.

### Changes Required:

#### 1. Choose the starting floors

Collect each metric's total from four runs:

- the two CI legs recorded at the end of Phase 2;
- a warm local run on engywook;
- a run as root in a disposable x86_64 Linux container (matching
  `ubuntu-latest`), so that root-owned artefacts never land in the working
  `cli/target`. Inside it, `mise install` provisions the repo's pinned
  toolchain, then
  `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS=1 mise run --skip-deps test:unit:cli`
  runs the lane.

For each metric, take the minimum of the four, subtract one percentage point,
and round down to a whole percent. For example, a minimum of 77.99% gives
a floor of 76. Record the four totals lines and the resulting floors in the
PR description. Beside each line, record its OS, architecture and
`cargo llvm-cov --version`.

#### 2. Floor judgement (tests first)

**File**: `tests/unit/tasks/test_coverage.py`
**Changes**:

- A floor of `{LINES: 70, FUNCTIONS: 80, REGIONS: 60}` has no shortfalls
  when every total is above its floor, and none when a total exactly equals
  its floor.
- Parametrised over each `Metric`, with those distinct floors: dropping only
  that metric below its own floor gives exactly one shortfall, naming that
  metric, its total and its floor.
- All three below gives three shortfalls, in `Metric` order.
- `str(Shortfall(Metric.LINES, 69.5, 70))` is `lines 69.50% (floor 70%)`.
- A floor missing a metric raises `ValueError` naming it. A floor below 0 or
  above 100 raises `ValueError` naming the metric.

**File**: `tasks/shared/coverage.py`

```python
@dataclasses.dataclass(frozen=True)
class Shortfall:
    metric: Metric
    total: float
    floor: float

    def __str__(self) -> str:
        return f"{self.metric} {self.total:.2f}% (floor {self.floor:g}%)"


@dataclasses.dataclass(frozen=True)
class CoverageFloor:
    percents: Mapping[Metric, float]

    def __post_init__(self) -> None:
        _require_every_metric(self.percents)
        impossible = [m for m, p in self.percents.items() if not 0 <= p <= 100]
        if impossible:
            raise ValueError(f"floor outside 0-100 for {', '.join(impossible)}")

    def shortfalls(self, totals: CoverageTotals) -> list[Shortfall]:
        return [
            Shortfall(metric, totals.percents[metric], self.percents[metric])
            for metric in Metric
            if totals.percents[metric] < self.percents[metric]
        ]
```

Because `parse_totals` already rounds to two decimals, a total that prints as
`77.00%` meets a floor of 77.

#### 3. Enforce the floor in the task (tests first)

**File**: `tests/unit/tasks/test_rust.py`
**Changes**: In `TestTestUnitCli`, the class fixture also monkeypatches
`test_cli.COVERAGE_FLOOR` to a floor of zero for every metric. The Phase 2
tests then never depend on the production floor values, and raising a floor
later breaks none of them. Only the floor tests override it:

- `test_totals_below_the_floor_fail_the_task`: monkeypatch the floor above
  `EXPORT`'s totals for one metric. `Exit` is raised, and its message names
  that metric, `COVERAGE_FLOOR` and `cargo llvm-cov clean --workspace`. The
  totals line is still printed first.
- `test_totals_at_the_floor_pass`: monkeypatch the floor to exactly
  `EXPORT`'s parsed totals. No `Exit` is raised.
- Keep `test_carries_no_coverage_threshold`, renamed
  `test_passes_no_threshold_flag_to_cargo_llvm_cov`, to pin that the floor
  is applied in Python and not with `--fail-under-*`.

**File**: `tasks/test/cli.py`

```python
COVERAGE_FLOOR = CoverageFloor(
    {Metric.LINES: ..., Metric.FUNCTIONS: ..., Metric.REGIONS: ...}
)
```

`COVERAGE_FLOOR` is built at import, so a floor that misses a metric, is
left at a placeholder or is out of range fails every test that imports the
task. The production value is checked without a test of its own.

`_run_instrumented` gains one step:

```python
def _run_instrumented(context: Context) -> None:
    _clear_previous_profile_data(context)
    _run_tests(
        context, f"cargo llvm-cov nextest {_MANIFEST} --no-clean --no-report"
    )
    totals = _report_totals(context)
    print(totals.summary_line())
    _enforce_floor(totals)


def _enforce_floor(totals: CoverageTotals) -> None:
    shortfalls = COVERAGE_FLOOR.shortfalls(totals)
    if shortfalls:
        raise Exit(
            "coverage below COVERAGE_FLOOR in tasks/test/cli.py: "
            + ", ".join(map(str, shortfalls))
            + ". If nothing that affects coverage changed, run "
            "`cargo llvm-cov clean --workspace` and rerun.",
            code=1,
        )
```

Docstring: replace Phase 2's sentence "The totals are reported, not
enforced." with:

> The totals must meet COVERAGE_FLOOR. Raise a floor as coverage improves;
> never lower one to get a change through, except to reset the floors after
> a Rust toolchain, llvm-tools or cargo-llvm-cov bump, by the rule in
> tasks/README.md.

**File**: `tasks/README.md`: add a "Coverage floor" subsection after
"Permission tests under root". It covers four points:

- `COVERAGE_FLOOR` in `tasks/test/cli.py` gates `test:unit:cli` whenever
  coverage is on.
- The rule for setting the floors: the minimum of both CI legs, a local run
  and a root skip-permission run, less one point, rounded down. Include the
  root run's recipe: a disposable x86_64 Linux container, `mise install`,
  then
  `ACCELERATOR_TEST_SKIP_PERMISSION_TESTS=1 mise run --skip-deps test:unit:cli`.
  Record each run's OS, architecture and `cargo llvm-cov --version`.
- The ratchet: floors only go up. The one exception is resetting them by the
  same rule after a Rust toolchain, llvm-tools or cargo-llvm-cov bump.
- If the floor trips with no relevant change, the first step is
  `cargo llvm-cov clean --workspace`.

The `ACCELERATOR_COVERAGE` row stays about the switch.

**File**: `mise.toml`: add "fails below COVERAGE_FLOOR" to the
`test:unit:cli` description.

**File**: `meta/work/0363-remove-repeated-work-from-the-rust-test-lanes.md`:
add a note that the coverage floor was added beyond the item's original
scope, at the user's request. It reverses the "No coverage gating" decision
in `meta/plans/2026-07-02-0162-rust-toolchain-guard-rails.md`.

### Success Criteria:

#### Automated Verification:

- [ ] The tests pass:
      `uv run pytest tests/unit/tasks/test_coverage.py tests/unit/tasks/test_rust.py -v`
- [ ] The build-system checks pass: `mise run build-system:check`
- [ ] The full local run passes: `mise run`
- [ ] Both CI `test-unit` legs pass with the floor in force.

#### Manual Verification:

- [ ] Temporarily raising one floor above the local total makes
      `mise run --skip-deps test:unit:cli` fail with a message naming that
      metric, after printing the totals line. Revert the change afterwards.

---

## Testing Strategy

### Unit Tests:

- `tests/unit/tasks/test_coverage.py`: parsing a trimmed real 0.8.7 export
  (per-file entries ignored), two-decimal rounding and formatting, non-JSON
  and malformed exports, and floor judgement at, above and below each
  metric's own floor.
- `tests/unit/tasks/test_rust.py::TestTestUnitCli`: command order, flags and
  shared manifest and feature fragments, capture only on the report, early
  stops on each failing step, a failing off-path run, the docstring remedy,
  and floor enforcement isolated from the production floor values.
- `tests/unit/tasks/test_rust.py::TestVisualiserHasNoDependents`: no member
  depends on the visualiser in the resolved `cargo metadata`, the guard
  catches each dependency kind, and it ignores other dependencies.
- `log::tests`: literal-value assertions, checked against the four mutations.

### Integration Tests:

- `mise run` end-to-end on engywook, and the CI `test-unit` matrix on both
  operating systems.

### Manual Testing Steps:

1. Phase 1 timing and mutation checks (above).
2. Phase 2 timing (whole lane and each step), totals-equality, `kernel`
   control, stale-binary dependency-bump and profraw-freshness checks
   (above).
3. Phase 3 floor selection across the four runs, and the deliberate-shortfall
   check (above).

## Performance Considerations

- The coverage lane is expected to drop from about 247 s to about 160 s warm.
  The target is under 180 s.
- `log::tests` is expected to drop from about 42 sys-CPU-s per lib pass to
  under 2 CPU-s. It runs twice in `test:unit:visualiser`. libtest runs the
  tests as threads, so the lane's wall-clock saving is smaller than its CPU
  saving.
- The profraw-only clean deletes about 9,200 profile files left by the
  previous run. Its cost is timed on its own in Phase 2.
- `--no-clean` keeps instrumented binaries across runs. Phase 2's
  dependency-bump check confirms that superseded binaries neither change the
  totals nor slow the report.
- The report step adds a JSON export of a few MB that is captured in memory.
  Its cost is the same as today's report (about 4 s).

## Migration Notes

- `cli/target/llvm-cov-target` holds about 10,288 leftover `*.profraw` files.
  The first profraw-only clean removes them. Measurements of the stale-profile
  control need a deliberately seeded, different, wider prior run.
- Rollback: each phase is a self-contained revert.
- 0365's timings must be taken after Phases 1 and 2 have landed. 0370 counts
  this item as landed once both halves (Phase 1, and Phases 2 and 3) have
  landed or been abandoned.

## References

- Original work item: `meta/work/0363-remove-repeated-work-from-the-rust-test-lanes.md`
- Codebase research: `meta/research/codebase/2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes.md`
- Source research: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Baseline: `meta/measurements/build_2026-11-01T143523Z_engywook_warm-no-op_devtools.json`
- Capture-and-parse pattern: `tasks/docs.py:53-58`, `tasks/shared/npm_audit.py:38-55`
- Crate-dependency lint gaps: `tasks/lint/crate_dependencies.py:319-329`
  (`edges`) and `:369-401` (`broken_rules`)
- Resolved dependency graph: `tasks/shared/cargo_metadata.py`
