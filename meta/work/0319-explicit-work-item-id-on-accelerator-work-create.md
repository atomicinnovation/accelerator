---
type: "work-item"
id: "0319"
title: "Explicit Work Item ID on accelerator work create"
date: "2026-10-09T09:34:53+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0146"
relates_to: ["work-item:0312"]
tags: ["work", "cli", "allocation"]
last_updated: "2026-10-27T17:39:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-905"
---

# 0319: Explicit Work Item ID on accelerator work create

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As someone creating work items while other branches have unmerged items,
I want to pass the work item ID to `accelerator work create`, so that new
items avoid numbers already taken elsewhere without bypassing the CLI's
atomic write and tracker push.

## Context

`work create` always takes max+1 over the local work directory
(`allocate_id` in `cli/work-cli/src/create.rs`). It cannot see numbers taken
on other branches, so parallel work collides at merge. Today's workaround
writes the file by hand, which skips the `NNNN` substitution, the
create-time validation, and `--push`, leaving the item unsynced.

## Requirements

- Add an `--id <ID>` option to `accelerator work create`. When omitted,
  allocation is unchanged.
- Require the exact full ID the configured `work.id_pattern` produces,
  including the `{key}` prefix where the pattern has one. Accept any key
  the pattern's key syntax allows, not only `work.key`. Do not normalise
  short or unpadded numbers.
- Refuse an ID that an existing work item in the directory already uses.
- Refuse `--id` when the pattern is `{tracker}`, since the tracker owns
  every ID.
- Refuse `--id` together with `--project`, since the ID already carries its
  key.
- Refuse, under every pattern, an ID that `canonical_draft_id` accepts,
  since that form is reserved for provisional drafts.
- Make `--id` compatible with `--push` and `--dry-run`.
- Let the `create-work-item` skill pass a user-requested ID through.

## Acceptance Criteria

- [ ] Given `--id 0400` under `{number:04d}` and no work item 0400, when
      `work create` runs, then it writes `0400-<slug>.md` with `id: "0400"`
      and H1 `# 0400: <title>`.
- [ ] Given `--id XY-0400` under `{key}-{number:04d}` with `work.key: PP`
      and no work item `XY-0400`, when `work create` runs, then it writes
      `XY-0400-<slug>.md` with `id: "XY-0400"`.
- [ ] Given `--id` naming an ID an existing work item's filename uses,
      under any slug, when `work create` runs, then it exits non-zero,
      naming the existing file, and writes nothing.
- [ ] Given two concurrent `work create` runs with the same `--id`, when
      both complete, then exactly one writes the item and the other exits
      non-zero as an ID collision.
- [ ] Given `--id` that does not match the configured `id_pattern`, when
      `work create` runs, then it exits non-zero, naming the pattern, and
      writes nothing.
- [ ] Given `--id 400` under `{number:04d}`, when `work create` runs, then
      it exits non-zero as a pattern mismatch and writes nothing.
- [ ] Given the `{tracker}` pattern, when `work create` runs with `--id`,
      then it exits non-zero, naming the pattern, and writes nothing.
- [ ] Given `--id` and `--project` together, when `work create` runs, then
      it exits non-zero and writes nothing.
- [ ] Given an `--id` that `canonical_draft_id` accepts, in any case, when
      `work create` runs, then it exits non-zero and writes nothing.
- [ ] Given `--id draft-0400` under `{key}-{number:04d}` with
      `work.key: draft`, when `work create` runs, then it writes the item,
      since a digit-only suffix is not a draft ID.
- [ ] Given each `--id` refusal above, when `work create` exits, then each
      refusal carries its own distinct `E_` code.
- [ ] Given `--id` with `--push`, when the push succeeds, then the remote
      issue is created and the local file carries both the given ID and the
      returned `external_id`.
- [ ] Given `--id` naming a taken ID with `--push`, when `work create`
      runs, then it exits non-zero, sends no create to the tracker, and
      writes nothing.
- [ ] Given `--id` with `--push --dry-run`, when `work create` runs, then
      it runs the pattern, `{tracker}`, `--project`, draft-reservation and
      collision checks, prints the preview, and writes nothing.
- [ ] Given `--dry-run` with an `--id` another work item uses, when `work
      create` runs, then it reports the collision without taking the
      create lock.
- [ ] Given a user asking `/create-work-item` for a specific ID, when the
      skill writes the item, then it passes that ID via `--id`, presents it
      as the projected ID without calling `work next-number`, and leaves
      collision detection to the CLI.
- [ ] Given the `{tracker}` pattern and a user asking `/create-work-item`
      for a specific ID, when the skill starts, then it refuses before
      drafting, explaining that the tracker owns every ID.

## Open Questions

None.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- Collision checking covers the local work directory only. Other branches
  are invisible to the CLI; choosing a free number across branches stays
  the caller's job.
- Automatic allocation stays max+1. After `--id 0400`, the next automatic ID
  is 0401 and the numbers below it stay unused unless passed explicitly.
  Filling gaps would reintroduce cross-branch collisions.

## Technical Notes

- The `NNNN` substitution lives in the same atomic write as allocation, so
  `--id` takes the place of the `allocate_id` step inside the create lock.
  That step runs before `push_legacy_item`, so a refusal sends nothing.
- The existing guard, `refuse_to_overwrite`, checks only the exact
  `{id}-{slug}.md` path and misses a same-ID file under another slug. The
  collision check is new: a filename scan with `compile_scan_regex` keyed
  on the key parsed from the `--id`, not on `work.key`, run under the
  create lock.
- `--dry-run` currently returns from `run_with` before `creation_context`,
  so the `--id` checks need adding to that path. Its collision scan is
  advisory and takes no lock.
- `extract-work-items` and `create-batch` are out of scope.
- `create.rs` already refuses `--project` under tracker ownership
  (`IdOwnership::Tracker`). The `--id` refusal can follow the same check.

## Drafting Notes

- Placed under 0146 by request. It touches the same `work create`
  surface as the sync push.
- Raised while creating 0312–0319, which had to be written by hand to start
  numbering at 0312.
- `--id` requires the exact ID, unlike `work resolve`. Resolve is a fuzzy
  matcher for finding an existing item; `--id` names the file that gets
  written, so it should not guess.

## References

- `cli/work-cli/src/create.rs`, `cli/work/src/next_number.rs`
- Related: 0146, 0312
