---
type: "work-item"
id: "0351"
title: "Sync Preview Hides Pending-Push Refusals"
date: "2026-10-24T11:40:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0300", "work-item:0296"]
tags: ["sync", "preview", "pending-push"]
last_updated: "2026-10-24T11:40:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0351: Sync Preview Hides Pending-Push Refusals

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

`accelerator work sync --preview` reports a create-from-local for an item whose
pending-push marker the apply run would refuse. The preview prints
`create-from-local` and exits 0; the apply run prints `failed … terminal`,
`create failed: pending push refused (<reason>)` and exits 71. A preview is
meant to show every failure the run would meet as `would fail`.

## Context

Found on 2026-10-24 during 0300's manual checks against the `TES` sandbox
team. A create whose `Created` marker write failed left an `Attempted` marker
for item 0004. `--target 0004 --preview` reported
`0004\tcreate-from-local\tunsynced\t-` and exited 0; `--target 0004` then
refused with `prior-attempt-unknown` and exited 71.

The preview path builds its report without reading any marker:
`validate_pushes` attaches a payload check to each planned send, and each entry
of `creates_from_local` is reported `NotApplied`
(`cli/work-adapters/src/sync/run.rs:2326-2342`). Only the apply path evaluates
`push_precondition` (`cli/work-adapters/src/sync/apply.rs:1181`). Adopt and
create orders are previewed (`previewed_adoptions`), so only the bare create
is affected.

`/sync-work-items` tells users to preview first; a clean preview here leads
them into a run that refuses.

## Requirements

Reproduction:

1. Leave an `Attempted` marker at
   `<integrations>/<tracker>/pending-push/<id>.json` for an unsynced item.
2. `accelerator work sync --target <id> --preview` prints `create-from-local`
   and exits 0.
3. `accelerator work sync --target <id>` prints a `terminal` row and exits 71.

Expected: the preview reports the refusal the apply run would meet.

Changes:

- The preview evaluates each create-from-local's marker with the same
  precondition the apply run uses, and reports a refusal as the failed row and
  `error: <id>: create would fail: pending push refused (<reason>): …` line the
  apply run would print.
- The preview exit code follows the same precedence as the apply run's.
- A `Created` marker the apply run would reuse previews as the link it would
  make, not as a new create.

## Acceptance Criteria

- [ ] Given an `Attempted` marker, when a preview runs, then the item's row is
      `failed\tunsynced\tterminal`, stderr carries `create would fail: pending
      push refused (prior-attempt-unknown)`, and it exits 71.
- [ ] Given an unreadable marker, a `Created` marker whose fingerprint no
      longer matches, or a `Created` marker whose key another item carries,
      when a preview runs, then each reports the refusal the apply run reports.
- [ ] Given a `Created` marker the apply run would link, when a preview runs,
      then it reports no new create.
- [ ] Given no marker, when a preview runs, then its output is unchanged.
- [ ] A preview never writes or removes a marker.

## Open Questions

- Should a reused `Created` marker preview as its own action word rather than
  `create-from-local`?

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- Reading a marker during a preview is acceptable: it is local state the
  apply run reads anyway.

## Technical Notes

- `push_precondition` (`cli/work/src/sync/push_precondition.rs:125`) is pure
  and takes the marker state, request digest and a corpus-carries predicate,
  so the preview can call it with the inputs `create_from_local_steps`
  (`cli/work-adapters/src/sync/apply.rs:1164`) builds.

## References

- Found during: `meta/plans/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently.md`
  (Phase 7 manual verification)
- Skill: `skills/work/sync-work-items/SKILL.md`
