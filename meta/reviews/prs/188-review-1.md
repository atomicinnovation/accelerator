---
type: "pr-review"
id: "188-review-1"
title: "[0352] Report each unsynced item once, by its create"
date: "2026-11-11T09:17:20+00:00"
author: "Phil Helm"
producer: "review-pr"
status: "complete"
target: "pr:188"
reviewer: "Phil Helm"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "documentation", "usability", "standards"]
review_number: 1
pr_number: 188
tags: []
last_updated: "2026-11-11T09:17:20+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

## Code Review: #188 - [0352] Report each unsynced item once, by its create

**Verdict:** COMMENT

A small, well-aimed fix. `unsynced_creates` now moves entries out of `plan.actions` instead of cloning them, the same ownership pattern `relinks` already uses. Each unsynced item therefore has exactly one owner and reports once, and the failed-create watermark bug disappears with no change to `finalise_baseline`. The correctness lens traced every reader of `plan.actions` and found that moving the call earlier in `prepare_run` changes nothing else: the unsupported-recipe set, pending-push handling, `untracked_to_import`, `refused_orders`, the push/pull bounds and the dossier mapping all behave as before. No lens found a blocking issue. The findings ask to make two behaviours explicit rather than incidental (the dropped unknown-recipe warning and the unreachable `Noop` guard), fix a terminology slip in the skill table, and adjust how the CHANGELOG records the change.

### Cross-Cutting Themes

- **The dropped unknown-recipe warning follows from loop structure, not a stated rule** (flagged by: architecture, code-quality, usability, compatibility, correctness). `unsupported_recipe` is still built from every reconciled item, and created items lose the "left untouched" warning only because create rows bypass `reported_as`/`noting_unsupported_recipe`. If all rows are later routed through that wrapper, the false warning would come back for previewed creates, and only the apply-mode test would catch it. A create also now replaces a baseline entry written by a newer release with no signal at all. A *failed* create leaves that entry in place with no warning, which is narrower than the CHANGELOG's "its create runs regardless". Building `unsupported_recipe` from the ids still in `plan.actions` would state the rule. A short `note:` when a create replaces an unsupported entry would keep a downgrade visible.
- **The new Membership row names an `order-refused` *row*** (flagged by: standards, documentation). The CLI prints `<id>\tfailed\t-\torder-refused`, and the rest of the table (line 457) says "`failed` row, … `order-refused`". The skill model applies this table literally, so the row should use the table's own vocabulary.
- **The plan-extraction steps now sit side by side but differ in shape and pull-only semantics** (flagged by: architecture, code-quality, documentation). `relinks` partitions and then checks `PullOnly`; `unsynced_creates` checks first and partitions with a hand-written loop. Some differences are deliberate (pull-only keeps `noop` rows), but nothing in the code says which ones. Longer term, the planner already receives direction, orders and `DraftId`, so it could emit `CreateFromLocal` itself.
- **Script-facing output change filed under Fixed** (flagged by: standards, compatibility). Every other `[Unreleased]` entry that tells scripts to change sits under Breaking or Changed. The duplicate row only appeared in 1.24.0 pre-releases, so either say so or add a short Breaking line.

### Tradeoff Analysis

- **Test coverage vs comment tolerance**: the test-coverage lens wants a unit test in `run.rs`'s in-file module to pin the unreachable `Action::Noop` clause. The documentation lens suggests extending the `unsynced_creates` doc comment to explain its pull-only ordering, but the project has a very low tolerance for comments. A unit test, together with the existing pull-only guard, states both invariants without adding prose, so prefer the test.

### Strengths

- ✅ The duplication is removed at its source. `std::mem::take` moves each `PlannedAction` into either the residual plan or a `LocalCreate`, so no deduplication pass is needed afterwards.
- ✅ `creatable_item` puts the eligibility rule in one named predicate (unsynced, `Noop`, not a draft, with a matching item), so the rule no longer depends on running after `relinks`. The PR is upfront that the `Noop` clause cannot be reached through `run_sync`.
- ✅ The watermark fix needs no special case. Once the duplicate `(Unsynced, Noop, NotApplied)` row is gone, `definitively_reconciled` already gives the right answer, and `finalise_baseline`'s doc comment becomes true for failed creates.
- ✅ The `--pull-only` early return comes before the partition, and regression guards pin both it and the draft exclusion.
- ✅ `only_row` turns the whole class of duplicate-row bugs into one assertion with a diagnostic message (`0001 reported as [Noop, CreateFromLocal]`), and the existing order-refused tests now use it too.
- ✅ The tests assert user-visible behaviour (rows, outcomes, stored watermark), and the CLI test pins the exact rendered stdout, so the script contract is covered end to end.
- ✅ The CHANGELOG's migration guidance is accurate: every create, adopt and failed-create row still carries `unsynced` in the state column.
- ✅ The skill ships in lockstep with the binary (both pinned to `plugin.json`), and a structure test guards the narrowed Membership row.
- ✅ Doc comments explain only *why*, with no work-item or plan references, and the new test names reuse the module's existing domain words.

### General Findings

- 🔵 **Architecture**: The one-row-per-item invariant is enforced only by tests. `execute` builds `reported` from three independent sources, so a future extractor that clones instead of moving would silently bring the duplicates back. Consider a `debug_assert!` on unique ids in `reported`, or a small report builder that rejects a second row for the same id.
- 🔵 **Correctness**: A failed create over an unknown-recipe entry now leaves the entry on disk with no `unsupported-recipe` row or warning. The impact is low: the `failed` row already blocks completion, and the next successful create replaces the entry. Still, the CHANGELOG's rationale "its create runs regardless" holds only when the create lands.
- 🔵 **Compatibility**: On a downgrade, an older binary's create now replaces a newer release's baseline entry without saying so. The overwrite predates this PR; only the inaccurate warning has gone. A `note:` on the create path when it replaces an `Entry::Unsupported` would keep forward compatibility visible.

### Additional Findings

- 🔵 `skills/work/sync-work-items/SKILL.md:466`: A lone `noop\tunsynced` row (drafts, `--pull-only`) still has no Membership entry. The gap predates this PR, but this is now the only form the row takes (usability)
- 🔵 `cli/work-adapters/tests/sync_create.rs:3532`: `seed_recorded_entry` hard-codes the "Draft one" remote body and timestamp (code-quality)
- 🔵 `cli/work-adapters/src/sync/run.rs:1343`: The `unsynced_creates` doc does not say that it must return before partitioning under `--pull-only`, unlike `relinks` (documentation)
- 🔵 `CHANGELOG.md:781`: The script-facing output change is filed only under Fixed (standards, compatibility)
- 🔵 `CHANGELOG.md:785`: A create now silently replaces a baseline entry written by a newer release (usability)
- 🔵 `skills/work/sync-work-items/SKILL.md:466`: Consider making the refused-order case report once at the engine level, as `relinks` does, so this Membership row can go (architecture)


## Inline Comments

### `cli/work-adapters/src/sync/run.rs:2225-2230` — Unknown-recipe warning suppression for creates is implicit in loop structure
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

`unsupported_recipe` is still built from every reconciled item, including the ids just moved into `creates_from_local`. Created items now lose the "left untouched; upgrade the plugin" warning and the `Unobserved` dossier status only because their rows never pass through `reported_as` / `noting_unsupported_recipe`, which is mapped over plan rows only. Create rows are pushed directly.

**Impact**: The PR intends this change, but nothing in the code states the rule. If someone later routes all rows through `reported_as` for consistency, or merges the report loops, previewed creates (which have the `NotApplied` outcome that `noting_unsupported_recipe` matches on) would silently regain the false warning. `an_unknown_recipe_entry_leaves_only_the_create_row` covers only `RunMode::Apply`, so it would not catch that.

**Suggestion**: Build `unsupported_recipe` from the ids that remain in `plan.actions` after the partition, so the set means "items this run leaves untouched". Alternatively, add a preview-mode case to the unknown-recipe test.

---

### `skills/work/sync-work-items/SKILL.md:466` — Membership row calls the refusal an `order-refused` row
**Severity**: minor | **Confidence**: medium | **Lens**: standards, documentation

🔵 **Standards / Documentation**

The new record reads "`noop\tunsynced` row for an id that also has an `order-refused` row". Everywhere else in the skill, the refusal is a `failed` row whose column 4 is `order-refused`: line 457 of this table says "`failed` row, … `order-refused`", and the dispatch entry and exit-code table use the same wording. The CLI prints `<id>\tfailed\t-\torder-refused`, so `order-refused` is a detail, not a row type.

**Impact**: This table is the single source of grouping rules for an LLM-driven skill, and the model applies it literally. A record naming an action that never appears in column 2 is harder to match against the output, and a missed match would summarise the draft twice.

**Suggestion**: Use the table's own vocabulary, e.g. "`noop\tunsynced` row for an id that also has a `failed` row, `order-refused`". The structure test's `` "`order-refused`" in noop[0] `` assertion still holds.

---

### `cli/work-adapters/src/sync/run.rs:1371-1376` — The `Action::Noop` clause in `creatable_item` has no test demanding it
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

The PR notes that this clause cannot be reached by a test because `relinks` has already taken the `Relink` entries out. That is only true through `run_sync`. `run.rs` already has an in-file `#[cfg(test)] mod tests`, and `PlannedAction::new(id, SyncState::Unsynced, action)` can be constructed there, so the predicate can be exercised directly. As it stands, deleting the clause breaks no test.

**Impact**: The clause exists to guard against call-order dependence. If `prepare_run` were reordered so `unsynced_creates` runs before `relinks`, an unsynced `Relink` entry would be taken as a plain create, and nothing would catch it.

**Suggestion**: Add a unit test such as `an_unsynced_relink_is_not_a_create_from_local` that calls `creatable_item` with an `(Unsynced, Relink)` planned action and a matching `LocalItem`, and asserts `None`.

---

### `cli/work-adapters/tests/sync_create.rs:3938-3960` — No guard that drafts and --pull-only items keep the unknown-recipe warning
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

This test covers the half of the change where the warning is dropped. The PR description says the other half also holds: unsynced drafts and `--pull-only` items keep the warning. No test covers that half. The only other `UnsupportedRecipe` test (in `sync_recipe_migration.rs`) uses a linked item, not an unsynced one.

**Impact**: A future change that broadens the partition could silently drop the "upgrade the plugin" warning for those items, contrary to the documented behaviour.

**Suggestion**: Reuse `seed_unknown_recipe_entry` and `sync_item` in sibling tests for a `draft-…` id and for `SyncDirection::PullOnly`. Assert that `only_row(...)` is a `Noop` row with the `ItemOutcome::UnsupportedRecipe` outcome.

---

### `cli/work-adapters/tests/sync_create.rs:3889-3895` — `refusing_creates` uses the word 'refused' for an unreachable tracker
**Severity**: minor | **Confidence**: medium | **Lens**: standards

🔵 **Standards**

This helper builds a tracker whose create fails with `TrackerError::Retryable { detail: "jira create: could not connect" }`. Elsewhere in this file that fault is an *unreachable* tracker (`an_unreachable_tracker_leaves_no_marker_for_a_local_create`). In this module, "refused" has a specific meaning (`order-refused`, an adopt refused at apply time, `local-write-refused`, `assert_order_refused`), and a tracker rejecting a request is "rejected".

**Impact**: Someone reading `a_failed_create_is_the_items_only_row` may take this for a terminal, `rejected`-class failure when it is a retryable connection failure. That blurs a domain word the module otherwise uses precisely.

**Suggestion**: Use the file's existing wording, e.g. `unreachable_tracker()`.

---

### `CHANGELOG.md:785-786` — CHANGELOG uses the internal term 'baseline watermark' without saying what users see
**Severity**: minor | **Confidence**: medium | **Lens**: documentation

🔵 **Documentation**

"A failed create no longer advances the item's baseline watermark" is the only use of "watermark" in CHANGELOG.md. It is an internal sync-engine term (`local_synced_at`). The surrounding Fixed entries describe what the user sees.

**Impact**: Readers cannot tell what was wrong before or why the fix matters to them. The real effect is that a local edit made around a failed create could have been missed by the next sync's local-change check.

**Suggestion**: State the consequence, e.g. "A failed create no longer marks the item as synced up to the run's start, so the next sync still notices a local edit made during the failed run."

---

### `cli/work-adapters/src/sync/run.rs:1345-1378` — Create decisions live in the adapter as a rewrite of the pure planner's output
**Severity**: minor | **Confidence**: medium | **Lens**: architecture

🔵 **Architecture**

`unsynced_creates`, like `relinks` before it, rewrites the pure planner's `SyncPlan` after the fact. After it runs, `PreparedRun.plan` is no longer the complete plan that `work::sync::plan` documents ("one `PlannedAction` per classifiable item"); it is what is left once the creates are taken out. The research gives "only the adapter knows direction, drafts and orders" as the reason for keeping this in the adapter. But `work::sync::plan` already takes `direction` and `PlanOrders.pending_push`, and `DraftId` lives in the `work` crate.

**Impact**: The decision "this unsynced item will be created" is split. The core says `Noop` and the shell overrides it by mutation. That split is why the order-dependency guard in `creatable_item` exists, and why the core's decide golden cannot show the real outcome for an unsynced item.

**Suggestion**: No change is needed for this fix. Consider a follow-up in which the planner emits `CreateFromLocal` for unsynced, non-draft items under a push-capable direction. Until then, consider naming the residual (e.g. `remaining_plan`) so readers of `PreparedRun` know it is not the planner's full output.

---

### `cli/work-adapters/tests/sync_create.rs:3921-3923` — The failed-create watermark test relies on an implicit seed value
**Severity**: suggestion | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

The test asserts only `before.is_some()` and then compares the value after the run with it. It can detect the bug only because `support::split_entry` hard-codes `local_synced_at: 0`, which differs from the fixed run clock of `1_700_000_000`. The test does not show that dependency.

**Impact**: If the shared seed helper ever stamped entries with the run-start epoch, this test would pass vacuously.

**Suggestion**: State the precondition, e.g. `assert_eq!(before, Some(0))` or `assert_ne!(before, Some(1_700_000_000))`.

---

### `tests/unit/tasks/test_sync_work_items_skill_structure.py:454-460` — The skill-structure test inspects only the first `noop\tunsynced` row
**Severity**: suggestion | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`next(...)` returns the first Record row starting with `` `noop\tunsynced` ``, so `` "`create-from-local`" not in noop[0] `` still passes if the retired create-duplicate rule reappears later in the table. That is exactly the regression this test should guard against.

**Suggestion**: Collect every matching row, assert there is exactly one, and then run the existing assertions on it.

---

### `cli/work-adapters/src/sync/run.rs:1353-1362` — The two side-by-side plan-extraction steps are written differently
**Severity**: suggestion | **Confidence**: medium | **Lens**: code-quality

🔵 **Code Quality**

`unsynced_creates` and `relinks` now sit next to each other, and both take entries out of `plan.actions` and pair each with its `LocalItem`, but they are written differently. `relinks` partitions first, then checks `PullOnly`, and drops a relink that has no matching item. `unsynced_creates` checks `PullOnly` first, partitions with a hand-written loop, and keeps an entry that has no matching item. Both repeat the same `items.iter().find(...)` lookup.

**Impact**: A reader has to work out which differences are deliberate (pull-only keeps `noop` rows; relinks are refused under pull-only) and which are accidents of style.

**Suggestion**: Consider a small shared helper, e.g. `take_from_plan(plan, select)`, so that each caller supplies only its selection rule and pull-only policy. The deliberate differences would then be the only visible ones.

---

## Per-Lens Results

### Architecture

**Summary**: The fix is structurally sound and narrow. `unsynced_creates` now moves entries out of the plan rather than cloning them, matching its sibling `relinks`, and the two plan-extraction steps sit together in `prepare_run`. The watermark bug goes away without special-casing `finalise_baseline`. The main tension is that the adapter keeps making create decisions by rewriting the pure planner's output, although `work::sync::plan` already receives direction, orders and drafts. Some behaviours, such as suppressing the unknown-recipe warning, follow from which report loop a row passes through rather than from a stated rule.

**Strengths**:
- Each planned entry now has exactly one owner (the residual plan or a `LocalCreate`), which removes the duplicate row at its root.
- Nothing between the old and new call sites reads `plan.actions`, and `push_count` ignores `Noop`, so the move does not change the bounds check.
- The `--pull-only` early return precedes the partition, and guards pin it.
- `creatable_item` states eligibility as a predicate, independent of call order.
- The watermark fix needs no change to `finalise_baseline` or `definitively_reconciled`.

**Comments**:
- `cli/work-adapters/src/sync/run.rs:1345-1378` (RIGHT), minor/medium: Create decisions live in the adapter as a rewrite of the pure planner's output (posted inline).
- `cli/work-adapters/src/sync/run.rs:2225-2230` (RIGHT), minor/medium: Unknown-recipe warning suppression for creates is implicit in loop structure (posted inline, merged with code-quality).
- `skills/work/sync-work-items/SKILL.md:466` (RIGHT), suggestion/low: The skill still compensates for a two-line pairing that sibling relinks already avoid. A refused order on a draft or under `--pull-only` could report once at the engine level, as `relinks` does, so this Membership row could go (deferred to summary).

**General Findings**:
- suggestion: The one-row-per-item invariant is enforced only by tests. Consider a `debug_assert!` on unique ids in `reported`, or a report builder that rejects duplicates.

### Code Quality

**Summary**: The production change is small and well shaped. Moving entries instead of cloning them removes the duplicate-row smell at its source, and the eligibility rule now lives in one named predicate, which removes a hidden ordering dependency on `relinks`. The remaining concerns are minor: the two side-by-side extraction steps are written differently, the dropped unknown-recipe warning depends on which rows pass through `reported_as`, and one extracted test helper hard-codes the fixture's title.

**Strengths**:
- Uses `std::mem::take` with no deduplication pass afterwards.
- `creatable_item` has a single guard clause and returns `Option<&LocalItem>`.
- The two plan-mutating steps now sit together.
- `only_row` gives a diagnostic failure message.
- `seed_recorded_entry` removes duplicated test setup.
- The renamed Python test names the rule it now checks.

**Comments**:
- `cli/work-adapters/src/sync/run.rs:1353-1362` (RIGHT), suggestion/medium: The two side-by-side plan-extraction steps are written differently (posted inline).
- `cli/work-adapters/src/sync/run.rs:2230` (RIGHT), minor/medium: The dropped unknown-recipe warning depends on which rows pass through `reported_as` (merged into the architecture inline comment).
- `cli/work-adapters/tests/sync_create.rs:3532-3542` (RIGHT), suggestion/medium: The extracted `seed_recorded_entry` hard-codes the "Draft one" remote body and timestamp. Pass the remote body in, or name the helper after the fixture it assumes (deferred to summary).

**General Findings**: none.

### Test Coverage

**Summary**: Coverage is strong. Every reporting path (create, adopt, ordered create, failed create, refused adopt, preview and apply) has a behaviour test asserting exactly one row per id. The watermark consequence is pinned on both the success and failure sides, and the draft and `--pull-only` guards catch the obvious partition mutations. The gaps: no guard that drafts and `--pull-only` items keep the unknown-recipe warning, no test demanding the `Action::Noop` clause, and a structure test that inspects only the first matching row.

**Strengths**:
- `only_row` makes any duplicate-row regression fail one clear assertion.
- The tests assert user-visible behaviour, not plan internals.
- The failed-create watermark test directly reproduces the second defect.
- Regression guards pin what must not change.
- The CLI test pins the exact rendered stdout.
- The PR reports red-first TDD for each new test.

**Comments**:
- `cli/work-adapters/src/sync/run.rs:1371-1376` (RIGHT), minor/medium: The `Action::Noop` clause in `creatable_item` has no test demanding it (posted inline).
- `cli/work-adapters/tests/sync_create.rs:3938-3960` (RIGHT), minor/medium: No guard that drafts and `--pull-only` items keep the unknown-recipe warning (posted inline).
- `cli/work-adapters/tests/sync_create.rs:3921-3923` (RIGHT), suggestion/medium: The failed-create watermark test relies on an implicit seed value (posted inline).
- `tests/unit/tasks/test_sync_work_items_skill_structure.py:454-460` (RIGHT), suggestion/medium: The skill-structure test inspects only the first `noop\tunsynced` row (posted inline).

**General Findings**: none.

### Correctness

**Summary**: I traced every downstream reader of `plan.actions` and found no logic errors. The partition removes exactly the entries that `unsynced_creates` turns into creates. Moving the call earlier changes nothing, because none of the steps it now precedes (the unsupported-recipe set, `PendingPushes::outstanding`, `untracked_to_import`, `refused_orders`) reads `plan.actions`. Watermarking, the dossier mapping and the push/pull bounds behave as intended.

**Strengths**:
- `creatable_item` selects the same items as the old filter: `classify` gives `Unsynced` only without an `external_id`, and `decide` maps `Unsynced` to `Noop` in every direction.
- Relinks are still appended after plain creates, so apply order and the bound arithmetic are unchanged.
- `definitively_reconciled` already excludes `Failed`, and `apply_local_create` returns only `Applied` or `Failed`.
- `(Unsynced, Noop)` and `(Unsynced, CreateFromLocal)` both map to `NotInConflict`, so the dossier behaviour holds.
- Drafts stay in the plan because of the `DraftId` guard, matching the updated Membership rule.

**Comments**: none.

**General Findings**:
- suggestion: A failed create over an unknown-recipe entry now leaves the entry with no sign of it. The impact is low, but the rationale "its create runs regardless" holds only when the create lands.

### Compatibility

**Summary**: This PR removes the duplicate `noop\tunsynced` row from `work sync` stdout and stops a failed create from advancing the item's watermark. The only parser of this output is the sync-work-items skill, which is updated in this PR and ships in lockstep with the binary. Exit codes, the summary count, the dossier mapping and the baseline file format are unchanged. The duplicate row arrived during this Unreleased cycle, so stable 1.23.0 users never saw it.

**Strengths**:
- The contract change is pinned end to end (`cli_sync.rs`, `only_row`).
- The skill and the binary are version-locked through `plugin.json`.
- Drafts and `--pull-only` runs keep their single `noop` row.
- The CHANGELOG's migration guidance is accurate: `render_report` always prints `planned.state()`.
- `exit_code_for_report` never looked at these rows, so exit codes cannot change.

**Comments**:
- `CHANGELOG.md:781-785` (RIGHT), suggestion/low: A stdout contract change sits under Fixed, although the project files `work sync` row changes under Breaking (deferred to summary, merged with standards).

**General Findings**:
- suggestion: The downgrade signal for a newer-release baseline entry is lost when sync creates the item. Consider a `note:` when a create replaces an `Entry::Unsupported`.

### Documentation

**Summary**: The documentation is in good shape. The CHANGELOG describes the visible change and gives script authors migration guidance. The Membership row is narrowed to the one pairing that remains, and a test pins it. No other live docs describe the old duplicate row. The remaining points are small: internal jargon in the CHANGELOG, a terminology drift in the skill table, and a doc comment that omits the `--pull-only` invariant.

**Strengths**:
- The CHANGELOG's migration guidance is accurate.
- The Membership row is narrowed rather than deleted, and a test guards it.
- `finalise_baseline`'s existing doc comment is now true for failed creates.
- The stale doc comment was replaced, and its rationale moved next to its clause.
- No remaining references to the duplicate row in `docs-site/`, the README or other skills.

**Comments**:
- `CHANGELOG.md:785-786` (RIGHT), minor/medium: The CHANGELOG uses the internal term "baseline watermark" without saying what users see (posted inline).
- `skills/work/sync-work-items/SKILL.md:466` (RIGHT), suggestion/medium: The Membership row calls the refusal "an `order-refused` row" (merged into the standards inline comment).
- `cli/work-adapters/src/sync/run.rs:1343-1344` (RIGHT), suggestion/low: The `unsynced_creates` doc omits the `--pull-only` invariant that differs from `relinks` (deferred to summary).

**General Findings**: none.

### Usability

**Summary**: A clear developer-experience improvement: each item sync creates or adopts gets one row, so a failed create no longer sits beside a reassuring `noop` row. Two small gaps remain. The Membership table has no entry for a lone `noop\tunsynced` row (now its only form). Removing the false warning also removed the only signal that a create was overwriting a newer release's baseline entry.

**Strengths**:
- A failed create can no longer be hidden beside a harmless-looking `noop` row.
- The push-only preview prints one line per item.
- The failed row still carries `unsynced` in column 3.
- A false stderr message is gone.
- `only_row` failures are easy to diagnose.
- The skill's narrowed rule says why the paired row joins no group.

**Comments**:
- `skills/work/sync-work-items/SKILL.md:466` (RIGHT), minor/medium → suggestion after verification: A lone `noop\tunsynced` row still has no Membership entry or dispatch rule. The gap predates this PR, so it was downgraded and deferred to the summary.
- `CHANGELOG.md:785-788` (RIGHT), suggestion/low: A create now silently replaces a baseline entry written by a newer release (deferred to summary).

**General Findings**: none.

### Standards

**Summary**: The PR follows project conventions closely: `unsynced_creates` mirrors `relinks`, the doc comments explain only *why*, and the test names reuse existing domain words. There are three minor drifts. `refusing_creates` uses "refused" for an unreachable tracker. The new Membership record calls the refusal an "`order-refused` row". The CHANGELOG entry asks scripts to change but sits under Fixed.

**Strengths**:
- `unsynced_creates` has the same shape as `relinks`, and the two calls sit side by side.
- The doc comments explain only why and avoid work-item references.
- New test names and helpers reuse existing module vocabulary.
- The recorded-entry fixture is built in one place.
- The renamed Python test keeps its naming pattern, and the SKILL.md row ordering holds.
- The CHANGELOG entry uses the bold-lead style and gives migration guidance.

**Comments**:
- `cli/work-adapters/tests/sync_create.rs:3889-3895` (RIGHT), minor/medium: `refusing_creates` uses the word "refused" for an unreachable tracker (posted inline).
- `skills/work/sync-work-items/SKILL.md:466` (RIGHT), minor/medium: The new Membership record calls the line an "`order-refused` row" (posted inline, merged with documentation).
- `CHANGELOG.md:781-788` (RIGHT), suggestion/low: A script-facing `work sync` output change is filed only under Fixed (deferred to summary, merged with compatibility).

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
