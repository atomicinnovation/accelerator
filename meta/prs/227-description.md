---
type: "pr-description"
id: "227"
title: "Run the Vitest suite after the unit lanes that compile Rust"
date: "2026-11-16T16:34:57+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/227"
pr_number: 227
tags: ["build-system", "ci", "testing", "vitest", "flaky-test"]
revision: "edfb82c256e49cbdbefbb461997388b40dcfd174"
repository: "accelerator"
last_updated: "2026-11-16T16:34:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Run the Vitest suite after the unit lanes that compile Rust

## Summary

"Run unit tests (macos-latest)" has been failing intermittently since
15 November, even when every test passes. In each failure checked, all
2615 frontend tests passed, and Vitest then failed the run with
`Error: [vitest-worker]: Timeout calling "onTaskUpdate"`. This PR makes
`test:unit:frontend` wait for the two unit lanes that compile Rust, so
the Vitest suite no longer competes with cargo for the small macOS
runner.

## Changes

- **`mise.toml`:** `test:unit:frontend` gains
  `wait_for = ["test:unit:cli", "test:unit:visualiser"]`. `wait_for`
  only orders tasks that are already part of the same run, and it never
  pulls a task in. So `mise run test:unit` and the bare `mise run` start
  Vitest once both cargo lanes have finished, while
  `mise run test:unit:frontend` on its own starts straight away, as
  before.
- **`tests/unit/tasks/test_mise.py`:** two tests in the existing
  task-topology style. One pins the ordering edge, and the other checks
  that both lanes are still in the `test:unit` roll-up, so the edge
  cannot be silently emptied.

## Context

**Why tests that pass still fail.** A Vitest worker reports results to
the main Vitest process over an RPC channel. In vitest 3.2.4 every call
on that channel has a fixed 60-second timeout
(`DEFAULT_TIMEOUT = 6e4` in birpc), and no Vitest option reaches it. A
worker that stalls for more than a minute makes the run fail, with no
failing test to show for it. The main process was not the one stalling:
its output never paused for more than 19 s in any failing log.

**Why only macOS.** `test:unit` starts every lane at once, so Vitest
shares the machine with the coverage-instrumented cargo build and
nextest run (`test:unit:cli`) and with the visualiser's cargo builds.
The `macos-latest` runner has 3 vCPUs and 7 GB of RAM, against
Ubuntu's 4 vCPUs and 16 GB. The suite's duration follows the
contention:

| Frontend suite on macOS | Vitest-reported duration |
|---|---|
| Passing runs (6 checked) | 68–279 s |
| Failing runs (7 checked) | 243–358 s |

The suite takes 68 s when it has the runner to itself. Every failure
sits in the contended band.

**Out of scope.** The `accelerator-work::cli_rekey_required` failure in
run 43889533733 also failed on Ubuntu. It came from PR #178's branch,
where a test depended on a global git identity, and it has been fixed
on that branch.

## Testing

- [x] Wrote the ordering test first and saw it fail, then added the
      `wait_for` edge. `uv run pytest tests/unit/tasks/test_mise.py`
      now passes (77 tests).
- [x] Checked `wait_for` on mise 2026.10.8 with a throwaway config. In
      a shared run the waiting task started only after the slow one
      finished, and run on its own it did not pull the slow one in.
- [x] `mise run build-system:check` is clean.
- [ ] The full `mise run` default task was not run locally.
- [ ] Confirm over several CI runs that the macOS unit leg stops
      failing with `Timeout calling "onTaskUpdate"`. This could not be
      reproduced locally: pausing a Vitest fork for 65 s did not trigger
      the timeout.

## Notes for Reviewers

- **Wall time.** Uncontended, Vitest takes about 70–90 s, and
  `test:unit:cli` should speed up a little without Vitest competing for
  the CPUs. The unit job should finish in about the same time as now,
  since the contended Vitest suite was already its longest lane.
- **Alternatives rejected.** Lowering Vitest's worker count would not
  stop cargo taking the CPUs and memory. Raising the RPC timeout is not
  possible without patching Vitest. Dropping the frontend suite from the
  macOS leg would lose coverage.

https://claude.ai/code/session_01HEhrwWKyY2JP2hbeu6CVhi
