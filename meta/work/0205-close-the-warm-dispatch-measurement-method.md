---
type: work-item
id: "0205"
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
last_updated: "2026-08-13T07:05:06+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0205: Close the warm-dispatch latency measurement method

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

## Spike Outcome

**Date**: 2026-08-13. **Time spent**: one session (no box was stated in the
brief; one was agreed at the outset). **Revision**:
`a9260dd74f964ba848317fe10a04f4e5cbdf2652`.

**Verdict**: every question closed against measurement, and the answers invert
two of the brief's own premises. The method is specifiable — SQ-1's
decomposition is reachable through the launcher's public library surface with no
dev override and nothing signed locally; SQ-4's feared undecidability does not
fire, because `n` is cheap and it was 0169's stated `n = 20` that was
infeasible, not the gate. But the gate itself **fails**, by a margin far outside
sampling error, and the reason it fails is not the one the brief anticipated.

All figures below were taken on one darwin-arm64 host in one session: Apple M4
Max, 16 cores, macOS 26.3 build 25D125, plugin `v1.24.0-pre.36`, source tree
`1.24.0-pre.37`, `/sbin/sha256sum` resolved as the digest backend, Python
3.14.4.

### Headline: `G ≤ 1.1 × B` fails

n = 300 interleaved pairs with order alternating within each pair, one Python
process reading `perf_counter` around each `subprocess.run`, every sample's
decision normalised and asserted equal to `block` with the expected reason text:

| Variant | min | median | p90 | IQR |
| --- | --- | --- | --- | --- |
| `B` — recovered shell guard | 28.05 | **33.00** | 38.07 | 3.81 |
| `G` — bootstrap → launcher → `vcs guard` | 36.89 | **42.28** | 46.51 | 3.08 |
| floor: `/usr/bin/true` | 1.36 | 1.72 | 2.01 | 0.29 |
| floor: trivial bash script | 5.72 | 7.04 | 8.18 | 0.92 |

All in ms. **Ratio of medians 1.2813**, 95% two-sided paired-bootstrap CI
**[1.2662, 1.2899]** at 20,000 resamples, `P(ratio > 1.1) = 1.0000`. Ceiling
`1.1 × B` = 36.30 ms against `G` = 42.28 ms — an **overrun of 5.98 ms**.
`ratio(min) = 1.3150`; `p90(G)/p90(B) = 1.2216`.

**The verdict is conservative, not marginal.** The ratio *rises* as host load
falls — 1.16 at load 11 (n = 40), 1.2320 at load 38 (n = 400), 1.2813 at load
19 (n = 300, the run tabulated above, whose floors sit nearest 0186's quiet
session: bash 7.04 against 6.10, `true` 1.72 against 1.41). Load compresses the
ratio toward 1.0 by inflating the cheaper variant proportionally more, so a
quieter host makes the gate **harder** to pass, not easier. A quiet-host
re-measurement should be expected to return a ratio at or above 1.28.

Both variants were driven from a fresh pure-jj fixture created with `jj --config
git.colocate=false git init --quiet`, cwd set to the fixture, byte-identical
stdin
`{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git
status"}}`, `PATH` pinned to
`/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin`, `LANG=C`, and no
`ACCELERATOR_*` key present in the subprocess environment. The baseline was
recovered from `cf42441e2aad-`, resolved commit id
`2cfbf81e2e7b4934e868bd42c69374c335b05317`.

## Findings

### SQ-1: the decomposition is reachable through the library

**Answered by attempt.** `cli/launcher/src/lib.rs` is a real lib target
(`pub mod launch`), auto-detected with no `[lib]` stanza, and `main.rs:31`
already consumes it. Every term the brief named is already `pub`:
`verifier::sha256_hex` and `verify_binary` (`verifier.rs:14`, `:29`),
`TrustedKeys::embedded` and `verifies` (`keys.rs:55`, `:62`), `cache::find`
(`cache.rs:51`), `Fetcher::new` (`fetcher.rs:67`),
`tls::install_crypto_provider` (`tls.rs:9`).

**No term is unreachable, and the dev-override route was never needed.** The
brief's premise that the verified path cannot be instrumented is correct about
the *bootstrap* and irrelevant to the decomposition: nothing needs to be
instrumented, because each term is called directly from a throwaway target
linked against the same library the shipped binary links, built under the
workspace `[profile.release]` (`strip = true`, `lto = "thin"`). No marker, no
`ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER`, no `ACCELERATOR_LAUNCHER_BIN`, and no
write to `keys/accelerator-release.pub`.

**`reverify` is the one private term, and its replica is exact rather than
approximate.** `FetchVerifyCacheResolver::reverify` (`resolve/mod.rs:90-109`,
read at revision `2bb98478e7f7`) is three statements: `std::fs::read` of the
cached asset, `std::fs::read_to_string` of the `.minisig` sidecar, then
`verifier::verify_binary(file_name, bytes, cached.sha256, signature, keys)` —
which is itself `sha256_hex` followed by `TrustedKeys::verifies`
(`verifier.rs:29-49`). The replica reproduces that composition and call order
statement for statement; both are public, so nothing is inferred.

**The bootstrap/launcher-startup separability question: separable.** The brief
feared they were one composite, on the grounds that a built-in dispatch already
contains the launcher's `execve`, dynamic loading, `logging::init` and clap
parse. That is true of a built-in dispatch, but it does not force a composite,
because the cached shim and the cached launcher are both **directly invocable
by absolute path from the warm cache**, needing no override. Measured this
session at 60 samples each, medians in ms:

| Term | median | min | p90 |
| --- | --- | --- | --- |
| floor: `/usr/bin/true` | 1.31 | 1.13 | 1.59 |
| bootstrap → built-in (`accelerator version`) | 32.11 | 28.52 | 34.72 |
| cached launcher direct (`version`) | 4.85 | 4.16 | 5.57 |
| shim minisign-verify of the 8,234,704-byte launcher | 8.71 | 8.06 | 9.02 |
| `vcs` sub-binary direct (`guard`) | 5.10 | 3.80 | 5.81 |
| full `G` (bootstrap → `vcs guard`) | 41.58 | 38.53 | 44.32 |

So the composite splits three ways: shim minisign-verify 8.71 independently,
launcher startup 4.85 independently (3.54 net of the 1.31 fork+exec floor), and
the shell bootstrap body as the derived remainder at 32.11 − 8.71 − 4.85 =
18.55.

### SQ-2: one cross-domain subtraction, specified on both sides

**The question fires — it is not eliminated.** The `reverify` term is an
in-process span compared against a difference of two harness brackets
(`G` − built-in dispatch), so an in-process span is subtracted from a harness
bracket.

Both sides are the same clock domain, so the subtraction is valid. Python's
`time.get_clock_info('perf_counter')` on this interpreter reports
`namespace(implementation='mach_absolute_time()', monotonic=True,
adjustable=False, resolution=4.166666666666666e-08)` under Python 3.14.4 —
i.e. the darwin uptime clock, equivalent to `CLOCK_UPTIME_RAW`. Rust's
`std::time::Instant` on macOS reads the same uptime clock. No wall-clock bracket
is used anywhere, and no `time.time()` value enters any term.

### SQ-3: the residual, with closure and per-term agreement

**Closure.** The independently measured terms are compared against
`median(G) = 42.28`:

| Term | median (ms) | Cross-checked |
| --- | --- | --- |
| shell bootstrap body (bash startup + 2 × `sha256_file` + logic) | 18.55 | derived |
| shim minisign-verify of the 8.23 MB launcher | 8.71 | yes |
| launcher startup + clap, net of fork floor | 3.54 | yes |
| `cache::find` | 0.06 | yes |
| `reverify` | 6.34 | yes |
| `vcs` exec + guard work, net of fork floor | 3.79 | yes |
| **sum** | **40.99** | — |

**Signed residual +1.29 ms (+3.1% of `G`). Absolute residual +1.29 ms** — the
two coincide here because no pair of terms disagrees in opposing directions, so
nothing cancels. The residual is positive, so there is no evidence of
double-counting or a misplaced boundary.

**Cross-checked fraction of `G`: 53%** — 22.44 of 42.28 ms carries an
independent cross-check. Counting the measured trivial-bash floor (7.04 ms)
inside the derived bootstrap body raises it to **70%**. The uncross-checked
remainder is the bootstrap body's own script logic and its two `sha256_file`
pipelines; those sub-terms are individually measurable and were not separated
this session, which is a stated limit rather than a claim of coverage.

### SQ-4: the statistical design closes, and `n` was never the constraint

- **Raw medians, not floor-subtracted.** The floor-subtracted ratio is *higher*
  (1.171 against 1.1625 on the n = 40 pilot), because subtracting a constant
  from both sides of a ratio greater than 1 increases it. Raw medians therefore
  bias **toward passing**, which is the conservative direction for a gate the
  measurement is trying to fail honestly. Both floors are recorded regardless.
- **Gate statistic**: ratio of medians, with a paired bootstrap over the
  interleaved pairs at 20,000 resamples, reported as a **two-sided 95%**
  interval. Two-sided deliberately, so a half-width exists and the sizing rule
  below is expressible; the one-sided 95% lower bound is reported alongside as
  context, not as the gate.
- **Target precision and the `n` rule.** Half-width at n = 300 is **0.0119**,
  costing roughly 40 seconds of sampling at ~106 ms per pair. Scaling as
  `1/√n`, the rule is `n = 300 × (0.0119 / target)²`: half-width 0.02 → n ≈ 107;
  0.01 → n ≈ 425; 0.005 → n ≈ 1,700 (~4 minutes). **Pilot size 40**, which is
  what produced the first dispersion estimate.
- **The design-infeasible branch does not fire.** A gate at this margin is
  decidable at an `n` costing minutes. What was infeasible is 0169's stated
  `n = 20`: at the pilot's dispersion that yields a half-width near 0.15, wider
  than the entire 0.1 margin the gate is testing.
- **No post-hoc sampling.** The n = 300 run's interval was not extended after
  being seen. The two earlier runs (n = 40, n = 400) are recorded as pilot and
  load-sensitivity probes respectively, not as candidates for the gate
  statistic, and the gating run is the one named as such above.

**Outcome taxonomy, disjoint on stated arithmetic.** Let `U` and `L` be the
upper and lower bounds of the two-sided 95% interval on `ratio = median(G) /
median(B)`:

1. **Pass** — `U ≤ 1.1`.
2. **Fail** — `L > 1.1`.
3. **Indeterminate** — `L ≤ 1.1 < U`; escalate once to the `n` the sizing rule
   gives for half-width 0.005, then re-classify into branch 1, 2 or 4.
4. **Still indeterminate after the single escalation** — `L ≤ 1.1 < U` at
   half-width ≤ 0.005; recorded as indistinguishable from the threshold.
5. **Invalidated session** — any per-sample decision mismatch, any inode/mtime
   change on the cached asset, launcher, `.minisig` or staged shim, or any
   precondition failure. No ratio is produced.
6. **Design-infeasible** — no `n` within the stated budget reaches half-width
   0.005. Did not fire.

The branches are disjoint because 1, 2 and 3 partition the position of 1.1
relative to a single interval; 4 is 3 after its one permitted escalation; 5 and
6 produce no interval at all and so cannot co-select with any of 1 to 4.
**This session selects branch 2**, with `L = 1.2662 > 1.1`.

### SQ-5: `reverify` measured, and the prediction falsified

The asset is `vcs-1.24.0-pre.36-569559d8…` at **2,493,792 bytes** — not the
2,512,576 the brief cites from `meta/work/0188-…:1059`, which is the
`aarch64-apple-darwin` reference-artefact figure from a different build and is
stale for this purpose. 100 warm in-loop samples; cold-process figures are the
first call in each of 12 fresh processes.

**Predicted band, stated before measuring**: `fs::read` 0.3–0.8 ms,
`sha256_hex` 0.4–1.0 ms (assuming ARMv8 SHA2 intrinsics), `verifies` 1.8–3.5 ms,
composite 3–5 ms.

| Sub-operation | warm median | warm min | warm p90 | cold-process median | throughput |
| --- | --- | --- | --- | --- | --- |
| `std::fs::read` (asset) | 0.1184 | 0.0678 | 0.1535 | 0.2314 | — |
| `verifier::sha256_hex` | **4.4895** | 4.2874 | 4.5494 | 4.4788 | 555 MB/s |
| `TrustedKeys::verifies` (BLAKE2b-512 + Ed25519) | **1.7184** | 1.6505 | 1.7512 | ~1.75 | 1,451 MB/s |
| `reverify` replica (composite) | **6.3359** | 6.0994 | 6.4365 | 6.4637 | — |
| `cache::find` (context) | 0.0565 | 0.0275 | 0.0865 | 0.0558 | — |

**The prediction is falsified, and in the opposite direction to the brief's
reasoning.** The brief expected sha256 to be the cheap pass because "`sha2`
selects hardware SHA instructions on aarch64 and BLAKE2b does not". Measured,
**sha256 is 2.6× slower than BLAKE2b**: 4.4895 ms against 1.7184 ms over the
same bytes. `sha2` is running its software path at 555 MB/s. The cross-check is
`openssl sha256` over the identical file, which costs 1.46 ms net of its own
5.15 ms process floor — 1,708 MB/s, so the crate is **3.1× off the hardware
rate available on this chip**. BLAKE2b, which has no hardware path at all,
outruns it.

Cold-process adds nothing to either hash term (4.4788 against 4.4895;
composite 6.4637 against 6.3359). The first-call cost lands entirely in
`fs::read` (0.2314 cold against 0.1184 warm), `cache::find` (0.6879 on the very
first call in the process, 0.0558 thereafter) and the once-per-process
constructors: `install_crypto_provider` 0.0282, `TrustedKeys::embedded` 0.0104,
`Fetcher::new` 0.5030. None of those is material against a 5.98 ms overrun.

### Reachability: reachable, but only by shipping two changes

The overrun is 5.98 ms and `reverify` is 6.34 ms of it, so **`reverify` alone
does not exceed the plausible gap — it very nearly equals it.** Levers, each
sized from this session rather than assumed:

| Lever | Saving | `G` after | Ratio after |
| --- | --- | --- | --- |
| Drop the cache-hit `sha256` | −4.49 ms | 37.79 | 1.145 — still fails |
| 0191's shim-hash batching | −2.48 ms (its own cap) | 39.80 | 1.206 — still fails |
| Both together | −6.97 ms | 35.31 | **1.070 — passes** |
| `sha2` hardware intrinsics instead of lever 1 | −3.03 ms | 39.25 | 1.190 — weaker |

**The `sha256` in the cache-hit path is a corruption check, not a trust check,
and the code says so.** `cache::find` (`cache.rs:51-73`) parses the expected
digest out of the **filename of the file it is about to check**, so the
comparison establishes name/content consistency and nothing about provenance;
`verifier.rs:1-2` names minisign as "the security boundary" and sha256 as the
"corruption check" explicitly. Anyone able to write the cache entry's bytes can
write its name. Removal must be scoped to that call site only — `verify_binary`
is shared with `fetch_verify_store`, where the digest arrives from the
signature-verified manifest and does bind the bytes, as the 0189 plan already
notes at `:527-531`. That plan hypothesised this lever; this spike confirms it
and sizes it at 4.49 ms.

### Blind-spot sweep: `B` and `G` do not do the same work

**The 0189 plan's cost model for `B` is wrong.** Its Current State Analysis
(`:79-89`) attributes `B`'s cost to `classify_checkout` (`vcs-common.sh:177`)
spawning `jj workspace root`, two `git rev-parse` forms, `realpath` and `jq`,
and carries 0188's 23.84 ms `jj log` figure as an upper bound on "the guard's
`jj` spawn".

The recovered guard never calls `classify_checkout`. It calls **`find_repo_root`
(`vcs-common.sh:8-18`)** — a pure-bash loop testing `-e "$dir/.jj"` and
`-e "$dir/.git"` upward, spawning only `dirname` per path level — and then
decides mode with two literal `[ -d ]` tests on `.jj` and `.git`. There is no
`jj` spawn and no `git` spawn anywhere in `B`. Its cost is bash startup plus
roughly fifteen `jq`/`grep`/`sed`/`awk`/`cat`/`timeout` pipeline spawns.

Two consequences. First, the plan's fixture-choice rationale — that "a colocated
or live-workspace cwd adds tens of milliseconds to `B`" at ~24 ms per `jj`
spawn — does not hold; fixture depth still changes the `dirname` loop's spawn
count, but at roughly 1 ms per spawn, not 24. A pinned non-colocated fixture
remains correct for the *decision-shape* reason (a colocated fixture emits warn,
not block), which is independently sufficient.

Second, and more consequentially: **the 1.1 margin compares a stat-based
heuristic against a real VCS classification.** `B` decides pure-jj versus
colocated by testing for two directory entries. `G` loads the repository
through jj-lib. The Rust guard is not a faster reimplementation of the shell
guard; it is a more correct one, and the gate charges it for that correctness at
a margin calibrated against the cheaper behaviour. 0186's own
method-incomparability note (`meta/work/0186-…:615`) already flagged that its
35.1 ms shell-guard row was not comparable; this is a second, independent reason
the two sides are not like-for-like.

## Recommendation

**Reopen the 1.1 threshold in 0169 before 0189 measures anything.** This is the
recommendation the evidence supports most directly, and it is a deliberate
departure from this spike's own stated assumption — recorded as a deviation
below rather than adopted silently.

The reasoning, in order:

1. **The gate as written fails and cannot be made to pass by measuring more
   carefully.** `L = 1.2662` at a half-width of 0.0119, and the ratio worsens as
   the host quietens. No sampling choice, floor policy or clock refinement moves
   a point estimate of 1.28 to 1.10.
2. **The comparison is not like-for-like.** `B` performs two directory-entry
   tests where `G` performs a jj-lib repository load. A ratio gate is only
   meaningful between variants doing the same work, and 0169 calibrated 1.1
   against a `B` whose cost model was misread — the plan it fed carried a 24 ms
   `jj` spawn into `B` that does not exist.
3. **Passing the gate as written would require shipping two optimisations whose
   only justification is the gate.** Dropping the cache-hit `sha256` is
   defensible on its own merits and should be raised regardless. But pairing it
   with 0191 purely to clear a 1.1 threshold sets the launcher's verification
   posture by an arithmetic target rather than by a threat model.
4. **The absolute numbers are defensible even though the ratio is not.** 42.28 ms
   for a fully signature-verified, jj-lib-backed hook against 33.00 ms for an
   unverified stat-and-grep script is a 9.28 ms premium for the whole trust
   chain — two minisign verifications over 10.7 MB combined, plus a real VCS
   classification. A threshold expressed as an absolute budget, or as a ratio
   against a like-for-like baseline, would be defensible in a way that 1.1
   against this `B` is not.

Concretely, in this order:

- **0169 decides what the criterion should be**, with the `B`/`G` work
  asymmetry and this session's figures in front of it. Candidate reframings: an
  absolute warm-dispatch budget; a ratio against a baseline that also performs a
  real classification; or the existing 1.1 retained as an explicit, accepted
  overrun with a named approver.
- **0189 then runs the formal measurement** against whatever criterion 0169
  lands, using the method this spike closed. SQ-1 to SQ-5 are answered and need
  no re-derivation; the harness shape, the fixture incantation, the
  normalisation mapping and the taxonomy are all specified above.
- **Raise the warm-dispatch verification cost as its own work item now**, not
  contingent on 0189's branch. The cache-hit `sha256` finding stands on its own:
  4.49 ms, a third of the launcher-side cost, buying a name/content consistency
  check that the minisign signature over the same bytes already subsumes.
- **Raise the `sha2` intrinsics gap separately.** 555 MB/s against openssl's
  1,708 MB/s on the same chip is a 3.1× shortfall affecting every sha256 the
  Rust binaries compute, not only this call site. It is a smaller lever here
  than dropping the call entirely, but it is not confined here.
- **Correct the 0189 plan's `B` cost model** (`:79-89`) and its fixture
  rationale (`:346-349`) before that plan is executed, so the correction does
  not have to be discovered again during measurement.

## Residual Risks & Open Questions

- **The gating run was not taken on a quiet host.** Load averaged 18.72 across
  16 cores. The direction of the bias is established rather than assumed — three
  runs at three load levels show the ratio rising as load falls — so the
  recorded 1.2813 is a floor, not a point estimate to be trusted absolutely.
  **Trigger to revisit**: a quiet-host run returning a ratio below 1.20 would
  contradict the load model and should be investigated before the reframing in
  the recommendation is acted on.
- **30% of `G` carries no independent cross-check.** The bootstrap body's script
  logic and its two `sha256_file` pipelines were not separated. They are
  individually measurable and 0186 measured comparable figures; this session
  stopped at the derived remainder. A residual of +3.1% over 70% coverage is not
  evidence about the other 30%.
- **`B` is a recovered artefact whose dependency is live.** `scripts/vcs-common.sh`
  was byte-identical to the sourced revision at measurement time, but work item
  0199 is scoped to decide whether `classify_checkout` leaves that file. Since
  the recovered guard calls `find_repo_root` rather than `classify_checkout`,
  0199's likely change is **inert for `B`** — a narrower exposure than the 0189
  plan assumed, but not zero, since `find_repo_root` shares the file.
- **The `reverify` replica was verified by reading, not by differential
  execution.** Composition and call order were checked against
  `resolve/mod.rs:90-109` at revision `2bb98478e7f7`. A future refactor of the
  private method would silently invalidate the replica. **Trigger**: re-read
  that range at the revision of any re-measurement.
- **Nothing here transfers off darwin-arm64.** The sha256-versus-BLAKE2b
  inversion is a property of this chip and this crate build; on a host where
  `sha2` does reach hardware intrinsics the ranking reverses and the lever
  ordering changes with it. Per-platform figures must be recorded against
  (architecture, SHA-extension support, libc), not the OS name.
- **Unaddressed by design**: the linux measurement, darwin-x64, and the
  committed-harness decision all remain 0189's named hand-offs.

## Deviations

- **The gate definition was reopened.** This spike's Assumptions state that
  `G ≤ 1.1 × B` is inherited from 0169 and not reopened here, and that any
  strengthening is a deviation to record rather than adopt. The recommendation
  above proposes reopening the threshold outright — a larger departure than a
  strengthening. It is recorded as the spike's recommendation to 0169, not as a
  decision this spike takes: the criterion remains as written until 0169 changes
  it.
- **The throwaway target is an ignored integration test, not an
  `examples/` target.** Acceptance criterion 8 names
  `cli/launcher/examples/`. The instrumentation was carried by
  `cli/launcher/tests/spike_0205_warm_terms.rs` instead — `tests/` already
  exists in that package, so no new target kind was introduced and the
  workspace's first `examples/` target was avoided. The criterion's substance
  was honoured: `mise run cli:check` was green with the target present
  (after one rustfmt pass and one `clippy::ptr_arg` fix), and the target has
  since been removed.
- **The gate statistic is an interval bound, not a bare median comparison.**
  0169's criterion reads as a median rule. Branch selection above uses the
  bootstrap interval's lower bound, which is a deliberate strengthening. It does
  not change this session's outcome — the bare median comparison fails too, at
  1.2813 against 1.1.
- **`n` departs from 0169's stated 20.** The gating run used n = 300. At n = 20
  the interval half-width exceeds the margin under test, which is recorded above
  as the reason the stated sample count, not the gate, was infeasible.
- **Three runs exist, one gates.** n = 40 (pilot, dispersion estimate), n = 400
  (load-sensitivity probe), n = 300 (gating). The gating run is named as such
  and its interval was not extended after being seen.

## Cleanup Evidence

Positively asserted after the session, not inferred from a VCS diff:

- `cli/launcher/tests/spike_0205_warm_terms.rs` — absent.
- The pure-jj fixture root and the recovered baseline tree — absent (both lived
  outside the plugin root and outside any jj workspace; the baseline was never
  parked in `bin/`, so no cache-root entry was added).
- `bin/.tmp-vcs-guard-baseline` — absent; never created.
- No dev-override input was ever set: no `.accelerator-dev-launcher` marker at
  either the repo or plugin root, and zero `ACCELERATOR_*` keys in the
  environment.
- `sha256(keys/accelerator-release.pub)` =
  `0f3fe9a91ab6869ce36209691e06c722259e5754f2228b1539ef566b00f6fb2e`, identical
  between the committed revision (`jj file show -r @`) and the working copy.
- `.accelerator-unverified.log` does not exist at the plugin root — the trust
  chain's alarm never fired, so no line needed attribution or removal.
- `jj diff --summary cli/` is empty.
- `mise run cli:check` is green.
