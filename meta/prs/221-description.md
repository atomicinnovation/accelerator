---
type: "pr-description"
id: "221"
title: "[0364] Shorten the dev suite's pure-delay waits"
date: "2026-11-16T00:26:57+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:PP-956"
relates_to: ["plan:2026-11-15-0364-parallelise-the-serial-test-suites"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/221"
pr_number: 221
tags: ["build-system", "testing", "pytest", "dev-task", "performance"]
revision: "6954a3482cb01e29da764db005a0ff7d2e57fec6"
repository: "accelerator"
last_updated: "2026-11-16T00:26:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Shorten the dev suite's pure-delay waits

## Summary

This PR implements Phase 2 of the 0364 plan. It is stacked on #220, which
made the dev integration suite's escalations visible. It lowers the
driver's two pure-delay budgets and replaces the remaining fixed settle
sleeps with condition waits. The serial suite now takes **61 s**, down
from 135–139 s after #220 and from 163 s before 0364 started.

The shorter waits are safe because of #220. Every happy-path stop now
asserts that nothing escalated, so a grace that is too short fails a test
instead of passing unnoticed.

## Changes

### Driver defaults (`dev_integration_driver.py`)

- **`probe_timeout` 5.0 → 2.0 s.** This is the production value, and it
  already bounds live arbiter calls in real use. Each probe of a dead
  arbiter costs the full `probe_timeout`.
- **`grace_kill` 3.0 → 0.5 s.** `reap_descendants` sleeps the whole
  `grace_kill` on every clean teardown, even though the fakes are already
  dead by then.
- `pidfile_timeout`, `readiness_timeout`, `frontend_active_timeout` and
  `grace_quit` are unchanged. Negative-path tests keep their explicit
  values.
- The comment above the defaults now says only what is still true:
  start-up budgets are generous for loaded runners, while the probe and
  grace budgets are tight because escalation is recorded and checked. The
  note on the old frontend-watcher flake stays.

### Condition waits (`test_dev_integration.py`)

- `test_orphan_reach_when_arbiter_already_dead` waits until the SIGKILLed
  arbiter is dead, instead of sleeping 0.5 s.
- `test_stale_cleanup_after_out_of_band_kill` waits until no recorded fake
  is still running, instead of sleeping 0.5 s.
- `wait_for_text` and the status poll in `test_status_exit_codes` are
  rebuilt on the shared `wait_until`. The poll keeps its 5 s budget.

No production code changes: `tasks/shared/dev` is untouched.

## Context

- Work item: `meta/work/PP-956-parallelise-the-serial-test-suites.md`
- Plan: `meta/plans/2026-11-15-0364-parallelise-the-serial-test-suites.md`
  (Phase 2)
- Base: #220 (Phase 1), which this PR must merge after.

## Testing

- [x] `mise run test:integration:dev` passes serially: 21 passed in 61 s.
- [x] Five consecutive full `mise run` runs are green on `b9866538`. The
      dev suite passed 21/21 in each, taking 92–120 s (median 95 s) under
      the full build's load. No `stop_cleanly` check failed, so the 0.5 s
      `grace_kill` caused no escalation.
- [x] No fixed settle sleep of 0.5 s or more remains:
      `grep -En '^[[:space:]]*time\.sleep\((0\.[5-9]|[1-9])' tests/integration/dev/test_dev_integration.py`
      returns nothing.
- [x] Production defaults are untouched: the diff under `tasks/shared/dev`
      is empty.
- [x] `mise run build-system:check` passes.

## Notes for Reviewers

- **Where the shorter budgets could still bite.** `probe_timeout` also
  bounds three calls that fail silently when a busy arbiter misses the
  deadline: the reuse gate, watcher-PID recording, and the frontend start
  and status polls. A slow reply there shows up as a missed `reused`
  (test 2) or a null recorded PID (tests 1, 13, 15, 16), not as an
  escalation. None appeared in the five load runs. If one appears under
  Phase 3's parallel load, the fix is to raise `probe_timeout`, not to
  weaken the assertion.
- **`grace_kill` and `kill_arbiter`.** If a status probe times out on a
  live arbiter, our code SIGTERMs circusd and then waits only `grace_kill`
  before SIGKILL. Both signals land in the escalation record, so the
  clean-stop check fails loudly rather than masking it. If it happens, the
  fix is `grace_kill` 2.5.
- **What's next.** Phase 3 adds `pytest-xdist` and runs the suite with
  `-n auto`. This PR merges first, so that Phase 3's CI before window
  already includes the shorter waits and judges parallelism on its own.
