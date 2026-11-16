---
type: "pr-review"
id: "188-review-2"
title: "[0352] Report each unsynced item once, by its create"
date: "2026-11-11T16:56:15+00:00"
author: "Phil Helm"
producer: "review-pr"
status: "complete"
target: "pr:188"
relates_to: ["pr-review:188-review-1"]
reviewer: "Phil Helm"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "documentation", "standards"]
review_number: 2
pr_number: 188
tags: []
last_updated: "2026-11-11T16:56:15+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Code Review: #188 - [0352] Report each unsynced item once, by its create

**Verdict:** COMMENT

This is the second review, of head `a39ec4f5c`. The engine change is still sound. Every lens that traced it agrees that `creatable_item` selects the same set the old clone-and-filter did, and that moving `unsynced_creates` up beside `relinks` changes no other reader of `plan.actions`: the bounds, `refused_orders`, `untracked_to_import`, `finalise_baseline` and the dossier mapping all behave as before. The fix-ups for review 1 all landed cleanly. What is left is mostly about the text around the code. The CHANGELOG's advice to scripts and its watermark rationale are both inaccurate. A skill Membership rule can swallow an unknown-recipe row. One narrow edge case, a *failed* create over a newer release's baseline entry, lost its warning and its dossier protection without a test. Nothing blocks merging.

### Cross-Cutting Themes

- **The unknown-recipe row that drafts and `--pull-only` items keep has no skill rule** (flagged by: compatibility, documentation, correctness). The tests added in 14fcda081 pin a `Noop`/`UnsupportedRecipe` row for these items. It renders as `<id>\tnoop\tunsynced\tunsupported-recipe`, but the skill's dispatch entry (line 355) and its `needs-upgrade` row (line 458) only match the `indeterminate` state. When the same draft also has a refused order, the rewritten rule on line 466 maps its row to "none". The gap is older than this PR, but the PR now pins the behaviour that produces the row and edits the rule that swallows it.
- **The CHANGELOG entry is less accurate than the code** (flagged by: correctness, compatibility, documentation). "Key on the `unsynced` state column" also matches `create-from-remote`, `relink`, `promote` and `not-promoted` rows. The watermark sentence names an edit window that was never at risk. "That sync creates or adopts" leaves out preview output and failed creates.
- **The `--pull-only` part of the create rule sits outside `creatable_item`** (flagged by: architecture, code-quality). This is not the declined request for a shared helper. The point here is narrower: the direction could go into the predicate, so the unit-tested rule is the whole rule and nothing depends on where an early return sits.
- **Two ways to fetch "the row for this id"** (flagged by: code-quality, test-coverage). `only_row` enforces one row per id, while `local_create_row`, `create_from_local_outcome` and `relink_row` still take the first match. The looser style is how the duplicate went unnoticed.

### Tradeoff Analysis

- **Silencing a false warning vs keeping a true one**: dropping the "left untouched" warning is right for a create that lands, because it replaces the entry. For a create that fails, the entry really is left untouched, so the warning was true. Either accept that (and widen the CHANGELOG and PR notes), or keep the signal for failed creates. Accepting it is the smaller change. Whichever you choose, pin it with a test.

### Strengths

- ✅ `creatable_item` is a pure guard-clause predicate, and the new unit test calls it directly to pin the `Action::Noop` clause without depending on call order.
- ✅ `unsynced_creates` now takes its entries out of the plan the same way `relinks` does, and the two calls sit side by side in `prepare_run`, so each item's report row has one source.
- ✅ Hoisting the call is behaviour-neutral. The unsupported-recipe set, `PendingPushes::outstanding`, `untracked_to_import` and `refused_orders` never read `plan.actions`, and `pull_count`/`push_count` ignore `Noop`.
- ✅ The watermark fix needs no special case. Without the duplicate `(Unsynced, Noop, NotApplied)` row, `definitively_reconciled` keeps a failed create's id out of `advance_ids`.
- ✅ `only_row` turns the whole class of duplicate-row regressions into one assertion with a clear message, and it is retrofitted into the order-refused and refused-adopt tests.
- ✅ The review-1 fix-ups all landed: `unreachable_tracker()`, the explicit `Some(0)` precondition, the preview and draft/pull-only unknown-recipe guards, the `[noop] = [...]` destructuring, and the "`failed` row, `order-refused`" wording.
- ✅ The CLI's push-only preview golden pins the exact stdout, so the script-facing contract is covered end to end.
- ✅ Doc comments give only the *why*, the test names reuse the module's domain words, and the new lines stay within 80 columns.

### General Findings

- 🔵 **Architecture** (code this PR did not change: `finalise_baseline`, `run.rs` ~334-363): an id's watermark advances if *any* of its rows is definitively reconciled. The failed-create bug was a passing `noop` row outvoting a `failed` row. This PR removes the second row, but not the coupling, so watermark safety now rests on one row per id, kept by convention upstream. Making the advance veto-based (an id advances only if *no* row is unreconciled) would make the finaliser safe however many rows an item has. This is a different point from review 1's `debug_assert`; a follow-up would be enough.

---
*Review generated by /review-pr*

## Inline Comments

### `skills/work/sync-work-items/SKILL.md:466` — The unknown-recipe row that drafts keep matches no rule, and the refused-order rule can swallow it
**Severity**: minor | **Confidence**: medium | **Lens**: compatibility, documentation, correctness

🔵 **Compatibility / Documentation / Correctness**

The tests added in 14fcda081 (`an_unknown_recipe_entry_on_a_draft_is_left_untouched`, `…_under_pull_only_…`) pin a `Noop` row with the `UnsupportedRecipe` outcome for an unsynced draft or `--pull-only` item. `classify` returns `Unsynced` for any item without an `external_id`, so stdout prints `<id>\tnoop\tunsynced\tunsupported-recipe`. The dispatch entry (line 355) and the `needs-upgrade` Membership row (line 458) only match `noop\tindeterminate\tunsupported-recipe`, so this row falls into no group. Worse, when the draft also has a refused order, this row's rule ("`noop\tunsynced` row … → none: the refusal speaks for the item") swallows it.

The mismatch on lines 355 and 458 is older than this PR. This PR, though, pins the behaviour that produces the row and rewrites the rule that swallows it.

**Impact**: `/sync-work-items` can leave an item that needs a plugin upgrade out of `needs-upgrade`. Because `UnsupportedRecipe` does not fail the run, it can report "sync complete" while stderr says the item was left untouched.

**Suggestion**: Key lines 355 and 458 on the detail column alone (a `noop` row with detail `unsupported-recipe`, in any state). Narrow this row to a `noop\tunsynced` row whose detail is `-`, so the refusal never speaks for a needs-upgrade item.

---

### `CHANGELOG.md:783-784` — The advice to key on the `unsynced` state column matches more than the old `noop` rows
**Severity**: minor | **Confidence**: high | **Lens**: correctness, compatibility, documentation

🔵 **Correctness / Compatibility / Documentation**

"Key on the `unsynced` state column instead" matches more than the `noop\tunsynced` rows it replaces:

- `create_report` gives every `create-from-remote` row `SyncState::Unsynced`, so a remote import prints `ENG-7\tcreate-from-remote\tunsynced\t-`.
- `relink` rows, and the draft `promote`/`not-promoted` lines, also carry `unsynced`.
- An unpromoted draft can appear on both a `not-promoted` line and its own `noop\tunsynced` row.

A script that follows the note would list tracker keys and promoted drafts as unsynced local items, and could count an id twice.

The scope "an item it creates or adopts in the tracker" also leaves out `--preview`, where nothing is created but the duplicate row is gone too (as the `cli_sync.rs` golden shows). "The `unsynced` state column" also reads as a column name rather than a value.

**Suggestion**: Something like: "An item it creates or adopts, or would under `--preview`, no longer prints…; scripts that listed unsynced items by their `noop` row should collect the distinct ids of `noop`, `create-from-local`, `adopt` and `failed` rows whose state is `unsynced`."

---

### `CHANGELOG.md:785-787` — The watermark sentence names the wrong edit window and overstates its scope
**Severity**: minor | **Confidence**: medium | **Lens**: correctness, documentation

🔵 **Correctness / Documentation**

This wording came from review 1's suggestion, so this corrects that advice with a new argument: the mtime pre-filter in `work::sync::classify::local_change` treats an item as possibly changed when `mtime > watermark`. Even with the watermark advanced to `run_start_epoch`, an edit made *during* the run (mtime after the run started) is still hashed and noticed. The edit the old advance could hide was one made *before* the run, since the item was last recorded.

The sentence also reads as if every failed create was affected. In fact only an unsynced item that still has a baseline entry has a per-item watermark to advance: the `seed_recorded_entry` case, typically an `external_id` removed by hand. An ordinary unsynced item is created again from its current file on the next sync either way.

**Impact**: The release note names the wrong edit window and overstates how often the bug mattered.

**Suggestion**: Something like: "A failed create of an item that still has a sync record no longer marks it as synced up to the run's start, so the next sync still notices a local edit made since it was last synced." Or simply: "A failed create now leaves the item's sync record untouched."

---

### `cli/work-adapters/src/sync/run.rs:2225-2227` — A failed create over an unknown-recipe entry loses its warning and dossier protection
**Severity**: minor | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

Review 1 noted this as a general finding; there are two new points here. Before this PR, for an unsynced item whose baseline entry used an unknown digest recipe, the leftover `Noop` row was relabelled `UnsupportedRecipe`. That gave the item two protections:

1. the stderr "left untouched … upgrade the plugin" warning, and
2. `ConflictStatus::of` → `Unobserved`, so `dossiers::settled_by` would not retire the item's dossier.

Now the create row is the only row, and create rows never go through `noting_unsupported_recipe`. That is right for a create that lands, because it replaces the entry. For a create that **fails**, the `Entry::Unsupported` stays on disk, so "left untouched" is literally true, yet no warning is printed. And because `ConflictShape::of_unresolved` maps `(Unsynced, CreateFromLocal)` to `NotInConflict` whatever the outcome, an apply run can now retire a dossier for that id that the old code kept.

**Impact**: The case is narrow (an unsynced item carrying a newer release's entry, plus a failed create), but the run then gives no upgrade hint and may drop a dossier. The CHANGELOG's "that sync creates or adopts" does not cover the failure case.

**Suggestion**: Either accept this and say so (in the CHANGELOG and the PR's "Dropped warning" note), or keep the signal for a failed create whose id is in `unsupported_recipe`, for example by treating such a row as `Unobserved`. Whichever you choose, see the test-coverage comment on `an_unknown_recipe_entry_leaves_only_the_create_row`.

---

### `cli/work-adapters/tests/sync_create.rs:3937-3960` — No test for a failed create over an unknown-recipe entry, or for the dossier consequence
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

The unknown-recipe tests cover a create that succeeds (apply and preview) and items the run leaves alone (draft, `--pull-only`). The one combination this PR changed without a test is a create that **fails** over an `Entry::Unsupported`: it lost the warning and the dossier protection (see the comment on `run.rs:2225`).

Separately, the PR notes say a created item no longer gets an `Unobserved` dossier status, but no test pins that run-level consequence. `DossierAuthority::may_retire("0001")` flips from false to true here.

**Suggestion**: Add a sibling test combining `seed_unknown_recipe_entry` with `unreachable_tracker()`. Assert that `only_row` is a `CreateFromLocal` row with outcome `Failed(_)`, that the unsupported entry survives (`stored_watermark` is `None`), and whichever warning and dossier behaviour you settle on. If the dossier retiring after a successful create is intended, add `assert!(DossierAuthority::for_run(&report).may_retire("0001"))` here.

---

### `cli/work-adapters/tests/sync_create.rs:3924-3932` — The failed-create watermark test does not prove a create was attempted and failed
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`a_failed_create_leaves_the_items_watermark_where_it_was` throws the report away. Nothing confirms that the run actually planned a create for `0001` and that it failed. The fixture is unusual: no `external_id`, but a recorded baseline entry. If the planner ever classified that shape differently (for example as `Indeterminate`), `definitively_reconciled` would also leave the watermark at `Some(0)`, and the test would pass without exercising the failed-create path it is named for.

**Suggestion**: Keep the report and pin the path:

```rust
let report = sync_item(…)?;
assert!(matches!(only_row(&report, "0001").outcome, ItemOutcome::Failed(_)));
```

---

### `cli/work-adapters/src/sync/run.rs:1350-1352` — The --pull-only part of the create rule depends on where an early return sits
**Severity**: suggestion | **Confidence**: medium | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

This is not the declined shared-helper suggestion; it is narrower. `creatable_item` is meant to state the whole rule, which is why the unreachable `Action::Noop` clause stays in. But the direction part of the rule (`--pull-only` creates nothing) is still an early return that must come *before* the `mem::take`. The sibling `relinks`, now placed right beside it, does the opposite: it partitions first, then returns. As a result, `creatable_item` answers `Some` for an item that a `--pull-only` run must not create.

**Impact**: Someone tidying the two siblings to match (moving this return below the partition, as `relinks` does) would silently drop the `noop` rows that `--pull-only` items keep. Only the integration guard would catch it; the unit-tested predicate would not.

**Suggestion**: Pass `direction` into `creatable_item` and return `None` under `PullOnly`. The loop then runs unconditionally, `--pull-only` entries fall through to `remaining`, and the existing unit test can pin the direction clause alongside the relink clause. This is optional if you would rather keep the early return.

---

### `cli/work-adapters/src/sync/run.rs:2562-2579` — The relink unit test has no positive control
**Severity**: suggestion | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`an_unsynced_relink_is_not_a_create_from_local` only asserts `None`. Many things give `None`: an id mismatch, a later extra guard on `LocalItem`, or the fixture drifting into a draft-shaped id. It fails today when the `Action::Noop` clause is removed, but nothing shows that the same fixture *is* creatable when the action is `Noop`, so a later change could leave it passing for the wrong reason.

**Suggestion**: Add a positive control over the same `items`, so the action is the only thing that differs:

```rust
let noop = PlannedAction::new("0001", SyncState::Unsynced, Action::Noop);
assert!(creatable_item(&noop, &items).is_some());
```

---

### `cli/work-adapters/tests/sync_create.rs:3512-3522` — `only_row` sits beside first-match row helpers it could replace
**Severity**: suggestion | **Confidence**: medium | **Lens**: code-quality, test-coverage

🔵 **Code Quality / Test Coverage**

`only_row` enforces one row per id. This file still has `local_create_row` (~4150, eight callers), `create_from_local_outcome` (~2621), `create_from_local_failure` and `relink_row`, which all take the first match with `find`. That lookup style is how the duplicate `noop` row went unnoticed, and it would still hide a duplicate on other create paths (targeted runs, marker-reuse creates, author faults). Those helpers are outside this diff.

**Suggestion**: Have `local_create_row` (and `create_from_local_outcome`) delegate to `only_row` and then assert the action, so every create-path test guards the one-row rule. Some new tests (e.g. `an_applied_create_is_the_items_only_row` beside `an_unsynced_item_issues_exactly_one_create_and_links_it`) could then fold into the older ones. A follow-up would be fine.

---

### `cli/work-adapters/tests/sync_create.rs:3889-3895` — `unreachable_tracker()` is not reused where the same tracker is built inline
**Severity**: suggestion | **Confidence**: medium | **Lens**: code-quality, standards

🔵 **Code Quality / Standards**

92fc5a87a named this tracker, but the same `RecordingTracker::holding(…).failing_create(TrackerError::Retryable { detail: "jira create: could not connect" })` is still built inline at ~2326 (in `an_unreachable_tracker_leaves_no_marker_for_a_local_create`, which is named after the same idea), ~2507 and ~5422. Those lines are outside this diff.

**Suggestion**: Use `unreachable_tracker()` at those sites. Consider moving it, along with `sync_item` (also defined between tests, after its first callers), next to the other tracker and run helpers. This is cosmetic.

---

## Per-Lens Results

Each lens ran twice: once in the background, and again in the foreground after the first hand-back. The two runs agreed; their results are merged below.

### Architecture

**Summary**: The fix is structurally sound and stays within the adapter's existing design. `unsynced_creates` now takes its entries out of the plan the same way `relinks` does, and the plan is reduced before the bounds, `refused_orders` and the preview/apply split read it. Two points are left: the `--pull-only` rule depends on where an early return sits, and watermark safety depends on one row per id, which the finaliser does not enforce.

**Strengths**: Each step that takes entries out of the plan now follows one pattern. `creatable_item` is a pure predicate that can be unit-tested. The `decide` golden and the dossier mapping are untouched. The CLI contract, skill table, structure test and CHANGELOG all change together.

**Comments**:
- `run.rs:1345-1378` (suggestion, low/medium): pass `direction` into `creatable_item`, since `relinks` handles `--pull-only` in the opposite order.

**General Findings**:
- (minor/suggestion, unchanged code) `finalise_baseline` advances an id if *any* of its rows is reconciled. Make the advance veto-based so it no longer depends on one row per id.

### Code Quality

**Summary**: The production change is small and readable. The fix-ups made the code clearer and reduced duplication. What remains is low-stakes tidying of the test helpers.

**Strengths**: `creatable_item` is a flat guard-clause predicate. `sync_item`, `only_row` and `seed_recorded_entry` cut repeated setup. The `[noop] = [...]` destructuring states its invariant.

**Comments**:
- `run.rs:1350-1377` (suggestion, low): the eligibility rule is split between the guard and the predicate.
- `sync_create.rs:3512-3522` (suggestion, medium): `only_row` sits beside the first-match helpers `local_create_row`, `create_from_local_outcome` and `relink_row`.
- `sync_create.rs:3889-3895` (suggestion, medium): `unreachable_tracker()` duplicates inline constructions at ~2326, ~2507 and ~5422. `sync_item` and `unreachable_tracker` are defined in the middle of the tests.

**General Findings**:
- (suggestion, unchanged code) The first-match row helpers would hide a duplicate row.

### Test Coverage

**Summary**: Coverage is strong and proportionate. Every reporting path asserts one row per id through `only_row`, and every eligibility clause has a test that fails if the clause is mutated. Small gaps remain.

**Strengths**: `only_row` gives a clear failure message. The preview and apply matrix is covered. The `Some(0)` precondition makes the watermark test meaningful. The preview unknown-recipe test pins the one mutation the apply-mode test cannot see. The CLI golden pins the whole stdout.

**Comments**:
- `sync_create.rs:3937-3960` (minor, medium): no test for a failed create over an `Entry::Unsupported`. The dossier-retirement consequence is not pinned either.
- `sync_create.rs:3924-3932` (minor, medium): the failed-create watermark test discards the report.
- `run.rs:2561-2579` (suggestion, medium): the relink unit test has no positive control.
- `sync_create.rs:3829-3845` (suggestion, medium): the new one-row tests overlap older tests whose `any`/`find` assertions are loose. Folded into inline comment 09.

**General Findings**: none.

### Correctness

**Summary**: The partition is logically sound. `creatable_item` selects exactly the old set, because unsynced items only ever get `Noop`, and the `Relink` entries were already removed. Hoisting the call is behaviour-neutral. The remaining problems are in the CHANGELOG and skill claims around the code, plus one narrow edge case: a failed create over an unknown-recipe entry.

**Strengths**: Every downstream reader was traced and keeps its behaviour: bounds, `refused_orders`, `untracked_to_import`, `finalise_baseline`, `identity_state`, `settled_by`, `failures()` and `awaiting_human()`. The document-level fallback watermark cannot hide an edit to an item with no entry, because `local_change` returns `Unknown` first.

**Comments**:
- `CHANGELOG.md:783-785` (minor, high): the migration advice also matches `create-from-remote`, `promote` and `not-promoted` rows.
- `CHANGELOG.md:786-787` (minor, medium): the watermark rationale names the wrong window, because the pre-filter compares `mtime > watermark`.
- `SKILL.md:466` (minor, medium): `noop\tunsynced\tunsupported-recipe` matches no `needs-upgrade` rule, and the refused-order rule can swallow it.
- `run.rs:2225-2227` (minor, medium): a failed create over an unknown-recipe entry loses its warning and dossier protection.

**General Findings**: none.

### Compatibility

**Summary**: Apart from the one intended removal, the stdout contract is unchanged. Exit codes, the column layout, the baseline file shape and the marker are unaffected. The launcher pins the binary to the plugin version, so the skill and the binary cannot drift apart. The CHANGELOG's migration advice is imprecise, and one older mismatch between the binary and the skill is now pinned.

**Strengths**: The row format is unchanged. `awaiting_human` never counted the removed rows. The only in-repo consumer is updated in lockstep. Drafts and `--pull-only` runs keep their rows.

**Comments**:
- `CHANGELOG.md:784-786` (minor, medium): "key on the `unsynced` state column" does not give one row per item (it also matches relink, promote and not-promoted rows).
- `SKILL.md:466` (minor, medium): the unknown-recipe row that drafts and `--pull-only` items keep has a shape the skill does not recognise.

**General Findings**: none.

### Documentation

**Summary**: The documentation keeps pace with the behaviour change, and the doc comments explain why rather than what. The CHANGELOG has small accuracy issues, and the skill's `needs-upgrade` entries miss the unsynced unknown-recipe row.

**Strengths**: The Membership row changed in the same PR, pinned by a test. No other skill or docs page refers to the removed row. The doc comment on `finalise_baseline` is true again for failed creates.

**Comments**:
- `CHANGELOG.md:785-787` (minor/suggestion, medium): the watermark sentence names a consequence an unsynced item does not show, and reads as if it applied to every failed create.
- `CHANGELOG.md:781-785` (suggestion, medium): the migration note leaves out preview output and words the state column ambiguously.
- `SKILL.md:466` (minor, medium): `noop\tunsynced\tunsupported-recipe` matches no `needs-upgrade` entry.

**General Findings**: none.

### Standards

**Summary**: The PR follows project conventions, and the fix-ups resolved the standards drift review 1 raised. The only issue left is consistency: `unreachable_tracker()` sits beside three inline copies of the same tracker.

**Strengths**: `unsynced_creates` and `relinks` have the same signature and doc-comment phrasing. Test names keep the module's style. Function-local `use` lines follow precedent. Lines stay within 80 columns. The CHANGELOG follows the house format, and the internal term "watermark" is gone from it.

**Comments**:
- `sync_create.rs:3889-3895` (suggestion, low): the new `unreachable_tracker()` fixture coexists with three inline copies.

**General Findings**: none. The Fixed-versus-Breaking placement was deliberately not raised again.

---
*Review generated by /accelerator:review-pr*
