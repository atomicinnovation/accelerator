---
type: plan-review
id: "2026-08-21-0190-classify-lock-mkdir-failures-review-1"
title: "Plan Review: acquire_lock mkdir classification and bounded reclaim"
date: "2026-08-21T15:44:37+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-08-21-0190-classify-lock-mkdir-failures"
target: "plan:2026-08-21-0190-classify-lock-mkdir-failures"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [correctness, test-coverage, code-quality, portability, safety, security]
review_number: 1
review_pass: 3
tags: [bug, shell, bootstrap, bash-3.2, locking]
last_updated: "2026-08-21T19:51:37+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: acquire_lock mkdir classification and bounded reclaim

**Verdict:** REVISE

The plan is a proportionate, well-researched fix that all six lenses agree improves on the status quo — it removes the unbounded busy-spin, fails fast on an uncreatable lock path, and carries a genuine tripwire test for the more severe defect. But the two core code edits each introduce a new defect the change is responsible for: the `[[ ! -d ]]` classifier misreads a competitor releasing its lock as a fatal error (a concurrency regression against the plan's own concurrent-cold-cache criterion), and the new `ACCELERATOR_LOCK_MAX_WAIT` reaches `[[ -gt ]]` arithmetic unvalidated, opening a bash arithmetic-injection surface in a root-of-trust bootstrap. Four major findings gate the plan to REVISE; both high-confidence majors are narrow, localised edits away from resolution.

### Cross-Cutting Themes

- **The `[[ ! -d "${lock_dir}" ]]` classifier is under-specified** (flagged by: correctness, security, portability, test-coverage, code-quality) — the single edit at the heart of the fix draws five findings. It misclassifies a *released competitor* as unrecoverable (correctness, major), *follows symlinks* so an attacker-planted link drives the reclaim `rm`/`rmdir` through the target (security, minor), has no direct test for its unwritable-parent trigger (test-coverage, minor), collapses distinct causes into one message (portability, suggestion), and widens the `else` arm's contract (code-quality, minor). The `[[ -e "${lock_dir}" && ! -d "${lock_dir}" ]]` predicate — proposed **independently by correctness and portability** — resolves the race and the cause-collapse at once; adding a `-L` symlink check closes the security vector.
- **`ACCELERATOR_LOCK_MAX_WAIT` flows in unvalidated** (flagged by: security, correctness, safety, code-quality) — the new env seam reaches arithmetic evaluation with no guard. A value like `a[$(cmd)]` executes `cmd` (security, major); a non-integer makes `[[ -gt ]]` a syntax error that, under `set -u` without `-e`, silently *removes the loop's bound* and re-introduces Defect 2 (correctness, minor); an accidental production value shrinks the budget (safety, suggestion); and the name reads as seconds not iterations (code-quality, minor). One validate-at-read-time change (`case`/regex to non-negative integer, else fall back to 300) closes the security and correctness halves together.
- **Three prose claims overstate completeness** (flagged by: correctness, safety, test-coverage) — "every arm terminates within a bounded budget" is false for the live-owner reset and the reclaim-*success* `continue` (both budget-free); "new tests fail against the current code" is false for the empty-pid guard (green pre-fix) and imprecise for the fail-fast test (reds only after ~30 s pre-fix). Precision fixes, not code changes.

### Tradeoff Analysis

- **Minimal fix vs. closing the residuals now**: The plan's minimalism is a genuine virtue and the two high-confidence majors (M1, M2) are *not* gold-plating — they are a regression and an injection surface this change introduces, so they should be fixed in scope, not deferred. The reclaim mutual-exclusion residual (M3) is the real defer-vs-fix call: the plan deliberately scopes out the locking-scheme redesign, and safety agrees deferral is acceptable **provided** the plan records both the non-atomic-reclaim gap and the idempotency assumption (per-PID temps + atomic rename) that currently makes it safe. Recommendation: fix M1/M2 now, record M3 as a known residual with its safety precondition stated.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Correctness**: TOCTOU — classifier misreads a released competitor as a fatal "cannot create" error
  **Location**: Phase 1 §1 — the `[[ ! -d "${lock_dir}" ]]` branch
  The directory's absence has three causes, not two: the third is a competitor that legitimately held the lock and removed its directory via `release_lock` in the window between this waiter's `mkdir` returning EEXIST and the `-d` stat. The waiter then spuriously aborts instead of retrying; the original code fell through to the genuine-race `else` arm and acquired on the next iteration. Directly threatens `test_concurrent_cold_cache_slow_downloader_all_succeed`. Fix: `[[ -e "${lock_dir}" && ! -d "${lock_dir}" ]]`.

- 🟡 **Security + Correctness**: Unvalidated `ACCELERATOR_LOCK_MAX_WAIT` reaches arithmetic evaluation — injection surface and bound-removal
  **Location**: Phase 1 §1 — `max_wait="${…:-300}"` and `[[ "${waited}" -gt "${max_wait}" ]]`; reinforced by the "always-on, ungated" decision in What We're NOT Doing
  `-gt` evaluates operands as arithmetic, and bash arithmetic re-expands array-subscript syntax, so `a[$(command)]` executes `command` — present on the bash 3.2 floor too. This is the first externally-influenced value to reach an arithmetic context in this root-of-trust entry point. Separately (correctness), a non-integer value makes `[[ … ]]` return non-zero every iteration under `set -uo pipefail` (no `-e`), so the timeout `fail` never fires and the reclaim arm loses its bound — re-introducing Defect 2. Fix: validate as a non-negative integer at read time, else fall back to 300; or gate behind `ACCELERATOR_TEST_MODE`.

- 🟡 **Safety**: Reclaim's non-atomic `rm -f pid; rmdir` is not single-winner; mutual-exclusion residual unrecorded
  **Location**: What We're NOT Doing / Phase 1 §1 — the dead-owner reclaim arm
  Two waiters reading the same dead owner can each reclaim, and a reclaim can destroy a *live* holder's freshly-created directory (a plain TOCTOU, distinct from the scoped-out reused-pid framing), letting two sessions enter the critical section at once. Tolerable here **only** because the protected op is idempotent (per-PID temps + atomic rename of identical verified content) — but that assumption is unstated. The sibling `atomic-common.sh` linearises this with an atomic `mv` of a nonce'd sentinel; this scheme lacks that step. Deferral is acceptable if recorded with the idempotency precondition.

- 🟡 **Test Coverage**: `timeout=5` is tighter than the 15 s precedent and risks a false `pytest.fail` under load
  **Location**: Phase 1 §3 — `test_unremovable_dead_owner_lock_terminates_within_budget`
  The correct-path loop exits in ~0.4 s, but the whole bootstrap subprocess must finish inside 5 s, and the closest analogue (`test_unverifiable_launcher_in_readonly_cache_fails_fast`) uses `timeout=15` for the same shape. Given documented flakiness of these shell suites under parallel CI load, a loaded runner could exceed 5 s on a *correctly bounded* run and red the fix intermittently. Raise toward 15 s, keeping it well below the default ~30 s.

#### Minor

- 🔵 **Security**: `[[ -d ]]` follows symlinks — a symlink at the lock path bypasses fail-fast and drives reclaim through the target
  **Location**: Phase 1 §1 — the classification branch
  A symlink at `${cache_dir}/.accelerator-lock-<platform>` reads as a genuine competitor; `rm -f "${lock_dir}/pid"` then deletes a `pid`-named file in the attacker-chosen target while `rmdir` fails ENOTDIR. A narrow file-deletion-plus-DoS primitive, realistic only on a shared/multi-user-writable `ACCELERATOR_CACHE_DIR`. Reject a symlink (`[[ -L … ]]`) in the same branch, or record the residual.

- 🔵 **Correctness + Safety**: "Every arm terminates within a bounded budget" overstates completeness
  **Location**: Desired End State
  The live-owner reset (`waited=0`, scoped out) and the reclaim-*success* `continue` (unchanged) advance no budget; the latter can loop while an external process recreates the dead-owner directory. Neither is a pure busy-spin (both need external progress), but the blanket claim is inaccurate. Restate to name the exceptions.

- 🔵 **Code Quality**: Compound `elif` folds the side-effecting reclaim into a boolean and silently widens the `else` arm
  **Location**: Phase 1 §1 — `elif [[ -n "${owner}" ]] && rm -f … && rmdir …; then continue`
  Two of the three conjuncts are mutations whose exit codes double as the predicate, so a partial reclaim (pid gone, dir remains) re-enters via an `else` that now silently means three things. Defensible as the deliberate DRY choice, but at minimum state that the `else` contract has broadened beyond "empty/unreadable pid".

- 🔵 **Code Quality**: `max_wait` / `ACCELERATOR_LOCK_MAX_WAIT` name implies seconds but denotes an iteration count
  **Location**: Phase 1 §1 & §2
  The value is consecutive no-progress iterations (~300 × 0.1 s ≈ 30 s) — a distinction the research had to correct once already. Prefer a unit-bearing name, or keep the header note's "iteration ceiling" wording as the one place the unit is clear.

- 🔵 **Test Coverage**: "New tests fail against the current code" overstates the pre-fix outcome
  **Location**: Success Criteria — the `-k "lock"` checkbox
  `test_empty_pid_lock_advances_budget` already passes pre-fix (a characterisation guard), and `test_uncreatable_lock_dir_fails_fast` only reds after ~30 s pre-fix (current code ignores the env knob). State the expected pre-fix outcome per test so the TDD red step is not misread as a broken test.

- 🔵 **Test Coverage**: The classifier's unwritable-parent trigger has no direct regression test
  **Location**: Key Discoveries / What We're NOT Doing
  Only the non-directory trigger is exercised (test 1); the unwritable-parent trigger is masked by the 0186 gate and guarded only indirectly. If that gate is ever refactored, the branch silently loses coverage. Note the coupling so a future gate change carries a test obligation.

- 🔵 **Portability**: The bash-3.2 floor is already enforced by the macOS CI integration leg, not just a manual replay
  **Location**: Success Criteria — Manual Verification
  The harness pins `/bin/bash` (bash 3.2 on `macos-latest`) and the `test-integration` job runs that leg, so the three tests already exercise the fix under 3.2 in CI. Framing it as manual-only risks under-valuing the leg that actually enforces the floor. Demote the replay to a spot-check and name the macOS leg as the automated gate.

#### Suggestions

- 🔵 **Test Coverage**: Tests 1 and 3 have no harness `timeout=` net against a ceiling-check regression
  **Location**: Phase 1 §3
  Both rely solely on the injected ceiling to terminate; a mutation of the `-gt` comparison itself would hang the suite from these two rather than red cleanly (only test 2 carries the tripwire). Add a modest `timeout=` to tests 1 and 3 as cheap hang insurance.

- 🔵 **Safety**: Always-on `ACCELERATOR_LOCK_MAX_WAIT` is a small production footgun vs the gated precedent
  **Location**: What We're NOT Doing / Migration Notes
  The `jira-common.sh` precedent gates its override behind `ACCELERATOR_TEST_MODE`; an accidental value here silently shrinks the lock budget. Either gate to match, or record in Migration Notes why always-overridable is acceptable for this file. (Related to the M2 validation fix.)

- 🔵 **Code Quality**: The retained live-owner comment's tail narrates control flow
  **Location**: Phase 1 §1 — the `# A live owner is fetching…` comment
  Pre-existing (already at `bin/accelerator:328-331`), not introduced by this plan. Its first half is a legitimate *why*; its tail ("a dead owner is reclaimed immediately below") narrates flow and carries a positional reference that can go stale. Since the plan already rewrites this region, optionally trim to the rationale sentence.

- 🔵 **Portability**: The cause-neutral fail-fast diagnostic collapses distinct cross-platform causes
  **Location**: Phase 1 §1
  A file at the path, an unwritable parent, a read-only mount, and ENOSPC all surface identically. Reasonable given `mkdir` has no portable errno, but a cheap `[[ -e ]]` follow-up could append a cause class without relying on errno. (Converges with the M1 predicate change.)

- 🔵 **Security**: The residual shared-cache DoS is bounded, not eliminated
  **Location**: Desired End State / What We're NOT Doing
  A co-writer of a shared cache can still repeatedly plant a dead-owner `0o555` directory to bounce the victim to a bounded failure. Inherent to a predictably-named shared lock path; state explicitly that this is knowingly bounded rather than closed, so it reads as a decision, not an oversight.

### Strengths

- ✅ Proportionate and minimal: three coupled edits to one ~30-line function, preserving the inline classification structure rather than redesigning the lock scheme — complexity matches the requirement (code-quality, safety).
- ✅ The compound `elif`'s leading `[[ -n "${owner}" ]]` plus its ordering after the live-owner `if` correctly guarantees `rm`/`rmdir` only ever touch a *confirmed-dead* owner, and the empty-pid genuine-race `else` arm is preserved exactly by short-circuit (correctness).
- ✅ The Defect 2 fix is correct and minimal — dropping the unconditional `continue` routes any `rm`/`rmdir` failure to the budget-advancing tail, eliminating the unbounded busy-spin; the fail-fast/timeout exits leave no partial lock state (trap installed only after a successful `mkdir`) (correctness, safety).
- ✅ Complete arm-by-arm test mapping with a genuine red-green tripwire: the harness converts a hang into `pytest.fail`, and the plan adds a manual step (revert the bound, confirm it flips to a hang) to prove the test is not a tautology (test-coverage).
- ✅ Cross-platform-aware: every construct is bash-3.2-safe, `[[ -d ]]` is the deliberate portable choice given `mkdir` exposes no shell-portable errno, and the dead-owner test rests on portable POSIX unlink semantics with `_restricted`/`_require_unprivileged` hard-failing where the precondition would not bite (portability).
- ✅ Retains the 0186 probe gate as defence-in-depth with its own regression guard, and leaves the minisign verification chain and on-disk lock contract untouched (security, safety).

### Recommended Changes

1. **Narrow the classifier to a genuinely-permanent condition** (addresses: correctness TOCTOU major; security symlink minor; portability cause-collapse suggestion; test-coverage untested-trigger minor). Change `[[ ! -d "${lock_dir}" ]]` to `[[ -e "${lock_dir}" && ! -d "${lock_dir}" ]]` so a merely-absent directory (released competitor) falls through to the retry loop, and add symlink rejection (`[[ -L … ]]`) so an attacker-planted link is not followed by the reclaim `rm`/`rmdir`. Update the concurrent-test success criterion to note it now guards this race.

2. **Validate `ACCELERATOR_LOCK_MAX_WAIT` at read time** (addresses: security arithmetic-injection major; correctness bound-removal minor; safety production-footgun suggestion). Read via a `case`/regex guard to a non-negative integer, else fall back to 300, e.g. `case "${ACCELERATOR_LOCK_MAX_WAIT:-300}" in ''|*[!0-9]*) max_wait=300 ;; *) max_wait="${ACCELERATOR_LOCK_MAX_WAIT}" ;; esac`. Decide explicitly whether to also gate behind `ACCELERATOR_TEST_MODE`; either way state the decision in Migration Notes.

3. **Record the reclaim mutual-exclusion residual and its safety precondition** (addresses: safety major; security shared-cache-DoS suggestion). In What We're NOT Doing, state both the non-atomic `rm+rmdir` gap (two waiters can co-reclaim; a reclaim can destroy a live holder's dir under a plain TOCTOU) and the idempotency assumption (per-PID temps + atomic rename) that makes it safe today, flagging that it must be revisited before the lock ever guards a non-idempotent op.

4. **Raise the dead-owner test's harness `timeout` and add nets to tests 1 and 3** (addresses: test-coverage flake major; test-coverage no-net suggestion). Move `timeout=5` toward the 15 s precedent, and pass a modest `timeout=` to `test_uncreatable_lock_dir_fails_fast` and `test_empty_pid_lock_advances_budget` so a ceiling-check regression reds cleanly instead of hanging.

5. **Correct the three overstated prose claims** (addresses: correctness/safety bounded-budget minor; test-coverage red-first minor; portability manual-gate minor). Qualify "every arm terminates within a bounded budget" to name the live-owner and reclaim-success exceptions; state the expected pre-fix outcome per new test; name the macOS integration leg as the automated bash-3.2 gate and demote the manual replay to a spot-check.

6. **Optional clarity nits** (addresses: code-quality widened-else minor; code-quality naming minor; code-quality comment suggestion). Note the widened `else` contract, prefer a unit-bearing ceiling name, and trim the retained live-owner comment's flow-narrating tail if the region is being rewritten anyway.

---

## Per-Lens Results

### Correctness

**Summary**: The three-part fix is logically sound in its core intent — the compound `elif` guarantees `rm`/`rmdir` only touch a dead owner, the empty-pid genuine-race `else` arm is preserved exactly by short-circuit, and dropping the unconditional `continue` genuinely bounds Defect 2's spin. However, the new `[[ ! -d "${lock_dir}" ]]` predicate is too broad: it converts a transient contention race (a competitor releasing its directory between a waiter's failed `mkdir` and the `-d` check) into a spurious fatal error, a concurrency regression against behaviour the original code handled gracefully. A secondary gap is incomplete reasoning about malformed `ACCELERATOR_LOCK_MAX_WAIT` values in the `-gt` comparison.

**Strengths**:
- The compound `elif [[ -n "${owner}" ]] && rm -f … && rmdir …` preserves the invariant that a *live* owner's pid/directory is never removed (leading `[[ -n owner ]]` + ordering after the live-owner `if`).
- The empty-owner `else` entry is preserved exactly — with `owner=""`, both the live-owner `if` and reclaim `elif` short-circuit before any mutation.
- The Defect 2 fix is correct and minimal; the boundary `[[ "${waited}" -gt "${max_wait}" ]]` with `waited` starting at 0 and incremented before the check yields a correct bounded count.
- `max_wait="${ACCELERATOR_LOCK_MAX_WAIT:-300}"` correctly covers unset and set-but-empty under `set -u`; the AC2 `0o555` precondition correctly makes `rm -f` fail while `cat pid` still succeeds.

**Findings**:
- **Major (high)** — Phase 1 §1, the `[[ ! -d ]]` branch. TOCTOU: the directory's absence has three causes, the third being a competitor that released its lock via `release_lock` (`bin/accelerator:310-315`) between this waiter's EEXIST and the `-d` stat; the waiter spuriously aborts with `cannot create the launcher cache lock` instead of retrying. The original code fell to the `else`/race arm and acquired next iteration. Impact: under concurrent cold-cache bootstraps (the 6-process, ~1 s-hold `test_concurrent_cold_cache_slow_downloader_all_succeed`), a waiter can intermittently fail-closed — a latent CI flake contradicting the plan's own criterion. Suggestion: `[[ -e "${lock_dir}" && ! -d "${lock_dir}" ]]`, letting a merely-absent directory retry; the unwritable-parent case stays covered by the 0186 gate.
- **Minor (medium)** — Current State / Key Discoveries, `set -uo pipefail` with a malformed ceiling. The `:-300` default covers unset/empty, but a *set, non-integer* value makes the `-gt` operand an arithmetic syntax error; with no `-e`, the `if` evaluates false every iteration, the timeout `fail` never fires, and the reclaim arm loses its bound (re-introducing Defect 2); a value resolving to 0 fails after one iteration. The interaction is undefined and value-dependent, not "harmless". Suggestion: narrow the claim to valid integers, or validate at read time (`^[0-9]+$` → else 300).
- **Minor (medium)** — Desired End State, "every arm terminates within a bounded budget". The post-fix loop retains two budget-free arms: the live-owner reset (scoped out) and the reclaim-*success* `continue`, which loops back to `mkdir` without advancing `waited`; if an external process recreates the dead-owner directory between `rmdir` and the next `mkdir`, it can loop without advancing the budget. Not a pure busy-spin (needs external progress) but not "bounded by budget" as stated. Suggestion: name the exceptions.

### Test Coverage

**Summary**: The test strategy is strong — a dedicated or retained test maps onto all six post-fix arms, the harness's timeout→`pytest.fail` conversion is a genuine red tripwire for the unbounded-spin defect, and assertions are discriminating (verbatim cause substring plus negatives). All referenced fixtures and helpers exist and are used correctly, and the root-guard reasoning is sound. The main gaps are calibration and precision: `timeout=5` is tighter than the 15 s precedent (a flake risk), the "new tests fail against current code" step overstates the empty-pid guard, and two of three tests lack a harness timeout net.

**Strengths**:
- Complete arm-by-arm coverage: fail-fast, live-owner reset, dead-owner reclaim success, dead-owner unremovable/bounded, and empty-pid `else` each have a dedicated or retained test.
- Genuine red-green tripwire for the severe defect, plus an explicit manual step to prove the test is a tripwire not a tautology.
- Discriminating, mutation-resistant assertions (verbatim substring + negative assertions on both tests 1 and 3).
- Correct, verified root-guard reasoning: fail-fast needs none (mkdir over a planted file fails for root too); the dead-owner test carries `_require_unprivileged`; `_restricted` probes `os.access`.
- Faithful reuse of existing harness idioms and retention of the three existing guards.

**Findings**:
- **Major (medium)** — Phase 1 §3, `test_unremovable_dead_owner_lock_terminates_within_budget`. `timeout=5` vs the `timeout=15` of the closest analogue; the whole subprocess must finish in 5 s. Given documented shell-suite flakiness under parallel CI load, a loaded runner could exceed 5 s on a correctly-bounded run and produce a false `pytest.fail`. Suggestion: raise toward 15 s, well below the default ~30 s.
- **Minor (medium)** — Success Criteria, the `-k "lock"` checkbox. `test_empty_pid_lock_advances_budget` already passes pre-fix (a guard), and `test_uncreatable_lock_dir_fails_fast` only reds after ~30 s pre-fix (current code ignores the env knob). Suggestion: state the expected pre-fix outcome per test.
- **Suggestion (medium)** — Phase 1 §3, tests 1 and 3. No `timeout=` passed; a regression of the ceiling comparison itself would hang the suite from these rather than red cleanly (only test 2 carries the tripwire). Suggestion: add a modest `timeout=`.
- **Minor (low)** — Key Discoveries / What We're NOT Doing. The classifier's unwritable-parent trigger has no direct test (masked by the 0186 gate, guarded only indirectly); a future gate refactor would silently drop coverage. Suggestion: note the coupling.

### Code Quality

**Summary**: A tight, proportionate fix to a single ~30-line function that keeps the existing inline structure rather than over-engineering, with a clean, well-documented testability seam. The main readability cost is the compound `elif` folding the reclaim's side effects into a boolean guard and silently widening the `else` arm; the `max_wait` naming and a retained flow-narrating comment are smaller nits. Error handling is sound — the fail-fast path is an early guard clause with a distinct, cause-neutral message mirroring the sibling timeout diagnostic.

**Strengths**:
- Proportionate, minimal change preserving the inline classification structure.
- The fail-fast path is an early guard clause that short-circuits the wait loop clearly.
- The `cannot create the launcher cache lock: ${lock_dir}` message is cause-neutral (correct, no portable errno), distinct from the timeout message, and matches the in-file `<cause>: ${lock_dir}` shape.
- The reclaim fix reuses the shared `else` tail rather than duplicating the increment (DRY); reading `max_wait` once keeps the body clean.
- The env ceiling is a genuine seam mirroring an existing template and documented in the header note.
- Not declaring `max_wait`/`waited` as `local` matches the file's convention.

**Findings**:
- **Minor (high)** — Phase 1 §1, bounded reclaim arm. The compound `elif` makes two state mutations double as the branch predicate, hiding side effects inside what scans as a classification test, and the trailing `else` silently becomes a three-way catch-all. A partial reclaim (pid removed, dir remains) re-enters and is thereafter read as an empty-pid race. Defensible as the deliberate DRY choice; if kept, state that the `else` contract has broadened.
- **Minor (medium)** — Phase 1 §1 & §2. `max_wait` / `ACCELERATOR_LOCK_MAX_WAIT` reads as a duration in seconds but is an iteration count (~300 × 0.1 s) — a distinction the research already had to correct. Prefer a unit-bearing name; at minimum keep the header note's "iteration ceiling" wording.
- **Suggestion (medium)** — Phase 1 §1, the live-owner comment. Already present verbatim at `bin/accelerator:328-331` (retained, not introduced). Its first half is a legitimate *why*; its tail narrates control flow and carries a positional reference that can go stale. Optionally trim to the rationale sentence since the region is being rewritten.

### Portability

**Summary**: A tightly-scoped, cross-platform-aware shell fix. Every construct in the rewrite is bash-3.2-safe, so the macOS `/bin/bash` floor is respected; directory-presence is a deliberate portable choice over an errno; and the dead-owner test's precondition rests on portable POSIX unlink semantics that hold on macOS APFS/HFS+ and Linux ext4/tmpfs. No new vendor or hardcoded-path coupling. The one framing gap: the plan treats bash-3.2 verification as manual when the integration suite already forces `/bin/bash` on a `macos-latest` CI leg, so the floor is enforced automatically.

**Strengths**:
- All rewritten constructs are bash-3.2-safe, and the suite pins `/bin/bash` (`installation.py:43`) — bash 3.2 on `macos-latest` — so the three tests run under the real floor on both matrix legs automatically.
- `[[ -d ]]` is a deliberate, portability-correct discriminator given `mkdir` exposes no portable errno.
- Test determinism depends only on portable POSIX semantics; `_restricted`'s `os.access` probe hard-fails on advisory-permission filesystems and `_require_unprivileged` hard-fails under root.
- No new hardcoded paths or vendor coupling; the ceiling is a documented runtime-injectable seam defaulting to production behaviour.

**Findings**:
- **Minor (medium)** — Success Criteria, Manual Verification. The bash-3.2 replay is listed only as manual, but the harness pins `/bin/bash` and the `test-integration` job runs the `macos-latest` leg (`main.yml:55-61`), so the tests already execute under 3.2 in CI. Mischaracterising the gate as manual risks under-valuing (or removing) that leg and misleads a Linux-only contributor whose `mise run` uses bash 5.x. Suggestion: name the macOS leg as the automated gate; demote the replay to a spot-check.
- **Suggestion (low)** — Phase 1 §1, the fail-fast diagnostic. Because `mkdir` has no portable errno, one cause-neutral message covers a file at the path, an unwritable parent, a read-only mount, and ENOSPC alike. Reasonable trade-off, but a cheap `[[ -e ]]` follow-up could append a cause class without relying on errno.

### Safety

**Summary**: A net safety improvement for a low-criticality dev-tool bootstrap lock: it eliminates the more severe defect (an unbounded, no-sleep busy-spin) and converts a misleading full-budget timeout into a fast, correctly-named fail. The fail-fast and timeout exits leave no partial lock state. Two residual footguns in the retained scheme deserve recording: the reclaim arm's non-atomic `rm -f pid; rmdir` is not single-winner (a TOCTOU mutual-exclusion gap the plan scopes out only on its live-owner dual), and the live-owner arm remains an unbounded wait under PID reuse, so "every arm terminates within a bounded budget" is not literally complete.

**Strengths**:
- Eliminates the unbounded busy-spin (Defect 2), removing a genuine CPU-pegging runaway with no timeout.
- Fail-fast and timeout exits leave on-disk state clean: the `[[ ! -d ]]` branch fires before any lock dir exists, the trap installs only inside the mkdir-success block, and `release_lock` short-circuits on empty `lock_held`; no foreign artifact is ever removed.
- The protected critical section is idempotent (per-PID `.tmp-launcher-$$` + atomic rename of identical verified content), structurally bounding the harm of any residual reclaim race.
- Retains the 0186 probe gate as defence-in-depth.
- Sub-second determinism via an injectable ceiling plus a real tripwire.

**Findings**:
- **Major (medium)** — What We're NOT Doing / Phase 1 §1. The plan scopes out only the live-owner-resets-forever concern but leaves unrecorded the dual reclaim-side hazard: `rm -f pid; rmdir` is a non-atomic read-then-act, so two waiters can each reclaim, and a reclaim can destroy a *live* holder's freshly-created dir (a plain TOCTOU, not the reused-pid framing), letting two sessions enter the critical section. Tolerable only because the protected op is idempotent — but that assumption is unstated, so a future read-modify-write critical section would inherit a lost-write hole. Suggestion: record both the gap and the idempotency assumption; flag it must be revisited before the lock guards a non-idempotent op.
- **Minor (medium)** — Desired End State / What We're NOT Doing. "Every arm terminates within a bounded budget" overstates: the live-owner arm resets `waited=0` and, under PID reuse of the recorded owner, resets forever — a false-positive unbounded hang recorded only as a budget-*reset* concern, not reconciled with the headline claim. Suggestion: qualify the claim and record the live-owner PID-reuse residual (the `holder.start` precedent being the future fix).
- **Suggestion (low)** — What We're NOT Doing / Migration Notes. Always-on `ACCELERATOR_LOCK_MAX_WAIT` (vs the `ACCELERATOR_TEST_MODE`-gated `jira-common.sh` precedent) is a small production footgun; an accidental value shrinks the budget and could cause spurious timeouts under genuine contention (a non-numeric value degrades safe via `[[ -gt ]]`). Suggestion: gate to match, or record why always-overridable is acceptable.

### Security

**Summary**: A net security improvement for its core purpose — it removes an attacker-triggerable unbounded busy-spin (CPU-exhaustion DoS) by bounding the reclaim arm, and fails fast with a non-leaking message. However, it introduces `ACCELERATOR_LOCK_MAX_WAIT`, which flows unvalidated into a `[[ -gt ]]` arithmetic comparison inside a root-of-trust bootstrap — a classic bash arithmetic-injection surface — and the "always-on" decision keeps that surface live in production. The filesystem-level vectors (symlink following, a foreign/unremovable lock directory in a shared cache) are bounded but not eliminated; those are largely inherent to the mkdir+pid scheme the plan deliberately does not redesign.

**Strengths**:
- Removing the reclaim `continue` closes a genuine CPU-exhaustion DoS.
- The fail-fast message discloses only the local lock path (no permissions/uid/errno), consistent with the `fail` idiom.
- Explicitly retains the 0186 probe gate as defence-in-depth.
- The on-disk lock/pid contract and minisign verification chain are untouched.
- The unset-env case degrades to the safe default via `${…:-300}` under `set -u`.

**Findings**:
- **Major (high)** — Phase 1 §1 (`max_wait="${…:-300}"` and `[[ "${waited}" -gt "${max_wait}" ]]`), reinforced by the always-on decision. `-gt` evaluates operands as arithmetic, and bash arithmetic re-expands array-subscript syntax, so `a[$(command)]` executes `command` (present on the bash 3.2 floor). This is the first externally-influenced value to reach an arithmetic context in `bin/accelerator`; non-injection abuse (`0`/negative → immediate DoS; huge → long wait) also applies, and ShellCheck will not flag it. Impact: an actor able to influence the process environment (direnv/`.envrc`, shared profile, CI config) can achieve command execution as the user before verification, or force a lock-wait DoS. Suggestion: validate as a non-negative integer with a `300` fallback (`case … in ''|*[!0-9]*) …`), or gate behind `ACCELERATOR_TEST_MODE`.
- **Minor (medium)** — Phase 1 §1, the classification branch. `[[ -d ]]` follows symlinks: a symlink at the lock path reads as a competitor and drives the reclaim through the target — `rm -f "${lock_dir}/pid"` deletes a `pid`-named file in the attacker-chosen dir; `rmdir` fails ENOTDIR. A narrow file-deletion-plus-DoS primitive, pre-existing, realistic only on a shared/multi-user-writable cache. Suggestion: reject a symlink (`[[ -L … ]]`) in the classification branch, or record the residual.
- **Suggestion (medium)** — Desired End State / What We're NOT Doing. The residual shared-cache DoS is bounded, not eliminated: a co-writer can repeatedly plant a dead-owner `0o555` directory to bounce the victim to a bounded failure; and the `mkdir`/`[[ -d ]]` TOCTOU only steers between two bounded outcomes (no escalation). Suggestion: state explicitly that the shared-cache DoS is knowingly bounded rather than closed.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-08-21T16:24:32+00:00

**Verdict:** REVISE

The Pass 1 revision resolved every major and nearly every minor, and both high-confidence Pass 1 majors (the classifier TOCTOU and the arithmetic-injection surface) are confirmed closed by the lenses that raised them. But the re-review surfaced a **new high-confidence major that the Pass 1 fix itself introduced** — the `case` ceiling validation accepts all-digit strings without forcing base 10, so a leading-zero value (`08`/`09`) reaches `[[ -gt ]]` as an invalid-octal literal and, under `set -uo pipefail` with no `-e`, silently removes the loop's bound — flagged independently by Correctness and Safety. A second major (Correctness + Test Coverage) found the Pass 1 rewrite of the red-first Success Criteria became inaccurate and self-contradictory once the `timeout=15` nets were added, and the newly-added `-L` symlink branch had no test. All Pass 2 findings were addressed in an immediate follow-up edit (see Assessment); the REVISE verdict reflects the state the six agents reviewed and is pending a third verification pass.

### Previously Identified Issues

- 🟡 **Correctness** — TOCTOU classifier misreads a released competitor — **Resolved.** Narrowed to `[[ -L … ]] || [[ -e … && ! -d … ]]`; an absent directory now falls through and retries. Correctness re-review confirms the ordering and short-circuit are sound.
- 🟡 **Security + Correctness** — unvalidated `ACCELERATOR_LOCK_MAX_WAIT` (injection + bound-loss) — **Partially resolved.** Injection fully sealed (Security confirms the `case` glob matches lexically, no evaluation). The bound-loss half recurred via the octal sub-case (see New Issues).
- 🟡 **Safety** — reclaim non-single-winner mutual exclusion — **Resolved.** Recorded in *What We're NOT Doing* with the idempotency precondition; Safety confirms the record accurate and the idempotency claim true of `fetch_and_verify`.
- 🟡 **Test Coverage** — `timeout=5` flake risk — **Resolved.** Raised to `timeout=15`; Test Coverage confirms it is well-calibrated (above the injected budget, below the default ~30 s).
- 🔵 **Security** — `[[ -d ]]` follows symlinks — **Resolved.** The `-L` guard rejects symlink-to-dir, symlink-to-file, and dangling symlinks before any reclaim `rm`/`rmdir`.
- 🔵 **Correctness + Safety** — "every arm terminates within a bounded budget" overstated — **Partially resolved.** Desired End State now qualifies it; Safety noted the reclaim-*success* no-sleep spin was still conflated with the sleeping waits (addressed in Pass 2).
- 🔵 **Test Coverage** — red-first pre-fix outcomes overstated — **Regressed → new major.** The Pass 1 rewrite plus the added nets made the narrative inaccurate (see New Issues).
- 🔵 **Portability** — bash-3.2 gate framed as manual-only — **Resolved.** Portability verified the `macos-latest` integration leg runs the tests under `/bin/bash` 3.2 end-to-end.
- 🔵 **Code Quality** — retained comment tail narrated flow — **Resolved.** Trimmed to rationale; Code Quality confirms it now fits the low-comment rule.
- 🔵 Other Pass 1 minors/suggestions (widened-`else` note, no-net tests, always-on footgun, shared-cache DoS record, unwritable-parent coupling) — **Resolved** in Pass 1.

### New Issues Introduced

- 🔴 **Correctness + Safety** (high) — Leading-zero ceiling silently disables the bound. `case … *[!0-9]*` accepts `08`/`09`; `[[ -gt "08" ]]` reads octal, errors on the invalid digit, and returns non-zero every iteration under no-`-e`, so the timeout `fail` never fires — the exact loss-of-bound Pass 1 claimed to close. Introduced by the Pass 1 validation.
- 🟡 **Correctness + Test Coverage** (high) — Success Criteria red-first narrative inaccurate and self-contradictory. With the injected ceiling inert on current `main`, all new tests red as `timeout=15` hangs, contradicting the stated "wrong message after ~30 s" (test 1) and "green both before and after" (test 3).
- 🟡 **Test Coverage** (high) — The `-L` symlink fail-fast branch had no test; a refactor dropping `-L` would fail no planned case.
- 🟡 **Safety** (med) — The reclaim-*success* `continue` is a no-sleep spin the qualification lumped with the sleeping live-owner reset.
- 🔵 **Code Quality** (med) — The `{ …; }` grouping in the guard is over-engineered; `300` appears twice.
- 🔵 **Security / Safety** (minor) — A check-to-`rm` symlink-swap TOCTOU and the fact that `release_lock` shares the non-owner-gated `rm+rmdir` shape were unrecorded.
- 🔵 **Portability / Test Coverage** (low/minor) — The `case` matching arm and the absent-path fall-through were unexercised; the latter cannot be pinned deterministically.

### Assessment

The re-review did its job: it caught a real regression the Pass 1 fix introduced (the octal bound-loss) — found independently by two lenses — and a prose fix that had turned self-contradictory. Both, plus every other Pass 2 finding, were addressed in an immediate follow-up edit:

- Ceiling now normalised with `case … *) max_wait=$((10#${max_wait})) ;;` (base 10; `08` → 8, not an octal error).
- Guard simplified to `[[ -L … ]] || [[ -e … && ! -d … ]]` (no brace group).
- Success Criteria restated: all five new tests red against current `main` as `timeout=15` hangs and pass sub-second after the fix.
- Two tests added — `test_symlink_lock_path_fails_fast` (pins the `-L` guard and that reclaim does not follow the link) and `test_leading_zero_ceiling_is_decimal_not_octal` (pins base-10 normalisation).
- *What We're NOT Doing* extended: reclaim-success no-sleep spin, `release_lock` shares the non-owner-gated shape, the check-to-`rm` symlink TOCTOU, and the per-user-cache trust assumption.

The plan is in materially better shape than at either prior state. The verdict remains REVISE because these Pass 2 fixes have not themselves been reviewed; a third pass (Correctness, Safety, Test Coverage) would confirm the octal fix, the restated red-first narrative, and the two new tests, and is expected to clear to APPROVE.

## Re-Review (Pass 3) — 2026-08-21T19:51:37+00:00

**Verdict:** APPROVE

The three lenses whose findings the Pass 2 edits touched re-reviewed the twice-revised plan. Correctness and Safety returned **zero findings** — both verified, against the live `bin/accelerator`, that the base-10 ceiling normalisation fully closes the octal loss-of-bound (only all-digit strings reach `$((10#…))`, and the `*)` arm references `${max_wait}` not the bare env var, so no `set -u` abort), that the simplified `[[ -L … ]] || [[ -e … && ! -d … ]]` guard is logically identical to its predecessor, and that the reclaim/`release_lock`/TOCTOU residuals are accurately recorded. Test Coverage found no defect in the fix itself but three consistency/coverage issues — a `-k "lock"` verification command that silently dropped the leading-zero test, three stale "three tests" lead-ins, a vacuous no-follow assertion in the symlink test, and the untested non-numeric fallback arm. All were addressed in a follow-up edit; the verdict is APPROVE because the two substantive lenses are clean and the remaining items were documentation/consistency fixes plus one inherent, now-recorded coverage limitation.

### Previously Identified Issues (Pass 2 findings)

- 🔴 **Correctness + Safety** — leading-zero octal loss-of-bound — **Resolved and verified.** `$((10#${max_wait}))` normalises to decimal; both lenses confirmed no residual value can disable the timeout.
- 🟡 **Correctness + Test Coverage** — red-first narrative inaccurate — **Resolved and verified.** Correctness and Test Coverage both confirm the restated "all five red as `timeout=15` hangs pre-fix, pass sub-second post-fix" is accurate per test.
- 🟡 **Test Coverage** — `-L` branch untested — **Resolved.** `test_symlink_lock_path_fails_fast` added; its no-follow assertion strengthened in Pass 3 (see below).
- 🟡 **Safety** — reclaim-success no-sleep spin conflated — **Resolved and verified.** Recorded distinctly; Safety confirms the characterisation accurate.
- 🔵 Code-quality / Security / Portability minors (brace-group simplified, `release_lock` note, symlink-swap TOCTOU, case-arm coverage) — **Resolved.**

### New Issues Introduced

- 🟡 **Test Coverage** (high) — `-k "lock"` selector silently excluded `test_leading_zero_ceiling_is_decimal_not_octal` (no "lock" substring). **Fixed:** selector broadened to `-k "lock or leading_zero"`.
- 🟡 **Test Coverage** (high) — three stale "three tests" lead-ins (Overview, Changes #3, Manual step 1) contradicted the five enumerated tests. **Fixed:** all three now say "five".
- 🟡 **Test Coverage** (med) — the non-numeric / arithmetic-injection fallback arm has no test. **Recorded as an inherent limitation** (not fixed with a test): any rejected value falls back to the 300-iteration ~30 s default and a fail-fast setup never reaches `[[ -gt ]]`, so a payload never executes regardless of the guard — a behavioural test is either vacuous or ~30 s. Covered by the `case`-before-arithmetic ordering (verified by Correctness Pass 3) and ShellCheck.
- 🔵 **Test Coverage** (minor) — the symlink test's `target.is_dir()` no-follow assertion was vacuous (empty target → reclaim never reached). **Fixed:** the test now plants a dead-owner pid inside the target and asserts it survives, a genuine reclaim-no-follow tripwire.

### Assessment

The three-pass review converged. The substantive correctness and safety of the fix are verified clean by two independent passes; the fix now closes both original defects (misclassification, unbounded reclaim) without the regression Pass 2 introduced (octal loss-of-bound). The Pass 3 items were mechanical consistency fixes (a selector string, count numbers, one strengthened assertion) plus one coverage limitation that is inherent to the design and now explicitly recorded. Those Pass 3 fixes are self-evidently correct and low-risk; the plan is ready for implementation. Remaining scoped-out residuals (reclaim not single-winner, reused-pid live-owner reset, shared-cache DoS) are recorded decisions, safe under the current idempotent critical section.
