---
type: pr-description
id: "53"
title: "Fix the lock reclaim race and the dev frontend-activation flake"
date: "2026-08-06T08:25:28+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
pr_url: "https://github.com/atomicinnovation/accelerator/pull/53"
pr_number: 53
tags: []
revision: "84e6a9f8e7f0bc1154de84555d272911ca6ec4f5"
repository: "accelerator"
last_updated: "2026-08-06T08:25:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Fix the lock reclaim race and the dev frontend-activation flake

## Summary

Closes out the remaining CI flakiness after the 2026-08-03 load-sensitive batch. Two of these were not flakes at all but genuine correctness bugs that a timeout bump would have masked: the mkdir advisory lock could hand the same lock to two acquirers, and a circus start refusal was being discarded, which is what actually produced "the frontend watcher did not become active". The other three are tests asserting the wrong property or synchronising on the wrong thing.

## Changes

- **Bind the mkdir lock's reclaim to the holder it read.** Reclaim is a read-then-act and the read goes stale. Renaming the whole lockdir aside moved a *live* holder's directory and freed its path, so a second acquirer took the same lock — a lost write, since the guarded operation is a read-modify-write of an append-only store. The holder sentinel is now `owner.<nonce>` and reclaim renames that exact name to `reclaiming.<reclaimer-pid>.<nonce>`; `rename` is atomic, so the step is single-winner among contenders that read the same holder, and the nonce makes it a no-op against any other holder. Removal is gated on the *reclaimer's* liveness, not the original holder's.
- **Assert mutual exclusion, not a single winner, in the workspace lock test.** The property owed to callers is that two holders never overlap, not that exactly one of eight contenders succeeds. Acquisition is non-blocking, so a contender descheduled past the winner's hold and release legitimately takes the freed lock. The test now tracks occupancy and asserts it never exceeds one.
- **Surface a circus start refusal instead of discarding it.** circus answers a rejected command with `{"status": "error", "reason": ...}` rather than raising, so discarding the reply from `start()` turned a refused start into a silent no-op. The caller then polled a watcher that had never been asked to run until its deadline, and pointed at a log that was empty because nothing had ever written to it.
- **Say why a frontend activation failed.** The abort message now folds in the watcher status and the tail of `frontend.log`, gathered before teardown (teardown stops the watcher, so the status afterwards is `stopped` regardless of cause). The integration driver's assertions quoted only stderr while the orchestrators report refusals on stdout, producing a bare `AssertionError`; they now quote both streams. Happy-path budgets go to 60s as headroom.
- **Synchronise the dev-activation specs on mount, not on memory history.** `renderAt` waited on `router.state.location.pathname`, which `createMemoryHistory` already reports at construction — so the assertion was true before `render` was called and synchronised on nothing. The specs now wait on the route's rendered marker and then on the session write that is the capture effect's only observable trace.

## Context

Came out of a triage of ~70 failed `Main` runs between 2026-07-20 and 2026-08-06. Two findings shaped the scope:

- Mapping each failure signature to the set of distinct branches and SHAs it appears on separates flakes from branch breaks. A signature confined to one branch and reproducing on *every* commit there is a break, not a flake — that reclassified roughly 16 of the failures (notably `a_spawn_through_the_stub_path_is_recorded`, all on `0188` across five commits, and `migrated corpus validates clean`, all on `0169` at one SHA).
- The largest genuine class, the frontend vitest timeouts, was already fixed on main by the 2026-08-03 batch (`asyncUtilTimeout`, `EVENT_BUDGET`, the indexer tripwire, and nextest temp-dir isolation). This PR is the remainder.

No linked work item.

## Testing

- [x] `mise run` — the full local CI mirror — green twice consecutively against this tree, with the exit status captured directly rather than through a pipe (`MISE_EXIT=0` both times; piping into `tail` masks it).
- [x] `test:integration:dev` 17/17 on both runs (177s, 168s).
- [x] `test:unit:cli` 722/722, including the lock module's 18 tests.
- [x] `test:unit:frontend` 2536/2536 across 122 files; `use-dev-activation.test.tsx` also passes 12/12 in isolation.
- [x] `test:e2e:visualiser` 343 passed, 1 skipped.
- [x] The lock race has a deterministic repro, `a_stale_reclaim_does_not_displace_a_live_holder`, which forces the interleaving through the `is_alive` hook — it fails on the old implementation and passes on the new. Sampling it with real threads does not reproduce in 200 runs.
- [x] Crash-recovery paths covered both sides of the gate: an abandoned reclaim whose reclaimer is dead is finished by the next acquirer; one whose reclaimer is alive is left alone.
- [ ] Each commit has **not** been verified to build in isolation — the gate was run against the tip of the stack. Matters only if these land as a stack rather than squashed.
- [ ] The dev frontend-activation failure has not been observed again in CI post-fix; it was intermittent at roughly one run in ten, so a few green runs will not by themselves confirm it.

## Notes for Reviewers

- **The sentinel format is a cross-implementation contract.** `cli/corpus-adapters/src/lock.rs` and `scripts/atomic-common.sh` lock against and reclaim after each other over the same `<target>.lockdir`, so they move together in the first commit. Changing the naming in one alone means a held lock stops being recognised. A legacy nonce-less `owner` is still read by both as a pre-upgrade orphan, and never written.
- **The reclaimer-liveness gate is the subtle part.** Re-noncing alone is not sufficient: two contenders can each win a rename of a *different* name (one of `owner.<n>`, one of the `reclaiming.<n>` it became) and both then proceed to remove, the slower one landing on whatever fresh live lockdir had since replaced it. Gating removal on the reclaimer's own PID — read from the name, published atomically by the rename that created it — is what keeps removal single-owner.
- **`_atomic_lock_nonce` respects the bash 3.2 floor.** `$RANDOM` alone is 15 bits and collides readily across the many short-lived subshells a parallel run spawns, so it draws twice and mixes in the subshell PID, falling back to `$$` where `BASHPID` is unset.
- **The budget widening in "Say why a frontend activation failed" is headroom, not the cure.** "Surface a circus start refusal" is the actual fix. The 60s figure is deliberately far above need — a bring-up measures ~1s idle and ~1.6s at 8x concurrency — so that load can be ruled out if the symptom ever returns, at which point the new message should name the cause directly.
- Worth a close look at `reclaim_if_stale` and `_atomic_lock_reclaimable` in particular, since a mistake there is a silent lost write rather than a visible failure.
