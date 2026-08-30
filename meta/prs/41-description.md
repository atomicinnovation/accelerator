---
type: "pr-description"
id: "41"
title: "Probe the cache directory only on the bootstrap's cold path"
date: "2026-08-03T12:29:40+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0186"
parent: "work-item:0186"
relates_to: ["work-item:0169", "work-item:0182", "work-item:0189", "work-item:0190", "work-item:0191"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/41"
pr_number: 41
tags: ["shell", "performance", "bootstrap", "bash-3.2", "testing"]
revision: "e503155c7bdd7c2379448331bb0f81e1e251a219"
repository: "accelerator"
last_updated: "2026-08-03T12:29:40+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Probe the cache directory only on the bootstrap's cold path

## Summary

`bin/accelerator` wrote, `chmod +x`'ed, exec'ed and removed a probe file on **every** invocation — a capability check the warm path never needed, since a warm call proves the same capability for real by running the staged verify shim and the launcher out of that directory. On macOS the exec of a freshly written file pays a first-exec check that dominated the whole invocation. Splitting `probe_dir` into an always-run `ensure_dir` and a cold-path-only `probe_exec_capable` takes the warm median from **125.35 ms to 29.92 ms** on darwin-arm64. Every SessionStart hook and every skill's `!`-preprocessor site pays that cost, so it is the most-executed path in the plugin.

## Changes

- **`bin/accelerator` — the split.** `ensure_dir` keeps the `mkdir -p` (guarded by `[[ -d ]]`, so a warm call does not even fork `mkdir`) and stays reachable on every invocation via `resolve_cache_dir`. `probe_exec_capable` keeps the write-`chmod`-exec-`rm` and now returns **1** for a write failure and **2** for an exec failure, so the caller can report the cause it actually detected. Every exit path removes the probe file.
- **Two gates, not one.** `require_exec_capable_cache` sits behind a `probed` flag and is called from (a) the first statement of the shim-staging `if` **body**, which is the first required write into the cache directory on every path, and (b) the top of the cold branch, before `acquire_lock`. Both sit below the dev-launcher override, so the contributor path stops paying for a probe it never needed. A cold run reaching both probes exactly once.
- **One diagnostic, three causes.** `fail_no_cache_dir` gives `no writable, exec-capable cache directory` a single definition; each site supplies what it detected (`could not be created` / `is not writable` / `rejected an executable file — possibly a noexec mount`). It also fixes a pre-existing wart: the message named `${plugin_root}/bin` even when an `ACCELERATOR_CACHE_DIR` override was the thing that failed.
- **New supported configuration** — a cache directory populated once may afterwards be **read-only for warm bootstrap invocations**. Documented in `docs/internals.md` and `CHANGELOG.md` together with the limit that matters: dispatching a subcommand to a separate binary makes the *launcher* probe the same directory, and that probe writes.
- **Test harness gains a per-call `xtrace` seam** (`run_bootstrap(..., xtrace=True)`), defaulting `PS4` alongside it, so probe absence can be asserted by function name rather than by residue.
- **Eight new entrypoint cases plus three retrofits**, and a hard-failing `_require_unprivileged()` guard.
- **Three follow-ups raised** — 0189, 0190, 0191 — and 0169's hand-off note re-confirmed against the measured figures.

Production surface is small: `bin/accelerator` (+82/-…), `docs/internals.md`, `CHANGELOG.md`. Tests are +279 across three files. The remaining ~6,000 lines are `meta/` artefacts (research, plan, plan review, work items, validation report).

## Context

Work item [`0186`](../work/0186-remove-exec-probe-from-bootstrap-warm-path.md), under epic 0136, unblocked by 0182 and blocking [`0169`](../work/0169-vcs-subdomain-and-hooks-migration.md). Plan and its review are in `meta/plans/` and `meta/reviews/plans/`; the validation report is `meta/validations/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path-validation.md`.

### Why two call sites

A single gate in the staging body is not enough. In the residual case — launcher present and executable, signature present, staged shim present and matching (so staging is skipped), *verification fails*, directory unwritable — control reaches `acquire_lock`, whose `mkdir` can never succeed and whose loop treats a missing pid file as an imminent competitor. It then burns its full 300 × 0.1 s budget and reports a **lock timeout** instead of the real cause. That case fails instantly today, so the second gate prevents a regression rather than merely preserving a nicer message. It is measured at 27 ms with the gate in place.

Equally, a gate placed only in the cold branch would miss the `chmod -x` scenarios: `cp` fails first with `could not stage the verify shim into …`, losing the cache-dir substring entirely.

### Measured result

50 interleaved samples per variant in one Python process, order alternated, on darwin-arm64 (Apple M4 Max, macOS 26.3 build 25D125). Both variants shared one pre-cached signed release launcher, confirmed by inode across the run.

| Variant | min | median | p90 | median − harness floor |
| --- | --- | --- | --- | --- |
| before | 119.02 | **125.35** | 234.15 | 123.75 |
| after | 27.18 | **29.92** | 32.44 | 28.32 |

All ms. Gate `after ≤ 0.5 × before` passes at a ratio of **0.239** — materially better than the ~41 ms the plan expected. Instrument floors: 1.60 ms (`/usr/bin/true`) and 6.10 ms (a trivial bash script). The remaining ~30 ms is accounted for term by term in the work item's Validation Results, with ~10% unexplained (under the 25% threshold that would have triggered a per-line attribution).

### Two figures in circulation were wrong, and are corrected here

- **The retained double-hash residual is backend-dependent.** The ~11.7 ms per `sha256_file` call quoted in 0186 and 0169's hand-off note describes the **Perl `shasum` fallback**. On a host where `command -v sha256sum` resolves to Apple's `/sbin/sha256sum` — as it does on the measuring host — a call costs **~3.5 ms**, so the two-hash residual is **~7 ms or ~24 ms depending on the host**. 0169 is handed the range and the backend, not a point estimate. Corrected in 0186's Dependencies, Assumptions and Validation Results, not only in 0169's note.
- **The probe's first-exec penalty is re-derived rather than inherited.** The Context table's methodology was unrecorded and its "re-exec of a pre-existing probe file | 10.6 ms" row is seven times this harness's fork+exec floor. Measured directly: 1.41 ms bare fork+exec, 3.72 ms to re-exec a probe file left in place, 107.15 ms to write+`chmod`+exec+`rm` in `/tmp`, 131.97 ms in the repo's `bin/`. The host runs **no third-party EndpointSecurity or anti-malware agent** (only Apple's `xprotectd`, SIP and Gatekeeper on), so the penalty is stock macOS behaviour rather than a machine artefact — which means the ratio should transfer to other macOS hosts, and the extrapolation to the launcher-side probe (0189) holds.

## Testing

- [x] `mise run test:integration:entrypoint` — **54 passed**, including all eight new cases.
- [x] `mise run test:integration:skill-invocation` — **128 passed**. The `run_bootstrap` seam is shared with this suite; the `xtrace=False` default keeps it unaffected.
- [x] `uv run pytest tests/unit/tasks/test_bootstrap_coverage.py tests/unit/tasks/test_mise.py` — 33 passed, including the new name-pin assertion and the standing no-`build:cli:dev` guard.
- [x] `mise run check` — green end to end across all four components.
- [x] `mise run scripts:check` — shfmt, ShellCheck, bashisms and exec-bits clean over `bin/accelerator` (which `tasks/shared/sources.py:110` puts in shell-source scope despite being extensionless, and which is tab-indented because `.editorconfig`'s `[*.sh]` section does not match it).
- [x] `mise run build-system:check` — format, lint and types clean.
- [x] `mise run test:integration:config` — 58 passed; the corpus frontmatter validator accepts the amended items and the three new ones.
- [x] **Written test-first, with the red step recorded per case** — 5 failed / 3 passed before the change, exactly as predicted. Three of the eight are green before *and* after: they are preservation guards, which the record says explicitly rather than claiming a uniform red.
- [x] **Both gates confirmed by mutation after the change.** Deleting the staging gate reds `test_cold_path_keeps_the_noexec_diagnostic`, `test_warmed_then_non_executable_cache_keeps_the_diagnostic` and `test_readonly_root_without_override_is_a_named_error`; deleting the cold-branch gate reds only `test_unverifiable_launcher_in_readonly_cache_fails_fast`, via its timeout. That mutation is the only demonstration the cold-branch gate is guarded at all, since its case passes before the change too.
- [x] **Manual**: a tampered cached launcher in an unwritable cache dir fails in **27 ms** with the cache-dir diagnostic, not after the ~30 s lock budget. Cold trace shows the probe entered and the probe file executed as its own command word exactly once, with `+main:require_exec_capable_cache` appearing twice — so the idempotence flag is exercised, not merely present. Warm trace shows `ensure_dir` and `verify_launcher` but no `probe_exec_capable`.
- [ ] **The linux lane.** Not observed — this branch has not had a CI run. See below.

## Notes for Reviewers

**The one genuinely outstanding item is the ubuntu lane, and 0186 stays `in-progress` until it is seen.** The harness pins `BASH = "/bin/bash"`, which is 3.2.57 on darwin and **5.2** on `ubuntu-latest`, so the two trace cases exercise both interpreters on every CI run with no extra wiring — but that also means a trace-format divergence surfaces in CI rather than locally. `.github/workflows/main.yml` has no `container:` key and `test-integration` is a plain `runs-on: ${{ matrix.os }}` matrix, so both lanes run unprivileged and `_require_unprivileged` should not fire; no lane exclusion is expected. The work item's own Drafting Notes call this observation "a genuine closure condition, not a formality", which is why the status is held rather than moved to `complete`. Worth also capturing `command -v sha256sum` on both lanes while CI runs — it costs nothing and tells 0169 whether the Perl fallback is reachable in CI at all.

**One of the three diagnostic causes ships untested, deliberately.** No directory-permission combination can produce exec-without-write: clearing a directory's search bit blocks name resolution, so the probe fails at its *write* step and the exec branch is never evaluated. The exec **half** is covered by the positive control's assertion that the probe file is executed as its own command word; the `rejected an executable file` **cause clause** is not covered by any automated case. Closing it needs a genuine `mount -o noexec` filesystem, which is explicitly out of scope. If you would rather have a cheap seam than an untested branch, say so — but note 0189 will face the identical gap on the launcher side, so it may be better solved once, there.

**The trace matcher's `:/` anchor is load-bearing and easy to "simplify" wrongly.** `probe_exec_capable`'s own `probe=/…/.accelerator-probe-<pid>` **assignment** is traced too. The unanchored form `…:\S*\.accelerator-probe-\S*$` matches **two** lines and therefore passes on an implementation that never execs anything; the anchored form matches exactly one. Verified against a real trace. Relatedly, `PS4` is `'+${FUNCNAME[0]:-main}:'` rather than the acceptance criterion's literal `'+${FUNCNAME[0]}:'` — a bare `${FUNCNAME[0]}` is unbound at top level under the bootstrap's `set -u`, which on bash 3.2 emits `FUNCNAME[0]: unbound variable` per command and leaves PS4 literally unexpanded, and aborts a non-interactive bash 5 outright. And trace depth is not stable (`resolve_cache_dir` runs inside `$( )`, so its lines carry `++`), which is why every matcher allows one *or more* leading `+`.

**`_require_unprivileged` asserts where the neighbouring idiom skips, and that is intentional.** `tests/integration/hooks/test_launcher_link_refresh.py:275-293` uses `skipif`. Root bypasses both the write permission and the execute bit, so a skip would report green on a lane that verified nothing. The asymmetry worth knowing when reading these: cases asserting **success** under a restrictive mode go silently green under uid 0; cases asserting **failure** red noisily but for the wrong reason. One guard covers both.

**Two deviations from the plan are corrections *to* the plan, both recorded in place.** `_restricted`'s mode check: the plan's single `not os.access(path, os.W_OK)` assertion is wrong for the two `0o666` cases, which keep the owner write bit and clear only search — it fired on correctly-restricted directories. Implemented as a loop over both owner bits, checking each probe only when the mode clears the corresponding bit. And the staging comment's byte figure: the plan instructed correcting `475KB` to `465KB`, but 465,568 B is the **linux-x64** shim — the four vendored shims measure 486,672 / 496,896 / 426,400 / 465,568 bytes, so `475KB` was already right for darwin-arm64 and the comment now reads `~475KB`.

**One reordering to be aware of.** Because the staging gate sits inside the staging body, a cold invocation destined to fail with the cache-dir diagnostic now spends ~7 ms hashing first, where `resolve_cache_dir` previously failed before any hashing. No test depends on the old ordering, and it only affects a path that is about to abort.

**The saving does not reach the paths that motivated the work item, and 0169 must not wait for it.** `cache_root::resolve` runs the identical write-`chmod`-exec probe on **every** external-subcommand dispatch, before the sub-binary cache-hit test — so a warm `accelerator vcs guard` still pays a first-exec penalty of the same shape (measured 131.97 ms in the repo's `bin/`). Built-ins like `version` never reach it, which is why this PR's measurement cannot see it. Raised as **0189**; **0190** covers `acquire_lock`'s inability to classify an unusable lock directory (one arm spins *unbounded* — `rm -f`/`rmdir` then `continue` with no `sleep` and no `waited` increment); **0191** covers batching the two shim hashes (~2.5 ms measured, essentially 0169's whole shortfall). 0169's hand-off note is re-confirmed with **its threshold and rationale untouched** — that decision remains 0169's own.

**Not done here, deliberately**: the shim staging block's two hashes stay (three tests assert the planted-stub defence they provide); no `launcher`/`launcher_sig` hoisting; no `sha256_file` backend change (the native backend is already in use and `openssl dgst` is slower); no new `test:integration:*` leaf; no pytest marker for the root guard until a root lane actually exists.
