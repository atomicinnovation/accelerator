---
type: "plan-review"
id: "2026-10-15-0302-work-item-re-key-command-review-1"
title: "Plan Review: Work Item Re-Key Command Implementation Plan"
date: "2026-10-15T23:21:21+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-15-0302-work-item-re-key-command"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "usability", "compatibility"]
review_number: 1
review_pass: 5
tags: ["work", "rekey", "retirement"]
last_updated: "2026-10-16T16:38:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Work Item Re-Key Command Implementation Plan

**Verdict:** REVISE

The plan has a sound shape: pure decisions in `work::rekey`, I/O in
`work-adapters`, resume by reusing retirement's interrupted-target re-planning,
full AC traceability, and claims about existing code that the correctness lens
verified as accurate. Its weaknesses cluster around the run-plan lifecycle's
edges — an in-progress plan has no exit when an entry becomes unappliable, the
unguarded `work create` allocator can manufacture exactly that, state checks
run outside the lock, and the write set omits the tracked sync baseline so a
real resume trips the dirty-tree refusal. A second cluster is the
keyword/exit-code contract: thirteen outcomes spread over six types, three
frozen, with existing skills unaware of the new exit-4 refusal.

### Cross-Cutting Themes

- **No exit from a stuck in-progress plan** (flagged by: safety, correctness,
  usability, compatibility, test-coverage) — `retirement-failed`,
  `retirement-incomplete`, an unnamed mid-run `FinishFailure::Refused`, a
  malformed plan and a cross-version schema all leave a plan `--discard`
  refuses, blocking sync/promote/push. Manual deletion of the file is the only
  way out and reintroduces the duplication hazard.
- **Unguarded allocators take reserved targets** (flagged by: architecture,
  safety, correctness) — `work create` without `--push` allocates from
  filenames, can mint a pending entry's target after an interruption, and
  turns the resume into a permanent `IdTaken`.
- **Run-plan entries lose the retirement kind** (flagged by: architecture,
  code-quality) — `RekeyEntry { old, new, external_id }` forces `execute` to
  re-derive renumbering vs tracker key vs draft from string shape or live
  config; `ACC-0042` is tracker-shaped, so the inference is ambiguous.
- **Keyword/exit-code contract is fragmented** (flagged by: code-quality,
  usability, test-coverage, compatibility) — 13 keywords across `RekeyRefusal`,
  `SyncBlocker`, `WorkingCopyRefusal`, `StoreError`, `FinishFailure`,
  `ExecutionFailure`, `Unreachable`; only 3 frozen; kebab vs `E_REKEY_*`
  printed form undefined; `retirement-failed` is 1 here and 71 elsewhere.
- **Existing skills misread the new exit 4** (flagged by: usability,
  compatibility) — `sync-work-items`, `create-work-item`, `extract-work-items`
  read 4 as "items await a human"; `create-batch --push` retires drafts but is
  not guarded.
- **Checks before the lock** (flagged by: safety, architecture) — `--discard`,
  `--preview` and apply read plan state before taking the retirement lock;
  `refuse_during_rekey` runs once before sync's long write phase.
- **Fingerprint too narrow** (flagged by: safety, test-coverage, correctness)
  — added/removed items and the resolved scheme (`id_pattern`, `key`) are not
  pinned, so a stale preview can fail part-way instead of refusing up front.
- **`ACCELERATOR_FORCE` truthiness and silence** (flagged by: usability,
  safety) — any non-empty value forces (including `0`), nothing is printed
  when it bypasses the guard, and the skill does not forbid setting it.

### Tradeoff Analysis

- **Compatibility vs safety on `forces`**: compatibility credits carrying over
  migrate's `!is_empty()` rule unchanged; safety and usability want `1`/`true`
  only for the new generic name. Recommendation: keep `!is_empty()` for the
  deprecated `ACCELERATOR_MIGRATE_FORCE`, tighten `ACCELERATOR_FORCE`, and warn
  on stderr whenever force bypasses a refusal.
- **`create --push` refusal vs save-intent contract**: refusing protects the
  half-applied mapping; saving locally honours `create --push`'s documented
  promise but leaves an untracked draft that fails the resume's write-set
  check. Recommendation: keep the refusal and document it as a deliberate
  exception, with a skill recovery step.

### Findings

#### Major

- 🟡 **Correctness**: Write set omits the tracked sync baseline, so resuming hits the dirty-tree refusal
  **Location**: Phase 5: Adapters (`write_set`), Composition (resume path)
  `RenameBaseline` rewrites tracked `.accelerator/state/integrations/<i>/last-sync.json`; after one synced entry completes and a later one is interrupted, resume refuses `dirty-working-copy`.
- 🟡 **Safety / Correctness / Usability / Compatibility**: No supported way out of an in-progress plan that keeps failing
  **Location**: Exit codes and keywords; Run-plan lifecycle; Phase 5 `execute`; Phase 6 `--discard`
  Mid-run `Refused` has no keyword; malformed or other-schema plans and repeated `retirement-failed` leave sync/promote/push blocked with only manual deletion as escape.
- 🟡 **Architecture / Safety / Correctness**: `work create` can take a fresh number an interrupted run has reserved
  **Location**: Decisions (guard reach); What We're NOT Doing
  `next_number::allocate` scans filenames only; reserved targets live in the gitignored plan.
- 🟡 **Correctness**: Exact-match target-shape test conflicts with retirement's case-insensitive identity
  **Location**: Key Discoveries; Phase 4 mapping; Phase 7 `tracker_mapping`
  `acc-0042 → ACC-0042` is a self-retirement to `same_id`; on case-insensitive FS it rolls back forever, on resume it can `Remove` the item's only file.
- 🟡 **Architecture / Code Quality**: `RekeyEntry` does not record the retirement kind
  **Location**: Phase 4 Module; Phase 7 `execute`
  Executor must infer kind from string shape or live config; `external_id` duplicates or is ignored.
- 🟡 **Code Quality / Test Coverage / Usability**: Keyword/exit-code contract fragmented and mostly unfrozen
  **Location**: Exit codes and keywords; Phase 4 `keyword_exit_codes.rs`; Phase 5 Composition
  13 outcomes over six types; only `RekeyRefusal`'s three are pinned; printed `E_REKEY_*` vs kebab keywords undefined.
- 🟡 **Usability / Compatibility**: New exit-4 refusals change meaning for skills that branch on 4
  **Location**: Phase 5 `refuse_during_rekey`; Phase 8 Docs
  Three skills and the `exit_codes.rs` band read 4 as "items await a human"; none is updated.
- 🟡 **Safety**: Run-plan state transitions check state before taking the retirement lock
  **Location**: Phase 6 flow
  Concurrent `--discard`/`--preview` can delete or replace a just-started plan.
- 🟡 **Safety**: Sync/promote/push guard checked once, before the lock, while those commands keep writing
  **Location**: Phase 5 `refuse_during_rekey`
  A sync past the guard can pull into paths a concurrent rekey has renamed.
- 🟡 **Safety / Test Coverage**: Fingerprint misses added/removed items; apply does not re-check collisions
  **Location**: Decisions (Fingerprint); Phase 4 `run_plan.rs`; Phase 6 apply
  A new item at a previewed target is caught only mid-run.
- 🟡 **Safety**: A lost run-plan file silently brings back the duplication hazard
  **Location**: Key Discoveries; Phase 5 "none" branch
  `git clean -X` or a manual delete leads to a fresh mapping over a half-converged corpus.
- 🟡 **Architecture**: Dirty-tree check covers fixed paths; rekey writes to configured corpus directories
  **Location**: Phase 2 `GUARDED_SCOPES`; Phase 5 `guarded_working_copy`
  Custom `paths.*` outside `meta/` are rewritten unguarded.
- 🟡 **Test Coverage**: `evals.json` without `benchmark.json` fails the real-skills-tree guard
  **Location**: Phase 8 Failing checks
  `test_evals_structure::test_the_real_skills_tree_passes` turns `test:unit:tasks` red.
- 🟡 **Test Coverage**: Reading `ACCELERATOR_FORCE` inside `rekey::run` makes in-process tests racy
  **Location**: Phase 5 Composition
  Process-global env mutation races parallel tests and is `unsafe` in Rust 2024.
- 🟡 **Test Coverage**: Resume tested against one hand-crafted partial state
  **Location**: Phase 5 `rekey_execute.rs`
  Drive `execute` with the `Faulty` store at each write index instead.
- 🟡 **Test Coverage**: No collision tests under `{tracker}` mapping
  **Location**: Phase 7 `mapping.rs`
  Dropping the collision call from `tracker_mapping` would pass every listed test.
- 🟡 **Test Coverage**: Malformed plan and `rekey_in_progress` behaviour untested
  **Location**: Phase 5 Adapters; Exit codes section
- 🟡 **Usability**: Skill has no path for an in-progress plan, the main recovery case
  **Location**: Phase 8 Skill steps
- 🟡 **Usability**: Remedy text required by ACs is mostly unspecified and untested
  **Location**: Phase 5/6 Failing tests
- 🟡 **Code Quality**: `RunPlan` allows invalid states (public fields, orphaned `write_set`)
  **Location**: Phase 4 Module; Phase 6 apply flow
- 🟡 **Compatibility**: Run-plan schema has no cross-version read rule
  **Location**: Phase 4 `RUN_PLAN_SCHEMA`

#### Minor

- 🔵 **Correctness**: Phase 6 apply does not say the in-progress state is saved before the first write — **Location**: Phase 6 apply flow.
- 🔵 **Correctness**: Fingerprint does not pin `id_pattern`/`key` — **Location**: Decisions (Fingerprint).
- 🔵 **Correctness**: Kept sequence numbers beyond pattern width not refused — **Location**: Phase 4 mapping.
- 🔵 **Correctness**: Scope-boundary test could break prefix matching that guards `.claude/accelerator.md` — **Location**: Phase 2 tests.
- 🔵 **Correctness**: Floor ignores bare legacy aliases that canonicalise into target shape — **Location**: Phase 4 mapping.
- 🔵 **Usability / Compatibility**: `retirement-failed` exits 1 here, 71 under sync/promote; `retirement-incomplete` 71 implies a remote mutation — **Location**: Exit codes table.
- 🔵 **Usability / Compatibility**: `work create-batch --push` retires drafts but is not guarded — **Location**: Decisions; Phase 5.
- 🔵 **Usability / Safety**: `ACCELERATOR_FORCE=0` forces; forcing is silent — **Location**: Phase 2 `forces`.
- 🔵 **Safety**: Skill does not forbid retrying with `ACCELERATOR_FORCE` — **Location**: Phase 8 Skill steps.
- 🔵 **Usability**: Dirty preview gives no warning before the user confirms — **Location**: Phase 6; Phase 8.
- 🔵 **Usability**: Apply output `<old>\t<new>` differs from preview `old → new` and omits paths — **Location**: Phase 5 Composition.
- 🔵 **Usability**: `number-space-exhausted` duplicates `E_PATTERN_OVERFLOW` — **Location**: Exit codes table.
- 🔵 **Compatibility**: `create --push` refusal breaks save-intent contract — **Location**: Phase 5 `create.rs`.
- 🔵 **Compatibility**: Migrate messages/notice should cover both variables during deprecation — **Location**: Phase 2 §3.
- 🔵 **Architecture**: Run-level workflow lives in the composition root, unlike `promote` — **Location**: Phase 5/6 Composition.
- 🔵 **Architecture**: Run-plan guard opt-in per command and before lock acquisition — **Location**: Phase 5.
- 🔵 **Architecture**: Accelerator-specific policy (`GUARDED_SCOPES`, `FORCE_VARIABLE`) moves into `vcs` — **Location**: Phase 2.
- 🔵 **Code Quality**: `rekey_in_progress -> bool` hides the malformed-file error — **Location**: Phase 5 Adapters.
- 🔵 **Code Quality**: Composition flow has five branches with reordered guard sequences — **Location**: Phase 5/6.
- 🔵 **Code Quality**: Destination path computation duplicated in `plan_retirement`/`retired_content` — **Location**: Phase 3.
- 🔵 **Code Quality**: Self-alias removal adds a second inline/block alias editor — **Location**: Phase 3.
- 🔵 **Code Quality**: Flag argument on `guarded_working_copy`; five-parameter `unfinished_sync_work` — **Location**: Phase 5 Adapters.
- 🔵 **Code Quality**: `tracker_mapping` takes an untyped closure whose `None` is ambiguous — **Location**: Phase 7.
- 🔵 **Test Coverage**: Exemption and ignore-rule tests do not exercise jj or untracked new directories — **Location**: Phase 5.
- 🔵 **Test Coverage**: No seam for a failed probe at CLI level — **Location**: Phase 5 in-process tests.
- 🔵 **Test Coverage**: Live contract subjects pass vacuously and count as evidence — **Location**: Phase 1.
- 🔵 **Test Coverage**: Temporary tracker-pattern refusal test has no retirement step — **Location**: Phase 7.
- 🔵 **Test Coverage**: Numeric apply test cannot distinguish stored-plan execution from recompute — **Location**: Phase 6.
- 🔵 **Test Coverage**: Boundary cases missing (both force vars, mixed-case alias, 72-74, no `drafts/`) — **Location**: Phases 2/3/7.

#### Suggestions

- 🔵 **Architecture**: Share one move-detection rule (`is_move`) between sync and rekey — **Location**: Phase 7.
- 🔵 **Architecture**: Model the no-integration `IdentityWorkspace` explicitly rather than `Option<&str>` — **Location**: Phase 5.
- 🔵 **Code Quality**: `CreationDate::of` belongs in `rekey`, not `work_item_files` — **Location**: Phase 4.
- 🔵 **Usability**: Interim `{tracker}` refusal should not use exit 2; document `--per-item-reads` scope — **Location**: Phase 5; Phase 7.
- 🔵 **Safety**: Pin `AtomicWrite` for the plan file and test each interruption boundary — **Location**: Phase 5.
- 🔵 **Compatibility**: State the key comparison (exact vs ASCII-case) in the `fetch_all` obligation — **Location**: Phase 1.

### Strengths

- ✅ Functional core / imperative shell: classification, ordering, allocation, collisions, fingerprint and preview are pure in `work::rekey`; `tracker_mapping` takes refreshed keys as input.
- ✅ Resume reuses retirement's interrupted-target detection; recording the mapping before the first write is sufficient at item level, verified against the AC partial state.
- ✅ Chained and cyclic mappings are correctly refused by the target-collision rule.
- ✅ Claims about existing code (`is_interrupted_target`, `refuse_collisions` exemption, `BaselineStore::rename`, `allocate`'s scan) were verified accurate.
- ✅ Probe-failure policy pushed to each context's adapter: migrate keeps warn-as-clean, rekey refuses.
- ✅ `ACCELERATOR_MIGRATE_FORCE` deprecation keeps the old name working, prints a notice, and schedules removal (0346).
- ✅ `fetch_all` obligation tightened additively via a defaulted trait method plus a contract property.
- ✅ Every refusal fires before any corpus write, with one "leaves the plan as it found it" test per refusal.
- ✅ Complete AC traceability (52 ACs) and mutation-sensitive test names (`PP-7601`, byte-order ties, scope boundary, case-only move).
- ✅ Whole-run lock is a stated, justified departure from 0230; phase graph is explicit and each phase ships a complete command.

### Recommended Changes

1. **Add an explicit recovery path for in-progress plans** (addresses: no exit from a stuck plan; malformed plan; schema cross-version; skill has no in-progress path)
   Define a keyword for a mid-run `FinishFailure::Refused`; add `work rekey --abandon` (or `--discard` on a malformed/other-schema plan) that refuses while any entry is partly applied; define the schema-version read policy; add the transition to the state diagram; teach the skill to offer finishing an interrupted run.
2. **Reserve in-progress targets against local allocation** (addresses: `work create` steals reserved targets)
   Either guard `work create` while a plan is in progress or have `allocate` take the plan's unapplied targets as part of its floor; add an interleaving test.
3. **Include the sync baseline in the write set** (addresses: resume hits dirty-tree refusal)
   Add `last-sync.json` whenever an integration is configured; add an `rekey_execute` case with a completed synced entry before the interruption.
4. **Compare IDs ignoring ASCII case in the mapping** (addresses: case-only re-key)
   Use `same_id` semantics for "already on target" and "unmoved"; add a lower-case-ID test.
5. **Type the entry target** (addresses: `RekeyEntry` loses retirement kind; `RunPlan` invalid states)
   `RekeyTarget::{Renumbered, TrackerKey, Draft}` with `RekeyEntry::retirement()`; private `RunPlan` fields with `proposed(..)` / `started(self, write_set)` transitions; record the resolved scheme in the plan.
6. **Unify and freeze the rekey outcome contract** (addresses: fragmented keywords; printed form; `retirement-failed` 1 vs 71; remedy text)
   One `NotRekeyed` enum with `keyword()`/`exit_code()`; add Printed form and Remedy columns to the table; frozen test grows per phase; refusal tests assert the remedy.
7. **Teach existing consumers the new refusal** (addresses: exit-4 meaning; `create-batch --push`; `create --push` save-intent)
   Update `sync-work-items`, `create-work-item`, `extract-work-items` and the `exit_codes.rs` band; guard or explicitly exempt `create-batch --push`; document the `create --push` exception.
8. **Do every run-plan transition and guard check under the retirement lock** (addresses: checks before the lock; guard before sync's write phase)
   Lock, reload, decide, write; consider checking the run plan inside lock acquisition so a `RetirementLockGuard` implies no rekey in progress.
9. **Widen the fingerprint and re-check before starting** (addresses: added/removed items; lost plan file)
   Fingerprint membership and scheme; re-run collision and target-exists under the lock before `started`; refuse a fresh mapping when an item's `id` equals another item's alias or a `retirement-recovery` directory exists.
10. **Fix test seams** (addresses: env var racy; resume coverage; tracker collisions; benchmark)
    Resolve force in `main.rs` and pass it in; scrub force vars in subprocess harness; drive resume with the `Faulty` store; add `{tracker}` collision tests; commit `benchmark.json` in Phase 8.
11. **Guard scope and force semantics** (addresses: fixed scopes vs configured dirs; `ACCELERATOR_FORCE=0`; skill sets force)
    Rekey checks `GUARDED_SCOPES` ∪ corpus roots; `ACCELERATOR_FORCE` accepts `1`/`true` only and warns when bypassing; skill never sets it.

## Per-Lens Results

### Architecture

**Summary**: Sound functional-core / imperative-shell shape; recovery reuses 0230's re-planning. Main risks: entries don't record retirement kind, in-progress targets are invisible to `work create`, and the dirty-tree scope is fixed while rekey writes to configured corpus directories.

**Strengths**: Pure decisions in `work::rekey`; resume via `is_interrupted_target`; justified whole-run lock; per-caller probe-failure policy; `fetch_all` obligation at the port; write set as durable exemption; explicit phase graph with staged `{tracker}` refusal.

**Findings**:
- 🟡 major / medium — **Run-plan entries do not record what kind of retirement they are** (Phase 4 `RekeyEntry`; Phase 7 `execute`). `ACC-0042` is tracker-shaped per `is_tracker_key`, so shape inference is ambiguous; config inference couples execution to config that can change. Suggest `RekeyTarget` enum with `retirement()`.
- 🟡 major / medium — **An in-progress plan's target IDs are invisible to the `work create` allocator** (Run-plan guard reach; What We're NOT Doing). Resume then refuses `IdTaken` with no recovery. Guard allocation or raise its floor; define the mid-run refusal keyword.
- 🟡 major / medium — **Dirty-tree check covers fixed paths; rekey writes to configured corpus directories** (Phase 2 `GUARDED_SCOPES`; Phase 5). Pass `GUARDED_SCOPES` ∪ corpus roots to `observe_guarded`, or document the gap.
- 🔵 minor / medium — **Run-level workflow lives in the composition root, unlike promote** (Phase 5/6). Add `work_adapters::rekey::run(request, &RekeyPorts)`.
- 🔵 minor / medium — **Run-plan guard is opt-in per command and runs before the lock** (Phase 5). Check at lock acquisition.
- 🔵 minor / medium — **Accelerator-specific policy moves into the `vcs` platform domain** (Phase 2). Keep `vcs` generic; `GuardedChanges` duplicates `WorkingCopyState`.
- 🔵 suggestion / medium — **Share one move-detection rule** (Phase 7). Lift `is_move` into `work` or `tracker`.
- 🔵 suggestion / low — **An optional integration makes `IdentityWorkspace` partial** (Phase 5). Model no-integration explicitly.

### Code Quality

**Summary**: Clean pure/impure split reusing existing idioms (lock-guard token, write-then-verify ignore, keyword tables). Risks concentrate in domain types: stringly-typed `RekeyEntry`, representable invalid `RunPlan` states, and 13 refusal keywords across six types.

**Strengths**: Pure decisions testable without I/O; `&RetirementLockGuard` makes unlocked apply uncompilable; probe-failure policy unburied; resume reuses re-planning; target-shape via `canonicalise_id` with no regex; behaviour-named tests.

**Findings**:
- 🟡 major / high — **`RekeyEntry` stores its target as bare strings** (Phase 4; Phase 7). Classification lives in mapping and executor; suggest `RekeyTarget` enum mirroring `RetirementRecord::retirement()`.
- 🟡 major / high — **Thirteen refusal keywords across six error types, no single outcome enum** (Exit codes; Phases 4/5/7). Follow `NotPromoted`; `write_set` can also surface `IdTaken`/`KeyLinked`/`ItemNotFound` absent from the table.
- 🟡 major / medium — **`RunPlan` allows invalid states** (Phase 4; Phase 6). Public fields; `started` takes no write set; test names a non-existent "propose over" operation.
- 🔵 minor / high — **`rekey_in_progress` hides the malformed-file error** (Phase 5). Return a `RekeyStatus` enum.
- 🔵 minor / medium — **Five-branch composition with reordered guards** (Phase 5/6). Dispatch on `(RekeyMode, Option<RunPlanState>)`.
- 🔵 minor / high — **Destination path computation already duplicated** (Phase 3, `retirement.rs:375-384`, `:455-460`). Extract `destination_of`.
- 🔵 minor / medium — **Self-alias removal adds a second alias editor** (Phase 3). Single `rewrite_aliases`.
- 🔵 minor / medium — **Flag argument and data clump** (Phase 5). Caller decides force; group integration location.
- 🔵 minor / medium — **`tracker_mapping` takes an ambiguous closure** (Phase 7). Move `RefreshedKeys` into `work::rekey`.
- 🔵 suggestion / low — **`creation_date_of` reaches into rekey's type** (Phase 4). `CreationDate::of(&WorkItemFile)`.

### Test Coverage

**Summary**: Strong: all 52 ACs trace to a named test, phases start red, pure logic is unit-tested. Gaps: Phase 8 breaks `mise run` without `benchmark.json`, env-var reads make tests racy, resume rests on one hand-built state, and collision/fingerprint/malformed-plan/keyword cases are partly covered.

**Strengths**: Complete traceability; pure decisions below the CLI; mutation-sensitive cases named; per-refusal "plan unchanged" tests; two-level no-network check; reuse of existing safety nets; fakes contract-tested in Phase 1.

**Findings**:
- 🟡 major / high — **`evals.json` without `benchmark.json` fails the real-skills-tree guard** (Phase 8). Commit a benchmark with all four eval ids.
- 🟡 major / high — **Reading `ACCELERATOR_FORCE` inside `rekey::run` makes tests racy** (Phase 5). Resolve in `main.rs`; subprocess tests for parsing; scrub vars in harness.
- 🟡 major / medium — **Resume tested against one hand-crafted partial state** (Phase 5 `rekey_execute.rs`). Drive with the `Faulty` store at each write index; include a `{tracker}` drafts case.
- 🟡 major / medium — **Frozen keyword pin covers only 3 of 13 keywords** (Phase 4; Exit codes). Grow a frozen table per phase.
- 🟡 major / medium — **No collision tests under `{tracker}` mapping** (Phase 7).
- 🟡 major / medium — **Fingerprint tests miss added/removed items** (Phase 4; Phase 6).
- 🟡 major / medium — **Malformed plan and `rekey_in_progress` untested** (Phase 5; Exit codes).
- 🔵 minor / medium — **Exemption tests don't exercise jj or untracked new directories** (Phase 5). Use `Hermetic` git and jj.
- 🔵 minor / medium — **No seam for a failed probe at CLI level** (Phase 5). Use `assert_no_repository_ancestor` for no-VCS.
- 🔵 minor / high — **Live contract subjects pass vacuously** (Phase 1). Report as skipped.
- 🔵 minor / medium — **Temporary tracker-pattern refusal test has no retirement step** (Phase 7).
- 🔵 minor / low — **Numeric apply test can't tell stored plan from recompute** (Phase 6). Commit a reordering `date` change between preview and apply.
- 🔵 minor / medium — **Boundary cases missing** (Phases 2/3/7): both force vars, mixed-case alias, 72-74 selection errors, no `drafts/` dir.

### Correctness

**Summary**: Core recovery model holds and the verified code claims are accurate; chained/cyclic mappings are correctly refused. Gaps: write set omits the tracked baseline, an in-progress plan has no exit for a permanently unappliable entry, and exact-match target-shape conflicts with case-insensitive retirement identity.

**Strengths**: Chained/cyclic mappings refused correctly; item-level resume verified against the AC partial state; floor plus kept numbers cannot overlap; `canonicalise_id` claim verified with `resolve_scheme` rejecting keyless `{key}`; write set computed under lock before save.

**Findings**:
- 🟡 major / high — **Write set omits the tracked sync baseline** (Phase 5). `last-sync.json` is tracked and under `.accelerator/`; resume refuses after a synced entry completes.
- 🟡 major / high — **In-progress plan with a permanently unappliable entry has no exit; unguarded allocators can create one** (Run-plan lifecycle; Phase 5). `sync_author` takes only the create lock.
- 🟡 major / medium — **Exact-match target-shape vs case-insensitive retirement identity** (Key Discoveries; Phases 4/7). Case-only entry can roll back forever or `Remove` the item's only file on resume.
- 🔵 minor / medium — **Proposed-plan apply doesn't say in-progress is saved before first write** (Phase 6).
- 🔵 minor / medium — **Fingerprint doesn't cover the ID configuration** (Decisions; Phase 6). Record pattern and key.
- 🔵 minor / medium — **Kept numbers beyond pattern width not refused** (Phase 4). `PP-12345` under `{number:04d}`.
- 🔵 minor / medium — **Scope-boundary test could change prefix semantics** (Phase 2). Pin `.claude/accelerator.md` as reported; use an unrelated `.claude/` path as "outside".
- 🔵 minor / low — **Floor ignores bare legacy aliases that canonicalise into target shape** (Phase 4).

### Safety

**Summary**: Protective at its core — mapping recorded first, every refusal before any write, clean probeable tree required, per-entry recovery reused. Remaining risks: state checks before the lock, no exit from a repeatedly failing plan, and loss of the gitignored plan file reintroducing duplication.

**Strengths**: Mapping persisted before first write; all refusals pre-write; fail-safe probe policy for rekey; refresh completes or exits 70 before recording; write-set-only exemption on resume; malformed plan fails closed; dead-holder lock reclaim.

**Findings**:
- 🟡 major / high — **Run-plan state transitions check before the lock** (Phase 6).
- 🟡 major / high — **No supported way out of an in-progress plan that keeps failing** (Exit codes; Phase 5; Phase 6). Consider `--abandon` gated on no partly-applied entry.
- 🟡 major / medium — **Applying a preview doesn't re-check mid-run failure conditions** (Decisions; Phase 4; Phase 6).
- 🟡 major / medium — **Sync/promote/push guard checked once, before locking** (Phase 5).
- 🟡 major / medium — **Plain `work create` can take a reserved fresh number** (What We're NOT Doing; Phase 5).
- 🟡 major / medium — **Lost run-plan file brings back duplication** (Key Discoveries; Phase 5). Refuse when an `id` equals another item's alias or a recovery dir exists.
- 🔵 minor / high — **`ACCELERATOR_FORCE` treats `0`/`false` as force** (Phase 2).
- 🔵 minor / medium — **Skill doesn't forbid retrying with `ACCELERATOR_FORCE`** (Phase 8).
- 🔵 suggestion / medium — **Plan-file atomicity and inter-entry interruption points unpinned** (Phase 5).

### Usability

**Summary**: Command surface mirrors `work sync` conventions; `--discard` is idempotent and resume is automatic. Gaps are in error experience: keyword vs printed form undefined, remedies unspecified, malformed plan has no CLI exit, existing skills unaware of the new exit 4, and the skill has no in-progress path.

**Strengths**: `work sync` conventions reused; idempotent `--discard` and replacing re-preview; flagless resume; preview allowed on dirty tree; clean `ACCELERATOR_MIGRATE_FORCE` migration; single frozen keyword table as a contract.

**Findings**:
- 🟡 major / high — **Kebab keywords vs printed `E_REKEY_*` undefined** (Exit codes; Phase 5).
- 🟡 major / high — **New exit-4 refusal reaches skills that read 4 as "conflicts await a human"** (Phase 5; Phase 8).
- 🟡 major / high — **Skill has no path for an in-progress plan** (Phase 8). Add `finishes_an_interrupted_run` eval.
- 🟡 major / high — **Malformed run plan blocks everything with no way out** (Exit codes; Phase 5).
- 🟡 major / medium — **Remedy text required by ACs mostly unspecified and untested** (Phases 5/6).
- 🔵 minor / high — **`retirement-failed` exits 1 under rekey, 71 under sync/promote** (Exit codes).
- 🔵 minor / medium — **Dirty preview gives no warning before confirmation** (Phase 6; Phase 8).
- 🔵 minor / medium — **Apply output differs from preview and omits paths** (Phase 5; Phase 8).
- 🔵 minor / medium — **`work create-batch` pushes are not guarded** (Phase 5).
- 🔵 minor / medium — **`ACCELERATOR_FORCE` treats `0` as forcing and forces silently** (Phase 2; Phase 5).
- 🔵 minor / medium — **`number-space-exhausted` duplicates `E_PATTERN_OVERFLOW`** (Exit codes).
- 🔵 suggestion / medium — **Interim exit 2 for `{tracker}`; `--per-item-reads` silently ignored** (Phase 5; Phase 7).

### Compatibility

**Summary**: Main contract changes handled carefully — deprecation path, additive `fetch_all` obligation, public-API updates. Weak spots: consumers of the new exit-4 refusals are not updated, and the run-plan schema has no cross-version read rule despite a planned bump.

**Strengths**: Backward-compatible override rename with notice and scheduled removal; `forces` matches existing `!is_empty()`; migrate's probe policy pinned by an existing test; scopes pinned equal by test; defaulted `moved_issue`; additive pinned-crate changes; self-creating `.gitignore`.

**Findings**:
- 🟡 major / high — **New exit-4 refusals change what code 4 means for skills** (Phase 5; Phase 8). `sync-work-items:195-199`, `create-work-item:587,606-607`, `extract-work-items:662`.
- 🟡 major / medium — **Run-plan schema has no cross-version read rule** (Phase 4). Precedent `retirement_records.rs:105-106` treats other schemas as malformed; pin path serialisation.
- 🔵 minor / medium — **Refusing `create --push` breaks its save-intent contract** (Phase 5; `exit_codes.rs:117-121`).
- 🔵 minor / high — **`work create-batch --push` retires drafts but is not guarded** (`create_batch.rs:487`).
- 🔵 minor / high — **Existing keywords reused with a different code or meaning** (Exit codes; `keyword_exit_codes.rs:115`).
- 🔵 minor / medium — **Migrate messages and notice should cover both variables** (Phase 2 §3; `STALE_RUN_EXPLANATION`).
- 🔵 suggestion / low — **State the key comparison in the `fetch_all` obligation** (Phase 1; `jira-client/src/client.rs:747`).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-15T23:41:48+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness**: Write set omits the tracked sync baseline — Resolved
- 🟡 **Safety / Correctness / Usability / Compatibility**: No supported way out of a stuck in-progress plan — Partially resolved (`--abandon` added; its partly-applied precondition is unsound, see new issues)
- 🟡 **Architecture / Safety / Correctness**: `work create` can take a reserved number — Partially resolved (up-front guard added; `work next-number` unguarded and the create lock is not covered by the lock-time check)
- 🟡 **Correctness**: Exact-match target-shape vs case-insensitive identity — Resolved for on-target/unmoved; collision, floor and partly-applied comparisons unspecified (minor)
- 🟡 **Architecture / Code Quality**: `RekeyEntry` does not record the retirement kind — Resolved
- 🟡 **Code Quality / Test Coverage / Usability**: Keyword/exit-code contract fragmented — Partially resolved (`NotRekeyed` unifies outcomes; `save`/`remove`, lock and `write_set` error mappings unspecified; printed line not pinned per variant)
- 🟡 **Usability / Compatibility**: New exit-4 refusals change meaning for skills — Partially resolved (three skills updated; `refine-work-item` and `next-number` callers not)
- 🟡 **Safety**: Run-plan transitions checked before the lock — Resolved
- 🟡 **Safety**: Sync/promote/push guard checked before locking — Partially resolved (lock-time check added; pull-write race worse than the Decisions rationale states)
- 🟡 **Safety / Test Coverage**: Fingerprint misses membership; apply does not re-check — Resolved
- 🟡 **Safety**: Lost run-plan file reintroduces duplication — Partially resolved (detection added but misses rewrite-phase and post-remove interruptions)
- 🟡 **Architecture**: Dirty-tree check covers fixed paths — Resolved
- 🟡 **Test Coverage**: `benchmark.json` missing — Resolved
- 🟡 **Test Coverage**: `ACCELERATOR_FORCE` read in-process — Resolved
- 🟡 **Test Coverage**: Resume tested against one hand-built state — Partially resolved (fault injection added, but `Faulty` simulates a failed write, not a kill)
- 🟡 **Test Coverage**: No `{tracker}` collision tests — Resolved
- 🟡 **Test Coverage**: Malformed plan untested — Resolved
- 🟡 **Usability**: Skill has no in-progress path — Partially resolved (branch added; the mapping is not shown before finishing, no decline route)
- 🟡 **Usability**: Remedy text unspecified — Resolved (`E_REKEY_PARTLY_APPLIED` remedy wrong in two cases, see new issues)
- 🟡 **Code Quality**: `RunPlan` invalid states — Resolved
- 🟡 **Compatibility**: Run-plan schema read rule — Resolved (minor follow-up on older schemas)
- 🔵 All other pass-1 minors and suggestions — Resolved, except: in-process failed-probe seam (partial), guard sequences per branch (partial), no-integration workspace typing (partial)

### New Issues Introduced

- 🟡 **Architecture / Compatibility / Correctness / Code Quality**: Lock-time rekey refusal in `acquire_retirement_lock` creates a `retirement` ↔ `rekey` module cycle, and its five callers (`identity_settlement.rs:245,308,517,547`, `promotion.rs:370`, `sync_author.rs:262`) flatten it to internal/store failures, so it surfaces as exit 1 or 71 mid-run, not `E_REKEY_IN_PROGRESS → 4`.
- 🟡 **Usability / Compatibility**: `work next-number` is unguarded; `create-work-item`, `extract-work-items` and `refine-work-item` allocate through it under numeric patterns and write with the Write tool.
- 🟡 **Correctness / Architecture / Safety**: Allocators run under the create lock (`create.rs:198-209`; `sync_author.rs:259-275` under local ownership), which neither rekey nor the lock-time check takes; a create that passes the up-front check can still mint a reserved number.
- 🟡 **Correctness**: Partly-applied detection misses an entry interrupted after rewrites but before Create, or after Remove but before `RenameBaseline`; retirement only copies dirty files, so a clean-tree run leaves no recovery directory.
- 🟡 **Correctness**: The `retirement-recovery/` signal over-fires (the parent persists forever; COMPLETED children are kept and only `work sync` sweeps them) and under-fires if built on `kept()`, which skips marker-less children.
- 🟡 **Usability / Correctness**: Partly-applied runs before the sync guards, so an interrupted sync or promote retirement reports `E_REKEY_PARTLY_APPLIED`; with no plan its "run `work rekey` to finish" remedy loops, and "remove any recovery directory" can discard a sync's preserved copies.
- 🟡 **Safety**: Abandon remedies scope the revert to `meta/` or the named items, missing referrers, corpus roots outside `meta/` and `last-sync.json`; `--abandon` cannot verify the revert was complete.
- 🟡 **Safety**: The accepted sync race is worse than documented: `SyncApplier::pull` writes unconditionally and `baseline.set` races `BaselineStore::rename`, so pulls landing mid-run recreate retired files or lose baseline renames, which the dirty-tree refusal does not catch.
- 🟡 **Correctness**: A multi-project `{key}` corpus (`work create --project XYZ`) collapses onto the configured key, because the target-shape test renders with `scheme.key` only.
- 🟡 **Test Coverage**: The `Faulty` store returns `Err`, so `finish_retirement` rolls back; the crash-resume test never produces a killed process's state, and recovery-copy writes are outside its index space.
- 🔵 **Usability / Correctness / Test Coverage**: `--abandon` vs `--discard` overlap; `--abandon` on a proposed plan unspecified; the unreadable state is missing from the diagram.
- 🔵 **Correctness**: With git, revert leaves untracked targets that keep `--abandon` refusing.
- 🔵 **Safety**: An exported `ACCELERATOR_FORCE` (prompted by the deprecation notice) silently disarms rekey inside the skill.
- 🔵 **Architecture**: Tracker resolution sits in the composition root, but whether a mapping is computed is known only inside `run`.
- 🔵 **Compatibility / Test Coverage**: `E_REKEY_TRACKER_PATTERN_PENDING` is frozen in Phase 5 and removed in Phase 7.
- 🔵 **Compatibility**: A schema bump strands older in-progress plans instead of finishing them.
- 🔵 **Code Quality**: `changed_since -> Vec<PathBuf>` cannot express a scheme change; two independent readers of the run-plan file; `exit_code -> i32` vs local `u8` constants; `NumberSpaceExhausted { item: Option }` hides two refusals.
- 🔵 **Test Coverage**: "Run plan never makes the tree dirty" AC traced to Phase 5, where no proposed plan exists; the skill rekey branches have no evals; there is no per-variant render test; the interleaving tests have no defined seam.
- 🔵 **Usability**: The number-space remedy points at another error's text.
- 🔵 **Compatibility**: Forced `migrate` now writes extra stderr; the hand-edited fixtures need updating.

### Assessment

The pass-1 domain-model, contract, test-seam and lifecycle findings are largely resolved, and every reviewer credits the typed `RekeyTarget`, the split `RunPlan`, the `NotRekeyed` enum and the lock-held transitions. The revision's new recovery machinery is where the remaining risk concentrates. Its precondition is a heuristic over aliases and recovery directories that misses real interruption points and fires on legitimate states. The lock-time guard is grafted onto a primitive whose callers misreport it. Allocator coverage stops at the retirement lock. Three structural changes would close most of the new majors:

1. Make `--abandon`'s precondition "the run's write set (or every guarded scope and corpus root) is clean against VCS, untracked included".
2. Have rekey hold the create lock as well, guard `next-number`, and give the lock-time refusal a typed error each caller maps to `E_REKEY_IN_PROGRESS → 4`.
3. Either close the sync pull race with a run-level lock or correct its rationale.

The multi-project `{key}` collapse and the crash-simulation gap need their own targeted fixes.

## Re-Review (Pass 3) — 2026-10-16T00:11:00+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Architecture / Compatibility / Correctness / Code Quality**: Lock-time refusal in `acquire_retirement_lock` creates a cycle and is misreported by callers — Resolved (identity-run lock at command roots; `acquire_retirement_lock` untouched)
- 🟡 **Usability / Compatibility**: `work next-number` unguarded — Resolved
- 🟡 **Correctness / Architecture / Safety**: Allocators under the create lock not covered — Resolved (every allocator holds `IdentityRun`)
- 🟡 **Correctness**: Partly-applied detection misses rewrite-phase and post-Remove interruptions — Partially resolved (rewrite phase documented as accepted; post-Remove/pre-`RenameBaseline` not documented)
- 🟡 **Correctness**: `retirement-recovery/` signal over- and under-fires — Resolved
- 🟡 **Usability / Correctness**: Partly-applied before the sync guards; looping remedy — Resolved
- 🟡 **Safety**: Abandon remedies scope the revert too narrowly — Resolved (`RevertScope`)
- 🟡 **Safety**: Sync pull race worse than documented — Resolved (sync holds `IdentityRun`; `SyncApplier::pull` has no other caller; reclaim requires a dead holder)
- 🟡 **Correctness**: Multi-project `{key}` corpus collapses onto `work.key` — Partially resolved (fixed point at target width; a width change still re-renders other-key items under `work.key`)
- 🟡 **Test Coverage**: `Faulty` simulates a failed write, not a kill — Resolved (`Halting` + `catch_unwind`, following `promotion.rs:1014-1073`)
- 🔵 **Usability / Correctness / Test Coverage**: `--abandon` vs `--discard` overlap — Resolved (merged)
- 🔵 **Safety**: Inherited `ACCELERATOR_FORCE` disarms rekey in the skill — Partially resolved (preview path covered; the in-progress Finish/Undo branches and forced `--discard` are not)
- 🔵 **Architecture**: Tracker resolution in the composition root — Resolved (lazy `KeyRefresh`)
- 🔵 **Compatibility / Test Coverage**: Provisional code frozen then removed — Resolved
- 🔵 **Compatibility**: Schema bump strands older plans — Resolved
- 🔵 **Code Quality**: Staleness type, two readers, `i32`, `Option`-encoded refusal, per-branch guard sequences, workspace typing — Resolved
- 🔵 **Code Quality**: Keyword/exit-code contract — Partially resolved (rekey's own lock timeout has no `NotRekeyed` mapping)
- 🔵 **Test Coverage**: Skill rekey branches lack evals — Partially resolved (`refine-work-item` has an eval suite the plan says it lacks)
- 🔵 **Test Coverage**: Per-variant render test — Partially resolved (both `NotReverted` remedy forms not pinned)

### New Issues Introduced

- 🟡 **Architecture / Compatibility / Correctness**: Rekey's held lockdirs dirty the scope it checks. `LockdirLock` puts every lockdir under the work directory (`corpus-adapters/src/lock.rs:40-51`), each holds an untracked `owner.<nonce>` sentinel (`store/src/lock.rs:145-154`), and nothing ignores `*.lockdir`. Every apply refuses `E_REKEY_DIRTY_WORKING_COPY` naming its own lock, and an unreadable plan can never be discarded. `migrate` already exempts its held lockdir by prefix (`migrate/src/manifest.rs:63`).
- 🟡 **Correctness**: `IdentityRun` cannot be taken before the work directory exists. Lockdirs use a non-recursive `fs::create_dir` (`store/src/lock.rs:103`), and the guard sits before `create_dir_all(work_dir)` (`create.rs:447`), so the first `work create`/`next-number`/`sync` in a fresh repository fails with a lock I/O error.
- 🟡 **Safety / Correctness**: `--discard` accepts a committed half-run as reverted. "Clean against VCS" compares with HEAD or jj's parent, so `git commit -a` or `jj new` makes a half-applied write set clean, and an unforced command drops the only record of the mapping.
- 🟡 **Correctness**: A width change under a `{key}` pattern re-renders other-key kept numbers under `work.key` (`canonicalise_id` uses `scheme.key` only): `ACC-001`, `XYZ-001` → both `ACC-0001`, or `XYZ-005` → `ACC-0005` silently.
- 🟡 **Usability**: The `E_REKEY_PLAN_UNREADABLE` remedy ("revert every guarded scope … delete untracked files") pushes users to destroy unrelated uncommitted `meta/` work; a newer-schema plan needs an upgrade, not a revert.
- 🔵 **Architecture / Compatibility / Test Coverage / Correctness**: Guard sites are ambiguous. `create-batch` reuses `create::create_item`, so a guard there self-deadlocks for 300 s. The lock order omits `BatchJournal` and the `corpus::lock` module doc. `promote` reads the corpus and resolves the tracker (`promote.rs:181-188`) before `IdentityWorkspace::open`.
- 🔵 **Usability / Compatibility**: Lock waits are silent for up to 300 s and end in a generic timeout rather than `E_REKEY_IN_PROGRESS`. `next-number` loses its read-only contract (`Intent::Read`, "never writes a file"), and the lock-timeout exit for `next-number` and `sync` is undefined.
- 🔵 **Safety**: Unguarded `work update --push` read-modify-writes `last-sync.json` (`update.rs:246-256`), which rekey renames; the write-set exemption hides a lost rename.
- 🔵 **Architecture / Code Quality**: `IdentityRunGuard` is held by convention, not required by type as `RetirementLockGuard` is; `identity_run` and `rekey` depend on each other; `rekey::run` takes the lock inside a use case despite the "command roots only" rule.
- 🔵 **Architecture**: `baseline: Option` also reaches `apply_retirement` and `retirement_outstanding`'s `.unwrap_or(true)`; the meaning of `None` there is unstated.
- 🔵 **Code Quality**: Unreadable-plan `{path, reason}` is repeated across three types; `load`'s error type is too wide; `EntryRolledBack` can hold `RestoreIncomplete`; `NotReverted.plan_readable: bool`; `TrackerUnselectable` has no static code; `RevertScope` is undefined and overlaps `guarded_working_copy`; `forced` is repeated in every `RekeyRequest` variant.
- 🔵 **Usability**: The `IN_PROGRESS` remedy says to revert "the run's paths" without naming them. The skills offer "finish" for an unreadable plan. The skill's undo flow does not say who runs `--discard`. The dirty-row wording is wrong after Finish.
- 🔵 **Usability / Correctness**: The `PARTLY_APPLIED` remedy reverts only the named items, not their rewritten referrers, and does not mention that an alias clash made outside rekey also triggers it.
- 🔵 **Test Coverage**: The `Halting` counter must cover `ExclusiveCreate`/`FileRemove` and plan removal, not just writes. The lock-wait test is timing-based. The hold-across-pulls test has no observation point. Refusal tests do not assert zero tracker calls or an unchanged `meta/`. The amended discard AC is traced to Phase 6, although it ships in Phase 5.

### Assessment

The architecture has converged. Every reviewer credits the identity-run lock with closing the allocator, sync-pull and lock-time-guard problems at once, and the domain model, contract and test seams are now solid. Five majors remain, all targeted rather than structural:

1. Put the identity-run lockdir in a state directory that ignores itself and is created on demand, and exempt held lockdirs (and `ForFile` sentinels and atomic-write temporaries) from rekey's dirty checks, as `migrate` does. That fixes both lockdir majors.
2. Record a per-path content digest of the write set when the run starts, and have `--discard` compare against it rather than HEAD.
3. Render kept other-key numbers under their own project key.
4. Split the `PLAN_UNREADABLE` remedy by cause and drop "delete untracked files".

The minors are mostly about naming exact guard sites and lock order, and about remedy and eval wording.

## Re-Review (Pass 4) — 2026-10-16T11:31:47+00:00

**Verdict:** COMMENT

Lenses re-run: architecture, compatibility, correctness, safety, usability.

### Previously Identified Issues

- 🟡 **Architecture / Compatibility / Correctness**: Rekey's held lockdirs dirty the scope it checks — Resolved (`.accelerator/state/locks/` with a verified ignore rule; `*.lockdir/` and `TEMP_PREFIX` exempted; `the_check_passes_while_rekey_holds_its_locks`)
- 🟡 **Correctness**: `IdentityRun` cannot be taken before the work directory exists — Resolved
- 🟡 **Safety / Correctness**: `--discard` accepts a committed half-run — Resolved (per-path start digests; `discard_refuses_a_committed_half_run`)
- 🟡 **Correctness**: Width change moves other-key items onto `work.key` — Resolved
- 🟡 **Usability**: `PLAN_UNREADABLE` remedy destroys unrelated work — Resolved (remedy chosen by cause)
- 🔵 **Architecture / Compatibility / Test Coverage / Correctness**: Guard sites ambiguous — Partially resolved (create, create-batch, promote and sync match the code; `update --push` and `next-number` do not, see below)
- 🔵 **Usability / Compatibility**: Silent waits and undefined timeout — Partially resolved (waiting line and `E_IDENTITY_RUN_BUSY` added; port and wording gaps remain)
- 🔵 **Safety**: `work update --push` baseline race — Resolved in scope (guard site wrong, see below)
- 🔵 **Architecture / Code Quality**: `IdentityRunGuard` by convention and module cycle — Partially resolved (typed guard added; cycle survives through `RekeyStatus`)
- 🔵 **Architecture**: `baseline: Option` semantics — Resolved
- 🔵 **Safety / Usability**: Inherited `ACCELERATOR_FORCE` in the skill — Partially resolved (the Undo flow under force never learns the paths)
- 🔵 **Usability / Correctness**: `PARTLY_APPLIED` remedy — Partially resolved (revision unnamed; the hand-finish recipe omits the baseline)
- 🔵 **Correctness**: Post-Remove/pre-`RenameBaseline` window — Resolved (documented as accepted)
- 🔵 **Test Coverage**: `Halting` counter scope, amended AC traced to Phase 6, `refine-work-item` evals — Resolved

### New Issues Introduced

- 🟡 **Safety / Architecture** (minor per Compatibility and Correctness): The `work update --push` guard sits "before the tracker is resolved", which is inside `push_update` (`update.rs:204`). By then `try_run` already holds `ForFile` (`:308`) and has read the file, which inverts `IdentityRun` → … → `ForFile`. Rekey and update can then block each other until the 300 s ceiling, and a sync holding `IdentityRun` widens update's lost-update window. Fix: take the guard at the top of `try_run` when `args.push` is set, and test the acquisition order.
- 🔵 **Correctness**: The `next-number` guard sits after the `{tracker}` `mint_drafts` return (`next_number.rs:121-123`) and after the filename read. Fix: place it after `resolve_work_dir`.
- 🔵 **Architecture / Correctness / Usability**: `ExclusiveLock` has no holder query or non-blocking attempt, so the waiting line cannot be produced through the port. The sentinel records only a PID, never a command name. A timeout on an unreadable plan yields `BUSY` instead of `PLAN_UNREADABLE`. The timeout surfaces as `StoreError::LockTimeout`, not `LockError::Timeout`.
- 🔵 **Compatibility / Usability**: The 300 s wait outlasts the Bash tool's 120 s default, so skills never see `E_IDENTITY_RUN_BUSY`. `migrate`'s `FileRunLock` uses a 2 s ceiling for this reason.
- 🔵 **Architecture / Compatibility**: Widening `LockdirLock` with a state directory touches about 15 construction sites for one variant. A dedicated `IdentityRunLock` adapter, like `FileRunLock`, and a shared ignored-directory helper would avoid that.
- 🔵 **Compatibility**: Moving `next-number` to `Intent::Write` makes an insecure personal file a hard stop. This is neither tested nor in the changelog.
- 🔵 **Safety / Correctness**: A resume can rewrite paths outside the start-time write set, which a later `--discard` never sees. The remedy for a write-set path edited deliberately after a halt is unstated.
- 🔵 **Safety**: A forced discard hard-deletes the plan, including a valid newer-schema plan. Partly-applied evidence prints only in the deleting command.
- 🔵 **Usability**: `NotReverted` cannot separate restore from remove, or name the start revision. The skill's step 1 lacks exit-1 handling. "Keep the plan and stop" promises an apply that a re-preview would replace.
- 🔵 **Correctness**: The `PP-760` fixtures must say `external_id: "PP-760"` to take fresh numbers, now that unsynced `PP-760` keeps `PP-0760`. The Key Discoveries target-shape formula still renders under `scheme.key` only.
- 🔵 **Compatibility**: Older binaries running at the same time bypass `IdentityRun`. They are caught as `ENTRY_REFUSED`, not duplicated.

### Assessment

The plan is ready to implement once the `update --push` guard site moves to the top of `try_run`, a one-line placement fix. Every pass-3 major is resolved. Across five lenses only one major remains, raised by two lenses, and the other two lenses that saw it rate it minor. The remaining minors fall into three clusters: lock-port shape (holder query, a dedicated `IdentityRunLock`, a short ceiling), resume and write-set extension, and wording of remedies and fixtures. Each can be settled in the plan or during implementation without changing the design.

## Re-Review (Pass 5) — 2026-10-16T16:17:45+00:00

**Verdict:** COMMENT

Lenses re-run: architecture, correctness, safety.

### Previously Identified Issues

- 🟡 **Safety / Architecture**: `work update --push` guard inverts identity run → `ForFile` — Resolved (top of `try_run`, before `update.rs:308`; order test added)
- 🔵 **Correctness**: `next-number` guard after the `{tracker}` return and filename read — Resolved (after `resolve_work_dir`, `next_number.rs:116`)
- 🔵 **Architecture / Correctness / Usability**: Lock port lacks holder query, non-blocking attempt, command name; wrong timeout outcome — Partially resolved (`IdentityRunPort`, holder file, gate re-read; holder-file lifecycle, busy payload and `acquire_within` I/O errors remain loose)
- 🔵 **Compatibility / Usability**: 300 s wait outlasts skills' Bash timeout — Resolved (60 s ceiling)
- 🔵 **Architecture / Compatibility**: Widening `LockdirLock` — Resolved (dedicated `IdentityRunLock`)
- 🔵 **Architecture**: `identity_run` ↔ `rekey` cycle — Resolved (`RunPlanGate`)
- 🔵 **Compatibility**: `next-number` `Intent::Write` personal-file change — Resolved (stays `Intent::Read`)
- 🔵 **Safety / Correctness**: Resume writes outside the write set — Resolved (per-entry extension saved before writing)
- 🔵 **Safety**: Forced discard hard-deletes — Resolved (`set_aside`; unforced refusal prints evidence)
- 🔵 **Safety / Usability**: Inherited `ACCELERATOR_FORCE` and the skill's Undo — Partially resolved (step 1 withholds Undo; outcome-table paths still offer it)
- 🔵 **Usability**: `NotReverted` restore/remove and base, `ENTRY_REFUSED` remedy, skill exit-1 handling, "Keep the plan" wording, `PARTLY_APPLIED` revision and baseline step — Resolved
- 🔵 **Correctness**: `PP-760` fixtures and target-shape formula — Resolved (rule key-parametrised; "alphabetic" wording gap, below)
- 🔵 **Compatibility**: Older binaries bypass the identity-run lock — Resolved (rekey holds `Create`; older sync's unlocked pull remains, below)

### New Issues Introduced

- 🔵 **Architecture / Correctness**: Rekey's `Retirement` and `Create` lockdirs live in the work directory, which nothing creates before `work rekey` takes them (`store::lock` uses non-recursive `create_dir`). Create it first, as `sync_author.rs:257` does, or treat a missing work directory as nothing to re-key.
- 🔵 **Correctness**: Paths added to the write set on resume, and every path under a forced start, record states that are not the base revision's, so "restore from `<base>`" is wrong for them. Record a base per entry. The base also has no route out of `guarded_working_copy`, and a jj base with several parents is not one restorable revision.
- 🔵 **Architecture**: `execute(&InProgressPlan, …)` has no store port and returns nothing, so it cannot save the extended plan. The plan store's `remove` bypasses `Halting`'s counter.
- 🔵 **Architecture / Safety**: The write-set extension comes from `execute`'s plan, while `finish_retirement` re-plans (and re-plans after a rollback). A `before_apply` hook would extend from the plan actually applied.
- 🔵 **Architecture**: `IdentityRunPort` mints `IdentityRunGuard` itself, so a caller can bypass the gate. The port should return a raw token that only `begin_*` wraps.
- 🔵 **Architecture**: `WriteSet::with_path` and `extended` allow a recorded start state to be overwritten. Make extension additive and pin it with a domain test.
- 🔵 **Architecture / Correctness**: The holder file outside the lockdir can go stale or be deleted by the wrong holder. Write it inside the lockdir, where release and reclaim remove it. Also: `IdentityRunBusy` carries only a PID, `acquire_within` has no I/O-error arm, and a timeout behind a live rekey prints the finish or discard remedy.
- 🔵 **Correctness**: `prepare_ignored_dir` runs outside any lock and truncates before rewriting, so two concurrent starts can fail verification. Write only when the rule is missing, and write atomically.
- 🔵 **Correctness**: Project keys are alphanumeric (`is_project_prefixed`), not alphabetic, so `AB1-0001` would be pulled onto `work.key`.
- 🔵 **Safety**: A forced discard of a changed plan prints the mapping but not the restore and remove lists.
- 🔵 **Safety**: An older binary's unlocked `work sync` pull can still race a rekey, which is unrecorded as a residual.
- 🔵 **Correctness**: Desired End State and What We're NOT Doing omit or contradict `work update --push`.

### Assessment

No critical or major findings remain across the three lenses re-run. Every pass-4 item is resolved or reduced to a detail. The new minors are interface and wording gaps: `execute`'s signature, the guard's constructor, additive extension, the holder file, per-entry base, work-directory creation, alphanumeric keys and an atomic ignore rule. Each is local to one adapter or type and can be settled in the plan text or during implementation without changing the design. The plan is ready to implement once the work item's discard criterion is amended.

## Final Verdict — 2026-10-16T16:38:06+00:00

**Verdict:** APPROVE

The pass-5 minors were folded into the plan without a further pass:
work-directory creation before the corpus locks, a base revision per
write-set entry, `execute` owning a store port with a `before_apply` hook
on `finish_retirement_with`, a gate-only `IdentityRunGuard` constructor,
additive write-set extension, the command file inside the lockdir, an
atomic `prepare_ignored_dir`, alphanumeric project keys, Undo withheld
under an inherited force throughout the skill, and the consistency fixes.
The plan is approved for implementation once the work item's discard
criterion is amended as the plan describes.
