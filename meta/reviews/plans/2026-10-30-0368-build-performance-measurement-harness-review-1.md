---
type: "plan-review"
id: "2026-10-30-0368-build-performance-measurement-harness-review-1"
title: "Plan Review: Build-Performance Measurement Harness and Host Guidance Implementation Plan"
date: "2026-10-30T17:50:02+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-30-0368-build-performance-measurement-harness"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "usability", "standards", "documentation"]
review_number: 1
review_pass: 4
tags: ["build-system", "measurement", "performance", "mise", "jj", "macos", "tasks"]
last_updated: "2026-10-31T14:47:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Build-Performance Measurement Harness and Host Guidance Implementation Plan

**Verdict:** REVISE

The plan carries the measure namespace's pure-core, injected-shell shape into
a well-partitioned package, with a clean `History` port, one shared timeline
epoch, rich `StrEnum` vocabulary and phasing that follows the sibling gates.
Its analysis rules match mise's probed behaviour. The weaknesses sit at the
boundaries. The text-mode pipe breaks the `\r`-joined lines the parser is
built for. Raw logs exist only in memory until the whole session succeeds.
The record has a writer but no typed reader, the tracked-process set is
defined in two places, and interrupt cleanup can leave a detached build
running. Several acceptance criteria are asserted at layers that cannot
observe them. No finding is critical, but 18 major findings remain after
deduplication, well past the threshold of 3.

### Cross-Cutting Themes

- **Logs held in memory until the session succeeds** (flagged by:
  architecture, code-quality, usability, safety). `write_measured_build`
  writes every log only after `measure_build` returns. A parser surprise,
  a late `HistoryError` or Ctrl-C in run 3 discards up to 25 minutes of
  evidence. Golden capture becomes chicken-and-egg: the first real run is
  the one most likely to hit an unparsed shape.
- **The record has a writer but no typed reader** (flagged by:
  architecture, code-quality, standards, test-coverage, documentation,
  usability). `record_document` writes `dict[str, object]` and
  `compare_records` reads `Mapping[str, object]`. Nothing parses,
  validates `schema_version` or round-trips the shape, yet 0361, 0365 and
  0366 read these records for months.
- **Two sources of truth for tracked processes** (flagged by:
  code-quality, architecture, test-coverage, correctness, usability).
  `real_ports` hard-codes the `ps` filter from `DEFAULT_TRACKED_PROCESSES`,
  while `BuildSettings.tracked` drives analysis. `--ide-pattern` has no
  seam to swap both. The anchored `^/usr/libexec/syspolicyd` pattern also
  fails unless the filter isolates the command field, and the filter has
  no test.
- **Interrupt cleanup is incomplete** (flagged by: safety, correctness,
  architecture, code-quality, test-coverage). Samplers stop on the happy
  path only, and the `PeriodicSampler` thread is not a daemon. A second
  signal during the reap skips SIGKILL, and a one-shot `children()`
  snapshot misses tasks mise spawns as their dependencies die. The
  interrupt test pairs a real child with `FakeProcs`, so it never signals
  anything real.
- **Domain rules and side effects sit outside the ports** (flagged by:
  architecture, code-quality, safety, test-coverage). Several pieces live
  in `tasks/measure.py`, which hard-codes `REPO_ROOT`: the `--cold` with
  `--isolate` refusal, `--isolate` name validation, `_history` selection
  and cache-state `rmtree`. The task-layer refusal tests are therefore
  vacuous: they assert that `tmp_path` survives a task that never touches
  it. A regression could start a real `mise run default` and delete the
  checkout's `cli/target`.
- **Isolation mode is bolted on** (flagged by: code-quality, architecture,
  correctness). `analyse_timeline` has no readiness-origin parameter, so
  every isolated task after the first absorbs earlier invocations into its
  queueing delay. `Mode` and `BuildSettings.isolate` are also redundant,
  and `cpu_buckets` takes a single `Timeline`.
- **Progress output has no port** (flagged by: usability, architecture,
  code-quality). Manual verification expects `priming…` and `timed run
  1/1` lines, but no step designs them and no test demands them.
- **Later modes ship undocumented** (flagged by: documentation, usability).
  Phases 6, 7 and 9 add `--isolate`, `--incremental`, `--cold` and
  `measure:compare-builds` without touching the README, docstring or
  `mise.toml` description that Phase 4 wrote for warm full runs only.

### Tradeoff Analysis

- **Preflight strictness vs measurement flexibility.** Usability wants an
  operator who forgot `--host-settings`, or left a stray edit, to find out
  before 20 minutes are spent. Requirement 7 and the consumers legitimately
  measure dirty trees. Recommendation: print a preflight block with
  warnings, and optionally add a `--baseline` strict mode that refuses.
- **`--keep` on fetch failures.** Withholding the kept copy honours
  Requirement 7 ("retake rather than keep") but contradicts Requirement 5's
  "`--keep` also writes". Recommendation: withhold and print `not kept: N
  fetch failures`. The run directory still holds the record.
- **Deduplicating helpers vs the plan's "not changing warm-dispatch"
  scope.** Architecture, code quality and standards want
  `repository_kind`, `ambient_diagnostic_runner`, the reaper and signal
  unwinding moved into `tasks/shared/`. That touches `tasks/measure.py`
  imports but not warm-dispatch behaviour. Recommendation: move them; the
  plan's exclusion targets behaviour and record formats, not import
  locations.
- **A single `--cache-state` enum vs an explicit destructive flag.**
  Usability prefers `--cache-state {warm-no-op,incremental,cold}`.
  Requirement 4 asks for cold "behind an explicit flag because it is
  destructive", and `--cache-state cold` still satisfies that.
  Recommendation: either works; state the choice.

### Findings

#### Major

- 🟡 **Correctness**: Text-mode pipes split cargo's `\r` progress segments
  away from their task prefix
  **Location**: Phase 4 §3: Adapters and wiring (`SubprocessBuild`)
  `Popen(text=True)` opens stdout with universal newlines, so
  `"[t] a\rb\n"` arrives as `['[t] a\n', 'b\n']` (verified locally). The
  `Compiling` segment after a `\r` arrives unprefixed and belongs to no
  task. `build:cli:fixture-size` runs `cargo build --release` with
  `pty=True` inside `default`. Reading logs back with `read_text()` repeats
  the problem, and one non-UTF-8 byte raises `UnicodeDecodeError`
  mid-session.
- 🟡 **Architecture / Code Quality / Usability / Safety**: Raw logs reach
  disk only after every run and its analysis succeed
  **Location**: Phase 4 §2: Session and record (`measure_build` step 5,
  `write_measured_build`); §5 Capture the goldens
  Any late failure discards every captured log. Create the run directory
  after the platform check, stream each log to disk as it arrives, and
  analyse only after capture finishes, so that a fixed parser can
  re-derive a record.
- 🟡 **Architecture / Code Quality / Standards / Test Coverage**: The record
  has a writer but no typed reader, and its contract is untested
  **Location**: Phase 4: `record.py`; Phase 9: `comparison.py`
  The record shape is encoded twice, with no `schema_version` refusal and
  no round-trip test. Add `BuildRecord.parse` beside `record_document`,
  have `compare_records` take parsed records, and round-trip
  `compare_records(r, r)` in a unit test.
- 🟡 **Code Quality / Test Coverage / Correctness**: The tracked-process set
  has two sources of truth, and the `ps` filter is untested
  **Location**: Phase 4 §3: `wiring.py`, `--ide-pattern`; Phase 2:
  `tracked_line_filter`
  When the two drift, the process reads as `absent`, a plausible wrong
  answer that triggers Phase 10's retake rule forever. The `ps` golden is
  captured after filtering, so it cannot reveal wrongly dropped lines.
- 🟡 **Test Coverage / Code Quality**: Exit-status and no-record criteria
  are asserted at layers that cannot observe them
  **Location**: Phase 4: `TestBuildTask`, `test_session.py`
  The refusal test monkeypatches `measure_build_with`, the very function
  that does the mapping. "No run directory" is asserted against a session
  that never writes. The `--runs` default of 3 lives in the `@task`
  signature, not in `BuildSettings`. `_history(REPO_ROOT)` is evaluated
  outside the mapped region, so its `PreconditionFailureError` escapes as
  a traceback before the macOS check.
- 🟡 **Usability / Architecture / Code Quality**: No designed progress
  output during a 20–25 minute run
  **Location**: Phase 4: `BuildPorts`; Manual Verification
  Without a port, an operator cannot tell a hung build from a slow one.
  Add `progress: Callable[[str], None]` to `BuildPorts` and assert the
  ordered events in `test_session.py`.
- 🟡 **Documentation / Usability**: Later modes ship undocumented and leave
  the Phase 4 docs inaccurate
  **Location**: Phases 6, 7 and 9
  The README never says what `--cold` deletes. The docstring and
  description stay wrong for `--cold`, which skips priming, and for
  `--isolate`, which does not run `default`. The flags have no `help=`
  mapping.
- 🟡 **Safety / Correctness / Test Coverage**: Interrupt cleanup can be cut
  short and leave a detached build running
  **Location**: Phase 4 §3: `SubprocessBuild`, samplers,
  `unwinding_signals`
  Mask the unwind signals during the reap, signal the process group with
  `os.killpg`, re-scan for children, and stop the samplers in `finally`
  with a daemon thread. Test with the real `PsutilProcessOps` and a
  grandchild.
- 🟡 **Usability**: Retake conditions known at start are discovered only
  after the run
  **Location**: Phase 4 `measure_build` steps 1–3; Phase 5 step 5;
  Phase 10 retake rules
  Commit match, label, power, jobs and running tracked processes are all
  known before idle sampling. Print a preflight block and warn when
  `--keep` is combined with `unspecified`, a mismatch or an absent IDE.
- 🟡 **Architecture / Code Quality / Safety / Test Coverage**: Refusal
  rules and destructive effects sit outside the ports, and task-layer
  tests target the real checkout
  **Location**: Phases 6 and 7: `tasks/measure.py` validation;
  `prepare_timed_run`
  `measure_build` accepts illegal plans, and `prepare_timed_run` calls
  `shutil.rmtree` directly. Validate inside `measure_build` against the
  `TaskGraph` it already loads, put cache preparation behind a port, and
  guard task-layer tests with an untouchable `real_ports`.
- 🟡 **Code Quality / Architecture / Correctness**: Isolation mode adds
  redundant state and needs signature changes the earlier phases do not
  plan
  **Location**: Phase 6: Implementation; Phase 1: `analyse_timeline`
  Without a `ready_from` origin, queueing delay is inflated for every
  isolated task after the first. Model a `BuildScope` (full or isolated)
  that yields invocations, and give `analyse_timeline` an origin parameter
  in Phase 1.
- 🟡 **Test Coverage**: The across-runs tests omit the present-process
  happy path and `SILENT_IN_SOME_RUNS`
  **Location**: Phase 2: `test_across_runs.py`
  `process_across_runs` returning `spread=None` for every input would pass
  the suite, as would a truthiness check that treats `0.0` as absent.
- 🟡 **Test Coverage**: The captured-run tests have vacuous assertions and
  read the live task graph
  **Location**: Phase 4: `test_captured_run.py`, §5
  "Every task has an outcome" and "no negative queueing" hold by
  construction. `mise tasks --json` is not captured, so the closure drifts
  as the epic reshapes `default`. `wc -l` counts lines, not occurrences.
- 🟡 **Correctness / Test Coverage**: No rule for a zero process-CPU median
  **Location**: Phase 9: `comparison.py`
  A resident-but-idle `XprotectService` under `devtools` gives `0.0`, so
  the ratio divides by zero. Define the ratio as current ÷ previous, and
  name a zero denominator in words.
- 🟡 **Test Coverage / Correctness**: The incremental mtime assertion
  flakes on Linux CI
  **Location**: Phase 7: `test_cache_states.py`
  The Linux coarse clock gives equal mtimes for back-to-back touches.
  Reset the file to a fixed old mtime after each observation, or touch it
  with an explicit time from the injected clock.
- 🟡 **Test Coverage**: `cpu_used` does not test a PID that exits during
  the build
  **Location**: Phase 2: `test_host_activity.py`
  `XprotectService` is short-lived, so an implementation that reads only
  the final snapshot would undercount it unnoticed.
- 🟡 **Documentation**: The procedure for taking a comparable record lives
  only in the plan
  **Location**: Phase 4 README; Phases 5 and 10
  Operators re-measuring after an OS update need the label meanings, the
  IDE-open rule, the no-commits rule and the retake rules in the README.
- 🟡 **Documentation**: The `compare-builds` comparability verdict differs
  from the documented rule
  **Location**: Phase 9
  The tool ignores the host-settings label for Phase 10, while 0361 needs
  the labels to match. Say so in the docstring and README, and always
  print both labels.

#### Minor

- 🔵 **Architecture / Code Quality / Standards**: The kept-record directory
  is defined twice, and other paths are spelled out again rather than
  taken from `tasks/shared/paths.py`
  **Location**: Phase 3 `provenance.py`; Phase 4 `record.py`; Phase 7
  `cache_states.py`
  If `MEASUREMENTS` and `KEPT_RECORD_DIRECTORY` drift apart, every kept
  record moves the measured commit. `CLI_TARGET_DIR`, `FRONTEND` and
  `MANIFEST_DIRNAME` already exist.
- 🔵 **Architecture / Code Quality / Standards**: Helpers stranded in
  `tasks/measure.py` split the composition root and spawn duplicates
  **Location**: Phase 4 §3–§4
  `Probe`/`ambient_probe` copy `DiagnosticRunner`, and the reaper and
  signal unwinding are second or third copies. Names drift too:
  `ports.now()` and `ports.clock.now()` mean different things, as do
  `CacheState.COLD` and `CacheMode.COLD`, and `keep` has two meanings.
- 🔵 **Usability / Code Quality**: Several refusal messages are unspecified
  or bare
  **Location**: Phase 3 `measured_commit`, `HistoryError`; Phases 6 and 7
  `NoMeasurableCommitError(MEASUREMENTS)` prints only
  `meta/measurements/`.
- 🔵 **Architecture / Correctness**: A task mise never started is
  indistinguishable from a roll-up
  **Location**: Phase 1 `TaskGraph`, timeline rules
  An early mise abort makes every task `SILENT` and the run look clean.
  Read whether each task has a `run`, and mark `contains_failure` when
  mise exits non-zero.
- 🔵 **Architecture / Usability**: The comparability key omits `mode` and
  `isolated_tasks`, and `compare-builds` has no designed handling for bad
  inputs
  **Location**: Phase 9
  An isolated record would compare as equivalent to a full one.
- 🔵 **Safety / Architecture**: Cold deletion is checked once but runs N
  times, and an interrupted `rmtree` leaves half-deleted trees
  **Location**: Phase 7 `cache_states.py`
  Re-check before each deletion, and rename into the run directory before
  `rmtree`.
- 🔵 **Safety**: `--cold` deletes the dev launcher, so the `vcs guard`
  hook fails open
  **Location**: Phase 7; Phase 4 README
- 🔵 **Standards**: The example summary uses an EN DASH, which ruff RUF001
  rejects
  **Location**: Phase 4 `summary_report` example
  Verified: RUF001 flags `–` in a string literal, and `tasks/**` does not
  ignore it.
- 🔵 **Standards / Documentation**: Record field names are inconsistent
  about units
  **Location**: Implementation Approach: record format
  `wall_clock_seconds` sits beside unitless `duration` and `lock_wait`.
  The schema freezes at the Phase 5 baseline.
- 🔵 **Correctness**: The jj chain from `@-` is ambiguous when `@` is a
  merge
  **Location**: Phase 3 `history.py`
  Use `first_ancestors(first_parent(@))`.
- 🔵 **Usability**: `--keep` writes a record that the summary then says not
  to keep
  **Location**: Phase 4 summary example
- 🔵 **Usability**: The end-of-run summary omits the commit match and
  absent tracked processes
  **Location**: Phase 4 `summary_report`
- 🔵 **Documentation**: Existing warm-dispatch text will misdescribe
  `measure:build`
  **Location**: Phase 4 `mise.toml` and README placement
  The exclusion comment says "needs a quiet host" and "owned by its own
  CI job".
- 🔵 **Documentation**: The summary sections promise documentation and
  behaviour that no phase delivers
  **Location**: Performance Considerations; Migration Notes
  Neither the lower-bound caveat nor the `schema_version` refusal is
  scheduled.
- 🔵 **Documentation**: Nothing says how long run directories live or
  whether to delete them
  **Location**: Phase 4 README
- 🔵 **Documentation**: How to re-capture the goldens exists only in the
  plan
  **Location**: Phase 4 §5
- 🔵 **Documentation**: Post-merge work-item edits leave Phase 5's
  dependency text stale
  **Location**: Phases 6 and 7
- 🔵 **Code Quality**: Partial presence is modelled differently for tasks
  and processes
  **Location**: Phase 2 `across_runs.py`
- 🔵 **Code Quality**: Failed probe readings sit in-band in numeric fields
  (`int | str`)
  **Location**: Phase 3 `host.py`
  Two records that failed the same probe compare as matching.
- 🔵 **Test Coverage**: Comparison tests miss the noise boundaries and the
  schema refusal
  **Location**: Phase 9 `test_comparison.py`
- 🔵 **Test Coverage**: Lock-wait tests miss consecutive waits and an open
  wait in a failed task
  **Location**: Phase 1 `test_timeline.py`
- 🔵 **Test Coverage**: The cold-deletion safety guards are not tested
  individually
  **Location**: Phase 7 `test_cache_states.py`
- 🔵 **Test Coverage**: Working-copy match tests miss the mixed cases and
  a gitignored run directory
  **Location**: Phase 3 `test_history.py`; Phase 4 `test_session.py`
- 🔵 **Test Coverage**: Probe parse-failure paths are tested for one probe
  only
  **Location**: Phase 3 `test_host.py`
- 🔵 **Test Coverage**: The isolation non-overlap test may only check the
  script's own stamps
  **Location**: Phase 6 `test_session.py`; Phase 4 `ScriptedBuild`

#### Suggestions

- 🔵 **Usability**: Use one `--cache-state` enum rather than two exclusive
  booleans
  **Location**: Phase 7
- 🔵 **Usability**: Make kept-record names show the condition they measure
  **Location**: Implementation Approach: Record name
- 🔵 **Safety**: Add an interlock against two `measure:build` sessions in
  one workspace
  **Location**: Phase 4 `measure_build_with`
  An `fcntl.flock` on `.accelerator-measure/build/.lock` would do it.
- 🔵 **Code Quality**: Make the timeline analysis's intermediate model
  explicit
  **Location**: Phase 1 `timeline.py`
- 🔵 **Standards**: Replace the catch-all `adapters.py` and fix the
  mismatched adapter class names
  **Location**: Module table; Phase 4 §3
- 🔵 **Standards**: Wrap `FETCH_FAILURE_PATTERNS` in `MappingProxyType`
  **Location**: Phase 1 `fetch_failures.py`
- 🔵 **Standards**: Document the third naming scheme for kept records
  **Location**: Phase 4 README
- 🔵 **Documentation**: Tell readers on other Macs how to read the figures
  **Location**: Phases 8 and 10 `CONTRIBUTING.md`
- 🔵 **Test Coverage**: Test the CPU bucket boundaries
  **Location**: Phase 2 `test_host_activity.py`
- 🔵 **Test Coverage**: Check Phase 10's cited figures mechanically
  **Location**: Phase 10 `test_contributing.py`
- 🔵 **Correctness**: `socket.gethostname()` can drift on macOS
  **Location**: Phase 3 `host.py`; Phase 4 `run_id`

### Strengths

- ✅ The pure-core, injected-shell split is kept rigorously. `timeline`,
  `host_activity`, `across_runs`, `provenance` and `comparison` are pure,
  and all process, clock, VCS and probe access goes through `BuildPorts`,
  following the pin-guard and dev precedents.
- ✅ The `History` port has three methods and interchangeable jj and git
  adapters. The adapter is chosen from `repository_kind`, never by probing
  `git`, which avoids the secondary-workspace hazard.
- ✅ One `epoch` from one clock stamps the build log and both samplers, so
  per-run analysis shares one timeline.
- ✅ The timeline rules match the probed mise contract: end-of-run status
  lines give a status without extending a duration, a skipped task ends at
  its skip line, and readiness recurses through silent roll-ups.
- ✅ Comparing each commit with its first parent fixes jj's `files()`
  merge blind spot. Excluding `meta/measurements/` means a kept record
  never moves the measured commit.
- ✅ Absent and zero CPU are kept distinct. The since-boot `iostat` report
  is discarded, `ps` TIME is parsed past 60 minutes, and IDE matching by
  main executable avoids counting `fsnotifier` twice.
- ✅ Cold deletion is opt-in, limited to two paths, and checked for
  containment and symlinks. `--cold` with `--isolate` is refused before
  ports are built, and a sentinel proves warm runs delete nothing.
- ✅ Timestamp-plus-hostname run ids avoid the max+1 collision across jj
  workspaces. Records default to the gitignored run directory.
- ✅ Registration reuses the run-string-keyed closure guard unchanged, and
  the five strings pinned by `TestMeasureNamespaceDocs` are kept.
- ✅ Test names read as behaviours. Hand-seeded fixtures carry worked
  answers, goldens are never hand-edited, and the real jj and git tests are
  gated by `require`, so they fail rather than skip in CI.
- ✅ The plan states the ruff and pyrefly constraints accurately and splits
  the timeline rules up front to stay under C901 and PLR0912.

### Recommended Changes

1. **Read the build's output as bytes and split on `\n` only**
   (addresses: text-mode `\r` splitting).
   In `SubprocessBuild`, read raw bytes, decode each line with
   `errors="replace"`, and drop `bufsize=1`. Write and read stamped logs
   and goldens with `newline=""` or as bytes. Add an adapter test whose
   child prints a prefixed `\r`-joined line.
2. **Split capture from analysis, and stream logs to disk** (addresses:
   logs held in memory; golden chicken-and-egg).
   Create the run directory immediately after `require_macos`, and have
   `SubprocessBuild` and both samplers tee lines to their log files. Run
   `analyse_session(captured)` once all runs finish. On an interrupt or an
   analysis error, keep the logs and print the path, and still write no
   record.
3. **Add a typed `BuildRecord` with a strict parse and a round-trip test**
   (addresses: writer with no reader; untested contract; `schema_version`
   promise; `compare-builds` bad inputs; comparability key missing mode).
   Put `BuildRecord.parse` in `record.py` and have it refuse an unknown
   `schema_version`. Make `compare_records` take parsed records, and add
   `mode` and `isolated_tasks` to the key. Settle the field units
   (`_seconds` everywhere) before Phase 5 freezes the schema.
4. **Define the tracked-process set once** (addresses: two sources of
   truth; untested filter; anchored pattern).
   Pass `tracked` to `real_ports(root, tracked)` or to `Sampler.start`.
   Specify that `tracked_line_filter` splits with `maxsplit=3` and matches
   the command field, and test it over real padded `ps` lines and one
   unfiltered tick.
5. **Move refusal rules into the session and side effects behind ports**
   (addresses: domain rules in the task layer; destructive step outside
   the ports; vacuous task-layer tests; refusal mapping).
   Validate `--isolate` names and the incompatible flag combinations in
   `measure_build`, after `require_macos`. Put cache preparation behind a
   `workspace` port. Build the ports inside the mapped region. Test
   `measure_build_with` over fake ports rooted at `tmp_path`, covering
   exit 0 on a failed task, `Exit(1)` on Linux with no directory created,
   and the `--runs` default at the task layer.
6. **Harden interrupt cleanup** (addresses: samplers not stopped; reap cut
   short; interrupt test with `FakeProcs`).
   Make samplers context managers, mark the periodic thread as a daemon,
   and bound its joins. Mask the unwind signals during the reap, call
   `os.killpg`, and re-scan for children. Test with the real
   `PsutilProcessOps` and a grandchild, and add a session test that a
   `ScriptedBuild` raising in run 2 stops both samplers. Add `iostat` to
   the manual Ctrl-C check.
7. **Add progress and preflight output** (addresses: no progress port;
   retake conditions discovered late; summary omissions; `--keep`
   contradiction).
   Add `progress: Callable[[str], None]` to `BuildPorts`. Before idle
   sampling, print the commit and match, label, power, jobs, the tracked
   processes running and the `--cold` targets, with warnings. Add the
   match and absent processes to the summary, and withhold the kept copy
   when there are fetch failures.
8. **Give `analyse_timeline` a readiness origin in Phase 1 and model the
   build scope** (addresses: isolation-mode redundancy; inflated isolated
   queueing).
   Add `ready_from: float = 0.0` to `analyse_timeline`. Replace `Mode`
   plus `isolate` with a `BuildScope` that yields invocations, and define
   how per-invocation timelines feed `cpu_buckets`, the wall-clock and the
   exit status.
9. **Close the test gaps** (addresses: across-runs omissions; exiting PID;
   vacuous goldens; mtime flake; zero median; boundaries).
   Add the present-process spread, `SILENT_IN_SOME_RUNS`, `0.0`-not-absent,
   an exiting PID, the noise boundaries and a zero median. Capture
   `mise tasks --json` with the goldens and assert the exact `SILENT` and
   `SKIPPED` sets. Reset the mtime to a fixed value in the incremental
   test.
10. **Document each mode as it lands, plus the record-taking procedure**
    (addresses: undocumented later modes; procedure only in the plan;
    `compare-builds` label rule; warm-dispatch text; summary-section
    promises).
    Give Phases 6, 7 and 9 each a docs step and extend
    `TestMeasureNamespaceDocs`. Add a "Taking a baseline or condition
    record" list to the README. Give the new tasks their own exclusion
    comment in `mise.toml`.
11. **Consolidate shared helpers and paths** (addresses: stranded
    helpers; duplicated constants; naming drift; EN DASH).
    Move `repository_kind`, `DiagnosticRunner`, the reaper and signal
    unwinding into `tasks/shared/`. Derive paths from
    `tasks/shared/paths.py`. Rename `now` to `utc_now` and the sampler's
    `keep` to `retains`. Use an ASCII separator in the summary.

## Per-Lens Results

### Architecture

**Summary**: The plan carries the measure namespace's pure-core,
injected-shell shape into a well-partitioned `tasks/shared/build_measurement/`
package. It has a clean `History` abstraction, one measurement epoch shared
by the build and both samplers, and phasing that lets the pure modules merge
independently. The structural weaknesses are at the boundaries. The
persisted record has a writer but no typed reader. Capture, analysis and
persistence are coupled, so a late failure loses every log. Several domain
invariants are owned by the invoke adapter or defined twice rather than by
one domain module. Helpers stranded in `tasks/measure.py` also split the
composition root.

**Strengths**:
- The functional core and imperative shell are kept rigorously apart, with
  a single wiring module, following the pin_guard and dev precedents.
- The `History` protocol gives interchangeable jj and git adapters, chosen
  from `repository_kind`, never by probing `git`.
- One `epoch` from one injected clock stamps all three sources.
- Run ids remove the max+1 collision. The gitignored run directory and
  opt-in `--keep` fit jj's snapshot semantics.
- Every "cannot measure" exit is a `MeasurementRefusedError` subclass,
  mapped to `Exit` at one point.
- Phasing follows the dependency gates.
- Tradeoffs are stated explicitly: `ps` filtering, `Spread`, local `pmset`
  parsing and the lower-bound CPU figure.
- Registration reuses the closure guard unchanged, and
  `FETCH_FAILURE_PATTERNS` is additive.

**Findings**:

- 🟡 major (high): **The record schema has a writer but no typed reader.**
  Location: Phase 9 `comparison.py` / Phase 4 `record.py`. The shape is
  encoded twice, with no strict parse or `schema_version` gate. Suggestion:
  add a typed `BuildRecord` with `parse_record`, a round-trip test, and
  have `compare_records` take two `BuildRecord`s.
- 🟡 major (medium): **Capture, analysis and persistence are coupled, so
  logs exist only if the whole session succeeds.** Location: Phase 4 §2.
  Suggestion: stream logs to the run directory as they arrive, and analyse
  only after all captures finish. A `reanalyse <run-dir>` then becomes a
  pure addition.
- 🟡 major (medium): **Settings invariants live in the invoke adapter, so
  the session accepts illegal plans.** Location: Phases 6 and 7,
  `tasks/measure.py` validation. Isolate-name validation also runs before
  `require_macos`. Suggestion: give `BuildSettings` a smart constructor,
  and validate names inside `measure_build` after `require_macos`.
- 🔵 minor (medium): **The cold-deletion safety check is separated in time
  and place from the deletion.** Location: Phase 7. Suggestion: return a
  verified `ColdDeletions` value, re-verify before each delete, and call
  the check from `measure_build`.
- 🔵 minor (high): **Shared helpers stranded in `tasks/measure.py` split
  the composition root.** Location: Phase 4 §3–§4. `_history(REPO_ROOT)`
  escapes the refusal mapping. Suggestion: move `repository_kind` and the
  diagnostic runner into `tasks/shared/`, and let `real_ports(root)`
  choose the history adapter.
- 🔵 minor (high): **The tracked-process set has two sources of truth.**
  Location: Phase 4 §3. Suggestion: pass `tracked` into `real_ports` or
  `Sampler.start`.
- 🔵 minor (high): **The kept-record directory is defined independently in
  provenance and record.** Suggestion: define it once, and test that a
  kept path is excluded by `outside_measurements`.
- 🔵 minor (medium): **Later modes extend `measure_build` by branching,
  and `analyse_timeline` lacks an origin.** Suggestion: add a run-plan
  seam with a per-run `prepare` hook in Phase 4, like `time_summaries`,
  and a `ready_at` parameter in Phase 1.
- 🔵 minor (medium): **Interrupt teardown misses the samplers and reaps
  the build tree from one snapshot.** Suggestion: use sampler context
  managers, bounded joins and `ps` calls, `os.killpg`, and re-snapshot in
  the grace loop.
- 🔵 minor (medium): **The mise output contract is an unguarded external
  dependency.** Suggestion: record in `TaskGraph` whether each task has a
  `run`, and refuse when mise exited 0 but a runnable task is `SILENT` or
  has no verdict.
- 🔵 minor (medium): **Comparability conditions are not a first-class
  concept, and mode is missing from the key.** Suggestion: add a
  `MeasurementConditions` value object, written to the record as one
  block.
- 🔵 suggestion (medium): **Progress output has no port.** Suggestion: add
  `progress` or a `Reporter` to `BuildPorts`.

### Code Quality

**Summary**: The plan keeps the measure namespace's shape. Its modules are
small and focused, the `StrEnum` vocabulary is rich, every refusal shares
one error hierarchy, and the tests mirror the package. The biggest
maintainability risks are facts held in two places: the tracked-process
set, the record shape, the measurements path, and the duplicated probe,
reap and signal helpers. Capture is coupled to analysis, and isolation
mode arrives through redundant state.

**Strengths**:
- Frozen, slotted value types and Protocol ports are gathered into
  `BuildPorts` with one wiring module.
- Each module in the table has one clear role, and the tests mirror the
  package.
- The domain vocabulary is explicit and backed by `@unique` `StrEnum`s,
  including `CacheState.primes`.
- One `MeasurementRefusedError` hierarchy; probe failures name what was
  tried.
- `Spread` avoids widening `Summary`.
- The `History` port is small, and the provenance functions are short and
  pure.
- C901 and PLR0912 are considered up front.
- The summary and comparison reports are pure strings.
- `untouchable_ports()` makes "refused before anything runs" checkable.

**Findings**:

- 🟡 major (high): **Raw logs exist only in memory until the whole session
  succeeds.** Location: Phase 4 `session.py`, `record.py`. Suggestion:
  write logs as they arrive, and add a pure `analyse_session(captured)`.
- 🟡 major (high): **Record comparison walks untyped JSON and duplicates
  the record shape.** Location: Phase 9. `FigureChange` is a bag of
  optional fields. Suggestion: add `parse_kept_record` beside the writer,
  and make `FigureChange` two cases.
- 🟡 major (high): **Tracked processes have two sources of truth.**
  Location: Phase 4 `wiring.py`, `--ide-pattern`. Suggestion: derive the
  filter and the analysed set from one value.
- 🟡 major (medium): **Isolation mode is added through redundant state and
  unplanned signature changes.** Location: Phase 6. Suggestion: model a
  `BuildScope` (`FullBuild` or `IsolatedTasks`) with `invocations()`, and
  give `analyse_timeline` an origin in Phase 1.
- 🔵 minor (high): **The circular-import workaround duplicates the probe
  and splits the composition root.** Suggestion: move `DiagnosticRunner`
  and `repository_kind` into `tasks/shared/`.
- 🔵 minor (high): **A third copy of process-tree reaping and a second
  signal-unwinding implementation.** Suggestion: extract `reap_tree` into
  `tasks/shared/processes.py`, and `unwinding_signals` into
  `tasks/shared/`.
- 🔵 minor (medium): **The refusal-to-`Exit` mapping does not cover all
  the code that can refuse.** `_history` is evaluated outside the mapping,
  and the test monkeypatches the mapper itself. Suggestion: build the
  ports inside the mapped region, and make `UnsafeDeletionError` and the
  unknown-repository case `MeasurementRefusedError` subclasses.
- 🔵 minor (medium): **Isolated task names are validated in the task
  layer, outside the ports.** Suggestion: raise `UnknownTaskError` from
  `measure_build`.
- 🔵 minor (high): **The measurements directory is defined twice.**
  Suggestion: derive the provenance prefix from `KEPT_RECORD_DIRECTORY`.
- 🔵 minor (high): **Some refusal messages will not explain themselves.**
  Suggestion: write full-sentence messages and assert them in tests.
- 🔵 minor (high): **Overloaded names for clocks and `keep`.**
  Suggestion: rename to `utc_now` and `retains`/`line_filter`.
- 🔵 minor (medium): **Progress output has no seam.** Suggestion: add
  `progress` to `BuildPorts`, and make any line callback explicit.
- 🔵 minor (medium): **Samplers are not guaranteed to stop when the
  session unwinds.** Suggestion: use context managers or an `ExitStack`.
- 🔵 minor (medium): **Partial presence is modelled differently for tasks
  and processes.** Suggestion: one shared standing with a reason flag.
- 🔵 minor (medium): **Failed probe readings are mixed in-band with
  numeric fields.** Suggestion: an `Unread(argv)` type, treated as not
  comparable.
- 🔵 suggestion (medium): **Make the timeline analysis's intermediate
  model explicit.** Suggestion: a `TaskTranscript` stage and named
  `TaskFigures` constructors.
- 🔵 suggestion (medium): **Cache-state preparation puts filesystem
  effects into the session.** Suggestion: a `CachePreparation` object
  built in the wiring.

### Test Coverage

**Summary**: The test design is strong. A pure core sits behind injected
ports, tests are named as behaviours, hand-seeded fixtures carry worked
answers, goldens have independent oracles, and the real jj and git tests
fail rather than skip in CI. The gaps are at the seams.
`measure_build_with` is never tested with fake ports, the serialised
record shape has no test, the `ps` filter is untested, and some
criterion-critical paths are missing.

**Strengths**:
- The pure core is testable without a build, with one behaviour-named file
  per module.
- Hand-seeded edge fixtures are paired with never-edited goldens and
  independent oracles.
- `ScriptedBuild` observes the workspace on every call.
- `untouchable_ports()` proves no collaborator was touched.
- Real-repository tests cover the merge discovery and read-only
  subcommands.
- The closure-guard expected set is extended, and `TestMeasureNamespaceDocs`
  is extended rather than bypassed.

**Findings**:

- 🟡 major (high): **The exit-status and no-record criteria are asserted
  at layers that cannot observe them.** Location: Phase 4 `TestBuildTask`,
  `test_session.py`. Suggestion: test `measure_build_with` over fake ports
  at `tmp_path`, and test the runs default at the task layer by capturing
  `BuildSettings`.
- 🟡 major (high): **The record contract between `record_document` and
  `compare_records` is untested.** Suggestion: a round-trip test through
  `json.dumps` and `json.loads` asserting `compare_records(r, r)` is a
  zero change, and fixtures built from `record_document` output.
- 🟡 major (medium): **The `ps` filter is untested, and the `ps` golden is
  captured through it.** Suggestion: unit-test `tracked_line_filter` over
  raw padded lines, derive the filter and tracked set from one tuple, and
  capture one unfiltered tick.
- 🟡 major (high): **The across-runs tests omit the present-process happy
  path and one task flag.** Suggestion: add a present-in-every-run spread,
  `SILENT_IN_SOME_RUNS`, a `0.0` value, and combined flags.
- 🟡 major (medium): **`cpu_used` does not test a PID that exits during
  the build.** Suggestion: add an exiting-PID case and a zero-increase
  case.
- 🟡 major (medium): **The incremental mtime assertion is likely to flake
  on the Linux CI leg.** Suggestion: reset the mtime to `(0, 0)` after
  each observation.
- 🟡 major (medium): **Interrupt cleanup is checked by a test that may not
  signal a real process.** Suggestion: use the real `PsutilProcessOps`
  with a grandchild, and add a session test that the samplers stop when
  the build raises.
- 🟡 major (medium): **Task-layer refusal tests target the real checkout
  with no guard.** Suggestion: a `nothing_read`-style fixture, `REPO_ROOT`
  monkeypatched to `tmp_path`, and isolate validation moved into the
  session.
- 🟡 major (medium): **Captured-run tests carry vacuous assertions and
  depend on the live task graph.** Suggestion: capture
  `mise tasks --json`, and assert the exact `SILENT` and `SKIPPED` sets,
  one queueing delay, one bucket and one CPU increase. Use `grep -o`.
- 🔵 minor (high): **Comparison tests miss the noise boundary, zero
  baselines and the schema refusal.**
- 🔵 minor (medium): **Lock-wait tests miss consecutive waits and an open
  wait in a failed task.**
- 🔵 minor (medium): **Each cold-deletion safety guard is not tested on
  its own.** Suggestion: a symlink on the second target, and a symlinked
  ancestor.
- 🔵 minor (medium): **Working-copy match tests miss the mixed cases and a
  gitignored run directory.**
- 🔵 minor (medium): **Probe parse-failure paths are tested for one probe
  only.**
- 🔵 minor (medium): **The isolation non-overlap test may only check the
  script's own stamps.** Suggestion: stamp relative to
  `clock.now() - epoch` and advance `FakeClock`.
- 🔵 suggestion (medium): **CPU bucket boundaries are untested.**
- 🔵 suggestion (low): **Phase 10's cited figures are checked only by
  hand.** Suggestion: run `compare_records` in the docs test.

### Correctness

**Summary**: The analysis rules are mostly sound and rest on mise's
observed behaviour. The main risks are at the boundaries. The text-mode
pipe breaks up `\r`-joined lines, cleanup after an interrupt can be cut
short, and the comparison has no rule for a zero median. Several
interfaces leave edge cases undefined: the isolation-mode readiness
origin, a jj working copy that is a merge, and the tracked-process filter
wiring.

**Strengths**:
- The timeline rules match the probed mise contract, and queueing delay is
  clamped.
- Readiness recurses through silent roll-ups, and after-failure comes from
  the graph.
- The working-copy match is checked at both ends.
- First-parent comparison fixes jj's merge blind spot.
- Absent is kept distinct from zero. The since-boot `iostat` report is
  discarded, and `ps` TIME is parsed past 60 minutes.
- `--runs` of at least 1, and spreads only when a value exists in every
  run, keep `median()` off empty input.
- Cold deletion is contained, and `--cold` with `--isolate` is refused
  first.

**Findings**:

- 🟡 major (high): **Text-mode pipes split cargo's `\r` progress segments
  away from their task prefix.** Location: Phase 4 §3 `SubprocessBuild`.
  `build:cli:fixture-size` runs with `pty=True` inside `default`.
  Suggestion: read bytes, split on `\n`, decode with `errors="replace"`,
  and use `newline=""` for logs and goldens.
- 🟡 major (medium): **Cleanup after an interrupt can be cut short,
  leaving a detached build running.** Suggestion: mask signals during the
  reap, call `os.killpg`, re-scan for children, and stop the samplers in
  `finally`.
- 🟡 major (medium): **No rule for a zero process-CPU median: the ratio
  divides by zero.** Suggestion: current ÷ previous, with a zero
  denominator named in words.
- 🔵 minor (high): **Isolated queueing delay has no origin parameter.**
  Suggestion: `ready_from: float = 0.0`, read before each `build.run`, and
  define how the per-invocation timelines merge.
- 🔵 minor (medium): **The jj chain from `@-` is ambiguous when `@` is a
  merge.** Suggestion: `first_ancestors(first_parent(@))`, and a test.
- 🔵 minor (medium): **A task mise never started is indistinguishable from
  a roll-up.** Suggestion: read `run` presence, set `contains_failure`
  when mise exits non-zero, and base `ready_at` on `end is None`.
- 🔵 minor (medium): **The `ps` capture filter and the analysed set can
  diverge, and the anchored pattern needs the command field.**
  Suggestion: `maxsplit=3`, tested against a real padded line.
- 🔵 minor (medium): **The incremental-touch mtime assertion depends on
  Linux clock granularity.**
- 🔵 suggestion (low): **`socket.gethostname()` can drift on macOS.**
  Suggestion: `scutil --get LocalHostName`, or confirm `HostName` is set.

### Safety

**Summary**: The plan handles `--cold` carefully: it is opt-in, limited
to two paths, checked for containment and symlinks, refused with
`--isolate`, and backed by sentinel tests. The harness never commits. The
weaker area is the unwind path: the samplers and the detached build depend
on happy-path cleanup, every log is written only at the end, and the
destructive step sits in the orchestration core rather than behind a port.

**Strengths**:
- Cold deletion is opt-in, contained, refuses symlinks, and leaves
  `docs-site/node_modules` alone.
- `--isolate` with `--cold` is refused before the ports are built.
- A sentinel in `cli/target` proves that warm and incremental runs delete
  nothing.
- No commits. `--keep` is opt-in, and the sibling `build/` directory is
  safe from warm-dispatch teardown.
- The non-macOS refusal is tested with `untouchable_ports()`.
- The VCS adapters are pinned to read-only subcommands, and the test
  builders are hermetic.
- Incremental mode changes only the mtime, and the `ps` filter keeps logs
  small.

**Findings**:

- 🟡 major (medium): **Samplers are not stopped when a session is
  interrupted or raises.** Suggestion: context managers, a daemon periodic
  thread, a join timeout, and a session test.
- 🟡 major (medium): **The destructive cache-state step is reachable from
  tests through `settings.root` instead of an injected port.**
  Suggestion: a `BuildPorts.workspace` port, with task-level refusals
  proven by untouchable factories.
- 🔵 minor (medium): **The detached build session plus interruptible
  reaping can orphan a full `default` build.** Suggestion: mask signals
  during the reap, call `os.killpg`, print the pgid, and add a
  double-interrupt manual check.
- 🔵 minor (high): **All logs and the record are written only after the
  whole session succeeds.** Suggestion: stream logs, write atomically with
  `os.replace`, and record an end-of-run match failure as
  `matched_at_end: null` rather than refusing.
- 🔵 minor (medium): **Cold deletion is checked once but runs N times, and
  an interrupted `rmtree` leaves half-deleted trees.** Suggestion: re-check
  before each delete, and `os.rename` into the run directory before
  `rmtree`.
- 🔵 minor (low): **`--cold` deletes the dev launcher, which makes the
  `vcs guard` hook fail open while it is missing.** Suggestion: warn when
  the dev-launcher marker is present, and say so in the README.
- 🔵 suggestion (medium): **No interlock against two `measure:build`
  sessions in the same workspace.** Suggestion: an `fcntl.flock` on
  `.accelerator-measure/build/.lock`.

### Usability

**Summary**: The operator gets a sound basic contract: a familiar
invocation, early refusal of misused flags, and a documented exit-0 rule.
The main weakness is the 20–25 minute run itself. Nothing designed reports
progress, raw logs reach disk only after success, and every retake
condition is found afterwards with a hand-run `jq`. Smaller gaps are in
flag discoverability, refusal messages, and how `compare-builds` handles
inputs it cannot compare.

**Strengths**:
- Invalid flag combinations are refused before any port is built.
- The exit-code contract is explicit in the docstring.
- The invocation matches the existing `measure:*` convention, and the mise
  description states the run length.
- The flag refusals shown are actionable.
- The planned summary gives a readable verdict.
- `measure:compare-builds` turns Requirement 9's citation rules into one
  command.
- Timestamp-plus-hostname names stay sortable and do not collide.

**Findings**:

- 🟡 major (high): **No designed progress output during a 20–25 minute
  run.** Suggestion: a progress port asserted in `test_session.py`, plus
  an optional `--echo`.
- 🟡 major (high): **Raw logs reach disk only after every run and its
  analysis succeed.** Suggestion: stream logs, and add a
  `--from-run-directory` re-analysis.
- 🟡 major (high): **Retake conditions that are known at start are only
  discovered after the run.** Suggestion: a preflight block with warnings,
  or a `--baseline` strict mode.
- 🔵 minor (high): **`--keep` writes a record that the summary then says
  not to keep.** Suggestion: withhold the kept copy when there are fetch
  failures, or name the path to delete.
- 🔵 minor (high): **The end-of-run summary omits the commit match and
  absent tracked processes.** The `runs 2` typo should read `run 2`, with
  a log pointer.
- 🔵 minor (high): **Several refusal messages are unspecified or would
  print only a bare path.**
- 🔵 minor (medium): **Flags have no per-flag help, and the Phase 6–7
  modes are documented nowhere.** Suggestion: a `help={...}` mapping on
  the task, with README and docstring updates.
- 🔵 minor (medium): **The tracked-process set is configured in two places
  for `--ide-pattern`.** The replace-or-extend semantics are unspecified.
- 🔵 minor (high): **`measure:compare-builds` has no designed behaviour
  for inputs it cannot compare**: a missing file, warm-dispatch records,
  an unknown `schema_version`, or isolated against full.
- 🔵 suggestion (medium): **Cache state is two exclusive booleans, unlike
  the enum-valued `--host-settings`.**
- 🔵 suggestion (low): **Kept-record names do not show the condition they
  measure.**

### Standards

**Summary**: The plan follows the measure namespace's conventions closely:
the registration, the ports and wiring pattern, the exception naming, the
kebab-case enum values and the test conventions. The remaining problems
are naming and consistency details that are cheapest to fix before the
Phase 5 baseline freezes the record schema.

**Strengths**:
- Registration matches the existing measure leaves exactly.
- The ports and wiring copy the pin-guard pattern.
- Exception names meet N818.
- Enums are `@unique` `StrEnum`s with kebab-case values, matching
  committed records.
- Test conventions match the codebase.
- `Spread` reuses `Summary`'s field names, and the package layout mirrors
  `tasks/shared/dev/`.
- The ruff and pyrefly constraints are stated accurately.

**Findings**:

- 🔵 minor (high): **Record field names are inconsistent about units.**
  Suggestion: `_s` or `_seconds` on every scalar.
- 🔵 minor (high): **Path constants are written out again instead of
  reused.** Suggestion: use `tasks/shared/paths.py` for `CLI_TARGET_DIR`,
  `FRONTEND`, and a moved `MANIFEST_DIRNAME`.
- 🔵 minor (medium): **The example summary uses an EN DASH, which ruff's
  RUF001 rejects.** Suggestion: an ASCII separator.
- 🔵 minor (medium): **New names drift from the measure namespace's
  existing names**: `Probe` against `DiagnosticRunner`, `now` against
  `clock.now`, and `CacheState.COLD` against `CacheMode.COLD`.
- 🔵 minor (medium): **Record comparison works on raw dicts instead of a
  parsed domain type.**
- 🔵 suggestion (medium): **A catch-all `adapters.py` and mismatched
  adapter class names.** Suggestion: `build_runner.py`
  (`SubprocessBuildRunner`) and `samplers.py`.
- 🔵 suggestion (medium): **A `Mapping`-annotated module constant is not
  wrapped in `MappingProxyType`.**
- 🔵 suggestion (low): **The third naming scheme for kept records is not
  written down.**

### Documentation

**Summary**: The plan treats documentation as a deliverable pinned by
tests. The main gaps are currency and durability. The Phase 4 docs are
never updated as Phases 6, 7 and 9 add modes. The operator procedure for a
comparable record exists only in the plan. Existing warm-dispatch text
will misdescribe the new task.

**Strengths**:
- The documentation changes are pinned by `TestMeasureNamespaceDocs`,
  `TestHostSettingsGuidance` and the Phase 10 path test.
- The docstring states the non-obvious contract.
- Test names carry the reasoning in code rather than comments.
- Known answers sit beside the assertions, and the `zgrep` derivations
  beside the golden constants.
- `CONTRIBUTING.md` states the security tradeoff and the limit of IDE
  exclusions.
- Refusal messages explain themselves.
- The plan's file and line cross-references were spot-checked as accurate.
- Phase 5 updates every affected sibling work item.

**Findings**:

- 🟡 major (high): **Later modes ship undocumented and make the Phase 4
  task docs inaccurate.** Suggestion: a docs step in Phases 6 and 7, with
  `--isolate`, `--incremental` and `--cold` pinned in the doc test.
- 🟡 major (medium): **The procedure for taking a comparable record lives
  only in the plan.** Suggestion: a README list titled "Taking a baseline
  or condition record".
- 🟡 major (medium): **The `compare-builds` comparability verdict differs
  from the documented rule.** Suggestion: say that the label is excluded,
  and always print both labels.
- 🔵 minor (high): **Existing warm-dispatch text will misdescribe
  `measure:build`.**
- 🔵 minor (high): **The summary sections promise documentation and
  behaviour no phase delivers.**
- 🔵 minor (medium): **Nothing says how long run directories live or
  whether to delete them.**
- 🔵 minor (medium): **How to re-capture the goldens exists only in the
  plan.** Suggestion: put it in the `test_captured_run.py` module
  docstring.
- 🔵 minor (medium): **Post-merge work-item edits leave Phase 5's
  dependency text out of date.**
- 🔵 suggestion (medium): **The host guidance should tell readers on other
  Macs how to read the figures.**
- 🔵 suggestion (medium): **Only the plan's example explains what record
  fields mean.**

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-30T23:17:49+00:00

**Verdict:** REVISE

### Previously Identified Issues

All 91 pass-1 findings were re-checked by the lens that raised them: 82 are
resolved, 9 are partially resolved, and none is still present.

| Lens | Resolved | Partially resolved | Still present |
|---|---|---|---|
| Architecture | 11 | 1 | 0 |
| Code Quality | 16 | 1 | 0 |
| Test Coverage | 16 | 1 | 0 |
| Correctness | 8 | 1 | 0 |
| Safety | 6 | 1 | 0 |
| Usability | 10 | 1 | 0 |
| Standards | 8 | 0 | 0 |
| Documentation | 7 | 3 | 0 |

The partially resolved findings:
- 🟡 **Safety / Correctness**: Interrupt cleanup can be cut short —
  Partially resolved. `killpg`, re-listing and the `ExitStack` landed, but
  `blocked_signals` defers the second signal only in the main thread (see
  the first new issue below).
- 🟡 **Architecture**: Capture, analysis and persistence are coupled —
  Partially resolved. Logs stream to disk and analysis runs last, but the
  graph, provenance, settings and per-invocation exit statuses are never
  written to the run directory, so re-derivation is not the "pure
  addition" the plan claims.
- 🟡 **Test Coverage**: Exit-status criteria asserted at the wrong layer —
  Partially resolved. `TestMeasureBuildWith` covers the mapping, but
  `history_of`'s `NotARepositoryError` conversion is untested.
- 🔵 **Code Quality**: Duplicated reaping and signal unwinding — Partially
  resolved. `MeasurementSession` keeps its own handler swap, and
  `reap_session` adds a second grace loop beside `reap_tree`.
- 🔵 **Usability**: Refusal messages unspecified — Partially resolved.
  `SessionInProgressError` and `UnsafeDeletionError` messages remain
  unspecified.
- 🔵 **Documentation**: Existing warm-dispatch text misdescribes
  `measure:build` — Partially resolved. The section's exclusion rationale
  and "Who runs what" stay warm-dispatch-only.
- 🔵 **Documentation**: Re-capture steps only in the plan — Partially
  resolved. Phase 4's goldens are covered; the advisory fixture and the
  `ps` tick are not.
- 🔵 **Documentation**: Only the plan explains record fields — Partially
  resolved. The README's "`null` means absent" is wrong for several fields,
  and `presence`, `not-run`, `Unread` and `conditions` are unexplained.

### New Issues Introduced

#### Major

- 🟡 **Safety / Correctness / Test Coverage**: `pthread_sigmask` defers
  signals only in the main thread, while the sampler threads are live.
  The reap runs in the build runner's `finally` before the session's
  `ExitStack` stops the samplers. A second Ctrl-C or SIGTERM is delivered
  to a sampler thread, and CPython still runs `_unwind` in the main thread
  mid-reap, skipping SIGKILL and orphaning part of the detached build. The
  planned test is single-threaded, so it passes regardless. Fix by
  switching the unwind handlers to a recording handler for the reap's
  duration (Python handlers always run in the main thread), and test with
  a live helper thread.
- 🟡 **Architecture / Code Quality / Standards**: The record's types live
  in `session.py`, creating a `session`↔`record` import cycle.
  `record.py` needs `MeasuredBuild` and `BuildSettings`; `measure_build`
  calls `record.run_id` and `record.preflight_report`. The pure
  `comparison.py` then imports from the orchestration module. Fix by
  moving the record types (with `Mode`, `HostSettings`,
  `MeasurementConditions`) into a pure `measured_build.py`, putting
  `analyse_session` in a pure `analysis.py`, and having `run_id` take
  primitives.
- 🟡 **Test Coverage / Architecture**: Nothing in CI parses the committed
  baseline, and the schema has no evolution rule. The models are strict
  and closed, and Phases 6, 7 and 9 edit them after Phase 5 commits the
  baseline. A rename or a required field would pass every test and leave
  the baseline unreadable by `measure:compare-builds`. Fix by
  parametrising a test over every committed `build-*.json`, and stating
  the rule: additive fields carry defaults at version 1; renames bump the
  version with an upgrade in `parse_record`.

#### Minor

Grouped by theme; each was raised by the lenses named.

- 🔵 **Reuse and lint (Standards, Architecture)**: `FlockSessionLock`
  re-implements `tasks/shared/locking.py` `workspace_lock`; `os.replace`
  and `os.rename` trip PTH105/PTH104 and bypass `atomic_write_text`;
  `Unread.__str__` lacks `@override`; a `partial(...)` dataclass default
  trips RUF009.
- 🔵 **Record shape before the freeze (Standards, Correctness, Code
  Quality)**: `schema_version` is declared last with a default, so it is
  written last and a missing key validates; Pydantic writes `Z`, not
  `+00:00`; `summary.process_cpu_seconds` holds objects, not seconds;
  `Presence` can hold contradictory states; derived copies in `conditions`
  and `fetch_failures` are unchecked on read, and Pydantic `==` on
  conditions disagrees with `differences()`.
- 🔵 **Kept-copy write (Correctness, Safety)**: "exclusive, the same way"
  contradicts `os.replace`; `os.link` from a temporary in the run
  directory gives both properties.
- 🔵 **Reap and sampler robustness (Correctness, Test Coverage, Safety)**:
  `reap_session` lists descendants only after the group SIGTERM, and
  `signal_group` can raise `ProcessLookupError` in the `finally`;
  `FakeProcs` models no process groups; a failed `ps` tick ends the
  thread silently, and a failed first tick counts long-lived processes
  from zero; the shared helper does not guard against signalling its
  caller's own group.
- 🔵 **Cold path (Safety, Code Quality, Usability, Correctness,
  Architecture)**: a failed trash removal is silent and repeats per run;
  the order of preparation against the epoch is unstated;
  `cache_states.py` mixes the deleting adapter with a persisted enum.
- 🔵 **Comparison (Correctness, Code Quality, Usability)**: no rule for a
  process absent from both records or flagged on either side; the
  `FigureChange` cases carry pre-rendered prose; the first line reads
  "comparable apart from" when records do not compare.
- 🔵 **Edge cases (Correctness, Test Coverage, Usability)**: a repeated
  `--isolate` name breaks `Timeline.merged`; an empty probe output never
  becomes `Unread`; `read_stamped`'s split rule is unspecified;
  `--ide-pattern` regexes are not compiled up front.
- 🔵 **Operator output (Usability)**: progress and log pointers cover warm
  full runs only, with possibly relative paths; a fetch failure is
  reported only after every run; help text misdescribes `--host-settings`,
  `--keep` and `--runs`; two refusal messages are unspecified or invite
  the wrong fix.
- 🔵 **Model shape (Code Quality, Architecture)**: `BuildScope` cannot
  express the priming invocation; `OwnFigures` duplicates eight
  `TaskFigures` fields; the preflight's `ps` reading duplicates the
  sampler's argv; the run directory lacks the non-log inputs re-derivation
  needs.
- 🔵 **Tests (Test Coverage)**: the real-process interrupt test is
  timer-synchronised and sees zombies; the round trip omits `Unread` and
  the later enum members; `history_of`, per-line streaming in the real
  adapters, the preflight's inputs, and `NOT_RUN` with mise exit 0 are
  untested.
- 🔵 **Docs (Documentation)**: `--keep` descriptions omit the withholding;
  the 0368 amendment is underspecified; the record-taking list omits the
  lock's per-workspace scope, comparing before committing, and how to
  re-measure the baseline; the plan's summary sections and some Phase 4
  text have drifted from the later phases.

#### Suggestions

- 🔵 **Usability**: a `--preflight-only` flag; per-task changes in
  `measure:compare-builds` for 0365 and 0366; clearer field separators in
  record names.
- 🔵 **Standards / Code Quality**: probe pyrefly with real models rather
  than an empty one; key `help` by parameter name; name the factory
  parameters as factories; define `meta/measurements` once across both
  harnesses; consider `NoReading` for `Unread`.
- 🔵 **Safety**: write the build's pgid into the lock file; repeat the
  dev-launcher warning at the end of a cold session.
- 🔵 **Code Quality / Documentation**: decide cold-state effects inside
  the workspace; tell 0361 to use `measure:compare-builds` for its
  closing comparison.

### Assessment

The revision is substantial and sound: every pass-1 finding was addressed
in full or in part, and no lens found a pass-1 issue still present. The
verdict stays REVISE only because three new major findings meet the
threshold of three. Each is narrow and mechanical: a recording handler in
place of `pthread_sigmask` for the reap, a module move that breaks the
import cycle, and a parse-every-committed-record test with a one-paragraph
evolution rule. The minor findings are mostly lint fixes, reuse of
existing shared helpers, and record-shape details that are cheap now and
expensive after the Phase 5 baseline freezes the schema. Once the three
majors are fixed, the plan is ready for implementation.

## Re-Review (Pass 3) — 2026-10-31T11:31:34+00:00

**Verdict:** COMMENT

### Previously Identified Issues

All 74 pass-2 findings were re-checked by the lens that raised them: 63
are resolved, 11 are partially resolved, and none is still present. Every
pass-2 major is resolved: the signal deferral now swaps Python-level
handlers, the record schema lives in a pure `measured_build.py`, and CI
parses every committed `build_*.json` under a stated evolution rule.

| Lens | Resolved | Partially resolved | Still present |
|---|---|---|---|
| Architecture | 5 | 1 | 0 |
| Code Quality | 7 | 4 | 0 |
| Test Coverage | 10 | 1 | 0 |
| Correctness | 9 | 1 | 0 |
| Safety | 5 | 1 | 0 |
| Usability | 10 | 0 | 0 |
| Standards | 10 | 1 | 0 |
| Documentation | 7 | 2 | 0 |

The partially resolved findings, each narrowed to a residue:
- 🔵 **Code Quality**: Derived copies — the cross-part copies are checked,
  but `commit.matched`, `contains_failure` and `unit_totals` are not.
- 🔵 **Code Quality / Architecture**: Run naming — priming shares
  `invocations`, but its `run` argument is unused and naming is split
  between the scope and the session.
- 🔵 **Code Quality**: Failed trash removal — reported and refused, but its
  cause is discarded.
- 🔵 **Code Quality**: Cold-state decisions — the workspace owns the
  effects, but the reports still special-case cold.
- 🔵 **Architecture**: `cache_states.py` — the adapter moved out, but the
  `Workspace` port still sits beside the persisted enum.
- 🔵 **Test Coverage**: Deferred reap — `deferred_signals` is tested with a
  live thread, but no test checks that the runner wraps its reap in it.
- 🔵 **Correctness / Standards**: Record example — `started_at` will carry
  microseconds, and the per-task key order differs from the inherited
  declaration order.
- 🔵 **Safety**: Dev-launcher warning — repeated at the end and on
  interrupt, but not on a refusal or error after the first deletion.
- 🔵 **Documentation**: `--keep` and summaries — the `mise.toml`
  description and some summary sections still lag.

### New Issues Introduced

No new critical or major findings. The minors cluster as follows; each
was raised by the lenses named.

#### Minor

- 🔵 **Orphan check (Safety, Correctness, Usability, Documentation)**: a
  stale `build.pgid` whose ID has been reused can name an unrelated live
  group, a `PermissionError` reads as alive, an unvalidated value of `1`
  would make the advice `kill -TERM -1`, and a deferred re-raise can skip
  removing the file. Record the leader's create time, treat
  `PermissionError` and PGIDs ≤ 1 or equal to the caller's as not the
  orphan, list the group's members before advising a kill, and remove the
  file inside the deferred block.
- 🔵 **Session start order (Test Coverage, Correctness)**: three steps each
  claim the slot straight after `require_macos`, and their tests pin
  incompatible orders. State one sequence and assert it once.
- 🔵 **Own-group guard (Code Quality, Test Coverage, Correctness)**: the
  guard needs `group_of(pid)` and the caller's group, which `ProcessOps`
  does not offer, and a direct `os.getpgid` can raise inside the
  `finally`.
- 🔵 **Hand-off types and notices (Architecture, Code Quality,
  Standards)**: `CapturedRun`, `BuildResult` and `Preflight` have no stated
  home, and the obvious one recreates the import cycle; leftover trash and
  the dev-launcher warning have no route from the session to the summary.
  Name the homes, add an import-boundary test, and return a
  `SessionOutcome` with typed notices.
- 🔵 **Comparison edges (Correctness, Test Coverage, Architecture, Code
  Quality, Usability)**: a nextest phase on one side only fits no change
  case; a named task missing from a record has no fact type; in isolation
  mode the `--task` lookup should use `summary.tasks`, not `task_set`;
  records tracked with different IDE patterns compare as matching; the
  report has no worked example and does not name its records.
- 🔵 **Reaping and trash windows (Safety, Correctness, Test Coverage)**:
  the `try` should open straight after `Popen`; an interrupt during
  `remove_tree` leaves unreported trash; the real group operations and a
  malformed `build.pgid` are untested; two real-process tests can flake or
  hang.
- 🔵 **Record consistency (Code Quality, Correctness)**: "failed" is
  derived three ways; `ProcessSampling`'s first-tick flag is redundant;
  `unreadable-in-some-runs` is also used for every run; `TaskFigures`
  inherits factories that can only raise.
- 🔵 **Operator output (Usability)**: idle load is never shown; without
  `--keep` the summary omits fetch failures; the Ctrl-C hint is wrong on
  the last run; an orphan or trash refusal gives no cause or safe check.
- 🔵 **Docs (Documentation, Usability)**: Phase 4 names
  `measure:compare-builds` and the `CONTRIBUTING.md` steps before they
  exist; the retake criteria are stated three times, disagree, and omit
  unreadable CPU; the re-measure bullet does not say how the new record
  reaches `main`; the advisory capture command likely needs
  `--output prefix`.
- 🔵 **Standards**: `unwind_signals()` survives beside
  `unwinding_signals()`; `RunLabel` reuses "label"; four prescribed test
  names exceed 80 columns.
- 🔵 **Correctness**: `PS_ARGV` should pass `-ww`, or a long command can
  be truncated and the end-anchored patterns miss.

#### Suggestions

- 🔵 Split `host_activity.py` into `iostat` and `ps` modules; move the
  `Workspace` port beside the other session ports; state that upgrades run
  over raw JSON and that newer code reads older records (Architecture).
- 🔵 A shared `measured_build(**overrides)` test builder; a non-empty
  assertion for the committed-record glob; an interrupt test for leftover
  trash (Test Coverage).
- 🔵 Report renderers split per section; one root shared by settings and
  ports; cold-state data properties on `BuildCacheState` (Code Quality).
- 🔵 Rename `FlockSessionLock`, the non-variant change types and
  `ProcessSampling`; give the Pydantic pin comment its "what and where"
  clause; one name for the kept-record directory (Standards).
- 🔵 Bold-lead README paragraphs instead of one long one; the
  `run-<n>-test_unit_cli.log` mapping; a total across unit kinds for 0366
  (Documentation).
- 🔵 A closest-match hint for a mistyped `--task`; readable rendering of
  `NoReading` and tuple conditions (Usability).
- 🔵 Refuse incremental mode when the touch target is missing (Safety).

### Assessment

The plan is ready for implementation. Three passes have resolved every
critical and major finding, and pass 3 raised no new ones. What remains is
minor and mechanical. The one minor worth fixing before Phase 4 merges is
the orphan check: as specified, a stale `build.pgid` could lead an
operator to signal an unrelated process group. The next is the session's
start order, whose phase tests contradict one another. The rest can be
taken up during implementation or left as accepted.

## Re-Review (Pass 4) — 2026-10-31T14:24:54+00:00

**Verdict:** COMMENT

### Previously Identified Issues

All 71 pass-3 findings were re-checked by the lens that raised them: 64
are resolved, 7 are partially resolved, and none is still present.

| Lens | Resolved | Partially resolved | Still present |
|---|---|---|---|
| Architecture | 6 | 1 | 0 |
| Code Quality | 10 | 1 | 0 |
| Test Coverage | 7 | 1 | 0 |
| Correctness | 9 | 0 | 0 |
| Safety | 5 | 0 | 0 |
| Usability | 12 | 0 | 0 |
| Standards | 7 | 3 | 0 |
| Documentation | 8 | 1 | 0 |

The partially resolved findings, each narrowed to a residue:
- 🟡 **Architecture / Code Quality / Standards**: Hand-off types — every
  home is named, but `SessionOutcome` sits in `session.py` while the pure
  `reports.summary_report` takes it (see the first new issue).
- 🔵 **Test Coverage**: Deferred reap — the `unwind_then_defer` path is
  pinned; the `deferred_signals` branch, a reap entered with no prior
  signal, is not.
- 🔵 **Standards**: Four 73–74 character test names fit 80 columns only
  as module-level functions, and the plan does not state the layout.
- 🔵 **Standards**: The comparison variants still mix noun and participle
  forms (taste only).
- 🔵 **Documentation**: Phase 4 defers a README pointer to the
  `CONTRIBUTING.md` steps to Phase 8, which never schedules it.

### New Issues Introduced

#### Major

- 🟡 **Architecture, Code Quality, Standards, Correctness**:
  `summary_report` takes `SessionOutcome` from `session.py`. The pure
  `reports.py` would import `session`, which already imports `reports`,
  so `test_no_pure_module_imports_the_session_or_an_adapter` fails as
  written. Architecture rates it major; the other three, minor. Fix by
  declaring `SessionOutcome` in `reports.py` beside `SessionNotice`.

#### Minor

- 🔵 **`BuildGroup` port (Correctness, Safety, Standards, Architecture)**:
  `record(pgid)` disagrees with its test's `record(pgid, create_time)`;
  nothing says who reads the create time, or when; `ProcessOps` has no
  query listing a group's members for the refusal; `OrphanedBuild` has no
  pure home; and "record" now names both the PGID file and the
  measurement record.
- 🔵 **Unremoved trash timing (Correctness)**: "stops at once" and "stopped
  before timed run 2" read differently. State that run n proceeds and the
  session refuses before preparing run n+1.
- 🔵 **Exit report (Safety, Code Quality)**: an error inside the callback
  replaces the exception that ended the session, and its wording is
  written apart from the reports' notices renderer.
- 🔵 **Untested exit and reap paths (Test Coverage)**: the "group survived
  the reap" message; the callback being pushed before the first
  preparation; the trash listing `TrashNotRemovedError` points to; and
  exact text for most comparison renderer variants.
- 🔵 **Comparison output (Usability)**: a one-run record gives a
  zero-width noise range and the report hides run counts; the word
  `none` means both a label and an empty tuple.
- 🔵 **Docs (Documentation)**: the Cache states paragraph overstates when
  trash stops the session and when a cause is shown; the
  "compare before committing" bullet does not fit jj or a condition's
  first record.

#### Suggestions

- 🔵 Move `LeftoverTrash` and `TRASH_PREFIX` out of `cache_states.py`
  (Architecture); keep both notice variants in `reports.py`, split
  `LeftoverTrash` by producer, and replace `TaskMissing`'s two booleans
  with one discriminator (Code Quality).
- 🔵 Settle the nextest phase from presence first; read the hostname once;
  skip the `group_alive(1)` test as root (Correctness).
- 🔵 List leftover trash in every preflight; accept a run id in
  `measure:compare-builds` (Usability).
- 🔵 Declare `test_the_baseline_is_among_the_committed_records` in
  Phase 4 and run `build-system:check` in Phase 5 (Test Coverage).
- 🔵 Match Phase 10's `jq` bar to Phase 5's; mark `session.py` as "4, 7"
  in the module table (Documentation).

### Assessment

The plan is ready for implementation. Every lens resolved its pass-3
findings in full or in part, and safety and usability report nothing
beyond minors. The one major is a single type in the wrong module, which
four lenses found independently and which the plan's own import-boundary
test would catch: moving `SessionOutcome` into `reports.py` is a one-line
fix and should be made before Phase 4 starts. The `BuildGroup` signature
and the trash-stop timing are the other two worth settling first, since
each could be built two ways; the rest can be taken up during
implementation.

## Approval — 2026-10-31T14:47:49+00:00

**Verdict:** APPROVE

The pass-4 major (`SessionOutcome` moved into `reports.py`) and every
pass-4 minor were addressed in the plan. Two suggestions were declined:
accepting a run id in `measure:compare-builds`, since the summary already
prints each record's full path, and renaming the comparison variants to
one form, which the standards lens rated as taste. The plan was approved
for implementation without a further review pass.
