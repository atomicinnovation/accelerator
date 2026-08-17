---
type: work-item
id: "0191"
title: "Batch the bootstrap's two shim hashes into one sha256 invocation"
date: "2026-08-03T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: draft
kind: task
priority: low
parent: "work-item:0136"
relates_to:
  ["work-item:0186", "work-item:0169", "work-item:0189", "work-item:0205"]
tags: [shell, performance, bootstrap, bash-3.2]
last_updated: "2026-08-17T12:30:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0191: Batch the bootstrap's two shim hashes into one sha256 invocation

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

The shim-staging condition in [`bin/accelerator`](../../bin/accelerator) calls
`sha256_file` twice on every invocation — once for the source shim and once for
the staged copy — each forking the backend plus an `awk`. Batching both into a
single `sha256sum f1 f2` with no `awk` saves roughly 2.5 ms per warm call
without weakening anything: both digests are still computed and compared.

## Context

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

| | 0205 | Attempt 2 (`meta/measurements/warm-dispatch-2.json`) |
| --- | --- | --- |
| `median(B)` | 33.00 ms | **27.98 ms** |
| `median(G)` | 42.28 ms | **37.56 ms** |
| ratio of medians | 1.2813 | **1.3423** [1.3395, 1.3445] |
| `median(G)` needed for a ratio of 1.3 | 42.90 (already met) | **36.31 ms** |
| shortfall against 1.3 | none | **1.25 ms** |
| this item's measured saving (fast backend) | 2.48 ms | 2.48 ms |

**So 2.48 ms against a 1.25 ms shortfall — roughly twice what is needed.** The
two `sha256_file` substitutions were re-measured in attempt 2's own session at
**4.316 ms** combined, consistent with the 7.05 ms row below once the 2.02 ms
bash baseline is netted off, so the saving is expected to hold on this host.

⚠️ **The projection is a projection.** It assumes `median(B)` is unmoved by a
change that touches only `G`, which is sound, and that the whole 2.48 ms lands
on the warm path, which the short-circuit below could reduce on a cold run but
not on the warm one this ratio is measured over. Projected ratio ≈ **1.254**.

**What 0189 now expects of this item.** 0189's C5 threshold was raised from 1.3
to **1.4** on 2026-08-17 by author decision, which attempt 2 clears without any
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

- [ ] The warm path forks the sha256 backend once, not twice, and forks no
      `awk` — assertable from a `bash -x` trace using the seam 0186 added to
      `run_bootstrap`.
- [ ] The three planted-stub tests pass unmodified.
- [ ] A cold run with no staged shim behaves as today and produces no spurious
      diagnostic from the missing second input.
- [ ] Warm-path median measured before and after in one session on one host,
      both figures and the resolved backend recorded.
- [ ] **The warm-dispatch ratio is re-measured after this lands**, via `mise run
      measure:warm-dispatch`, and the before/after `median(G) / median(B)`
      recorded beside the millisecond figures. Added 2026-08-17: this item's
      2.48 ms is roughly twice the 1.25 ms that separates the measured 1.3423
      from a ratio of 1.3, so it is the route to tightening 0189's C5 threshold
      back from 1.4 to 1.3 on evidence. ⚠️ Reaching 1.3 is **not** a pass
      condition for this item — its own case stands on the millisecond saving —
      but the re-measurement is, because without it the tightening has nothing
      to rest on.
- [ ] `scripts/lint-bashisms.sh`, shfmt and ShellCheck report no findings.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- **Relates to**: 0186 (measured the saving and declined to absorb it), 0169
  (whose latency gate this most directly affects).
- **Parent**: epic 0136.

## Assumptions

- Every supported backend prints one `<digest>  <path>` line per input in
  argument order. Verified for Apple `/sbin/sha256sum`; unverified for GNU
  coreutils and `shasum` at time of writing.

## References

- `bin/accelerator` — `sha256_file` and the shim-staging condition
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — Validation
  Results, which carries the measurement and the backend range
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md` —
  What We're NOT Doing
