---
type: "plan-validation"
id: "2026-08-21-0190-classify-lock-mkdir-failures-validation"
title: "Validation Report: acquire_lock mkdir classification and bounded reclaim"
date: "2026-08-22T19:11:26+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "work-item:0190"
target: "plan:2026-08-21-0190-classify-lock-mkdir-failures"
tags: ["bug", "shell", "bootstrap", "bash-3.2", "locking"]
last_updated: "2026-08-22T19:11:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: acquire_lock mkdir classification and bounded reclaim

Implementation matches the plan verbatim. The single phase landed in one commit
(`mumkrlwxwxvz`, "Classify acquire_lock mkdir failures and bound the reclaim
arm") touching exactly the three planned surfaces; all runnable automated checks
pass.

### Implementation Status

✓ Phase 1: Classify the mkdir failure and bound the reclaim arm — fully
implemented

All three coupled edits landed in the planned 10-line region of
`bin/accelerator`:

- **Classification branch** — `[[ -L ]] || [[ -e && ! -d ]]` fail-fast on a
  symlink or non-directory at the lock path, sitting between the `mkdir`-success
  block and the pid read (`bin/accelerator:337-341`).
- **Bounded reclaim** — the reclaim arm is now a compound `elif` whose
  `continue` fires only when both `rm -f` and `rmdir` succeed; any failure falls
  through to the budget-advancing `else` (`bin/accelerator:346-349`).
- **Injectable ceiling** — `max_wait` read once, validated all-digit, base-10
  normalised via `$((10#…))` (`bin/accelerator:318-323`), compared as a decimal
  integer at `:351`.

Header "Test seams" note extended with `ACCELERATOR_LOCK_MAX_WAIT` (`:18`). Five
integration tests added to `test_accelerator_entrypoint.py:325-445`.

### Automated Verification Results

✅ New lock tests pass (`pytest … -k "lock or leading_zero"`): 6 passed, 3.52 s —
the five new tests plus `test_stale_lock_is_reclaimed`.
✅ Retained guards pass (`-k "stale_lock or slow_downloader or
readonly_cache_fails_fast"`): 3 passed.
✅ Shell lint clean (`mise run scripts:check`): bashisms, exec-bits, shfmt,
ShellCheck all green.
✅ Read-only CI mirror (`mise run check`): exit 0.

⚠️ Full default gate (`mise run`, AC6) not re-run this session — it was marked
`[x]` at implementation time. The read-only mirror plus the affected test suite
are green; the outstanding delta is the docs CI lane and the full test sweep,
neither touched by this change. The dead-owner test terminated sub-second inside
the 6-test run (3.52 s total), confirming the bound rather than the `timeout=15`
tripwire ends it (Manual Verification item 2).

### Code Review Findings

#### Matches Plan:

- Loop body rewrite is byte-identical to the plan's proposed `acquire_lock`,
  including the widened `else` contract and the arm ordering documented in the
  plan.
- The `sleep 0.1` shared tail sits outside the `if/elif/else`, so the reclaim
  arm's `continue` is the only branch that skips it — exactly the plan's
  intended budget mechanism.
- All five tests carry the planned preconditions, discriminators, and root
  guards (`_require_unprivileged` on the dead-owner test only).
- The live-owner-reset comment was extended one clause beyond the plan snippet
  ("The budget elapses only while no live owner is seen…") — a clarifying
  addition, not a deviation.

#### Deviations from Plan:

- None material. The plan file itself shows an 18-line diff in the commit
  (success-criteria boxes flipped to `[x]`), expected as part of closing the
  plan.

#### Potential Issues:

- None introduced. The residual availability classes (reused-pid live-owner
  reset, shared-cache DoS, reclaim-success hot-spin) are explicitly scoped out
  in the plan's "What We're NOT Doing" and remain unchanged.
- Two branches stay behavioural-test-blind by design — absent-path fall-through
  and non-numeric ceiling fallback — covered by inspection plus ShellCheck, as
  the plan states.

### Manual Testing Required:

1. bash 3.2 floor:
  - [ ] The `macos-latest` integration leg is the automated 3.2 gate; a local
    `/bin/bash` spot-check on macOS is an optional supplementary backstop. The
    fix uses only 3.2-safe constructs.

2. Tripwire confirmation (optional):
  - [ ] Temporarily restore the bare `continue` and confirm
    `test_unremovable_dead_owner_lock_terminates_within_budget` flips to a
    `pytest.fail` hang, proving the test is a genuine tripwire.

### Recommendations:

- Run the full `mise run` default gate once before merge to cover the docs CI
  lane, per AC6 — the only automated criterion not re-exercised this session.
- No code changes recommended. The implementation is faithful and the fix
  strictly improves worst-case behaviour (removes an unbounded busy spin).
