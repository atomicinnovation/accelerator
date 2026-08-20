---
type: work-item
id: "0190"
title: "acquire_lock misclassifies an unusable lock directory and can spin unbounded on reclaim"
date: "2026-08-03T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: draft
kind: bug
priority: medium
parent: "work-item:0136"
relates_to: ["work-item:0186", "work-item:0164", "work-item:0191"]
tags: [bug, shell, bootstrap, bash-3.2]
last_updated: "2026-08-21T00:00:06+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0190: acquire_lock misclassifies an unusable lock directory and can spin unbounded on reclaim

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`acquire_lock` in [`bin/accelerator`](../../bin/accelerator) treats every failed
`mkdir "${lock_dir}"` as "someone else holds the lock". It has no notion of an
`mkdir` that can never succeed, so an unusable lock directory is reported as a
lock timeout after the full 300 × 0.1 s budget — and one branch of the retry
loop has no bound at all.

## Context

Found while implementing 0186. That work added a probe gate at the top of the
cold branch specifically to keep the one reachable instance of this from
regressing: with an unwritable cache directory and a skipped staging step,
control reached `acquire_lock`, `mkdir` could never succeed, no pid file
existed, so the loop took the `else` arm and burned its whole budget before
failing with the wrong diagnostic (measured at a reduced ceiling as
`TIMEOUT after 31 iters, 3s`). The gate masks that instance; it does not fix the
classification.

There is a worse arm, which the probe gate does not prevent. When the pid file names a
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
  immediately, naming the lock path and a permission-or-I/O cause).
- Give the dead-owner reclaim arm a bound: a failed `rmdir` must not loop
  without advancing the budget. This arm is the more severe of the two defects
  — an unbounded hang, not a wrong diagnostic after a bounded budget — even
  though the item as a whole is medium priority.
- Make the loop's iteration ceiling env-injectable so the bounded arm can be
  exercised under a small budget: the default stays `300` (× `sleep 0.1`), and
  a test overrides it to a low value for a sub-second, deterministic
  bounded-vs-unbounded check.
- Stay within the bash 3.2 floor.

**Scope**: classifying `mkdir` and `rmdir` failures, and the env-injectable
ceiling that makes the bounded arm testable. **Not** re-wording the timeout
message, and not redesigning the locking scheme.

## Acceptance Criteria

The first two criteria manufacture their preconditions by `chmod`, so 0186's
permission-test rule governs them: each asserts `id -u` ≠ 0 and **hard-fails
rather than skips** when run as root, since root bypasses the write and removal
restrictions these cases rely on and would satisfy the assertions regardless of
the fix. A lane structurally unable to comply is **excluded explicitly** by the
implementer, justified by a recorded privilege check (`id -u` returning 0, or a
temp-dir check that file creation inside a `chmod`-restricted directory fails),
never skipped silently. See the Acceptance Criteria preamble of
`meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` for the full
rule.

- [ ] A cold run against a cache directory whose lock directory cannot be
      created fails fast, its combined output containing the lock directory path
      and a fixed cause substring the fail-fast branch emits (asserted verbatim)
      — not after the budget with a lock-timeout message. Run under the injected
      low ceiling so a regression that falls through to the `else` arm still
      completes in well under a second, making the diagnostic the discriminator
      rather than wall-clock.
- [ ] A lock directory holding a dead owner's pid file that cannot be removed
      terminates within the loop's budget rather than spinning unbounded, and
      its terminating exit is non-zero carrying the existing lock-timeout
      message. Run under a small env-injected ceiling so the case completes in
      well under a second, pinned by a harness subprocess `timeout=` set above
      that injected budget (distinct from, and far below, the default
      `300`-iteration budget) — so an unbounded regression trips the harness
      timeout and shows as a failure rather than a hung suite.
- [ ] `test_stale_lock_is_reclaimed` (a dead owner is still reclaimed) and
      `test_concurrent_cold_cache_slow_downloader_all_succeed` (a live owner
      still extends the budget) stay green.
- [ ] A lock directory holding an empty or unreadable pid file advances the
      budget rather than failing fast or reclaiming — a deterministic case that
      pins the `else` arm the classification branch must leave intact, run under
      the injected low ceiling so it terminates in well under a second.
- [ ] `scripts/lint-bashisms.sh`, shfmt and ShellCheck report no findings.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- **Relates to**: 0186, which masked the one reachable instance and recorded the
  unbounded arm; 0164, which introduced the lock; 0191, which edits a different
  region of `bin/accelerator` (the shim-staging block), so the two can land in
  either order — the shared file is not a merge coupling.
- **On 0186's masking gate**: the cold-branch probe gate 0186 added is retained
  as defence-in-depth, not removed by this fix — the item does not redesign the
  locking scheme, and the gate keeps its own regression guard
  (`test_unverifiable_launcher_in_readonly_cache_fails_fast`). Nothing
  downstream waits on this fix.
- **Parent**: epic 0136.

## Technical Notes

The loop in `acquire_lock` has three post-`mkdir`-failure arms. The fix adds one
classification branch and bounds a second arm; the mkdir+pid scheme is otherwise
unchanged.

**Classification branch (mkdir arm).** After a failed `mkdir "${lock_dir}"`,
test `[[ -d "${lock_dir}" ]]` before reading the pid file. A failed `mkdir`
whose directory is *absent* is not `EEXIST` — the parent cache directory is
unwritable and the `mkdir` can never succeed. Fail immediately, naming the lock
path and the likely cause (permission or I/O), instead of entering the wait
loop. `mkdir` exposes no shell-portable errno, so directory-presence is the only
portable `EEXIST` discriminator — which is why the Requirements pick that
predicate.

**Bounding the dead-owner arm.** The reclaim arm (`rm -f pid` → `rmdir` →
`continue`) advances neither `waited` nor `sleep`, so a `rmdir` that fails on a
foreign lock directory spins unbounded. Gate the `continue` on `rmdir`
succeeding; on `rmdir` failure fall through to `waited=$((waited + 1))` and
`sleep 0.1`, so the arm shares the `else` arm's 300-iteration cap. The
acceptance criterion specifies "terminates within the timeout budget" for this
arm rather than fail-fast, so a foreign lock directory is caught by the existing
bound, not a new immediate-fail path.

**Post-fix arm order.** mkdir succeeds → hold; mkdir fails + directory absent →
fail fast (new); directory + live owner → reset budget; directory + dead owner +
`rmdir` ok → reclaim and retry; directory + dead owner + `rmdir` fails → advance
budget; directory + empty/unreadable pid (the `else` arm) → advance budget. That
`else` arm still covers the genuine race window — a competitor that created the
directory but has not yet written its pid.

**bash 3.2 floor.** `[[ -d ]]`, `[[ -n ]]`, `kill -0`, and `$(( ))` are all
3.2-safe; the fix needs no bash-4 construct (no associative arrays, `${var,,}`,
or `mapfile`).

**Testability of the timeout case.** The ceiling defaults to `300`
(× `sleep 0.1` ≈ 30 s wall) but is env-injectable (a Requirement), so the
bounded-arm case overrides it to a low value and completes in well under a
second. The acceptance criterion's harness subprocess `timeout=`
(`_run_bootstrap(..., timeout=N)` in
`tests/integration/entrypoint/test_accelerator_entrypoint.py`) is a **distinct**
timeout from the loop's own budget: set it above the injected ceiling but far
below the default 30 s, so a correctly-bounded loop exits cleanly before it
fires while an unbounded regression trips it — either way a failure, never a
hung suite and never a permanently ~30 s test. The env-injectable ceiling is a
testability seam, not a redesign of the scheme.

## References

- `bin/accelerator` — `acquire_lock`, and the `rm -f`/`rmdir`/`continue` arm
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — Validation
  Results, which records the measurement and the reachable instance
- `tests/integration/entrypoint/test_accelerator_entrypoint.py` —
  `test_unverifiable_launcher_in_readonly_cache_fails_fast` guards the masked
  instance
