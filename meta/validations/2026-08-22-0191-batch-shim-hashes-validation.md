---
type: plan-validation
id: "2026-08-22-0191-batch-shim-hashes-validation"
title: "Validation Report: Batch the bootstrap's two shim hashes into one sha256 invocation"
date: "2026-08-24T22:51:44+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: partial
parent: "plan:2026-08-22-0191-batch-shim-hashes"
target: "plan:2026-08-22-0191-batch-shim-hashes"
tags: [shell, performance, bootstrap, bash-3.2, sha256]
last_updated: "2026-08-24T22:51:44+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Batch the bootstrap's two shim hashes into one sha256 invocation

Both phases are implemented and every automated check that can run on this host is green. The result is **partial** for one reason only: two success criteria (AC-5 GNU coreutils format, AC-8 full `mise run` on the linux lane) are structurally CI-gated and cannot be confirmed before the branch is pushed. Nothing is failing; the plan authors marked these `[~]` themselves.

### Implementation Status

- ✓ **Phase 1: Glob-discovered batched digest** — fully implemented (`d208b5cc4758`). `sha256_files` variadic helper, hex-constrained glob discovery, path-column parse of both source and staged digests, and the rewritten rationale comment are all present in `bin/accelerator`. All seven test additions (changes 1, 2, 6, 7 plus the two shim helpers) landed.
- ✓ **Phase 2: Measurement and evidence** — fully implemented (`d13b571cc3c9`). `warm-dispatch-4.json` + samples written; the work item carries before/after digest-bracket medians, seeded-accumulation break-even, DoS ceiling, and the re-measured warm-dispatch ratio.

### Automated Verification Results

- ✅ `scripts/lint-bashisms.sh bin/accelerator` — clean.
- ✅ Full entrypoint suite — `63 passed` (`uv run pytest tests/integration/entrypoint/test_accelerator_entrypoint.py`).
- ✅ New + regression subset — `7 passed` (`forks_the_sha256_backend_once`, `on_the_shasum_fallback`, `stale_staged_shim`, `cold_path_forks`, three `planted_staged_shim`).
- ✅ Newline cache-dir guard — `test_a_record_is_always_one_line` passes (the `-k newline` filter in the plan matches nothing; see Deviations).
- ✅ `mise run check` — exit 0 (format + lint + types across all four components).
- 🟡 `mise run` (bare default, incl. linux CI GNU-coreutils lane) — passed locally per `7a5d7bd50e4d`; the linux lane rides on the push.

### Code Review Findings

#### Matches Plan

- **Variadic helper** — `sha256_files` runs one backend fork over `"$@"`, no `awk`, raw `<digest>␣␣<path>` lines (`bin/accelerator:274-281`).
- **Hex-constrained glob** — the candidate loop admits only regular executable files with a bare 64-lowercase-hex suffix; empty-array-safe branch on `${#shim_candidates[@]}` (`bin/accelerator:298-321`), honouring the bash-3.2 `set -u` gotcha.
- **Path-column keying** — both the source digest (matched on `${shim_source}`) and the staged digest (matched on `${shim}`) are keyed to the path column, never output position; `[[ -n "${shim_digest}" ]]` aborts only on a genuinely absent source line. This is the AC-4 discharge exactly as the plan argued.
- **Fail-open batched call, fail-closed source call** — the multi-candidate `else` branch tolerates a vanished candidate (no `|| fail_integrity`); the zero-candidate branch keeps `|| fail_integrity`.
- **Rationale comment rewritten** — states the content-addressing why, the glob-forks-once mechanism, and the hex-constraint-makes-the-parse-unforgeable invariant; the stale per-test-name references are gone (`bin/accelerator:264-289`).
- **Phase 2 evidence complete** — Apple `sha256sum` 3.557→1.204 ms (saving 2.352 ms), Perl `shasum` 18.208→9.134 ms (saving 9.074 ms), n=200 interleaved with IQR; break-even N=16 stale candidates; k=64 adversarial worst case (15.4 ms); warm-dispatch ratio 1.3260→**1.2773** [1.2747, 1.2806], clears 1.3.

#### Deviations from Plan

- **`2>/dev/null` moved into the helper** — the plan (change 3) left `sha256_files` unredirected and put `2>/dev/null` on the batched call site (change 4). The implementation suppresses stderr inside the helper for both branches instead. Semantically equivalent — the zero-candidate branch still catches non-zero exit via `|| fail_integrity`, and the batched branch still fails open. Cleaner; no behavioural change.
- **`-k newline` filter is a no-op** — the plan's criterion `mise run test:integration -- -k newline` matches no test name. The intended guard is `test_a_record_is_always_one_line` (its cache dir is `cache\nwith-newline`), which exists and passes. Documentation-only slip in the plan; the test coverage is real.

#### Potential Issues

- **None material.** The two `[~]` criteria are CI-lane confirmations, not defects. The documented directory-path domain constraint (newline/backslash/edge-whitespace in the cache path degrades to a benign re-stage) is guarded by `test_a_record_is_always_one_line`.

### Manual Testing Required

1. Cross-backend confirmation (blocks closing the plan):
  - [ ] Push the branch and confirm the linux CI lane runs the Phase 1 tests green against GNU coreutils `sha256sum` (AC-5, AC-8).
2. Eyeball trace sanity (plan's standing manual check):
  - [ ] A `bash -x` warm-path trace shows exactly one `sha256sum`/`shasum` line and no `awk` line — already asserted by `test_warm_path_forks_the_sha256_backend_once_without_awk`, kept as a by-eye backstop.

### Recommendations

- **Push to confirm the third backend, then close.** The only gap is the linux-CI GNU-coreutils lane; once it is green, flip AC-5 and AC-8 to `[x]` and re-run `/validate-plan` (or set the plan `status: done` directly) — the result becomes an unqualified pass.
- **Feed the ratio evidence to 0189.** The after-ratio 1.2773 clears 1.3, which is the stated evidence route to tightening 0189's C5 threshold from the relaxed 1.4 back to 1.3. Track that follow-up on 0189, not here.
- **Fix the plan's `-k newline` criterion if the plan is revised** — point it at `test_a_record_is_always_one_line` so the filter is runnable; low priority, the coverage already exists.
