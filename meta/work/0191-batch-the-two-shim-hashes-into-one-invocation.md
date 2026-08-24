---
type: work-item
id: "0191"
title: "Batch the bootstrap's two shim hashes into one sha256 invocation"
date: "2026-08-03T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: ready
kind: task
priority: low
parent: "work-item:0136"
relates_to:
  ["work-item:0186", "work-item:0169", "work-item:0189", "work-item:0205",
   "work-item:0215", "work-item:0216"]
tags: [shell, performance, bootstrap, bash-3.2]
last_updated: "2026-08-24T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-721
---

# 0191: Batch the bootstrap's two shim hashes into one sha256 invocation

**Kind**: Task
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

The shim-staging condition in [`bin/accelerator`](../../bin/accelerator) calls
`sha256_file` twice on every invocation — once for the source shim and once for
the staged copy — each forking the backend plus an `awk`. Batching both into a
single `sha256sum f1 f2` with no `awk` saves roughly 2.5 ms per warm call
without weakening anything: both digests are still computed and compared.

## Context

**Current position (2026-08-22).** Batching the two hashes is worth doing on its
own merits — a measured ~2.48 ms warm-path saving on this host's fast backend.
The dated blocks below are the history of how its relationship to the latency
gate shifted; the settled reading is that it is **not** required to close that
gate (0189 closed at a ratio ceiling of 1.4, which the current measurement
already clears) but is the evidence route to tightening the ceiling back to 1.3.
Throughout, **G** is warm-call latency of the guarded `bin/accelerator` dispatch
path and **B** the bare shell baseline it is measured against; `median(G) /
median(B)` is 0189's **C5** criterion (its fifth acceptance criterion). Both
definitions are inherited from 0169.

Raised from 0186's closeout, which removed the exec probe and left this as the
largest remaining warm-path term. 0186 deliberately did **not** absorb this: it
needs a branch to preserve today's short-circuit, and ~2.5 ms is essentially the
whole of 0169's ~2.4 ms shortfall, so it deserves its own before/after rather
than riding along inside another change's measurement.

**Retracted 2026-08-13.** The shortfall was **5.98 ms**, not ~2.4 ms. Work item
0205 measured warm dispatch at `median(G) = 42.28 ms` against `median(B) = 33.00
ms` — a ratio of medians of 1.2813 against the inherited `G ≤ 1.1 × B` ceiling
of 36.30 ms. This item's saving on that host is a **measured 2.48 ms** (the 7.05
ms and 4.57 ms rows below), so it was never sufficient to reach the inherited
threshold; it covers under half the overrun. And under the criterion 0189 now
carries — an absolute `median`/`p90` budget per digest backend with the ratio
retained at 1.3 as a historical comparison — this item is **not a latency-gate
co-requisite at all**. Its case now rests on the fallback backend, per "The
saving is backend-dependent" below. The item's own merits are unaffected.

**Amended 2026-08-17: this item may reach a ratio of 1.3, and a re-measurement
should follow it.** The retraction above reasoned from 0205's figures, where the
overrun was 5.98 ms and this item's 2.48 ms could not cover it. Two sessions
under the committed harness measured the same host quieter, and the arithmetic
inverts:

| | 0205 | Attempt 2 (invalid) | **Attempt 3 (valid)** |
| --- | --- | --- | --- |
| `median(B)` | 33.00 ms | 27.98 ms | **26.796 ms** |
| `median(G)` | 42.28 ms | 37.56 ms | **35.531 ms** |
| ratio of medians | 1.2813 | 1.3423 | **1.3260** [1.3236, 1.3279] |
| `median(G)` needed for a ratio of 1.3 | 42.90 (already met) | 36.31 ms | **34.784 ms** |
| shortfall against 1.3 | none | 1.25 ms | **0.747 ms** |
| this item's measured saving (fast backend) | 2.48 ms | 2.48 ms | 2.48 ms |

Attempt 3 is the valid, closing session (`meta/measurements/warm-dispatch-3.json`);
attempt 2 is retained above because it was quoted here first.

**So 2.48 ms against a 0.747 ms shortfall — over three times what is needed.**
The two `sha256_file` substitutions were re-measured in attempt 3's own session at
**3.824 ms** combined (4.316 ms in attempt 2). These come from a different session
than the 7.05 ms table row below and were taken in a different shape, so they only
broadly agree rather than reconcile term-for-term — but both put the two-hash cost
well above the 0.747 ms shortfall, so the saving is expected to hold on this host.

⚠️ **The projection is a projection.** It assumes `median(B)` is unmoved by a
change that touches only `G`, which is sound, and that the whole 2.48 ms lands
on the warm path, which the short-circuit below could reduce on a cold run but
not on the warm one this ratio is measured over. Projected ratio ≈ **1.233**.

**What 0189 now expects of this item.** 0189's C5 threshold was raised from 1.3
to **1.4** on 2026-08-17 by author decision, which attempt 3 clears without any
lever, so this item is **not** a blocker on closing 0189. Its role is the
opposite: if it lands and a fresh session confirms the ratio under 1.3, the
threshold can be **tightened back to 1.3 on evidence** rather than left relaxed
on it. That is the only route on the table that reverses a post-hoc relaxation.
**Re-measure with `mise run measure:warm-dispatch` after this item lands** and
record the before/after ratio alongside the before/after millisecond figures
this item's own criteria already ask for.

Measured on darwin-arm64 (macOS 26.3, Apple M4 Max):

| Shape | ms |
| --- | --- |
| two `$(sha256sum f \| awk …)` substitutions | 7.05 |
| one `$(sha256sum f1 f2)`, no `awk` | 4.57 |
| bash-interpreter baseline | 2.02 |

Re-measured during 0186's closeout in a slightly different shape — one
`sha256_file` call marginal over a `bash -c` baseline — at **3.55 ms**, so the
two calls cost around 7 ms together, consistent with the table.

**The saving is backend-dependent.** On this host `command -v sha256sum`
resolves to `/sbin/sha256sum`, an Apple-signed Mach-O. Where only the Perl
`shasum` fallback exists the per-call cost is ~12 ms rather than ~3.5 ms, so the
two-hash residual swings roughly 3×. Confirm which backend each CI lane resolves
before quoting a figure.

The staged-shim hash currently runs **only when the staged shim is executable**
(`[[ ! -x "${shim}" ]] ||` short-circuits), so a naive batch would hash a
nonexistent file on a cold run. Apple's `/sbin/sha256sum` handles that
gracefully — it prints the surviving digests, writes the error to stderr and
exits 1 — but the parse must not silently mis-assign digests to paths when one
input is missing.

## Requirements

- Compute both digests in one backend invocation, dropping the two `awk` forks.
- Preserve the short-circuit: a cold run must not pay for hashing a file that
  does not exist, and must not mis-parse the output when it does.
- Preserve the planted-stub defence exactly — both digests still compared. The
  three tests at
  `tests/integration/entrypoint/test_accelerator_entrypoint.py`
  (`test_planted_staged_shim_rehashed_then_succeeds`,
  `test_planted_staged_shim_is_not_trusted`,
  `test_planted_staged_shim_via_cache_dir_is_not_trusted`) stay green
  unmodified.
- Confirm the multi-file output format and the missing-file exit semantics on
  **both** the Apple `/sbin/sha256sum` and the GNU coreutils backends, and on
  the `shasum` fallback if the batched form is used there too.
- Stay within the bash 3.2 floor.

## Acceptance Criteria

- [x] The warm path forks the sha256 backend once, not twice, and forks no
      `awk` — assertable from a `bash -x` trace using the seam 0186 added to
      `run_bootstrap`.
- [x] The three planted-stub tests pass unmodified.
- [x] A cold run with no staged shim exits with the same status as before the
      change, and its combined output carries no new stderr line referencing the
      missing second hash input — verified against a captured before/after diff
      of the cold-run output.
- [x] On the one-input-missing path (no staged shim), the source shim's digest
      from the batched call equals its standalone `sha256_file` value, asserting
      digests are keyed to their path rather than to output position so a missing
      second input cannot mis-assign the surviving digest.
- [~] The batched multi-file output format and the missing-second-input exit
      behaviour are confirmed on the GNU coreutils backend (which the linux CI
      lane resolves) and on the `shasum` fallback if the batched form is used
      there, with the observed output recorded — mirroring how the criterion
      below records the resolved backend. Apple `/sbin/sha256sum` and Perl
      `/usr/bin/shasum` confirmed locally (`<hex>␣␣<path>` per input, argument
      order, exit 0); GNU coreutils awaits the linux CI lane running the Phase 1
      tests green.
- [x] Warm-path median measured before and after in one session on one host,
      both figures and the resolved backend recorded, and the after-median is
      strictly less than the before-median on the resolved backend. The absolute
      delta is recorded but not itself gated, to absorb host variance.
- [x] **The warm-dispatch ratio is re-measured after this lands**, via `mise run
      measure:warm-dispatch`, and the before/after `median(G) / median(B)`
      recorded beside the millisecond figures. Added 2026-08-17: this item's
      2.48 ms is over three times the 0.747 ms that separates the measured
      1.3260 from a ratio of 1.3, so it is the route to tightening 0189's C5
      threshold back from 1.4 to 1.3 on evidence. ⚠️ Reaching 1.3 is **not** a pass
      condition for this item — its own case stands on the millisecond saving —
      but the re-measurement is, because without it the tightening has nothing
      to rest on.
- [x] `scripts/lint-bashisms.sh`, shfmt and ShellCheck report no findings.
- [x] `mise run` (bare default task) exits 0 end-to-end.

## Measurement Results

Measured 2026-08-24 on darwin-arm64 (macOS 26.3, Apple M4 Max), one session,
n=200 interleaved samples per shape, each marginal over an empty `bash -c :`
bracket. `before` is the two `sha256sum f | awk` substitutions; `after` is the
one `sha256sum f1 f2` call with no `awk`.

**Digest bracket, both resolved backends (AC-6, AC-5).**

| Backend | before | after | saving | after < before |
| --- | --- | --- | --- | --- |
| `/sbin/sha256sum` (Apple, fast) | 3.557 ms (IQR 0.488) | 1.204 ms (IQR 0.276) | **2.352 ms** | yes |
| `/usr/bin/shasum` (Perl, fallback) | 18.208 ms (IQR 0.550) | 9.134 ms (IQR 0.345) | **9.074 ms** | yes |

The 2.352 ms fast-backend saving agrees with the ~2.48 ms projected from the
attempt-3 session. The fallback saving is ~3.9× larger, consistent with the
~3× backend swing noted in Context. Both backends print `<hex>␣␣<path>` per
input in argument order and exit 0 on the multi-file form; **GNU coreutils
remains confirmed only once the linux CI lane runs the Phase 1 tests green**.

**Steady-state accumulation and the DoS ceiling (AC-6 steady state).** The
batched call hashes the source plus every hex-named candidate, so a growing
cache costs one extra in-process read+hash each. Sweeping the candidate count
`k` (distinct ~475 KB copies):

| k candidates | fast marginal | fallback marginal |
| --- | --- | --- |
| 1 (steady state) | 1.204 ms | 9.134 ms |
| 2 | 1.406 ms | 10.020 ms |
| 4 | 1.847 ms | 11.868 ms |
| 8 | 2.751 ms | 15.485 ms |
| 16 (break-even) | 4.511 ms | 22.766 ms |
| 32 | 8.194 ms | 37.382 ms |
| 64 (worst case) | 15.412 ms | 66.817 ms |

**Break-even N = 16 stale candidates** on both backends — the count at which the
accumulated read cost cancels the two-fork `before`. A cache dir reaches that
only with 16 distinct historical shim digests, far above realistic release
churn (the verify shim changes rarely, and only a changed shim adds a digest).
The k=64 row is the adversarial cache-dir-write worst case: a known 15.4 ms
(fast) / 66.8 ms (fallback) per-warm-start amplification, a denial of service
against an already-attacker-writable cache, never a trust breach.

**Warm-dispatch ratio (AC-7).** `mise run measure:warm-dispatch` after the
change wrote `meta/measurements/warm-dispatch-4.json` (host uncalibrated for
bash/shasum; load ~4.0/16, so absolutes are indicative not gated):

| | before (`warm-dispatch-3`) | after (`warm-dispatch-4`) |
| --- | --- | --- |
| C5 `median(G)/median(B)`, fast | 1.3260 [1.3236, 1.3279] | **1.2773 [1.2747, 1.2806]** |
| `median(G)`, fast | 35.531 ms | 36.155 ms [36.079, 36.231] |

The after-ratio **clears 1.3** (and the relaxed 1.4 the gate now carries), so it
is the evidence to tighten 0189's C5 back from 1.4 to 1.3. `median(G)` rose
slightly against the attempt-3 figure because this session's host was busier
(higher `median(B)` too), which is exactly why the ratio, not the absolute, is
the transferable quantity — and the ratio moved in the expected direction.

## Open Questions

- Does the batched `sha256sum f1 f2` form print one `<digest>  <path>` line per
  input in argument order, and exit sanely when the second input is missing, on
  GNU coreutils and on the `shasum` fallback? Verified only on Apple
  `/sbin/sha256sum`. This blocks quoting a saving on any non-Apple lane, and
  decides whether the parse can key on argument order or must key on the path
  column.
- Is the residual still worth removing once 0215 and 0216 are weighed? The
  measured 2.48 ms is on the fast Apple backend; 0216 may cut the digest cost at
  source and 0215 may remove a whole warm-path hash, either of which changes
  what this term is worth — and which backend a CI lane resolves already swings
  it roughly 3×. Resolved for scheduling as intentionally independent (see
  Dependencies → Sequencing): this is context, not a gate on when the item runs.

## Dependencies

- **Relates to**: 0186 (measured the saving and declined to absorb it), 0169
  (whose latency gate this most directly affects, and the origin of the
  `median(G)/median(B)` definition), 0189 (measured the ratio this item can
  move), **0205** (the source of the baseline warm-dispatch measurement —
  `median(B)`/`median(G)`, the 1.3260 ratio and `warm-dispatch-3.json` — that
  this item is measured against), **0215** (the other warm-path lever — removing
  the cache-hit sha256) and **0216** (which may cut the digest cost instead,
  changing what this item is worth).
- **Sequencing**: intentionally independent of 0215/0216. On the fast backend the
  two-hash batching stands on its own measured saving regardless of whether a
  sibling later removes or cheapens a hash, so this item is not gated on that
  decision; the second Open Question records the contingency as context, not a
  scheduling constraint.
- **Parent**: epic 0136.

## Assumptions

- The warm-path `median(B)` is unaffected by a change that touches only `G`. The
  projected post-change ratio of ≈1.233 depends on it.

## Technical Notes

- Two call sites, one guarded. `sha256_file "${shim_source}"` always runs;
  `sha256_file "${shim}"` runs only past the `[[ ! -x "${shim}" ]] ||`
  short-circuit. A batch must not unconditionally hash `${shim}` on a cold run.
- `sha256_file` is inlined into `bin/accelerator` deliberately — the
  root-of-trust entry point sources nothing. Any batched helper stays inline.
- Two viable shapes: keep the guard and batch only when the staged shim exists;
  or always batch both, tolerate the missing-file stderr plus exit 1, and parse
  the surviving digests by path column rather than position. The requirement not
  to mis-assign digests when one input is missing favours the keyed parse, or
  the guarded form that avoids the case entirely.
- Bash 3.2 floor: no `mapfile`/`readarray`, no associative arrays. Read the two
  lines with `read` over a here-string or a positional split.
- Assert the fork count via the trace seam 0186 added to `run_bootstrap` — one
  backend fork, zero `awk`, from a `bash -x` trace.

## Drafting Notes

- Moved the backend output-format item from Assumptions to Open Questions on the
  author's instruction, and narrowed Assumptions to the one load-bearing
  projection assumption so the section stays substantive.
- Confirmed against 0189 (now `done`) that its measurement approach is settled:
  absolute `median`/`p90` budgets gate, and C5 (`median(G)/median(B) ≤ 1.4` on
  the fast backend) is retained only as the historical comparison discharging
  0169. This item's "tighten C5 to 1.3 on evidence" reading remains accurate as
  an opportunity, not a blocker.
- Read the current `bin/accelerator` to ground the Technical Notes rather than
  infer them.
- Review pass 1 (2026-08-22) drove the AC and clarity edits: defined `G`/`B` and
  `C5` in Context, gated the measurement AC, added the cross-backend AC, and
  recorded 0205 plus the intentional-independence decision on 0215/0216. The
  warm-dispatch ratio re-measurement criterion's 0189-tightening rationale was
  kept as-is by author decision.

## References

- `bin/accelerator` — `sha256_file` and the shim-staging condition
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — Validation
  Results, which carries the measurement and the backend range
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md` —
  What We're NOT Doing
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — origin (its deferred
  Phase 10) of the `median(G)/median(B)` ratio definition this item inherits
- `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md` — defines the
  C5 ratio criterion and the 1.4 ceiling this item's re-measurement can tighten
- `meta/work/0205-close-the-warm-dispatch-measurement-method.md` — the baseline
  warm-dispatch figures (`warm-dispatch-3.json`) this item is measured against
