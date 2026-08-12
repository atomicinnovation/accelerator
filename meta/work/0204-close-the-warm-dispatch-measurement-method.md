---
type: work-item
id: "0204"
title: "Close the warm-dispatch latency measurement method"
date: "2026-08-12T11:40:39+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: spike
priority: medium
parent: "work-item:0136"
relates_to:
  ["work-item:0189", "work-item:0169", "work-item:0186", "work-item:0188",
   "work-item:0191"]
derived_from: ["plan:2026-08-11-0189-warm-dispatch-latency-measurement"]
tags: [cli, launcher, performance, bootstrap, measurement]
last_updated: "2026-08-12T11:40:39+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0204: Close the warm-dispatch latency measurement method

**Kind**: Spike
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

0169's Phase 10 deferred a warm-call latency gate — `G ≤ 1.1 × B` on one
darwin host — and 0189 inherited the obligation. Three attempts to specify the
method have now failed review: the decomposition route, the clock domain and
the residual definition have each been answered in prose, reviewed, and found
unsound. This spike answers them against real measurements instead, so
`meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` can specify a
measurement rather than a methodology.

The distinguishing fact is that every failure was a *design* failure, not a
measurement failure. Each proposed method was authored ahead of the evidence
that would have settled it — an instrumented launcher that cannot reach the
verified path, a budget whose terms close by construction, a sample count
inherited from a measurement of a four-fold effect and applied to a ten percent
margin. The remedy is to measure first and specify second.

## Context

Three constraints make this harder than a stopwatch:

**The verified path cannot be instrumented.** `bin/accelerator:378`
minisign-verifies the cached launcher against the committed release key before
`exec`, and the only bypass — the dev-launcher override at `:239-251` — `exec`s
at `:250`, above both `sha256_file` calls (`:291`, `:295`) and above the
launcher verification. A launcher built to emit timestamps therefore cannot be
sampled through the bootstrap it is meant to measure. Signing one locally is
not an option either: `bin/accelerator:165` reads
`keys/accelerator-release.pub` and `cli/launcher/build.rs:31-32,:43` embeds
that same file, so the bootstrap's verification key and the launcher's embedded
key are one artefact.

**A contiguous budget proves nothing.** 0186's composition budget was credible
because its terms were measured independently and then compared against the
median. A budget whose terms are contiguous slices of the interval they explain
closes by construction and can detect nothing — and a budget whose *dominant*
term carries no independent cross-check has the same defect over most of its
mass, whatever the arithmetic says.

**A degraded sample looks fast.** `swallow_under_fail_safe` (`core.rs:219-224`)
swallows `kernel::Error::Failed` and `handle_dispatch_error`
(`main.rs:215-226`) then exits 0 without exec'ing the sub-binary, skipping both
`reverify` and the sub-binary run. Fifty such samples would record a PASS. Any
method must discriminate a real deny from a degradation on every sample, and
the two guards emit structurally different envelopes to do it with.

## Requirements

### SQ-1: How is the launcher-side decomposition obtained?

Determine, by attempting it, whether every launcher-internal term can be
measured independently through public API in the shipped release profile —
`verifier::sha256_hex`, `verify_binary`, `TrustedKeys::embedded`/`verifies`,
`cache::find`, `install_crypto_provider`, `Fetcher::new` — with the bootstrap
term taken from a **built-in** dispatch through the shipped, unmodified
bootstrap, as 0186 obtained its 29.92 ms median.

Record which terms this reaches and which it does not. If any term is
unreachable, record what it would cost to reach it, including whether the
dev-override route (marker plus `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER` plus
`ACCELERATOR_LAUNCHER_BIN` inside `cli/target/`) is the only alternative and
what it defeats.

Two boundary questions must be answered explicitly, because a prior attempt got
them wrong: a built-in dispatch already contains the launcher's `execve`,
dynamic loading, `logging::init` and clap parse, so **state whether the
bootstrap term and any launcher-startup term are separable at all** under this
route, or whether they are one composite. And any `reverify` replica built from
public API is not `reverify` — record how the replica's composition and call
order were verified against the private method at a named revision.

### SQ-2: Which clock, and in which domain?

Establish whether any budget term requires subtracting an in-process span from
a harness bracket. If none does, record that no cross-domain subtraction is
performed and the question is closed. If one does, specify the syscall on both
sides — `CLOCK_UPTIME_RAW` on darwin and `CLOCK_MONOTONIC` on linux to match
CPython's `perf_counter`, or wall-clock bracketed with `time.time()` — and
record `time.get_clock_info('perf_counter')` for the interpreter used.

### SQ-3: What makes the residual falsifiable?

Define the residual so it can fail. Two properties must hold together, and a
prior attempt kept only the second:

- **Closure.** The independently measured terms must be compared against
  `median(G)`. Because the terms are independent rather than contiguous, this
  sum is *not* structurally zero, and it is the only check that the budget
  accounts for `G` at all.
- **Per-term agreement.** Each term above a stated fraction of `G` must have an
  independent cross-check, and the disagreement reported. Report signed and
  absolute residuals separately — signed aggregation masks two opposing
  disagreements that cancel.

State the cross-checked fraction of `G` as a number. A residual computed over a
fifth of the measurement is not evidence about the other four fifths.

Independent cross-checks for the bootstrap composite's sub-terms are in scope:
the shim's two `sha256_file` invocations and the launcher's BLAKE2b verify are
individually measurable, and 0186 measured comparable figures.

### SQ-4: Close the statistical design

Pre-register, before any comparative number exists:

- Whether the gate uses raw or floor-subtracted medians, with the bias
  direction of the choice stated and the supporting arithmetic reproducible
  from named figures.
- The gate statistic and its interval — flavour, resample count, one- or
  two-sided, and the confidence level — noting that a one-sided bound has no
  half-width, so any sizing or agreement rule stated in half-widths needs a
  two-sided interval or a different formulation.
- A target precision as a number, the rule for turning pilot dispersion into
  `n`, and the pilot size. If no feasible `n` separates 1.1 from a plausible
  point estimate, that is this spike's answer and the gate is undecidable at
  this margin.
- An outcome taxonomy whose branches are **disjoint on stated arithmetic** and
  which covers every reachable result, including an invalidated session and a
  design-infeasible finding. A taxonomy in which one measurement selects two
  branches is not pre-registration.
- Whether additional sampling is permitted after seeing an interval, and if so
  under what single pre-stated escalation — open-ended extension until a bound
  crosses a threshold is optional stopping and voids the stated confidence
  level.

### SQ-5: Measure `reverify` and assess reachability

`FetchVerifyCacheResolver::reverify` (`mod.rs:90-109`) is the only
O(binary-size) term inside the launcher: a whole-file `std::fs::read`, a full
`sha256_hex` (`verifier.rs:36`) and a full BLAKE2b (`keys.rs:62-69`) over
2,512,576 bytes (`meta/work/0188-library-backed-vcs-adapter.md:1059`).

Measure the three sub-operations **separately** plus the composite, in the
shipped release profile and target, both in-loop warm and cold-process. The
separation is load-bearing: ranking the optimisation levers 0189 would need on
an overrun requires knowing read cost against sha256 cost against BLAKE2b cost,
and a composite ms-per-MB cannot transfer across sizes or architectures when
`sha2` selects hardware SHA instructions on aarch64 and BLAKE2b does not.

State the expected band **before** measuring, so the result confirms or
falsifies a prediction. Then record whether `G ≤ 1.1 × B` is plausibly
reachable at all. If `reverify` alone exceeds the plausible gap, that is this
spike's headline result.

## Acceptance Criteria

- [ ] SQ-1 is answered by attempt, not assertion: the reachable term set is
      recorded, each unreachable term is named with what reaching it would
      cost, and the bootstrap/launcher-startup separability question is
      answered explicitly
- [ ] SQ-2 is answered, with cross-domain subtraction either eliminated or
      specified on both sides and the interpreter's clock info recorded
- [ ] SQ-3 defines the residual with both closure and per-term agreement, and
      states the cross-checked fraction of `G` as a number
- [ ] SQ-4's design is recorded in full, its outcome branches are disjoint on
      stated arithmetic, every reachable outcome has a branch, and any
      post-hoc sampling rule is pre-stated
- [ ] `reverify`'s three sub-operations and composite are recorded, warm and
      cold-process, with the predicted band and whether it held
- [ ] The reachability assessment is recorded, including the case where the
      answer is that the gate is unreachable or undecidable
- [ ] Every throwaway artefact is positively asserted absent — the
      `cli/launcher/examples/` target is removed, no dev-override input is set,
      and `sha256(keys/accelerator-release.pub)` matches its committed value
- [ ] `mise run cli:check` is green while any throwaway example exists (it
      would be the `cli/` workspace's first `examples/` target, in scope for
      pedantic clippy via `--all-targets`)

## Dependencies

- Blocks: `plan:2026-08-11-0189-warm-dispatch-latency-measurement`, which
  cannot specify its measurement until SQ-1 to SQ-4 are answered.
- Relates to: work-item:0189 (owns the obligation this spike unblocks),
  work-item:0169 (deferred the gate; closed `done` with the criterion
  unticked).
- Parent: epic 0136.

## Assumptions

- The gate definition itself — `G ≤ 1.1 × B`, one darwin host, one session, a
  pure-jj fixture — is inherited from 0169 and is not reopened here. This spike
  decides *how* to measure it, not *what* it should be. Any strengthening of
  the criterion (such as gating on an interval bound rather than a median) is a
  deviation this spike must record as such, not adopt silently.
- The measurement is no longer release-blocked: `v1.24.0-pre.36` ships
  `accelerator-vcs-darwin-arm64` with its `.minisig`, and its signed
  `manifest.json` carries all four platform records at `schema_version: 1`.
- No route in this spike writes `keys/accelerator-release.pub`.

## References

- `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` — the plan
  this spike unblocks
- `meta/reviews/plans/2026-08-11-0189-warm-dispatch-latency-measurement-review-1.md`
  — two review passes; pass 2's assessment is what raised this spike
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — the
  delivered figures (29.92 ms warm bootstrap median at `:517`, the 35.1 ms
  shell-guard row at `:66` and its `:615` method-incomparability note, the
  re-derived 3.72 ms at `:577`) and the deviation-recording pattern at
  `:608-616`
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md` —
  the harness shape (`:1073-1135`) and per-sample stdout guard (`:1096-1102`)
- `meta/work/0188-library-backed-vcs-adapter.md:1059` — the 2,512,576-byte
  `aarch64-apple-darwin` asset
- `cli/launcher/src/launch/outbound/resolve/mod.rs:90-109` — `reverify`
- `cli/launcher/src/launch/outbound/resolve/verifier.rs:36`,
  `keys.rs:62-69` — the sha256 and BLAKE2b passes
- `cli/launcher/src/launch/core.rs:219-224`,
  `cli/launcher/src/main.rs:215-226` — the fail-safe swallow and exit-0 path
- `cli/launcher/build.rs:31-32`, `:43` — the embed of
  `keys/accelerator-release.pub` and its `cargo:rerun-if-changed`
- `bin/accelerator:165`, `:239-251`, `:291`, `:295`, `:353`, `:378` — the
  verification key, the dev-launcher bypass, the shim hashes and the launcher
  verification
