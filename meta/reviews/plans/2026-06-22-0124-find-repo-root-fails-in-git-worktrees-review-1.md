---
type: plan-review
id: "2026-06-22-0123-find-repo-root-fails-in-git-worktrees-review-1"
title: "Plan Review: find_repo_root fails in git worktrees (-d → -e)"
date: "2026-06-22T14:18:48+00:00"
author: Phil Helm
producer: review-plan
status: complete
parent: "plan:2026-06-22-0123-find-repo-root-fails-in-git-worktrees"
target: "plan:2026-06-22-0123-find-repo-root-fails-in-git-worktrees"
reviewer: Phil Helm
verdict: REVISE
lenses: [correctness, test-coverage, code-quality, architecture, portability]
review_number: 1
review_pass: 1
tags: [plan-review, scripts, vcs, git, worktree, conductor, bug]
last_updated: "2026-06-22T14:18:48+00:00"
last_updated_by: Phil Helm
schema_version: 1
---

## Plan Review: find_repo_root fails in git worktrees (-d → -e)

**Verdict:** REVISE

This is a high-quality, tightly-scoped bug-fix plan: the diagnosis is verified
against live source, the `-d`→`-e` change is genuinely monotonic, the `.jj`-WINS
ordering is correctly preserved, and it is driven red-green against an existing
fixture. The verdict is REVISE rather than APPROVE only because of a cluster of
low-cost-but-real issues — chiefly a **non-existent test runner** named in Phase
2's verification, two **stale comments** that the change leaves contradicting the
code, and the **unverified consumer-protection claim** for `work-item-file-dirty`
(the existing test bypasses the very function being fixed). None are structural;
all are addressable with small, targeted edits to the plan.

### Cross-Cutting Themes

- **The plan leaves comments that contradict the post-change code** (flagged by:
  code-quality, correctness) — the test block's "we deliberately do NOT lock in
  worktree behaviour … -d test skips it" comment (`test-vcs-detect.sh:443-446`)
  and the `vcs_mode` header comment (`vcs-common.sh:20-26`, which justifies the
  marker tests but never mentions that `.git` is a *file* in a worktree) both
  become stale once the change lands.

- **Phase 2's consumer-protection benefit is asserted but not automatically
  verified** (flagged by: test-coverage ×2) — the named runner `uv run pytest -k
  work_item_file_dirty` does not exist (these are shell tests), and the only
  existing automated coverage of `work-item-file-dirty.sh` drives it through
  `WORK_DIRTY_MODE_OVERRIDE`, which short-circuits the real `vcs_mode()` call. So
  the "no longer always-dirty in a worktree" outcome is verified only manually.

- **A deliberate deferral is named but not tracked — the same pattern that
  produced this bug** (flagged by: architecture ×2) — 0058 carved `find_repo_root`
  out of scope, that deferral was never recorded as actionable, and the gap
  resurfaced as 0123. This plan again defers the convergence of the two detection
  strategies in prose only, with no follow-up work item, ADR, or `relates_to`
  link to keep it from becoming permanent.

### Tradeoff Analysis

- **Minimalism (YAGNI) vs. drift safeguards**: code-quality praises the plan for
  correctly declining the larger delegation refactor; architecture warns that
  improving the legacy lexical strategy (rather than delegating to the 0058 probe
  layer) re-entrenches it at the most widely-sourced entry point, with no test
  asserting the two strategies agree. *Recommendation*: keep the minimal fix, but
  record the convergence as tracked debt and consider a cheap cross-strategy
  agreement assertion on the shared fixtures.

- **Scope: work item vs. plan**: the work item (0123) scopes only `find_repo_root`
  and lists "any broader refactor of the VCS detection helpers" as out of scope;
  the plan correctly also fixes `vcs_mode` because the acceptance criterion ("all
  ~30 downstream callers work unchanged in Conductor workspaces") is otherwise
  unmet. This is the right call — but the two artifacts now disagree on scope.
  *Recommendation*: amend the work item's scope/acceptance criteria, or note the
  expansion in the plan's Overview.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Test Coverage**: Phase 2 names a non-existent pytest runner for a shell-tested consumer
  **Location**: Phase 2, Success Criteria → Automated Verification
  `uv run pytest -k work_item_file_dirty -v` finds zero tests and exits 0 silently;
  `work-item-file-dirty.sh` is covered by `skills/work/scripts/test-work-item-scripts.sh`
  (run via `bash`). An implementer following the plan literally gets false confidence.

- 🟡 **Test Coverage**: vcs_mode→work-item-file-dirty integration is never exercised end-to-end
  **Location**: Phase 2, Desired End State / Manual Verification
  The only automated coverage of the consumer drives it through
  `WORK_DIRTY_MODE_OVERRIDE`/`WORK_DIRTY_STATUS_OVERRIDE`, which bypass the real
  `vcs_mode()`. The new unit assertion proves `vcs_mode` returns `git`, but nothing
  automated proves the "no longer always-dirty in a worktree" outcome; it is left to a manual step.

- 🟡 **Code Quality**: Stale "we deliberately do NOT lock in" comment must be deleted, not patched
  **Location**: Phase 1, Section 1: Regression test (RED first)
  Once worktree assertions are added and `-d`→`-e` lands, every clause of the
  `test-vcs-detect.sh:443-446` comment (incl. ".git is a file there, -d test skips it")
  is false. The plan says "update" the comment, which risks leaving the contradictory rationale in place.

- 🟡 **Code Quality**: vcs_mode header comment doesn't mention the file-vs-dir worktree case
  **Location**: Phase 2, Section 2: The fix (GREEN)
  The `vcs_mode` header (`vcs-common.sh:20-26`) documents the `.jj`-WINS ordering but
  predates the change and never explains why existence (`-e`) is now correct. A future
  reader could "tighten" `-e` back to `-d` believing the file case is a bug.

- 🟡 **Architecture**: Deferred convergence of the two detection strategies is named but not tracked
  **Location**: What We're NOT Doing (lines 99–110)
  The deferral lives only as a sentence, with no follow-up work item / ADR / `relates_to`.
  This is exactly the failure mode that produced 0123 (0058's untracked deferral). Without
  a tracked artifact, the two-strategy duplication becomes permanent-by-default.

- 🟡 **Architecture**: Patch re-entrenches the legacy strategy with no drift safeguard
  **Location**: Overview / What We're NOT Doing (lines 36–41, 101–104)
  Improving the legacy lexical layer at the single source of truth (~26 call sites) rather than
  delegating strengthens the incentive to keep extending it; nothing asserts the legacy helpers
  and `classify_checkout` agree, so they can drift to contradictory verdicts for the same checkout.

#### Minor

- 🔵 **Test Coverage**: "Colocated covered by existing tests" overstates current coverage
  **Location**: Phase 2, Key Edge Cases / Testing Strategy
  No existing test calls `vcs_mode` against a colocated checkout (the colocated cases test
  `classify_checkout`, or use the dirty-detection override). The `.jj`-WINS ordering under `-e`
  is not actually locked in at the function level. Suggest a direct `vcs_mode` colocated assertion.

- 🔵 **Test Coverage**: Loud-failure exit-code contract is verified only manually
  **Location**: Phase 1, Manual Verification
  The new assertions check `find_repo_root`'s return *value*, not the exit-0-under-`set -e`
  contract that was the actual production failure. A refactor that emitted the path but exited
  non-zero would pass the assertions yet re-break the visualiser. Suggest an explicit `RC=0` check.

- 🔵 **Test Coverage**: Red-green proof relies on manual reading, not the suite
  **Location**: Phase 1 & 2, Automated Verification
  `assert_eq` never `exit`s on failure (only tallies), so the RED proof depends on a human
  reading `FAIL:` output and hand-editing `-d`/`-e`; the test file carries no trace of the red phase.

- 🔵 **Code Quality**: Redundant repeated `make_git_linked_worktree` calls
  **Location**: Phase 1 & Phase 2 regression stanzas
  Phase 2 rebuilds the worktree even though `$FIXTURE_WORKTREE` is already in scope from Phase 1.
  Build once and reuse, or comment why the rebuild is intentional.

- 🔵 **Code Quality**: Two-phase ceremony for a one-character change (borderline, defensible)
  **Location**: Implementation Approach (two-phase split)
  Heavier process than the diff warrants, but justified by independent mergeability and distinct
  defect surfaces. Acceptable as-is; consider collapsing if the two land together.

- 🔵 **Architecture**: Work-item→plan scope expansion is sound but stretches traceability
  **Location**: Implementation Approach / Phase 2
  Adding `vcs_mode` is the right call, but the work item's "Out of scope" and "Acceptance Criteria"
  still describe the narrower fix. Amend the work item or note the expansion in the plan.

- 🔵 **Architecture**: Phase-independence claim partially eroded by a shared, mutated test stanza
  **Location**: Phase 1 vs Phase 2 / Testing Strategy
  Both phases edit the same regression block and rewrite the same comment, so "mergeable in either
  order" is weaker than stated. Use clearly-delimited sub-stanzas or soften the claim.

- 🔵 **Correctness**: Stray non-directory `.git`/`.jj` edge is dismissed without tracing its outcome
  **Location**: Key Edge Cases / What We're NOT Doing
  Under `-e` a symlink/file marker now *stops* the walk where `-d` walked past it — a real behavioural
  change, but correct-by-construction (presence is the only signal needed). Replace the "unrealistic"
  dismissal with that one-line reasoning.

- 🔵 **Correctness**: vcs_mode RED value depends on fixture isolation, not the function
  **Location**: Phase 2, Section 1: Regression test (RED first)
  `none` pre-fix is correct only because the fixture has no stray `.jj`/`.git` directory at the root.
  Add a one-line `# pre-fix: .git is a file, -d fails → none` comment so the red-green contract is self-documenting.

- 🔵 **Correctness**: Nested-subdir assertion is correct but depends on a canonicalised fixture path
  **Location**: Phase 1, Section 1: nested-subdir assertion
  Holds on macOS (`/var`→`/private/var`) only because `FIXTURE_WORKTREE` is already realpath'd. A
  confirmation, not a defect; no change required.

- 🔵 **Correctness**: End-to-end worktree dirty-detection relies on unstated relpath invariant
  **Location**: Desired End State / Phase 2 Manual Verification
  Only the `git` branch executes for a plain worktree (path-robust); the `jj` relpath logic is untouched.
  Worth stating that no relpath-stripping correctness is altered.

#### Suggestions

- 🔵 **Portability**: Make the `-e` rationale explicit (follows symlinks; broken-symlink `.git` is unsupported)
  **Location**: Key Discoveries / Phase 1 Section 2 ("-e is bash-3.2-safe")
  The consistency claim is correct for macOS/Linux; the only case where `-e` differs from `-d` in the
  *false* direction is a broken-symlink `.git`, which is not a supported repo layout. State it in one line.

- 🔵 **Portability**: Note that the new tests inherit (don't add) the fixture's environment assumptions
  **Location**: Phase 1, Section 1 / Testing Strategy
  `make_git_linked_worktree` needs a writable `$TMPDIR` and a working `git worktree add`; a build failure
  surfaces as a confusing test failure rather than a clean exit-77 skip. Already mitigated by the fixture's
  local git config; just confirm during implementation.

### Strengths

- ✅ The diagnosis is verified against live source: `find_repo_root:11` and `vcs_mode:29,31` both use `-d`, and `_ancestor_has_marker:49` already uses `-e` — a genuine in-file inconsistency, not a design choice.
- ✅ The `-d`→`-e` change is genuinely monotonic: `-e` matches a superset of `-d`, so no currently-working caller can regress; it only *adds* the worktree `.git`-file match.
- ✅ The `.jj`-WINS ordering in `vcs_mode` is correctly preserved under `-e` (false on a plain worktree → falls through to `.git`; a real `.jj` directory in a colocated checkout still wins).
- ✅ The test correctly targets `find_repo_root`'s `$PWD`-based contract via `(cd … && find_repo_root)` subshells, and the macOS `/var`→`/private/var` concern is neutralised by the fixture's `realpath` canonicalisation.
- ✅ `-e` is a POSIX primitive, bash-3.2-safe, and identical across macOS/Linux; the new tests reuse the existing fixture and the exit-77 skip preflight, adding no new tooling prerequisite.
- ✅ Reuses the existing `make_git_linked_worktree` fixture rather than inventing setup, with red-green-plus-revert proof, and correctly applies YAGNI by declining the larger delegation refactor.
- ✅ The two-phase split cleanly maps to the actual fault boundaries: a loud crash (`find_repo_root`/visualiser) vs. a silent correctness degradation (`vcs_mode`/work-item-dirty).

### Recommended Changes

1. **Fix the Phase 2 automated-verification runner** (addresses: "Phase 2 names a non-existent pytest runner")
   Replace `uv run pytest -k work_item_file_dirty -v` with `bash skills/work/scripts/test-work-item-scripts.sh` (or its `mise run` wrapper), and resolve the "confirm the exact runner" hedge before approval.

2. **Add real end-to-end worktree coverage for the consumer** (addresses: "integration never exercised end-to-end")
   Add a `work-item-file-dirty.sh` assertion in a `make_git_linked_worktree` fixture *without* the mode override: clean file → exit 1, modified file → exit 0. This is what genuinely locks in Phase 2's claimed benefit.

3. **Delete-and-replace the stale test-block comment; extend the `vcs_mode` header** (addresses: both stale-comment findings)
   Make the plan explicit that `test-vcs-detect.sh:443-446` is removed wholesale (no `-d`-skips-it clause survives), and add a one-line note to the `vcs_mode` (and ideally `find_repo_root`) header that `-e` is intentional because `.git` is a regular file in a linked worktree.

4. **Record the deferred convergence as tracked debt** (addresses: "deferral named but not tracked", "re-entrenches legacy strategy")
   Add a backlog work item ("migrate legacy lexical helpers to delegate to the 0058 probe layer") linked via `relates_to`, or a short ADR capturing the deliberate two-strategy coexistence. Optionally add a cheap cross-strategy agreement assertion on the shared fixtures.

5. **Reconcile work-item vs plan scope** (addresses: "scope expansion stretches traceability")
   Amend 0123's "Out of scope" / "Acceptance Criteria" to include `vcs_mode`, or note the expansion in the plan's Overview.

6. **Tidy the regression stanzas** (addresses: "redundant fixture calls", "phase-independence eroded by shared stanza")
   Build the worktree once and reuse `$FIXTURE_WORKTREE`; give each phase a clearly-delimited sub-stanza so the diffs don't overlap, or soften the "mergeable in either order" claim. Add the self-documenting RED-expectation comment for `vcs_mode`.

7. **Strengthen the exit-code regression guard** (addresses: "loud-failure exit-code contract verified only manually")
   Capture `find_repo_root`'s exit code in the worktree fixture (`… || RC=$?`; assert `RC=0`) so the exit-0-under-`set -e` contract — not just stdout — is regression-protected.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan's core logic is sound: the `-d`→`-e` change is genuinely
monotonic (a superset relation), the `.jj`-WINS ordering in `vcs_mode` is
correctly preserved under `-e` (false in a plain git worktree, a directory in a
colocated checkout), and the walk-up logic is unaffected. The regression tests
correctly exercise the `$PWD`-driven `find_repo_root` via `(cd … &&
find_repo_root)` subshells, and the macOS `/var`→`/private/var` concern is
neutralised because `FIXTURE_WORKTREE` is itself a realpath. Residual gaps are
minor: RED-state determinism for the `vcs_mode` assertion is presented as
self-evident, and the stray-symlink edge is dismissed rather than reasoned to its
(benign) outcome.

**Strengths**:
- Monotonicity/superset claim is correct — no working caller can regress.
- `.jj`-WINS preservation under `-e` verified correct for both worktree and colocated cases.
- Test correctly targets the `$PWD` contract via subshell `cd`.
- Nested-subdir walk-up is sound and macOS-symlink-robust via the realpath'd fixture.
- Phase independence correctly reasoned at the function level.

**Findings**:
- 🔵 minor / high — *vcs_mode RED assertion proves the right failure value but depends on fixture isolation* (Phase 2, Section 1). `none` pre-fix holds only because the fixture has no stray `.jj`/`.git` directory at the root; add a one-line comment documenting the RED expectation.
- 🔵 minor / medium — *Stray non-directory `.git`/`.jj` edge dismissed without tracing its outcome* (Key Edge Cases). Under `-e` a file/symlink marker now stops the walk; this is a real change but correct-by-construction. Replace the "unrealistic" dismissal with that reasoning.
- 🔵 minor / high — *Nested-subdir assertion returns the worktree root, depends on canonicalised FIXTURE_WORKTREE* (Phase 1, Section 1). Correct as written; a confirmation, not a defect.
- 🔵 minor / medium — *End-to-end worktree dirty-detection depends on relpath stripping the plan does not assert* (Desired End State / Phase 2). Only the `git` branch executes for a plain worktree; the `jj` relpath logic is untouched and already correct. Worth stating explicitly.

### Test Coverage

**Summary**: The plan correctly reuses the existing `make_git_linked_worktree`
fixture and adds targeted regression assertions for both `find_repo_root` (root +
nested subdir) and `vcs_mode`, locking in worktree behaviour against `-d`→`-e`
reversion. Two real gaps weaken confidence: Phase 2 names a non-existent pytest
runner for a pure shell-tested consumer, and the only automated coverage of the
`vcs_mode`-via-`work-item-file-dirty.sh` integration runs through the
`WORK_DIRTY_MODE_OVERRIDE` seam that bypasses the very function being fixed — so
the consumer-protection claim and the colocated non-regression are verified only
by manual steps.

**Strengths**:
- Reuses the already-validated `make_git_linked_worktree` fixture (constructs a real `.git` *file*).
- Adds a nested-subdirectory assertion covering walk-up, beyond the work item's minimum.
- Correctly retires the deliberately-open "room for a future fix" comment.
- Pairs each assertion with explicit RED-before-GREEN and revert-to-`-d` steps.
- Preserves and implicitly checks the `.jj`-WINS ordering rationale.

**Findings**:
- 🟡 major / high — *Phase 2 lists `uv run pytest -k work_item_file_dirty -v`, which finds zero tests and exits 0* (Phase 2 Automated Verification). The consumer is covered by `skills/work/scripts/test-work-item-scripts.sh` (lines 1551-1588), run via `bash`. Replace the command and resolve the hedge before approval.
- 🟡 major / high — *The vcs_mode→consumer path is stubbed, not exercised* (Phase 2 Desired End State / Manual Verification). `test-work-item-scripts.sh:1551-1588` drives the consumer through `WORK_DIRTY_MODE_OVERRIDE`/`WORK_DIRTY_STATUS_OVERRIDE`, short-circuiting `vcs_mode()` (`work-item-file-dirty.sh:41-46`). Add an assertion in a real worktree fixture without the override (clean → exit 1, modified → exit 0).
- 🔵 minor / medium — *Red-green proof relies on manual reading* (Phase 1 & 2 Automated Verification). `assert_eq` never `exit`s on failure (`test-helpers.sh:20-31`); the proof depends on a human reading `FAIL:` output and editing `-d`/`-e`. Capture the expected-FAIL evidence in the validation record.
- 🔵 minor / medium — *"Colocated covered by existing tests" overstates coverage* (Phase 2 Key Edge Cases). No existing test calls `vcs_mode` against a colocated checkout; the WINS ordering under `-e` is unasserted at the function level. Add a direct colocated `vcs_mode` → `jj` assertion.
- 🔵 minor / low — *Loud-failure exit-code contract verified only manually* (Phase 1 Manual Verification). The assertions check the return *value*, not exit-0-under-`set -e`. Add an explicit `RC=0` capture mirroring lines 286-288.

### Code Quality

**Summary**: A minimal, well-targeted one-character fix that brings
`find_repo_root`/`vcs_mode` in line with the existing `_ancestor_has_marker`
`-e` idiom, red-green test-driven against an existing fixture. The chief
code-quality risk is staleness: the `vcs_mode` header comment no longer explains
the file-vs-dir worktree case, and the plan does not commit to fully *deleting*
(vs patching) the "we do NOT lock in worktree behaviour" comment, leaving a real
chance of a self-contradictory comment surviving. The two-phase split is light
ceremony justified by independent mergeability, but the new stanzas duplicate a
fixture call.

**Strengths**:
- The fix mirrors the established in-file `-e` idiom (`_ancestor_has_marker:49`) — all three marker sites become consistent.
- Red-green with explicit revert-to-`-d` checks serves long-term maintainability.
- Reuses the existing fixture, keeping the suite DRY.
- Scope is proportional — correctly declines the larger delegation refactor (YAGNI).
- New stanzas follow the surrounding `echo`/`assert_eq` and `[NNNN]`-label conventions.

**Findings**:
- 🟡 major / high — *Stale "we deliberately do NOT lock in" comment must be removed, not patched* (Phase 1, Section 1). Once worktree assertions land, every clause of `test-vcs-detect.sh:443-446` is false. Make the plan explicit that the whole block is deleted and replaced, with no `-d`-skips-it clause surviving.
- 🟡 major / medium — *vcs_mode header comment doesn't mention `-e` and may read as inconsistent* (Phase 2, Section 2). The header (`vcs-common.sh:20-26`) documents WINS ordering but not why existence is now used. Add a one-line note that `.git` is a regular file in a linked worktree, keeping the terse style.
- 🔵 minor / high — *Redundant repeated `make_git_linked_worktree` calls* (Phase 1 & 2 stanzas). Phase 2 rebuilds the worktree though `$FIXTURE_WORKTREE` is already in scope. Build once and reuse, or comment the rebuild.
- 🔵 minor / medium — *Two-phase ceremony for a one-character change is borderline but defensible* (Implementation Approach). Justified by independent mergeability and distinct defect surfaces; acceptable as-is, consider collapsing if landed together.

### Architecture

**Summary**: The plan makes a well-reasoned, deliberately minimal structural
choice — a monotonic `-d`→`-e` patch on the legacy lexical helpers rather than
delegating to the 0058 authoritative-probe layer. The central tension (two
parallel detection strategies that can drift) is explicitly acknowledged in both
research and plan, and deferring convergence is defensible for a high-priority
bug fix. The main architectural gap is that the deferral is *named but not
recorded* as a tracked artifact, so the duplication risks becoming permanent, and
the patch re-entrenches the legacy strategy at the most widely-sourced entry
point.

**Strengths**:
- Explicitly names the core tradeoff in "What We're NOT Doing".
- Correctly characterises the change as monotonic / low-blast-radius across ~26 call sites.
- Two-phase decomposition aligns with the actual fault boundaries (loud crash vs silent degradation).
- Respects the functional-core boundary — `find_repo_root` only needs existence; topology-sensitive helpers keep explicit file-vs-dir checks.
- Carries the `.jj`-WINS / lagging-git-index invariant through the Phase 2 change.

**Findings**:
- 🟡 major / high — *Deferred convergence is named but not recorded as a tracked artifact* (What We're NOT Doing, 99-110). This is the exact failure mode that produced 0123 (0058's untracked deferral). Record a follow-up work item (`relates_to`) or a short ADR.
- 🟡 major / medium — *Patch re-entrenches the legacy strategy at the most widely-sourced entry point without a drift safeguard* (Overview / What We're NOT Doing, 36-41, 101-104). The legacy and probe layers can drift to contradictory verdicts; nothing asserts they agree. Add (or follow up to add) a cross-strategy agreement assertion on the shared fixtures.
- 🔵 minor / medium — *Scope expansion from work item to plan is sound but stretches the boundary* (Implementation Approach / Phase 2). Adding `vcs_mode` is correct, but 0123's "Out of scope"/"Acceptance Criteria" still describe the narrower fix. Amend the work item or note it in the plan.
- 🔵 minor / medium — *Claimed phase independence partially eroded by a shared, mutated test stanza* (Phase 1 vs Phase 2 / Testing Strategy). Both phases edit the same block and rewrite the same comment; "mergeable in either order" is weaker than stated. Use delimited sub-stanzas or soften the claim.

### Portability

**Summary**: A one-character shell fix plus regression tests in an existing,
already-portable harness. Low-risk and well-aligned with the bash-3.2 floor and
dual macOS/Linux targets: `-e` is a POSIX primitive supported identically across
bash 3.2, POSIX sh, macOS, and Linux; the new tests reuse the existing fixture,
the exit-77 skip preflight, and `realpath`-canonicalised paths that already defend
against macOS's `/var`→`/private/var` symlinking. No new environment coupling,
hardcoded paths, or vendor dependencies. Minor caveats around graceful
degradation when `git worktree add` is constrained and the implicit `-e`
symlink-following semantics.

**Strengths**:
- `-e` is POSIX-defined and behaves identically across all project targets; in-file precedent (`_ancestor_has_marker:49`) and the test file's own `[ -e … ]` usage (line 149) confirm it.
- New tests reuse the exit-77 skip-on-missing-tooling preflight (lines 11-16), so minimal CI runners skip cleanly.
- The fixture's `realpath` canonicalisation neutralises macOS tempdir symlinking; the nested-subdir assertion compares against the same realpath'd value.
- No hardcoded paths, env assumptions, locale/encoding deps, or vendor coupling introduced.

**Findings**:
- 🔵 minor / medium — *New fixture needs a writable tempdir and a working `git worktree add`* (Phase 1, Section 1 / Testing Strategy). A build failure surfaces as a confusing test failure rather than a clean exit-77 skip (the preflight checks tool presence, not whether `git worktree add` works). Already mitigated by the fixture's local git config; confirm `$TMPDIR` writability during implementation and note the inherited assumptions.
- 🔵 minor / medium — *"-e is bash-3.2-safe" leaves the symlink semantics implicit* (Key Discoveries / Phase 1 Section 2). `-e` follows symlinks and is true for any existing entry; the only divergent case is a broken-symlink `.git` (where `-e` is false), which is not a supported layout. State this in one line.
