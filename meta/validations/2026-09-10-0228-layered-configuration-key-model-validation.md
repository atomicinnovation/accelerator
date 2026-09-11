---
type: "plan-validation"
id: "2026-09-10-0228-layered-configuration-key-model-validation"
title: "Validation Report: Layered Configuration Key Model Implementation Plan"
date: "2026-09-10T23:59:02+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-10-0228-layered-configuration-key-model"
tags: ["configuration", "work-management", "migration", "tracker"]
last_updated: "2026-09-10T23:59:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Layered Configuration Key Model Implementation Plan

All seven phases are implemented and committed (`vlyrntrz`..`uyprzlpr`), every
automated gate passes, and all twelve acceptance criteria have named,
passing tests. Result: **pass**. Three deviations from the plan text survive,
all minor and none breaking an acceptance criterion; the load-bearing one is an
incomplete domain rename that leaves `project` vocabulary in two error paths.

### Implementation Status

- ✅ Phase 1: Config schema (three keys) — fully implemented
- ✅ Phase 2: `{key}` pattern token — fully implemented
- ✅ Phase 3: `work.key` prefix + deprecation-alias helper — implemented; two deviations
- ✅ Phase 4: Jira scope-key ownership — fully implemented
- ✅ Phase 5: Linear scope-key ownership + sync dispatch — fully implemented
- ✅ Phase 6: Init writeback — fully implemented
- ✅ Phase 7: `m0009` migration — fully implemented

### Automated Verification Results

| Gate | Command | Result |
|------|---------|--------|
| Format + lint + types | `mise run cli:check` | 🟢 exit 0 |
| Public-API snapshots | `mise run public-api:check` | 🟢 exit 0 |
| Visualiser frontend | `mise run frontend:check` | 🟢 exit 0 |
| CLI unit + integration | `mise run test:unit:cli` | 🟢 2938 passed, 1 skipped, 0 failed |

The one skip is `test:integration:tracker-contract` (needs live tracker
credentials, out of the roll-up by design).

⚠️ Not run here: the full `mise run` default (docs lane needing network +
Chromium, Playwright e2e, and the Python `tasks/` suite). The Rust workspace
holds all 0228 logic and is fully green; the frontend TypeScript change is
covered by `frontend:check`; the `config dump` golden refresh is covered inside
the passing CLI suite. The docs/e2e/Python lanes are unverified in this run —
run the full default before merge.

### Code Review Findings

#### Matches Plan

- **Catalogue** adds `work.key` (empty scalar), `jira.project_key`,
  `linear.team_key`; count test renamed and bumped to 56 (`catalogue.rs:102,127,130,260,267`).
- **`{key}` token** recognised in both pattern engines; renames applied
  (`TokenKind::Key`, `is_valid_key`, `PatternError::MissingKey`/`BadKeyValue`,
  `ParsedId.key`); byte-identical output to `{project}` asserted
  (`work_item_pattern.rs:901`).
- **`references_key` is brace-aware** — walks a tokeniser, excludes escaped
  `{{key}}`; the recognised-spelling set is single-sourced in `corpus`
  (`KEY_TOKEN_SPELLINGS`, `work_item_id.rs:198`) and re-exported; the two-engine
  parity test asserts both predicates agree across a nested-brace corpus
  (`parity.rs:67`).
- **Alias helper** `resolve_with_deprecated_fallback` / `AliasedScalar` /
  `REMOVAL_RELEASE = "1.25.0"` present and re-exported; `work.integration`
  gate returns `None` on mismatch; the removal-release-vs-`CARGO_PKG_VERSION`
  later-minor test exists (`legacy_alias.rs:18,57,278`).
- **Field gating (AC #7)** — `scheme.key` set only when the pattern references
  the prefix token (`config.rs:77`, test `resolve_scheme_gates_the_prefix_field_on_a_bare_numeric_pattern`).
- **`work_key_required` (AC #2)** names the token, states independence from the
  scope key, points to `.accelerator/config.md` (`config.rs:28`).
- **Once-per-command warning** via process-global dedup on the deprecated key
  name (`emit_deprecation_once`, `legacy_alias.rs:101`).
- **Jira/Linear ownership** — `project_code` and `team_key` resolve their own
  section's key via the gated helper; Linear precedence is
  `linear.team_key` → gated legacy → catalogue `/team/key` (`auth.rs`).
- **Sync dispatch** — extracted `resolve_active_scope_key` dispatches on
  `work.integration`; the default arm (trello / github-issues / unset)
  preserves the legacy read; Linear routes through the catalogue-aware resolver
  so `sync.rs` and `auth.rs` never diverge (`sync.rs:702`).
- **Init writeback** — Linear offers a confirmed overwrite, is fail-safe with
  no TTY, and exposes `--force`; the confirmer is injectable and tested
  (`init_writeback.rs`).
- **`m0009`** — read-once snapshot, pattern-conditional `work.key`
  materialisation, scope-key insert that never clobbers a pinned key,
  collision-only mixed-state abort, `0600` on the personal file + backup,
  always `Applied` (never `NoOpPending`); eight e2e arms
  (`m0009.rs`, `migration_0009.rs`).

#### Deviations from Plan

- **Incomplete `project`→`key` rename in two error paths.** Plan Phase 3 §2
  called for the project-vocabulary error strings to move. `work-cli`'s
  `create.rs`/`next_number.rs` moved to `E_PATTERN_MISSING_KEY`, but
  `E_PATTERN_MISSING_PROJECT` still fires on the canonicalise path
  (`cli/work-cli/src/canonicalise_id.rs:54`), and the lower `work` crate keeps
  `AllocationError::MissingProject` plus its "the pattern needs `{project}`"
  docstring (`cli/work/src/next_number.rs:20,71,88,207`; pinned in the passing
  public-api snapshot). Behaviourally harmless; violates the strict-DDD
  convention the plan invoked.
- **Warning channel differs from the sketch.** `resolve_scheme` returns a bare
  `WorkItemIdScheme` and emits via `emit_deprecation_once` (stderr,
  process-global dedup) rather than the planned `Resolved<T>` +
  `ScalarView.warnings`/`render::emit` channel (`config.rs:52`). The warning is
  emitted and deduped, so AC #9/#10 hold; the deviation is mechanism, and the
  process-global dedup arguably satisfies once-per-command more robustly than
  per-caller threading.
- **Scheme helpers gate on the resolved field, not the predicate.**
  `is_canonical_id_token` / `normalise_id` / `extract_id` branch on `self.key`
  rather than routing through `references_key` (`work_item_id.rs:47,105,176`).
  Spelling-agnostic in effect because the prefix is already resolved into the
  field; a scope difference from the plan wording, not a defect.
- **Cosmetic naming** — `backup_once` vs the plan's `backup_config_once`
  (`m0009.rs:150`); `--force` is OR'd ahead of and short-circuits the confirmer
  rather than routed "through" it (`init_writeback.rs:83`);
  `resolve_active_scope_key` carries an extra `integration` parameter.

#### Potential Issues

- **Legacy-present-and-ignored warning is not distinct.** When the canonical
  key resolves and the legacy key is also set, the same deprecation text is
  emitted (`legacy_alias.rs:27`); the plan wanted a distinct "legacy key
  present and ignored" message. Minor UX; the user is still told to remove the
  legacy key.
- **Linear writeback coverage is unit-level.** AC #11/#12 are exercised over
  `write_team_key` unit tests, not a live `init discover` integration test; the
  Jira section write is skill-driven (`init-jira/SKILL.md` Step 5) and manual —
  a documented asymmetry, not a gap, but worth a manual pass.

### Manual Testing Required

1. Minting and legacy rendering:
   - [ ] Tracker-less repo, `id_pattern: "{key}-{number:04d}"`, `work.key: PP` → `accelerator work create` mints `PP-0001` (AC #6).
   - [ ] Legacy repo (`default_project_code: PP`, `{project}` pattern) still renders `PP-0001` and prints one deprecation warning naming 1.25.0 (AC #10).
2. Integration skills on own-section-only config:
   - [ ] `accelerator jira search` / `show` against `jira: { project_key: ... }` with no `work.*` (AC #1).
   - [ ] `accelerator work sync` against a Linear repo scopes discovery from `linear.team_key` and imports untracked remotes (0220 regression).
3. Init writeback:
   - [ ] `/init-jira` and `/init-linear` write the discovered key into `.accelerator/config.md`, overwriting a stale value (AC #11/#12).
4. Migration:
   - [ ] `/accelerator:migrate` on a real legacy repo materialises canonical keys at the original config level, leaves a `.0009.bak`, tracker tooling keeps working; a second run is a no-op.

### Recommendations

- **Finish the domain rename** before the 1.25.0 removal work builds on it:
  `E_PATTERN_MISSING_PROJECT` → `_KEY` on the canonicalise path, and
  `AllocationError::MissingProject` → `MissingKey` in `cli/work` (regenerate the
  `work` public-api snapshot). This closes Phase 3 §2 and the strict-DDD gap.
- **Run the full `mise run`** before merge to cover the docs, Playwright-e2e,
  and Python lanes this validation did not exercise. The plan's Phase 7 note
  records only the environment-only signing-test failure as expected red.
- **Consider the distinct legacy-present warning** if AC intent requires the
  user to distinguish an ignored-legacy key from a resolved-legacy fallback.
