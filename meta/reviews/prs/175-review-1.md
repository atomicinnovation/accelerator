---
type: "pr-review"
id: "175-review-1"
title: "[0368] Add an operator-run full-build timer with host analysis"
date: "2026-10-31T19:29:18+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:175"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "portability", "documentation", "standards"]
review_number: 1
pr_number: 175
tags: []
last_updated: "2026-10-31T19:29:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #175 - [0368] Add an operator-run full-build timer with host analysis

**Verdict:** COMMENT

The harness is well built: a self-contained `build_measurement` package where `session.py` orchestrates over injected ports, `wiring.py` is the single composition root, `test_imports.py` enforces the layering, and the analysis is checked against a real captured warm no-op run. The one substantive defect is in the build runner's teardown: a process group that outlives `mise` is never reaped, and `build.pgid` is deleted before the survival check, which disarms the orphan guard in exactly the case it exists for. The rest is minor: a locale-sensitive `ps` parse, an `iostat` cadence that contradicts the docs, a few untested fallback branches, and README structure.

### Cross-Cutting Themes

- **Teardown leaves the orphan guard disarmed** (flagged by: correctness, safety) — `_reap` signals only when the leader is alive and calls `build_group.forget()` unconditionally, before `_end` checks `group_alive`. A surviving group aborts the session with no `build.pgid`, so the next session measures alongside leftovers.
- **`ProcessOps` and port placement** (flagged by: architecture, code-quality, standards) — five group methods widen `ProcessOps` for every consumer; port protocols and their value types are split across `session.py`, `provenance.py` and `analysis.py`, so adapters import the orchestrator and `OrphanedBuildError` lives outside `errors.py`. A `ports.py` plus a `ProcessGroupOps` protocol would settle all three.
- **Sampling cadence is unpinned and misdescribed** (flagged by: correctness, test-coverage) — `iostat -w 2` samples every 2 s while the PR and README say once a second; no test pins the wired argv or intervals.
- **Silent degradation of process readings** (flagged by: code-quality, test-coverage, portability) — a failed preflight `ps` becomes "not running" for every tracked process, untested; a comma-decimal locale makes every `ps` line unparseable and the figures read as `absent`.
- **Shared `.accelerator-measure/` scratch root** (flagged by: architecture, safety) — warm-dispatch's `_contained` admits any direct child for removal, now including `build/`. Safe today; a subdirectory per harness removes the latent hazard.

### Tradeoff Analysis

- **Architecture purity vs pragmatism**: architecture suggests a `RunDirectory` port and moving record persistence out of the "pure" partition; code-quality notes single-variant abstractions already anticipate later phases. Renaming `_PURE` to what it checks is the cheap fix; a full port can wait for isolation mode.

### Strengths

- ✅ Hexagonal layout with one composition root, enforced by an import-direction fitness test.
- ✅ Capture is separated from analysis, so `analyse_session` is a pure function over `CapturedSession` and can be checked against real golden captures.
- ✅ Pydantic validators recompute persisted derived fields and enforce "spread present iff measured", so a drifted record fails to load rather than misreads.
- ✅ Signal handling swaps handlers rather than using `pthread_sigmask` (sampler threads), and holds further signals once cleanup starts.
- ✅ `flock` session lock on a non-inheritable fd, hardlink-based no-overwrite keep, and logs always written before analysis.
- ✅ Measured commit walks first parents comparing with each parent, skipping measurement-only and empty commits; start and end checks use the same filter.
- ✅ Shared extractions (`signals`, `processes`, `diagnostics`, `repository`) are behaviour-preserving and carry their tests with them.
- ✅ Comments are rare and each records a real constraint (`MISE_TASK_TIMINGS` inheritance, lone `\r`, pydantic pin).

### General Findings

- 🔵 **Portability**: macOS coupling extends into analysis and the record schema (`iostat` column positions, tracked-process defaults, `host_settings` vocabulary); a Linux port means a schema bump, not just an adapter. Worth one README sentence.
- 🔵 **Safety**: warm-dispatch `_contained` admits `.accelerator-measure/build` for `rmtree` via a corrupted manifest; narrow it or give warm-dispatch its own subdirectory.
- 🔵 **Code Quality**: `tasks/measure.py` (~3,700 lines) now hosts a second measurement domain; consider `tasks/measure_build.py` under the same namespace.
- 🔵 **Code Quality**: `BuildCacheState`, `Mode` and `BuildScope` each have one variant, making `if not cache_state.primes` unreachable until phase 5.
- 🔵 **Architecture**: port definitions are split between `session.py`, `provenance.py` and `analysis.py`; consolidate into `ports.py`.
- 🔵 **Test Coverage**: real-subprocess and real-signal tests sit in the unit lane with deadlines up to 10 s; watch the first ubuntu and macOS CI runs for flakes.

### Additional Findings

- 🔵 `tasks/shared/build_measurement/session.py:118` — `OrphanedBuildError` defined outside `errors.py` (standards)
- 🔵 `tasks/README.md:383` — line exceeds 80 columns after manual edit (standards)
- 🔵 `tasks/shared/build_measurement/samplers.py:115` — samplers inherit operator locale; comma decimals break `ps` parsing silently (portability)
- 🔵 `tasks/shared/build_measurement/build_runner.py:56` — signal between `Popen` and `try` leaves an unrecorded build (safety)
- 🔵 `tasks/shared/build_measurement/samplers.py:45` — background `iostat` untracked for orphan recovery; log handle leaks on failed spawn (safety)
- 🔵 `tests/unit/tasks/shared/build_measurement/test_imports.py:14` — "pure" partition includes I/O modules (architecture)
- 🔵 `tasks/README.md:271` — transitive-closure guard sentence has ambiguous referent (documentation)
- 🔵 `mise.toml:418` — `measure:build` registered apart from other `measure:*` tasks; description long (standards)
- 🔵 `tasks/shared/diagnostics.py:7` — shared `DiagnosticRunner` docstring speaks warm-dispatch (standards)
- 🔵 `tasks/shared/build_measurement/samplers.py:75` — kill-after-timeout stop path untested (test-coverage)
- 🔵 `tasks/shared/build_measurement/record.py:74` — no test that the run record survives a refused keep (test-coverage)
- 🔵 `tasks/shared/build_measurement/history.py:82` — git path listing depends on `core.quotePath` (portability)
- 🔵 `tasks/shared/build_measurement/session.py:190` — session touches the filesystem directly (architecture)
- 🔵 `tasks/measure.py:358` — two harnesses share one scratch root (architecture)
- 🔵 `tasks/shared/build_measurement/build_group.py:11` — adapters import port types from the orchestrator (code-quality)
- 🔵 `tasks/shared/build_measurement/reports.py:216` — run directory derived from `record_path.parent` (code-quality)
- 🔵 `tasks/shared/build_measurement/measured_build.py:256` — recurring condition parameters could be one value object (code-quality)
- 🔵 `tasks/README.md:373` — retake rule names processes by prose, not record keys (documentation)
- 🔵 `tasks/README.md:352` — schema paragraph describes an upgrade hook that does not exist yet (documentation)
- 🔵 `tasks/README.md:380` — durable rule tied to work item 0361 (documentation)

## Inline Comments

### `tasks/shared/build_measurement/build_runner.py:95-113` — Surviving build group is never reaped, and `build.pgid` is forgotten before the survival check
**Severity**: major | **Confidence**: high | **Lens**: correctness, safety

🟡 **Correctness / Safety**

`_reap` calls `reap_session` only when `child.poll() is None`. If `mise` exits normally but something it spawned is still in the group (a daemonised `sccache` from a global `RUSTC_WRAPPER`, a task that backgrounds a helper), no signal is sent. `_reap` then calls `self.build_group.forget()` unconditionally, and only afterwards does `_end` check `group_alive` and raise `BuildLeftRunningError`. The interrupt path has the same ordering if the group survives SIGKILL.

**Impact**: A lingering member aborts the whole session with no record even though the build finished, and the orphan guard's file is already gone. The next session's `orphaned()` check passes and it measures alongside the survivors, silently skewing the baseline 0361 depends on.

**Suggestion**: Reap whenever the group is alive, not just the leader (`if child.poll() is None or self.procs.group_alive(child.pid)`; `reap_session` already falls back to a group SIGKILL). Move `forget()` into `_end` after `group_alive` returns false. Add a test that a surviving group leaves `build.pgid` in place.

---

### `tasks/README.md:311-381` — Build-timing docs are spliced into the middle of warm-dispatch prose
**Severity**: major | **Confidence**: medium | **Lens**: documentation

🟡 **Documentation**

The build-timing paragraphs (Build timing, Run directories, Reading a build record, the schema, the baseline protocol) sit between the warm-dispatch summary-latency paragraph and "What a warm-dispatch run requires". The warm-dispatch material then resumes with no heading, so "linux-arm64 is a calibrated platform too" reads as if it applies to the macOS-only `measure:build`.

**Impact**: An operator cannot tell where `measure:build` guidance ends, and a warm-dispatch reader skips ~70 unrelated lines mid-section.

**Suggestion**: Give each harness its own sub-heading (e.g. `#### measure:build`) and move the build-timing block after the warm-dispatch content.

---

### `tasks/shared/build_measurement/wiring.py:27` — `iostat` samples every 2 s, not once a second, and no test pins the cadence
**Severity**: minor | **Confidence**: high | **Lens**: correctness, test-coverage

🔵 **Correctness / Test Coverage**

`_IOSTAT_ARGV` is `("iostat", "-w", "2")`; only `ps` samples at `interval=1.0`. The PR description and README say both sample once a second, and the golden `iostat` fixture is stamped ~2 s apart. `cpu_buckets` assigns each reading by arrival stamp, so a reading credited to a bucket covers `[at-2, at)`. `test_the_real_ports_are_rooted_at_the_checkout` checks only roots, so neither cadence is pinned.

**Impact**: Readers assume twice the CPU-load resolution that exists, readings near a 30 s bucket edge leak into the next bucket, and a future change to the cadence would silently change what records mean.

**Suggestion**: Pick one (`-w 1`, or keep 2 s and correct the docs), name the interval as a domain constant, and add a wiring test asserting the composed `cpu_sampler.argv`, `process_sampler.interval` and `tick_timeout`.

---

### `tasks/shared/build_measurement/session.py:248-250` — A failed preflight `ps` reports every tracked process as not running, and the branch is untested
**Severity**: minor | **Confidence**: high | **Lens**: code-quality, test-coverage

🔵 **Code Quality / Test Coverage**

A `FileNotFoundError`/`PermissionError` from the `ps` probe becomes `""`, so `preflight_report` lists every tracked process as "not running" and, under `--keep`, warns that `syspolicyd` and the IDE are absent. Every session test uses the default diagnostics table, so this branch never runs.

**Impact**: The operator is pointed at the host when the probe failed, and removing or narrowing the `except` would go unnoticed.

**Suggestion**: Carry the failure as a distinct state (`tuple[str, ...] | NoReading(argv=PS_ARGV)`, as `host.py` does) and report "no reading from ps". Add a session test mapping `PS_ARGV` to `FileNotFoundError`.

---

### `tasks/shared/processes.py:46-50` — `ProcessOps` widened for every consumer to serve one
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

`ProcessOps` gains `signal_group`, `group_of`, `own_group`, `group_alive` and `group_members`. Only `reap_session`, `SubprocessBuildRunner` and `BuildGroupFile` use them; `tasks/shared/dev/lifecycle.py` and the dev host server only terminate and reap trees.

**Impact**: Every fake and alternative implementation for the dev lifecycle must now implement group operations it never uses, coupling it to build-measurement changes.

**Suggestion**: Split a `ProcessGroupOps(ProcessOps, Protocol)` with the five group methods and type the build-measurement callers against it. `PsutilProcessOps` implements both.

---

### `tasks/shared/build_measurement/provenance.py:33-34` — `working_copy_matches` is only called from tests; the session repeats its rule inline
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

`session.py` writes `not outside_measurements(history.working_copy_changes(commit))` in both `_provenance` and `_matches_at_end` rather than calling `working_copy_matches`, which only `test_provenance.py` and `test_history.py` exercise.

**Impact**: The tested function is not the code that runs; a rule change could pass its tests while the session keeps the old logic.

**Suggestion**: Have `_matches_at_end` call `working_copy_matches(self.ports.history, commit)`, or expose a `differing_paths` helper used for both the preflight list and the boolean, and delete the unused form.

---

### `tasks/shared/build_measurement/build_runner.py:82-93` — Stamp-and-log streaming duplicated between build runner and stream sampler
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

🔵 **Code Quality**

`SubprocessBuildRunner._stream` and `_RunningStream._read` in `samplers.py` both wrap a byte pipe with `newline="\n"`, stamp each line against `epoch`, write `stamped_text(line)` and collect. `samplers.py` already has `_StampedLog` for half of this, and only `build_runner.py` records why `newline="\n"` matters.

**Impact**: The lone-`\r` invariant is enforced in two places; changing one copy makes build and sampler logs stamp lines differently.

**Suggestion**: Extract one shared helper (e.g. `stamped_lines(pipe, clock, epoch, log)`) used by both, with the rationale stated once.

---

### `tasks/shared/build_measurement/wiring.py:49-52` — A failing `mise tasks --json` has no test for its refusal
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

🔵 **Test Coverage**

`_mise_tasks_json` turns a non-zero exit into `TaskGraphError`. `test_wiring.py` covers only repository detection and rooting.

**Impact**: Dropping the check would hand mise's stderr to `TaskGraph.parse` as JSON, giving the operator a confusing parse error, with no test failing.

**Suggestion**: Inject the argv or runner so a test can point it at `sys.executable -c 'import sys; sys.stderr.write("boom"); sys.exit(2)'`, and assert a `TaskGraphError` naming `mise tasks --json` and carrying the stderr.

---

### `tasks/README.md:358-363` — Baseline protocol omits the `unspecified` default
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

The `--host-settings` bullet lists three values but not that omitting the flag records `unspecified`. That is a `conditions` field, so such a record compares with no labelled baseline, and the plan's phase 5 check rejects it. "Condition record" in the heading is also undefined.

**Impact**: An operator following the README alone can keep a record that is unusable as a baseline, warned only by the preflight.

**Suggestion**: State that `--host-settings` is required for a baseline or condition record and defaults to `unspecified`, and define "condition record" in a few words.

---

### `tasks/README.md:335-337` — `<label>` in the record name is never defined
**Severity**: minor | **Confidence**: high | **Lens**: documentation

🔵 **Documentation**

The record name is given as `build_<UTC time>_<hostname>_<cache state>_<label>`, but `record.run_id`'s last field is the `--host-settings` value; "label" appears only in the CLI help.

**Impact**: An operator scanning `meta/measurements/` cannot map the last field to the flag without reading code.

**Suggestion**: Write `<host settings>`, or add "where `<label>` is the `--host-settings` value".

---

## Per-Lens Results

### Architecture

**Summary**: Self-contained package with a ports-and-adapters split, a single composition root and an import-direction fitness test; shared extractions follow the dependency rule. Issues are small: `ProcessOps` is wider than most consumers need, the "pure" partition includes I/O modules, and the session does some filesystem work directly.

**Strengths**: hexagonal layout enforced by `test_imports.py`; capture separated from pure analysis; `BuildScope`/`Invocation` ready for isolation mode without speculative code; process-group isolation, `build.pgid`, lock and `ExitStack` teardown; versioned schema with a stated evolution policy; `repository_kind` now raises its own `NotARepositoryError`.

**Comments**:
- `tasks/shared/processes.py:46-50` (minor, high) — `ProcessOps` widened for every consumer to serve one.
- `tests/unit/tasks/shared/build_measurement/test_imports.py:14-19` (minor, medium) — "pure" partition includes `record.py`, which writes files and hardlinks; move persistence to an adapter or rename the partition.
- `tasks/shared/build_measurement/session.py:190-192` (suggestion, medium) — session calls `mkdir` and computes run-directory layout directly; consider a `RunDirectory` port.
- `tasks/measure.py:358` (suggestion, low) — warm-dispatch and build share one scratch root with different ownership semantics; give warm-dispatch its own subdirectory.

**General Findings**:
- (suggestion) Port definitions split between `session.py`, `provenance.py` and `analysis.py`; consolidate into `ports.py`.

### Code Quality

**Summary**: Well decomposed, with a thin orchestrator, rich self-validating models, consistent domain vocabulary and small mostly-pure functions. Findings: a test-only helper duplicated inline, duplicated stamp-and-log code, adapters importing port types from the orchestrator, and two error paths that turn a failed probe into misleading output.

**Strengths**: ports-and-adapters with one composition root; consistent domain vocabulary (`TaskOutcome`, `Presence`, `Caveat`, `CommitMatch`, `MeasurementConditions`); derived fields recomputed in validators; behaviour-preserving extractions; specific, actionable refusal errors; short named helpers in `timeline.py`; sparse why-only comments.

**Comments**:
- `tasks/shared/build_measurement/provenance.py:33-34` (minor, high) — `working_copy_matches` test-only; session repeats the rule.
- `tasks/shared/build_measurement/build_runner.py:82-93` (minor, high) — stamp-and-log streaming duplicated with `samplers.py`.
- `tasks/shared/build_measurement/session.py:248-250` (minor, medium) — failed preflight `ps` gives a wrong diagnosis.
- `tasks/shared/build_measurement/build_group.py:11` (suggestion, medium) — adapters import port types from the orchestrator.
- `tasks/shared/build_measurement/reports.py:216-219` (suggestion, medium) — run directory derived from `record_path.parent` instead of `measured.run_directory`.
- `tasks/shared/build_measurement/measured_build.py:256-275` (suggestion, low) — `cache_state`, `mode`, `isolated_tasks`, `host_settings` travel together; a `SessionConditions` value object.

**General Findings**:
- (suggestion) `tasks/measure.py` keeps growing as a multi-purpose module.
- (suggestion) `ProcessOps` widening into a broad interface.
- (suggestion) Single-variant abstractions ahead of later phases.

### Test Coverage

**Summary**: Strong. Every new domain module has a behaviour-named test file; parsing is pinned by small fixtures and a real captured run; the session runs end to end over doubles; process-group and signal tests use real subprocesses and signals. Gaps are minor untested fallback branches and unpinned wiring constants.

**Strengths**: behaviour-specification test names; `test_captured_run.py` over real golden captures; real child, grandchild and SIGTERM-ignoring processes with bounded polling; moved tests follow extractions; record validation tested in both directions; import-boundary guard.

**Comments**:
- `tasks/shared/build_measurement/session.py:249` (minor, high) — preflight `ps` fallback untested.
- `tasks/shared/build_measurement/wiring.py:49-52` (minor, high) — failing `mise tasks --json` refusal untested.
- `tasks/shared/build_measurement/wiring.py:27` (minor, medium) — no test pins sampler commands or intervals; `iostat -w 2` contradicts "once a second".
- `tasks/shared/build_measurement/samplers.py:75-76` (minor, medium) — kill-after-timeout stop path untested.
- `tasks/shared/build_measurement/record.py:74-78` (minor, medium) — no test that the run record survives a refused keep.

**General Findings**:
- (suggestion) Real-subprocess and real-signal tests in the unit lane are the likeliest flake source; watch first CI runs.

### Correctness

**Summary**: The analysis core holds up across every outcome combination traced. The real gap is the build runner's teardown: a group outliving `mise` is never reaped, and `build.pgid` is deleted before the survival check. Also a sampling-interval mismatch with the description.

**Strengths**: `own_figures` excludes mise's final "exited with status" line; process CPU uses the first snapshot as baseline and treats any `ps` gap as `NoReading`; spread-iff-measured validated; `unwind_then_defer` handler swap accounts for sampler threads; first-parent walk skips measurement-only and empty commits.

**Comments**:
- `tasks/shared/build_measurement/build_runner.py:95-113` (major, medium) — surviving group never reaped; pgid forgotten before survival check.
- `tasks/shared/build_measurement/wiring.py:27` (minor, medium) — `iostat` samples every 2 s, not once a second.

**General Findings**: none.

### Safety

**Summary**: Careful for its scale: flock lock, own process group, escalating reap, fail-safe orphan detection, hardlink no-overwrite keep. Main gap: `build.pgid` deleted before the surviving-group check. Two smaller windows can leave an unrecorded build or sampler running.

**Strengths**: fail-safe orphan check with confirm-before-kill remediation; second Ctrl-C cannot interrupt cleanup; no-overwrite keep and `mkdir` without `exist_ok`; kernel-released non-inheritable lock; logs streamed before analysis; `reap_tree` refactor preserves dev-server semantics.

**Comments**:
- `tasks/shared/build_measurement/build_runner.py:103-113` (major, high) — `build.pgid` forgotten before the surviving-group check.
- `tasks/shared/build_measurement/build_runner.py:56-66` (minor, medium) — signal between `Popen` and `try` leaves an unrecorded, unreaped build.
- `tasks/shared/build_measurement/samplers.py:45-48` (minor, medium) — background `iostat` untracked and unbounded; log handle leaks on failed spawn.

**General Findings**:
- (suggestion) Run logs now live under a directory warm-dispatch teardown admits for `rmtree`.

### Portability

**Summary**: macOS-only by design, with the choice made visible by `require_macos` before any artefact and OS tools behind ports; tests use Python stand-ins so they run on both CI OSes. The one coupling risk is inherited locale breaking `ps` parsing.

**Strengths**: platform gate runs first; OS behaviour behind ports in `wiring.real_ports`; stand-in-based tests with root-aware skip; `_PS_TIME` accepts BSD and procps forms; `MISE_TASK_*` stripped; probes degrade to `NoReading`.

**Comments**:
- `tasks/shared/build_measurement/samplers.py:115-127` (minor, medium) — samplers inherit locale; comma decimals silently break parsing. Pin `LC_ALL=C` as `farm_environment` already does.
- `tasks/shared/build_measurement/history.py:82-86` (suggestion, low) — `git` path output depends on `core.quotePath`; use `-z` or `-c core.quotePath=false`.

**General Findings**:
- (suggestion) macOS coupling extends into analysis and the record schema; name the seams in the README.

### Documentation

**Summary**: The README covers what the harness measures, where it writes, how to read a record, schema evolution and the operator protocol, and the claims checked hold. Placement is the main problem, plus a few record-reading details left implicit.

**Strengths**: README claims verified against code; rare why-only comments; `measure_build_with` docstring explains ordering; help text, `mise.toml`, docstring and README agree; plan honestly reflects phases 1–4.

**Comments**:
- `tasks/README.md:311-381` (major, medium) — build-timing docs spliced into warm-dispatch prose.
- `tasks/README.md:271-276` (minor, medium) — transitive-closure guard sentence has ambiguous referent.
- `tasks/README.md:335-337` (minor, high) — `<label>` never defined.
- `tasks/README.md:358-363` (minor, high) — `unspecified` default omitted; "condition record" undefined.
- `tasks/README.md:373-375` (suggestion, medium) — retake rule uses prose names, not record keys (`ide`); `XprotectService` exemption unstated.
- `tasks/README.md:352-356` (suggestion, medium) — schema paragraph describes an upgrade hook that does not exist yet.
- `tasks/README.md:380-381` (suggestion, medium) — durable rule tied to work item 0361.

**General Findings**: none.

### Standards

**Summary**: Follows build-system conventions: module placement, mirrored tests without `__init__.py`, exact pydantic pin, task registered and in the closure guard, Python 3.14 idioms. Findings are consistency nits.

**Strengths**: cohesive package under `tasks/shared/`; exact pin with sibling-style comment; `measure:build` registration rules followed; import guard makes layering executable; idioms match (`StrEnum`, slotted frozen dataclasses, PEP 758); no work-item references in comments.

**Comments**:
- `tasks/shared/build_measurement/session.py:118-130` (minor, high) — `OrphanedBuildError` defined outside `errors.py`.
- `mise.toml:418-424` (minor, medium) — `measure:build` registered apart from other `measure:*` tasks; ~230-character description.
- `tasks/README.md:383` (minor, high) — line exceeds 80 columns.
- `tasks/shared/diagnostics.py:7-12` (minor, medium) — shared `DiagnosticRunner` docstring speaks warm-dispatch.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
