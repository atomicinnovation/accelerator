---
type: work-item
id: "0190"
title: "acquire_lock cannot classify an unusable lock directory"
date: "2026-08-03T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: draft
kind: bug
priority: medium
parent: "work-item:0136"
relates_to: ["work-item:0186", "work-item:0164"]
tags: [bug, shell, bootstrap, bash-3.2]
last_updated: "2026-08-03T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0190: acquire_lock cannot classify an unusable lock directory

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`acquire_lock` in [`bin/accelerator`](../../bin/accelerator) treats every failed
`mkdir "${lock_dir}"` as "someone else holds the lock". It has no notion of an
`mkdir` that can never succeed, so an unusable lock directory is reported as a
lock timeout after the full 300 × 0.1 s budget — and one arm has no bound at
all.

## Context

Found while implementing 0186. That work added a probe gate at the top of the
cold branch specifically to keep the one reachable instance of this from
regressing: with an unwritable cache directory and a skipped staging step,
control reached `acquire_lock`, `mkdir` could never succeed, no pid file
existed, so the loop took the `else` arm and burned its whole budget before
failing with the wrong diagnostic (measured at a reduced ceiling as
`TIMEOUT after 31 iters, 3s`). The gate masks that instance; it does not fix the
classification.

There is a worse arm, which neither gate prevents. When the pid file names a
dead process, the loop does `rm -f`/`rmdir` then `continue` — **with no `sleep`
and no `waited` increment**. If the lock directory cannot be removed (created by
another user, or the cache directory is writable but the lock directory is not),
the loop spins **unbounded, with no timeout at all**. The probe gate does not
help: the probe passes on a writable cache directory whose lock directory
happens to be foreign.

## Requirements

- Classify the `mkdir` failure instead of assuming contention: after a failed
  `mkdir`, `[[ -d "${lock_dir}" ]]` distinguishes `EEXIST` (a genuine
  competitor) from a permission or I/O failure (unrecoverable — fail
  immediately, naming the cause).
- Give the dead-owner reclaim arm a bound: a failed `rmdir` must not loop
  without advancing the budget.
- Stay within the bash 3.2 floor.

**Scope**: classifying `mkdir` and `rmdir` failures. **Not** re-wording the
timeout message, and not redesigning the locking scheme.

## Acceptance Criteria

- [ ] A cold run against a cache directory whose lock directory cannot be
      created fails within a second with a diagnostic naming the cause, not
      after the timeout budget with a lock-timeout message.
- [ ] A lock directory holding a dead owner's pid file that cannot be removed
      terminates within the timeout budget rather than spinning unbounded —
      pinned by a case with an explicit `timeout=`, so a regression shows as a
      failure rather than a hung suite.
- [ ] `test_stale_lock_is_reclaimed` and
      `test_concurrent_cold_cache_slow_downloader_all_succeed` stay green: a
      live owner still extends the budget and a dead owner is still reclaimed.
- [ ] `scripts/lint-bashisms.sh`, shfmt and ShellCheck report no findings.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- **Relates to**: 0186, which masked the one reachable instance and recorded the
  unbounded arm; 0164, which introduced the lock.
- **Parent**: epic 0136.

## References

- `bin/accelerator` — `acquire_lock`, and the `rm -f`/`rmdir`/`continue` arm
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — Validation
  Results, which records the measurement and the reachable instance
- `tests/integration/entrypoint/test_accelerator_entrypoint.py` —
  `test_unverifiable_launcher_in_readonly_cache_fails_fast` guards the masked
  instance
