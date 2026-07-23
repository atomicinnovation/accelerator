---
type: plan-review
id: "2026-07-11-0179-corpus-crates-parsing-conventions-review-1"
title: "Plan Review: corpus and corpus-adapters Crates for Parsing and Conventions Implementation Plan"
date: "2026-07-11T13:45:34+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-07-11-0179-corpus-crates-parsing-conventions"
reviewer: "Toby Clemson"
verdict: APPROVE
lenses: [architecture, correctness, test-coverage, code-quality, compatibility, portability, standards, safety]
review_number: 1
review_pass: 3
tags: [rust, corpus, crates, plan-review, vcs, frontmatter]
last_updated: "2026-07-11T21:25:45+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: corpus and corpus-adapters Crates for Parsing and Conventions Implementation Plan

**Verdict:** REVISE

The plan is a disciplined, convention-faithful extension of the proven 0178
hexagonal pattern — an acyclic crate DAG, kernel-only domains with injected ports,
a genuine reduction in parser surface (one `document` crate, no `catch_unwind`),
and a strong test-first posture anchored to bash/visualiser parity oracles. Every
lens credited the structure. It earns a REVISE (not for being unsound but for
being under-specified in ~15 major spots) because several consolidations and the
new VCS domain contain concrete logic gaps that would ship wrong results as
written: two collapsed width parsers with different missing-width defaults, a
dropped slug fallback, a `RepoFacts` type that can't represent the bare-repo state
Phase 5 must blank, and a deny-ban test change that would break the very success
criterion it's meant to satisfy. The dominant theme is the VCS command-probe,
which five separate lenses flagged for missing failure/hygiene/timeout semantics.

### Cross-Cutting Themes

- **VCS command-probe robustness & hygiene** (flagged by: architecture,
  correctness, code-quality, portability, safety — 5 lenses) — the `CommandProbe`
  that shells to `jj`/`git` for the revision is under-specified on every failure
  edge: no defined mapping for missing binary / non-zero exit / empty stdout
  (should be `None`, not `Some("")` or `Some(stderr)`); no env hygiene
  (`vcs-common.sh` scrubs `GIT_DIR`; the metadata test isolates `HOME`/`XDG`/
  `JJ_CONFIG`), so ambient config can inject ANSI/colour or a wrong root into the
  captured revision; no timeout, so a locked/networked repo can hang metadata
  derivation; and "bare repo → blank" is conflated with "probe errored", silently
  dropping provenance. This is the single highest-value cluster to fix.

- **Value-model layering: drift risk across near-identical enums** (flagged by:
  architecture, code-quality, correctness, standards) — four structurally-identical
  Scalar/Seq/Map enums (`document::Value`, `config::Node`, `corpus::FrontmatterValue`,
  the `corpus-adapters` serde mirror) joined by hand-written mappings with
  `#[non_exhaustive]` wildcard arms that silently downgrade unknown scalars to
  `Null`. Adding one scalar kind means editing four enums; the wildcard makes
  precision loss a runtime bug, not a compile error. Plus a naming collision:
  `document::Value` vs the existing `config::Value`.

- **"Port verbatim" vs deliberate behaviour changes** (flagged by: test-coverage,
  correctness) — the strategy says port the visualiser test tables verbatim, but
  the plan deliberately changes number policy (f64/Null → String) and YAML-tag
  handling, so those specific ported assertions must be *rewritten*, not copied,
  or the String-preservation and clean-error ACs go unverified.

- **External-tool tests can pass silently green** (flagged by: test-coverage,
  portability) — the parity and VCS-detection suites shell to bash scripts / a real
  `jj`, but Rust's harness has no skip primitive, so a gated-out or tool-missing
  test early-returns as a green PASS. The 0178 plan handled exactly this with a
  loud bash-probe; 0179 must carry it forward and make "fails loudly if the script
  moves" an automated assertion, not a manual checkbox.

- **Serialize wire contract: divergent or premature** (flagged by: compatibility,
  code-quality) — the hand-written `Serialize` diverges from the SPA's current JSON
  shape (order-preserving vs `BTreeMap` sorted keys; big-int → String vs numeric),
  yet has no consumer inside 0179 (the SPA boundary is 0168). Either defer it or
  record the divergence as a conscious 0168 hand-off decision.

### Tradeoff Analysis

- **Parser resilience vs surface reduction** (safety vs architecture/code-quality):
  Safety wants a thin `catch_unwind` retained (or fuzz-grade coverage) on the
  corpus-scan Malformed path so one poison file can't crash a future 0168 scan;
  Architecture/Code-Quality credit removing the `catch_unwind`/libyml machinery as
  a real simplification enabled by pure-Rust serde-saphyr. Recommendation: keep the
  removal (the blast radius is nil until 0168 wires a scanner), but broaden the
  adversarial corpus toward fuzz-style inputs and note the resilience guarantee as
  a 0168 hand-off item — don't re-introduce `catch_unwind` speculatively.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Compatibility**: Deny ban-test change is mis-specified; the regression will break
  **Location**: Phase 1, Change 3 (test_serde_saphyr_ban.py)
  The clean fixture package is named `config-adapters` and depends directly on serde-saphyr; once it's no longer a listed wrapper, cargo-deny flags it and the "passes" test fails. The *clean* fixture (not the banned one) must be renamed to `document`.

- 🟡 **Correctness**: Collapsed width parsers have different missing-width defaults (4 vs 0)
  **Location**: Phase 2 §7 (Consolidations); Key Discoveries
  `number_width_from_id_pattern` returns `unwrap_or(4)`; `canonical_digit_width` returns `0` ("admit any"). A single merged default flips one former caller's semantics (id-canonicalisation padding, or token rejection). Also `number_width_from_id_pattern` is server-side, so the collapse is really deferred to 0168 like the title-casers.

- 🟡 **Correctness**: WorkItems slug arm drops the visualiser's regex→pure-numeric fallback
  **Location**: Phase 2 §6 (Slug conventions)
  The visualiser composes `derive_work_item_with_regex(...).or_else(|| slug::derive(WorkItems, ...))` in `build_entry`. The scanner-only arm has no equivalent, so a legacy bare-numeric `0042-foo.md` under a `{project}-` pattern yields no slug instead of `foo` — a silent parity regression.

- 🟡 **Correctness**: `RepoFacts` is total but can't represent the bare/no-repo state Phase 5 must blank
  **Location**: Phase 4 §2 + Phase 5 §2
  `RepoFacts.root/name` are non-optional but `discover → None` for bare/non-VCS dirs. `facts()` must still return something, and Phase 5's "blank revision/name rather than erroring" is unrepresentable. Make `facts` return `Option<RepoFacts>` (or optional fields) and define the None→all-blank mapping.

- 🟡 **Correctness**: FrontmatterState parity for non-mapping root / tags / null root unspecified
  **Location**: Phase 3 §2 (FrontmatterState)
  The visualiser's `Malformed` covers a non-mapping root (`- a`/`- b`), a `Tagged` value, and maps a `Null` root to an *empty mapping* (Parsed). The plan must specify the root-shape rule and verify serde-saphyr actually errors on custom tags (some `!!` tags resolve to base types).

- 🟡 **Test Coverage**: External-tool parity/detection tests can pass silently green
  **Location**: Phase 3 (parity/harness) + Phase 4 (detection.rs)
  Rust has no skip primitive; a gated-out or `jj`-missing test early-returns as PASS. The 0178 plan asserts tool presence and hard-fails; 0179 must carry that forward and automate the "script moved" check.

- 🟡 **Test Coverage**: "Port test tables verbatim" contradicts the number-policy and YAML-tag changes
  **Location**: Testing Strategy > Unit Tests
  A verbatim port imports assertions that now contradict intended behaviour; the changed cases must be called out as deliberate rewrites so String-preservation and clean-error semantics stay verified.

- 🟡 **Test Coverage**: AC-6 single-source test under-specifies extraction from runtime-derived surfaces
  **Location**: Phase 3 §6
  The migration snapshot re-serialises a config-injected table at runtime and the awk `path_to_typed` is a matcher, not a static table. Specify per-surface extraction (execute the emit/matcher against a known path set) and assert the extracted set is non-empty / variant-complete.

- 🟡 **Test Coverage**: Parity corpus claims "14 variants" but Templates/PrDescriptions have no bash oracle
  **Location**: Phase 3 §7
  Bash emits 13 rows; `Templates` (virtual, slug→None) and `PrDescriptions` (wire/config-key mismatch) can't be diff-tested. Split the corpus into a diff-tested subset (13) and a declared-value subset asserted directly.

- 🟡 **Code Quality**: Four structurally-identical enums with wildcard silent-degradation arms
  **Location**: Implementation Approach (value-model layering); Phase 3 §2
  `#[non_exhaustive] Scalar` forces `_` wildcards in each mapping, so a future variant is silently downgraded to `Null` rather than caught at compile time. State the no-wildcard invariant and add a `document::Value ↔ FrontmatterValue ↔ Node` round-trip conformance test.

- 🟡 **Compatibility**: Hand-written `Serialize` wire contract diverges from the SPA's current JSON shape
  **Location**: Key Discoveries / Phase 3 §2
  Shipped SPA sees `BTreeMap` (sorted keys) with numeric big-ints; the new model is order-preserving with big-int → String. 0179 is safe (server untouched), but a naive 0168 swap silently alters SPA JSON — record it as a conscious 0168 decision.

- 🟡 **Architecture**: VCS root-detection blind spot for `.git`-as-file worktrees left untested
  **Location**: Phase 4 (design decision + fixtures)
  The research names `.git`-as-file worktrees/submodules as a fourth divergence; the plan neither specifies marker semantics (existence vs is-directory) nor adds a fixture. As a foundational primitive, a baked-in blind spot forces 0169 to *correct* rather than *extend* it.

- 🟡 **Portability**: Filename-timestamp timezone assumption unstated, can diverge from the bash oracle
  **Location**: Phase 5 (Clock); Phase 2 §8
  Bash uses `date -u` for the ISO line but plain `date` (host-local) for the filename timestamp. The FakeClock test can't catch a UTC-vs-local mismatch. Specify local-time for the filename timestamp (or a deliberate UTC deviation) and add a real-clock parity assertion.

- 🟡 **Portability**: Shipped jj/git probe inherits ambient config/env without the oracle's isolation
  **Location**: Phase 4 §3 (CommandProbe)
  The bash test isolates `HOME`/`XDG`/`JJ_CONFIG` and scrubs `GIT_DIR`; the probe mentions no equivalent, so ambient config can corrupt the revision string. Add `--color=never`/`--no-pager`, `-c color.ui=false`, and `GIT_DIR`/`GIT_WORK_TREE` scrubbing.

- 🟡 **Safety**: Fail-closed write guard not pinned in `document`'s own contract
  **Location**: Phase 1 §1 (render) + §2 (retrofit)
  The shipped config write is fail-closed only because `preserved_body` *re-parses* (not merely splits) the existing frontmatter. `document::render`'s contract must state the re-parse, and `a_write_against_a_malformed_file_fails_closed` should be an explicit Phase 1 success criterion, not just "suites unchanged".

#### Minor

- 🔵 **Correctness**: `MAX_SCAN` cap consistency between `fence_offsets` (capped) and `split`/`parse` (uncapped) unresolved — deciding whether owned `split` inherits the 1 MiB cap either silently adds a ceiling to shipped config or lets the two forms disagree at the boundary. **Location**: Phase 1 §1 / Phase 3 §2
- 🔵 **Correctness**: Empty/failed revision command and marker-walk vs `discover_root` divergence not handled — a no-commit repo could surface `Some("")`; the "matches discover_root" manual check can't hold (`discover_root` also stops at `.accelerator` and returns `start`). **Location**: Phase 4 (Revision) + Manual Verification
- 🔵 **Architecture**: `document::Value` couples the two previously-independent adapter stacks at the parse layer; keep it the pure YAML-scalar union and push shaping into each adapter's mapping. **Location**: Phase 1 (value-model layering)
- 🔵 **Architecture**: Three domain enums synced only by hand — add a cross-enum round-trip test so drift fails a test. **Location**: Implementation Approach
- 🔵 **Architecture**: Two coexisting Rust parsers (corpus + untouched visualiser) already fork on number policy; point the corpus fixtures at the visualiser's own test inputs and record the fork as a 0168 delta. **Location**: What We're NOT Doing
- 🔵 **Code Quality**: Stringly-typed `Result<_, String>` persists at the newly-shared `document` boundary; a typed `DocumentError` (`Unterminated`/`InvalidYaml`/`NonMappingRoot`) lets consumers branch and map into `kernel::Error`. **Location**: Phase 1 §1
- 🔵 **Code Quality**: Hand-written `Serialize` mirror has no consumer within 0179 (YAGNI) — defer to 0168/0173 or justify + round-trip test it. **Location**: Phase 3 §2
- 🔵 **Code Quality**: `CommandProbe` collapses every failure to a blank revision with no diagnostic — log at debug/warn via `kernel::logging` on unexpected spawn/exit failure. **Location**: Phase 4 §3
- 🔵 **Test Coverage**: Byte-for-byte body preservation is only a manual spot-check though it's an explicit AC — add an automated `document`-level render test (CRLF, trailing-newline edges). **Location**: Phase 1 Manual Verification
- 🔵 **Test Coverage**: Bounded-time guard lacks a named alias-expansion (billion-laughs) fixture to actually exercise it. **Location**: Phase 1 §4
- 🔵 **Test Coverage**: Scanner `match_end` (full-match end, delimiter consumed) vs capture-end not pinned by a targeted assertion. **Location**: Phase 3 §4
- 🔵 **Safety**: Removing `catch_unwind` drops the corpus-scan poison-file fail-safe — broaden adversarial coverage toward fuzz before 0168 wires a scanner. **Location**: Phase 1 §4 / Phase 3 §2
- 🔵 **Compatibility / Standards**: New `regex` declared inline `= "1"` instead of via `[workspace.dependencies]` with `{ workspace = true }` — diverges from the exact-pin convention. **Location**: Phase 3 §1
- 🔵 **Standards**: `document::Value` collides with the existing `config::Value` re-export — consider `document::Frontmatter`/`document::Yaml` or note the collision. **Location**: Phase 1 §1
- 🔵 **Standards**: Domain modules must use single-item `use crate::...;` imports to satisfy the whole-crate pup rule (a cost 0178 already paid) — surface it. **Location**: Phase 2 §1 / Phase 4 §1
- 🔵 **Portability**: Clock time source unspecified and no time crate declared — state whether `SystemClock` shells to `date` (POSIX specifiers only) or uses a pure-Rust time crate (prefer the latter for self-contained musl/darwin binaries). **Location**: Phase 3 §1 / Phase 5 §2
- 🔵 **Portability**: Parity harness couples to a repo-relative path + exec bit + resolvable bash; gate on existence+executability with a loud error and confirm it never runs from a published artifact. **Location**: Phase 3 §4

#### Suggestions

- 🔵 **Architecture**: `Clock` placement asymmetry — it's cross-cutting like VCS but lives in `corpus`; note it's scoped to artifact-metadata (or colocate with infra if a second consumer is foreseeable). **Location**: Phase 2 §8
- 🔵 **Architecture**: State the `CommandProbe` subprocess failure contract on the port (missing binary / non-zero → `revision: None`). **Location**: Phase 4 §3
- 🔵 **Correctness**: Big-int-as-String only covers `u64`; beyond-`u64` still widens to `Float` — note the residual (accepted, out of domain range) or catch it. **Location**: Phase 1 §1
- 🔵 **Code Quality**: Title-caser consolidation is partial (one canonical in `corpus`, two server copies retire in 0168) — make Success Criteria say so and anchor the eventual retirement. **Location**: Phase 2 §7
- 🔵 **Standards**: Specify the final `members` ordering (domain-before-adapters grouping). **Location**: Phases 1-5
- 🔵 **Safety**: Consider a modest wall-clock cap on the VCS probe and distinguish "bare → blank" from "probe error". **Location**: Phase 4 / Phase 5

### Strengths

- ✅ Faithfully reuses the shipped 0178 hexagonal pattern (kernel-only domain + adapters, per-domain cargo-pup rule, cargo-deny wrapper ban) rather than inventing structure; the enforcement re-homing keeps the single-wrapper invariant.
- ✅ Injected ports (`IdScanner`, `Clock`, `RepoRoot`/`VcsProbe`) push regex, wall-clock, and subprocess I/O out of the domains — a textbook functional-core/imperative-shell split, unit-testable behind fakes.
- ✅ Acyclic, inward-pointing dependency DAG; no domain crate names another domain's or an adapter's types.
- ✅ Genuine surface reduction: three frontmatter parsers → one `document`; the `catch_unwind`/libyml sandbox removed in favour of a pure-Rust parser under a bounded-time guard (also a musl/cross-compile win).
- ✅ Test-first discipline: the visualiser's own test tables + the bash parity suites as independent behavioural oracles, spanning all identity schemes and pinned edge filenames; enforcement (deny/pup) treated as testable regressions.
- ✅ Explicitly reconciles behavioural deltas (number policy, YAML tags, `catch_unwind` removal, `extract_id` fallback) instead of lifting one implementation blindly.
- ✅ Consolidation with a single source of truth: one `canonical_digit_width`, `split` derived from `fence_offsets`, big-int policy applied once at the visitor boundary, doc-type single-source test.
- ✅ Narrow danger surface: the only shipped code touched is the config read/render path; `atomic_write`/`store.rs` untouched; risky persistence deferred to 0180; patcher lands as pure byte-preserving fail-closed functions.
- ✅ Convention-faithful naming, British spelling, `#[non_exhaustive]` scalar, and the kernel-only rule honoured in letter and spirit.

### Recommended Changes

1. **Fix the deny ban-test spec** (addresses: Compatibility deny-test) — rename the *clean* fixture package (and its `[[bin]]`) from `config-adapters` to `document` to match `wrappers = ["document"]`; leave the banned fixture (named `config`) unchanged. This is a one-line correction but blocks a Phase 1 success criterion.

2. **Harden the VCS `CommandProbe` contract** (addresses: the 5-lens VCS theme — Architecture/Correctness/Code-Quality/Portability/Safety) — specify on the port: missing binary / non-zero exit / empty stdout → `revision: None`; env hygiene (`--color=never`/`--no-pager`, `git -c color.ui=false`, scrub `GIT_DIR`/`GIT_WORK_TREE`); a modest wall-clock cap; log unexpected failures via `kernel::logging`; and distinguish "bare → blank" from "probe error". Add an empty-repo (no-commit) fixture.

3. **Resolve the width-parser collapse** (addresses: Correctness width-parser) — state the merged `canonical_digit_width` default explicitly, add unit cases pinning the missing-width and `{number:0d}` inputs, and note that since `number_width_from_id_pattern`'s call site is server-side, the collapse (like the title-casers) is partly deferred to 0168.

4. **Restore the WorkItems slug fallback** (addresses: Correctness slug fallback) — reproduce `regex_slug.or_else(pure strip_prefix_work_item_id)` in the WorkItems arm and add a bare-numeric-under-project-pattern parity fixture.

5. **Make `RepoFacts` representable for bare/no-repo** (addresses: Correctness RepoFacts) — `facts()` returns `Option<RepoFacts>` (or optional fields); define the None → all-blank `ArtifactMetadata` mapping.

6. **Specify the FrontmatterState root-shape rule** (addresses: Correctness FrontmatterState) — Null/empty → empty mapping; Seq/Scalar root → Malformed; add root-sequence and custom-tag fixtures proving serde-saphyr yields a clean parse error.

7. **Make external-tool tests fail loud, not skip** (addresses: Test-Coverage silent-green, Portability harness) — assert tool/script presence+executability at test start and hard-fail when absent in CI; convert the "script moved" manual check to an automated assertion.

8. **Split the parity corpus and reconcile the "verbatim" instruction** (addresses: Test-Coverage 14-variants + verbatim) — diff-test the 13 bash-emitted types; assert Templates→None and the PrDescriptions wire/config mismatch as declared values; call out which ported assertions must be *rewritten* for the number-policy/tag changes; add the AC-6 per-surface extraction contract (execute emit/matcher against a known path set; assert non-empty/variant-complete).

9. **Tighten the value-model layering** (addresses: Code-Quality four-enums, Architecture coupling/drift, Standards naming) — state the no-`_`-wildcard mapping invariant; add a `document::Value ↔ FrontmatterValue ↔ Node` round-trip conformance test covering every scalar kind incl. big-int-as-String; keep `document::Value` the pure scalar union; and rename it (e.g. `document::Frontmatter`) to avoid the `config::Value` collision, or note the collision.

10. **Decide the Serialize contract** (addresses: Compatibility wire-shape, Code-Quality YAGNI) — either defer the `corpus-adapters` serde mirror to the work item that first serialises over the wire (0168/0173), or keep it and record the deliberate divergence (order-preserving keys, big-int → String) as a 0168 hand-off with a round-trip test.

11. **Pin the timezone + clock source** (addresses: Portability timezone + clock source) — filename timestamp = host-local (matching un-`-u`'d `date`), ISO line = UTC; state whether `SystemClock` uses a pure-Rust time crate (preferred) or shells to `date` with POSIX-only specifiers; add a real-clock parity assertion for the filename timestamp zone.

12. **Smaller tightenings** (addresses: remaining minors/suggestions) — `regex` via `[workspace.dependencies]` + `{ workspace = true }`; a typed `DocumentError`; the `document`-level byte-for-byte render test + an alias-expansion adversarial fixture + a scanner `match_end` assertion; the `MAX_SCAN` cap decision for `split`; the single-item `use crate::...;` pup note; the final `members` ordering; the `Clock`-placement and title-caser-partial notes; and reword the "matches `discover_root`" manual check to hold only where a `.jj`/`.git` marker exists with no shallower `.accelerator`.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: A disciplined extension of the proven 0178 hexagonal pattern — five crates with a clean acyclic DAG, kernel-only domains, injected ports that keep regex/time/process out of the domain, and coherent re-homing of the three enforcement mechanisms. The two pressure points are the shared `document` crate (which couples two previously-independent adapter stacks at the parse layer) and the `vcs`/`vcs-adapters` pair, positioned as a foundational convergence point yet scoped to leave a known root-detection blind spot untested.

**Findings**:
- 🟡 major (medium) — **Phase 4 design + fixtures**: root-detection blind spot for `.git`-as-file worktrees/submodules; marker semantics (existence vs is-directory) unspecified and no fixture. As a foundational primitive, the blind spot forces 0169 to correct rather than extend.
- 🔵 minor (high) — **Phase 1 value-model layering**: `document::Value` becomes a shared type under both adapter stacks; a corpus- or config-driven change now ripples across both. Keep it the pure YAML-scalar union; push shaping into each adapter's mapping.
- 🔵 minor (high) — **Implementation Approach**: three structurally-identical enums synced only by hand; add a cross-enum round-trip test so drift fails a test.
- 🔵 minor (medium) — **What We're NOT Doing / number policy**: two coexisting Rust parsers fork on number policy; point corpus fixtures at the visualiser's own inputs and record the fork as a 0168 delta.
- 🔵 suggestion (medium) — **Phase 2 §8**: `Clock` placement asymmetry vs the reasoning that justified the `vcs` split; scope-note it or colocate with infra.
- 🔵 suggestion (low) — **Phase 4 CommandProbe**: state subprocess failure semantics (missing binary / non-zero → `None`).

### Correctness

**Summary**: Unusually diligent about behavioural deltas (number policy, YAML tags, catch_unwind, extract_id fallback), grounded in parity oracles. But several consolidations and the VCS model contain concrete logic gaps that would produce wrong results as written: two width parsers with different missing-width defaults; the WorkItems slug arm dropping the regex→pure-numeric fallback; and a total `RepoFacts` that cannot represent the bare/no-repo state Phase 5 must blank.

**Findings**:
- 🟡 major (high) — width parsers collapse with different defaults (4 vs 0); pin both, note server-side deferral.
- 🟡 major (medium) — WorkItems slug drops the `or_else` pure fallback; restore + fixture.
- 🟡 major (medium) — `RepoFacts` total type can't represent bare/no-repo; make `facts` optional + define blank mapping.
- 🟡 major (medium) — FrontmatterState parity (non-mapping root, tags, null-root→empty) unspecified; specify + fixtures.
- 🔵 minor (medium) — `MAX_SCAN` cap consistency between capped `fence_offsets` and uncapped `split`/`parse`.
- 🔵 minor (medium) — empty/failed revision + marker-walk vs `discover_root` divergence (`.accelerator` stop, returns `start`).
- 🔵 suggestion (low) — big-int policy only covers `u64`; beyond-`u64` still widens to `Float`.

### Test Coverage

**Summary**: Test-conscious — test-first, ports the visualiser's ~163-test oracle, differential parity against live bash over a broad corpus, faked Clock/VCS ports, the 0178 bounded-time adversarial guard, and deny/pup regressions. Main gaps: external-tool suites can pass silently green (Rust no-skip), "port verbatim" collides with the deliberate number/tag changes, and the AC-6 single-source + 14-vs-13 doc-type reconciliation are under-specified about what is genuinely diff-testable.

**Findings**:
- 🟡 major (high) — external-tool tests pass silently green; assert presence + hard-fail (0178 bash-probe pattern).
- 🟡 major (high) — "port verbatim" contradicts number-policy/tag changes; call out edited assertions.
- 🟡 major (medium) — AC-6 single-source extraction under-specified; per-surface execute + non-empty/complete assertions.
- 🟡 major (medium) — parity corpus "14 variants" but Templates/PrDescriptions have no bash oracle; split diff-tested vs declared-value subsets.
- 🔵 minor (medium) — byte-for-byte body preservation only a manual spot-check; add automated `document` test.
- 🔵 minor (medium) — bounded-time guard lacks a named alias-expansion fixture.
- 🔵 minor (low) — scanner `match_end` vs capture-end not pinned by a targeted assertion.

### Code Quality

**Summary**: Disciplined, well-layered, high on testability (every impure seam a faked port); real duplications collapsed. Main tension is a proliferation of structurally-identical value enums with hand-written cross-mappings plus a stringly-typed error boundary on a newly-shared crate; a couple of pieces (a Serialize mirror with no in-scope consumer) reach ahead of demand.

**Findings**:
- 🟡 major (high) — four near-identical enums + `#[non_exhaustive]` wildcard silent-degradation; state no-wildcard invariant + round-trip conformance test; consider a shared newtype.
- 🔵 minor (medium) — hand-written `Serialize` mirror has no 0179 consumer (YAGNI); defer or justify + test.
- 🔵 minor (medium) — stringly-typed `Result<_, String>` at the shared `document` boundary; introduce typed `DocumentError`.
- 🔵 minor (low) — `CommandProbe` collapses every failure to blank revision with no diagnostic; log via `kernel::logging`.
- 🔵 suggestion (high) — title-caser consolidation only partial; make Success Criteria say so + anchor the 0168 retirement.

### Compatibility

**Summary**: Disciplined about not regressing shipped 0178 config read/write: Phase 1 keeps signatures intact, delegates to `document`, leans on the green config suites, and defers all visualiser changes to 0168. Two concerns: the described deny-ban-test update would break that test after the wrapper re-homes, and the hand-written `Serialize` shape doesn't match the SPA's current JSON. New `regex` pin is looser than convention.

**Findings**:
- 🟡 major (high) — deny clean-fixture rename mis-specified; rename the CLEAN fixture (`config-adapters` → `document`), leave the banned `config` fixture.
- 🟡 major (medium) — `Serialize` wire contract diverges from the SPA's `BTreeMap`/numeric-big-int shape (key order + big-int typing); record as a conscious 0168 decision.
- 🔵 minor (medium) — `regex = "1"` inline diverges from exact-pin discipline; use `[workspace.dependencies]` + `{ workspace = true }`.

### Portability

**Summary**: Largely portable-by-design for the stated targets: pure-Rust serde-saphyr (a musl/static win), std::path/fs, graceful VCS fall-through, and a conscious literal `+00:00` (not `%Z`). Main risks are environmental: the filename timestamp's timezone is unstated, and the shipped jj/git probe runs against ambient config/env without the isolation the bash oracle relies on.

**Findings**:
- 🟡 major (medium) — filename-timestamp timezone assumption unstated; specify host-local + real-clock parity assertion.
- 🟡 major (medium) — probe inherits ambient config/env without hygiene; add `--color=never`/`--no-pager`/`-c color.ui=false`/`GIT_DIR` scrub.
- 🔵 minor (medium) — clock time source unspecified / no time crate declared; prefer a pure-Rust time crate over shelling to `date`.
- 🔵 minor (low) — parity harness couples to repo-relative path + exec bit + bash; gate loudly, confirm never run from a published artifact.

### Standards

**Summary**: Highly convention-aware: mirrors the 0178 crate topology and pup/deny patterns, snake_case modules, British spelling, correct serde-saphyr re-home; introduces no new shell scripts (so exec-bit/SHELL_LIBRARIES/bash-3.2 not triggered). A handful of minor naming/dependency-declaration inconsistencies to tighten.

**Findings**:
- 🔵 minor (high) — `regex` declared inline vs `[workspace.dependencies]`.
- 🔵 minor (medium) — `document::Value` collides with existing `config::Value`; consider `document::Frontmatter`/`Yaml`.
- 🔵 minor (medium) — domain modules must use single-item `use crate::...;` imports for the whole-crate pup rule.
- 🔵 suggestion (low) — specify final `members` ordering (domain-before-adapters grouping).

### Safety

**Summary**: Keeps the dangerous surface narrow: the only shipped code touched is config read/render; `atomic_write`/`store.rs` untouched; risky persistence deferred to 0180; patcher/render land as pure byte-preserving fail-closed functions. The main data-loss guard (render failing closed on a malformed existing file) is load-bearing but only transitively pinned, not asserted in `document`'s own contract. Removing `catch_unwind` trades a proven fail-safe for a bet on serde-saphyr's panic-freedom.

**Findings**:
- 🟡 major (medium) — fail-closed re-parse not pinned in `document::render`'s contract; make it explicit + add the fail-closed test to Phase 1 criteria.
- 🔵 minor (medium) — removing `catch_unwind` drops the corpus-scan poison-file fail-safe; broaden adversarial coverage toward fuzz before 0168.
- 🔵 suggestion (low) — VCS subprocess probe has no timeout and blanks on any failure; add a wall-clock cap + distinguish bare from error.

## Re-Review (Pass 2) — 2026-07-11

**Verdict:** REVISE

The revision closed essentially every substantive pass-1 finding — all 15 majors
are resolved or deliberately deferred with a recorded hand-off, and every lens
credited the changes (Option-typed `RepoFacts`, hard-fail tool gating, the
diff-tested/declared-value corpus split, no-wildcard mappings + round-trip test,
env-hygienic VCS probe, the `document::Yaml` rename, the fail-closed render
contract). Verdict stays REVISE only because the finer second pass surfaced a
smaller set of tighter issues — several of them second-order consequences of the
pass-1 fixes rather than the original defects. These are close-out items, not a
re-litigation.

### Previously Identified Issues

- 🟡 **Compatibility** — deny ban-test mis-specified → **Resolved** (clean-fixture
  rename is now correct); residual below (its `Cargo.lock` entry).
- 🟡 **Correctness** — width-parser defaults → **Resolved** (0-default pinned; 4→0
  fold hand-off noted).
- 🟡 **Correctness** — WorkItems slug fallback → **Resolved** (`or_else` restored +
  fixture; now cited as a strength).
- 🟡 **Correctness** — `RepoFacts` totality → **Resolved** (`facts → Option`, None→blank).
- 🟡 **Correctness** — FrontmatterState parity → **Partially resolved** (root-shape
  rule added; the YAML-tag sub-rule is now self-contradictory — see New Issues).
- 🟡 **Test Coverage** — silent-green external tools → **Resolved** (hard-fail gating,
  credited); residual: AC-6 test's own tool gating unstated.
- 🟡 **Test Coverage** — "port verbatim" → **Resolved** (rewrite note added).
- 🟡 **Test Coverage** — AC-6 extraction → **Resolved** (execute-based, non-vacuous).
- 🟡 **Test Coverage** — 14-vs-13 corpus → **Resolved** (diff-tested/declared split).
- 🟡 **Code Quality** — four-enum wildcard drift → **Resolved** (no-`_` invariant +
  round-trip test).
- 🟡 **Compatibility** — Serialize wire divergence → **Resolved** (deferred + recorded
  0168 hand-off).
- 🟡 **Architecture** — `.git`-as-file blind spot → **Resolved** (existence-tested
  markers + worktree fixture).
- 🟡 **Portability** — filename-timestamp timezone → **Partially resolved** (local/UTC
  split stated; `time`-crate features + multithread mechanism newly flagged).
- 🟡 **Portability** — ambient VCS env → **Resolved** (`GIT_DIR` scrub + flags);
  residual: also scrub `JJ_CONFIG`.
- 🟡 **Safety** — fail-closed re-parse → **Resolved** (render contract + criterion);
  residual: the criterion wording is mis-scoped (see New Issues).
- **Cross-cutting VCS theme** → **Largely resolved** (failure→None, env hygiene,
  warn-logging, existence markers all landed); residual is the timeout-outcome
  specificity below.

### New Issues Introduced

- 🟡 **Correctness** — **YAML-tag rule is self-contradictory**: the FrontmatterState
  text says "a YAML tag → `Malformed`" but also "some `!!` tags resolve to base
  types" (which would be `Parsed`). The visualiser is uniformly fail-closed (any
  `Tagged` → Malformed). Verify serde-saphyr's `deserialize_any` tag behaviour and
  state one unambiguous rule. *(Introduced by the FrontmatterState edit.)*
- 🟡 **Compatibility** — **deny clean-fixture `Cargo.lock` rename omitted**: the ban
  test runs `cargo deny --frozen`; the fixture's `Cargo.lock` also pins
  `[[package]] name = "config-adapters"`, so a manifest-only rename makes
  `cargo metadata --frozen` error before bans evaluate. Add the lockfile entry to
  the Phase 1 §3 edit list.
- 🟡 **Portability / Test Coverage** — **host-local timestamp is under-specified and
  hard to test**: the workspace `time` pin lacks `formatting`/`local-offset`;
  `now_local()` errors in a multithreaded process (how `cargo test` and CI run);
  and the zone assertion is vacuous under `TZ=UTC` CI. State the feature additions
  + the explicit offset-resolution strategy, and drive the zone test through a
  controlled non-UTC `TZ` (or an injected offset), single-threaded/subprocess.
- 🟡 **Safety** — **VCS probe *timeout* outcome unstated**: the wall-clock cap is
  specified but a killed probe is neither a spawn failure nor a clean non-zero
  exit; pin timeout → `revision: None` (+ warn log), give the cap headroom, and add
  a fake-probe timeout-branch test.
- 🟡 **Safety** — **catch_unwind hand-off is about fault-isolation, not just fuzz
  breadth**: sharpen the 0168 note to require *either* proven panic-freedom *or*
  re-established per-file isolation (a boundary that maps a per-file panic/abort to
  Malformed and continues the walk) before a corpus-walking consumer ships.
- 🟡 **Standards** — **`document::Yaml` is flat (`Str`/`Seq`/`Map`) while `Node`/
  `FrontmatterValue` are nested (`Scalar(Scalar)`/`Sequence`/`Mapping`)**: the
  "structurally-identical" framing (and the round-trip test premised on it) doesn't
  hold. Either align `Yaml` to the nested shape/full-word variants, or drop the
  "identical" claim and state the flat shape is kept to minimise config-adapters
  churn.
- 🟡 **Architecture** — **transitional twin duplication is unguarded**: the canonical
  `canonical_digit_width`/`title_case_segment` copies have no conformance test
  binding the server-side twins to them (unlike the doc-type fact), so they can
  drift silently in the 0179→0168 window. Add a cheap cross-check test or record it
  as a tracked hand-off.
- 🔵 **Minors/suggestions** (worth folding in): `document::split` must slice the body
  verbatim (not inherit the visualiser's `trim_start_matches('\n')`) or the
  byte-for-byte body guarantee regresses on bodies starting with a blank line
  *(Correctness)*; add value-model assertions pinning the `i64`/`u64`/beyond-`u64`
  number boundaries *(Correctness/Test Coverage)*; `DocumentError` needs an
  emit/serialize variant for `render`'s third failure path *(Code Quality)*; add a
  `config-adapters`-observable over-cap fail-closed test + port the relocated
  `split` tests into `document` *(Test Coverage/Compatibility)*; restate the
  render success-criterion as "render returns Err, emits no output" and list
  store.rs's `a_write_against_a_malformed_file_fails_closed` as the end-to-end pin
  *(Safety)*; pin the `regex` version constraint explicitly *(Compatibility)*;
  narrow the single-item-`use crate::` note to production modules (test modules are
  pup-exempt) *(Standards)*; consider `Clock` in `kernel` rather than `corpus`
  *(Architecture, suggestion)*; a patcher-writer hand-off note (atomic write +
  fence/value-shape-only validation) for 0168/0173 *(Safety, suggestion)*.

### Assessment

The plan is in good shape and its architecture is sound — the pass-1 defects that
could have shipped wrong results are closed. The remaining items are close-out
precision work: one self-contradiction the FrontmatterState edit introduced, two
concrete build/test correctness gaps (the deny `Cargo.lock` entry and the `time`
feature/multithread specifics), a couple of VCS/parser safety statements to pin,
and a naming/framing cleanup for `document::Yaml`. None requires structural
rework; a focused revision pass over these eight tighter findings should reach
APPROVE. I'd prioritise the three concrete build/test correctness items (tag rule,
`Cargo.lock`, `time` features + testable zone) since those would otherwise fail an
implementer at the keyboard.

## Re-Review (Pass 3) — 2026-07-11

**Verdict:** REVISE

Every pass-2 finding is resolved. This pass earned its keep by catching one
genuinely important defect that the pass-2 edits *introduced* — flagged
independently by four lenses — plus a handful of second-order consequences of
those same edits. The plan's architecture remains sound; the trajectory is
converging, but this round found real implementation-blockers, so it is not yet
APPROVE.

### Previously Identified Issues (pass 2)

- 🟡 **Correctness** — YAML-tag self-contradiction → **Resolved** (now one
  fail-closed rule); residuals below (fallback mechanism, Null-in-Scalar ordering).
- 🟡 **Compatibility** — deny `Cargo.lock` rename → **Resolved** (verified correct
  and load-bearing for `--frozen`).
- 🟡 **Portability/Test Coverage** — timestamp testability → **Resolved**
  (controlled-`TZ`); new consequences below (format golden, multithread offset).
- 🟡 **Safety** — VCS timeout outcome → **Resolved** (timeout → `None` + test).
- 🟡 **Safety** — catch_unwind fault-isolation → **Resolved** (hand-off sharpened).
- 🟡 **Standards** — `document::Yaml` flat/nested → **Resolved** (now nested);
  residual: `Mapping` newtype mismatch below.
- 🟡 **Architecture** — twin drift-guard → **Resolved** (recorded 0168 hand-off).
- All pass-2 minors (verbatim body slice, number boundaries, `DocumentError::Emit`,
  over-cap test, render criterion, `regex="1"`, production-only import note, Clock
  reasoning, `value.rs` note, patcher hand-off, AC-6 gating, `JJ_CONFIG` scrub) →
  **Resolved**.

### New Issues Introduced

- 🔴 **Architecture + Compatibility** (high) — **`#[non_exhaustive]` on `Scalar`
  defeats the no-wildcard exhaustiveness guarantee.** The plan marks
  `document::Scalar` and `corpus::Scalar` `#[non_exhaustive]` (copied from `config`)
  yet claims the cross-crate mapping arms are wildcard-free so a new variant fails
  to compile. Rust *forces* a `_` arm when matching a `#[non_exhaustive]` enum from
  another crate — the shipped `config-adapters/document.rs:101`
  (`Node::Scalar(_) => Parsed::Null`) already proves it, the exact silent downgrade
  the design exists to prevent. Fix: drop `#[non_exhaustive]` on these
  workspace-internal (`publish = false`) `Scalar` enums, or replace the
  "fails-to-compile" claim with an explicit coverage test.
- 🔴 **Code Quality** (high) — **infallible `Clock` cannot express the mandated
  error path.** `now_utc_iso()`/`filename_timestamp()` return `String`, but Phase 5
  requires the clock to *error* (not fall back to UTC) when the local offset is
  unresolvable. Make the offset-sensitive surface fallible (`Result`, or a fallible
  `SystemClock::try_new()` that resolves the offset once).
- 🟡 **Code Quality** (medium) — **`vcs-adapters` declared "std only" but must
  `warn`-log.** Phase 4 §3 requires warn logging via `tracing`/`kernel::logging`;
  add `tracing = { workspace = true }` and drop the "std only" characterisation.
- 🟡 **Correctness** (medium) — **newline-less closing fence regresses shipped
  config.** The current `split` (`split_inclusive('\n')`) accepts `---\n…\n---`
  with no trailing newline; `fence_offsets` returns `Malformed` for it. Expressing
  `split` over `fence_offsets` flips such a config to `MalformedFrontmatter`. Decide
  and pin: accept the newline-less close (match current lenience) or document it as
  a deliberate change (like the 1 MiB cap), with a test.
- 🟡 **Compatibility + Portability** (medium) — **`time` `local-offset` is
  unsatisfiable by a library clock in a multithreaded process, and erroring diverges
  from the non-erroring bash `date`.** A library `SystemClock` can't guarantee
  single-thread init before threads spawn; `now_local()` then errors where bash
  degrades. Resolve the host-local timestamp via a short-lived single-threaded
  subprocess (as the tests already do), or pin the composition-root contract
  (resolve offset in `main()` before spawning) and record `tzdata`/`TZ` as a runtime
  prerequisite hand-off.
- 🟡 **Test Coverage** (high) — **filename-timestamp format not pinned to a
  golden.** The one field that distinguishes the three subsumed helpers
  (`_H-M-S` / date-only / `-HMMSS`) is unasserted; the reused contract checks only
  the ISO line. Add a golden per `FilenameTimestampFormat` variant against a fixed
  clock/`TZ`.
- 🟡 **Test Coverage** (medium) — **doc-type longest-dir-wins tiebreak unexercised.**
  One-path-per-dir fixtures never hit a path matching multiple configured prefixes,
  so the matcher's core logic passes vacuously. Add a nested-directory + exact-length-tie
  fixture on both crate and bash sides.

### Minor / suggestion follow-ups

- 🔵 **Correctness** — Null is a `Scalar`, so "Scalar root → Malformed" must match
  `Null` → empty-mapping *first*; a jj no-commit repo yields a real revision (build
  the no-commit fixture as git, narrow the parity wording to genuinely-bare); prefer
  a structural YAML-tag guard over a raw-text token scan (false-positive risk).
- 🔵 **Test Coverage** — assert `RepoFacts.name == basename(root)`; call out the
  review-suffix slug edges (internal `-review-` preserved, trailing `-review-N`,
  non-numeric → `None`) as required unit cases (no bash oracle).
- 🔵 **Standards** — give `document::Yaml` a `Mapping(Mapping)` newtype (not a bare
  `Vec`) to be genuinely 1:1 with `Node`/`FrontmatterValue`; put `DocumentError` in
  its own `error.rs` (mirroring `config::error`); optionally split the `vcs` types
  into modules.
- 🔵 **Safety** — the end-to-end fail-closed pin reuses `config-adapters`'
  unterminated-fence test; add a store-level fence-valid-but-YAML-invalid test for
  the *re-parse* branch; record that a consuming writer must surface (not swallow)
  the VCS probe `warn` so transient failure isn't silent blank provenance.
- 🔵 **Architecture** — add a convergence-ledger line for
  `config-adapters::discover_root` vs the new `vcs` marker-walk; consider a narrower
  instant+offset `Clock` port with pure formatting functions.
- 🔵 **Portability** — state the OS support matrix (macOS + Linux) explicitly.

### Assessment

The plan is architecturally sound and its data-safety posture is strong — safety
credited it with essentially no live-write hazard. But this pass surfaced a real,
high-value defect the pass-2 nested-`Scalar` edit introduced (`#[non_exhaustive]`
silently reintroducing the drift the design forbids) and two more inconsistencies
those edits created (infallible `Clock` vs error-path; `vcs-adapters` std-only vs
logging), plus the newline-less-fence regression and the filename-format golden
gap. These are worth one more focused revision — they'd bite an implementer. After
that batch the plan should reach APPROVE; note that further passes are now yielding
tighter, lower-severity refinements (a sign of convergence, not of hidden
structural problems).

## Verdict Update — APPROVE (2026-07-11)

The full pass-3 batch was applied to the plan: `#[non_exhaustive]` dropped from the
three internal `Scalar` enums (restoring the wildcard-free exhaustiveness guarantee);
`SystemClock::try_new()` made fallible with the host offset resolved via a
single-threaded subprocess; `vcs-adapters` given a `tracing` dependency; the
newline-less-closing-fence relaxation, filename-format goldens, doc-type tiebreak
fixture, structural YAML-tag guard, Null-first arm ordering, git no-commit fixture,
`RepoFacts.name` assertion, store-level re-parse fail-closed test, `bash-parity`
feature gating, OS-support matrix, and the `discover_root` / transient-provenance /
tzdata hand-offs all landed.

Every finding across the three passes is now either resolved in the plan text or
recorded as an explicit, deliberate **0168/0173 hand-off** (server fold + wire
`Serialize` + title-caser/width-parser twin retirement + `discover_root`
convergence) — none is an open defect in 0179's own scope. The plan is **APPROVE**d
for implementation; the plan's `status` advances to `ready`.

### Post-approval refinement (2026-07-11)

A hexagonal-placement refinement was applied after approval (not a review finding —
raised separately): the two **pure convention algorithms** the plan had sitting
adapter-side — the **doc-type inference matcher** and the **single-document
typed-linkage resolver** — moved into the `corpus` **domain** (Phase 2), leaving
`corpus-adapters` as the infra boundary + per-document assembler that *invokes* them.
This re-aligns Phase 3 with the very 0178 precedent the plan cites (`config` keeps
its precedence-resolution and path-walk domain-side, not in `config-adapters`); slug
was already domain (the Phase 3 wording had overstated the adapter's role); and a
placement note flags that the byte-pure `patcher` arguably belongs in `document`.
Verdict stays **APPROVE** — the change tightens the domain/adapter boundary without
altering scope, behaviour, or any success criterion.
