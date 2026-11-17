---
type: "pr-description"
id: "234"
title: "[0351] Preview the pending-push refusal a create would meet"
date: "2026-11-17T16:14:05+00:00"
author: "Phil Helm"
producer: "describe-pr"
status: "complete"
work_item_id: "0351"
parent: "work-item:0351"
relates_to: ["plan:2026-11-17-0351-sync-preview-hides-pending-push-refusals"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/234"
pr_number: 234
tags: ["sync", "preview", "pending-push"]
revision: "90da644edff4f7111f2704a0dd84b17bfc6e0ec3"
repository: "accelerator"
last_updated: "2026-11-17T16:14:05+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
---

# [0351] Preview the pending-push refusal a create would meet

## Summary

`accelerator work sync --preview` said a create-from-local was clean even
when the run would refuse it because of its pending-push marker. The
preview printed `create-from-local` and exited 0. The run then printed a
`terminal` row and exited 71.

Now, for each create without a `--resolve` order, the preview reads the
item's marker and makes the same decision the run makes. It reports a
refusal exactly as the run does, with `failed` read as `would fail`. With
this fixed, `/sync-work-items` no longer needs its manual marker check.

## Changes

- **One decision for a create with no `--resolve` order**
  (`work-adapters/src/sync/apply.rs`):
  - `MarkerReading::read` used to be the applier's private `read_marker`.
    It still distrusts a marker that version control tracks, or one that
    records a key the tracker couldn't have issued.
  - `bare_create_decision` maps `push_precondition` to `Proceed`,
    `ReuseId(key)`, or the error the run reports. That error is either
    the `PendingPushRefused` refusal or the `CorpusIncomplete` deferral of
    a reuse.
  - Neither needs an `ItemApplier`. The applier now calls them itself, so
    the run's behaviour is unchanged.
- **The inputs a create is built from** (`run.rs`):
  `LocalCreateInputs::gather` reads the work file and derives the
  marker's path. `create_request` builds the `CreateFromLocalRequest`.
  The run and the preview both use them.
- **The preview branch** (`run.rs`): a new `previewed_outcome` runs that
  decision for each bare create. It reports an error as `Failed` and
  anything else as `NotApplied`.
  - The key-carrier closure moves above the preview branch, so a key that
    this run's identity settlement follows is refused in both modes.
  - A create carrying a `--resolve` order still previews as it did; its
    marker is checked only on the run.
- **What a preview can now report**, each exactly as the run reports it:
  - a refused pending push (`prior-attempt-unknown`, `marker-unreadable`,
    `fingerprint-mismatch`, `already-written`), including the run's
    distrust causes and remedy hint;
  - a reuse deferred because the corpus is incomplete;
  - a work file that can't be read or parsed;
  - an id that can't name a marker.
- **The skill, CLI help, guide and changelog**:
  - `/sync-work-items` drops the instruction to read
    `pending-push/<id>.json` after a preview. Both of its statements of
    what a preview carries now include the create's failure.
  - The `work sync` help, its golden file and the user guide say the same.
  - `CHANGELOG.md` has a Fixed entry and a Breaking pointer.
- **Work item, plan and reviews**: the 0351 work item with its review,
  and the plan with its two reviews.

## Breaking

A preview can now exit non-zero where it used to exit 0:
- 71 for a refused pending push;
- 75 for an unparseable work file, or a reuse blocked by one;
- 70 for an unreadable work file, or a reuse blocked by an environment
  fault.

A whole-corpus preview over an outstanding `Attempted` marker used to
exit 70 for the retryable `pending-push` discovery deferral. It now exits
71, a signal not to retry. Scripts that read `work sync` exit codes, or
that require a preview to exit 0, need updating.

## Context

- Work item: `meta/work/0351-sync-preview-hides-pending-push-refusals.md`
- Plan: `meta/plans/2026-11-17-0351-sync-preview-hides-pending-push-refusals.md`
- The bug was found during 0300's manual checks against the `TES` Linear
  sandbox.

## Testing

Each test was written first and seen to fail for the right reason before
the preview branch changed. The guards passed both before and after.

- [x] Engine tests (`work-adapters`, `--test sync_create --test
      sync_settled`): 196 passed. The new preview tests cover:
  - each refusal reason;
  - tracked, unknown and non-tracker-key markers;
  - a reuse deferred over both corpus blockers;
  - undecodable and unclosed-fence work files;
  - an id that can't name a marker;
  - a key that this run's identity settlement follows, in both modes.

  Guards:
  - a reusable marker previews as `NotApplied`;
  - `adopt`, `create` and relink orders preview exactly as before.

  Every test asserts that nothing is sent and that nothing under
  `pending-push/` is written or removed.
- [x] CLI tests (`accelerator-work --bin`): 474 passed. They compare a
      preview with an applied run on a copy of the same fixture. Repository
      roots are normalised and `would fail` reads as `failed`, then stdout,
      stderr and exit code must match for six seedings: `Attempted`,
      unreadable, digest-mismatched, key carried by another item, reuse
      beside a malformed item, and unparseable frontmatter. Further tests
      cover:
  - a committed marker;
  - a refusal outranking a conflict;
  - a refusal outranking the discovery deferral;
  - the clean-create guard.

  `pending_push_tree` shows the markers are byte-identical after every
  preview.
- [x] Skill content test (`test_sync_work_items_skill_structure.py`): 74
      passed. The new test failed before the skill edit.
- [x] `cli_surface` golden test passes.
- [x] `mise run test` exits 0: 7432 CLI tests and 2615 frontend tests,
      plus the integration suites.
- [x] `frontend:check`, `server:check`, `cli:check`, `pup:check`,
      `public-api:check`, `notices:check`, `build-system:check` and
      `scripts:check` all pass.
- [ ] `deny:check` fails on RUSTSEC-2026-0542: `humantime`, a dependency
      of the visualiser, is unmaintained. This PR doesn't touch
      `Cargo.lock`, so the check fails on the base commit too.
- [ ] `docs:check` wasn't run locally, because it needs network access
      and Chromium. The guide change is one sentence.
- [ ] Manual check against the `TES` sandbox:
  - Leave an `Attempted` marker on an unsynced item.
  - `--target <id> --preview` prints `<id>\tfailed\tunsynced\tterminal`
    and the `would fail` line, and exits 71. The marker is unchanged.
  - Deleting the marker turns the same preview back into
    `create-from-local`, exiting 0.
- [ ] `/sync-work-items --preview` with an `Attempted` marker reports a
      `terminal` refusal and points to the recovery skill, without reading
      the marker itself.

Failing (red) run, before the preview branch changed:

```
FAIL accelerator-work sync::tests::a_preview_reports_a_pending_push_refusal_as_the_run_would
FAIL accelerator-work sync::tests::a_preview_distrusts_a_committed_marker
FAIL accelerator-work sync::tests::a_whole_corpus_preview_ranks_a_refusal_above_the_discovery_deferral
FAIL accelerator-work sync::tests::a_preview_and_the_run_report_the_same_create_failure
FAIL accelerator-work sync::tests::a_previewed_refusal_outranks_a_conflict
PASS accelerator-work sync::tests::a_preview_without_a_refusal_is_a_clean_create
FAIL work-adapters::sync_create a_preview_reports_each_pending_push_refusal_the_run_would
FAIL work-adapters::sync_create a_preview_distrusts_a_marker_as_the_run_does
FAIL work-adapters::sync_create a_preview_defers_a_reuse_while_the_corpus_is_incomplete
FAIL work-adapters::sync_create a_preview_reports_an_unusable_create_the_run_would
FAIL work-adapters::sync_create a_preview_reports_an_id_that_cannot_name_a_marker
FAIL work-adapters::sync_settled a_create_whose_marker_key_is_followed_this_run_is_refused
PASS work-adapters::sync_create a_previewed_reusable_marker_reports_no_failure
PASS work-adapters::sync_create a_preview_leaves_an_ordered_create_to_its_order
PASS work-adapters::sync_create a_preview_leaves_a_relink_to_its_order
```

## Notes for Reviewers

- **Departures from the plan:**
  - `settle_order` works out the request digest itself, from
    `request.digest()`, instead of taking it as a parameter. Clippy
    rejected the owned `String` parameter (`needless_pass_by_value`).
  - The CLI comparison test uses invalid frontmatter YAML instead of an
    unclosed fence. In a full CLI run, an unclosed fence is reported as
    an unidentified work file before any create is planned, so it never
    reached the code under test. The engine test still covers the
    unclosed fence directly.
- **Coordination with 0377 and 0379:** both are still drafts on `main`,
  so this PR lands first.
  - 0377 pins the sync stdout row contract, so it needs to pin the new
    `failed` row for a bare create with a refused marker.
  - 0379 should move `previewed_outcome` into the create's preview
    strategy.
- **Out of scope**: a create carrying a `--resolve` order still has its
  marker checked only on the run.
