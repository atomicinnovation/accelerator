---
type: "work-item"
id: "PP-926"
title: "Sync Preview Hides Pending-Push Refusals"
date: "2026-10-24T11:40:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "in-progress"
kind: "bug"
priority: "medium"
parent: "work-item:PP-167"
relates_to: ["work-item:PP-884", "work-item:PP-880", "work-item:PP-927"]
tags: ["sync", "preview", "pending-push"]
last_updated: "2026-11-17T14:14:34+00:00"
last_updated_by: "Phil Helm"
schema_version: 1
external_id: "PP-926"
aliases: ["0351"]
---
# PP-926: Sync Preview Hides Pending-Push Refusals

**Kind**: Bug
**Status**: In Progress
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`accelerator work sync --preview` reports a create-from-local for an item whose
pending-push marker the apply run would refuse. The preview prints
`create-from-local` and exits 0; the apply run prints a `terminal` row,
`create failed: pending push refused (<reason>)` and exits 71. A preview is
meant to show, as `would fail`, every failure the run can determine from local
state, and a pending-push refusal is one.

## Context

Found on 2026-10-24 during 0300's manual checks against the `TES` sandbox
team. A create whose `Created` marker write failed left an `Attempted` marker
for item 0004. `--target 0004 --preview` reported
`0004\tcreate-from-local\tunsynced\t-` and exited 0; `--target 0004` then
refused with `prior-attempt-unknown` and exited 71.

The preview path builds its report without reading any marker:
`validate_pushes` attaches a payload check to each planned send, and each entry
of `creates_from_local` is reported `NotApplied`
(`cli/work-adapters/src/sync/run.rs`, the `RunMode::Preview` branch of the
run). Only the apply path evaluates `push_precondition`
(`create_from_local` in `cli/work-adapters/src/sync/apply.rs`).

This item covers a *bare create*: a create-from-local with no `--resolve`
order. A create carrying a `--resolve <id>=adopt:<KEY>` or
`--resolve <id>=create` order goes through `settle_order`, which the preview
does not check either. Its local refusals belong with 0379's per-entry
preview strategy and are out of scope here.

`/sync-work-items` tells users to preview first. To cover this gap, its "Read
the run's output" section tells the agent to read each previewed
create-from-local's marker by hand and warn that the preview cannot confirm
that item. Fixing the preview retires that workaround.

## Requirements

Reproduction:

1. Leave an `Attempted` marker at
   `<integrations>/<tracker>/pending-push/<id>.json` for an unsynced item.
2. `accelerator work sync --target <id> --preview` prints `create-from-local`
   and exits 0.
3. `accelerator work sync --target <id>` prints a `terminal` row and exits 71.

Expected: the preview reports the refusal the apply run would meet.

Changes:

- For each bare create, the preview reads the item's marker the way the apply
  run does, including distrusting a marker that version control tracks or that
  records a key the tracker could not have issued, and evaluates it with the
  same precondition and key-carrier check.
- A refusal is reported as the apply run reports it, with `failed` read as
  `would fail`: the row `<id>\tfailed\tunsynced\tterminal` and the stderr line
  `error: <id>: create would fail: pending push refused (<reason>): <detail>`,
  where `<detail>`, including any remedy hint, is the apply run's text.
- A reusable `Created` marker whose reuse the apply run would defer because the
  corpus is incomplete is reported as the failure the apply run reports.
- A bare create whose work file cannot be read or parsed, which the apply run
  fails before it reads the marker, is reported as the failure the apply run
  reports.
- A reusable `Created` marker the apply run would link previews exactly as the
  apply run reports it.
- The preview's exit code follows the apply run's precedence,
  `71 > 1 > 4 > 75 > 74 > 70`.
- `skills/work/sync-work-items/SKILL.md` drops the manual marker check for
  previewed creates ("A preview does not yet read a create's pending-push
  marker…"), and its `--preview` paragraph adds pending-push refusals to what
  a preview carries. `tests/unit/tasks/test_sync_work_items_skill_structure.py`
  follows.
- `CHANGELOG.md` notes that a preview can now exit 71 for a create the run
  would refuse, for callers that gate on a preview exiting 0.

Out of scope:

- The `#\tdiscovery\tskipped\tpending-push` record. It is derived from the
  `pending-push/` directory while the run is prepared, which the preview and
  the apply run share, so it already matches.
- The action word for a reused `Created` marker. Both the preview and the
  apply run print `create-from-local`, so they already agree.
- Failures the tracker reports only when the run applies.
- Local refusals of a create carrying a `--resolve` order.

## Acceptance Criteria

Each criterion runs the preview against a fixture whose `pending-push/`
directory has identical entries and bytes before and after the preview.

- [ ] Given an `Attempted` marker, when a preview runs, then the item's row is
      `<id>\tfailed\tunsynced\tterminal`, stderr carries
      `error: <id>: create would fail: pending push refused
      (prior-attempt-unknown): `, and it exits 71.
- [ ] Given an unreadable marker, when a preview runs, then the row is
      `<id>\tfailed\tunsynced\tterminal`, stderr carries
      `create would fail: pending push refused (marker-unreadable): `, and it
      exits 71.
- [ ] Given a marker that version control tracks, or a `Created` marker whose
      key is not a tracker key, when a preview runs, then it reports the
      `marker-unreadable` refusal with the apply run's distrust cause, and it
      exits 71.
- [ ] Given a `Created` marker whose request digest no longer matches the
      item, when a preview runs, then it reports the `fingerprint-mismatch`
      refusal and exits 71.
- [ ] Given a `Created` marker whose key another work item carries, and
      separately one whose key this run's identity settlement would follow,
      when a preview runs, then each reports the `already-written` refusal,
      without the remedy hint, and exits 71.
- [ ] Given a reusable `Created` marker and a corpus with an unreadable work
      file, when a preview runs, then it reports the same `failed` row, error
      line and exit code as the apply run on a copy of the same fixture.
- [ ] Given a bare create whose work file is undecodable or has frontmatter
      that cannot be parsed, when a preview runs, then it reports the same
      `failed` row, error line and exit code as the apply run on a copy of the
      same fixture.
- [ ] Given a reusable `Created` marker and a complete corpus, when a preview
      runs, then the row is `<id>\tcreate-from-local\tunsynced\t-`, stderr has
      no `error: <id>:` line, and it exits 0.
- [ ] Given one item with an `Attempted` marker and another with an unresolved
      conflict, when a preview runs, then it exits 71, as the apply run does
      on a copy of the same fixture.
- [ ] Given no marker, when a preview runs, then the row is
      `<id>\tcreate-from-local\tunsynced\t-`, stderr has no `error: <id>:`
      line, it exits 0, and `pending-push/` is still empty.
- [ ] The `/sync-work-items` skill no longer tells the agent to read
      `pending-push/<id>.json` after a preview, and its `--preview` paragraph
      names pending-push refusals.

## Open Questions

None.

## Dependencies

- Blocked by: none. 0300 (with 0296) merged in PR #169 and provides
  `push_precondition`, the refusal text and the exit precedence this reuses.
  0352 merged in PR #188.
- Blocks: none.
- Coordinates with 0377 and 0379, drafted on PR #224 under epic 0372. 0377
  pins the sync stdout row contract: whichever of 0377 and 0351 lands second
  updates the other's preview expectations. 0379 restructures the apply loop
  and expects this precondition check to live in the create's preview
  strategy; if 0379 lands first, the fix goes there.

## Assumptions

- Reading a marker during a preview is acceptable: it is local state the
  apply run reads anyway.

## Technical Notes

- `push_precondition` (`cli/work/src/sync/push_precondition.rs`) is pure and
  takes the marker state, request digest and a corpus-carries predicate. The
  preview can call it with the inputs `create_from_local_steps` builds, once
  the marker reading and distrust in `ItemApplier::read_marker` are shared
  between the two paths.
- The refusal's `<detail>` comes from `refused_marker_state` and the marker
  path, and the remedy hint from `ApplyError::cause`; the preview should reuse
  both rather than restate them.

## References

- Found during: `meta/plans/2026-10-11-PP-884-sync-pulls-of-tracked-work-items-fail-silently.md`
  (Phase 7 manual verification)
- Skill: `skills/work/sync-work-items/SKILL.md`
- Review: `meta/reviews/work/PP-926-sync-preview-hides-pending-push-refusals-review-1.md`
