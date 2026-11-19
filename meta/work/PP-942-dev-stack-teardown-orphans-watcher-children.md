---
type: "work-item"
id: "PP-942"
title: "Dev-Stack Teardown Orphans Watcher Children"
date: "2026-11-15T23:06:54+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "low"
parent: "work-item:PP-953"
derived_from: ["plan:2026-11-15-0364-parallelise-the-serial-test-suites"]
relates_to: ["work-item:PP-956", "work-item:PP-123"]
external_id: "PP-942"
aliases: ["0371"]
tags: ["build-system", "dev-task", "circus", "teardown"]
last_updated: "2026-11-15T23:06:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-942: Dev-Stack Teardown Orphans Watcher Children

**Kind**: Bug
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

`dev:stop` can leave a watcher's children running after the watcher itself
has gone. It happens on two paths: when a watcher was SIGKILLed out of band,
and when `dev.json` was lost while the arbiter was still reachable. Both
break 0101's promise that no process in a watcher's group survives a stop.

## Context

0364's Phase 1 made the dev integration suite record every fake's SIGTERM
receipt and assert that no recorded fake survives a stop. That exposed two
paths on which children of the server and frontend watchers are orphaned:

1. **Out-of-band kill.** When a watcher is SIGKILLed outside our code, its
   children are reparented. `reap_descendants`
   (`tasks/shared/dev/lifecycle.py`) lists the children of recorded PIDs
   only, so it can no longer reach them.
2. **Lost `dev.json`.** `do_stop` falls back to `_stop_without_state`, which
   has no recorded watcher PIDs. Circus's `stop_children` relies on
   `send_signal_process` → `send_signal_child`, which lists the worker's
   children again only *after* signalling the worker. A watcher that exits
   promptly on SIGTERM therefore leaves its children unsignalled. On the
   normal path, `reap_descendants` hides this race by reaching the children
   through the recorded PIDs.

The suite found this; nobody has reported it in real use. npm usually
forwards SIGTERM and waits for its child, which narrows the second path in
practice, but the gap is still real.

Circus already starts every watcher with `os.setsid()`, so each watcher
leads its own process group and its children inherit it. A group outlives
its leader, so it can still be signalled after the watcher is gone.
`reap_session` in `tasks/shared/processes.py` already reaps exactly that
kind of group.

To keep the suite green, 0364 worked around the gap in its tests: test 10
(`test_discovery_survives_lost_state_file`) runs childless watchers, and
tests 11 and 15 SIGKILL a watcher's whole subtree instead of just the
watcher.

## Requirements

1. **Out-of-band kill.** When a watcher has been SIGKILLed outside our code
   and its children are still alive, `dev:stop` reaps the children through
   the watcher's process group. It sends SIGTERM first, and SIGKILL only to
   what outlives the grace period.
2. **Lost `dev.json`.** When the arbiter is still reachable, `dev:stop` gets
   the watcher PIDs from it before sending `quit`, then reaps their process
   groups as in Requirement 1.
3. **Reuse `reap_session`.** Reuse `reap_session` (or the group primitives
   behind it) instead of writing new reaping code, unless it turns out not to
   fit. If it doesn't fit, record why.
4. **Undo 0364's workarounds.**
   - Test 10 runs child-spawning watchers again.
   - Tests 11 and 15 SIGKILL only the watcher process.

## Acceptance Criteria

- [ ] **Watcher killed out of band.** Given a running dev stack whose server
      (or frontend) watcher has children, when the watcher alone is SIGKILLed
      and `dev:stop` runs, then no member of that watcher's process group
      survives, and each child received SIGTERM before any SIGKILL.
- [ ] **Arbiter and watchers killed.** Given the arbiter and both watchers
      have been SIGKILLed but their children are alive, when `dev:stop` runs,
      then no child survives.
- [ ] **Lost `dev.json`.** Given a running dev stack whose `dev.json` has
      been deleted, when `dev:stop` runs, then every watcher child received
      SIGTERM and none survives.
- [ ] **Dev suite.** Given the dev integration suite, when it runs:
  - test 10 runs with child-spawning watchers;
  - tests 11 and 15 SIGKILL only the watcher process;
  - all three pass `assert_stopped_cleanly`.
- [ ] **Unit tests.** Given `tests/unit/tasks/test_dev.py`, when it runs,
      then unit tests with fake process-group ops cover:
  - a watcher group that outlives its leader;
  - watcher PIDs learned from a live arbiter when no state is recorded.

## Open Questions

- If `reap_session` is used, its group-wide SIGKILL may appear in the dev
  suite's escalation record as "our code SIGKILLed". There are two options:
  - accept a group SIGKILL in the clean-stop check once every member has
    logged SIGTERM;
  - have the reap skip the group SIGKILL when the group is already empty.

  The implementer can settle this.

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- **Children stay in the watcher's group.** That holds for the suite's fakes
  and for npm → node → esbuild. It fails for a child that calls `setsid`
  itself. `reap_session` covers that case through its descendant snapshot,
  but only while the leader is alive.
- **The arbiter reports the right PIDs.** Before `quit`, the arbiter's
  `pids` command for each watcher returns the PIDs the watchers would have
  recorded.

## Technical Notes

- **Circus.** Watchers are started with `os.setsid()` in
  `circus/process.py`'s preexec. The child-signalling race is in
  `circus/watcher.py` (`send_signal_process`) and `circus/process.py`
  (`send_signal_child`).
- **Reaping helpers.** `reap_session` and `_reap_leaderless_group` live in
  `tasks/shared/processes.py`. `build_measurement/build_runner.py` already
  uses `reap_session`.
- **The killer's type.** `DevDeps.killer` is typed `ProcessOps`. Group
  reaping will probably widen it to `ProcessGroupOps`. The dev suite's
  `EscalationRecorder` (`tests/integration/dev/dev_integration_driver.py`)
  would then have to wrap `signal_group` as well.
- **Where the PIDs come from.** The lost-state path could take the watcher
  PIDs from `Supervisor.pids(name)`, which `_record_watcher_pid` already
  uses.

## Drafting Notes

- **Kind and priority.** Filed as a low-priority bug: the suite found it,
  nobody has hit it, and it needs an out-of-band kill or a lost state file
  to show up.
- **Relation to 0101.** 0101 (done) is background and is not reopened. Its
  unticked teardown criteria (no surviving process in the watcher groups, no
  orphaned node or server processes) are what this item delivers.
- **Approach.** Group-based reaping through `reap_session` is a mild
  preference, not a constraint.
- **Out of scope.**
  - Lost `dev.json` **and** an unreachable arbiter: nothing records the PIDs
    or groups, so nothing can be reaped.
  - Changing or patching circus itself.
  - Production defaults (`grace_kill`, `grace_quit`) and any other part of
    the stop sequence.

## References

- Source: `meta/plans/2026-11-15-0364-parallelise-the-serial-test-suites.md`
  (Phase 1, the negative-path record's known gaps)
- Related: 0364, 0101
