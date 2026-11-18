---
type: "codebase-research"
id: "2026-11-10-0352-sync-reports-every-unsynced-item-twice"
title: "Research: Sync Reports Every Unsynced Item Twice — Review Findings Against the Code"
date: "2026-11-10T12:04:22+00:00"
author: "Phil Helm"
producer: "research-codebase"
status: "complete"
work_item_id: "0352"
parent: "work-item:0352"
relates_to: ["codebase-research:2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently", "codebase-research:2026-10-10-0296-sync-round-trip-defects"]
topic: "Sync Reports Every Unsynced Item Twice"
tags: ["research", "codebase", "sync", "report", "work-adapters", "baseline", "sync-work-items"]
revision: "c1b9cff9e11e6b32bedabeebd371d700191813de"
repository: "accelerator-git"
last_updated: "2026-11-10T12:04:22+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# Research: Sync Reports Every Unsynced Item Twice — Review Findings Against the Code

**Date**: 2026-11-10T12:04:22+00:00
**Author**: Phil Helm
**Git Commit**: c1b9cff9e11e6b32bedabeebd371d700191813de
**Branch**: t3/pick-next-task-0351-0352
**Repository**: accelerator-git

## Research Question

Check work item 0352 (`meta/work/0352-sync-reports-every-unsynced-item-twice.md`)
and the open findings of its review
(`meta/reviews/work/0352-sync-reports-every-unsynced-item-twice-review-1.md`,
pass 3) against the live code. The aim is to settle the findings that turn on
facts in the code: today's watermark behaviour, the literal report rows, the
`order-refused` output, the run's watermark, the failed-create trigger, the
sibling overlap and the dependencies.

## Summary

The work item's diagnosis is right, and every line reference except one is
correct. The research turned up five facts the work item does not yet account
for. Two of them change what it claims:

1. **Something does read the leftover entry.** `noting_unsupported_recipe`
   (`cli/work-adapters/src/sync/run.rs:1622-1636`) rewrites the leftover
   `(Unsynced, Noop)` plan row when the unsynced item still has an
   `Entry::Unsupported` baseline entry. The row then reads
   `<id>\tnoop\tunsynced\tunsupported-recipe`, which feeds the
   unsupported-recipe warning and marks the dossier `Unobserved`. The create row
   goes through no such rewrite, so the fix silently drops that signal in this
   edge case.
2. **A rejected create is not `terminal`.** A rejection prints
   `failed\tunsynced\trejected` (Jira body conversion) or
   `failed\tunsynced\tretryable` (most Linear and Jira 4xx responses). The way
   to reproduce `failed … terminal` by hand is a leftover pending-push marker,
   which `push_precondition` refuses before the tracker is called.
3. **A successful apply prints no "created" word.** The row is
   `<id>\tcreate-from-local\tunsynced\t-`, the same as in a preview. An adopt
   prints `<id>\tadopt\tunsynced\t-` plus a `#\tadopt\t<id>\t<KEY>\t<title>`
   record.
4. **AC6 does not hold for every successful create.** An adopt whose title or
   body differ writes no baseline entry (`CreateNotice::DifferingAdoption`,
   `apply.rs:1385`).
5. **Today, drafts and `--pull-only` items join `advance_ids` through their
   `noop` row, and the advance does nothing.** They have no baseline entry, and
   classification never reads the watermark while `external_id` is absent.
   After the fix they keep their `noop` row, so their behaviour does not change.

The "run's watermark" is `run_start_epoch`, which `prepare_run`
(`run.rs:2178-2179`) captures once, in whole seconds. AC5 can only fail red if
the test seeds the entry's `local_synced_at` well apart from the fixed test
clock (`1_700_000_000`). 0353 edited neighbouring rows of the same skill table
and test file, but it is done and does not conflict. 0354 does not overlap.
`bin/accelerator` ties the launcher to the plugin version.

## Detailed Findings

### Line references

Every reference is correct except the decision table one:

- `cli/work/src/sync/decide.rs:18-23` is the doc comment on `enum Action`.
- The arm that maps `Unsynced` to `Noop` is
  `cli/work/src/sync/decide.rs:116-119`.
- `the_two_create_actions_never_come_from_the_decision_table`
  (`decide.rs:202-253`) pins that mapping.

These are all correct as cited:

- `unsynced_creates` (`run.rs:1346-1368`) and `relinks` (`run.rs:1288-1310`).
- The preview loop (`run.rs:2360-2401`) and `definitively_reconciled`
  (`run.rs:368-384`).
- `finalise_baseline` (`run.rs:334-363`), `refused_orders`
  (`run.rs:1373-1395`) and `validate_pushes` (`run.rs:1000-1037`).
- `PlannedAction::ordered` (`cli/work/src/sync/plan.rs:90-99`).
- `record` (`apply.rs:1672-1692`), `link_and_baseline` (`apply.rs:1596-1620`)
  and `advance_watermark` (`baseline.rs:483-487`).
- The dossier arm (`dossiers.rs:90-95`) and the decide golden (lines 32-34).
- `cli_sync.rs:326-363`, `sync_create.rs:2349-2381`, `SKILL.md:466` and the
  Python test at line 450.

The prepare step runs in this order (`run.rs`):

- 2212-2213: build the plan.
- 2214: `relinks(&mut plan, …)`.
- 2235-2237: `unsynced_creates(&plan, …)`, extended by the relinks.
- 2242-2257: check the pull and push bounds.
- 2259: `refused_orders`.

### How `unsynced_creates` selects entries

`run.rs:1346-1368`:

1. Under `PullOnly` it returns empty before reading the plan (1351-1353).
2. It keeps only entries whose state is `SyncState::Unsynced` (1356).
   `Unsynced` means the item has no `external_id`, whatever its baseline holds
   (`cli/work/src/sync/classify.rs:545-546`).
3. It finds the matching item in `request.reconciled()` (1357-1365). The plan
   is built from that slice, so a match always exists.
4. It drops drafts: `DraftId::parse(&create.item.id).is_none()` (1366).

Each entry is **cloned** into a `LocalCreate`, and the original stays in
`plan.actions`. That is the defect.

`relinks` behaves differently. It partitions the `Relink` entries out first
(1293-1297) and only then returns empty under `--pull-only` (1298-1300). So
under `--pull-only` a relink item has no engine row at all, only an
`order-refused` line. The work item's warning that `unsynced_creates` must not
copy that ordering is correct.

An unconfirmed link carrying an order is planned as `Relink`, not as the
ordered `Noop` (`plan.rs:310-312`, checked before the unsynced arm at
313-315). AC4's "unsynced item carrying an adopt order" therefore never covers
a relink.

### Report rows

`render_report` (`cli/work-cli/src/sync.rs:584-616`) builds each row as
`TsvRecord::of([id, action_field, state, detail])`:

- **action field:** `failed` for a `Failed` outcome. Otherwise
  `action_keyword` (`sync.rs:430-439`): `adopt` for a `CreateFromLocal`
  carrying an adopt order, and otherwise `plain_action_keyword` (441-456).
- **state:** `Unsynced` renders as `unsynced` (`cli/work/src/sync/state.rs:38`).
  Every create row is stated `Unsynced` (`create_report`, `run.rs:1459-1472`).
- **detail:** the failure class keyword (`apply.rs:89-101`). For `Applied` and
  `NotApplied` it is `-`, except that a `Relink` shows its order (`row_detail`,
  `sync.rs:421-426`).

The row lines are sorted lexicographically before printing (`sync.rs:621`).
`adopt`, `create-from-local` and `failed` all sort before `noop`, so the create
row always prints first. `RunReport.reported` keeps the engine's order, in
which the `noop` row comes first. That is why `row()` and `outcome_of` find
the `noop` row.

| Case | Today | After the fix |
|---|---|---|
| Preview, plain create | `create-from-local\tunsynced\t-` + `noop\tunsynced\t-` | `create-from-local` row only |
| Preview, adopt order | `adopt\tunsynced\t-` + `noop` + `#\tadopt\t<id>\t<KEY>\t<title>` | `adopt` row + `#\tadopt` record |
| Apply, successful create | `create-from-local\tunsynced\t-` + `noop` (no `created` word, `sync.rs:603-604`, test at `sync.rs:8543`) | `create-from-local` row only |
| Apply, successful adopt | `adopt\tunsynced\t-` + `#\tadopt\t<id>\t<KEY>\t<title>` + `noop` (`sync.rs:8277, 8284`) | `adopt` row + `#\tadopt` record |
| Apply, failed create | `failed\tunsynced\t<class>` + `noop` | `failed` row only |
| Unsynced draft | `noop\tunsynced\t-` | unchanged |
| Unsynced, `--pull-only` | `noop\tunsynced\t-` | unchanged |
| Order on a draft or under `--pull-only` | `failed\t-\torder-refused` + `noop\tunsynced\t-`; exit 75 (`sync.rs:871-872`); cause on stderr | unchanged (out of scope) |
| Unsynced item with an `Unsupported` baseline entry | `create-from-local…` + `noop\tunsynced\tunsupported-recipe` | `create-from-local` row only; the warning is lost (see below) |

A draft reaches the engine only when it is not being promoted; `plannable`
drops drafts that are promoting or promoted
(`cli/work-adapters/src/sync/identity_settlement.rs:819-823`).

`<class>` for a failed create:

- `terminal`: `Tracker{Terminal}`, `PendingPushRefused`, or any I/O failure
  after something landed (`apply.rs:287, 301-305`; promoted by
  `attempt.rs:189-230`).
- `rejected`: a Jira `RequestInvalid`, the ADF conversion failure
  (`sync.rs:4468`).
- `retryable`: most tracker 4xx responses on a fresh create, whose progress
  `abandon_attempt` rolls back (`apply.rs:1527-1548`).
- `order-refused`: an adopt refused at apply time.
- `unparseable`: a `create_inputs` parse failure.

### The one reader of the leftover entry

The work item says nothing reads the leftover entries after planning. That
holds with one exception.

What checks out:

- **Bounds:** `pull_count` and `push_count` count no `Noop`
  (`plan.rs:185-198`).
- **`validate_pushes`:** gives a `Noop` `NotApplied` with no validation.
- **Apply loop:** `Action::Noop => NotApplied` (`run.rs:1696`).
- **Dossiers:** treat `(Unsynced, Noop)` and `(Unsynced, CreateFromLocal)`
  alike (`dossiers.rs:90-95`). `settled_by` needs every row for an id to be
  settled (`dossiers.rs:260-268`).
- **Summary:** counts only `Synced` rows (`sync.rs:589-593`).
- **Exit code:** reads only failures (`report.rs:236-306`,
  `sync.rs:855-875`).
- **`refused_orders`:** reads only `pending_push_orders`, the creates and
  `plan.unclassifiable`.

**The exception: `noting_unsupported_recipe` (`run.rs:1622-1636`).** It runs
on plan rows only, through `reported_as` (`run.rs:2357-2359, 2364, 2450`).
When an unsynced item still has an `Entry::Unsupported` baseline entry, the
item joins `unsupported_recipe` (`run.rs:2217-2227`) whatever its state. Three
things follow:

- its `noop` row becomes `UnsupportedRecipe`, rendering
  `<id>\tnoop\tunsynced\tunsupported-recipe`;
- that row feeds `unsupported_recipe_warning` (`sync.rs:879-895`);
- that row gives the item's dossier status `Unobserved`
  (`dossiers.rs:137-139`), which blocks retiring the dossier.

The create row is built by `create_report` and is not routed through
`reported_as`. Removing the `noop` entry therefore removes the warning and the
`Unobserved` status for this case. It needs an unsynced item with an
unknown-recipe baseline entry, which takes a hand-removed `external_id` on an
item synced by a newer binary. Rare, but the work item's "nothing downstream
reads the removed entries" (Size line) and its "Nothing after planning reads
the leftover entries" bullet are not quite true. Before planning, decide
whether this signal should move onto the create row, or be accepted as lost.

### The watermark

**The run's watermark** is `run_start_epoch`:

- It is captured once at the top of `prepare_run` (`run.rs:2178-2179`), from
  `ports.clock.run_start_epoch()`.
- `SystemClock` (`cli/work-cli/src/sync.rs:120-129`) truncates it to whole
  seconds.
- It is taken after preflight, identity settlement and rediscovery
  (`settled_run.rs:159-243`), so it is the engine's start, not the process's.
- The same value goes to `finalise_run`'s per-entry advance
  (`baseline_store.rs:158-160`) and to the document timestamp, which advances
  only on whole-corpus runs (`run.rs:348-351`).
- It is also what `set_baseline` writes as a new entry's `local_synced_at`
  (`apply.rs:1008-1011`).
- A preview never finalises (`Finalisation::NotAttempted`, `run.rs:2394`).

"The run's watermark" in AC6 can be stated precisely as "the run's start
epoch, `run_start_epoch`".

**Who joins `advance_ids` today.** Every apply-run row that passes
`definitively_reconciled` (`run.rs:341-347`). That function treats `Noop` as
deferred only for `LocallyModified` and `RemotelyModified` (`run.rs:369-371`),
so an `(Unsynced, Noop, NotApplied)` row passes. As a result:

- **An unsynced draft, or an unsynced item under `--pull-only`**, joins through
  its `noop` row. `advance_watermark` does nothing without an entry
  (`baseline.rs:483-487`), and these items normally have none. Even with one,
  `classify` returns `Unsynced` as soon as `external_id` is absent
  (`classify.rs:545-547`), so the watermark is never consulted. This is
  "today's behaviour", and it stays the same after the fix because these items
  keep their `noop` row. A criterion can pin it as "the item's id still joins
  the advance", or simply "its baseline entry, if any, advances to the run
  start", but no observable effect follows from it.
- **A failed create** joins through the leftover `noop` row; its own `Failed`
  row is excluded (`run.rs:375`). This is the defect.
- **A successful create** joins through its `Applied` row. That is redundant,
  because `record` → `set_baseline` has already written the same epoch. AC6
  (success direction) therefore passes against unfixed code too. It is a
  regression guard, not a red test.

**When the advance is visible.** `local_change` pre-filters with
`mtime > watermark` (`classify.rs:272-275`). The comparison is strict, and
both sides are in whole seconds (`item_snapshot.rs:137-146`). A link written
in the same second as the run start, followed by a failure, hides that edit
from the next run. The failure points after the link are:

- `ReadLocal` (`apply.rs:1614-1617`);
- `HashLocal` and `BaselineSet` inside `record` (`apply.rs:1679, 1689`);
- a link write that landed but was not durable (`attempt.rs:139-141`);
- the same three steps on the adopt path (`apply.rs:1375-1391`).

All of these report `terminal`.

**When an unsynced item can carry a baseline entry.**

- Its `external_id` was removed by hand, or the work file was reverted to a
  version before the link. The baseline is gitignored integration state
  (`baseline.rs:496-500`), so it survives a VCS revert.
- `record_adoption` forgets the stale entry only when
  `differs || request.linked.is_some()` (`apply.rs:1370-1373, 1408-1418`).
- It cannot happen through a relink (`adoption_records.rs:268-282`), a fresh
  failed create, or a rename (`baseline_store.rs:114-126`).

**AC6 and differing adoptions.** A successful adopt whose title or body differ
links the file but writes **no** baseline entry (`apply.rs:1385`,
`CreateNotice::DifferingAdoption`). AC6 ("the item has a baseline entry whose
`local_synced_at` is the run's watermark") needs to exclude that path, or say
"a create or an equivalent adopt".

### Orders

Pending-push orders come **only** from the
`--resolve <id>=create|adopt:<KEY>` flag:

- `cli/work-cli/src/sync.rs:378-418` parses it, through
  `resolve_pending_push_token` (`push_precondition.rs:99-115`).
- It is passed as `pending_push_orders` (`sync.rs:2270`) into
  `PlanOrders.pending_push` (`plan.rs:205`).
- `plan()` attaches the order to the unsynced `Noop` entry (`plan.rs:313-315`).
- The clone carries it into the `LocalCreate`. `apply_local_create` reads it
  (`run.rs:1775`), and `with_order_of` copies it back onto the row
  (`run.rs:1278`, `plan.rs:128-136`).

The pending-push *marker* (`<integrations>/<integration>/pending-push/<id>.json`,
`pending_push.rs:72-79`) is crash-recovery state that an order settles. It is
not where orders are stored.

The work item's gloss, "an … instruction recorded against an unsynced item",
would be clearer as "an instruction given for this run with
`--resolve <id>=adopt:<KEY>|create`".

### Reproducing a failed create by hand

A tracker "rejection" in the everyday sense does not produce `terminal`:

- **Linear** (`cli/linear-client/src/classify.rs:154-191`): a `Create` that
  fails with BadRequest, auth or rate-limit errors (an invalid `teamId`,
  validation errors) is **retryable**. Non-JSON 2xx responses, transport
  failures, 5xx after retries and a missing identifier are terminal.
- **Jira** (`cli/jira-client/src/classify.rs:52-73`): 400, 401, 403, 404, 410
  and 429 are **retryable**. That covers an invalid project, a missing
  required field and an unknown issue type. `RequestInvalid`, the ADF
  conversion of a body with a table, is **rejected**. 5xx, non-JSON and a
  missing key are terminal.

The most reliable manual way to get `failed\tunsynced\tterminal` is:

1. Place an `Attempted` marker, or an unparseable `{`, at
   `<integrations>/<integration>/pending-push/<id>.json`.
2. Run `accelerator work sync --target <id>`.
3. `push_precondition` refuses with `prior-attempt-unknown` or
   `marker-unreadable` (`push_precondition.rs:137-144`). That is
   `PendingPushRefused`, which is terminal (`apply.rs:305`), and the tracker is
   never called.

`record_terminal_failure` keeps an `Attempted` marker
(`apply.rs:1566-1575`). That is a plausible origin of the `0004 failed
unsynced terminal` row seen in the sandbox; it is an inference, not traced.

The work item's "Failed-create form … against a tracker that rejects the
create. Actual: a `failed\tunsynced\tterminal` row" should become one of:

- "a tracker that rejects the create" with Actual `failed\tunsynced\t<class>`;
  or
- the marker recipe above with Actual `failed\tunsynced\tterminal`.

The Summary's "`failed … terminal` when the create fails" should also say
`failed` rows carry whichever class the failure has.

### Tests

**Rows that depend on the duplicate.** Only
`a_push_only_preview_plans_an_unsynced_item_without_reaching_the_tracker`
(`cli/work-cli/tests/cli_sync.rs:326-363`) asserts a `noop\tunsynced` row. It
compares the whole of stdout:

```text
0001\tcreate-from-local\tunsynced\t-
0001\tnoop\tunsynced\t-
#\tdiscovery\tskipped\tpush-only
```

Every other test that touches an unsynced item's rows uses `.any`, `.find` or
`contains`, so none breaks:

- `sync.rs:8263, 8524, 7289`;
- `sync_create.rs`: `create_from_local_outcome`, `create_from_local_failure`,
  `local_create_row`, `relink_row` and `local_create_failing`.

The `reported.len()` count checks in `sync_run.rs` and
`sync_run_real_client.rs` use corpora with no unsynced item.

**Helpers.**

- `row()` and `outcome_of` (`cli/work-adapters/tests/support/repo.rs:756-770`)
  return the first match.
- `Repo` gives every item an `external_id` (`repo.rs:526`), so it cannot host
  an unsynced item. The new tests belong with `sync_create.rs` and its
  `Fixture`.
- No all-rows helper exists. The local pattern to copy is `failure_for`
  (`sync_create.rs:3501-3509`, filter then `assert_eq!(len, 1)`), together with
  the production filter in `settled_by` (`dossiers.rs:260-268`).
- `failure_for` counts failures, not rows, so it passes today even with the
  duplicate.

**Failure seams.**

- `RecordingTracker::failing_create(TrackerError::Rejected | Retryable | Terminal)`
  (`sync_create.rs:2325, 2350, 2926`).
- `AuthorFault::LinkFails`, `LinkWritesInvalidUtf8` (a `ReadLocal` failure
  after the link) and `LinkNotDurable`, through `local_create_failing`
  (`:2827`).
- `Spy::fail_nth_write_to(BASELINE_PATH, 1)`, a `BaselineSet` failure after
  the link that leaves the later finalise write working (`:2985`).
- `each_failure_after_a_link_reads_created_then_after_link`
  (`sync_create.rs:2967-3004`) already drives each after-link failure.

**Watermark tests.**

- Every test clock is a fixed `1_700_000_000`: `FixedClock`
  (`sync_create.rs:183-189`), `repo.rs:286-292`, `sync_run.rs:173`,
  `sync_settled.rs:259` and `sync_recipe_migration.rs:166`.
- Seed an entry with a raw fragment (`sync_run.rs:636-638`),
  `support::split_entry` (`support/mod.rs:533-555`) or `support::entry_fragment`
  with custom `SyncStamps` (`support/mod.rs:563`).
- Read it back with `Baseline::read(..).get(id).and_then(Entry::stamps)`
  (`sync_run.rs:655-669`, `sync_create.rs:1741-1755`).
- `run_create_from_local_in` (`sync_create.rs:2442`) re-seeds an **empty**
  baseline each run. AC5 has to use the `fixture.spy`-based `run_sync` or
  `run_ordered` paths, seeding first.
- Because the advance writes `local_synced_at` whenever the entry exists, any
  failed create shows the defect in the stored value, for example
  `failing_create` with `Retryable`. An after-link failure is not needed.
- An unsynced item with a baseline entry still classifies `Unsynced`
  (`cli/work/tests/fixtures/work-item-sync-classify.json:170`). No existing
  test seeds one.
- **For AC5 to fail red,** seed `local_synced_at` well apart from
  `1_700_000_000` (for example `5`) and assert the stored value directly. A
  test that goes through the next run's classification cannot tell fixed from
  unfixed code: fixture mtimes are in 2026, so `mtime > watermark` always
  holds.

**New ground.** No test today asserts that a draft or a `--pull-only` unsynced
item yields a `noop` row. The order-refusal tests check only `failure_for` and
nothing sent, not the `noop` row:

- `an_order_under_pull_only_is_refused` (`:3533`);
- `an_order_naming_a_draft_is_refused` (`:3560`).

AC3 and any criterion pinning the ordered-draft output need new tests.
Selection harnesses exist for both run modes: `run_sync` (whole corpus),
`run_sync_targeted` (targeted) and `OrderedRun.selection`. Targeted and
whole-corpus runs build and report creates identically. They differ only in
which items are reconciled, the stderr refusal cause (`TargetedOut` against
`DeferredByIdentity`), the document watermark and dossier retirement.

### `/sync-work-items` interim rule

The **Membership table** is at `skills/work/sync-work-items/SKILL.md:444-466`,
with columns `Record | Group | Blocks completion`. It is "the single source for
Step 5's groups":

- the `needs-retry`, `needs-attention`, `needs-upgrade`, `recorded`,
  `kept-edits:`, `in sync:` and `adopted:` groups;
- "their Step 5 group" for skipped conflicts, `not-found`, `remote-absent` and
  `not promoted`.

The Completion rule (`:471-474`) and Step 5 (`:652-657`) defer to it. Line 466
is the interim row:

```
| `noop\tunsynced` row for an id that also has a `failed`, `create-from-local` or `adopt` row | none: that row speaks for the item | no |
```

`test_a_create_s_duplicate_noop_row_joins_no_group`
(`tests/unit/tasks/test_sync_work_items_skill_structure.py:450-457`) pins
it. Removing both leaves the neighbouring tests unaffected:

- `test_every_group_is_listed_with_its_blocking_flag` (`:425-432`) still gets
  `no` from other rows;
- the dispatch parametrisation (`:317-337`) never lists `noop\tunsynced`.

The work item can gloss "membership rule" as "a row in the skill's Membership
table, which maps each report record to a Step 5 summary group and says
whether it blocks completion".

### Docs and changelog

`docs-site/` never mentions `noop` or `unsynced`. `CHANGELOG.md:132-133`
documents a different row, `noop\tindeterminate\tunsupported-recipe`. Nothing
documents the duplicate row, so the work item's Assumption holds.

## Code References

- `cli/work-adapters/src/sync/run.rs:1346-1368` — `unsynced_creates`; it clones
  and does not partition.
- `cli/work-adapters/src/sync/run.rs:1288-1310` — `relinks`; it partitions,
  then returns empty under `--pull-only`.
- `cli/work-adapters/src/sync/run.rs:334-363, 368-384` — `finalise_baseline`
  and `definitively_reconciled`.
- `cli/work-adapters/src/sync/run.rs:1622-1636, 2217-2227, 2357-2364` —
  `noting_unsupported_recipe`, the one reader of the leftover entry.
- `cli/work-adapters/src/sync/run.rs:2178-2179` — `run_start_epoch` capture.
- `cli/work-adapters/src/sync/run.rs:1373-1426` — `refused_orders` and
  `why_no_create`.
- `cli/work-adapters/src/sync/run.rs:2360-2401, 2434-2470` — the preview and
  apply reporting loops.
- `cli/work-adapters/src/sync/apply.rs:1157-1212, 1464-1620, 1672-1692` — the
  create lifecycle, `link_and_baseline` and `record`.
- `cli/work-adapters/src/sync/apply.rs:1385` — a differing adoption writes no
  baseline entry.
- `cli/work-adapters/src/sync/apply.rs:277-317` — failure classification.
- `cli/work-adapters/src/sync/baseline.rs:483-487` — `advance_watermark`.
- `cli/work/src/sync/classify.rs:272-275, 545-547` — the mtime pre-filter, and
  `Unsynced` short-circuiting before the watermark is read.
- `cli/work/src/sync/decide.rs:116-119` — the `Unsynced => Noop` arm.
- `cli/work/src/sync/plan.rs:90-99, 309-320` — ordered noop entries and the
  relink precedence.
- `cli/work-cli/src/sync.rs:378-418` — parsing `--resolve` orders.
- `cli/work-cli/src/sync.rs:421-456, 584-621` — row rendering and sorting.
- `cli/work-cli/tests/cli_sync.rs:326-363` — the only test asserting the
  duplicate.
- `cli/work-adapters/tests/support/repo.rs:756-770` — first-row helpers.
- `cli/work-adapters/tests/sync_create.rs:183-189, 2349-2381, 2967-3004, 3501-3509, 3533, 3560`
  — the clock, failure seams, the `failure_for` pattern and order refusals.
- `skills/work/sync-work-items/SKILL.md:444-474` — the Membership table and
  Completion rule.
- `tests/unit/tasks/test_sync_work_items_skill_structure.py:450-457` — the
  interim rule's test.
- `bin/accelerator:139-144, 404-406` — the plugin-version-locked launcher.

## Architecture Insights

- `decide` never produces creates. Creates and relinks are chosen out-of-band
  after planning, so the plan's `actions` and the create list are two separate
  row sources merged at reporting time. `relinks` moves its entries from one
  source to the other; `unsynced_creates` copies them. The fix makes the two
  consistent, minus the `--pull-only` ordering difference.
- Plan rows and create rows get different post-processing. `reported_as`
  (the unsupported-recipe rewrite) applies to plan rows only, so moving an
  entry between the two sources silently changes which decorations it gets.
- Stdout rows are sorted, so the engine's row order matters only to test
  helpers that take the first match, not to users.
- The watermark is per entry, and an unsynced item's watermark is inert until
  the item is linked. That is why the defect is visible only in a failure
  after the link.

## Historical Context

- `meta/work/0300-sync-pulls-of-tracked-work-items-fail-silently.md` (done) and
  its plan `meta/plans/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently.md`:
  - Lines 2219-2237 introduced `unsynced_creates`, `create_report` and the
    `order-refused` line.
  - Phase 7 (from line 3596) does not mention the duplicate `noop` row.
  - Line 4158 records that 0351-0354 came from its manual checks.
  - 0300 shipped with 0296 as one change.
- `meta/work/0296-sync-round-trip-defects-between-local-work-items-and-the-tracker.md`
  (done) explains why 0352 relates to 0296:
  - it defines the baseline `Entry` with `local_synced_at` (line 592);
  - it owns the `finalise_baseline` / `BaselineStore::finalise_run` behaviour;
  - it covers the baseline written after a create (`record_baseline_after_create`
    in `cli/work-adapters/src/creation.rs`);
  - it made failure classification total, which is why the `failed` row exists.

  Suggested Dependencies line: "Related: 0296 (done) — defines the baseline
  entry (`local_synced_at`) and the `finalise_baseline` behaviour this item
  changes; shipped with 0300."
- `meta/work/0351-sync-preview-hides-pending-push-refusals.md` (draft) has the
  same `relates_to` as 0352 but never mentions 0352. The branch name
  `t3/pick-next-task-0351-0352` pairs them.
  - Its AC1 says "the item's row" (singular), which assumes 0352 has landed.
  - Its AC4 says the no-marker preview "output is unchanged". If 0351 lands
    first, that pins today's two rows and 0352 must update it. If 0352 lands
    first, 0351 writes single-row assertions and nothing of 0352's changes.
  - So the sentence "whichever lands second updates the other's row
    assertions" holds only when 0352 lands second. Suggested rewording: "if
    0351 lands first, 0352 drops the `noop` lines from 0351's preview tests; if
    0352 lands first, 0351 asserts single rows."
  - 0351's open question about giving a reused `Created` marker its own action
    word would add a surviving row that 0352's AC1 does not list.
- `meta/work/0353-sync-work-items-skill-ignores-the-summary-record.md` (done)
  touched the same files 0352 changes, in neighbouring places:
  - the summary-record membership row at `SKILL.md:462`, four lines above
    0352's interim row at 466;
  - `test_the_summary_record_never_blocks_completion` at `:442`, just before
    0352's test at `:450`.

  There is no conflict. The summary record counts `synced` rows, so removing an
  `unsynced` `noop` row cannot change it.
- `meta/work/0354-kept-edits-note-ignores-singular-counts.md` (done) only
  changes `earlier_copies_note` wording, so it does not overlap.
- `bin/accelerator` ties the launcher to the plugin version:
  - it reads the version from `.claude-plugin/plugin.json` (lines 139-144);
  - the cache key and release URL use `v${version}` (lines 404-406);
  - the download is signature-checked.

  Sub-binaries are resolved by the Rust launcher, which this research did not
  check. The contributor overrides (`ACCELERATOR_LAUNCHER_BIN`,
  `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER`) bypass the lock. The review's
  one-line Assumption (skill and binary ship together) holds for released
  installs.

## Related Research

- `meta/research/codebase/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently.md`
- `meta/research/codebase/2026-10-10-0296-sync-round-trip-defects.md`
- `meta/research/codebase/2026-06-18-0051-sync-work-items-skill.md`

## Open Questions

- Should the unsupported-recipe signal (the warning and the dossier
  `Unobserved` status) move onto the create row, or is losing it for an
  unsynced item with an `Entry::Unsupported` baseline entry acceptable? This
  decides whether the fix is a pure partition or also touches `create_report`
  or `reported_as`.
- How does the Rust launcher pick the version of the `work` sub-binary? This
  matters only if the version-lock Assumption must hold beyond the launcher.
- Did the sandbox's `0004 failed unsynced terminal` come from a leftover
  `Attempted` marker? It is inferred, not traced, and matters only for the
  reproduction wording.
