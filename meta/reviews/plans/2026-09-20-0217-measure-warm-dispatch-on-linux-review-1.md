---
type: "plan-review"
id: "2026-09-20-0217-measure-warm-dispatch-on-linux-review-1"
title: "Plan Review: Measure Warm Dispatch on Linux"
date: "2026-09-20T21:03:00+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-20-0217-measure-warm-dispatch-on-linux"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "portability", "compatibility", "safety", "documentation"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-21T21:55:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Measure Warm Dispatch on Linux

**Verdict:** REVISE

The plan is architecturally well-grounded — it leans into the data-driven
`PLATFORM_TABLE` (adding a platform is an additive, open-closed change), respects
the functional-core / imperative-shell split, reuses the existing
`closure_verdict` acceptance seam rather than inventing control flow, and
sequences the pure darwin-testable Phase 1 ahead of the VM chain to de-risk. But
two independent lenses (Correctness, Architecture) found that the Phase 1
graceful-degradation change surface is **incomplete** — a Perl-less host still
aborts, just at a later line — and three lenses (Correctness, Architecture, Code
Quality) found that `chip`/`shasum` are **exact-match calibration keys, not
free-text**, so the plan's `chip="<brand> (musl)"` (compounded by `observed_chip`
returning `{` on Linux) makes the `calibrated` verdict unsatisfiable. Both defeat
named acceptance criteria (AC 5 and AC 2), so the plan needs revision before
implementation.

### Cross-Cutting Themes

- **Phase 1 degradation is incomplete — the session still aborts** (flagged by:
  Correctness 🔴, Architecture 🟡) — the enumerated change surface
  (`build_rig`/`assert_backends`/`sample_blocks`/`analyse`) misses two more
  consumers of the fallback farm: `record_provenance` → `tool_provenance` runs
  `farm.iterdir()` on the never-created directory (FileNotFoundError), and
  `close_the_budget` → `measure_digest_bracket` runs `shasum` through the missing
  farm (PreconditionFailureError). AC 5 is not met and the planned unit tests
  would not catch it.
- **`chip`/`shasum` are exact-match keys, not free-text** (flagged by:
  Correctness 🔴, Architecture 🟡, Code Quality 🟡) — `unconfirmed_calibration_fields`
  compares recorded vs observed by exact string equality, so appending `(musl)`
  guarantees a mismatch. Worse, `observed_chip` returns `splitlines()[0]` of
  `lscpu -J`, which is `{`, so even the raw brand string can never match. The
  `calibrated` note the Desired End State and Phase 4 require can never hold.
- **Phase 1 signatures break existing tests, and analyse wiring is
  under-specified** (flagged by: Compatibility 🟡, Test Coverage 🟡, Correctness 🔵)
  — the required keyword-only `fallback_available` and the non-defaulted `Rig`
  field make existing call sites TypeErrors; `test_a_host_without_shasum_makes_the_fallback_cells_inapplicable`
  asserts the *removed* abort behaviour and must be inverted, not supplemented;
  and `analyse` needs a new parameter + call-site change the snippet omits. Phase
  1's own gate (`test:unit:tasks` green) cannot pass until this is resolved.
- **`power_state` keys on `probe[0]`, forcing artificial command diversity**
  (flagged by: Code Quality 🟡, Architecture 🔵, Test Coverage 🔵) — the plan
  dodges key collapse by picking `cat`/`head`/`grep` leaders, including
  `grep -h ""`, a probe whose only purpose is a distinct key. It reads as a
  mistake, has no guard test, and traps the next probe author.
- **`power_state` contract change ripples cross-platform** (flagged by: Code
  Quality 🔵, Compatibility 🔵, Architecture 🔵, Test Coverage 🔵) — the rewrite
  retires the `"unknown"` sentinel, drops the function's docstring, collapses
  three failure modes, and flips the existing `test_an_absent_probe_yields_unknown`
  test; the "no-op on any host with both backends" claim covers only the
  fallback change, not this one.
- **Commit / placeholder / record discipline is manual and un-backstopped**
  (flagged by: Safety 🟡🟡🟡, Portability 🟡) — the Phase 2 loose-gate run writes a
  valid-looking, closure-true record with no deletion step; the Phase 4 gate
  can't tell a degraded (fewer-cells) run from a full one; and revert relies on
  the operator with `check`/`test` deliberately disabled.

### Tradeoff Analysis

- **Fix `power_state` keying / dynamic power probes vs. minimal change surface
  and pre-registered determinism**: Code Quality, Architecture and Portability
  all want the shared core improved — key on full argv, enumerate
  `/sys/class/power_supply/*` — but the plan's stated discipline is a bounded
  measurement task, not a harness re-authoring, and the harness's value rests on
  *pre-registered, reproducible* constants rather than run-time enumeration.
  Recommendation: take the minimal correctness fix now (a distinctness guard test
  + `cat` for all `/sys` reads if keying is left as-is) and record the `probe[0]`
  keying limitation as a known constraint for the `Linux/x86_64` secondary, rather
  than widening this item into a core redesign.

### Findings

#### Critical

- 🔴 **Correctness + Architecture**: Graceful degradation still aborts —
  `record_provenance` and `close_the_budget` touch the unbuilt fallback farm
  **Location**: Phase 1, Changes 1–3
  Phase 1 guards only `build_rig`/`assert_backends`/`sample_blocks`/`analyse`, but
  `record_provenance` → `tool_provenance(rig.fallback_farm)` calls `iterdir()` on
  the never-created directory, and `close_the_budget` → `measure_digest_bracket`
  runs `shasum` through it — so a Perl-less host still aborts before reaching
  `analyse`, defeating AC 5, and the planned tests would pass anyway.

- 🔴 **Correctness + Architecture + Code Quality**: `chip`/`shasum` are
  exact-match keys and `observed_chip` returns `{` on Linux, so the `calibrated`
  note can never hold
  **Location**: Key Discoveries (`observed_chip`); Phase 3, Change 2
  `unconfirmed_calibration_fields` compares by exact equality; appending `(musl)`
  breaks the match, and `observed_chip` returns the first line of `lscpu -J`
  (`{`), never the brand. AC 2, the Desired End State, and the Phase 4
  `note == "calibrated"` criterion are unsatisfiable as planned.

#### Major

- 🟡 **Compatibility + Test Coverage**: Phase 1 signatures are source-incompatible
  and an existing test asserts the removed behaviour
  **Location**: Phase 1, Changes 1–2; Migration Notes
  `assert_backends` gains a required keyword-only `fallback_available` and `Rig` a
  non-defaulted field, so four existing call sites become TypeErrors, and
  `test_a_host_without_shasum_makes_the_fallback_cells_inapplicable` asserts the
  abort the plan removes. "Migration Notes: None" understates this.

- 🟡 **Test Coverage + Correctness + Compatibility**: `analyse()`'s
  acceptance-reason attachment is the net-new AC 5 logic — untested and
  under-specified
  **Location**: Phase 1, Change 3
  `analyse`'s signature (`analyse(entry, samples, *, floors, elapsed)`) and its
  call site take no `fallback_available`; `FALLBACK_CELLS` must be exactly
  `{C3, C4, C6}`; C6 is non-gating so `closure_verdict` can never catch a missing
  C6 reason. No `analyse` unit test exercises the wiring.

- 🟡 **Safety**: Graceful degradation lets a fewer-cells run pass the Phase 4
  commit gate as fully calibrated
  **Location**: Phase 1, Change 3; Phase 4 Success Criteria
  The gate checks only `validity`, `closure_verdict`, and `calibration.note`, all
  of which stay green if `shasum` disappears between Phase 2 and 4 — a fail-open on
  the very run meant to produce the calibrated evidence. Bind `accepted_by` to the
  cell actually lacking an interval, not to `fallback_available` alone.

- 🟡 **Safety**: The Phase 2 loose-gate record is a committable-looking artefact
  with no deletion step
  **Location**: Phase 2, Change 3; Phase 4, Change 4
  Phase 2 writes a full numbered `warm-dispatch-N.json` reporting `valid` +
  `closure_verdict: true` on 10000 ms placeholder ceilings; `teardown` doesn't
  remove it, no test validates records, and Phase 4 says only "commit the latest
  N" — so a placeholder-gated record can be swept in and consumed by 0219.

- 🟡 **Safety**: Revert discipline is manual with the safety gate deliberately
  disabled
  **Location**: Phase 2, Change 2; Risks and Mitigations
  Phase 2 skips `check`/`test` and relies on the operator remembering "revert
  before Phase 3". The code entry is contained (shared dict key + lockstep test),
  but the record artefact is not, and a mid-workflow commit can carry either into
  history before CI sees it.

- 🟡 **Code Quality + Architecture + Test Coverage**: `power_state` keying on
  `probe[0]` forces artificial command diversity and is unguarded
  **Location**: Phase 3, Change 2 (`power_probes`); Phase 1, Change 4
  Two probes sharing a leading command silently collapse to one reading. The plan
  picks `cat`/`head`/`grep` — including `grep -h ""` — purely to keep keys
  distinct, with no test pinning the invariant.

- 🟡 **Test Coverage**: The linux farm-completeness test guards less than the
  darwin coverage it parallels
  **Location**: Phase 3, Change 4
  It mirrors only the dispatch-side `bin/accelerator` check, omitting a parallel
  of `test_the_recovered_baseline_spawns_nothing_the_farm_lacks` (the B variant),
  and `spawned_executables` returns neither `bash` nor `true`, so floor binaries
  are unguarded — a baseline-side or floor omission surfaces only at guest
  runtime.

- 🟡 **Portability**: VM provisioning is prose-only and unpinned, so the
  measurement environment cannot be reproduced
  **Location**: Phase 2, Change 1
  Base OS is loosely "Debian/Ubuntu"; Colima version, kernel, apt packages and the
  rust toolchain source are unpinned. The harness's whole value is reproducible
  pre-registered constants, yet the environment that produces them can't be
  reconstructed for the 0219 confirmation or an audit.

- 🟡 **Portability**: Shared `linux-arm64` `power_probes` bake in host-specific
  `/sys` paths that won't generalise
  **Location**: Phase 3, Change 2 (`power_probes`)
  `AC/online`, `BAT0/status`, `cpu0/cpufreq/scaling_governor` are per-host; laptops
  name supplies `ADP1`/`BAT1`, servers/VMs often expose none. Keyed
  architecture-only and consumed by 0219 for every `linux-arm64` host, the probes
  will most likely read nothing on the VM itself.

- 🟡 **Portability**: The cross-target execution fallback isn't wired into
  `decompose_terms`; a built-but-not-run cargo outcome silently drops the
  throughput
  **Location**: Risks and Mitigations; Phase 4, Change 2
  `decompose_terms` shells `cargo test --test warm_terms` and scrapes stdout; if
  cargo builds but skips execution for the cross target, the return code is 0
  while the parse yields empty, so AC 6 is silently unmet with a passing-looking
  run and the manual fallback lives outside the harness.

- 🟡 **Correctness**: The ceiling-derivation rule does not reproduce darwin's own
  C2/C4 exemplars
  **Location**: Phase 3, Change 1; Phase 3 Manual Verification
  Over C2's p90 of 38.230, "smallest multiple of 10 ms with ≥18% headroom" yields
  50 (30.8% headroom), not darwin's 60; C4 over 55.291 yields 70, not 80. Darwin's
  C2/C4 reproduce only if the base is the interval upper bound, not the point
  estimate — the base is unpinned.

- 🟡 **Documentation**: The README "measure namespace" operator prose stays
  darwin-only after Linux becomes a calibrated platform
  **Location**: Phase 3, Change 3; Desired End State
  Phase 3 updates only the `### Criterion constants` bullets, leaving "A quiet
  darwin-arm64 host" as the run guide. A reader (including 0219) can't learn from
  the README that `linux-arm64` is now runnable and calibrated.

#### Minor

- 🔵 **Code Quality + Compatibility + Architecture + Test Coverage**:
  `power_state` rewrite drops its docstring, collapses three failure modes,
  retires the `"unknown"` sentinel, and flips an existing test
  **Location**: Phase 1, Change 4
  The snippet carries no docstring (every function in the file has one, and the
  current one still says `unknown`), folds absent/denied/empty into one string,
  and changes the serialised `quietness.power` vocabulary cross-platform. The
  retirement should be a deliberate, recorded decision with the existing test
  replaced.

- 🔵 **Code Quality + Architecture**: `FALLBACK_CELLS` is a stringly-typed
  parallel source of truth
  **Location**: Phase 1, Change 3
  Which cells depend on the fallback backend is already encoded in `analyse`'s
  arm-to-cell wiring and the `assert_backends` message; a third hardcoded
  `{"C3","C4","C6"}` set can drift. Prefer deriving acceptance from
  `interval is None and not fallback_available`.

- 🔵 **Code Quality**: `fallback_available` is hard-wired to `shutil.which` inside
  `build_rig`, complicating testing
  **Location**: Phase 1, Change 1
  Reading global PATH state directly forces the Phase 1 tests to monkeypatch or
  manipulate PATH, at odds with the codebase's Protocol-injection style. Compute
  it at the composition root and inject it (note `build_farm` already calls
  `shutil.which`).

- 🔵 **Test Coverage**: The Phase 1 test list restates an existing test and omits
  positive-direction assertions
  **Location**: Phase 1 Success Criteria
  The `closure_verdict` case already exists; only the negative `sample_blocks`
  direction is listed (no fallback-present counterpart), and nothing asserts
  `Rig.fallback_available` is computed correctly. A regression dropping the
  fallback arm on a healthy host is only loosely covered.

- 🔵 **Safety**: Guest/host checkout isolation and the record-transfer path are
  unspecified
  **Location**: Phase 2, Change 1; Phase 4, Change 4
  If the host tree is mounted into the guest, placeholder edits and intermediate
  records land in the host working copy; if a separate clone, the calibrated
  record must be transferred back with no defined step. Specify a separate clone
  with a verified transfer.

- 🔵 **Portability**: Reproduction is coupled to macOS + Apple Silicon via Colima
  with no portable alternative
  **Location**: Phase 2, Change 1; Assumptions
  The only documented native-aarch64 mechanism requires macOS on Apple Silicon;
  there's no path on the project's Linux CI or an arm64 Linux box (KVM/containers)
  — blocking 0219's re-derivation on one operator's host family.

- 🔵 **Portability**: VM-scoped ceilings and floor gates are committed under an
  architecture-only key applied to all `linux-arm64` hosts
  **Location**: What We're NOT Doing; Performance Considerations
  The constants gate CI arm64, bare metal and other VMs alike. Mark them
  VM-scoped/provisional in the *record and provenance*, not just plan prose, so
  0219 knows to re-derive.

- 🔵 **Portability**: The provisioning checklist omits `mise` itself and
  under-documents network egress
  **Location**: Phase 2, Change 1 (prerequisites); Change 3
  Every command is `mise run …` yet `mise` (and the uv/python it fronts) isn't
  listed; egress is documented only to the release base URL, though the build
  needs crates.io and provisioning needs apt + rustup/mise mirrors.

- 🔵 **Documentation**: "Byte-identical" mischaracterises what the lockstep test
  enforces
  **Location**: Phase 3, Change 3; Phase 3 Manual Verification
  The test float-coerces both sides and compares with `pytest.approx`, so `50` and
  `50.0` both pass (darwin stores `50.0`, README reads `50`). Reframe as numeric
  equality; keep integer rendering as a style choice, not a parser constraint.

- 🔵 **Documentation**: The transfer-verdict destination "closure notes" is not a
  convention any work item uses
  **Location**: Phase 4, Change 3
  No work item has a "Closure Notes" section; the established pattern (and what AC
  4 compares against in 0189) is `## Validation Results`. Direct the operator
  there, and give the derived throughput a stated home too.

- 🔵 **Portability**: Reproduction knowledge for the Linux target isn't captured
  in durable operator docs
  **Location**: Phase 3, Change 3
  The Colima/musl-tools/`CARGO_BUILD_TARGET`/linker how-to lives only in this
  plan, which is archived once done — leaving committed constants with no durable
  record of the environment to recreate.

- 🔵 **Architecture**: The `power_state` contract change lives in the shared core
  and alters every platform's recorded shape
  **Location**: Phase 1, Change 4; Migration Notes
  Restate the migration note to acknowledge the vocabulary change as a
  cross-platform provenance-format change and confirm no consumer or golden-file
  test keys on the literal `"unknown"`.

#### Suggestions

- 🔵 **Compatibility**: Add an HWCAP / musl-version provenance check so a
  soft-backend fallback is diagnosable directly
  **Location**: Key Discoveries (sha2/cpufeatures); Phase 4, Change 2
  The dependency chain supports ARMv8 SHA-2 under musl (verified against
  `Cargo.lock`), but if detection silently fell back, AC 6 would fail as a
  measurement outcome, hard to attribute. Record the resolved cpufeatures/musl
  version or assert `HWCAP_SHA2` on the guest.

- 🔵 **Test Coverage**: Compute the throughput as a tested pure helper, not by
  hand
  **Location**: Phase 4, Change 2
  A by-hand `asset_bytes / (median_ms × 1000)` is exactly the plausible-figure
  risk the harness architecture eliminates elsewhere. A tiny
  `sha256_throughput_mb_s(...)` with a unit test keeps it in the tested core.

- 🔵 **Test Coverage**: Frame each phase red-first and name the pre-existing tests
  that must flip
  **Location**: Implementation Approach; Phase 1
  The repo mandates red-green-refactor, but "Changes Required" precedes "Success
  Criteria" without naming the failing test that precedes each behaviour — e.g.
  `test_an_absent_probe_yields_unknown` must be migrated.

- 🔵 **Code Quality**: Unify the two mechanisms for including/excluding backend
  tools
  **Location**: Phase 1, Changes 1–2
  The fast backend is toggled per-farm via `include_fast_backend`; the fallback is
  excluded by pre-filtering `tools`. Two techniques in the farm-construction path
  is a small KISS cost — consider one backend-policy argument to `build_farm`.

- 🔵 **Correctness**: Drop the "to match the lockstep parser" justification for
  integer rendering
  **Location**: Phase 3, Change 3
  The parser accepts both `50` and `50.0`; integer rendering is a style choice for
  consistency with the darwin bullets, not a correctness constraint.

### Strengths

- ✅ Exploits the data-driven `PLATFORM_TABLE` so a new platform is an additive,
  open-closed change (new `PlatformEntry` + README bullets in lockstep), with no
  re-authoring of the criterion, cells or branches. (Architecture, Compatibility)
- ✅ Adheres to the functional-core / imperative-shell boundary: pure changes
  (`power_state`, `classify_cell`, `closure_verdict` reuse) land in
  `tasks/shared/measurement.py`; process/filesystem changes land in the
  `tasks/measure.py` shell. (Architecture, Code Quality)
- ✅ Reuses the existing `closure_verdict` contract (a `NOT_APPLICABLE` gating
  cell with a recorded `accepted_by` doesn't block closure) instead of a parallel
  degradation path — a genuinely DRY, evolutionary fit. (Architecture, Code
  Quality, Test Coverage, Correctness)
- ✅ Phase sequencing de-risks correctly: the pure, darwin-testable Phase 1 lands
  and merges before the VM chain, keeping the risky measurement off the critical
  path for the code changes. (Architecture, Code Quality, Safety, Test Coverage)
- ✅ Serialisation is forward-compatible: `accepted_by` already exists on
  `CellOutcome` and is emitted as `null`; the new `("Linux","aarch64")` key slots
  into the existing raw keying; no Rust/frontend/schema consumer breaks.
  (Compatibility)
- ✅ Corrects the research's incorrect claim that a wrong `entry.key` diverges
  cache-name prediction, modelling the actual `entry_platform()`/`host_platform()`
  derivation from host uname. (Architecture, Correctness)
- ✅ Requires genuinely native (non-emulated) aarch64 execution measuring the
  actually-shipped musl libc, so the calibration matches the real distribution
  artefact. (Portability)
- ✅ Records libc/SHA-extension facts in free-text provenance under an
  architecture-only key rather than re-keying the harness — a documented,
  conscious choice (work-item open question resolved 2026-09-15). (Portability,
  Documentation)
- ✅ Lands the entry, README bullets and a new linux-keyed farm-completeness test
  as one atomic Phase 3 change, preventing doc/code drift; the lockstep test is
  bidirectional and data-driven. (Documentation, Test Coverage, Compatibility)
- ✅ Builds on an unusually safety-conscious harness: manifest-before-creation,
  signal unwinding, idempotent restore/verify on every exit, and fail-closed on
  integrity problems (`closure_verdict=False`, `INVALID_POST_RUN`). (Safety)
- ✅ Upholds the repo's strict comment policy: Phase 1 Manual Verification
  requires the degradation path to add no explanatory comments. (Documentation,
  Code Quality)
- ✅ The sha2 0.11 → cpufeatures 0.3.0 → libc `getauxval(AT_HWCAP)` chain
  genuinely supports ARMv8 SHA-2 detection under musl (verified against
  `Cargo.lock`; detection is gated on `target_arch`/`target_os`, not libc).
  (Compatibility)

### Recommended Changes

1. **Complete the Phase 1 degradation change surface** (addresses: "Graceful
   degradation still aborts"). Guard `record_provenance`/`tool_provenance` and
   `close_the_budget`/`measure_shell_terms`/`measure_digest_bracket` (and its
   backend-delta check) on `rig.fallback_available`, and add unit coverage that
   drives the full `run_session` degraded path — not just the rig builders.

2. **Fix the calibration match** (addresses: "`chip`/`shasum` are exact-match
   keys"). Parse the CPU brand out of `lscpu -J` ("Model name") in
   `observed_chip` before recording, record `chip` as exactly what
   `observed_chip` returns, and carry `musl` somewhere not exact-match-compared
   (a dedicated `Calibration` field, or a portion the matcher normalises away).
   Drop the "`observed_chip` needs no new code" Key Discovery.

3. **Enumerate the Phase 1 test migration and analyse wiring** (addresses: the
   signature-incompatibility and analyse findings). Either default the new
   parameters (`fallback_available: bool = True` on `assert_backends` and `Rig`)
   or explicitly invert `test_a_host_without_shasum_makes_the_fallback_cells_inapplicable`
   and update the three other `assert_backends` call sites. Make the `analyse`
   signature + call-site change explicit, define `FALLBACK_CELLS = {"C3","C4","C6"}`
   and `FALLBACK_ABSENT_REASON`, and add an `analyse()` test asserting C3/C4/**C6**
   carry the reason (C6 is non-gating, so `closure_verdict` can't catch it).

4. **Harden the Phase 4 commit gate** (addresses: "fewer-cells run passes as
   calibrated"). For the calibrated linux record, assert C1–C4 are branch 1 and
   no fallback cell carries `accepted_by`; bind `accepted_by` to
   `interval is None and not fallback_available`, reserving the branch-7 path for
   the explicitly Perl-less scenario.

5. **Add record hygiene and a Phase 2 revert checkpoint** (addresses: the Phase 2
   record, revert discipline, and checkout-isolation findings). Delete all
   intermediate `warm-dispatch-*.json`/`-samples.json` before the Phase 4 run;
   commit by the exact printed filename, not "latest N". After Phase 2 teardown,
   run `jj diff tasks/measure.py` (expect empty), confirm no untracked records
   remain, and run the lockstep test as the last Phase 2 action. Specify a
   separate guest clone with a verified record-transfer step.

6. **Resolve the `power_state` keying and contract change** (addresses: the
   `probe[0]` and `power_state`-contract findings). Prefer keying on the full argv
   or a stable label (reading all `/sys` files with `cat`); failing that, add a
   test pinning the entry's probes to distinct leading commands. Refresh the
   docstring, and state that the `"unknown"` sentinel is intentionally retired
   with its existing test replaced.

7. **Strengthen the linux farm-completeness coverage** (addresses: "guards less
   than darwin"). Add a linux parallel of
   `test_the_recovered_baseline_spawns_nothing_the_farm_lacks`, and assert the
   linux `path_tools` contains `bash` and `true` explicitly.

8. **Pin the VM environment and its egress** (addresses: the provisioning and
   host-coupling findings). Capture provisioning as a committed Colima profile +
   setup script pinning the base-image digest, apt package versions and the rust
   toolchain to the `mise.toml` pin; add `mise` (and uv/python) plus the full
   egress set (release base URL, crates.io, apt, rustup/mise mirrors); record the
   resolved versions in the record's provenance; note the equivalent native
   mechanism for an arm64 Linux host.

9. **Generalise or explicitly scope the `power_probes`** (addresses: "host-specific
   `/sys` paths"). Enumerate `/sys/class/power_supply/*` and detect cpufreq
   presence, or mark the probes best-effort-per-arch-key in the entry, and record
   the VM-scoped/provisional nature of the constants in the record itself.

10. **Wire the throughput fallback and detection provenance** (addresses: the
    cross-target and HWCAP findings). Configure
    `target.aarch64-unknown-linux-musl.runner` (or make `decompose_terms` assert
    `asset_bytes`/terms present and fail loudly), and record the resolved
    cpufeatures/musl version or a `HWCAP_SHA2` assertion alongside the throughput.
    Consider a tested `sha256_throughput_mb_s` helper.

11. **Pin the ceiling-derivation base** (addresses: "doesn't reproduce darwin's
    C2/C4"). State whether the base is the point estimate or the bootstrap
    interval upper bound, confirm it reproduces darwin's 50/60/70/80 from the
    recorded darwin statistics, and rephrase the manual-verification criterion to
    match.

12. **Update the README operator prose and the verdict destination** (addresses:
    the README-darwin-only, closure-notes, and byte-identical findings). Add
    `linux-arm64` to the "measure namespace" narrative and its run path; record
    the per-cell transfer verdict and derived throughput in a `## Validation
    Results` section of work item 0217; reframe "byte-identical" as numeric
    equality.

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally well-grounded — it leans into the
data-driven `PLATFORM_TABLE` (additive, open-closed), respects the
functional-core / imperative-shell split, and reuses the existing
`closure_verdict` `NOT_APPLICABLE`-with-`accepted_by` seam rather than inventing
control flow. Two structural gaps undercut it: the Phase 1 change surface is
incomplete (`record_provenance`/`tool_provenance` still iterates a fallback farm
directory Phase 1 no longer creates, so the Perl-less path still aborts), and the
calibration mechanism treats `chip`/`shasum` as exact-match keys while the plan
treats them as free-text, so overloading `chip` with `(musl)` defeats the
`calibrated` verdict AC 2 demands. Both are fixable omissions; phase sequencing
is sound.

**Strengths**:
- Adheres to the functional-core / imperative-shell boundary: pure changes in
  `tasks/shared/measurement.py`, process/filesystem changes in `tasks/measure.py`.
- Exploits the data-driven `PLATFORM_TABLE`: a new platform is a new
  `PlatformEntry` + README bullets in lockstep — additive, open-closed.
- Reuses the existing `closure_verdict` contract instead of a parallel degradation
  path.
- Phase sequencing de-risks: the pure, darwin-testable Phase 1 lands before the VM
  chain (which can't be parallelised because calibration depends on measurement).
- Corrected the research's incorrect `entry.key`/cache-name claim, modelling the
  actual `entry_platform()`/`host_platform()` derivation from host uname.
- Preserves the harness's fail-closed posture toward the external release host.

**Findings**:
- 🟡 major (high) — *Graceful-degradation change surface omits
  `record_provenance`/`tool_provenance`, so the Perl-less path still aborts*
  (Phase 1, Change 1). `run_session` calls `record_provenance`
  (`tasks/measure.py:1644`) → `tool_provenance(rig.fallback_farm, ...)` →
  `sorted(farm.iterdir())`; `register_artefact` records the path but never creates
  the directory, and only `build_farm` mkdirs it, so on a Perl-less host `iterdir`
  raises FileNotFoundError. The planned tests only exercise
  `build_rig`/`build_farm`/`assert_backends`/`sample_blocks`, so they miss it. AC
  5 is defeated — the session aborts at provenance-gathering instead of rig setup.
  Fix: mkdir an empty fallback-farm dir even when absent, or make
  `tool_provenance`/`record_provenance` tolerate an absent farm, and add an
  end-to-end `run_session`/`record_provenance` test with the backend faked absent.
- 🟡 major (high) — *`chip`/`shasum` are exact-match keys, not free-text;
  overloading `chip` with `(musl)` defeats the `calibrated` verdict* (Phase 3,
  Change 2; Key Discoveries). `unconfirmed_calibration_fields`
  (`measurement.py:839-869`) compares by exact equality, and `calibration_note`
  reports `calibrated` only when every field matches, so ` (musl)` guarantees
  `recorded != observed`. The "`observed_chip` needs no new code" discovery is
  unsound: `observed_chip` (`measure.py:2490-2507`) returns `splitlines()[0]` of
  `lscpu -J`, i.e. `{`. Fix: parse the model-name field from `lscpu -J`, record
  `chip` as exactly what `observed_chip` returns, and carry `(musl)` in a
  non-match-compared field.
- 🔵 suggestion (medium) — *`FALLBACK_CELLS` constant duplicates which cells
  depend on the fallback backend* (Phase 1, Change 3). C3/C4/C6 are already
  encoded in the arm-to-cell wiring (`measure.py:2096-2105`) and the
  `assert_backends` message; a third hardcoded list can drift. Derive from
  `interval is None and not fallback_available` in the `classify_cell` loop.
- 🔵 minor (low) — *`power_state` contract change is in the shared core and alters
  every platform's recorded shape* (Phase 1, Change 4; Migration Notes). Replacing
  the `unknown` sentinel with `no reading from <argv>` touches the darwin record's
  provenance vocabulary too (inert only because `pmset` always emits output). The
  migration claim understates the blast radius; confirm no consumer keys on
  `"unknown"`.
- 🔵 suggestion (medium) — *`power_state` keys on `probe[0]`, forcing
  artificially-distinct commands* (Phase 3, Change 2). Picking `cat`/`head`/`grep`
  to read three `/sys` paths leaks argv[0]-as-key into platform config; the
  `Linux/x86_64` secondary will hit the same trap. Consider keying on the joined
  argv.

### Code Quality

**Summary**: The plan is largely well-shaped for maintainability: Phase 1's
changes land in the pure analysis core and clearly-seamed driver functions, it
reuses the existing `NOT_APPLICABLE` + `accepted_by` closure mechanism, and it
models fallback-backend availability as an explicit property rather than inferring
it from an empty arm. The most significant concerns are two field/primitive
overloads: appending `(musl)` to the exact-match `chip` field, and working around
`power_state`'s `probe[0]` keying with three artificially distinct leading
commands (including an opaque `grep -h ""`). Both mislead a future maintainer, and
the first quietly defeats the plan's own `calibrated` goal.

**Strengths**:
- Phase 1's changes concentrate in pure or well-seamed functions (`power_state`,
  `classify_cell`, `closure_verdict`), unit-testable in isolation on darwin.
- Modelling `Rig.fallback_available` explicitly, rather than inferring it from an
  empty arm, distinguishes a deliberate skip from a truncated run.
- Reuses the existing `closure_verdict` acceptance rule — DRY, KISS-consistent.
- Sequencing the darwin-testable core hardening before VM work keeps the risky
  measurement off the critical path.

**Findings**:
- 🟡 major (high) — *Libc fact `(musl)` overloads the exact-match `chip`
  calibration field* (Phase 3, Change 2; also Phase 2, Change 2). `chip` is an
  exact-equality key: `observed_calibration` (`measure.py:2538-2571`) compares it
  against `observed_chip` and `unconfirmed_calibration_fields` reports any
  inequality. The recorded chip can never equal the observed chip, so
  `calibration_note` returns "uncalibrated" — defeating the Desired End State. The
  same trap applies to `shasum` (version-matched). Carry libc in a
  non-equality-matched field.
- 🟡 major (medium) — *`power_state` keying on `probe[0]` forces artificial command
  diversity* (Phase 3, Change 2; Phase 1, Change 4). Two probes sharing a leading
  command collapse and silently lose a reading; `grep -h ""` exists only to be a
  distinct key. Anyone adding a fourth probe must invent another unique leader or a
  reading vanishes with no failing test. Key on a stable label / source path / full
  argv, read all `/sys` files with `cat`, or add a distinctness test.
- 🔵 minor (medium) — *`power_state` rewrite drops its docstring and collapses
  three failure modes* (Phase 1, Change 4). Every function in the file has a
  docstring, and the current one still says it records `unknown`. The rewrite folds
  executable-missing / permission-denied / empty-output into one string. Refresh
  the docstring; decide whether the absent-vs-empty distinction is worth keeping.
- 🔵 minor (medium) — *`FALLBACK_CELLS` is a stringly-typed parallel set
  duplicating cell knowledge* (Phase 1, Change 3). `{"C3","C4","C6"}` is used but
  never defined in the plan, and duplicates what `cells_for` already encodes. Model
  fallback-dependence as a `Cell` attribute, or at least define the set.
- 🔵 minor (medium) — *`fallback_available` probe hard-wired to `shutil.which`
  inside `build_rig`* (Phase 1, Change 1). Reading global PATH forces PATH
  manipulation/monkeypatching in tests, at odds with the Protocol-injection style.
  Compute at the composition root and inject (note `build_farm` already calls
  `shutil.which`).
- 🔵 suggestion (low) — *Two divergent mechanisms for including/excluding backend
  tools* (Phase 1, Changes 1–2). The fast backend toggles per-farm via
  `include_fast_backend`; the fallback is excluded by pre-filtering `tools`. Two
  techniques in one path is a small KISS cost — consider one backend policy.

### Test Coverage

**Summary**: The plan is unusually test-literate — it enumerates concrete unit
tests per phase, leans on the pure-analysis-core design, and correctly reuses the
already-tested `closure_verdict` acceptance mechanism. But the net-new integration
logic of Phase 1 — `analyse()` wiring fallback-absence through to the cells'
`accepted_by` — is left untested at the unit level, and the plan doesn't account
for existing `assert_backends` tests that encode the behaviour it removes. The
Phase 3 linux farm-completeness test is narrower than the darwin coverage it
parallels.

**Strengths**:
- The lockstep test is data-driven over `PLATFORM_TABLE` and bidirectional, so it
  covers the six new constants and pins README↔table agreement (catching a leaked
  Phase-2 placeholder that lacks matching bullets).
- Phase 1 decomposes into pure, unit-testable functions and correctly identifies
  that `closure_verdict` + `CellOutcome.accepted_by` already exist and are tested.
- The `power_state` test list is thorough: all three no-reading paths plus the
  successful-reading path.

**Findings**:
- 🟡 major (high) — *`analyse()`'s acceptance-reason attachment — the net-new
  integration logic — is untested* (Phase 1, Change 3; Success Criteria). The
  behaviour delivering AC 5 lives in `analyse()`: thread the rig's
  `fallback_available` through to a `FALLBACK_CELLS` set and set
  `accepted_by=FALLBACK_ABSENT_REASON` on C3/C4/C6. Nothing tests `analyse()`
  itself; a mutation (flag not threaded, C6 omitted, wrong cells) passes every
  listed test yet breaks AC 5, and C6 is non-gating so `closure_verdict` can't
  catch a missing C6 reason. Add a direct `analyse()` test.
- 🟡 major (high) — *Existing `assert_backends` tests encode the removed behaviour
  and the signature change breaks their call sites* (Phase 1, Change 2; Testing
  Strategy). `test_a_host_without_shasum_makes_the_fallback_cells_inapplicable`
  asserts `assert_backends` *raises* — the abort Phase 1 removes — and the new
  required keyword-only param makes existing `assert_backends(fast, fallback)` calls
  TypeErrors. The plan adds a "does not raise" test but never says these must be
  migrated. `test:unit:tasks` can't go green until resolved.
- 🟡 major (medium) — *The linux farm-completeness test guards less than the darwin
  coverage it parallels* (Phase 3, Change 4). The darwin suite has a *second*
  completeness test for the recovered baseline (the B variant:
  `vcs-guard.sh`/`vcs-common.sh`); since `path_tools` is the union of both variants,
  a baseline-only tool absent from linux `path_tools` passes the added test yet
  reintroduces the missing-tool bug. `spawned_executables` also never returns `bash`
  or `true`. Add a baseline parallel and assert `bash`/`true` explicitly.
- 🔵 minor (medium) — *No automated guard that the linux `power_probes` have
  distinct leading commands* (Phase 3, Change 2). A future edit sharing a leading
  command silently drops a reading with no failing test. Assert
  `len({probe[0] for probe in ...power_probes})` equals the probe count.
- 🔵 minor (medium) — *The Phase 1 list restates an existing test while omitting a
  positive fallback-present assertion* (Phase 1 Success Criteria). The
  `closure_verdict` case already exists; only the negative `sample_blocks` direction
  is listed, and nothing asserts `Rig.fallback_available` is computed correctly. A
  regression dropping the fallback arm on a healthy host is only loosely covered.
- 🔵 suggestion (medium) — *Phases are not framed red-first, and a pre-existing
  `power_state` test that must fail is not called out* (Implementation Approach;
  Phase 1). `test_an_absent_probe_yields_unknown_rather_than_propagating` asserts
  `{'pmset': 'unknown'}`, a natural "red" — yet the plan never says it must be
  updated. Name the failing test that precedes each behaviour.
- 🔵 suggestion (low) — *Throughput MB/s is computed by hand rather than as a tested
  pure helper* (Phase 4, Change 2). A by-hand division is the plausible-figure risk
  the harness architecture eliminates elsewhere. A tiny
  `sha256_throughput_mb_s(asset_bytes, median_ms)` with a unit test keeps it in the
  tested core.

### Correctness

**Summary**: The data-driven framing is sound and several correctness-critical
claims check out (`power_state` keys on `probe[0]`; `closure_verdict` needs a
truthy `accepted_by` on a `NOT_APPLICABLE` gating cell; `entry_platform()` derives
the alias from host uname, not `entry.key`; `_absolute`/`_ratio` return `None` on
empty arms). But two high-confidence logic gaps would make the implementation fail
its own acceptance criteria: the Phase 1 change is incomplete (two other paths
still touch the unbuilt fallback farm and abort), and the claim that
`observed_chip` needs no new code is wrong for linux (it returns `{`). A third,
medium-confidence issue: the ceiling-derivation rule doesn't reproduce darwin's
C2/C4 exemplars.

**Strengths**:
- Correctly identifies `power_state` keys on `probe[0]` (`measurement.py:710`) and
  mandates distinct leading commands.
- Correctly reads `closure_verdict` semantics (`measurement.py:591-595`).
- Accurately corrects the research: `entry_platform()`→`host_platform()` derives
  the alias from uname via `UNAME_TO_ALIAS` (`targets.py:38-40`).
- The empty-fallback-arm path is safe by construction: `_absolute`/`_ratio` return
  `None` (`measure.py:2340, 2365`), funnelling into `classify_cell` as
  `NOT_APPLICABLE`.
- The Phase 1 `power_state` rewrite is correct Python 3 and covers all three
  no-reading cases; the throughput formula is dimensionally correct decimal MB/s.

**Findings**:
- 🔴 critical (high) — *Graceful degradation still aborts: `record_provenance` and
  `close_the_budget` touch the unbuilt fallback farm* (Phase 1, Changes 1–3).
  `record_provenance` (`measure.py:1644`) → `tool_provenance` →
  `farm.iterdir()` (`measure.py:2807`) raises FileNotFoundError since the fallback
  dir is never created; later `close_the_budget` (`measure.py:1697`) →
  `measure_shell_terms` → `measure_digest_bracket(farm=rig.fallback_farm,
  backend=FALLBACK_BACKEND)` (`measure.py:2004-2011`) runs `shasum` through the
  missing farm and raises PreconditionFailureError (`measure.py:2742`). A Perl-less
  host still aborts before `analyse`, so AC 5 is unmet, and the planned tests miss
  it. Extend Phase 1 to guard both paths and add full `run_session` degraded-path
  coverage.
- 🔴 critical (high) — *`observed_chip` returns `{` on linux, so the calibrated
  note can never hold* (Key Discoveries; Phase 3, Change 2). `observed_chip`
  (`measure.py:2490-2507`) runs `sysctl -n machdep.cpu.brand_string` (fails silently
  on linux), then `lscpu -J`, returning `observed.splitlines()[0]` = `{`; it never
  extracts the brand field. The recorded chip can never equal `{`, so
  `calibration_note` returns "uncalibrated" (`measure.py:2575-2584`). AC 2, the
  Desired End State, and Phase 4's `note == "calibrated"` are unsatisfiable.
  Recording `chip="{"` to force a match would also violate AC 2's `musl`
  requirement. Parse the brand out of `lscpu -J`/`lscpu` "Model name".
- 🟡 major (medium) — *Ceiling-derivation rule does not reproduce darwin's C2/C4
  exemplars* (Phase 3, Change 1; Manual Verification). Over C2's p90 of 38.230, 50
  leaves 30.8% headroom, so the rule yields 50 not darwin's 60; C4 over 55.291
  yields 70 not 80. Only C1/C3 match. Darwin's C2/C4 reproduce only if the base is
  the interval upper bound, not the point estimate. Applying the rule literally over
  point estimates would produce ceilings tighter than darwin's methodology. Pin the
  derivation base and confirm it reproduces 50/60/70/80.
- 🔵 minor (medium) — *`analyse`'s `accepted_by` wiring omits the required
  signature and call-site change* (Phase 1, Change 3). `analyse(entry, samples, *,
  floors, elapsed)` (`measure.py:2076-2082`) and its call site
  (`measure.py:1700-1705`) pass no flag; `FALLBACK_CELLS` membership (exactly
  `{C3,C4,C6}`) is unstated. Following the snippet literally, `analyse` can't
  compute `accepted_by`. Thread `rig.fallback_available` in and define the
  constants.
- 🔵 suggestion (low) — *Integer-vs-decimal rendering is not a lockstep-parser
  requirement* (Phase 3, Change 3). The regex + `float()`/`approx` accepts `50` and
  `50.0` alike. Keep integer rendering for consistency, drop the "to match the
  lockstep parser" justification.

### Portability

**Summary**: The plan is portability-conscious in the right places — it insists on
genuinely native aarch64 execution, measures the actually-shipped musl libc,
carries libc/SHA-extension facts in free-text provenance under an
architecture-only key, and adds a graceful-degradation path. The main risks are
that the measurement environment itself is not reproducible (prose-only, unpinned
Colima/base-image/OS-package provisioning coupled to a macOS + Apple-Silicon
host), that a single VM's constants and hardcoded `/sys` power-probe paths are
baked into an architecture-only key applied to all `linux-arm64` hosts (including
0219's future consumers), and that the cross-target throughput fallback is not
wired into the driver so a "built but not run" cargo outcome can silently drop the
AC-6 figure.

**Strengths**:
- Requires genuinely native (non-emulated) aarch64 execution and measures
  `aarch64-unknown-linux-musl` — the libc actually shipped per `targets.py`.
- Records libc identity and SHA-extension facts in free-text `chip`/`shasum` under
  the existing architecture-only key, avoiding a re-keying (a documented conscious
  choice).
- Phase 1's graceful degradation genuinely improves cross-distro portability
  (Perl `shasum` is not universal).
- Host-derived environment capture is portable (system/machine/release/platform,
  `lscpu` chip, cgroup-quota CPU count; alias from uname independent of `entry.key`).
- The new linux farm-completeness test pins the linux `path_tools` set to the
  actual spawns rather than transcribing it.
- The VM-scoped nature of the absolute ceilings is explicitly acknowledged as
  provisional and deferred to 0219.

**Findings**:
- 🟡 major (high) — *VM provisioning is prose-only and unpinned, so the environment
  cannot be reproduced* (Phase 2, Change 1). No declarative spec/script; base OS
  loosely "Debian/Ubuntu"; Colima version, kernel and apt packages unpinned; rust
  toolchain source ambiguous (`rustup` vs `mise run deps:install:rust-targets`). The
  one environment producing the constants can't be faithfully reconstructed. Capture
  a committed Colima profile + setup script pinning image digest / package versions
  / toolchain, and record resolved versions in provenance.
- 🟡 major (high) — *Shared `linux-arm64` `power_probes` bake in host-specific
  `/sys` paths that won't generalise* (Phase 3, Change 2). `AC/online`,
  `BAT0/status`, `cpu0/cpufreq/scaling_governor` are per-host; the entry is keyed
  architecture-only and consumed by 0219 for every `linux-arm64` host, yet on the VM
  all three probes will most likely return "no reading". Enumerate
  `/sys/class/power_supply/*` and detect cpufreq, or scope the probes best-effort.
- 🟡 major (medium) — *Cross-target execution fallback is not wired into
  `decompose_terms`; a built-but-not-run cargo outcome silently drops the
  throughput* (Risks; Phase 4, Change 2). `decompose_terms` (`measure.py:2637`)
  shells `cargo test --test warm_terms` and scrapes stdout; if cargo builds but
  skips execution, the return code is 0 while the parse yields empty, so AC 6 is
  silently unmet. Configure `target.…musl.runner` or assert `asset_bytes`/terms
  present and fail loudly.
- 🔵 minor (high) — *Reproduction is coupled to macOS + Apple-Silicon via Colima
  with no portable alternative* (Phase 2, Change 1; Assumptions). The only
  documented native-aarch64 mechanism requires macOS on Apple Silicon; there's no
  path on the project's Linux CI or an arm64 Linux box. Make the coupling explicit
  and note the equivalent native mechanism for an arm64 Linux host.
- 🔵 minor (high) — *VM-scoped ceilings/floor gates are committed under an
  architecture-only key applied to all `linux-arm64` hosts* (What We're NOT Doing;
  Performance Considerations). The constants gate CI arm64, bare metal and other VMs
  alike. Mark them VM-scoped/provisional in the record and provenance, not just plan
  prose.
- 🔵 minor (medium) — *Provisioning checklist omits `mise` itself and
  under-documents network egress* (Phase 2, Changes 1 & 3). Every command is
  `mise run …` yet `mise` (and uv/python) isn't listed; egress is documented only
  to the release base URL, though the build needs crates.io and provisioning needs
  apt + rustup/mise mirrors. A "fresh environment with only the documented
  prerequisites" would fail.
- 🔵 minor (high) — *Reproduction knowledge for the linux target is not captured in
  durable operator docs* (Phase 3, Change 3). Phase 3 updates only the criterion
  bullets; the "What a run requires" section still says "A quiet darwin-arm64 host",
  and the linux how-to lives only in this plan. Extend the measure-namespace
  prerequisites to document the linux/aarch64 path.

### Compatibility

**Summary**: The change is fundamentally additive and the darwin *runtime* path is
a verified no-op when both digest backends are present. Serialisation is
forward-compatible — `accepted_by` already exists on `CellOutcome` and is emitted
as `null`, the new `("Linux","aarch64")` key slots into the raw keying that
`UNAME_TO_ALIAS` already maps, and no Rust/frontend/schema consumer of the records
exists in-repo. The main gaps are internal: the proposed signatures are
*source*-incompatible with existing callers/tests, and the `power_state` value
semantics shift. The sha2 0.11 → cpufeatures 0.3.0 → libc `getauxval` chain does
support ARMv8 SHA-2 detection under musl.

**Strengths**:
- The serialised record schema is not broken: `CellOutcome` already declares
  `accepted_by: str | None = None` and `run_session` already emits it via `asdict`;
  no in-repo consumer reads `power_state`/`accepted_by`/the records.
- The new key is consistent with the raw `(system, machine)` keying;
  `UNAME_TO_ALIAS` already maps `('linux','aarch64')` → `linux-arm64`; no existing
  test pins `PLATFORM_TABLE` to darwin-only (the superset check uses `>=`).
- The README↔`criterion_constants` lockstep is auto-derived and parses `50`/`50.0`
  alike; the entry, bullets and linux farm test land atomically.
- The dependency claim is sound: `Cargo.lock` resolves sha2 0.11.0 → cpufeatures
  0.3.0 → libc, and aarch64 detection uses `getauxval(AT_HWCAP)` gated on
  `target_arch`/`target_os`, not libc.

**Findings**:
- 🟡 major (high) — *Proposed signatures are source-incompatible with existing
  callers/tests* (Phase 1, Changes 1–2; Migration Notes). `assert_backends` gains a
  required keyword-only `fallback_available` (no default in the snippet), so the
  four `assert_backends(fast, fallback)` call sites
  (`test_measure.py:1595,1604,1611,1623`) become TypeErrors, and
  `test_a_host_without_shasum_makes_the_fallback_cells_inapplicable` (1613-1623)
  asserts the *old* abort behaviour. `Rig` likewise gains a non-defaulted field.
  "Migration Notes: None" hides this. Either default the new params or enumerate the
  test inversion + call-site updates.
- 🔵 minor (medium) — *`power_state` rewrite changes the serialised value semantics
  of `quietness.power`* (Phase 1, Change 4; Desired End State). Absent/denied
  previously recorded `"unknown"` and empty-output `""`; the new code records
  `"no reading from <argv>"` for all three. On darwin's happy path values are
  unchanged, so it's *not* a strict no-op, and
  `test_an_absent_probe_yields_unknown_rather_than_propagating` (967-973) will fail.
  State that the sentinel is intentionally retired and the test replaced.
- 🔵 minor (medium) — *`classify_cell`/`analyse` `accepted_by` wiring needs an
  `analyse` signature change* (Phase 1, Change 3). Adding `accepted_by=None` to
  `classify_cell` is backward-compatible, but the snippet references
  `fallback_available` inside `analyse`, whose signature/call site carry no such
  flag. If threaded incorrectly (defaulting False), darwin's C3/C4/C6 would gain a
  spurious reason. Make the signature change explicit and assert `accepted_by is
  None` on darwin.
- 🔵 suggestion (high) — *Add a resolved-version / HWCAP provenance check for the
  sha2 path* (Key Discoveries; Phase 4, Step 2). Detection engages on both glibc
  and musl provided the guest's musl provides `getauxval` (musl ≥ 1.1.21) and the VM
  passes HWCAP through. If it silently fell back, AC 6 fails as a measurement
  outcome — hard to attribute. Record the cpufeatures/musl version or assert
  `HWCAP_SHA2` on the guest.

### Safety

**Summary**: The harness itself is unusually safety-conscious — a manifest is
written before any artefact is created, signals unwind into the with-block,
restore/verify runs on every exit, and any teardown failure forces
`closure_verdict=False` and `INVALID_POST_RUN`. The plan's main gaps are around the
deliberately un-gated Phase 2 pass: it writes a numbered, valid-looking,
closure-true record built on loose placeholder gates with no deletion step and no
automated backstop, and the graceful-degradation path lets a fewer-cells run pass
every automated commit check while silently under-calibrating C3/C4. The
placeholder *code* entry is reasonably contained (shared dict key + CI lockstep
test), but the record artefact and the degradation-masking risk are not.

**Strengths**:
- The `MeasurementSession` is a well-built protective mechanism:
  manifest-before-creation, signal unwinding (incl. SIGHUP), idempotent restore,
  containment-checked removal, aggregate verify, plus `measure:teardown` to replay a
  stale manifest. The plan correctly invokes teardown in Phases 2 and 4.
- The pipeline fails safe on integrity: `capture()` refuses a dirty diff over
  guarded paths and a substituted release key, and any exit-time verify failure
  forces `closure_verdict=False`/`INVALID_POST_RUN`/non-zero exit.
- Records are numbered and never clobbered; the samples sidecar is always written,
  so an interrupted attempt is recoverable.
- Phase 1 lands independently on darwin and pins darwin's behaviour unchanged.
- The degradation change is narrowly scoped to the single fallback backend; any
  other missing tool still makes `build_farm` raise.

**Findings**:
- 🟡 major (high) — *Phase 2 loose-gate record is a committable-looking artefact
  with no deletion step* (Phase 2, Change 3; Phase 4, Change 4). Phase 2 runs
  `measure:warm-dispatch` (not `--rehearse`) against 10000 ms / 1000 ms placeholders,
  writing a full numbered record reporting `valid` + `closure_verdict: true` (and
  `calibrated` if the placeholder `Calibration` is real). `teardown` removes only the
  manifest/temp; no test validates records; Phase 4 says only "commit the calibrated
  N". A `jj commit`/`git add meta/measurements/` can sweep in the placeholder-gated
  record, and 0219 consumes it. Add an explicit deletion step; commit by the exact
  printed filename.
- 🟡 major (high) — *Graceful degradation lets a fewer-cells run pass the commit
  gate as fully calibrated* (Phase 1, Change 3; Phase 4 Success Criteria). The gate
  checks only `validity`, `closure_verdict`, `calibration.note`, all green even when
  C3/C4 degraded (e.g. `shasum` disappears between Phase 2 and 4). The finalised
  entry still carries fallback ceilings, so the record shows those cells branch-7
  accepted while the entry claims to gate them — indistinguishable from a full
  calibration. Assert C1–C4 branch 1 and no fallback cell carries `accepted_by`;
  bind `accepted_by` to the cell lacking an interval, not to `fallback_available`
  alone.
- 🟡 major (medium) — *Revert discipline is manual with the safety gate deliberately
  disabled* (Phase 2, Change 2; Risks). Phase 2 skips `check`/`test` and relies on
  "revert before Phase 3". The code entry is contained, but the record artefact has
  no backstop, and a mid-workflow commit can carry either into history before CI.
  Add an explicit revert checkpoint: after teardown, `jj diff tasks/measure.py`
  (expect empty), confirm no untracked records, run the lockstep test as the last
  Phase 2 action.
- 🔵 minor (medium) — *Guest/host checkout isolation and record-transfer path are
  unspecified* (Phase 2, Change 1; Phase 4, Change 4). If the host tree is mounted,
  placeholder edits and records land in the host working copy; if a separate clone,
  the record must be transferred back with no defined step. Specify a separate clone
  with a verified transfer (confirm `samples_path` matches the transferred sidecar).

### Documentation

**Summary**: The plan is unusually well-documented for an operator-run task: it
names exact record fields, ties the six criterion-constant bullets to their
lockstep test plus a new linux farm-completeness test, documents the calibration
provenance, and explicitly forbids explanatory comments in line with the repo's
policy. Two accuracy/currency gaps remain: it mischaracterises what the lockstep
test enforces ("byte-identical" rather than numeric equality after float
coercion), and it never updates the README's darwin-centric "measure namespace"
prose to reflect that `linux-arm64` becomes a calibrated platform. It also records
the transfer verdict in a "closure notes" section that no work item actually uses.

**Strengths**:
- Actively upholds the strict comment policy: Phase 1 Manual Verification requires
  "no explanatory comments"; rationale rides in plan prose and a test, not code
  comments.
- Calibration provenance documented precisely and consistently with AC 2 and the
  2026-09-15 resolution (all four fields, `musl` in the free-text `chip` field).
- The throughput derivation is reproducibly documented (exact record inputs +
  formula matching the README's derivation block).
- The uncommitted-placeholder workflow and its mandatory revert are clearly
  documented and reinforced by a "Placeholder leakage" risk entry.
- The three lockstep-coupled artefacts land together as one atomic Phase 3 change,
  preventing doc/code drift.

**Findings**:
- 🟡 major (medium) — *README "measure namespace" operator prose stays darwin-only
  after linux becomes a calibrated platform* (Phase 3, Change 3; Desired End State).
  The plan updates only the `### Criterion constants` bullets, leaving "A quiet
  darwin-arm64 host" (README:261) as the run guide. After this item `linux-arm64` is
  a calibrated, supported platform, but a reader (including 0219) can't learn that
  from the README. Decide explicitly whether to add a measure-namespace line; if
  scoped out, say why.
- 🔵 minor (high) — *"Byte-identical" mischaracterises what the lockstep test
  enforces* (Phase 3, Change 3; Manual Verification). The test parses each bullet by
  regex, coerces via `float()`, and compares with `approx` (test_measure.py:1155-1179)
  — numeric equality, not byte identity (darwin stores `50.0`, README reads `50`).
  The inaccurate rationale could send the operator chasing an impossible byte match.
  Reframe as numeric equality; keep integer rendering as a style choice.
- 🔵 minor (medium) — *Transfer-verdict destination "closure notes" is not a
  convention any work item uses* (Phase 4, Change 3). No work item has a "Closure
  Notes" section; the established pattern for measured figures + verdict is
  `## Validation Results` (what AC 4 compares against in 0189). The derived throughput
  also has no stated recording location. Direct the operator to `## Validation
  Results` in 0217.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-20T22:44:52+00:00

**Verdict:** REVISE

Re-ran all eight lenses against the revised plan. **Both pass-1 criticals are
resolved** — Correctness verified that every `run_session` consumer of the
fallback farm is now guarded (so a Perl-less host no longer aborts), and that the
`observed_chip` brand-parse plus the non-matched `Calibration.libc` field
correctly let the `calibrated` note hold. Most pass-1 majors are resolved or
improved. However, the pass-1 remediation **introduced a new critical** (the
ceiling-derivation base still does not reproduce darwin's ceilings) and a cluster
of majors where fixes were written as prose intent rather than scheduled as
concrete code changes with tests. Verdict remains REVISE.

### Previously Identified Issues

- 🔴 **Correctness/Architecture**: Perl-less path still aborts — **Resolved**
  (all fallback-farm consumers guarded; verified against the code).
- 🔴 **Correctness/Architecture/Code Quality**: `calibrated` note unsatisfiable
  (`chip`/`observed_chip`) — **Resolved** (`libc` field + brand parse; now cited
  as a strength).
- 🟡 **Compatibility/Test Coverage**: signatures break existing tests —
  **Resolved** (defaults + complete migration accounting; now a strength).
- 🟡 **Test Coverage/Correctness/Compatibility**: `analyse()` untested /
  under-specified — **Resolved** (targeted `analyse` test incl. non-gating C6;
  wiring made explicit).
- 🟡 **Safety**: Phase 4 gate fails open — **Resolved** (C1–C4 branch 1 + no
  `accepted_by` assertion).
- 🟡 **Safety**: Phase 2 record has no deletion step — **Resolved**, but the
  deletion step it added introduced a data-loss glob (see New Issues).
- 🟡 **Safety**: revert discipline manual — **Resolved** (checkpoint now a
  strength).
- 🟡 **Code Quality/Architecture/Test Coverage**: `power_state` `probe[0]` keying
  — **Partially resolved (accepted tradeoff)** — distinctness guard added per the
  keep-and-guard decision; the core keying is deliberately unchanged, so
  Architecture and Code Quality still flag it as major.
- 🟡 **Test Coverage**: linux farm test guards less — **Resolved** (baseline
  parallel + `bash`/`true` assertion; now a strength).
- 🟡 **Portability**: VM provisioning unpinned — **Partially resolved** —
  inventory now thorough (strength), but the "record versions in provenance" fix
  is prose, not a scheduled code change.
- 🟡 **Portability**: `power_probes` host-specific `/sys` — **Partially resolved**
  — scoped best-effort in prose; lenses still prefer enumeration or an explicit
  calibrating-host caveat.
- 🟡 **Portability**: cross-target throughput not wired — **Partially resolved** —
  runner config added, but the fail-loud guard is claimed, not scheduled.
- 🟡 **Correctness**: ceiling rule ≠ darwin C2/C4 — **Not resolved (regressed)** —
  the pinned bootstrap-interval-upper-bound base also yields 50/50/70/70.
- 🟡 **Documentation**: README stays darwin-only — **Partially resolved** —
  Change 5 added; needs the VM-scoped caveat and a verification step.

### New Issues Introduced

- 🔴 **Correctness**: The pinned ceiling-derivation base (bootstrap interval upper
  bound) does not reproduce darwin's 50/60/70/80 — over 0189's recorded intervals
  it gives 50/50/70/70, identical to the point base it discards. (Root cause, per
  0189's cell table: darwin's ceilings were pre-registered from 0205's higher base
  figures — C2 from 46.51 ms at +29%, C4 from ~63.4 ms at +26% — so reproducing
  them from 0189's validation figures is impossible and is not a valid
  confirmation step.)
- 🟡 **Safety**: The cleanup glob `warm-dispatch-*.json` also matches the four
  already-committed darwin records (`warm-dispatch-1..4.json`); a literal deletion
  destroys committed evidence, and the Phase 2 checkpoint (`jj diff` on
  `measure.py` + no untracked records) cannot detect a tracked-file deletion.
- 🟡 **Safety**: Delete-then-commit ordering — the same glob also matches the
  just-produced final record, which the final run already wrote before the
  deletion.
- 🟡 **Correctness/Portability/Test Coverage**: The "`decompose_terms` fails
  loudly if `asset_bytes`/`verifier::sha256_hex` is missing" guard is described as
  existing but is not scheduled as a code change, and an existing test
  (`test_asset_bytes_is_none_when_the_line_is_absent`) asserts the None-not-raise
  behaviour.
- 🟡 **Portability**: "Record the resolved versions in the record's provenance" is
  stated but not scheduled as a code change (`record_provenance` is unextended).
- 🟡 **Test Coverage**: The "full `run_session` degraded path" is listed as a unit
  test but is infeasible (real `jj`/`cargo`/network); should be targeted unit
  tests on `record_provenance` and `measure_shell_terms`/`close_the_budget`
  through the injected runner.
- 🟡 **Documentation**: The new README line omits the VM-scoped/provisional
  caveat (the constants block cannot carry it), with no success criterion
  verifying the prose landed.
- 🔵 **Compatibility**: New darwin records additively gain `recorded.libc: null`,
  so Migration Notes' "strict no-op" is imprecise (gating/behaviour is unchanged).
- 🔵 **Correctness/Test Coverage**: The `accepted_by` binding assumes the fast arm
  is always populated; a Perl-less host with an empty fast arm (non-standard
  `--blocks`) could tag C1/C2/C5 and close on an empty session.
- 🟡 **Architecture** (pre-existing, not introduced): calibrating a new platform
  requires mutating and reverting production `PLATFORM_TABLE` — a recurring
  bootstrap footgun; suggested as a follow-up (a first-class uncalibrated
  full-sample mode), out of this item's "not re-authoring the harness" scope.

### Assessment

The core of the plan is now sound: the two blocking defects are genuinely fixed
and the darwin no-op is verified. The plan is not yet ready — the ceiling
critical must be corrected, and the throughput-guard, provenance-recording, and
degraded-path-test fixes must be scheduled as concrete Changes-Required code
items (with tests) rather than asserted as existing behaviour, and the cleanup
glob must be bounded so it cannot delete committed records. These are addressed in
a second remediation pass following this re-review.

## Re-Review (Pass 3) — 2026-09-21T07:01:11+00:00

**Verdict:** REVISE

Re-ran the six lenses the second remediation touched (Correctness, Safety,
Portability, Test Coverage, Documentation, Compatibility). **No criticals** — the
pass-2 ceiling-derivation critical is resolved and verified: Correctness confirmed
that ≥18% headroom over 0189's validation figures yields 50/50/70/70 and correctly
diverges from darwin's 0205-derived 50/60/70/80. Documentation is effectively
clean, and Safety verified the data-loss glob is closed. However, the pass-2
remediation's newly-added Changes 7 and 8 (throughput fail-loud guard, provenance
build block) and their test/prose scaffolding introduced a fresh set of majors —
all internal-consistency gaps in that added material, not defects in the core
plan. Verdict remains REVISE.

### Previously Identified Issues (pass 2 → pass 3)

- 🔴 **Correctness**: ceiling base doesn't reproduce darwin — **Resolved &
  verified** (≥18% rule applied to linux figures directly; darwin attributed to
  0205's base).
- 🟡 **Safety**: cleanup glob deletes committed darwin records — **Resolved &
  verified** (exact-filename deletion; records numbered 5+ can't match 1–4).
- 🟡 **Safety**: delete-then-commit ordering — **Resolved** (capture final
  filename first).
- 🟡 **Correctness/Portability/Test Coverage**: throughput guard didn't exist —
  **Resolved as intent** (scheduled as Change 7), but its placement introduced a
  new test-breakage major (below).
- 🟡 **Portability**: provenance recording not scheduled — **Resolved as intent**
  (scheduled as Change 8), but introduced a provenance-home contradiction (below).
- 🟡 **Test Coverage**: `run_session` degraded test infeasible — **Resolved**
  (retargeted to `measure_shell_terms`/`record_provenance`), though feasibility
  nits remain (below).
- 🟡 **Documentation**: README VM-scoped caveat — **Resolved** (caveat + prose
  checkbox added).
- 🔵 **Compatibility**: Migration Notes "strict no-op" — **Partially resolved**
  (reworded, but still incomplete — below).

### New Issues Introduced (by the pass-2 remediation)

- 🟡 **Correctness/Compatibility/Test Coverage** (converged, high): Change 7's
  guard in `assemble_terms_report` breaks three existing `TestTermsReport` tests
  (`test_an_absent_asset_size_is_persisted_as_none` and the two carrier tests,
  which omit `verifier::sha256_hex`) not listed for migration — so the Phase 1
  "unit suite passes" criterion cannot hold. Fix all three lenses endorse: site
  the guard at `decompose_terms`/`close_the_budget` (where "built-but-not-run"
  originates), leaving the pure assembler and its tests intact.
- 🟡 **Correctness** (high): Change 3's prose promises the acceptance reason is
  applied "only when the fast/baseline arm is populated", but the code snippet
  (`interval is None and not fallback_available`) omits that clause — so a
  `--blocks=B` run on a Perl-less host could mark C1/C2/C5 not-applicable and let
  `closure_verdict` pass on an empty session.
- 🟡 **Test Coverage** (high): `build_rig` is listed as a unit test but drives
  real `jj` through hardcoded seams; scope the unit tests to `build_farm` +
  `assert_backends` and extract the `shutil.which` decision into a testable
  helper, or state build_rig is covered only by the manual Perl-less run.
- 🟡 **Portability** (high): Phase 2 Change 1 ("record Colima/apt/mise versions in
  the record's provenance") contradicts Phase 1 Change 8 ("VM identity → work
  item; build block records only rust triple/cargo/rustc/linker"); the apt/mise
  versions have no code change behind them.
- 🟡 **Portability** (medium): nothing cross-checks the recorded build triple
  against the entry's `libc="musl"`, so a forgotten `CARGO_BUILD_TARGET` could
  persist a glibc figure labelled musl.
- 🟡 **Safety** (medium): the `jj status meta/measurements/` checkpoint added to
  Phase 2 was not mirrored into Phase 4 — the higher-stakes commit moment.
- 🟡 **Compatibility** (medium): Migration Notes enumerate two serialisation
  deltas but omit Change 8's new `provenance.build` block, which also lands in
  darwin records; the "None affecting behaviour" opening also understates Change
  7's silent-drop → loud-failure change.
- 🔵 Minor/suggestion: co-locate the VM-scoped caveat with the constants block
  (Documentation); `record_provenance`/`close_the_budget` unit-test feasibility
  nits (Test Coverage); runner-config location + distro/host coupling not
  surfaced (Portability).

### Assessment

The core plan is sound and its two original criticals are resolved and verified
against the code. Every pass-3 major is an internal-consistency gap in the
scaffolding the pass-2 remediation added (Changes 7/8, the accepted_by prose, the
Phase 2/4 checkpoint symmetry, the Migration Notes), not a defect in the
measurement work itself — a sign the plan is being specified at an
implementation granularity it cannot sustain without direct code verification, and
that Changes 7/8 edge past the "not re-authoring the harness" scope. Recommended
close: apply one final, tight remediation for the genuine inconsistencies (move
the Change 7 guard off the pure assembler; put the fast-arm clause in the Change 3
snippet; reconcile the provenance homes; mirror the Phase 4 checkpoint; complete
the Migration Notes; add the triple cross-check) and simplify the over-specified
unit-test enumerations to intent, letting red-green-refactor find the feasible
seams during implementation — then stop the multi-pass loop.

## Disposition — 2026-09-21T21:55:44+00:00

**Verdict overridden to APPROVE by the reviewer (Toby Clemson).** The pass-3
agents' suggested verdict was REVISE. Both original criticals are resolved and
verified against the code, and the final remediation addressed every pass-3 major.
The reviewer accepts the remaining items — the exact code seams for the Change 7
throughput check, the `fallback_backend_available()` helper, and the Change 8
provenance shape — as **implementation-level**, to be settled test-first during
`/implement-plan` rather than by a further review pass. Plan status set to `ready`.
