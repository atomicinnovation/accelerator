---
type: "work-item"
id: "0353"
title: "Sync Work Items Skill Ignores the Summary Record"
date: "2026-10-24T11:40:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "low"
parent: "work-item:0146"
relates_to: ["work-item:0300"]
tags: ["sync", "sync-work-items", "skill", "documentation"]
last_updated: "2026-10-24T11:40:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-928"
---
# 0353: Sync Work Items Skill Ignores the Summary Record

**Kind**: Task
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

`accelerator work sync` ends its report with `#\tsummary\tsynced\t<N>`, the
count of items already in sync, but neither `/sync-work-items` nor the user
guide documents the record. The skill's dispatch list and membership table
have no entry for it, so its summary cannot say how many items were already
in sync, and a reader of the raw report has to guess what it means.

## Context

Found on 2026-10-24 during 0300's manual checks: whole-corpus and targeted runs
printed `#\tsummary\tsynced\t4` and `#\tsummary\tsynced\t1`, and the skill had
no rule to apply to them.

`render_report` (`cli/work-cli/src/sync.rs:543-645`) leaves each item planned
as `synced` out of the rows and counts it instead. It emits the record when
that count is above zero or when the run has no other row, so an empty corpus
still reports `synced 0`.

## Requirements

- `skills/work/sync-work-items/SKILL.md` **Read the run's output** gains a
  dispatch entry for `#\tsummary\tsynced\t<N>`: informational, never blocking
  completion.
- Step 5's summary gains an `in sync: <N> items` line drawn from it.
- The membership table lists it as non-blocking.
- The user guide's description of the report names the record.
- The skill's content check pins the new entry.

## Acceptance Criteria

- [ ] Given a report carrying `#\tsummary\tsynced\t3`, when the skill
      summarises, then it says three items were already in sync and does not
      treat the record as a failure.
- [ ] Given a report with no summary record, when the skill summarises, then
      it omits the line.
- [ ] `tests/unit/tasks/test_sync_work_items_skill_structure.py` fails when the
      dispatch entry is removed.

## Open Questions

- None.

## Dependencies

- Blocked by: none.
- Blocks: none.

## Assumptions

- The record's shape is stable; this item documents it and does not change it.

## Technical Notes

- `render_report_emits_the_summary_row_for_an_empty_corpus`
  (`cli/work-cli/src/sync.rs:3264`) pins the empty-corpus case.

## References

- Found during: `meta/plans/2026-10-11-0300-sync-pulls-of-tracked-work-items-fail-silently.md`
  (Phase 7 manual verification)
- Skill: `skills/work/sync-work-items/SKILL.md`
