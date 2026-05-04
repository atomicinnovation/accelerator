---
date: "2026-05-05T08:15:00+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-05-0031-consolidate-accelerator-owned-files.md"
review_number: 1
verdict: REVISE
lenses: [architecture, code-quality, test-coverage, correctness, safety, compatibility, documentation]
review_pass: 2
status: complete
---

## Plan Review: 0031 Consolidate Accelerator-Owned Files Under `.accelerator/`

**Verdict:** REVISE

The plan is unusually well-structured for a reorg of this scope: phase ordering is internally coherent, TDD is applied per phase, atomic delivery of migration 0003 + driver + discoverability hook is correctly identified, and the existing `config-read-path.sh` indirection is leveraged to keep runtime changes minimal. However, four critical defects need to be fixed before implementation: (1) the `paths.tmp` probe via `config-read-path.sh tmp ""` returns an empty string, not `meta/tmp`, in the default case, which inverts the intended move logic; (2) creating the `.accelerator/` scaffold before performing directory moves causes `mv` to nest the source inside the existing destination (e.g. `.accelerator/skills/skills/`); (3) multi-`mv` failures leave the repo in an unrecoverable mixed state with no preflight or rollback; (4) `mv` to a destination that already exists silently clobbers user-authored content. A consistent set of major findings around scaffold-logic duplication, gitignore-rule idempotency (`grep -qF` substring vs `-qFx` exact), discoverability-hook gaps, and CHANGELOG omissions reinforce that the migration script needs more design work and the plan needs more precision before implementation.

### Cross-Cutting Themes

- **`paths.tmp` probe semantics** (flagged by: Correctness, Safety, Code Quality, Architecture) — Both the empty-default-arg behaviour and the literal-equals-`meta/tmp` heuristic create incorrect default-detection. The migration would silently fail to move `meta/tmp/` for default-config users, and would silently move it for users who explicitly pinned `paths.tmp: meta/tmp`. The init script and migration script also disagree on canonical-tmp semantics, creating a long-lived divergence.

- **Scaffold-before-move `mv` collision** (flagged by: Correctness, Compatibility, Architecture) — Phase 3 creates `.accelerator/skills/.gitkeep`, `.accelerator/lenses/.gitkeep`, etc. before performing `mv .claude/accelerator/skills .accelerator/skills`. Standard POSIX `mv` semantics nest the source inside the existing destination, producing `.accelerator/skills/skills/` and similar broken layouts. Cross-platform `mv` differences (macOS vs Linux) compound the risk.

- **Scaffold logic duplicated between `init.sh` and migration 0003** (flagged by: Code Quality, Architecture, Test Coverage) — Both surfaces independently re-implement the same `.gitignore` body, `.gitkeep` set, and rule formatting. No shared helper exists, no test asserts the two routes produce equivalent trees, and drift is the most likely future bug.

- **Gitignore-rule idempotency and user-customisation preservation** (flagged by: Correctness, Safety, Code Quality) — Per-rule `grep -qF` performs substring matching, which produces false-positive idempotency (a comment containing `site.json` would block the rule append). The rule list is also duplicated across `_jira_ensure_gitignore` and migration 0003. The plan does not specify how user-authored comments, negated rules, or trailing whitespace are preserved during root-`.gitignore` rewrites.

- **Discoverability hook gaps** (flagged by: Compatibility, Correctness, Architecture, Code Quality) — The hook only fires on SessionStart, so mid-session skill invocations on un-migrated repos hit raw failures rather than the warning. The fallback chain has a gap when `.accelerator/` exists but `state/migrations-applied` does not. The fallback is declared "permanent" with no sunset criterion.

- **Migration atomicity and no-VCS recovery** (flagged by: Safety, Correctness) — The migration performs ~9 sequential `mv` operations under `set -euo pipefail` with no rollback. Failure halfway leaves the repo unrecoverable for users without VCS (the project's only sanctioned recovery path). Destination-clobber when `.accelerator/config.md` already exists (e.g. partial recovery) is unspecified.

- **CHANGELOG and init SKILL.md documentation gaps** (flagged by: Documentation, Compatibility) — The new CHANGELOG entry omits the `paths.templates`/`paths.integrations` pinned-path caveat, the root-`.gitignore` rewrite, and the session-restart requirement. The existing `[Unreleased]` Jira entry contradicts the new layout. Init SKILL.md prose updates are incomplete (Step 3 heading, Step 4 results template, closing paragraph all unaddressed).

### Tradeoff Analysis

- **Explicit-override detection vs bash 3.2 simplicity**: Safety and Correctness recommend reading `paths.tmp` directly from the config file with a sentinel to detect explicit overrides; Code Quality notes the work item Assumptions deferred this for `templates`/`integrations` precisely because of bash 3.2 awkwardness. The probe approach in the plan is the intermediate option that hits both downsides — it neither correctly detects explicit overrides nor avoids the awkwardness. Recommendation: pick one extreme — either implement true explicit-override detection (a single `grep -E '^[[:space:]]*tmp:'` is fine in bash 3.2) or document `paths.tmp == meta/tmp` as default-equivalent and accept the trade-off uniformly.

- **Discoverability fallback in runtime scripts vs strict hard-cut**: Compatibility wants a lightweight legacy-path detector at the top of `config-common.sh` so mid-session users see a useful error; Architecture has retained the strict hard-cut posture except for the SessionStart hook. Recommendation: a one-line check ("if `.claude/accelerator.md` exists and `.accelerator/config.md` doesn't, exit non-zero with a directive to run `/accelerator:migrate`") preserves the hard-cut spirit while closing the mid-session gap.

### Findings

#### Critical

- 🔴 **Correctness**: `paths.tmp` default-case probe returns empty string, breaking the move condition
  **Location**: Phase 3, Section 1: Conditional moves — paths.tmp probe
  `config-read-path.sh tmp ""` returns `""` (the default arg) when `paths.tmp` is unset, not `meta/tmp`. The equality check therefore fails for the common default case and `meta/tmp/` is silently NOT moved. Either probe with the literal default explicit (`config-read-path.sh tmp meta/tmp`) or branch on `[ -z "$tmp" ] || [ "$tmp" = "meta/tmp" ]`.

- 🔴 **Correctness**: Scaffold initialisation creates destination directories before mv, causing nested moves
  **Location**: Phase 3, Section 1: Initialise scaffold then move sources
  The plan creates `.accelerator/skills/.gitkeep`, `lenses/.gitkeep`, `templates/.gitkeep`, `tmp/.gitkeep`, `state/.gitkeep` before performing `mv .claude/accelerator/skills .accelerator/skills`. POSIX `mv` semantics move the source *into* the existing destination, producing `.accelerator/skills/skills/` etc. Reorder so per-subdirectory `.gitkeep` files are written only after the moves complete (and only for directories not sourced from a legacy tree), or use `mv source/* dest/ && rmdir source` with explicit dotfile handling.

- 🔴 **Safety**: Mid-flight `mv` failure leaves repo in unrecoverable mixed state without VCS
  **Location**: Phase 3, Section 1: New migration script — conditional moves + post-move source removal
  Nine sequential `mv` operations under `set -euo pipefail` with no preflight, no checkpoint, and no rollback. If any move fails (cross-device link, permissions, EBUSY), the repo is left half-migrated and users without VCS commits have no recovery path. Add a preflight that builds the move list and verifies destinations are writable before the first real `mv`; on failure, emit explicit per-source recovery instructions.

- 🔴 **Safety**: Destination clobber when `.accelerator/config.md` already exists with different content
  **Location**: Phase 3, Section 1: New migration script — conditional moves
  The `no_op_pending` sentinel only handles "no source paths exist". The mixed case (source exists AND destination exists, with potentially divergent content) is unspecified — `mv` silently overwrites. Before each move, refuse if both source and destination exist; require manual reconciliation.

#### Major

- 🟡 **Correctness**: `grep -qF` performs substring match, not exact-rule match — false-positive idempotency
  **Location**: Phase 3 inner Jira `.gitignore`; Phase 5 `_jira_ensure_gitignore` per-rule grep
  Per-rule `grep -qF 'site.json'` matches `!site.json`, `site.json.bak`, comments containing the substring, etc. Use `grep -qFx` consistently (matches the init script's idiom) — this is a load-bearing safety property.

- 🟡 **Correctness**: Migration's `paths.tmp` probe depends on which config path `config-read-path.sh` reads at runtime
  **Location**: Phase 3, Section 1 — interaction with Phase 6 source-of-truth update
  Migration ships in Phase 3 reading `.claude/accelerator.md` correctly, but the migration also runs against repos at any future time after Phase 6 has shipped. After Phase 6, `config_find_files` reads only `.accelerator/config.md` — which migration 0003 has not yet created — so the probe sees no config and treats every user as "no override". Migration scripts should not depend on the runtime config layer they themselves are about to mutate; read `paths.tmp` directly from the legacy config file.

- 🟡 **Correctness**: Driver still creates legacy `meta/` directory before appending state
  **Location**: Phase 3, Section 2: Driver updates
  `run-migrations.sh:203` calls `mkdir -p "$PROJECT_ROOT/meta"` before `atomic_append_unique`. After STATE_FILE relocation, this re-creates an empty `meta/` on every migrate run on a clean post-migration repo (also re-triggers the discoverability hook's three-clause sentinel). Replace with `mkdir -p "$(dirname "$STATE_FILE")"`.

- 🟡 **Correctness**: Discoverability hook fallback chain leaves a gap when `.accelerator/` exists but state file is missing
  **Location**: Phase 3, Section 3
  Partial-recovery state (scaffold created, state file not yet moved) makes the hook choose the new branch and find nothing — warning fires erroneously, and re-running migrate would attempt to re-apply 0001/0002. Make the fallback exist-aware: read whichever state file actually exists.

- 🟡 **Safety**: No-VCS repos bypass the clean-tree safety net entirely
  **Location**: Pre-flight clean-tree check (run-migrations.sh:42-69, extended in Phase 3)
  When `vcs=""`, the dirty-tree check passes silently and the migration runs with no rollback path. Refuse to run unless `ACCELERATOR_MIGRATE_FORCE=1` is set, with a message explaining VCS-based recovery is the project's only rollback mechanism.

- 🟡 **Safety**: User explicitly pinning `paths.tmp: meta/tmp` is silently treated as unset and migrated
  **Location**: Phase 3 paths.tmp probe; Test case 6
  Test case 6 enshrines the misclassification as desired behaviour, contradicting the work-item statement that "explicit override leaves the source untouched". Either implement true explicit-override detection or warn loudly when the heuristic forces a move; align the test case with the chosen contract.

- 🟡 **Safety**: Gitignore mutation overwrites user customisations without preservation
  **Location**: Phase 3 root `.gitignore` rewrite; Phase 6 Section 5
  The plan does not specify how the rewrite handles inline comments, negated rules, surrounding context, or section headers. Specify the algorithm precisely (whole-line match only, refuse to rewrite lines with trailing content, emit a notice listing every line touched).

- 🟡 **Safety**: Pinned `paths.templates` / `paths.integrations` overrides moved unconditionally with no detection
  **Location**: Assumptions / Migration Notes
  The plan correctly chose unconditional moves for these keys, but provides no detect-and-warn pass. A user with `paths.templates: custom/templates` and a stale `meta/templates/` will see content shuffled with no warning. A one-line `grep` of the config file is sufficient for detection.

- 🟡 **Safety**: State-file relocation during the same run that records migration 0003
  **Location**: Phase 3, Section 2: STATE_FILE relocation
  If migration 0003 succeeds but the state-file move fails partway, the driver appends `0003` to a fresh empty file, losing prior migration history (`0001`, `0002`). Read source content first, write to destination atomically, verify, then remove source.

- 🟡 **Architecture**: Inconsistent `paths.tmp` semantics between init and migration create an architectural divergence
  **Location**: Phase 4 init.sh Step 3 vs Phase 3 migration paths.tmp handling
  Re-running `init` after migration on a repo with `paths.tmp: meta/tmp` recreates the legacy directory the user was supposed to migrate away from, with no warning. Lift override-detection into a shared helper or document canonical-location semantics in one place.

- 🟡 **Architecture**: Destination-collision behaviour is unspecified for the directory moves
  **Location**: Phase 3 / Migration 0003
  The plan asserts atomicity but does not specify which behaviour the migration uses when destinations exist. Specify explicitly and add an explicit test case for `mv` colliding with the pre-created scaffold. (Closely related to the critical mv-nesting finding.)

- 🟡 **Architecture**: Permanent discoverability fallback chain is an open-ended architectural commitment
  **Location**: Phase 3, Section 3
  No sunset condition for the legacy state-file path. Either bound the fallback to a future release with a follow-up work item, or document a sunset criterion in the hook source.

- 🟡 **Code Quality**: Migration 0003 bundles too many responsibilities into one script with no decomposition
  **Location**: Phase 3, Section 1 / Changes Required #1
  Preflight + scaffold + root-gitignore rewrite + 9 conditional moves + override detection + inner gitignore + cleanup, all flat. Decompose into named steps mirroring `0002-...sh` and lift reusable primitives (`_replace_or_append_gitignore_rule`, `_remove_gitignore_rule`, `_move_if_source_exists`) into `scripts/migration-helpers.sh`.

- 🟡 **Code Quality**: `.accelerator/` scaffold logic duplicated between init.sh and migration 0003
  **Location**: Phase 4 step #2 / Phase 3 step #1
  Same byte sequence specified twice. Extract a shared `accelerator_init_scaffold` helper and a single test for the scaffold contract.

- 🟡 **Code Quality**: `paths.tmp` default-vs-override detection is under-specified for bash 3.2
  **Location**: Phase 3, Section 1 / paths.tmp conditional move
  See cross-cutting theme. The plan and tests contradict the work-item statement; pick one semantic and align everything.

- 🟡 **Test Coverage**: Acceptance criteria for config-summary/dump path-output cleanliness are not mapped to a test
  **Location**: Phase 6 / work item ACs
  No positive assertion that `config-summary.sh` / `config-dump.sh` emit no `.claude/` or `meta/` prefixed paths post-migration. Add explicit `assert_not_matches_regex` cases.

- 🟡 **Test Coverage**: No planned test verifies the seven Jira skill scripts are free of hardcoded legacy paths
  **Location**: Phase 6 / work item AC: "no Jira integration skill script contains a hardcoded reference to `meta/integrations/jira/`"
  Promote the legacy-path grep into an assertion within `test-jira-init-flow.sh` (or a small `test-jira-paths.sh`) iterating the seven skill files.

- 🟡 **Test Coverage**: Visualiser launch-server effective-tmp-path AC has no explicit assertion
  **Location**: Phase 6 / work item AC
  Add a `test-launch-server.sh` case where `paths.tmp` is **not** set in the seeded config and assert the resolved tmp path is `.accelerator/tmp` (exercises the default-arg path the AC actually targets).

- 🟡 **Compatibility**: Discoverability hook only fires on SessionStart — users invoking a skill mid-session before migrate hit raw failures
  **Location**: Phase 3 / Migration Notes
  Add a lightweight legacy-path detector at the top of `config-common.sh` redirecting users to `/accelerator:migrate`, or document the session-restart requirement in the CHANGELOG.

- 🟡 **Compatibility**: CHANGELOG entry omits the pinned-path caveat for templates/integrations and the session-restart requirement
  **Location**: Phase 7
  Extend the entry to call out unsupported overrides for `paths.templates` / `paths.integrations`, the preserved-override behaviour for `paths.tmp`, and the recommended session restart.

- 🟡 **Compatibility**: `mv` of directories where destination already partially exists has divergent macOS vs Linux semantics
  **Location**: Phase 3 migration script
  Specify the move semantics explicitly (move contents, not directory; or remove the pre-created `.gitkeep` first). See critical mv-nesting finding.

- 🟡 **Compatibility**: Anchored/unanchored `.gitignore` rule replacement portability and `grep -qFx` behaviour not specified
  **Location**: Phase 3 migration root `.gitignore` rewrite
  Specify the rewrite algorithm in awk/sed terms and add tests covering trailing-whitespace, commented variants, and surrounding gitignore content.

- 🟡 **Documentation**: Init SKILL.md prose updates are incomplete
  **Location**: Phase 4 / Phase 7
  Phase 4 covers lines 20-33 only. Lines 83 (Step 3 heading), 86-97 (rule example), 129 (results template), 136 (closing paragraph) all reference legacy paths. Add init SKILL.md to Phase 7 or extend Phase 4 prose subsection.

- 🟡 **Documentation**: Existing `[Unreleased]` entry for `meta/integrations/jira/` will contradict the new layout
  **Location**: Phase 7 — CHANGELOG.md lines 109-113
  The "lines 109-632 are historical" exemption applies only to shipped versioned headings. Rewrite or fold the Jira `[Unreleased]` entry into the new BREAKING entry.

- 🟡 **Documentation**: CHANGELOG entry omits user-actionable migration details
  **Location**: Phase 7, Section 5
  Missing: pinned-path caveat for `paths.templates`/`paths.integrations`, root `.gitignore` rewrite, init-jira ownership-model change, integration init skill convention.

#### Minor

- 🔵 **Architecture**: Implicit init-skill ordering coupling between `init-jira` and `init` (Phase 5) — "warn but proceed" leaves `.accelerator/` partially owned by neither component.
- 🔵 **Architecture**: Defence-in-depth `.gitignore` duplication adds three write-points for one rule (root, inner, migration).
- 🔵 **Architecture**: `.accelerator/skills/` and `.accelerator/lenses/` are hardcoded with no path-config override key, while sibling `templates`/`tmp`/`integrations` support overrides — asymmetric design.
- 🔵 **Architecture**: Phase-1 test fixture intentionally uses legacy `.claude/accelerator.md`, creating a temporal inconsistency with the new default it asserts.
- 🔵 **Code Quality**: Inner `.accelerator/.gitignore` body is dead defence-in-depth — drop or add a regression test asserting it's load-bearing.
- 🔵 **Code Quality**: Hook fallback chain hand-coded inline rather than abstracted into a `_resolve_migrations_applied_path` helper.
- 🔵 **Code Quality**: `_jira_ensure_gitignore` rewrite mixes path resolution and idempotent file mutation; rule list duplicated between Phase 5 helper and Phase 3 migration. Define `JIRA_INNER_GITIGNORE_RULES` once.
- 🔵 **Code Quality**: `<!-- DIR_COUNT:14 -->` marker semantics deferred to discovery (TBD); resolve at planning time via `grep -rn 'DIR_COUNT'`.
- 🔵 **Code Quality**: Phase 1 test-case template uses `trap RETURN` but tests are flat scripts — would leak temp dirs. Use `EXIT` or explicit cleanup matching `test-config.sh:174`.
- 🔵 **Code Quality**: Stderr-warning string format unspecified; use existing `log_warn` helper rather than raw `>&2 echo`.
- 🔵 **Test Coverage**: Dirty-tree refusal test enumeration does not call out the `.accelerator/` sentinel explicitly — the most novel of the four sentinels.
- 🔵 **Test Coverage**: sha256sum-based idempotency comparison is fragile against directory-walk ordering; reuse `tree_hash()` helper from `test-migrate.sh:26-34`.
- 🔵 **Test Coverage**: Lock-cleanup test asserts post-condition only on success; failure-mode (EXIT trap firing on abnormal termination) is untested.
- 🔵 **Test Coverage**: No test asserts migration 0003 produces same `.accelerator/` scaffold as init.sh — equivalence-invariant absent.
- 🔵 **Test Coverage**: `paths.tmp: meta/tmp/` (trailing slash) variant unspecified — pin behaviour with a test.
- 🔵 **Test Coverage**: Discoverability hook "silent" assertion does not specify whether silence means stderr-empty AND stdout-empty, or just no warning string — assert both.
- 🔵 **Correctness**: Source removal after `mv` is redundant or destructive on partial-failure paths — drop the explicit removal step.
- 🔵 **Correctness**: Partial-state recovery semantics undefined when both legacy and new state files exist — specify a merge rule (union of lines).
- 🔵 **Correctness**: Root `.gitignore` rewrite must idempotently handle anchored AND unanchored variants; current wording could yield duplicates.
- 🔵 **Correctness**: Init's tmp probe uses new default but pre-migration `paths.tmp: meta/tmp` overrides could create `meta/tmp/.gitignore` on a clean repo — overlaps with the architectural divergence finding.
- 🔵 **Safety**: Dirty-tree check should explicitly include `.accelerator/` sentinel in tests for forward-compat with migrations 0004+.
- 🔵 **Safety**: Scaffold creation before moves is destructive on an existing partial `.accelerator/` (no idempotency guards specified for the scaffold-write step).
- 🔵 **Safety**: No documented manual recovery procedure for failed migration — add a runbook reference to CHANGELOG/migration `# DESCRIPTION:`.
- 🔵 **Compatibility**: Driver clean-tree check `git status --porcelain .accelerator/` behaviour for non-existent directory varies across git 2.x versions.
- 🔵 **Compatibility**: bash 3.2 compatibility constraint not asserted — add target-shell version to `scripts/test-helpers.sh` or CI invocation.
- 🔵 **Compatibility**: Visualiser `config.valid.json` fixture update may break e2e tests pinned against fixture content — verify lockstep.
- 🔵 **Compatibility**: Stopping `init-jira` root `.gitignore` mutation leaves orphan rules in repos that initialised Jira post-migration without ever having legacy state — have migration 0003 unconditionally scrub.
- 🔵 **Documentation**: `<!-- DIR_COUNT:14 -->` marker TBD (overlaps with Code Quality finding).
- 🔵 **Documentation**: Superseding ADR for `accelerator.md` → `config.md` deferred without a status-banner update on ADR-0016.
- 🔵 **Documentation**: README Migrations section (lines 105-129) needs structural rework, not just path substitution, to reflect the fallback-chain behaviour.
- 🔵 **Documentation**: `meta/notes/2026-04-29-accelerator-config-state-reorg.md` source proposal will read as if reorg is pending — consider a status banner.
- 🔵 **Documentation** (suggestion): `config-read-path.sh` header parenthetical defaults will diverge from runtime defaults after Phase 6 — update in lockstep.

### Strengths

- ✅ Clear separation of concerns: `init` owns the core scaffold; integration init skills own their own state subtrees — codified as a forward-looking convention rather than left as implicit precedent.
- ✅ Single point of indirection preserved: `config-read-path.sh` remains the centralised path-resolution delegate.
- ✅ Atomic delivery of migration 0003 with driver and discoverability hook updates correctly identified as a single-commit requirement.
- ✅ Phase ordering is internally consistent — documentation-only and pure-refactor commits land first; destructive migration is one atomic commit; source-of-truth runtime switch happens after migration is in place.
- ✅ TDD discipline is explicit per phase, with failing tests preceding logic, mirroring the `test-migrate.sh` and `test-jira-init-flow.sh` precedent.
- ✅ Each commit is required to leave the suite green and is internally consistent (test fixtures move with source updates).
- ✅ Phase 2's init-bootstrap extraction introduces a testable seam where one didn't exist (markdown-only with no test surface).
- ✅ Each migration acceptance criterion in the work item has a corresponding numbered test case in Phase 3 (12 cases mapped to ACs).
- ✅ The new test suite for `hooks/test-migrate-discoverability.sh` covers both pre- and post-migration sentinel paths plus the state-file fallback chain.
- ✅ Discoverability hook is correctly retained as the only fallback path for pre-migration repos.
- ✅ `paths.tmp` pinned-path preservation is explicit and correctly applied only to the one path key with non-trivial override population.
- ✅ Phase 7 enumerates specific line numbers per file rather than vague "update docs" instructions, making coverage auditable.
- ✅ The grep-based clean-room check in Desired End State catches stale legacy-path references in source/skills/hooks, providing an automated guardrail.
- ✅ Naming is consistent with the rest of the codebase (`_jira_ensure_gitkeep`, `_jira_warn_if_accelerator_absent` follow the existing `_jira_*` private-helper convention).
- ✅ The `integrations` path-config key documentation gap is correctly identified as a real defect (runtime already supports the key) and is fixed in its own additive Phase 1 commit.

### Recommended Changes

In priority order, these are the changes that would address the highest-impact findings. Many are interrelated and should be tackled together.

1. **Fix the `paths.tmp` probe semantics (single design decision)** (addresses: Critical/Correctness probe-returns-empty; Major/Correctness probe-depends-on-Phase-6; Major/Safety silent-override-of-explicit-pin; Major/Code-Quality under-specified-detection; Major/Architecture init-vs-migration-divergence; Test case 6 alignment).
   Pick one of:
   - **(a)** Migration reads `paths.tmp` directly from `.claude/accelerator.md` / `.claude/accelerator.local.md` with a simple `grep -E '^[[:space:]]*tmp:'` to detect explicit-set, and only auto-moves when the key is absent. Update test case 6 to assert "explicit `meta/tmp` left untouched".
   - **(b)** Document `paths.tmp == meta/tmp` as default-equivalent (move regardless), warn loudly when this triggers, update CHANGELOG, and apply the same logic in `init.sh` so the two surfaces agree.

2. **Specify directory-move semantics to avoid scaffold collision** (addresses: Critical/Correctness mv-nesting; Critical/Safety destination-clobber; Major/Compatibility mv-portability; Major/Architecture destination-collision-unspecified).
   Either: create only the parent `.accelerator/` and `.accelerator/state/` (and the top-level `.gitignore`) before moves; create per-subdirectory `.gitkeep` files only after moves complete and only for directories not sourced from a legacy tree. OR: use `mv -n` plus per-entry merge logic. Add an explicit test-fixture variant where destinations pre-exist. Reject the move if both source and destination contain content with diverging files.

3. **Add migration preflight, cross-platform safety, and recovery documentation** (addresses: Critical/Safety mid-flight-failure; Major/Safety no-VCS-bypass; Minor/Safety no-recovery-procedure).
   Build the move list at preflight, verify all destinations are writable, refuse on no-VCS unless `ACCELERATOR_MIGRATE_FORCE=1`. On any failure mid-run, emit an explicit per-source recovery message. Add a "Failure recovery" runbook reference in the migration's `# DESCRIPTION:` section.

4. **Standardise idempotency on `grep -qFx` and extract gitignore-helper primitives** (addresses: Major/Correctness grep-qF-substring; Major/Safety gitignore-overwrites-customisations; Major/Code-Quality migration-monolith; Minor/Code-Quality jira-rule-list-duplicated; Minor/Code-Quality hook-hand-coded-inline).
   Lift `_replace_or_append_gitignore_rule`, `_remove_gitignore_rule`, `_resolve_migrations_applied_path`, and `accelerator_init_scaffold` into shared helpers under `scripts/`. Replace every `grep -qF` with `grep -qFx`. Define `JIRA_INNER_GITIGNORE_RULES` once in `jira-common.sh`. Specify how user comments / negated rules / surrounding context are preserved during root-`.gitignore` rewrites.

5. **Close the discoverability gap for mid-session and partial-recovery cases** (addresses: Major/Compatibility hook-only-on-SessionStart; Major/Correctness hook-fallback-gap; Major/Architecture permanent-fallback-unbounded; Minor/Test-Coverage discoverability-silence-assertion).
   Add a one-line legacy-path check at the top of `config-common.sh` that exits non-zero with the migrate directive when `.claude/accelerator.md` exists and `.accelerator/config.md` does not. Make the hook's state-file fallback exist-aware (read whichever state file is present, prefer the new path). Either bound the fallback's lifetime in a follow-up work item or document a sunset criterion in the hook source.

6. **Promote work-item ACs into automated assertions** (addresses: Major/Test-Coverage three-AC-mapping-gaps; Major/Architecture skill-customisation-paths-asymmetric).
   Add `test-config.sh` cases asserting `config-summary.sh`/`config-dump.sh` emit no `.claude/` or `meta/` prefixes. Add `test-jira-paths.sh` (or extend `test-jira-init-flow.sh`) iterating the seven Jira skill files for legacy-path absence. Add a `test-launch-server.sh` case with no `paths.tmp` override asserting `.accelerator/tmp` resolution. Document the asymmetry that `skills`/`lenses` are not pin-able as either deliberate (with rationale) or as an additional path-config key.

7. **Correct driver `mkdir`, state-file move semantics, and clean-tree test coverage** (addresses: Major/Correctness driver-creates-meta; Major/Safety state-file-history-loss; Minor/Correctness source-removal-redundant; Minor/Correctness partial-state-merge; Minor/Safety dirty-tree-includes-accelerator).
   Replace `run-migrations.sh:203` `mkdir -p meta` with `mkdir -p "$(dirname "$STATE_FILE")"` (and same at line 24). Specify the state-file move as read-write-verify-remove rather than `mv`. Drop the redundant post-move `rm` step. Specify partial-state merge as union-of-lines. Confirm test 1 explicitly seeds an uncommitted file under `.accelerator/`.

8. **Complete the documentation sweep** (addresses: Major/Documentation init-SKILL.md-incomplete; Major/Documentation existing-Unreleased-Jira-entry; Major/Documentation CHANGELOG-omits-actionable-details; Major/Compatibility CHANGELOG-omits-pin-caveat; Minor/Documentation README-Migrations-prose; Minor/Documentation ADR-0016-stub; Suggestion/Documentation config-read-path-header-defaults).
   Add `skills/config/init/SKILL.md` to Phase 7 with explicit lines (83, 86-97, 129, 136). Rewrite or fold the existing `[Unreleased]` Jira CHANGELOG entry. Expand the new CHANGELOG entry with: pinned-path caveat for `templates`/`integrations`, root `.gitignore` rewrite, `init-jira` ownership-model change, integration init skill convention, recommended session restart. Update README Migrations prose at 105-129 structurally. Add a status banner to ADR-0016 referencing the forthcoming superseding ADR. Add `config-read-path.sh` to Phase 7 with parenthetical-default updates.

9. **Resolve the `<!-- DIR_COUNT:14 -->` TBD at plan time** (addresses: Minor/Code-Quality, Minor/Documentation).
   Run `grep -rn 'DIR_COUNT' .` now; if parsed by tooling, update count to 12; if unreferenced, remove the marker. Replace the TBD in Phase 4 with the definitive instruction.

10. **Tighten test fixtures and helpers** (addresses: Minor/Code-Quality trap-RETURN; Minor/Test-Coverage sha256-fragility; Minor/Test-Coverage scaffold-equivalence; Minor/Test-Coverage trailing-slash-variant; Minor/Test-Coverage lock-cleanup-failure-mode).
   Use the established `trap EXIT` or explicit `rm -rf` pattern from `test-config.sh:174`. Reuse `tree_hash()` from `test-migrate.sh` for idempotency comparisons. Add a single test asserting `.accelerator/` scaffold from init.sh and migration 0003 are byte-identical. Add `paths.tmp: meta/tmp/` (trailing slash) parametrised case. Add a failure-injection test for the `.lock/` cleanup trap.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan establishes a sound architectural improvement: consolidating four classes of accelerator-owned state into a single `.accelerator/` root, with clear ownership boundaries between the `init` skill (core scaffold) and integration init skills (own subdirectories). Phase ordering, atomic delivery of migration 0003 with its driver/hook updates, and the consistent use of `config-read-path.sh` as the single point of indirection are all architecturally well-handled. However, several boundary and consistency concerns remain: the `init` skill writes one default that diverges from the migration's behaviour for `paths.tmp` overrides, the migration's destination-overwrite semantics are not specified, and the state-file fallback chain creates a long-lived dependency that the plan should explicitly bound.

**Strengths**:
- Clear separation of concerns between `init` core scaffold and integration init subtrees, codified as forward convention.
- Single point of indirection preserved via `config-read-path.sh`.
- Atomic delivery of migration 0003 + driver state-file relocation + discoverability hook update.
- Phase ordering internally consistent; each commit leaves the suite green.
- Hard-cut posture explicitly justified with one principled exception (discoverability fallback).
- Init script extraction (Phase 2) introduces a testable seam where none existed.

**Findings**:
- 🟡 **Major/High**: Inconsistent `paths.tmp` semantics between init and migration (Phase 4 vs Phase 3).
- 🟡 **Major/High**: Destination-collision behaviour unspecified for the directory moves (Phase 3 / Migration 0003).
- 🟡 **Major/Medium**: Permanent fallback chain is an open-ended architectural commitment (Phase 3).
- 🔵 **Minor/High**: Implicit init-skill ordering coupling between `init-jira` and `init` (Phase 5).
- 🔵 **Minor/High**: Defence-in-depth gitignore duplication adds three write-points for one rule (Phase 4).
- 🔵 **Minor/Medium**: Tier-2 skill-customisation directory hardcoded, not routed through `config-read-path.sh` (Phase 6).
- 🔵 **Minor/Medium**: Phase 1 test fixture uses legacy `.claude/accelerator.md` while documenting new default — temporal inconsistency (Phase 1).

### Code Quality

**Summary**: The plan is well-structured and demonstrates strong code-quality awareness: it extracts inline bash from a markdown skill into a testable script (Phase 2), keeps the change-set commit-by-commit green, and respects existing patterns. However, several maintainability concerns surface: Phase 3's migration script consolidates a large number of distinct responsibilities without a decomposition strategy; init.sh and migration 0003 will independently re-implement near-identical scaffold logic; and a few conditional behaviours (`paths.tmp` default-vs-override detection in pure bash 3.2) are hand-waved rather than designed.

**Strengths**:
- Phase 2 deliberate refactor improves testability (inline markdown bash → testable script).
- TDD discipline explicit per phase, mirroring established convention.
- Each commit leaves the suite green and is internally consistent.
- Single points of change correctly identified (`config-common.sh:176`, `jira-common.sh:62`, `config-read-path.sh` delegate).
- Naming consistent with `_jira_*` private-helper convention.
- Atomic-delivery dependencies between commits made explicit.

**Findings**:
- 🟡 **Major/High**: Migration 0003 bundles too many responsibilities into one script with no decomposition.
- 🟡 **Major/High**: `.accelerator/` scaffold logic duplicated between init.sh and migration 0003.
- 🟡 **Major/High**: `paths.tmp` default-vs-override detection under-specified for bash 3.2 (test case 6 contradicts work-item statement).
- 🔵 **Minor/High**: Inner `.accelerator/.gitignore` body is dead defence-in-depth.
- 🔵 **Minor/Medium**: Hook fallback chain hand-coded inline rather than abstracted.
- 🔵 **Minor/Medium**: `_jira_ensure_gitignore` rewrite mixes path resolution and idempotent file mutation; rule list duplicated.
- 🔵 **Minor/Medium**: `<!-- DIR_COUNT:14 -->` marker semantics deferred to discovery (TBD).
- 🔵 **Minor/High**: Test-case template uses `trap RETURN` but tests are flat scripts (would leak temp dirs).
- 🔵 **Minor/Medium**: Stderr-warning string format unspecified; not using existing `log_warn` helper.

### Test Coverage

**Summary**: The plan is unusually strong on test coverage: TDD is applied per phase, each phase enumerates concrete test cases, and the work item's acceptance criteria map well to the planned migration tests. However, several acceptance criteria are not clearly mirrored by a planned test (notably ACs covering config-summary/dump output, launch-server effective tmp path, and absence of hardcoded `meta/integrations/jira/` in all seven Jira skills). A few planned tests rely on filesystem hashes that introduce non-determinism, and the migration test enumeration omits a regression case for the migration framework's clean-tree refusal across the new `.accelerator/` sentinel.

**Strengths**:
- TDD-led ordering explicit (failing tests before implementation).
- Each migration AC has a numbered test case (12 cases mapped).
- `hooks/test-migrate-discoverability.sh` covers pre- and post-migration sentinels and the fallback chain.
- Test fixture posture internally consistent per commit.
- Tests patterned on existing fixture-on-disk approach.

**Findings**:
- 🟡 **Major/High**: ACs for config-summary/dump path-output cleanliness not mapped to a test.
- 🟡 **Major/High**: No planned test verifies seven Jira skill scripts free of hardcoded legacy paths.
- 🟡 **Major/Medium**: Visualiser launch-server effective-tmp-path AC has no explicit assertion.
- 🔵 **Minor/High**: Dirty-tree refusal test enumeration does not call out `.accelerator/` sentinel explicitly.
- 🔵 **Minor/Medium**: sha256sum-based idempotency comparison fragile against directory walk ordering.
- 🔵 **Minor/Medium**: Lock-cleanup test asserts post-condition but not EXIT-trap mechanism under failure.
- 🔵 **Minor/High**: No test asserts migration 0003 produces same `.accelerator/` scaffold as init.sh.
- 🔵 **Minor/Medium**: Edge case test on paths.tmp override — also test trailing-slash variant.
- 🔵 **Minor/Low**: Discoverability hook 'silent' assertion does not specify isolation strategy.

### Correctness

**Summary**: The plan is generally well-structured and identifies the bootstrap problem and several edge cases up front. However, two critical correctness defects exist: (1) the `paths.tmp` probe via `config-read-path.sh tmp ""` returns an empty string (not `meta/tmp`) when no override is set, so the equality comparison the plan describes will incorrectly skip the default-case move; and (2) the migration's order of operations creates `.accelerator/skills/`, `lenses/`, `templates/`, `tmp/` scaffold directories *before* moving `.claude/accelerator/skills/` etc., which causes `mv source dest` to nest the source inside the existing destination directory rather than replacing it. Several secondary correctness concerns exist around `grep -qF` substring matching, partial-state recovery, the discoverability hook fallback chain, and the meta directory still being created post-migration by the driver.

**Strengths**:
- Bootstrap problem of driver pre-reading STATE_FILE correctly identified.
- Edge case `paths.tmp = meta/tmp` literal acknowledged (test case 6).
- Idempotency, dirty-tree refusal, partial-recovery enumerated as test cases.
- Each commit designed to leave the suite green.
- Three-clause OR sentinel correctly preserves pre-migration repo detection.

**Findings**:
- 🔴 **Critical/High**: paths.tmp default-case probe returns empty string, breaking the move condition.
- 🔴 **Critical/High**: Scaffold initialisation creates destination directories before mv, causing nested moves.
- 🟡 **Major/High**: `grep -qF` performs substring match, not exact-rule match — false-positive idempotency.
- 🟡 **Major/High**: Driver still creates legacy `meta/` directory before appending state.
- 🟡 **Major/Medium**: Discoverability hook fallback chain leaves a gap when `.accelerator/` exists but state file missing.
- 🟡 **Major/Medium**: Migration's `paths.tmp` probe depends on which config path `config-read-path.sh` reads at runtime.
- 🔵 **Minor/High**: Source removal after `mv` is redundant and can mask logic errors.
- 🔵 **Minor/Medium**: Partial-state recovery semantics undefined when both legacy and new state files exist.
- 🔵 **Minor/Medium**: Root `.gitignore` rewrite must idempotently handle anchored AND unanchored variants.
- 🔵 **Minor/Low**: Init's tmp probe uses new default but pre-migration repos may have legacy override semantics.

### Safety

**Summary**: The plan correctly leans on the existing clean-tree check + VCS revert pattern as the rollback mechanism, consistent with project convention. However, several concrete data-loss vectors are inadequately addressed: the multi-mv migration is not atomic and has no failure-recovery path beyond VCS, the `paths.tmp == meta/tmp` heuristic silently overrides explicit user intent, destination-clobber on pre-existing `.accelerator/` content is undefined, and the no-VCS-repo case bypasses the only safety net entirely. The gitignore-mutation logic also lacks safeguards against user customisations.

**Strengths**:
- Reuses established clean-tree refusal pattern.
- Atomic delivery of migration 0003 + driver + discoverability hook.
- Adopts `MIGRATION_RESULT: no_op_pending` sentinel for the already-migrated case.
- `paths.tmp` override case acknowledged with conditional logic.
- Migration state recorded so re-runs short-circuit.
- Discoverability hook fallback chain preserved.

**Findings**:
- 🔴 **Critical/High**: Mid-flight `mv` failure leaves repo in unrecoverable mixed state without VCS.
- 🔴 **Critical/High**: Destination clobber when `.accelerator/config.md` already exists with different content.
- 🟡 **Major/High**: No-VCS repos bypass the clean-tree safety net entirely.
- 🟡 **Major/High**: User explicitly pinning `paths.tmp: meta/tmp` silently treated as unset and migrated.
- 🟡 **Major/Medium**: Gitignore mutation overwrites user customisations without preservation.
- 🟡 **Major/High**: Pinned `paths.templates`/`paths.integrations` overrides moved unconditionally with no detection.
- 🟡 **Major/Medium**: State-file relocation during the same run that records migration 0003 — risk of history loss.
- 🔵 **Minor/High**: Dirty-tree check covers `.accelerator/` only after Phase 3 lands; ensure tests seed it.
- 🔵 **Minor/Medium**: Scaffold creation before moves is destructive on existing partial `.accelerator/`.
- 🔵 **Minor/Medium**: No documented manual recovery procedure for failed migration.

### Compatibility

**Summary**: The plan executes a deliberate hard-cut breaking change with migration 0003 as the sole recovery path. The compatibility posture is internally coherent — discoverability hook fallback is correctly identified as the only retained fallback — but several gaps could leave existing users unable to cleanly recover: the discoverability hook is not invoked by every entry point a user might use first, the CHANGELOG entry omits release-gate guidance and the pinned-path caveat for `paths.templates`/`paths.integrations`, and several cross-platform/portability concerns in the proposed migration script are not explicitly addressed.

**Strengths**:
- Discoverability hook correctly retained as the only fallback path.
- Atomic delivery requirement (driver + migration in same commit) explicit.
- `paths.tmp` pinned-path preservation explicit and limited to the one path with non-trivial override population.
- Test coverage for migration 0003 enumerates idempotency, partial-state recovery, dirty-tree refusal, and override branching.
- `paths.integrations` doc-only Phase 1 genuinely additive and safe to ship independently.

**Findings**:
- 🟡 **Major/High**: Discoverability hook only fires on SessionStart — mid-session skill invocations hit raw failures.
- 🟡 **Major/High**: CHANGELOG entry omits pinned-path caveat for templates/integrations and the session-restart requirement.
- 🟡 **Major/Medium**: `mv` of directories where destination already partially exists has divergent macOS vs Linux semantics.
- 🟡 **Major/Medium**: Anchored/unanchored `.gitignore` rule replacement portability and `grep -qFx` behaviour not specified.
- 🔵 **Minor/High**: Driver clean-tree check must include `.accelerator/` for both jj and git branches with correct regex anchoring.
- 🔵 **Minor/Medium**: init.sh patterns are bash 3.2 compatible but not asserted.
- 🔵 **Minor/High**: Visualiser `config.valid.json` fixture update may break e2e tests pinned against fixture content.
- 🔵 **Minor/Medium**: Stopping root `.gitignore` mutation in init-jira leaves orphan rules in some repos.

### Documentation

**Summary**: The plan has a dedicated Phase 7 documentation sweep with explicit line-number coverage, which is unusually thorough for a plan of this size. However, several documentation gaps remain: the init SKILL.md prose is only partially covered, the existing `[Unreleased]` CHANGELOG entry for `meta/integrations/jira/` will ship in the same release as the reorg and contradict the new layout, the new CHANGELOG entry omits practically important migration details, and the deferred `accelerator.md` → `config.md` superseding ADR is not referenced anywhere users will land. The `<!-- DIR_COUNT:14 -->` marker semantics are flagged as TBD without resolution.

**Strengths**:
- Phase 7 enumerates specific line numbers per file rather than vague update instructions.
- Per-phase prose updates colocated with behaviour changes (init-jira in Phase 5, visualise in Phase 6).
- `config-read-path.sh` header doc gap correctly identified and fixed in its own additive Phase 1 commit.
- Desired End State grep-based verification catches stale legacy-path references.
- Plan correctly excludes legacy-path references in immutable historical artefacts.

**Findings**:
- 🟡 **Major/High**: Init SKILL.md prose updates incomplete (lines 83, 86-97, 129, 136).
- 🟡 **Major/High**: Existing `[Unreleased]` entry for `meta/integrations/jira/` will contradict the new layout.
- 🟡 **Major/High**: CHANGELOG entry omits user-actionable migration details (pin caveat, root `.gitignore` rewrite, ownership-model change).
- 🔵 **Minor/High**: `<!-- DIR_COUNT:14 -->` marker semantics flagged TBD without resolution path.
- 🔵 **Minor/Medium**: Superseding ADR deferred without an in-tree placeholder for users who land on ADR-0016.
- 🔵 **Minor/Medium**: README Migrations section needs structural rework, not just path substitution.
- 🔵 **Minor/Medium**: Source proposal note `meta/notes/2026-04-29-accelerator-config-state-reorg.md` will read as if reorg is pending.
- 🔵 **Suggestion/Medium**: `config-read-path.sh` header parenthetical defaults will diverge from runtime defaults after Phase 6.

## Re-Review (Pass 2) — 2026-05-05T08:15:00+00:00

**Verdict:** REVISE

The plan edits substantially address the prior findings: 39 of 61 prior findings are fully resolved, 9 are partially resolved (including the four originally-critical defects, all of which are now closed by the idempotency model + `_move_if_pending` primitive + awk-based `paths.tmp` probe + minimal-scaffold ordering), 6 are still present (mostly minor architectural notes deliberately deferred), and 2 were accepted out-of-scope per the user's earlier direction. However, one previously-suggested change introduced a new **major correctness/compatibility regression**: the mid-session legacy-path detector inside `config_find_files()` calls `exit 1` from within a function that is conventionally invoked via command substitution (`$(config_find_files)`), so the exit fires only inside the subshell and the parent script continues with empty output. The detector therefore does not fire as designed for the very compatibility gap it was added to close. This needs fixing before the plan is implementation-ready.

### Previously Identified Issues

#### Critical (originally)

- 🔴 **Correctness**: `paths.tmp` default-case probe returns empty string — **Resolved**
  Replaced with awk-based parse of `.claude/accelerator.md` / `.local.md` walking the column-0 `paths:` block. Empty-string default-case bug is structurally impossible.
- 🔴 **Correctness**: Scaffold initialisation creates destination directories before mv — **Resolved**
  Plan now explicitly enumerates which directories are NOT pre-created (skills, lenses, templates, tmp, integrations, config files) and creates only `.accelerator/.gitignore` + `state/.gitkeep` in the minimal scaffold. Trailing-scaffold step runs only after moves complete.
- 🔴 **Safety**: Mid-flight `mv` failure unrecoverable without VCS — **Accepted (verified by idempotency model)**
  User direction: idempotency model substitutes for preflight + rollback. Plan now formalises `_move_if_pending` per-pair semantics with non-destructive conflict detection; test 11 covers idempotency from any partial state.
- 🔴 **Safety**: Destination clobber when `.accelerator/config.md` already exists — **Resolved**
  `_move_if_pending` source-present-AND-dest-present branch exits non-zero with reconciliation message; test 11a verifies non-destructive behaviour with differing content.

#### Major (originally)

- 🟡 **Correctness**: `grep -qF` substring match — **Resolved** (now `grep -qFx` everywhere with explicit substring-risk callouts)
- 🟡 **Correctness**: Driver still creates legacy `meta/` — **Resolved** (line 24/203 `mkdir -p` rewritten to use `dirname STATE_FILE`)
- 🟡 **Correctness**: Hook fallback chain gap — **Resolved** (per-file existence-aware; test case 6 covers partial-recovery state)
- 🟡 **Correctness**: Migration `paths.tmp` probe Phase-6 dependency — **Resolved** (now reads legacy files directly via awk)
- 🟡 **Safety**: No-VCS bypass — **Accepted out-of-scope** per user direction; recovery posture now documented
- 🟡 **Safety**: `paths.tmp: meta/tmp` literal silently migrated — **Resolved** (awk probe treats any explicit value as override; tests 6/6a/6b)
- 🟡 **Safety**: Gitignore mutation overwrites customisations — **Resolved** (whole-line match, refuse-on-trailing-content via test 8a)
- 🟡 **Safety**: Pinned `paths.templates`/`paths.integrations` no detection — **Partially resolved** (warn-and-proceed via `log_warn`; new minor finding: stderr-only signal is fragile if missed)
- 🟡 **Safety**: State-file relocation history loss — **Resolved** (read-write-verify-remove with union-merge; test 12)
- 🟡 **Architecture**: `paths.tmp` init/migration divergence — **Resolved** (both surfaces honour explicit pin)
- 🟡 **Architecture**: Destination-collision behaviour unspecified — **Resolved** (`_move_if_pending` four-state semantics + test 11a)
- 🟡 **Architecture**: Permanent fallback chain unbounded — **Partially resolved** (now framed as deprecation-track with follow-up work item; no concrete sunset trigger named)
- 🟡 **Code Quality**: Migration 0003 monolithic — **Partially resolved** (scaffold extracted to shared helpers; migration body still flat — recommend named step functions in script)
- 🟡 **Code Quality**: Scaffold logic duplicated — **Resolved** (`scripts/accelerator-scaffold.sh` shared)
- 🟡 **Code Quality**: `paths.tmp` under-specified for bash 3.2 — **Resolved** (awk approach + test cases 6/6a/6b)
- 🟡 **Test Coverage**: ACs for config-summary/dump cleanliness not tested — **Resolved** (Phase 6 / 4b)
- 🟡 **Test Coverage**: 7 Jira skills not tested for legacy paths — **Resolved** (new `test-jira-paths.sh`)
- 🟡 **Test Coverage**: Launch-server effective-tmp-path AC not tested — **Resolved** (Phase 6 / 4b)
- 🟡 **Compatibility**: Discoverability hook only on SessionStart — **Resolved (in design)** but **see new finding** below: implementation via `exit 1` inside `config_find_files` is bypassed by standard `$(...)` callers
- 🟡 **Compatibility**: CHANGELOG omits pin caveat / session restart — **Resolved**
- 🟡 **Compatibility**: `mv` cross-platform divergence — **Resolved** (move-target dirs not pre-created; `_move_if_pending` semantics explicit)
- 🟡 **Compatibility**: Anchored/unanchored `.gitignore` portability — **Resolved**
- 🟡 **Documentation**: Init SKILL.md prose incomplete — **Resolved** (Phase 4 / 3 covers lines 83/86-97/129/136)
- 🟡 **Documentation**: `[Unreleased]` Jira CHANGELOG entry contradicts new layout — **Resolved** (Phase 7 / 5(a) rewrites it)
- 🟡 **Documentation**: CHANGELOG omits user-actionable details — **Resolved** (Phase 7 / 5(b) covers all callouts)

#### Minor (originally)

- 🔵 **Architecture**: init-jira/init "warn but proceed" coupling — **Still present** (deliberate choice; coupling persists)
- 🔵 **Architecture**: Defence-in-depth `.gitignore` duplication — **Resolved** (shared helper prevents drift; rationale documented)
- 🔵 **Architecture**: skills/lenses asymmetric — **Resolved** (asymmetry now explicit + documented in `What We're NOT Doing`)
- 🔵 **Architecture**: Phase-1 fixture temporal inconsistency — **Partially resolved** (now explicitly justified)
- 🔵 **Code Quality**: Hook fallback chain hand-coded inline — **Still present** (no helper extracted)
- 🔵 **Code Quality**: `_jira_ensure_gitignore` rule list duplicated — **Partially resolved** (shared `JIRA_INNER_GITIGNORE_RULES`; migration keeps parallel local copy by design — recommend equality test)
- 🔵 **Code Quality**: `<!-- DIR_COUNT:14 -->` TBD — **Resolved** (both parser sites identified; Phase 4 / 3 prescribes lockstep update to 12; **but** Phase 4 Overview wording still says TBD)
- 🔵 **Code Quality**: `trap RETURN` template — **Resolved** (now `trap EXIT` with explicit cleanup function)
- 🔵 **Code Quality**: Stderr warning format unspecified — **Resolved** (uses `log_warn`; test asserts substring of fixed phrase)
- 🔵 **Test Coverage**: Dirty-tree `.accelerator/` sentinel — **Resolved** (test 1 explicitly seeds)
- 🔵 **Test Coverage**: sha256sum fragility — **Partially resolved** (Phase 2/3 now use `tree_hash`; Phase 4 case 5 still says sha256sum)
- 🔵 **Test Coverage**: Lock-cleanup failure mode — **Resolved** (test case 8a injects forced failure)
- 🔵 **Test Coverage**: Scaffold equivalence init.sh ↔ migration — **Partially resolved** (mitigated structurally via shared helpers; no explicit byte-equivalence test)
- 🔵 **Test Coverage**: paths.tmp trailing-slash variant — **Resolved** (test case 6a)
- 🔵 **Test Coverage**: Hook silent-assertion isolation — **Resolved** (asserts both stderr AND stdout independently)
- 🔵 **Correctness**: Source removal redundant — **Resolved** (dropped from plan)
- 🔵 **Correctness**: Partial-state state-file semantics — **Resolved** (union merge specified; test 12)
- 🔵 **Correctness**: Anchored/unanchored gitignore idempotency — **Resolved**
- 🔵 **Correctness**: Init's tmp probe legacy-override — **Resolved** (mid-session detector blocks init on legacy-only repos; *but see new finding about init.sh failing opaquely on legacy-only*)
- 🔵 **Safety**: Dirty-tree includes `.accelerator/` in tests — **Resolved**
- 🔵 **Safety**: Scaffold-before-move destructive — **Resolved** (minimal scaffold)
- 🔵 **Safety**: No recovery procedure documented — **Resolved** (Migration Notes + CHANGELOG)
- 🔵 **Compatibility**: Driver clean-tree includes `.accelerator/` — **Resolved**
- 🔵 **Compatibility**: bash 3.2 not asserted — **Still present** (no version-floor test added)
- 🔵 **Compatibility**: Visualiser fixture lockstep — **Partially resolved** (line numbers enumerated; e2e-test impact not verified)
- 🔵 **Compatibility**: init-jira root `.gitignore` orphan rules — **Resolved** (migration scrubs unconditionally)
- 🔵 **Documentation**: `<!-- DIR_COUNT -->` TBD — **Resolved** (overlap with Code Quality finding)
- 🔵 **Documentation**: ADR-0016 status banner — **Resolved** (Phase 7 / 4b)
- 🔵 **Documentation**: README Migrations structural rework — **Resolved** (Phase 7 / 1 explicit)
- 🔵 **Documentation**: Source proposal note status — **Still present**
- 🔵 **Documentation/Suggestion**: `config-read-path.sh` header parentheticals — **Resolved** (Phase 7 / 4a)

### New Issues Introduced

The most consequential is one major correctness/compatibility issue introduced by the new mid-session detector; the rest are minor or documentation-level cleanups.

#### Major

- 🟡 **Compatibility/Correctness/Code-Quality** (flagged by 3 lenses): `config_find_files()` mid-session detector's `exit 1` is bypassed by the standard `$(config_find_files)` caller pattern.
  **Location**: Phase 6 / Section 1b
  `config_find_files` is invoked by callers (e.g. `config-read-value.sh`, every `config-read-*.sh`) via command substitution. `exit 1` inside a subshell only kills the subshell, leaving the parent with empty captured output and the user with a downstream "no config found" failure rather than the actionable migrate directive. The compatibility shim therefore does not fire as designed.
  **Fix**: Extract the legacy-path check into a separate `config_assert_no_legacy_layout` function called explicitly by entry-point scripts (or a wrapper `scripts/config-precheck.sh`). Keep `config_find_files` a pure path-enumerator. Add a test that runs a real caller (e.g. `bash scripts/config-read-path.sh`) against a legacy-only fixture and asserts non-zero exit on the parent process.

#### Minor

- 🔵 **Correctness**: awk probe's multi-file precedence and trailing-comment handling are implicit
  **Location**: Phase 3 / Section 1
  The probe spec shows the awk body but not the bash wrapper that runs it per-file with local-overrides-team precedence; value extraction does not strip trailing comments, so a pinned-override warning could include `# legacy default` in the displayed value.
  **Fix**: Specify the wrapper explicitly; pipe the awk output through `sed 's/[[:space:]]*#.*$//; s/[[:space:]]*$//'` for value extraction.

- 🔵 **Correctness**: Mid-session detector causes init.sh to fail opaquely on legacy-only repos
  **Location**: Phase 6 / 1b interaction with Phase 4 init.sh
  init.sh invokes `config-read-path.sh` for every directory key; on a legacy-only repo this propagates the detector's exit-1 through init's `set -euo pipefail`, surfacing as an opaque non-zero exit.
  **Fix**: Either document this as intended behaviour (init refuses on un-migrated repos) or have init detect legacy state directly and emit a clearer "init does not run on un-migrated repos" message. Add a test for the chosen behaviour.

- 🔵 **Correctness**: Conflict-branch exit can leave earlier successful moves applied
  **Location**: Phase 3 / Section 1 `_move_if_pending`
  The conflict reconciliation message (per-pair) may mislead users into thinking nothing has been moved, when in fact pairs 1..N-1 succeeded.
  **Fix**: Either reword the conflict message to say "this and N-1 prior moves may have been applied" + recommend VCS revert, or perform a preflight conflict check before the move loop starts.

- 🔵 **Safety**: Conflict reconciliation guidance is under-specified for directory moves
  **Location**: Phase 3 / Section 1 `_move_if_pending`
  Telling a user to "merge or remove one" of two divergent directory trees risks silent customisation loss. No diff is emitted, no algorithmic guidance provided.
  **Fix**: Emit `diff -r` or `find` listings of both sides in the reconciliation message; explicitly direct the user to VCS-revert before reconciling.

- 🔵 **Safety**: Pinned `paths.templates`/`paths.integrations` warning is stderr-only
  **Location**: Phase 3 / Section 1 pinned-override warn
  If the warning is missed (CI log truncation, automated runs), the user ends up with two divergent copies and no in-band detection.
  **Fix**: Persist the notice to `.accelerator/state/migration-notices` so the user can audit post-migration; or escalate to non-zero exit with explicit `ACCELERATOR_MIGRATE_ACCEPT_PIN_LOSS=1` opt-in.

- 🔵 **Architecture**: `config_find_files` exit overload conflates query with policy
  **Location**: Phase 6 / 1b (same as the major Compatibility finding)
  Same fix: extract a separate assertion helper.

- 🔵 **Test Coverage**: Phase 4 init idempotency test still uses `sha256sum` instead of `tree_hash()`
  **Location**: Phase 4 / Section 1, test case 5
  Inconsistent with Phase 2 case 4 and Phase 3 case 11 which use the canonical `tree_hash` helper.
  **Fix**: Update Phase 4 case 5 to reuse `tree_hash`.

- 🔵 **Test Coverage**: No explicit byte-equivalence test ties migration's `.accelerator/` tree to init.sh's
  **Location**: Phase 3 / 1b + Phase 4
  Shared helpers reduce drift but the cross-surface invariant is not asserted.
  **Fix**: Add a test that runs init on a fresh repo, runs migration on a fully-seeded legacy fixture, and asserts the resulting `.accelerator/` subtrees are `tree_hash`-equal modulo populated extension-point contents.

- 🔵 **Test Coverage**: No test asserts migration script can source `config-common.sh` on legacy-only repo without triggering exit
  **Fix**: Add a test sourcing `config-common.sh` from a legacy-only fixture and asserting return 0 with no error message — pins the no-side-effect-at-source contract.

- 🔵 **Test Coverage**: No automated assertion that `config-read-path.sh` header parentheticals match runtime defaults
  **Fix**: Add a small grep test asserting the header parenthetical for each documented key matches the canonical caller's second-arg default.

- 🔵 **Code Quality**: Migration 0003 body still described as a long flat sequence despite extracted helpers
  **Fix**: Specify named step-functions inside the script (`_step_preflight`, `_step_root_gitignore`, `_step_relocate_state_file`, etc.) mirroring the `0002-...sh` shape.

- 🔵 **Code Quality**: `JIRA_INNER_GITIGNORE_RULES` shared but migration keeps a parallel local copy
  **Fix**: Add a test asserting byte-equality between the migration's local copy and `jira-common.sh`'s canonical array.

- 🔵 **Compatibility**: Cross-device `mv` failure (`EXDEV`) undocumented in `_move_if_pending`
  **Fix**: Either document `EXDEV` as a supported abort with the VCS-revert recovery, or use `cp -a + rm -rf` semantics for cross-device support.

- 🔵 **Documentation**: DIR_COUNT TBD wording lingers in Phase 4 Overview
  **Location**: Phase 4 Overview
  Section 3 resolves the marker but the Overview still says "(TBD — verify whether anything else parses this marker)".
  **Fix**: Replace the Overview line with the resolved instruction.

- 🔵 **Documentation**: Owner phase for `JIRA_INNER_GITIGNORE_RULES` is ambiguous
  **Location**: Phase 3 Section 1 / Phase 5 Section 2 / Phase 6 Section 2
  Three phases reference the array; none explicitly says "add to jira-common.sh in this phase".
  **Fix**: Pin the introduction to Phase 5 (where it is first consumed by runtime code) and update Phase 3's cross-reference comment to match.

- 🔵 **Documentation**: Source proposal note `meta/notes/2026-04-29-accelerator-config-state-reorg.md` lacks status banner
  **Location**: References — **Still present from prior review**
  **Fix**: Add a one-line status banner in Phase 7 alongside the ADR-0016 banner edit.

- 🔵 **Documentation/Suggestion**: `scripts/accelerator-scaffold.sh` header contract not specified
  **Location**: Phase 3 / 1b
  No requirement for a file-level header doc comment naming callers / idempotency contract.
  **Fix**: Require a header doc comment naming the two callers + the idempotency contract; one-liner above each helper signature.

### Assessment

The plan is much closer to implementation-ready than after pass 1: every original critical defect is closed, the recovery posture is coherent, and the test coverage for AC-mapped invariants is now adequate. The single material problem is the `config_find_files` exit-from-subshell bug — a regression introduced by my own edit, identified independently by 3 lenses (Compatibility high-confidence major, Architecture and Code Quality medium-confidence). It must be fixed before implementation: extract the legacy-path check into a separate `config_assert_no_legacy_layout` (or `scripts/config-precheck.sh`) called explicitly from entry-point scripts, leaving `config_find_files` a pure enumerator.

Beyond that one fix, the residual minors are low-cost cleanups (Phase 4 sha256sum → tree_hash, Phase 4 Overview DIR_COUNT wording, awk probe wrapper specification, source-proposal note status banner, named step-functions in migration body, scaffold equivalence test). Recommend one more small revision pass to land these, then the plan can be approved.
