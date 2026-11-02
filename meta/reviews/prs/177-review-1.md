---
type: "pr-review"
id: "177-review-1"
title: "[0368] Isolate and cold-start build measurements, and compare build records"
date: "2026-11-02T18:38:21+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:177"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "safety", "usability", "documentation", "standards"]
review_number: 1
pr_number: 177
tags: []
last_updated: "2026-11-02T18:38:21+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #177 - [0368] Isolate and cold-start build measurements, and compare build records

**Verdict:** COMMENT

A well-built extension of the `measure:build` harness. Isolation slots into the `BuildScope` union, cache-state behaviour lives on the `BuildCacheState` enum, deletion sits behind a `Workspace` port, and `compare_records` is a pure function. The deleting path is guarded, and real-filesystem tests cover symlinks, containment and re-verification. No lens found a blocking issue. The findings gather around the cold-trash failure paths, which give contradictory or misleading messages and do not roll back. `comparison.py` mixes its model with its text rendering. Some work-item frontmatter has drifted.

### Cross-Cutting Themes

- **Cold-trash failure paths** (flagged by: correctness, safety, usability, test-coverage, code-quality) — When the session stops on unremoved trash, it reports that trash twice: first "delete it once the cause is fixed", then "safe to delete". A failed second rename leaves `cli/target` stranded in the trash with no rollback. The cold runs already completed produce no record. The failure hook warns that the dev launcher needs rebuilding even when nothing was deleted. Only the fake workspace tests a partial-move failure, and `_timed_runs` is hard to follow. All of these come from one under-modelled concern: what a cold session has actually done when it ends early.
- **`comparison.py` mixes the comparison model with its rendering** (flagged by: architecture, code-quality) — The ~540-line module holds the result types, the citation rules and the whole text renderer. Two symptoms follow: `_plural` is copied from `reports.py`, and the closest-match lookup duplicates `TaskGraph.closest_to`.
- **Unit totals come from run 1 only** (flagged by: correctness, test-coverage, code-quality) — `_unit_totals` reads `runs[0]` through reflection over `CargoUnits.model_fields`. No check or test confirms that runs agree, and 0366 will cite these totals.
- **The dev-launcher rebuild rule is decided twice** (flagged by: code-quality, test-coverage, correctness) — `Preflight.dev_launcher_notices` and `_deletion_notices` each decide it. The `OSError` → marker-present fallback is silent and untested.
- **Work-item frontmatter drift** (flagged by: standards, documentation) — `last_updated` was not bumped on 0362, 0363, 0364, 0367 and 0368. `blocks:` was appended after `schema_version` in 0363 and 0367.

### Tradeoff Analysis

- **Architecture vs Code Quality on `BuildCacheState`**: code quality praises the enum for owning `touches` and `deletions`, so callers ask the state what to do. Architecture notes that the enum now encodes the checkout's directory layout, which widens the `Workspace` port. Keep the enum's behaviour and move the path tables into `paths.py` or `workspace.py`, so neither lens loses out.
- **Safety vs Usability on stopping a cold session**: safety values stopping before trash piles up, at ~39 GB per run. Usability notes that the stop throws away completed ~7-minute runs. Analysing and reporting the runs collected so far as a stopped-early session would keep both.

### Strengths

- ✅ Deletion targets are fixed constants under `REPO_ROOT`. Symlinks and resolved escapes are refused before any move, and `verify` runs again before every timed run, with a test pinning this.
- ✅ Same-filesystem renames into `cold-trash-<n>/` keep each directory either whole or gone. `rmtree` does not follow inner symlinks, and `require_macos` keeps Linux CI off the real path.
- ✅ `BuildScope` became a `FullBuild | IsolatedTasks` union, and `BuildCacheState` carries `primes`, `touches` and `deletions`. New modes therefore arrive through polymorphism rather than flags.
- ✅ The task graph is parsed once, before the lock. Unknown `--isolate` names are refused early with a closest match, and `compare-builds --task` behaves the same way.
- ✅ `compare_records` is pure and returns named variant types rendered by exhaustive `match`. Zero medians and one-run records are handled without dividing by zero.
- ✅ Testing matches the risk. Real `tmp_path` filesystem tests cover `workspace.py`, event logs pin the session order, a golden worked example covers the comparison, and every `MeasurementConditions` field is parametrised.
- ✅ Errors are specific `MeasurementRefusedError` subclasses whose messages give the cause and the remedy. The CONTRIBUTING guidance states its security cost and its file-watching limit next to the benefit.

### General Findings

- 🔵 **Correctness**: The failure hook emits `DevLauncherNeedsRebuild` even when the session fails before any deletion, for example on re-verify or on `trash.mkdir`. Track whether a deletion happened.
- 🔵 **Documentation**: The lead paragraph of `tasks/README.md`'s "Build timing" section still describes only "default plus priming". That no longer holds for `--isolate` or `cold`.
- 🔵 **Documentation**: The `CONTRIBUTING.md` figure "369 test binaries" will go stale as 0363 and 0366 land. Moved here from inline because the line is outside the diff hunk.
- 🔵 **Documentation**: The `tasks/measure.py` module docstring still describes only warm-dispatch.
- 🔵 **Usability**: The one-line `measure:build` description in `mise.toml` is now too long to scan in `mise tasks`.
- 🔵 **Code Quality**: `comparison.py` mixes the model, the rules and the rendering. See Cross-Cutting Themes.
- 🔵 **Architecture**: The new `Workspace` port adds to the deferred `ports.py` consolidation. Move it together with `UnremovedTrash` and `FoundTrash`.

### Additional Findings

- 🔵 `tasks/shared/build_measurement/record.py:61` — `read_record` lets `UnicodeDecodeError` escape as a traceback (correctness)
- 🔵 `tasks/shared/build_measurement/session.py:366` — Stopping on unremoved trash discards the completed timed runs (usability)
- 🔵 `tasks/shared/build_measurement/session.py:351` — `_timed_runs` control flow is hard to follow (code-quality)
- 🔵 `tasks/shared/build_measurement/comparison.py:311` — The wall-clock change bypasses `_RecordPair.spread_change` (code-quality)
- 🔵 `tasks/shared/build_measurement/comparison.py:273` — Unit totals use reflection over `CargoUnits.model_fields` (code-quality)
- 🔵 `tasks/shared/build_measurement/comparison.py:273` — Unit totals are compared from run 1 only, unchecked and untested (correctness, test-coverage)
- 🔵 `tasks/shared/build_measurement/comparison.py:267` — Closest-task matching duplicates `TaskGraph.closest_to` (code-quality)
- 🔵 `tasks/shared/build_measurement/comparison.py:418` — Three comparison-report branches are never rendered in a test (test-coverage)
- 🔵 `tasks/shared/build_measurement/comparison.py:335` — Side names switch between previous/current and host-settings labels (usability)
- 🔵 `tasks/shared/build_measurement/comparison.py:329` — The comparison model is mixed with its text rendering (architecture)
- 🔵 `tasks/shared/build_measurement/reports.py:49` — Session-notice domain types live in the presentation module (architecture)
- 🔵 `tasks/shared/build_measurement/cache_states.py:4` — The cache-state enum encodes the checkout's concrete layout (architecture)
- 🔵 `tasks/measure.py:3791` — `compare-builds` exits 0 when conditions differ, so it cannot gate a script (usability)
- 🔵 `tasks/README.md:415` — The README should say that `compare-builds` takes record file paths (documentation)
- 🔵 `meta/work/0367-stop-rebuilding-unchanged-inputs.md:100` — Says cold deletes every `node_modules`, but it deletes only the frontend's (documentation)
- 🔵 `meta/work/0368-build-performance-measurement-and-host-guidance.md:15` — `last_updated` predates the 0370 edge it carries (standards)
- 🔵 `tasks/shared/build_measurement/errors.py:87` — `MissingTouchTargetError` also covers a target that is not a regular file (standards)
- 🔵 `tasks/shared/build_measurement/timeline.py:150` — A conditional expression inside a dict comprehension is dense (code-quality)
- 🔵 `tasks/shared/build_measurement/workspace.py:97` — Nothing checks for live consumers, such as the dev server or rust-analyzer, of the directories being deleted (safety)

---
*Review generated by /review-pr*

## Inline Comments

### `tasks/shared/build_measurement/session.py:383` — Unremoved trash is reported twice with contradictory advice
**Severity**: minor | **Confidence**: high | **Lens**: correctness

When a cold run's trash cannot be removed and more timed runs remain, `_stop_for_unremoved` prints `cold trash not removed: <path> (...): <cause>; delete it once the cause is fixed` and raises `TrashNotRemovedError`. The exit callback `_report_deletion_notices_on_failure` then calls `_deletion_notices(reported=())`. With the real `FilesystemWorkspace`, `leftover_trash()` globs `*/cold-trash-*`, which matches that same `cold-trash-<n>`. It is printed again as `cold trash left behind: <path> (...); safe to delete`. The test double's `found=()` default hides this.

**Impact**: The operator gets two contradictory instructions about the same directory, and the "safe to delete" message comes last.

**Suggestion**: Record the stopping `UnremovedTrash` on the session and pass it as `reported=` from the failure callback, as the success path already does with `last_unremoved`. Add a test whose double reports the same trash from `leftover_trash()`.

---

### `tasks/shared/build_measurement/analysis.py:59` — Neither guard on the isolated exit-status backfill is pinned by a test
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage

No test pins either guard on the new exit-status backfill: `if not self.runs_one_task` here, and `and figures.exit_status is None` in `Timeline.with_exit_status_unprinted_by_mise`. The only failing single-task fixture prints no status line, and every full-build failure fixture prints a status equal to the invocation's. Deleting either guard leaves the suite green.

**Impact**: A full-build failure whose task prints only `ERROR task failed` could take the whole `default` build's exit status. An isolated task's printed status could be overwritten. Either way the committed record would be wrong.

**Suggestion**: Add two `Timeline` or `CapturedInvocation.timeline()` tests. First, a multi-task invocation with a failed task that has no status line should keep `exit_status is None`. Second, a single-task invocation that prints status 2 while exiting 5 should keep 2.

---

### `tasks/shared/build_measurement/session.py:403` — The dev-launcher marker fallback is silent, duplicated and untested
**Severity**: minor | **Confidence**: high | **Lens**: test-coverage, code-quality

**Test Coverage**: `_deletion_notices` treats an `OSError` from `has_dev_launcher_marker()` as "marker present". No test reaches that branch, because `RecordingWorkspace.has_dev_launcher_marker` can only return a bool. Flipping `marker = True` to `False` leaves the suite green. Let the double raise, as `found_later` already can for `leftover_trash`, and assert the rebuild notice still appears and the original exception still propagates.

**Code Quality**: The rule "deletions plus marker means warn" is coded both here and in `Preflight.dev_launcher_notices`. The probe failure is also swallowed silently, while the trash-listing failure just above is logged. Extract one function over `(deletions, marker_present)` for both callers, and log the probe failure.

---

### `tasks/shared/build_measurement/workspace.py:68` — A failed second move leaves the first directory stranded in the trash
**Severity**: minor | **Confidence**: medium | **Lens**: safety, test-coverage

**Safety**: `prepare_timed_run` moves each deletion in turn and calls `remove_tree` only after every move succeeds. If the second `rename` fails (EXDEV, EBUSY or permissions), the exception escapes with `cli/target` already in `cold-trash-<n>/`. The trash is neither removed nor moved back. The exit hook lists it, so the failure is not silent, but tens of GB stay on disk and the checkout has lost its build cache without a measurement to show for it. Either roll back the directories already moved, or remove the trash, before re-raising.

**Test Coverage**: Only `RecordingWorkspace` exercises a partial-preparation failure. Add a `test_workspace.py` case on a real `tmp_path` that makes the second rename fail, for example with a read-only `cli/visualiser/frontend` skipped under root. It should assert the state left behind and that `leftover_trash()` reports it.

---

### `tasks/shared/build_measurement/settings.py:188` — The incompatible `--isolate`/`--cache-state cold` error gives no reason or alternative
**Severity**: minor | **Confidence**: high | **Lens**: usability

The refusal reads only `--isolate does not support --cache-state cold`. The harness's other refusals explain the cause and what to do instead.

**Suggestion**: Give the reason and the options, e.g. `…: a cold run deletes the caches the isolated task's skipped dependencies would rebuild; isolate with warm-no-op or incremental, or time the full build cold`.

---

### `tasks/README.md:419` — The noise rule omits the single-run "unjudged" case
**Severity**: minor | **Confidence**: high | **Lens**: documentation

The README says a change is within noise when the current median lies in the previous record's [min, max]. When the previous record has one run, the report prints `no spread to judge noise (1 run)`. The work items recommend `--runs 1` for cold records, so readers will meet this output.

**Suggestion**: Add "and is not judged when the previous record has a single run".

---

### `tasks/shared/build_measurement/workspace.py:97` — `_refuse` always raises but is typed `-> None`
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

In the code that guards deletions, a reader or type checker sees the symlink branch falling through to the `resolve()` check that follows. `session.py` already uses `NoReturn` for `_stop_for_unremoved`.

**Suggestion**: Annotate it `-> NoReturn`, or have a function build the error so each branch reads `raise self._refusal(path, why)`.

---

### `tasks/shared/build_measurement/comparison.py:346` — `_plural` is duplicated verbatim from `reports.py`
**Severity**: minor | **Confidence**: high | **Lens**: code-quality

This is a character-for-character copy of `reports._plural`. A change to how counts are pluralised has to be made twice, and the two reports can drift apart. Share one helper. This also argues for moving the comparison's rendering next to `reports.py`.

---

### `meta/work/0363-remove-repeated-work-from-the-rust-test-lanes.md:14` — Frontmatter edits miss `last_updated` and the usual key order
**Severity**: minor | **Confidence**: high | **Lens**: standards

The PR swaps this item's `blocked_by` for `blocks: ["work-item:0370"]` and rewrites its Dependencies section. `last_updated` stays at 2026-10-29, and the new `blocks:` sits after `schema_version` rather than after `parent:`, where 0362 and the other items keep relationship keys. 0362 and 0364 also have stale `last_updated`, while 0365 and 0366 were bumped.

**Suggestion**: Move `blocks:` to just after `parent:`, and bump `last_updated` and `last_updated_by` on every edited sibling.

---

### `meta/work/0367-stop-rebuilding-unchanged-inputs.md:17` — `blocks:` appended after `schema_version`, and `last_updated` not bumped
**Severity**: minor | **Confidence**: high | **Lens**: standards, documentation

This has the same misplacement as 0363: the new `blocks:` key belongs after `parent:`, where `blocked_by:` used to be. The frontmatter and the Dependencies section changed, but `last_updated` still reads 2026-10-30.

---

## Per-Lens Results

### Architecture

**Summary**: The PR is architecturally sound and keeps the package's ports-and-adapters style. The new `Workspace` port has a single filesystem adapter registered in the import-boundary test, isolation fits the `BuildScope` union, and the comparison is pure. The findings concern where things are placed within modules. The deferred structural items are acknowledged.

**Strengths**: `Workspace` sits behind a protocol, with `FilesystemWorkspace` wired in `wiring.py`, registered in `test_imports.py` and doubled by `RecordingWorkspace`. `BuildScope` became a union. The graph is parsed once, before the lock. `isolated_tasks` and `mode` reuse `Conditions.differences`. `compare_records` is pure. Cache-state behaviour lives on the enum.

**Comments**:
- `reports.py:49-54` (minor, medium) — The session-notice domain types are in the presentation module, and the notice concept is split across `reports.py` and `cold_trash.py`. Move them together into a `session_notices.py`.
- `comparison.py:329` (minor, medium) — The comparison model is mixed with its text rendering. Split the rendering into `comparison_report.py`.
- `cache_states.py:4-12` (suggestion, low) — The enum encodes the checkout layout. Pass `touches` and `deletions` to the port, or keep the path tables in `paths.py` or `workspace.py`.

**General Findings**: The `Workspace` port adds to the deferred `ports.py` consolidation (suggestion).

### Correctness

**Summary**: The new logic holds up. Re-verification happens before every move, cold-plus-isolate is refused early, and the comparison handles zero medians and one-run records. Three small problems remain.

**Strengths**: Re-verification before each move. Trash containment is checked. Zero-median and zero-count cases are handled. The `Measured` ⇒ spread invariant is enforced at parse time. The `match` statements are exhaustive. A single graph serves both the check and the runs. Preparation is excluded from the wall-clock.

**Comments**:
- `session.py:383` (minor, high) — Unremoved trash is reported twice with contradictory advice.
- `record.py:61-68` (minor, medium) — `UnicodeDecodeError` escapes `read_record`. Read as UTF-8 and catch the decode error.
- `comparison.py:273-289` (suggestion, low) — Unit totals come from run 1 only, and nothing checks that runs agree.

**General Findings**: The failure hook warns of a dev-launcher rebuild even when nothing was deleted (suggestion).

### Code Quality

**Summary**: The code is well made, with small frozen value types, exhaustive `match` and injected ports. The issues are minor: duplicated helpers and rules, dense control flow in `_timed_runs`, a `_refuse` typed `None` that should be `NoReturn`, and a 540-line `comparison.py`.

**Strengths**: `Workspace` and `remove_tree` are seams. The comparison has named variant types. Behaviour lives on the enum. `IsolatedTasks` works through polymorphism. The graph parameter threading was removed. Errors are specific.

**Comments**:
- `comparison.py:346-347` (minor, high) — `_plural` is duplicated.
- `comparison.py:311-316` (minor, medium) — The wall-clock change bypasses `spread_change`.
- `comparison.py:273-289` (minor, medium) — Unit totals are read by reflection over `CargoUnits.model_fields`.
- `comparison.py:267-270` (suggestion, medium) — The closest-match lookup duplicates `TaskGraph.closest_to`.
- `session.py:351-374` (minor, medium) — `_timed_runs` control flow is hard to follow.
- `session.py:403-407` (suggestion, medium) — The dev-launcher rule is decided twice, and the probe failure is not logged.
- `workspace.py:97-109` (minor, high) — `_refuse` should be `NoReturn`.
- `timeline.py:150-163` (suggestion, low) — A conditional inside a comprehension is dense.

**General Findings**: `comparison.py` mixes the model, the rules and the rendering (suggestion).

### Test Coverage

**Summary**: Coverage is strong and proportionate to the risk. The deletion code is tested against a real filesystem. The gaps are narrow: two exit-status guards, the marker `OSError` fallback, a few report branches, which run's unit totals are compared, and a partial-move failure in the real adapter.

**Strengths**: Real-filesystem `workspace.py` tests. `ObservedBuild` behaviour checks. Session ordering pinned by event logs. A golden worked example with noise boundaries. Conditions parametrised. The baseline test is now live. Golden digests cover the pulldown-cmark bump. Invoke-boundary CLI tests.

**Comments**:
- `analysis.py:59-63` (minor, high) — The exit-status backfill guards are unpinned.
- `session.py:404-407` (minor, high) — The marker `OSError` fallback is untested.
- `comparison.py:418-438` (suggestion, high) — Three report branches are never rendered.
- `comparison.py:273-275` (suggestion, medium) — Nothing pins which run's unit totals are compared.
- `workspace.py:119-125` (suggestion, medium) — A partial-move failure is tested only through the fake.

**General Findings**: None.

### Safety

**Summary**: The deleting path is well guarded for a local tool. Targets are fixed, symlink and escape checks run before every run, trash containment is checked, and the path is macOS-gated. Residual risks: a partial move leaves trash with no rollback, and nothing checks for live consumers. Only regenerable build output is at stake.

**Strengths**: Fixed deletion constants. Re-verification before each run. Atomic same-filesystem renames. `rmtree` does not follow inner symlinks. The session stops before trash accumulates. The exit hook never raises. `require_macos` keeps Linux CI off the real path.

**Comments**:
- `workspace.py:68-71` (minor, medium) — A failed second move strands the first directory in the trash.
- `workspace.py:97-103` (suggestion, low) — There is no check for a live dev server or rust-analyzer using the deleted directories.

**General Findings**: None.

### Usability

**Summary**: The new operator surfaces are ergonomic and consistent, with actionable refusals and closest-match suggestions. The friction is small.

**Strengths**: Consistent closest-match suggestions. Actionable destructive messages. Leftover trash is listed with its size. The comparison verdict comes first. The help text and README agree.

**Comments**:
- `settings.py:188-190` (minor, high) — The incompatibility error is terse.
- `measure.py:3791-3799` (suggestion, medium) — `compare-builds` always exits 0. Consider an opt-in `--require-matching-conditions`.
- `session.py:366-367` (minor, medium) — Stopping on unremoved trash discards the completed runs.
- `comparison.py:335-340` (suggestion, low) — The side names switch vocabulary.

**General Findings**: The `mise.toml` description for `measure:build` is too long (suggestion).

### Documentation

**Summary**: The documentation is thorough and mostly accurate. The gaps are small drifts.

**Strengths**: The CONTRIBUTING section is concise and states its costs. The README's comparison paragraph matches the output. The `--host-settings` cross-link. Work-item dependencies agree. Help strings match the code.

**Comments**:
- `tasks/README.md:419-422` (minor, high) — The noise rule omits the single-run case.
- `tasks/README.md:415-418` (suggestion, medium) — The README should say that the arguments are record file paths.
- `meta/work/0367…:100` (minor, medium) — Says cold deletes every `node_modules`.
- `CONTRIBUTING.md:18-20` (suggestion, medium) — The hard-coded "369 test binaries" figure. Moved to general because the line is outside the diff hunk.

**General Findings**: The Build-timing lead paragraph overstates (minor). The `tasks/measure.py` docstring covers only warm-dispatch (suggestion). The `last_updated` on 0367 was not bumped (suggestion).

### Standards

**Summary**: The Python code follows the package conventions. The gaps are in work-item metadata.

**Strengths**: The `measure:compare-builds` naming pair. Named `MeasurementRefusedError` subclasses. The `Workspace` Protocol mirrors `BuildGroup`. Flags are documented and pinned by tests. The confusables relaxation is justified inline. The 0370 frontmatter shape is right.

**Comments**:
- `meta/work/0363…:17` (minor, high) — `blocks:` is out of order.
- `meta/work/0367…:17` (minor, high) — `blocks:` is out of order.
- `meta/work/0363…:14` (minor, high) — `last_updated` was not bumped (also on 0362, 0364 and 0367).
- `meta/work/0368…:15` (minor, medium) — `last_updated` predates the 0370 edge.
- `errors.py:87` (suggestion, medium) — The `MissingTouchTargetError` name is too narrow.

**General Findings**: None.

---
*Review generated by /accelerator:review-pr*
