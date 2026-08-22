---
type: plan
id: "2026-08-21-0190-classify-lock-mkdir-failures"
title: "acquire_lock mkdir classification and bounded reclaim Implementation Plan"
date: "2026-08-21T15:21:54+00:00"
author: Toby Clemson
producer: create-plan
status: done
work_item_id: "work-item:0190"
parent: "work-item:0190"
derived_from: ["codebase-research:2026-08-21-0190-acquire-lock-mkdir-classification"]
tags: [bug, shell, bootstrap, bash-3.2, locking]
revision: "4390192c1d375fa7646436166b422a1268f6ea42"
repository: "accelerator"
last_updated: "2026-08-21T15:21:54+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# acquire_lock mkdir classification and bounded reclaim Implementation Plan

## Overview

Fix two defects in `acquire_lock` (`bin/accelerator:317-345`): a failed `mkdir`
on an unusable lock path is misclassified as contention and reported as a lock
timeout after the full ~30 s budget, and the dead-owner reclaim arm `continue`s
past the loop's shared budget tail so a `rmdir` that keeps failing spins
unbounded. Three coupled edits to one function — a classification branch, a
bounded reclaim arm, and an env-injectable iteration ceiling that makes the
bounded arm testable sub-second — plus five deterministic integration tests.

## Current State Analysis

The loop `mkdir`s the lock directory and, on any failure, unconditionally
assumes contention: it reads `${lock_dir}/pid` and branches three ways on owner
liveness.

```bash
acquire_lock() {
	waited=0
	while true; do
		if mkdir "${lock_dir}" 2>/dev/null; then
			printf '%s\n' "$$" >"${lock_dir}/pid" 2>/dev/null
			lock_held=1
			trap release_lock EXIT INT TERM
			return 0
		fi
		owner=$(cat "${lock_dir}/pid" 2>/dev/null)
		if [[ -n "${owner}" ]] && kill -0 "${owner}" 2>/dev/null; then
			# ... live owner: reset budget
			waited=0
		elif [[ -n "${owner}" ]]; then
			rm -f "${lock_dir}/pid" 2>/dev/null
			rmdir "${lock_dir}" 2>/dev/null
			continue
		else
			waited=$((waited + 1))
			if [[ "${waited}" -gt 300 ]]; then
				fail "timed out acquiring the launcher cache lock: ${lock_dir}"
			fi
		fi
		sleep 0.1
	done
}
```

The `sleep 0.1` at `:343` is **outside** the `if/elif/else` — a shared tail run
by the live-owner and `else` arms but skipped by the reclaim arm's `continue`.
That `continue`, bypassing the loop's single budget-and-sleep tail, is the
unbounded-spin mechanism.

- **Defect 1 — misclassification.** A failed `mkdir` whose directory is absent
  (a non-directory occupies the path, or the parent is unwritable) yields
  `owner=""`, routing to the `else` arm, which advances the budget toward a
  lock-timeout `fail`. Wrong diagnostic, and only after 300 × `sleep 0.1`.
- **Defect 2 — unbounded reclaim.** A dead owner on a lock directory that
  cannot be removed (`rm -f`/`rmdir` both fail) fires `continue` with `waited`
  untouched and no sleep. A busy spin with no timeout — the more severe defect.

## Desired End State

`acquire_lock` classifies the `mkdir` failure before assuming contention. The
three failure-classification arms — fail-fast, dead-owner-unremovable, and
empty-pid — terminate within a bounded budget. Two arms are *not* bounded by
`max_wait`: the live-owner reset (which sleeps 0.1 s per iteration) is bounded by
the owner completing, and the reclaim-*success* `continue` is a no-sleep
fast-retry bounded only by an external process ceasing to recreate a removable
dead-owner directory. See What We're NOT Doing. Verify by:

- `bin/.accelerator-lock-<platform>` occupied by a **file or symlink** → fails
  fast with `cannot create the launcher cache lock: <path>`, not a lock timeout.
- A dead-owner lock directory whose pid cannot be removed → terminates with the
  existing lock-timeout message within the budget, never spins.
- An empty-pid lock directory → still advances the budget to the same timeout
  (the genuine-race `else` arm is intact).
- A competitor that releases its lock mid-wait → the waiter retries and acquires,
  never fails fast (guarded by the concurrent-cold-cache test).
- Existing reclaim and concurrent-cold-cache tests stay green.

### Key Discoveries:

- Both defects and the fix live in one function: `bin/accelerator:317-345`.
- The `continue` at `:336` is the unbounded-spin mechanism — it skips the shared
  `sleep 0.1`+budget tail at `:343` (`bin/accelerator:333-343`).
- ⚠️ The 0186 gate `require_exec_capable_cache` (`bin/accelerator:386`) probes
  **`cache_dir`**, not the lock **subdirectory**, so a `chmod`-unwritable cache
  dir is caught before `acquire_lock`. AC1's fail-fast branch is therefore
  reachable in a test only via a **non-directory (or symlink) at the lock path**
  — no `chmod`, no root guard. The branch's *unwritable-parent* trigger stays
  masked by this gate, so its coverage is coupled to gate retention: a future
  change removing or refactoring the gate must add a direct test for that
  trigger. AC2's precondition (`chmod` the lock dir `0o555`) is unaffected and
  keeps the root guard.
- `bin/accelerator` runs under `set -uo pipefail` — **no `-e`** (`:25`). The
  ceiling is validated to an all-digit string and normalised to base 10
  (`$((10#…))`) at read time, so `[[ "${waited}" -gt "${max_wait}" ]]` always
  compares two decimal integers: a non-numeric `ACCELERATOR_LOCK_MAX_WAIT` falls
  back to 300 and a leading-zero value (`08`, `09`) is read decimally, not as an
  invalid-octal literal. This closes the arithmetic-injection surface (`-gt`
  evaluates operands as arithmetic, which re-expands `a[$(cmd)]` subscript
  syntax) and the silent loss-of-bound — whether from a bare non-integer or an
  octal error — that would otherwise return non-zero every iteration under
  no-`-e` so the timeout `fail` never fires. `"${VAR:-300}"` still covers the
  `set -u` unset case.
- `run_bootstrap` converts a subprocess `timeout=` into a hard `pytest.fail`
  (`tests/integration/support/installation.py:333-334`), so an unbounded
  regression reds the suite rather than hanging it.
- Precedent for the fail-fast writability pre-check: `atomic-common.sh:104-109`.
  Precedent for the env ceiling: `jira-common.sh:146-151` (gated on
  `ACCELERATOR_TEST_MODE`; this file has no such gate, so we go always-on).

## What We're NOT Doing

- Not re-wording the existing lock-timeout message.
- Not redesigning the mkdir+pid locking scheme.
- Not removing the 0186 probe gate — retained as defence-in-depth with its own
  guard `test_unverifiable_launcher_in_readonly_cache_fails_fast`.
- Not making `sleep 0.1` injectable — only the iteration ceiling; a low ceiling
  already yields sub-second tests.
- Not gating the ceiling behind `ACCELERATOR_TEST_MODE`. The override stays
  always-on to match this file's plain `"${VAR:-default}"` convention; the value
  is instead validated and base-10 normalised, which is what actually closes the
  injection and bound-loss risks. Recorded in Migration Notes.
- Not making the reclaim single-winner. `rm -f pid; rmdir` is non-atomic, so two
  waiters that read the same dead owner can both reclaim, and a reclaim can
  destroy a live holder's freshly-created directory (a plain TOCTOU, distinct
  from the reused-pid case below). This is safe **only** because the critical
  section is idempotent — `fetch_and_verify` writes per-PID temp files and
  commits via atomic rename of identical verified content, so a double-entry
  cannot corrupt the cache. Revisit before the lock ever guards a non-idempotent
  op; `atomic-common.sh` linearises this via an atomic `mv` of a nonce'd
  sentinel. `release_lock` (`:310-315`) is the same non-owner-gated
  `rm -f pid; rmdir`, so a future single-winner change must cover both call
  sites.
- Not adding a `sleep`/budget step to the reclaim-*success* `continue`. It is a
  deliberate no-sleep fast-retry (to grab a just-freed lock quickly), so a
  co-writer repeatedly recreating a *removable* dead-owner directory in the
  `rmdir`→`mkdir` window can hot-spin it — a pre-existing, tight-race,
  low-probability path unchanged by this fix, distinct from the sleeping
  live-owner reset.
- Not closing the shared-cache denial-of-service. On a shared
  `ACCELERATOR_CACHE_DIR` a co-writer can repeatedly plant a dead-owner `0o555`
  directory to bounce a victim to a bounded failure; the fix bounds the worst
  case (no more unbounded spin) but does not eliminate denial on a predictably
  named, shared lock path. A co-writer could also swap the lock path for a
  symlink between the `-L` check and the reclaim `rm`, a narrow
  delete-a-`pid`-file primitive against an attacker-chosen target. Both rest on
  `ACCELERATOR_CACHE_DIR` pointing at a shared, world-writable directory; the
  supported cache is the per-user `${plugin_root}/bin`, which is not
  attacker-writable.
- Not fixing the live-owner-resets-forever-on-reused-pid concern: a foreign
  process at a reused pid keeps resetting the budget — a residual *unbounded
  wait*, the same availability class 0190 otherwise closes. `jira-common.sh`
  solves the analogue with a `holder.start` check; 0190 scopes it out.

## Implementation Approach

One phase. The three code changes edit the same 10-line region and share the
ceiling test seam, so splitting them would leave either a dead env knob or a
half-fix whose tests cannot run sub-second. The additional-instructions rule
("phases independently mergeable") is satisfied trivially by a single
self-contained, green phase; 0191 edits a different region of the file, so there
is no merge coupling.

TDD sequence within the phase: the bounded-arm test (Defect 2) is written first
because it is red against the current code as a hang, then the ceiling seam and
reclaim bound turn it green; the classification tests (Defect 1 — a file and a
symlink at the lock path) follow the same red-green step; the empty-pid and
leading-zero-ceiling tests land last as guards that the `else` arm survived and
that the ceiling is read as base 10.

---

## Phase 1: Classify the mkdir failure and bound the reclaim arm

### Overview

Add a classification branch, fold the reclaim into a compound condition so any
`rm`/`rmdir` failure falls to the budget-advancing `else`, and replace the
literal `300` with a validated `"${ACCELERATOR_LOCK_MAX_WAIT:-300}"` ceiling.

### Changes Required:

#### 1. `acquire_lock` — classification, bounded reclaim, injectable ceiling

**File**: `bin/accelerator`
**Changes**: Rewrite the loop body. A classification branch sits between the
`mkdir`-success block and the pid read: it fails fast only on a **permanent**
condition — a symlink at the lock path, or a non-directory occupying it — while a
merely-*absent* directory (a competitor that released its lock between this
waiter's failed `mkdir` and the check) falls through to the retry loop, so a
released competitor is retried rather than misclassified. The `-L` guard also
stops the reclaim `rm`/`rmdir` following an attacker-planted symlink. The reclaim
arm becomes a compound `elif` whose `continue` fires only when both `rm -f` and
`rmdir` succeed; any failure falls through to the `else` — whose contract is thus
wider than empty-pid alone, also absorbing a dead owner whose lock could not be
removed. `max_wait` is read once at the top, validated to an all-digit string,
and normalised to base 10 with `$((10#…))`, so a non-numeric
`ACCELERATOR_LOCK_MAX_WAIT` falls back to 300 and a leading-zero value (e.g.
`08`) is read decimally — the value can never reach the `[[ -gt ]]` arithmetic
operand as an unchecked string or as an octal literal.

```bash
acquire_lock() {
	max_wait="${ACCELERATOR_LOCK_MAX_WAIT:-300}"
	case "${max_wait}" in
		*[!0-9]*) max_wait=300 ;;
		*) max_wait=$((10#${max_wait})) ;;
	esac
	waited=0
	while true; do
		if mkdir "${lock_dir}" 2>/dev/null; then
			printf '%s\n' "$$" >"${lock_dir}/pid" 2>/dev/null
			lock_held=1
			trap release_lock EXIT INT TERM
			return 0
		fi
		if [[ -L "${lock_dir}" ]] ||
			[[ -e "${lock_dir}" && ! -d "${lock_dir}" ]]; then
			fail "cannot create the launcher cache lock: ${lock_dir}"
		fi
		owner=$(cat "${lock_dir}/pid" 2>/dev/null)
		if [[ -n "${owner}" ]] && kill -0 "${owner}" 2>/dev/null; then
			# A live owner is fetching, bounded by curl's --max-time; reset the
			# abort budget so a slow-but-progressing cold fetch never fails a
			# waiter.
			waited=0
		elif [[ -n "${owner}" ]] && rm -f "${lock_dir}/pid" 2>/dev/null &&
			rmdir "${lock_dir}" 2>/dev/null; then
			continue
		else
			waited=$((waited + 1))
			if [[ "${waited}" -gt "${max_wait}" ]]; then
				fail "timed out acquiring the launcher cache lock: ${lock_dir}"
			fi
		fi
		sleep 0.1
	done
}
```

Post-fix arm order: `mkdir` ok → hold; `mkdir` fails + symlink or non-directory
at the path → fail fast; `mkdir` fails + path absent (a competitor just
released) → fall through and retry; live owner → reset budget; dead owner +
`rm`+`rmdir` ok → reclaim and retry; dead owner + `rm`/`rmdir` fails, or
empty/unreadable pid → advance budget then shared `sleep`.

#### 2. Document the new env seam

**File**: `bin/accelerator`
**Changes**: Extend the header "Test seams" note (`:17-18`) so the knob is
discoverable alongside the existing seams.

```diff
 # Test seams (unset in production): ACCELERATOR_UNAME_S/_M override host
-# detection; ACCELERATOR_BOOTSTRAP_DOWNLOADER injects a `downloader <url> <dest>`.
+# detection; ACCELERATOR_BOOTSTRAP_DOWNLOADER injects a `downloader <url> <dest>`;
+# ACCELERATOR_LOCK_MAX_WAIT caps the lock-wait iteration ceiling (default 300).
```

#### 3. Integration tests

**File**: `tests/integration/entrypoint/test_accelerator_entrypoint.py`
**Changes**: Add five tests near `test_stale_lock_is_reclaimed` (`:311-323`),
reusing its default-cache lock path (`bin/.accelerator-lock-<platform>`),
`_require_unprivileged`, and `_restricted`.

```python
def test_uncreatable_lock_dir_fails_fast(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    # A file at the lock path: mkdir fails and the path is a non-directory.
    (root / f"bin/.accelerator-lock-{host_platform}").write_text("")
    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={"ACCELERATOR_LOCK_MAX_WAIT": "3"},
        timeout=15,
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "cannot create the launcher cache lock" in output, output
    assert "timed out" not in output, output


def test_unremovable_dead_owner_lock_terminates_within_budget(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    _require_unprivileged()
    harness = make_harness()
    root, server = harness.root, harness.server
    lock = root / f"bin/.accelerator-lock-{host_platform}"
    lock.mkdir()
    (lock / "pid").write_text("999999\n")  # dead PID, unreadable-to-rm below
    with _restricted(lock, 0o555):  # rm of pid fails; the pid persists
        result = _run_bootstrap(
            root,
            server,
            downloader,
            extra_env={"ACCELERATOR_LOCK_MAX_WAIT": "3"},
            timeout=15,
        )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "timed out acquiring the launcher cache lock" in output, output


def test_empty_pid_lock_advances_budget(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    lock = root / f"bin/.accelerator-lock-{host_platform}"
    lock.mkdir()
    (lock / "pid").write_text("")  # competitor made the dir, no pid written yet
    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={"ACCELERATOR_LOCK_MAX_WAIT": "3"},
        timeout=15,
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "timed out acquiring the launcher cache lock" in output, output
    assert "cannot create the launcher cache lock" not in output, output


def test_symlink_lock_path_fails_fast(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    target = root / "foreign-lock-target"
    target.mkdir()
    (target / "pid").write_text("999999\n")  # a dead owner: reclaim would rm this
    (root / f"bin/.accelerator-lock-{host_platform}").symlink_to(target)
    result = _run_bootstrap(
        root,
        server,
        downloader,
        extra_env={"ACCELERATOR_LOCK_MAX_WAIT": "3"},
        timeout=15,
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "cannot create the launcher cache lock" in output, output
    assert "timed out" not in output, output
    assert (target / "pid").read_text() == "999999\n", "reclaim followed symlink"


def test_leading_zero_ceiling_is_decimal_not_octal(
    make_harness: Callable[..., Harness],
    downloader: Path,
    host_platform: str,
) -> None:
    harness = make_harness()
    root, server = harness.root, harness.server
    lock = root / f"bin/.accelerator-lock-{host_platform}"
    lock.mkdir()
    (lock / "pid").write_text("")  # empty pid → else arm, bounded by the ceiling
    result = _run_bootstrap(
        root,
        server,
        downloader,
        # 08: invalid octal; base-10 makes it a ceiling of 8, not an error
        extra_env={"ACCELERATOR_LOCK_MAX_WAIT": "08"},
        timeout=15,
    )
    output = result.stdout + result.stderr
    assert result.returncode != 0, output
    assert "timed out acquiring the launcher cache lock" in output, output
```

Why each is deterministic and sub-second, and what each pins:

| Test | Precondition | Discriminates | Root guard |
| --- | --- | --- | --- |
| `test_uncreatable_lock_dir_fails_fast` | file at lock path | fail-fast vs timeout (AC1) | none |
| `test_symlink_lock_path_fails_fast` | symlink at lock path | `-L` guard, no-follow | none |
| `test_unremovable_dead_owner_lock_terminates_within_budget` | dead pid, dir `0o555` | bounded vs unbounded (AC2) | `_require_unprivileged` |
| `test_empty_pid_lock_advances_budget` | empty pid file | `else` arm intact (AC4) | none |
| `test_leading_zero_ceiling_is_decimal_not_octal` | empty pid, ceiling `08` | base-10 not octal | none |

The dead-owner test is the tripwire: against the current `continue`, `rm -f`
fails on the `0o555` directory so the pid persists, the dead-owner arm fires
every iteration and busy-spins, and the `timeout=15` (well above the injected
`3 × 0.1 s` budget, far below the default ~30 s) trips `pytest.fail`. After the
fix the failed `rm` drops the compound `elif`, control reaches the `else`, and
the loop exits in ~0.4 s with the timeout message. The 15 s value matches the
`test_unverifiable_launcher_in_readonly_cache_fails_fast` precedent, leaving
headroom for a correctly-bounded run under parallel CI load rather than a tight
`timeout=5` a loaded runner could trip on a passing fix. The other four tests
also pass `timeout=15` as a hang net, so a regression of the ceiling comparison
itself reds cleanly instead of hanging the suite. Root would remove the pid and
void the dead-owner case, so it hard-fails rather than skips.

The symlink test pins the `-L` guard — a regression to a bare
`[[ -e && ! -d ]]` would let a symlink-to-directory through, and reclaim would
then follow the link and delete the dead-owner pid planted inside the target; the
test asserts that pid survives, so it reds on such a regression. The leading-zero
test pins the base-10 normalisation, since `08` reaches `[[ -gt ]]` as an octal
error and hangs without it. Neither plants a `chmod`, so neither needs a root
guard. Two branches cannot be pinned by a fast behavioural test and are covered
by inspection plus ShellCheck instead: the absent-path fall-through (the next
`mkdir` would win the race, so it is only incidentally exercised by the
concurrent-cold-cache test), and the non-numeric ceiling fallback (any rejected
value falls back to the 300-iteration ~30 s default and a fail-fast setup never
reaches the `[[ -gt ]]` operand, so a rejected injection payload never executes
regardless of the guard — making such a test vacuous rather than a tripwire).

### Success Criteria:

#### Automated Verification:

- [x] All five new tests red against current `main` and pass after the fix
      (`uv run pytest tests/integration/entrypoint/test_accelerator_entrypoint.py -k "lock or leading_zero"`).
      Against current `main` neither the classifier nor the
      `ACCELERATOR_LOCK_MAX_WAIT` seam exists, so the injected `=3`/`=08`
      ceilings are inert: every new test either burns the literal 300×0.1 s≈30 s
      budget or busy-spins, exceeds its `timeout=15` net, and reds as a
      hang→`pytest.fail`. After the fix each terminates in well under a second
      with its asserted message. (In the TDD-ordered intermediate — seam landed,
      classifier not yet — the fail-fast and symlink tests instead red
      sub-second on the wrong lock-timeout message; the empty-pid test is green
      once the seam is present, guarding that the classifier left the `else` arm
      intact.)
- [x] Existing lock guards stay green (AC3):
      `uv run pytest tests/integration/entrypoint/test_accelerator_entrypoint.py -k "stale_lock or slow_downloader or readonly_cache_fails_fast"`
- [x] The `macos-latest` integration leg exercises the new tests under bash 3.2
      automatically — the harness pins `/bin/bash` (`installation.py:43`, bash
      3.2 on macOS) and `test-integration` runs that leg — so the 3.2 floor is a
      CI gate, not only a manual replay.
- [x] Shell lint clean (AC5): `mise run scripts:check`
      (`scripts/lint-bashisms.sh`, shfmt, ShellCheck)
- [x] Read-only CI mirror passes: `mise run check`
- [x] Full local gate exits 0 end-to-end (AC6): `mise run`
      (one unrelated pre-existing flake in the parallel integrations shell suite;
      passes clean on a standalone `mise run test:integration:integrations`).

#### Manual Verification:

- [x] Optional `/bin/bash` spot-check on macOS as a supplementary backstop (the
      bashisms linter is documented KNOWN-INCOMPLETE); the automated bash-3.2
      gate is the `macos-latest` integration leg above. The fix uses only
      3.2-safe constructs — `[[ -d/-e/-L/-n ]]`, `{ …; }` grouping, `case`,
      `kill -0`, `$(( ))`, and `${VAR:-default}` — no bash-4 construct.
- [x] The dead-owner test terminates in well under a second on a green run
      (not near the `timeout=15` ceiling), confirming the bound, not the
      tripwire, is what ends it.

---

## Testing Strategy

### Unit Tests:

Shell has no unit harness here; behaviour is exercised end-to-end through the
bootstrap subprocess. The five integration tests above are the unit-equivalent,
each isolating one arm of the post-fix `if/elif/else`.

### Integration Tests:

The five new tests plus the three retained guards
(`test_stale_lock_is_reclaimed`,
`test_concurrent_cold_cache_slow_downloader_all_succeed`,
`test_unverifiable_launcher_in_readonly_cache_fails_fast`) cover every arm:
fail-fast (file and symlink), base-10 ceiling, live-owner reset, dead-owner
reclaim, dead-owner bounded, empty-pid race, and the 0186 gate.

### Manual Testing Steps:

1. On macOS, run the five new tests under the system bash 3.2 to confirm no
   4.x construct slipped in.
2. Temporarily revert the reclaim bound (restore the bare `continue`) and
   confirm the dead-owner test flips to a `pytest.fail` hang — proving the test
   is a genuine tripwire, not a tautology.

## Performance Considerations

⏱️ None on the warm or cold happy path: `max_wait` is read once per
`acquire_lock` call, and the new `[[ ! -d ]]` test runs only after a `mkdir`
failure. The fix removes an unbounded busy spin, so worst-case behaviour
strictly improves.

## Migration Notes

None. `ACCELERATOR_LOCK_MAX_WAIT` is unset in production, so the default `300`
ceiling and existing timing are unchanged; when set, it is validated to an
all-digit value, normalised to base 10 (so a leading-zero value is read
decimally, never as octal), and otherwise falls back to `300`, with the override
active in all environments (not gated behind `ACCELERATOR_TEST_MODE`). No on-disk
lock format change — the `owner.<nonce>` / pid contract is untouched.

## References

- Work item: `meta/work/0190-acquire-lock-cannot-classify-mkdir-failures.md`
- Research: `meta/research/codebase/2026-08-21-0190-acquire-lock-mkdir-classification.md`
- Defect site: `bin/accelerator:317-345`
- 0186 gate and guard: `bin/accelerator:380-396`,
  `tests/integration/entrypoint/test_accelerator_entrypoint.py:1290-1318`
- Harness hang→fail: `tests/integration/support/installation.py:333-334`
- Precedents: `scripts/atomic-common.sh:104-109`,
  `skills/integrations/jira/scripts/jira-common.sh:146-151`
