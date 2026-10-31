---
type: "work-item"
id: "0368"
title: "Build-performance measurement and host guidance"
date: "2026-10-28T16:18:46+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "task"
priority: "high"
parent: "work-item:0361"
blocks: ["work-item:0362", "work-item:0363", "work-item:0364", "work-item:0365", "work-item:0366", "work-item:0367"]
relates_to: ["work-item:0369", "codebase-research:2026-10-30-0368-build-performance-measurement-harness"]
source: "codebase-research:2026-10-28-full-build-duration-contention-and-slowness"
tags: ["build-system", "performance", "tooling", "documentation", "macos"]
last_updated: "2026-10-30T17:19:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0368: Build-performance measurement and host guidance

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

The research measured the build with three ad-hoc scripts kept outside the
repository. Turning them into a `measure:build` task, and recording a warm
no-op baseline as the median of three runs, lets every other item in the
epic show its saving. Alongside it, documenting the host settings expected
to cut background contention should give contributors a faster local build
without weakening any of the build's checks; Requirement 9 measures by how
much. The Developer Tools exemption does trade away XProtect scanning of
anything the exempted app launches, and the guidance says so.

## Context

The research harness is three scripts:
- **`timed-run.sh`** runs `mise run --output prefix` with every line
  timestamped, samples `iostat` and per-process cumulative CPU, and records
  wall-clock.
- **`iso.sh`** runs each task alone with `--skip-deps`.
- **`analyse.py`** reports each task's duration, queueing delay, cargo
  `Compiling`/`Checking` counts and lock waits, plus a 30-second profile of
  CPU busy and running tasks.

Their source is in the research document's appendix.

On the research machine, two host processes reacted to what the build wrote:
- **`syspolicyd`**, the XProtect/Gatekeeper assessment of new executables,
  used 44–78 CPU-s per run.
- **IntelliJ** used 270–380 CPU-s per run.

Each coverage run relinks 369 test binaries, and the bootstrap suites copy
binaries into temporary installations. After a build, both processes stayed
above 80% of a core for minutes. These effects were observed but not
separated out by experiment.

Web research since adds three facts:
- **XProtect scans each new executable on first launch**, serially on a
  single thread, at about 100–300 ms each on recent macOS. The scan runs in
  `XprotectService`, with `syspolicyd` brokering the assessment. A freshly
  linked or copied binary therefore costs launch latency, not just
  background CPU.
- **The Developer Tools exemption follows the responsible app.** The
  exemption is the Developer Tools list under System Settings › Privacy &
  Security. macOS attributes each launched process to a responsible app,
  so the exemption covers every descendant of the listed app (terminal or
  IDE). It takes effect only after that app restarts, and needs separate
  entries for tmux and zellij.
- **A JetBrains exclusion stops indexing, not file watching.**
  `fsnotifier`, the IDE's native file watcher, watches all of `/` on macOS
  and the IDE filters events itself (IJPL-2175, open since 2011). RustRover
  excludes `<workspace>/target` only for an attached Cargo project.

## Requirements

1. Add a `measure:build` task, reimplementing the research harness, that
   runs a full `default` build with `--continue-on-error` and reports:
   - per task: duration, mise's `Finished in` figure when the task prints
     one and otherwise its first to last output line; queueing delay, the
     time from the latest end of its dependencies to its first output
     line, mostly spent waiting for a mise job slot plus any start-up
     before that line; cargo `Compiling`, `Checking` and `Documenting`
     unit counts; lock wait, the time from each `Blocking waiting for file
     lock` line to the task's next line, or to its end if none follows;
     and, where its output has nextest's `Summary [ <n>s]` line, that
     duration as its nextest phase;
   - CPU busy (user plus system CPU, from `iostat`) and the running tasks,
     in 30-second buckets across the whole run;
   - per timed run, the CPU-seconds that `syspolicyd`, `XprotectService`,
     the IDE and `fsnotifier` used during the build: the increase in `ps`
     TIME, summed across every matching PID. A PID present in the build's
     first sample counts from that sample; a PID first seen later counts
     from zero, since it started during the build.

   A task's end is its start plus its duration. A task with no
   dependencies is ready at the run's start, and a dependency that printed
   nothing ends when its own dependencies end.

   The IDE is the IntelliJ IDEA and RustRover main processes, unless the
   operator overrides the name patterns. A process never seen during a
   run's build is recorded as absent for that run, not 0.

   Every task that ran is reported with its exit status. Under
   `--continue-on-error`, mise still runs a failed task's dependants, so
   each task downstream of a failure in the `default` DAG is flagged as
   having run after a failed dependency. A task mise skips because its
   sources are up to date is marked skipped, with no duration; for its
   dependants' queueing delay it ends at its skip line. The task exits 0
   whenever it writes a record, printing a summary that names any failed
   task, and non-zero only when it cannot measure.

   The record lists fetch failures as `fetch_failures`: each line of a
   timed run's output that matches a fetch-failure pattern for an
   external input named in Dependencies, with its task and run. The
   patterns cover the launcher download's `curl` errors, `deny:check`
   failing to fetch the advisory database, and npm network errors such as
   `ETIMEDOUT`, `ECONNRESET`, `ENOTFOUND` and `EAI_AGAIN`. The summary
   warns when the list is not empty.
2. Repeat runs: record idle load as the mean CPU busy over 30 s of
   `iostat`, then one priming run unless the cache state skips it, then N
   timed runs (default 3). Keep the priming run's wall-clock in the
   record, marked as discarded. Report the median, min and max of each
   task's duration and nextest phase, of the wall-clock and of each
   tracked process's CPU-seconds. A task skipped in every timed run is
   reported as skipped. Any other task that failed, ran after a failed
   dependency or was skipped in any timed run gets per-run values and a
   flag saying which, but no median, min or max. Each run containing a
   failure is marked, and the wall-clock median still covers every timed
   run. A tracked process absent from every timed run is absent for the
   record; one absent from only some gets per-run values and a flag but
   no median, min or max, as a failed task does.
3. Isolation mode: run named tasks alone with `--skip-deps`, one at a
   time, under Requirement 2's priming and repeat rules, reporting
   Requirement 1's per-task figures and each task's start and end in each
   run. It supports the warm no-op and
   incremental states only; asked for cold, it exits non-zero before
   deleting anything.
4. Cache states, each set up as follows:
   - warm no-op (default; no source change since the previous build): a
     priming run, then nothing before each timed run;
   - incremental: a priming run, then a touch of `cli/kernel/src/lib.rs`
     before each timed run, leaving its contents unchanged;
   - cold, behind an explicit flag because it is destructive: no priming
     run, and before each timed run, delete `cli/target` and every
     `node_modules` that `deps:install:node` installs. Cold deletes only
     those directories and leaves the build's own freshness rules alone,
     so a task whose outputs are still current, such as an up-to-date
     `dist/` once 0367 lands, is skipped.
5. Write each record as JSON, with the metadata needed to compare runs,
   beside its raw logs in a gitignored run directory. With `--keep`, also
   write it under `meta/measurements/` for the operator to commit, unless
   the record holds a fetch failure; the harness never commits. That holds
   for every `--keep` run, not only baseline and condition records: under
   jj a kept file joins the working-copy commit at once, and a
   fetch-failed record is never one to commit. Keep the baseline, the three host-settings
   condition records, and any record a consumer's acceptance criterion
   cites. The metadata includes a host-settings label, given with
   `--host-settings` as `none`, `devtools` or `devtools+ide-exclusion`,
   and `unspecified` by default. The recorded commit is the nearest
   commit on the first-parent chain behind the working copy that differs
   from its own first parent outside `meta/measurements/`. The working
   copy matched it when its tree outside that directory equals the
   recorded commit's. A kept record, committed or not, therefore never
   changes the commit the next run names.
6. Support macOS only.
7. Record a warm no-op baseline before the first speed change merges, from
   a clean working copy on the reference host: the research workstation,
   an Apple M4 Max (12P + 4E cores, 128 GB) on macOS 26.3, labelled with
   the host settings in force rather than `unspecified`. Two records
   compare only when they share the machine (its hostname as well as its
   model), OS version, mise job limit, cache state and host-settings
   label. When any of these changes, such as after an OS update or a
   settings change, re-measure the baseline commit under the new
   conditions. Each record also lists its task set, the `default` closure
   from `mise tasks --json` at the measured commit, which the epic's own
   changes alter and so is reported rather than matched.
   Retake, rather than keep, a baseline or condition record whose
   `fetch_failures` list is not empty. Other failures, such as a newly
   published advisory failing `deny:check`, do not stop a record being
   kept: the record marks the task failed and gives it no median.
8. Document the host settings in `CONTRIBUTING.md`:
   - add the terminal, the IDE, and any tmux or zellij to Developer Tools,
     then restart them;
   - confirm `cli/target` and each `node_modules` are excluded in the IDE;
   - state that the exemption stops XProtect scanning anything the
     exempted app launches;
   - state that an IDE exclusion stops indexing but not file watching.
9. Measure the host settings and commit the records, then cite them in
   `CONTRIBUTING.md`:
   - record `none`, then `devtools`, then `devtools+ide-exclusion`, each a
     warm no-op `--runs 3` on the reference host at one commit;
   - take them once 0362, 0363 and 0367 have each landed or been
     abandoned, since those remove most of what the settings save;
     Requirement 8's steps may land earlier;
   - cite each step's effect as the change from the previous condition in
     median wall-clock and in each tracked process's median CPU-seconds,
     both in seconds and as a ratio, stating the IDE exclusion's effect
     as on top of the exemption;
   - where a condition's median lies within the previous condition's
     [min, max], for wall-clock or for a tracked process's CPU-seconds,
     say that figure's change was not distinguishable from noise on the
     reference host;
   - name a change to or from absent in words, for example
     "`XprotectService` did not run in any `devtools` run", with no ratio
     and no noise verdict;
   - retake a condition record in which `syspolicyd`, the IDE or
     `fsnotifier` is absent from any timed run.
10. Keep `measure:build` out of the `check` and `default` closures.

Out of scope: Linux support, Spotlight exclusions, IDEs other than
JetBrains, and gating `check` or CI on any figure.

## Acceptance Criteria

- [ ] Given a macOS host and a warm workspace, when `mise run measure:build`
      runs with `--keep` and no fetch failure, then it writes a JSON record to its run
      directory and under `meta/measurements/` with,
      per task, duration, queueing delay, cargo `Compiling`, `Checking`
      and `Documenting` unit counts and lock-wait seconds; CPU busy and
      running tasks in 30-second buckets; the wall-clock; and, per timed
      run, the CPU-seconds of `syspolicyd`, `XprotectService`, the IDE and
      `fsnotifier`.
- [ ] Given fixture logs with known answers, when the analysis runs, then
      it reports the stated values for:
      - duration from a `Finished in` line, and from first to last line
        where none is printed;
      - a failed task's exit status from mise's end-of-run
        `exited with status <n>` line, which does not extend its
        duration;
      - a task skipped as up to date, marked skipped with no duration,
        and the tasks downstream of a failure, flagged from the `default`
        DAG;
      - queueing delay, including for a task with no dependencies and for
        one whose dependency printed nothing;
      - lock wait, including a wait still open at the task's last line;
      - the count of each of `Compiling`, `Checking` and `Documenting`,
        and a nextest phase from a `Summary [ <n>s]` line;
      - a fetch failure from each external input, and a non-network
        failure that leaves `fetch_failures` empty;
      - each process's CPU increase, for a PID in the build's first sample
        and for one first seen mid-run;
      - a process missing from every sample, recorded as absent, and one
        missing from one run of three, flagged with no median, min or
        max;
      - CPU busy and running tasks per 30-second bucket, and the idle-load
        mean;
      - the median, min and max over three runs.
- [ ] Given a completed run, then its raw logs and record are in a
      directory that `.gitignore` excludes, and the record names that
      directory.
- [ ] Given no `--keep`, when it runs, then nothing is written under
      `meta/measurements/`, and in either case the harness makes no
      commit.
- [ ] Given `--keep` and a fetch failure, when it runs, then nothing is
      written under `meta/measurements/`, the run directory still holds
      `record.json`, and the summary says the record was not kept.
- [ ] Given an injected task that exits non-zero mid-run, when the run
      completes, then the record marks it failed with no median, reports
      every task that ran with its exit status, and flags each task
      downstream of it as having run after a failed dependency, with no
      median. The harness exits 0, although `mise` exited non-zero, and
      prints a summary naming the failed task.
- [ ] Given `--runs 3`, when it runs, then it records the priming run's
      wall-clock marked as discarded, the per-run values, and the median,
      min and max of the wall-clock, of each tracked process's
      CPU-seconds, and of the duration and nextest phase of each task that
      ran, without failing or running after a failure, in every timed
      run.
- [ ] Given three timed runs in which one task fails in the second run
      only, then the record keeps that task's three per-run values and
      flags it failed with no median, min or max, marks the second run as
      containing a failure, and still gives the wall-clock median, min and
      max over all three runs.
- [ ] Given no `--runs`, when it runs, then the record holds three timed
      runs.
- [ ] Given task names in isolation mode, when it runs, then each runs
      alone with `--skip-deps`, their start-to-end intervals do not
      overlap, and the record gives each one's per-run duration, nextest
      phase where its output has one, and exit status, with the median,
      min and max of the duration and nextest phase.
- [ ] Given isolation mode and the cold flag, when it runs, then it exits
      non-zero and `cli/target` and every `node_modules` still exist.
- [ ] Given the incremental state, when it runs, then
      `cli/kernel/src/lib.rs` was touched before each timed run and its
      contents are unchanged.
- [ ] Given no cold flag and a sentinel file placed in `cli/target`
      beforehand, when it runs, then the sentinel still exists. The build
      would recreate a deleted `cli/target` or `node_modules`, so only a
      file the build never writes shows that the harness deleted nothing.
- [ ] Given the cold flag, when it runs, then `cli/target` and every
      `node_modules` that `deps:install:node` installs were deleted before
      each timed run, `deps:install:node` ran in every timed run, the
      record holds no priming run, and its cache state is `cold`.
- [ ] Given any record, then it states the commit, whether the working copy
      matched it, hostname, host model, OS version, core count, mise
      version and job limit, idle load before the priming run, cache
      state, host-settings label, and its task set: the `default`
      closure from `mise tasks --json` at the measured commit.
- [ ] Given a record kept by a previous run, whether or not it has since
      been committed, when the next run starts, then it reports the same
      commit and a matching working copy.
- [ ] Given a change to a file outside `meta/measurements/`, when it runs,
      then the record states that the working copy did not match. This
      and the previous criterion hold in both a jj workspace and a git
      repository.
- [ ] Given a non-macOS host, when it runs, then it exits non-zero naming
      macOS as the supported platform and writes no record.
- [ ] Given the epic, when the first of 0362–0367 merges, then
      `meta/measurements/` already holds a warm no-op `--runs 3` baseline
      taken on the reference host, from a clean working copy, at a commit
      containing none of their changes, with an empty `fetch_failures`
      list and a host-settings label other than `unspecified`.
- [ ] Given the three host-settings condition records, then they are
      labelled `none`, `devtools` and `devtools+ide-exclusion`, are
      comparable under Requirement 7 apart from that label, share one
      commit and `--runs 3`, were taken at a commit
      after 0362, 0363 and 0367 each landed or were abandoned, have empty
      `fetch_failures` lists, and show `syspolicyd`, the IDE and
      `fsnotifier` present in every timed run.
- [ ] Given `CONTRIBUTING.md`, when a contributor reads it, then it gives
      the steps for both settings, states the exemption's security
      tradeoff and that IDE exclusion does not stop file watching, and
      cites each step's change in median wall-clock and per-process
      CPU-seconds, in seconds and as a ratio, from the three condition
      records, naming the host. It states the IDE exclusion's effect as
      on top of the exemption, and says which changes were within noise:
      those where the condition's median lies within the previous
      condition's [min, max]. A change to or from absent is named in
      words, with no ratio or noise verdict.
- [ ] Given `check` and `default`, when their task closure is computed,
      then neither reaches `measure:build`, as enforced by the
      transitive-closure guard in `tests/unit/tasks/test_mise.py`.
- [ ] Given the orchestration criteria above (failures, repeat runs and
      priming, cache-state setup, sentinels, isolation mode, `--keep`, the
      kept-record commit and the non-macOS refusal), then each is
      verified against a stub build and stub samplers. Only the
      end-to-end `--keep` record, the baseline and the three condition
      records need real runs on the reference host.

## Open Questions

None.

## Dependencies

- Blocked by: none. Requirement 9 alone waits until 0362, 0363 and 0367
  have each landed or been abandoned. That is not a `blocked_by` edge,
  since this item blocks those three and only one requirement waits on
  them.
- Blocks: 0362, 0363, 0364, 0365, 0366 and 0367, through Requirements 1,
  2, 5 and 7 alone: the warm full-run path of the `measure:build` harness
  and its warm no-op baseline. The edge gates merging only, and is
  discharged once the baseline is recorded, whether or not this item is
  done, except for two consumers: 0365 also waits for Requirement 3 and
  0366 for Requirement 4 (see Consumers). Isolation mode and the
  incremental and cold states may land after the baseline, and the host
  guidance (Requirements 8–9) blocks nothing.
- Consumers: 0365 compares the nextest phase of an isolated
  `test:unit:cli` (Requirements 1 and 3); 0366 counts cold compile units
  (Requirement 4), comparing cold records before and after its change.
  Each mode must land before its consumer measures with it, so 0365
  keeps its `blocked_by` edge until Requirement 3 merges as well as the
  baseline, and 0366 until Requirement 4 does. Each consumer takes its
  own before-change measurement with `measure:build`; a missed one can
  be re-taken at the commit before the change.
- External: GitHub releases (the hooks smoke test downloads the published
  launcher for any commit before 0362, including re-measurements of the
  baseline commit, so that commit's release must stay published until
  0361 closes), the RustSec advisory database
  (`deny:check`) and, in cold runs, the npm registry. A baseline or
  condition record whose `fetch_failures` list is not empty is retaken
  (Requirement 7).
- Related: 0361, whose 200 s target compares its final warm no-op run
  against this item's baseline. That comparison holds only between records
  comparable under Requirement 7. It lists the task-set difference between
  the baseline and the final record, naming the child or other change
  behind each task added or removed.
- Related: 0367, whose freshness guard on `deps:install:node` must treat
  a deleted `node_modules` as stale, or a cold run would skip the
  reinstall and its frontend tasks would fail.
- Related: 0369, which changes what `default` runs and so what
  `measure:build` times. 0369 is blocked by 0361, so it merges only
  after the epic's closing comparison. An out-of-order merge would still
  show as tasks in the closing comparison's task-set difference that no
  child of 0361 accounts for.

## Assumptions

- `measure:build` follows the existing `measure:*` convention in
  `tasks/README.md`: it is operator-run, outside `check` and the bare
  `default` task. It runs a full `default` itself, and it records idle
  load so a reader can judge background activity, rather than requiring
  a quiet host.
- The host guidance targets JetBrains IDEs, the only IDE observed; other
  IDEs are neither documented nor measured.
- Ratios between the host-setting conditions carry over to other Macs;
  absolute figures do not.

## Technical Notes

- Harness source: the research document's "Appendix: Measurement Harness".
- Existing `measure:*` tasks and their conventions: `tasks/README.md`
  ("The measure namespace").
- The `test_mise.py` guard keys on the `run` string `invoke measure.` and
  the module `tasks/measure.py`. A `measure:build` task placed elsewhere
  must extend the guard.
- Reimplementing the harness in Python drops `ts`, `gdate` and `bc`: stamp
  each line as it is read from the `mise` subprocess, sample
  `ps -Ao pid=,ppid=,time=,command=` each second, and take CPU busy from
  `iostat -w 2`.
- Keep the log analysis pure and unit-tested over fixture logs, as
  `tasks/shared/measurement.py` is. Capture the fixtures from real runs.
- Run the build, the `iostat` and `ps` samplers and the platform check
  through replaceable commands. By default the build is
  `mise run --output prefix --continue-on-error default`, or
  `--skip-deps <task>` in isolation mode. Tests substitute a stub build
  that follows a scripted per-run plan (which tasks run, what they
  print, which fails in which run, which is skipped as up to date) and
  records the workspace it sees at each run's start: sentinels, the
  `cli/kernel/src/lib.rs` mtime and the `node_modules` present. That
  makes per-run setup observable without a full build. Like mise, the
  stub still runs a failed task's dependants.
- Under `--output prefix`, mise 2026.10.8 prints `[t] $ <run>` when a
  task spawns, `[t] Finished in <n>ms` or `<n>s` when it succeeds, and
  `[t] sources up-to-date, skipping` when it skips it. A failed task
  prints no `Finished in`. Its `exited with status <n>` and
  `ERROR task failed` lines come only after the run-level `Finished in`,
  so they give its status but not its end. A roll-up with no `run`
  prints nothing. `mise run` exits with the failed task's status even
  under `--continue-on-error`. The behaviour is version-specific and
  mise is unpinned, hence the recorded mise version.
- One-second sampling misses short-lived processes, and the post-build
  tail, when `syspolicyd` and the IDE stay busy for minutes, is not
  counted. Per-process CPU is therefore a lower bound.
- jj snapshots every file that `.gitignore` does not exclude, so a record
  written under `meta/measurements/` joins the working-copy commit. That
  is why records default to the gitignored run directory and only
  `--keep` writes there. It is also why, under jj, the search for the
  recorded commit starts at the working-copy commit's parent: the
  working-copy commit's ID changes on every snapshot, while its
  ancestors can be checked out and re-measured. Under git the search
  starts at `HEAD`. Either way, a commit that only adds records is
  skipped. A merge counts by its difference from its first parent:
  jj's `files()` revset treats a clean merge as changing nothing, so a
  search built on it names the commit before a PR merge and reports a
  clean checkout of `main` as not matching.
- A cold run at the default of 3 takes about 25 minutes and fetches npm
  packages each time. Cargo unit counts do not vary between runs, so
  `--runs 1` is enough for 0366's counts.
- Record the no-settings condition first: undoing the Developer Tools
  exemption can need a reboot. Keep the IDE open on the project for every
  condition, or its CPU never shows.

## Drafting Notes

- Grouped C1 and C2, agreed in conversation. `measure:build` is also how the
  effect of the host settings gets measured.
- Kept C1 and C2 in one item after review suggested splitting them, since
  only C1 gates the siblings. The gate does not depend on this item's
  status: each sibling drops its `blocked_by` edge once the baseline is
  recorded. One item keeps the instrument and the measurement it was built
  for together, so the guidance's figures come from the same harness, in
  the same record format, as the baseline.
- Kept `measure:build` out of `check` and `default`. This is an
  interpretation drawn from the existing `measure:*` convention; the
  research does not say so.
- Chose the research workstation as the reference host, answering 0361's
  open question. It produced every figure the epic cites, so ratios stay
  comparable. A baseline commit can be re-measured later, so the host's
  settings need not be frozen until the epic ends.
- Settled the record location by precedent: `measure:warm-dispatch` commits
  its records under `meta/measurements/`.
- Added `XprotectService` to the attributed processes: it does the
  scanning, while `syspolicyd`, which the research observed, brokers it.
- Added cold and incremental cache states because the research measured
  all three and 0366 counts cold compile units. Cold is behind a flag
  because it is destructive.
- The harness exits 0 when a measured task fails, because a live input
  such as the advisory database can fail a task in any run; a non-zero
  exit would make such a measurement look broken. When the research ran,
  every run failed `deny:check` on RUSTSEC-2026-0414; pinning
  pulldown-cmark to 0.13.5 has since cleared it. The record and summary
  carry the failure.
- Replaced "not run" after probing mise 2026.10.8: under
  `--continue-on-error` it still runs a failed task's dependants, so no
  task in a real `default` run is left unrun. Those tasks are flagged
  and given no median instead, since they ran against a failed task's
  outputs.
- Added the skipped state: mise skips a task whose `sources` are up to
  date, as the four docs tasks in `default` do today and
  `build:frontend` and `deps:install:node` will once 0367 lands.
- The harness records idle load rather than refusing a busy host. The
  research host idled at 22–30%, so a quiet-host gate would refuse the
  machine that produced every figure.
- Measured the settings cumulatively (neither, exemption, both) rather than
  as a full 2×2 grid, which keeps it to about 9 timed runs. The IDE
  exclusion's effect without the exemption is therefore not isolated.
- Left Spotlight exclusions out: no credible measurement of their effect
  on Rust or Node builds was found.

## References

- Source: `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
- Implementation research, including the mise probes:
  `meta/research/codebase/2026-10-30-0368-build-performance-measurement-harness.md`
- Related: 0361, 0362, 0363, 0364, 0365, 0366, 0367, 0369
- Precedent: 0189 (`measure:warm-dispatch` records under
  `meta/measurements/`)
- nextest, macOS installation notes:
  https://nexte.st/docs/installation/macos/
- Cargo PR #15908, slow first run of new binaries on macOS:
  https://github.com/rust-lang/cargo/pull/15908
- Nicholas Nethercote, "Faster Rust builds on Mac" (September 2025):
  https://nnethercote.github.io/2025/09/04/faster-rust-builds-on-mac.html
- JetBrains IJPL-2175, excluded folders still watched:
  https://youtrack.jetbrains.com/issue/IJPL-2175
- RustRover help, project analysis and `target` exclusion:
  https://www.jetbrains.com/help/rust/2025.3/project-analysis.html
