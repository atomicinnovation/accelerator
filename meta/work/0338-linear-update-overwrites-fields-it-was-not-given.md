---
type: "work-item"
id: "0338"
title: "Linear Update Overwrites Fields It Was Not Given"
date: "2026-10-15T00:01:33+00:00"
author: "Toby Clemson"
producer: "accelerator-work"
status: "draft"
kind: "bug"
priority: "high"
parent: "work-item:0146"
relates_to: ["work-item:0048", "work-item:0296"]
tags: ["linear", "cli", "update"]
last_updated: "2026-10-15T00:01:33+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-921"
---
# 0338: Linear Update Overwrites Fields It Was Not Given

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

`accelerator linear update` writes fields the caller did not name and drops
fields the caller did. Without `--title` it sends an empty title, which Linear
rejects with exit `34`; without `--description` it sends an empty description,
which Linear accepts, silently erasing the issue's body. `--assignee-id` and
`--priority` are parsed and never sent, and `--state` sends only the state,
discarding every other flag given with it.

## Context

Found on 2026-10-15 while repairing `PP-714` during 0296's merge-time
verification: `linear update PP-714 --description "…"` exited `34` with
`linear update failed`, and succeeded only once `--title` repeated the issue's
current title.

`run_update` turns each absent flag into an empty string
(`args.title.unwrap_or_default()`, `args.description.unwrap_or_default()`) and
calls `update_op(id, Some(&title), &body)`. `update_op` already omits the
title when given `None`, but always sends `description`. The
`update-linear-issue` skill tells the user that at least one mutating flag is
required and confirms "the updated fields", so a title-only update it
proposes would erase the description after the user approved a title change.

`work sync` is unaffected: it calls the tracker port's `update` and `retitle`
with both values or the title alone, never through `run_update`.

## Requirements

Reproduction:

1. `accelerator linear update <KEY> --description "x"` exits `34`, and the
   issue is unchanged.
2. `accelerator linear update <KEY> --title "x"` exits 0, and the issue's
   description is now empty.
3. `accelerator linear update <KEY> --priority 1` sends an empty title and
   description and no priority, so it exits `34` and writes nothing.

Step 1 was observed; steps 2 and 3 follow from the code and were not run
against a real issue.

Expected: each command changes exactly the fields named, and no other.

Changes:

- `update` sends one `issueUpdate` whose input holds exactly the fields given,
  with each value as typed: `title`, `description`, `assigneeId`, `priority`,
  and `stateId` resolved through the catalogue as today.
- A `--state` combined with other flags joins the same `issueUpdate`; it never
  applies only the state.
- `update` reports success as `updated\t<KEY>` whether or not `--state` is
  given. The JSON a `--state` update prints today came from routing it through
  `transition`; once `--state` joins the single `issueUpdate`, that JSON goes
  rather than being rebuilt, so every `update` reports the same way.
- `skills/integrations/linear/update-linear-issue/SKILL.md` Step 4 drops the
  JSON `outcome` case.
- `CHANGELOG.md` `[Unreleased]`: `Fixed`, naming the erased-description case.

Out of scope: error reporting stays `linear update failed`, `--priority` and
`--assignee-id` are not validated locally, there is no way to unassign, and
neither `linear transition` nor the input `work sync` sends changes.

## Acceptance Criteria

- [ ] Given `update <KEY> --description "x"`, when it runs, then the request's
      input is `{"description": "x"}` and it exits 0.
- [ ] Given `update <KEY> --title "x"`, when it runs, then the request's input
      is `{"title": "x"}` and the description is untouched.
- [ ] Given `update <KEY> --priority 1`, when it runs, then the request's
      input is `{"priority": 1}` and it exits 0.
- [ ] Given `update <KEY> --assignee-id <ID>`, when it runs, then the
      request's input is `{"assigneeId": "<ID>"}` and it exits 0.
- [ ] Given `--state` with `--title`, when `update` runs, then one request is
      sent whose input carries both `stateId` and `title`.
- [ ] Given an unknown or ambiguous `--state` name alongside other flags, when
      `update` runs, then it refuses before any request.
- [ ] Given `--description ""`, when `update` runs, then the request's input
      is `{"description": ""}`.
- [ ] Given any successful `update`, with or without `--state`, then stdout is
      `updated\t<KEY>`.
- [ ] Given no mutating flag, when `update` runs, then it still refuses with
      `E_UPDATE_NO_OPS` before any request.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- Linear's `issueUpdate` accepts `assigneeId` and `priority` alongside
  `title`, `description` and `stateId` in one input.

## Technical Notes

- `stateId` in `issueUpdate` is already proven: `transition`
  (`cli/linear-client/src/transition.rs:52-62`) sends it through the same
  mutation.
- `run_update` (`cli/linear-cli/src/main.rs:299-341`) and `UpdateArgs`
  (`cli/linear-cli/src/cli.rs:59-73`).
- `LinearClient::update_op` (`cli/linear-client/src/client.rs:721-735`) also
  backs the tracker port's `update`, which `work sync` calls; the input sync
  sends is pinned by `cli/linear-client/tests/port.rs:224-237`.
- `cli/linear-cli/tests/flow_update.rs` covers only a title-and-description
  update and the no-flag refusal.

## References

- Found during: `meta/plans/2026-10-10-0296-sync-round-trip-defects.md`
  (Phase 11 merge-time verification)
- Skill: `skills/integrations/linear/update-linear-issue/SKILL.md`
