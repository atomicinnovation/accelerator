---
type: "plan-review"
id: "2026-11-11-0352-sync-reports-every-unsynced-item-twice-review-1"
title: "Plan Review: Sync Reports Every Unsynced Item Twice Implementation Plan"
date: "2026-11-11T08:18:27+00:00"
author: "Phil Helm"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-11-0352-sync-reports-every-unsynced-item-twice"
reviewer: "Phil Helm"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "compatibility"]
review_number: 1
review_pass: 3
tags: ["sync", "report", "work-adapters", "baseline", "sync-work-items"]
last_updated: "2026-11-11T08:41:47+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Plan Review: Sync Reports Every Unsynced Item Twice Implementation Plan

**Verdict:** COMMENT

The plan is sound. The partition sketch type-checks against the real types and
keeps every existing behaviour of `unsynced_creates`. Every downstream reader
of `plan.actions` was checked, and none depends on the removed `Noop` entries.
All seven behaviour tests are genuinely red today, all five regression tests
are genuinely green, and almost every cited line reference is accurate. It is
acceptable as it stands but could be improved; see the major finding below.
That finding is a gap in acceptance-criterion coverage: no test covers a
create order or a failed adopt. The minor findings fall into three groups:
comment-policy and banner-style slips in the proposed code, TDD sequencing for
the CLI test and for Phase 2, and claims (in the CHANGELOG and Phase 2's
premise) that are broader than what the fix does.

### Cross-Cutting Themes

- **Proposed comments break the repo's comment policy** (flagged by:
  standards, code-quality). The new doc comment on `unsynced_creates` restates
  the guard and the `--pull-only` return. The new section banner adds a
  comment and does not match the file's dash-padded style.
- **TDD sequencing is incomplete outside the engine tests** (flagged by:
  test-coverage, standards). The CLI expectation changes alongside the
  partition, not before it. Phase 2 deletes a skill row and its test with no
  red step, and its grep check cannot see the Python test.
- **The fix's reach is stated more broadly than it is** (flagged by:
  correctness, compatibility, test-coverage). The unknown-recipe warning still
  fires for unsynced drafts and under `--pull-only`. A `noop\tunsynced` row
  still sits beside an `order-refused` `failed` row. Dossier retirement also
  changes, and the plan does not say so.
- **Order-refusal regression tests duplicate existing tests** (flagged by:
  code-quality, test-coverage).

### Tradeoff Analysis

- **Mirroring `relinks` vs a clearer partition**: `relinks` uses
  `mem::take(..).partition(..)`. The proposed `for` loop does one lookup per
  entry, and an entry leaves the plan only when it becomes a create. Keep the
  loop, but stop citing `relinks` as "the pattern to follow" without saying how
  this one differs.

### Findings

#### Critical

None.

#### Major

- 🟡 **Test Coverage**: No test covers a create order or a failed adopt/order
  (AC4 partly untested)
  **Location**: Phase 1, §2 Behaviour tests
  AC4 covers an `adopt` row, a `create-from-local` row from a create order,
  and that create's `failed` row. Only successful adopts are tested, although
  `creating` (sync_create.rs:3800) and the not-found adopt setup
  (`adopting_a_missing_issue_is_refused`, sync_create.rs:3892) already exist.

#### Minor

- 🔵 **Compatibility**: Phase 2 deletes a Membership rule that still matches
  live output
  **Location**: Phase 2 Overview; Changes §1
  A draft with an order, or any order under `--pull-only`, still prints
  `<id>\tnoop\tunsynced\t-` beside `<id>\tfailed\t-\torder-refused`
  (cli/work-cli/src/sync.rs:563-566; run.rs:1373-1392). SKILL.md:466 matches
  that pair, and the plan's own regression tests 3.4 and 3.5 pin it.
- 🔵 **Correctness**: The unknown-recipe warning claim is broader than the fix
  **Location**: Desired End State; Phase 1 §6 CHANGELOG
  `unsupported_recipe` collects every reconciled id whatever its state
  (run.rs:2217-2227), and `noting_unsupported_recipe` (run.rs:1622-1636) still
  relabels draft and `--pull-only` `noop` rows. The warning survives for those
  items.
- 🔵 **Standards / Code Quality**: The new doc comment restates the code and
  sits apart from the draft check it explains
  **Location**: Phase 1 §4 The partition
  It repeats the `creatable_item` guard and the `PullOnly` return. The one
  non-obvious reason it gives (a draft reaches the tracker only through
  promotion) explains the `DraftId::parse` check, which now lives in
  `creatable_item`.
- 🔵 **Standards / Code Quality**: The section banner does not match the
  file's style
  **Location**: Phase 1 §2
  Every banner in sync_create.rs is padded with dashes to column 80 (lines
  562, 1203, 1855, 2629). `// --- One row per unsynced item ---` is not.
- 🔵 **Standards / Correctness / Code Quality**: The import note would cause
  E0252
  **Location**: Phase 1 §3 (imports note)
  `ReportedItem` is already imported as `work_adapters::sync::run::ReportedItem`
  (sync_create.rs:54). Only `work::sync::SyncState` needs adding.
- 🔵 **Test Coverage / Standards**: Phase 2 has no red step, and its grep is
  blind to the Python test
  **Location**: Phase 2; Success Criteria
  The test source spells the string `noop\\tunsynced` with an escaped
  backslash (test_sync_work_items_skill_structure.py:455), so
  `git grep -nF 'noop\tunsynced'` never matches it.
- 🔵 **Test Coverage**: The CLI expectation is changed alongside the
  partition, not red first
  **Location**: Phase 1 §5
  Editing the expected stdout after the partition repairs a broken test after
  the fact; no failing test demands the change. Move §5 before §4.
- 🔵 **Test Coverage**: AC6 is pinned only for a plain create, not for an
  equivalent adopt
  **Location**: Phase 1 §3; Desired End State
  An adopt writes its entry by a different path (apply.rs:1385-1390), and
  `adopting_an_equivalent_issue_records_the_baseline` checks only `is_some()`.
- 🔵 **Code Quality / Test Coverage**: Regression tests 4 and 5 duplicate
  whole existing tests
  **Location**: Phase 1 §3, tests 4–5
  Add the `only_row` assertion to `an_order_under_pull_only_is_refused`
  (sync_create.rs:3534) and `an_order_naming_a_draft_is_refused` (:3561)
  instead of copying them.
- 🔵 **Code Quality**: The expected `Some(0)` depends on a constant hidden in
  `split_entry`
  **Location**: Phase 1 §2, test 6
  `local_synced_at: 0` comes from support/mod.rs:550. Capture the watermark
  before the run and compare after.
- 🔵 **Code Quality**: Inline seeding and a copied magic recipe number
  **Location**: Phase 1 §2, tests 6–7
  The file already has a named fixture for this, `linked_and_recorded`
  (sync_create.rs:4841). The literal `digest_recipe: 2` means "unknown" only
  while `DigestRecipe::CURRENT == 1`.
- 🔵 **Architecture**: The one-row-source-per-item rule rests on call order
  between two partitions
  **Location**: Phase 1 §4
  `creatable_item` checks only `state() == Unsynced`. It does not check
  `action() == Noop`, and nothing backstops it the way `unreachable!` at
  run.rs:1697-1703 backstops relinks.

#### Suggestions

- 🔵 **Correctness / Test Coverage**: Dropping the unknown-recipe row also
  unblocks dossier retirement
  **Location**: What We're NOT Doing; test 7
  `ConflictStatus::of` maps `UnsupportedRecipe` to `Unobserved`
  (dossiers.rs:137-139), which blocks `settled_by` (dossiers.rs:260-268). The
  remaining `(Unsynced, CreateFromLocal)` row is `NotInConflict`
  (dossiers.rs:93).
- 🔵 **Correctness**: Test 7's watermark assertion is green today
  **Location**: Phase 1 §2, test 7; Success Criteria
  A successful create replaces the entry at the run start regardless. Label it
  as a guard, not a red signal.
- 🔵 **Correctness**: The unknown-recipe seed reference is off by one line
  **Location**: Key Discoveries; test 7
  The `raw` literal is at sync_recipe_migration.rs:900-901, not 899-900.
- 🔵 **Compatibility**: The migration note ignores scripts that used `noop` to
  list unsynced items
  **Location**: Migration Notes; CHANGELOG
  Tell script authors to key on column 3 (`unsynced`), matching the
  CHANGELOG's "scripts that parse `work sync` output need updating"
  convention (CHANGELOG.md:172-177).
- 🔵 **Architecture**: The domain plan still models a to-be-created item as
  `Noop`, and the plan does not state that tradeoff
  **Location**: What We're NOT Doing
- 🔵 **Code Quality**: Keep the two plan-extracting calls side by side in
  `prepare_run`
  **Location**: Phase 1 §4
  `relinks` runs at run.rs:2214 and `unsynced_creates` at 2235-2236, with
  unrelated code between them.
- 🔵 **Code Quality**: The partition does not follow the `relinks` style the
  plan cites
  **Location**: Phase 1 §4; References
- 🔵 **Code Quality**: `only_row` sits beside overlapping first-match row
  helpers
  **Location**: Phase 1 §1
  `local_create_row` (sync_create.rs:3769), `create_from_local_outcome`,
  `relink_row`, and `row`/`outcome_of` in support/repo.rs.

### Strengths

- ✅ Partitioning in the adapter after planning follows the `relinks`
  precedent. The decide golden, the dossier mapping and `work list` are
  untouched.
- ✅ The plan correctly spots that `relinks` returns early under
  `--pull-only` only after partitioning. A regression test pins the reverse
  order this change needs.
- ✅ The partition sketch compiles against the real types. It keeps plan
  order, push-only/bidirectional parity, and the exclusion of drafts and
  unmatched entries.
- ✅ Downstream readers are verified safe: `refused_orders`, `pull_count`,
  `push_count`, `validate_pushes`, the exit codes, `synced_count`, and the
  bounds refusal payload.
- ✅ The red and green claims hold: tests 1–7 fail today, and regression
  tests 1–5 pass today. Every named helper exists with the assumed shape.
- ✅ `rows_for`/`only_row` close the first-row blind spot in
  `row`/`outcome_of`/`failure_for`.
- ✅ The phases merge independently in order, because `bin/accelerator` pins
  the binary to the plugin version.
- ✅ The verification commands, package names, mise tasks, `ACCELERATOR_COVERAGE`
  and the CHANGELOG location all check out.

### Recommended Changes

1. **Add create-order and failed-adopt tests** (addresses: AC4 partly
   untested). Add `an_ordered_create_is_the_items_only_row` (`creating`) and
   `a_refused_adopt_is_the_items_only_row` (not-found tracker), both red first
   through `only_row`.
2. **Revisit the Phase 2 premise** (addresses: Phase 2 deletes a live rule).
   Either narrow SKILL.md:466 to the order-refused pair and update its test,
   or state why that pair needs no rule. Correct the Overview's claim either
   way.
3. **Scope the warning claim** (addresses: unknown-recipe claim broader) in
   the Desired End State and the CHANGELOG to "an unsynced item sync creates
   or adopts".
4. **Fix the TDD order** (addresses: CLI expectation not red first; Phase 2 no
   red step). Edit cli_sync.rs before writing the partition and see it fail.
   In Phase 2, replace the old test with an absence assertion, run it red,
   then delete the row. Fix or drop the grep check.
5. **Trim the comments** (addresses: doc comment; banner). Cut the doc comment
   on `unsynced_creates` to its purpose. Move the promotion rationale to
   `creatable_item`, or remove it. Drop the banner, or pad it to column 80.
6. **Tidy the tests** (addresses: import note; regression duplication;
   `Some(0)`; magic recipe; AC6 adopt). Remove the `ReportedItem` import note.
   Extend the existing order-refusal tests instead of copying them. Compare
   the watermark before and after. Derive the unknown recipe from
   `DigestRecipe::CURRENT + 1`. Assert the watermark in test 4.
7. **Record the accepted side effects** (addresses: dossier retirement;
   migration note; domain tradeoff) in What We're NOT Doing and the Migration
   Notes.

## Per-Lens Results

### Architecture

**Summary**: The fix sits at the right seam. It turns a copy into a move,
following `relinks`, and leaves the decide golden, the dossiers and `work list`
untouched. The release lock makes the phase split safe. The remaining concerns
are how well the design holds up as it changes: the one-row-per-source rule is
enforced only by call order, and the domain `SyncPlan` still models a
to-be-created item as `Noop`.

**Findings**:
- 🔵 minor (medium): The one-row-source-per-item rule rests on call order
  between two separate partitions — Phase 1 §4. `relinks` (run.rs:2214) then
  `unsynced_creates` (run.rs:2235-2237). `creatable_item` does not require
  `action() == Noop`, so it is safe only because a relink always carries an
  `external_id`. Suggest one `take_local_creates` step, or at least an `Action::Noop` check.
- 🔵 suggestion (high): The domain plan still models a to-be-created item as
  `Noop`, and the tradeoff is not stated — What We're NOT Doing. decide.rs:116-119
  maps `Unsynced` to `Noop`, although `Action::CreateFromLocal` exists. Add a
  sentence saying that create eligibility deliberately lives in the adapter.

### Code Quality

**Summary**: A small, well-shaped change, a clone turned into a move, but with
comment-policy slips and test fixtures that hide intent.

**Findings**:
- 🔵 minor (high): The rewritten doc comment restates the code and separates
  the draft rationale from the draft check — Phase 1 §4.
- 🔵 minor (high): The new section banner does not match the file's style —
  Phase 1 §2 (sync_create.rs:562/1203/1855/2629 are padded to 80).
- 🔵 minor (high): The expected `Some(0)` depends on a constant hidden in
  `split_entry` (support/mod.rs:550) — Phase 1 §2, test 6.
- 🔵 minor (medium): Inline seeding and a copied magic recipe number, where
  `linked_and_recorded` (sync_create.rs:4841) offers a precedent — tests 6–7.
- 🔵 minor (medium): Regression tests 4–5 duplicate whole existing tests
  (sync_create.rs:3533-3583) — Phase 1 §3.
- 🔵 suggestion (medium): Keep the `relinks` and `unsynced_creates` calls
  side by side in `prepare_run` — Phase 1 §4.
- 🔵 suggestion (medium): The partition does not follow the `relinks` style
  the plan cites — Phase 1 §4 / References.
- 🔵 suggestion (low): `only_row` overlaps the existing first-match helpers
  (`local_create_row` sync_create.rs:3769, `row`/`outcome_of`
  support/repo.rs) — Phase 1 §1.

### Test Coverage

**Summary**: The engine tests are well chosen and genuinely red and green as
claimed, and the helpers exist with the assumed shapes. The gaps are AC4
(create orders, failed adopts), AC6 for adopts, and TDD sequencing for the CLI
test and for Phase 2.

**Findings**:
- 🟡 major (high): No test covers a create order or a failed adopt/order —
  Phase 1 §2 / AC4.
- 🔵 minor (high): The CLI expectation is changed alongside the partition,
  not red first — Phase 1 §5.
- 🔵 minor (medium): Phase 2 has no red step, and its grep check cannot see
  the Python test (escaped `\\t` at test_sync_work_items_skill_structure.py:455).
- 🔵 minor (medium): AC6 is pinned only for a plain create, not for an
  equivalent adopt (apply.rs:1385-1390) — Phase 1 §3.
- 🔵 suggestion (medium): The order-refusal regression tests duplicate
  existing tests — Phase 1 §3.
- 🔵 suggestion (low): The dossier-retirement side effect is unpinned
  (dossiers.rs:137-139, 260-268) — test 7.

### Correctness

**Summary**: The logic holds. The partition type-checks and keeps behaviour.
Only `noting_unsupported_recipe` read the removed entries. The red/green
claims check out. What remains are small overstatements and one off-by-one
reference.

**Findings**:
- 🔵 minor (high): The unknown-recipe warning claim is broader than the fix
  (run.rs:2217-2227, 1622-1636) — Desired End State; CHANGELOG.
- 🔵 suggestion (high): Dropping the unknown-recipe row also unblocks dossier
  retirement — What We're NOT Doing.
- 🔵 suggestion (high): Test 7's watermark assertion passes today
  (baseline.rs:458-460; apply.rs:1004-1014) — test 7.
- 🔵 suggestion (medium): `ReportedItem` is already imported, so a second
  import fails with E0252 (sync_create.rs:54) — Phase 1 §3.
- 🔵 suggestion (high): The unknown-recipe seed reference is off by one
  (sync_recipe_migration.rs:900-901) — Key Discoveries.

### Standards

**Summary**: Conventions are followed closely. Package names, mise tasks,
`ACCELERATOR_COVERAGE`, the CHANGELOG section and style, test naming, and the
80-column limit all check out. There are four small deviations.

**Findings**:
- 🔵 minor (high): The interim-row grep can never match the Python test —
  Phase 2 Success Criteria.
- 🔵 minor (high): The section banner does not follow the file's dash-padded
  style — Phase 1 §2.
- 🔵 minor (medium): The suggested import path differs from the file's own,
  and `ReportedItem` is already imported — Phase 1 §3.
- 🔵 minor (medium): The new doc comment repeats the code and sits on the
  wrong function — Phase 1 §4.

### Compatibility

**Summary**: Exit codes, the summary count, the bounds, and the state column
are unchanged. The binary is pinned to the plugin version, so the skill and
binary cannot drift apart. The gap is Phase 2's premise and the migration
note.

**Findings**:
- 🔵 minor (high): Phase 2 deletes a Membership rule that still matches live
  output: `noop` beside an `order-refused` `failed` row
  (cli/work-cli/src/sync.rs:563-566; SKILL.md:318-320, 466) — Phase 2.
- 🔵 suggestion (medium): The migration note covers only scripts that skipped
  the duplicate row (CHANGELOG.md:172-177 convention) — Migration Notes.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-11T08:29:23+00:00

**Verdict:** COMMENT

Lenses re-run: architecture, code-quality, test-coverage, correctness,
standards, compatibility (every lens that had findings in pass 1).

### Previously Identified Issues

- 🟡 **Test Coverage**: No test covers a create order or a failed adopt/order
  — Resolved (test 5 `an_ordered_create_is_the_items_only_row`; extended
  `adopting_a_missing_issue_is_refused`)
- 🔵 **Compatibility**: Phase 2 deletes a Membership rule that still matches
  live output — Partially resolved: the plan now names the order-refused
  pair, but the reason it gives for deleting the rule anyway is wrong (see
  the new major finding below)
- 🔵 **Correctness**: The unknown-recipe warning claim is broader than the
  fix — Resolved (scoped to items the run creates or adopts; drafts and
  `--pull-only` keep it)
- 🔵 **Standards / Code Quality**: The new doc comment restates the code —
  Resolved (purpose-only comment; the promotion rationale moved to
  `creatable_item`)
- 🔵 **Standards / Code Quality**: The section banner does not match the
  file's style — Resolved (no banner)
- 🔵 **Standards / Correctness / Code Quality**: The import note would cause
  E0252 — Resolved (only `SyncState` is added)
- 🔵 **Test Coverage / Standards**: Phase 2 has no red step, and its grep is
  blind to the Python test — Grep resolved (`noop\\{1,2}tunsynced` matches
  both escapes). No red step: declined on purpose; the work item says to
  remove the test and the plan says why
- 🔵 **Test Coverage**: The CLI expectation is not red first — Resolved (§2
  runs before the partition)
- 🔵 **Test Coverage**: AC6 is pinned only for a plain create — Resolved (test
  4 asserts the adopt's watermark)
- 🔵 **Code Quality / Test Coverage**: Regression tests duplicate existing
  tests — Resolved (the existing order-refusal tests are extended)
- 🔵 **Code Quality**: The expected `Some(0)` depends on a hidden constant —
  Resolved (the test compares the watermark before and after the run)
- 🔵 **Code Quality**: Inline seeding and a copied magic recipe number —
  Resolved (named fixtures; `DigestRecipe::CURRENT + 1`)
- 🔵 **Architecture**: The one-row-source rule rests on call order —
  Resolved (`action() == Noop` guard; the calls sit side by side)
- 🔵 Suggestions (dossier retirement, test 8 labelled as a guard, off-by-one
  reference, migration note, domain tradeoff, side-by-side calls, `relinks`
  style) — Resolved
- 🔵 **Code Quality**: `only_row` overlaps the first-match helpers —
  Declined on purpose (out of scope)

### New Issues Introduced

#### Major

- 🟡 **Correctness / Architecture / Compatibility**: Phase 2 claims that a
  record the Membership table does not list joins no group, but Step 5 says
  otherwise
  **Location**: Key Discoveries (Membership-table bullet); Implementation
  Approach; Phase 2 Overview; Phase 2 Manual Verification
  Step 5 tells the skill to "Print a summary grouped by action"
  (SKILL.md:652). It also has groups that no Membership row feeds:
  `pushed`, `pulled`, `pushed-unsynced` and `unsynced (not pushed)`
  (SKILL.md:660-662, 681, against the table at 446-466). So records the
  table does not list do join groups. After Phase 1, a draft with an order,
  or any order under `--pull-only`, still prints `<id>\tnoop\tunsynced\t-`
  beside `<id>\tfailed\t-\torder-refused` (cli/work-cli/src/sync.rs:563-566).
  Today SKILL.md:466 matches that pair, because `order-refused` is a
  `failed` row (SKILL.md:457). Once row 466 is deleted, the skill may list
  the draft under both `needs-attention` and `unsynced (not pushed)`, and
  Phase 2's manual check ("summarises the draft once") would fail.
  **Fix**: Correct the premise in all three places. Then choose one of two
  options. (a) Keep a narrowed rule that matches only this pair and that the
  work item's acceptance criterion allows: "`noop\tunsynced` row for an id
  that also has an `order-refused` row | none | no". Rename its test to
  match, and point the grep at the `create-from-local`/`adopt` wording.
  (b) Accept the double listing and reword the Phase 2 manual check.

#### Minor

- 🔵 **Code Quality / Standards**: The new seed helpers break the file's
  `seed_*` naming, and `with_recorded_entry` repeats existing seeding code
  **Location**: Phase 1 §1
  Every helper in sync_create.rs that seeds the fixture is named `seed_*`
  (:3006, :3210, :3744, :4345, :4823). `with_recorded_entry` repeats the
  `split_entry` + `spy.seed` body of `linked_and_recorded` (:4840-4856).
  **Fix**: Rename the helpers to `seed_recorded_entry` and
  `seed_unknown_recipe_entry`.
- 🔵 **Test Coverage**: Test 7 can pass vacuously
  **Location**: Phase 1 §3, test 7
  If the seed never lands, `before` and `after` are both `None`.
  `advance_watermark` (baseline.rs:480-488) makes no change for an id with no
  entry, so the test would pass without exercising the bug.
  **Fix**: Assert the precondition, e.g. `assert!(before.is_some())`, before
  the run.

#### Suggestions

- 🔵 **Test Coverage**: The new `action() != Action::Noop` clause in
  `creatable_item` cannot be reached by any test
  **Location**: Phase 1 §5
  The planner always gives `Unsynced` a `Noop` action (decide.rs:116-119;
  `PlannedAction::ordered`, plan.rs:90-99), and `relinks` has already taken
  out the relink entries. Deleting the clause would leave every test green.
  This clause answers pass 1's architecture finding, so the two findings pull
  in opposite directions. Either keep it and say in the plan that it is a
  deliberate, untested guard, or drop it as pure TDD requires.
- 🔵 **Code Quality**: `rows_for` has only one caller, `only_row`
  **Location**: Phase 1 §1
  **Fix**: Fold the filter into `only_row`, as `failure_for` does
  (sync_create.rs:3502-3509).
- 🔵 **Standards**: The `SyncState` import is hedged
  **Location**: Phase 1 §1
  It is definitely absent (sync_create.rs:30-34).
  **Fix**: Say to add it after `use work::sync::SyncDirection;`.
- 🔵 **Compatibility**: The CHANGELOG names only the `-` form of the row that
  goes away
  **Location**: Phase 1 §6
  With an unknown-recipe entry, the duplicate prints as
  `noop\tunsynced\tunsupported-recipe` (cli/work-cli/src/sync.rs:599-602).
  **Fix**: Say "a second `noop\tunsynced` row".

### Assessment

Phase 1 is sound. Every lens confirmed the following:

- The partition type-checks against the real API.
- Moving the call up beside `relinks` is safe, because nothing between
  run.rs:2214 and 2235 reads `plan.actions`.
- Every red/green claim holds. Test 7 is red, because the leftover `Noop` row
  passes `definitively_reconciled` (run.rs:368-384). Test 8 is red, because
  `noting_unsupported_recipe` relabels that row (run.rs:1622-1636).
- The verification commands and line references are accurate.

All of pass 1's findings are resolved, apart from the two declined on
purpose. One issue remains: the reason Phase 2 gives for deleting the
interim rule is wrong. Fix that premise, and choose between the narrowed
rule and the double listing, before implementing Phase 2. Phase 1 can go
ahead as written.

## Re-Review (Pass 3) — 2026-11-11T08:41:47+00:00

**Verdict:** COMMENT

Lenses re-run: correctness, test-coverage, code-quality (with standards),
compatibility (with architecture). These are the lenses that had findings in
pass 2.

### Previously Identified Issues

- 🟡 **Correctness / Architecture / Compatibility**: Phase 2's premise that
  an unlisted record joins no group — Resolved. The plan now keeps a narrowed
  rule: "`noop\tunsynced` row for an id that also has an `order-refused` row
  | none: the refusal speaks for the item | no". This rule is complete
  against the code. After Phase 1, every reconciled, unsynced, non-draft item
  outside `--pull-only` becomes a create, and `refused_orders` skips any id
  that has a create (run.rs:1373-1392). Items that are targeted out, deferred
  or missing have no plan entry (`why_no_create`, run.rs:1397-1426).
  Unclassifiable items have no plan entry either. Promotion rows are never
  `failed` rows (promotion_report.rs:15-33). The only remaining pair is
  therefore a draft's or `--pull-only` item's `noop` row beside
  `<id>\tfailed\t-\torder-refused` (cli/work-cli/src/sync.rs:563-566). The
  phases also merge safely in order, because the old row matches a superset
  of the cases the new row matches. "`order-refused` row" is the term
  sync-work-items-recovery/SKILL.md:38 already uses.
- 🔵 **Code Quality / Standards**: The seed helpers break the `seed_*`
  naming and duplicate seeding code — Resolved. The helpers are now
  `seed_recorded_entry` and `seed_unknown_recipe_entry`, and
  `linked_and_recorded` calls `seed_recorded_entry`. That keeps its behaviour:
  `item.id` is "0001", and the projection and timestamp are the same.
- 🔵 **Test Coverage**: Test 7 can pass vacuously — Resolved
  (`assert!(before.is_some())`).
- 🔵 **Test Coverage**: The `Action::Noop` clause cannot be reached by any
  test — Resolved, by stating that the guard is deliberate. The reason the
  plan gives is accurate: `PlannedAction::ordered` builds on `Noop`
  (plan.rs:90-99), and `relink` is planned "whatever its classified state"
  (plan.rs:101-112, 309-311).
- 🔵 **Code Quality**: `rows_for` has one caller — Resolved (folded into
  `only_row`).
- 🔵 **Standards**: The `SyncState` import is hedged — Resolved. The plan
  now says to add it after `use work::sync::SyncDirection;`
  (sync_create.rs:34). It is not already imported.
- 🔵 **Compatibility**: The CHANGELOG names only the `-` form of the row —
  Resolved ("a second `noop\tunsynced` row").

Verified directly:

- The new Python test fails against today's table: row 466 has no
  `` `order-refused` `` and does contain `` `create-from-local` ``. It passes
  after the edit. The check was run against `_table_rows`/`_section` copied
  from the test module. `"`noop\\tunsynced`"` is a backslash followed by `t`,
  which matches the Markdown. No cell contains a pipe.
  `test_an_adopt_row_is_a_non_blocking_adopted_entry` still picks the
  `adopt`/`relink` row.
- `-k refused_order` selects only the new test.
- The `git grep -nE` check matches SKILL.md:466 and the test name at :450
  today, and would match nothing in `skills`/`tests` after the edits.
- `only_row`, `stored_watermark` and both seed helpers compile against the
  real APIs. `Action` is `Debug + Copy`, `Entry::stamps` is
  `const fn(&self) -> Option<&SyncStamps>`, and `DigestRecipe::CURRENT` is a
  `pub u64`.

### New Issues Introduced

#### Minor

- 🔵 **Compatibility**: The work item still says the Membership rule is
  removed, not narrowed
  **Location**: Phase 2; Desired End State
  The work item's Requirements bullet ("interim membership rule … and its
  structure test are removed"), its last acceptance criterion and its
  Assumptions section still describe removing the old rule. Phase 2 instead
  replaces it with an `order-refused` rule, which is the right call. If
  someone checks that criterion literally, they could delete the narrowed row
  and bring back the double summary of a refused draft.
  **Fix**: Reword those three places in
  `meta/work/0352-sync-reports-every-unsynced-item-twice.md` to "narrowed to
  the `order-refused` pairing". Alternatively, add one sentence to Phase 2
  saying that it deliberately reads the criterion this way.

#### Suggestions

- 🔵 **Test Coverage**: The successful-create watermark guard cannot catch
  the failure the plan says it catches
  **Location**: Implementation Approach; Phase 1 §4 item 1; §3 test 4
  The Implementation Approach groups "a successful create's watermark" with
  the guards that "catch a partition placed before the `--pull-only` check".
  But the create path writes the new entry with `local_synced_at` set to the
  run start itself, so this assertion holds whatever the partition does. The
  same applies to test 4's watermark assertion; test 4 is red first only
  through its row count. Test 8 already labels its watermark assertion as a
  guard.
  **Fix**: Describe it as pinning the end state, not as catching a misplaced
  partition. No new test is needed.

### Assessment

All of pass 2's findings are resolved. Phase 2's narrowed rule is correct
against the CLI's actual output and is the only pairing that remains. Its new
test goes red, then green, as claimed, and the grep check works. The
remaining minor finding is a mismatch in the work item's wording. The
suggestion is a wording fix. Neither blocks implementation. The plan is ready
to execute.
