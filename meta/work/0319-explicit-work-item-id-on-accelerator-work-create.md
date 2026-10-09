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
last_updated: "2026-10-09T09:34:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
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
(`cli/work-cli/src/create.rs:184-200`). It cannot see numbers taken on other
branches, so parallel work collides at merge. Today's workaround writes the
file by hand, which skips the `NNNN` substitution, the create-time
validation, and `--push`, leaving the item unsynced.

## Requirements

- Add an `--id <ID>` option to `accelerator work create`. When omitted,
  allocation is unchanged.
- Validate the ID against the configured `work.id_pattern`, including the
  `{key}` prefix where the pattern has one.
- Refuse an ID that an existing work item in the directory already uses.
- Make `--id` compatible with `--push` and `--dry-run`.
- Let the `create-work-item` skill pass a user-requested ID through.

## Acceptance Criteria

- [ ] Given `--id 0400` under `{number:04d}` and no work item 0400, when
      `work create` runs, then it writes `0400-<slug>.md` with `id: "0400"`
      and H1 `# 0400: <title>`.
- [ ] Given `--id` naming an ID an existing work item uses, when `work
      create` runs, then it exits non-zero, naming the existing file, and
      writes nothing.
- [ ] Given `--id` that does not match the configured `id_pattern`, when
      `work create` runs, then it exits non-zero, naming the pattern, and
      writes nothing.
- [ ] Given `--id` with `--push`, when the push succeeds, then the remote
      issue is created and the local file carries both the given ID and the
      returned `external_id`.
- [ ] Given a user asking `/create-work-item` for a specific number, when
      the skill writes the item, then it passes that number via `--id`.

## Open Questions

None.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- Collision checking covers the local work directory only. Other branches
  are invisible to the CLI; choosing a free number across branches stays
  the caller's job.

## Technical Notes

- The `NNNN` substitution and the existence guard live in the same atomic
  write as allocation. `--id` replaces only the allocation step.

## Drafting Notes

- Placed under 0146 by request. It touches the same `work create`
  surface as the sync push.
- Raised while creating 0312–0319, which had to be written by hand to start
  numbering at 0312.

## References

- `cli/work-cli/src/create.rs`, `cli/work/src/next_number.rs`
- Related: 0146, 0312
