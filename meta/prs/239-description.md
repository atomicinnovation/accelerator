---
type: "pr-description"
id: "239"
title: "[0364] Assert the relocated root's footprint after an explicit cache prune"
date: "2026-11-18T07:50:31+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0364"
parent: "work-item:PP-956"
relates_to: ["plan:2026-11-15-0364-parallelise-the-serial-test-suites"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/239"
pr_number: 239
tags: ["launcher", "testing", "integration", "flakiness"]
revision: "69cf1ff69065e9000a65557727c2b515f9f3b0a4"
repository: "accelerator"
last_updated: "2026-11-18T07:50:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0364] Assert the relocated root's footprint after an explicit cache prune

## Summary

`test_a_relocated_root_keeps_only_the_current_versions_footprint` checked
that one `visualiser` dispatch leaves only the current version's entries
in a relocated cache root. A dispatch's prune is best-effort by design,
so on a busy CI runner the superseded versions' claims survived it. The
test now runs `cache prune` after the dispatch, which always finishes,
and then asserts the footprint. Only the test changes.

## Changes

- **`tests/integration/entrypoint/test_accelerator_entrypoint.py`**:
  - After the `visualiser` dispatch, the test runs `cache prune` through
    the same bootstrap with the same environment, and asserts that it
    exits 0.
  - The footprint assertions are unchanged. They now run after that
    prune.
  - Both calls share one `env` dict.

## Context

- **Where it came from.** Work item 0364's Phase 7 paired CI comparison
  found this failure (`meta/plans/2026-11-15-0364-parallelise-the-serial-test-suites.md`).
  The comparison ran 11 `Main` runs on each of two draft PRs from `main`
  at `a646f527`: #235 kept the dev suite's xdist workers, and #236
  reverted them.
  - The test failed on 2 of #235's 11 ubuntu integration jobs (run
    44189947764, attempts 8 and 10), and on none of #236's.
  - It also failed once on 0351's branch, in run 44124140720 on
    `ba4fe8fa`, which includes #222.
  - Each time, the two seeded claims of superseded versions,
    `claims/0.1.0.*` and `claims/0.2.0.*`, were left.
- **Why one dispatch isn't enough.** A dispatch prunes in
  `RelocatedRetention::after_store`, and that prune is best-effort in two
  ways:
  - It only *tries* the root's prune lock (`LockPatience::Try`).
  - It runs under `COLD_PATH_BUDGET`, 2 s counted with `now_seconds()`,
    so 1–2 s of real time. Within the single-file prune it removes stale
    version claims last, checking `has_budget()` before each one, and
    leaves the rest to a later prune.

  On a 4-vCPU runner shared with the dev suite's xdist workers, the
  budget runs out before the claims. The launcher behaved as designed;
  the test asserted something a single dispatch doesn't promise.
- **Why `cache prune` fixes it.** It applies the same `RootPrune` with
  the same `CLAIM_WINDOW`, but with `PruneBudget::Unbounded` and
  `LockPatience::Wait`. So it always reaches the footprint the test
  describes.
- This is outside 0364's two suites. It was fixed here because #222's
  workers make `main`'s ubuntu integration job fail about 1 run in 5.

## Testing

- [x] **Red.** A temporary harness, not committed, held `flock` on
      `cache/.accelerator-prune.lock` through the dispatch. That forces
      the dispatch's try-lock prune to skip, which is the same outcome as
      an exhausted budget. The unchanged test failed with CI's exact
      `AssertionError`: both superseded claims plus the running
      version's.
- [x] **Green.** With the fix and the same harness, the test passed.
      With the harness removed, it passes too.
- [x] `mise run test:integration:entrypoint`: 79 passed.
- [x] `mise run build-system:check` passes.
- [ ] **CPU load alone didn't reproduce the failure locally.** With 48
      busy loops on engywook's 16 cores, the unchanged test passed 5 of
      5 runs. The lock harness reproduces the outcome deterministically
      instead. This PR's CI and later `main` runs are the check under
      real load.

## Notes for Reviewers

- **What this test no longer covers end to end.** Before, the test also
  showed, when it passed, that a dispatch prunes at all. It no longer
  does, because `cache prune` would hide a dispatch that pruned nothing.
  That property was never deterministic: it held only when the prune
  beat its budget. The Rust unit tests in `root_retention.rs` cover the
  cold path, for example
  `a_cold_path_logs_what_its_prune_could_not_do`. If an end-to-end check
  of dispatch-time pruning is wanted, it should assert something within
  the budget's promise, and is better as its own test.
- **Network.** `cache prune` runs inside the release-server block with
  the same environment. It makes no request, so the test's
  `requests.count(asset) == 1` assertion still holds.
- **The running version's claim.** Both runs leave the running version's
  claim, so the test's single-claim assertion still matches it.

https://claude.ai/code/session_01HEhrwWSAMeTH1BGPYLkEZP
