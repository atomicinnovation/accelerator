---
type: "pr-description"
id: "234"
title: "[0351] Show pending-push refusals in a sync preview"
date: "2026-11-17T16:14:05+00:00"
author: "Phil Helm"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-926"
parent: "work-item:PP-926"
relates_to: ["plan:2026-11-17-PP-926-sync-preview-hides-pending-push-refusals"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/234"
pr_number: 234
tags: ["sync", "preview", "pending-push"]
revision: "ba4fe8fa6aa55aeb9800e5035a13a30886eccd3f"
repository: "accelerator"
last_updated: "2026-11-18T08:16:04+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# [0351] Show pending-push refusals in a sync preview

## Problem

A preview is meant to show, as `would fail`, every failure the run can
determine from local state. For a create-from-local, it didn't. It never
read the item's pending-push marker, so a create the run would refuse
previewed as clean:

- `--preview` printed `create-from-local` and exited 0;
- the run then printed a `terminal` row,
  `create failed: pending push refused (<reason>)`, and exited 71.

The same blind spot hid three other failures the run would meet:
- a reuse deferred because the corpus is incomplete;
- a work file that can't be read or parsed;
- an id that can't name a marker.

To work around it, `/sync-work-items` had the agent read each previewed
create's marker by hand. This was found during 0300's manual checks
against the `TES` sandbox.

## Summary

The preview now makes the same decision the run makes, for each create
that has no `--resolve` order:

```diff
 execute(mode = Preview)
   validate_pushes(plan)
   for create in creates_from_local
-    report NotApplied
+    if create carries a --resolve order
+      report NotApplied                      # checked only on the run
+    else
+      gather inputs                          # read + parse the work file
+      MarkerReading::read                    # distrusts tracked / non-tracker keys
+      bare_create_decision                   # shared with the run
+        refusal | deferred reuse | unusable  → report Failed(error)
+        Proceed | ReuseId                    → report NotApplied
```

`MarkerReading::read` and `bare_create_decision` were previously inside
the applier, and the run now calls the same shared functions. A refusal
therefore previews exactly as the run reports it, with `failed` read as
`would fail`.

## Evidence

The scenario is an unsynced item `0001` with an `Attempted` marker,
synced with `--target 0001`.

- **Before** (`--preview`):
  ```text
  stdout  0001  create-from-local  unsynced  -
          #     discovery          skipped   targeted
  stderr  warning: <root>/.accelerator/state/integrations/jira/pending-push/0001.json names a pending push for 'Draft one' attempted at 1
  exit    0
  ```
  **After** (`--preview`):
  ```text
  stdout  0001  failed     unsynced  terminal
          #     discovery  skipped   targeted
  stderr  error: 0001: create would fail: pending push refused (prior-attempt-unknown): an earlier create attempted at 1 has an unknown outcome (marker <root>/…/pending-push/0001.json); search the tracker for this item, then settle with --resolve 0001=adopt:<KEY> if it exists or --resolve 0001=create if it does not
          warning: <root>/…/pending-push/0001.json names a pending push for 'Draft one' attempted at 1
  exit    71
  ```
  The run itself prints the same output, with `create failed` in place of
  `create would fail`.

- **Before:** 11 new tests fail and 4 guards pass.
  **After:** all 15 pass.

  ```text
  preview and run on copies of one fixture, for each seeding:
    Attempted | unreadable | digest-mismatched | key carried elsewhere
    | reuse beside a malformed item | unparseable frontmatter
  assert as_applied(preview) == as_applied(run)   # stdout, stderr, exit
  assert pending-push/ unchanged by the preview
  ```

  Failing run before the preview branch changed:

  ```
  FAIL sync::tests::a_preview_reports_a_pending_push_refusal_as_the_run_would
  FAIL sync::tests::a_preview_distrusts_a_committed_marker
  FAIL sync::tests::a_whole_corpus_preview_ranks_a_refusal_above_the_discovery_deferral
  FAIL sync::tests::a_preview_and_the_run_report_the_same_create_failure
  FAIL sync::tests::a_previewed_refusal_outranks_a_conflict
  PASS sync::tests::a_preview_without_a_refusal_is_a_clean_create
  FAIL sync_create a_preview_reports_each_pending_push_refusal_the_run_would
  FAIL sync_create a_preview_distrusts_a_marker_as_the_run_does
  FAIL sync_create a_preview_defers_a_reuse_while_the_corpus_is_incomplete
  FAIL sync_create a_preview_reports_an_unusable_create_the_run_would
  FAIL sync_create a_preview_reports_an_id_that_cannot_name_a_marker
  FAIL sync_settled a_create_whose_marker_key_is_followed_this_run_is_refused
  PASS sync_create a_previewed_reusable_marker_reports_no_failure
  PASS sync_create a_preview_leaves_an_ordered_create_to_its_order
  PASS sync_create a_preview_leaves_a_relink_to_its_order
  ```

## Merge Danger

**Door:** two-way

The change is code only. The preview only reads markers and writes
nothing, and no stored format changes. Reverting restores the old
preview.

**Blast Radius:** narrow

The change only affects `work sync --preview`, and only when a create has
no `--resolve` order and has a marker or an unusable work file. Callers
that require a preview to exit 0 will now see:
- 71 for a refused pending push;
- 75 for an unparseable work file, or a reuse blocked by one;
- 70 for an unreadable work file, or a reuse blocked by an environment
  fault.

A whole-corpus preview over an `Attempted` marker used to exit 70, a
retryable result. It now exits 71, a signal not to retry. `CHANGELOG.md`
records this under both Fixed and Breaking.

## Changes

- **`work-adapters/src/sync/apply.rs`:**
  - `MarkerReading::read` and `bare_create_decision` are lifted out of the
    applier.
  - `settle_order` and `create_anew` now work out the request digest
    themselves.
- **`work-adapters/src/sync/run.rs`:**
  - `LocalCreateInputs` (with `gather` and `create_request`) is shared by
    the run and the preview.
  - `previewed_outcome` is new.
  - The key-carrier closure moves above the preview branch, so a key this
    run's identity settlement follows is refused in both modes.
- **Skill, help, guide:**
  - `/sync-work-items` drops the manual marker check, and its `--preview`
    statements now name the create's failure.
  - The `work sync` help, its golden file and the user guide follow.
- **Planning documents**: the 0351 work item and its review, and the plan
  and its two reviews.

## Testing

- [x] `work-adapters` `sync_create` and `sync_settled`: 196 passed.
- [x] `accelerator-work` binary tests: 474 passed.
- [x] Skill content test: 74 passed. The new test failed before the skill
      edit.
- [x] `cli_surface` golden test passes.
- [x] `mise run test` exits 0.
- [x] `mise run check` exits 0, including `deny:check` now that #233 is
      merged.
- [x] `mise run docs:check` exits 0.

## Notes for Reviewers

- **Departures from the plan:**
  - `settle_order` derives the digest itself, because clippy rejected the
    owned `String` parameter (`needless_pass_by_value`).
  - The CLI comparison uses invalid frontmatter YAML, not an unclosed
    fence. In a full run, an unclosed fence is reported as an
    unidentified work file before any create is planned. The engine test
    still covers the unclosed fence.
- **0377 and 0379** are both drafts on `main`, so this PR lands first.
  - 0377 should pin the new `failed` row for a bare create with a refused
    marker.
  - 0379 should move `previewed_outcome` into the create's preview
    strategy.
