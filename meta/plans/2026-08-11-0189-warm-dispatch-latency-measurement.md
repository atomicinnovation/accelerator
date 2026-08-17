---
type: plan
id: "2026-08-11-0189-warm-dispatch-latency-measurement"
title: "Warm-Dispatch Latency Measurement Implementation Plan"
date: "2026-08-11T19:43:42+00:00"
author: "Toby Clemson"
producer: create-plan
status: done
work_item_id: "work-item:0189"
parent: "work-item:0189"
derived_from:
  ["codebase-research:2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee"]
relates_to:
  ["plan:2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee",
   "work-item:0205", "work-item:0169", "work-item:0186", "work-item:0188",
   "work-item:0191", "work-item:0199"]
tags: [cli, launcher, performance, bootstrap, measurement]
revision: "18042973ddd816622577925948c3db142852ffb9"
repository: "accelerator"
last_updated: "2026-08-17T13:30:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Warm-Dispatch Latency Measurement Implementation Plan

## Overview

Close 0189 by reframing the warm-dispatch latency criterion it inherited from
0169, confirming the reframed criterion by measurement on a quiet host, and
discharging the recording obligations orphaned across 0169, 0189 and 0191.

**The measurement has already been taken.** Work item 0205 was raised to close
the *method*; it closed the method and then ran it, at n = 300 with a
pre-registered taxonomy and full provenance. Its headline: `G ≤ 1.1 × B`
**fails**, ratio of medians **1.2813**, two-sided 95% paired-bootstrap CI
**[1.2662, 1.2899]**, `P(ratio > 1.1) = 1.0000`, an overrun of 5.98 ms against a
36.30 ms ceiling. No sampling choice moves a point estimate of 1.28 to 1.10.

So this plan is no longer a measurement waiting on a method. It is a
**disposition**: the inherited threshold was calibrated against a baseline whose
cost model 0169 misread, and against a baseline that does structurally less work
than the variant it gates.

The criterion is restructured into a **re-runnable absolute budget as the
primary gate, with the ratio retained as the historical comparison** that
discharges 0169's inherited wording. It is defined once, normatively, in [The
Criterion](#the-criterion) below — six identified cells, their statistics, their
sizing and the taxonomy each is classified by. Every later phase refers to cells
by identifier rather than restating values, and 0189 carries the criterion as
landed; this plan's copy is the working definition until Phase 1 lands it, and a
restatement thereafter.

The whole set is then confirmed on a quiet host under a committed harness so
0189's record is its own rather than borrowed from a spike.

This is the second plan against work item 0189. The first,
`meta/plans/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md`, is
`status: done` and validated `pass` — it delivered the at-most-once probe
guarantee and discharges every one of 0189's acceptance criteria except the
latency pair. The two plans share no code, no test and no source file.

## The Criterion

**This section is normative.** Every other section — Overview, Desired End
State, Phase 1, Phase 3, Phase 4, the Validation Results slots — refers to cells
by identifier and must not restate their values. Phase 1 lands this section's
content on 0189, which then becomes authoritative; the copy here is the working
definition until then and a restatement afterwards. The harness holds the same
constants in one place and a lockstep guard binds them (Phase 2).

### Cells

| ID | Statistic | Backend | Ceiling | Gates | Base figure | Headroom |
| --- | --- | --- | --- | --- | --- | --- |
| **C1** | `median(G)` | fast | ≤ 50 ms | yes | 42.28 ms (0205, measured) | +18.3% |
| **C2** | `p90(G)` | fast | ≤ 60 ms | yes | 46.51 ms (0205, measured) | +29.0% |
| **C3** | `median(G)` | fallback | ≤ 70 ms | yes | ~59.2 ms (predicted) | +18.2% |
| **C4** | `p90(G)` | fallback | ≤ 80 ms | yes | ~63.4 ms (predicted) | +26.2% |
| **C5** | `median(G) / median(B)` | fast | ≤ 1.4 | yes | 1.3423 (attempt 2, measured) | +4.1% |
| **C6** | `median(G) / median(B)` | fallback | recorded | **no** | ~1.79 (predicted) | — |

**C1-C4 are the primary gate** and the only re-runnable cells: `B` is a deleted
artefact recovered from `cf42441e2aad-`, so no lane can ever reproduce C5 or C6,
whereas an absolute ceiling can — and it bounds what users actually feel on
every Bash tool call. **C5 is the historical comparison** that discharges 0169's
inherited ratio wording. C6 is context only, because a ratio against a baseline
that hashes nothing is least meaningful where `G` hashes most.

Every derivation above is re-checkable. C1-C3's bases are 0205's published
figures; **C4's base is fast p90 46.51 ms plus the predicted ~16.9 ms backend
delta = 63.4 ms** (SD-1). C3 and C4 rest on a cross-session import of 0186's
per-call pair, which §7 declines to rely on for the composition budget — so **C3
and C4 are provisional on first measurement**: the first in-session fallback
figures become the bases any future re-run is gated against, and that is stated
in the criterion's limitations.

⚠️ The ceilings are round numbers rather than tuned constants, deliberately: a
ceiling fitted to three significant figures against one session's dispersion
would be a gate calibrated to noise.

### Statistics, by cell kind

The two kinds take different estimators (SD-2).

**C1-C4 — absolute.** An **unpaired** percentile bootstrap on the single
variant's statistic; a paired bootstrap over `(B, G)` pairs is not the estimator
for a single-variant quantity. Target **upper distance** — not half-width — of
**1.0 ms** on the medians and **2.0 ms** on the p90s, consistent with §Sizing's
rule for every cell. Latency distributions are right-skewed, so an unpaired
bootstrap on a median or p90 is asymmetric for the same reason 0205's ratio
interval was, and only the upper tail can breach a ceiling. Acceptance is the
interval's **upper bound at or below the ceiling**. There is **no
floor-subtraction robustness clause**: subtracting a shared spawn floor from an
absolute median makes it *smaller*, so the clause would be strictly weaker than
the primary test rather than a check on it.

**C5 — ratio.** A **paired** percentile bootstrap on the ratio of medians over
interleaved pairs, seeded, at ≥ 10,000 resamples. Two conditions, and both must
hold:

1. **Gate** — the raw-median interval's **upper bound** ≤ 1.4.
2. **Robustness** — the `true`-floor-subtracted ratio's **point estimate** ≤
   1.4, with its interval recorded as context.

⚠️ **The threshold is 1.4 as of 2026-08-17**, raised from 1.3 by author decision
after two sessions measured 1.3177 and 1.3423. 0189's Latency Criterion is
authoritative; this restatement follows it. The point-estimate form of condition
2 has lost its justification at 1.4 — see that section.

⚠️ The robustness condition is a **point-estimate** test, and that is a
deliberate, pre-registered weakening with a stated reason. Its margin is 0.003
(1.297 against 1.3), while the upper distance achievable at any practical n is
larger: 0.0036 at n = 1,700, 0.0027 at n = 3,000, and ~0.001 only at n ≈ 22,000
(~39 minutes). An upper-bound form would therefore be undecidable at every
sample size this plan can afford, so branch 1 would be unattainable and the
expected outcome would be branch 3 by construction. The point-estimate form
keeps the check meaningful — it still fails if floor treatment flips the verdict
— at the cost of not bounding its own sampling error, which is recorded.

**Floor treatment.** Three ratios are computed and recorded, in three fixed
roles: raw medians **gate** (C5.1); `true`-floor-subtracted is the **robustness
check** (C5.2); bash-floor-subtracted is **diagnostic only**, because it
over-subtracts — bash interpreter startup is real cost `G` pays, since
`bin/accelerator` *is* a bash script. Raw medians are the **lenient** statistic
for a `ratio ≤ k` gate, since `(G−c)/(B−c) > G/B` (SD-3).

### Sizing

The sizing rule is `n = n₀ × (h₀ / target)²`, and `h₀` and `target` are both the
interval's **upper distance** `U − point estimate`, not its half-width. 0205's
interval is materially asymmetric — `[1.2662, 1.2899]` around 1.2813 is 0.0151
below and 0.0086 above — so the symmetric half-width 0.0119 corresponds to
neither tail, and for a `ratio ≤ k` gate only the upper tail can decide
anything. `h₀ = 0.0086`.

| Block | Samples | Arms | n | Achieved upper distance | Wall clock |
| --- | --- | --- | --- | --- | --- |
| **A** | interleaved `(B, G-fast)` pairs | 2 | 1,700 | ~0.0036 on C5 | ~3.0 min |
| **B** | `G-fallback` alone | 1 | 900 | ~1 ms on C3, ~2 ms on C4 | ~1.3 min |

Block B needs no `B` samples: C3 and C4 are absolute, and C6 is not gated, so
its `B` pairing would be unused work. Total ~4.3 minutes of sampling, ~6 minutes
including probes, floors and provenance. C5's margin is 0.0187 against an
achieved upper distance of ~0.0036 — **5.2 upper-distances**, which is what
makes 1.3 decidable and is why the threshold is the floor of the 1.3–1.5 band
rather than its middle.

⚠️ Both `h₀` and the expected-pass reasoning come from 0205's **two-arm**
sequence, and Block B inserts a workload that hashes a further ~10.7 MB per
sample. Block A and Block B are therefore run as **separate interleaved blocks**
rather than one four-arm rotation, so Block B's load does not enter the pairs C5
is computed from. **Re-derive `h₀` from the first 200 pairs of Block A — and the
first 200 samples of Block B — as an in-session pilot**, and confirm each tabled
n still reaches its target before committing to it. Block B's n = 900 comes from
that pilot rather than from a published figure, since 0205 measured no fallback
dispersion.

Four things are pre-registered about the pilot so it does not become a second
route to a data-dependent sample size beside the prohibition in §6: it is a
**dispersion estimate only** and its samples are **discarded**, not pooled, so
the final interval's coverage is unaffected; a size-up recomputes n from the
**same** targets (0.0036 / 1.0 ms / 2.0 ms), never from a relaxed one; it does
**not** consume `escalations_used`; and it is bounded by the same 6,900 / 3,600
caps and the 35-minute budget.

### Provenance of the band

The 1.3–1.5 band is an **author instruction given in conversation on
2026-08-13**, not a corpus figure — nothing in `meta/` states it, and 0205 names
no numeric band. Phase 1 records that provenance and its approver on 0189
alongside the criterion, because the stated mitigation for a post-hoc relaxation
is "the floor of the band was taken", and that mitigation is unauditable unless
the band's origin is on the record.

## Current State Analysis

**The measurement is not release-blocked, and never was after `pre.35`.** 0189's
Dependencies section states the item cannot close before an epic-0136 release
cut produces a signed `accelerator-vcs` asset. Both `v1.24.0-pre.36` and
`v1.24.0-pre.35` ship `accelerator-vcs-darwin-arm64` alongside its `.minisig`,
and `pre.36`'s signed `manifest.json` carries `vcs` entries for all four
platforms at `schema_version: 1`. 0205 measured the real bootstrap → launcher →
sub-binary path with no dev override, which settles the premise empirically
rather than by inspection.

**0189's acceptance criteria are all still unchecked.** Twelve boxes at
`:162-203`, none ticked, despite the sibling plan being `done` and its
validation report recording `mise run` green with every criterion but the
latency pair discharged. The sibling validation's own recommendations
(`:151-161`) are three and different — correct the Mutation A cell, demote an
intra-doc link, and close 0189 only after this measurement plan lands. The
**checkbox-reconciliation** recommendation is the *0169* validation's
(`meta/validations/2026-08-05-0169-…:196-199`) and is aimed at 0169's criteria;
(SD-4.) 0169's own reconciliation is therefore a live, separately recommended
obligation that this plan does **not** take on beyond the four latency locations
Phase 4 discharges — stated so the omission is a recorded scope decision rather
than a gap.

**`B` does not do what the previous draft of this plan said it does.** This
plan's own Current State Analysis previously attributed `B`'s cost to
`classify_checkout` (`scripts/vcs-common.sh:177`) spawning `jj workspace root`,
two `git rev-parse` forms, `realpath` and `jq`, and carried 0188's 23.84 ms `jj
log` figure as an upper bound on "the guard's `jj` spawn". That is wrong, and it
is verified wrong: the recovered guard's line 19 reads
`REPO_ROOT=$(find_repo_root)`, and `find_repo_root`
(`scripts/vcs-common.sh:8-18`) is a pure-bash loop testing `-e "$dir/.jj"` and
`-e "$dir/.git"` upward, spawning only `dirname` per path level. Mode is then
decided by two literal `[ -d ]` tests. **There is no `jj` spawn and no `git`
spawn anywhere in `B`.** Its cost is bash startup plus roughly fifteen
`jq`/`grep`/`sed`/`awk`/`cat`/`timeout` pipeline spawns.

Two consequences follow. The fixture-choice rationale that cited "tens of
milliseconds" from a ~24 ms `jj` spawn does not hold — fixture depth still moves
the `dirname` loop's spawn count, at roughly 1 ms per spawn. The pinned
non-colocated fixture stays, for the independently sufficient reason that a
colocated fixture emits **warn** rather than the blocked decision, so the
harness would silently measure the wrong path.

**`B` and `G` do not perform comparable work.** `B` decides pure-jj versus
colocated by testing for two directory entries. `G` loads the repository through
jj-lib. The Rust guard is not a faster reimplementation of the shell guard; it
is a more correct one, and a ratio gate calibrated against the cheaper behaviour
charges it for that correctness. 0186's own method-incomparability note
(`meta/work/0186-…:615`) already flagged that its 35.1 ms shell-guard row was
not comparable; this is a second, independent reason the two sides are not
like-for-like.

**The digest backend moves the ratio by more than any threshold in the
acceptable band.** `bin/accelerator:272-278` selects `sha256sum` when present
and otherwise falls back to `shasum -a 256`. Both `sha256_file` calls run on
every warm dispatch — `shim_digest=$(sha256_file "${shim_source}")`
unconditionally at `:291`, then `$(sha256_file "${shim}")` inside the staging
condition at `:295`. `B` hashes nothing, so the whole backend delta lands on
`G`:

| Host | two `sha256_file` calls | `G` | ratio against `B` = 33.00 |
| --- | --- | --- | --- |
| `/sbin/sha256sum` resolves | ~7 ms | 42.28 | **1.28** |
| Perl `shasum -a 256` only | ~24 ms | ~59.2 (predicted) | **~1.79** (predicted) |

Per-call figures are 0186's delivered pair, 3.55 ms against 11.99 ms
(`meta/work/0186-…:553-563`), which 0186 explicitly declined to call a platform
fact: `/sbin/sha256sum` is present on this host and absent from a stock macOS
image. A criterion binding both backends therefore fails at **any** threshold in
the 1.3–1.5 band. The reframed criterion binds the fast backend; the shasum
figure is measured and recorded as context, and its cost is routed to 0191.

### Key Discoveries:

- **0205's load model is not supported by its own three points.** It claims the
  ratio rises as host load falls, and sets a revisit trigger at "a quiet-host
  run returning a ratio below 1.20". Sorted by load: 11 → **1.16**, 19 → 1.2813,
  38 → 1.2320. That is non-monotone, and the lowest-load run already returned
  1.16, so the trigger has already fired on the spike's own data. The direction
  of the load bias is **unknown**, not established. Phase 3 therefore
  pre-registers a taxonomy rather than an expectation, and records load with the
  instrument floors so quietness is evidenced two ways.
- **Dropping the cache-hit `sha256` does not weaken the trust boundary.**
  `cache::find` (`cache.rs:51-73`) takes `sha256` from the entry's own filename
  via `file.strip_prefix(&prefix)`; `reverify` (`mod.rs:90-109`) passes that
  name-derived digest into `verify_binary`, which compares it and **then** calls
  `keys.verifies(bytes, signature)` — Ed25519 over minisign's BLAKE2b of the
  same bytes (`keys.rs:62-69`). `verifier.rs:1-2` names minisign "the security
  boundary" and sha256 the "corruption check" outright. What removal costs is a
  distinct `ChecksumMismatch` diagnostic, not provenance. 0191 likewise still
  computes and compares both digests. The optimisation route is declined on the
  ground 0205 itself argues — verification posture should not be set by an
  arithmetic target — not on a security ground that does not hold.
- `swallow_under_fail_safe` (`core.rs:219-224`) swallows only
  `kernel::Error::Failed`, and `handle_dispatch_error` (`main.rs:215-226`) then
  exits 0 without ever exec'ing the sub-binary. A degraded sample skips both
  `reverify` and the sub-binary run and records a spuriously *low* latency. This
  is the failure the per-sample validity gate exists to catch.
- **The dual-shape envelope mapping is in Rust, not Python.** The shell guard
  emits the legacy `{"decision":"block","reason":…}` shape; the Rust guard emits
  `hookSpecificOutput.permissionDecision` (`kernel/src/hooks.rs:29-31`).
  `cli/vcs-cli/tests/guard_decision_table.rs:99-141` maps
  `permissionDecisionReason` onto the label **`block`** and is the mapping to
  reuse. `hooks/test-fixtures/vcs-guard/generate_decision_table.py:160`
  `normalise()` is **shell-only** — its docstring says so, and it silently maps
  the Rust deny envelope to `("allow","")`, the same value an empty stdout from
  a swallowed fail-safe produces. It must not be used for this comparison.
- The two guards' reason strings are byte-identical
  (`cli/vcs-cli/src/guard.rs:19-20` against the golden `decision-table.json`),
  so pinning expected reason text across variants is achievable once the
  decision label is mapped.
- **No VCS gate can see anything this measurement creates.** The scratch tree
  and fixture live outside the repository entirely; the cache-root artefacts the
  bootstrap touches are gitignored (`.gitignore:45-56`), and the shell linter's
  tree walk honours `.gitignore` via `tasks/shared/sources.py`. Such artefacts
  are invisible to jj's auto-snapshot **and** to any `jj diff`/`jj status`
  cleanup gate, so cleanup is asserted positively by recorded resolved path,
  never by absence of a VCS diff.
- The dev-launcher marker is plugin-root-relative
  (`dev_launcher_marker="${plugin_root}/.accelerator-dev-launcher"`,
  `bin/accelerator:225`), not root-absolute.
- `tasks/` is a flat set of invoke modules collected in `tasks/__init__.py`,
  with `tests/unit/tasks/test_python_coverage.py` guarding that ruff and pyrefly
  actually reach every in-scope `.py`, and `test_mise.py` carrying the precedent
  for keeping a namespace out of the aggregate `check` and the bare default
  (`test_docs_tasks_stay_out_of_default_and_aggregate_check`).

## Desired End State

0189's latency criterion carries the cell table from The Criterion — an absolute
`median(G)`/`p90(G)` budget per digest backend as the primary, re-runnable gate,
and `G ≤ 1.3 × B` on the fast backend as the historical comparison — with its
reframing rationale recorded on 0189 itself; a confirmatory measurement taken on
a quiet darwin-arm64 host in one session under a committed harness satisfies it;
every stale release-gated premise and every surviving assertion of the
superseded threshold across 0189, 0169 and 0191 is retracted as a dated note
beside the text that asserted it; 0169's four record locations carry the figures
with the superseded threshold named; 0189's twelve acceptance criteria are
reconciled and the item closes; and the five follow-ups the evidence raises
exist as work items.

Verified by: the Validation Results section below is complete, every throwaway
artefact is positively asserted absent, `mise run` exits 0, and the repo's own
corpus frontmatter gate is green over every meta document touched.

**The threshold is reframed post-hoc, and that is recorded as a deviation.** The
plan pre-registered against exactly this and is breaking that commitment (SD-5):
the threshold is relaxed from 1.1 to 1.3 after seeing 1.2813. Three things bound
the damage.

The structural argument for reframing — `B` performs two directory-entry tests
where `G` performs a jj-lib repository load — is independent of the measured
value and would hold at any ratio. The absolute argument is likewise
value-independent in kind: a 5.98 ms overrun on a hook is imperceptible, and
42.28 ms for a fully signature-verified, jj-lib-backed guard against 33.00 ms
for an unverified stat-and-grep script is a 9.28 ms premium for the whole trust
chain. And **the threshold is the floor of the stated band, not a point chosen
inside it** — 1.3 clears the point estimate by 5.2 upper-distances at the sizing
The Criterion states, so taking the floor and paying for the precision is
strictly less discretionary than taking the band's middle and citing imprecision
(SD-6, SD-7).

⚠️ Even so, this is not pre-registration and must not be recorded as such. A
threshold set at 1.3 after seeing 1.2813 is a threshold that the observed value
informed, and the margin — 0.0187 — is small enough that a materially different
quiet-host ratio could fail it. That is the intended behaviour of a gate; it is
recorded here so a pass is not mistaken for a comfortable one.

**Both digest backends are gated, and the gate is verified on darwin-arm64
only.** Of the four shipped platforms **darwin-x64 and linux-arm64 are exercised
by no CI lane at all** — `.github/workflows/main.yml` matrixes `ubuntu-latest`
and `macos-latest` alone. That scope is inherited from 0169 rather than chosen
here; the linux measurement is a named hand-off, and 0205 established that
nothing in its findings transfers off darwin-arm64 — the sha256-versus-BLAKE2b
inversion is a property of this chip and this crate build.

⚠️ Neither ratio cell is reproducible on `macos-latest`, since that runner
resolves no `sha256sum`, and `B` cannot be reproduced on any CI lane at all. The
**absolute** cells are the ones a future lane could enforce, which is the reason
they are primary. Record `command -v sha256sum` on both CI lanes — 0186 already
noted this costs nothing — so the fast-backend population is a known number
rather than an assumption.

**The `B`/`G` work asymmetry is accommodated, not resolved.** A ratio gate is
only fully meaningful between variants doing the same work. Constructing a
baseline that also performs a real classification would make the ratio
defensible on its own terms; this plan demotes the ratio to a historical
comparison instead, and records the asymmetry as a stated limitation of that
comparison rather than a property of the implementation.

## What We're NOT Doing

- **No methodology work.** 0205 answered SQ-1 to SQ-5 against measurement. This
  plan carries the method concretely so it is executable standalone, and cites
  0205 for the derivation rather than re-deriving it.
- **No shipping optimisations to reach a threshold.** The cache-hit `sha256`
  removal (−4.49 ms) and 0191 (−2.48 ms measured on the fast backend) together
  reach 1.070, and that route is declined: it sets the launcher's verification
  posture by an arithmetic target. Both are raised as work items on their own
  merits in Phase 4.
- **No like-for-like baseline construction.** Named as a limitation above.
- **No mutation of `keys/accelerator-release.pub`.** `bin/accelerator:165` reads
  it and `cli/launcher/build.rs:31-32,:43` embeds it, so the bootstrap's
  verification key and the launcher's embedded key are one artefact; `mise run
  keys:generate` force-overwrites it. No route here touches it.
- **No dev-override route.** 0205 established the decomposition is reachable
  through the launcher's public library surface with nothing signed locally and
  no marker, `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER` or
  `ACCELERATOR_LAUNCHER_BIN` set.
- **No linux measurement, and no darwin-x64 measurement.** Both are the
  hand-off.
- **No `measure:*` task in the aggregate `check` or the bare default.** The
  harness needs a quiet host, several minutes and network egress.

## Implementation Approach

Four phases. The dependency graph is a chain with two independent roots, not two
independent halves:

```
Phase 1 (documents) ─┐
                     ├─> Phase 3 (one recorded session) ─> Phase 4 (discharge)
Phase 2 (harness) ───┘                                          ^
         Phase 1 ─────────────────────────────────────────────┘
```

`1 → 3`, `2 → 3`, `{1, 3} → 4`. Phase 3 depends on Phase 1 as well as Phase 2:
every classifier branch is expressed against the threshold Phase 1 lands, so
running the measurement first would reintroduce the post-hoc problem this plan
works to bound (SD-8).

**Phases 1, 2 and 4 are independently mergeable; Phase 3 is not a merge.** It
produces no reviewable change beyond this plan's own Validation Results — it is
a one-shot operator session on a quiet host, and calling it "mergeable"
alongside the others obscures that.

Phase 1 is documents only — it makes the corpus stop asserting things now known
false, and lands the criterion the measurement will be judged against **before**
that measurement runs. Phase 2 is code with tests and no measurement. Phase 3
runs once and records. Phase 4 discharges.

⚠️ Phase 1's value does not depend on Phase 3's outcome. The stale release-gated
premises and the superseded threshold are known false today, and 0205's closure
is owed regardless — so a failed, indeterminate or invalidated session must not
strand them. If the phases are ever split across changes, Phase 1 lands first
and alone.

### Pre-flight and teardown

Both live **inside `tasks/measure.py`** as a context manager — captured on
entry, restored and verified on exit, with a signal handler so SIGINT and
SIGTERM unwind it rather than bypassing it. Phase 2 commits them alongside the
harness; nothing about a run depends on a throwaway script. Two reasons: a
committed harness that calls an uncommitted dependency cannot run
self-contained, which defeats the linux hand-off Phase 2 is justified by; and
the uncommitted half is precisely where the safety-critical logic lives.

The paths below are derived the way `bin/accelerator` derives them, never
hard-coded — in particular the unverified log is cache-root-relative,
`${ACCELERATOR_CACHE_DIR:-${plugin_root}/bin}/.accelerator-unverified.log`
(`bin/accelerator:117`, `:216`; `.gitignore:55`), and 0205 recorded it absent
"at the plugin root", which is the wrong directory.

#### The artefact manifest

Written **first, before anything is created**, to a fixed gitignored path
**outside** the launcher's cache root —
`${plugin_root}/.accelerator-measure/manifest.json`, with
`/.accelerator-measure/` added to `.gitignore` in Phase 2 — at a fixed name so
it is findable without the harness.

⚠️ It must **not** live under `bin/` — that is the launcher's live cache root,
whose `.tmp-` namespace `store::TEMP_PREFIX` reserves, whose entry count is a
budget term and whose entry set is an integrity witness (SD-9). It records, as
**one enumerated list** that both teardown phases drive from:

| Artefact | Created by |
| --- | --- |
| the scratch tree `T` (guard + `vcs-common.sh`) | harness, §1 |
| the fixture root | harness, §3 |
| the fast `PATH` farm | harness, §5 |
| the fallback `PATH` farm | harness, §5 |
| the instrument-floor bash script | harness, §2 |
| the advisory concurrent-session marker | harness, §5 |

The manifest itself is **not** a row: it is the interlock token, not a managed
artefact. Its removal is verify step 8, and it is exempt from the containment
assertion and from the absence assertions of verify item 1 and the signal
rehearsals (SD-10).

The containment predicate admits exactly three roots: the harness's temp parent,
the gitignored `bin/.tmp-*` namespace, and the gitignored
`/.accelerator-measure/` directory.

Plus the captured baseline state below. A **`measure:teardown` task** replays
restore and verify from the manifest, and the harness **refuses to start** while
a manifest from an unclean prior run exists (SD-11).

⚠️ Every artefact in that table lives under the harness's own temp parent or the
gitignored `bin/.tmp-*` namespace, and the manifest under the gitignored
`/.accelerator-measure/` — **nothing untracked at the repository root**. An


**Exhaustiveness is a property of the enumeration, not of one directory.** Every
artefact is created through a single `register_artefact` seam, and a unit test
asserts that every creating call site in `tasks/measure.py` appears in the
manifest table — then that **each** recorded parent is empty after teardown. The
stale-manifest start-up refusal and `measure:teardown` each get their own test.

#### Captured on entry

1. `sha256` of the **working-copy** `keys/accelerator-release.pub` — the
   artefact `bin/accelerator:165` reads and `cli/launcher/build.rs` embeds —
   compared immediately against the published constant
   `0f3fe9a91ab6869ce36209691e06c722259e5754f2228b1539ef566b00f6fb2e`. The
   comparison against a published value, not the act of recording a digest, is
   what detects a key substituted *before* the session; recording it is what
   detects one *during*. (SD-12.)
2. Digests of the cached launcher, its `.minisig` and the staged shim.
3. The unverified log's byte count and full contents verbatim, or its absence.
4. The **sorted entry list** of the cache root, so a leaked
   `.tmp-<stem>-<pid>-<seq>` from an interrupted `cache::store`, a second staged
   shim or an orphaned `.accelerator-lock-<platform>` directory is detectable.
5. **`jj diff --summary` over `keys/ bin/ hooks/ scripts/ cli/`**, and per-file
   digests of `bin/accelerator`, `scripts/vcs-common.sh` and `hooks/hooks.json`.
   A **non-empty diff over those paths refuses the run** rather than being
   reported at the end: `scripts/vcs-common.sh` is `B`'s only dependency and
   `bin/accelerator` is `G`'s whole bootstrap, so a pre-existing uncommitted
   edit there — ordinary mid-stack jj state — invalidates the measurement from
   sample one. (SD-13.)
6. The operator's pre-existing dev-launcher state, **recorded and left
   untouched**. The override needs all three of
   `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER`, the marker file and
   `ACCELERATOR_LAUNCHER_BIN` (`bin/accelerator:239-240`), so under the pinned
   environment the marker alone is inert. Requiring its absence would force a
   remove-and-restore whose crash window silently switches the operator's later
   sessions from their dev build to the fetched release binary.
7. The canonical OS temp root, `os.path.realpath`-resolved once, so the restore
   phase's containment guard compares canonical forms on both sides. macOS
   `gettempdir()` returns `/var/folders/…` while a canonicalised recorded path
   is `/private/var/folders/…` — the same `/var → /private/var` symlink the
   fixture section flags for `GIT_CEILING_DIRECTORIES`.

#### Restore phase — runs first, unconditionally

Idempotent, hand-re-runnable from the manifest, and reached from the context
manager's exit on every path including the signal handlers — **SIGINT, SIGTERM
and SIGHUP**. SIGHUP matters: a closed terminal or dropped ssh across a
multi-minute unattended run would otherwise terminate by default and bypass the
context manager entirely. All three are rehearsed in Phase 2.

It removes every artefact in the manifest's table by recorded resolved path,
after asserting each is under the canonical temp root (or the `bin/.tmp-*`
namespace) and was created by this run. Teardown never resolves symlinks and
never runs `find … -delete` or `cp -rL`.

⚠️ SIGKILL, an OOM kill and power loss bypass any handler. The manifest is what
covers them: `mise run measure:teardown` replays restore and verify afterwards,
and the start-up refusal ensures the next run cannot silently adopt the residue
as its baseline.

#### Verify phase — aggregates, never exits on the first failure

1. Every artefact in the manifest's table does not exist, by recorded resolved
   path — the two farms and the floor script included, not just `T` and the
   fixture.
2. The cache root's entry set matches its captured list; any new entry is a
   cleanup failure, with `.tmp-*` and `.accelerator-lock-*` called out.
3. `sha256` of the working-copy public key still matches the published constant.
4. The cached launcher, its `.minisig` and the staged shim match their captured
   digests.
5. `[k for k in env if k.startswith("ACCELERATOR_")] == []` — the **same**
   assertion the entry-side precondition uses, so entry and exit cannot diverge
   — and `${PLUGIN_ROOT}/.accelerator-dev-launcher` matches its recorded state.
6. The unverified log is **byte-identical** to its captured contents. It is
   treated as **append-only**: any appended line is written by `fail_integrity`
   or the dev-override exec (`bin/accelerator:58-68`, `:246-249`) and therefore
   *is* a trust-chain failure or an engaged override, so growth invalidates the
   session under branch 5 and aborts on the first line rather than being
   attributed and tidied. Nothing edits this file. Attribution was never
   reliable — the record format is `timestamp pid=$$ message`, the pid belongs
   to the bootstrap subprocess, and this repo has already been bitten by pid
   reuse.
7. `jj diff --summary keys/ bin/ hooks/ scripts/ cli/` **equals its captured
   value**, and the three per-file digests are unchanged — an equality check, so
   a mid-run edit is distinguishable from pre-existing state.
8. The manifest is removed **once the restore phase's removals have completed**,
   whether or not the integrity assertions passed. Integrity failures are
   recorded and select branch 5; they do **not** hold the interlock closed —
   separating "artefacts removed" from "integrity verified" is what stops an
   unclearable failure wedging the harness shut (SD-14). `mise run
   measure:teardown` remains the hand-run path, and the manifest's location is
   documented in `tasks/README.md` so manual deletion is always available as a
   last resort.

**Any verify-phase failure selects branch 5** (5b if figures already exist), so
a cleanup failure blocks the outcome-keyed closure guard rather than being
recorded as a documented fact beneath a passing verdict. Remediation is concrete
— the removal commands for the `.tmp-*` and lock cases — followed by a
**mandatory re-run of the verify phase**, whose second result is what gets
recorded. (SD-15.)

⚠️ `mise run cli:check` is deliberately **not** in the teardown. A multi-minute
cargo pass inside a `trap … INT TERM` handler invites the operator to Ctrl-C a
second time and kill the handler before anything is restored. It runs as a
separate step on normal completion only.

---

## Phase 1: Reframe the Criterion and Retract the Stale Premises

### Overview

Land the absolute-budget-primary criterion with `G ≤ 1.3 × B` as its historical
comparison on 0189, with the rationale; retract every premise the evidence has
falsified across three work items; reconcile 0189's acceptance criteria against
what the sibling plan delivered; and close 0205. Documents only.

### Changes Required:

#### 1. Replace 0189's latency criterion

**File**: `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`

0189 carries **twelve** acceptance criteria. The latency criterion is the
**tenth** (`:197-199`), the recording criterion the **eleventh** (`:200-202`)
and `mise run` exits 0 the **twelfth** (`:203`); there is no thirteenth. Each is
identified below by quoted opening text as well as ordinal, since an earlier
draft of this plan was off by one and invoked a criterion 13 that does not exist
— following it literally would have ticked the unmeasured latency criterion and
discharged the recording criterion in its place.

Criterion 10, "Warm-call latency G and shell baseline B…", currently reads:

```markdown
- [x] Warm-call latency G and shell baseline B are both recorded from one darwin
      host in one session, with `G ≤ 1.1 × B`. Blocked until a signed
      `accelerator-vcs` release asset exists; see Dependencies.
```

It is replaced by the content of [The Criterion](#the-criterion), transcribed
onto 0189 — which thereby becomes the **authoritative** definition, with this
plan's copy a restatement from that point on. Transcribe, in full and without
re-deriving:

- The **cell table** (C1-C6) with each cell's statistic, backend, ceiling,
  whether it gates, its base figure and its headroom — including that C3 and C4
  are **provisional on first measurement**, since their bases are predictions
  resting on a cross-session import.
- The **statistics by cell kind**: unpaired bootstrap with ms targets for C1-C4
  and no floor-subtraction clause; paired bootstrap for C5 with its two
  conditions, the second a **point-estimate** test with the reason for that form
  stated (an upper-bound form is undecidable at any affordable n).
- The **sizing** — upper distance, not half-width, `h₀ = 0.0086`, the two blocks
  and their n — and the **floor treatment** with the three ratios' three roles.
- The **seven-branch taxonomy** as parameterised per cell kind, and that the
  item closes only when C1-C5 all select branch 1.
- The **superseded `G ≤ 1.1 × B`**, named with its measured 1.2813 and the
  reason it was reframed.
- The **provenance of the 1.3–1.5 band** — an author instruction given in
  conversation on 2026-08-13, with its approver named. Nothing in `meta/` states
  the band and 0205 names no numeric band, so without this the plan's stated
  mitigation for a post-hoc relaxation ("the floor of the band was taken") is
  unauditable.

⚠️ Do not paraphrase the numbers into a second form here or anywhere else — the
cell identifiers exist so every other reference is a pointer (SD-16).

The rationale is recorded on 0189 as its own subsection, stating: the `B`/`G`
work asymmetry, and that the ratio is demoted to a historical comparison because
`B` cannot be reproduced by any CI lane; that 0169 calibrated 1.1 against a `B`
cost model that attributed `jj` and `git` spawns to a guard which makes neither;
the 9.28 ms absolute premium and its imperceptibility on a hook; that the
optimisation route was declined because it sets verification posture by an
arithmetic target **and not** because it weakens the trust boundary, which it
does not; that 1.3 is the **floor** of the stated 1.3–1.5 band, taken by paying
n = 1,700 for the precision that makes it decidable rather than by citing
imprecision to justify the band's middle; and that the reframing is nonetheless
post-hoc, with a margin of 0.0187 that a materially different quiet-host ratio
could fail.

⚠️ 0169's own criterion text is **not** rewritten. It is a closed story and its
document records what was believed when it was written. Phase 4 discharges it
with a dated note pointing at 0189's superseding criterion.

#### 2. Retract 0189's stale premises — two search-derived sets

Each retraction is a dated note beside the original, never a silent rewrite.
**Both sets are derived by search at execution time and the found set
recorded**, rather than transcribed from a list here. Line numbers are omitted
deliberately — the searches are the specification (SD-17).

**Set A — the release gate.** Run over **all four documents**, not 0189 alone:

```bash
rg -n 'release-gated|pre-release|cannot close|release cut|release asset|cannot be started immediately' \
  meta/work/0189-*.md meta/work/0169-*.md \
  meta/plans/2026-08-05-0169-*.md meta/validations/2026-08-05-0169-*.md
```

At the plan's recorded revision this returns **nine matching lines across seven
passages** in 0189 (SD-17). The Requirements bullet's "release-gated (see
Dependencies)" **and** its "the only part of this item that cannot be started
immediately" (`:127-128`); criterion 10's "`accelerator-vcs` release asset
exists" (`:199`); the Dependencies passage's "which does not exist pre-release"
and "**This item cannot close before that release.**" (`:244-246`); Drafting
Notes' "this item cannot close **until** the epic-0136 release cut produces a
signed `accelerator-vcs` asset" (`:305-306`); and Drafting Notes' "release-gated
rather than urgent" (`:317`) — plus the two 0169 documents' passages that §5
previously hard-coded, plus `meta/validations/2026-08-05-0169-…:30` and `:155`,
which both still assert "three manual/release-gated items", one of them the very
measurement this plan's Current State Analysis opens by proving unblocked.

⚠️ Criterion 10 (`:199`) needs no separate note, being replaced wholesale by §1.
Three earlier defects in this step — a pattern that missed "cannot close until",
a miscounted found set, and a hard-coded 0169 list — are recorded at SD-17.

**Set B — the superseded threshold.** `rg -n '1\.1 ×' meta/work/0189-*.md`,
which returns **five** occurrences: Requirements (`:127`), criterion 10 (`:198`,
replaced by §1), the Dependencies co-requisite (`:251`, retracted separately
below), Assumptions (`:269`) and Drafting Notes (`:303`). Replacing criterion 10
alone would leave four live, so the document would emerge with a criterion
reading 1.3 and its Requirements and Assumptions reading 1.1 — the corpus
contradiction this phase exists to remove (SD-18).

The Assumptions bullet needs particular care: it reads "0169's Phase 10
definition of the gate … is still the right shape … If the epic has since
revised the threshold, this item follows the epic", so its own escape clause has
now fired, and the retraction says so.

Both notes follow the dated form 0189 already uses — `**Retracted 2026-08-12.**`
as a paragraph or sub-bullet immediately after the superseded text (see the five
existing notes in that document) — dated to the day the edit lands, so all
retractions across the five documents this plan touches land in one shape rather
than in a format each executor invents.

0189's `blocked_by: ["work-item:0169"]` edge is already satisfied — 0169 is
`done`. The Dependencies bullet is replaced by a closure guard **keyed on the
outcome, not on the recording**: 0189 may not close while the measured result
falls in any branch of Phase 3's taxonomy other than Pass, absent a recorded,
owner-named acceptance. Keying it on "the figures are recorded" would make it
born discharged, since Phase 4 records them.

The co-requisite claim at `:247-251` — that `G ≤ 1.1 × B` "may not be reachable
without 0191" — is retracted with 0205's arithmetic: on this host 0191 buys 2.48
ms against a 5.98 ms overrun, so it was never sufficient; and under the reframed
criterion it is not required at all.

#### 3. Reconcile 0189's acceptance criteria

Ticking criteria **1 to 9**, criterion **11**'s non-latency clauses and
criterion **12** against the sibling plan's delivered, validated state, each
with the discharging evidence named. Criterion **10** and the latency clause of
criterion 11 stay unticked until Phase 4.

Criterion 11's non-latency clauses — the mutation command and output, the crate
search, the old-test → discharging-test mapping and the pick-up confirmation —
are discharged by the sibling plan's Validation Results and its validation
report; criterion 11 is amended to name which plan records which artefact, since
it is now split across two.

Also update each item's body `**Status**:` line in lockstep with its
frontmatter. Every work item in this corpus mirrors status in prose (`0189:23`
reads `**Status**: In Progress`, per `templates/work-item.md:29`), and the
corpus frontmatter gate cannot see the body — so editing frontmatter alone
leaves the document internally contradictory. 0189's `blocked_by:
["work-item:0169"]` edge is **retained** as a historical record of what gated
the item, with a dated note recording that it is satisfied, rather than deleted.

⚠️ The sibling validation records one criterion-6 subtlety: the warm-hit test
was authored under Mutation B, so the "warm-hit stays green under Mutation A"
clause was observed during validation rather than during implementation. It is
green; the tick cites the validation report rather than the plan.

⚠️ The sibling *plan* carries a cell the project has already established is
false: its Validation Results marks `a_warm_hit_never_probes_the_cache_root` as
✗ under Mutation A, which the validation reran green (6 failed / 19 passed over
25 tests) and recommended correcting, on the ground that "leaving a known-false
cell in the evidence record is worse than the gap it was papering over". Since
that document is the discharging evidence cited above, correct the cell as a
dated note in the same pass.

#### 4. Retract 0191's stale shortfall framing

**File**: `meta/work/0191-batch-the-two-shim-hashes-into-one-invocation.md`

Its Context (`:38-40`) states "~2.5 ms is essentially the whole of 0169's ~2.4
ms shortfall". The measured shortfall against the inherited threshold was 5.98
ms, not 2.4 ms, so the framing is retracted as a dated note.

⚠️ Do **not** restate the backend dependence — 0191 already documents it at
`:54-58` under a bold "**The saving is backend-dependent.**" heading, with the
3.55 ms re-measurement at `:50-52`. The retraction **cross-references** that
existing section rather than appending a near-duplicate paragraph to a short
item, so the genuinely new content is not buried in restated material.

The genuinely new content is: the shortfall was 5.98 ms, not 2.4 ms, so 0191 was
never sufficient to reach the inherited threshold; and under the reframed
criterion it is not a latency-gate co-requisite at all. Quote the saving as the
**measured 2.48 ms** on the fast backend (0191's own 7.05 ms for two
substitutions against 4.57 ms batched) — not as "two ~3.55 ms calls into one",
which implies a ~3.55 ms saving. The fallback-backend saving is ~11-12 ms, which
is ~4-5× larger, not "an order of magnitude", and is a **projection**: 0191
flags the batched multi-file form and its missing-file exit semantics as
unconfirmed on `shasum`. Phase 4 attaches the measured figures to the existing
`:54-58` section.

#### 5. Retract the stale release-asset premise across 0169's three documents

Set A above already spans them, so this step is the record of what it returns
there rather than a second hard-coded list. At the recorded revision that is:

- `meta/validations/2026-08-05-0169-…-validation.md` — "requires a published,
  minisign-signed `accelerator-vcs` release asset that does not yet exist", plus
  `:30` and `:155`'s "three manual/release-gated items"
- `meta/plans/2026-08-05-0169-…` — "neither exists pre-release"

Each retracted as a dated note citing the `v1.24.0-pre.36` assets by name.
Leaving them live means the corpus keeps asserting, in three more places across
two documents, the claim this plan's Current State Analysis opens by disproving.

#### 6. Close 0205

⚠️ **Already discharged, 2026-08-13, ahead of this phase.** 0205 is `status:
done` with its body line in lockstep, all eight criteria ticked (7 and 8 citing
its examples-target deviation), the stale `Blocks:` line retracted as a dated
note, and all three corrections appended. When Phase 1 runs, **verify** this
rather than redoing it; the specification below is retained as the record of
what was required.

**File**: `meta/work/0205-close-the-warm-dispatch-measurement-method.md`

`status: draft` → `done`, with the body `**Status**:` line (`:25`, currently
`Draft`) updated in lockstep. Its Spike Outcome, Findings, Recommendation,
Residual Risks and Cleanup Evidence are complete, and its blocking role is
discharged the moment this plan consumes its answers.

**Tick its eight acceptance criteria** (`:175-195`), each against the named
section that discharges it — **except criteria 7 and 8**, which name a
`cli/launcher/examples/` target that never existed. 0205's own Deviations
(`:630-638`) record that the instrumentation was carried by
`cli/launcher/tests/spike_0205_warm_terms.rs` instead, so "the criterion's
substance was honoured" while its literal text was not. Tick those two citing
`:630-638` as the discharging evidence, exactly as criterion 6 on 0189 cites the
validation report. A plain tick would record as satisfied a criterion whose
stated artefact never existed — the checkbox-versus-reality divergence this
plan's Current State Analysis condemns.

Then retract its Dependencies line "Blocks:
`plan:2026-08-11-0189-warm-dispatch-latency-measurement`, which cannot specify
its measurement until SQ-1 to SQ-4 are answered", which is stale the moment the
item closes. Closing an item `done` while its checkbox record says nothing was
delivered is precisely the orphaned-record failure this plan's Current State
Analysis opens by condemning — reproducing it here, in the phase that exists to
end it, would be self-defeating.

**Three** corrections are appended as dated notes rather than edits:

1. The load-model direction is unsupported by its own three points.
2. The security rationale for the optimisation levers overstates what the
   cache-hit `sha256` protects.
3. **The criterion was reframed on 0189**, contrary to 0205's Assumptions
   (`:209`, "is inherited from 0169 and is not reopened here") and its
   Deviations (`:624`, `:628`, "the criterion remains as written until 0169
   changes it"). Name the cell table. Without this note, Phase 1 sets `done` a
   document that asserts in the same pass that the criterion is still 1.1 and
   only 0169 can change it — the orphaned-stale-premise failure this phase
   exists to end, re-created inside it, in the document a reader reaches first
   because it holds the measurement.

⚠️ 0205's Recommendation is explicit that **0169** owns the criterion decision —
"Reopen the 1.1 threshold in 0169 before 0189 measures anything … 0169 decides
what the criterion should be". This plan lands the reframed criterion on 0189
instead, which is defensible for a closed story but is a departure from the
recommendation it otherwise adopts. It is recorded in Deviations, and Phase 4's
dated note on 0169's criterion states plainly that the decision was taken on
0189 contrary to 0205's sequencing — so the criterion's authoritative location
is discoverable without reading both documents in full.

⚠️ The plan/0205 `derived_from` direction is already correct — this plan's
frontmatter names the codebase-research document with `work-item:0205` in
`relates_to`, and 0205 retains its edge to the plan. Nothing to do (SD-19).

### Success Criteria:

#### Automated Verification:

- [x] `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-corpus -E
  'test(this_repositorys_own_corpus_is_clean)'` is green — the unconditional
  repo-corpus frontmatter gate over every meta document edited
- [x] `mise run check` is green

#### Manual Verification:

- [x] 0189's criterion **10** states the reframed threshold, its backend
  binding, its interval-upper-bound rule and the superseded 1.1 with its
  measured 1.2813
- [x] 0189 carries the reframing rationale, including that the optimisation
  route was declined on posture grounds and **not** on trust-boundary grounds
- [x] Both retraction sets are derived by the recorded searches, the found set
  of each is recorded, and every occurrence carries a dated note in the form
  0189 already uses; the release-cut blocker is replaced by an outcome-keyed
  closure guard; the 0191 co-requisite claim is retracted with the arithmetic
- [x] `rg '1\.1 ×' meta/work/0189-*.md` returns no occurrence lacking an
  adjacent dated retraction — 0189 no longer asserts the superseded threshold
  anywhere unqualified
- [x] 0189's criteria **1-9**, **11**'s non-latency clauses and **12** are
  ticked with discharging evidence named; **10** and 11's latency clause remain
  unticked
- [x] The sibling plan's Mutation A / warm-hit cell is corrected as a dated note
- [x] 0191's shortfall framing is retracted, cross-referencing its existing
  `:54-58` backend note rather than restating it
- [x] The stale release-asset premise is retracted in both 0169 documents
- [x] 0205 is `done`, its eight criteria ticked against their discharging
  sections (7 and 8 citing the examples-target deviation), its stale Blocks line
  retracted, and the three corrections appended
- [x] Every body `**Status**:` line matches its frontmatter `status`
- [x] `last_updated`/`last_updated_by` refreshed on every meta document touched

---

## Phase 2: Commit the Measurement Harness

### Overview

Build the harness as a first-class task module so the transcript is a
one-generation cost rather than a recurring one. This is the epic's third
hand-rolled warm-path harness; the linux hand-off then runs a task instead of
retyping a script. No measurement is taken in this phase.

### Changes Required:

#### 1. The analysis core, test-first

**File**: `tasks/measure.py`, with tests in `tests/unit/tasks/test_measure.py`

The harness splits into a pure analysis core and a thin subprocess driver. The
core is where the TDD loop lives; every function below gets a failing test
first:

- **Summary statistics** — n, min, median, p90 and IQR over a sample vector.
- **Paired-bootstrap interval** on the ratio of medians: resamples pairs, not
  variants, at a caller-supplied resample count and confidence level, two-sided,
  returning both bounds. Seeded, so the test is deterministic and the recorded
  run carries its seed.
- **The sizing rule** — `n = n₀ × (h₀ / target)²` over **upper distances**, so a
  target upper distance maps to a sample count from a pilot's observed
  dispersion.
- **Interleaved pair generation** — yields the two variants per pair with order
  alternating between pairs, so batching cannot alias drift onto the difference.
- **Envelope normalisation** to `(decision, reason)`. Specified as a **five-case
  union**, not as a port (SD-20). The five cases:

  | Input | Normalises to |
  | --- | --- |
  | legacy `{"decision":…,"reason":…}` | that decision and reason verbatim |
  | `hookSpecificOutput.permissionDecision == "deny"` + `…Reason` | `block` + the reason |
  | `systemMessage` at either position | `warn` + the message |
  | empty or unparseable stdout | `degraded` — never `allow` |
  | parseable JSON, no case above | `unrecognised` + the raw envelope |

  The fifth case makes the function **total**. Without it a well-formed envelope
  that is none of the first four — `permissionDecision` equal to `"allow"` or
  `"ask"`, or the `session_start` shape at `kernel/src/hooks.rs:9-23` — matches
  nothing, and the implementation picks a fallback silently, most likely
  `allow`: the conflation this section condemns twice. Reachable precisely
  *because* the spec strengthens the check to assert `deny` rather than infer it
  from the presence of `permissionDecisionReason`. The Rust function returns
  `Err` here; `unrecognised` is the same refusal with the envelope retained for
  the record.

  Two deliberate divergences from the Rust function, recorded as such: the
  `degraded` label, and the legacy branch it lacks. Two deliberate
  strengthenings: `permissionDecision` is asserted equal to `"deny"` rather than
  inferred from the mere presence of `permissionDecisionReason`, and the warn
  shape is accepted at both positions. The `warn` case is **not** optional — a
  colocated fixture emits warn rather than the blocked decision, which is
  exactly the hazard Phase 3's fixture pin guards, so a normaliser that cannot
  express it cannot diagnose that failure.

  ⚠️ The reason for not reusing `generate_decision_table.py:160` is that it does
  not understand the Rust envelope **at all**, so it cannot compare variants —
  *not* that it would silently pass a degraded sample — under an
  expected-`block` comparison its deny→`("allow","")` mapping would abort on the
  first pair, a loud false failure (SD-20).
- **The outcome classifier** — the seven-branch taxonomy (5a/5b, 6a/6b) of Phase
  3 §6, over the **full pre-registered state**:

  ```python
  classify(
      cell_kind,          # "absolute" | "ratio" — selects the predicate set
      lower, upper,       # bounds on this cell's statistic
      threshold,          # the cell's ceiling, in ms or dimensionless
      upper_distance,     # achieved U - point estimate, in the cell's units
      target_distance,    # this cell kind's target
      robustness_ok,      # bool | None — None for absolute cells
      escalations_used,   # session-level scalar, not per-cell
      validity,           # Valid | Invalid5a | Invalid5b
      sizing_feasible,
      applicable,         # bool — False when the cell cannot be measured here
      budget_exhausted,   # bool — mid-run wall-clock abort, selects 6b
  ) -> Branch
  ```

  ⚠️ **Whenever a branch is added, the parameter it is decided from must be
  added in the same edit.** Four successive drafts broke this rule, each
  shipping a signature narrower than the branches of the same revision (SD-21) —
  so the implementation silently encodes a narrower taxonomy that no listed test
  can distinguish from the intended one. `applicable` and `budget_exhausted` are
  the parameters branches 7 and 6b are decided from.

  Tests parametrise **both** cell kinds across all seven branches and their
  sub-labels (1, 2, 3, 4, 5a, 5b, 6a, 6b, 7): `L == t` and `U == t` exactly; `U
  ≤ t` with `robustness_ok` true and false; a spent escalation with each of the
  two terminal causes; `Invalid5a` and `Invalid5b` selecting 5 regardless of the
  bounds; and infeasible sizing selecting 6.

  Better: enumerate the domain **exhaustively** rather than by sampled cases.
  Reduced to relative positions the space is small — `cell_kind` (2) × position
  of `L`,`U` against `t` (3) × `robustness_ok` (True/False/None) ×
  `escalations_used` (2) × `validity` (3) × `sizing_feasible` (2) × `applicable`
  (2) × `budget_exhausted` (2) = **288 well-formed** states — so one
  parametrised test asserts each returns **exactly one** branch and never
  raises. That proves totality and disjointness; a case list proves only
  reachability.

  ⚠️ `cell_kind` and `robustness_ok` are **coupled**, not independent: the
  contract is `bool` for ratio cells, `None` for absolute ones. Crossing them
  freely gives 864 combinations of which 432 are ill-formed — and for a
  well-formed interval position under `(ratio, None)` neither branch 1 (needs
  the robustness condition to hold) nor branch 3 (needs it to fail) matches, so
  the cascade falls through with no verdict and the test is unsatisfiable.
  Enumerate only the well-formed pairs, and assert the ill-formed combinations
  **raise a named error** rather than returning a branch. `L == t` and `U == t`
  are included explicitly, and the cascade order in Phase 3 §6 is what the test
  asserts where two predicates overlap.
- **The closure aggregator** — `closure_verdict(cells) -> bool`, true only when
  every **gating** cell (C1-C5) is either branch 1 **or** branch 7 carrying a
  recorded acceptance flag; C6 is ignored entirely. Tested over all-pass; one
  applicable cell failing; a cell in 5b; a branch-7 cell with and without
  acceptance; every gating cell in branch 7; and C6 in any branch. An
  `any`-for-`all` defect here would tick 0189's criterion 10 on a failing cell,
  which is the outcome the whole taxonomy exists to prevent.
- **The runaway brakes**, as pure predicates rather than inline driver logic —
  `outlier_trip(sample, arm_median, arm_count)`, `budget_exhausted(elapsed,
  budget, n_done, n_max)` and `drift_verdict(first_third, last_third, band)`.
  These are the plan's stated defence against tens of gigabytes of egress and a
  drift-dominated verdict, and on a happy-path session they never fire — so
  their first execution would otherwise be the incident they exist to stop. Each
  of the **three** boundaries in `outlier_trip` is tested separately:
  `arm_count` at 19, 20 and 21 for the warm-up window, the 500 ms absolute
  ceiling that governs before it, and 5× the arm's running median after.
  `budget_exhausted` is tested at and either side of both the wall clock and
  each block's escalated cap; `drift_verdict` at and either side of its band on
  the ratio. A 10× sample injected through the measurement-runner port must
  abort the run with its diagnostic.
- **The schedule generator** — Block A's interleaved `(B, G-fast)` pairs and
  Block B's `G-fallback` samples, order randomised under the recorded seed.
  Tested for: each block reaching its own n; every Block A pair containing both
  variants exactly once; no run of same-variant samples longer than the schedule
  permits; Block B's samples never entering Block A's pairs; **segments
  alternating, so a generator emitting all of Block A then all of Block B
  fails** (without this the other four properties all hold for the batched
  sequence §2 rejects); and reproducibility under a fixed seed.

The remaining logic is **not** left to the Phase 3 run: that is where the
measurement's validity lives, a defect there yields a plausible-looking ratio
rather than a crash, and Phase 3 runs once (SD-22). Each decidable predicate is
therefore extracted as a pure, injectable function over recorded observations
and unit-tested, following the repo's existing pattern for exactly this class of
code (`tests/unit/tasks/shared/doubles.py`, and `test_limits.py`'s monkeypatched
`resource` driving a refusal path):

| Function | Tested against |
| --- | --- |
| `validate_sample(raw_b, raw_g, expected_reason)` | a well-formed matching block pair **accepted**; empty stdout, Rust deny with the wrong reason text, a warn envelope, an `unrecognised` envelope and malformed JSON each **rejected** |
| `accelerator_override_keys(env)` | an env carrying `ACCELERATOR_CACHE_DIR`, one carrying nothing |
| `ceiling_directories(tmpdir)` | a symlinked path must canonicalise |
| `unchanged_artefacts(before, after)` | identical state **accepted**; a changed inode, a changed mtime, and a **missing file** (what a self-healing re-fetch produces) each rejected |
| `log_appended_lines(before, after)` | byte-identical contents **accepted**; one appended line must invalidate, never truncate |
| `resolve_cpu_count(probes)` | a resolving cgroup quota; the literal `max` treated as "rung did not fire"; each rung forced absent in turn; returns `(count, rung)` |
| `power_state(diagnostic_runner)` | a probe returning a real state; a probe raising `FileNotFoundError` yielding `unknown`, not propagating |
| `tmp_containment(path, tmproot)` | a symlinked temp root (macOS `/var → /private/var`) accepted on both canonicalised sides; a path outside it rejected |
| `expected_decision(probe_stdout)` | the legacy envelope, the Rust deny envelope, a warn envelope; empty stdout must **refuse** rather than yield `allow` or `degraded` as an expectation |
| `residual_verdict(term_intervals, g_median, attempts_used)` | at the band boundary; equal-magnitude residuals of both signs treated alike; the propagated-uncertainty branch dominating the ±1.5 ms floor; a second attempt exhausting the cap |
| `pilot_sizing(samples, target)` | `h₀` taken as the **upper distance** against a deliberately asymmetric input (0205's `[1.2662, 1.2899]` around 1.2813, where half-width and upper distance differ); a worse-dispersion pilot sizing **up**; an escalation target beyond the cap yielding `sizing_feasible = False` |
| `platform_constants(key, table)` | a calibrated non-host key returning that platform's constants; an uncalibrated key yielding a context-only verdict; the override honoured, stripped from the subprocess environment and present in provenance |
| `retry_budget(attempts_used, cap)` | the third attempt permitted, the fourth refused — shared by the floor gate and the outlier abort |

Four ports carry the rest, split by responsibility rather than lumped into two:
a **measurement runner** returning `(stdout, stderr, exit_code, elapsed)` under
the pinned farm; a **diagnostic runner** with the ambient environment, for
`pmset` and friends; an **artefact-witness probe** (`os.stat`, digests, entry
sets); and a **host-environment probe** (env keys, cgroup, load, canonical
paths). That leaves only `subprocess.run`, `os.stat` and the `jj`/temp-dir
invocation genuinely untested, which is the irreducible part.

⚠️ The diagnostic runner is separate for a concrete reason: the measurement farm
holds exactly the two variants' tools, and `pmset` is not among them — so
routing power probes through the measurement runner would return `unknown` on
every run by construction, silently degrading half the two-way quietness
evidence while the port's own `FileNotFoundError`-to-`unknown` contract made the
degradation invisible.

⚠️ Phase 2 additionally **rehearses the abort paths as automated tests**, not as
one-time operator observations — a gate never shown to fire is not evidence, and
a rehearsal that lives only in a checklist does not survive the session it is
performed in. Three, all driven through the injected ports:

1. **Validity-gate rehearsal.** Feed the runner port a canned `allow`, `warn`,
   empty and `unrecognised` stdout with the expectation pinned to `block`, and
   assert the specific mismatch diagnostic and that **no figures are emitted**.
   ⚠️ Do *not* rehearse this by pointing the harness at a non-repo fixture: §4
   derives the expected decision from the pre-sampling probe, so the probe would
   derive `allow` as the expectation and the validity gate would have nothing to
   mismatch — what fires is the separate blocked-shape assertion, a different
   gate with a different diagnostic (SD-23).
2. **Brake rehearsal.** Inject a 10× sample and assert the outlier trip aborts
   with its diagnostic; drive `budget_exhausted` past its limit and assert
   branch 6.
3. **Signal rehearsal.** Launch the harness in a subprocess and send **SIGINT,
   SIGTERM and SIGHUP** in three separate cases, asserting for each that the
   restore phase ran, the verify phase recorded its result, and every artefact
   in the manifest table is positively absent — the fixture, `T`, both farms,
   the floor script and the marker. The manifest is exempt — it is the interlock
   token, removed by verify step 8.

#### 2. Registration

Follow the conventions in `tasks/README.md` and the existing module pattern:

- Add `measure` to the import tuple and the `add_collection` block in
  `tasks/__init__.py`, at its **alphabetical position** (between `marketplace`
  and `public_api`).
- Add the mise task in full — every one of the ~120 existing tasks carries a
  `description`, which `mise tasks` renders and which the success criterion
  below greps for:

  ```toml
  [tasks."measure:warm-dispatch"]
  description = "Measure warm-dispatch latency vs the shell baseline (quiet host, network egress, ~8 min, up to ~30 min if the interval escalates)"
  depends = ["deps:install:python"]
  run = "invoke measure.warm-dispatch"
  ```

  Register **`measure:teardown`** in the same pass, with the same `depends` edge
  and its own description — it is the documented escape from the stale-manifest
  start-up refusal (SD-15):

  ```toml
  [tasks."measure:teardown"]
  description = "Replay the measurement harness's restore and verify phases from a stale manifest"
  depends = ["deps:install:python"]
  run = "invoke measure.teardown"
  ```

  And the integration smoke check, whose `run` **must** invoke the measure
  module so the widened guard predicate can see it:

  ```toml
  [tasks."test:integration:measure"]
  description = "Live-dispatch smoke check for the warm-dispatch harness (n=2, no gating figure)"
  depends = ["deps:install:python"]
  run = "invoke measure.smoke-check"
  ```

  ⚠️ The `run` string is load-bearing. Every `run` in `mise.toml` is either
  `invoke <module>.<task>` or `uv run pytest <dir>`, so "reaches
  `tasks/measure.py`" is decidable only if the task invokes the measure module
  directly. Had it followed the sibling `uv run pytest tests/integration/...`
  shape, the widened guard would have been exactly as blind to it as the
  `measure:*` prefix form it replaced — a third instance of the same blindspot.

  The `depends` edge matters: 24 tasks carry `deps:install:python`, including
  every invoke-backed leaf doing real work, and without it the linux hand-off
  fails on a fresh checkout at `invoke` not being installed — the friction the
  committed harness exists to remove. The runtime figures come from Phase 3 §2's
  per-block arithmetic, not from a two-arm estimate.

  Note the invoke naming conversion: the Python task is `warm_dispatch` and mise
  addresses it as `warm-dispatch`.
- **Keep the namespace out of the aggregate `check` and the bare default task**,
  with a guard in `tests/unit/tasks/test_mise.py` written over the **transitive
  closure** of `depends` from both `check` and `default`. The cited
  `test_docs_tasks_stay_out_of_default_and_aggregate_check` asserts membership
  in a single `depends` list, which would stay green if a future edge added
  `measure:*` under any task already inside the closure — and `default` reaches
  work through `lint:check` rather than through `check`, so one-level assertions
  miss real indirection. The harness needs a quiet host, the runtime §2 states
  and network egress; an accidental transitive edge would make every `mise run`
  a benchmark.
- **Give the namespace an owner, without breaking lane hermeticity.** The docs
  precedent is only half applicable: `docs:*` is excluded from `check`/`default`
  but is *owned and run by the docs CI lane*, whereas `measure:*` would be owned
  by nothing while depending on volatile external contracts — the digest-backend
  selection, the cache root derivation, the launcher's cache/verify layout, the
  hook envelope shape, `jj`'s colocation default, and a revset anchoring two
  deleted files. A module no automated path ever executes rots invisibly, and
  the first person to discover it would be the linux hand-off, months later.

  The owner is split in two, because the two halves have incompatible
  requirements:

  - **`test:unit:tasks` gets the hermetic half only** — the predicates, the
    classifier, the closure aggregator, the brakes, the schedule generator and
    the normaliser (driven from the committed `decision-table.json` goldens and
    literal envelopes), through the **measurement-runner and artefact-witness
    port doubles**; and the host-environment predicates through the
    **host-environment probe** double. No `bin/accelerator` dispatch, no live
    cache root, no network.

    **Fixture construction is tested against a real `jj git init`**, not a
    double: `jj` is pinned in `mise.toml` and the init is offline, so it stays
    hermetic in this repo's sense, and it is the only way to test the property
    that matters — that `--config git.colocate=false` leaves `.git` absent and
    `.jj` present. A colocated fixture emits **warn** rather than the blocked
    decision, so this is the one fixture defect that would invalidate the whole
    session without crashing anything, and stubbing the init would leave it
    untested by construction.
  - **`test:integration:measure` gets the live-dispatch smoke check** — `n = 2`,
    floors only, no gating figure, asserting the driver resolves the bootstrap
    and the fixture emits the blocked shape. Its registration is decided here,
    not left to the executor, because `tests/unit/tasks/test_mise.py` enforces
    two independent classifications and the plan must satisfy both:

    - **Excluded from the `test:integration` roll-up**, recorded in
      `_NOT_IN_INTEGRATION_ROLLUP` with the reason "live release fetch;
      quiet-host harness, owned by its own lane". Roll-up membership would put
      it under `test` → `default` (`mise.toml:374`) and on both CI legs
      (`main.yml:91`) — reinstating the very breach this split exists to end,
      including on `macos-latest`, which resolves no `sha256sum`.
    - **Owned by a dedicated non-blocking CI job**, following the
      `test:integration:pup` / `zero-spawn:strong` precedent for
      roll-up-excluded tasks. Without a named lane the exclusion just restores
      the unowned state, so the job is part of this deliverable rather than a
      later nicety. The "Own the recurring absolute-budget check" follow-up is
      its natural long-term home.
    - **In `_NO_LAUNCHER_NEEDED`**, not the launcher-dependent set, with the
      reason "fetches the released launcher; builds nothing". ⚠️ An earlier
      draft said it carried a `build:cli:dev` edge *and* followed
      `test:integration:entrypoint`'s shape — impossible, since `entrypoint`
      sits in `_NO_LAUNCHER_NEEDED` ("builds it in-fixture") and
      `test_the_two_launcher_sets_are_disjoint` asserts the two sets disjoint.

  ⚠️ **Widen the closure guard's predicate.** It must match **any task whose
  `run` reaches `tasks/measure.py`**, not the `measure:*` name prefix. As
  written the prefix form does not match `test:integration:measure`, so the one
  live-dispatch path the guard exists to contain would be invisible to it — the
  second time in this plan a guard could not see the path that broke its own
  rule. Keying on the `run` string removes the class of failure rather than this
  instance of it.

  ⚠️ **Do not wire the live-dispatch check into `test:unit:tasks`.** An earlier
  draft did, and five review lenses independently found the same breach:
  `test:unit:tasks` → `test:unit` (`mise.toml:264`) → `test` → `default`
  (`:604`), and `.github/workflows/main.yml:53` runs it on both matrix legs — so
  every `mise run` would need network egress and a published signed release for
  the tree's own version, mutate the live cache root that Phase 3's own
  integrity witness is measured against, and fail on `macos-latest`, which
  resolves no `sha256sum`. It contradicted this plan's own "No `measure:*` task
  in the aggregate `check` or the bare default"; it breached the classification
  `tests/unit/tasks/test_mise.py:39-68` encodes, which requires
  launcher-reaching tasks to be `test:integration:*` with a `build:cli:dev`
  edge; and — decisively — the transitive-closure guard designed to enforce the
  exclusion asserts absence from mise `depends` and **cannot see a pytest under
  `tests/unit/tasks/`**, so the guard could not detect the path that broke its
  own rule. Update the Testing Strategy's "Integration Tests" section
  accordingly rather than leaving it reading "None".

- **Make the environment pin *and every gate constant* data, not code.** One
  per-platform table keyed on **`(platform.system(), platform.machine())`** —
  following `tasks/shared/targets.py:33` rather than `system()` alone, since
  darwin-x64 and linux-arm64 need distinct entries — carrying the pinned `PATH`,
  the symlink-farm recipe, the power probes, **the four absolute ceilings, the
  two instrument-floor gates and the reference bash identity**. Selected by host
  key, with a documented override: a `--platform-key` task option plus a
  non-`ACCELERATOR_`-prefixed env fallback, stripped from the subprocess
  environment and recorded in the provenance set. A unit test selects a non-host
  key so the linux data path is exercised from darwin.

  **The harness refuses to emit a gating verdict for a platform key with no
  calibrated entry**, recording its figures as uncalibrated context instead —
  branch 7.

  Each entry additionally carries **calibration provenance**: source session,
  chip, resolved `bash` identity and `shasum` implementation. The verdict
  demotes to uncalibrated context when the observed host record disagrees with
  the entry's provenance, extending to all four what the plan already does for
  `bash` alone. `(system, machine)` under-determines the calibration on its own:
  the floor gates come from one 0205 session on one chip, `/bin/bash` 3.2 and
  homebrew bash 5 differ materially in startup *within* a single key, and C3/C4
  encode this host's Perl startup — so two hosts sharing a key would otherwise
  be judged by numbers calibrated for one of them while being reported as
  calibrated.

  ⚠️ The gate *numbers* must move with the mechanism, not stay host-specific
  behind it (SD-24).
- **Document the namespace** as a `### The measure namespace` subsection under
  `tasks/README.md`'s "Conventions (learn once)", beside "Executable-bit
  invariant" and "The Rust nightly lane": why it is out of `check` and
  `default`, who runs it, and what a run requires (quiet host, pinned `PATH`,
  network egress, runtime). ⚠️ `tasks/README.md` contains no `docs:*` section to
  mirror (SD-25). The rationale to follow lives in the root `CLAUDE.md` and in
  the comment at `tests/unit/tasks/test_mise.py:71-82`. Without this the linux
  hand-off inherits a `mise tasks` leaf with no stated prerequisites, and the
  operating knowledge lives only in a plan Phase 4 marks done — which would
  defeat the reason Phase 2 commits the harness at all.

Two toolchain constraints the module must satisfy without touching config:

- `tasks/*.py` is already in ruff's and pyrefly's walk (`pyproject.toml` scopes
  pyrefly by `project-includes = ["tasks/**/*.py", …]`), so
  `test_python_coverage.py` covers the new module with no config change — a
  property to confirm rather than assume, and one that **must not** be satisfied
  by adding an exclude, since that test pins the exclude sets by exact value.
- ruff runs `select = ["ALL"]` with an ignore list that does not include `S311`,
  and `tests/**` is the only per-file relaxation — so the seeded resampler needs
  an inline `# noqa: S311 — statistical resampling, not a security context`, per
  the repo's inline-exemption convention (`tasks/signing.py:21`,
  `tasks/version.py:100`, `tasks/shared/playwright.py:68`). No `tasks/` module
  uses `random` today, so there is no precedent to copy. The module is also
  fully annotated for pyrefly's strict preset.

### Success Criteria:

#### Automated Verification:

- [x] `mise run test:unit:tasks` is green — the aggregate task, not a bare `uv
  run pytest`, since `test_python_coverage.py`'s sentinel probes `pytest.skip`
  when the tools are off `PATH`, so a bare invocation can be satisfied by two
  skips rather than two passes
- [x] `tests/unit/tasks/test_measure.py` covers every core function, every
  extracted predicate in the table above, and every classifier terminal label
  (1, 2, 3, 4, 5a, 5b, 6a, 6b, 7)
- [x] The closure guard in `test_mise.py` — keyed on tasks whose `run` reaches
  `tasks/measure.py`, not on the `measure:*` prefix — asserts absence from the
  **transitive closure** of `depends` from both `check` and `default`
- [x] `test_python_coverage.py` reports (not skips) with `tasks/measure.py` in
  scope and no new ruff or pyrefly exclude
- [x] `mise run build-system:check` is green
- [x] `mise run` (bare default task) exits 0 end-to-end
- [x] `mise tasks` lists `measure:warm-dispatch` **with its description**

#### Manual Verification:

- [x] Each core function and each extracted predicate was driven by a failing
  test first — evidenced by the commit sequence showing a failing-test commit
  preceding its implementation, since greenness alone cannot distinguish
  red-first from test-after
- [x] The normaliser is checked case by case against all five union cases —
  including `unrecognised`, the case that makes it total — with its two
  deliberate divergences from `cli/vcs-cli/tests/guard_decision_table.rs:99-141`
  (the `degraded` label and the legacy branch) recorded as such rather than
  treated as defects
- [x] The harness runs end to end against a throwaway fixture without recording
  a gating figure, confirming the driver works before Phase 3 commits to a
  single session
- [x] The fault-injection rehearsal aborts with the mismatch diagnostic rather
  than producing a ratio, and the SIGINT rehearsal leaves every artefact
  positively absent
- [x] `tasks/README.md` documents the `measure:*` namespace and its
  prerequisites

---

## Phase 3: The Confirmatory Measurement

### Overview

Measure `B` and `G` on a quiet darwin-arm64 host in one session under both
digest backends, over Block A (1,700 pairs) and Block B (900 samples), and
classify C1-C5 against the seven-branch taxonomy (5a/5b, 6a/6b), recording C6's
figures without a branch.

### Changes Required:

#### 1. Recover the baseline's subject

0169's Phase 9 deleted `hooks/vcs-guard.sh` in `cf42441e2aad`. It recovers from
`cf42441e2aad-`, resolved commit id `2cfbf81e2e7b4934e868bd42c69374c335b05317`
(0205's recorded resolution).

**Both** the guard and its single dependency are recovered at that one revision,
**by the harness, inside the context manager** — a `recover_baseline(revision)`
step, not operator shell. The block below documents what it does; it is not a
step the operator runs first (SD-26).

`T` is a scratch tree in an OS temp directory outside the plugin root and
outside any jj workspace, laid out so the guard's
`"$SCRIPT_DIR/../scripts/vcs-common.sh"` resolves within `T`:

```bash
mkdir -p "${T}/bin" "${T}/scripts"
jj file show -r cf42441e2aad- hooks/vcs-guard.sh > "${T}/bin/vcs-guard"
jj file show -r cf42441e2aad- scripts/vcs-common.sh > "${T}/scripts/vcs-common.sh"
chmod +x "${T}/bin/vcs-guard"
```

Three properties follow:

- **Nothing is written inside the repository.** No copy is staged in `bin/`,
  which is the launcher's **live cache root** (`bin/accelerator:201`) and whose
  `.tmp-` namespace is `store::TEMP_PREFIX` (`cli/store/src/lib.rs:24`) — so a
  staged copy would both park an unreviewed executable where the launcher execs
  from and add an entry to the very directory whose `cache::find` scan is a
  budget term and whose entry count Phase 3 §7 separately records.
- **`B` is revision-pinned rather than coupled to the live tree.** 0205
  symlinked `T/scripts` at the live `scripts/`, so its baseline resolved a
  mutable dependency. Recovering `vcs-common.sh` at the guard's own revision
  makes the subject self-contained: work item 0199, scoped to decide whether
  `classify_checkout` leaves that file, can no longer invalidate the recorded
  `B`, and the teardown has no symlink to step around.
- **`T` is outside every lint and VCS gate.** Being outside the repository it is
  invisible to `shell_sources()`, the exec-bit invariant and any `jj diff`/`jj
  status`, which is why its removal is asserted **positively** by recorded
  resolved path rather than inferred from a clean diff.

Record the resolved git commit id and the **sha256** of both recovered files.
Without the sha256 the recovery is verifiable only inside this jj workspace. On
a plain git clone the equivalent is `git show <commit>:hooks/vcs-guard.sh`,
which needs unshallowed history; a short hex prefix can be a jj *change* id,
which is why the resolved commit id is recorded rather than the revset alone.

⚠️ Before sampling, run the recovered guard once and **record its raw envelope
shape**, deriving the expected `(decision, reason)` from that observation rather
than assuming the legacy `{"decision","reason"}` form. 0169's Drafting Notes
record the PreToolUse envelope moving to the `permissionDecision` shape on
2026-07-30 — *before* the guard was deleted — so the recovered revision may
already emit the new shape.

#### 2. The run

- **Two blocks, sized and rotated separately**, per the sizing table in [The
  Criterion](#the-criterion): Block A is 1,700 interleaved `(B, G-fast)` pairs
  feeding C1, C2 and C5; Block B is 900 `G-fallback` samples feeding C3 and C4
  (and C6, recorded). Within-pair order is **randomised under the recorded
  seed** rather than deterministically alternated — alternation defeats monotone
  drift but can alias with a periodic perturbation commensurate with the pair
  cadence, landing preferentially on one variant.

  ⚠️ **Block B is a separate block, not a fourth arm in one rotation.** A single
  four-arm rotation left C5's pairing rule undefined, contradicted the seeded
  within-pair randomisation, and put Block B's ~10.7 MB-per-sample load *inside*
  the pairs C5 is computed from (SD-27). Separate blocks keep C5's pairs shaped
  like the sequence its dispersion estimate came from.

  Blocks alternate in **segments of 100 samples**, recorded, so neither block
  occupies one contiguous half of the session and monotone drift cannot land
  wholly on one. Inter-segment thermal carry-over is accepted rather than
  eliminated — interleaving keeps Block B's samples out of C5's pairs but places
  its load adjacent in time to them, and the pre- **and** post-sampling
  instrument floors are the witness that the instrument did not move across the
  session.

  The **pilot segments run first** — 200 Block A pairs, then 200 Block B samples
  — are discarded, and do **not** count against the 6,900 / 3,600 caps. A
  size-up regenerates the schedule from the new n before sampling proper begins.
  The schedule generator's tests assert pilot samples never appear in the
  analysed pairs.

- **Runtime, computed per block.** Block A: 1,700 pairs at ~106 ms ≈ **3.0
  min**. Block B: 900 samples at ~83 ms ≈ **1.3 min**. Sampling ≈ 4.3 min; with
  probes, floors, provenance and the two farm builds, **≈ 6 min** end to end.
  Branch 3's escalation takes Block A to ~6,900 pairs ≈ 12.2 min **and** Block B
  to ~3,600 samples ≈ 5.0 min, so an escalated session is **≈ 19 min** — inside
  the escalated sampling alone. ⚠️ **The budget must cover the whole session,
  not the escalated run.** Escalation fires only after the initial 1,700/900 run
  has been taken and analysed, and the escalated run *replaces* rather than
  pools — but both samplings happen inside the one session the criterion's "same
  host, same session" wording requires. Initial ~6 min + escalated ~17.2 min +
  the 400-sample dispersion pilot (~0.6 min) + §7's in-session term
  re-measurement at n = 200 each plus the direct `sha256_file` bracket (~2 min)
  ≈ **26 min**, which a 25-minute budget exhausts — so `budget_exhausted` would
  fire, the cascade would select 6b (which precedes 4), and the escalation
  branch 3 authorises would be unreachable. **The budget is therefore 35
  minutes**, measured from harness entry, with ~9 minutes of headroom; the mise
  `description` reads "~8 min, up to ~30 min if the interval escalates". The
  mise `description` and the wall-clock budget below are both derived from the
  figures here (SD-28).

- **A hard wall-clock budget of 35 minutes from harness entry**, and escalated
  maxima of 6,900 Block A pairs and 3,600 Block B samples, all pre-registered as
  numbers rather than left to the operator. A budget set by discretion at run
  time is the same optional-stopping hazard the floor-retry cap closes.
  Exceeding the budget selects **branch 6**, recorded as a budget abort and
  explicitly distinguished from branch 4.

- **One Python process** reading `perf_counter` around each `subprocess.run`. A
  per-call `python3` clock read would put an interpreter startup inside the
  measured interval. The per-sample inode/mtime witness, the envelope
  normalisation and the validity comparison all run **outside** the timed
  bracket.

- **Two instrument floors** — a trivial bash script and `true` resolved via
  `shutil.which('true')` against the **subprocess's** environment, asserting the
  floor binary was found before sampling. Floor treatment and the three ratios'
  roles are fixed in [The Criterion](#the-criterion); this section only measures
  them. Both floors are measured **before and after** sampling and both pairs
  recorded: 1,700+ samples hashing ~10.7 MB each drive real thermal load, so
  end-of-run floors are the cheapest witness that the instrument itself did not
  move.

- **Dispersion.** n, min, median, p90 and IQR per variant, plus `p90(G)/p90(B)`
  as non-gating context — for a hook on every Bash tool call, the tail is what
  users feel. Also record `median(Gᵢ/Bᵢ)` over Block A's pairs alongside C5's
  ratio of medians: pairing currently enters C5 only through the resampling, not
  the estimator, so the two diverge under drift and their divergence is the most
  direct drift diagnostic the collected data affords.

- **Drift diagnostic, banded on the gated quantity.** Compare Block A's
  first-third and last-third `median(G)/median(B)`; a shift exceeding **0.005**
  (about a quarter of C5's 0.0187 margin, ~1.4 achieved upper-distances)
  invalidates the session under **branch 5b**. Per-variant medians are recorded
  as context, not as the gate (SD-29).

- **Both digest backends, each with its own farm.** C1/C2/C5 are measured under
  the fast farm and C3/C4/C6 under the fallback farm, the two differing solely
  in whether the `sha256sum` link is present. The harness asserts **both**
  directions before sampling the block that depends on it: `command -v
  sha256sum` **resolves** in the fast farm and its resolved path matches the
  expected backend, and **fails** in the fallback farm with `shasum` resolving
  instead (SD-30).

  Record `shasum`'s resolved path, its implementation and `perl --version`
  alongside its own `--version`: on macOS `shasum` is a **Perl script**, so C4's
  ceiling and the predicted delta encode this host's Perl interpreter startup
  rather than a property of the algorithm or the OS — and a minimal linux image
  with `sha256sum` but no Perl cannot construct the fallback farm at all, in
  which case C3, C4 and C6 are recorded "not applicable on this host". State
  that in the criterion's limitations and in the linux hand-off's prerequisites.

  State the prediction before measuring: from 0186's 3.55 ms against 11.99 ms
  per call, two calls put the fallback delta at ~16.9 ms, `median(G)` at ~59.2
  ms and C6 at ~1.79. Record whether it held.

`B` is the recovered baseline; `G` is `${PLUGIN_ROOT}/bin/accelerator vcs guard
--format=hook --fail-safe`, dispatched through the real bootstrap with the cache
warm. Both are invoked by **absolute path** — the subprocess cwd is the fixture,
so a repo-relative invocation would resolve the baseline's `SCRIPT_DIR` against
the wrong directory.

Both receive **byte-identical stdin**, the envelope 0205 used:

```json
{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git status"}}
```

This matches 0169's criterion, which names `git status` specifically. The guard
reads only `.tool_input.command`, so every other field is inert and is recorded
as such.

#### 3. Fixture

A fresh pure-jj scratch repository in an OS temp directory **outside the plugin
root and outside any jj workspace**, created via a context manager so an
abnormal exit still removes it, with its resolved path recorded. A fixture
nested inside the live workspace would let the outer auto-snapshot pull it into
the working copy — the hazard `.gitignore:30` already documents.

**Pinned and asserted.** `jj git init` colocates by default at 0.43, and a
colocated fixture emits **warn**, not the blocked decision, so the harness would
silently measure the wrong path. Create it with `jj --config git.colocate=false
git init --quiet`, matching what `generate_decision_table.py:130` already does.
Set `GIT_CEILING_DIRECTORIES` from an `os.path.realpath`-canonicalised root —
git ignores non-canonical entries and does not resolve symlinks by default, and
macOS `$TMPDIR` sits under a `/var → /private/var` symlink, so a naive entry is
silently ignored on exactly the primary host. `GIT_CEILING_DIRECTORIES` has no
jj equivalent, so the pre-sampling decision-shape probe is the authority, not
the ceiling.

Record the fixture's canonicalised path depth and the **observed** `dirname`
spawn count for `find_repo_root`, taken from a `bash -x` trace rather than
implied by depth.

⚠️ The expected count is **zero at any depth** (SD-31). `find_repo_root`
(`scripts/vcs-common.sh:8-18`) initialises `dir="$PWD"` and tests `-e
"$dir/.jj"` **before** its first `dirname` call, and the subprocess cwd is the
fixture root, which is where `jj git init` created `.jj` — so the loop matches
on iteration one. Confirm this empirically rather than assuming it in either
direction: if it holds, `B` is **depth-insensitive**, which is a positive
reproducibility property worth recording, because it means the linux hand-off's
shallow `/tmp` and this host's deep `/var/folders/…` temp root do not perturb
the denominator and no depth pinning is needed. If the trace shows spawns, pin a
canonical depth on both platforms instead and say so.

⚠️ The empty single-operation pure-jj fixture is `G`'s **best case** for the
jj-lib repository load, while `B`'s two directory-entry tests are
repository-state-independent — so the ratio is calibrated at the point most
favourable to `G`. The magnitude is bounded rather than unknown: 0188
re-measured its library-backed probe at 4.81 ms on this repo's real colocated
workspace against 4.03 ms on a pure-jj fixture (2.80 against 3.58 ms for a
single query), putting the fixture bias at roughly ±1 ms, about 2% of `G`. Cite
that pair as the recorded bound, and state in the criterion's limitations that
the empty fixture is required by the blocked-decision shape rather than chosen
for favourability.

#### 4. Per-sample validity

Assert on every sample that both variants produce the expected **blocked**
decision on the same stdin, and abort on the first mismatch.

Compare **after normalisation through Phase 2's five-case union**, which maps a
`permissionDecision == "deny"` envelope onto `block` and empty or unparseable
stdout onto `degraded`. Do **not** use `generate_decision_table.py:160`: it does
not understand the Rust envelope at all, so it cannot compare variants. (The
reason is incapacity, not silent leniency — under an expected-`block` comparison
its deny→`allow` mapping would abort on the first pair.) Derive the expected
`(decision, reason)` for `B` from the pre-sampling probe of §1 rather than
assuming a shape, pin the expected reason text, and record both raw envelope
shapes verbatim.

The exit code carries no decision information — 0 for block, allow and
degradation alike — so it is asserted as a liveness check only.

**The inode/mtime witness runs per sample, not once at the end.** Assert that
the cached asset's, the cached launcher's, its `.minisig`'s and the staged
shim's inode and mtime are unchanged, and abort on the first change. Every
non-hit route ends in `cache::store`, which renames a fresh inode over the
entry, so this is a cheap branch witness — and restricted to the sub-binary it
would let a re-fetched launcher or re-staged shim inflate the sample undetected.

⚠️ Per-sample is not a refinement; it is what bounds a runaway.
`FetchVerifyCacheResolver::resolve` (`resolve/mod.rs:198-211`) **self-heals** on
any cache-hit re-verification failure by calling `fetch_verify_store`, which
re-downloads the ~8 MB asset. Across Block A's 1,700 pairs and Block B's 900
samples, with branch 3's escalation to ~6,900, a single corrupt entry, a mid-run
eviction or a key mismatch would turn a supposedly offline warm-path benchmark
into thousands of consecutive fetches — tens of gigabytes of egress,
near-certain rate-limiting, and a meaningless ratio — with a post-run-only
witness detecting it after all of it had happened. Add two further brakes, both
numbered rather than left to the operator. An **outlier trip**: per **arm**, not
pooled — the arms span 33 to 59 ms, so a pooled median is arm-blind — aborting
when a sample exceeds 5× that arm's running median once at least 20 of its
samples exist, and against an absolute 500 ms ceiling before that. A network
fetch is orders of magnitude above ~42 ms, so the absolute form is well defined
from sample one, which is the riskiest one since the warm-up dispatch is
discarded. An outlier abort carries the **same three-attempt recorded retry
cap** as the floor gate, so the operator's re-run is not unrecorded repeated
sampling. And the 35-minute wall-clock budget with its 6,900-pair maximum from
§2, whose exhaustion selects branch 6.

#### 5. Preconditions

Asserted by the harness over the exact environment handed to `subprocess.run`:

- No `ACCELERATOR_*` override is set — matching on key *names* (`[k for k in env
  if k.startswith("ACCELERATOR_")] == []`) rather than grepping `env` output,
  which also matches values and is line-oriented. This covers
  `ACCELERATOR_VCS_BIN`, `ACCELERATOR_BIN`, `ACCELERATOR_CACHE_DIR`,
  `ACCELERATOR_PLUGIN_ROOT`, `ACCELERATOR_RELEASE_BASE_URL` (`main.rs:38-40`)
  and the `ACCELERATOR_UNAME_S`/`_M` seams (`bin/accelerator:17-18`). Print the
  observed keys rather than only asserting absence. Since no override is set,
  the cache root is always `${plugin_root}/bin` (`bin/accelerator:201`).

  ⚠️ **One documented exception**: `ACCELERATOR_RELEASE_BASE_URL`
  (`bin/accelerator:349`) is the only seam for pointing the bootstrap at
  anything other than
  `https://github.com/atomicinnovation/accelerator/releases/…`. Rejecting it
  outright hard-couples the harness to anonymous github.com egress, which a
  mirrored, proxied or air-gapped hand-off host cannot satisfy without violating
  the precondition. It is therefore permitted, asserted to be either absent or
  explicitly recorded in the provenance set, with any figures measured against a
  mirror marked as such. Every other `ACCELERATOR_*` key is rejected.
- `${PLUGIN_ROOT}/.accelerator-dev-launcher` is **recorded, not required
  absent**. The override needs all three of the marker,
  `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER` and `ACCELERATOR_LAUNCHER_BIN`
  (`bin/accelerator:239-240`), and the preceding bullet already rejects every
  `ACCELERATOR_*` key — so under the pinned environment the marker alone is
  inert, and requiring its absence would force a remove-and-restore whose crash
  window silently switches the operator's later sessions from their dev build to
  the fetched release binary.
- **The plugin root is resolved, recorded, and confirmed to be the one the
  driving session's hooks dispatch against** — resolved the way
  `bin/accelerator` resolves it, and compared against the session's observed
  `CLAUDE_PLUGIN_ROOT`. This repo is worked through jj workspaces, so a
  session's plugin root may be the main checkout while the harness runs in a
  workspace, giving two distinct `bin/` cache roots — in which case every
  integrity witness would point at one while the interfering session wrote the
  other, reporting green throughout. Refuse to sample if they differ.
- **A published, minisign-signed release exists for the tree's own version.**
  `bin/accelerator:138-141` reads `version` from `.claude-plugin/plugin.json`
  and `:349` derives the release base URL from it, so the warm path is reachable
  only if a launcher — and `accelerator-vcs` — is published for that exact
  version and platform. The tree is at `1.24.0-pre.37` while this plan's
  evidence covers `pre.36`/`pre.35`. Assert this before sampling and record it;
  otherwise the warm-up dispatch 404s and `fail_integrity` fires, surfacing as
  an opaque bootstrap abort rather than a named unmet prerequisite. The same
  line belongs in the linux hand-off's prerequisites.
- **No other Claude Code session is active against this plugin root**, checked
  concretely rather than asserted: enumerate `claude` processes, and create an
  advisory marker at the path the manifest table records — **not** at the
  repository root, per the pre-flight rule. A concurrent session appends to the
  unverified log and can flip the launcher or shim inode, failing the branch
  witness for a benign reason — and an unenforced precondition makes that
  indistinguishable from tampering. ⚠️ The driving session is itself one: this
  plan is executed by a Claude Code session whose SessionStart and PreToolUse
  hooks dispatch `bin/accelerator` against this same plugin root. Record its pid
  and exclude it explicitly, so its own dispatches are not counted as
  **tampering** — but **record its dispatch count during the run**, because the
  exclusion suppresses *detection* of its interference rather than the
  interference itself: its hook dispatches contend for CPU against a floor-gated
  quietness requirement regardless of whose pid issues them. The operator issues
  no tool calls while sampling, stated as an operating condition, and that
  condition plus the recorded count are what bound the exposure rather than the
  pid filter.
- One discarded warm-up dispatch has completed, so the launcher takes the
  cache-hit branch.
- **`PATH` is pinned and printed**, twice — once per digest backend — and built
  as a **harness-created symlink farm** whose contents are the **union of both
  variants' tool sets**, enumerated mechanically from `bin/accelerator` as well
  as from the recovered guard and `vcs-common.sh`. The two farms differ only in
  whether the `sha256sum` link is present. Record each farm's contents with
  every link's target realpath.

  ⚠️ Enumerating from the baseline alone breaks the variant being gated. `G`
  spawns `uname` twice (`bin/accelerator:122-123`), `sed` to read the version
  from `plugin.json` (`:138-139`), `awk` inside each `sha256_file` (`:274`,
  `:276`), and `curl` or `wget` if the warm-up takes the cold branch
  (`:145-159`) — none of which `B` uses. A `B`-only farm fails *silently*:
  absent `uname` gives `fail "unsupported architecture"`, absent `sed` gives
  `fail "could not read version from plugin.json"`, and under `--fail-safe` each
  exits **0**, which is exactly the degraded-sample shape that records a
  spuriously low latency. Assert that one dispatch under each farm reaches the
  expected blocked envelope before any timing is taken.

  Link the `os.path.realpath`-resolved concrete binary, never a wrapper or a
  mise shim: a shim re-resolves its version from the config discovered at the
  cwd, and the sampling cwd is the fixture — outside every mise config — so the
  `jj` version assertion could pass where the harness runs it while a different
  `jj` is used at sample time. Re-run that assertion **through the farm with cwd
  set to the fixture**, and compare each link's `--version` output byte-for-byte
  against the same probe taken before the farm was built. Pruning a directory
  instead would drop that directory's other tools with it (`/opt/homebrew/bin`
  also supplies `jj`, `jq` and GNU `grep`/`sed`/`awk`; on linux `sha256sum` sits
  in `/usr/bin` alongside every tool `B` spawns, so a prune-based second arm is
  not constructible there at all). The farm also makes the isolation exact
  rather than incidental. ⚠️ Never by moving, renaming or `chmod`-ing a system
  binary: that mutates host state with no entry in the teardown and no
  restoration path.
- **Every executable the baseline actually spawns is recorded**, enumerated from
  the recovered script rather than from memory, each with absolute path and
  `--version`, and each asserted resolvable before sampling. `B`'s cost is bash
  startup plus roughly fifteen `jq`/`grep`/`sed`/`awk`/`cat`/`timeout` spawns,
  so recording only `jj`, `git`, `bash`, `realpath` and `jq` leaves the gate's
  denominator resting on an unpinned tool flavour — GNU on a homebrew-first
  `PATH`, BSD on stock macOS, with `timeout` absent from the macOS base system
  entirely. Also record `sys.executable`, `sys.version`,
  `time.get_clock_info('perf_counter')` and the bootstrap seed.
- **`LC_ALL=C` and a fixed `TZ`**, asserted in the subprocess environment and
  recorded. `LANG` alone is insufficient, and `B` is dominated by
  `grep`/`sed`/`awk` spawns whose speed varies materially between `C` and a
  UTF-8 locale — an uncontrolled multiplier on the denominator, and an
  unrecorded one makes the darwin figure non-comparable with any later linux
  run.
- **`jj`'s resolved version equals the `mise.toml` pin** (`0.43.0`, held in
  lockstep with the `jj-lib` crate), asserted rather than merely recorded. The
  fixture's `git.colocate=false` incantation is justified by the 0.43 default,
  so a differently-versioned `jj` from the farm could change which repo mode the
  fixture is in — the exact failure the pin exists to prevent. The linux
  hand-off already lists this; the darwin run must match.
- Host, OS version, chip and plugin version.
- **Quietness, evidenced two ways, both with decision rules.** Because 0205's
  load model is unsupported, quietness is not inferred from load average alone.
  Record the raw load and the CPU count as **two separate values**, never a
  derived ratio: on linux `/proc/loadavg` is host-scoped regardless of cgroup
  membership, so dividing it by a container's quota yields a meaningless number.
  Resolve the count through cgroup v2 quota if resolvable — the leaf path from
  `/proc/self/cgroup` joined under `/sys/fs/cgroup`, reading `cpu.max`, treating
  the literal `max` as "rung did not fire", with cgroup v1 explicitly out of
  scope — else `os.process_cpu_count()`, recording which rung fired. Read load
  via `os.getloadavg()`, portable across both OSes, rather than `/proc/loadavg`,
  which does not exist on darwin. The chain stops there because `mise.toml` pins
  `python = "3.14.4"` and `pyproject.toml` declares `requires-python =
  ">=3.14"`, so `process_cpu_count` is unconditionally present and any further
  rung is unreachable scaffolding.

  Then measure the **instrument floors before sampling** and gate on them: the
  bash floor must be ≤ **7.8 ms** and the `true` floor ≤ **1.95 ms** — ~10%
  above the floors **0205's own session implies**. Inverting its recorded
  subtracted ratios, `(42.28−c)/(33.00−c) = 1.297` gives `c ≈ 1.75 ms` and `=
  1.358` gives `c ≈ 7.08 ms`. 0186's quieter figures (6.10 / 1.41 ms) are kept
  as the reference; 0205's are the calibration, because 0205 is the result being
  confirmed (SD-32).

  A breach is a precondition failure under branch 5a, not a note. At most
  **three** abort-and-retry attempts, every attempt recorded including its
  floors — an operator free to retry informally until the floors look good is
  running optional stopping through the back door. Record the farm's `bash`
  resolved path and version alongside both reference sessions' bash identity;
  homebrew bash 5 and `/bin/bash` 3.2 differ materially in startup, so where
  they differ the floor comparison is recorded as indicative rather than as a
  gate.
- Power probes are **additive**, each recording `unknown` when absent so one
  harness runs on both OSes: `pmset -g ps` plus `pmset -g therm` and the
  low-power-mode flag on darwin (AC-versus-battery alone misses the thermal
  throttling that perturbs a margin this size);
  `/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor`,
  `intel_pstate/no_turbo` and `power_supply/*/status` on linux.

#### 6. Classify the outcome

Each of the six cells defined in [The Criterion](#the-criterion) is classified
independently, and **the item closes only if C1-C5 all select branch 1**. C6 is
recorded, never classified. The taxonomy is parameterised by cell kind, because
the two kinds carry different statistics:

| Symbol | C1-C4 (absolute) | C5 (ratio) |
| --- | --- | --- |
| `L`, `U` | unpaired bootstrap bounds on the cell's statistic | paired bootstrap bounds on the ratio of medians |
| `t` | the cell's ms ceiling (50 / 60 / 70 / 80) | `k = 1.3` |
| `h` | achieved upper distance, in ms | achieved upper distance, dimensionless |
| `h_target` | 1.0 ms (medians) / 2.0 ms (p90s) | 0.0036 at n = 1,700 |
| robustness | **none** (see The Criterion) | `true`-floor point estimate ≤ `k` |

Pre-registered, before the run:

1. **Pass** — `U ≤ t`; and for C5 only, the robustness condition also holds.
2. **Fail** — `L > t`.
3. **Indeterminate** — `L ≤ t < U`, or (C5 only) `U ≤ t` while the robustness
   condition fails. Escalate **once**, to the n the sizing rule gives for an
   upper distance of 0.0018 on C5 (≈ 6,900 Block A pairs) or half the ms target
   on C1-C4, then re-classify into branch 1, 2 or 4.
4. **Terminal indeterminate** — after the one permitted escalation the cell
   selects **neither branch 1 nor branch 2**, for whatever reason: the interval
   still straddles `t`, C5's robustness condition still fails, or
   `upper_distance > target_distance` (the cell never reached its precision
   target). Record which of the three caused it — that is what `upper_distance`
   and `target_distance` are classifier inputs **for**, and without this clause
   they would be declared parameters no branch reads. Record the achieved `h`
   and **which condition caused it**. Both qualifiers matter (SD-33).
5. **Invalidated session** — any per-sample decision mismatch, any inode/mtime
   change on the cached asset, launcher, `.minisig` or staged shim, any growth
   of the unverified log, **any verify-phase assertion failure** (a leaked
   `.tmp-*`, an orphaned `.accelerator-lock-*`, a surviving farm or scratch
   tree), or any precondition failure. Split by when it fires: **5a,
   pre-sampling or in-flight** — no figures are produced; **5b, post-run** —
   figures are computed but recorded as explicitly **non-gating**, with the
   failing witness named, so a number in hand is never mistaken for a verdict.
   The post-run inode/mtime witness and the drift diagnostic are both 5b, since
   neither can be evaluated before sampling ends.
6. **Design-infeasible** — split by cause, since the two differ in whether
   figures exist. **6a, a priori**: no n within the wall-clock budget reaches
   the escalation target (`sizing_feasible` false); no figures. **6b, mid-run**:
   the 35-minute budget is exhausted (`budget_exhausted`); partial figures exist
   and are recorded explicitly non-gating. 0205 established 6a does not fire at
   C5's margin.
7. **Not applicable** — the cell cannot be measured on this host at all: no
   `shasum`/Perl, so the fallback farm is unconstructible (C3, C4, C6), or the
   platform key carries no calibrated entry (any cell). Recorded with its
   reason; produces figures as **uncalibrated context** where any exist, never a
   verdict.

⚠️ Branch 7 exists because two states the plan itself enumerates mapped to no
branch: Phase 3 §2 records C3/C4/C6 "not applicable on this host" where Perl is
absent, and Phase 2 §2 has the harness refuse a gating verdict under an
uncalibrated platform key. Without a branch for them, `closure_verdict` was
false by construction on exactly the hosts the plan anticipates, and Phase 1's
outcome-keyed closure guard was unsatisfiable there. **`closure_verdict`
requires branch 1 on every *applicable* gating cell**; a branch-7 gating cell
needs a recorded, owner-named acceptance before the item closes, on the same
terms as an accepted deviation.

**Evaluated as an ordered cascade**, first match wins, so precedence is stated
rather than implied:

```
7  not applicable         →  5  invalidated         →  6a  sizing infeasible
→  6b  budget exhausted   →  4  escalation spent and neither 1 nor 2
→  2  L > t               →  3  straddles, or robustness fails
→  1  U ≤ t (+ robustness)
```

The order matters at two junctions the prose alone left ambiguous. Branch 3's
predicate is positional and carries no `escalations_used` term, so
post-escalation a straddle satisfies both 3 and 4 — the cascade puts 4 first, so
one escalation cannot be spent twice. And a validity failure coinciding with
infeasible sizing resolves to 5, because an invalid session's sizing is moot.
Branches 1, 2 and 3 still partition the position of `t` relative to one
interval, with C5's robustness condition folded into 1 and 3. **No sampling
beyond the single escalation branch 3 permits** — open-ended extension until a
bound crosses a threshold is optional stopping and voids the stated confidence
level.

⚠️ **Escalation is session-level, not per-cell.** One scalar `escalations_used`
governs the session, and the escalated run **replaces the initial run's
samples** rather than pooling with them — "initial run", not "pilot", since
§Sizing reserves *pilot* for the 400 discarded dispersion samples. So when *any*
cell selects branch 3, **all** cells are re-classified from the escalated run
alone, and the initial run's classifications are recorded as superseded. Both
alternatives are worse: escalating per-cell would spend the budget several times
over, and retaining a passing cell's pilot classification alongside an escalated
one would produce a cross-session record that the criterion's "same host, same
session" wording forbids. The consequence is stated plainly — a cell that passed
in the pilot can straddle its ceiling in the escalated run and take the session
to branch 4.

⚠️ The single escalation is a two-stage procedure, so the final interval's
coverage is not the nominal 95% unconditionally. Three things bound that: the
replacement rule above; ≥ 10,000 resamples, keeping Monte-Carlo error well below
the escalated target; and the recorded level described as **approximate under
the single escalation** rather than as an exact 95%.

Given 0205's `U = 1.2899` against `k = 1.3`, and the point-estimate form of the
robustness condition, branch 1 is the expected outcome for C5 — with 0.0101 on
the gating bound and 0.003 on the robustness point estimate. C1-C4 have 6.7 to
10.8 target-widths of margin against their ceilings. None of this is assumed:
the load-bias direction is unresolved, and a quiet host may return materially
different figures in either direction.

#### 7. Re-check the composition budget for closure

0205's independently measured term set, against its `median(G) = 42.28`:

| Term | median (ms) | Cross-checked |
| --- | --- | --- |
| shell bootstrap body (bash startup + 2 × `sha256_file` + logic) | 18.55 | derived |
| shim minisign-verify of the 8.23 MB launcher | 8.71 | yes |
| launcher startup + clap, net of fork floor | 3.54 | yes |
| `cache::find` | 0.06 | yes |
| `reverify` | 6.34 | yes |
| `vcs` exec + guard work, net of fork floor | 3.79 | yes |
| **sum** | **40.99** | — |

Signed residual +1.29 ms (+3.1% of `G`), absolute residual +1.29 ms — the two
coincide because no pair of terms disagrees in opposing directions.
Cross-checked fraction of `G`: **53%**, or **67.5%** counting the measured bash
floor inside the derived bootstrap body — so the uncross-checked share is
**32.5%** (SD-34).

**Re-measure the term set in the same session as the confirmatory `G`**, rather
than re-checking closure against 0205's figures. 0205 established every term is
reachable through the launcher's public library surface, so this is cheap, and
it removes a cross-session mismatch that is already visible in the published
numbers: 0205's decomposition session recorded full `G` at 41.58 ms while its
gating run recorded 42.28 ms, so ~0.7 ms of the +1.29 ms residual is session
difference rather than unattributed cost.

Each term is measured at a **pre-registered n = 200 with its own percentile
interval**, and those are propagated into an uncertainty on the sum. Report the
residual signed and absolute against a band of **max(±1.5 ms, propagated
uncertainty)**. The ±1.5 ms floor is narrower than the smallest lever this plan
costs and declines (0191's 2.48 ms and the cache-hit `sha256`'s 4.49 ms), so the
check can detect a term moving by as much as the decisions under discussion; the
propagated term stops the band being tighter than the measurement can resolve.

Re-measurement is capped at **two** attempts, every attempt recorded, and is
triggered by the residual's **magnitude** exceeding the band — never by its
sign.

⚠️ Term medians are **not** expected to sum to the `G` median, and a negative
residual remains a signal of double-counting or a misplaced boundary — it is
just no longer an unbounded trigger (SD-35).

Record the cache-root entry count and total size: `cache::find`
(`cache.rs:51-73`) scans a directory the module header declares needs no
eviction (`cache.rs:1-6`), so the scan term is history-dependent and not
comparable between a fresh install and a long-lived plugin root.

⚠️ **Measure the two `sha256_file` substitutions directly**, in-session, as a
`bash -c` bracket marginal over an empty-body baseline — the shape both 0186 and
0191 used, taking seconds. The dual-backend difference is the **cross-check** on
that measurement, not the measurement itself: the delta yields `2 × (shasum −
sha256sum)` ≈ 16.9 ms, whereas the quantity the budget needs is the ~7.1 ms
absolute cost under the gating configuration. Recovering the absolute figure
from the delta alone requires importing 0186's 3.55 ms from a different session,
so the backend delta alone cannot supply the absolute figure the budget needs
(SD-36). Record the direct measurement, the delta, their agreement, and whatever
share of `G` remains uncross-checked as a stated limit rather than as coverage.

### Success Criteria:

#### Automated Verification:

- [x] The recovered baseline runs and exits cleanly against the test envelope,
  and its raw envelope shape is probed and recorded before sampling
- [x] The fixture's blocked decision shape is asserted on a probe sample before
  sampling begins
- [x] Preconditions pass over the environment actually handed to
  `subprocess.run` — including the published-release check, the
  concurrent-session check, `LC_ALL`/`TZ`, and `jj`'s version against the
  `mise.toml` pin — and the observed `ACCELERATOR_*` keys, both `PATH` farms'
  contents and every resolved tool path and version are printed
- [x] The fallback farm positively fails `command -v sha256sum` and resolves
  `shasum`
- [x] Pre- and post-sampling instrument floors clear the ≤ 7.8 / ≤ 1.95 ms gate,
  with every retry attempt and its floors recorded, capped at three
- [x] Every sample's normalised decision matches the expected `block` for both
  variants, using Phase 2's five-case union; the run aborts on first mismatch
- [x] The inode/mtime witness runs **per sample** and the outlier trip and
  wall-clock budget are armed
- [x] The unverified log is byte-identical to its captured contents

#### Manual Verification:

- [x] `B`, `G`, the ratio and the two-sided 95% interval are recorded **per
  digest backend**, with n, min, median, p90, IQR per variant and
  `p90(G)/p90(B)` as context
- [x] All three ratios are recorded with intervals, in their three roles — raw
  gates, `true`-floor-subtracted is the robustness check that must also clear,
  bash-floor-subtracted is diagnostic only with its over-subtraction stated
- [x] `median(Gᵢ/Bᵢ)` is recorded alongside the ratio of medians, and a material
  divergence between them is reported as a drift finding
- [x] The first-third/last-third drift diagnostic is within the band §2 states
  (on `median(G)/median(B)`, not per variant)
- [x] **C1-C5** are each classified into exactly one branch and C6's figures are
  recorded without a branch, and `closure_verdict` held (every gating cell
  branch 1, or branch 7 with a recorded acceptance); C6's figures recorded
  without a branch
- [x] Quietness is evidenced two ways — raw load with its CPU-count rung, and
  both instrument floors against their gate — and the resolved `bash` matches
  the calibration provenance recorded on the platform entry (0205's session),
  not 0186's
- [x] Baseline provenance is recorded: resolved git commit id, and the sha256 of
  **both** files recovered into `T`
- [x] The envelope, fixture path and canonicalised depth, **observed** `dirname`
  spawn count from a `bash -x` trace, host/OS/chip, plugin version, tool paths
  and versions, interpreter, clock info, seed, locale and power state are
  recorded
- [x] The term set is re-measured in-session, the residual reported signed and
  absolute against the band §7 states, with re-measurement triggered by
  magnitude only and capped at two recorded attempts
- [x] The two `sha256_file` calls are measured directly in-session, with the
  backend delta reported as the cross-check and their agreement stated; the
  residual uncross-checked fraction is stated as a number
- [x] The cache-root entry count and total size are recorded
- [x] The harness invocation and its full output are recorded; the harness
  itself is cited by path, since Phase 2 committed it
- [x] `T` and the fixture root do not exist, asserted positively by recorded
  resolved path, and the cache-root entry set matches its captured list
- [x] The teardown's restore and verify phases both ran and passed, and `mise
  run cli:check` ran as a separate post-run step

---

## Phase 4: Discharge and Close

### Overview

Land the figures where they stop the obligation being orphaned a fourth time,
close 0189, and raise the five follow-ups the evidence supports.

### Changes Required:

#### 1. Discharge 0169 — four locations across three documents

Each location is found by **quoted text**, not line number. Phase 1 inserts
dated notes into two of these same documents, which shifts every anchor below
the insertion point — so line numbers recorded here would be stale by the time
Phase 4 runs, in the phase whose whole purpose is to stop an obligation being
orphaned a fourth time. (The line numbers in References are indicative only.)

- Fill the five `_pending_` slots (B, G, ratio, payload + fixture, host + OS)
  under `## Validation Results` in
  **`meta/work/0169-vcs-subdomain-and-hooks-migration.md`**, with a pointer
  here.
- Resolve the work item's own latency acceptance criterion — the one opening
  "Warm-call latency" — distinct from the Validation Results slots.
- Resolve the Phase 10 latency criterion in
  **`meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`**.
- Resolve the unchecked "Warm-call latency" item in
  **`meta/validations/2026-08-05-0169-…-validation.md`**.

The work-item-versus-plan split matters: the 0169 *plan* has no `## Validation
Results` section and no `_pending_` markers, so a step aimed there would find
nothing to fill.

**Each of the four stays unticked.** They assert `G ≤ 1.1 × B`, and that is not
what was measured. Each carries a dated resolution recording: the measured ratio
and interval; that the threshold was superseded by 0189's `G ≤ 1.3 × B`, itself
demoted to a historical comparison beneath an absolute budget; the reason — the
`B`/`G` work asymmetry, and that 1.1 was calibrated against a `B` cost model
attributing `jj` and `git` spawns to a guard that makes neither; and that the
obligation is discharged at 0189 rather than here. Ticking a criterion whose
stated threshold was not met would be the post-hoc relaxation this plan
pre-registers against, re-entering at the point of discharge.

⚠️ Each resolution also states that the criterion decision was taken **on 0189
rather than on 0169**, contrary to 0205's Recommendation that "0169 decides what
the criterion should be" before 0189 measures. Without that note the reframing's
authoritative location is discoverable only by reading both documents in full.

Each resolution carries an inline note wherever the method differed from what
the criterion states — n (1,700 against 20), the gate statistic (an interval
bound against a bare median rule), the demotion of the ratio beneath an absolute
budget, and the per-backend split are all candidates.

#### 2. Record 0189's figures and close the item

**File**: `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`

- Add a `## Validation Results` section carrying the outcome: both medians with
  dispersion, all three ratios with their intervals and roles, **the branch
  selected for each of C1-C5 by identifier, and C6's figures without a branch**,
  the in-session budget with its residual, the cross-checked fraction and the
  host record.
- Tick criterion **10** if and only if `closure_verdict` held — every gating
  cell C1-C5 in branch 1, or in branch 7 with a recorded acceptance; C6 is
  recorded, never classified. Do not restate the cell list as a conjunction here
  — restating it re-encodes the pre-branch-7 rule (SD-37). On any other outcome
  leave it unticked and record a re-opened obligation with a named owner and a
  follow-up work item; an accepted deviation is available only with an approver
  named outside this measurement and a rationale that does not appeal to the
  observed number.
- Tick criterion **11**'s latency clause once the figures are recorded.
- Set `status: done`, with the body `**Status**:` line updated in lockstep, if
  and only if every criterion is discharged and the outcome-keyed closure guard
  Phase 1 installed is satisfied.

**0189's criterion is authoritative for the criterion text; the harness's
per-platform table is authoritative for the numbers; this plan's [The
Criterion](#the-criterion) is a restatement of both.** Stated so a later ceiling
change has one place to land and two guarded mirrors, rather than six unranked
copies.

Note the corpus convention: `## Validation Results` otherwise appears in work
items, not plans. This plan keeps the harness invocation and the raw record here
because they are too large for the work item, and makes the **work item the
authoritative summary** with the plan as its appendix — stated explicitly so the
two cannot be read as competing records.

#### 3. Amend 0191 with the measured backend figures

Attach the measured per-backend `sha256_file` cost to 0191's **existing**
section whose heading reads "**The saving is backend-dependent.**" — located by
that quoted heading, not by line number, because Phase 1 §4 inserts a dated note
above it and shifts every anchor below. (`:54-58` at the plan's recorded
revision; indicative only.) State that 0191 is no longer a latency-gate
co-requisite under the reframed criterion, and that its case now rests on the
fallback-backend figure — ~11-12 ms against the fast backend's measured 2.48 ms,
roughly **4-5×** larger, not "an order of magnitude". Mark the fallback saving
as a **projection** until confirmed: 0191 itself flags the batched multi-file
form and its missing-file exit semantics as unverified on `shasum`.

#### 4. Raise the follow-ups

**Five** work items. Ids come from the work-creation producer rather than being
read off the corpus maximum, since a concurrent workspace may have claimed the
next id. Each is created **through the work-creation producer**, which fills the
full `templates/work-item.md` frontmatter (`type`, `id`, `title`, `date`,
`author`, `status`, `tags`, `producer`, `schema_version`, `last_updated`,
`last_updated_by`) and the body `**Kind**/**Status**/**Priority**/**Author**`
block. The executor sets only what the producer cannot infer: `kind`,
`priority`, `parent: "work-item:0136"`, `derived_from` back to this plan, and
`tags` (SD-38). Back-links are added on 0189 (all five), 0191 (the
cache-hit-sha256 and `sha2` items) and epic 0136. Refer to each item by its
title throughout.

- **Remove the cache-hit sha256 from warm dispatch** (`kind: task`, `priority:
  medium`). Raised **unconditionally**, not gated on Phase 3's branch: the
  cache-hit `sha256` is 4.49 ms and a third of the launcher-side cost, buying a
  name/content consistency check the minisign signature over the same bytes
  largely subsumes. Scope the removal to that call site only — `verify_binary`
  is shared with `fetch_verify_store`, where the digest arrives from the
  signature-verified manifest and does bind the bytes. Note that an mmap is
  **not** behaviour-preserving: two passes over a mapping of a user-writable
  file can observe different bytes, and truncation raises SIGBUS rather than a
  clean `Cache` error.

  ⚠️ Removal costs more than the `ChecksumMismatch` diagnostic. Minisign signs
  only **bytes**; nothing in the signature binds the asset's **name or
  version**, while `cache::find` selects the entry by its `{name}-{version}-`
  prefix — so today the name-derived digest comparison is what rejects an entry
  whose filename disagrees with its content. Without it, a stale copy, a botched
  manual cache edit or a version rename becomes a silent wrong-version
  (potentially downgraded) execution rather than a clean error. Make it an
  acceptance criterion that a cheap replacement preserves the name/version
  binding — e.g. verifying the sub-binary's reported version after exec, or
  keeping one digest comparison and dropping only the redundant second hash.

- **Close the `sha2` hardware-intrinsics gap** (`kind: spike`, `priority: low`).
  The question: *why does our sha256 run at a third of the hardware's rate, and
  which remedy is worth taking?* 0205 measured `verifier::sha256_hex` at 555
  MB/s against `openssl sha256` at 1,708 MB/s over the identical file on the
  same chip — a 3.1× shortfall affecting **every** sha256 the Rust binaries
  compute, not only this call site. BLAKE2b, with no hardware path at all,
  outruns it 2.6× (1.7184 ms against 4.4895 ms over 2,493,792 bytes), which
  inverts the assumption 0205's brief was written under. Candidate remedies to
  evaluate and cost: enabling the `sha2` crate's `asm`/intrinsics feature; a
  crate swap; a vendored assembly path; or accepting the gap and switching the
  corruption check to BLAKE2b, which minisign already computes. Acceptance is a
  recommendation with measured before/after per candidate, not a fix.

- **Measure warm dispatch on linux** (`kind: task`, `priority: medium`). State
  the darwin-only scope, that darwin-x64 and linux-arm64 have no CI lane, and
  that the transfer direction is an open question rather than a known one —
  0186's breakdown makes `G`'s bootstrap term overwhelmingly spawn cost while
  linux ships coreutils `sha256sum` universally, so the two effects push
  opposite ways. Prerequisites for *measuring*: a linux host with `jj` **at the
  `mise.toml` pin**, `git`, `jq`, `realpath`, `bash`, **`curl` or `wget`**
  (`bin/accelerator:145-159` hard fails without one), **`awk`** (both digest
  pipelines use it), a resolvable sha256 backend, **a published signed release
  for the tree's own version**, and network egress to the release base URL — no
  build, since the shipped musl artefact is fetched and verified. For
  *decomposing*: `rustup target add <musl triple>` plus a musl-capable linker
  natively; `cargo-zigbuild` and `ziglang` are the cross-from-darwin mechanism,
  not a native requirement. `reverify` ms-per-MB must be re-measured per
  platform and recorded against (architecture, SHA-extension support, libc)
  rather than the OS name. The harness is committed and its `PATH` is per-OS
  data rather than a darwin constant, so this item runs a task rather than
  re-authoring or patching a script.

- **Bound cache-root growth** (`kind: task`, `priority: low`). `cache::find`
  scans a directory the module header declares needs no eviction
  (`cache.rs:1-6`) on **every** warm dispatch, so the scan term grows without
  limit in accumulated versions and staged shims — 0.06 ms today at a handful of
  entries, unbounded over a long-lived plugin root. Size it from the entry count
  and total size Phase 3 records. Scope: a retention policy, or eviction on
  successful store. Without this item the measurement that would have prompted
  action is about to close, leaving a measured term on the hook path with no
  owner.

- **Own the recurring absolute-budget check** (`kind: task`, `priority:
  medium`). C1-C4 are designated *primary* on the ground that an absolute
  ceiling is re-runnable where a ratio against a deleted baseline is not — but
  nothing in this plan re-runs them: `measure:*` is outside `check`/`default`,
  the integration smoke check emits no gating figure, the ceilings are bound to
  a quiet darwin-arm64 host, and the instrument-floor gate is one no shared
  runner reliably clears. Without an owner, the primary gate is as one-shot as
  the ratio it displaced, and the first regression is discovered the way this
  one was — by a spike months later, which is the orphaned-obligation pattern
  this plan exists to end. Scope: a scheduled, **non-blocking**
  `measure:warm-dispatch` lane on a self-hosted or best-effort runner, recording
  a trend and alerting on ceiling breach, with the host-quietness caveat stated
  and uncalibrated platform keys reporting context rather than a verdict.
  **Striking the re-runnability argument is an acceptance criterion of this
  item**, fired by declining it: closing it `wontfix` requires the same change
  to remove that argument from 0189's rationale, leaving C1-C4 justified on
  user-perceptibility and the `B`/`G` asymmetry alone — the arguments that hold
  without it. Attaching the corrective action here, rather than as a conditional
  sentence on 0189, avoids landing a dangling obligation inside a document Phase
  4 sets `done` and nobody re-opens.

The committed-harness / measurement-policy decision the previous draft named as
a follow-up **dissolves**: Phase 2 commits the harness, which is the decision.

### Success Criteria:

#### Automated Verification:

- [x] `cargo nextest run --manifest-path cli/Cargo.toml -p accelerator-corpus -E
  'test(this_repositorys_own_corpus_is_clean)'` is green
- [x] `mise run check` is green
- [x] `mise run` (bare default task) exits 0 end-to-end

#### Manual Verification:

- [x] All four 0169 record locations carry the figures, each **unticked**, each
  with a dated resolution naming the superseded threshold, the measured value,
  the reason and where the obligation is discharged
- [x] Each 0169 resolution carries an inline note wherever the method differed
  from the criterion as written
- [x] Each 0169 resolution states that the criterion decision was taken on 0189
  rather than on 0169, contrary to 0205's stated sequencing
- [x] 0189 carries a `## Validation Results` section, named as the authoritative
  summary, with criterion **10** ticked only if every gating cell selected
  branch 1, and criterion **11**'s latency clause discharged
- [x] 0189's `status` and its body `**Status**:` line both reflect whether every
  criterion is discharged and the outcome-keyed closure guard is satisfied
- [x] 0191's existing section headed "**The saving is backend-dependent.**"
  carries the measured figures, with the fallback saving marked a projection and
  quoted as ~4-5×, not an order of magnitude
- [x] **Five** follow-up work items exist with producer-assigned ids, titles,
  `kind`, `priority`, parent and back-links on 0189, 0191 and 0136; the "Remove
  the cache-hit sha256 from warm dispatch" is present regardless of Phase 3's
  branch, and it carries the name/version-binding acceptance criterion
- [x] `last_updated`/`last_updated_by` refreshed on every meta document touched
- [x] **The Deviations section is complete, or explicitly records "none"**
- [x] The teardown's restore and verify phases both ran and passed

---

## Testing Strategy

### Unit Tests:

Phase 2's analysis core **and its extracted predicates** are the new production
code, and every function is TDD'd:

- Summary statistics over known vectors, including even and odd n for the
  median, and **both** the IQR's and p90's quartile conventions pinned
  explicitly against hand-computed vectors — p90 is reported per variant and
  `p90(G)/p90(B)` is recorded, and linear-interpolation versus nearest-rank
  differ materially at n = 1,700.
- The paired bootstrap: reproducibility under a fixed seed (same seed twice
  yields identical bounds, driven by an **injected** `random.Random` rather than
  the module global, so tests stay order-independent), plus behavioural
  assertions a golden value cannot give — `L ≤ point estimate ≤ U`; a wider
  confidence level yields a wider or equal interval; zero-variance input
  collapses to zero width; the resample count is honoured; and **unequal-length
  vectors raise** rather than being silently truncated by `zip`, which would
  misalign pairs into a confident wrong interval.
- The sizing rule: a round-trip, plus a deliberately non-integral case asserting
  the returned n rounds **up** (a truncating `int()` passes an approximate
  round-trip while systematically under-sampling, which is how branch 3's
  escalation would land in branch 4 for a purely arithmetic reason), and that
  `target ≥ h₀` never returns fewer samples than the pilot.
- Pair generation: within-pair order is randomised under the seed, reproducible
  across runs, and every pair contains both variants exactly once.
- Envelope normalisation over all five union cases, `unrecognised` included —
  legacy block, legacy allow, Rust deny, warn at both positions, empty stdout
  and unparseable stdout — driven as a parametrised table from literal envelopes
  taken from `hooks.rs:29-31` and the committed `decision-table.json`. Deny must
  **not** normalise to `allow`; empty must normalise to `degraded`.
- Each extracted predicate in Phase 2's table, including `validate_sample`
  rejecting every degraded shape and `log_appended_lines` escalating rather than
  truncating.
- The outcome classifier over its full input tuple, parametrising all six
  branches including `L == threshold` and `U == threshold` exactly, a spent
  escalation, an invalidated session that must select 5 regardless of the
  bounds, and an infeasible sizing that must select 6.
- **A lockstep guard** over **all** the pre-registered constants, not the ratio
  threshold alone — the four ceilings, `k = 1.3`, the two target distances, the
  two floor gates, the drift band, the residual band and the ≥ 10,000 resample
  floor. It binds the per-platform table in `tasks/measure.py` to a **`###
  Criterion constants` block in `tasks/README.md`**, bidirectionally: every
  constant appears in the doc block, and every number in the doc block resolves
  to a named constant — the shape `test_registration_docs.py` uses.

  ⚠️ It deliberately does **not** read `meta/`: no test under `tests/` does, and
  corpus documents are guarded by `this_repositorys_own_corpus_is_clean` instead
  (SD-39). 0189's criterion cites the harness table as the authoritative numeric
  source, so the two cannot drift without the doc block changing.

  ⚠️ This guard imposes **no** phase ordering: both sides of the binding —
  `tasks/measure.py` and `tasks/README.md` — are Phase 2 artefacts. An earlier
  draft carried an ordering constraint over from the superseded `meta/`-reading
  form, which contradicted the dependency graph's independent roots. The genuine
  remaining constraint is only that Phase 1's criterion text and the harness
  table agree by the time Phase 4 closes the item.

### Integration Tests:

One: **`test:integration:measure`**, the live-dispatch smoke check (`n = 2`,
floors only, no gating figure) that owns the `measure:*` namespace against rot.
Its registration is decided in Phase 2 §2 and is **not** restated here:
`_NO_LAUNCHER_NEEDED` (it fetches the released launcher and builds nothing, so
no `build:cli:dev` edge), `_NOT_IN_INTEGRATION_ROLLUP` with the reason "live
release fetch", and a dedicated non-blocking job (SD-40). It must **not** live
in `tests/unit/tasks/`: that tree is hermetic and is reached from the bare
default task, so a real bootstrap dispatch there would need network egress and a
published release for the tree's own version.

It also carries a cheap **rot guard for the recovery contract** — the most
fragile of the volatile contracts the self-test exists to protect, and the one
and the one previously uncovered (SD-34): assert the recorded commit id still
resolves and that both recovered files' sha256 match their recorded provenance
digests.

⚠️ That guard needs history the lane must be configured for. A GitHub checkout
has no `.jj`, and `actions/checkout` defaults to a shallow fetch — `fetch-depth:
0` appears on only two unrelated jobs — so neither `jj file show -r
cf42441e2aad-` nor `git show <commit>:hooks/vcs-guard.sh` resolves there. The
owning job sets `fetch-depth: 0` and uses the git form; if that is declined, the
guard moves into the operator-run pre-flight where jj history is guaranteed and
the lane keeps only the hermetic digest-constant check.

The `test_mise.py` namespace guard and the `test_python_coverage.py` discovery
confirmation both live in `tests/unit/tasks/` and run under `mise run
test:unit:tasks`, not under this heading (SD-41).

### Manual Testing Steps:

1. Run Phase 1's document edits and confirm the corpus frontmatter gate is
   green.
2. **Run the pre-flight capture** — before Phase 2's end-to-end trial, not only
   before Phase 3. That trial dispatches through the real bootstrap and creates
   a fixture, so without a prior capture anything it leaves is silently baked
   into the baseline Phase 3's integrity assertions are measured against.
3. Build Phase 2's harness test-first, and confirm `mise run` is green with the
   `measure:*` namespace registered and excluded from `check`. Run the
   fault-injection and SIGINT rehearsals.
4. Re-run the pre-flight capture for the measurement session. Recover the
   baseline into `T`; probe its envelope shape; run Block A and Block B in
   alternating segments on a quiet darwin host in one session. Confirm quietness
   against the pre-sampling floor gate, not only the load average.
5. Classify **C1-C5** into exactly one branch each, record C6's figures without
   a branch, and evaluate `closure_verdict`; re-measure the term set in-session
   and check closure.
6. Discharge 0169's four locations, record 0189's results, close 0189, amend
   0191, raise the five follow-ups.
7. Run the teardown's verify phase and confirm every throwaway artefact is
   positively asserted absent.

## Performance Considerations

This plan changes no shipped code and adopts no instrumentation. 0205
established that the launcher-side decomposition is reachable through the
crate's public library surface — `verifier::sha256_hex`, `verify_binary`,
`TrustedKeys::embedded`/`verifies`, `cache::find`, `Fetcher::new`,
`tls::install_crypto_provider` are all already `pub`, and
`cli/launcher/src/lib.rs` is a real lib target — so no dev-override route and no
locally signed launcher is needed.

If instrumentation is ever added, note that it must write to stderr or a
dedicated file, **never stdout**: stdout is the guard's decision envelope
(`hooks/hooks.json:47`, `kernel/src/hooks.rs:29-31`), and an unparseable
decision from a fail-safe guard lets the blocked `git` command proceed. Note
also that a dev-override route's residual risk outlives the source revert — the
compiled binary sits in gitignored `cli/target/`, exactly where a contributor's
normal dev-launcher build lives, so the artefact must be removed or rebuilt
rather than left unreferenced.

⚠️ 0205's `reverify` replica was verified by reading `resolve/mod.rs:90-109` at
revision `2bb98478e7f7`, not by differential execution. If Phase 3 re-measures
terms, re-read that range at the revision of the re-measurement — a refactor of
the private method would silently invalidate the replica.

## Migration Notes

None. No shipped artefact changes.

## Validation Results

### Pre-flight capture

_Pending Phase 3._ Slots: the **working-copy**
`sha256(keys/accelerator-release.pub)` compared against the published constant
`0f3fe9a9…`; digests of the cached launcher, its `.minisig` and the staged shim;
the cache root's sorted entry list; the captured `jj diff --summary` over `keys/
bin/ hooks/ scripts/ cli/` and the three per-file digests; the unverified log's
byte count and full contents, or its absence; the operator's pre-existing
dev-launcher state; the canonical temp root; and the artefact manifest's
resolved paths.

### Method

Closed by work item 0205 and restated concretely in Phase 3. Slots: a pointer to
0205's recorded answers for SQ-1 to SQ-5, and the `reverify` sub-operation
figures it delivered, so this record is readable standalone.

### Criterion reframing

Recorded 2026-08-13.

**The criterion as landed.** `meta/work/0189-…:## Latency Criterion`, a new
section carrying the cell table C1-C6, the statistics by cell kind, the sizing
table with `h₀ = 0.0086` and its two blocks, the floor treatment's three roles,
the seven-branch taxonomy with its ordered cascade and session-level escalation
rule, the superseded threshold with its measured 1.2813, the band's provenance
(author instruction in conversation on 2026-08-13, approver Toby Clemson), the
five-part reframing rationale, and seven stated limitations. That section is
named authoritative for the criterion text, with `tasks/measure.py`'s
per-platform table authoritative for the numbers. 0189's criterion **10** was
replaced wholesale and now states the reframed gate, its backend binding, the
interval-upper-bound rule and the superseded 1.1 with its measured value.

**Set A — the release gate.** The recorded search returns **15 lines across 10
passages** over the four documents: nine lines in **five** passages of 0189
(`:127-128`, `:199`, `:244-246`, `:305-306`, `:317`), three lines in two
passages of the 0169 validation (`:30`, and `:155-166`'s three-item list), one
in the 0169 plan (`:1517`), and two describing the release cut as a *process*
prerequisite: `meta/work/0169-…:524` and `meta/plans/2026-08-05-0169-…:200`.

⚠️ The plan's §2 states "nine matching lines across seven passages" in 0189. The
line count is right; the passage count is five, not seven, on any grouping that
keeps a single bullet or paragraph as one passage. Recorded rather than
reconciled, since the searches — not the counts — are the specification.

Every passage asserting that the asset or release does not exist carries a dated
retraction: 0189's Requirements bullet, its Dependencies passage, and both
Drafting Notes passages; the 0169 validation's Phase 10 line and its "Three
manual/release-gated items" passage (whose item 2 and item 3 are named
individually); and the 0169 plan's Phase 10 latency criterion. 0189's criterion
10 needed none, being replaced wholesale.

The two process-prerequisite matches are **deliberately left unretracted**:
0169's work item `:524` says the release "is **not** produced by this story's
code changes — it requires a release run and the minisign signing key", and the
0169 plan `:200` scopes the cut out of its own changes. Both remain true
statements about that story's scope; neither asserts the release has not
happened. Retracting them would be a rewrite, not a retraction.

**Set B — the superseded threshold.** `rg -n '1\.1 ×' meta/work/0189-*.md`
returned **five** occurrences: Requirements (`:127`), criterion 10 (`:198`),
the Dependencies co-requisite (`:251`), Assumptions (`:269`) and Drafting Notes
(`:303`). Criterion 10 was replaced; the other four each carry a dated
retraction. The Assumptions retraction records that the bullet's own escape
clause ("If the epic has since revised the threshold, this item follows the
epic") has fired, and that the revision was landed on 0189 rather than 0169.
The co-requisite retraction carries 0205's arithmetic: 0191's measured 2.48 ms
against a 5.98 ms overrun, so it was never sufficient, and it is not required
under the reframed criterion at all.

The release-cut blocker in Dependencies is replaced by an **outcome-keyed
closure guard** — 0189 may not close while any applicable gating cell selects a
branch other than 1, absent a recorded owner-named acceptance — and the
`blocked_by: ["work-item:0169"]` edge is retained with a dated note recording it
satisfied.

**0189's reconciled criteria.** Criteria 1-9 and 12 ticked, with a per-criterion
evidence table naming the discharging test or recorded section in the sibling
plan, and criterion 6's warm-hit-under-Mutation-A clause attributed to the
**validation report** rather than the plan. Criterion 11 was amended to name
which plan records which artefact and carries a dated note discharging its
non-latency clauses; it stays unticked, as does criterion 10, until Phase 4.

**0191.** The "~2.5 ms is essentially the whole of 0169's ~2.4 ms shortfall"
framing is retracted with the measured 5.98 ms overrun and the measured 2.48 ms
saving, cross-referencing 0191's existing "The saving is backend-dependent"
section rather than restating it.

**0205.** Verified already discharged on 2026-08-13 ahead of this phase:
`status: done` with its body line in lockstep, all eight criteria ticked (7 and
8 against the examples-target deviation, recorded in a note beneath the list),
the stale `Blocks:` line retracted at `:205`, and all three corrections appended
under a `## Corrections` heading at `:652`.

**The sibling plan's Mutation A cell.** Verified already corrected ahead of this
phase: the observed table reads PASS for
`a_warm_hit_never_probes_the_cache_root` under Mutation A, and a dated
"Correction, 2026-08-12 (validate-plan)" note beneath it records the original ✗,
the rerun over the complete 25-test binary (6 failed / 19 passed) and that the
rerun also discharges criterion 6's last clause.

### Phase 3 hand-off

Recorded 2026-08-13. **The harness is complete and rehearsed; the session
itself is handed off**, because it cannot be run faithfully from inside a Claude
Code session. Every tool call the driving session makes dispatches
`bin/accelerator` through its SessionStart and PreToolUse hooks, and the
criterion gates on instrument floors with "the operator issues no tool calls
while sampling" as a stated operating condition. The concurrent-session
precondition, run on this host during development, found **five** other `claude`
processes live — exactly the interference it exists to refuse.

**What the operator runs**, on a quiet darwin-arm64 host with no other Claude
Code session active:

```bash
mise run measure:warm-dispatch -- --rehearse   # optional: proves the path, non-gating
mise run measure:warm-dispatch                 # the recorded session
```

The record lands at `meta/measurements/warm-dispatch.json` and carries every
slot the sections below name. `mise run measure:teardown` is the escape if a run
dies; the harness refuses to start while that run's manifest survives.

**Prerequisites the harness asserts and records** (a failure is branch 5a, no
figures): no `ACCELERATOR_*` override but the permitted release-base-URL seam;
the driving session's plugin root equal to the measured one; `jj` at the
`mise.toml` pin; no other `claude` process outside this session's own ancestry;
a clean `jj diff --summary` over `keys/ bin/ hooks/ scripts/ cli/`; the
working-copy release key matching its published digest; a published signed
release for the tree's own version; both digest backends resolvable; and both
instrument floors inside their gate within three recorded attempts.

**What the rehearsal already establishes**, at a token sample count on a
deliberately *noisy* host — recorded as corroboration of the design, never as a
figure:

| Quantity | Rehearsed | 0205 / predicted |
| --- | --- | --- |
| `reverify` | 6.31 ms | 6.34 ms (measured) |
| launcher startup net of the fork floor | 3.52 ms | 3.54 ms (measured) |
| `cache::find` | 0.02 ms | 0.06 ms (measured) |
| `verifier::sha256_hex` over 2.49 MB | 4.45 ms | 4.49 ms (measured) |
| `TrustedKeys::verifies` (BLAKE2b + Ed25519) | 1.72 ms | 1.72 ms (measured) |
| `median(G)` fallback backend | ~59.8 ms | ~59.2 ms (predicted) |
| C6 fallback ratio | ~1.94 | ~1.79 (predicted) |
| cross-checked fraction of `G` | 70.3% | 67.5% |

The launcher-side terms reproduce 0205's to within 1%, which is the strongest
available evidence that the committed harness measures what the spike measured.
The fallback prediction is corroborated in direction and magnitude. **The ratio
cells are not corroborated and must not be read as such** — a noisy host is
where the ratio is least trustworthy, which is the whole reason the session is
gated on quietness.

### Attempt 1 — invalidated (branch 5b)

Recorded 2026-08-17 on darwin-arm64, full record at
`meta/measurements/warm-dispatch-1.json`. **Every figure below is explicitly
non-gating**, because the session selects branch 5b. It is kept as the record of
an attempt, not as evidence about the criterion.

**The failing witness is the drift diagnostic**: Block A's first-third ratio
1.3175 against its last-third 1.3043, |Δ| = 0.0132 against a pre-registered band
of 0.005. Nothing else failed — both instrument floors cleared their gate at
both ends (5.65 / 1.69 ms pre, 6.07 / 1.43 ms post), no precondition was
violated, no `ACCELERATOR_*` key was set, `jj` matched its pin, the cache was
warm, the warm-up dispatch blocked, and the teardown verify passed with every
artefact absent.

| Cell | Statistic | Interval | Ceiling | Branch had the session been valid |
| --- | --- | --- | --- | --- |
| C1 | `median(G)` fast | 40.96 [40.88, 41.06] | ≤ 50 | 1 |
| C2 | `p90(G)` fast | 45.96 [45.80, 46.17] | ≤ 60 | 1 |
| C3 | `median(G)` fallback | 55.47 [55.23, 55.87] | ≤ 70 | 1 |
| C4 | `p90(G)` fallback | 62.60 [62.07, 62.91] | ≤ 80 | 1 |
| C5 | ratio, fast | 1.3177 [1.3149, 1.3207] | ≤ 1.3 | **2 — fail** |
| C6 | ratio, fallback | not recorded (estimator defect; 1.7846 from the medians) | — | ungated |

⚠️ **The asymmetry is the finding worth carrying into attempt 2.** The primary
gate — the absolute budget C1-C4, designated primary precisely because it is
re-runnable and bounds what users feel — clears every ceiling with 18% to 26%
of headroom. The cell that fails is C5, the *demoted historical comparison*,
and it fails with its whole interval above the threshold at an achieved upper
distance of 0.0030, tighter than the 0.0036 target. That is a decidable fail,
not an indeterminate one.

The mechanism is visible in the dispersion: `G` came in **faster** than 0205's
(median 40.96 against 42.28 ms) while `B` came in faster still (31.09 against
33.00 ms), so the ratio rose even as the absolute figure improved. Both
floor-subtracted treatments agree — `true`-floor 1.3359, bash-floor 1.3883 —
and `median(Gᵢ/Bᵢ)` at 1.3180 sits on top of the ratio of medians at 1.3177,
so the two estimators do not diverge.

Sizing: the in-session pilot measured an upper distance of 0.0209 over 200
pairs and sized Block A up to **6,762 pairs**, inside the 6,900 cap; the
session ran 589 s, inside the 35-minute budget. Composition budget: seven terms
summing to 30.11 ms against an observed 40.96, residual −10.85 ms and 73.5%
cross-checked. `dirname` spawns: 1.

⚠️ **A load average of 38.25 over 16 CPUs was recorded** while both floors
passed. Load is not a gate here and the direction of its bias on the ratio is
unresolved, so this is context rather than an explanation — but it is consistent
with a host that was not in a steady state, which is what the drift diagnostic
detects.

### Attempt 2 — the measured session

Recorded 2026-08-17 on a quiet darwin-arm64 host (Apple M4 Max, macOS 26.3),
full record at `meta/measurements/warm-dispatch-2.json` with its raw samples
beside it at `warm-dispatch-2-samples.json`. n = 1,700 Block A pairs and 900
Block B samples, 192 s, seed 20260813, ≥ 10,000 resamples per interval.

**This is the session the criterion should be read against.** Load 10.63 over 16
CPUs against attempt 1's 38.25; instrument floors 4.85 / 1.40 ms pre and 4.62 /
1.39 ms post, against a gate of ≤ 7.8 / ≤ 1.95 and against 0205's own 7.04 /
1.72; per-variant IQRs a third of attempt 1's (1.21 / 1.20 / 1.62 against 3.99 /
3.99 / 5.83). The pilot's achieved upper distance was 0.0051, so no size-up was
needed and Block A ran at its tabled 1,700.

| Cell | Statistic | Interval | Upper distance | Ceiling | Branch |
| --- | --- | --- | --- | --- | --- |
| C1 | `median(G)` fast | 37.560 [37.498, 37.619] | 0.058 | ≤ 50 | 1 but for validity |
| C2 | `p90(G)` fast | 39.245 [39.051, 39.490] | 0.245 | ≤ 60 | 1 but for validity |
| C3 | `median(G)` fallback | 52.004 [51.886, 52.109] | 0.105 | ≤ 70 | 1 but for validity |
| C4 | `p90(G)` fallback | 54.450 [54.155, 54.833] | 0.384 | ≤ 80 | 1 but for validity |
| C5 | ratio, fast | **1.3423 [1.3395, 1.3445]** | 0.0022 | ≤ 1.3 | **2 — fail** |
| C6 | ratio, fallback | 1.8585 [1.8527, 1.8635] | 0.0050 | ungated | recorded |

Dispersion: `B` n = 1,700, min 26.534, median 27.982, p90 29.795, IQR 1.212.
`G-fast` n = 1,700, min 35.448, median 37.560, p90 39.245, IQR 1.199.
`G-fallback` n = 900, min 50.042, median 52.004, p90 54.450, IQR 1.621.
`p90(G)/p90(B)` = 1.3171 as context.

**All three floor treatments in their three roles**: raw medians gate at
**1.3423**; the `true`-floor-subtracted robustness check is **1.3603**, so it
fails too and in the same direction; the bash-floor-subtracted diagnostic is
1.4140, over-subtracting as stated. `median(Gᵢ/Bᵢ)` is 1.3409 against a ratio of
medians of 1.3423 — the two estimators agree to 0.0014, so pairing is not
carrying a drift artefact.

**❌ C5 fails, and quietness made it worse.** Both variants sped up against
attempt 1 — `G` 40.96 → 37.56, `B` 31.09 → 27.98 — but `B` sped up
proportionally more, so the ratio rose from 1.3177 to 1.3423. Both sessions put
the entire interval above 1.3; this one does so at an upper distance of 0.0022,
tighter than the 0.0036 target, so the fail is decidable rather than
indeterminate. Against 0205's 1.2813 the direction is consistent: every session
this plan has evidence for exceeds the threshold, and the two taken under this
harness exceed it by 18 to 65 achieved upper-distances.

⚠️ **In absolute terms the miss is 1.25 ms.** Bringing C5's upper bound to 1.3
requires `median(G)` at 36.31 rather than 37.560. Work item 0191 — batching the
bootstrap's two `sha256_file` substitutions into one invocation — is measured at
2.48 ms on the fast backend, and this session measured those two calls at 4.316
ms. One lever is roughly twice the gap.

**The primary gate is not close to breaching.** C1-C4 clear their ceilings by
25%, 35%, 26% and 32%, and every one improved on attempt 1. The cell that fails
is the one the criterion itself demotes to a historical comparison.

**Re-classified 2026-08-17 against the raised threshold.** C5's ceiling is now
**1.4**. Against it this session's C5 reads 1.3423 with an upper bound of 1.3445
— a pass with 0.0555 to spare, about 25 achieved upper-distances — and the
robustness check reads 1.3603 [1.3574, 1.3627], passing in both its stated
point-estimate form and the upper-bound form that becomes decidable at 1.4. **All
five gating cells would therefore select branch 1 but for the session's validity.**
`closure_verdict` remains false only because the drift diagnostic invalidates the
session; nothing in the figures blocks closure at 1.4.

**⚠️ The session is nonetheless invalid (branch 5b), on drift.** Block A's
first-third ratio 1.3388 against its last-third 1.3479, |Δ| = 0.00915. Attempt 1
failed the same check at −0.0132, in the opposite direction, so this is
session-scale wander rather than a thermal ramp, and it is not attributable to
load: attempt 2's load was a quarter of attempt 1's and its dispersion a third,
yet it still drifted.

**The band was re-derived from this session's own samples** — permuting the pair
order to build the no-drift null — and the invalidation survives it. The 0.95
quantile of that null is **0.00615** against the observed 0.00915, and
`P(|Δ| ≥ observed | no drift) = 0.0050`; the verdict is unchanged at every
quantile up to 0.99. The superseded constant of 0.005 turns out to sit at the
**88.8th** percentile of the null, so its false-positive rate was an unstated
~11% — it was too tight, not unattainable as this plan's earlier note claimed.

⚠️ **The drift does not explain C5's level.** Sliced into ten equal windows in
collection order the ratio runs 1.3364, 1.3353, 1.3415, 1.3421, 1.3404, 1.3411,
1.3373, 1.3427, 1.3569, 1.3506 — a spread of 0.0216 with **every window above
1.3 by at least 0.035**. So the C5 finding is robust to the invalidation: no
slice of this session approaches the threshold.

**Composition budget**, re-measured in-session: bash startup 4.848, two
`sha256_file` calls 4.316, shim minisign-verify of the launcher 6.435, launcher
startup net of the fork floor 2.494, `cache::find` 0.020, `reverify` 5.954, and
`vcs` exec plus guard work net of the fork floor 2.376 — summing to 26.443
against an observed median of 37.560, a signed residual of **−11.117 ms** against
a ±1.5 ms band, and **70.4%** of `G` cross-checked. The sub-operations inside
`reverify` are recorded but not summed. The dual-backend cross-check gives a
delta of **+15.219 ms**, or 7.61 ms per call, against 0186's 8.44 ms — the right
sign and order, confirming the direct figure. Cache root: 21 entries, 47.8 MB.

**Provenance**: plugin version 1.24.0-pre.41 with a published signed release;
`jj` 0.43.0 matching the `mise.toml` pin; no `ACCELERATOR_*` key set; no
concurrent Claude Code session; no warm-cache gap; dev-launcher marker absent;
fixture depth 9 with an observed `dirname` spawn count of **1**; warm-up
dispatch validated as blocking; both farms 28 tools with every link's realpath
and version recorded; `LC_ALL=C`, `TZ=UTC`; CPython 3.14.4 with
`mach_absolute_time()` as `perf_counter`; teardown restore and verify both
passed with every artefact positively absent and the cache-root entry set
unchanged.

⚠️ **The verdict is uncalibrated, and that is a defect in the table rather than
in the host.** The criterion demotes a verdict to context when the observed host
disagrees with the platform entry's calibration provenance. Two of that
provenance's four fields — resolved `bash` and `shasum` — were **never recorded
by 0205** and had been filled in with plausible values that no session measured.
They are now `None`, and an unrecorded field is treated as unconfirmable rather
than as agreement, so this host reports "uncalibrated: the 0205 session recorded
none". The chip matches (Apple M4 Max). The consequence for reading this record:
the *absolute* cells are judged against ceilings whose calibration cannot be
confirmed on the instrument-identity axis, while C5, being a within-session
ratio, is unaffected by it.

### Attempt 3 — the valid session, and the one the criterion is met on

Recorded 2026-08-17, `meta/measurements/warm-dispatch-3.json` with its raw
samples beside it. **`closure_verdict` holds.** Quietest of the three by every
measure: load 3.81 over 16 CPUs, instrument floors 4.449/1.339 ms before and
4.255/1.349 ms after, each clearing on the first attempt at each end. n = 2,659
interleaved pairs after the in-session pilot sized Block A up from 1,700 (achieved
pilot upper distance 0.0131), plus 900 Block B samples; 247 s.

| Cell | Statistic | Interval | Ceiling | Headroom | Branch |
| --- | --- | --- | --- | --- | --- |
| C1 | `median(G)` fast | 35.531 [35.467, 35.584] | ≤ 50 | 29% | **1** |
| C2 | `p90(G)` fast | 38.230 [37.979, 38.427] | ≤ 60 | 36% | **1** |
| C3 | `median(G)` fallback | 51.496 [51.411, 51.616] | ≤ 70 | 26% | **1** |
| C4 | `p90(G)` fallback | 55.291 [54.889, 55.666] | ≤ 80 | 31% | **1** |
| C5 | ratio, fast | 1.3260 [1.3236, 1.3279] | ≤ 1.4 | 5.2% | **1** |
| C6 | ratio, fallback | 1.9218 [1.9172, 1.9266] | recorded | — | 2, ungated |

Dispersion — `B`: n = 2,659, min 25.182, median 26.796, p90 28.896, IQR 1.192.
`G-fast`: min 33.999, median 35.531, p90 38.230, IQR 1.642. `G-fallback`: n = 900,
min 49.402, median 51.496, p90 55.291, IQR 1.903. `p90(G)/p90(B)` 1.3230.

Three floor treatments in their three roles: raw medians **gate** at 1.3260; the
`true`-floor-subtracted **robustness check** is 1.3432 [1.3407, 1.3451], clearing
1.4 in the stated point-estimate form *and* the upper-bound form, so the
deliberate weakening decided nothing; the bash-floor-subtracted **diagnostic** is
1.3909. `median(Gᵢ/Bᵢ)` 1.3308 against the ratio of medians 1.3260, agreeing to
0.005.

**Validity holds.** Drift −0.00308 against a band of 0.00527 derived from this
session's own permutation null at the 0.95 quantile, `p = 0.228`; it also holds
under the superseded 0.005 constant, so no verdict turns on the change of basis.
Every per-sample check passed; the inode/mtime witness, the outlier trip and the
wall-clock budget never fired.

**Composition budget**: bash startup 4.449, two `sha256_file` calls 3.824, shim
minisign-verify 6.471, launcher startup net of the fork floor 2.218,
`cache::find` 0.036, `reverify` 6.049, `vcs` exec plus guard work net of the fork
floor 1.996 — summing to 25.042 against an observed 35.531, residual −10.49 ms,
**70.5%** cross-checked and a **29.5%** uncross-checked share stated as a limit.
Backend cross-check +15.753 ms (7.876 per call against 0186's 8.44). Cache root
21 entries, 47.8 MB.

⚠️ **C5 does not meet 1.3**, which was the threshold until the same day. It misses
by **0.747 ms** of `median(G)` — a shortfall 0191's measured 2.48 ms covers three
times over, which is why that item now carries a re-measurement criterion.

⚠️ **The verdict is uncalibrated on two provenance fields**: 0205 recorded neither
the `bash` nor the `shasum` it resolved, so this host's `bash` 5.3.15 and
`shasum` 6.02 confirm nothing. The chip matches. C5 is a within-session ratio and
unaffected; the absolute ceilings are the cells whose instrument identity cannot
be confirmed.

### Latency figures

_Superseded by the two attempts recorded above; retained as the slot list._
Slots: `B`, `G` and the two-sided 95% interval **per digest
backend**; all three ratios with their intervals and their three roles (raw
gates, `true`-floor robustness check, bash-floor diagnostic); `median(Gᵢ/Bᵢ)`
alongside the ratio of medians; n/min/median/p90/IQR per variant;
`p90(G)/p90(B)`; the branch selected **for each of C1-C5 by identifier, and C6's
figures without a branch**; the first-third / last-third drift diagnostic;
pre-sampling instrument floors against the gate The Criterion states, with every
retry attempt; stdin envelope verbatim; both raw envelope shapes including `B`'s
probed shape; fixture path, canonicalised depth and **observed** `dirname` spawn
count from the `bash -x` trace; baseline provenance (resolved git commit id,
sha256 of both files recovered into `T`); the harness invocation and full
output; host/OS/chip; plugin version and the published-release assertion; both
`PATH` farms' contents with every resolved tool path and version — including
every executable `B` spawns; `LC_ALL` and `TZ`; `jj`'s version against the pin;
interpreter, `perf_counter` clock info and bootstrap seed; machine load with its
CPU-count rung; power state; the fallback prediction and whether it held.

### Composition budget

Recorded from attempt 3 above; the term set, the residual against its band, the
direct digest measurement with its backend cross-check, the cross-checked and
uncross-checked fractions, and the cache-root count and size are all in that
section. 0188's 4.81 / 4.03 ms pair remains the recorded bound on fixture bias.
Slots: the **in-session** re-measured term set against the
confirmatory `median(G)`; the residual signed and absolute against the ±1.5 ms
band §7 states, with the magnitude-only trigger and the two-attempt cap
honoured; the two `sha256_file` calls measured **directly** with the backend
delta as cross-check and their agreement stated; the cross-checked fraction of
`G` as a number and the residual uncross-checked fraction as a stated limit; the
cache-root entry count and total size; 0188's 4.81 / 4.03 ms pair as the
recorded bound on fixture bias.

### Cleanup evidence

All three attempts: the per-sample inode/mtime witness ran on every sample and
never fired, and neither the outlier trip nor the wall-clock budget fired in any
session. Teardown restore and verify passed on all three, with every artefact in
the manifest table positively absent by recorded resolved path and the cache-root
entry set matching its captured list. The unverified log was byte-identical
throughout. Slots: the **per-sample** inode/mtime witness and whether the
outlier trip or wall-clock budget fired; positive absence of `T` and the fixture
root by recorded resolved path; the cache-root entry set against its captured
list; the unverified log's byte-identity (append-only — any growth is a branch-5
invalidation, never a cleanup); the restore and verify phases' output.

### Deviations

_Pending._ Known already, and to be recorded regardless of what else arises:

- **The threshold was relaxed after seeing the number** (SD-5). The reframing
  from `G ≤ 1.1 × B` to `G ≤ 1.3 × B` was decided with 0205's 1.2813 in view,
  and the resulting margin is 0.0187. Mitigations are in Desired End State; the
  departure itself is the deviation, and a pass must not be recorded as a
  comfortable one.
- **The gate statistic is an interval bound, not a bare median comparison**,
  with a `true`-floor-subtracted robustness check alongside. 0169's criterion
  reads as a median rule; this is a deliberate strengthening of the statistic
  alongside a relaxation of the threshold.
- **The ratio is demoted beneath an absolute budget.** 0169's obligation is a
  ratio; this plan makes a re-runnable absolute `median`/`p90` ceiling the
  primary gate and keeps the ratio as the historical comparison, because `B` is
  a deleted artefact no CI lane can reproduce. That is a change in the *kind* of
  criterion, not only its value.
- **n departs from 0169's stated 20**, to 1,700 Block A pairs plus 900 Block B
  samples. At n = 20 the interval's upper distance vastly exceeds the margin
  under test, which is why the stated sample count — not the gate — was
  infeasible.
- **The robustness condition is a point-estimate test, not an interval bound.**
  A deliberate, pre-registered weakening: its 0.003 margin is smaller than the
  upper distance achievable at any affordable n, so an interval form would make
  branch 1 unattainable and the expected outcome branch 3 by construction.
- **The criterion is split per digest backend and per statistic**, into the six
  identified cells C1-C6. 0169's criterion named no backend and one statistic.
  Gating both configurations at their own absolute ceilings, with the ratio
  bound only to the fast one and the fallback ratio recorded ungated, is a
  restructuring of scope rather than a narrowing (SD-2).
- **C3 and C4 are provisional on first measurement.** Their bases are
  predictions resting on a cross-session import of 0186's per-call pair — the
  same import §7 declines to rely on for the composition budget — so the first
  in-session fallback figures become the bases any future re-run is gated
  against.
- **The criterion decision was taken on 0189, not 0169.** 0205 recommended
  reopening the threshold on 0169 before 0189 measured anything. This plan lands
  it on 0189 because 0169 is closed, and records the departure at all four 0169
  discharge points.
- **Three attempts were needed, and all three are recorded.** Attempts 1 and 2
  were invalidated on drift and attempt 3 is the valid session. The plan
  pre-registers "no sampling beyond the single escalation branch 3 permits", which
  governs *extending* a session; re-running after a recorded invalidation is a
  different act and every attempt is on the record with its own numbered file, so
  nothing was discarded silently. ⚠️ Attempt 3 also has the lowest ratio of the
  three (1.3260 against 1.3423 and 1.3177), so a reader should note that the
  session which happened to pass is also the one most favourable to `G` — the
  quietest host produced both the tightest floors and the best ratio.
- **The work-creation producer's id allocation collided, and its frontmatter
  needed repair.** `bin/accelerator work create` self-allocated 0210-0214, of
  which four were already claimed on other branches of the shared repository
  (0210 twice over): the allocator sees one checkout's `meta/work/` and not
  sibling workspaces' unmerged commits, so in a multi-workspace repository its
  ids are a proposal, not a reservation. The five were re-numbered to 0215-0219
  above the repo-wide maximum. It also emitted **unquoted** linkage values and
  omitted the `# NNNN: Title` heading and the `**Kind**/**Status**/**Priority**/
  **Author**` block when `--body-file` was used, all of which its own corpus gate
  rejects; each was repaired by hand. The plan's instruction to create these
  through the producer was followed — the producer's output simply needed fixing
  after the fact.
- **The threshold was relaxed a second time, from 1.3 to 1.4** (2026-08-17,
  author decision, Toby Clemson), after two sessions measured 1.3177 and 1.3423.
  This deviation compounds the first one recorded above rather than replacing it:
  1.1 → 1.3 was defended as "the floor of the band was taken", and taking the
  band's middle **voids that defence**. No mitigation is claimed in its place.

  What survives untouched is the argument that never depended on a measured
  value: `B` performs two directory-entry tests where `G` performs a jj-lib
  repository load behind a verified signature chain, so no ratio between them is
  a like-for-like comparison, and the ratio is a demoted comparison beneath an
  absolute budget that passes with 25% to 35% of headroom. ⚠️ C5 should be read
  as evidence that the ratio was measured and recorded, not as evidence that a
  ratio ceiling was independently justified at 1.4.

  The 1.3 route is deferred rather than abandoned: 0191's measured 2.48 ms is
  roughly twice the 1.25 ms separating 1.3423 from 1.3, so it carries a new
  acceptance criterion to re-measure the ratio after it lands, which would allow
  the threshold to be tightened back **on** evidence rather than left relaxed on
  it.
- **Attempt 1 was invalidated by drift, and three harness defects were found in
  its record.** Recorded 2026-08-17 as
  `meta/measurements/warm-dispatch-1.json`; every figure in it is non-gating.
  The three defects, all fixed before attempt 2:

  **C6 recorded no interval.** It was computed with the *paired* estimator, but
  Block B is single-arm by design and takes no baseline samples, so the arms
  differ in length and the estimator returned nothing — the cell fell to branch
  7 for an implementation reason rather than the host reason branch 7 exists
  for. C6 now takes an unpaired ratio-of-medians bootstrap. From attempt 1's
  recorded medians it would have read **1.7846**, against the predicted ~1.79.

  **Raw samples were not persisted**, so ten minutes of sampling could not be
  re-interrogated and C6 could not be re-derived without another session. Each
  attempt now writes a `-samples.json` sidecar, and records are numbered so a
  re-run cannot clobber an invalidated attempt's evidence.

  **The dual-backend cross-check measured the wrong thing silently.** The
  bracket's `backend` parameter was accepted and ignored — the body hard-coded
  `sha256sum`, which the fallback farm deliberately lacks, so the substitution
  came back empty, the bracket timed a failed lookup, and the fallback figure
  landed *below* the fast one for a **negative** delta. Exactly the
  silent-degradation class as the `chmod` defect below: a missing command makes
  `$(...)` empty rather than making the script fail. The bracket now asserts it
  computed one 64-hex digest per target and names the absent backend with its
  stderr. Re-measured, the delta is **+14.4 ms** (~7.2 ms per call, against
  0186's 8.44 ms) — the right sign and order.

  ⚠️ Attempt 1 also recorded a one-minute load average of **38.25 on 16 CPUs**
  while both instrument floors passed their gate. The floors are the gate and
  load is deliberately not one, since the direction of the load bias is
  unresolved — but the pairing is now printed **before** sampling with an
  oversubscription warning, because a session invalidated by drift after ten
  minutes is worth avoiding and the floors alone did not catch it.
- **The farm's tool set was hand-written and missed `chmod`**, which the plan
  warned about in exactly these terms and which the implementation did anyway.
  `probe_exec_capable` (`bin/accelerator:180-191`) writes a probe file and
  `chmod +x` it; with `chmod` absent from the farm the chmod failed, the
  bootstrap reported "no writable, exec-capable cache directory", and
  `--fail-safe` turned that into an exit 0 with empty stdout — the degraded
  shape §4 exists to catch. It was caught, but 100 subprocesses into the pilot
  and attributed to a sample rather than to the environment.

  Three changes, each removing the class rather than the instance. The tool set
  is now **derived mechanically** from `bin/accelerator`, the recovered guard
  and `vcs-common.sh` by `spawned_executables`, with a test that re-derives it
  and fails if either script gains a spawn the farm lacks — the plan's own
  "enumerated mechanically from `bin/accelerator`" instruction, honoured this
  time. The hand-written list also omitted `date`, `readlink`, `rmdir`,
  `sleep`, `timeout`, `wget` and `git`. The **validity diagnostic now carries
  the dispatch's stderr**, which is the only clue a degraded sample has, since
  stdout is empty by construction and the exit code is 0 either way. And the
  **warm-up dispatch is validated rather than discarded**, so an environment
  fault is reported against the prerequisite that failed.
- **A warm-cache precondition was added, which the plan required and the
  implementation had omitted.** §5 asks for "a published, minisign-signed
  release exists for the tree's own version" to be asserted before sampling,
  "otherwise the warm-up dispatch 404s and `fail_integrity` fires, surfacing as
  an opaque bootstrap abort rather than a named unmet prerequisite". That is
  what happened: the tree moved from `pre.38` to `pre.41` mid-implementation,
  the cache held only `pre.38`, and the first dispatch silently took the fetch
  branch. `warm_cache_gaps` now names the missing launcher, signature or
  sub-binary by version and refuses before any sampling.
- **The composition budget's residual is reported open, not absorbed.** §7's
  term set carries "shell bootstrap body (bash startup + 2 × `sha256_file` +
  logic)" as one **derived** term. The harness measures its two separable parts
  — bash startup and the two `sha256_file` substitutions — and leaves the
  bootstrap's remaining shell logic unmeasured, because separating it would
  need an edit to `bin/accelerator`. The residual is therefore expected to be
  negative and outside the ±1.5 ms band, and the uncross-checked share is
  reported as a number rather than hidden inside a derived term that closes by
  construction. That is the plan's own stated preference — "whatever share of
  `G` remains uncross-checked as a stated limit rather than as coverage" — but
  it means the band check itself does not pass, and the two-attempt
  re-measurement cap is what stops it looping.
- **The `dirname` spawn count is 1, not the predicted 0** (SD-31). The plan
  reasons that `find_repo_root` tests `-e "$dir/.jj"` on `$PWD` before its first
  `dirname` call, so the count should be zero at any depth. A `bash -x` trace of
  the recovered guard against the fixture observes **one** spawn — the guard
  resolves its own `SCRIPT_DIR` before it ever calls `find_repo_root`. The
  conclusion the prediction supported still holds: one spawn at any depth is
  still depth-independent, so the baseline is depth-insensitive and no depth
  pinning is needed across platforms. The plan asked for this to be confirmed
  empirically rather than assumed in either direction, and it was.
- **The red-first commit evidence covers the analysis core, not the driver.**
  Phase 2's manual criterion asks for a commit sequence showing a failing-test
  commit preceding each implementation. That holds for the analysis core: "Add
  failing tests for the warm-dispatch measurement analysis core" was committed
  against a `ModuleNotFoundError` and is the parent of the implementation
  commit. For the extracted predicates, the platform table, the session and the
  rehearsals the red state was observed — 45 failures on the predicates alone,
  before any of them existed — but was folded into one commit rather than
  committed separately. The loop was run; the commit sequence evidences half of
  it.
- **The analysis core lives in `tasks/shared/measurement.py`, not
  `tasks/measure.py`.** Phase 2 §1 names one module; the estimators,
  predicates, schedule generator, normaliser, classifier and closure aggregator
  are in a `tasks/shared/` sibling instead, following the repo's own split
  between the invoke surface and its helpers. Every guard the plan binds to
  `tasks/measure.py` still binds there: the per-platform table, the
  `register_artefact` seam whose call sites the exhaustiveness test scans, and
  the `run`-string predicate the closure guard keys on.
- **Precision gates branch 1, not only branch 4.** As written, branch 1's
  predicate is `U ≤ t` alone and branch 3's is positional, so branch 4's third
  stated cause — `upper_distance > target_distance`, the cell never reaching
  its precision target — is unreachable under the stated cascade, and the two
  parameters it is decided from would be declared but unread. The
  implementation makes an imprecise interval fail branch 1 and select branch 3
  (escalate, which is exactly what an imprecise interval calls for), so after
  the escalation it lands in branch 4 as the plan intends.
- **The exhaustive classifier enumeration is 1,440 well-formed states, not
  288.** The plan's count crosses three interval positions where five are
  needed (`L == t` and `U == t` are called out separately and are distinct
  positions) and omits the precision dimension the preceding deviation
  requires. The test enumerates the well-formed `(cell_kind, robustness_ok)`
  pairs rather than crossing them, as the plan requires, and asserts each state
  returns exactly one branch and that the ill-formed pairs raise.
- **No `# noqa: S311` on the resampler.** The plan predicts one; ruff does not
  flag `randrange` on an *injected* `random.Random`, and injection was already
  required so the tests stay order-independent. Two are needed instead at the
  `random.Random(SEED)` construction sites in the driver, in the repo's inline
  form with a stated reason.
- **`test:integration:measure` reports an absent published release as an unmet
  prerequisite rather than a failure**, after a discarded warm-up dispatch so a
  cold cache is not mistaken for one. The tree is routinely ahead of the last
  release cut, and `--fail-safe` exits 0 either way, so a lane that reddened on
  it would be red by default and would stop being read. The rot guards it owns
  — the recovery contract, the fixture's colocation, both farms, the baseline's
  decision shape — all fail loudly.
- **The teardown's containment predicate refuses anything under the plugin root
  beyond the two named allowances**, in addition to the three admitted roots.
  The plan's predicate admits any path under the temp parent, which would admit
  a tracked path by transitivity wherever the temp root is an ancestor of the
  checkout.
- **The recovery contract's digests are pinned in the harness**, so every
  recovery — not only the CI lane's rot guard — refuses a rotted revision
  rather than measuring whatever it found.
- **The cache witness covers the sub-binary asset**, not only the launcher, its
  `.minisig` and the staged shim: `vcs-<version>-<digest>` is the entry
  `cache::find` resolves on every dispatch, and omitting it would let a
  re-fetched sub-binary inflate a sample undetected.
- **0205's own deviations carry forward** and are cited rather than restated:
  its reopening of the gate definition, its throwaway target being an ignored
  integration test rather than an `examples/` target, and its
  three-runs-one-gates structure.

Further slots: the stdin payload where it differs from `git status`; any success
criterion discharged differently from as written. Mirrors 0186's
deviation-recording pattern (`meta/work/0186-…:608-616`). Records "none"
explicitly for any category left empty.

### Discharge record

Completed 2026-08-17. 0169's four locations carry the figures and each stays
**unticked** with a dated resolution naming the superseded threshold, the measured
value, the reason and the 0189-not-0169 ownership departure: the work item's own
criterion, its five `_pending_` Validation Results slots, the 0169 plan's Phase 10
criterion, and the 0169 validation's unchecked item. 0189 carries its
`## Validation Results` as the authoritative summary, criteria 10 and 11 ticked,
and `status: done` with its body line in lockstep. 0191's backend section carries
the measured figures and the 0.747 ms shortfall. Five follow-ups exist as
**0215-0219**, re-numbered above the repo-wide maximum after the producer's
allocation collided with four ids already claimed on other branches. Slots: 0169's four locations resolved, unticked, with dated
notes naming the 0189-not-0169 ownership departure; 0189's Validation Results
added, criteria **10** and **11** discharged, `status` and its body line set;
0191's existing backend section amended with the measured figures; the **five**
follow-up work items raised with their producer-assigned ids.

## Appendix: Superseded Decisions

Every decision this plan reversed during review, with what was wrong and why.
The phase bodies cite these by identifier so the instructions stay readable as
instructions; nothing here is required to execute the plan, and nothing here
overrides a phase body. Four review passes across eight quality lenses produced
them; there are 41.

**SD-1 — C4's ceiling had no stated base, and a uniform "18%/29% headroom" was
claimed across all four cells.**

It covered only three. C4's base is fast p90 46.51 ms + the predicted ~16.9 ms
backend delta = 63.4 ms, giving +26.2%.

**SD-2 — One taxonomy rule was applied to both cell kinds.**

It made C1-C4 unclassifiable: a dimensionless half-width against a millisecond
ceiling, and a floor-subtraction clause that *inverts* direction for an absolute
median (subtracting a shared floor makes it smaller, so the clause would be
weaker than the primary test). Also recorded: the per-backend split was once
described as a narrowing of scope rather than a restructuring.

**SD-3 — Raw medians were called "the conservative direction for a gate".**

`(G−c)/(B−c) > G/B`, so raw medians are the **lenient** statistic for a `ratio ≤
k` gate. The label inverted the meaning.

**SD-4 — The checkbox-reconciliation recommendation was attributed to the
sibling 0189 validation.**

It belongs to the 0169 validation (`:196-199`) and targets 0169's criteria. The
sibling report recommends three different things.

**SD-5 — The plan pre-registered "no relaxation of the threshold after seeing
the number", then relaxed it from 1.1 to 1.3 with 0205's 1.2813 in view.**

Recorded as the first Deviation. The structural and absolute arguments for
reframing are value-independent, and the band's floor was taken rather than its
middle — but the commitment was broken, not honoured.

**SD-6 — Threshold 1.4 was chosen on the ground that 1.3 would be "decided by
noise" at a half-width of 0.0119.**

Half-width is purchasable under the plan's own sizing rule, so the argument
defeated itself. 1.3 at n = 1,700 clears the point estimate by 5.2
upper-distances.

**SD-7 — Sizing was stated in symmetric half-widths, and Desired End State
quoted "0.005 / ~3.7 half-widths".**

0205's interval is asymmetric (0.0151 below, 0.0086 above), so the half-width
corresponds to neither tail and only the upper one gates. 0.005 also corresponds
to n ≈ 890, not the 1,700 the same sentence's runtime derived from.

**SD-8 — Phases 1 and 2 were described as simply "independent of each other".**

That read as licence to run Phase 3 ahead of Phase 1, which would reintroduce
the post-hoc-threshold problem, since every classifier branch is expressed
against the threshold Phase 1 lands.

**SD-9 — The artefact manifest was placed at `bin/.tmp-measure-manifest.json`.**

Three violations of the plan's own invariant: it collides with
`store::TEMP_PREFIX`; it adds a dirent to the directory whose entry count is a
budget term and whose entry set is an integrity witness, scanned on every timed
dispatch; and it makes the "any new `.tmp-*` entry is a cleanup failure" check
self-referential, where excluding `.tmp-*` would blind the leaked-temp detector.

**SD-10 — The manifest was listed as a row in its own artefact table.**

Restore's containment guard admits only the temp parent and `bin/.tmp-*`, and
the manifest is under neither — so the assertion failed on **every** run, and
since any verify failure selects branch 5, `closure_verdict` could never hold
and 0189 could never close.

**SD-11 — Artefact paths were held only in process memory, with teardown
enumerating just `T` and the fixture.**

Three artefact classes were uncovered, and the claimed "hand-re-runnable"
restore was impossible after SIGKILL, OOM or power loss.

**SD-12 — The release-key digest was read with `jj file show -r @`, described as
reading the committed rather than working-copy bytes.**

Under jj, `@` is the auto-snapshotted working-copy commit, so it does not. What
detects a prior substitution is comparison against the published constant.

**SD-13 — The repo-state gate ran only on exit and tested emptiness rather than
equality.**

A pre-existing uncommitted edit under `scripts/` or `bin/` — ordinary mid-stack
jj state — invalidated the measurement from sample one but was reported only
after all sampling; and a benign `cli/` edit failed the session at the end with
nothing distinguishing it from tampering.

**SD-14 — Manifest removal was gated on every verify item passing.**

A failure the operator cannot clear left the harness permanently refusing to
start with no documented override.

**SD-15 — `measure:teardown` was named as the escape from the start-up interlock
but registered nowhere; cleanup entries were "to be remediated" with no
mechanism.**

The only way out of the interlock did not exist, and a leaked lock directory
could be recorded beneath a passing verdict.

**SD-16 — The criterion was restated in five sections, which then disagreed on
how many cells gate.**

Resolved by `## The Criterion` as the single normative definition with cell
identifiers; every other reference is a pointer.

**SD-17 — Phase 1's retraction sets were hard-coded with line anchors, every one
drifted by 15-49 lines; the Set A pattern read "cannot close **before**" and
missed `:305-306`'s "cannot close **until**"; and the found-set count was stated
as eight where the pattern returns nine lines across seven passages.**

One anchor landed on the acceptance criteria a different step edits, and §5
hard-coded two 0169 locations, leaving the 0169 validation's two "three
manual/release-gated items" assertions live. Resolved by making the searches the
specification.

**SD-18 — Only three surviving `1.1 ×` passages were enumerated.**

The search returns five; four survive after criterion 10 is replaced.

**SD-19 — An instruction was carried to break a plan/0205 `derived_from`
cycle.**

The cycle was already broken, and the instruction misdescribed this plan's own
frontmatter.

**SD-20 — The envelope normaliser was specified as a port of
`guard_decision_table.rs:99-141` that must "match" it, while also distinguishing
empty stdout from an allow; the reason for rejecting the Python fixture
normaliser was stated as silent leniency.**

Unsatisfiable: the Rust function returns `("allow","")` for empty stdout, has no
legacy branch, and reads `systemMessage` at the top level. And the rejection
reason was inverted — under an expected-`block` comparison a deny→allow mapping
aborts loudly on pair one. Resolved as a five-case total union.

**SD-21 — Four successive drafts shipped a classifier signature narrower than
the branches the same revision added: bounds-and-threshold only; then without
`robustness_ok`/`cell_kind`; then without `applicable` after branch 7; and the
exhaustive domain crossed `cell_kind` with `robustness_ok` although they are
coupled.**

Each time the implementation would have silently encoded a narrower taxonomy
that no listed test could distinguish from the intended one. The rule now stated
in Phase 2 §1 exists because of this recurrence.

**SD-22 — The subprocess driver, fixture construction and provenance capture
were called "thin" and excluded from unit testing.**

That is where the measurement's validity lives; a defect there yields a
plausible-looking ratio rather than a crash, in a session that runs once.

**SD-23 — The validity-gate rehearsal pointed the harness at a non-repo fixture
so `G` would legitimately allow.**

The expectation is derived from the pre-sampling probe, so the probe would
derive `allow` and the validity gate would have nothing to mismatch — what fires
is the separate blocked-shape assertion, a different gate with a different
diagnostic.

**SD-24 — Only the `PATH`, farm recipe and probes were moved into per-OS data;
every gate number stayed a darwin-arm64 constant.**

The linux hand-off would have run a committed task judging linux against darwin
ceilings, aborted its retries on an uncalibrated floor, and been unable to
satisfy the reference-bash assertion at all.

**SD-25 — `tasks/README.md` was cited as documenting the `docs:*` namespace, to
be mirrored.**

It contains no `docs:*` section. The rationale lives in the root `CLAUDE.md` and
in `tests/unit/tasks/test_mise.py:71-82`.

**SD-26 — `T` was built by operator shell before the harness was entered.**

Restore could not assert "created by this run", and a SIGINT between recovery
and harness entry orphaned an executable copy of a deleted security-relevant
hook.

**SD-27 — Sampling used a single four-arm rotation (`B`, `G`-fast, `B`,
`G`-fallback).**

It left C5's pairing rule undefined (two `B` samples per cycle, no statement of
which pairs with which `G`), contradicted the seeded within-pair randomisation
mandated in the same section, and put Block B's ~10.7 MB-per-sample load inside
the pairs C5 is computed from — while `h₀` came from 0205's two-arm sequence.

**SD-28 — Runtime was computed on a two-arm pair cost while mandating a four-arm
rotation (~3 min / ~12 min), then over Block A alone (~15 min), then over both
blocks but omitting the initial run it escalates from (~19 min); the task
description separately said ~10 minutes.**

The escalated session is ~26 min including the run it escalates from, which a
25-minute budget exhausts — `budget_exhausted` would select 6b and make branch
3's escalation unreachable. The budget is now 35 minutes from harness entry.

**SD-29 — The drift diagnostic was banded at 5% of each variant's median and
filed under branch 5a.**

The gated margin is 1.4%, so a 4.9% shift in `G` alone passed while moving the
ratio three times the margin, and a benign common 5% drift discarded a good
session. A first-third/last-third comparison is also computable only after
sampling ends, which is 5b.

**SD-30 — Only the fallback farm's backend assertion was specified.**

A fast farm missing its `sha256sum` link would silently measure the fallback
backend under the cells carrying C1, C2 and C5.

**SD-31 — The `dirname` spawn count was derived from fixture path depth at
"roughly 1 ms per spawn".**

`find_repo_root` tests `-e "$dir/.jj"` on `$PWD` before its first `dirname`, and
the subprocess cwd is the fixture root — so the count is zero at any depth.

**SD-32 — The instrument-floor gate was calibrated from 0186's quiet session at
≤ 6.7 / ≤ 1.6 ms.**

Inverting 0205's own subtracted ratios gives implied floors of ~7.08 and ~1.75
ms, which **both breach** that gate — so a session reproducing 0205 exactly
would have failed the precondition three times and been recorded invalidated.

**SD-33 — Branch 4 was pinned to "half-width ≤ 0.005", then to "still straddles
`t`".**

The first left a hole when escalated dispersion is worse than the pilot's; the
second left a hole for a robustness failure after the escalation is spent —
which at a 0.003 margin was the *likely* terminal state, not a corner.

**SD-34 — The uncross-checked share of `G` was rounded to 30%, and the
recovery-contract rot guard was absent.**

The figure is 32.5% (67.5% cross-checked counting the measured bash floor).

**SD-35 — The composition-budget band was ±8% of `G`, then ±1.5 ms with a
sign-triggered uncapped re-measure.**

±8% (±3.38 ms) was wider than either lever it was meant to detect, and
contradicted its own negative-residual rule. The sign trigger was an uncapped
loop and a selection filter, since a sum of six noisy medians lands negative
roughly half the time.

**SD-36 — The dual-backend delta was claimed to "isolate the two `sha256_file`
calls' cost directly" and to partially close the uncross-checked gap.**

The delta yields `2 × (shasum − sha256sum)` ≈ 16.9 ms, not the ~7.1 ms absolute
cost the budget needs, so the absolute figure would have come from a borrowed
cross-session 0186 measurement.

**SD-37 — `closure_verdict`, Phase 4's two `iff`s and two checklists all encoded
the all-of-C1-C5 rule after branch 7 was added.**

On exactly the hosts branch 7 anticipates, closure remained blocked — the defect
branch 7 was added to remove, relocated rather than fixed.

**SD-38 — The new work items' frontmatter was listed as six fields "the template
mandates".**

The template mandates fifteen; the nine omitted are checked by the corpus gate.
Resolved by creating them through the work-creation producer.

**SD-39 — The lockstep guard was bound to 0189's criterion prose, with a
Phase-1-before-Phase-2 ordering constraint.**

No test under `tests/` reads `meta/`; it inverted the normal direction, made a
closed work item's wording load-bearing for a green build, and pointed at text
Phase 4 rewrites. The ordering constraint became vestigial once both sides of
the binding were Phase 2 artefacts.

**SD-40 — `test:integration:measure` was specified as carrying the
`build:cli:dev` edge, sitting in the launcher-dependent set, **and** following
`test:integration:entrypoint`'s stubbed-fetch shape.**

Impossible: `test_task_needing_no_launcher_omits_the_build_edge` forbids the
edge for `_NO_LAUNCHER_NEEDED` members,
`test_launcher_dependent_carries_the_build_edge` requires it for the other set,
and the two are asserted disjoint. It also mislabelled a live fetch as stubbed.
The live-dispatch check was additionally wired into `test:unit:tasks`, which
`mise.toml:264`/`:604` reach from the bare default and CI runs on both matrix
legs — and the closure guard, keyed on `measure:*`, could not see a pytest under
`tests/unit/tasks/`.

**SD-41 — The `test_mise.py` and `test_python_coverage.py` guards were filed
under Integration Tests.**

Both live in `tests/unit/tasks/` and run under `mise run test:unit:tasks`.

## References

- Work item: `meta/work/0189-once-per-dispatch-cache-root-probe-guarantee.md`
- **The spike that closed the method and ran it**:
  `meta/work/0205-close-the-warm-dispatch-measurement-method.md` — SQ-1 to SQ-5
  answered, its n = 300 gating run, the seven-branch taxonomy (5a/5b, 6a/6b),
  the `reverify` sub-operation figures, and the `B` cost-model correction this
  plan adopts
- Sibling plan:
  `meta/plans/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee.md` —
  `done`, and
  `meta/validations/2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee-validation.md`
  — `pass`, which discharges every 0189 criterion but the latency pair
- Reviews of this plan:
  `meta/reviews/plans/2026-08-11-0189-warm-dispatch-latency-measurement-review-1.md`
  — pass 2's assessment raised 0205; and `…-review-2.md` — the eight-lens review
  whose pass-1 criticals drove the criterion renumbering, the normaliser
  respecification, the baseline relocation and the threshold rebuild, and whose
  pass-2 re-review drove The Criterion section, the two-block sampling design
  and the teardown manifest
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — five `_pending_` slots
  at `:700-701`, and its own criterion at `:382-389`
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Phase 10's
  criterion at `:1507`, the stale premise at `:1514-1518`
- `meta/validations/2026-08-05-0169-vcs-subdomain-and-hooks-migration-validation.md`
  — the third record location at `:189-194`; stale premise at `:163-167`
- `meta/work/0186-remove-exec-probe-from-bootstrap-warm-path.md` — the
  **delivered** figures: 29.92 ms warm bootstrap median (`:517`), the
  per-backend `sha256_file` pair 3.55 ms against 11.99 ms (`:553-563`), the
  quiet-session instrument floors, the 35.1 ms shell-guard row (`:66`) with its
  method-incomparability note (`:615`), the 10.6 ms row (`:71`) re-derived at
  3.72 ms (`:577`), and the deviation-recording pattern (`:608-616`)
- `meta/plans/2026-08-02-0186-remove-exec-probe-from-bootstrap-warm-path.md` —
  the harness shape (`:1073-1135`) the committed harness supersedes
- `meta/work/0188-library-backed-vcs-adapter.md:1084-1085` — delivered
  subprocess figures, retained as context now that `B` is known to spawn neither
  `jj` nor `git`
- `meta/work/0191-batch-the-two-shim-hashes-into-one-invocation.md` — the stale
  "~2.4 ms shortfall" framing at `:38-40`, and the item Phase 4 re-sizes
- `meta/work/0199-retire-vcs-common-sh-residue-and-launcher-link-refresh.md` —
  scoped to decide whether `classify_checkout` leaves `scripts/vcs-common.sh`;
  inert for `B`, which calls `find_repo_root`
- `scripts/vcs-common.sh:8-18` — `find_repo_root`, the pure-bash walk that is
  `B`'s actual repo-root detection; `:177` — `classify_checkout`, which `B`
  never calls
- `cli/vcs-cli/tests/guard_decision_table.rs:99-141` — the Rust-side envelope
  normaliser Phase 2 ports, mapping `permissionDecisionReason` onto `block`
- `cli/kernel/src/hooks.rs:29-31` — the Rust guard's deny envelope
- `hooks/test-fixtures/vcs-guard/generate_decision_table.py:130`, `:160` — the
  fixture's `git.colocate=false` incantation, and the **shell-only** normaliser
  that must not be used for cross-variant comparison
- `cli/corpus-cli/tests/frontmatter_goldens.rs:309` —
  `this_repositorys_own_corpus_is_clean`, in package `accelerator-corpus`
- `cli/launcher/src/launch/core.rs:219-224`, `cli/launcher/src/main.rs:38-40`,
  `:215-226` — the fail-safe swallow, the environment seams and
  `handle_dispatch_error`
- `cli/launcher/src/launch/outbound/resolve/mod.rs:90-109` — `reverify`
- `cli/launcher/src/launch/outbound/resolve/verifier.rs:1-2`, `:29-49` — sha256
  named the corruption check and minisign the security boundary, and the order
  in which `verify_binary` applies them
- `cli/launcher/src/launch/outbound/resolve/keys.rs:62-69` — `verifies`, the
  Ed25519-over-BLAKE2b check that binds the bytes independently of the sha256
- `cli/launcher/src/launch/outbound/resolve/cache.rs:1-6`, `:51-73` — the
  never-evicted cache root, its scan, and the name-derived digest
- `cli/launcher/build.rs:31-32`, `:43` — the embed of
  `keys/accelerator-release.pub` and its `cargo:rerun-if-changed`
- `bin/accelerator:117`, `:165`, `:201`, `:216`, `:225`, `:239-240`, `:272-278`,
  `:291`, `:295` — the unverified-log path, the verification key, the cache
  root, the dev-launcher marker, the three-gate dev override, the digest backend
  selection and the two `sha256_file` call sites on the warm path
- `tasks/README.md` — task-tree shape and registration conventions;
  `tasks/__init__.py` — the invoke collection Phase 2 extends;
  `tasks/shared/sources.py` — `shell_sources()`'s `*.sh` plus
  `_EXTRA_SHELL_SOURCES` allowlist and `walk_files`'s gitignore honouring
- `tests/unit/tasks/test_mise.py` —
  `test_docs_tasks_stay_out_of_default_and_aggregate_check`, the precedent for
  keeping `measure:*` out of the CI mirror;
  `tests/unit/tasks/test_python_coverage.py` — the ruff/pyrefly discovery guard
