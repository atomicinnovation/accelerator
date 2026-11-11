---
type: "plan"
id: "2026-11-11-0352-sync-reports-every-unsynced-item-twice"
title: "Sync Reports Every Unsynced Item Twice Implementation Plan"
date: "2026-11-11T08:08:16+00:00"
author: "Phil Helm"
producer: "create-plan"
status: "draft"
work_item_id: "work-item:0352"
parent: "work-item:0352"
derived_from: ["codebase-research:2026-11-10-0352-sync-reports-every-unsynced-item-twice"]
relates_to: ["work-item:0351"]
tags: ["sync", "report", "work-adapters", "baseline", "sync-work-items"]
revision: "c1b9cff9e11e6b32bedabeebd371d700191813de"
repository: "accelerator-git"
last_updated: "2026-11-11T08:08:16+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# Sync Reports Every Unsynced Item Twice Implementation Plan

## Overview

`accelerator work sync` reports every unsynced item it plans a create for
twice: once by its create (`create-from-local`, `adopt` or `failed`) and once
as a leftover `noop\tunsynced` row. The leftover row also counts as settled,
so a failed create's baseline watermark advances. This plan makes each such
item report once, by its create, and leaves a failed create's watermark where
it was. Unsynced drafts and items under `--pull-only` keep their single
`noop` row.

## Current State Analysis

A run's report merges two row sources:

1. **The plan** (`plan.actions`). The decision table maps `Unsynced` to
   `Noop` (`cli/work/src/sync/decide.rs:116-119`), so every unsynced item has
   a `Noop` entry.
2. **The create list** (`creates_from_local`). After planning,
   `unsynced_creates` (`cli/work-adapters/src/sync/run.rs:1346-1368`)
   **clones** each unsynced, non-draft entry into a `LocalCreate` and leaves
   the original in `plan.actions`.

`relinks` (`run.rs:1288-1310`), called earlier in `prepare_run`, **moves**
its entries out of the plan with `std::mem::take` and `partition`. Both the
preview loop (`run.rs:2360-2401`) and the apply loop (`run.rs:2434-2470`)
report every plan entry and then every create, so each create's item gets two
rows.

`finalise_baseline` (`run.rs:334-363`) advances the watermark of every id
with a row that passes `definitively_reconciled` (`run.rs:368-384`). The
leftover `(Unsynced, Noop, NotApplied)` row passes, so a failed create's id
advances. This is visible whenever the unsynced item still has a baseline
entry: `advance_watermark` (`baseline.rs:483-487`) overwrites
`local_synced_at` with the run's start time.

The only reader of the leftover entry after planning is
`noting_unsupported_recipe` (`run.rs:1622-1636`). For an unsynced item whose
baseline entry was written under an unknown digest recipe, it relabels the
`noop` row as `UnsupportedRecipe`. That row produces the stderr warning
"…they were left untouched…; upgrade the plugin"
(`cli/work-cli/src/sync.rs:879-895`), and it marks the item's dossier
`Unobserved` (`dossiers.rs:137-139`). Neither is true of an item the run
creates:

- its create runs regardless of the entry;
- a successful create replaces the entry, because `Baseline::set`
  (`baseline.rs:458-460`) inserts unconditionally;
- `classify` returns `Unsynced` before it reads the entry
  (`cli/work/src/sync/classify.rs:545-547`).

The fix drops that relabelled row, with its warning and its `Unobserved`
dossier status, for an item the run creates or adopts. A test pins the drop.
Unsynced drafts and items under `--pull-only` keep their `noop` row, so they
still get the relabelling and the warning.

## Desired End State

- An unsynced, non-draft item in a run other than `--pull-only` has exactly
  one report row. That row is its create's: `create-from-local`, or `adopt`
  when it carries an adopt order, in a preview; and that create's outcome
  (`Applied` or `Failed`) in an apply, including a create or adopt order and
  an adopt refused at apply time.
- An unsynced draft, and any unsynced item under `--pull-only`, keeps exactly
  one `noop` row. An order on such an item still produces its `order-refused`
  failure line.
- A failed create leaves an existing baseline entry's `local_synced_at`
  unchanged.
- A successful create, or an equivalent adopt, leaves the item with a
  baseline entry whose `local_synced_at` is the run's start epoch. A
  differing adopt writes no entry, as today.
- An unsynced item the run creates, carrying an unknown-recipe baseline
  entry, reports only its create row, with no `UnsupportedRecipe` outcome.
- `/sync-work-items` no longer carries the interim rule for a create's
  duplicate `noop` row. Its Membership table instead says that the `noop` row
  beside an `order-refused` line joins no group, the one pairing that still
  occurs.
- `mise run` exits 0.

### Key Discoveries:

- `relinks` handles `--pull-only` **after** partitioning
  (`run.rs:1293-1300`). `unsynced_creates` must return early **before** it
  partitions, or `--pull-only` runs lose their `noop` rows.
- The plan is built from `request.reconciled()`, so a matching local item
  always exists. The partition still checks for the match, so that an entry
  leaves the plan only when it becomes a create.
- `ReportedItem` does not derive `Debug` (`cli/work-adapters/src/sync/report.rs:38`),
  so the new row helper's assertion message prints each row's `Action`
  instead. `ReportedItem` is already imported in `sync_create.rs:54`.
- `split_entry` (`cli/work-adapters/tests/support/mod.rs:533-555`) seeds an
  entry with `local_synced_at: 0`. Every test clock is
  `FixedClock(1_700_000_000)` (`sync_create.rs:183-189`), so a seeded entry
  is enough to make the watermark test fail first.
- `run_sync` (`sync_create.rs:356`) and `run_ordered` (`sync_create.rs:3437`)
  read the baseline from `fixture.spy` without re-seeding it.
  `run_create_from_local_in` (`sync_create.rs:2442`) re-seeds an empty
  baseline, so it cannot host the watermark tests.
- An unknown-recipe entry is any entry whose `digest_recipe` is outside
  `1..=DigestRecipe::CURRENT` (`baseline.rs:19-28`, `:296-303`). The existing
  seed is at `sync_recipe_migration.rs:900-901`.
- Only `a_push_only_preview_plans_an_unsynced_item_without_reaching_the_tracker`
  (`cli/work-cli/tests/cli_sync.rs:326-363`) asserts the duplicate row. Every
  other existing test finds rows with `.any`, `.find` or `failure_for`, so
  none breaks and none can see the duplicate.
- The Membership table in `/sync-work-items` (`skills/work/sync-work-items/SKILL.md:444-466`)
  does not cover every Step 5 group: `pushed`, `pulled`, `pushed-unsynced`
  and `unsynced (not pushed)` (`SKILL.md:660-662, 681`) are fed by no row. So
  a record the table does not list is not guaranteed to join no group. An
  order on a draft, or any order under `--pull-only`, still prints
  `<id>\tnoop\tunsynced\t-` beside `<id>\tfailed\t-\torder-refused`
  (`cli/work-cli/src/sync.rs:563-566`). Today's interim row (`SKILL.md:466`)
  covers that pair, because `order-refused` is a `failed` row.

## What We're NOT Doing

- Changing the decision table. Creates are chosen in the adapter after
  planning, because only the adapter knows the run's direction, drafts and
  orders; the decide golden
  (`cli/work/tests/fixtures/work-item-sync-decide.golden`) and the dossier
  mapping (`cli/work-adapters/src/sync/dossiers.rs:90-95`) stay as they are.
- Changing the `order-refused` failure line that an order on a draft or under
  `--pull-only` produces beside its `noop` row. That predates this bug.
- Routing create rows through `noting_unsupported_recipe`, or keeping the
  `Unobserved` dossier status for an item the run creates.
- Touching 0351's tests. 0351 has not landed, so it will assert single rows
  itself.
- Changing `relinks`, or reworking the existing single-row helpers
  (`local_create_row`, `row`, `outcome_of`).

## Implementation Approach

Test-first throughout. The tests that describe the new behaviour, including
the CLI's expected stdout, are written first and seen to fail. The tests that
guard behaviour which must not change are written before the change and pass
from the start. The draft and `--pull-only` guards catch a partition placed
before the `--pull-only` check. The successful-create watermark checks pin
the end state: the create path writes its own watermark, so they hold
whatever the partition does. Then
`unsynced_creates` becomes a partition, and the CHANGELOG records the fix.

Phase 2 narrows the `/sync-work-items` interim row. After Phase 1, the only
`noop\tunsynced` row that can sit beside a `failed` row for the same id is a
draft's or a `--pull-only` item's, beside its `order-refused` line. The rule
for a create's duplicate row goes; a rule for that pair takes its place, so
the item is still summarised once, under `needs-attention`. Each phase can
merge on its own, in order.

## Phase 1: Report each unsynced item once, by its create

### Overview

Make `unsynced_creates` take the entries it turns into creates out of
`plan.actions`, and drive that change with tests that check every row for an
id.

### Changes Required:

#### 1. Every-row helpers and named baseline fixtures

**File**: `cli/work-adapters/tests/sync_create.rs`
**Changes**: Add beside `failure_for` (around line 3502), and use them in
every new or extended test.

```rust
fn only_row<'r>(report: &'r RunReport, id: &str) -> &'r ReportedItem {
    let rows: Vec<&ReportedItem> = report
        .reported
        .iter()
        .filter(|item| item.planned.id() == id)
        .collect();
    let actions: Vec<Action> =
        rows.iter().map(|row| row.planned.action()).collect();
    assert_eq!(rows.len(), 1, "{id} reported as {actions:?}");
    rows[0]
}

fn stored_watermark(fixture: &Fixture, id: &str) -> Option<u64> {
    fixture
        .baseline()
        .get(id)
        .and_then(work_adapters::sync::baseline::Entry::stamps)
        .map(|stamps| stamps.local_synced_at)
}

fn seed_recorded_entry(
    fixture: &Fixture,
    item: &LocalItem,
) -> Result<(), TestError> {
    let entry = support::split_entry(
        &item.id,
        &PlainDialect,
        &std::fs::read_to_string(&item.path)?,
        "Draft one\nBody\n",
        "2026-06-01T00:00:00Z",
    );
    fixture.spy.seed(BASELINE_PATH, &baseline_document(&[entry]));
    Ok(())
}

fn seed_unknown_recipe_entry(fixture: &Fixture, id: &str) {
    let unknown = work_adapters::sync::baseline::DigestRecipe::CURRENT + 1;
    let entry = format!(
        "\"{id}\":{{\"digest_recipe\":{unknown},\"local_hash\":\"l\",\
         \"local_synced_at\":5,\"remote_updated_at\":\"t\"}}"
    );
    fixture.spy.seed(BASELINE_PATH, &baseline_document(&[entry]));
}
```

`seed_recorded_entry` is the seeding half of `linked_and_recorded`
(`sync_create.rs:4840-4856`). Rewrite `linked_and_recorded` to call it, so
the entry is built in one place.

Add `use work::sync::SyncState;` after `use work::sync::SyncDirection;`
(`sync_create.rs:34`). No other import is needed.

#### 2. CLI preview test (red first)

**File**: `cli/work-cli/tests/cli_sync.rs`
**Changes**: In
`a_push_only_preview_plans_an_unsynced_item_without_reaching_the_tracker`
(lines 357-361), drop the `noop` line from the expected stdout, and see the
test fail before the partition is written:

```rust
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "0001\tcreate-from-local\tunsynced\t-\n\
         #\tdiscovery\tskipped\tpush-only\n"
    );
```

#### 3. Behaviour tests (red first)

**File**: `cli/work-adapters/tests/sync_create.rs`
**Changes**: Add these after `a_preview_reports_an_inert_order_without_sending`
(around line 3728), with no section banner. Each must fail against today's
code, by showing two rows or a moved watermark, before the partition is
written.

1. `a_previewed_create_is_the_items_only_row`: `run_sync` over
   `fixture.unsynced_item("0001", "Draft one")` with `RunMode::Preview`.
   `only_row(&report, "0001")` has action `CreateFromLocal` and outcome
   `NotApplied`.
2. `a_previewed_adopt_is_the_items_only_row`: `run_ordered` with
   `orders(&[("0001", adopt("ENG-7"))])`, `mode: RunMode::Preview`, and a
   tracker holding `keyed("ENG-7", "Draft one\nBody\n")`. The only row is
   `CreateFromLocal`, with `planned.order() == Some(&adopt("ENG-7"))`.
3. `an_applied_create_is_the_items_only_row`: `run_sync` with
   `RunMode::Apply`. The only row is `CreateFromLocal` with outcome
   `Applied`.
4. `an_equivalent_adopt_is_the_items_only_row_and_watermarks_its_entry`:
   `adopting(...)` with `"ENG-7"`, the tracker holding the equivalent issue.
   The only row is `CreateFromLocal`, `Applied`, carrying the adopt order,
   and `stored_watermark(&fixture, "0001") == Some(1_700_000_000)`.
5. `an_ordered_create_is_the_items_only_row`: `creating(...)`
   (`sync_create.rs:3800`) against a tracker that accepts the create. The
   only row is `CreateFromLocal`, `Applied`, with
   `planned.order() == Some(&PendingPushOrder::Create)`.
6. `a_failed_create_is_the_items_only_row`: `run_sync` with
   `RecordingTracker::holding(Vec::new()).failing_create(TrackerError::Retryable { .. })`.
   The only row is `CreateFromLocal` with an `ItemOutcome::Failed(_)`
   outcome.
7. `a_failed_create_leaves_the_items_watermark_where_it_was`: call
   `seed_recorded_entry(&fixture, &item)?`, read
   `let before = stored_watermark(&fixture, "0001");` and
   `assert!(before.is_some())`, run `run_sync` in apply mode with the failing
   tracker from test 6, then assert
   `stored_watermark(&fixture, "0001") == before`. Today it reads
   `Some(1_700_000_000)`.
8. `an_unknown_recipe_entry_leaves_only_the_create_row`: call
   `seed_unknown_recipe_entry(&fixture, "0001")`, then run `run_sync` in
   apply mode against a tracker that accepts the create.
   - The only row is `CreateFromLocal`, `Applied`. This fails today.
   - No row in the report has `ItemOutcome::UnsupportedRecipe`. This fails
     today.
   - `stored_watermark(&fixture, "0001") == Some(1_700_000_000)`, showing
     the create replaced the entry under a recipe this binary knows. This is
     a guard: it passes before and after the partition.

Also extend `adopting_a_missing_issue_is_refused` (`sync_create.rs:3892`):
after its existing assertions, assert that `only_row(&report, "0001")` has
action `CreateFromLocal` and an `ItemOutcome::Failed(_)` outcome. See it fail
before the partition.

#### 4. Regression tests (green before and after)

**File**: `cli/work-adapters/tests/sync_create.rs`
**Changes**: Write these before the partition. They pass today and must still
pass after it.

1. `a_successful_create_watermarks_its_new_entry_at_the_run_start`:
   `run_sync` in apply mode with an empty baseline.
   `stored_watermark(&fixture, "0001") == Some(1_700_000_000)`.
2. `an_unsynced_draft_keeps_its_single_noop_row`: `run_sync` in apply mode
   over `fixture.unsynced_item("draft-k7mq3x", "Draft one")`. The only row is
   `Noop` / `NotApplied`, stated `SyncState::Unsynced`.
3. `an_unsynced_item_under_pull_only_keeps_its_single_noop_row`: `run_sync`
   with `SyncDirection::PullOnly`. The only row for `0001` is `Noop` /
   `NotApplied`, and the tracker received no `Call::Create`. This guard fails
   if the partition runs before the `--pull-only` return.
4. Extend `an_order_under_pull_only_is_refused` (`sync_create.rs:3534`) and
   `an_order_naming_a_draft_is_refused` (`sync_create.rs:3561`): after their
   existing assertions, assert that `only_row` for the item has action
   `Action::Noop`.

#### 5. The partition

**File**: `cli/work-adapters/src/sync/run.rs`
**Changes**: Replace `unsynced_creates` (lines 1343-1368) with a partition
that returns early under `--pull-only`. It uses a loop rather than
`partition`, as `relinks` does, because each entry that leaves the plan
carries the local item it matched.

The `Action::Noop` clause in `creatable_item` is a deliberate guard that no
test can reach: the planner gives every unsynced item `Noop` and `relinks`
has already taken the `Relink` entries out. It keeps the predicate stating
the domain rule, "an unsynced item the plan leaves untouched", rather than
relying on `relinks` running first.

```rust
/// Takes the unsynced items a run creates from local out of `plan`, so each
/// is reported once, by its create.
fn unsynced_creates<'a>(
    plan: &mut SyncPlan,
    items: &'a [LocalItem],
    direction: SyncDirection,
) -> Vec<LocalCreate<'a>> {
    if matches!(direction, SyncDirection::PullOnly) {
        return Vec::new();
    }
    let mut creates = Vec::new();
    let mut remaining = Vec::new();
    for planned in std::mem::take(&mut plan.actions) {
        match creatable_item(&planned, items) {
            Some(item) => creates.push(LocalCreate { item, planned }),
            None => remaining.push(planned),
        }
    }
    plan.actions = remaining;
    creates
}

/// A draft is never created from local: it reaches the tracker only through
/// promotion, which retires its ID.
fn creatable_item<'a>(
    planned: &PlannedAction,
    items: &'a [LocalItem],
) -> Option<&'a LocalItem> {
    if planned.state() != SyncState::Unsynced
        || planned.action() != Action::Noop
        || DraftId::parse(planned.id()).is_some()
    {
        return None;
    }
    items.iter().find(|item| item.id == planned.id())
}
```

**File**: `cli/work-adapters/src/sync/run.rs`
**Changes**: In `prepare_run`, move the call up to sit directly after
`relinks` (line 2214), so the two steps that take entries out of the plan are
side by side, and pass the plan mutably. Delete the old call at lines
2235-2237.

```rust
    let relinks = relinks(&mut plan, request.reconciled(), request.direction);
    let mut creates_from_local =
        unsynced_creates(&mut plan, request.reconciled(), request.direction);
    creates_from_local.extend(relinks);
```

`plan` is already `let mut` (line 2212). Nothing between the old and new
positions reads `plan.actions`, and `pull_count`/`push_count` count no
`Noop`.

#### 6. CHANGELOG

**File**: `CHANGELOG.md`
**Changes**: Add to `## [Unreleased]` → `### Fixed` (line 748), in the
section's bold-lead style:

```markdown
- **`work sync` reports an unsynced item once.** An item it creates or adopts
  in the tracker no longer prints a second `noop\tunsynced` row beside its
  `create-from-local`, `adopt` or `failed` row; scripts that listed unsynced
  items by their `noop` row should key on the `unsynced` state column
  instead. A failed create no longer advances the item's baseline watermark,
  and an unsynced item that sync creates or adopts no longer draws the "left
  untouched" warning when its baseline entry was written by a newer release,
  since its create runs regardless.
```

### Success Criteria:

#### Automated Verification:

- [ ] Before the partition, the CLI preview test fails:
      `cd cli && cargo nextest run -p accelerator-work --test cli_sync a_push_only_preview_plans_an_unsynced_item_without_reaching_the_tracker`
- [ ] Before the partition, behaviour tests 1-8 and the extended
      `adopting_a_missing_issue_is_refused` fail (two rows, an
      `UnsupportedRecipe` row, or a moved watermark), and the regression tests
      in §4 pass:
      `cd cli && cargo nextest run -p work-adapters --test sync_create`
- [ ] After the partition, both commands above pass.
- [ ] The whole `cli/` workspace passes:
      `ACCELERATOR_COVERAGE=off mise run test:unit:cli`
- [ ] Format and lint pass: `mise run cli:check`
- [ ] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [ ] With a tracker configured and an unsynced, non-draft item `<id>`,
      `accelerator work sync --target <id> --preview` prints exactly one row
      for `<id>`: `<id>\tcreate-from-local\tunsynced\t-`.
- [ ] With a leftover `Attempted` pending-push marker at
      `<integrations>/<integration>/pending-push/<id>.json`,
      `accelerator work sync --target <id>` prints exactly one row for
      `<id>`: `<id>\tfailed\tunsynced\tterminal`.
- [ ] `accelerator work sync --target <id> --pull-only` still prints
      `<id>\tnoop\tunsynced\t-`.

---

## Phase 2: Narrow the `/sync-work-items` interim rule to refused orders

### Overview

After Phase 1, a `noop\tunsynced` row sits beside a `failed` row for the same
id only for an order on a draft or under `--pull-only`, where the `failed`
row is the `order-refused` line. The interim rule for a create's duplicate
`noop` row no longer describes any output, so it and its test go. The table
is not exhaustive (Step 5 has groups no row feeds), so the `order-refused`
pair keeps a rule of its own. Without one, the draft could be summarised
under `unsynced (not pushed)` as well as `needs-attention`.

This is a deliberate reading of the work item's last acceptance criterion.
"The duplicate-`noop` membership rule" means the rule for a create's
duplicate row, and that rule is removed. The narrowed `order-refused` rule
covers output that predates this bug, and must not be deleted to satisfy the
criterion literally.

### Changes Required:

#### 1. Structure test (red first)

**File**: `tests/unit/tasks/test_sync_work_items_skill_structure.py`
**Changes**: Replace `test_a_create_s_duplicate_noop_row_joins_no_group`
(lines 450-457) with a test of the narrowed rule, and see it fail against
today's table:

```python
    def test_a_refused_order_s_noop_row_joins_no_group(
        self, output_section: str
    ) -> None:
        rows = _table_rows(output_section, "Record")
        noop = next(
            row for row in rows if row[0].startswith("`noop\\tunsynced`")
        )
        assert "`order-refused`" in noop[0]
        assert "`create-from-local`" not in noop[0]
        assert noop[1].startswith("none")
        assert noop[2] == "no"
```

`test_every_group_is_listed_with_its_blocking_flag` (`:425-432`) and the
dispatch parametrisation (`:317-337`) are unaffected.

#### 2. Membership table

**File**: `skills/work/sync-work-items/SKILL.md`
**Changes**: Replace line 466:

```markdown
| `noop\tunsynced` row for an id that also has a `failed`, `create-from-local` or `adopt` row | none: that row speaks for the item | no |
```

with:

```markdown
| `noop\tunsynced` row for an id that also has an `order-refused` row | none: the refusal speaks for the item | no |
```

Nothing else in the skill refers to the old row. The Completion rule
(`:471-474`) and Step 5 (`:652-657`) defer to the table as a whole.

### Success Criteria:

#### Automated Verification:

- [ ] Before the table edit, the new structure test fails:
      `uv run pytest tests/unit/tasks/test_sync_work_items_skill_structure.py -k refused_order`
- [ ] After it, all the skill structure tests pass:
      `uv run pytest tests/unit/tasks/test_sync_work_items_skill_structure.py`
- [ ] The old rule and its test are gone:
      `git grep -nE 'create_s_duplicate_noop|that also has a .failed., .create-from-local' -- skills tests`
      returns nothing
- [ ] The Python toolchain checks pass: `mise run build-system:check`
- [ ] The full local CI mirror passes: `mise run`

#### Manual Verification:

- [ ] `/sync-work-items` over a corpus with one unsynced item whose create
      fails summarises that item under `needs-attention` (or `needs-retry`
      for a retryable class) and nowhere else.
- [ ] `/sync-work-items` over a run with a `--resolve` order on a draft
      summarises the draft once, under `needs-attention`.

---

## Testing Strategy

### Unit Tests:

- Engine-level tests in `cli/work-adapters/tests/sync_create.rs` check every
  row for an id through `only_row`, never only the first row.
- `tests/unit/tasks/test_sync_work_items_skill_structure.py` pins the
  narrowed Membership rule for a refused order's `noop` row.
- The behaviour tests cover preview and apply; plain create, create order and
  adopt order; success, tracker failure and apply-time adopt refusal; the
  failed-create watermark; and the unknown-recipe entry.
- The regression tests cover drafts, `--pull-only`, orders refused on both,
  and a successful create's watermark.

### Integration Tests:

- `cli/work-cli/tests/cli_sync.rs`: the push-only preview compares the whole
  of stdout, so it confirms the printed form end to end.

### Manual Testing Steps:

1. Configure the `TES` sandbox team. Create an unsynced, non-draft item and
   run `accelerator work sync --target <id> --preview`. Expect one
   `create-from-local` row.
2. Place an `Attempted` marker for the item and run
   `accelerator work sync --target <id>`. Expect one
   `failed\tunsynced\tterminal` row.
3. Remove the marker, run without `--preview`, and expect one
   `create-from-local` row, with the item linked.
4. Repeat step 1 with `--pull-only`. Expect one `noop\tunsynced\t-` row.

## Performance Considerations

None. The partition replaces a clone with a move over the same list.

## Migration Notes

No stored state changes shape. A script that skipped the duplicate
`noop\tunsynced` row keeps working. A script that listed unsynced items by
their `noop` row no longer sees an item the run creates; it should key on the
`unsynced` state column, which every create row also carries. The CHANGELOG
entry says so.

## References

- Original work item: `meta/work/0352-sync-reports-every-unsynced-item-twice.md`
- Related codebase research:
  `meta/research/codebase/2026-11-10-0352-sync-reports-every-unsynced-item-twice.md`
- Work item review:
  `meta/reviews/work/0352-sync-reports-every-unsynced-item-twice-review-1.md`
- Plan review:
  `meta/reviews/plans/2026-11-11-0352-sync-reports-every-unsynced-item-twice-review-1.md`
- Pattern to follow: `relinks` (`cli/work-adapters/src/sync/run.rs:1288-1310`)
- Failure-count pattern: `failure_for`
  (`cli/work-adapters/tests/sync_create.rs:3502-3509`)
