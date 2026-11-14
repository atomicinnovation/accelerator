---
type: "codebase-research"
id: "2026-11-13-0363-remove-repeated-work-from-the-rust-test-lanes"
title: "Remove repeated work from the Rust test lanes"
date: "2026-11-13T16:26:11+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0363"
parent: "work-item:0363"
relates_to: ["codebase-research:2026-10-28-full-build-duration-contention-and-slowness"]
topic: "Remove repeated work from the Rust test lanes"
tags: ["research", "codebase", "build-system", "testing", "rust", "coverage", "cargo-llvm-cov", "visualiser", "file-rotate"]
revision: "51f325d8134cc3c32de3722304fd538f1d3c48b4"
repository: "accelerator"
last_updated: "2026-11-13T16:26:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Remove repeated work from the Rust test lanes

**Date**: 2026-11-13T16:26:11+00:00
**Author**: Toby Clemson
**Git Commit**: 51f325d8134cc3c32de3722304fd538f1d3c48b4
**Branch**: detached working copy `omqrqontxkwp` (build-system workspace)
**Repository**: accelerator

## Research Question

What does the codebase look like where work item 0363 will land, and does it
hold up against the item's premises? Work item 0363 replaces the coverage
lane's full clean with profraw-only clean, then `--no-clean --no-report`, then
`report --json --summary-only`, printing a totals line. It also rewrites the
visualiser `log::tests` rotation tests to use a few large writes.

## Summary

Both halves are small, contained changes, and the premises mostly hold. Five
findings matter for planning, and three of them correct or sharpen the work
item:

1. **`--summary-only` JSON is not totals-only.** The work item's Technical
   Notes say `report --json --summary-only` "emits the export's `totals`
   object only". Verified against the local profile data, it actually emits
   `{data: [{files: [...], totals: {...}}], type, version}`. Every file still
   has a `summary` entry; only the per-line segments are dropped. The totals
   live at `data[0].totals.{lines,functions,regions}.percent`. The task must
   therefore capture stdout (not stream it) and pick out that path.
2. **The visualiser lane runs `cargo test`, not nextest.** `test:unit:visualiser`
   runs `cargo test --lib` twice (`tasks/test/unit.py:23-27`), so every
   `log::tests` test shares one libtest process. The work item's CPU-time
   criterion times `cargo nextest run -p accelerator-visualiser --lib …`
   instead. That works, and the filter is unambiguous, but it is not the
   command the lane runs, and cargo and nextest start-up count against the
   2 CPU-s budget.
3. **The existing rotation tests cannot detect `MAX_FILES` mutations.** They
   assert `rotated <= MAX_FILES` by name, and the retention test makes five
   rotations. So `MAX_FILES = 2` and `MAX_FILES = 4` both still pass. The
   literal-value assertions in Requirement 3 are therefore needed, not
   cosmetic.
4. **file-rotate 0.7.6 splits at exactly the cap and loops across several
   caps in one write.** That confirms the premise of Requirement 3: one write
   of N × 5 MiB produces N rotations of exactly 5 MiB each. One subtlety:
   rotation fires only when a write would go *past* the cap (`>`). A write
   that lands exactly on 5 MiB does not rotate until the next non-empty
   write.
5. **`cargo llvm-cov report` in 0.8.7 has no `--workspace`/`--exclude`.** The
   current single command excludes `accelerator-visualiser` from both the
   tests and the report. The split `report` step cannot repeat that
   exclusion. Today it doesn't matter: no workspace member depends on the
   visualiser, and `cli/target/llvm-cov-target` holds no visualiser objects.
   The equivalence rests on that dependency fact.

Existing test guards also shape the change: `tests/unit/tasks/test_rust.py`
pins the command's shape, `test_nextest_filter.py` bans the substrings
`--profile`/`--ignore-default-filter` anywhere in `tasks/test/cli.py`, and
`test_cli_permission_skip.py` uses a single-result `MockContext`.

## Detailed Findings

### Coverage lane: current implementation

- The task body (`tasks/test/cli.py:38-50`) builds one string and runs it
  once with `context.run(command, warn=True, pty=False)`. It never reads the
  output; it only checks `result.exited` and raises
  `Exit("nextest: cli tests failed", code=1)` on failure.
  - Coverage on: `cargo llvm-cov nextest {_MANIFEST} --summary-only`
  - Coverage off: `cargo nextest run {_MANIFEST}`
- `_MANIFEST` (`tasks/test/cli.py:13-16`) =
  `--manifest-path <abs>/cli/Cargo.toml --workspace --exclude
  accelerator-visualiser --all-features`. The path is absolute
  (`tasks/shared/paths.py:10,18,25`) and there is no `context.cd`. Because
  `tasks/test/integration.py:8,94-101` (`tracker_contract`) also imports
  `_MANIFEST`, the new `clean` and `report` commands should not be folded
  into it.
- `coverage_enabled()` (`tasks/shared/rust.py:37,41-52`) reads
  `ACCELERATOR_COVERAGE` at call time. It defaults to `on`; only
  `off`/`false`/`0`/`no` switch coverage off. `tasks/test/cli.py:45` is its
  only caller.
- The docstring (`tasks/test/cli.py:22-37`) describes coverage as "folded
  into the test run", report-only with no `--fail-under`, plus the
  `--all-features` and permission-skip notes. Requirement 4's
  `cargo llvm-cov clean --workspace` remedy goes here. The phrase "the default
  instrumented `cargo llvm-cov nextest` pass both runs the tests and reports
  coverage in one go" stops being true once the steps are split.
- The mise description (`mise.toml:333-336`) also says "coverage folded in".
  The task has no `sources`/`outputs`/`env` and depends on
  `deps:install:rust-components`. That task installs `llvm-tools-preview` up
  front so concurrent `cargo llvm-cov` runs don't race on `~/.rustup`
  (`tasks/deps.py:33-43`).
- Tool pins: `mise.toml:30-31` pins `cargo-nextest` 0.9.138 and
  `cargo-llvm-cov` 0.8.7, hash-locked in `mise.lock:70-90`. The Rust toolchain
  is 1.90.0 (`mise.toml:8`).
- No llvm-cov configuration exists anywhere (no `ignore-filename-regex`, no
  `CARGO_LLVM_COV_TARGET_DIR`). Profiles land in the default
  `cli/target/llvm-cov-target`.
- `cli/.config/nextest.toml:1-8` has only `default-filter` entries: the
  default profile excludes `binary(=contract)`. There is no slow-timeout,
  retry or test-group setting.

### Coverage lane: cargo-llvm-cov 0.8.7 behaviour (verified locally)

- `cargo llvm-cov clean` accepts `--profraw-only`, `--workspace` and
  `--manifest-path`. It has no package selection.
- `cargo llvm-cov` (and so `nextest`) accepts `--no-report`, `--no-clean`,
  `--workspace`, `--exclude`, `--exclude-from-test` and
  `--exclude-from-report`.
- `cargo llvm-cov report` accepts `--json`, `--summary-only`,
  `--output-path`, `--manifest-path`, `-p/--package`, `--all-features`,
  `--ignore-filename-regex` and the `--fail-under-*` family. It has **no**
  `--workspace`, `--exclude` or `--exclude-from-report`.
- The `report` help says `--summary-only` means "Export only summary
  information **for each file**" and is valid only with `--json`, `--lcov`
  or `--cobertura`. This matches the research's finding that text mode
  ignores it.
- Export shape, checked by running `llvm-cov export -format=text
  -summary-only` against `cli/target/llvm-cov-target/cli.profdata` and the
  `kernel` test binary, with the output written to the scratchpad so the
  target directory was not touched:
  ```json
  {"type": "llvm.coverage.json.export", "version": "2.0.1",
   "data": [{"files": [{"filename": "...", "summary": {...}}, ...],
             "totals": {"lines": {"percent": 77.9874...},
                        "functions": {"percent": 84.375},
                        "regions": {"percent": 75.5905...},
                        "branches": ..., "instantiations": ..., "mcdc": ...}}]}
  ```
  For `kernel`, lines read 77.99%, matching the research's clean-run figure.
  Functions read 84.38%, also matching the research's clean run.
  - The work item's example line uses `functions 81.20%, regions 74.05%`.
    Those are illustrative numbers, not this export's values; a test fixture
    should use its own known values.
- Nothing in the repository depends on `accelerator-visualiser`. No
  `Cargo.toml` names it, and `cli/target/llvm-cov-target/debug/deps` contains
  no visualiser artefacts. So an unfiltered `report` covers the same files as
  today's `--exclude`d combined command.
- `cli/target/llvm-cov-target` currently holds **10,288 `*.profraw` files**
  and a 4.8 MB `cli.profdata`. The research measured about 9,200 per full run,
  so this directory holds leftovers from earlier experiments. Treat the
  existing state as dirty before taking any "warm" measurement.

### Coverage lane: output handling patterns to follow

- **Streamed run, then parse stdout:** `tasks/test/unit.py:140-161` runs
  node `--test-reporter=tap` with `warn=True, pty=False`, then
  `_tap_counts(result.stdout)`. The nextest step can keep streaming.
- **Hidden capture, then JSON parse via a pure shared helper:**
  `tasks/docs.py:53-58` captures `npm audit --json` with `hide="out"` and
  passes it to `parse_report` in `tasks/shared/npm_audit.py:38-45`. That
  helper has its own unit tests (`tests/unit/tasks/test_npm_audit.py`), and
  the task-level tests are in `tests/unit/tasks/test_docs_audit_check.py:13-58`.
  This is the closest model for the report step: capture with `hide="out"`,
  then format the totals line through a pure function that the
  formatting criterion can test directly.
- **subprocess + JSON:** `tasks/shared/cargo_metadata.py:12-29`.

### Coverage lane: existing tests that will need updating

- `tests/unit/tasks/test_rust.py:14-18`: the `ctx` fixture is
  `MagicMock(spec=Context)`, with every `run` returning
  `MagicMock(exited=0, stdout="")`. An empty stdout would break a JSON parse
  in the report step.
- `TestTestUnitCli` (`tests/unit/tasks/test_rust.py:104-139`) reads only
  `ctx.run.call_args.args[0]`, i.e. the **last** call:
  - `test_instrumented_by_default` asserts it starts with
    `cargo llvm-cov nextest` and contains `--summary-only`. After the split,
    the last call is `report`, so this needs rewriting around the
    three-call sequence.
  - `test_plain_nextest_when_coverage_off`,
    `test_carries_no_coverage_threshold` and
    `test_raises_when_inner_tests_fail` also need checking. The last asserts
    `Exit` on `exited=1`; decide whether a failing nextest step should skip
    the report.
- `tests/unit/tasks/test_cli_permission_skip.py:8-38` uses invoke's
  `MockContext(run=Result(exited=0))` with `ACCELERATOR_COVERAGE=off`, so it
  makes only one call. It is unaffected unless the off-path changes.
- `tests/unit/tasks/test_nextest_filter.py:37-47` fails if `--profile` or
  `--ignore-default-filter` appears anywhere in `tasks/test/cli.py`,
  docstrings and comments included. `--profraw-only` does not contain
  `--profile`, but any wording near it must avoid that substring.
- `tests/unit/tasks/fixtures/build-measurement/warm-no-op-tasks.golden.json:4970`
  contains the literal string `invoke test.cli.run`. This fixture is
  unaffected as long as the mise `run` line stays the same.

### CI

- The `test-unit` job (`.github/workflows/main.yml:15-61`) runs on an
  `ubuntu-latest`/`macos-latest` matrix with `fail-fast: false`. It runs
  `mise run test:unit` (the whole roll-up, `main.yml:60-61`), not
  `test:unit:cli` on its own.
  - `ACCELERATOR_COVERAGE` is never set, so coverage is on in CI.
  - `tasks/README.md:769-778` says CI ignores contributor variables.
- The `Swatinem/rust-cache` step (`main.yml:43-50`) sets only
  `workspaces: cli`, with no key, shared-key, cache-targets or
  cache-directories. Whether `cli/target/llvm-cov-target` is restored depends
  on rust-cache's default handling of `cli/target`. Nothing in the repo pins
  this down, which is why the item's Dependencies section flags it.
- No workflow consumes coverage output: no artifact upload, no codecov, no
  log parsing. This confirms the item's assumption.

### Log-rotation tests

- Constants and writer (`cli/visualiser/server/src/log.rs:19-43`):
  - `MAX_BYTES = 5 * 1024 * 1024` and `MAX_FILES = 3`, both `pub(crate)`.
  - `make_writer` calls `FileRotate::new(path, AppendCount::new(MAX_FILES),
    ContentLimit::Bytes(MAX_BYTES), Compression::None, Some(0o600))`, with
    the mode only under `#[cfg(unix)]`. It creates the parent directory
    first and maps a failure to `LoggingError::CreateDir`, because
    file-rotate would otherwise panic (`file-rotate lib.rs:432`).
- Tests in `log::tests` (`log.rs:103-243`):
  - `rotates_when_active_segment_crosses_5_megabytes` (`:121-146`): writes
    `b"x\n"` 2,654,208 times (5 MiB + 64 KiB). It asserts exactly one rotated
    segment and an active file under `MAX_BYTES`. It never checks the rotated
    segment's size.
  - `retains_at_most_three_rotated_segments` (`:148-169`): five rounds of
    5 MiB + 64 KiB in 2-byte writes (about 13.3 M writes, 5 rotations). It
    asserts `rotated <= MAX_FILES`. This is the 22.3 s test.
  - `active_and_rotated_segments_are_owner_only` (`:171-198`, Unix only):
    the same volume as the first test. It asserts `mode & 0o777 == 0o600`
    for every file whose name starts with `server.log`.
  - Already cheap, untouched by 0363:
    `make_writer_errors_when_parent_path_is_a_file` (`:200-213`),
    `pre_existing_oversized_log_rotates_on_first_write` (`:215-225`, which
    already uses a single large `fs::write`), and
    `emits_json_lines_with_message_and_field` (`:227-242`).
  - Helper `count_segments` (`:110-119`) counts names starting with
    `server.log.`, i.e. rotated segments only.
- file-rotate 0.7.6 (`cli/Cargo.lock:1307-1310`; source in
  `~/.cargo/registry/src/index.crates.io-*/file-rotate-0.7.6/src/lib.rs`):
  - `Write::write`, `ContentLimit::Bytes` branch (`lib.rs:637-654`):
    `while self.count + buf.len() > bytes { write buf[..bytes_left]; rotate;
    buf = &buf[bytes_left..] }`. So a single buffer is split exactly at the
    cap, and buffers spanning several caps rotate several times. `write`
    always returns `Ok(buf.len())`.
  - `rotate` (`lib.rs:573-589`) renames the active file through a cascading
    `move_file_with_suffix` (`lib.rs:517-569`, so `.1` → `.2` → …), reopens
    the file, resets `count = 0`, then prunes.
  - Pruning: `AppendCount::too_old` is `file_number >= max_files`
    (`suffix.rs:138-140`). With 3, the files `server.log`, `.1`, `.2` and
    `.3` survive, and `.4` is deleted (`lib.rs:590-607`).
  - Permissions come only from `OpenOptions::mode` on create
    (`lib.rs:486-497`), subject to umask. Rotated files keep their mode
    through `fs::rename`. There is no explicit chmod.
  - At open time, `count` is seeded from the existing file's length
    (`lib.rs:464-469`).
- What this means for the rewrite:
  - **Exact segment size.** Every non-final segment is exactly 5,242,880
    bytes whatever the chunking, because the split happens inside the write.
  - **A single write that crosses the cap.** A buffer such as
    `vec![b'x'; 5 MiB + 64 KiB]` written once rotates once. The active file
    is left at 64 KiB.
  - **Retention.** One buffer of 5 × 5 MiB + δ makes five rotations, so
    `==` 3 rotated segments holds. Any count of four or more rotations
    distinguishes `MAX_FILES = 3` from 4. For example, four rotations leave
    three segments under `MAX_FILES = 3` and four under `MAX_FILES = 4`.
  - **Mutation coverage**, with literal assertions:
    - `MAX_BYTES / 2`: segment size ≠ 5,242,880.
    - `MAX_BYTES * 2`: a 5 MiB + 64 KiB write doesn't rotate, so the segment
      count is 0 and the exact-size assertion fails.
    - `MAX_FILES = 2`: 2 ≠ 3.
    - `MAX_FILES = 4`: 4 ≠ 3, provided there are at least four rotations.
  - **Boundary trap.** A chunk sequence whose total lands exactly on a cap
    multiple does not rotate on that last write.
- Production path: `main.rs:108` calls `log::init`, which wraps the writer
  in `tracing_appender::non_blocking` behind a JSON `tracing_subscriber`.
  Each event arrives as one `write_all` of a whole JSON line
  (tracing-subscriber `fmt_layer.rs:1049-1050`; tracing-appender
  `worker.rs:32-44`). Even so, `ContentLimit::Bytes` splits a line that
  straddles the cap. That is existing production behaviour, out of scope.
- Features: `log.rs` has no feature `cfg`s, and `pub mod log;` at
  `lib.rs:31` is unconditional. So all six tests run in both lib-suite
  passes (`dev-frontend` and the default `embed-dist`). The fix halves the
  cost twice over.

### Visualiser test tasks

- `test:unit:visualiser` (`mise.toml:308-311` → `tasks/test/unit.py:23-27`)
  runs sequentially:
  1. `cargo test --manifest-path …/visualiser/server/Cargo.toml --lib
     --no-default-features --features dev-frontend`
  2. `cargo test … --lib` (the default `embed-dist`, which needs
     `frontend/dist/index.html`)

  Neither run uses `warn=True`, so a failure in the first skips the second.
- `test:integration:visualiser` (`tasks/test/integration.py:28-33`) now uses
  `--test '*'`, with a docstring at `:16-23` explaining why. That confirms
  A4 has shipped.
- In `nextest`, the filter `log::tests` matches only `log.rs` in the
  visualiser server crate; no other module there is named `*log`. Because
  `-p accelerator-visualiser` applies, the `log` modules in `cli/vcs` and
  `cli/vcs-cli` are not matched.

## Code References

- `tasks/test/cli.py:13-16`: `_MANIFEST`, also imported by `integration.py:8`.
- `tasks/test/cli.py:22-37`: task docstring (Requirement 4 target).
- `tasks/test/cli.py:38-50`: command selection and the single `context.run`.
- `tasks/shared/rust.py:37,41-52`: `_FALSEY`, `coverage_enabled()`.
- `tasks/shared/npm_audit.py:38-45` and `tasks/docs.py:53-58`: capture-and-parse JSON pattern.
- `tasks/test/unit.py:23-27`: visualiser lib suite, two `cargo test` runs.
- `tasks/test/unit.py:140-161`: streamed run plus stdout parse pattern.
- `tasks/test/integration.py:28-33`: `--test '*'` (A4 shipped).
- `tasks/deps.py:33-43`: `llvm-tools-preview` preinstall.
- `mise.toml:30-31`: nextest and llvm-cov pins.
- `mise.toml:333-336`: `test:unit:cli` definition and description.
- `mise.toml:347-349, 531-533, 792-794`: `test:unit`, `test`, `default` roll-ups.
- `tests/unit/tasks/test_rust.py:14-18,104-139`: command-shape tests to rewrite.
- `tests/unit/tasks/test_cli_permission_skip.py:8-38`: single-result `MockContext`.
- `tests/unit/tasks/test_nextest_filter.py:37-47`: forbidden-substring guard.
- `tests/unit/tasks/test_docs_audit_check.py:13-58`: model for JSON-stdout task tests.
- `cli/.config/nextest.toml:1-8`: default and contract filters.
- `.github/workflows/main.yml:15-61`: `test-unit` job, rust-cache at `:43-50`.
- `cli/visualiser/server/src/log.rs:19-20`: `MAX_BYTES`, `MAX_FILES`.
- `cli/visualiser/server/src/log.rs:22-43`: `make_writer`.
- `cli/visualiser/server/src/log.rs:110-198`: `count_segments` and the three slow tests.
- `cli/visualiser/server/src/log.rs:215-225`: existing single-write rotation test (model).
- `file-rotate-0.7.6/src/lib.rs:637-654`: byte-limit split loop.
- `file-rotate-0.7.6/src/lib.rs:573-607`: rotate and prune.
- `file-rotate-0.7.6/src/suffix.rs:138-140`: `AppendCount::too_old`.

## Architecture Insights

- **Coverage is a property of the test lane, not a separate task.** It
  follows ADR-0050's "a local run mirrors CI" and the 0162 plan, which keeps
  coverage in `test` rather than `check` because instrumentation recompiles
  the workspace. The three-step sequence keeps that shape inside a single
  invoke task.
- **Pure parsers live in `tasks/shared/` with their own unit tests.** The
  task body orchestrates `context.run` calls and checks exit codes. A totals
  formatter (taking the export text and returning the line) fits that split,
  and the formatting criterion can be tested without invoke.
- **Source-text guards** (`test_nextest_filter.py`) constrain wording in
  task files, not just behaviour.
- **file-rotate's `Bytes` limit is a hard byte cut**, not line-aware. Tests
  can drive it with arbitrary buffer sizes, and production can split JSON
  lines at the boundary.

## Historical Context

- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
  - Coverage-lane anatomy (L183-217): clean 61 s, recompile 30 s, nextest
    149 s, report 4 s, total 247 s. With `--no-clean` the lane took 160 s;
    with `ACCELERATOR_COVERAGE=off`, 152 s.
  - Stale-profraw proof on `kernel` (L638-654): 89.94% vs 77.99% vs 77.99%.
  - `--summary-only` is ineffective in text mode (L617-621).
  - The "default mirrors CI" follow-up re-scoped A2 (L687-701).
  - The log tests cost about 42 sys-CPU-s per lib pass (L284-307).
  - Measurement method and harness (L106-132, L759-897).
- `meta/measurements/build_2026-11-01T143523Z_engywook_warm-no-op_devtools.json`:
  baseline medians are `test:unit:cli` 303.48 s (nextest phase 148.18 s; it
  is the tail of `default`), `test:unit:visualiser` 61.82 s and wall-clock
  315.1 s. There is no per-phase or `log::tests` breakdown, so 0363's before
  figures must be measured fresh.
- `meta/plans/2026-07-02-0162-rust-toolchain-guard-rails.md` (Phase 5,
  L820-887) introduced the coverage lane. `--summary-only` came from
  luminosity, and the plan expected a "TOTAL line" (L877), so the intent was
  always a totals summary.
- `meta/plans/2026-04-28-meta-visualiser-phase-10-error-handling-accessibility-polish.md`
  (Phase 10.2, L678+) introduced `log.rs` and file-rotate. Its only
  rationale for the write volume is MAX_BYTES + 64 KiB "regardless of
  whether file-rotate counts pre- or post-write" (L769-772). Nothing records
  why the lines are 2 bytes.
- `meta/decisions/ADR-0050-mise-invoke-task-runner.md:35`: "a local run mirrors CI".
- `meta/work/0365-…`: its timings must come after both halves of 0363 land.
  `meta/work/0367-…`: its `dist/` rewrite recompiles the visualiser in the
  `embed-dist` pass, which could confound before and after timings of
  `test:unit:visualiser`. `meta/work/0368-…`: describes the `measure:build`
  tooling (`--isolate`, cache states and record comparability).

## Related Research

- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`

## Open Questions

1. **Report scope without `--exclude`.** Should the `report` step pass
   `--ignore-filename-regex` for `visualiser/`, so the totals stay
   independent of whether visualiser objects ever reach
   `llvm-cov-target`? An alternative is a test that pins the no-dependency
   fact. Today the two commands are equivalent only because nothing depends
   on the visualiser.
2. **Failure semantics.** If nextest fails, should the task still run
   `report`? It currently raises immediately; skipping the report on failure
   is the natural choice. Also, should a missing or malformed `totals` be an
   `Exit` or a warning? Coverage is report-only by design.
3. **The work item's note on the export shape.** Technical Notes say the
   export is "the export's `totals` object only". It should say the totals
   sit at `data[0].totals` beside per-file summaries.
4. **CPU-time criterion command.** The `log::tests` criterion uses nextest,
   but the lane uses `cargo test`. Should the before and after be measured
   with the lane's own `cargo test … --lib … log::tests` instead? Either
   way, cargo start-up (fingerprint checks) counts against the 2 CPU-s
   budget.
5. **Dirty profile state.** `cli/target/llvm-cov-target` holds 10,288
   profraw files from earlier experiments. The before timing for the
   coverage lane (a default full clean) is unaffected. The stale-profile
   control in the acceptance criteria needs a deliberately seeded,
   different, wider prior run, as the item already notes.
6. **Docstring and mise description wording.** Both say coverage is "folded
   into" one pass. Should the description change alongside the docstring
   once there are three steps?
