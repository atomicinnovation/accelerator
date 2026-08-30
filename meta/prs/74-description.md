---
type: "pr-description"
id: "74"
title: "Classify acquire_lock mkdir failures and bound the reclaim arm"
date: "2026-08-22T19:25:05+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0190"
parent: "work-item:0190"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/74"
pr_number: 74
tags: ["bug", "shell", "bootstrap", "bash-3.2", "locking"]
revision: "70175a80a981c370bfa1d89d5ba87802e23474f7"
repository: "accelerator"
last_updated: "2026-08-22T19:25:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0190] Classify acquire_lock mkdir failures and bound the reclaim arm

## Summary

Fixes two defects in `acquire_lock` (`bin/accelerator`): a failed `mkdir` on an unusable lock path was misclassified as contention and reported as a ~30 s lock timeout, and the dead-owner reclaim arm could spin unbounded when the lock directory could not be removed. The production change is 19 lines in one function, guarded by five deterministic integration tests; the remaining files are the 0190 lifecycle artefacts (work item, research, reviews, plan, validation).

## Changes

- **Classify the mkdir failure before assuming contention.** A symlink or non-directory occupying the lock path now fails fast with `cannot create the launcher cache lock: <path>` instead of burning the wait budget and reporting a lock timeout. A merely-absent directory (a competitor that just released) falls through and retries, so a released competitor is never misclassified. The `-L` guard also stops the reclaim `rm`/`rmdir` from following an attacker-planted symlink.
- **Bound the dead-owner reclaim arm.** The reclaim `continue` now fires only when both `rm -f` and `rmdir` succeed; any failure falls through to the budget-advancing `else`, so an unremovable foreign lock terminates within the ceiling rather than busy-spinning with no timeout — the more severe of the two defects.
- **Make the iteration ceiling env-injectable and injection-safe.** `max_wait` is read once from `ACCELERATOR_LOCK_MAX_WAIT` (default 300), validated to an all-digit string, and normalised to base 10 via `$((10#…))`. A non-numeric value falls back to 300 and a leading-zero value (`08`) is read decimally, not as an invalid octal literal — closing the arithmetic-injection surface and the silent loss-of-bound under `set -uo pipefail` (no `-e`). The knob also lets the bounded arm test sub-second.
- **Five integration tests** in `test_accelerator_entrypoint.py`, one per post-fix arm: fail-fast on a file and on a symlink at the lock path, bounded termination on an unremovable dead owner, the empty-pid `else` arm, and the base-10 ceiling.

## Context

- Work item: `meta/work/0190-acquire-lock-cannot-classify-mkdir-failures.md`
- Research: `meta/research/codebase/2026-08-21-0190-acquire-lock-mkdir-classification.md`
- Plan: `meta/plans/2026-08-21-0190-classify-lock-mkdir-failures.md`
- Validation: `meta/validations/2026-08-21-0190-classify-lock-mkdir-failures-validation.md`

## Testing

- [x] New lock tests pass: `uv run pytest tests/integration/entrypoint/test_accelerator_entrypoint.py -k "lock or leading_zero"` (6 passed).
- [x] Retained lock guards stay green: `-k "stale_lock or slow_downloader or readonly_cache_fails_fast"` (3 passed).
- [x] Shell lint clean: `mise run scripts:check` (bashisms, exec-bits, shfmt, ShellCheck).
- [x] Read-only CI mirror passes: `mise run check` (exit 0).
- [ ] Full default gate `mise run` not re-run locally this session — the outstanding delta is the docs CI lane, untouched by this change; CI covers it.
- [x] bash 3.2 floor: the `macos-latest` integration leg exercises the tests under system bash 3.2; the fix uses only 3.2-safe constructs.

## Notes for Reviewers

- The reclaim remains intentionally non-single-winner and the reclaim-*success* `continue` stays a no-sleep fast-retry — both scoped out in the plan's "What We're NOT Doing", safe because the guarded critical section is idempotent. Residual availability classes (reused-pid live-owner reset, shared-cache DoS) are documented there and unchanged.
- ⚠️ The AC1 fail-fast branch is reachable in tests only via a non-directory/symlink at the lock path, because the 0186 gate `require_exec_capable_cache` probes `cache_dir` (not the lock subdirectory). If that gate is ever removed, add a direct test for the unwritable-parent trigger.
- Follow-up 0191 edits a different region of the same file, so there is no merge coupling.
