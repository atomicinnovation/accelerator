---
type: plan-review
id: "2026-08-11-0189-warm-dispatch-latency-measurement-review-1"
title: "Plan Review: Warm-Dispatch Latency Measurement Implementation Plan"
date: "2026-08-12T08:58:06+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
parent: "work-item:0189"
target: "plan:2026-08-11-0189-warm-dispatch-latency-measurement"
relates_to:
  ["plan-review:2026-08-11-0189-once-per-dispatch-cache-root-probe-guarantee-review-1"]
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses:
  [performance, correctness, architecture, standards, portability, safety,
   documentation]
review_number: 1
review_pass: 2
tags: [cli, launcher, performance, bootstrap, measurement]
last_updated: "2026-08-12T11:37:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Warm-Dispatch Latency Measurement Implementation Plan

**Verdict:** REVISE

The plan's methodological instincts are excellent and several are better than the
work it inherits from: it identifies the `--fail-safe` short-circuit as the
confound that could manufacture a PASS from fifty fast failures, it recognises
that a composition budget of contiguous slices closes by construction and
demands independent cross-checks instead, and Standards verified that every one
of roughly twenty-five file:line citations resolves exactly. But three of its
four phases are gated on Open Decisions whose option set does not survive
scrutiny — OD-1(b) is infeasible as written because the two keys it distinguishes
are the same file — its central per-sample guard compares two structurally
different envelope shapes and would abort on the first sample, its cleanup
verification is blind to almost everything it creates because those artefacts are
gitignored, and its gating premise over-estimates the measured asset by
threefold. Seven criticals and thirty-six majors across seven lenses.

### Cross-Cutting Themes

- **OD-1's option set does not survive scrutiny** (flagged by: Safety,
  Correctness, Architecture, Performance) — (b) is infeasible as specified;
  (a) is silently incompatible with OD-2's subtraction derivation and bypasses
  the very bootstrap terms the budget attributes; (c) makes Phase 3's own
  success criteria unsatisfiable. Architecture additionally identifies a missing
  option that dominates all three.
- **Cleanup and revert verification is structurally blind** (Safety,
  Correctness) — the plan cites gitignoring as a *benefit* (jj will not snapshot
  the recovered baseline) without noticing that the same property makes its
  `jj diff`/`jj status` gates unable to detect a substituted cached launcher, an
  active dev-launcher marker, a throwaway secret key, or the baseline itself.
  Phase 2's criterion is a tautology.
- **The gate verdict has undefined branches** (Correctness, Architecture,
  Performance) — a legitimately skipped CI leaves Phase 3's PASS rule
  unsatisfiable; an indeterminate-by-dispersion result triggers a 0191 action
  set that does not apply; and a measured overrun against an already-closed 0169
  has no defined procedure at all.
- **Post-hoc analysis choices undercut the pre-registration** (Performance,
  Correctness) — floor-subtraction policy, sample size and when the CI is
  computed are all left to be decided after the numbers exist, in a plan whose
  "no relaxation of the threshold after seeing the number" is its stated
  discipline. A floor does not cancel in a ratio: at 0186's figures the choice
  moves a 1.100 ratio to 1.115, across the threshold.
- **Borrowed figures are mis-attributed in ways that change conclusions**
  (Performance, Portability, Correctness) — the asset is 2,512,576 bytes, not
  "~8 MB comparable" (a 3× error in the *gating* phase's premise); 35.1 ms comes
  from a table 0186 itself declared "not method-comparable"; 23.84 ms is
  `jj log -r @ -T commit_id`, not the guard's `jj workspace root`; the
  digest-backend divergence is a within-darwin host fact that 0186 explicitly
  refused to call a platform fact, quoted with a figure 0186 retired.
- **The discharge is incomplete** (Documentation, Standards, Architecture) —
  0169's obligation lives in three documents, not two; 0169's work item carries
  its own unchecked latency criterion the plan never ticks; 0189 asserts the
  stale release blocker in four passages of which two are named; and no phase
  names the corpus-frontmatter validator that gates the four meta documents
  edited.
- **"The two share no code, no file and no test" is false** (Architecture,
  Standards, Documentation) — both 0189 plans make dated-note edits to the same
  work item, with line anchors into text the other will shift and no declared
  ordering.

### Tradeoff Analysis

- **Evidence strength vs blast radius**: Performance and Architecture want the
  strongest decomposition; Safety notes OD-1 ranks its options on cost and
  evidential weight while never comparing what each can break — (a)'s worst
  residue is a gitignored marker on one machine, (b) mutates the tracked file
  every future launcher embeds as its trust root, with a path to `main`.
  Recommendation: adopt Architecture's missing option — measure the bootstrap
  term from a **built-in** dispatch through the shipped, unmodified bootstrap
  (how 0186 got its 29.92 ms) and take launcher-internal terms from independent
  micro-measurements. It is stronger evidence than (a), needs no trust-root
  mutation, and its terms are independent, which is what OD-3 requires anyway.
- **Statistical rigour vs decidability**: the CI-upper-bound rule is more
  defensible than a bare median comparison, but Performance shows n=50 was
  inherited from a measurement of a 4× effect and is being used to decide a 10%
  margin — the design may be structurally unable to render a PASS, collapsing
  every outcome to "indeterminate, treated as an overrun". Recommendation: take a
  10–15 pair pilot alongside Phase 1 and pre-register the `n` needed for a stated
  CI half-width, rather than inheriting 50.
- **Inheriting 0169's criterion vs strengthening it**: the plan commits to
  inheriting `G ≤ 1.1 × B` "rather than inventing a new one", then gates on a CI
  upper bound — a strictly tighter test that can convert a criterion-satisfying
  measurement into a recorded overrun. Recommendation: keep the CI, but state
  plainly that it is a deliberate strengthening, so the stricter verdict is not
  mistaken for the inherited one.

### Findings

#### Critical

- 🔴 **Safety / Correctness**: OD-1(b) is infeasible — the two keys it
  distinguishes are the same file
  **Location**: Open Decisions, OD-1 option (b)
  The option requires the launcher to "keep the *real* embedded key" while
  "swapping the bootstrap's verification key", but `bin/accelerator:165` reads
  `${plugin_root}/keys/accelerator-release.pub` with no override and
  `cli/launcher/build.rs:32` embeds *that same file* with
  `cargo:rerun-if-changed`. Any intervening `cargo build`, `mise run cli:check`
  or `mise run` re-embeds the throwaway key, `reverify` then fails
  `SignatureMismatch`, and the run leaves the cache-hit branch. **Verified.**

- 🔴 **Safety**: A swapped release key is auto-snapshotted, passes all CI green,
  and has a documented path to shipping
  **Location**: Open Decisions, OD-1 option (b); What We're NOT Doing
  `mise run keys:generate` defaults to the tracked trust root and runs
  `minisign -G -W -f` — force-overwriting it with an unencrypted keypair. The
  only assertions on that file are that it parses and is not a bare placeholder,
  both of which a throwaway key satisfies, so `mise run` stays green with a
  laptop key as the plugin's trust root. `RELEASING.md` then makes the rollout
  consequence explicit. **Verified.**

- 🔴 **Safety / Correctness**: The revert gate is structurally blind to nearly
  every artefact it must catch
  **Location**: Phase 3 Success Criteria; Phase 2 Automated Verification
  Cleanup rests on `jj diff` over `bin/`, `keys/`, `cli/launcher/src/` and
  `jj status` for the baseline — but `bin/.tmp-*`, `bin/accelerator-launcher-*`,
  `bin/*.minisig`, `/.accelerator-dev-launcher`, `keys/*.sec` and `cli/target/`
  are all gitignored. Phase 2's criterion is vacuously true whether the script
  was removed or not. The plan cites this same gitignoring as a benefit two
  paragraphs earlier.

- 🔴 **Correctness**: The per-sample B/G equivalence assertion cannot succeed as
  written
  **Location**: Phase 2, item 4: Per-sample validity
  The shell guard emits the legacy `{"decision":"block","reason":…}` shape; the
  Rust guard emits `hookSpecificOutput.permissionDecision` (`hooks.rs:29-31`).
  `hooks/test-fixtures/vcs-guard/generate_decision_table.py:160` exists
  precisely to normalise between them. Written literally the assertion aborts on
  sample 1; written loosely enough to pass both, it stops discriminating a deny
  from a degraded fail-safe sample — the exact failure it exists to catch.
  **Verified.**

- 🔴 **Architecture**: Retracting 0189's closure guard while this plan's figures
  are pending re-orphans the obligation
  **Location**: Phase 4, item 2; Desired End State
  0189's `blocked_by: ["work-item:0169"]` edge is already satisfied (0169 is
  `done`), so the Dependencies bullet "**This item cannot close before that
  release**" is its only structural guard. Phase 4 retracts it while the figures
  sit `_pending_` in a `draft` plan, and the sibling — `ready`, CI-closable —
  discharges eleven of twelve criteria. Nothing then prevents 0189 closing with
  the measurement unrecorded. **Verified.**

- 🔴 **Architecture**: Phase 4 is written as though the gate passes
  **Location**: Phase 4 Success Criteria vs Phase 3, item 2
  Phase 4 unconditionally requires 0169's Phase 10 criterion ticked; Phase 3's
  overrun branch says it must not be. 0169 is already closed `done`, so a
  measured overrun means a completed story shipped failing its own gate — and
  the plan defines no procedure for that state.

- 🔴 **Documentation**: 0169's obligation lives in three documents, not two
  **Location**: Phase 4, item 1: Backfill 0169
  `meta/validations/2026-08-05-0169-…-validation.md` carries an unchecked
  "Warm-call latency" checkbox and states the obligation "requires a published,
  minisign-signed `accelerator-vcs` release asset that does not yet exist" — the
  premise this plan retracts as stale. Every implemented plan gets such an
  artefact, so this is a standing record location. **Verified.**

#### Major

- 🟡 **Performance**: The asset-size premise contradicts 0188's delivered table
  by 3× (2,512,576 bytes on `aarch64-apple-darwin`, not "~8 MB comparable"),
  and it is the premise the *gating* phase's reachability verdict rests on.
  **Verified.**
- 🟡 **Performance / Correctness**: OD-1(a) and OD-2's subtraction derivation are
  mutually inconsistent — the dev override `exec`s above the shim hashes and the
  launcher verification, so the derived "bootstrap term" is the bypassed prefix,
  understating `G`'s largest group of terms by roughly 14 ms.
- 🟡 **Performance / Correctness**: Floor-subtraction policy is deferred to a
  post-hoc choice, and a floor does not cancel in a ratio.
- 🟡 **Correctness**: The overrun policy's action set does not apply to an
  indeterminate-by-dispersion result — a run comfortably under 1.1 with a wide CI
  is indeterminate *because of spread*, so 0191 has no gap to close and `reverify`
  does not exceed it; that branch falls through the policy entirely. The
  CI-upper-bound rule also silently tightens 0169's median-based criterion, which
  the plan elsewhere commits to inheriting rather than reinventing.
- 🟡 **Performance / Correctness**: The conditional statistical tail is
  ill-defined — evaluated in a phase that has no `B`, `G` or IQR; stated in IQRs
  when the decision quantity is the median ratio's standard error; and if taken,
  Phase 3's PASS rule becomes unsatisfiable.
- 🟡 **Performance**: No precision target — n=50 inherited from a 4×-effect
  measurement, used for a 10% margin. The design may be unable to render a PASS.
- 🟡 **Performance**: The gate ratio is an artefact of one fixture and one tool
  set; `B`'s dominant term is conditional on `jj` being on `PATH`, and `G`'s on
  the digest backend.
- 🟡 **Performance / Correctness**: `B` is anchored to 35.1 ms, which 0186
  declared "not method-comparable" (`:615`); the same table's 10.6 ms row was
  re-derived at 3.72 ms, a 2.8× error. **Verified.**
- 🟡 **Performance**: The budget has an unbudgeted boundary between "launcher
  exec" and "resolver construction" — dynamic loading, `logging::init`, clap
  parse; 0186 measured the comparable term at ~2.4 ms.
- 🟡 **Correctness**: The cache-hit proof covers only the sub-binary; a re-fetched
  launcher or re-staged shim passes every per-sample assertion while inflating
  the sample.
- 🟡 **Correctness**: The fixture is unpinned — `jj git init` colocates by
  default at 0.43, and a colocated fixture emits **warn**, not deny.
- 🟡 **Correctness**: `jq` is an unrecorded hard dependency and per-sample spawn
  of the shell baseline, so "recovery is one file" understates the dependency
  set. **Verified.**
- 🟡 **Correctness**: The mmap lever is not behaviour-preserving — two passes over
  a mapping of a user-writable file can see different bytes, and truncation
  raises SIGBUS rather than a clean `Cache` error.
- 🟡 **Correctness**: The residual is defined against cross-checks for only two of
  five budget rows, yet inherits 0186's 25% whole-median threshold; signed
  aggregation can mask two opposing disagreements.
- 🟡 **Safety**: Cleanup obligations are gated in later phases that the plan's own
  abort paths skip — an early abort or a Phase 1 reachability finding bypasses
  every removal criterion.
- 🟡 **Safety**: OD-1(a) requires satisfying all three gates of a defence that
  exists specifically to stop unattended unverified execs, and no criterion
  removes the marker or unsets the variables.
- 🟡 **Safety**: The measurement floods `.accelerator-unverified.log`, the trust
  chain's only durable alarm, and never resets it — training the developer to
  dismiss the one persistent signal for a real compromise.
- 🟡 **Safety**: `bin/` is not actually forced for the recovered baseline (any
  parent containing `scripts/` satisfies the constraint), and parking there puts
  an unreviewed executable in the launcher's live cache root under the `.tmp-`
  prefix reserved for in-flight atomic writes.
- 🟡 **Safety**: Partial-revert states are silent and asymmetric — one disables
  the git guard's protection and re-downloads ~8 MB per dispatch; the other
  leaves the trust root substituted.
- 🟡 **Portability**: The gate ratio's platform sensitivity is asymmetric and the
  direction is never stated — `B` is spawn-dominated and shrinks more on linux
  than IO/hash-dominated `G`, so linux is the *harder* host and a darwin PASS is
  closer to a best case.
- 🟡 **Portability**: The digest-backend divergence is a within-darwin host fact,
  not a platform fact, and the 11.7 ms figure is one 0186 retired. **Verified.**
- 🟡 **Portability**: `os.process_cpu_count()`/`sched_getaffinity` are
  affinity-based, not quota-based, so the stated cgroup fix does not hold on a
  container runner.
- 🟡 **Portability**: The linux hand-off's prerequisites describe a cross-compile
  the *measurement* does not need — the shipped musl artefact is fetched and
  verified; the toolchain is needed only for the OD-1 decomposition. The
  allocator rationale is also the wrong mechanism.
- 🟡 **Portability**: `PATH` is unspecified, yet every host-dependent budget term
  is `PATH`-resolved, and the floor is resolved in the harness's environment
  rather than the subprocess's.
- 🟡 **Architecture**: The Open Decisions have no decision procedure, no owner
  and no defaults, and one enumerated option makes a later phase's own criteria
  unsatisfiable — the artefact is a decision document, not an implementable plan.
- 🟡 **Architecture**: OD-1's option set omits the option that dominates it —
  bootstrap term from a built-in dispatch through the shipped bootstrap, exactly
  how 0186 obtained its 29.92 ms median, and independent as OD-3 requires.
- 🟡 **Architecture**: The baseline pins only the guard's revision while its
  behaviour depends on the live `scripts/vcs-common.sh`; work item 0199 is scoped
  to decide whether `classify_checkout` is deleted from that very file, and
  neither plan nor work item records the relation. **Verified.**
- 🟡 **Documentation**: 0189 asserts the stale release blocker in four passages;
  the plan names two and gives no line references, where the sibling enumerates
  all six of its own.
- 🟡 **Documentation**: The resolved commit id of the deleted `hooks/vcs-guard.sh`
  — the one input that makes `B` reproducible — is demanded in prose but appears
  in no slot list and no criterion.
- 🟡 **Documentation**: OD-1(a) omits two of the three required dev-override
  inputs (`ACCELERATOR_LAUNCHER_BIN` and the `cli/target/` containment
  constraint), so the "explicitly exempted and labelled" carve-out names nothing.
- 🟡 **Documentation**: The 20→50 sample deviation is claimed to follow 0186's
  pattern, but Validation Results has no deviations slot and Phase 4 requires no
  method note at the tick site.
- 🟡 **Documentation**: The `reverify` slot records no asset identity and no `n`
  or dispersion, while OD-3 makes that figure a load-bearing cross-check.
- 🟡 **Documentation**: 0189's own figures land only in this plan — no
  `## Validation Results` in the work item, no tick, no pointer saying which
  sibling holds the numbers.
- 🟡 **Documentation**: The Open Decisions carry no "Default if unresolved", the
  convention both 0189 and 0169 already use for open questions.
- 🟡 **Standards**: 0169's work item carries its own unchecked latency acceptance
  criterion at `:382-389` that Phase 4 never ticks.
- 🟡 **Standards**: No automated gate for the four meta documents Phase 4 edits,
  despite the repo running `accelerator corpus frontmatter validate` over `meta/`
  as a cargo test; 0186's closeout phase carried exactly such a bullet.

#### Minor

- 🔵 **Standards**: `**File**`/`**Changes**` labelling is absent from seven of
  eleven change items and uses three different forms.
- 🔵 **Standards**: Two new work items are specified without the repo's id,
  naming, frontmatter or cross-linking conventions.
- 🔵 **Standards**: The recovered baseline takes a `.sh` extension that 0186
  deliberately avoided (`bin/.tmp-accelerator-before`), putting it in
  `shell_sources()` scope if the `bin/` globs are ever narrowed.
- 🔵 **Standards / Correctness**: The stdin envelope diverges from the shipped
  one-field contract — the guard reads only `.tool_input.command`, so the `cwd`
  field is inert. **Verified.**
- 🔵 **Standards**: `## Open Decisions` is a novel top-level section appearing in
  1 of 166 plans with no template slot, and the block it creates is invisible to
  frontmatter.
- 🔵 **Standards**: The throwaway example is the `cli/` workspace's first
  `examples/` target and is in scope for pedantic clippy via `--all-targets`.
- 🔵 **Correctness**: The baseline is invoked by repo-relative path while the
  subprocess cwd is the fixture, so `SCRIPT_DIR` resolves against the wrong
  directory.
- 🔵 **Correctness**: The envelope must name one of the thirteen *blocked* git
  subcommands; the exit code carries no decision information (0 for deny, allow
  and degradation alike).
- 🔵 **Correctness**: `command -v` is a bash builtin, so counting it among
  "subprocess spawns" overstates the spawn count.
- 🔵 **Correctness**: The sha256 lever must be scoped to the cache-hit call site —
  `verify_binary` is shared with `fetch_verify_store`, where the digest comes
  from the signed manifest and does bind the bytes.
- 🔵 **Correctness**: No precondition asserts the instrumented binary's
  `CARGO_PKG_VERSION` matches `plugin.json`'s version; if they diverge,
  `cache::find`'s `vcs-<version>-` prefix misses and
  `Manifest::parse_and_validate` raises `ManifestVersionMismatch` → `Refusal` →
  exit 2, so the run measures neither a hit nor `G`.
- 🔵 **Safety**: The instrumentation writes to stderr on a user-visible hook
  path, mitigated only by a gate that cannot see it — once `cli/launcher/src/` is
  reverted the compiled binary remains in gitignored `cli/target/`, reachable via
  the dev override, and during the measurement itself the noise is live on every
  Bash tool call.
- 🔵 **Performance**: An in-loop warm median with an intercept-free ms-per-MB
  model understates what `G` pays once in a fresh process and mis-transfers to a
  differently-sized asset.
- 🔵 **Performance**: The overrun levers are ordered "in descending size" without
  measurement, and the ordering is probably inverted.
- 🔵 **Performance**: 0191's saving is hard-capped at ~2.48 ms, so "name the
  figure at which 0191 must land" may be unsatisfiable for the common gap.
- 🔵 **Performance**: The bootstrap flavour, resample count, one- vs two-sided
  bound, and the second run's role in the decision are all unspecified.
- 🔵 **Portability**: Of four shipped platforms, darwin-x64 and linux-arm64 are
  exercised by no CI lane at all, yet the hand-off is named in the singular.
  **Verified.**
- 🔵 **Portability**: `os.process_cpu_count()` requires Python 3.13+, so the
  harness would not run unmodified on a stock linux host.
- 🔵 **Portability**: The power probes go vacuous on exactly the hosts the linux
  hand-off will use, and `pmset`'s subcommand is unnamed.
- 🔵 **Portability**: Baseline recovery is verifiable only inside this jj
  workspace — no sha256 is recorded for independent validation.
- 🔵 **Documentation**: The five backfilled slots sit six lines below a "Cost
  figures" paragraph anchoring `B = 35.1 ms` by a different method, with no link
  between them.

#### Suggestions

- 🔵 **Safety**: OD-1 weighs its options on evidence and cost but not on blast
  radius; add that column and pre-register (a) as the default.
- 🔵 **Architecture**: Give the deferred committed-harness / measurement-policy
  decision an owner — this is the epic's third hand-rolled warm-path harness.
- 🔵 **Performance**: Record `p90(G)/p90(B)` as non-gating context; for a hook on
  every Bash tool call the tail is what users feel.
- 🔵 **Portability**: Permit a mirrored base URL for the discarded warm-up only,
  so the procedure is runnable without public GitHub reachability.

### Strengths

- ✅ The `--fail-safe` short-circuit is correctly identified as the dominant
  threat to a spurious PASS, with the right mechanism (`Failed` → swallowed →
  exit 0 without exec'ing the sub-binary) and the right response (abort on first
  mismatch, not post-filter). Four lenses independently called this the single
  most valuable guard in the design.
- ✅ OD-3's insistence that the residual be disagreement with *independent*
  measurements rather than the closure error of a contiguous partition is a
  genuine methodological correction — the proposed partition really would close
  by construction and detect nothing.
- ✅ Citation accuracy is exceptional: Standards verified roughly twenty-five
  file:line references with no hallucinated or drifted line numbers, including
  the awkward ones across `bin/accelerator`, `.gitignore`, and three other
  meta documents.
- ✅ The release-profile requirement for the `reverify` measurement is correct and
  well-reasoned — debug `sha2`/BLAKE2b throughput differs by an order of
  magnitude, and aarch64 `sha2` selects hardware SHA instructions at runtime
  while BLAKE2b does not, so the composite figure is architecture-specific and is
  explicitly fenced against the linux hand-off inheriting it.
- ✅ The diagnosis of unmeasured warm-path waste is accurate and new:
  `install_crypto_provider`, `TrustedKeys::embedded` and `Fetcher::new` — a
  reqwest client spawning a tokio runtime thread plus a rustls `ClientConfig`
  over bundled webpki roots — are all constructed before `cache::find` and none
  is used on a hit.
- ✅ The work-item-versus-plan split in Phase 4 is precisely right and
  non-obvious: the 0169 plan genuinely has no Validation Results section, while
  the work item has exactly five `_pending_` markers under its heading.
- ✅ Per-sample decomposition with the explicit statement that term medians are
  not expected to sum to the `G` median is correct handling of a non-additive
  statistic that most budgets get wrong.
- ✅ The inode/mtime invariance check is a sound, cheap branch witness — every
  non-hit route ends in `cache::store`, which renames a fresh inode over the
  entry.
- ✅ Provenance discipline on borrowed figures is mostly careful: 0188's delivered
  figures are distinguished from its superseded plan-table rows, and the plan
  flags the superseded pair explicitly.
- ✅ Darwin-only scope is stated rather than implied, the three unverified
  platforms are counted, and the hand-off is required to exist as a named work
  item rather than prose in a closed plan.
- ✅ The overrun consequence is pre-registered and a straddling interval counts as
  an overrun, removing the discretion that lets a gate quietly relax.

### Recommended Changes

1. **Replace OD-1's option set** (addresses: OD-1(b) infeasible; key-swap blast
   radius; OD-1(a)×OD-2 inconsistency; missing dominant option)
   Delete option (b) — it is infeasible as specified and mutates the trust root
   for less evidence than the alternative. Add the option Architecture
   identifies: bootstrap term from a **built-in** dispatch through the shipped,
   unmodified bootstrap, launcher-internal terms from independent
   micro-measurements. Pre-register that as the default, state the OD-1×OD-2
   compatibility matrix explicitly, and either delete (c) or say plainly that it
   terminates the plan without discharging Phase 3.

2. **Fix the per-sample equivalence assertion** (addresses: envelope mismatch)
   Reuse the existing `normalise()` mapping at
   `generate_decision_table.py:160`, assert `(decision, reason)` equality after
   normalisation, pin the expected reason text per variant, and record both raw
   envelope shapes verbatim so the normalisation is auditable.

3. **Replace the VCS-based cleanup gates with positive assertions** (addresses:
   revert gate blind; partial-revert states; abort paths skip cleanup)
   Assert non-existence of the baseline, the dev marker and any `keys/*.sec`;
   assert the sha256 of `keys/accelerator-release.pub` against a value recorded
   *before* any change; assert the cached launcher and `.minisig` digests and a
   successful shim verification. Make each artefact's removal a criterion of the
   phase that creates it, and add a restore-the-trust-chain checklist that MUST
   run on any abort.

4. **Make the retraction a substitution, and branch Phase 4 on the outcome**
   (addresses: 0189 closure guard; Phase 4 assumes PASS; third 0169 document)
   Replace 0189's release-cut blocker with an explicit closure guard naming the
   measurement, rather than removing it. Branch Phase 4: on PASS tick and
   backfill; on overrun or indeterminate, backfill the figures anyway and record
   a dated resolution against 0169's Phase 10 criterion naming it either an
   accepted deviation with rationale or a re-opened obligation with an owner. Add
   `meta/validations/2026-08-05-0169-…-validation.md` and 0169's own criterion at
   `:382-389` to the discharge list.

5. **Correct the borrowed figures and the gating premise** (addresses: 3× asset
   size; 35.1 ms method-incomparability; 23.84 ms wrong command; digest backend
   host-vs-platform)
   Use 0188's delivered 2,512,576 bytes and state the expected `reverify` band
   before measuring, so Phase 1 confirms or falsifies a prediction. Mark 35.1 ms
   as a sanity range not an anchor, and name 0186's *measured* 29.92 ms wherever
   the bootstrap figure is used. Attribute 23.84 ms to `jj log -r @` as an upper
   bound on `jj workspace root`. Restate the digest divergence as within-darwin
   with 0186's delivered ~7/~24 ms range.

6. **Close the statistical design before measuring** (addresses: floor policy;
   no precision target; conditional tail; undefined branches)
   Pre-register: raw medians for the gate with floors as context only; a pilot to
   size `n` for a stated CI half-width; the skip rule expressed in standard
   errors of the median ratio and evaluated on Phase 2 data; the bootstrap
   flavour, resample count and one-sided bound; a numeric agreement tolerance
   between the two runs; and a third indeterminate-by-dispersion branch with its
   own action set.

7. **Pin the fixture and the environment** (addresses: unpinned fixture; `PATH`;
   `jq`; cache-hit proof scope; `SCRIPT_DIR`)
   Pin `jj --config git.colocate=false git init --quiet`, set
   `GIT_CEILING_DIRECTORIES`, and assert the deny shape before sampling. Pin and
   print one `PATH`; resolve every tool against the subprocess environment and
   record absolute paths and versions including `jq`. Extend inode/mtime
   invariance to the cached launcher, its `.minisig` and the staged shim. Invoke
   both variants by absolute path.

8. **Complete the record** (addresses: baseline provenance; deviations slot;
   `reverify` slot; 0189 Validation Results; Open Decision defaults; linux
   hand-off mechanics; corpus gate)
   Add slots for the resolved commit id and sha256 of the recovered baseline and
   its dependency, a Deviations section mirroring 0186's, asset identity and
   dispersion for `reverify`, and a `## Validation Results` section in 0189's
   work item with a pointer to whichever plan holds the figures. Give each Open
   Decision a "Default if unresolved". Specify the new work items' ids, parent,
   frontmatter and cross-links. Add an Automated Verification block naming the
   corpus frontmatter validator.

9. **State the platform story honestly** (addresses: asymmetric sensitivity;
   hand-off prerequisites; cgroup awareness; uncovered platforms)
   State that spawn-dominated `B` shrinks more on linux than `G`, making linux
   the harder host. Split the hand-off into *measure* (a plain linux host, no
   build) and *decompose* (musl target plus `cargo-zigbuild` and `ziglang`).
   Replace the affinity-based CPU count with a quota-aware one and record the
   source. Name darwin-x64 and linux-arm64 as having no CI lane, with either host
   requirements or a recorded accepted-unverified decision.

10. **Correct the split claim and sequence the two plans** (addresses: "share no
    file"; line-anchored edits; criterion 11 ownership)
    Soften to "no code, no test, no source file — only 0189's work item, in
    disjoint passages", declare that the `ready` sibling lands first, replace
    line-number anchors into 0189 with quoted-text anchors, and amend criterion
    11 to name which plan records which artefact.

## Per-Lens Results

### Performance

**Summary**: An unusually disciplined measurement plan — it correctly identifies
the fail-safe short-circuit as a PASS-manufacturing confound, insists on
independent rather than contiguous budget terms, requires the shipped release
profile for hash-throughput measurement, and names the cache-hit `reverify` and
the unused reqwest/rustls/tokio construction as the real warm-path costs. Its
weaknesses are in the measurement design rather than the intent: the gating
phase's reachability rationale rests on an asset size that contradicts 0188's
delivered figures (~8 MB assumed vs 2.51 MB delivered), the gate ratio is derived
from a single fixture whose dominant term is a conditional `jj` spawn, and three
decisions that can flip a 10%-margin verdict are left to be made after the
numbers are seen. As written the plan would most likely produce a defensible
*number* but an indeterminate *verdict*, with an unbudgeted launcher-startup
boundary.

**Findings**: 8 major, 6 minor, 1 suggestion — asset-size premise 3× wrong;
`B` anchored to a method-incomparable figure; gate ratio an artefact of one
fixture and tool set; floor-subtraction deferred post-hoc; conditional tail
ill-defined (wrong phase, wrong dispersion scale, data-dependent); OD-1(a)×OD-2
inconsistency; no precision target; unbudgeted launcher-startup term. Minors:
release profile not carried to the instrumented build; intercept-free ms-per-MB
model; 23.84 ms is a different command; lever ordering probably inverted; 0191
capped at ~2.48 ms; bootstrap flavour and second-run role unspecified.

### Correctness

**Summary**: Rigorous about measurement validity, and its citations of the
launcher and bootstrap control flow all check out. Correctness problems
concentrate in three places: OD-1(b) is self-contradictory because the
bootstrap's verification key and the launcher's embedded key are literally the
same file; the load-bearing per-sample B/G equivalence assertion cannot succeed
because the two guards emit structurally different envelopes; and the overrun
policy has two branches with no defined verdict or action. Several verification
criteria are also structurally unable to fail, because the artefacts they check
are gitignored.

**Findings**: 2 critical, 10 major, 4 minor — OD-1(b) infeasible; envelope
mismatch. Majors: conditional tail unevaluable and Phase 3 PASS unsatisfiable if
skipped; indeterminate-by-dispersion branch undefined; floor policy; OD-1×OD-2
matrix; unpinned fixture emits warn not deny; cache-hit proof too narrow;
gitignored revert gates; residual cross-checks cover 2 of 5 rows; mmap not
behaviour-preserving; `jq` an unrecorded dependency and spawn.

### Architecture

**Summary**: Structurally a decision document wearing a plan's clothes — one of
four phases is implementable today, the other three gated on Open Decisions with
no decision procedure, no owner, and one option that makes a later phase's own
criteria unsatisfiable. The split from the sibling is clean on code, tests and CI
evidence but not on documents. Most seriously, the plan exists to repair an
orphaned obligation yet reproduces that shape twice over: it retracts 0189's only
closure blocker while its own figures are pending in a draft plan, and its overrun
branch hands the obligation to a draft, unscheduled 0191 with no procedure for a
closed-`done` 0169 whose gate has now been measured as failed.

**Findings**: 2 critical, 4 major, 2 minor, 1 suggestion — closure guard
retracted prematurely; Phase 4 assumes PASS. Majors: Open Decisions have no
procedure or owner; OD-1 omits the dominant option; baseline pins only the guard
while 0199 is scoped to change its dependency; "share no file" false and
criterion 11 unowned. Minors: 0191 edge contingent and buried; linux hand-off has
no graph placement.

### Standards

**Summary**: Unusually disciplined against project conventions — every one of
roughly twenty-five file:line citations verified exactly, frontmatter is fully
house-conformant, the gitignore/exec-bit-invariant claim holds end to end, and the
musl/`zigbuild` claim matches the build system. The gaps are in change-management
discharge: Phase 4 tracks 0169's Validation Results slots and the plan's criterion
but omits the work item's own unchecked acceptance criterion, and names no
automated gate for the four meta documents it edits despite an unconditional
corpus-frontmatter validator over `meta/`.

**Findings**: 2 major, 5 minor, 2 suggestions — 0169's own criterion never ticked;
no corpus-validation gate. Minors: `**File**`/`**Changes**` labelling absent from
seven of eleven items; new work items lack id/frontmatter conventions; `.sh`
extension the 0186 precedent avoided; stdin envelope diverges from the shipped
one-field contract; "share no file" false. Suggestions: `## Open Decisions` is a
novel section with no template slot; the throwaway example is the workspace's
first `examples/` target and is in pedantic-clippy scope.

### Portability

**Summary**: Self-aware about platform scope — darwin-only verification stated
explicitly, ms-per-MB labelled architecture-specific, digest backend and host
identity recorded as first-class outputs, power probes additive so one harness
runs on both OSes. The weaknesses are in the transfer story: the gate ratio is
composed of terms whose platform sensitivities move in opposite directions, and
the plan never states that a darwin PASS is arguably adverse rather than merely
weak evidence for linux. Two supporting claims are also wrong — the digest
divergence is a within-darwin host fact 0186 refused to call a platform fact, and
the stated cgroup-awareness mechanism is affinity-based, not quota-based.

**Findings**: 5 major, 4 minor, 1 suggestion — asymmetric platform sensitivity
undeclared; digest divergence mis-scoped with a retired figure; affinity vs quota;
hand-off prerequisites describe an unneeded cross-compile; `PATH` unspecified.
Minors: darwin-x64 and linux-arm64 have no CI lane; `os.process_cpu_count()`
needs Python 3.13+; power probes vacuous on the hand-off's hosts; baseline
recovery verifiable only inside this jj workspace.

### Safety

**Summary**: A measurement-only plan, but the route it leaves open under OD-1
reaches into the plugin's signature trust chain — the one tracked file that both
the bootstrap verifies against and `build.rs` embeds — and into the launcher's
live cache root, which is also where the PreToolUse git guard and every
SessionStart hook dispatch from. Protective mechanisms for the *measurement* are
strong; for the *developer's installation* they are weak: the single revert gate
is structurally blind to almost every artefact the plan creates, and cleanup
obligations are gated in phases the plan's own abort paths skip.

**Findings**: 3 critical, 5 major, 1 minor, 1 suggestion — OD-1(b) only
implementable by overwriting the shared trust root; a swapped key is
auto-snapshotted, passes CI and has a documented path to shipping; the revert gate
is blind. Majors: cleanup gated in phases abort paths skip; OD-1(a) defeats a
three-gate defence with no marker removal; the unverified-launcher log is flooded
and never reset; `bin/` is not actually forced; partial-revert states are silent
and one makes the git guard fail open.

### Documentation

**Summary**: The plan's deliverable is a record, and it is unusually disciplined
about that — every Validation Results section carries an explicit slot list, the
harness is recorded verbatim rather than by cross-reference, cited figures carry
provenance, and every file:line citation checked resolves. The gaps are in the
discharge half: Phase 4 asserts 0169's obligation lives in two documents when it
lives in three, 0189's stale claim appears in four passages of which two are
named, and the one input that makes `B` reproducible is demanded in prose but
absent from every slot list and criterion.

**Findings**: 1 critical, 8 major, 1 minor — the third 0169 document (validation
artefact) carries an unchecked latency checkbox and the retracted premise.
Majors: 0189's four stale passages; missing baseline provenance slot; OD-1(a)
omits two required dev-override inputs; no deviations slot for the 20→50 change;
`reverify` slot lacks asset identity and dispersion; 0189's own figures land
nowhere durable; Open Decisions lack defaults and owners; linux hand-off lacks
mechanics.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-08-12

**Verdict:** REVISE

All seven lenses re-ran against the restructured plan (`## Open Decisions`
converted to a Phase 1 spike with pre-registered defaults; roughly forty pass-1
findings applied). Raw: 74 findings — Performance 14, Architecture 13,
Correctness 10, Portability 10, Documentation 10, Safety 9, Standards 8.
Aggregated: **6 critical, 31 major, 21 minor, 3 suggestions**.

**Every one of the six criticals is a defect introduced by the pass-1 fix
batch, not a pass-1 finding left unaddressed.** That is the dominant signal of
this pass and it recurs: pass-1 fixes introduced criticals, and so did this
batch. Findings are being applied faster than they are being verified.

### Previously Identified Issues

Pass-1 criticals:

- 🔴 **Safety / Correctness**: OD-1(b) infeasible — the two keys are the same
  file — **Resolved.** Option deleted outright, moved to What We're NOT Doing
  with the mechanism recorded. Safety confirms no route in the plan now writes
  `keys/accelerator-release.pub`.
- 🔴 **Safety**: A swapped release key passes CI and can ship — **Resolved** by
  the same deletion.
- 🔴 **Safety / Correctness**: The revert gate is structurally blind —
  **Partially resolved.** VCS gates replaced by an Abort checklist of positive
  assertions, which is the right shape; but three of its six items compare
  against a baseline nothing captures, and item 1 names a file the plan never
  creates. See new criticals.
- 🔴 **Correctness**: The per-sample B/G equivalence assertion cannot succeed —
  **Not resolved; made worse.** See new critical 1.
- 🔴 **Architecture**: Retracting 0189's closure guard re-orphans the obligation
  — **Partially resolved.** Substitution replaces removal, but the installed
  guard keys on the outcome being *recorded* rather than on the gate *holding*,
  and Phase 4 records it in the same pass — so it is born discharged, while a
  stronger unticked acceptance criterion is unblocked. Possible net weakening.
- 🔴 **Architecture**: Phase 4 written as though the gate passes — **Resolved**
  for PASS/OVERRUN/INDETERMINATE; two further reachable outcomes still have no
  discharge path.
- 🔴 **Documentation**: 0169's obligation lives in three documents —
  **Resolved.** All four locations across three documents verified by
  Documentation and Standards as existing and carrying live obligations.

Pass-1 majors: the large majority resolved — asset size, floor policy, fixture
pinning, `jq`, `PATH`, cache-hit proof scope, cleanup ownership, mmap caveat,
0169's own criterion, the corpus gate, the "share no file" claim, the linux
hand-off split, affinity-vs-quota. Four recur in altered form (residual
falsifiability, per-sample decomposition, precision target, digest backend).

### New Issues Introduced

#### Critical

- 🔴 **Correctness / Documentation / Standards**: `normalise()` does not map the
  Rust guard's deny envelope. `generate_decision_table.py:160` returns
  `("block", reason)` only for the shell's legacy shape, then checks
  `hookSpecificOutput.systemMessage`, else falls through to `("allow","")`. The
  Rust guard emits `permissionDecision`/`permissionDecisionReason`
  (`kernel/src/hooks.rs:29-31`) and so normalises to `("allow","")` — as does
  empty stdout from a swallowed fail-safe dispatch. The gate is simultaneously
  unsatisfiable across variants and blind to the failure it exists to catch.
  Standards names the genuine dual-shape mapper:
  `cli/vcs-cli/tests/guard_decision_table.rs:99-141`, which labels the blocked
  outcome **`block`**, not `deny`.
- 🔴 **Safety / Correctness / Architecture / Documentation**: The Abort
  checklist compares against a pre-measurement state no phase captures. Items 3
  to 5 reference the key digest, the launcher/`.minisig`/shim digests and the
  log byte count "recorded before any phase ran"; no step, criterion or
  Validation Results slot captures them, and the checklist explicitly covers a
  Phase 1 abort. Three of six items are unevaluable on the first run.
- 🔴 **Architecture / Performance**: Phase 1's pilot depends on Phase 2. A pilot
  *pair* needs the recovered baseline, the pinned fixture, the envelope, the
  validity gate and the precondition block — all Phase 2 items — contradicting
  "Phase 1 depends on nothing" and leaving the baseline Phase 1 must create with
  no owning cleanup criterion.
- 🔴 **Architecture**: Phases 2 and 3 are one atomic session. Phase 3's
  per-sample decomposition requires Phase 2's harness to have emitted per-sample
  term data; Phase 2 specifies only wall-clock brackets. Phase 3 is unexecutable
  once Phase 2 closes, and "independently integratable" is false for the pair.
- 🔴 **Performance / Correctness / Architecture**: Budget rows 1 and 2 cannot be
  separated under the SQ-1 default. A built-in dispatch already contains the
  `execve`, dynamic loading, `logging::init` and clap parse that row 2 budgets
  separately, so the table double-counts ~2.4 ms — or forces the plan onto the
  SQ-1 fallback, the trust-gate-defeating route it labels last resort.
- 🔴 **Performance**: The dominant budget term carries no independent
  cross-check. SQ-3 requires one for every term above 10% of `G`; the
  bootstrap row is 60-75% of `G` and its only measurement *is* its measurement.
  The falsifiable residual covers ~20-25% of `G`, so the 25% threshold passes
  while the bulk of warm-path cost is unexplained — the "closes by construction"
  failure the plan's Key Discoveries sets out to avoid.

#### Major (selected; 31 aggregated)

- 🟡 **Performance**: The INDETERMINATE branch permits optional stopping —
  sampling until the one-sided bound crosses 1.1 inflates the false-PASS rate
  above the nominal 5%, under a heading that pre-registers against relaxation.
- 🟡 **Correctness / Performance**: OVERRUN and INDETERMINATE overlap. A point
  estimate under 1.1 with a straddling CI satisfies both, and they prescribe
  mutually inapplicable actions; "comfortably" carries the discrimination and is
  never given a number.
- 🟡 **Architecture / Performance**: Two reachable outcomes have no branch —
  an invalidated session (run-to-run disagreement) and a Phase 1
  design-infeasible finding. Neither reaches Phase 4, the sole recorder.
- 🟡 **Architecture**: The closure guard is born discharged and may be a net
  weakening; 0189's unticked latency acceptance criterion is the stronger guard
  and the same item unblocks it.
- 🟡 **Portability**: "Linux is the harder host" is plausibly inverted. 0186's
  breakdown shows `G`'s bootstrap term is itself overwhelmingly spawn cost, and
  linux ships coreutils `sha256sum` universally while darwin risks the slow
  `shasum`. Phase 4 propagates the claim into a new work item.
- 🟡 **Portability**: The CPU-count fallback chain has no branch that works on
  darwin — `os.sched_getaffinity` is Linux-only, `os.process_cpu_count()` needs
  3.13+, macOS system `python3` is 3.9. The precondition block crashes on the
  primary measurement host.
- 🟡 **Portability**: The cgroup v2 `cpu.max` procedure is under-specified in
  four ways that make it silently no-op on the container hosts it targets.
- 🟡 **Portability**: Nothing downstream conditions on the digest backend, so a
  verdict decided by whether `/sbin/sha256sum` exists on one laptop is recorded
  in three documents as platform-neutral.
- 🟡 **Standards**: The `cargo nextest` gate names a non-existent package
  (`corpus-cli` is the directory; the package is `accelerator-corpus`) and omits
  `--manifest-path cli/Cargo.toml` — there is no root `Cargo.toml`. The one
  automated verification the plan relies on cannot run.
- 🟡 **Correctness / Documentation / Standards**: Abort-checklist item 1 names
  `bin/.tmp-vcs-guard-baseline.sh`; Phase 2 creates the extensionless path.
- 🟡 **Documentation / Standards**: The `:615` citation is wrong (the note is at
  0186 plan `:1252`) and 29.92/35.1/10.6/3.72 ms live in the 0186 **work item**,
  which References does not list.
- 🟡 **Documentation**: 0189 asserts the stale premise in **five** passages, not
  four; the count is hard-coded into a discharge instruction.
- 🟡 **Documentation**: The stale release-asset premise also lives in the two
  0169 documents Phase 4 already edits; the retraction covers only 0189.
- 🟡 **Documentation**: The Deviations slot has no owning success criterion and
  omits the largest deviation — Phase 3 gates on a CI upper bound where 0169's
  criterion says median, at a different `n` and a different payload.
- 🟡 **Safety**: The Abort checklist has no trigger — no trap, no `finally`, no
  script. A crashed session leaves the dev-launcher marker and an unreviewed
  executable in the live cache root indefinitely.
- 🟡 **Safety**: Blind byte-count truncation of `.accelerator-unverified.log`
  destroys genuine integrity records, including any from a concurrent session,
  with no copy retained.
- 🟡 **Safety**: The instrumented binary in `cli/target/` is never removed or
  rebuilt, and that path is where a contributor's normal dev-launcher build
  lives.
- 🟡 **Safety**: Instrumentation is not constrained off stdout, which is the
  guard's decision envelope — interleaved output fails the guard open.
- 🟡 **Performance**: Cold-process discipline is applied only to `reverify`;
  `install_crypto_provider` is idempotent, so in-loop medians report near-zero
  for costs `G` pays in full, mis-ranking the overrun levers.
- 🟡 **Performance**: `reverify` is measured as one composite, so the overrun
  branch's requirement to order levers "by measurement rather than assumption"
  is unsatisfiable from the measurement that precedes it.
- 🟡 **Performance**: The pilot-`n` procedure states no target half-width and no
  extrapolation rule; a one-sided bound has no half-width, yet both the sizing
  rule and the agreement tolerance are stated in half-widths.
- 🟡 **Correctness / Architecture / Performance**: Per-sample decomposition is
  undefined under the SQ-1 default — independent micro-measurements have no
  pairing with `G` samples.
- 🟡 **Correctness**: SQ-3's residual definition removed the closure check. Under
  the SQ-1 default the terms are independent, so their sum against `median(G)`
  is not structurally zero — it is 0186's discipline. Terms summing to 60% of
  `G` could report a near-zero residual.
- 🟡 **Standards**: `## Validation Results` is a work-item section in this
  corpus, not a plan section; Phase 4 simultaneously creates a second one in
  0189's work item.
- 🟡 **Architecture**: Phase 4's "accepted deviation with rationale" re-admits
  the post-hoc relaxation the plan pre-registers against, with no named
  approver.
- 🟡 **Architecture**: The `reverify` example replicates a private method via
  public API on a working tree the sibling plan changes first, decoupling the
  cross-check from the artefact under measurement.
- 🟡 **Portability**: Linux measuring prerequisites understate the runtime
  dependency set; the musl decompose prerequisite is `rustup target add`
  natively, not `cargo-zigbuild`.
- 🟡 **Documentation**: "The three follow-up work items" is asserted
  unconditionally but the third is conditional on the overrun branch.
- 🟡 **Architecture**: Phase 3 mutates 0189's frontmatter while Phase 4 owns
  0189's edits and runs the only corpus gate.

### Tradeoffs and Lens Disagreements

- **Platform transfer direction**: Architecture lists the adverse-transfer claim
  among the plan's strengths; Portability derives from 0186's breakdown that it
  is plausibly inverted. Portability's reasoning is grounded in figures;
  Architecture's is not. Recommendation: downgrade to "direction unknown, both
  variants spawn-dominated" and derive it from the Phase 3 budget.
- **Reuse vs correctness on the normaliser**: Architecture credits reusing
  `normalise()` as sound reuse of a surface the repo owns; Correctness,
  Documentation and Standards each independently show it does not handle the
  Rust shape. Reuse is still right — of
  `cli/vcs-cli/tests/guard_decision_table.rs:99-141`, not the Python fixture.
- **Spike placement**: Standards notes the repo ships `conduct-spike`, whose
  contract records outcomes on a *spike work item*. Inlining the spike in a plan
  buries its durable value in a `draft` artefact.

### Assessment

The restructure was directionally right and closed the trust-root criticals
cleanly. But six new criticals, all self-inflicted, against a plan that has now
had two full review passes says the failure is in the process, not the content:
each fix batch is written faster than it can be verified, and the fixes land in
a document whose parts are tightly coupled, so a change to one phase silently
invalidates another.

Recommended next step is **not** a third fix batch. Raise SQ-1 to SQ-3 as a
spike work item per the repo's own `conduct-spike` contract, let it answer the
method questions against real measurements, and reduce this plan to the
measurement it can actually specify once those answers exist. The statistical
design, the budget decomposition and the outcome policy have each now failed
review twice while being authored ahead of the evidence that would settle them.
