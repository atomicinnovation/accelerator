---
type: "codebase-research"
id: "2026-08-21-0190-acquire-lock-mkdir-classification"
title: "Research: acquire_lock mkdir misclassification and unbounded reclaim (0190)"
date: "2026-08-21T14:47:24+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0190"
parent: "work-item:0190"
topic: "acquire_lock cannot classify an unusable lock directory and can spin unbounded on reclaim"
tags: ["research", "codebase", "shell", "bootstrap", "bash-3.2", "locking", "acquire_lock"]
revision: "8c5eebef10dd63b532d9113e13b37b572456147e"
repository: "accelerator"
last_updated: "2026-08-21T14:47:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: acquire_lock mkdir misclassification and unbounded reclaim (0190)

**Date**: 2026-08-21T14:47:24+00:00
**Author**: Toby Clemson
**Git Commit**: 8c5eebef10dd63b532d9113e13b37b572456147e
**Branch**: working copy `8c5eebef` (no 0190 bookmark yet; nearest named bookmark on ancestry is `0213-refine-conflict-flow`, parent commit `c25fbd1b` "Add the 0190 review and tighten its lock-failure criteria")
**Repository**: accelerator

## Research Question

How does `acquire_lock` in `bin/accelerator` currently classify a failed `mkdir`, where exactly are the two defects in work item 0190 (misclassification of an unusable lock directory, and an unbounded dead-owner reclaim arm), and what does the fix need — the live control flow, the test harness that must exercise it, the bash-3.2 and env-knob conventions to mirror, and the historical thread through 0186/0164?

## Summary

Both defects live in one 29-line loop, `acquire_lock` at `bin/accelerator:317-345`. The loop `mkdir`s a lock directory and, on any failure, unconditionally assumes contention — it reads a pid file and branches three ways on process liveness. It has no notion of an `mkdir` that can never succeed (unwritable parent, foreign lock directory), and its dead-owner reclaim arm `continue`s without advancing the budget, so a `rmdir` that keeps failing spins forever.

**The work item's two-part fix is well-founded and the code confirms it.** Add a classification branch — after a failed `mkdir`, `[[ -d "${lock_dir}" ]]` distinguishes `EEXIST` (a real competitor) from an absent directory (unwritable parent → fail fast); and bound the reclaim arm by gating its `continue` on `rmdir` succeeding, so a failed `rmdir` advances the shared budget instead of looping free. A third change — an env-injectable iteration ceiling — makes the bounded arm testable sub-second.

One correction to the work item's framing surfaced. The `sleep 0.1` it attributes to the `else` arm is actually a **shared loop tail** at `bin/accelerator:343`, run by the live-owner and empty-pid arms but skipped by the reclaim arm's `continue`. That `continue` skipping the shared sleep-and-budget tail is precisely the unbounded-spin mechanism, which makes the fix cleaner than the work item states: on `rmdir` failure, drop the `continue` and let control fall to the shared tail after incrementing the budget.

The repo already contains the exact structural precedent the fix needs — `atomic-common.sh:104-109` is a fail-fast parent-writability pre-check guarding the identical "burn the whole timeout on an `mkdir` that can never recover" bug class, with a comment saying so. The env-injectable-ceiling template is the Jira lock at `jira-common.sh:146-151`. The test harness converts a hang into a red test via `pytest.fail`, and the permission-rule helpers (`_require_unprivileged`, `_restricted`) already exist.

## Detailed Findings

### The defect: `acquire_lock` control flow (`bin/accelerator:317-345`)

The loop, verbatim:

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

The `mkdir` is line 320. On success it writes `$$` to the pid file, sets `lock_held=1`, installs the cleanup trap, and returns. On failure it reads the owning pid (line 326) and takes one of three arms:

| Arm | Lines | Condition | Action | Budget effect |
| --- | --- | --- | --- | --- |
| Live owner | 327-332 | pid present and `kill -0` succeeds | `waited=0` (reset), then shared `sleep` | Resets — never advances |
| Dead-owner reclaim | 333-336 | pid present, `kill -0` fails | `rm -f` pid, `rmdir` dir, `continue` | None — skips sleep and budget |
| `else` (empty/absent/unreadable pid) | 337-342 | `owner` empty | `waited++`, `fail` if `> 300`, then shared `sleep` | Advances — the only bounded arm |

The `sleep 0.1` at line 343 is **outside** the `if/elif/else` — a shared tail run by the live-owner and `else` arms, but not the reclaim arm, which `continue`s past it.

**Defect 1 — misclassification (the bounded-but-wrong arm).** A failed `mkdir` on an *unwritable parent* leaves no directory, so `cat` yields `""`, so control takes the `else` arm and advances the budget toward a lock-timeout `fail`. The diagnostic is wrong (it reports contention on a directory that can never be created), and it arrives only after the full 300 × `sleep 0.1` ≈ 30 s budget.

**Defect 2 — unbounded reclaim (the more severe arm).** When the pid file names a dead process on a lock directory that *cannot be removed* (foreign owner, or a writable cache dir whose lock subdir is not writable), `rm -f` and `rmdir` both fail silently, `continue` fires, and the loop re-enters with `waited` untouched and no sleep. It spins **unbounded, with no timeout at all** — a hang, not a slow wrong answer.

**Correction to the work item's `300` framing.** `waited` advances only in the `else` arm (line 338); the live-owner arm resets it and the reclaim arm never touches it. So `300` bounds *consecutive empty-pid observations*, not total iterations — the wait for a lock directory that exists but whose pid file never appears or reads. The `300` at line 339 and the `0.1` at line 343 are bare literals; nothing in the loop is env-injectable.

**The pid file** is `${lock_dir}/pid` (`lock_dir="${cache_dir}/.accelerator-lock-${platform}"`, line 307). Written after a successful `mkdir` (line 321), so a competitor can momentarily see the directory with no pid file yet — the genuine-race window the `else` arm exists to cover. Read via `owner=$(cat ... 2>/dev/null)` (line 326), which collapses absent, empty, and unreadable pid files into the same empty string, all routed to the `else` arm.

**`fail`** (`bin/accelerator:48-51`) prints `accelerator: <msg>` to stderr and `exit "${abort_status}"` (1 normally, 0 under `--fail-safe`). **`release_lock`** (lines 310-315) `rm -f`s the pid and `rmdir`s the directory; the trap installed at line 323 fires it on EXIT/INT/TERM. The **only caller** is line 387.

### The 0186 probe gate does not cover either 0190 case fully

The gate above the single `acquire_lock` call site is `require_exec_capable_cache` at `bin/accelerator:386`, inside the cold `else` branch (lines 380-396). Its comment (lines 381-385) names the exact hazard:

```bash
# Staging was skipped (the staged shim already matched) but verification
# failed. Without this, an unwritable cache dir reaches acquire_lock, whose
# mkdir can never succeed and whose loop treats a missing pid file as an
# imminent competitor — burning its whole timeout budget and reporting a
# lock timeout instead of the real cause.
require_exec_capable_cache
acquire_lock
```

`require_exec_capable_cache` (lines 258-267) calls `probe_exec_capable` (lines 180-191), which writes/chmods/execs/removes a probe file **in `cache_dir`** and maps the result to `fail_no_cache_dir`.

- ✅ **Unwritable `cache_dir` — prevented.** The probe fails, `acquire_lock` is never reached. This is what the gate is for.
- ⚠️ **Foreign / pre-existing `lock_dir` — not prevented.** The probe writes a *file* into `cache_dir`, which succeeds even when the `.accelerator-lock-*` *subdirectory* already exists and is foreign. Control reaches `acquire_lock`, `mkdir` fails on `EEXIST`, and the loop engages. This is the only realistic residual reason `mkdir` still fails after the gate — precisely the case the gate cannot catch, and the entry point for Defect 2's unbounded spin.

The work item retains this gate as defence-in-depth; it does not remove it. The gate's regression guard is `test_unverifiable_launcher_in_readonly_cache_fails_fast`.

### The fix shape, reconciled against the live structure

Three changes, all bash-3.2-safe (`[[ -d ]]`, `[[ -n ]]`, `kill -0`, `$(( ))` are all permitted — confirmed against the linter below, and all four idioms already appear in this file):

1. **Classification branch (new fail-fast).** After the failed `mkdir`, before reading the pid file, test `[[ -d "${lock_dir}" ]]`. Absent directory → the parent is unwritable and `mkdir` can never succeed → `fail` immediately naming the lock path and a permission-or-I/O cause. Directory present → `EEXIST` → fall through to the existing wait logic. `mkdir` exposes no shell-portable errno, so directory-presence is the only portable `EEXIST` discriminator.

2. **Bound the reclaim arm.** Gate the `continue` on `rmdir` succeeding. On `rmdir` failure, advance the budget (`waited=$((waited + 1))` + ceiling check) and fall to the shared `sleep 0.1` tail rather than `continue`-ing. Because the `continue` is exactly what skips the shared budget-and-sleep tail, the minimal fix is: on `rmdir` failure, do not `continue`. The arm then shares the `else` arm's cap and terminates with the existing lock-timeout `fail` — which is why the work item specifies "terminates within budget" for this arm, not a new fail-fast path.

3. **Env-injectable ceiling.** Replace the literal `300` with `"${ACCELERATOR_...:-300}"` so a test can inject a low value and exercise the bounded arm sub-second. Default stays 300.

❓ **Open design point for the plan.** Change 2 needs the `rmdir`-failure path to increment `waited` *and* run the ceiling check, then reach the shared sleep. The current `if/elif/else` puts the increment+check only in the `else` body. The implementer must either restructure so the reclaim arm's failure path reaches that logic, or duplicate the two lines into the elif. This is the one place the fix is not a pure insertion; worth pinning in the plan to keep the `else` arm (the genuine-race window) intact.

### Post-fix arm order (from the work item, mapped to line regions)

`mkdir` succeeds → hold (320-324); `mkdir` fails + directory absent → **fail fast (new)**; directory + live owner → reset budget (327-332); directory + dead owner + `rmdir` ok → reclaim and retry (333-336); directory + dead owner + `rmdir` fails → **advance budget (new)**; directory + empty/unreadable pid (`else`) → advance budget (337-342).

### Test harness: how the new tests attach (`tests/integration/entrypoint/test_accelerator_entrypoint.py` + `tests/integration/support/installation.py`)

The subprocess funnel is `run_bootstrap` (`installation.py:275-334`), imported and aliased `_run_bootstrap` (test file lines 41, 53). Its `timeout=` (default `None`) threads into `subprocess.run(..., timeout=timeout)` (line 330); on expiry it does **not** raise to the caller but converts the hang into a hard failure (lines 333-334):

```python
except subprocess.TimeoutExpired:
    pytest.fail(f"the bootstrap did not terminate: {entry}")
```

So an unbounded regression reds the suite rather than hanging it — the tripwire AC2 relies on. Output is `capture_output=True, text=True`; every test concatenates `result.stdout + result.stderr` itself and asserts on `.returncode` plus a substring. Env is injected via the `extra_env` dict, merged over a fresh (non-`os.environ`) base at `installation.py:313-314` — the same mechanism a new low-ceiling env var would use: `extra_env={"<CEILING_VAR>": "…"}`.

The three tests the work item names:

- **`test_stale_lock_is_reclaimed`** (test lines 311-323) — manufactures a pre-existing `bin/.accelerator-lock-<platform>` with `pid` = `999999\n` (a dead PID), asserts exit 0. Guards the reclaim arm still reclaims. The default cache dir is `bin/` (no `ACCELERATOR_CACHE_DIR`), so the lock path matches `bin/accelerator:307`.
- **`test_concurrent_cold_cache_slow_downloader_all_succeed`** (test lines 682-700) — 6 concurrent cold-cache bootstraps with `DL_SLEEP=1`; the winner holds the lock a full second while five wait. Asserts all 6 exit 0 and exactly 2 download-log lines (bin + `.minisig`), proving the live-owner arm keeps resetting waiters' budgets rather than a waiter aborting and re-fetching.
- **`test_unverifiable_launcher_in_readonly_cache_fails_fast`** (test lines 1290-1318) — the 0186 gate guard: warm the cache, poison the launcher (`b"poisoned"`), `_restricted(cache, 0o555)`, `timeout=15`, assert non-zero exit and `"no writable, exec-capable cache directory"`.

**Permission-rule helpers to reuse** (0186's discipline, which the 0190 criteria now adopt):

- `_require_unprivileged()` (test lines 58-68) — a bare `assert os.getuid() != 0`, hard-fail-not-skip under root. Called at the top of every chmod-dependent test (lines 273, 297, 1085, 1178, 1251, 1272, 1300, 1325).
- `_restricted(path, mode)` (test lines 1115-1137) — chmods, then probes with `os.access(..., W_OK/X_OK)` and asserts the cleared bits are genuinely unavailable (catching advisory-permission filesystems), restoring `0o755` in `finally`.

There is **no** custom pytest marker or skip for these — the convention is the hard `assert`. Faking a live owner is done with a real concurrent process, never a hand-written pid; a dead owner is the literal `999999`.

### Conventions to mirror

**bash 3.2 floor.** `scripts/lint-bashisms.sh` is an awk denylist (lines 42-62) banning associative arrays (`-A`), namerefs (`-n`), escaped braces in expansion defaults, `mapfile`/`readarray`, case-modification (`${var^^}`/`${var,,}`), `&>>`, `|&`, and negative subscripts. `bin/accelerator` is extensionless so the `*.sh` glob misses it — it is appended explicitly at line 33, so it *is* scanned. None of the fix's constructs are banned. The linter is documented KNOWN-INCOMPLETE; the manual bash-3.2 replay is the behavioural backstop.

**Env-injectable ceiling.** The template is the Jira lock (`jira-common.sh:146-151`):

```bash
local timeout_secs=60
local sleep_secs=0.1
if [[ "${ACCELERATOR_TEST_MODE:-}" == "1" ]]; then
  timeout_secs="${JIRA_LOCK_TIMEOUT_SECS:-$timeout_secs}"
  sleep_secs="${JIRA_LOCK_SLEEP_SECS:-$sleep_secs}"
fi
```

`bin/accelerator` uses `ACCELERATOR_`-prefixed env overrides exclusively (`ACCELERATOR_CACHE_DIR`, `ACCELERATOR_UNAME_M/S`, `ACCELERATOR_RELEASE_BASE_URL`), always `"${VAR:-default}"`. The existing in-file iteration ceiling `max_hops=16` (line 45) is a hard literal for comparison. Mirror the prefix for a new ceiling (e.g. `ACCELERATOR_LOCK_MAX_WAIT`); the work item does not require gating behind `ACCELERATOR_TEST_MODE`, so a plain always-overridable `local` default is the simpler match unless production immutability is wanted.

**Fail-fast message.** Use the in-file `fail` (idiom A), not the library `log_die` — `bin/accelerator` sources nothing (root-of-trust entry point). Match the sibling lock-timeout message shape at line 340, `"<cause>: ${lock_dir}"`, and the writability wording of `fail_no_cache_dir` (lines 208-211). Idiomatic form: `fail "cannot acquire the launcher cache lock (parent not writable): ${lock_dir}"`.

### The structural precedent already in the repo

`atomic-common.sh:104-109` is the fail-fast parent-writability pre-check `acquire_lock` lacks, guarding the identical bug class, with a comment (lines 100-103) that reads as a description of Defect 1: *"Without this check the spin-loop would burn the full timeout on an error mkdir can never recover from (e.g., chmod 555 on the parent)."* Its staleness classification (lines 181-215) gates reclaim on `kill -0` and linearises the reclaim via an atomic `mv` of a sentinel before `rm -rf`, avoiding the race a bare `rm -rf` on a live holder would cause. The Jira lock (`jira-common.sh:186-212`) additionally records `holder.start` (process start time) to detect PID reuse — relevant to the live-owner arm, which today resets the budget forever if a foreign process happens to sit at the reused pid, though 0190 scopes that out.

## Code References

- `bin/accelerator:317-345` — `acquire_lock`; both defects live here.
- `bin/accelerator:320` — the `mkdir` whose failure is misclassified.
- `bin/accelerator:333-336` — the dead-owner reclaim arm; `continue` skips budget + sleep (Defect 2).
- `bin/accelerator:337-342` — the `else` arm; the only budget-advancing arm; literal `300` at 339.
- `bin/accelerator:343` — the shared `sleep 0.1` tail (skipped only by the reclaim arm).
- `bin/accelerator:48-51` — `fail`; the fail-fast idiom to mirror.
- `bin/accelerator:310-315`, `:323` — `release_lock` and the EXIT/INT/TERM trap.
- `bin/accelerator:380-396` — the cold `else` branch; the sole `acquire_lock` call site (387) behind the 0186 gate (386).
- `bin/accelerator:180-191`, `:258-267` — `probe_exec_capable` / `require_exec_capable_cache`; probes `cache_dir`, not `lock_dir`.
- `tests/integration/support/installation.py:275-334` — `run_bootstrap`; `timeout=` → `pytest.fail` on hang (333-334); `extra_env` merge (313-314).
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:311-323` — `test_stale_lock_is_reclaimed`.
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:682-700` — `test_concurrent_cold_cache_slow_downloader_all_succeed`.
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:1290-1318` — `test_unverifiable_launcher_in_readonly_cache_fails_fast`.
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:58-68`, `:1115-1137` — `_require_unprivileged`, `_restricted`.
- `scripts/atomic-common.sh:104-109`, `:181-215` — the fail-fast writability pre-check and PID-liveness reclaim precedent.
- `skills/integrations/jira/scripts/jira-common.sh:146-151`, `:186-212` — env-injectable lock-ceiling template and PID-reuse-aware classifier.
- `scripts/lint-bashisms.sh:33`, `:48-55` — `bin/accelerator` explicitly scanned; the bash-4 denylist.

## Architecture Insights

- **One choke point, three inline classifications.** `acquire_lock` inlines live/dead/absent-owner handling rather than delegating to helpers like `atomic-common.sh`. The fix keeps that shape; it adds one branch and removes one `continue`, not a redesign.
- **`continue` as the bug.** Defect 2 is not a missing bound so much as a `continue` that bypasses the loop's single shared budget-and-sleep tail. Framing the fix as "don't `continue` when `rmdir` fails" is both minimal and self-documenting.
- **The gate probes the wrong granularity.** The 0186 gate probes `cache_dir` writability but the lock is a *subdirectory*; a foreign lock subdir passes the probe. Any durable fix has to live inside `acquire_lock`, not in a pre-check on the parent — which is why 0190 exists despite 0186.
- **Collapsed pid-read states.** Absent, empty, and unreadable pid files are indistinguishable after `cat ... 2>/dev/null`. The `else` arm deliberately covers all three as "advance the budget", which is correct for the genuine-race window and must survive the fix (the work item's preserved-`else`-arm criterion).
- **Root-of-trust means no shared library.** `bin/accelerator` sources nothing, so the fix reuses no `log-common`/`atomic-common` helper — it inlines `fail`. The precedents inform the shape but cannot be called.

## Historical Context

- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — removed the warm-path exec probe (warm median 125.35 → 29.92 ms, 0186 Validation Results). Added the cold-branch gate that masks the *reachable-today* instance of Defect 1 and recorded Defect 2 as an explicit follow-up it did not fix. Its Acceptance Criteria preamble (0186:135-146) is the permission-test rule the 0190 criteria now inherit: assert `id -u ≠ 0`, hard-fail under root, record a privilege check for any excluded lane. The `TIMEOUT after 31 iters, 3s` figure is 0190's own reduced-ceiling restatement, not a string in 0186; 0186 supplies the "~30 s spin" framing and PR #41 records the 27 ms fail-fast.
- `meta/prs/41-description.md:84` — "a tampered cached launcher in an unwritable cache dir fails in 27 ms with the cache-dir diagnostic, not after the ~30 s lock budget."
- `meta/work/0164-launcher-and-git-style-dispatch.md` (+ its 2026-07-03 plan/research) — introduced the `bin/accelerator` bootstrap and the mkdir lock.
- `meta/work/0136-migrate-shell-scripts-to-rust-cli.md` — the parent epic.
- `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md` and `0205-close-the-warm-dispatch-measurement-method.md` — adjacent warm-path follow-ups; 0189's research is the nearest existing analysis of the bootstrap cache-root probe path (no dedicated `acquire_lock` research existed before this document).
- `meta/work/0191-batch-the-two-shim-hashes-into-one-invocation.md` — edits a different region of `bin/accelerator` (the shim-staging block); no merge coupling with 0190, now cross-referenced in the work item.
- No locking-specific ADR exists; the launcher/dispatch ADRs are 0046, 0053, 0054, 0059, 0060.

## Related Research

- `meta/reviews/work/0190-acquire-lock-cannot-classify-mkdir-failures-review-1.md` — two-pass work-item review. Pass 1 REVISE raised three testability majors (AC1 diagnostic not pinned to a substring; AC1/AC2 omitting 0186's root-guard; AC2's ~30 s budget guard slow and flake-prone with the fast seam left optional). Pass 2 COMMENT records all resolved — the env-injectable ceiling became a mandatory Requirement, AC1 pins a verbatim cause substring, the criteria adopt the root-guard, and AC2 asserts the terminating outcome (non-zero + lock-timeout message), not just termination. The item is marked ready for implementation.
- `meta/research/codebase/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md`, `meta/research/codebase/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md` — adjacent bootstrap research.

## Open Questions

- ❓ **Reclaim-arm restructure (implementation).** Where does the `rmdir`-failure path land so it increments `waited`, runs the ceiling check, and reaches the shared `sleep` while leaving the `else` (genuine-race) arm intact? A pure insertion won't do it given the `if/elif/else` + shared-tail shape; the plan should pin the exact restructure.
- ❓ **Ceiling env var: gated or always-on?** The Jira template gates the override behind `ACCELERATOR_TEST_MODE=1` for production immutability; the work item only requires injectability. Decide whether a plain always-overridable `"${ACCELERATOR_LOCK_MAX_WAIT:-300}"` is acceptable or the fix should gate it. (Naming also unresolved — no existing lock env var in this file to match.)
- ❓ **Live-owner-resets-forever (scoped out, worth recording).** A foreign process at a reused pid keeps the live-owner arm resetting the budget indefinitely. `jira-common.sh` solves the analogue with a `holder.start` process-start-time check. 0190 scopes this out; confirm it stays a separate concern rather than creeping in.
- ❓ **AC2 harness `timeout=` value.** It must sit above the injected low ceiling's wall time but far below the default ~30 s, so a correctly-bounded loop exits cleanly while an unbounded regression trips `pytest.fail`. The exact injected ceiling and `timeout=` pair is a plan detail, not yet chosen.
