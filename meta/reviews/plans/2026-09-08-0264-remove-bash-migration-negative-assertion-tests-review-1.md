---
type: "plan-review"
id: "2026-09-08-0264-remove-bash-migration-negative-assertion-tests-review-1"
title: "Plan Review: Remove Bash-Migration Negative-Assertion Tests"
date: "2026-09-08T22:29:06+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "plan:2026-09-08-0264-remove-bash-migration-negative-assertion-tests"
target: "plan:2026-09-08-0264-remove-bash-migration-negative-assertion-tests"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["correctness", "test-coverage", "safety", "architecture", "code-quality", "documentation"]
review_number: 1
review_pass: 3
tags: ["testing", "cleanup", "bash-parity", "migration"]
last_updated: "2026-09-09T13:38:38+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Remove Bash-Migration Negative-Assertion Tests

**Verdict:** REVISE

The plan is a well-grounded subtractive change: edit boundaries, line
references, the delicate Phase 4 Python import-prune, and the phase-disjointness
claims all verified accurate against the live tree, and the guard-preservation
instinct (rewiring `build:cli:fixture-size` rather than deleting it) is right. It
fails on one point that both the correctness and architecture lenses reached
independently: the fixture-size rewire drops a `build:cli:*` task into a roll-up
that a `test_mise.py` regression guard proves must be homogeneous, and Phase 4
never touches that guard — so Phase 4 as written leaves the full suite red,
directly falsifying its own "every phase stays green and is independently
mergeable" invariant. Around that sit several majors: the README rewrite is
under-scoped, the "equivalent protection" framing for the deleted zero-spawn and
getconf guards is inaccurate, and the phase's verification commands cannot detect
the regression it introduces.

### Cross-Cutting Themes

- **Fixture-size rewire breaks the `test:integration` roll-up invariant** (flagged by: correctness, architecture) — Adding `build:cli:fixture-size` to the roll-up's `depends`, and deleting the two zero-spawn tasks, breaks three tests in `tests/unit/tasks/test_mise.py` (`_NO_LAUNCHER_NEEDED` and `_NOT_IN_INTEGRATION_ROLLUP` still name the deleted tasks; the rollup-composition equality rejects a non-`test:integration:*` member). Phase 4 never edits that file. This is the critical finding.
- **`test:integration:fixture-size` phantom task in verification** (flagged by: correctness, code-quality, safety) — Phase 4's success criterion names a task the plan never creates; the rewire adds a `depends` edge, not a new leaf. The clause is dead and the surviving disjunct proves the guard passes in isolation, not that it runs in the roll-up.
- **README rewrite under-scoped and leaves an orphan** (flagged by: documentation ×2, architecture) — The six line anchors miss most of the "Zero-spawn strong form" section (`tasks/README.md:218-288`), which documents deleted env-var contracts and sudo-shadowing machinery; the one surviving paragraph (the fixture-size guard) is left orphaned under a heading whose subject no longer exists, with no relocation guidance.
- **"Equivalent protection" claims are inaccurate** (flagged by: safety, test-coverage) — The plan frames the surviving cargo-pup rules and `bare_invocation` lint as covering what the deleted tests asserted. The reworded pup.ron comments themselves admit the deny is blind to inline `Command::new()`; the getconf and config-cluster properties have no static equivalent at all.
- **Verification criteria weaker than their stated intent** (flagged by: correctness, code-quality, test-coverage) — `build-system:check` runs no pytest so cannot catch the `test_mise.py` break; the parity/golden inventory glob misses `*_goldens.rs`; the Phase 5 grep uses BRE alternation that false-passes on macOS BSD grep.

### Tradeoff Analysis

- **Coverage retention vs. sanctioned cleanup**: The safety and test-coverage lenses want compensating controls for the zero-spawn inline-spawn class and the getconf/container-portability property. The work item explicitly signs off removing these guards. The tradeoff is legitimately the owner's to make — but the reviewers' point stands that the plan's prose should stop asserting *equivalence* where none exists, and the decision-maker should confirm the loss with the specific gap named. Recommendation: keep the removals if signed off, reword the plan to state plainly what protection is retired, and consider one cheap compensating control (a `no_awk`-style static scan or a narrowed pup rule) only if the inline-spawn class is judged reachable enough to matter.

### Findings

#### Critical

- 🔴 **Correctness + Architecture**: Fixture-size rewire breaks the `test_mise.py` roll-up invariant, leaving Phase 4 red
  **Location**: Phase 4, steps 4 & 5 (mise task deletion + fixture-size rewire)
  Deleting `test:integration:zero-spawn`/`:strong` and adding `build:cli:fixture-size` to the `test:integration` roll-up breaks three tests in `tests/unit/tasks/test_mise.py`, which Phase 4 never touches: a `KeyError` on the deleted tasks still listed in `_NO_LAUNCHER_NEEDED`, an integration-task set-equality failure, and the rollup-composition equality (`rollup | excluded == integration_tasks`) rejecting a non-`test:integration:*` member. `mise run test:unit` — and any full `mise run` — goes red, falsifying the plan's core mergeability claim.

#### Major

- 🟡 **Documentation**: README "Zero-spawn strong form" section documented far beyond the six enumerated anchors
  **Location**: Phase 4, step 8 (`tasks/README.md`)
  The retired suite is documented across `tasks/README.md:218-288`, not just `:223/:232/:235/:252`. Two unreferenced paragraphs (238-250, 257-278) describe deleted env-var contracts (`ACCELERATOR_ZERO_SPAWN_*`), sudo shadow/restore, and the `if: always()` backstop; the section heading and "Two mechanisms prove it" framing also become false. Applied literally, the change leaves ~50 lines describing a removed CI job.

- 🟡 **Documentation**: Surviving fixture-size paragraph left orphaned with no relocation guidance
  **Location**: Phase 4, step 8 (`tasks/README.md:280`)
  The `build:cli:fixture-size` paragraph is the sole documentation of a guard the plan deliberately keeps, and "the third guard" is defined relative to the two zero-spawn mechanisms being deleted. Rewording only line 280 leaves it dangling under a stale heading; the guard is also moving into the roll-up the surrounding section said it was excluded from.

- 🟡 **Safety + Test Coverage**: Zero-spawn inline-spawn protection lost, "equivalent protection" framing inaccurate
  **Location**: Phases 1 & 4 (pup.ron rewording / Desired End State)
  Deleting the runtime zero-spawn tests and the sudo-gated strong CI job leaves only the crate-wide `std::process` use-path deny — which the plan's own reworded comments admit is blind to inline `std::process::Command::new("git")`. Unlike the retired shell tests, this regression class stays reachable in adapters that still read live VCS state; a fully-qualified inline spawn would compile, pass pup, and pass the suite.

- 🟡 **Test Coverage**: Phase 3 getconf-guard removal leaves the container-portability property uncovered
  **Location**: Phase 3 (`the_probe_shells_out_to_nothing`)
  This test is the only guard that the start-time probe reads the clock tick via compiled-in `sysconf` rather than a `getconf` subprocess. `process-probe` has no pup rule and the `design_adapters_read_in_process` rule exempts the `process` module; the positive `_agrees_across_*` tests stay green on any CI host with getconf. A regression degrades the probe in exactly the distroless/static-musl containers it exists for, silently.

- 🟡 **Correctness**: Phase 4 verification commands cannot detect the regression it introduces
  **Location**: Phase 4, Success Criteria (Automated Verification)
  `mise run build-system:check` runs pyrefly + ruff only (no pytest) and `mise run check` is read-only; neither runs `test:unit:tasks`, the lane where the `test_mise.py` break surfaces. The regression passes every command Phase 4 lists and fails only in the full `mise run`.

#### Minor

- 🔵 **Correctness + Code Quality + Safety**: `test:integration:fixture-size` verification clause names a task the plan never creates
  **Location**: Phase 4, Success Criteria — fixture-size roll-up check
  The rewire adds a `depends` edge, not a `test:integration:fixture-size` leaf, so the first clause never resolves and the surviving disjunct proves only that the guard passes standalone — not that it runs in the roll-up, which is the phase's actual goal.

- 🔵 **Test Coverage**: `no_awk.rs` is a self-documented permanent guard, not migration scaffolding
  **Location**: Phase 2 (`cli/migrate-cli/tests/no_awk.rs`)
  The file calls itself "A permanent regression guard" and runs unconditionally; its scan is broader than the migrate-adapters use-path pup rule. Confirm the owner's sign-off explicitly covers it on the same basis as the Python guard, and frame the removal as a permanent-guard retirement.

- 🔵 **Test Coverage**: `bare_invocation` does not cover what `call_site_migration` asserted
  **Location**: Phase 5 (call-site-migration guard)
  `bare_invocation` forbids pathed/`bash`-wrapped launcher calls; `call_site_migration` forbids `scripts/config-` references and stray `--allow-legacy-layout` flags. After removal no static guard catches a reintroduced config call site. Fine if signed off — but the plan should not imply equivalence.

- 🔵 **Code Quality**: Phase 5 grep uses BRE `\|` alternation that false-passes on macOS
  **Location**: Phase 5, Success Criteria
  `! grep -rq 'call_site_migration\|call-site-migration' ...` degrades to a literal on BSD grep (the repo's stated macOS floor), so the negated check always passes regardless of leftover wiring. Use `grep -Erq` for consistency with the other phases.

- 🔵 **Code Quality + Test Coverage**: Parity/golden inventory glob misses `*_goldens.rs`
  **Location**: Phase 4 / Desired End State — inventory diff
  `git ls-files 'cli/**/*parity*.rs' 'cli/**/migration_*.rs'` does not match `*_goldens.rs` or shared golden fixtures, so the criterion overstates its "parity/golden" coverage. Low risk here (no goldens deleted), but the proof is narrower than claimed.

- 🔵 **Code Quality**: Phase 4 per-phase test criterion under-exercises the `--all-features` CI config
  **Location**: Phase 4, Success Criteria
  Removing the `bash-parity` feature is best exercised by `test:unit:cli` (`--workspace --all-features`), which no Phase 4 criterion or `check` runs. The removal is in fact clean under `--all-features`, so the risk is latent — but the phase's green gate does not prove it.

- 🔵 **Architecture**: Roll-up gains a release-profile build, diverging from its dev-build members and the zero-spawn exclusion precedent
  **Location**: Phase 4, step 5 / Implementation Approach
  Every other `test:integration` member compiles dev artifacts; `build:cli:fixture-size` runs `cargo build --release`. `test:integration:zero-spawn` was deliberately kept out of the roll-up for this same cost reason (`test_mise.py:179`). The rewire adds a release compile to every bare `mise run` and mixes two profiles in one roll-up.

- 🔵 **Safety**: Rewired fixture-size guard now runs on the macOS leg it was never validated on
  **Location**: Phase 4, step 5
  The guard previously ran only on ubuntu inside `check-zero-spawn`. The roll-up runs on ubuntu + macos, so the gix/jj-lib link-ratio floor executes against Mach-O for the first time and may false-fail or pass vacuously. Confirm `mise run build:cli:fixture-size` passes on macos-latest, or scope the lane to the ubuntu leg.

### Strengths

- ✅ Edit boundaries are exceptionally precise and independently verified: both pup.ron comment diffs reproduce the exact source, the Cargo.toml block ranges (12-18, 20-25, 40-44), the mise.toml references (349-357, 579-582, 653), and the workflow job span (355-409, ending before 411) all match the live tree with no drift.
- ✅ The Phase 4 Python import-prune — the plan's most delicate non-pure-delete edit — is provably correct: every "keep" (`os`, `shlex`, `Path`, `Context`/`Exit`/`task`, `CARGO_TOML`, `_MANIFEST`, `.helpers`) has a verified surviving consumer, and every "drop" (`tempfile`, `Sequence`, `CLI_WORKSPACE_CARGO_TOML`) is used only by removed code.
- ✅ The plan correctly identifies wiring the work item omitted — the `mise.toml:653` `lint:check` depends entry and the `_ABSOLUTE_VCS_PATHS` constant — and rightly refuses to delete `build:cli:fixture-size`, preserving the architecturally-meaningful gix/jj-lib static-linking floor rather than deleting a guard along with the scaffolding it rode on.
- ✅ Phase decomposition is genuinely disjoint where it claims: Phase 1 vs 4 edit different pup.ron blocks, Phase 4 vs 5 edit different mise.toml regions; Phase 4 correctly bundles the interdependent corpus-cluster deletions into one atomic PR so no CI-coverage window opens.
- ✅ Deleting the sudo-gated `check-zero-spawn` job removes a genuine operational hazard the job documents itself: a failed binary restore on a non-ephemeral runner would leave git/jj persistently shadowed.
- ✅ Verification leans on a before/after `git ls-files` inventory diff rather than trusting a green suite alone to prove the positive parity/migration set is untouched.

### Recommended Changes

1. **Add a Phase 4 step to update `tests/unit/tasks/test_mise.py`** (addresses: the critical rewire/roll-up finding). Remove the two zero-spawn entries from `_NO_LAUNCHER_NEEDED` and `_NOT_IN_INTEGRATION_ROLLUP`, and reconcile `test_every_integration_task_is_in_the_rollup_or_excluded_with_a_reason` with the decision to place a `build:*` task in the roll-up. Prefer the architecture lens's route: introduce a `test:integration:fixture-size` leaf that runs the same check, declare its launcher need, add *that* to the roll-up, and update the exclusion sets — this keeps the namespace homogeneous, satisfies the guard, and makes the plan's existing `test:integration:fixture-size` verification clause real.

2. **Add `mise run test:unit:tasks` (or `test:unit`) to Phase 4's automated verification** (addresses: Phase 4 verification cannot detect the regression). The current commands run no pytest; the mise-topology guard must be exercised inside the phase's own gate.

3. **Rescope Phase 4 step 8 to rewrite the whole "Zero-spawn strong form" section** (`tasks/README.md:218-288`) (addresses: README under-scoped; orphaned fixture-size paragraph). Enumerate the machinery paragraphs (238-250, 257-278) and the section heading explicitly, and specify where the fixture-size guard's prose is rehomed — a standalone subsection framed around the gix/jj-lib static-linking floor and its new roll-up membership, not "the third guard".

4. **Reword the plan's coverage claims to drop the "equivalent protection" framing** (addresses: inaccurate equivalence; getconf gap; no_awk and bare_invocation findings). State plainly what each removal retires: the inline-`Command::new()` class for the adapters, the getconf/container-portability property for the probe, and the `scripts/config-`/legacy-flag static checks. Confirm the owner's sign-off covers `no_awk.rs` explicitly. Decide — and record — whether any cheap compensating control (a static scan or a narrowed pup rule) is warranted, or whether the loss is accepted.

5. **Fix the weak verification commands** (addresses: phantom task clause; macOS grep; inventory glob; `--all-features` gap). Drop the non-existent `test:integration:fixture-size` clause (or make it real per change 1); use `grep -Erq` in Phase 5; extend the inventory glob to `*golden*.rs` or soften the claim; add a `--all-features` nextest to Phase 4.

6. **Confirm `build:cli:fixture-size` passes on macos-latest** before relying on the roll-up membership, or scope the lane to the ubuntu leg (addresses: macOS-leg validation; dual-profile roll-up cost). Record the release-profile-in-roll-up cost tradeoff, given the zero-spawn exclusion precedent.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan's deletion logic is largely sound — the Phase 4 import-prune is correct, all cited line references are accurate against the working copy, and the phases are textually disjoint as claimed. But Phase 4 has a critical gap: it deletes two mise tasks and mutates the `test:integration` roll-up without touching `tests/unit/tasks/test_mise.py`, a regression guard that hard-references those tasks and enforces an invariant the rewire violates, leaving the full suite red.

**Strengths**:
- The Phase 4 step 3 import-prune is provably correct: `tempfile`, `Sequence`, and `CLI_WORKSPACE_CARGO_TOML` are used only inside the deleted functions; `os`/`shlex`/`Path`/`CARGO_TOML`/`_MANIFEST`/`.helpers` remain live in surviving tasks.
- The plan correctly adds `_ABSOLUTE_VCS_PATHS` to the removal set (the work item omitted it); it is used only in the deleted `_resolve_vcs_binaries`.
- All line references verified accurate with no drift: Cargo.toml blocks, mise.toml 349-357/579-582/653, workflow 355-409 and needs entry at 617, both pup.ron blocks.
- Phase disjointness is real (pup.ron 283-287 vs 324-328; mise.toml 349-357/393-404 vs 579-582/653).
- The `corpus-adapters-fixture` bin removal is genuinely clean — only `zero_spawn.rs:211` consumes it via `CARGO_BIN_EXE`.

**Findings**:
- 🔴 critical (high) — Phase 4, steps 4 & 5: Phase 4 leaves surviving references to deleted mise tasks and violates a guarded roll-up invariant in `test_mise.py`. Three tests break — `test_task_needing_no_launcher_omits_the_build_edge` (KeyError via `_NO_LAUNCHER_NEEDED`), `test_every_integration_task_declares_its_launcher_need` (set inequality), and `test_every_integration_task_is_in_the_rollup_or_excluded_with_a_reason` (`rollup | excluded == integration_tasks` broken by a non-`test:integration:*` member). Leaves `mise run test:unit` red.
- 🟡 major (high) — Phase 4 Success Criteria: verification commands cannot detect the `test_mise.py` regression. `build-system:check` runs pyrefly + ruff only; `check` is read-only; neither runs `test:unit:tasks`.
- 🔵 minor (high) — Phase 4 Success Criteria: `mise run test:integration:fixture-size` names a task the plan never creates; the first clause always fails to resolve.

### Test Coverage

**Summary**: Removes four negative-assertion tests plus a Python guard and its CI suite. Most deletion is genuinely safe, but the plan overstates "equivalent protection" in three places where the deleted tests covered a strictly larger surface than what remains — most seriously the start-time probe's getconf guard, which has no pup equivalent and which the surviving positive tests cannot catch.

**Strengths**:
- Each phase carries a green-suite gate and the phases are textually disjoint.
- The out-of-scope guards (SURVIVING_SHELL_SOURCES, bash-3.2 floor, bare_invocation, cargo-pup rules, positive parity/golden tests) are explicitly enumerated, reducing over-delete risk.
- The orphaned `build:cli:fixture-size` guard is rewired with a CI-log manual verification.
- A before/after `git ls-files` inventory diff is used rather than trusting a green suite alone.

**Findings**:
- 🟡 major (high) — Phase 3: removing the start-time probe's getconf guard leaves the container-portability property with no coverage. `process-probe` has no pup rule; `design_adapters_read_in_process` exempts the `process` module; positive tests stay green on any host with getconf.
- 🟡 major (high) — Phases 1 & 4: deleting the zero-spawn tests leaves the inline `Command::new()` blind spot the pup rules cannot catch. The reworded comments themselves state the deny catches only use-path imports.
- 🔵 minor (high) — Phase 2: `no_awk.rs` is a self-documented permanent guard ("A permanent regression guard"), broader than the use-path pup rule; confirm sign-off covers it and reframe as permanent-guard retirement.
- 🔵 minor (medium) — Phase 5: `bare_invocation` does not cover what `call_site_migration` asserted (`scripts/config-` references, `--allow-legacy-layout`); don't imply equivalence.
- 🔵 suggestion (medium) — Phase 4 / Desired End State: the inventory-diff glob misses `*_goldens.rs` and shared golden fixtures.

### Safety

**Summary**: Removes a cluster of anti-regression guards and rewires the orphaned fixture-size guard into `test:integration`. The rewire genuinely preserves CI execution (verified `test:integration` runs on both matrix legs). The material concern is that the zero-spawn runtime guards and the unique sudo-gated strong job are deleted with no compensating control for the inline-`Command::new()` blind spot the plan's own comments admit the surviving pup deny does not catch.

**Strengths**:
- The rewire is verified sound: `test-integration` runs `mise run test:integration` on ubuntu + macos, so the guard's CI execution survives (on more platforms than before).
- Fully reversible — touches only tests, manifests, task definitions, and CI config.
- Deleting the sudo-gated `check-zero-spawn` job removes a documented operational hazard (persistent git/jj shadowing on a failed restore).
- Phase 4 correctly bundles the interdependent deletions into one atomic PR.

**Findings**:
- 🟡 major (medium) — Phase 4 / Phase 1&4 pup.ron rewording: zero-spawn regression protection lost with no compensating control; the inline fully-qualified spawn class stays reachable in code that reads live VCS state. Suggests a `no_awk`-style static scan or an extended pup rule.
- 🔵 minor (medium) — Phase 4 step 5: rewired fixture-size guard now runs on the macOS leg never validated; Mach-O link ratio may false-fail or pass vacuously. Confirm on macos-latest or scope to ubuntu.
- 🔵 suggestion (high) — Phase 4 Success Criteria: criterion references `test:integration:fixture-size`, a task the rewire does not create.

### Architecture

**Summary**: Sound subtractive evolution — retires dead negative-assertion tests and a whole dedicated CI suite while correctly refusing to delete the one guard that protects kept surface. The five-phase split is well-reasoned. The one architecturally weak decision is the fixture-size rewire: dropping a `build:cli:*` task into the `test:integration` roll-up fights the roll-up's explicitly-guarded homogeneous taxonomy and leaves the guard's documentation orphaned.

**Strengths**:
- Net structural improvement: collapses an entire single-purpose CI surface to nothing, reducing build-system coupling.
- Correctly preserves the gix/jj-lib static-linking floor rather than deleting a guard along with its scaffolding.
- Phase decomposition is disjoint where it claims to be.
- Phase 4 is cohesive by single responsibility, with the one atomic sub-coupling (handing fixture-size CI coverage to a surviving lane before deleting the old job) kept in one PR.

**Findings**:
- 🔴 major (high) — Phase 4 step 5 / Implementation Approach: the rewire injects a `build:cli:*` task into a guarded, homogeneous `test:integration` roll-up. `test_mise.py` derives `_integration_tasks` from the `test:integration:` prefix (line 105) and asserts `set(rollup_depends) | _NOT_IN_INTEGRATION_ROLLUP == _integration_tasks` (line 294); a foreign-prefixed member fails that equality, and the deleted tasks are still named in `_NOT_IN_INTEGRATION_ROLLUP`/`_NO_LAUNCHER_NEEDED`. Recommends introducing a `test:integration:fixture-size` leaf. (Same underlying issue as the correctness critical.)
- 🔵 minor (medium) — Phase 4 step 8: the fixture-size guard's conceptual home dissolves when the zero-spawn section is gutted; relocate its rationale to a standalone subsection about the static-linking floor.
- 🔵 minor (medium) — Implementation Approach / Performance: the roll-up gains a release-profile build, diverging from its dev-build members and from the zero-spawn exclusion precedent (`test_mise.py:179`); a release compile is added to every bare `mise run`.

### Code Quality

**Summary**: A well-structured, deletion-heavy plan whose edit boundaries are precise and verified — the pup.ron diffs, the Cargo.toml ranges, the import-prune reasoning, the mise.toml references, and the workflow span all match the live tree. The weaknesses are in the success criteria rather than the edits: one check names a task that will never exist and doesn't prove the rewire, one grep uses BRE alternation that can false-pass, and the inventory check is narrower than its stated intent.

**Strengths**:
- Edit boundaries exceptionally precise and independently verifiable (pup.ron diffs, Cargo.toml 12-18/20-25/40-44, workflow 355-409 ending before 411).
- The integration.py import-prune reasoning is fully correct and gated on `build-system:check` (ruff + pyrefly).
- Correctly enumerates the two mise.toml wiring points the work item omitted (579-582 task, 653 depends) and adds `_ABSOLUTE_VCS_PATHS`.
- Phase disjointness is real and correctly argued.

**Findings**:
- 🔵 minor (high) — Phase 4 Success Criteria: the fixture-size criterion names a non-existent task and does not prove the rewire; suggests asserting the guard appears in the resolved dependency graph.
- 🔵 minor (medium) — Phase 5 Success Criteria: `grep -rq 'call_site_migration\|call-site-migration'` uses BRE `\|`, which on BSD grep degrades to a literal and always passes; use `grep -Erq`.
- 🔵 minor (medium) — Phase 4 Success Criteria: the per-phase test criterion (bare `cargo nextest -p corpus-adapters`) under-exercises the `--workspace --all-features` config where a feature-removal break would surface; add `test:unit:cli`.
- 🔵 minor (medium) — Phase 4 / Desired End State: the inventory-diff glob misses `*_goldens.rs`, overstating its coverage.

### Documentation

**Summary**: The plan correctly identifies that `tasks/README.md` and the two pup.ron comments carry prose references to the retired suite, and its pup.ron rewording diffs match the live file exactly. But its README guidance is scoped by six line anchors when the retired suite is documented across a whole ~70-line section (218-288); applied literally, the change would leave the canonical task-tree doc describing a deleted CI job, deleted env-var contracts, and removed sudo-shadowing machinery, and gives no guidance for rehoming the one surviving paragraph.

**Strengths**:
- The pup.ron rewording diffs (283-287, 324-328) match the live file exactly and drop only the stale runtime-complement clause.
- The `check-zero-spawn` CI-job → local-command table row (README:724) and the `main.yml` needs entry are correctly identified.
- No live reference to the retired surface leaks into the CLAUDE.md files, the sub-binary checklist, or the "Surviving thin shell" text; remaining hits are historical meta/ prose.

**Findings**:
- 🔴 major (high) — Phase 4 step 8: the README zero-spawn section is documented well beyond the six enumerated anchors. The target-resolution/harness-contract paragraph (238-250) and the gate/containment paragraph (257-278) — naming `ACCELERATOR_ZERO_SPAWN_*`, the sudo shadow/restore, the `if: always()` backstop — plus the heading and "Two mechanisms prove it" framing (220-221, 253) all become false. Rescope to the whole section (218-288).
- 🔵 major (high) — Phase 4 step 8 (README:280): the surviving fixture-size paragraph is left orphaned under a to-be-deleted section with no relocation guidance; "the third guard" ordinal becomes meaningless and the guard is moving into a roll-up the section said it was excluded from. Specify where the prose is rehomed and reframe as a standalone linker-drop guard.

## Re-Review (Pass 2) — 2026-09-08

**Verdict:** REVISE

The revised plan resolves **every** finding from pass 1: correctness independently
verified the `test_mise.py` reconciliation is arithmetically sound (13 integration
tasks; rollup 9 ∪ excluded 4, disjoint; all guard assertions green), and
architecture endorsed the `test:integration:fixture-size` leaf as the correct fix
for the critical roll-up-invariant break. The documentation majors, the
verification-strategy majors, and all seven minors are confirmed fixed. What
remains is a set of new, lower-stakes refinements — almost all of them refining
the very edits pass 1 prompted (the leaf, the `test_mise.py` step, and the new
"Coverage Retired" section), none blocking. The plan is now structurally sound and
implementable; the verdict stays REVISE only because the new refinements clear the
configured major-count threshold, not because any structural defect survives.

### Previously Identified Issues

- 🔴 **Correctness + Architecture**: Rewire breaks the `test_mise.py` roll-up invariant — **Resolved**. The leaf approach + step-6 reconciliation verified sound against every guard assertion; `build:cli:fixture-size` (self-contained in `build.py`) executes correctly via the depends-only leaf.
- 🟡 **Documentation**: README section under-scoped beyond six anchors — **Resolved**. Step 9 now enumerates the whole 218-288 section paragraph by paragraph.
- 🟡 **Documentation**: fixture-size paragraph orphaned — **Resolved**. Rehomed into a standalone subsection reframed away from "the third guard".
- 🟡 **Safety + Test Coverage**: inline `Command::new()` blind spot / "equivalent protection" inaccurate — **Resolved**. The new "Coverage Retired (Accepted)" section records the loss honestly under owner sign-off.
- 🟡 **Test Coverage**: Phase 3 getconf gap — **Resolved (documented)**. In the Coverage Retired section.
- 🟡 **Correctness**: Phase 4 verification could not detect the regression — **Resolved**. `test:unit:tasks` added to the criteria.
- 🔵 **Correctness/Code Quality/Safety**: phantom `test:integration:fixture-size` clause — **Resolved**. The task now exists; the clause is real.
- 🔵 **Test Coverage**: `no_awk.rs` permanent-guard framing — **Resolved (documented)**.
- 🔵 **Test Coverage**: `bare_invocation` ≠ `call_site_migration` — **Resolved (documented)**.
- 🔵 **Code Quality**: Phase 5 grep BRE alternation — **Resolved**. Now `grep -rEq`.
- 🔵 **Code Quality/Test Coverage**: inventory glob missed goldens — **Resolved**. `*golden*.rs` added; catches the nine real files.
- 🔵 **Code Quality**: `--all-features` under-exercised — **Resolved**. `test:unit:cli` added.
- 🔵 **Architecture**: release-profile roll-up divergence — **Partially resolved**. The tradeoff is now stated, but see the new cost-baseline finding.
- 🔵 **Safety**: macOS-leg validation — **Partially resolved**. A macOS check + fallback are added, but the fallback mechanism is under-specified (new finding).

### New Issues Introduced

- 🟡 major (high) — **Test Coverage** (Coverage Retired, bullet 1): the section under-documents corpus-adapters. It has **no cargo-pup rule at all**, so its metadata-read path drops to *zero* static coverage — not the use-path floor the bullet implies for "the zero-spawn adapters". Add corpus-adapters as a distinct entry.
- 🟡 major (medium) — **Safety** (Phase 4 steps 5-6): the runless pass-through leaf's protective value is one `depends` edge that nothing pins. If that edge is later dropped, the leaf passes vacuously and the guard fails open silently. Add a `_transitive_depends` assertion in `test_mise.py` that `build:cli:fixture-size` is reachable from the leaf.
- 🟡 major (medium) — **Architecture** (Fixture-size rewire decision): the release-compile cost is reasoned against the CI-only `check-zero-spawn` baseline, but the leaf lands the `--release` compile on the local `mise run` "done" path where it had no predecessor — a net addition, not a replacement. Re-baseline the argument for the local loop.
- 🟡 major (medium) — **Architecture** (steps 5-6): the leaf satisfies the guard's *nominal* homogeneity (name prefix) while introducing build-profile heterogeneity the guard cannot see. Record the build-linkage intent in the `test_mise.py` reason string so the classification captures intent, not only launcher-need.
- 🔵 minor (medium) — **Documentation + Correctness** (step 6): removing only the dict entries leaves the stale rationale comment blocks above them (`test_mise.py:178-183`, `186-189`) naming `check-zero-spawn` and the shadow contract — one would misattribute to the adjacent `design-automation` entry. Remove the comment blocks with the entries.
- 🔵 minor/suggestion (low-medium) — **Safety + Correctness** (macOS fallback): "scope the leaf to the ubuntu leg" has no concrete mechanism — a roll-up member is OS-agnostic. Specify the real shape (a platform check in `cli_fixture_size_check`, or a dedicated ubuntu-only step).
- 🔵 minor (medium) — **Architecture** (step 9): the README rehome could drop the still-live cross-compile darwin-gating rationale (musl absolute floor; darwin's ~9% margin kept off `prerelease:prepare`'s critical path). Have the new subsection preserve both the host-native ratio role and the cross-compile role.
- 🔵 minor/suggestion (low-medium) — **Code Quality** (step 5): the leaf reads as a redundant depends-only shim; surface its invariant-satisfying purpose at the task site, and give it a distinct description rather than duplicating `build:cli:fixture-size`'s.
- 🔵 suggestion (low) — **Test Coverage** (Coverage Retired, bullets 2-3): broaden the process-probe bullet to "any subprocess in process-probe" (getconf as the example), and note `migrate`/`migrate-cli` have no zero-spawn pup rule at all.
- 🔵 suggestion (medium) — **Architecture** (Phase 4): optionally split Phase 4 into 4a (add leaf + roll-up membership + `_NO_LAUNCHER_NEEDED` entry — additive, green) and 4b (delete the zero-spawn cluster), one concern per PR with no red state.
- 🔵 suggestion (low) — **Code Quality** (Phase 4 criteria): note the manifest-absence grep is a smoke check; well-formedness is proven by the adjacent compile gate.

### Assessment

The plan is in good shape and ready to implement once the two solid new items are folded in: the corpus-adapters entry in "Coverage Retired" (a factual completeness gap in a section whose whole purpose is a complete accounting) and the `_transitive_depends` edge assertion (closing the fail-open gap the new leaf introduces). The remaining items are cheap documentation and prose refinements — stale-comment removal in step 6, the cost re-baseline, the README cross-compile rationale, the leaf's self-documentation — worth doing in one more pass but not structural blockers. The Phase 4a/4b split is a genuine judgment call the owner should make, not a defect.

## Re-Review (Pass 3) — 2026-09-09

**Verdict:** COMMENT

Pass 2's findings are all folded in and verified: correctness independently traced
both intermediate states of the 4a/4b split (14 → 15 tasks after 4a, → 13 after 4b)
and confirmed every topology-guard assertion stays green; the fail-open edge is
closed by `test_fixture_size_leaf_reaches_the_guard`; the Coverage Retired
corpus-adapters entry is confirmed accurate (no `^corpus_adapters` pup rule
exists). No critical or correctness-breaking issue survives. The verdict drops to
COMMENT — the plan is implementable as-is. What pass 3 surfaced is a further round
of refinements, almost all on the pass-2 edits themselves; the two majors (both
about the macOS ratio floor the rewire introduced) have since been folded in by
scoping the guard to Linux.

### Previously Identified Issues (pass 2 → now)

- 🟡 **Test Coverage**: corpus-adapters under-documented — **Resolved**. Distinct zero-static-coverage entry added and verified against the pup rule inventory.
- 🟡 **Safety**: runless leaf could fail open — **Resolved**. `test_fixture_size_leaf_reaches_the_guard` pins the edge; a second assertion now pins the roll-up → guard half too.
- 🟡 **Architecture**: cost reasoned against CI baseline — **Resolved (then refined)**. Re-baselined for the local loop; pass 3 flagged the "cache-warm" claim as unsound (separate dev/release trees) — now corrected.
- 🟡 **Architecture**: build-profile heterogeneity intent — **Resolved**. Recorded in the shim comment and the `test_mise.py` reason.
- 🔵 **Documentation/Correctness**: stale comment blocks in step 6 — **Resolved**. Phase 4b step 5 removes them explicitly.
- 🔵 **Safety/Correctness**: macOS fallback mechanism — **Resolved (superseded)**. Now a proactive Linux-scoping of the guard, not a reactive fallback.
- 🔵 **Architecture**: README cross-compile rationale — **Resolved**. Step 8 preserves both roles.
- 🔵 **Code Quality**: leaf shim readability + description — **Resolved**. Distinct description + shim comment.
- 🔵 **Test Coverage**: process-probe / migrate breadth — **Resolved**. Bullets broadened.
- 🔵 **Code Quality**: manifest grep smoke-check — **Resolved**. Annotated.
- 🔵 **Architecture (suggestion)**: 4a/4b split — **Applied**. Phase 4 split into additive 4a and deletion 4b, ordered 4a→4b.

### New Issues Introduced (pass 3) — folded in

- 🟡 major (medium) — **Safety**: additive-first ordering ran a never-before-executed macOS ratio floor on every macOS local `mise run`. **Superseded by measurement** — see the Post-Review Note below. Initially folded in by scoping the guard to Linux; that mitigation was then reverted once the macOS ratio was measured at 32.6× (floor 3×), disproving the false-fail premise.
- 🟡 major (medium) — **Architecture**: the "cache-warm" justification is unsound (cargo compiles dev/release into separate trees). **Folded in**: cost prose corrected to a genuine cold-run release compile, with the CI-only-step alternative recorded as an explicit owner open-decision (roll-up leaf vs. dedicated CI step).
- 🔵 minor (high) — **Correctness/Safety**: the 4a→4b order was not a hard gate. **Folded in**: Phase 4b carries a pre-flight gate (confirm the leaf reaches the guard on `main` before deleting the strong job).
- 🔵 minor (medium) — **Code Quality**: the mise shim comment hard-coded `test_mise.py`. **Folded in**: reworded abstractly; triple-documentation trimmed.
- 🔵 minor (high) — **Architecture**: leaf falsifies the invariant's premise; precedent-erosion risk. **Partially addressed**: surfaced as the owner open-decision; the explicit build-linkage-guard category is left as a future option, not built now.
- 🔵 minor (low) — **Architecture**: only the leaf→guard half was pinned. **Folded in**: `test_fixture_size_guard_runs_in_the_integration_rollup` pins the roll-up→guard half.
- 🔵 minor (high) — **Documentation**: Key Discoveries README note understated/misattributed the surface. **Folded in**: reworded to the whole 218-288 section, `:280` separated as the kept guard.
- 🔵 minor (medium) — **Test Coverage**: vcs-adapters loss understated (no-spawn/no-degradation), `_SC_CLK_TCK` source not enumerated. **Folded in**: both Coverage Retired bullets broadened.
- 🔵 suggestion (medium) — **Documentation**: References line `:280` will go stale. **Folded in**: annotated as pre-change, points at the new subsection.

### Assessment

The plan is implementable and internally consistent. One item is left
deliberately open for the owner rather than decided: the guard's home — the
`test:integration` roll-up leaf (local + CI coverage, at the cost of roll-up
heterogeneity and a local release compile) versus a dedicated CI step (clean, but
no local-mirror coverage). The architecture precedent-erosion point is surfaced,
not engineered away. Further review passes are into diminishing returns — each has
refined the prior pass's own edits rather than the underlying plan. Recommend
stopping here and implementing.

## Post-Review Note — macOS ratio measured (2026-09-09)

The pass-3 safety and architecture lenses both worried that the fixture-size
ratio floor, running on `macos-latest` for the first time via the roll-up, might
false-fail against a Mach-O artefact — and the plan initially mitigated this by
scoping `cli_fixture_size_check` to Linux (a new `tasks/build.py` change in Phase
4a). That concern was **speculation, and it was tested rather than trusted.**
Building the two release fixtures on arm64 macOS (the `macos-latest` architecture)
gave `vcs-adapters-fixture` = 11,439,968 bytes against
`vcs-adapters-fixture-stub` = 350,912 bytes — a **32.6× ratio versus the 3×
floor**, over an order of magnitude of headroom. `build.py`'s own comment already
stated the ratio "has wide margin" and is "asserted everywhere"; the measurement
confirms it holds on Mach-O.

Consequently the Linux-scoping mitigation was **reverted**: Phase 4a step 3 (the
`build.py` / `test_build.py` change) is removed, and the guard runs unmodified and
green on both CI legs. This is a case of a reviewer's unverified worry driving a
real complexity addition that empirical measurement then retired — the simpler
plan (leaf in the roll-up, no `build.py` change) is correct. The only genuine,
OS-independent tradeoff that remains is the local release-compile cost (~27s warm),
which is the substance of the roll-up-leaf-vs-CI-step decision.

**Guard-home decision — settled (owner chose the roll-up leaf).** The guard is
homed in the `test:integration` roll-up (option A), chosen over a dedicated
CI-only step, on the strength of the repo's "`mise run` is the full local CI
mirror" rule — the regression should fail on a contributor's own machine, not
first in CI. The accepted cost is the local release compile and the roll-up
carrying one build-linkage guard; the follow-up guidance (evolve the topology
invariant to classify such guards if a second one ever needs the roll-up) is
recorded in the plan. No open items remain.

**Verdict: APPROVE.** With every finding across three passes resolved, the macOS
ratio verified empirically, and the guard-home decision settled, the plan is sound
and ready for implementation. The COMMENT verdict from pass 3 is upgraded to
APPROVE now that the sole remaining open item is closed.
