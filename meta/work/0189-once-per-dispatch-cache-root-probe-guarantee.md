---
type: work-item
id: "0189"
title: "At-Most-Once Guarantee for the Launcher's Cache-Root Probe"
date: "2026-08-03T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: in-progress
kind: task
priority: low
parent: "work-item:0136"
blocked_by: ["work-item:0169"]
relates_to:
  ["work-item:0186", "work-item:0164", "work-item:0191", "work-item:0205"]
tags: [cli, launcher, performance, bootstrap]
last_updated: "2026-08-17T12:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0189: At-Most-Once Guarantee for the Launcher's Cache-Root Probe

**Kind**: Task
**Status**: In Progress
**Priority**: Low
**Author**: Toby Clemson

## Summary

The launcher's write-chmod-exec cache-root probe has already been moved off the
warm dispatch path — 0169 landed the split this item originally asked for. Three
things remain. Pin the probe's invocation count with a test across the warm-hit,
cold-miss and refetch paths; delete `cache_root::resolve`, the now-unused
wrapper that keeps a second probe path alive in the module; and take the
warm-dispatch latency measurement 0169 deferred, which no other open item owns.

Throughout this item, **dispatch** means one launcher process. One
`verify_writable` call performs exactly one probe and increments the
`SEQUENCE` counter exactly once; "probe count" always means that counter.

**Retracted 2026-08-12.** `SEQUENCE` cannot serve as the probe count.
`SEQUENCE.fetch_add` sits *after* the `create_dir_all` early return inside
`probe_writable_and_executable`, so a `verify_writable` call whose directory
creation fails increments it zero times — demonstrated by running
`a_probe_against_an_uncreatable_directory_still_counts` against a hoisted
`SEQUENCE` accessor, which read a delta of 0. "Probe count" therefore means the
invocation count held in `PROBE_ATTEMPTS`, a thread-local incremented as the
first statement of `verify_writable`. `SEQUENCE` keeps its filename-uniqueness
meaning unchanged.

## Context

`cache_root::candidate`
(`cli/launcher/src/launch/outbound/resolve/cache_root.rs`) selects the cache
root — the `ACCELERATOR_CACHE_DIR` override when set, otherwise
`${ACCELERATOR_PLUGIN_ROOT}/bin` — and does nothing else: no filesystem write,
no process spawn. That derivation is why assertions about a read-only *plugin*
root and a read-only *cache* root both appear below; they are the same probe
reached by different selections.

The probe itself is `verify_writable` (renamed by 0169 from
`probe_writable_and_executable`, which survives as the private function it
calls). It runs from exactly one production call site:
`FetchVerifyCacheResolver::fetch_verify_store`
(`cli/launcher/src/launch/outbound/resolve/mod.rs:141`), reached on a cache miss
or a failed re-verification, never on a warm hit. `main.rs` calls
`cache_root::candidate`, never `cache_root::resolve`.

Two of this item's original acceptance criteria are therefore already satisfied,
by tests 0169 landed:

- A warm dispatch writes no probe file —
  `resolve_succeeds_from_a_read_only_cache_root_on_a_hit`
  (`cli/launcher/tests/resolution.rs:549`).
- A cold dispatch still probes and an unwritable root still fails with
  `CacheRootUnavailable` —
  `an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss`
  (`cli/launcher/tests/resolution.rs:568`), which additionally asserts the
  failure precedes any network round trip.

An amendment to this item dated 2026-08-06 claimed the
`CorruptCacheAndRefetchFailed` retry path could invoke `verify_writable` twice
within one process. Reading `FetchVerifyCacheResolver::resolve` shows it cannot:
every branch returns, so a single resolution reaches `fetch_verify_store` at
most once. That amendment is superseded by this paragraph and was removed rather
than left in place — which makes 0169's Phase 10 hand-off record correspondingly
stale, since it records as complete that dated 2026-08-06 amendments were
grep-verified onto 0125, 0172, 0183 and 0189. The gap the amendment was reaching
for is real, though: no test pins the invariant, so a future refactor could
reintroduce a second probe silently.

`cache_root::resolve` (`candidate` followed by `verify_writable`) survives with
no production caller — only its own four unit tests, which do probe, and which
share a test process with the counting tests below. Removing it leaves
`fetch_verify_store` as the module's single probe entry point, making the
at-most-once property structural rather than incidental.

The pre-fix cost this item was raised to remove — measured on darwin-arm64
(macOS 26.3, Apple M4 Max, 2026-08-03) at **131.97 ms** for the
write-chmod-exec-rm cycle against a **3.72 ms** re-exec of a file left in place
— describes a cost the warm path no longer pays. The probe itself survives on
the miss path, and these figures are retained to explain why the split was made
and as the starting point for the outstanding measurement.

## Requirements

- Pin the probe's invocation count with a test: the count observed across a
  single `FetchVerifyCacheResolver::resolve` call is exactly 1 on a cold miss,
  exactly 1 on each refetch-after-failed-re-verification path, and exactly 0 on
  a warm hit. The assertions take the form of a **delta** captured either side
  of that call, permanently and not as an interim measure — the counter is
  process-wide, so an absolute read is never the right observation.

  **Retracted 2026-08-12.** The counter is thread-local, not process-wide. The
  delta requirement stands, but for a different reason: a single test process
  performs several resolutions on one thread, so an absolute read stops meaning
  anything the moment a second resolution enters it.
- Delete `cache_root::resolve` and re-home its unit tests onto `candidate` and
  `verify_writable`, so that every assertion they make is discharged by a named
  test after the change.
- Do not memoise the probe result. The invariant is to be established by
  structure and asserted by test, not by caching across calls — a process-wide
  cache would also change behaviour for the launcher's concurrent-first-use
  tests, which deliberately resolve from more than one thread.
- Take the warm-dispatch latency measurement 0169's Phase 10 deferred,
  inheriting its definition rather than inventing a new one: warm-call latency
  G against the shell baseline B on one darwin host in one session, both figures
  recorded, gated on `G ≤ 1.1 × B`. This is release-gated (see Dependencies) and
  is the only part of this item that cannot be started immediately.

  **Retracted 2026-08-13.** Both halves of this bullet are false. The
  measurement is **not release-gated**: `v1.24.0-pre.35` and `v1.24.0-pre.36`
  both ship `accelerator-vcs-darwin-arm64` alongside its `.minisig`, and
  `pre.36`'s signed `manifest.json` carries `vcs` entries for all four
  platforms — work item 0205 measured the real bootstrap → launcher →
  sub-binary path with no dev override, which settles it empirically rather
  than by inspection. And the inherited definition is **superseded**: `G ≤ 1.1
  × B` was measured at a ratio of medians of 1.2813 (n = 300, two-sided 95%
  paired-bootstrap CI [1.2662, 1.2899]) and fails. The criterion this item now
  carries is the six-cell definition in [Latency Criterion](#latency-criterion)
  below.
- Land the work in this order: settle the counting seam, then the invariant
  test, then the deletion of `cache_root::resolve`. The deletion goes last
  because its four unit tests are themselves probe call sites; until they are
  gone or re-homed, they can run concurrently with the counting tests in the
  same process, which is what the delta convention and the isolation
  precondition below exist to survive.

  **Retracted 2026-08-12.** The stated reason is wrong twice. The `cli/`
  workspace runs under `cargo nextest`, which gives each test function its own
  process, so those unit tests never share a process with the counting tests;
  and the counter is thread-local, so it would not matter if they did. The
  ordering is kept — it is the right TDD sequence — but nothing depends on it
  for soundness.
- Before starting, re-confirm the two premises this scope rests on:
  `cache_root::resolve` still has no production caller, and no branch or loop in
  `FetchVerifyCacheResolver::resolve` reaches `fetch_verify_store` more than once
  per call. If either has changed, re-scope rather than proceed.

Every count criterion below is captured with **no other probe in flight in the
same process** — each counting test runs in its own test process, or is
serialised against the concurrent-first-use tests and the `cache_root` unit
tests. Without that isolation a delta of 2 is ambiguous between a regression and
cross-test interference.

**Retracted 2026-08-12.** This precondition is withdrawn rather than satisfied.
A thread-local counter makes cross-test interference impossible — every
assertion reads the count from the same thread that drove the calls — so no
runner precondition, `nextest.toml` test-group or serialisation is needed, and
none was built. The delivered assertions are sound under `cargo test` and
`cargo nextest` alike.

## Acceptance Criteria

- [x] Given an empty cache directory and a stubbed fetcher serving a valid
      asset, when a sub-binary is resolved, then the probe count delta across
      that single `FetchVerifyCacheResolver::resolve` call is exactly 1.
- [x] Given a cache pre-populated by the fixture writing a verified binary
      directly to disk (not by a prior resolution), when a sub-binary is
      resolved and re-verification succeeds, then the probe count delta across
      that single `FetchVerifyCacheResolver::resolve` call is exactly 0.
- [x] Given a cached binary whose re-verification fails by a test-only failing
      verifier (never by filesystem permissions on the cache root), when the
      stubbed fetcher serves a valid asset and the refetch **succeeds**, then
      the probe count delta across that single call is exactly 1.
- [x] Given a cached binary whose re-verification fails by the same test-only
      failing verifier, when the stubbed fetcher fails and resolution ends in
      `CorruptCacheAndRefetchFailed`, then the probe count delta across that
      single call is exactly 1.
- [x] Given two successive cold-miss resolutions within one process, with the
      cache directory emptied between them, when both complete, then the probe
      count increments once per resolution — total 2. This is the criterion that
      fails under memoisation.
- [x] With a second `verify_writable` call deliberately introduced into
      `fetch_verify_store`, the cold-miss, both refetch and the two-resolution
      criteria go red with the cold-miss delta observed as exactly 2, while the
      warm-hit criterion stays green; the mutation is then reverted. Without
      this the guard cannot be shown to guard anything, since the invariant
      already holds.
- [x] `verify_writable` has exactly one production call site,
      `fetch_verify_store`, confirmed by a recorded search of the crate.
- [x] `cache_root::resolve` is absent from the crate, and each of the four
      assertions its unit tests made — unset plugin root, override honoured,
      writable plugin root, read-only root — is discharged by a named test
      against `candidate` or `verify_writable`. The read-only case may be
      discharged by the existing `verify_writable_rejects_a_read_only_directory`
      rather than a re-homed copy.
- [x] The two pick-up premises were re-confirmed before work began, and the
      confirmation recorded.
- [ ] Warm-call latency is recorded from one darwin host in one session and
      satisfies the six-cell criterion defined in [Latency
      Criterion](#latency-criterion): an absolute `median(G)`/`p90(G)` budget
      per digest backend as the primary gate (C1-C4, ≤ 50 / 60 / 70 / 80 ms,
      each accepted on its bootstrap interval's **upper bound** at or below the
      ceiling), with `median(G) / median(B) ≤ 1.3` on the fast digest backend
      (C5) retained as the historical comparison that discharges 0169's
      inherited ratio wording, and the fallback-backend ratio (C6) recorded
      ungated. The item closes only when every applicable gating cell C1-C5
      selects branch 1 of the taxonomy that section states. **Supersedes `G ≤
      1.1 × B`**, which was measured at 1.2813 and fails; see that section for
      why it was reframed.
- [ ] The mutation command and its output, the crate search, the old-test →
      discharging-test mapping and the pick-up confirmation are recorded in the
      Validation Results of
      `meta/plans/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`,
      and the latency figures in the Validation Results of
      `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md`.

      **The non-latency clauses are discharged, 2026-08-13.** The record is
      split across two plans because this item is delivered by two: the sibling
      plan (`status: done`, validated `pass`) records the mutation exercise, the
      crate search, the mapping and the pick-up confirmation; the latency plan
      records the figures. Only the latency clause remains open.
- [x] `mise run` (bare default task) exits 0 end-to-end.

**Criteria 1-9 and 12 discharged 2026-08-13** against the delivered, validated
state of the sibling plan
`meta/plans/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`
(`status: done`) and its validation report
`meta/validations/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee-validation.md`
(verdict `pass`). The discharging evidence, by criterion:

| # | Discharged by |
| --- | --- |
| 1 | `a_cold_miss_probes_the_cache_root_exactly_once`; sibling plan's Mutation exercise |
| 2 | `a_warm_hit_never_probes_the_cache_root`; same |
| 3 | `a_successful_refetch_probes_the_cache_root_exactly_once`, via byte poisoning (Open Question 2's resolution) |
| 4 | `a_failed_refetch_probes_the_cache_root_exactly_once`, same seam |
| 5 | `each_of_two_cold_misses_probes_the_cache_root_once` |
| 6 | Sibling plan's Mutation exercise, 4 × 8 sweep with the observed table and the cold-miss delta of 2 quoted; the warm-hit-green-under-Mutation-A clause is discharged by the **validation report**, which reran A over the complete 25-test binary (6 failed / 19 passed), since the warm-hit test was authored under Mutation B |
| 7 | Sibling plan's "Crate search for probe call sites" — one production site, `mod.rs:141` |
| 8 | Sibling plan's "Old-test → discharging-test mapping", four rows |
| 9 | Sibling plan's "Pick-up premise confirmation", recorded before work began |
| 12 | Sibling plan's Validation Results and the validation report, both recording `mise run` green |

## Latency Criterion

**Landed 2026-08-13. This section is authoritative for the criterion text**;
`meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` restates it,
and the per-platform constants table in `tasks/measure.py` is authoritative for
the numbers, bound to a `### Criterion constants` block in `tasks/README.md` by
a lockstep guard.

### Cells

| ID | Statistic | Backend | Ceiling | Gates | Base figure | Headroom |
| --- | --- | --- | --- | --- | --- | --- |
| **C1** | `median(G)` | fast | ≤ 50 ms | yes | 42.28 ms (0205, measured) | +18.3% |
| **C2** | `p90(G)` | fast | ≤ 60 ms | yes | 46.51 ms (0205, measured) | +29.0% |
| **C3** | `median(G)` | fallback | ≤ 70 ms | yes | ~59.2 ms (predicted) | +18.2% |
| **C4** | `p90(G)` | fallback | ≤ 80 ms | yes | ~63.4 ms (predicted) | +26.2% |
| **C5** | `median(G) / median(B)` | fast | ≤ 1.3 | yes | 1.2813 (0205, measured) | — |
| **C6** | `median(G) / median(B)` | fallback | recorded | **no** | ~1.79 (predicted) | — |

`G` is `bin/accelerator vcs guard --format=hook --fail-safe` dispatched through
the real bootstrap with the cache warm; `B` is `hooks/vcs-guard.sh` recovered at
the revision preceding its deletion. **Fast** backend means `command -v
sha256sum` resolves; **fallback** means only the Perl `shasum -a 256` does.

**C1-C4 are the primary gate** and the only re-runnable cells: `B` is a deleted
artefact recovered from `cf42441e2aad-`, so no lane can ever reproduce C5 or C6,
whereas an absolute ceiling can — and it bounds what users actually feel on
every Bash tool call. **C5 is the historical comparison** that discharges 0169's
inherited ratio wording. C6 is context only, because a ratio against a baseline
that hashes nothing is least meaningful where `G` hashes most.

C1-C3's bases are 0205's published figures; C4's base is fast p90 46.51 ms plus
the predicted ~16.9 ms backend delta = 63.4 ms. **C3 and C4 are provisional on
first measurement**: their bases are predictions resting on a cross-session
import of 0186's per-call `sha256_file` pair (3.55 ms against 11.99 ms), so the
first in-session fallback figures become the bases any future re-run is gated
against.

The ceilings are round numbers rather than tuned constants, deliberately: a
ceiling fitted to three significant figures against one session's dispersion
would be a gate calibrated to noise.

### Statistics, by cell kind

The two kinds take different estimators.

**C1-C4 — absolute.** An **unpaired** percentile bootstrap on the single
variant's statistic; a paired bootstrap over `(B, G)` pairs is not the estimator
for a single-variant quantity. Target **upper distance** — `U` minus the point
estimate, not half-width — of **1.0 ms** on the medians and **2.0 ms** on the
p90s. Latency distributions are right-skewed, so an unpaired bootstrap on a
median or p90 is asymmetric, and only the upper tail can breach a ceiling.
Acceptance is the interval's **upper bound at or below the ceiling**. There is
**no floor-subtraction robustness clause**: subtracting a shared spawn floor
from an absolute median makes it *smaller*, so the clause would be strictly
weaker than the primary test rather than a check on it.

**C5 — ratio.** A **paired** percentile bootstrap on the ratio of medians over
interleaved pairs, seeded, at ≥ 10,000 resamples. Two conditions, both of which
must hold:

1. **Gate** — the raw-median interval's **upper bound** ≤ 1.3.
2. **Robustness** — the `true`-floor-subtracted ratio's **point estimate** ≤
   1.3, with its interval recorded as context.

The robustness condition is a **point-estimate** test, and that is a deliberate,
pre-registered weakening with a stated reason. Its margin is 0.003 (1.297
against 1.3), while the upper distance achievable at any practical n is larger:
0.0036 at n = 1,700, 0.0027 at n = 3,000, and ~0.001 only at n ≈ 22,000 (~39
minutes). An upper-bound form would be undecidable at every sample size the
measurement can afford, so branch 1 would be unattainable and the expected
outcome would be branch 3 by construction. The point-estimate form keeps the
check meaningful — it still fails if floor treatment flips the verdict — at the
cost of not bounding its own sampling error, which is recorded.

**Floor treatment.** Three ratios are computed and recorded, in three fixed
roles: raw medians **gate**; `true`-floor-subtracted is the **robustness
check**; bash-floor-subtracted is **diagnostic only**, because it
over-subtracts — bash interpreter startup is real cost `G` pays, since
`bin/accelerator` *is* a bash script. Raw medians are the **lenient** statistic
for a `ratio ≤ k` gate, since `(G−c)/(B−c) > G/B`.

### Sizing

The sizing rule is `n = n₀ × (h₀ / target)²`, where `h₀` and `target` are both
the interval's **upper distance**, not its half-width. 0205's interval is
materially asymmetric — `[1.2662, 1.2899]` around 1.2813 is 0.0151 below and
0.0086 above — so the symmetric half-width 0.0119 corresponds to neither tail,
and for a `ratio ≤ k` gate only the upper tail can decide anything. `h₀ =
0.0086`.

| Block | Samples | Arms | n | Achieved upper distance | Wall clock |
| --- | --- | --- | --- | --- | --- |
| **A** | interleaved `(B, G-fast)` pairs | 2 | 1,700 | ~0.0036 on C5 | ~3.0 min |
| **B** | `G-fallback` alone | 1 | 900 | ~1 ms on C3, ~2 ms on C4 | ~1.3 min |

Block B needs no `B` samples: C3 and C4 are absolute and C6 is not gated. Block
A and Block B are run as **separate interleaved blocks** rather than one
four-arm rotation, so Block B's ~10.7 MB-per-sample hashing load does not enter
the pairs C5 is computed from. `h₀` is re-derived from the first 200 pairs of
Block A and the first 200 samples of Block B as an in-session pilot, whose
samples are **discarded rather than pooled**; a size-up recomputes n from the
same targets, never a relaxed one, does not consume the escalation, and is
bounded by the same 6,900 / 3,600 caps and the 35-minute budget.

C5's margin is 0.0187 against an achieved upper distance of ~0.0036 — **5.2
upper-distances**, which is what makes 1.3 decidable and why the threshold is
the floor of the 1.3–1.5 band rather than its middle.

### Outcome taxonomy

Each cell is classified independently, and **the item closes only when every
applicable gating cell C1-C5 selects branch 1**. C6 is recorded, never
classified. `L` and `U` are the cell's interval bounds, `t` its ceiling (50 / 60
/ 70 / 80 ms, or `k = 1.3`), `h` the achieved upper distance against a target of
1.0 ms on medians, 2.0 ms on p90s and 0.0036 on C5. C1-C4 carry no robustness
condition.

1. **Pass** — `U ≤ t`; and for C5 only, the robustness condition also holds.
2. **Fail** — `L > t`.
3. **Indeterminate** — `L ≤ t < U`, or (C5 only) `U ≤ t` while the robustness
   condition fails. Escalate **once**, to the n the sizing rule gives for an
   upper distance of 0.0018 on C5 or half the ms target on C1-C4, then
   re-classify into branch 1, 2 or 4.
4. **Terminal indeterminate** — after the one permitted escalation the cell
   selects neither branch 1 nor branch 2: the interval still straddles `t`,
   C5's robustness condition still fails, or the cell never reached its
   precision target (`h` > target). Record the achieved `h` and which of the
   three caused it.
5. **Invalidated session** — any per-sample decision mismatch, any inode/mtime
   change on the cached asset, launcher, `.minisig` or staged shim, any growth
   of the unverified log, any teardown verify-phase assertion failure, or any
   precondition failure. **5a, pre-sampling or in-flight** — no figures are
   produced. **5b, post-run** — figures are computed but recorded as explicitly
   **non-gating**, with the failing witness named.
6. **Design-infeasible** — **6a, a priori**: no n within the wall-clock budget
   reaches the escalation target; no figures. **6b, mid-run**: the 35-minute
   budget is exhausted; partial figures are recorded explicitly non-gating.
7. **Not applicable** — the cell cannot be measured on this host at all: no
   `shasum`/Perl, so the fallback farm is unconstructible (C3, C4, C6), or the
   platform key carries no calibrated entry (any cell). Figures, where any
   exist, are **uncalibrated context**, never a verdict. A branch-7 *gating*
   cell needs a recorded, owner-named acceptance before this item closes, on the
   same terms as an accepted deviation.

**Evaluated as an ordered cascade**, first match wins:

```
7  not applicable         →  5  invalidated         →  6a  sizing infeasible
→  6b  budget exhausted   →  4  escalation spent and neither 1 nor 2
→  2  L > t               →  3  straddles, or robustness fails
→  1  U ≤ t (+ robustness)
```

The order matters at two junctions: branch 3's predicate is positional and
carries no escalation term, so the cascade puts 4 first and one escalation
cannot be spent twice; and a validity failure coinciding with infeasible sizing
resolves to 5, because an invalid session's sizing is moot.

**Escalation is session-level, not per-cell.** One scalar governs the session,
and the escalated run **replaces** the initial run's samples rather than pooling
with them — so when any cell selects branch 3, all cells are re-classified from
the escalated run alone and the initial classifications are recorded as
superseded. A cell that passed initially can therefore straddle its ceiling in
the escalated run and take the session to branch 4. No sampling beyond the
single escalation branch 3 permits; open-ended extension until a bound crosses a
threshold is optional stopping and voids the stated confidence level, which is
in any case recorded as **approximate under the single escalation** rather than
an exact 95%.

### The superseded threshold

`G ≤ 1.1 × B`, inherited from 0169's Phase 10 and asserted throughout this item
before 2026-08-13, **fails**. 0205 measured a ratio of medians of **1.2813** at
n = 300, two-sided 95% paired-bootstrap CI **[1.2662, 1.2899]**, `P(ratio > 1.1)
= 1.0000` — an overrun of 5.98 ms against a 36.30 ms ceiling. No sampling choice
moves a point estimate of 1.28 to 1.10.

### Provenance of the band

The 1.3–1.5 band from which the C5 threshold is taken is an **author instruction
given in conversation on 2026-08-13**, approved by **Toby Clemson**. It is not a
corpus figure: nothing in `meta/` states it and 0205 names no numeric band. The
provenance is recorded because the stated mitigation for a post-hoc relaxation
is "the floor of the band was taken", and that mitigation is unauditable unless
the band's origin is on the record.

### Why the criterion was reframed

- **`B` and `G` do not perform comparable work.** `B` decides pure-jj versus
  colocated by testing for two directory entries; `G` loads the repository
  through jj-lib. The Rust guard is not a faster reimplementation of the shell
  guard, it is a more correct one, and a ratio gate calibrated against the
  cheaper behaviour charges it for that correctness. The ratio is further
  demoted to a historical comparison because `B` is a deleted artefact **no CI
  lane can reproduce**, whereas an absolute ceiling is re-runnable.
- **0169 calibrated 1.1 against a `B` cost model that does not hold.** That
  model attributed `jj` and `git` spawns to the shell guard via
  `classify_checkout`. The recovered guard calls `find_repo_root`
  (`scripts/vcs-common.sh:8-18`), a pure-bash upward walk spawning only
  `dirname`, and decides mode by two literal `[ -d ]` tests. There is no `jj`
  spawn and no `git` spawn anywhere in `B`.
- **The absolute premium is imperceptible.** 42.28 ms for a fully
  signature-verified, jj-lib-backed guard against 33.00 ms for an unverified
  stat-and-grep script is a 9.28 ms premium for the whole trust chain, and a
  5.98 ms overrun on a hook is not felt.
- **The optimisation route was declined on posture grounds, not
  trust-boundary grounds.** Removing the cache-hit `sha256` (−4.49 ms) plus 0191
  (−2.48 ms measured on the fast backend) together reach 1.070. That route is
  declined because it sets the launcher's verification posture by an arithmetic
  target. It is **not** declined on the ground that it weakens the trust
  boundary, which it does not: minisign's Ed25519-over-BLAKE2b signature over
  the same bytes is the security boundary and sha256 the corruption check
  (`resolve/verifier.rs:1-2`). Both levers are raised as work items on their own
  merits.
- **1.3 is the floor of the stated band, not a point chosen inside it** — taken
  by paying n = 1,700 for the precision that makes it decidable (5.2
  upper-distances of margin), rather than by citing imprecision to justify the
  band's middle.

⚠️ **The reframing is nonetheless post-hoc and must not be recorded as
pre-registration.** A threshold set at 1.3 after seeing 1.2813 is a threshold
the observed value informed, and the margin — 0.0187 — is small enough that a
materially different quiet-host ratio could fail it. That is the intended
behaviour of a gate; it is stated here so a pass is not mistaken for a
comfortable one.

### The ratio threshold is reopened, 2026-08-17

**C5 is not met, and the measurement is what reopens it.** Two sessions under
the committed harness, recorded at `meta/measurements/warm-dispatch-1.json` and
`-2.json`, put the ratio's whole 95% interval above 1.3. **This subsection is a
decision brief, not a decision**: the threshold is unchanged until an approver
named outside the measurement records one, on the same terms this section's
Provenance paragraph sets for the band itself.

| Session | Load / 16 cpus | `median(B)` | `median(G)` | C5 | Interval |
| --- | --- | --- | --- | --- | --- |
| 0205, n = 300 | 19 | 33.00 | 42.28 | 1.2813 | [1.2662, 1.2899] |
| Attempt 1, n = 6,762 | 38.25 | 31.09 | 40.96 | 1.3177 | [1.3149, 1.3207] |
| Attempt 2, n = 1,700 | 10.63 | 27.98 | 37.56 | **1.3423** | [1.3395, 1.3445] |

Three things the evidence settles, none of which depends on a threshold choice:

- **The absolute budget passes comfortably and improves as the host quietens.**
  Attempt 2 clears C1-C4 by 25%, 35%, 26% and 32%. The primary gate is not in
  question.
- **A quieter host raises the ratio.** Both variants get faster; `B` gets
  proportionally faster, because it is dominated by process spawns while `G`
  carries fixed verification work. So no amount of additional quiet brings C5
  down — the trend runs the wrong way, which also disposes of any expectation
  that 0205's 1.2813 was the pessimistic figure.
- **The miss is 1.25 ms of `median(G)`.** C5's upper bound reaches 1.3 at
  `median(G)` = 36.31 against the measured 37.56.

Four dispositions, with what each costs:

1. **Keep 1.3 and take one lever.** 0191 (batching the two `sha256_file`
   substitutions into one invocation) is measured at 2.48 ms on the fast
   backend, roughly twice the gap, and weakens nothing — both digests are still
   computed and compared. Projected C5 ≈ 1.254. ⚠️ This is the route [Why the
   criterion was reframed](#why-the-criterion-was-reframed) declines, on the
   ground that it sets verification posture by an arithmetic target. 0191's own
   merit is independent of that, so the objection is to the *sequencing*, not to
   the change.
2. **Raise the threshold to 1.4.** Inside the author's stated 1.3–1.5 band, so
   it needs no new band provenance, and it clears both attempts by ~0.055 —
   about 25 achieved upper-distances. ⚠️ It would be the **second** post-hoc
   relaxation of the same threshold, and it voids the first one's stated
   mitigation, that "the floor of the band was taken".
3. **Retire C5 as a gate and keep it as a recorded comparison.** This section
   already calls C5 "the historical comparison that discharges 0169's inherited
   ratio wording" while also marking it `Gates: yes` — a comparison that gates
   is still a gate, and the two readings are in tension. Discharging 0169 by
   *recording* 1.3423 rather than passing 1.3 is consistent with the demotion
   already made, and leaves the criterion purely absolute and re-runnable, which
   is the property C1-C4 were made primary for. ⚠️ It removes the only cell that
   compares against the artefact 0169 actually shipped against.
4. **Close 0189 with a named accepted deviation.** The available
   value-independent rationale: 37.56 ms for a fully signature-verified,
   jj-lib-backed guard against 27.98 ms for an unverified stat-and-grep script
   is a **9.58 ms** premium for the whole trust chain, and imperceptible on a
   hook. ⚠️ Requires an approver named outside this measurement and a rationale
   that does not appeal to the observed number.

⚠️ **The drift band has been re-derived, and it does not stand in the way.**
Attempt 2 is genuinely invalid on drift (`p = 0.0050`), but the ratio sits above
1.3 in every tenth of the session, so the C5 finding is robust to the
invalidation and any disposition above can be taken on it. See the next
subsection.

### The drift band, re-derived 2026-08-17

**Retracting the claim that the band may be unattainable.** An earlier version of
this subsection reasoned that, because both sessions failed the 0.005 band in
opposite directions, no session could ever be valid. Re-deriving the band from
attempt 2's persisted samples refutes that: **a stationary session clears 0.005
88.8% of the time.** The constant was too tight, carrying an unstated
false-positive rate of ~11% — not unattainable.

**The basis.** Permuting the pair *order* destroys temporal structure while
preserving the pairing and both arms' dispersion, so the spread of the
first-third-versus-last-third statistic over permutations is what no-drift looks
like at this sample size on this instrument. A quantile of that null is a band
with a **stated** false-positive rate, derived without reference to the observed
drift — scrambling the session's order leaves the band unchanged while changing
the observed statistic completely. It is a **procedure**, not a constant: the
null narrows with n, so one number is too tight at large n and too loose at
small n.

At attempt 2's n = 1,700, over 10,000 permutations, observed |Δ| = 0.00915:

| Band | Value | Fires |
| --- | --- | --- |
| superseded constant | 0.00500 | yes |
| null quantile 0.90 | 0.00517 | yes |
| **null quantile 0.95 (adopted)** | **0.00615** | **yes** |
| null quantile 0.99 | 0.00825 | yes |
| null quantile 0.999 | 0.01054 | no |

**Attempt 2 genuinely drifted.** `P(|Δ| ≥ observed | no drift) = 0.0050`. The
verdict is unchanged at every quantile up to 0.99, so the change of basis does
not rescue the session — the branch-5b invalidation stands, and it stands on a
sounder footing than the constant gave it. The harness now computes the band and
the significance per session and records the superseded constant's verdict
alongside, so a reader can see whether the change of basis changed the outcome
rather than taking it on trust.

⚠️ **The drift is real but immaterial to C5.** Sliced into ten equal windows in
collection order, the ratio runs 1.3364, 1.3353, 1.3415, 1.3421, 1.3404, 1.3411,
1.3373, 1.3427, 1.3569, 1.3506 — a range of 0.0216, **every window above 1.3 by
at least 0.035**. The drift moves the ratio *within* a band that lies wholly
above the threshold, so it does not explain the level and the C5 finding survives
the invalidation. Any disposition in the preceding subsection can be taken on
that basis.

This also explains why every rehearsal reported branch 5b: at n = 8 the null's
0.95 quantile is ~0.078, so the 0.005 constant fired on pure sampling noise.

⚠️ Adopting the derived band as the *gate* rather than as a recorded diagnostic
is a criterion change. It is implemented as the computed default because its
predecessor's stated basis no longer exists, and because it changes no verdict
here — but confirming it belongs with whoever settles the threshold above.

### Limitations

- **Verified on darwin-arm64 only.** Of the four shipped platforms, darwin-x64
  and linux-arm64 are exercised by no CI lane at all. The linux measurement is a
  named hand-off, and 0205 established that nothing in its findings transfers
  off darwin-arm64 — the sha256-versus-BLAKE2b inversion is a property of this
  chip and this crate build.
- **Neither ratio cell is reproducible on `macos-latest`**, which resolves no
  `sha256sum`, and `B` is reproducible on no CI lane at all. The absolute cells
  are the ones a future lane could enforce, which is why they are primary.
- **The `B`/`G` work asymmetry is accommodated, not resolved.** Constructing a
  baseline that also performs a real classification would make a ratio
  defensible on its own terms; that is not done here.
- **C3 and C4 are provisional on first measurement**, per the Cells table.
- **The empty single-operation pure-jj fixture is `G`'s best case** for the
  jj-lib repository load, while `B`'s two directory-entry tests are
  repository-state-independent. The magnitude is bounded rather than unknown:
  0188 re-measured its library-backed probe at 4.81 ms on this repo's real
  colocated workspace against 4.03 ms on a pure-jj fixture, putting the fixture
  bias at roughly ±1 ms, about 2% of `G`. The empty fixture is **required** by
  the blocked-decision shape, not chosen for favourability — a colocated fixture
  emits **warn** rather than the blocked decision.
- **A host with `sha256sum` but no Perl cannot construct the fallback farm at
  all**, in which case C3, C4 and C6 are recorded not applicable (branch 7).
- **The fallback figures encode this host's Perl interpreter startup**, since
  macOS `shasum` is a Perl script — not a property of the algorithm or the OS.

## Open Questions

- Where should the probe counter live? **Default if unresolved**: expose the
  per-process `SEQUENCE` atomic already inside `probe_writable_and_executable`
  as a single test-only accessor, read as a delta around the call under test —
  which is what makes a process-wide counter sufficient for a per-resolution
  assertion. The accessor is the whole of the permitted public-surface growth;
  anything wider, including injecting the probe behind a port the resolver
  holds, is **out of scope** and becomes its own work item.

  **Resolved 2026-08-12, against the default.** `SEQUENCE` cannot count
  invocations (see the retraction in the summary above). The counter is a new
  thread-local `PROBE_ATTEMPTS` incremented as the first statement of
  `verify_writable`, read through a single `pub fn probe_attempts()`. Public
  surface growth is still one function, as the default permitted.
- Does a seam for injecting a re-verification failure already exist in
  `cli/launcher/tests/resolution.rs`? The stubbed fetcher and the read-only-root
  fixtures do; a failing *verifier* is assumed by two criteria above and has not
  been confirmed. If it does not exist, building it is a prerequisite inside
  this item.

  **Resolved 2026-08-12.** A seam exists in a different shape, and no verifier
  port is built. `resolution.rs` already produces both refetch outcomes by
  poisoning the cached *bytes*, which works because the expected sha256 is
  parsed out of the cache filename rather than recomputed. Byte poisoning is
  not "filesystem permissions on the cache root", so it discharges what
  acceptance criteria 3 and 4 ask for; a validator reading those criteria
  literally should look for byte poisoning, not for a failing verifier.

- **Is `G ≤ 1.3 × B` the right threshold, given that it is not met?**
  **Reopened 2026-08-17** by the measurement — see [The ratio threshold is
  reopened](#the-ratio-threshold-is-reopened-2026-08-17) for the evidence and
  the four dispositions. **Default if unresolved**: none. The threshold stays at
  1.3 and this item stays open; criterion 10 is unticked and the obligation is
  live. No default is offered deliberately — every disposition changes either
  the shipped code, the criterion, or the closure terms, and none of those is a
  measurement's call.
- **Should the re-derived drift band be the gate?** **Answered 2026-08-17 as to
  the number, open as to its adoption.** The band is re-derived from attempt 2's
  own null at 0.00615 for a stated 5% false-positive rate, replacing a constant
  whose rate was an unstated ~11%; the harness computes it per session. Attempt 2
  drifted at `p = 0.0050` under either, so no verdict turns on it. **Default if
  unresolved**: the derived band stands as the computed diagnostic and the
  session stays invalidated. See [The drift band,
  re-derived](#the-drift-band-re-derived-2026-08-17).

## Dependencies

- **Delivered by 0169.** 0169's Phase 5 implemented this item's original
  Requirements in full, pulled forward because its own Phase 10 warm-call
  latency gate could not be measured while every warm `vcs guard` dispatch paid
  the probe cost.
- **This item now owns the deferred latency measurement.** 0169 is closed
  (`status: done`) with its Phase 10 criterion unchecked and B, G, ratio,
  payload, fixture and host all recorded as pending, so no other open item
  carries it. Taking it requires the launcher resolving against a real,
  minisign-signed `accelerator-vcs` release asset, which does not exist
  pre-release; that release cut and its signing key are owned by whoever
  performs epic-0136 releases. **This item cannot close before that release.**

  **Retracted 2026-08-13.** The asset exists and the release blocker is
  discharged. `v1.24.0-pre.35` and `v1.24.0-pre.36` both ship
  `accelerator-vcs-darwin-arm64` with its `.minisig`, and `pre.36`'s signed
  `manifest.json` carries `vcs` entries for all four platforms at
  `schema_version: 1`; 0205 dispatched the real bootstrap → launcher →
  sub-binary path with no dev override.

  **The blocker is replaced by an outcome-keyed closure guard.** This item may
  not close while any applicable gating cell C1-C5 of [Latency
  Criterion](#latency-criterion) selects a branch other than 1, absent a
  recorded, owner-named acceptance. The guard is keyed on the measured
  **outcome**, not on the figures being recorded — keying it on the recording
  would make it born discharged, since recording the figures is itself a step of
  the closing plan. The prerequisite the measurement does still carry is a
  published, minisign-signed release for the tree's *own* version, since
  `bin/accelerator:138-141` derives the release URL from
  `.claude-plugin/plugin.json`.
- **The latency gate has co-requisites beyond this item.** 0169's hand-off notes
  identify 0191 (batching the two verify-shim hashes into one invocation, ~2.5
  ms — "essentially this story's whole shortfall") as the cheapest remaining
  lever, alongside the backend-dependent `sha256_file` residual 0186
  deliberately retained. `G ≤ 1.1 × B` may not be reachable without 0191.

  **Retracted 2026-08-13.** 0191 was never sufficient, and under the reframed
  criterion it is not a co-requisite at all. On this host 0191 buys a **measured
  2.48 ms** on the fast digest backend (its own 7.05 ms for two substitutions
  against 4.57 ms batched) against 0205's measured overrun of **5.98 ms** — less
  than half of it. Under [Latency Criterion](#latency-criterion) the gate is an
  absolute budget with the ratio at 1.3, which 0205's figures already satisfy,
  so no lever is required to reach it. 0191 keeps its own merits, whose case now
  rests on the fallback backend.
- **Relates to 0186**, which established the pattern, the diagnostic shape and
  the measurement method on the shell side.
- **Relates to 0164**, which established the fetch-verify-cache resolver and
  the probe itself. The at-most-once property is a refinement of the
  cache-resolution contract 0164 defined.
- **Blocks: none.** 0169 and 0186 both still carry prose naming the launcher
  probe as the dominant unaddressed cost gating 0169's latency threshold. That
  framing is stale — 0169's own Phase 5 absorbed the fix — and nothing now waits
  on this item.
- **Parent**: epic 0136.

**The `blocked_by: ["work-item:0169"]` edge is satisfied, 2026-08-13.** 0169 is
`status: done`. The edge is retained rather than deleted, as the historical
record of what gated this item.

## Assumptions

- A launcher process serves a single dispatch and performs exactly one
  resolution, so per-process and per-resolution probe counts coincide in
  production. The criteria assert a per-resolution delta regardless, which holds
  in a multi-resolution test process too.
- 0169's Phase 10 definition of the gate (`G ≤ 1.1 × B`, one darwin host, one
  session) is still the right shape for the measurement inherited here. If the
  epic has since revised the threshold, this item follows the epic.

  **Retracted 2026-08-13.** This assumption's own escape clause has fired. `G ≤
  1.1 × B` is not the right shape: it was measured at 1.2813 and it is
  calibrated against a `B` cost model that does not hold. The threshold has been
  revised, and the revision was landed **on this item** rather than on 0169
  (which is closed) — see [Latency Criterion](#latency-criterion). The "one
  darwin host, one session" half of the assumption stands.

## Technical Notes

- Production call site to protect:
  `cli/launcher/src/launch/outbound/resolve/mod.rs:141`.
- The four unit tests bound to the doomed `cache_root::resolve`:
  `unset_plugin_root_with_no_override_is_a_named_error`,
  `a_writable_plugin_root_is_used`,
  `a_read_only_plugin_root_with_no_override_is_a_named_error`, and
  `an_override_is_honoured`. The read-only case is substantially a
  `verify_writable` test and already has a direct equivalent in
  `verify_writable_rejects_a_read_only_directory`.
- `verify_writable` delegates to `probe_writable_and_executable`, one call to
  one increment, which carries the per-process `SEQUENCE` atomic alongside the
  PID in the probe filename — the atomic exists because the concurrent-first-use
  tests resolve from multiple threads and would otherwise collide on one path.
  That atomic is the counter the default seam exposes.

  **Retracted 2026-08-12.** The last sentence is false: `SEQUENCE` is not the
  counter exposed, because its increment sits after the `create_dir_all` early
  return. It keeps its filename-uniqueness role, unchanged and untouched, and a
  separate thread-local `PROBE_ATTEMPTS` carries the invocation count.
- The integration tests in `cli/launcher/tests/resolution.rs` are a separate
  crate, so the counter accessor must be public — the cost to weigh when
  settling the Open Question.

## Drafting Notes

- The original before/after measurement criterion was dropped, then restored on
  the author's instruction after review pass 2 established that 0169 is closed
  with its Phase 10 gate unmeasured, leaving the obligation with no open owner.
  It returns in 0169's own form (`G ≤ 1.1 × B`) rather than the original
  `after ≤ 0.5 × before`, because the pre-fix "before" no longer exists in the
  tree. The consequence, accepted deliberately: this item cannot close until the
  epic-0136 release cut produces a signed `accelerator-vcs` asset.

  **Retracted 2026-08-13, twice over.** The release cut has happened —
  `v1.24.0-pre.35` and `pre.36` both ship the signed asset — so the stated
  consequence no longer holds. And the criterion no longer returns in 0169's
  form: `G ≤ 1.1 × B` was measured at 1.2813 and is superseded by the six-cell
  definition in [Latency Criterion](#latency-criterion), in which the ratio
  survives only as a historical comparison beneath an absolute budget.
- The title still names only the probe guarantee, not the inherited latency
  measurement. It was already changed twice — once because the original asserted
  something no longer true of the code, once from "Once-Per-Dispatch" to
  "At-Most-Once" because the dominant production case is zero probes — and was
  left alone this time rather than churned again. The filename slug still reads
  `once-per-dispatch`.
- "At most once per process" was reinterpreted as a per-resolution delta. The
  two coincide in production, and a delta is the only form observable through a
  process-wide counter in a multi-resolution test process.
- Priority stays low. The guard work is small, and the measurement half is
  release-gated rather than urgent — but the item now carries an epic-level
  obligation, so raising it is a reasonable challenge.

  **Retracted 2026-08-13.** The measurement half is not release-gated; the
  signed asset ships in `v1.24.0-pre.35` and `pre.36`. The priority is left at
  low regardless, on the unchanged ground that the work is small.
- Memoisation was ruled out in favour of a test on the author's instruction, and
  carries its own criterion because every *other* per-path count criterion
  passes under a memoising implementation.
- The stale "dominant unaddressed cost" framing inside 0169 and 0186 was
  retracted in place on 2026-08-11. Both retractions are dated notes appended
  beside the original text rather than edits to it, so each document still
  records what was believed when it was written.

## References

- `meta/reviews/work/0189-once-per-dispatch-cache-root-probe-guarantee-review-1.md`
  — five-lens review, verdict REVISE across two passes, which drove this revision
- `meta/plans/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`
  and its validation report — the sibling plan that discharges criteria 1-9 and
  12
- `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md` — the plan
  that lands the reframed criterion and takes the measurement
- `meta/work/0205-close-the-warm-dispatch-measurement-method.md` — the spike
  that closed the measurement method and ran it at n = 300, source of every
  measured figure in Latency Criterion
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — delivered the split;
  closed with the Phase 10 latency gate unmeasured
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Phase 5,
  and Phase 10's deferred latency gate and hand-off record
- `meta/work/0191-batch-the-two-shim-hashes-into-one-invocation.md` —
  co-requisite for the latency gate
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — the
  shell-side change, its measurement method and its diagnostic shape
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md`
- `cli/launcher/src/launch/outbound/resolve/cache_root.rs`
- `cli/launcher/src/launch/outbound/resolve/mod.rs` —
  `FetchVerifyCacheResolver::fetch_verify_store`
- `cli/launcher/tests/resolution.rs` — the two already-satisfied criteria
- `docs/internals.md` — "Offline, mirrored and read-only installs"
