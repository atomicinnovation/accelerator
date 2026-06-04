---
type: plan-review
id: "2026-06-04-migration-framework-upgrade-fixes-review-1"
title: "Plan Review: Migration Framework Upgrade-Failure Fixes"
date: "2026-06-04T16:21:41+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-04-migration-framework-upgrade-fixes"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [correctness, safety, compatibility, portability, test-coverage, architecture, code-quality]
review_number: 1
review_pass: 2
tags: [migrate, run-migrations, bash-compatibility, merge-move, shell-lint]
last_updated: "2026-06-04T17:00:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Migration Framework Upgrade-Failure Fixes

**Verdict:** REVISE

The plan is well-grounded, faithful to its research, and structurally sound:
it correctly diagnoses the abort-on-conflict policy as an idempotency-contract
violation, consolidates three divergent relocation policies behind one
primitive, restores ADR-0023's single-gate invariant, and sequences a
regression net (Phase 1) ahead of the behaviour-changing phases. However, the
review surfaced **two critical safety findings** and a cluster of **major**
findings concentrated on the new `merge_move` helper — its destructive
`rm -rf` path, swallowed failure modes, untested branches, and the fact that
nothing actually exercises it under bash 3.2 — plus the simultaneous removal of
the only no-VCS guard. These need addressing before implementation.

### Cross-Cutting Themes

- **`merge_move` failure handling & non-atomicity** (flagged by: correctness,
  safety, code-quality) — The helper does per-entry `mv` then `rmdir`, swallows
  `rmdir`/`mv`/`mkdir` failures (`2>/dev/null || true`), and has no rollback. On
  a partial failure it silently leaves a non-empty source un-removed while
  reporting success, directly undermining the convergence goal. It is the only
  non-atomic primitive in a file (`atomic-common.sh`) whose entire contract is
  atomicity.

- **No-VCS guard removal + destructive merge = unrecoverable data loss**
  (flagged by: safety [critical], compatibility) — The orchestrator's clean-tree
  gate silently *passes* when no VCS is present; 0004's deleted hard-fail is the
  *only* mechanism stopping a non-VCS user. Removing it in the same release that
  makes moves destructively overwrite leaves non-VCS consumers with irreversible
  overwrites and no recovery path — contradicting the "VCS revert is the safety
  net" premise, which presupposes VCS exists.

- **The bash-3.2 floor is asserted, not verified** (flagged by: portability,
  test-coverage, compatibility) — CI runs bash 5.x and the suites resolve to the
  dev's first-on-PATH bash (often Homebrew 5.x on macOS), so no automated layer
  runs the code under 3.2. The guard is a fixed-enumeration denylist (cannot
  catch future bash-4 constructs) plus a manual replay — the exact silent-pass
  failure mode that shipped `declare -A` can recur.

- **`merge_move` is the riskiest new code yet has the thinnest test coverage**
  (flagged by: test-coverage, correctness, safety) — No dedicated unit harness;
  coverage is indirect via the specific path-pairs each migration moves. The
  type-mismatch branch (file-over-dir / dir-over-file, which `rm -rf`s an entire
  destination subtree), symlink handling, empty-source, and the partial-merge
  residual are all unexercised.

- **Snapshot regeneration could baptise a real regression** (flagged by: safety,
  test-coverage) — Wholesale regen overwrites the `files.sha256` manifest and
  exit codes too, not just the stderr the change intends; manual diff review
  across a multi-file sha256 manifest is the only safeguard.

### Tradeoff Analysis

- **DRY/consolidation vs coupling & module purity**: Architecture credits the
  single shared `merge_move` as a real cohesion win, but also flags that it (a)
  doesn't belong in the atomicity-focused `atomic-common.sh` and (b) couples
  three frozen migrations to a living abstraction. Recommendation: keep the
  consolidation, but home the helper in a relocation-focused module and pin its
  contract with a dedicated test so the frozen-migration coupling is safe.

- **Helper simplicity vs robustness**: The research deliberately wanted a
  *simple* helper ("VCS is the safety net, so the helper stays simple"). Safety
  and code-quality want guards (`rm -rf` assertions, failure messages, audit
  trail). Recommendation: add the cheap defensive guards (non-empty `$dst`
  assertion, `--`, an overwrite audit line, surfacing non-empty-source rmdir
  failures) — these don't compromise simplicity and close the highest-blast-
  radius gaps.

### Findings

#### Critical

- 🔴 **Safety**: Deleting the only no-VCS guard while making moves destructive leaves non-VCS users with no recovery net
  **Location**: Phase 3 + "What We're NOT Doing" (delete `ACCELERATOR_MIGRATE_FORCE_NO_VCS`)
  The orchestrator's clean-tree gate only computes dirtiness when `.git`/`.jj` exists — with no VCS it passes silently, so 0004's deleted hard-fail is the *only* stop for a non-VCS user. Removing it as Phase 2 makes relocations destructively overwrite creates a path where a non-VCS user's files are irreversibly lost with no recovery whatsoever.

- 🔴 **Safety**: `merge_move`'s `rm -rf` has no guard against empty/unexpected `dst` or symlinked targets
  **Location**: Phase 2, Section 2: `merge_move` helper (`rm -rf "$dst"` branch)
  `rm -rf "$dst"` fires on any type mismatch; if `PROJECT_ROOT` resolved unexpectedly or a component is empty/relative it could target far more than intended, and `[ ! -d "$dst" ]` follows symlinks (a symlinked `.accelerator/` merges into the link target). This is the highest-blast-radius operation in the plan, run across every user's meta-directory.

#### Major

- 🟡 **Correctness**: `merge_move` silently abandons a non-empty source on partial merge
  **Location**: Phase 2, Section 2
  `rmdir "$src" 2>/dev/null || true` swallows the "directory not empty" signal — the postcondition-violation indicator — yet the helper returns 0 and the caller records the move as done. Unlike 0004's `_cleanup_legacy_parent` (which warns and lists residuals), this leaves stale legacy dirs invisibly behind.

- 🟡 **Correctness**: Source-wins type-mismatch branch obliterates an entire destination subtree, and the path is untested
  **Location**: Phase 2, Section 2 & Section 1 (tests)
  A source *file* colliding with a destination *directory* (or vice versa) `rm -rf`s the whole subtree — a far larger blast radius than the "leaf-file overwrite" rule the research describes. The planned red tests cover leaf-onto-leaf and nested-subdir merges but never the type-mismatch directions.

- 🟡 **Safety**: Irreversible source-wins overwrite has no audit trail or pre-change backup
  **Location**: Phase 2 + Migration Notes
  Leaf collisions overwrite silently (no byte-compare, no log of which files were clobbered). 0004 already writes a `.0004.bak` before config rewrites and echoes per-move lines; the new content-file overwrite produces no equivalent breadcrumb, so even VCS users can't tell genuine collisions from normal moves in the diff.

- 🟡 **Safety**: Partial failure mid-merge leaves both source and target in an inconsistent half-merged state
  **Location**: Phase 2, Section 2 (non-atomic recursive move)
  No atomicity or rollback (unlike `atomic_write` in the same file). A crash mid-merge splits data across two locations; re-run convergence then overwrites any leaf independently re-created in the interim.

- 🟡 **Compatibility**: No-VCS environments silently change from hard-fail to proceed (supported-environment contract change)
  **Location**: Phase 3 + "What We're NOT Doing"
  Beyond env-var removal, this changes *which environments* migrations run destructively in. A non-VCS checkout (CI sandbox, tarball, vendored copy) previously got a protective abort; now it gets irreversible moves with no signal the net was removed.

- 🟡 **Compatibility**: `merge_move` semantics must be verified identical on bash 3.2 and 5.x, including the `.*` dotfile glob
  **Location**: Phase 2, Section 2
  The load-bearing primitive runs on 3.2 (consumer) and 5.x (CI). If the `.*` glob or empty-glob fallback diverges, a relocation could drop dotfiles or fail while CI stays green — recreating the exact bug class this plan fixes.

- 🟡 **Portability**: No automated layer actually exercises the code under bash 3.2 — the floor rests on a denylist + manual replay
  **Location**: Phase 1 (suite wiring) + Phase 5 (bashisms lint); Desired End State
  CI is `ubuntu-latest` (bash 5.x) and suites resolve `#!/usr/bin/env bash` to first-on-PATH (often Homebrew 5.x on macOS). A grep denylist is a fixed enumeration; a future bash-4-only construct outside it passes both CI and lint and only crashes on a real 3.2 consumer.

- 🟡 **Test Coverage**: `merge_move` has no dedicated unit harness; several branches untested
  **Location**: Phase 2, Section 2 + Testing Strategy: Unit Tests
  All coverage routes through the migration integration tests, which exercise only each migration's specific path-pairs. Type-mismatch, symlink, empty-source, and the `rmdir` failure path are never directly driven.

- 🟡 **Test Coverage**: Iteration-order / index-alignment regression in the `declare -A` port is not pinned
  **Location**: Phase 4, Section 1
  "Refactor-under-green" with the assertion deferred "only if a gap appears", but the cited tests don't pin traversal order or which key is recorded as owner, and `config-dump`'s positional `REVIEW_DEFAULTS` could slip against `REVIEW_KEYS` for seven of eight keys undetected.

- 🟡 **Test Coverage**: Red steps anchored to exact line numbers will drift and mis-target silently
  **Location**: Implementation Approach / per-phase red steps
  `test-migrate.sh` is a flat 2000+-line script with no named tests. Phase 2's rewrites shift every line below them, so Phase 3's "delete 1103-1108" / "simplify 1004-1010" point at the wrong lines; the plan's own 0001 reference (183-184) is already off (that's the banner, not the assertions).

- 🟡 **Test Coverage**: Replacing the no-VCS refusal test with a no-VCS-success test removes the only VCS-present coverage of 0004
  **Location**: Phase 3
  The deleted test was the only fixture touching VCS detection; the new success test is also VCS-less. `build_scan_corpus` is retained and still inspects VCS state for Step 3 — a regression there would go uncaught.

- 🟡 **Architecture**: `merge_move` violates `atomic-common.sh`'s single responsibility (atomic file writes)
  **Location**: Phase 2, Section 2
  The module's contract is atomic single-file writes; `merge_move` is recursive directory relocation and is explicitly *non*-atomic — the opposite guarantee. Placement is justified only by proximity, and widens the dependency surface of every `atomic-common.sh` consumer.

- 🟡 **Architecture**: Repo-wide lint family + CI job is a separate initiative bundled into a bugfix plan
  **Location**: Phase 5 + Overview
  Phase 5 adds a `tasks/lint.py` family, a denylist scanner, shfmt + `.editorconfig`, a ~161-file mechanical reformat, a ShellCheck backlog cleanup, and a new CI job — a build/CI architecture change of comparable weight to the bugfix, riding on a bugfix plan. The only real dependency is narrow (bashisms must not fire on un-ported `declare -A`, already resolved by Phase 4).

- 🟡 **Code Quality**: `merge_move` swallows `mv`/`rmdir`/`mkdir` failures with no migration context
  **Location**: Phase 2, Section 2
  Unchecked `mv`/`mkdir` and a `|| true` on `rmdir`. A failure surfaces as a bare `mv: ...` with no src/dst context (unlike the old reconciliation banner), and a non-empty residual source is silently ignored.

- 🟡 **Code Quality**: The dotfile-merge glob idiom is fragile and the `.`/`..` guard is misleading
  **Location**: Phase 2, Section 2 (directory-merge loop)
  `for entry in "$src"/* "$src"/.*` always yields `.`/`..`; the guard filters on `basename "$entry"`, but `basename "$src/.."` returns the *parent dir name*, not `..`, so the explicit filter doesn't reliably exclude parent traversal — safety rests on the `[ -e ]` guard instead. Hard for the next maintainer to verify.

- 🟡 **Code Quality**: The hand-rolled `lint-bashisms.sh` grep denylist has an unmitigated false-positive surface
  **Location**: Phase 5, Section 3
  A literal forbidden construct in a comment, heredoc, or string trips the lint with no opt-out, and regex anchoring fragility risks silent misses. As the durable bash-3.2 guard, its long-term reliability matters.

#### Minor

- 🔵 **Correctness**: `_walked_owner` overloads empty-string for "unseen", conflating it with "seen but empty owner" (safe only because keys are non-empty). **Location**: Phase 4, Section 2

- 🔵 **Correctness**: After merge into an existing `meta/work/`, 0001's Step 2 frontmatter rewrite never visited the destination-resident files, leaving legacy `ticket_id:` un-canonicalised. **Location**: Phase 2, Section 5

- 🔵 **Correctness**: Interrupted-recursion convergence is asserted but not tested (one entry no-op, rest transfer, second run converges). **Location**: Phase 2, Section 2

- 🔵 **Safety**: 0001 loses its retry-safe abort when a directory target is a pre-existing regular file (now overwritten via the type-mismatch `rm -rf`). **Location**: Phase 2, Section 5

- 🔵 **Safety / Test Coverage**: Snapshot regeneration could mask a real regression — regen overwrites the sha256 manifest and exit codes, not just stderr; manual diff review is the only safeguard. **Location**: Phase 3, Section 3

- 🔵 **Compatibility**: The abort→source-wins behaviour change lands without a version-bump or changelog note (documented only in the internal plan). **Location**: Phase 2 + Migration Notes

- 🔵 **Compatibility**: The "byte-identical" port claim relies on tests that don't pin the recorded-owner interpolation in 0006's warning strings. **Location**: Phase 4, Section 1

- 🔵 **Compatibility / Portability**: shfmt pin left as `<pin>` placeholder; the aqua backend is a new install-time dependency and unpinned shfmt drifts the `-d` drift check. **Location**: Phase 5, Section 1

- 🔵 **Portability**: Suite discovery via `os.access(X_OK)` is filesystem-sensitive and *silently skips* (rather than fails) on exec-bit-lossy filesystems — the regression net could quietly not run. **Location**: Phase 1, Section 1

- 🔵 **Portability**: `merge_move`'s whole-tree `mv` assumes same-filesystem relocation; cross-fs moves are non-atomic and BSD/GNU-divergent on partial failure (note the assumption in the comment, as `atomic_write` does). **Location**: Phase 2, Section 2

- 🔵 **Architecture**: Claimed phase independence understates real 2→5 / 3→5 coupling — `merge_move` must pass Phase 5 gates and the repo-wide reformat rewrites the files Phases 2–4 edit. **Location**: Implementation Approach

- 🔵 **Architecture**: 0001 gains a new dependency edge (sources `atomic-common.sh`, transitively the JSONL/lock machinery it doesn't use), eroding migration self-containment. **Location**: Phase 2, Section 5

- 🔵 **Architecture**: `build_scan_corpus` is left under a now-inaccurate "dirty-tree pre-flight" section header after its sole pre-flight consumer is removed — re-home/relabel it next to Step 3. **Location**: Phase 3

- 🔵 **Code Quality**: Reference error — the accumulate-then-`raise Exit(code=1)` pattern is in `tasks/test/unit.py:42-49`, not `tasks/build.py:34-50` (which holds Mach-O/ELF magic bytes). **Location**: Phase 5, Section 4

- 🔵 **Code Quality**: Phase 2/3 success-criteria greps assert absence of *specific prose*, not behaviour — they silently pass if a future abort uses different wording; treat as belt-and-suspenders behind the behavioural merge tests. **Location**: Phase 2 & 3 Success Criteria

- 🔵 **Code Quality**: Consider a single generic `_array_lookup KEYS VALS needle` helper rather than two bespoke near-identical lookups for `WALKED`/`TEMPLATE_PATHS`. **Location**: Phase 4, Sections 2 & 3

- 🔵 **Test Coverage**: `lint.py` tests as specced assert command strings, not actual exclusion-glob *behaviour*; add a discovery-level test over a tmp tree with a fixture + `workspaces/` entry. **Location**: Phase 5, Section 6

- 🔵 **Test Coverage**: No test pins that 0001 can resolve `PLUGIN_ROOT` and source `atomic-common.sh` in the orchestrator-dispatched run context — run at least one 0001 merge test through the orchestrator. **Location**: Phase 2, Section 1

### Strengths

- ✅ Correctly diagnoses abort-on-conflict as an ADR-0023 idempotency-contract violation and consolidates 0001/0003/0004's three divergent policies behind one `merge_move` primitive — a genuine DRY/cohesion win.
- ✅ The `merge_move` four-state logic is complete with no fall-through gap, and the empty-glob `[ -e ]` guard + `.|..` skip handle empty dirs, dotfiles, and spaces without nullglob/globstar dependencies.
- ✅ Idempotency reasoning is sound (per-entry moves leave the source empty on a clean re-run), and Phase 2 re-asserts byte-identical-no-op re-run as a success criterion.
- ✅ Removing 0004's `_preflight_scan_corpus_clean` restores ADR-0023's single-gate invariant while correctly *retaining* `build_scan_corpus` for Step 3's inbound-link scan.
- ✅ Test-first sequencing: Phase 1 lands the regression net before the behaviour-changing phases; each fix phase is explicit red→green; no-change decisions (0002, orchestrator) are scoped out with rationale.
- ✅ The bash-3.2 port follows established in-repo idioms (parallel indexed arrays + linear-search), and the 0006/config-dump ports drive iteration order via explicit literal `for` loops, preserving deterministic output.
- ✅ Phase 5 closes the root-cause coverage gap (no bash-3.2 guard) with a static CI gate; the collision-string negative-grep is mutation-resistant and correctly carves out 0001's legitimate "cannot proceed" string.
- ✅ The snapshot-suite platform-sensitivity risk is anticipated with a sound conditional-exec-bit escape hatch rather than papered into CI; tooling is version-pinned for determinism.

### Recommended Changes

1. **Resolve the no-VCS data-loss path** (addresses: both Safety findings on no-VCS; Compatibility no-VCS contract change)
   Do not remove the last no-VCS guard *and* make moves destructive in the same release. Either lift a single no-VCS check to the orchestrator (one gate for the whole batch, matching ADR-0023), or have `merge_move` refuse to overwrite a colliding leaf when no VCS is detected. Decide and document the supported-environment contract explicitly.

2. **Harden `merge_move`'s destructive path** (addresses: Safety `rm -rf` guard; Code Quality swallowed failures; Correctness silent-abandon)
   Before `rm -rf`: assert `[ -n "$dst" ]`, that `$dst` is within `$PROJECT_ROOT`, use `rm -rf -- "$dst"`, and decide symlink handling explicitly. Add failure messages naming src/dst on `mv`/`mkdir`. Surface a warning (don't `|| true`-swallow) when `rmdir "$src"` fails for non-emptiness, listing residual entries — mirror `_cleanup_legacy_parent`.

3. **Give `merge_move` a dedicated unit harness** (addresses: Test Coverage no-harness; Correctness type-mismatch; Safety symlink/partial)
   Add `test-merge-move.sh` sourcing the helper directly and driving every branch: dir-over-file, file-over-dir, symlink src/dst, empty source, deep nested same-named-subdir merge, leaf-collision source-wins byte content, and an interrupted/partial-merge convergence case. Mutation-check (`rm -rf`→`rm -f`, flipped type operator) must fail an assertion.

4. **Verify the bash-3.2 floor rather than assert it** (addresses: Portability no-3.2-exercise; Compatibility 3.2/5.x divergence)
   Add a lightweight 3.2 smoke layer — at minimum the migrate subtree under a pinned `bash:3.2` container (or an installed old bash) in CI — and document in `lint-bashisms.sh` that the denylist is known-incomplete (catches enumerated constructs only).

5. **Add an audit trail for overwrites** (addresses: Safety audit-trail; Compatibility changelog)
   Have `merge_move` emit a distinct line on each actual leaf overwrite (`merge_move: overwrote <dst> with <src> (source-wins collision)`), and add a changelog/release-note entry for the abort→source-wins behaviour change.

6. **Re-anchor the red steps and make the Phase 4 order assertion mandatory** (addresses: Test Coverage line-drift; iteration-order)
   Anchor each test edit on the unique `Test: ...` banner string, not line numbers, and require Phases 2–4 in order (or re-grep banners before each edit). Promote the 0006 owner-key and `config-dump` first/last-key default assertions from conditional to mandatory.

7. **Reconsider Phase 5 scoping and the helper's module home** (addresses: Architecture bundling; module SRP; phase coupling)
   Consider splitting the repo-wide lint rollout into its own follow-on plan (it arguably warrants an ADR), home `merge_move` in a relocation-focused module rather than `atomic-common.sh`, pin its contract with the dedicated test from (3), and restate the real 2→5/3→5 ordering coupling in the Implementation Approach.

8. **Restore VCS-present coverage for 0004 and fix the `tasks/build.py` reference** (addresses: Test Coverage VCS-present gap; Code Quality reference error)
   Add a 0004-direct test in a committed-clean git fixture asserting moves + Step 3 inbound-link rewrite still succeed; correct the pattern reference to `tasks/test/unit.py:42-49`.

---

## Per-Lens Results

### Correctness

**Summary**: The plan is logically well-grounded: the merge-move primitive correctly handles the four relocation states, the bash-3.2 ports preserve the observable contracts (iteration order, warning strings, the seen/unseen distinction), and the idempotency reasoning is sound because per-file moves leave the source empty on a clean re-run. The main correctness gaps are in the merge-move helper's failure/edge behaviour: it silently abandons a non-empty source on a partial merge, and the source-wins type-mismatch branch can obliterate an entire destination subtree without that path being pinned by the planned tests.

**Strengths**:
- The merge_move four-state logic (src absent no-op; dst absent plain move; type/leaf mismatch source-wins; both-dirs recurse) is complete and each branch returns, with no fall-through gap.
- The empty-glob guard plus the `.|..` case skip correctly handle empty directories, dotfiles, and the literal-glob case under bash 3.2; all paths are quoted so filenames with spaces survive.
- Idempotency reasoning is correct: a clean completed migration leaves nothing for the next planning pass to move, so the existing tree_hash re-run tests remain valid.
- The 0006 port preserves iteration order, exact warning strings keyed on the previously-recorded owner, and the WALKED-only stdout skip line.
- config-dump.sh REVIEW_DEFAULTS is correctly enumerated to nine values in REVIEW_KEYS order; the index-based loop mirrors the proven AGENT_KEYS/AGENT_DEFAULTS pattern.

**Findings**:
- **major / high** — Phase 2, Section 2: `rmdir "$src" 2>/dev/null || true` swallows the non-empty signal; helper returns 0 and caller records success, leaving orphaned source trees invisibly (vs `_cleanup_legacy_parent` which warns).
- **major / high** — Phase 2, Section 2 & Section 1: type-mismatch branch `rm -rf`s an entire dst subtree on file-over-dir / dir-over-file; planned red tests never cover the type-mismatch directions.
- **minor / medium** — Phase 4, Section 2: `_walked_owner` empty-string sentinel conflates unseen with empty-owner (safe only because keys are non-empty).
- **minor / medium** — Phase 2, Section 5: post-merge, 0001's Step 2 frontmatter rewrite never visited destination-resident files, leaving legacy `ticket_id:` un-canonicalised.
- **minor / low** — Phase 2, Section 2: interrupted-recursion convergence relies on re-entrancy against partial output, asserted but not tested.

### Safety

**Summary**: This is a data-migration plan that rewrites users' meta-directories in place, and it makes two safety-significant changes simultaneously: it replaces abort-on-conflict with a destructive source-wins merge (including `rm -rf "$dst"` on type mismatch), and it deletes 0004's no-VCS hard-fail — the ONLY no-VCS guard in the framework, since the orchestrator's clean-tree gate silently passes when no VCS is present. The net effect for a non-VCS user is irreversible overwrites with zero recovery path, contradicting the project's "VCS revert is the recovery path" stance (which presumes VCS is present).

**Strengths**:
- Phase 1 lands the regression harness FIRST, protecting the destructive Phase 2/3 changes with tests as they are made.
- Preserves per-migration idempotency and adds explicit re-run convergence tests.
- Documents the merge contract as an observable behaviour change in Migration Notes; keeps the orchestrator gate and ACCELERATOR_MIGRATE_FORCE unchanged.
- Phase 3 retains build_scan_corpus; Phase 4 preserves byte-identical behaviour, limiting blast radius.
- The snapshot-regen step requires reviewing the diff rather than blindly regenerating.

**Findings**:
- **critical / high** — Phase 3 + "What We're NOT Doing": deleting the only no-VCS guard while moves become destructive leaves non-VCS users with no recovery net (orchestrator gate verified to pass silently with no VCS).
- **critical / medium** — Phase 2, Section 2: `rm -rf "$dst"` has no guard against empty/unexpected `dst` or symlinked targets; highest-blast-radius op in the plan.
- **major / high** — Phase 2 + Migration Notes: irreversible source-wins overwrite has no audit trail or pre-change backup (vs 0004's existing `.0004.bak`).
- **major / medium** — Phase 2, Section 2: non-atomic recursive move; partial failure leaves a half-merged state with no rollback.
- **minor / medium** — Phase 3, Section 3: snapshot regeneration could mask a real regression if diff review is cursory.
- **minor / medium** — Phase 2, Section 5: 0001 loses its retry-safe abort when a directory target is a pre-existing regular file.

### Compatibility

**Summary**: The plan is fundamentally a compatibility-restoration effort: it brings two scripts back under the ADR-0016 bash-3.2 floor and adds CI enforcement. The headline contract changes (abort→source-wins, removal of the no-VCS guard and env var) are real behaviour changes, but the consumer base is internal automated migrations whose recovery model is VCS revert, and the prior abort itself violated the idempotency contract — so most are defensible. Main residual risks: unversioned/uncommunicated supported-environment change (no-VCS now silently proceeds) and the unverified "byte-identical" claim for the new merge_move helper.

**Strengths**:
- Restores compatibility with the documented bash-3.2 floor and adds a durable CI guard so it stops silently regressing.
- Verifiably confirms ACCELERATOR_MIGRATE_FORCE_NO_VCS has no user-facing doc surface — deleting it is a genuine silent no-op.
- Iteration order for the ports is driven by explicit literal loops, preserving deterministic output.
- Tool pinning discipline maintained (shellcheck 0.11.0; shfmt to be pinned).

**Findings**:
- **major / medium** — Phase 3 + "What We're NOT Doing": no-VCS environments silently change from hard-fail to proceed — a supported-environment contract change.
- **major / medium** — Phase 2, Section 2: merge_move semantics (esp. the `.*` dotfile glob) must be verified identical on bash 3.2 and 5.x or CI stays green while 3.2 breaks.
- **minor / high** — Phase 2 + Migration Notes: the abort→source-wins change lands without a version-bump or changelog note.
- **minor / medium** — Phase 4, Section 1: the "byte-identical" claim relies on tests that don't pin the recorded-owner interpolation in 0006's warnings.
- **minor / medium** — Phase 5, Section 1: shfmt pin left as `<pin>` placeholder risks lint drift between dev and CI.

### Portability

**Summary**: The plan is squarely about cross-platform shell portability and is unusually portability-aware: merge_move sticks to POSIX-portable primitives, the snapshot caveat has a sound escape hatch, and tooling is pinned. The dominant residual risk is that no automated layer actually exercises the code under bash 3.2 — CI runs 5.x, suites run via the dev's first-on-PATH bash — so the entire 3.2 contract rests on a denylist grep plus a manual replay.

**Strengths**:
- merge_move is built from POSIX-portable primitives and explicitly avoids GNU-only conveniences, so it behaves identically on BSD/macOS and GNU/Linux.
- The literal-glob guard is the right bash-3.2-safe idiom for the empty-directory case.
- The snapshot platform-sensitivity risk is anticipated with a sound exec-bit escape hatch.
- shellcheck pinned (0.11.0) and shfmt to be pinned, so lint results are reproducible across hosts.
- The port follows established in-repo portable idioms, keeping the codebase internally consistent.

**Findings**:
- **major / high** — Phase 1 + Phase 5; Desired End State: no automated layer runs the code under bash 3.2; the floor rests on a fixed-enumeration denylist + manual replay.
- **minor / medium** — Phase 5, Sections 1 & 8: shfmt via aqua backend adds a new network/registry install dependency on both platforms.
- **minor / medium** — Phase 1, Section 1: suite discovery via `os.access(X_OK)` is filesystem-sensitive and silently skips rather than fails.
- **minor / low** — Phase 2, Section 2: merge_move's whole-tree `mv` assumes same-filesystem relocation; cross-fs moves are non-atomic and BSD/GNU-divergent (note the assumption in the comment).

### Test Coverage

**Summary**: The plan is genuinely test-first: every fix phase opens with a red step against the new contract, Phase 1 lands the regression net first, and Phase 5 adds a durable bash-3.2 guard. The weakest area is merge_move — the riskiest new code — which has no dedicated unit harness and is covered only indirectly, leaving several branches unexercised. Secondary concerns: line-number-anchored red steps that will drift, the snapshot regeneration safeguard resting on manual diff review, and the Phase 4 refactor-under-green claim where existing tests don't pin iteration order.

**Strengths**:
- Phase 1 sequences the regression harness ahead of the behavioural changes.
- Each fix phase writes/adjusts the failing test against the new contract first and confirms it fails.
- Phase 5's bashisms lint directly closes the root-cause coverage gap that let `declare -A` ship.
- The collision-string negative-assertion is mutation-resistant and correctly carves out 0001:22.
- Idempotency is explicitly re-asserted as a Phase 2 success criterion.

**Findings**:
- **major / high** — Phase 2, Section 2 + Testing Strategy: merge_move has no dedicated unit harness; type-mismatch, symlink, empty-source, and rmdir-failure branches untested.
- **major / high** — Phase 4, Section 1: iteration-order / index-alignment regression in the port is not pinned by any assertion.
- **major / medium** — Implementation Approach: line-number-anchored red steps will drift and mis-target (the 0001 reference is already off).
- **major / medium** — Phase 3: replacing the no-VCS refusal test with a no-VCS-success test removes the only VCS-present coverage of 0004 / build_scan_corpus.
- **minor / medium** — Phase 3, Section 3: snapshot regeneration relies on manual diff review across a multi-file sha256 manifest as the only safeguard.
- **minor / medium** — Phase 5, Section 6: lint task tests verify command strings, not actual exclusion-glob behaviour.
- **minor / low** — Phase 2, Section 1: no test pins that 0001 can source atomic-common.sh in the orchestrator-dispatched context.

### Architecture

**Summary**: The plan is architecturally sound at its core: it correctly identifies the abort-on-conflict idempotency violation and consolidates three duplicated relocation policies behind one merge-move primitive — a genuine cohesion improvement. The two main structural concerns are the module boundary for merge_move (atomic-common.sh is a pure atomic-file-write module and merge_move is a different, non-atomic concern) and the bundling of a whole repo-wide lint family + CI job into a bugfix plan. The claimed phase independence is slightly over-stated.

**Strengths**:
- Consolidating three independently-evolved abort policies behind one primitive removes divergent logic and aligns with ADR-0023's idempotency contract.
- Removing 0004's pre-flight restores the single-gate invariant while correctly retaining build_scan_corpus for its separate Step 3 consumer.
- Reasons explicitly about idempotency as the governing force and keeps VCS revert as the single recovery model.
- Test-first sequencing with the regression harness first; no-change decisions scoped out with rationale.

**Findings**:
- **major / high** — Phase 2, Section 2: merge_move violates atomic-common.sh's single responsibility; it is the only non-atomic primitive in an atomicity-contracted module.
- **major / medium** — Phase 5 + Overview: repo-wide lint family + CI job is a separate initiative (arguably ADR-worthy) bundled into a bugfix plan.
- **minor / high** — Implementation Approach: claimed phase independence understates real 2→5 / 3→5 coupling (merge_move must pass Phase 5 gates; the reformat rewrites Phase 2–4 files).
- **minor / medium** — Phase 2, Section 5: 0001 gains a new dependency edge (atomic-common.sh + transitive JSONL/lock machinery), eroding self-containment.
- **minor / medium** — Phase 3: build_scan_corpus left under a now-inaccurate "dirty-tree pre-flight" header after its sole consumer is removed.

### Code Quality

**Summary**: The plan is unusually well-structured for maintainability: it follows existing idioms, keeps phases independent, and consolidates three duplicated policies into one shared helper — a strong DRY win. The principal risks are concentrated in merge_move (silently swallows failures, fragile dotfile-glob idiom) and the hand-rolled lint-bashisms.sh grep denylist (false positives in comments/heredocs/strings, no stated mitigation). Several success-criteria greps and one code reference are brittle.

**Strengths**:
- Consolidating three separately-shaped abort policies into one shared merge_move is a clear DRY improvement.
- The bash-3.2 port faithfully follows the established parallel-array + linear-search idiom.
- Phases are decomposed to be mutually independent with one honestly-documented ordering constraint; each fix phase is test-first.
- Explicitly scopes out helpers that look similar but are semantically different (`_rename_user_template_file_if_present`, 0001's "cannot proceed").
- merge_move is well-documented with a header comment stating its contract.

**Findings**:
- **major / high** — Phase 2, Section 2: merge_move swallows mv/rmdir/mkdir failures; failures surface with no migration context, residual sources silently ignored.
- **major / high** — Phase 2, Section 2: the dotfile-merge glob is fragile and the `.`/`..` guard (operating on `basename`) is misleading — `basename "$src/.."` is the parent name, not `..`.
- **major / medium** — Phase 5, Section 3: the lint-bashisms.sh grep denylist has an unmitigated false-positive surface (comments, heredocs, strings) with no opt-out convention.
- **minor / high** — Phase 5, Section 4: reference error — the accumulate-then-`raise Exit(code=1)` pattern is in `tasks/test/unit.py:42-49`, not `tasks/build.py:34-50`.
- **minor / medium** — Phase 2 & 3 Success Criteria: grep assertions are message-coupled (assert absence of prose, not behaviour).
- **minor / medium** — Phase 4, Sections 2 & 3: consider a single generic `_array_lookup` helper rather than two bespoke near-identical lookups.

---

## Re-Review (Pass 2) — 2026-06-04T17:00:19+00:00

**Verdict:** APPROVE

The edits resolved every critical and major finding from pass 1 or recorded
them as consciously accepted tradeoffs. The two no-VCS findings (Safety
critical, Compatibility major) are accepted ("we assume users are on VCS",
documented in Migration Notes). `merge_move` was moved to a new dependency-free
`scripts/fs-common.sh`, the destructive `rm -rf` path gained a non-empty-`$dst`
bug-guard + `rm -rf --`, the fragile dotfile glob was replaced with the correct
`"$src"/* "$src"/.[!.]* "$src"/..?*` idiom (verified portable and identical on
bash 3.2/5.x), a dedicated `merge_move` unit harness with a mutation check was
added, the Phase 4 order/owner-key assertions were made mandatory, test edits
re-anchored on banners, the VCS-present 0004 coverage restored, the
`tasks/test/unit.py` reference fixed, and the lint spec hardened. **Zero
critical, zero major findings remain** — only minor polish and a few
self-flagged "acceptable as-is" items. The plan is ready for implementation.

### Previously Identified Issues

- 🔴 **Safety**: Deleting the only no-VCS guard leaves non-VCS users with no recovery net — **Accepted tradeoff** (recorded in Migration Notes)
- 🔴 **Safety**: `merge_move`'s `rm -rf` unguarded against empty/unexpected `dst` — **Resolved** (non-empty `$dst` guard + `rm -rf --`; symlink handling declined as accepted tradeoff)
- 🟡 **Correctness**: `merge_move` silently abandons a non-empty source — **Resolved** (warning surfaced instead of `|| true`)
- 🟡 **Correctness**: type-mismatch branch obliterates dst subtree, untested — **Resolved** (guarded + dedicated harness covers both directions)
- 🟡 **Safety**: source-wins overwrite has no audit trail — **Accepted tradeoff** (declined as over-engineering given VCS)
- 🟡 **Safety**: partial mid-merge leaves inconsistent state — **Resolved** (NON-ATOMIC documented, failure surfaced, convergence test added)
- 🟡 **Compatibility**: no-VCS hard-fail → silent proceed — **Accepted tradeoff**
- 🟡 **Compatibility**: `merge_move` cross-version (3.2/5.x) unverified — **Resolved** (glob bash-3.2-safe; 3.2 guarded by lint + manual replay, recorded residual risk)
- 🟡 **Portability**: no automated bash-3.2 exercise — **Partially resolved** (documented accepted residual risk; Option B not adopted)
- 🟡 **Test Coverage**: `merge_move` no dedicated unit harness — **Resolved** (`scripts/test-merge-move.sh` + mutation check)
- 🟡 **Test Coverage**: iteration-order/index regression unpinned — **Resolved** (Phase 4 owner-key + first/last config-dump assertions now mandatory)
- 🟡 **Test Coverage**: line-number-anchored red steps drift — **Resolved** (banner-anchored + ordered phases)
- 🟡 **Test Coverage**: removing no-VCS test leaves no VCS-present 0004 coverage — **Resolved** (committed-clean 0004 test added)
- 🟡 **Architecture**: `merge_move` violates `atomic-common.sh` SRP — **Resolved** (moved to new leaf `fs-common.sh`)
- 🟡 **Architecture**: lint family bundled into a bugfix plan — **Accepted decision** (kept in plan)
- 🟡 **Code Quality**: `merge_move` swallows mv/rmdir/mkdir failures — **Partially resolved** (rmdir surfaced + `$dst` guard; mv/mkdir still rely on `set -e`, acceptable)
- 🟡 **Code Quality**: dotfile-glob fragile + misleading `.|..` guard — **Resolved** (correct idiom + accurate comment)
- 🟡 **Code Quality**: bashisms denylist false-positive surface — **Resolved** (comment-stripping, opt-out marker, known-incomplete header, test coverage)
- 🔵 (all minor pass-1 findings: `tasks/build.py` ref, shfmt pin, build_scan_corpus relabel, snapshot regen, lint.py discovery test, 0001 sourcing, phase coupling, 0001 frontmatter scope) — **Resolved**

### New Issues Introduced

All minor / suggestion severity — none block implementation:

- 🔵 **Safety** (minor): the `rm -rf --` branch is guarded against empty/dash but not against an out-of-tree/`..`-escaping `$dst`. Safe for the three current call sites (all `$PROJECT_ROOT/$rel`), but a future `fs-common.sh` caller could pass a wide path. Optional: assert `$dst` resolves strictly under `$PROJECT_ROOT`.
- 🔵 **Test Coverage** (minor): the mid-batch sibling-dirty test asserts *absence* of the abort string only; pair it with a positive postcondition (exit 0 + files relocated) so it proves convergence, not just a missing message.
- 🔵 **Test Coverage** (minor): make the type-mismatch mutation-killing — for dir-over-file assert no former-destination leaf survives; for file-over-dir assert the destination is a regular file. Also add a concrete 0001 destination-resident frontmatter assertion.
- 🔵 **Compatibility** (minor): the abort→source-wins change is in Migration Notes but not in `CHANGELOG.md` (which has a `### Breaking` convention + empty `[Unreleased]`); add a consumer-facing entry.
- 🔵 **Code Quality** (minor): 0003/0004's rerouted `_move_if_pending` shims diverge gratuitously (`rel_src`/`rel_dst` vs `src_rel`/`dst_rel`) and duplicate `merge_move`'s own `[ -e "$src" ]` guard; align naming and drop the redundant guard. Optionally add mv/mkdir failure context to match the rmdir branch's quality bar.
- 🔵 **Correctness** (minor, benign): a symlink-to-directory source would route into the merge branch and trip a spurious "source not empty" warning; out of domain for migrations. Note it in the helper comment if symlinked roots are conceivable.
- 🔵 **Architecture** (suggestion): `lint` is the first cross-cutting task family that's a peer of `test` rather than a `test.*` subcommand — confirm top-level placement is the intended precedent for future gates (e.g. typecheck).
- 🔵 **Portability** (suggestion): a near-zero-cost tightening — a `macos-latest` CI job running just the migrate suites under stock `/bin/bash` 3.2 would actually exercise the floor (the release jobs already use `macos-latest`).

### Assessment

The plan is in good shape and ready for implementation. The destructive-operation
surface is now appropriately guarded for the VCS-assumed model, the riskiest new
code (`merge_move`) has dedicated branch-level coverage with a mutation check, and
the bash-3.2 enforcement gap is a documented, conscious tradeoff. The new issues
are all minor refinements that can be folded into implementation; none change the
plan's structure or block starting work.

---
*Review generated by /accelerator:review-plan*
