---
type: plan-review
id: "2026-08-05-0169-vcs-subdomain-and-hooks-migration-review-1"
title: "Plan Review: VCS Subdomain and Hooks Migration Implementation Plan"
date: "2026-08-05T16:27:45+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
parent: ""
target: "plan:2026-08-05-0169-vcs-subdomain-and-hooks-migration"
relates_to: []
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, security, safety, compatibility, performance]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-08-05T18:56:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: VCS Subdomain and Hooks Migration Implementation Plan

**Verdict:** REVISE

This is an unusually rigorous plan for a shell-to-Rust migration: it commits to golden-fixture parity before any deletion, closes a real launcher-level fail-safe gap the research pass missed, and applies hexagonal boundaries with cargo-pup enforcement rather than convention alone. But three independent lenses converged, from different angles, on the same conclusion — the plan is not yet internally consistent enough to implement as written. A `vcs detect` success criterion in Phase 5 contradicts another success criterion in the same phase for the identical scenario; the plan defers the one empirical check the work item's own Sequencing Constraint 1 says must gate *planning* to Phase 10, after three phases of shape-dependent code are already merged; and the plan's own stated performance gate is arithmetically incompatible with a cost the work item's own hand-off notes already measured. None of these are found in code the plan hasn't thought about — they are self-contradictions or unresolved tensions within the plan's own text.

### Cross-Cutting Themes

- **Phase 5's fail-safe fix is under-specified across five independent dimensions** (flagged by: code-quality, safety, security, test-coverage, correctness) — the mechanism that swallows a failed external-dispatch resolution into `ExitCode::SUCCESS` under `--fail-safe` needs, per these five lenses: a diagnostic trail so the failure isn't invisible to both logs and manual runs (code-quality, security); a way to distinguish binary-integrity/tamper failures from ordinary unavailability, since today's error mapping cannot (safety, security — this is the most severe instance, since `SignatureMismatch`/`ChecksumMismatch` currently map to the same `Failed` variant as a DNS timeout); extraction into a pure, injectable function so the promised unit test is actually writable, since `main()` has no test seam today (test-coverage); and confirmation of which error variants can actually reach the branch, since `kernel::Error::Refusal` appears unreachable on this path as specified (correctness). Five lenses landing on one ~15-line code change is a strong signal this phase needs a redesign pass, not five separate patches.
- **The new `vcs status`/`vcs log` subprocess calls (Phase 6) don't state reuse of safety machinery that already exists in the same file** (flagged by: architecture, performance, security) — the existing `CommandProbe::revision` in `cli/vcs-adapters/src/subprocess.rs` already has a 10-second timeout-and-kill cap (`capped_stdout`) and environment scrubbing (`scrub_environment`) for exactly the risk class (lock-contended or config-redirected repositories) the new `status()`/`log()` functions face, but Phase 6's description doesn't say whether they reuse either.
- **The release-ordering safety net (Sequencing Constraint 4 / Phase 9) relies entirely on human process** (flagged by: safety, compatibility) — no automated check anywhere in the ten phases verifies the *published* manifest lists `accelerator-vcs` before `hooks.json`'s rewrite can reach `main`, despite this being the widest-blast-radius failure mode in the plan (every installed plugin's SessionStart and PreToolUse hooks).

### Tradeoff Analysis

- **Byte-parity vs. a silent third departure** (Correctness, critical): Phase 5 requires `vcs detect` to both byte-match the existing goldens (which contain ~700 bytes of always-present "VCS Command Reference" text) and to write zero bytes for the identical "main checkout, no boundary" scenario. These cannot both be true. This isn't a values tradeoff between two lenses — it's an internal contradiction that must be resolved by deciding which contract governs before implementation starts.
- **Verifying the hook-schema floor early vs. building the envelope early** (Compatibility, critical): the work item's Sequencing Constraint 1 says the floor check is a "minutes-long empirical check" that "gates planning," precisely because building three phases of `permissionDecision`/bare-`systemMessage` code on an unverified assumption is expensive to unwind if the floor doesn't support it. The plan currently sequences the check last (Phase 10) rather than first, inverting the work item's own stated ordering with no discussion of why.

### Findings

#### Critical

- 🔴 **Correctness**: "Zero bytes for a main checkout" contradicts the byte-parity requirement against the same scenario's existing golden
  **Location**: Phase 5: Launcher Fail-Safe for External Dispatch, and `vcs detect` — Success Criteria
  Phase 5's success criteria require `vcs detect` to both byte-match the existing goldens (which contain ~700 bytes of always-emitted "VCS Command Reference" text per `hooks/vcs-detect.sh:40-82`) and to write zero bytes for the identical "main checkout, no boundary" scenario. Phase 5's own Changes Required only describes reproducing the boundary block, never the CONTEXT text the goldens actually contain — implementing either reading fails the other's test.

- 🔴 **Compatibility**: Hook-schema protocol compliance is verified after the shape-dependent code is built and merged, not before
  **Location**: Phase 10 (cross-referencing Phases 4, 5, 7)
  The work item's Sequencing Constraint 1 states the `permissionDecision`/bare-`systemMessage` floor check "gates planning" and must happen first. The plan instead builds and merges `kernel::hooks`, `vcs detect`'s envelope wiring, and the entire `vcs guard` deny/warn implementation (137-row decision table included) across Phases 4/5/7, deferring the floor check to Phase 10's manual verification — the very last phase. The plan also never designs the exit-2 fallback the work item's acceptance criteria require it to describe if the floor check fails.

- 🔴 **Performance**: `cache_root::resolve`'s write-chmod-exec probe runs on every guard dispatch, threatening the plan's own `G ≤ 1.1 × B` gate
  **Location**: Performance Considerations section / Phase 10
  The work item's own hand-off note measures this probe at ~132ms per external-subcommand dispatch (vs. a 3.72ms warm re-exec), against a gate requiring G ≤ ≈38.6ms. The plan names this cost and defers it to 0189 but doesn't reconcile it with Phase 10's own acceptance criterion, risking either a falsely-passing measurement (via the dev override, which bypasses the probe) or an outright failing gate on the real dispatch path.

#### Major

- 🟡 **Architecture**: Status/log subprocess exec drops the existing timeout-and-kill safeguard
  **Location**: Phase 6: `vcs status` and `vcs log`
  The existing `CommandProbe::revision` in the same file is bounded by a 10-second cap-and-kill (`capped_stdout`) for lock-contended repositories; Phase 6's new `status()`/`log()` functions give no indication they reuse it, risking an indefinite hang in `skills/vcs/commit`'s synchronous context injection.

- 🟡 **Architecture**: Guard's command-parsing and decision logic bypasses the domain crate the plan otherwise uses
  **Location**: Phase 7: `vcs guard`
  Phase 3 places `classify_checkout`'s pure logic in the `vcs` domain crate under cargo-pup protection; Phase 7 places the guard's equally-pure compound-splitting/blocklist logic directly in the `vcs-cli` binary crate instead, with no equivalent domain module — an inconsistent application of the plan's own functional-core pattern.

- 🟡 **Code Quality**: Fail-safe suppression swallows the underlying error with no diagnostic trail
  **Location**: Phase 5
  When `run(&cli)` fails under `--fail-safe`, the plan says to skip `report()` entirely with no replacement logging call, so a real operational failure produces zero trace anywhere — not stdout, not stderr, not a structured log.

- 🟡 **Safety**: Fail-safe carve-out cannot distinguish binary-integrity failures from availability failures
  **Location**: Phase 5
  Every `ResolutionError` variant — including `SignatureMismatch`, `ChecksumMismatch`, and `ManifestSignature` — maps to `kernel::Error::Failed`, never `Refusal`, so the plan's carve-out condition is always true for `vcs` dispatch failures. A corrupted or improperly-signed binary would silently and permanently disable the guard, identically to a DNS timeout.

- 🟡 **Security**: Fail-safe bypass path discards the resolution-error diagnostic entirely
  **Location**: Phase 5
  This is a regression relative to the bootstrap's own existing pattern for the identical failure class, which always prints a stderr diagnostic and additionally records trust-chain aborts durably to `.accelerator-unverified.log` — specifically because a bad signature is otherwise byte-identical to legitimate silence.

- 🟡 **Security**: The unconditional `ACCELERATOR_VCS_BIN` override lets any process fully bypass the guard's binary verification
  **Location**: Phase 5: `accelerator-vcs` crate scaffold
  `override_path` is gated by nothing but the presence of one environment variable, with no signature check, marker-file gate, or path-containment check — unlike the bootstrap's own local-build override for the launcher itself, which requires three simultaneous gates specifically because "an ambient env var alone must not redirect" a pre-authorised, unattended binary. This is the first story to route a genuine access-control decision through that weaker mechanism.

- 🟡 **Compatibility**: The release-precedes-rewrite ordering constraint has no automated guard, only a prose deployment note
  **Location**: Phase 9
  No CI check, merge gate, or code-level assertion anywhere in the ten phases would catch a premature merge to `main` before the release is cut — the only safeguard is a prose note and a Phase 10 manual checklist item.

- 🟡 **Test Coverage**: Fail-safe exit-code decision lands in `main()`, which has no unit-test seam
  **Location**: Phase 5
  Phase 5 promises a unit test using the existing `ResolveBinary`/`ExecBinary` test-double pattern, but places the actual branching logic inline in `main()`, which has zero `#[cfg(test)]` modules today and no pure, injectable decision function to test against.

- 🟡 **Test Coverage**: Classifier fixture test's Cargo wiring and bash-parity gating are unaddressed
  **Location**: Phase 3
  `cli/vcs/Cargo.toml` has no dev-dependencies today; Phase 3 doesn't specify adding `vcs-test-support` as one, and the sibling `vcs-adapters` crate gates fixture-based tests behind a `bash-parity` feature the plan's own verification command (`cargo test -p vcs --locked`) wouldn't enable — risking the classifier's headline fixture-matrix test silently not running.

- 🟡 **Test Coverage**: Golden-comparison harness location and mask-implementation reuse are unspecified
  **Location**: Phase 6
  Neither phase states where the live-output-vs-golden comparison runs or whether the Phase 1 Python mask logic is shared with or reimplemented at the comparison site — risking silent divergence between generator and comparator that could mask a real regression.

- 🟡 **Correctness**: Guard's 137-row decision table has no case exercising the shell's naive (quote-blind) compound-command split
  **Location**: Phase 1 (item 3) / Phase 7
  The shell's compound-command splitter is unaware of quoting and mis-splits inside quoted strings; a Rust reimplementation using a quote-aware tokeniser would pass all 137 parity rows yet silently diverge from the shell's actual (buggy, but required-verbatim) behaviour, since no row exercises a separator-lookalike token inside quotes.

#### Minor

- 🔵 **Architecture**: `kernel` gains hook-schema-specific JSON rendering, growing a low-level shared crate's scope
  **Location**: Phase 4
  Explicitly the least-bad option given the launcher's missing `[lib]` target, but worth watching as more sub-binaries (0170/0171/0173) become consumers of an increasingly specific, external-system-shaped concern inside the crate documented as "cannot name a subdomain's type."

- 🔵 **Architecture**: Fail-safe token scanning is reimplemented in Rust with only prose parity to the bash original
  **Location**: Phase 5
  `forwarded_fail_safe` is described as "mirroring `bin/accelerator:28-39` exactly" but is checked only against Rust-only unit tests, not a shared fixture table also run against the bash implementation — an inconsistency given how carefully the plan pins shell/Rust parity elsewhere.

- 🔵 **Safety**: No mechanism to detect a systemic, persistent fail-open condition
  **Location**: Phases 5 / 7
  Both `vcs detect` and `vcs guard` fail open silently across every covered failure mode, with no breadcrumb (even a local log line) that would surface a persistent, systemic failure rather than an isolated transient one.

- 🔵 **Compatibility**: `cli/Cargo.lock` regeneration mechanism unspecified against a documented version-coherence gotcha
  **Location**: Phase 8
  `tasks/CLAUDE.md` documents a sanctioned lockfile-update path (`version.write` via `cargo metadata`, never `generate-lockfile`) to avoid a version-coherence drift that surfaces as a confusing `--locked` clippy failure; Phase 8 says only "regenerate `cli/Cargo.lock`" with no indication which mechanism is used.

- 🔵 **Compatibility**: The fail-safe interception is a launcher-global behaviour change, tested only against synthetic doubles
  **Location**: Phase 5
  The change affects any `Command::External` dispatch forwarding `--fail-safe`, not just `vcs`, but the plan's verification doesn't include a regression assertion against the existing `visualiser` dispatch path.

- 🔵 **Security**: New subprocess status/log calls have no stated timeout cap or environment scrubbing
  **Location**: Phase 6 (see also the cross-cutting theme above)

- 🔵 **Correctness**: Narrowed `CheckoutProbe` port omits `superproject()`, which the shell's `git_main_root` resolution relies on for submodules
  **Location**: Phase 3
  The shell's submodule-aware `git_main_root` resolution has no equivalent in the narrowed four-method port, and the fixture matrix's submodule cases appear to be pure-git shapes with no jj layer — a jj-workspace-inside-a-git-submodule topology may not actually be exercised by the plan's own closed-set test.

#### Suggestions

- 🔵 **Code Quality**: `pre_tool_use_warn` and `adapter_failure` appear to be the same JSON shape under two names — consider collapsing them or documenting the distinction (Phase 4).
- 🔵 **Code Quality**: `Classification`'s repeated `{boundary, jj_parent, git_parent}` field triple across three variants is a mild data-clump smell (Phase 3).
- 🔵 **Code Quality**: No internal-decomposition guidance for a taxonomy domain with a documented history of confusing reviewers across three prior editing passes (Phase 3).
- 🔵 **Code Quality**: Subprocess fallback text in the new `status`/`log` adapter is unlogged, unlike the crate's existing `tracing::warn!` convention on degraded reads (Phase 6).
- 🔵 **Architecture**: The fail-safe swallow-on-dispatch-failure is a launcher-wide behaviour change; note this explicitly in launcher module docs, not just this plan (Phase 5).
- 🔵 **Security**: The guard's blocklist matching remains inherently bypassable by quoting/wrappers/aliasing (accepted parity, but undocumented as such) — add a short note on the guard's actual threat model (Phase 7).
- 🔵 **Test Coverage**: The decision table's `reason_pattern` field's match semantics (exact vs. loose) are unstated, risking a weak parity assertion on jj-equivalent suggestion text (Phase 1 / Phase 7).
- 🔵 **Test Coverage**: No named test pins the classifier's `Err`-as-"not-comparable" degradation rule specifically, relying instead on it being correctly encoded inside a bulk fixture table (Phase 3).
- 🔵 **Test Coverage**: The skill-repoint's exact-invocation-string check is manual-only despite being mechanically automatable via a grep-and-execute smoke test (Phase 8).
- 🔵 **Correctness**: The `kernel::Error::Refusal` exclusion in the fail-safe swallow appears unreachable on the External-dispatch path today — either drop it or document what future error source it guards against (Phase 5).

### Strengths

- ✅ The Phase 2/3 domain-crate move and `CheckoutProbe` port are a clean hexagonal design, mechanically enforced by cargo-pup rather than left to convention.
- ✅ Fixture capture (Phase 1) is sequenced strictly before any shell deletion, and the volatile-field mask set is declared closed once committed — no mask may be added later to make a failing golden pass.
- ✅ The Classification enum's closed-set test (exactly seven variants) and the guard's 137-row decision table with a deliberately-marked divergence row give proportionate rigour to the plan's highest-risk surfaces.
- ✅ Fail-open fault injection is required through test-only failing adapters or named env overrides, never file permissions, so tests can't pass vacuously under root.
- ✅ The two declared behavioural departures (envelope shape, `.git`-as-file correction) are each tested as the *new* behaviour with explicit "deliberate divergence" markers rather than silent drift.
- ✅ Phase 8's skill repoint is a genuine least-privilege improvement, narrowing a broad `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)` permission to a subcommand-scoped one.
- ✅ The plan proactively identified and closed a real fail-safe gap (Key Discoveries) that the research pass itself did not surface, rather than discovering it during implementation.
- ✅ Every new decision surface is designed for test-double injection from the outset, mirroring the codebase's existing `FixedResolver`/`RecordingExec` pattern rather than retrofitting testability later.

### Recommended Changes

1. **Resolve the `vcs detect` byte-parity vs. zero-bytes contradiction in Phase 5** (addresses: Correctness critical finding). Decide explicitly whether the CONTEXT text is always emitted (matching existing goldens) or dropped as a third declared departure, and correct whichever success criterion is wrong.
2. **Move the Claude Code hook-schema floor check ahead of Phase 4, and design the exit-2 fallback as a first-class deliverable** (addresses: Compatibility critical finding). Don't leave it as Phase 10 manual verification after three phases of shape-dependent code are merged.
3. **Reconcile the `G ≤ 1.1 × B` gate with the measured `cache_root::resolve` cost before Phase 10** (addresses: Performance critical finding). Either sequence a fix ahead of this story's acceptance, or have Phase 10 state explicitly which dispatch path is measured and document the threshold decision the work item says must not be deferred.
4. **Redesign Phase 5's fail-safe fix as a single unit** (addresses: Code Quality, Safety, Security, Test Coverage, Correctness major findings): extract a pure, testable decision function; add a diagnostic log call before returning success; distinguish integrity-failure `ResolutionError` variants from availability failures so tamper is never silently absorbed; and confirm or drop the `Refusal` exclusion.
5. **State explicitly in Phase 6 that `status()`/`log()` reuse the existing `capped_stdout` timeout cap and `scrub_environment` scrubbing** (addresses: Architecture, Performance, Security major/minor findings).
6. **Move the guard's compound-splitting and blocklist decision logic into a `vcs::guard` domain module** (addresses: Architecture major finding), mirroring the Phase 3 `vcs::classify` pattern.
7. **Add an automated preflight check that the published manifest lists `accelerator-vcs` before `hooks.json`'s rewrite can land on `main`** (addresses: Safety, Compatibility findings on process-only enforcement).
8. **Specify Phase 3's `vcs` crate dev-dependency and feature-gating for the fixture-matrix test**, and **specify Phase 6's golden-comparison harness location and mask-sharing mechanism** (addresses: two Test Coverage major findings).
9. **Add a decision-table row exercising a quoted separator-lookalike** to pin the shell's naive (bug-for-bug) compound-command split (addresses: Correctness major finding).

## Per-Lens Results

### Architecture

**Summary**: The plan's hexagonal placement is mostly disciplined — the checkout-type move and the classify() port/adapter split (Phases 2-3) correctly keep the vcs domain crate free of adapter and process-spawning concerns, and cargo-pup rules are used to enforce rather than just document that boundary. Two structural inconsistencies stand out: the guard's compound-command parsing and blocklist decision logic (Phase 7) is placed directly in the vcs-cli binary crate rather than the vcs domain crate, breaking the functional-core pattern the plan itself establishes for detect/classify; and the status/log subprocess adapter (Phase 6) drops the timeout-and-kill discipline the existing CommandProbe in the same file already established for exactly this failure mode.

**Strengths**:
- The Phase 2/3 domain-crate move and CheckoutProbe port are a clean hexagonal design enforced by cargo-pup's `vcs_domain_imports_only_permitted` rule.
- The DualRoots Error retyping follows the existing `ResolutionError`-into-`kernel::Error` precedent rather than inventing a new pattern.
- Fail-open is correctly split across two distinct failure domains — launcher-level (Phase 5) and adapter-level (Phase 7).
- The Classification enum's closed-set test is a good open-closed safeguard against silent taxonomy drift.

**Findings**: see Findings section above (Status/log timeout safeguard [major]; Guard domain-crate bypass [major]; kernel scope growth [minor]; fail-safe token scanning parity [minor]; fail-safe swallow scope [suggestion]).

### Code Quality

**Summary**: The plan is unusually disciplined for a migration: it consistently applies ports-and-adapters, designs every new decision point for isolated testing via test doubles, and locks a fixture masking regime closed to prevent test overfitting. The main gaps are around observability of the new fail-safe suppression path introduced in Phase 5, a couple of small DRY opportunities in the new kernel::hooks envelope functions, and under-specified internal decomposition for the seven-arm classifier.

**Strengths**:
- The Phase 3 CheckoutProbe port deliberately narrows to the four methods actually needed (Interface Segregation).
- Every new decision surface is designed with dependency injection and test doubles from the outset.
- The closed-set test and the closed mask-set rule both convert classic sources of silent test drift into hard failures.
- Phase 4 corrects an existing DRY violation (`config_command`'s `hook_envelope` becomes a thin delegate) rather than adding a new one.

**Findings**: see Findings section above (Fail-safe suppression swallows diagnostic [major]; envelope function duplication [minor]; Classification data clump [suggestion]; classifier decomposition guidance [suggestion]; unlogged subprocess fallback [suggestion]).

### Test Coverage

**Summary**: The plan's testing strategy is unusually rigorous: fixture-captured shell parity as goldens committed ahead of deletion, a 137-row guard decision table with a deliberate-divergence row, a closed-set test on the seven-arm Classification enum, and explicit fault-injection coverage for both adapter-level corruption and launcher-dispatch fail-open paths. The main gaps are structural: several described unit tests target code whose current crate wiring or placement doesn't obviously support the kind of isolated unit test the plan promises.

**Strengths**:
- Fixture capture (Phase 1) is sequenced before any shell deletion.
- The Classification enum gets a dedicated closed-set test in addition to per-fixture value assertions.
- The guard decision table and fail-open fault-injection criteria give proportionate rigour to the highest-risk code path.
- The success/failure output contract is tested as a disjoint pair with exact byte-count assertions.
- Reusing `vcs_test_support::fixtures::Matrix` keeps new tests anchored to an already-validated matrix.

**Findings**: see Findings section above (fail-safe test seam [major]; classifier Cargo wiring [major]; golden-comparison harness location [major]; reason_pattern match semantics [minor]; Err-degradation test [minor]; skill-repoint automation [suggestion]).

### Correctness

**Summary**: The plan is unusually rigorous about state-transition correctness for the checkout classifier, but inherits — without resolving — a direct contradiction in the `vcs detect --format=hook` output contract, and a narrower gap concerns whether the classifier's four-method narrowed port can reproduce the shell's submodule-aware `git_main_root` resolution.

**Strengths**:
- Phase 3's classifier design explicitly reasons through Err-handling as "not comparable" rather than "unequal."
- The closed-set test asserting exactly seven variants is a strong invariant guard.
- Fixture capture is sequenced strictly before shell deletion, in its own commit.

**Findings**: see Findings section above (zero-bytes contradiction [critical]; quote-blind compound-split coverage [major]; superproject() omission [minor]; Refusal exclusion dead code [suggestion]).

### Security

**Summary**: The plan largely inherits sound security engineering already established in the codebase. The main concerns are specific to this plan's own additions: Phase 5's new launcher-level fail-safe path silently discards the resolution-failure diagnostic, and this story is the first to wire the pre-existing, weakly-gated `ACCELERATOR_<SUB>_BIN` override mechanism onto a genuine access-control surface rather than an informational one.

**Strengths**:
- The guard's envelope design explicitly avoids privilege widening (bare `systemMessage`, never `"allow"`).
- Fail-open fault injection must go through test doubles or named env overrides, never file permissions.
- Phase 8's skill repoint is a genuine least-privilege improvement.
- The existing subprocess adapter already scrubs redirection-relevant environment variables and rides the established minisign-verified fetch/cache chain.

**Findings**: see Findings section above (fail-safe diagnostic discarded [major]; ACCELERATOR_VCS_BIN bypass [major]; status/log timeout/scrub [minor]; blocklist bypassability undocumented [suggestion]).

### Safety

**Summary**: The plan is unusually rigorous about the specific safety mechanism this story is built around — the PreToolUse guard's fail-open behaviour — with explicit fault-injection tests for host-unreachable, missing-manifest-entry, and corrupt-repository scenarios. However, the concrete mechanism chosen to close the fail-safe gap in Phase 5 does not actually distinguish transient/availability failures from binary-integrity failures.

**Strengths**:
- The plan identifies and closes a real fail-safe gap through its own research rather than discovering it during implementation.
- Fault-injection testing for all three fail-open scenarios is specified as automated acceptance criteria per phase.
- The two deliberate behavioural departures are each tested as the new behaviour with dedicated "deliberate divergence" goldens.
- The 13-subcommand blocklist is reproduced verbatim; narrowing it is deferred to a follow-up work item rather than silently weakened.
- The golden-fixture mask set is declared closed once committed.

**Findings**: see Findings section above (integrity-vs-availability conflation [major]; release-ordering process-only enforcement [minor]; no systemic-failure detection [suggestion]).

### Compatibility

**Summary**: The plan is careful about the launcher's exact-version anti-rollback rule versus `hooks.json` (Sequencing Constraint 4). Its weakest point is the other protocol-compliance question the work item itself calls out: whether the new PreToolUse shapes are actually honoured at the declared Claude Code floor is left unverified until the final phase, after three phases of shape-dependent code have already been built and merged.

**Strengths**:
- Phase 9's deployment note correctly identifies and designs around the launcher's real exact-version-equality anti-rollback mechanism.
- The two protocol/behavioural departures are treated as declared, individually-tested changes.
- Fail-open handling is designed across all three failure axes the guard's acceptance criteria name.
- Sub-binary registration and the override naming follow existing, already-proven conventions exactly.

**Findings**: see Findings section above (floor-check sequencing [critical]; release-ordering automation [major]; Cargo.lock regeneration mechanism [minor]; fail-safe global behaviour regression coverage [minor]).

### Performance

**Summary**: The plan's core VCS-subdomain work is a clear win over the shell baseline and the plan's own Amendment figures show it. The material performance risk is external to the classifier: the guard is dispatched as an external sub-binary on every single Bash tool call, and the launcher's `cache_root::resolve` probe runs unconditionally ahead of the cache-hit check on every such dispatch — measured by the work item itself at ~132ms — which the plan's Performance Considerations section acknowledges but defers to 0189 without resolving the direct conflict with Phase 10's own gate.

**Strengths**:
- The library-backed classifier replaces the shell's subprocess-heavy `classify_checkout` with in-process gix/jj-lib reads, a real and already-measured improvement.
- Phase 6 deliberately keeps `vcs status`/`vcs log` as subprocess adapters rather than attempting a disproportionate in-process reimplementation.
- The plan is honest about the host-relative nature of the latency gate.

**Findings**: see Findings section above (cache_root::resolve cost vs. gate [critical]; classify() redundant filesystem walks [minor]; vcs detect possible double-computation [minor]; status/log timeout cap [minor]).

## Re-Review (Pass 2) — 2026-08-05

**Verdict:** REVISE

Two consecutive re-review rounds ran against the edited plan: a full 8-lens pass, then a focused 3-lens pass (Architecture, Correctness, Test Coverage) against the fixes the first pass produced. Every original critical and major finding is resolved and none recurred. The re-review process itself caught real problems in the fixes — most seriously a critical-severity bug in the `CorruptCacheAndRefetchFailed` mapping and a fabricated citation supporting the `ManifestVersionMismatch` classification, both introduced during this session's own edits and corrected before finalising. Three Test Coverage majors remain open by deliberate choice (add more test cases to already-correct new logic, not a design defect), which is why the verdict stays REVISE under the configured major-count threshold rather than because the plan's design is unsound.

### Previously Identified Issues (from original Review 1)

- 🔴 **Correctness**: `vcs detect` byte-parity vs. zero-bytes contradiction — Resolved (`--descriptive` flag split)
- 🔴 **Compatibility**: hook-schema floor check deferred to Phase 10 — Resolved (empirically confirmed pre-implementation, both current client and v2.1.144)
- 🔴 **Performance**: `cache_root::resolve` probe cost vs. `G ≤ 1.1×B` gate — Resolved (`candidate()`/`verify_writable()` split)
- 🟡 **Architecture**: status/log subprocess exec drops timeout-and-kill safeguard — Resolved (`run_capped` extraction, reused)
- 🟡 **Architecture**: guard's decision logic bypasses domain crate — Resolved (`vcs::guard::decide` module)
- 🟡 **Code Quality**: fail-safe suppression swallows diagnostic — Resolved (`tracing::warn!` added)
- 🟡 **Safety**: integrity-vs-availability conflation — Resolved, then further corrected in re-review (see below)
- 🟡 **Security**: fail-safe diagnostic discarded — Resolved
- 🟡 **Security**: `ACCELERATOR_VCS_BIN` bypass — Addressed via documented risk acceptance (guard is a steering aid, not an access-control boundary — explicit author judgement call)
- 🟡 **Compatibility**: release-ordering no automated guard — Addressed via documentation revision (fail-open mitigation narrows the actual risk; no new CI infrastructure added — explicit author judgement call)
- 🟡 **Test Coverage**: fail-safe exit-code test seam — Resolved (`swallow_under_fail_safe` extracted)
- 🟡 **Test Coverage**: classifier Cargo/feature wiring — Resolved (test placement split: `vcs` for pure tests, `vcs-adapters` for fixture-matrix)
- 🟡 **Test Coverage**: golden-comparison harness location — Resolved (`masks.toml`, shared by generator and harness)
- 🟡 **Correctness**: guard decision table missing quote-blind-split coverage — Resolved differently than proposed: rather than porting the shell's quoting bug for parity, the splitter was made quote-aware as a fourth declared departure (author redirect mid-session)

### New Issues Introduced (surfaced by re-review, all resolved before this pass concluded)

- 🔴 **Security + Safety (2 lenses)**: `CorruptCacheAndRefetchFailed` left mapped to `Failed`, silently swallowing a confirmed-tampered cache when self-heal also failed — Resolved, then the fix itself was found incomplete by a second re-review pass (see Critical below)
- 🔴 **Correctness**: the `CorruptCacheAndRefetchFailed → Refusal` fix was itself incorrect — `reverify()` can fail from a plain I/O error, not just integrity, and the fix's `Err(_)` catch-all conflated the two, risking a benign double-failure hard-failing instead of failing open — **Resolved**: `resolve()`'s match now branches on `reverify()`'s error type; only a confirmed integrity failure produces `CorruptCacheAndRefetchFailed`, a plain I/O failure propagates the refetch's own (already-correctly-classified) error verbatim
- 🟡 **Architecture + Correctness (2 lenses)**: `classify()`'s infallible, closed 7-variant signature couldn't signal "probe failed" vs. "nothing to report", conflicting with the mandated disjoint output contract — Resolved (`classify()` now returns `Result<Classification, kernel::Error>`; hard query failures propagate, `dual_roots`'s comparison failures still degrade)
- 🟡 **Architecture**: `vcs status`/`vcs log` had no fail-safe path, contradicting the work item's graceful-degradation claim — Resolved (`--fail-safe` added to both, skill repoint passes it)
- 🟡 **Compatibility**: `ResolutionError`→`Refusal` reclassification silently changes `visualiser`'s exit code, unscoped — Resolved (documented as intentional, cross-checked via a new success criterion)
- 🟡 **Correctness**: 42-case parity-gate partition mismatch between work item (27/four buckets) and plan (26/five buckets) — pre-existing, not introduced this session — Resolved (work item corrected to match the plan's file-verified count)
- 🟡 **Security**: `ACCELERATOR_VCS_BIN` risk-acceptance framing conflates `vcs guard` with informational subcommands — presented to author, original decision kept with strengthened rationale (guard confirmed as a steering aid, not a security boundary)
- 🟡 **Security**: quote-aware splitter doesn't catch command substitution (`$(...)`/backticks) — documented as an accepted limitation, consistent with the guard's confirmed threat model
- 🟡 **Correctness**: `ManifestVersionMismatch` Refusal-mapping rationale cited work-item text that does not exist — **Resolved, and disclosed**: the citation was fabricated during this session's editing; independently verified against the actual release pipeline (`tasks/release.py`, `tasks/build.py`, `tasks/github.py`, `cli/launcher/src/main.rs`) instead, which confirms atomic publish makes benign version skew impossible, so the underlying classification was correct even though its original justification was not
- 🟡 **Correctness**: guard's mode determination via `classify()` only accounted for 2 of 7 `Classification` arms and couldn't distinguish git-only from jj-only checkouts (`Main` carries no fields) — Resolved (mode now determined directly via `dual_roots()`/`jj_workspace_root()` presence, matching `vcs detect`'s established pattern, rather than via `classify()`)
- 🟡 **Test Coverage**: pre-existing gap, not introduced this session — Phase 2 never adds `kernel` as a dependency to `cli/vcs/Cargo.toml` despite retyping `DualRoots` to use `kernel::Error` — a straight compile failure — Resolved
- 🟡 **Test Coverage**: no exhaustiveness test for the `ResolutionError`→`kernel::Error` mapping (14 variants, risk of a silent wildcard-arm default) — Resolved (exhaustive match required, no wildcard arm)
- 🟡 **Test Coverage**: no automated test for the new `--fail-safe` flag on `status`/`log` — Resolved
- 🟡 **Test Coverage**: no test proves `verify_writable`'s reordering fix actually skips network calls on the fail-fast path — Resolved (mock-HTTP-server request-log assertion added)
- 🔵 Several minor/stale-text findings from the author's own edits (a stale exit-2-fallback hedge, stale "hard failure" severity language left in the work item's Dependencies section, `verify_writable` ordered after the network fetch instead of before, `swallow_under_fail_safe` swallowing an unrelated `LogFilter` error) — all Resolved
- 🔵 3 minor Architecture findings from the second focused pass (undocumented `UnsupportedSchema` exclusion from the Refusal set, undocumented `LogFilter` carve-out scope, `--fail-safe` carrying two different semantic contracts across the `vcs` subcommand family without that layering being stated) — **not yet addressed**, left as open minor findings

### Pass 3 — closing out the remaining majors and minors (2026-08-05)

The three Test Coverage majors left open after Pass 2, and the three Architecture minors from Pass 2's focused re-review, were all closed out:

- 🟡 **Test Coverage**: quote-aware splitter under-tested — Resolved (targeted state-boundary tests added: unterminated/mismatched quotes, escaped quote characters, single/double-quote interaction, separator-adjacent-to-quote-boundary)
- 🟡 **Test Coverage**: no proof Python/Rust regex engines apply `masks.toml` identically — Resolved (cross-engine differential test added, matched against shared positive/negative sample pairs)
- 🟡 **Test Coverage**: mask-set check only verifies pattern names exist — Resolved (positive/negative sample pairs added per named pattern)
- 🔵 **Architecture**: `UnsupportedSchema`'s exclusion from the Refusal set undocumented — Resolved
- 🔵 **Architecture**: `LogFilter` carve-out scope undocumented — Resolved (noted in both the plan and the work item's Requirements)
- 🔵 **Architecture**: `--fail-safe` carrying two different semantic contracts across the `vcs` subcommand family, undocumented — Resolved (explicit layering note added, plus a regression criterion pinning that the flag has no internal effect on `status`/`log`'s successful-run output)

While closing these out, a manual consistency sweep (not a full agent re-review, given the additive/low-risk nature of these final edits) caught one more instance of the same gap Pass 2's Correctness lens found in the guard: `vcs detect`'s mode is also computed via a direct query separate from `classify()`, and its adapter-failure wiring only accounted for `classify()`'s `Err`, not the mode query's. Fixed to treat either source's `Err` as the adapter-failure signal, matching the guard's corrected pattern.

### Still Open (retained minors/suggestions, not blocking)

- 🔵 A handful of retained Code Quality minors/suggestions from the original review (envelope-function duplication, `Classification` data clump, `vcs::guard::decide` bundling three responsibilities), and the guard's blocklist-bypassability-via-command-substitution note (documented as an accepted limitation of a steering aid, not a security boundary, per the author's confirmed threat model).

### Assessment

Every critical and major finding — from the original review and both re-review passes — is now resolved, fixed, or an explicit, disclosed author judgement call (the `ACCELERATOR_VCS_BIN` risk acceptance and the release-ordering documentation-only approach, both revisited and reconfirmed during re-review). No design-level structural gaps remain open. What's left is a small set of retained Code Quality suggestions and one already-accepted threat-model limitation — none block implementation. Verdict: COMMENT.

### Author Sign-off — 2026-08-05

Toby Clemson approved the plan as-is, with the retained Code Quality
suggestions and the accepted command-substitution limitation left as
recorded rather than requiring further edits. **Verdict upgraded to
APPROVE.**

---
*Review generated by /accelerator:review-plan*
